package debug

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterDebugService ...
func RegisterDebugService(host rpc.ServiceRegistry, service DebugService) {
	host.RegisterMethod("strims.debug.v1.Debug.PProf", service.PProf)
	host.RegisterMethod("strims.debug.v1.Debug.ReadMetrics", service.ReadMetrics)
	host.RegisterMethod("strims.debug.v1.Debug.WatchMetrics", service.WatchMetrics)
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
	WatchMetrics(
		ctx context.Context,
		req *WatchMetricsRequest,
	) (<-chan *WatchMetricsResponse, error)
}

// DebugService ...
type UnimplementedDebugService struct{}

func (s *UnimplementedDebugService) PProf(
	ctx context.Context,
	req *PProfRequest,
) (*PProfResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDebugService) ReadMetrics(
	ctx context.Context,
	req *ReadMetricsRequest,
) (*ReadMetricsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDebugService) WatchMetrics(
	ctx context.Context,
	req *WatchMetricsRequest,
) (<-chan *WatchMetricsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ DebugService = (*UnimplementedDebugService)(nil)

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

// WatchMetrics ...
func (c *DebugClient) WatchMetrics(
	ctx context.Context,
	req *WatchMetricsRequest,
	res chan *WatchMetricsResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.debug.v1.Debug.WatchMetrics", req, res)
}
