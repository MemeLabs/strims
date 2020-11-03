package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterDebugService ...
func RegisterDebugService(host ServiceRegistry, service DebugService) {
	host.RegisterService("Debug", service)
}

// DebugService ...
type DebugService interface {
	PProf(
		ctx context.Context,
		req *pb.PProfRequest,
	) (*pb.PProfResponse, error)
	ReadMetrics(
		ctx context.Context,
		req *pb.ReadMetricsRequest,
	) (*pb.ReadMetricsResponse, error)
}

// DebugClient ...
type DebugClient struct {
	client UnaryCaller
}

// NewDebugClient ...
func NewDebugClient(client UnaryCaller) *DebugClient {
	return &DebugClient{client}
}

// PProf ...
func (c *DebugClient) PProf(
	ctx context.Context,
	req *pb.PProfRequest,
	res *pb.PProfResponse,
) error {
	return c.client.CallUnary(ctx, "Debug/PProf", req, res)
}

// ReadMetrics ...
func (c *DebugClient) ReadMetrics(
	ctx context.Context,
	req *pb.ReadMetricsRequest,
	res *pb.ReadMetricsResponse,
) error {
	return c.client.CallUnary(ctx, "Debug/ReadMetrics", req, res)
}
