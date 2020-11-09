package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class RPCTestClient(filepath: String) : RPCClient(filepath) {

    suspend fun callUnary(
        arg: RPCCallUnaryRequest = RPCCallUnaryRequest()
    ): RPCCallUnaryResponse =
        this.callUnary("RPCTest/CallUnary", arg)

    suspend fun callStream(
        arg: RPCCallStreamRequest = RPCCallStreamRequest()
    ): RPCResponseStream<RPCCallStreamResponse> =
        this.callStreaming("RPCTest/CallStream", arg)

}
