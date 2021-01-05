package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class DirectoryFrontendClient(filepath: String) : RPCClient(filepath) {

    suspend fun open(
        arg: DirectoryFrontendOpenRequest = DirectoryFrontendOpenRequest()
    ): RPCResponseStream<DirectoryFrontendOpenResponse> =
        this.callStreaming("DirectoryFrontend/Open", arg)

    suspend fun test(
        arg: DirectoryFrontendTestRequest = DirectoryFrontendTestRequest()
    ): DirectoryFrontendTestResponse =
        this.callUnary("DirectoryFrontend/Test", arg)

}
