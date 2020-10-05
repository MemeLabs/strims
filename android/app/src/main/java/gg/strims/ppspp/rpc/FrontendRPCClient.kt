package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*
import java.util.concurrent.Future

class FrontendRPCClient(filepath: String) : RPCClient(filepath) {

    fun createProfile(
        arg: CreateProfileRequest = CreateProfileRequest()
    ): Future<CreateProfileResponse> =
        this.callUnary("FrontendRPC/CreateProfile", arg)

    fun loadProfile(
        arg: LoadProfileRequest = LoadProfileRequest()
    ): Future<LoadProfileResponse> =
        this.callUnary("FrontendRPC/LoadProfile", arg)

    fun getProfile(
        arg: GetProfileRequest = GetProfileRequest()
    ): Future<GetProfileResponse> =
        this.callUnary("FrontendRPC/GetProfile", arg)

    fun deleteProfile(
        arg: DeleteProfileRequest = DeleteProfileRequest()
    ): Future<DeleteProfileResponse> =
        this.callUnary("FrontendRPC/DeleteProfile", arg)

    fun getProfiles(
        arg: GetProfilesRequest = GetProfilesRequest()
    ): Future<GetProfilesResponse> =
        this.callUnary("FrontendRPC/GetProfiles", arg)

    fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest()
    ): Future<LoadSessionResponse> =
        this.callUnary("FrontendRPC/LoadSession", arg)

    fun createNetwork(
        arg: CreateNetworkRequest = CreateNetworkRequest()
    ): Future<CreateNetworkResponse> =
        this.callUnary("FrontendRPC/CreateNetwork", arg)

    fun deleteNetwork(
        arg: DeleteNetworkRequest = DeleteNetworkRequest()
    ): Future<DeleteNetworkResponse> =
        this.callUnary("FrontendRPC/DeleteNetwork", arg)

    fun getNetwork(
        arg: GetNetworkRequest = GetNetworkRequest()
    ): Future<GetNetworkResponse> =
        this.callUnary("FrontendRPC/GetNetwork", arg)

    fun getNetworks(
        arg: GetNetworksRequest = GetNetworksRequest()
    ): Future<GetNetworksResponse> =
        this.callUnary("FrontendRPC/GetNetworks", arg)

    fun getNetworkMemberships(
        arg: GetNetworkMembershipsRequest = GetNetworkMembershipsRequest()
    ): Future<GetNetworkMembershipsResponse> =
        this.callUnary("FrontendRPC/GetNetworkMemberships", arg)

    fun deleteNetworkMembership(
        arg: DeleteNetworkMembershipRequest = DeleteNetworkMembershipRequest()
    ): Future<DeleteNetworkMembershipResponse> =
        this.callUnary("FrontendRPC/DeleteNetworkMembership", arg)

    fun createBootstrapClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest()
    ): Future<CreateBootstrapClientResponse> =
        this.callUnary("FrontendRPC/CreateBootstrapClient", arg)

    fun updateBootstrapClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest()
    ): Future<UpdateBootstrapClientResponse> =
        this.callUnary("FrontendRPC/UpdateBootstrapClient", arg)

    fun deleteBootstrapClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest()
    ): Future<DeleteBootstrapClientResponse> =
        this.callUnary("FrontendRPC/DeleteBootstrapClient", arg)

    fun getBootstrapClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest()
    ): Future<GetBootstrapClientResponse> =
        this.callUnary("FrontendRPC/GetBootstrapClient", arg)

    fun getBootstrapClients(
        arg: GetBootstrapClientsRequest = GetBootstrapClientsRequest()
    ): Future<GetBootstrapClientsResponse> =
        this.callUnary("FrontendRPC/GetBootstrapClients", arg)

    fun createChatServer(
        arg: CreateChatServerRequest = CreateChatServerRequest()
    ): Future<CreateChatServerResponse> =
        this.callUnary("FrontendRPC/CreateChatServer", arg)

    fun updateChatServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest()
    ): Future<UpdateChatServerResponse> =
        this.callUnary("FrontendRPC/UpdateChatServer", arg)

    fun deleteChatServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest()
    ): Future<DeleteChatServerResponse> =
        this.callUnary("FrontendRPC/DeleteChatServer", arg)

    fun getChatServer(
        arg: GetChatServerRequest = GetChatServerRequest()
    ): Future<GetChatServerResponse> =
        this.callUnary("FrontendRPC/GetChatServer", arg)

    fun getChatServers(
        arg: GetChatServersRequest = GetChatServersRequest()
    ): Future<GetChatServersResponse> =
        this.callUnary("FrontendRPC/GetChatServers", arg)

    fun startVPN(
        arg: StartVPNRequest = StartVPNRequest()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("FrontendRPC/StartVPN", arg)

    fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest()
    ): Future<StopVPNResponse> =
        this.callUnary("FrontendRPC/StopVPN", arg)

    fun joinSwarm(
        arg: JoinSwarmRequest = JoinSwarmRequest()
    ): Future<JoinSwarmResponse> =
        this.callUnary("FrontendRPC/JoinSwarm", arg)

    fun leaveSwarm(
        arg: LeaveSwarmRequest = LeaveSwarmRequest()
    ): Future<LeaveSwarmResponse> =
        this.callUnary("FrontendRPC/LeaveSwarm", arg)

    fun startRTMPIngress(
        arg: StartRTMPIngressRequest = StartRTMPIngressRequest()
    ): Future<StartRTMPIngressResponse> =
        this.callUnary("FrontendRPC/StartRTMPIngress", arg)

    fun startHLSEgress(
        arg: StartHLSEgressRequest = StartHLSEgressRequest()
    ): Future<StartHLSEgressResponse> =
        this.callUnary("FrontendRPC/StartHLSEgress", arg)

    fun stopHLSEgress(
        arg: StopHLSEgressRequest = StopHLSEgressRequest()
    ): Future<StopHLSEgressResponse> =
        this.callUnary("FrontendRPC/StopHLSEgress", arg)

    fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest()
    ): Future<PublishSwarmResponse> =
        this.callUnary("FrontendRPC/PublishSwarm", arg)

    fun pProf(
        arg: PProfRequest = PProfRequest()
    ): Future<PProfResponse> =
        this.callUnary("FrontendRPC/PProf", arg)

    fun openChatServer(
        arg: OpenChatServerRequest = OpenChatServerRequest()
    ): RPCResponseStream<ChatServerEvent> =
        this.callStreaming("FrontendRPC/OpenChatServer", arg)

    fun openChatClient(
        arg: OpenChatClientRequest = OpenChatClientRequest()
    ): RPCResponseStream<ChatClientEvent> =
        this.callStreaming("FrontendRPC/OpenChatClient", arg)

    fun callChatClient(
        arg: CallChatClientRequest = CallChatClientRequest()
    ): Future<CallChatClientResponse> =
        this.callUnary("FrontendRPC/CallChatClient", arg)

    fun openVideoClient(
        arg: VideoClientOpenRequest = VideoClientOpenRequest()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("FrontendRPC/OpenVideoClient", arg)

    fun openVideoServer(
        arg: VideoServerOpenRequest = VideoServerOpenRequest()
    ): Future<VideoServerOpenResponse> =
        this.callUnary("FrontendRPC/OpenVideoServer", arg)

    fun writeToVideoServer(
        arg: VideoServerWriteRequest = VideoServerWriteRequest()
    ): Future<VideoServerWriteResponse> =
        this.callUnary("FrontendRPC/WriteToVideoServer", arg)

    fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest()
    ): Future<ReadMetricsResponse> =
        this.callUnary("FrontendRPC/ReadMetrics", arg)

    fun createNetworkInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest()
    ): Future<CreateNetworkInvitationResponse> =
        this.callUnary("FrontendRPC/CreateNetworkInvitation", arg)

    fun createNetworkMembershipFromInvitation(
        arg: CreateNetworkMembershipFromInvitationRequest = CreateNetworkMembershipFromInvitationRequest()
    ): Future<CreateNetworkMembershipFromInvitationResponse> =
        this.callUnary("FrontendRPC/CreateNetworkMembershipFromInvitation", arg)

    fun getBootstrapPeers(
        arg: GetBootstrapPeersRequest = GetBootstrapPeersRequest()
    ): Future<GetBootstrapPeersResponse> =
        this.callUnary("FrontendRPC/GetBootstrapPeers", arg)

    fun publishNetworkToBootstrapPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest()
    ): Future<PublishNetworkToBootstrapPeerResponse> =
        this.callUnary("FrontendRPC/PublishNetworkToBootstrapPeer", arg)

}
