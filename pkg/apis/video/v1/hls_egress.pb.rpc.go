package video

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterHLSEgressService ...
func RegisterHLSEgressService(host rpc.ServiceRegistry, service HLSEgressService) {
	host.RegisterMethod("strims.video.v1.HLSEgress.IsSupported", service.IsSupported)
	host.RegisterMethod("strims.video.v1.HLSEgress.OpenStream", service.OpenStream)
	host.RegisterMethod("strims.video.v1.HLSEgress.CloseStream", service.CloseStream)
}

// HLSEgressService ...
type HLSEgressService interface {
	IsSupported(
		ctx context.Context,
		req *HLSEgressIsSupportedRequest,
	) (*HLSEgressIsSupportedResponse, error)
	OpenStream(
		ctx context.Context,
		req *HLSEgressOpenStreamRequest,
	) (*HLSEgressOpenStreamResponse, error)
	CloseStream(
		ctx context.Context,
		req *HLSEgressCloseStreamRequest,
	) (*HLSEgressCloseStreamResponse, error)
}

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
