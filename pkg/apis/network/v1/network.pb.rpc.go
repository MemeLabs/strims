package network

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterNetworkServiceService ...
func RegisterNetworkServiceService(host rpc.ServiceRegistry, service NetworkServiceService) {
	host.RegisterMethod("strims.network.v1.NetworkService.CreateServer", service.CreateServer)
	host.RegisterMethod("strims.network.v1.NetworkService.UpdateServerConfig", service.UpdateServerConfig)
	host.RegisterMethod("strims.network.v1.NetworkService.Delete", service.Delete)
	host.RegisterMethod("strims.network.v1.NetworkService.Get", service.Get)
	host.RegisterMethod("strims.network.v1.NetworkService.List", service.List)
	host.RegisterMethod("strims.network.v1.NetworkService.CreateInvitation", service.CreateInvitation)
	host.RegisterMethod("strims.network.v1.NetworkService.CreateNetworkFromInvitation", service.CreateNetworkFromInvitation)
	host.RegisterMethod("strims.network.v1.NetworkService.Watch", service.Watch)
	host.RegisterMethod("strims.network.v1.NetworkService.UpdateDisplayOrder", service.UpdateDisplayOrder)
	host.RegisterMethod("strims.network.v1.NetworkService.UpdateAlias", service.UpdateAlias)
}

// NetworkServiceService ...
type NetworkServiceService interface {
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
}

// NetworkServiceClient ...
type NetworkServiceClient struct {
	client rpc.Caller
}

// NewNetworkServiceClient ...
func NewNetworkServiceClient(client rpc.Caller) *NetworkServiceClient {
	return &NetworkServiceClient{client}
}

// CreateServer ...
func (c *NetworkServiceClient) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
	res *CreateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.CreateServer", req, res)
}

// UpdateServerConfig ...
func (c *NetworkServiceClient) UpdateServerConfig(
	ctx context.Context,
	req *UpdateServerConfigRequest,
	res *UpdateServerConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.UpdateServerConfig", req, res)
}

// Delete ...
func (c *NetworkServiceClient) Delete(
	ctx context.Context,
	req *DeleteNetworkRequest,
	res *DeleteNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.Delete", req, res)
}

// Get ...
func (c *NetworkServiceClient) Get(
	ctx context.Context,
	req *GetNetworkRequest,
	res *GetNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.Get", req, res)
}

// List ...
func (c *NetworkServiceClient) List(
	ctx context.Context,
	req *ListNetworksRequest,
	res *ListNetworksResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.List", req, res)
}

// CreateInvitation ...
func (c *NetworkServiceClient) CreateInvitation(
	ctx context.Context,
	req *CreateInvitationRequest,
	res *CreateInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.CreateInvitation", req, res)
}

// CreateNetworkFromInvitation ...
func (c *NetworkServiceClient) CreateNetworkFromInvitation(
	ctx context.Context,
	req *CreateNetworkFromInvitationRequest,
	res *CreateNetworkFromInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.CreateNetworkFromInvitation", req, res)
}

// Watch ...
func (c *NetworkServiceClient) Watch(
	ctx context.Context,
	req *WatchNetworksRequest,
	res chan *WatchNetworksResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.NetworkService.Watch", req, res)
}

// UpdateDisplayOrder ...
func (c *NetworkServiceClient) UpdateDisplayOrder(
	ctx context.Context,
	req *UpdateDisplayOrderRequest,
	res *UpdateDisplayOrderResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.UpdateDisplayOrder", req, res)
}

// UpdateAlias ...
func (c *NetworkServiceClient) UpdateAlias(
	ctx context.Context,
	req *UpdateAliasRequest,
	res *UpdateAliasResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.UpdateAlias", req, res)
}
