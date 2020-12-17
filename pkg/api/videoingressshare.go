package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterVideoIngressShareService ...
func RegisterVideoIngressShareService(host ServiceRegistry, service VideoIngressShareService) {
	host.RegisterMethod("VideoIngressShare/CreateChannel", service.CreateChannel)
	host.RegisterMethod("VideoIngressShare/UpdateChannel", service.UpdateChannel)
	host.RegisterMethod("VideoIngressShare/DeleteChannel", service.DeleteChannel)
}

// VideoIngressShareService ...
type VideoIngressShareService interface {
	CreateChannel(
		ctx context.Context,
		req *pb.VideoIngressShareCreateChannelRequest,
	) (*pb.VideoIngressShareCreateChannelResponse, error)
	UpdateChannel(
		ctx context.Context,
		req *pb.VideoIngressShareUpdateChannelRequest,
	) (*pb.VideoIngressShareUpdateChannelResponse, error)
	DeleteChannel(
		ctx context.Context,
		req *pb.VideoIngressShareDeleteChannelRequest,
	) (*pb.VideoIngressShareDeleteChannelResponse, error)
}

// VideoIngressShareClient ...
type VideoIngressShareClient struct {
	client Caller
}

// NewVideoIngressShareClient ...
func NewVideoIngressShareClient(client Caller) *VideoIngressShareClient {
	return &VideoIngressShareClient{client}
}

// CreateChannel ...
func (c *VideoIngressShareClient) CreateChannel(
	ctx context.Context,
	req *pb.VideoIngressShareCreateChannelRequest,
	res *pb.VideoIngressShareCreateChannelResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngressShare/CreateChannel", req, res)
}

// UpdateChannel ...
func (c *VideoIngressShareClient) UpdateChannel(
	ctx context.Context,
	req *pb.VideoIngressShareUpdateChannelRequest,
	res *pb.VideoIngressShareUpdateChannelResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngressShare/UpdateChannel", req, res)
}

// DeleteChannel ...
func (c *VideoIngressShareClient) DeleteChannel(
	ctx context.Context,
	req *pb.VideoIngressShareDeleteChannelRequest,
	res *pb.VideoIngressShareDeleteChannelResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngressShare/DeleteChannel", req, res)
}
