package node

import (
	"context"
	"strings"

	"github.com/golang/geo/s2"
)

type BillingType string

const (
	Monthly BillingType = "monthly"
	Hourly  BillingType = "hourly"
)

// A Driver is defines the implementation of a third party driver such as
// DigitalOcean. The driver is used to facilitate provisioning and tearing
// down resources.
type Driver interface {
	// Provider returns the name of the current provider configured
	Provider() string
	// Regions provides a list of available regions for the current credentials
	Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error)
	// SKUs returns a list of available machine specs and prices
	SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error)
	// Create provisions a new Node on the provider's service
	Create(ctx context.Context, req *CreateRequest) (*Node, error)
	// List returns existing nodes for the provider
	List(ctx context.Context, req *ListRequest) ([]*Node, error)
	// Delete allows for shutting down and deleting of a node
	Delete(ctx context.Context, req *DeleteRequest) error
	// DefaultUser returns the user allowed for connection and operations
	DefaultUser() string
}

// RegionsRequest ...
type RegionsRequest struct{}

// SKUsRequest ...
type SKUsRequest struct {
	Region string
}

// CreateRequest ...
type CreateRequest struct {
	Name   string
	Region string
	SKU    string
	SSHKey string
	// hourly(0) | monthly(1)
	BillingType BillingType
}

// ListRequest ...
type ListRequest struct {
}

// DeleteRequest ...
type DeleteRequest struct {
	Region     string
	ProviderID string
}

// Region represents the Node's datacenter location
type Region struct {
	Name   string
	City   string
	LatLng s2.LatLng
}

// Regions ...
type Regions []*Region

// SKU ...
type SKU struct {
	Name         string
	CPUs         int
	Memory       int
	Disk         int
	NetworkCap   int
	NetworkSpeed int
	PriceMonthly *Price
	PriceHourly  *Price
}

// Price ...
type Price struct {
	Value    float64
	Currency string
}

// Node represents a host
type Node struct {
	ProviderID       string    `json:"provider_id,omitempty"`
	Name             string    `json:"name,omitempty"`
	Memory           int       `json:"memory,omitempty"`
	CPUs             int       `json:"vcpus,omitempty"`
	Disk             int       `json:"disk,omitempty"`
	Networks         *Networks `json:"networks,omitempty"`
	Status           string    `json:"status,omitempty"`
	Region           *Region   `json:"region,omitempty"`
	SKU              *SKU      `json:"sku,omitempty"`
	WireguardPubKey  string    `json:"wireguard_pub_key,omitempty"` // TODO(jbpratt): do we really need this?
	WireguardPrivKey string    `json:"wireguard_priv_key,omitempty"`
	WireguardIPv4    string    `json:"wireguard_ipv4,omitempty"`
}

// Networks represents the Node's networks.
type Networks struct {
	V4 []string `json:"v4,omitempty"`
	V6 []string `json:"v6,omitempty"`
}

func (r *Regions) FindByName(name string) *Region {
	for _, reg := range *r {
		if name == reg.Name {
			return reg
		}
	}
	return nil
}

func ValidRegion(region string, regions []*Region) bool {
	for _, reg := range regions {
		if reg.Name == region {
			return true
		}
	}
	return false
}

func ValidSKU(skuName string, skus []*SKU) bool {
	for _, sku := range skus {
		if sku.Name == skuName {
			return true
		}
	}
	return false
}

func IsPriviledged(user string) bool {
	return strings.ToLower(user) == "root"
}
