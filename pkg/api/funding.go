package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterFundingService ...
func RegisterFundingService(host ServiceRegistry, service FundingService) {
	host.RegisterMethod("Funding/Test", service.Test)
	host.RegisterMethod("Funding/GetSummary", service.GetSummary)
	host.RegisterMethod("Funding/CreateSubPlan", service.CreateSubPlan)
}

// FundingService ...
type FundingService interface {
	Test(
		ctx context.Context,
		req *pb.FundingTestRequest,
	) (*pb.FundingTestResponse, error)
	GetSummary(
		ctx context.Context,
		req *pb.FundingGetSummaryRequest,
	) (*pb.FundingGetSummaryResponse, error)
	CreateSubPlan(
		ctx context.Context,
		req *pb.FundingCreateSubPlanRequest,
	) (*pb.FundingCreateSubPlanResponse, error)
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

// GetSummary ...
func (c *FundingClient) GetSummary(
	ctx context.Context,
	req *pb.FundingGetSummaryRequest,
	res *pb.FundingGetSummaryResponse,
) error {
	return c.client.CallUnary(ctx, "Funding/GetSummary", req, res)
}

// CreateSubPlan ...
func (c *FundingClient) CreateSubPlan(
	ctx context.Context,
	req *pb.FundingCreateSubPlanRequest,
	res *pb.FundingCreateSubPlanResponse,
) error {
	return c.client.CallUnary(ctx, "Funding/CreateSubPlan", req, res)
}
