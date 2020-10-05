package gg.strims.ppspp.rpc

import gg.strims.ppspp.proto.*
import java.util.concurrent.Future

class FrontendRPCClient(filepath: String) : RPCClient(filepath) {

    fun createProfile(
        arg: CreateProfileRequest = CreateProfileRequest()
    ): Future<CreateProfileResponse> =
        this.callUnary("createProfile", arg)

    fun loadProfile(
        arg: LoadProfileRequest = LoadProfileRequest()
    ): Future<LoadProfileResponse> =
        this.callUnary("loadProfile", arg)

    fun getProfile(
        arg: GetProfileRequest = GetProfileRequest()
    ): Future<GetProfileResponse> =
        this.callUnary("getProfile", arg)

    fun updateProfile(
        arg: UpdateProfileRequest = UpdateProfileRequest()
    ): Future<UpdateProfileResponse> =
        this.callUnary("updateProfile", arg)

    fun deleteProfile(
        arg: DeleteProfileRequest = DeleteProfileRequest()
    ): Future<DeleteProfileResponse> =
        this.callUnary("deleteProfile", arg)

    fun getProfiles(
        arg: GetProfilesRequest = GetProfilesRequest()
    ): Future<GetProfilesResponse> =
        this.callUnary("getProfiles", arg)

    fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest()
    ): Future<LoadSessionResponse> =
        this.callUnary("loadSession", arg)

    fun createNetwork(
        arg: CreateNetworkRequest = CreateNetworkRequest()
    ): Future<CreateNetworkResponse> =
        this.callUnary("createNetwork", arg)

    fun updateNetwork(
        arg: UpdateNetworkRequest = UpdateNetworkRequest()
    ): Future<UpdateNetworkResponse> =
        this.callUnary("updateNetwork", arg)

    fun deleteNetwork(
        arg: DeleteNetworkRequest = DeleteNetworkRequest()
    ): Future<DeleteNetworkResponse> =
        this.callUnary("deleteNetwork", arg)

    fun getNetwork(
        arg: GetNetworkRequest = GetNetworkRequest()
    ): Future<GetNetworkResponse> =
        this.callUnary("getNetwork", arg)

    fun getNetworks(
        arg: GetNetworksRequest = GetNetworksRequest()
    ): Future<GetNetworksResponse> =
        this.callUnary("getNetworks", arg)

    fun getNetworkMemberships(
        arg: GetNetworkMembershipsRequest = GetNetworkMembershipsRequest()
    ): Future<GetNetworkMembershipsResponse> =
        this.callUnary("getNetworkMemberships", arg)

    fun deleteNetworkMembership(
        arg: DeleteNetworkMembershipRequest = DeleteNetworkMembershipRequest()
    ): Future<DeleteNetworkMembershipResponse> =
        this.callUnary("deleteNetworkMembership", arg)

    fun createBootstrapClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest()
    ): Future<CreateBootstrapClientResponse> =
        this.callUnary("createBootstrapClient", arg)

    fun updateBootstrapClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest()
    ): Future<UpdateBootstrapClientResponse> =
        this.callUnary("updateBootstrapClient", arg)

    fun deleteBootstrapClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest()
    ): Future<DeleteBootstrapClientResponse> =
        this.callUnary("deleteBootstrapClient", arg)

    fun getBootstrapClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest()
    ): Future<GetBootstrapClientResponse> =
        this.callUnary("getBootstrapClient", arg)

    fun getBootstrapClients(
        arg: GetBootstrapClientsRequest = GetBootstrapClientsRequest()
    ): Future<GetBootstrapClientsResponse> =
        this.callUnary("getBootstrapClients", arg)

    fun createChatServer(
        arg: CreateChatServerRequest = CreateChatServerRequest()
    ): Future<CreateChatServerResponse> =
        this.callUnary("createChatServer", arg)

    fun updateChatServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest()
    ): Future<UpdateChatServerResponse> =
        this.callUnary("updateChatServer", arg)

    fun deleteChatServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest()
    ): Future<DeleteChatServerResponse> =
        this.callUnary("deleteChatServer", arg)

    fun getChatServer(
        arg: GetChatServerRequest = GetChatServerRequest()
    ): Future<GetChatServerResponse> =
        this.callUnary("getChatServer", arg)

    fun getChatServers(
        arg: GetChatServersRequest = GetChatServersRequest()
    ): Future<GetChatServersResponse> =
        this.callUnary("getChatServers", arg)

    fun startVPN(
        arg: StartVPNRequest = StartVPNRequest()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("startVPN", arg)

    fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest()
    ): Future<StopVPNResponse> =
        this.callUnary("stopVPN", arg)

    fun joinSwarm(
        arg: JoinSwarmRequest = JoinSwarmRequest()
    ): Future<JoinSwarmResponse> =
        this.callUnary("joinSwarm", arg)

    fun leaveSwarm(
        arg: LeaveSwarmRequest = LeaveSwarmRequest()
    ): Future<LeaveSwarmResponse> =
        this.callUnary("leaveSwarm", arg)

    fun getIngressStreams(
        arg: GetIngressStreamsRequest = GetIngressStreamsRequest()
    ): RPCResponseStream<GetIngressStreamsResponse> =
        this.callStreaming("getIngressStreams", arg)

    fun startHLSEgress(
        arg: StartHLSEgressRequest = StartHLSEgressRequest()
    ): Future<StartHLSEgressResponse> =
        this.callUnary("startHLSEgress", arg)

    fun stopHLSEgress(
        arg: StopHLSEgressRequest = StopHLSEgressRequest()
    ): Future<StopHLSEgressResponse> =
        this.callUnary("startHLSEgress", arg)

    fun startRTMPIngress(
        arg: StartRTMPIngressRequest = StartRTMPIngressRequest()
    ): Future<StartRTMPIngressResponse> = this.callUnary("startRTMPIngress", arg)

    fun stopRTMPIngress(
        arg: StartRTMPIngressRequest = StartRTMPIngressRequest()
    ): Future<StartRTMPIngressResponse> = this.callUnary("stopRTMPIngress", arg)

    fun startSwarm(
        arg: StartSwarmRequest = StartSwarmRequest()
    ): Future<StartSwarmResponse> =
        this.callUnary("startSwarm", arg)

    fun writeToSwarm(
        arg: WriteToSwarmRequest = WriteToSwarmRequest()
    ): Future<WriteToSwarmResponse> =
        this.callUnary("writeToSwarm", arg)

    fun stopSwarm(
        arg: StopSwarmRequest = StopSwarmRequest()
    ): Future<StopSwarmResponse> =
        this.callUnary("stopSwarm", arg)

    fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest()
    ): Future<PublishSwarmResponse> =
        this.callUnary("publishSwarm", arg)

    fun pprof(arg: PProfRequest = PProfRequest()): Future<PProfResponse> =
        this.callUnary("pProf", arg)

    fun openChatServer(
        arg: OpenChatServerRequest = OpenChatServerRequest()
    ): RPCResponseStream<ChatServerEvent> =
        this.callStreaming("openChatServer", arg)

    fun openChatClient(
        arg: OpenChatClientRequest = OpenChatClientRequest()
    ): RPCResponseStream<ChatClientEvent> =
        this.callStreaming("openChatClient", arg)

    fun callChatClient(
        arg: CallChatClientRequest = CallChatClientRequest()
    ) {
        this.call("callChatClient", arg)
    }

    fun openVideoClient(
        arg: VideoClientOpenRequest = VideoClientOpenRequest()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("openVideoClient", arg)

    fun openVideoServer(
        arg: VideoServerOpenRequest = VideoServerOpenRequest()
    ): Future<VideoServerOpenResponse> =
        this.callUnary("openVideoServer", arg)

    fun writeToVideoServer(
        arg: VideoServerWriteRequest = VideoServerWriteRequest()
    ): Future<VideoServerWriteResponse> =
        this.callUnary("writeToVideoServer", arg)

    fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest()
    ): Future<ReadMetricsResponse> =
        this.callUnary("readMetrics", arg)

    fun createNetworkInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest()
    ): Future<CreateNetworkInvitationResponse> =
        this.callUnary("createNetworkInvitation", arg)

    fun createNetworkMembershipFromInvitation(
        arg: CreateNetworkMembershipFromInvitationRequest = CreateNetworkMembershipFromInvitationRequest()
    ): Future<CreateNetworkMembershipFromInvitationResponse> =
        this.callUnary("createNetworkMembershipFromInvitation", arg)

    fun getBootstrapPeers(
        arg: GetBootstrapPeersRequest = GetBootstrapPeersRequest()
    ): Future<GetBootstrapPeersResponse> =
        this.callUnary("getBootstrapPeers", arg)

    fun publishNetworkToBootstrapPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest()
    ): Future<PublishNetworkToBootstrapPeerResponse> =
        this.callUnary("publishNetworkToBootstrapPeer", arg)
}
