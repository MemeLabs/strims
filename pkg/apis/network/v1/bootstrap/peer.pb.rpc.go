package bootstrap

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterPeerServiceService ...
func RegisterPeerServiceService(host rpc.ServiceRegistry, service PeerServiceService) {
	host.RegisterMethod("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", service.GetPublishEnabled)
	host.RegisterMethod("strims.network.v1.bootstrap.PeerService.ListNetworks", service.ListNetworks)
	host.RegisterMethod("strims.network.v1.bootstrap.PeerService.Publish", service.Publish)
}

// PeerServiceService ...
type PeerServiceService interface {
	GetPublishEnabled(
		ctx context.Context,
		req *BootstrapPeerGetPublishEnabledRequest,
	) (*BootstrapPeerGetPublishEnabledResponse, error)
	ListNetworks(
		ctx context.Context,
		req *BootstrapPeerListNetworksRequest,
	) (*BootstrapPeerListNetworksResponse, error)
	Publish(
		ctx context.Context,
		req *BootstrapPeerPublishRequest,
	) (*BootstrapPeerPublishResponse, error)
}

// PeerServiceService ...
type UnimplementedPeerServiceService struct{}

func (s *UnimplementedPeerServiceService) GetPublishEnabled(
	ctx context.Context,
	req *BootstrapPeerGetPublishEnabledRequest,
) (*BootstrapPeerGetPublishEnabledResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedPeerServiceService) ListNetworks(
	ctx context.Context,
	req *BootstrapPeerListNetworksRequest,
) (*BootstrapPeerListNetworksResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedPeerServiceService) Publish(
	ctx context.Context,
	req *BootstrapPeerPublishRequest,
) (*BootstrapPeerPublishResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ PeerServiceService = (*UnimplementedPeerServiceService)(nil)

// PeerServiceClient ...
type PeerServiceClient struct {
	client rpc.Caller
}

// NewPeerServiceClient ...
func NewPeerServiceClient(client rpc.Caller) *PeerServiceClient {
	return &PeerServiceClient{client}
}

// GetPublishEnabled ...
func (c *PeerServiceClient) GetPublishEnabled(
	ctx context.Context,
	req *BootstrapPeerGetPublishEnabledRequest,
	res *BootstrapPeerGetPublishEnabledResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.PeerService.GetPublishEnabled", req, res)
}

// ListNetworks ...
func (c *PeerServiceClient) ListNetworks(
	ctx context.Context,
	req *BootstrapPeerListNetworksRequest,
	res *BootstrapPeerListNetworksResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.PeerService.ListNetworks", req, res)
}

// Publish ...
func (c *PeerServiceClient) Publish(
	ctx context.Context,
	req *BootstrapPeerPublishRequest,
	res *BootstrapPeerPublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.PeerService.Publish", req, res)
}
