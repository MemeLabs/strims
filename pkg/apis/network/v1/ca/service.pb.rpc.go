package ca

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterCAService ...
func RegisterCAService(host rpc.ServiceRegistry, service CAService) {
	host.RegisterMethod("strims.network.v1.ca.CA.Renew", service.Renew)
}

// CAService ...
type CAService interface {
	Renew(
		ctx context.Context,
		req *CARenewRequest,
	) (*CARenewResponse, error)
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
