package video

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterHLSEgressService ...
func RegisterHLSEgressService(host rpc.ServiceRegistry, service HLSEgressService) {
	host.RegisterMethod("strims.video.v1.HLSEgress.IsSupported", service.IsSupported)
	host.RegisterMethod("strims.video.v1.HLSEgress.GetConfig", service.GetConfig)
	host.RegisterMethod("strims.video.v1.HLSEgress.SetConfig", service.SetConfig)
	host.RegisterMethod("strims.video.v1.HLSEgress.OpenStream", service.OpenStream)
	host.RegisterMethod("strims.video.v1.HLSEgress.CloseStream", service.CloseStream)
}

// HLSEgressService ...
type HLSEgressService interface {
	IsSupported(
		ctx context.Context,
		req *HLSEgressIsSupportedRequest,
	) (*HLSEgressIsSupportedResponse, error)
	GetConfig(
		ctx context.Context,
		req *HLSEgressGetConfigRequest,
	) (*HLSEgressGetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *HLSEgressSetConfigRequest,
	) (*HLSEgressSetConfigResponse, error)
	OpenStream(
		ctx context.Context,
		req *HLSEgressOpenStreamRequest,
	) (*HLSEgressOpenStreamResponse, error)
	CloseStream(
		ctx context.Context,
		req *HLSEgressCloseStreamRequest,
	) (*HLSEgressCloseStreamResponse, error)
}

// HLSEgressService ...
type UnimplementedHLSEgressService struct{}

func (s *UnimplementedHLSEgressService) IsSupported(
	ctx context.Context,
	req *HLSEgressIsSupportedRequest,
) (*HLSEgressIsSupportedResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedHLSEgressService) GetConfig(
	ctx context.Context,
	req *HLSEgressGetConfigRequest,
) (*HLSEgressGetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedHLSEgressService) SetConfig(
	ctx context.Context,
	req *HLSEgressSetConfigRequest,
) (*HLSEgressSetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedHLSEgressService) OpenStream(
	ctx context.Context,
	req *HLSEgressOpenStreamRequest,
) (*HLSEgressOpenStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedHLSEgressService) CloseStream(
	ctx context.Context,
	req *HLSEgressCloseStreamRequest,
) (*HLSEgressCloseStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ HLSEgressService = (*UnimplementedHLSEgressService)(nil)

// HLSEgressClient ...
type HLSEgressClient struct {
	client rpc.Caller
}

// NewHLSEgressClient ...
func NewHLSEgressClient(client rpc.Caller) *HLSEgressClient {
	return &HLSEgressClient{client}
}

// IsSupported ...
func (c *HLSEgressClient) IsSupported(
	ctx context.Context,
	req *HLSEgressIsSupportedRequest,
	res *HLSEgressIsSupportedResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.HLSEgress.IsSupported", req, res)
}

// GetConfig ...
func (c *HLSEgressClient) GetConfig(
	ctx context.Context,
	req *HLSEgressGetConfigRequest,
	res *HLSEgressGetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.HLSEgress.GetConfig", req, res)
}

// SetConfig ...
func (c *HLSEgressClient) SetConfig(
	ctx context.Context,
	req *HLSEgressSetConfigRequest,
	res *HLSEgressSetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.HLSEgress.SetConfig", req, res)
}

// OpenStream ...
func (c *HLSEgressClient) OpenStream(
	ctx context.Context,
	req *HLSEgressOpenStreamRequest,
	res *HLSEgressOpenStreamResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.HLSEgress.OpenStream", req, res)
}

// CloseStream ...
func (c *HLSEgressClient) CloseStream(
	ctx context.Context,
	req *HLSEgressCloseStreamRequest,
	res *HLSEgressCloseStreamResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.HLSEgress.CloseStream", req, res)
}
