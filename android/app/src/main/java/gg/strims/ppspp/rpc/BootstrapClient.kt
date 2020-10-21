package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class BootstrapClient(filepath: String) : RPCClient(filepath) {

    suspend fun createClient(
        arg: CreateBootstrapClientRequest = CreateBootstrapClientRequest()
    ): CreateBootstrapClientResponse =
        this.callUnary("Bootstrap/CreateClient", arg)

    suspend fun updateClient(
        arg: UpdateBootstrapClientRequest = UpdateBootstrapClientRequest()
    ): UpdateBootstrapClientResponse =
        this.callUnary("Bootstrap/UpdateClient", arg)

    suspend fun deleteClient(
        arg: DeleteBootstrapClientRequest = DeleteBootstrapClientRequest()
    ): DeleteBootstrapClientResponse =
        this.callUnary("Bootstrap/DeleteClient", arg)

    suspend fun getClient(
        arg: GetBootstrapClientRequest = GetBootstrapClientRequest()
    ): GetBootstrapClientResponse =
        this.callUnary("Bootstrap/GetClient", arg)

    suspend fun listClients(
        arg: ListBootstrapClientsRequest = ListBootstrapClientsRequest()
    ): ListBootstrapClientsResponse =
        this.callUnary("Bootstrap/ListClients", arg)

    suspend fun listPeers(
        arg: ListBootstrapPeersRequest = ListBootstrapPeersRequest()
    ): ListBootstrapPeersResponse =
        this.callUnary("Bootstrap/ListPeers", arg)

    suspend fun publishNetworkToPeer(
        arg: PublishNetworkToBootstrapPeerRequest = PublishNetworkToBootstrapPeerRequest()
    ): PublishNetworkToBootstrapPeerResponse =
        this.callUnary("Bootstrap/PublishNetworkToPeer", arg)

}
