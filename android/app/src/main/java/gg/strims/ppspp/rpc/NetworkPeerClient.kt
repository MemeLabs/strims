package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class NetworkPeerClient(filepath: String) : RPCClient(filepath) {

    suspend fun negotiate(
        arg: NetworkPeerNegotiateRequest = NetworkPeerNegotiateRequest()
    ): NetworkPeerNegotiateResponse =
        this.callUnary("NetworkPeer/Negotiate", arg)

    suspend fun open(
        arg: NetworkPeerOpenRequest = NetworkPeerOpenRequest()
    ): NetworkPeerOpenResponse =
        this.callUnary("NetworkPeer/Open", arg)

    suspend fun close(
        arg: NetworkPeerCloseRequest = NetworkPeerCloseRequest()
    ): NetworkPeerCloseResponse =
        this.callUnary("NetworkPeer/Close", arg)

    suspend fun updateCertificate(
        arg: NetworkPeerUpdateCertificateRequest = NetworkPeerUpdateCertificateRequest()
    ): NetworkPeerUpdateCertificateResponse =
        this.callUnary("NetworkPeer/UpdateCertificate", arg)

}
