package bootstrap

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterBootstrapService ...
func RegisterBootstrapService(host api.ServiceRegistry, service BootstrapService) {
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.CreateClient", service.CreateClient)
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.UpdateClient", service.UpdateClient)
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.DeleteClient", service.DeleteClient)
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.GetClient", service.GetClient)
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.ListClients", service.ListClients)
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.ListPeers", service.ListPeers)
	host.RegisterMethod(".strims.network.v1.bootstrap.Bootstrap.PublishNetworkToPeer", service.PublishNetworkToPeer)
}

// BootstrapService ...
type BootstrapService interface {
	CreateClient(
		ctx context.Context,
		req *CreateBootstrapClientRequest,
	) (*CreateBootstrapClientResponse, error)
	UpdateClient(
		ctx context.Context,
		req *UpdateBootstrapClientRequest,
	) (*UpdateBootstrapClientResponse, error)
	DeleteClient(
		ctx context.Context,
		req *DeleteBootstrapClientRequest,
	) (*DeleteBootstrapClientResponse, error)
	GetClient(
		ctx context.Context,
		req *GetBootstrapClientRequest,
	) (*GetBootstrapClientResponse, error)
	ListClients(
		ctx context.Context,
		req *ListBootstrapClientsRequest,
	) (*ListBootstrapClientsResponse, error)
	ListPeers(
		ctx context.Context,
		req *ListBootstrapPeersRequest,
	) (*ListBootstrapPeersResponse, error)
	PublishNetworkToPeer(
		ctx context.Context,
		req *PublishNetworkToBootstrapPeerRequest,
	) (*PublishNetworkToBootstrapPeerResponse, error)
}

// BootstrapClient ...
type BootstrapClient struct {
	client api.Caller
}

// NewBootstrapClient ...
func NewBootstrapClient(client api.Caller) *BootstrapClient {
	return &BootstrapClient{client}
}

// CreateClient ...
func (c *BootstrapClient) CreateClient(
	ctx context.Context,
	req *CreateBootstrapClientRequest,
	res *CreateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.CreateClient", req, res)
}

// UpdateClient ...
func (c *BootstrapClient) UpdateClient(
	ctx context.Context,
	req *UpdateBootstrapClientRequest,
	res *UpdateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.UpdateClient", req, res)
}

// DeleteClient ...
func (c *BootstrapClient) DeleteClient(
	ctx context.Context,
	req *DeleteBootstrapClientRequest,
	res *DeleteBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.DeleteClient", req, res)
}

// GetClient ...
func (c *BootstrapClient) GetClient(
	ctx context.Context,
	req *GetBootstrapClientRequest,
	res *GetBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.GetClient", req, res)
}

// ListClients ...
func (c *BootstrapClient) ListClients(
	ctx context.Context,
	req *ListBootstrapClientsRequest,
	res *ListBootstrapClientsResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.ListClients", req, res)
}

// ListPeers ...
func (c *BootstrapClient) ListPeers(
	ctx context.Context,
	req *ListBootstrapPeersRequest,
	res *ListBootstrapPeersResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.ListPeers", req, res)
}

// PublishNetworkToPeer ...
func (c *BootstrapClient) PublishNetworkToPeer(
	ctx context.Context,
	req *PublishNetworkToBootstrapPeerRequest,
	res *PublishNetworkToBootstrapPeerResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.network.v1.bootstrap.Bootstrap.PublishNetworkToPeer", req, res)
}
