package node

import (
	"context"
)

type CustomDriver struct{}

func NewCustomDriver() *CustomDriver {
	return &CustomDriver{}
}

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

func (d *CustomDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	return nil, nil
}

func (d *CustomDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	return &Node{
		User:         req.User,
		Driver:       "custom",
		ProviderName: "custom",
		ProviderId:   "",
		Name:         req.Name,
		Networks: &Networks{
			V4: []string{req.IPV4},
		},
		Status: "active",
		Region: &Region{
			Name:   req.Name,
			City:   "",
			LatLng: LatLngFromDegrees(0, 0),
		},
		Sku: &SKU{
			Name:         req.Name,
			Cpus:         0,
			Memory:       0,
			Disk:         0,
			NetworkCap:   0,
			NetworkSpeed: 0,
			PriceMonthly: &Price{
				Value:    0,
				Currency: "USD",
			},
			PriceHourly: &Price{
				Value:    0,
				Currency: "USD",
			},
		},
	}, nil
}

func (d *CustomDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	return nil
}
