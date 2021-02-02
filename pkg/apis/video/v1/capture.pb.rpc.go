package video

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterCaptureService ...
func RegisterCaptureService(host rpc.ServiceRegistry, service CaptureService) {
	host.RegisterMethod("strims.video.v1.Capture.Open", service.Open)
	host.RegisterMethod("strims.video.v1.Capture.Update", service.Update)
	host.RegisterMethod("strims.video.v1.Capture.Append", service.Append)
	host.RegisterMethod("strims.video.v1.Capture.Close", service.Close)
}

// CaptureService ...
type CaptureService interface {
	Open(
		ctx context.Context,
		req *CaptureOpenRequest,
	) (*CaptureOpenResponse, error)
	Update(
		ctx context.Context,
		req *CaptureUpdateRequest,
	) (*CaptureUpdateResponse, error)
	Append(
		ctx context.Context,
		req *CaptureAppendRequest,
	) (*CaptureAppendResponse, error)
	Close(
		ctx context.Context,
		req *CaptureCloseRequest,
	) (*CaptureCloseResponse, error)
}

// CaptureClient ...
type CaptureClient struct {
	client rpc.Caller
}

// NewCaptureClient ...
func NewCaptureClient(client rpc.Caller) *CaptureClient {
	return &CaptureClient{client}
}

// Open ...
func (c *CaptureClient) Open(
	ctx context.Context,
	req *CaptureOpenRequest,
	res *CaptureOpenResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.Capture.Open", req, res)
}

// Update ...
func (c *CaptureClient) Update(
	ctx context.Context,
	req *CaptureUpdateRequest,
	res *CaptureUpdateResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.Capture.Update", req, res)
}

// Append ...
func (c *CaptureClient) Append(
	ctx context.Context,
	req *CaptureAppendRequest,
	res *CaptureAppendResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.Capture.Append", req, res)
}

// Close ...
func (c *CaptureClient) Close(
	ctx context.Context,
	req *CaptureCloseRequest,
	res *CaptureCloseResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.Capture.Close", req, res)
}
