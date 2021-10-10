package ca

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterCAService ...
func RegisterCAService(host rpc.ServiceRegistry, service CAService) {
	host.RegisterMethod("strims.network.v1.ca.CA.Renew", service.Renew)
	host.RegisterMethod("strims.network.v1.ca.CA.Find", service.Find)
}

// CAService ...
type CAService interface {
	Renew(
		ctx context.Context,
		req *CARenewRequest,
	) (*CARenewResponse, error)
	Find(
		ctx context.Context,
		req *CAFindRequest,
	) (*CAFindResponse, error)
}

// CAClient ...
type CAClient struct {
	client rpc.Caller
}

// NewCAClient ...
func NewCAClient(client rpc.Caller) *CAClient {
	return &CAClient{client}
}

// Renew ...
func (c *CAClient) Renew(
	ctx context.Context,
	req *CARenewRequest,
	res *CARenewResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.ca.CA.Renew", req, res)
}

// Find ...
func (c *CAClient) Find(
	ctx context.Context,
	req *CAFindRequest,
	res *CAFindResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.ca.CA.Find", req, res)
}
