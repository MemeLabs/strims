package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class NetworkClient(filepath: String) : RPCClient(filepath) {

    suspend fun create(
        arg: CreateNetworkRequest = CreateNetworkRequest()
    ): CreateNetworkResponse =
        this.callUnary("Network/Create", arg)

    suspend fun update(
        arg: UpdateNetworkRequest = UpdateNetworkRequest()
    ): UpdateNetworkResponse =
        this.callUnary("Network/Update", arg)

    suspend fun delete(
        arg: DeleteNetworkRequest = DeleteNetworkRequest()
    ): DeleteNetworkResponse =
        this.callUnary("Network/Delete", arg)

    suspend fun get(
        arg: GetNetworkRequest = GetNetworkRequest()
    ): GetNetworkResponse =
        this.callUnary("Network/Get", arg)

    suspend fun list(
        arg: ListNetworksRequest = ListNetworksRequest()
    ): ListNetworksResponse =
        this.callUnary("Network/List", arg)

    suspend fun createInvitation(
        arg: CreateNetworkInvitationRequest = CreateNetworkInvitationRequest()
    ): CreateNetworkInvitationResponse =
        this.callUnary("Network/CreateInvitation", arg)

    suspend fun createFromInvitation(
        arg: CreateNetworkFromInvitationRequest = CreateNetworkFromInvitationRequest()
    ): CreateNetworkFromInvitationResponse =
        this.callUnary("Network/CreateFromInvitation", arg)

    suspend fun startVPN(
        arg: StartVPNRequest = StartVPNRequest()
    ): RPCResponseStream<NetworkEvent> =
        this.callStreaming("Network/StartVPN", arg)

    suspend fun stopVPN(
        arg: StopVPNRequest = StopVPNRequest()
    ): StopVPNResponse =
        this.callUnary("Network/StopVPN", arg)

    suspend fun getDirectoryEvents(
        arg: GetDirectoryEventsRequest = GetDirectoryEventsRequest()
    ): RPCResponseStream<DirectoryServerEvent> =
        this.callStreaming("Network/GetDirectoryEvents", arg)

    suspend fun testDirectoryPublish(
        arg: TestDirectoryPublishRequest = TestDirectoryPublishRequest()
    ): TestDirectoryPublishResponse =
        this.callUnary("Network/TestDirectoryPublish", arg)

}
