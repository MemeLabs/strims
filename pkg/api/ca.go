package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterCAService ...
func RegisterCAService(host ServiceRegistry, service CAService) {
	host.RegisterMethod("CA/Renew", service.Renew)
}

// CAService ...
type CAService interface {
	Renew(
		ctx context.Context,
		req *pb.CARenewRequest,
	) (*pb.CARenewResponse, error)
}

// CAClient ...
type CAClient struct {
	client *rpc.Client
}

// NewCAClient ...
func NewCAClient(client *rpc.Client) *CAClient {
	return &CAClient{client}
}

// Renew ...
func (c *CAClient) Renew(
	ctx context.Context,
	req *pb.CARenewRequest,
	res *pb.CARenewResponse,
) error {
	return c.client.CallUnary(ctx, "CA/Renew", req, res)
}
