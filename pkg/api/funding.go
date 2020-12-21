package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterFundingService ...
func RegisterFundingService(host ServiceRegistry, service FundingService) {
	host.RegisterMethod("Funding/Test", service.Test)
}

// FundingService ...
type FundingService interface {
	Test(
		ctx context.Context,
		req *pb.FundingTestRequest,
	) (*pb.FundingTestResponse, error)
}

// FundingClient ...
type FundingClient struct {
	client Caller
}

// NewFundingClient ...
func NewFundingClient(client Caller) *FundingClient {
	return &FundingClient{client}
}

// Test ...
func (c *FundingClient) Test(
	ctx context.Context,
	req *pb.FundingTestRequest,
	res *pb.FundingTestResponse,
) error {
	return c.client.CallUnary(ctx, "Funding/Test", req, res)
}
