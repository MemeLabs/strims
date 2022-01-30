package node

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"time"

	"github.com/davecgh/go-spew/spew"
	"github.com/golang/geo/s2"
	"github.com/gophercloud/gophercloud"
	"github.com/gophercloud/gophercloud/openstack"
	"github.com/gophercloud/gophercloud/openstack/compute/v2/extensions/keypairs"
	"github.com/gophercloud/gophercloud/openstack/compute/v2/flavors"
	"github.com/gophercloud/gophercloud/openstack/compute/v2/images"
	"github.com/gophercloud/gophercloud/openstack/compute/v2/servers"
	"golang.org/x/sync/errgroup"
)

// 20% hourly billing premium over 30 days
const dreamHostHourlyBillingRate = 6.0 / 5 / 30 / 24

const dreamHostOS = "Ubuntu-20.04"

var dreamHostRegion = Region{
	Name:   "RegionOne",
	City:   "Ashburn, Virginia",
	LatLng: s2.LatLngFromDegrees(39.0300, -77.4711),
}

// NewDreamHostDriver ...
func NewDreamHostDriver(tenantID, tenantName, username, password string) (*DreamHostDriver, error) {
	client, err := openstack.AuthenticatedClient(gophercloud.AuthOptions{
		IdentityEndpoint: "https://iad2.dream.io:5000/v2.0",
		TenantID:         tenantID,
		TenantName:       tenantName,
		Username:         username,
		Password:         password,
	})
	if err != nil {
		return nil, err
	}
	return &DreamHostDriver{
		client: client,
	}, nil
}

// DreamHostDriver ...
type DreamHostDriver struct {
	client *gophercloud.ProviderClient
}

// Provider ...
func (d *DreamHostDriver) Provider() string {
	return "dreamHost"
}

func (d *DreamHostDriver) DefaultUser() string {
	return "ubuntu"
}

// Regions ...
func (d *DreamHostDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return []*Region{&dreamHostRegion}, nil
}

// SKUs ...
func (d *DreamHostDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	computeClient, err := openstack.NewComputeV2(d.client, gophercloud.EndpointOpts{
		Region: dreamHostRegion.Name,
	})
	if err != nil {
		return nil, err
	}

	flavorsPager, err := flavors.ListDetail(computeClient, flavors.ListOpts{
		AccessType: flavors.PublicAccess,
	}).AllPages()
	if err != nil {
		return nil, err
	}

	flavorsList, err := flavors.ExtractFlavors(flavorsPager)
	if err != nil {
		return nil, err
	}

	extraSpecs := make([]map[string]string, len(flavorsList))

	var eg errgroup.Group
	for i := range flavorsList {
		i := i
		eg.Go(func() error {
			es, err := flavors.ListExtraSpecs(computeClient, flavorsList[i].ID).Extract()
			if err != nil {
				return err
			}
			extraSpecs[i] = es
			return nil
		})
	}
	if err := eg.Wait(); err != nil {
		return nil, err
	}

	skus := []*SKU{}
	for i := range flavorsList {
		sku, err := dreamHostSKU(&flavorsList[i], extraSpecs[i])
		if err != nil {
			return nil, err
		}
		skus = append(skus, sku)
	}
	return skus, nil
}

func (d *DreamHostDriver) findOrAddKey(ctx context.Context, region, public string) (*keypairs.KeyPair, error) {
	computeClient, err := openstack.NewComputeV2(d.client, gophercloud.EndpointOpts{
		Region: region,
	})
	if err != nil {
		return nil, err
	}

	keypairsPager, err := keypairs.List(computeClient).AllPages()
	if err != nil {
		return nil, err
	}

	keypairsList, err := keypairs.ExtractKeyPairs(keypairsPager)
	if err != nil {
		return nil, err
	}

	for _, keypair := range keypairsList {
		if keypair.PublicKey == public {
			return &keypair, nil
		}
	}

	return keypairs.Create(computeClient, keypairs.CreateOpts{
		Name:      fmt.Sprintf("key-%d", time.Now().UnixNano()),
		PublicKey: public,
	}).Extract()
}

func (d *DreamHostDriver) findSKU(ctx context.Context, region, name string) (*SKU, error) {
	computeClient, err := openstack.NewComputeV2(d.client, gophercloud.EndpointOpts{
		Region: region,
	})
	if err != nil {
		return nil, err
	}

	flavor, err := flavors.Get(computeClient, name).Extract()
	if err != nil {
		return nil, err
	}

	extraSpecs, err := flavors.ListExtraSpecs(computeClient, flavor.ID).Extract()
	if err != nil {
		return nil, err
	}

	return dreamHostSKU(flavor, extraSpecs)
}

// Create ...
func (d *DreamHostDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	computeClient, err := openstack.NewComputeV2(d.client, gophercloud.EndpointOpts{
		Region: req.Region,
	})
	if err != nil {
		return nil, err
	}

	var sku *SKU
	var id string

	var eg errgroup.Group
	eg.Go(func() error {
		imagesPager, err := images.ListDetail(computeClient, images.ListOpts{
			Name: dreamHostOS,
		}).AllPages()
		if err != nil {
			return err
		}

		imagesList, err := images.ExtractImages(imagesPager)
		if err != nil {
			return err
		}

		keyPair, err := d.findOrAddKey(ctx, req.Region, req.SSHKey)
		if err != nil {
			return err
		}

		server, err := servers.Create(computeClient, keypairs.CreateOptsExt{
			CreateOptsBuilder: servers.CreateOpts{
				Name:      req.Name,
				FlavorRef: req.SKU,
				ImageRef:  imagesList[0].ID,
			},
			KeyName: keyPair.Name,
		}).Extract()
		if err != nil {
			return err
		}
		id = server.ID
		return nil
	})

	eg.Go(func() (err error) {
		sku, err = d.findSKU(ctx, req.Region, req.SKU)
		return err
	})

	if err := eg.Wait(); err != nil {
		return nil, err
	}

	checkTick := time.NewTicker(5 * time.Second)
	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-checkTick.C:
			server, err := servers.Get(computeClient, id).Extract()
			if err != nil {
				return nil, err
			}
			if server.Status == "ACTIVE" {
				return dreamHostNode(server, sku)
			}
		}
	}
}

// List ...
func (d *DreamHostDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	computeClient, err := openstack.NewComputeV2(d.client, gophercloud.EndpointOpts{
		Region: dreamHostRegion.Name,
	})
	if err != nil {
		return nil, err
	}

	serversPager, err := servers.List(computeClient, servers.ListOpts{}).AllPages()
	if err != nil {
		return nil, err
	}

	serversList, err := servers.ExtractServers(serversPager)
	if err != nil {
		return nil, err
	}

	skus := map[string]*SKU{}
	var eg errgroup.Group
	for _, server := range serversList {
		id, ok := server.Flavor["id"].(string)
		if !ok {
			return nil, errors.New("flavor missing id")
		}
		if _, ok := skus[id]; ok {
			continue
		}
		skus[id] = nil
		eg.Go(func() error {
			sku, err := d.findSKU(ctx, dreamHostRegion.Name, id)
			if err != nil {
				return err
			}
			skus[id] = sku
			return nil
		})
	}
	if err := eg.Wait(); err != nil {
		return nil, err
	}

	nodes := []*Node{}
	for _, server := range serversList {
		node, err := dreamHostNode(&server, skus[server.Flavor["id"].(string)])
		if err != nil {
			return nil, err
		}
		nodes = append(nodes, node)
	}
	return nodes, nil
}

// Delete ...
func (d *DreamHostDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	computeClient, err := openstack.NewComputeV2(d.client, gophercloud.EndpointOpts{
		Region: req.Region,
	})
	if err != nil {
		return err
	}

	return servers.ForceDelete(computeClient, req.ProviderID).ExtractErr()
}

func dreamHostNode(server *servers.Server, sku *SKU) (*Node, error) {
	networks := &Networks{}
	for _, addrSetIf := range server.Addresses {
		addrSet, ok := addrSetIf.([]interface{})
		if !ok {
			return nil, errors.New("malformed addresses")
		}
		for _, addrIf := range addrSet {
			spew.Dump(addrIf)
			addrMap, ok := addrIf.(map[string]interface{})
			if !ok {
				return nil, errors.New("malformed address entry")
			}
			version, ok := addrMap["version"].(float64)
			if !ok {
				return nil, errors.New("address entry missing version key")
			}
			ip, ok := addrMap["addr"].(string)
			if !ok {
				return nil, errors.New("address entry missing version key")
			}
			switch version {
			case 4:
				networks.V4 = append(networks.V4, ip)
			case 6:
				networks.V6 = append(networks.V6, ip)
			default:
				return nil, errors.New("invalid address version")
			}
		}
	}

	return &Node{
		ProviderID: server.ID,
		Name:       server.Name,
		Memory:     sku.Memory,
		CPUs:       sku.CPUs,
		Disk:       sku.Disk,
		Networks:   networks,
		Status:     server.Status == "ACTIVE" || server.Status == "IN_PROGRESS",
		Region:     &dreamHostRegion,
		SKU:        sku,
	}, nil
}

func dreamHostSKU(flavor *flavors.Flavor, extraSpecs map[string]string) (*SKU, error) {
	priceMonthly, err := strconv.ParseFloat(extraSpecs["billing:monthly"], 64)
	if err != nil {
		return nil, err
	}

	return &SKU{
		Name:         flavor.ID,
		CPUs:         flavor.VCPUs,
		Memory:       flavor.RAM,
		Disk:         flavor.Disk,
		NetworkCap:   0,
		NetworkSpeed: 0,
		PriceMonthly: &Price{
			Value:    priceMonthly,
			Currency: "USD",
		},
		PriceHourly: &Price{
			Value:    priceMonthly * dreamHostHourlyBillingRate,
			Currency: "USD",
		},
	}, nil
}
