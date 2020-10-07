package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*
import com.squareup.wire.Message

class FrontendRPCClient(filepath: String) : RPCClient(filepath) {

    fun createProfile(
        arg: CreateProfileRequest = CreateProfileRequest()
    ): Message<CreateProfileResponse, *>? =
        this.callUnary("FrontendRPC/CreateProfile", arg)

    fun loadProfile(
        arg: LoadProfileRequest = LoadProfileRequest()
    ): Message<LoadProfileResponse, *>? =
        this.callUnary("FrontendRPC/LoadProfile", arg)

    fun getProfile(
        arg: GetProfileRequest = GetProfileRequest()
    ): Message<GetProfileResponse, *>? =
        this.callUnary("FrontendRPC/GetProfile", arg)

    fun deleteProfile(
        arg: DeleteProfileRequest = DeleteProfileRequest()
    ): Message<DeleteProfileResponse, *>? =
        this.callUnary("FrontendRPC/DeleteProfile", arg)

    fun getProfiles(
        arg: GetProfilesRequest = GetProfilesRequest()
    ): Message<GetProfilesResponse, *>? =
        this.callUnary("FrontendRPC/GetProfiles", arg)

    fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest()
    ): Message<LoadSessionResponse, *>? =
        this.callUnary("FrontendRPC/LoadSession", arg)

    fun createNetwork(
        arg: CreateNetworkRequest = CreateNetworkRequest()
    ): Message<CreateNetworkResponse, *>? =
        this.callUnary("FrontendRPC/CreateNetwork", arg)

    fun deleteNetwork(
        arg: DeleteNetworkRequest = DeleteNetworkRequest()
    ): Message<DeleteNetworkResponse, *>? =
        this.callUnary("FrontendRPC/DeleteNetwork", arg)

    fun getNetwork(
        arg: GetNetworkRequest = GetNetworkRequest()
    ): Message<GetNetworkResponse, *>? =
        this.callUnary("FrontendRPC/GetNetwork", arg)

    fun getNetworks(
        arg: GetNetworksRequest = GetNetworksRequest()
    ): Message<GetNetworksResponse, *>? =
        this.callUnary("FrontendRPC/GetNetworks", arg)

    fun getNetworkMemberships(
        arg: GetNetworkMembershipsRequest = GetNetworkMembershipsRequest()
    ): Message<GetNetworkMembershipsResponse, *>? =
        this.callUnary("FrontendRPC/GetNetworkMemberships", arg)

    fun deleteNetworkMembership(
        arg: DeleteNetworkMembershipRequest = DeleteNetworkMembershipRequest()
    ): Message<DeleteNetworkMembershipResponse, *>? =
        this.callUnary("FrontendRPC/DeleteNetworkMembership", arg)

    fun createBootstrapClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest()
    ): Message<CreateBootstrapClientResponse, *>? =
        this.callUnary("FrontendRPC/CreateBootstrapClient", arg)

    fun updateBootstrapClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest()
    ): Message<UpdateBootstrapClientResponse, *>? =
        this.callUnary("FrontendRPC/UpdateBootstrapClient", arg)

    fun deleteBootstrapClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest()
    ): Message<DeleteBootstrapClientResponse, *>? =
        this.callUnary("FrontendRPC/DeleteBootstrapClient", arg)

    fun getBootstrapClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest()
    ): Message<GetBootstrapClientResponse, *>? =
        this.callUnary("FrontendRPC/GetBootstrapClient", arg)

    fun getBootstrapClients(
        arg: GetBootstrapClientsRequest = GetBootstrapClientsRequest()
    ): Message<GetBootstrapClientsResponse, *>? =
        this.callUnary("FrontendRPC/GetBootstrapClients", arg)

    fun createChatServer(
        arg: CreateChatServerRequest = CreateChatServerRequest()
    ): Message<CreateChatServerResponse, *>? =
        this.callUnary("FrontendRPC/CreateChatServer", arg)

    fun updateChatServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest()
    ): Message<UpdateChatServerResponse, *>? =
        this.callUnary("FrontendRPC/UpdateChatServer", arg)

    fun deleteChatServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest()
    ): Message<DeleteChatServerResponse, *>? =
        this.callUnary("FrontendRPC/DeleteChatServer", arg)

    fun getChatServer(
        arg: GetChatServerRequest = GetChatServerRequest()
    ): Message<GetChatServerResponse, *>? =
        this.callUnary("FrontendRPC/GetChatServer", arg)

    fun getChatServers(
        arg: GetChatServersRequest = GetChatServersRequest()
    ): Message<GetChatServersResponse, *>? =
        this.callUnary("FrontendRPC/GetChatServers", arg)

    fun startVPN(
        arg: StartVPNRequest = StartVPNRequest()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("FrontendRPC/StartVPN", arg)

    fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest()
    ): Message<StopVPNResponse, *>? =
        this.callUnary("FrontendRPC/StopVPN", arg)

    fun joinSwarm(
        arg: JoinSwarmRequest = JoinSwarmRequest()
    ): Message<JoinSwarmResponse, *>? =
        this.callUnary("FrontendRPC/JoinSwarm", arg)

    fun leaveSwarm(
        arg: LeaveSwarmRequest = LeaveSwarmRequest()
    ): Message<LeaveSwarmResponse, *>? =
        this.callUnary("FrontendRPC/LeaveSwarm", arg)

    fun startRTMPIngress(
        arg: StartRTMPIngressRequest = StartRTMPIngressRequest()
    ): Message<StartRTMPIngressResponse, *>? =
        this.callUnary("FrontendRPC/StartRTMPIngress", arg)

    fun startHLSEgress(
        arg: StartHLSEgressRequest = StartHLSEgressRequest()
    ): Message<StartHLSEgressResponse, *>? =
        this.callUnary("FrontendRPC/StartHLSEgress", arg)

    fun stopHLSEgress(
        arg: StopHLSEgressRequest = StopHLSEgressRequest()
    ): Message<StopHLSEgressResponse, *>? =
        this.callUnary("FrontendRPC/StopHLSEgress", arg)

    fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest()
    ): Message<PublishSwarmResponse, *>? =
        this.callUnary("FrontendRPC/PublishSwarm", arg)

    fun pProf(
        arg: PProfRequest = PProfRequest()
    ): Message<PProfResponse, *>? =
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
    ): Message<CallChatClientResponse, *>? =
        this.callUnary("FrontendRPC/CallChatClient", arg)

    fun openVideoClient(
        arg: VideoClientOpenRequest = VideoClientOpenRequest()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("FrontendRPC/OpenVideoClient", arg)

    fun openVideoServer(
        arg: VideoServerOpenRequest = VideoServerOpenRequest()
    ): Message<VideoServerOpenResponse, *>? =
        this.callUnary("FrontendRPC/OpenVideoServer", arg)

    fun writeToVideoServer(
        arg: VideoServerWriteRequest = VideoServerWriteRequest()
    ): Message<VideoServerWriteResponse, *>? =
        this.callUnary("FrontendRPC/WriteToVideoServer", arg)

    fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest()
    ): Message<ReadMetricsResponse, *>? =
        this.callUnary("FrontendRPC/ReadMetrics", arg)

    fun createNetworkInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest()
    ): Message<CreateNetworkInvitationResponse, *>? =
        this.callUnary("FrontendRPC/CreateNetworkInvitation", arg)

    fun createNetworkMembershipFromInvitation(
        arg: CreateNetworkMembershipFromInvitationRequest = CreateNetworkMembershipFromInvitationRequest()
    ): Message<CreateNetworkMembershipFromInvitationResponse, *>? =
        this.callUnary("FrontendRPC/CreateNetworkMembershipFromInvitation", arg)

    fun getBootstrapPeers(
        arg: GetBootstrapPeersRequest = GetBootstrapPeersRequest()
    ): Message<GetBootstrapPeersResponse, *>? =
        this.callUnary("FrontendRPC/GetBootstrapPeers", arg)

    fun publishNetworkToBootstrapPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest()
    ): Message<PublishNetworkToBootstrapPeerResponse, *>? =
        this.callUnary("FrontendRPC/PublishNetworkToBootstrapPeer", arg)

}
