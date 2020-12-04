package node

import (
	"context"
	"fmt"
	"net"
	"sort"
	"strconv"
	"time"

	"github.com/digitalocean/godo"
	"github.com/golang/geo/s2"
)

const digitalOceanOS = "ubuntu-20-04-x64"

var digitalOceanRegions = []*Region{
	{
		Name:   "nyc1",
		City:   "New York City, United States",
		LatLng: s2.LatLngFromDegrees(40.6943, -73.9249),
	},
	{
		Name:   "nyc3",
		City:   "New Jersey, United States",
		LatLng: s2.LatLngFromDegrees(40.6943, -73.9249),
	},
	{
		Name:   "ams3",
		City:   "Amsterdam, The Netherlands",
		LatLng: s2.LatLngFromDegrees(52.3500, 4.9166),
	},
	{
		Name:   "sfo2",
		City:   "San Francisco, United States",
		LatLng: s2.LatLngFromDegrees(37.7562, -122.4430),
	},
	{
		Name:   "sfo3",
		City:   "San Francisco, United States",
		LatLng: s2.LatLngFromDegrees(37.7562, -122.4430),
	},
	{
		Name:   "sgp1",
		City:   "Loyang, Singapore",
		LatLng: s2.LatLngFromDegrees(1.2930, 103.8558),
	},
	{
		Name:   "lon1",
		City:   "London, United Kingdom",
		LatLng: s2.LatLngFromDegrees(51.5000, -0.1167),
	},
	{
		Name:   "fra1",
		City:   "Frankfurt, Germany",
		LatLng: s2.LatLngFromDegrees(50.1000, 8.6750),
	},
	{
		Name:   "tor1",
		City:   "Toronto, Canada",
		LatLng: s2.LatLngFromDegrees(43.7000, -79.4200),
	},
	{
		Name:   "blr1",
		City:   "Bengaluru, India",
		LatLng: s2.LatLngFromDegrees(12.9700, 77.5600),
	},
}

// NewDigitalOceanDriver ...
func NewDigitalOceanDriver(token string) *DigitalOceanDriver {
	return &DigitalOceanDriver{
		client: godo.NewFromToken(token),
	}
}

// DigitalOceanDriver ...
type DigitalOceanDriver struct {
	client *godo.Client
}

// Provider ...
func (d *DigitalOceanDriver) Provider() string {
	return "digitalocean"
}

func (d *DigitalOceanDriver) DefaultUser() string {
	return "root"
}

// Regions ...
func (d *DigitalOceanDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return append(make([]*Region, 0, len(digitalOceanRegions)), digitalOceanRegions...), nil
}

// SKUs ...
func (d *DigitalOceanDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	skus := []*SKU{}

	opt := &godo.ListOptions{}
	for {
		sizes, res, err := d.client.Sizes.List(ctx, opt)
		if err != nil {
			return nil, err
		}

		for _, size := range sizes {
			if req.Region != "" {
				if sort.SearchStrings(size.Regions, req.Region) != len(size.Regions) {
					skus = append(skus, digitalOceanSKU(&size))
				}
			} else {
				skus = append(skus, digitalOceanSKU(&size))
			}
		}

		if res.Links == nil || res.Links.IsLastPage() {
			break
		}

		page, err := res.Links.CurrentPage()
		if err != nil {
			return nil, err
		}

		opt.Page = page + 1
	}

	return skus, nil
}

func digitalOceanSKU(size *godo.Size) *SKU {
	return &SKU{
		Name:         size.Slug,
		CPUs:         size.Vcpus,
		Memory:       size.Memory,
		Disk:         size.Disk,
		NetworkCap:   int(size.Transfer * 1024),
		NetworkSpeed: 1000,
		PriceHourly: &Price{
			Value:    size.PriceHourly,
			Currency: "USD",
		},
		PriceMonthly: &Price{
			Value:    size.PriceMonthly,
			Currency: "USD",
		},
	}
}

func (d *DigitalOceanDriver) findOrAddKey(ctx context.Context, public string) (*godo.Key, error) {
	opt := &godo.ListOptions{}
	for {
		keys, resp, err := d.client.Keys.List(ctx, opt)
		if err != nil {
			return nil, err
		}

		for _, key := range keys {
			if key.PublicKey == public {
				return &key, nil
			}
		}

		if resp.Links == nil || resp.Links.IsLastPage() {
			break
		}

		page, err := resp.Links.CurrentPage()
		if err != nil {
			return nil, err
		}

		opt.Page = page + 1
	}

	key, _, err := d.client.Keys.Create(ctx, &godo.KeyCreateRequest{
		Name:      fmt.Sprintf("infra-key-%d", time.Now().UnixNano()),
		PublicKey: public,
	})
	return key, err
}

// Create ...
func (d *DigitalOceanDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	key, err := d.findOrAddKey(ctx, req.SSHKey)
	if err != nil {
		return nil, err
	}

	droplet, _, err := d.client.Droplets.Create(ctx, &godo.DropletCreateRequest{
		Name:   req.Name,
		Region: req.Region,
		Size:   req.SKU,
		Image: godo.DropletCreateImage{
			Slug: digitalOceanOS,
		},
		SSHKeys: []godo.DropletCreateSSHKey{
			{Fingerprint: key.Fingerprint},
		},
		IPv6: true,
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
			droplet, _, err := d.client.Droplets.Get(ctx, droplet.ID)
			if err != nil {
				return nil, err
			}
			if droplet.Status == "active" {
				return digitalOceanNode(droplet), nil
			}
		}
	}
}

// List ...
func (d *DigitalOceanDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	nodes := []*Node{}

	opt := &godo.ListOptions{}
	for {
		droplets, resp, err := d.client.Droplets.List(ctx, opt)
		if err != nil {
			return nil, err
		}

		for _, droplet := range droplets {
			nodes = append(nodes, digitalOceanNode(&droplet))
		}

		if resp.Links == nil || resp.Links.IsLastPage() {
			break
		}

		page, err := resp.Links.CurrentPage()
		if err != nil {
			return nil, err
		}

		opt.Page = page + 1
	}

	return nodes, nil
}

// Delete ...
func (d *DigitalOceanDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	id, err := strconv.Atoi(req.ProviderID)
	if err != nil {
		return fmt.Errorf("invalid provider id: %w", err)
	}
	_, err = d.client.Droplets.Delete(ctx, id)
	return err
}

func digitalOceanNode(droplet *godo.Droplet) *Node {
	node := &Node{
		ProviderID: strconv.Itoa(droplet.ID),
		Name:       droplet.Name,
		Memory:     droplet.Memory,
		CPUs:       droplet.Vcpus,
		Disk:       droplet.Disk,
		Networks:   &Networks{},
		Status:     droplet.Status,
		SKU:        digitalOceanSKU(droplet.Size),
	}

	for _, ipv4 := range droplet.Networks.V4 {
		if isPublicIP(net.ParseIP(ipv4.IPAddress)) {
			node.Networks.V4 = append(node.Networks.V4, ipv4.IPAddress)
		}
	}
	for _, ipv6 := range droplet.Networks.V6 {
		if isPublicIP(net.ParseIP(ipv6.IPAddress)) {
			node.Networks.V6 = append(node.Networks.V6, ipv6.IPAddress)
		}
	}

	for _, region := range digitalOceanRegions {
		if region.Name == droplet.Region.Slug {
			node.Region = region
		}
	}

	return node
}

func isPublicIP(IP net.IP) bool {
	if IP.IsLoopback() || IP.IsLinkLocalMulticast() || IP.IsLinkLocalUnicast() {
		return false
	}
	if ip4 := IP.To4(); ip4 != nil {
		switch {
		case ip4[0] == 10:
			return false
		case ip4[0] == 172 && ip4[1] >= 16 && ip4[1] <= 31:
			return false
		case ip4[0] == 192 && ip4[1] == 168:
			return false
		default:
			return true
		}
	}
	return false
}
