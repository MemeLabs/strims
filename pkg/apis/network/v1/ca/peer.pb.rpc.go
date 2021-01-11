package ca

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterCAPeerService ...
func RegisterCAPeerService(host api.ServiceRegistry, service CAPeerService) {
	host.RegisterMethod(".strims.network.v1.ca.CAPeer.Renew", service.Renew)
}

// CAPeerService ...
type CAPeerService interface {
	Renew(
		ctx context.Context,
		req *CAPeerRenewRequest,
	) (*CAPeerRenewResponse, error)
}

// CAPeerClient ...
type CAPeerClient struct {
	client api.Caller
}

// NewCAPeerClient ...
func NewCAPeerClient(client api.Caller) *CAPeerClient {
	return &CAPeerClient{client}
}

// Renew ...
func (c *CAPeerClient) Renew(
	ctx context.Context,
	req *CAPeerRenewRequest,
	res *CAPeerRenewResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.ca.CAPeer.Renew", req, res)
}
