package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

type FrontendRPCClient struct {
	client *rpc.Client
}

// New ...
func NewFrontendRPCClient(client *rpc.Client) *FrontendRPCClient {
	return &FrontendRPCClient{client}
}

// CreateProfile ...
func (c *FrontendRPCClient) CreateProfile(
	ctx context.Context,
	req *pb.CreateProfileRequest,
	res *pb.CreateProfileResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CreateProfile", req, res)
}

// LoadProfile ...
func (c *FrontendRPCClient) LoadProfile(
	ctx context.Context,
	req *pb.LoadProfileRequest,
	res *pb.LoadProfileResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/LoadProfile", req, res)
}

// GetProfile ...
func (c *FrontendRPCClient) GetProfile(
	ctx context.Context,
	req *pb.GetProfileRequest,
	res *pb.GetProfileResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetProfile", req, res)
}

// DeleteProfile ...
func (c *FrontendRPCClient) DeleteProfile(
	ctx context.Context,
	req *pb.DeleteProfileRequest,
	res *pb.DeleteProfileResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/DeleteProfile", req, res)
}

// GetProfiles ...
func (c *FrontendRPCClient) GetProfiles(
	ctx context.Context,
	req *pb.GetProfilesRequest,
	res *pb.GetProfilesResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetProfiles", req, res)
}

// LoadSession ...
func (c *FrontendRPCClient) LoadSession(
	ctx context.Context,
	req *pb.LoadSessionRequest,
	res *pb.LoadSessionResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/LoadSession", req, res)
}

// CreateNetwork ...
func (c *FrontendRPCClient) CreateNetwork(
	ctx context.Context,
	req *pb.CreateNetworkRequest,
	res *pb.CreateNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CreateNetwork", req, res)
}

// DeleteNetwork ...
func (c *FrontendRPCClient) DeleteNetwork(
	ctx context.Context,
	req *pb.DeleteNetworkRequest,
	res *pb.DeleteNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/DeleteNetwork", req, res)
}

// GetNetwork ...
func (c *FrontendRPCClient) GetNetwork(
	ctx context.Context,
	req *pb.GetNetworkRequest,
	res *pb.GetNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetNetwork", req, res)
}

// GetNetworks ...
func (c *FrontendRPCClient) GetNetworks(
	ctx context.Context,
	req *pb.GetNetworksRequest,
	res *pb.GetNetworksResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetNetworks", req, res)
}

// GetNetworkMemberships ...
func (c *FrontendRPCClient) GetNetworkMemberships(
	ctx context.Context,
	req *pb.GetNetworkMembershipsRequest,
	res *pb.GetNetworkMembershipsResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetNetworkMemberships", req, res)
}

// DeleteNetworkMembership ...
func (c *FrontendRPCClient) DeleteNetworkMembership(
	ctx context.Context,
	req *pb.DeleteNetworkMembershipRequest,
	res *pb.DeleteNetworkMembershipResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/DeleteNetworkMembership", req, res)
}

// CreateBootstrapClient ...
func (c *FrontendRPCClient) CreateBootstrapClient(
	ctx context.Context,
	req *pb.CreateBootstrapClientRequest,
	res *pb.CreateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CreateBootstrapClient", req, res)
}

// UpdateBootstrapClient ...
func (c *FrontendRPCClient) UpdateBootstrapClient(
	ctx context.Context,
	req *pb.UpdateBootstrapClientRequest,
	res *pb.UpdateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/UpdateBootstrapClient", req, res)
}

// DeleteBootstrapClient ...
func (c *FrontendRPCClient) DeleteBootstrapClient(
	ctx context.Context,
	req *pb.DeleteBootstrapClientRequest,
	res *pb.DeleteBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/DeleteBootstrapClient", req, res)
}

// GetBootstrapClient ...
func (c *FrontendRPCClient) GetBootstrapClient(
	ctx context.Context,
	req *pb.GetBootstrapClientRequest,
	res *pb.GetBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetBootstrapClient", req, res)
}

// GetBootstrapClients ...
func (c *FrontendRPCClient) GetBootstrapClients(
	ctx context.Context,
	req *pb.GetBootstrapClientsRequest,
	res *pb.GetBootstrapClientsResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetBootstrapClients", req, res)
}

// CreateChatServer ...
func (c *FrontendRPCClient) CreateChatServer(
	ctx context.Context,
	req *pb.CreateChatServerRequest,
	res *pb.CreateChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CreateChatServer", req, res)
}

// UpdateChatServer ...
func (c *FrontendRPCClient) UpdateChatServer(
	ctx context.Context,
	req *pb.UpdateChatServerRequest,
	res *pb.UpdateChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/UpdateChatServer", req, res)
}

// DeleteChatServer ...
func (c *FrontendRPCClient) DeleteChatServer(
	ctx context.Context,
	req *pb.DeleteChatServerRequest,
	res *pb.DeleteChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/DeleteChatServer", req, res)
}

// GetChatServer ...
func (c *FrontendRPCClient) GetChatServer(
	ctx context.Context,
	req *pb.GetChatServerRequest,
	res *pb.GetChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetChatServer", req, res)
}

// GetChatServers ...
func (c *FrontendRPCClient) GetChatServers(
	ctx context.Context,
	req *pb.GetChatServersRequest,
	res *pb.GetChatServersResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetChatServers", req, res)
}

// StartVPN ...
func (c *FrontendRPCClient) StartVPN(
	ctx context.Context,
	req *pb.StartVPNRequest,
	res chan *pb.NetworkEvent,
) error {
	return c.client.CallStreaming(ctx, "FrontendRPC/StartVPN", req, res)
}

// StopVPN ...
func (c *FrontendRPCClient) StopVPN(
	ctx context.Context,
	req *pb.StopVPNRequest,
	res *pb.StopVPNResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/StopVPN", req, res)
}

// JoinSwarm ...
func (c *FrontendRPCClient) JoinSwarm(
	ctx context.Context,
	req *pb.JoinSwarmRequest,
	res *pb.JoinSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/JoinSwarm", req, res)
}

// LeaveSwarm ...
func (c *FrontendRPCClient) LeaveSwarm(
	ctx context.Context,
	req *pb.LeaveSwarmRequest,
	res *pb.LeaveSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/LeaveSwarm", req, res)
}

// StartRTMPIngress ...
func (c *FrontendRPCClient) StartRTMPIngress(
	ctx context.Context,
	req *pb.StartRTMPIngressRequest,
	res *pb.StartRTMPIngressResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/StartRTMPIngress", req, res)
}

// StartHLSEgress ...
func (c *FrontendRPCClient) StartHLSEgress(
	ctx context.Context,
	req *pb.StartHLSEgressRequest,
	res *pb.StartHLSEgressResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/StartHLSEgress", req, res)
}

// StopHLSEgress ...
func (c *FrontendRPCClient) StopHLSEgress(
	ctx context.Context,
	req *pb.StopHLSEgressRequest,
	res *pb.StopHLSEgressResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/StopHLSEgress", req, res)
}

// PublishSwarm ...
func (c *FrontendRPCClient) PublishSwarm(
	ctx context.Context,
	req *pb.PublishSwarmRequest,
	res *pb.PublishSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/PublishSwarm", req, res)
}

// PProf ...
func (c *FrontendRPCClient) PProf(
	ctx context.Context,
	req *pb.PProfRequest,
	res *pb.PProfResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/PProf", req, res)
}

// OpenChatServer ...
func (c *FrontendRPCClient) OpenChatServer(
	ctx context.Context,
	req *pb.OpenChatServerRequest,
	res chan *pb.ChatServerEvent,
) error {
	return c.client.CallStreaming(ctx, "FrontendRPC/OpenChatServer", req, res)
}

// OpenChatClient ...
func (c *FrontendRPCClient) OpenChatClient(
	ctx context.Context,
	req *pb.OpenChatClientRequest,
	res chan *pb.ChatClientEvent,
) error {
	return c.client.CallStreaming(ctx, "FrontendRPC/OpenChatClient", req, res)
}

// CallChatClient ...
func (c *FrontendRPCClient) CallChatClient(
	ctx context.Context,
	req *pb.CallChatClientRequest,
	res *pb.CallChatClientResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CallChatClient", req, res)
}

// OpenVideoClient ...
func (c *FrontendRPCClient) OpenVideoClient(
	ctx context.Context,
	req *pb.VideoClientOpenRequest,
	res chan *pb.VideoClientEvent,
) error {
	return c.client.CallStreaming(ctx, "FrontendRPC/OpenVideoClient", req, res)
}

// OpenVideoServer ...
func (c *FrontendRPCClient) OpenVideoServer(
	ctx context.Context,
	req *pb.VideoServerOpenRequest,
	res *pb.VideoServerOpenResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/OpenVideoServer", req, res)
}

// WriteToVideoServer ...
func (c *FrontendRPCClient) WriteToVideoServer(
	ctx context.Context,
	req *pb.VideoServerWriteRequest,
	res *pb.VideoServerWriteResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/WriteToVideoServer", req, res)
}

// ReadMetrics ...
func (c *FrontendRPCClient) ReadMetrics(
	ctx context.Context,
	req *pb.ReadMetricsRequest,
	res *pb.ReadMetricsResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/ReadMetrics", req, res)
}

// CreateNetworkInvitation ...
func (c *FrontendRPCClient) CreateNetworkInvitation(
	ctx context.Context,
	req *pb.CreateNetworkInvitationRequest,
	res *pb.CreateNetworkInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CreateNetworkInvitation", req, res)
}

// CreateNetworkMembershipFromInvitation ...
func (c *FrontendRPCClient) CreateNetworkMembershipFromInvitation(
	ctx context.Context,
	req *pb.CreateNetworkMembershipFromInvitationRequest,
	res *pb.CreateNetworkMembershipFromInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/CreateNetworkMembershipFromInvitation", req, res)
}

// GetBootstrapPeers ...
func (c *FrontendRPCClient) GetBootstrapPeers(
	ctx context.Context,
	req *pb.GetBootstrapPeersRequest,
	res *pb.GetBootstrapPeersResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/GetBootstrapPeers", req, res)
}

// PublishNetworkToBootstrapPeer ...
func (c *FrontendRPCClient) PublishNetworkToBootstrapPeer(
	ctx context.Context,
	req *pb.PublishNetworkToBootstrapPeerRequest,
	res *pb.PublishNetworkToBootstrapPeerResponse,
) error {
	return c.client.CallUnary(ctx, "FrontendRPC/PublishNetworkToBootstrapPeer", req, res)
}
