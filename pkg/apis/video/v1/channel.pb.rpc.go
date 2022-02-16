package video

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterVideoChannelFrontendService ...
func RegisterVideoChannelFrontendService(host rpc.ServiceRegistry, service VideoChannelFrontendService) {
	host.RegisterMethod("strims.video.v1.VideoChannelFrontend.List", service.List)
	host.RegisterMethod("strims.video.v1.VideoChannelFrontend.Get", service.Get)
	host.RegisterMethod("strims.video.v1.VideoChannelFrontend.Create", service.Create)
	host.RegisterMethod("strims.video.v1.VideoChannelFrontend.Update", service.Update)
	host.RegisterMethod("strims.video.v1.VideoChannelFrontend.Delete", service.Delete)
}

// VideoChannelFrontendService ...
type VideoChannelFrontendService interface {
	List(
		ctx context.Context,
		req *VideoChannelListRequest,
	) (*VideoChannelListResponse, error)
	Get(
		ctx context.Context,
		req *VideoChannelGetRequest,
	) (*VideoChannelGetResponse, error)
	Create(
		ctx context.Context,
		req *VideoChannelCreateRequest,
	) (*VideoChannelCreateResponse, error)
	Update(
		ctx context.Context,
		req *VideoChannelUpdateRequest,
	) (*VideoChannelUpdateResponse, error)
	Delete(
		ctx context.Context,
		req *VideoChannelDeleteRequest,
	) (*VideoChannelDeleteResponse, error)
}

// VideoChannelFrontendClient ...
type VideoChannelFrontendClient struct {
	client rpc.Caller
}

// NewVideoChannelFrontendClient ...
func NewVideoChannelFrontendClient(client rpc.Caller) *VideoChannelFrontendClient {
	return &VideoChannelFrontendClient{client}
}

// List ...
func (c *VideoChannelFrontendClient) List(
	ctx context.Context,
	req *VideoChannelListRequest,
	res *VideoChannelListResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoChannelFrontend.List", req, res)
}

// Get ...
func (c *VideoChannelFrontendClient) Get(
	ctx context.Context,
	req *VideoChannelGetRequest,
	res *VideoChannelGetResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoChannelFrontend.Get", req, res)
}

// Create ...
func (c *VideoChannelFrontendClient) Create(
	ctx context.Context,
	req *VideoChannelCreateRequest,
	res *VideoChannelCreateResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoChannelFrontend.Create", req, res)
}

// Update ...
func (c *VideoChannelFrontendClient) Update(
	ctx context.Context,
	req *VideoChannelUpdateRequest,
	res *VideoChannelUpdateResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoChannelFrontend.Update", req, res)
}

// Delete ...
func (c *VideoChannelFrontendClient) Delete(
	ctx context.Context,
	req *VideoChannelDeleteRequest,
	res *VideoChannelDeleteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoChannelFrontend.Delete", req, res)
}
