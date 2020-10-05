package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func RegisterFrontendRPCService(host *rpc.Host, service FrontendRPCService) {
	host.RegisterService("FrontendRPC", service)
}

type FrontendRPCService interface {
	CreateProfile(
		ctx context.Context,
		req *pb.CreateProfileRequest,
	) (*pb.CreateProfileResponse, error)
	LoadProfile(
		ctx context.Context,
		req *pb.LoadProfileRequest,
	) (*pb.LoadProfileResponse, error)
	GetProfile(
		ctx context.Context,
		req *pb.GetProfileRequest,
	) (*pb.GetProfileResponse, error)
	DeleteProfile(
		ctx context.Context,
		req *pb.DeleteProfileRequest,
	) (*pb.DeleteProfileResponse, error)
	GetProfiles(
		ctx context.Context,
		req *pb.GetProfilesRequest,
	) (*pb.GetProfilesResponse, error)
	LoadSession(
		ctx context.Context,
		req *pb.LoadSessionRequest,
	) (*pb.LoadSessionResponse, error)
	CreateNetwork(
		ctx context.Context,
		req *pb.CreateNetworkRequest,
	) (*pb.CreateNetworkResponse, error)
	DeleteNetwork(
		ctx context.Context,
		req *pb.DeleteNetworkRequest,
	) (*pb.DeleteNetworkResponse, error)
	GetNetwork(
		ctx context.Context,
		req *pb.GetNetworkRequest,
	) (*pb.GetNetworkResponse, error)
	GetNetworks(
		ctx context.Context,
		req *pb.GetNetworksRequest,
	) (*pb.GetNetworksResponse, error)
	GetNetworkMemberships(
		ctx context.Context,
		req *pb.GetNetworkMembershipsRequest,
	) (*pb.GetNetworkMembershipsResponse, error)
	DeleteNetworkMembership(
		ctx context.Context,
		req *pb.DeleteNetworkMembershipRequest,
	) (*pb.DeleteNetworkMembershipResponse, error)
	CreateBootstrapClient(
		ctx context.Context,
		req *pb.CreateBootstrapClientRequest,
	) (*pb.CreateBootstrapClientResponse, error)
	UpdateBootstrapClient(
		ctx context.Context,
		req *pb.UpdateBootstrapClientRequest,
	) (*pb.UpdateBootstrapClientResponse, error)
	DeleteBootstrapClient(
		ctx context.Context,
		req *pb.DeleteBootstrapClientRequest,
	) (*pb.DeleteBootstrapClientResponse, error)
	GetBootstrapClient(
		ctx context.Context,
		req *pb.GetBootstrapClientRequest,
	) (*pb.GetBootstrapClientResponse, error)
	GetBootstrapClients(
		ctx context.Context,
		req *pb.GetBootstrapClientsRequest,
	) (*pb.GetBootstrapClientsResponse, error)
	CreateChatServer(
		ctx context.Context,
		req *pb.CreateChatServerRequest,
	) (*pb.CreateChatServerResponse, error)
	UpdateChatServer(
		ctx context.Context,
		req *pb.UpdateChatServerRequest,
	) (*pb.UpdateChatServerResponse, error)
	DeleteChatServer(
		ctx context.Context,
		req *pb.DeleteChatServerRequest,
	) (*pb.DeleteChatServerResponse, error)
	GetChatServer(
		ctx context.Context,
		req *pb.GetChatServerRequest,
	) (*pb.GetChatServerResponse, error)
	GetChatServers(
		ctx context.Context,
		req *pb.GetChatServersRequest,
	) (*pb.GetChatServersResponse, error)
	StartVPN(
		ctx context.Context,
		req *pb.StartVPNRequest,
	) (<-chan *pb.NetworkEvent, error)
	StopVPN(
		ctx context.Context,
		req *pb.StopVPNRequest,
	) (*pb.StopVPNResponse, error)
	JoinSwarm(
		ctx context.Context,
		req *pb.JoinSwarmRequest,
	) (*pb.JoinSwarmResponse, error)
	LeaveSwarm(
		ctx context.Context,
		req *pb.LeaveSwarmRequest,
	) (*pb.LeaveSwarmResponse, error)
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
	PublishSwarm(
		ctx context.Context,
		req *pb.PublishSwarmRequest,
	) (*pb.PublishSwarmResponse, error)
	PProf(
		ctx context.Context,
		req *pb.PProfRequest,
	) (*pb.PProfResponse, error)
	OpenChatServer(
		ctx context.Context,
		req *pb.OpenChatServerRequest,
	) (<-chan *pb.ChatServerEvent, error)
	OpenChatClient(
		ctx context.Context,
		req *pb.OpenChatClientRequest,
	) (<-chan *pb.ChatClientEvent, error)
	CallChatClient(
		ctx context.Context,
		req *pb.CallChatClientRequest,
	) (*pb.CallChatClientResponse, error)
	OpenVideoClient(
		ctx context.Context,
		req *pb.VideoClientOpenRequest,
	) (<-chan *pb.VideoClientEvent, error)
	OpenVideoServer(
		ctx context.Context,
		req *pb.VideoServerOpenRequest,
	) (*pb.VideoServerOpenResponse, error)
	WriteToVideoServer(
		ctx context.Context,
		req *pb.VideoServerWriteRequest,
	) (*pb.VideoServerWriteResponse, error)
	ReadMetrics(
		ctx context.Context,
		req *pb.ReadMetricsRequest,
	) (*pb.ReadMetricsResponse, error)
	CreateNetworkInvitation(
		ctx context.Context,
		req *pb.CreateNetworkInvitationRequest,
	) (*pb.CreateNetworkInvitationResponse, error)
	CreateNetworkMembershipFromInvitation(
		ctx context.Context,
		req *pb.CreateNetworkMembershipFromInvitationRequest,
	) (*pb.CreateNetworkMembershipFromInvitationResponse, error)
	GetBootstrapPeers(
		ctx context.Context,
		req *pb.GetBootstrapPeersRequest,
	) (*pb.GetBootstrapPeersResponse, error)
	PublishNetworkToBootstrapPeer(
		ctx context.Context,
		req *pb.PublishNetworkToBootstrapPeerRequest,
	) (*pb.PublishNetworkToBootstrapPeerResponse, error)
}
