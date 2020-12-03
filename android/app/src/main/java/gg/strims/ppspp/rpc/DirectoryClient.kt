package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class DirectoryClient(filepath: String) : RPCClient(filepath) {

    suspend fun publish(
        arg: DirectoryPublishRequest = DirectoryPublishRequest()
    ): DirectoryPublishResponse =
        this.callUnary("Directory/Publish", arg)

    suspend fun unpublish(
        arg: DirectoryUnpublishRequest = DirectoryUnpublishRequest()
    ): DirectoryUnpublishResponse =
        this.callUnary("Directory/Unpublish", arg)

    suspend fun join(
        arg: DirectoryJoinRequest = DirectoryJoinRequest()
    ): DirectoryJoinResponse =
        this.callUnary("Directory/Join", arg)

    suspend fun part(
        arg: DirectoryPartRequest = DirectoryPartRequest()
    ): DirectoryPartResponse =
        this.callUnary("Directory/Part", arg)

    suspend fun ping(
        arg: DirectoryPingRequest = DirectoryPingRequest()
    ): DirectoryPingResponse =
        this.callUnary("Directory/Ping", arg)

}
