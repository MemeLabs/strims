package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterCAPeerService ...
func RegisterCAPeerService(host ServiceRegistry, service CAPeerService) {
	host.RegisterMethod("CAPeer/Renew", service.Renew)
}

// CAPeerService ...
type CAPeerService interface {
	Renew(
		ctx context.Context,
		req *pb.CAPeerRenewRequest,
	) (*pb.CAPeerRenewResponse, error)
}

// CAPeerClient ...
type CAPeerClient struct {
	client Caller
}

// NewCAPeerClient ...
func NewCAPeerClient(client Caller) *CAPeerClient {
	return &CAPeerClient{client}
}

// Renew ...
func (c *CAPeerClient) Renew(
	ctx context.Context,
	req *pb.CAPeerRenewRequest,
	res *pb.CAPeerRenewResponse,
) error {
	return c.client.CallUnary(ctx, "CAPeer/Renew", req, res)
}
