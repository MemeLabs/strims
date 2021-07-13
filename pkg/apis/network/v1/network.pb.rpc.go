package network

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterNetworkServiceService ...
func RegisterNetworkServiceService(host rpc.ServiceRegistry, service NetworkServiceService) {
	host.RegisterMethod("strims.network.v1.NetworkService.Create", service.Create)
	host.RegisterMethod("strims.network.v1.NetworkService.Update", service.Update)
	host.RegisterMethod("strims.network.v1.NetworkService.Delete", service.Delete)
	host.RegisterMethod("strims.network.v1.NetworkService.Get", service.Get)
	host.RegisterMethod("strims.network.v1.NetworkService.List", service.List)
	host.RegisterMethod("strims.network.v1.NetworkService.CreateInvitation", service.CreateInvitation)
	host.RegisterMethod("strims.network.v1.NetworkService.CreateFromInvitation", service.CreateFromInvitation)
	host.RegisterMethod("strims.network.v1.NetworkService.Watch", service.Watch)
	host.RegisterMethod("strims.network.v1.NetworkService.SetDisplayOrder", service.SetDisplayOrder)
}

// NetworkServiceService ...
type NetworkServiceService interface {
	Create(
		ctx context.Context,
		req *CreateNetworkRequest,
	) (*CreateNetworkResponse, error)
	Update(
		ctx context.Context,
		req *UpdateNetworkRequest,
	) (*UpdateNetworkResponse, error)
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
		req *CreateNetworkInvitationRequest,
	) (*CreateNetworkInvitationResponse, error)
	CreateFromInvitation(
		ctx context.Context,
		req *CreateNetworkFromInvitationRequest,
	) (*CreateNetworkFromInvitationResponse, error)
	Watch(
		ctx context.Context,
		req *WatchNetworksRequest,
	) (<-chan *WatchNetworksResponse, error)
	SetDisplayOrder(
		ctx context.Context,
		req *SetDisplayOrderRequest,
	) (*SetDisplayOrderResponse, error)
}

// NetworkServiceClient ...
type NetworkServiceClient struct {
	client rpc.Caller
}

// NewNetworkServiceClient ...
func NewNetworkServiceClient(client rpc.Caller) *NetworkServiceClient {
	return &NetworkServiceClient{client}
}

// Create ...
func (c *NetworkServiceClient) Create(
	ctx context.Context,
	req *CreateNetworkRequest,
	res *CreateNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.Create", req, res)
}

// Update ...
func (c *NetworkServiceClient) Update(
	ctx context.Context,
	req *UpdateNetworkRequest,
	res *UpdateNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.Update", req, res)
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
	req *CreateNetworkInvitationRequest,
	res *CreateNetworkInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.CreateInvitation", req, res)
}

// CreateFromInvitation ...
func (c *NetworkServiceClient) CreateFromInvitation(
	ctx context.Context,
	req *CreateNetworkFromInvitationRequest,
	res *CreateNetworkFromInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.CreateFromInvitation", req, res)
}

// Watch ...
func (c *NetworkServiceClient) Watch(
	ctx context.Context,
	req *WatchNetworksRequest,
	res chan *WatchNetworksResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.NetworkService.Watch", req, res)
}

// SetDisplayOrder ...
func (c *NetworkServiceClient) SetDisplayOrder(
	ctx context.Context,
	req *SetDisplayOrderRequest,
	res *SetDisplayOrderResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkService.SetDisplayOrder", req, res)
}
