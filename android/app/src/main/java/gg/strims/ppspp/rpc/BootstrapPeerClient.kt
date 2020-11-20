package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class BootstrapPeerClient(filepath: String) : RPCClient(filepath) {

    suspend fun getPublishEnabled(
        arg: BootstrapPeerGetPublishEnabledRequest = BootstrapPeerGetPublishEnabledRequest()
    ): BootstrapPeerGetPublishEnabledResponse =
        this.callUnary("BootstrapPeer/GetPublishEnabled", arg)

    suspend fun listNetworks(
        arg: BootstrapPeerListNetworksRequest = BootstrapPeerListNetworksRequest()
    ): BootstrapPeerListNetworksResponse =
        this.callUnary("BootstrapPeer/ListNetworks", arg)

    suspend fun publish(
        arg: BootstrapPeerPublishRequest = BootstrapPeerPublishRequest()
    ): BootstrapPeerPublishResponse =
        this.callUnary("BootstrapPeer/Publish", arg)

}
