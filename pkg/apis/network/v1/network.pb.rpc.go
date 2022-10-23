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
	host.RegisterMethod("strims.network.v1.NetworkFrontend.ListPeers", service.ListPeers)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.GrantPeerInvitation", service.GrantPeerInvitation)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.TogglePeerBan", service.TogglePeerBan)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.ResetPeerRenameCooldown", service.ResetPeerRenameCooldown)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.DeletePeer", service.DeletePeer)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.ListAliasReservations", service.ListAliasReservations)
	host.RegisterMethod("strims.network.v1.NetworkFrontend.ResetAliasReservationCooldown", service.ResetAliasReservationCooldown)
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
	ListPeers(
		ctx context.Context,
		req *ListPeersRequest,
	) (*ListPeersResponse, error)
	GrantPeerInvitation(
		ctx context.Context,
		req *GrantPeerInvitationRequest,
	) (*GrantPeerInvitationResponse, error)
	TogglePeerBan(
		ctx context.Context,
		req *TogglePeerBanRequest,
	) (*TogglePeerBanResponse, error)
	ResetPeerRenameCooldown(
		ctx context.Context,
		req *ResetPeerRenameCooldownRequest,
	) (*ResetPeerRenameCooldownResponse, error)
	DeletePeer(
		ctx context.Context,
		req *DeletePeerRequest,
	) (*DeletePeerResponse, error)
	ListAliasReservations(
		ctx context.Context,
		req *ListAliasReservationsRequest,
	) (*ListAliasReservationsResponse, error)
	ResetAliasReservationCooldown(
		ctx context.Context,
		req *ResetAliasReservationCooldownRequest,
	) (*ResetAliasReservationCooldownResponse, error)
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

func (s *UnimplementedNetworkFrontendService) ListPeers(
	ctx context.Context,
	req *ListPeersRequest,
) (*ListPeersResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) GrantPeerInvitation(
	ctx context.Context,
	req *GrantPeerInvitationRequest,
) (*GrantPeerInvitationResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) TogglePeerBan(
	ctx context.Context,
	req *TogglePeerBanRequest,
) (*TogglePeerBanResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) ResetPeerRenameCooldown(
	ctx context.Context,
	req *ResetPeerRenameCooldownRequest,
) (*ResetPeerRenameCooldownResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) DeletePeer(
	ctx context.Context,
	req *DeletePeerRequest,
) (*DeletePeerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) ListAliasReservations(
	ctx context.Context,
	req *ListAliasReservationsRequest,
) (*ListAliasReservationsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedNetworkFrontendService) ResetAliasReservationCooldown(
	ctx context.Context,
	req *ResetAliasReservationCooldownRequest,
) (*ResetAliasReservationCooldownResponse, error) {
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

// ListPeers ...
func (c *NetworkFrontendClient) ListPeers(
	ctx context.Context,
	req *ListPeersRequest,
	res *ListPeersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.ListPeers", req, res)
}

// GrantPeerInvitation ...
func (c *NetworkFrontendClient) GrantPeerInvitation(
	ctx context.Context,
	req *GrantPeerInvitationRequest,
	res *GrantPeerInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.GrantPeerInvitation", req, res)
}

// TogglePeerBan ...
func (c *NetworkFrontendClient) TogglePeerBan(
	ctx context.Context,
	req *TogglePeerBanRequest,
	res *TogglePeerBanResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.TogglePeerBan", req, res)
}

// ResetPeerRenameCooldown ...
func (c *NetworkFrontendClient) ResetPeerRenameCooldown(
	ctx context.Context,
	req *ResetPeerRenameCooldownRequest,
	res *ResetPeerRenameCooldownResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.ResetPeerRenameCooldown", req, res)
}

// DeletePeer ...
func (c *NetworkFrontendClient) DeletePeer(
	ctx context.Context,
	req *DeletePeerRequest,
	res *DeletePeerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.DeletePeer", req, res)
}

// ListAliasReservations ...
func (c *NetworkFrontendClient) ListAliasReservations(
	ctx context.Context,
	req *ListAliasReservationsRequest,
	res *ListAliasReservationsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.ListAliasReservations", req, res)
}

// ResetAliasReservationCooldown ...
func (c *NetworkFrontendClient) ResetAliasReservationCooldown(
	ctx context.Context,
	req *ResetAliasReservationCooldownRequest,
	res *ResetAliasReservationCooldownResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkFrontend.ResetAliasReservationCooldown", req, res)
}
