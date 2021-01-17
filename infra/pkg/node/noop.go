package node

import (
	"context"
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/golang/geo/s2"
)

var _ Driver = (*NoopDriver)(nil)

type NoopDriver struct{}

func NewNoopDriver() *NoopDriver {
	return &NoopDriver{}
}

// Provider ...
func (d *NoopDriver) Provider() string {
	return "noop"
}

func (d *NoopDriver) DefaultUser() string {
	return ""
}

func (d *NoopDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return []*Region{
		{
			Name:   "NY",
			City:   "New York City, United States",
			LatLng: s2.LatLngFromDegrees(40.6943, -73.9249),
		},
		{
			Name:   "AM",
			City:   "Amsterdam, The Netherlands",
			LatLng: s2.LatLngFromDegrees(52.3500, 4.9166),
		},
	}, nil
}

func (d *NoopDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	return []*SKU{
		{
			Name:         "one",
			CPUs:         1,
			Memory:       2000,
			Disk:         0,
			NetworkCap:   0,
			NetworkSpeed: 100,
			PriceMonthly: &Price{
				Value:    4.3200,
				Currency: "USD",
			},
			PriceHourly: &Price{
				Value:    0.0120,
				Currency: "USD",
			},
		},
		{
			Name:         "two",
			CPUs:         2,
			Memory:       7000,
			Disk:         0,
			NetworkCap:   0,
			NetworkSpeed: 250,
			PriceMonthly: &Price{
				Value:    32.0040,
				Currency: "USD",
			},
			PriceHourly: &Price{
				Value:    0.0889,
				Currency: "USD",
			},
		},
		{
			Name:         "three",
			CPUs:         4,
			Memory:       15000,
			Disk:         0,
			NetworkCap:   0,
			NetworkSpeed: 250,
			PriceMonthly: &Price{
				Value:    60.8040,
				Currency: "USD",
			},
			PriceHourly: &Price{
				Value:    0.1689,
				Currency: "USD",
			},
		},
	}, nil
}

func (d *NoopDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	return nil, nil
}

func (d *NoopDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	skus, _ := d.SKUs(ctx, nil)

	var sku *SKU
	for _, s := range skus {
		if req.SKU == s.Name {
			sku = s
		}
	}

	if sku == nil {
		return nil, errors.New("invalid sku")
	}

	regions, _ := d.Regions(ctx, nil)

	var region *Region
	for _, r := range regions {
		if req.Region == r.Name {
			region = r
		}
	}

	id, err := dao.GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	return &Node{
		User:         d.DefaultUser(),
		Driver:       "noop",
		ProviderName: d.Provider(),
		ProviderID:   fmt.Sprint(id),
		Name:         req.Name,
		Memory:       sku.Memory,
		CPUs:         sku.CPUs,
		Disk:         sku.Disk,
		Networks: &Networks{
			V4: []string{req.IPV4},
		},
		Status: "ACTIVE",
		Region: region,
		SKU:    sku,
	}, nil
}

func (d *NoopDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	return nil
}
