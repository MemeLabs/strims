package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class SwarmPeerClient(filepath: String) : RPCClient(filepath) {

    suspend fun announceSwarm(
        arg: SwarmPeerAnnounceSwarmRequest = SwarmPeerAnnounceSwarmRequest()
    ): SwarmPeerAnnounceSwarmResponse =
        this.callUnary("SwarmPeer/AnnounceSwarm", arg)

}
