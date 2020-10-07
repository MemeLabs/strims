package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*
import com.squareup.wire.Message

class FrontendRPCClient(filepath: String) : RPCClient(filepath) {

    suspend fun createProfile(
        arg: CreateProfileRequest = CreateProfileRequest()
    ): CreateProfileResponse =
        this.callUnary("FrontendRPC/CreateProfile", arg)

    suspend fun loadProfile(
        arg: LoadProfileRequest = LoadProfileRequest()
    ): LoadProfileResponse =
        this.callUnary("FrontendRPC/LoadProfile", arg)

    suspend fun getProfile(
        arg: GetProfileRequest = GetProfileRequest()
    ): GetProfileResponse =
        this.callUnary("FrontendRPC/GetProfile", arg)

    suspend fun deleteProfile(
        arg: DeleteProfileRequest = DeleteProfileRequest()
    ): DeleteProfileResponse =
        this.callUnary("FrontendRPC/DeleteProfile", arg)

    suspend fun getProfiles(
        arg: GetProfilesRequest = GetProfilesRequest()
    ): GetProfilesResponse =
        this.callUnary("FrontendRPC/GetProfiles", arg)

    suspend fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest()
    ): LoadSessionResponse =
        this.callUnary("FrontendRPC/LoadSession", arg)

    suspend fun createNetwork(
        arg: CreateNetworkRequest = CreateNetworkRequest()
    ): CreateNetworkResponse =
        this.callUnary("FrontendRPC/CreateNetwork", arg)

    suspend fun deleteNetwork(
        arg: DeleteNetworkRequest = DeleteNetworkRequest()
    ): DeleteNetworkResponse =
        this.callUnary("FrontendRPC/DeleteNetwork", arg)

    suspend fun getNetwork(
        arg: GetNetworkRequest = GetNetworkRequest()
    ): GetNetworkResponse =
        this.callUnary("FrontendRPC/GetNetwork", arg)

    suspend fun getNetworks(
        arg: GetNetworksRequest = GetNetworksRequest()
    ): GetNetworksResponse =
        this.callUnary("FrontendRPC/GetNetworks", arg)

    suspend fun getNetworkMemberships(
        arg: GetNetworkMembershipsRequest = GetNetworkMembershipsRequest()
    ): GetNetworkMembershipsResponse =
        this.callUnary("FrontendRPC/GetNetworkMemberships", arg)

    suspend fun deleteNetworkMembership(
        arg: DeleteNetworkMembershipRequest = DeleteNetworkMembershipRequest()
    ): DeleteNetworkMembershipResponse =
        this.callUnary("FrontendRPC/DeleteNetworkMembership", arg)

    suspend fun createBootstrapClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest()
    ): CreateBootstrapClientResponse =
        this.callUnary("FrontendRPC/CreateBootstrapClient", arg)

    suspend fun updateBootstrapClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest()
    ): UpdateBootstrapClientResponse =
        this.callUnary("FrontendRPC/UpdateBootstrapClient", arg)

    suspend fun deleteBootstrapClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest()
    ): DeleteBootstrapClientResponse =
        this.callUnary("FrontendRPC/DeleteBootstrapClient", arg)

    suspend fun getBootstrapClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest()
    ): GetBootstrapClientResponse =
        this.callUnary("FrontendRPC/GetBootstrapClient", arg)

    suspend fun getBootstrapClients(
        arg: GetBootstrapClientsRequest = GetBootstrapClientsRequest()
    ): GetBootstrapClientsResponse =
        this.callUnary("FrontendRPC/GetBootstrapClients", arg)

    suspend fun createChatServer(
        arg: CreateChatServerRequest = CreateChatServerRequest()
    ): CreateChatServerResponse =
        this.callUnary("FrontendRPC/CreateChatServer", arg)

    suspend fun updateChatServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest()
    ): UpdateChatServerResponse =
        this.callUnary("FrontendRPC/UpdateChatServer", arg)

    suspend fun deleteChatServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest()
    ): DeleteChatServerResponse =
        this.callUnary("FrontendRPC/DeleteChatServer", arg)

    suspend fun getChatServer(
        arg: GetChatServerRequest = GetChatServerRequest()
    ): GetChatServerResponse =
        this.callUnary("FrontendRPC/GetChatServer", arg)

    suspend fun getChatServers(
        arg: GetChatServersRequest = GetChatServersRequest()
    ): GetChatServersResponse =
        this.callUnary("FrontendRPC/GetChatServers", arg)

    suspend fun startVPN(
        arg: StartVPNRequest = StartVPNRequest()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("FrontendRPC/StartVPN", arg)

    suspend fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest()
    ): StopVPNResponse =
        this.callUnary("FrontendRPC/StopVPN", arg)

    suspend fun joinSwarm(
        arg: JoinSwarmRequest = JoinSwarmRequest()
    ): JoinSwarmResponse =
        this.callUnary("FrontendRPC/JoinSwarm", arg)

    suspend fun leaveSwarm(
        arg: LeaveSwarmRequest = LeaveSwarmRequest()
    ): LeaveSwarmResponse =
        this.callUnary("FrontendRPC/LeaveSwarm", arg)

    suspend fun startRTMPIngress(
        arg: StartRTMPIngressRequest = StartRTMPIngressRequest()
    ): StartRTMPIngressResponse =
        this.callUnary("FrontendRPC/StartRTMPIngress", arg)

    suspend fun startHLSEgress(
        arg: StartHLSEgressRequest = StartHLSEgressRequest()
    ): StartHLSEgressResponse =
        this.callUnary("FrontendRPC/StartHLSEgress", arg)

    suspend fun stopHLSEgress(
        arg: StopHLSEgressRequest = StopHLSEgressRequest()
    ): StopHLSEgressResponse =
        this.callUnary("FrontendRPC/StopHLSEgress", arg)

    suspend fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest()
    ): PublishSwarmResponse =
        this.callUnary("FrontendRPC/PublishSwarm", arg)

    suspend fun pProf(
        arg: PProfRequest = PProfRequest()
    ): PProfResponse =
        this.callUnary("FrontendRPC/PProf", arg)

    suspend fun openChatServer(
        arg: OpenChatServerRequest = OpenChatServerRequest()
    ): RPCResponseStream<ChatServerEvent> =
        this.callStreaming("FrontendRPC/OpenChatServer", arg)

    suspend fun openChatClient(
        arg: OpenChatClientRequest = OpenChatClientRequest()
    ): RPCResponseStream<ChatClientEvent> =
        this.callStreaming("FrontendRPC/OpenChatClient", arg)

    suspend fun callChatClient(
        arg: CallChatClientRequest = CallChatClientRequest()
    ): CallChatClientResponse =
        this.callUnary("FrontendRPC/CallChatClient", arg)

    suspend fun openVideoClient(
        arg: VideoClientOpenRequest = VideoClientOpenRequest()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("FrontendRPC/OpenVideoClient", arg)

    suspend fun openVideoServer(
        arg: VideoServerOpenRequest = VideoServerOpenRequest()
    ): VideoServerOpenResponse =
        this.callUnary("FrontendRPC/OpenVideoServer", arg)

    suspend fun writeToVideoServer(
        arg: VideoServerWriteRequest = VideoServerWriteRequest()
    ): VideoServerWriteResponse =
        this.callUnary("FrontendRPC/WriteToVideoServer", arg)

    suspend fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest()
    ): ReadMetricsResponse =
        this.callUnary("FrontendRPC/ReadMetrics", arg)

    suspend fun createNetworkInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest()
    ): CreateNetworkInvitationResponse =
        this.callUnary("FrontendRPC/CreateNetworkInvitation", arg)

    suspend fun createNetworkMembershipFromInvitation(
        arg: CreateNetworkMembershipFromInvitationRequest = CreateNetworkMembershipFromInvitationRequest()
    ): CreateNetworkMembershipFromInvitationResponse =
        this.callUnary("FrontendRPC/CreateNetworkMembershipFromInvitation", arg)

    suspend fun getBootstrapPeers(
        arg: GetBootstrapPeersRequest = GetBootstrapPeersRequest()
    ): GetBootstrapPeersResponse =
        this.callUnary("FrontendRPC/GetBootstrapPeers", arg)

    suspend fun publishNetworkToBootstrapPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest()
    ): PublishNetworkToBootstrapPeerResponse =
        this.callUnary("FrontendRPC/PublishNetworkToBootstrapPeer", arg)

}
