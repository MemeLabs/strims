package bootstrap

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterBootstrapFrontendService ...
func RegisterBootstrapFrontendService(host rpc.ServiceRegistry, service BootstrapFrontendService) {
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.GetConfig", service.GetConfig)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.SetConfig", service.SetConfig)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", service.CreateClient)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", service.UpdateClient)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", service.DeleteClient)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", service.GetClient)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", service.ListClients)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", service.ListPeers)
	host.RegisterMethod("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", service.PublishNetworkToPeer)
}

// BootstrapFrontendService ...
type BootstrapFrontendService interface {
	GetConfig(
		ctx context.Context,
		req *GetConfigRequest,
	) (*GetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *SetConfigRequest,
	) (*SetConfigResponse, error)
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

// BootstrapFrontendService ...
type UnimplementedBootstrapFrontendService struct{}

func (s *UnimplementedBootstrapFrontendService) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
) (*GetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
) (*SetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) CreateClient(
	ctx context.Context,
	req *CreateBootstrapClientRequest,
) (*CreateBootstrapClientResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) UpdateClient(
	ctx context.Context,
	req *UpdateBootstrapClientRequest,
) (*UpdateBootstrapClientResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) DeleteClient(
	ctx context.Context,
	req *DeleteBootstrapClientRequest,
) (*DeleteBootstrapClientResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) GetClient(
	ctx context.Context,
	req *GetBootstrapClientRequest,
) (*GetBootstrapClientResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) ListClients(
	ctx context.Context,
	req *ListBootstrapClientsRequest,
) (*ListBootstrapClientsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) ListPeers(
	ctx context.Context,
	req *ListBootstrapPeersRequest,
) (*ListBootstrapPeersResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedBootstrapFrontendService) PublishNetworkToPeer(
	ctx context.Context,
	req *PublishNetworkToBootstrapPeerRequest,
) (*PublishNetworkToBootstrapPeerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ BootstrapFrontendService = (*UnimplementedBootstrapFrontendService)(nil)

// BootstrapFrontendClient ...
type BootstrapFrontendClient struct {
	client rpc.Caller
}

// NewBootstrapFrontendClient ...
func NewBootstrapFrontendClient(client rpc.Caller) *BootstrapFrontendClient {
	return &BootstrapFrontendClient{client}
}

// GetConfig ...
func (c *BootstrapFrontendClient) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
	res *GetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.GetConfig", req, res)
}

// SetConfig ...
func (c *BootstrapFrontendClient) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
	res *SetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.SetConfig", req, res)
}

// CreateClient ...
func (c *BootstrapFrontendClient) CreateClient(
	ctx context.Context,
	req *CreateBootstrapClientRequest,
	res *CreateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", req, res)
}

// UpdateClient ...
func (c *BootstrapFrontendClient) UpdateClient(
	ctx context.Context,
	req *UpdateBootstrapClientRequest,
	res *UpdateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", req, res)
}

// DeleteClient ...
func (c *BootstrapFrontendClient) DeleteClient(
	ctx context.Context,
	req *DeleteBootstrapClientRequest,
	res *DeleteBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", req, res)
}

// GetClient ...
func (c *BootstrapFrontendClient) GetClient(
	ctx context.Context,
	req *GetBootstrapClientRequest,
	res *GetBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.GetClient", req, res)
}

// ListClients ...
func (c *BootstrapFrontendClient) ListClients(
	ctx context.Context,
	req *ListBootstrapClientsRequest,
	res *ListBootstrapClientsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.ListClients", req, res)
}

// ListPeers ...
func (c *BootstrapFrontendClient) ListPeers(
	ctx context.Context,
	req *ListBootstrapPeersRequest,
	res *ListBootstrapPeersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", req, res)
}

// PublishNetworkToPeer ...
func (c *BootstrapFrontendClient) PublishNetworkToPeer(
	ctx context.Context,
	req *PublishNetworkToBootstrapPeerRequest,
	res *PublishNetworkToBootstrapPeerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", req, res)
}
