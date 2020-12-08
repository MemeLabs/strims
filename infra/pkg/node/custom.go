package node

import (
	"context"
	"errors"

	"github.com/golang/geo/s2"
)

type CustomDriver struct{}

// Provider ...
func (d *CustomDriver) Provider() string {
	return "custom"
}

func (d *CustomDriver) DefaultUser() string {
	return ""
}

func (d *CustomDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return nil, nil
}

func (d *CustomDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	return nil, nil
}

func (d *CustomDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	return &Node{
		User:         req.User,
		Driver:       "custom",
		ProviderName: "custom",
		ProviderID:   "",
		Name:         req.Name,
		Memory:       0,
		CPUs:         0,
		Disk:         0,
		Networks: &Networks{
			V4: []string{req.IPV4},
		},
		Status: "active",
		Region: &Region{
			Name: req.Name,
			City: "",
			LatLng: s2.LatLng{
				Lat: 0,
				Lng: 0,
			},
		},
		SKU: &SKU{
			Name:         req.Name,
			CPUs:         0,
			Memory:       0,
			Disk:         0,
			NetworkCap:   0,
			NetworkSpeed: 0,
			PriceMonthly: &Price{
				Value:    0,
				Currency: "",
			},
			PriceHourly: &Price{
				Value:    0,
				Currency: "",
			},
		},
	}, nil
}

func (d *CustomDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	return errors.New("unimplemented")
}
