package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterVideoIngressService ...
func RegisterVideoIngressService(host ServiceRegistry, service VideoIngressService) {
	host.RegisterMethod("VideoIngress/IsSupported", service.IsSupported)
	host.RegisterMethod("VideoIngress/GetConfig", service.GetConfig)
	host.RegisterMethod("VideoIngress/SetConfig", service.SetConfig)
	host.RegisterMethod("VideoIngress/ListStreams", service.ListStreams)
	host.RegisterMethod("VideoIngress/GetChannelURL", service.GetChannelURL)
}

// VideoIngressService ...
type VideoIngressService interface {
	IsSupported(
		ctx context.Context,
		req *pb.VideoIngressIsSupportedRequest,
	) (*pb.VideoIngressIsSupportedResponse, error)
	GetConfig(
		ctx context.Context,
		req *pb.VideoIngressGetConfigRequest,
	) (*pb.VideoIngressGetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *pb.VideoIngressSetConfigRequest,
	) (*pb.VideoIngressSetConfigResponse, error)
	ListStreams(
		ctx context.Context,
		req *pb.VideoIngressListStreamsRequest,
	) (*pb.VideoIngressListStreamsResponse, error)
	GetChannelURL(
		ctx context.Context,
		req *pb.VideoIngressGetChannelURLRequest,
	) (*pb.VideoIngressGetChannelURLResponse, error)
}

// VideoIngressClient ...
type VideoIngressClient struct {
	client Caller
}

// NewVideoIngressClient ...
func NewVideoIngressClient(client Caller) *VideoIngressClient {
	return &VideoIngressClient{client}
}

// IsSupported ...
func (c *VideoIngressClient) IsSupported(
	ctx context.Context,
	req *pb.VideoIngressIsSupportedRequest,
	res *pb.VideoIngressIsSupportedResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngress/IsSupported", req, res)
}

// GetConfig ...
func (c *VideoIngressClient) GetConfig(
	ctx context.Context,
	req *pb.VideoIngressGetConfigRequest,
	res *pb.VideoIngressGetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngress/GetConfig", req, res)
}

// SetConfig ...
func (c *VideoIngressClient) SetConfig(
	ctx context.Context,
	req *pb.VideoIngressSetConfigRequest,
	res *pb.VideoIngressSetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngress/SetConfig", req, res)
}

// ListStreams ...
func (c *VideoIngressClient) ListStreams(
	ctx context.Context,
	req *pb.VideoIngressListStreamsRequest,
	res *pb.VideoIngressListStreamsResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngress/ListStreams", req, res)
}

// GetChannelURL ...
func (c *VideoIngressClient) GetChannelURL(
	ctx context.Context,
	req *pb.VideoIngressGetChannelURLRequest,
	res *pb.VideoIngressGetChannelURLResponse,
) error {
	return c.client.CallUnary(ctx, "VideoIngress/GetChannelURL", req, res)
}
