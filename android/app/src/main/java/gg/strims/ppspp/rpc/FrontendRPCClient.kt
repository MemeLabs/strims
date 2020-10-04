package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.Api.*
import gg.strims.ppspp.proto.Chat.*
import gg.strims.ppspp.proto.Directory.*
import gg.strims.ppspp.proto.HashTable.*
import gg.strims.ppspp.proto.Nickserv.*
import gg.strims.ppspp.proto.PeerIndex.*
import gg.strims.ppspp.proto.Profile.*
import gg.strims.ppspp.proto.PubSub.*
import gg.strims.ppspp.proto.Rpc.*
import gg.strims.ppspp.proto.Video.*
import gg.strims.ppspp.proto.Vpn.*

import java.util.concurrent.Future

class FrontendRPCClient(filepath: String) : RPCClient(filepath) {

    fun createProfile(
        arg: CreateProfileRequest = CreateProfileRequest.newBuilder().build()
    ): Future<CreateProfileResponse> =
        this.callUnary("FrontendRPC/CreateProfile", arg)

    fun loadProfile(
        arg: LoadProfileRequest = LoadProfileRequest.newBuilder().build()
    ): Future<LoadProfileResponse> =
        this.callUnary("FrontendRPC/LoadProfile", arg)

    fun getProfile(
        arg: GetProfileRequest = GetProfileRequest.newBuilder().build()
    ): Future<GetProfileResponse> =
        this.callUnary("FrontendRPC/GetProfile", arg)

    fun deleteProfile(
        arg: DeleteProfileRequest = DeleteProfileRequest.newBuilder().build()
    ): Future<DeleteProfileResponse> =
        this.callUnary("FrontendRPC/DeleteProfile", arg)

    fun getProfiles(
        arg: GetProfilesRequest = GetProfilesRequest.newBuilder().build()
    ): Future<GetProfilesResponse> =
        this.callUnary("FrontendRPC/GetProfiles", arg)

    fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest.newBuilder().build()
    ): Future<LoadSessionResponse> =
        this.callUnary("FrontendRPC/LoadSession", arg)

    fun createNetwork(
        arg: CreateNetworkRequest = CreateNetworkRequest.newBuilder().build()
    ): Future<CreateNetworkResponse> =
        this.callUnary("FrontendRPC/CreateNetwork", arg)

    fun deleteNetwork(
        arg: DeleteNetworkRequest = DeleteNetworkRequest.newBuilder().build()
    ): Future<DeleteNetworkResponse> =
        this.callUnary("FrontendRPC/DeleteNetwork", arg)

    fun getNetwork(
        arg: GetNetworkRequest = GetNetworkRequest.newBuilder().build()
    ): Future<GetNetworkResponse> =
        this.callUnary("FrontendRPC/GetNetwork", arg)

    fun getNetworks(
        arg: GetNetworksRequest = GetNetworksRequest.newBuilder().build()
    ): Future<GetNetworksResponse> =
        this.callUnary("FrontendRPC/GetNetworks", arg)

    fun getNetworkMemberships(
        arg: GetNetworkMembershipsRequest = GetNetworkMembershipsRequest.newBuilder().build()
    ): Future<GetNetworkMembershipsResponse> =
        this.callUnary("FrontendRPC/GetNetworkMemberships", arg)

    fun deleteNetworkMembership(
        arg: DeleteNetworkMembershipRequest = DeleteNetworkMembershipRequest.newBuilder().build()
    ): Future<DeleteNetworkMembershipResponse> =
        this.callUnary("FrontendRPC/DeleteNetworkMembership", arg)

    fun createBootstrapClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest.newBuilder().build()
    ): Future<CreateBootstrapClientResponse> =
        this.callUnary("FrontendRPC/CreateBootstrapClient", arg)

    fun updateBootstrapClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest.newBuilder().build()
    ): Future<UpdateBootstrapClientResponse> =
        this.callUnary("FrontendRPC/UpdateBootstrapClient", arg)

    fun deleteBootstrapClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest.newBuilder().build()
    ): Future<DeleteBootstrapClientResponse> =
        this.callUnary("FrontendRPC/DeleteBootstrapClient", arg)

    fun getBootstrapClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest.newBuilder().build()
    ): Future<GetBootstrapClientResponse> =
        this.callUnary("FrontendRPC/GetBootstrapClient", arg)

    fun getBootstrapClients(
        arg: GetBootstrapClientsRequest = GetBootstrapClientsRequest.newBuilder().build()
    ): Future<GetBootstrapClientsResponse> =
        this.callUnary("FrontendRPC/GetBootstrapClients", arg)

    fun createChatServer(
        arg: CreateChatServerRequest = CreateChatServerRequest.newBuilder().build()
    ): Future<CreateChatServerResponse> =
        this.callUnary("FrontendRPC/CreateChatServer", arg)

    fun updateChatServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest.newBuilder().build()
    ): Future<UpdateChatServerResponse> =
        this.callUnary("FrontendRPC/UpdateChatServer", arg)

    fun deleteChatServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest.newBuilder().build()
    ): Future<DeleteChatServerResponse> =
        this.callUnary("FrontendRPC/DeleteChatServer", arg)

    fun getChatServer(
        arg: GetChatServerRequest = GetChatServerRequest.newBuilder().build()
    ): Future<GetChatServerResponse> =
        this.callUnary("FrontendRPC/GetChatServer", arg)

    fun getChatServers(
        arg: GetChatServersRequest = GetChatServersRequest.newBuilder().build()
    ): Future<GetChatServersResponse> =
        this.callUnary("FrontendRPC/GetChatServers", arg)

    fun startVPN(
        arg: StartVPNRequest = StartVPNRequest.newBuilder().build()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("FrontendRPC/StartVPN", arg)

    fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest.newBuilder().build()
    ): Future<StopVPNResponse> =
        this.callUnary("FrontendRPC/StopVPN", arg)

    fun joinSwarm(
        arg: JoinSwarmRequest = JoinSwarmRequest.newBuilder().build()
    ): Future<JoinSwarmResponse> =
        this.callUnary("FrontendRPC/JoinSwarm", arg)

    fun leaveSwarm(
        arg: LeaveSwarmRequest = LeaveSwarmRequest.newBuilder().build()
    ): Future<LeaveSwarmResponse> =
        this.callUnary("FrontendRPC/LeaveSwarm", arg)

    fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest.newBuilder().build()
    ): Future<PublishSwarmResponse> =
        this.callUnary("FrontendRPC/PublishSwarm", arg)

    fun pProf(
        arg: PProfRequest = PProfRequest.newBuilder().build()
    ): Future<PProfResponse> =
        this.callUnary("FrontendRPC/PProf", arg)

    fun openChatServer(
        arg: OpenChatServerRequest = OpenChatServerRequest.newBuilder().build()
    ): RPCResponseStream<ChatServerEvent> =
        this.callStreaming("FrontendRPC/OpenChatServer", arg)

    fun openChatClient(
        arg: OpenChatClientRequest = OpenChatClientRequest.newBuilder().build()
    ): RPCResponseStream<ChatClientEvent> =
        this.callStreaming("FrontendRPC/OpenChatClient", arg)

    fun callChatClient(
        arg: CallChatClientRequest = CallChatClientRequest.newBuilder().build()
    ): Future<CallChatClientResponse> =
        this.callUnary("FrontendRPC/CallChatClient", arg)

    fun openVideoClient(
        arg: VideoClientOpenRequest = VideoClientOpenRequest.newBuilder().build()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("FrontendRPC/OpenVideoClient", arg)

    fun openVideoServer(
        arg: VideoServerOpenRequest = VideoServerOpenRequest.newBuilder().build()
    ): Future<VideoServerOpenResponse> =
        this.callUnary("FrontendRPC/OpenVideoServer", arg)

    fun writeToVideoServer(
        arg: VideoServerWriteRequest = VideoServerWriteRequest.newBuilder().build()
    ): Future<VideoServerWriteResponse> =
        this.callUnary("FrontendRPC/WriteToVideoServer", arg)

    fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest.newBuilder().build()
    ): Future<ReadMetricsResponse> =
        this.callUnary("FrontendRPC/ReadMetrics", arg)

    fun createNetworkInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest.newBuilder().build()
    ): Future<CreateNetworkInvitationResponse> =
        this.callUnary("FrontendRPC/CreateNetworkInvitation", arg)

    fun createNetworkMembershipFromInvitation(
        arg: CreateNetworkMembershipFromInvitationRequest = CreateNetworkMembershipFromInvitationRequest.newBuilder().build()
    ): Future<CreateNetworkMembershipFromInvitationResponse> =
        this.callUnary("FrontendRPC/CreateNetworkMembershipFromInvitation", arg)

    fun getBootstrapPeers(
        arg: GetBootstrapPeersRequest = GetBootstrapPeersRequest.newBuilder().build()
    ): Future<GetBootstrapPeersResponse> =
        this.callUnary("FrontendRPC/GetBootstrapPeers", arg)

    fun publishNetworkToBootstrapPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest.newBuilder().build()
    ): Future<PublishNetworkToBootstrapPeerResponse> =
        this.callUnary("FrontendRPC/PublishNetworkToBootstrapPeer", arg)

}
