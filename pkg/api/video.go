package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterVideoService ...
func RegisterVideoService(host ServiceRegistry, service VideoService) {
	host.RegisterMethod("Video/OpenClient", service.OpenClient)
	host.RegisterMethod("Video/OpenServer", service.OpenServer)
	host.RegisterMethod("Video/WriteToServer", service.WriteToServer)
	host.RegisterMethod("Video/PublishSwarm", service.PublishSwarm)
	host.RegisterMethod("Video/StartRTMPIngress", service.StartRTMPIngress)
	host.RegisterMethod("Video/StartHLSEgress", service.StartHLSEgress)
	host.RegisterMethod("Video/StopHLSEgress", service.StopHLSEgress)
}

// VideoService ...
type VideoService interface {
	OpenClient(
		ctx context.Context,
		req *pb.OpenVideoClientRequest,
	) (<-chan *pb.VideoClientEvent, error)
	OpenServer(
		ctx context.Context,
		req *pb.OpenVideoServerRequest,
	) (*pb.VideoServerOpenResponse, error)
	WriteToServer(
		ctx context.Context,
		req *pb.WriteToVideoServerRequest,
	) (*pb.WriteToVideoServerResponse, error)
	PublishSwarm(
		ctx context.Context,
		req *pb.PublishSwarmRequest,
	) (*pb.PublishSwarmResponse, error)
	StartRTMPIngress(
		ctx context.Context,
		req *pb.StartRTMPIngressRequest,
	) (*pb.StartRTMPIngressResponse, error)
	StartHLSEgress(
		ctx context.Context,
		req *pb.StartHLSEgressRequest,
	) (*pb.StartHLSEgressResponse, error)
	StopHLSEgress(
		ctx context.Context,
		req *pb.StopHLSEgressRequest,
	) (*pb.StopHLSEgressResponse, error)
}

// VideoClient ...
type VideoClient struct {
	client Caller
}

// NewVideoClient ...
func NewVideoClient(client Caller) *VideoClient {
	return &VideoClient{client}
}

// OpenClient ...
func (c *VideoClient) OpenClient(
	ctx context.Context,
	req *pb.OpenVideoClientRequest,
	res chan *pb.VideoClientEvent,
) error {
	return c.client.CallStreaming(ctx, "Video/OpenClient", req, res)
}

// OpenServer ...
func (c *VideoClient) OpenServer(
	ctx context.Context,
	req *pb.OpenVideoServerRequest,
	res *pb.VideoServerOpenResponse,
) error {
	return c.client.CallUnary(ctx, "Video/OpenServer", req, res)
}

// WriteToServer ...
func (c *VideoClient) WriteToServer(
	ctx context.Context,
	req *pb.WriteToVideoServerRequest,
	res *pb.WriteToVideoServerResponse,
) error {
	return c.client.CallUnary(ctx, "Video/WriteToServer", req, res)
}

// PublishSwarm ...
func (c *VideoClient) PublishSwarm(
	ctx context.Context,
	req *pb.PublishSwarmRequest,
	res *pb.PublishSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "Video/PublishSwarm", req, res)
}

// StartRTMPIngress ...
func (c *VideoClient) StartRTMPIngress(
	ctx context.Context,
	req *pb.StartRTMPIngressRequest,
	res *pb.StartRTMPIngressResponse,
) error {
	return c.client.CallUnary(ctx, "Video/StartRTMPIngress", req, res)
}

// StartHLSEgress ...
func (c *VideoClient) StartHLSEgress(
	ctx context.Context,
	req *pb.StartHLSEgressRequest,
	res *pb.StartHLSEgressResponse,
) error {
	return c.client.CallUnary(ctx, "Video/StartHLSEgress", req, res)
}

// StopHLSEgress ...
func (c *VideoClient) StopHLSEgress(
	ctx context.Context,
	req *pb.StopHLSEgressRequest,
	res *pb.StopHLSEgressResponse,
) error {
	return c.client.CallUnary(ctx, "Video/StopHLSEgress", req, res)
}
