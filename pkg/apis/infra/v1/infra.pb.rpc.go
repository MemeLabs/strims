package infra

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterInfraService ...
func RegisterInfraService(host rpc.ServiceRegistry, service InfraService) {
	host.RegisterMethod("strims.infra.v1.Infra.GetHistory", service.GetHistory)
}

// InfraService ...
type InfraService interface {
	GetHistory(
		ctx context.Context,
		req *GetHistoryRequest,
	) (*GetHistoryResponse, error)
}

// InfraClient ...
type InfraClient struct {
	client rpc.Caller
}

// NewInfraClient ...
func NewInfraClient(client rpc.Caller) *InfraClient {
	return &InfraClient{client}
}

// GetHistory ...
func (c *InfraClient) GetHistory(
	ctx context.Context,
	req *GetHistoryRequest,
	res *GetHistoryResponse,
) error {
	return c.client.CallUnary(ctx, "strims.infra.v1.Infra.GetHistory", req, res)
}
