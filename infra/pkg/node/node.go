// Package node ...
package node

import (
	"context"

	infrav1 "github.com/MemeLabs/go-ppspp/pkg/apis/infra/v1"
)

type BillingType string

const (
	Monthly BillingType = "monthly"
	Hourly  BillingType = "hourly"
	Custom  BillingType = "custom"
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
	User   string
	IPV4   string
	Name   string
	Region string
	SKU    string
	SSHKey string
	// hourly(0) | monthly(1)
	BillingType BillingType
}

// ListRequest ...
type ListRequest struct{}

// DeleteRequest ...
type DeleteRequest struct {
	Region     string
	ProviderID string
}

// Region represents the Node's datacenter location
type Region = infrav1.Node_Region

// Regions ...
type Regions []*Region

// SKU ...
type SKU = infrav1.Node_SKU

// Price ...
type Price = infrav1.Node_Price

// Node represents a host
type Node = infrav1.Node

// Networks represents the Node's networks.
type Networks = infrav1.Node_Networks

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
