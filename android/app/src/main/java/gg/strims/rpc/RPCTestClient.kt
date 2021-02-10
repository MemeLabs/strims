import gg.strims.ppspp.proto.*


class RPCTestClient(filepath: String) : RPCClient(filepath) {

    suspend fun CallUnary(
        arg: RPCCallUnaryRequest = RPCCallUnaryRequest()
    ): RPCCallUnaryResponse =
        this.callUnary("rpc/test/test.proto/CallUnary", arg)

    suspend fun CallStream(
        arg: RPCCallStreamRequest = RPCCallStreamRequest()
    ): RPCResponseStream<RPCCallStreamResponse> =
        this.callStreaming("rpc/test/test.proto/CallStream", arg)
}

