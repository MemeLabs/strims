package debug

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterDebugService ...
func RegisterDebugService(host rpc.ServiceRegistry, service DebugService) {
	host.RegisterMethod("strims.debug.v1.Debug.PProf", service.PProf)
	host.RegisterMethod("strims.debug.v1.Debug.ReadMetrics", service.ReadMetrics)
}

// DebugService ...
type DebugService interface {
	PProf(
		ctx context.Context,
		req *PProfRequest,
	) (*PProfResponse, error)
	ReadMetrics(
		ctx context.Context,
		req *ReadMetricsRequest,
	) (*ReadMetricsResponse, error)
}

// DebugClient ...
type DebugClient struct {
	client rpc.Caller
}

// NewDebugClient ...
func NewDebugClient(client rpc.Caller) *DebugClient {
	return &DebugClient{client}
}

// PProf ...
func (c *DebugClient) PProf(
	ctx context.Context,
	req *PProfRequest,
	res *PProfResponse,
) error {
	return c.client.CallUnary(ctx, "strims.debug.v1.Debug.PProf", req, res)
}

// ReadMetrics ...
func (c *DebugClient) ReadMetrics(
	ctx context.Context,
	req *ReadMetricsRequest,
	res *ReadMetricsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.debug.v1.Debug.ReadMetrics", req, res)
}
