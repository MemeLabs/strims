package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class TransferPeerClient(filepath: String) : RPCClient(filepath) {

    suspend fun announceSwarm(
        arg: TransferPeerAnnounceSwarmRequest = TransferPeerAnnounceSwarmRequest()
    ): TransferPeerAnnounceSwarmResponse =
        this.callUnary("TransferPeer/AnnounceSwarm", arg)

}
