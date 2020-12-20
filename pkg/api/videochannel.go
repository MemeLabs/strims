package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterVideoChannelService ...
func RegisterVideoChannelService(host ServiceRegistry, service VideoChannelService) {
	host.RegisterMethod("VideoChannel/List", service.List)
	host.RegisterMethod("VideoChannel/Create", service.Create)
	host.RegisterMethod("VideoChannel/Update", service.Update)
	host.RegisterMethod("VideoChannel/Delete", service.Delete)
}

// VideoChannelService ...
type VideoChannelService interface {
	List(
		ctx context.Context,
		req *pb.VideoChannelListRequest,
	) (*pb.VideoChannelListResponse, error)
	Create(
		ctx context.Context,
		req *pb.VideoChannelCreateRequest,
	) (*pb.VideoChannelCreateResponse, error)
	Update(
		ctx context.Context,
		req *pb.VideoChannelUpdateRequest,
	) (*pb.VideoChannelUpdateResponse, error)
	Delete(
		ctx context.Context,
		req *pb.VideoChannelDeleteRequest,
	) (*pb.VideoChannelDeleteResponse, error)
}

// VideoChannelClient ...
type VideoChannelClient struct {
	client Caller
}

// NewVideoChannelClient ...
func NewVideoChannelClient(client Caller) *VideoChannelClient {
	return &VideoChannelClient{client}
}

// List ...
func (c *VideoChannelClient) List(
	ctx context.Context,
	req *pb.VideoChannelListRequest,
	res *pb.VideoChannelListResponse,
) error {
	return c.client.CallUnary(ctx, "VideoChannel/List", req, res)
}

// Create ...
func (c *VideoChannelClient) Create(
	ctx context.Context,
	req *pb.VideoChannelCreateRequest,
	res *pb.VideoChannelCreateResponse,
) error {
	return c.client.CallUnary(ctx, "VideoChannel/Create", req, res)
}

// Update ...
func (c *VideoChannelClient) Update(
	ctx context.Context,
	req *pb.VideoChannelUpdateRequest,
	res *pb.VideoChannelUpdateResponse,
) error {
	return c.client.CallUnary(ctx, "VideoChannel/Update", req, res)
}

// Delete ...
func (c *VideoChannelClient) Delete(
	ctx context.Context,
	req *pb.VideoChannelDeleteRequest,
	res *pb.VideoChannelDeleteResponse,
) error {
	return c.client.CallUnary(ctx, "VideoChannel/Delete", req, res)
}
