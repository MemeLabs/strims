package funding

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterFundingService ...
func RegisterFundingService(host rpc.ServiceRegistry, service FundingService) {
	host.RegisterMethod("strims.funding.v1.Funding.Test", service.Test)
}

// FundingService ...
type FundingService interface {
	Test(
		ctx context.Context,
		req *FundingTestRequest,
	) (*FundingTestResponse, error)
}

// FundingClient ...
type FundingClient struct {
	client rpc.Caller
}

// NewFundingClient ...
func NewFundingClient(client rpc.Caller) *FundingClient {
	return &FundingClient{client}
}

// Test ...
func (c *FundingClient) Test(
	ctx context.Context,
	req *FundingTestRequest,
	res *FundingTestResponse,
) error {
	return c.client.CallUnary(ctx, "strims.funding.v1.Funding.Test", req, res)
}
