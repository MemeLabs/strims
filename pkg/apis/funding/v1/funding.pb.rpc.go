package funding

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterFundingService ...
func RegisterFundingService(host api.ServiceRegistry, service FundingService) {
	host.RegisterMethod(".strims.funding.v1.Funding.Test", service.Test)
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
	client api.Caller
}

// NewFundingClient ...
func NewFundingClient(client api.Caller) *FundingClient {
	return &FundingClient{client}
}

// Test ...
func (c *FundingClient) Test(
	ctx context.Context,
	req *FundingTestRequest,
	res *FundingTestResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.funding.v1.Funding.Test", req, res)
}
