package video

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterVideoIngressService ...
func RegisterVideoIngressService(host rpc.ServiceRegistry, service VideoIngressService) {
	host.RegisterMethod("strims.video.v1.VideoIngress.IsSupported", service.IsSupported)
	host.RegisterMethod("strims.video.v1.VideoIngress.GetConfig", service.GetConfig)
	host.RegisterMethod("strims.video.v1.VideoIngress.SetConfig", service.SetConfig)
	host.RegisterMethod("strims.video.v1.VideoIngress.ListStreams", service.ListStreams)
	host.RegisterMethod("strims.video.v1.VideoIngress.GetChannelURL", service.GetChannelURL)
}

// VideoIngressService ...
type VideoIngressService interface {
	IsSupported(
		ctx context.Context,
		req *VideoIngressIsSupportedRequest,
	) (*VideoIngressIsSupportedResponse, error)
	GetConfig(
		ctx context.Context,
		req *VideoIngressGetConfigRequest,
	) (*VideoIngressGetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *VideoIngressSetConfigRequest,
	) (*VideoIngressSetConfigResponse, error)
	ListStreams(
		ctx context.Context,
		req *VideoIngressListStreamsRequest,
	) (*VideoIngressListStreamsResponse, error)
	GetChannelURL(
		ctx context.Context,
		req *VideoIngressGetChannelURLRequest,
	) (*VideoIngressGetChannelURLResponse, error)
}

// VideoIngressClient ...
type VideoIngressClient struct {
	client rpc.Caller
}

// NewVideoIngressClient ...
func NewVideoIngressClient(client rpc.Caller) *VideoIngressClient {
	return &VideoIngressClient{client}
}

// IsSupported ...
func (c *VideoIngressClient) IsSupported(
	ctx context.Context,
	req *VideoIngressIsSupportedRequest,
	res *VideoIngressIsSupportedResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngress.IsSupported", req, res)
}

// GetConfig ...
func (c *VideoIngressClient) GetConfig(
	ctx context.Context,
	req *VideoIngressGetConfigRequest,
	res *VideoIngressGetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngress.GetConfig", req, res)
}

// SetConfig ...
func (c *VideoIngressClient) SetConfig(
	ctx context.Context,
	req *VideoIngressSetConfigRequest,
	res *VideoIngressSetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngress.SetConfig", req, res)
}

// ListStreams ...
func (c *VideoIngressClient) ListStreams(
	ctx context.Context,
	req *VideoIngressListStreamsRequest,
	res *VideoIngressListStreamsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngress.ListStreams", req, res)
}

// GetChannelURL ...
func (c *VideoIngressClient) GetChannelURL(
	ctx context.Context,
	req *VideoIngressGetChannelURLRequest,
	res *VideoIngressGetChannelURLResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngress.GetChannelURL", req, res)
}

// RegisterVideoIngressShareService ...
func RegisterVideoIngressShareService(host rpc.ServiceRegistry, service VideoIngressShareService) {
	host.RegisterMethod("strims.video.v1.VideoIngressShare.CreateChannel", service.CreateChannel)
	host.RegisterMethod("strims.video.v1.VideoIngressShare.UpdateChannel", service.UpdateChannel)
	host.RegisterMethod("strims.video.v1.VideoIngressShare.DeleteChannel", service.DeleteChannel)
}

// VideoIngressShareService ...
type VideoIngressShareService interface {
	CreateChannel(
		ctx context.Context,
		req *VideoIngressShareCreateChannelRequest,
	) (*VideoIngressShareCreateChannelResponse, error)
	UpdateChannel(
		ctx context.Context,
		req *VideoIngressShareUpdateChannelRequest,
	) (*VideoIngressShareUpdateChannelResponse, error)
	DeleteChannel(
		ctx context.Context,
		req *VideoIngressShareDeleteChannelRequest,
	) (*VideoIngressShareDeleteChannelResponse, error)
}

// VideoIngressShareClient ...
type VideoIngressShareClient struct {
	client rpc.Caller
}

// NewVideoIngressShareClient ...
func NewVideoIngressShareClient(client rpc.Caller) *VideoIngressShareClient {
	return &VideoIngressShareClient{client}
}

// CreateChannel ...
func (c *VideoIngressShareClient) CreateChannel(
	ctx context.Context,
	req *VideoIngressShareCreateChannelRequest,
	res *VideoIngressShareCreateChannelResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngressShare.CreateChannel", req, res)
}

// UpdateChannel ...
func (c *VideoIngressShareClient) UpdateChannel(
	ctx context.Context,
	req *VideoIngressShareUpdateChannelRequest,
	res *VideoIngressShareUpdateChannelResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngressShare.UpdateChannel", req, res)
}

// DeleteChannel ...
func (c *VideoIngressShareClient) DeleteChannel(
	ctx context.Context,
	req *VideoIngressShareDeleteChannelRequest,
	res *VideoIngressShareDeleteChannelResponse,
) error {
	return c.client.CallUnary(ctx, "strims.video.v1.VideoIngressShare.DeleteChannel", req, res)
}
