package node

import (
	"context"
	"fmt"
	"strconv"
	"time"

	"github.com/hetznercloud/hcloud-go/hcloud"
)

const hetznerOS = "ubuntu-20.04"

// NewHetznerDriver ...
func NewHetznerDriver(token string) *HetznerDriver {
	return &HetznerDriver{
		client: hcloud.NewClient(hcloud.WithToken(token)),
	}
}

// HetznerDriver ...
type HetznerDriver struct {
	client *hcloud.Client
}

// Provider ...
func (d *HetznerDriver) Provider() string {
	return "hetzner"
}

func (d *HetznerDriver) DefaultUser() string {
	return "root"
}

// Regions ...
func (d *HetznerDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	locations, err := d.client.Location.All(ctx)
	if err != nil {
		return nil, err
	}
	regions := []*Region{}
	for _, location := range locations {
		regions = append(regions, hetznerRegion(location))
	}
	return regions, nil
}

func hetznerRegion(location *hcloud.Location) *Region {
	return &Region{
		Name:         location.Name,
		City:         fmt.Sprintf("%s, %s", location.City, location.Country),
		LatitudeDeg:  location.Latitude,
		LongitudeDeg: location.Longitude,
	}
}

// SKUs ...
func (d *HetznerDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	skus := []*SKU{}

	serverTypes, err := d.client.ServerType.All(ctx)
	if err != nil {
		return nil, err
	}

	for _, serverType := range serverTypes {
		if req.Region != "" {
			for _, pricing := range serverType.Pricings {
				if pricing.Location.Name == req.Region {
					sku, err := hetznerSKU(serverType, pricing)
					if err != nil {
						return nil, err
					}
					skus = append(skus, sku)
				}
			}
		} else {
			sku, err := hetznerSKU(serverType, serverType.Pricings[0])
			if err != nil {
				return nil, err
			}
			skus = append(skus, sku)
		}
	}

	return skus, nil
}

func hetznerSKU(serverType *hcloud.ServerType, pricing hcloud.ServerTypeLocationPricing) (*SKU, error) {
	pricingHourly, err := strconv.ParseFloat(pricing.Hourly.Gross, 64)
	if err != nil {
		return nil, err
	}
	pricingMonthly, err := strconv.ParseFloat(pricing.Monthly.Gross, 64)
	if err != nil {
		return nil, err
	}

	return &SKU{
		Name:         serverType.Name,
		Cpus:         int32(serverType.Cores),
		Memory:       int32(serverType.Memory * 1024),
		Disk:         int32(serverType.Disk),
		NetworkCap:   20 * 1024,
		NetworkSpeed: 1000,
		PriceHourly: &Price{
			Value:    pricingHourly,
			Currency: "EUR",
		},
		PriceMonthly: &Price{
			Value:    pricingMonthly,
			Currency: "EUR",
		},
	}, nil
}

func (d *HetznerDriver) findOrAddKey(ctx context.Context, public string) (*hcloud.SSHKey, error) {
	keys, err := d.client.SSHKey.All(ctx)
	if err != nil {
		return nil, err
	}

	for _, key := range keys {
		if key.PublicKey == public {
			return key, nil
		}
	}

	key, _, err := d.client.SSHKey.Create(ctx, hcloud.SSHKeyCreateOpts{
		Name:      fmt.Sprintf("key-%d", time.Now().UnixNano()),
		PublicKey: public,
	})
	return key, err
}

// Create ...
func (d *HetznerDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	key, err := d.findOrAddKey(ctx, req.SSHKey)
	if err != nil {
		return nil, err
	}

	createRes, _, err := d.client.Server.Create(ctx, hcloud.ServerCreateOpts{
		Name: req.Name,
		ServerType: &hcloud.ServerType{
			Name: req.SKU,
		},
		Image: &hcloud.Image{
			Name: hetznerOS,
		},
		SSHKeys: []*hcloud.SSHKey{key},
		Location: &hcloud.Location{
			Name: req.Region,
		},
		StartAfterCreate: hcloud.Bool(true),
	})
	if err != nil {
		return nil, err
	}

	checkTick := time.NewTicker(5 * time.Second)
	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-checkTick.C:
			server, _, err := d.client.Server.Get(ctx, strconv.Itoa(createRes.Server.ID))
			if err != nil {
				return nil, err
			}
			if server.Status == hcloud.ServerStatusRunning {
				return hetznerNode(server)
			}
		}
	}
}

// List ...
func (d *HetznerDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	nodes := []*Node{}

	servers, err := d.client.Server.All(ctx)
	if err != nil {
		return nil, err
	}
	for _, server := range servers {
		node, err := hetznerNode(server)
		if err != nil {
			return nil, err
		}
		nodes = append(nodes, node)
	}

	return nodes, nil
}

// Delete ...
func (d *HetznerDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	id, err := strconv.Atoi(req.ProviderID)
	if err != nil {
		return fmt.Errorf("invalid provider id: %w", err)
	}
	_, err = d.client.Server.Delete(ctx, &hcloud.Server{
		ID: id,
	})
	return err
}

func hetznerNode(server *hcloud.Server) (*Node, error) {
	node := &Node{
		ProviderId: strconv.Itoa(server.ID),
		Name:       server.Name,
		Networks: &Networks{
			V4: []string{server.PublicNet.IPv4.IP.String()},
			V6: []string{server.PublicNet.IPv6.IP.String()},
		},
		Status: string(server.Status),
		Region: hetznerRegion(server.Datacenter.Location),
	}

	for _, pricing := range server.ServerType.Pricings {
		if pricing.Location.Name == server.Datacenter.Location.Name {
			sku, err := hetznerSKU(server.ServerType, pricing)
			if err != nil {
				return nil, err
			}
			node.Sku = sku
		}
	}

	return node, nil
}
