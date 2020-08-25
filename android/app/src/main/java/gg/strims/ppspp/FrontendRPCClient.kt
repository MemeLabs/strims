package gg.strims.ppspp

import gg.strims.ppspp.proto.Api.*
import java.util.concurrent.Future

class FrontendRPCClient : RPCClient() {

    fun createProfile(
        arg: CreateProfileRequest = CreateProfileRequest.newBuilder().build()
    ): Future<CreateProfileResponse> =
        this.callUnary("createProfile", arg)

    fun loadProfile(
        arg: LoadProfileRequest = LoadProfileRequest.newBuilder().build()
    ): Future<LoadProfileResponse> =
        this.callUnary("loadProfile", arg)

    fun getProfile(
        arg: GetProfileRequest = GetProfileRequest.newBuilder().build()
    ): Future<GetProfileResponse> =
        this.callUnary("getProfile", arg)

    fun updateProfile(
        arg: UpdateProfileRequest = UpdateProfileRequest.newBuilder().build()
    ): Future<UpdateProfileResponse> =
        this.callUnary("updateProfile", arg)

    fun deleteProfile(
        arg: DeleteProfileRequest = DeleteProfileRequest.newBuilder().build()
    ): Future<DeleteProfileResponse> =
        this.callUnary("deleteProfile", arg)

    fun getProfiles(
        arg: GetProfilesRequest = GetProfilesRequest.newBuilder().build()
    ): Future<GetProfilesResponse> =
        this.callUnary("getProfiles", arg)

    fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest.newBuilder().build()
    ): Future<LoadSessionResponse> =
        this.callUnary("loadSession", arg)

    fun createNetwork(
        arg: CreateNetworkRequest = CreateNetworkRequest.newBuilder().build()
    ): Future<CreateNetworkResponse> =
        this.callUnary("createNetwork", arg)

    fun updateNetwork(
        arg: UpdateNetworkRequest = UpdateNetworkRequest.newBuilder().build()
    ): Future<UpdateNetworkResponse> =
        this.callUnary("updateNetwork", arg)

    fun deleteNetwork(
        arg: DeleteNetworkRequest = DeleteNetworkRequest.newBuilder().build()
    ): Future<DeleteNetworkResponse> =
        this.callUnary("deleteNetwork", arg)

    fun getNetwork(
        arg: GetNetworkRequest = GetNetworkRequest.newBuilder().build()
    ): Future<GetNetworkResponse> =
        this.callUnary("getNetwork", arg)

    fun getNetworks(
        arg: GetNetworksRequest = GetNetworksRequest.newBuilder().build()
    ): Future<GetNetworksResponse> =
        this.callUnary("getNetworks", arg)

    fun getNetworkMemberships(
        arg: GetNetworkMembershipsRequest = GetNetworkMembershipsRequest.newBuilder().build()
    ): Future<GetNetworkMembershipsResponse> =
        this.callUnary("getNetworkMemberships", arg)

    fun deleteNetworkMembership(
        arg: DeleteNetworkMembershipRequest = DeleteNetworkMembershipRequest.newBuilder().build()
    ): Future<DeleteNetworkMembershipResponse> =
        this.callUnary("deleteNetworkMembership", arg)

    fun createBootstrapClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest.newBuilder().build()
    ): Future<CreateBootstrapClientResponse> =
        this.callUnary("createBootstrapClient", arg)

    fun updateBootstrapClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest.newBuilder().build()
    ): Future<UpdateBootstrapClientResponse> =
        this.callUnary("updateBootstrapClient", arg)

    fun deleteBootstrapClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest.newBuilder().build()
    ): Future<DeleteBootstrapClientResponse> =
        this.callUnary("deleteBootstrapClient", arg)

    fun getBootstrapClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest.newBuilder().build()
    ): Future<GetBootstrapClientResponse> =
        this.callUnary("getBootstrapClient", arg)

    fun getBootstrapClients(
        arg: GetBootstrapClientsRequest = GetBootstrapClientsRequest.newBuilder().build()
    ): Future<GetBootstrapClientsResponse> =
        this.callUnary("getBootstrapClients", arg)

    fun createChatServer(
        arg: CreateChatServerRequest = CreateChatServerRequest.newBuilder().build()
    ): Future<CreateChatServerResponse> =
        this.callUnary("createChatServer", arg)

    fun updateChatServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest.newBuilder().build()
    ): Future<UpdateChatServerResponse> =
        this.callUnary("updateChatServer", arg)

    fun deleteChatServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest.newBuilder().build()
    ): Future<DeleteChatServerResponse> =
        this.callUnary("deleteChatServer", arg)

    fun getChatServer(
        arg: GetChatServerRequest = GetChatServerRequest.newBuilder().build()
    ): Future<GetChatServerResponse> =
        this.callUnary("getChatServer", arg)

    fun getChatServers(
        arg: GetChatServersRequest = GetChatServersRequest.newBuilder().build()
    ): Future<GetChatServersResponse> =
        this.callUnary("getChatServers", arg)

    fun startVPN(
        arg: StartVPNRequest = StartVPNRequest.newBuilder().build()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("startVPN", arg)

    fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest.newBuilder().build()
    ): Future<StopVPNResponse> =
        this.callUnary("stopVPN", arg)

    fun joinSwarm(
        arg: JoinSwarmRequest = JoinSwarmRequest.newBuilder().build()
    ): Future<JoinSwarmResponse> =
        this.callUnary("joinSwarm", arg)

    fun leaveSwarm(
        arg: LeaveSwarmRequest = LeaveSwarmRequest.newBuilder().build()
    ): Future<LeaveSwarmResponse> =
        this.callUnary("leaveSwarm", arg)

    fun getIngressStreams(
        arg: GetIngressStreamsRequest = GetIngressStreamsRequest.newBuilder().build()
    ): RPCResponseStream<GetIngressStreamsResponse> =
        this.callStreaming("getIngressStreams", arg)

    fun startHLSIngress(
        arg: StartHLSIngressRequest = StartHLSIngressRequest.newBuilder().build()
    ): Future<StartHLSIngressResponse> =
        this.callUnary("startHLSIngress", arg)

    fun stopHLSIngress(
        arg: StartHLSIngressRequest = StartHLSIngressRequest.newBuilder().build()
    ): Future<StartHLSIngressResponse> =
        this.callUnary("stopHLSIngress", arg)

    fun startHLSEgress(
        arg: StopHLSEgressRequest = StopHLSEgressRequest.newBuilder().build()
    ): Future<StopHLSEgressResponse> =
        this.callUnary("startHLSEgress", arg)

    fun startSwarm(
        arg: StartSwarmRequest = StartSwarmRequest.newBuilder().build()
    ): Future<StartSwarmResponse> =
        this.callUnary("startSwarm", arg)

    fun writeToSwarm(
        arg: WriteToSwarmRequest = WriteToSwarmRequest.newBuilder().build()
    ): Future<WriteToSwarmResponse> =
        this.callUnary("writeToSwarm", arg)

    fun stopSwarm(
        arg: StopSwarmRequest = StopSwarmRequest.newBuilder().build()
    ): Future<StopSwarmResponse> =
        this.callUnary("stopSwarm", arg)

    fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest.newBuilder().build()
    ): Future<PublishSwarmResponse> =
        this.callUnary("publishSwarm", arg)

    fun pprof(arg: PProfRequest = PProfRequest.newBuilder().build()): Future<PProfResponse> =
        this.callUnary("pProf", arg)

    fun openChatServer(
        arg: OpenChatServerRequest = OpenChatServerRequest.newBuilder().build()
    ): RPCResponseStream<ChatServerEvent> =
        this.callStreaming("openChatServer", arg)

    fun openChatClient(
        arg: OpenChatClientRequest = OpenChatClientRequest.newBuilder().build()
    ): RPCResponseStream<ChatClientEvent> =
        this.callStreaming("openChatClient", arg)

    fun callChatClient(
        arg: CallChatClientRequest = CallChatClientRequest.newBuilder().build()
    ) {
        this.call("callChatClient", arg)
    }

    fun openVideoClient(
        arg: VideoClientOpenRequest = VideoClientOpenRequest.newBuilder().build()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("openVideoClient", arg)

    fun openVideoServer(
        arg: VideoServerOpenRequest = VideoServerOpenRequest.newBuilder().build()
    ): Future<VideoServerOpenResponse> =
        this.callUnary("openVideoServer", arg)

    fun writeToVideoServer(
        arg: VideoServerWriteRequest = VideoServerWriteRequest.newBuilder().build()
    ): Future<VideoServerWriteResponse> =
        this.callUnary("writeToVideoServer", arg)

    fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest.newBuilder().build()
    ): Future<ReadMetricsResponse> =
        this.callUnary("readMetrics", arg)

    fun createNetworkInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest.newBuilder().build()
    ): Future<CreateNetworkInvitationResponse> =
        this.callUnary("createNetworkInvitation", arg)

    fun createNetworkMembershipFromInvitation(
        arg: CreateNetworkMembershipFromInvitationRequest = CreateNetworkMembershipFromInvitationRequest.newBuilder()
            .build()
    ): Future<CreateNetworkMembershipFromInvitationResponse> =
        this.callUnary("createNetworkMembershipFromInvitation", arg)

    fun getBootstrapPeers(
        arg: GetBootstrapPeersRequest = GetBootstrapPeersRequest.newBuilder().build()
    ): Future<GetBootstrapPeersResponse> =
        this.callUnary("getBootstrapPeers", arg)

    fun publishNetworkToBootstrapPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest.newBuilder()
            .build()
    ): Future<PublishNetworkToBootstrapPeerResponse> =
        this.callUnary("publishNetworkToBootstrapPeer", arg)
}
