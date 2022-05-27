package networkv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterNetworkFrontendService ...
func RegisterNetworkFrontendService(host rpc.ServiceRegistry, service NetworkFrontendService) {
	host.RegisterMethod("strims.network.v1.NetworkFrontend.CreateServer", service.CreateServer)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.UpdateServerConfig", service.UpdateServerConfig)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.Delete", service.Delete)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.Get", service.Get)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.List", service.List)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.CreateInvitation", service.CreateInvitation)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation", service.CreateNetworkFromInvitation)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.Watch", service.Watch)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.UpdateDisplayOrder", service.UpdateDisplayOrder)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.UpdateAlias", service.UpdateAlias)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.GetUIConfig", service.GetUIConfig)
}

// NetworkFrontendService ...
type NetworkFrontendService interface {
	CreateServer(
		ctx context.Context,
		req *CreateServerRequest,
	) (*CreateServerResponse, error)
	UpdateServerConfig(
		ctx context.Context,
		req *UpdateServerConfigRequest,
	) (*UpdateServerConfigResponse, error)
	Delete(
		ctx context.Context,
		req *DeleteNetworkRequest,
	) (*DeleteNetworkResponse, error)
	Get(
		ctx context.Context,
		req *GetNetworkRequest,
	) (*GetNetworkResponse, error)
	List(
		ctx context.Context,
		req *ListNetworksRequest,
	) (*ListNetworksResponse, error)
	CreateInvitation(
		ctx context.Context,
		req *CreateInvitationRequest,
	) (*CreateInvitationResponse, error)
	CreateNetworkFromInvitation(
		ctx context.Context,
		req *CreateNetworkFromInvitationRequest,
	) (*CreateNetworkFromInvitationResponse, error)
	Watch(
		ctx context.Context,
		req *WatchNetworksRequest,
	) (<-chan *WatchNetworksResponse, error)
	UpdateDisplayOrder(
		ctx context.Context,
		req *UpdateDisplayOrderRequest,
	) (*UpdateDisplayOrderResponse, error)
	UpdateAlias(
		ctx context.Context,
		req *UpdateAliasRequest,
	) (*UpdateAliasResponse, error)
	GetUIConfig(
		ctx context.Context,
		req *GetUIConfigRequest,
	) (*GetUIConfigResponse, error)
}

// NetworkFrontendService ...
type UnimplementedNetworkFrontendService struct{}

func (s *UnimplementedNetworkFrontendService) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
) (*CreateServerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) UpdateServerConfig(
	ctx context.Context,
	req *UpdateServerConfigRequest,
) (*UpdateServerConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) Delete(
	ctx context.Context,
	req *DeleteNetworkRequest,
) (*DeleteNetworkResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) Get(
	ctx context.Context,
	req *GetNetworkRequest,
) (*GetNetworkResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) List(
	ctx context.Context,
	req *ListNetworksRequest,
) (*ListNetworksResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) CreateInvitation(
	ctx context.Context,
	req *CreateInvitationRequest,
) (*CreateInvitationResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) CreateNetworkFromInvitation(
	ctx context.Context,
	req *CreateNetworkFromInvitationRequest,
) (*CreateNetworkFromInvitationResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) Watch(
	ctx context.Context,
	req *WatchNetworksRequest,
) (<-chan *WatchNetworksResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) UpdateDisplayOrder(
	ctx context.Context,
	req *UpdateDisplayOrderRequest,
) (*UpdateDisplayOrderResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) UpdateAlias(
	ctx context.Context,
	req *UpdateAliasRequest,
) (*UpdateAliasResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) GetUIConfig(
	ctx context.Context,
	req *GetUIConfigRequest,
) (*GetUIConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ NetworkFrontendService = (*UnimplementedNetworkFrontendService)(nil)

// NetworkFrontendClient ...
type NetworkFrontendClient struct {
	client rpc.Caller
}

// NewNetworkFrontendClient ...
func NewNetworkFrontendClient(client rpc.Caller) *NetworkFrontendClient {
	return &NetworkFrontendClient{client}
}

// CreateServer ...
func (c *NetworkFrontendClient) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
	res *CreateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.CreateServer", req, res)
}

// UpdateServerConfig ...
func (c *NetworkFrontendClient) UpdateServerConfig(
	ctx context.Context,
	req *UpdateServerConfigRequest,
	res *UpdateServerConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.UpdateServerConfig", req, res)
}

// Delete ...
func (c *NetworkFrontendClient) Delete(
	ctx context.Context,
	req *DeleteNetworkRequest,
	res *DeleteNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.Delete", req, res)
}

// Get ...
func (c *NetworkFrontendClient) Get(
	ctx context.Context,
	req *GetNetworkRequest,
	res *GetNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.Get", req, res)
}

// List ...
func (c *NetworkFrontendClient) List(
	ctx context.Context,
	req *ListNetworksRequest,
	res *ListNetworksResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.List", req, res)
}

// CreateInvitation ...
func (c *NetworkFrontendClient) CreateInvitation(
	ctx context.Context,
	req *CreateInvitationRequest,
	res *CreateInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.CreateInvitation", req, res)
}

// CreateNetworkFromInvitation ...
func (c *NetworkFrontendClient) CreateNetworkFromInvitation(
	ctx context.Context,
	req *CreateNetworkFromInvitationRequest,
	res *CreateNetworkFromInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation", req, res)
}

// Watch ...
func (c *NetworkFrontendClient) Watch(
	ctx context.Context,
	req *WatchNetworksRequest,
	res chan *WatchNetworksResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.NetworkFrontend.Watch", req, res)
}

// UpdateDisplayOrder ...
func (c *NetworkFrontendClient) UpdateDisplayOrder(
	ctx context.Context,
	req *UpdateDisplayOrderRequest,
	res *UpdateDisplayOrderResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.UpdateDisplayOrder", req, res)
}

// UpdateAlias ...
func (c *NetworkFrontendClient) UpdateAlias(
	ctx context.Context,
	req *UpdateAliasRequest,
	res *UpdateAliasResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.UpdateAlias", req, res)
}

// GetUIConfig ...
func (c *NetworkFrontendClient) GetUIConfig(
	ctx context.Context,
	req *GetUIConfigRequest,
	res *GetUIConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.GetUIConfig", req, res)
}
