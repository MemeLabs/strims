// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Package node ...
package node

import (
	"context"
	"errors"

	"github.com/golang/geo/s2"
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
	Spot        bool
}

// ListRequest ...
type ListRequest struct{}

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

type NodeType string

const (
	TypeWorker     NodeType = "worker"
	TypeController NodeType = "controller"
)

// String is used both by fmt.Print and by Cobra in help text
func (t *NodeType) String() string {
	return string(*t)
}

// Set must have pointer receiver so it doesn't change the value of a copy
func (t *NodeType) Set(v string) error {
	switch v {
	case "controller", "worker":
		*t = NodeType(v)
		return nil
	default:
		return errors.New(`must be one of "foo", "bar", or "moo"`)
	}
}

// Type is only used in help text
func (t *NodeType) Type() string {
	return "NodeType"
}

// Node represents a host
type Node struct {
	User             string    `json:"user,omitempty"`
	Driver           string    `json:"driver,omitempty"`
	ProviderName     string    `json:"provider_name,omitempty"`
	ProviderID       string    `json:"provider_id,omitempty"`
	Name             string    `json:"name,omitempty"`
	Memory           int       `json:"memory,omitempty"`
	CPUs             int       `json:"vcpus,omitempty"`
	Disk             int       `json:"disk,omitempty"`
	Networks         *Networks `json:"networks,omitempty"`
	Status           bool      `json:"status,omitempty"`
	Region           *Region   `json:"region,omitempty"`
	SKU              *SKU      `json:"sku,omitempty"`
	WireguardPrivKey string    `json:"wireguard_priv_key,omitempty"`
	WireguardIPv4    string    `json:"wireguard_ipv4,omitempty"`
	StartedAt        int64     `json:"started_at,omitempty"`
	StoppedAt        int64     `json:"stopped_at,omitempty"`
	Type             NodeType  `json:"node_type,omitempty"`
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
