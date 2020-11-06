package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterDebugService ...
func RegisterDebugService(host ServiceRegistry, service DebugService) {
	host.RegisterMethod("Debug/PProf", service.PProf)
	host.RegisterMethod("Debug/ReadMetrics", service.ReadMetrics)
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
	client *rpc.Client
}

// NewDebugClient ...
func NewDebugClient(client *rpc.Client) *DebugClient {
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
