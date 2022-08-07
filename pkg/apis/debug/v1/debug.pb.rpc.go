package debugv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterDebugService ...
func RegisterDebugService(host rpc.ServiceRegistry, service DebugService) {
	host.RegisterMethod("strims.debug.v1.Debug.PProf", service.PProf)
	host.RegisterMethod("strims.debug.v1.Debug.ReadMetrics", service.ReadMetrics)
	host.RegisterMethod("strims.debug.v1.Debug.WatchMetrics", service.WatchMetrics)
	host.RegisterMethod("strims.debug.v1.Debug.GetConfig", service.GetConfig)
	host.RegisterMethod("strims.debug.v1.Debug.SetConfig", service.SetConfig)
	host.RegisterMethod("strims.debug.v1.Debug.StartMockStream", service.StartMockStream)
	host.RegisterMethod("strims.debug.v1.Debug.StopMockStream", service.StopMockStream)
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
	GetConfig(
		ctx context.Context,
		req *GetConfigRequest,
	) (*GetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *SetConfigRequest,
	) (*SetConfigResponse, error)
	StartMockStream(
		ctx context.Context,
		req *StartMockStreamRequest,
	) (*StartMockStreamResponse, error)
	StopMockStream(
		ctx context.Context,
		req *StopMockStreamRequest,
	) (*StopMockStreamResponse, error)
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

func (s *UnimplementedDebugService) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
) (*GetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDebugService) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
) (*SetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDebugService) StartMockStream(
	ctx context.Context,
	req *StartMockStreamRequest,
) (*StartMockStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDebugService) StopMockStream(
	ctx context.Context,
	req *StopMockStreamRequest,
) (*StopMockStreamResponse, error) {
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

// GetConfig ...
func (c *DebugClient) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
	res *GetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.debug.v1.Debug.GetConfig", req, res)
}

// SetConfig ...
func (c *DebugClient) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
	res *SetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.debug.v1.Debug.SetConfig", req, res)
}

// StartMockStream ...
func (c *DebugClient) StartMockStream(
	ctx context.Context,
	req *StartMockStreamRequest,
	res *StartMockStreamResponse,
) error {
	return c.client.CallUnary(ctx, "strims.debug.v1.Debug.StartMockStream", req, res)
}

// StopMockStream ...
func (c *DebugClient) StopMockStream(
	ctx context.Context,
	req *StopMockStreamRequest,
	res *StopMockStreamResponse,
) error {
	return c.client.CallUnary(ctx, "strims.debug.v1.Debug.StopMockStream", req, res)
}
