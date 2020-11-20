package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterBootstrapPeerService ...
func RegisterBootstrapPeerService(host ServiceRegistry, service BootstrapPeerService) {
	host.RegisterMethod("BootstrapPeer/GetPublishEnabled", service.GetPublishEnabled)
	host.RegisterMethod("BootstrapPeer/ListNetworks", service.ListNetworks)
	host.RegisterMethod("BootstrapPeer/Publish", service.Publish)
}

// BootstrapPeerService ...
type BootstrapPeerService interface {
	GetPublishEnabled(
		ctx context.Context,
		req *pb.BootstrapPeerGetPublishEnabledRequest,
	) (*pb.BootstrapPeerGetPublishEnabledResponse, error)
	ListNetworks(
		ctx context.Context,
		req *pb.BootstrapPeerListNetworksRequest,
	) (*pb.BootstrapPeerListNetworksResponse, error)
	Publish(
		ctx context.Context,
		req *pb.BootstrapPeerPublishRequest,
	) (*pb.BootstrapPeerPublishResponse, error)
}

// BootstrapPeerClient ...
type BootstrapPeerClient struct {
	client Caller
}

// NewBootstrapPeerClient ...
func NewBootstrapPeerClient(client Caller) *BootstrapPeerClient {
	return &BootstrapPeerClient{client}
}

// GetPublishEnabled ...
func (c *BootstrapPeerClient) GetPublishEnabled(
	ctx context.Context,
	req *pb.BootstrapPeerGetPublishEnabledRequest,
	res *pb.BootstrapPeerGetPublishEnabledResponse,
) error {
	return c.client.CallUnary(ctx, "BootstrapPeer/GetPublishEnabled", req, res)
}

// ListNetworks ...
func (c *BootstrapPeerClient) ListNetworks(
	ctx context.Context,
	req *pb.BootstrapPeerListNetworksRequest,
	res *pb.BootstrapPeerListNetworksResponse,
) error {
	return c.client.CallUnary(ctx, "BootstrapPeer/ListNetworks", req, res)
}

// Publish ...
func (c *BootstrapPeerClient) Publish(
	ctx context.Context,
	req *pb.BootstrapPeerPublishRequest,
	res *pb.BootstrapPeerPublishResponse,
) error {
	return c.client.CallUnary(ctx, "BootstrapPeer/Publish", req, res)
}
