package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class ChatClient(filepath: String) : RPCClient(filepath) {

    suspend fun createServer(
        arg: CreateChatServerRequest = CreateChatServerRequest()
    ): CreateChatServerResponse =
        this.callUnary("Chat/CreateServer", arg)

    suspend fun updateServer(
        arg: UpdateChatServerRequest = UpdateChatServerRequest()
    ): UpdateChatServerResponse =
        this.callUnary("Chat/UpdateServer", arg)

    suspend fun deleteServer(
        arg: DeleteChatServerRequest = DeleteChatServerRequest()
    ): DeleteChatServerResponse =
        this.callUnary("Chat/DeleteServer", arg)

    suspend fun getServer(
        arg: GetChatServerRequest = GetChatServerRequest()
    ): GetChatServerResponse =
        this.callUnary("Chat/GetServer", arg)

    suspend fun listServers(
        arg: ListChatServersRequest = ListChatServersRequest()
    ): ListChatServersResponse =
        this.callUnary("Chat/ListServers", arg)

    suspend fun openServer(
        arg: OpenChatServerRequest = OpenChatServerRequest()
    ): RPCResponseStream<ChatServerEvent> =
        this.callStreaming("Chat/OpenServer", arg)

    suspend fun openClient(
        arg: OpenChatClientRequest = OpenChatClientRequest()
    ): RPCResponseStream<ChatClientEvent> =
        this.callStreaming("Chat/OpenClient", arg)

    suspend fun callClient(
        arg: CallChatClientRequest = CallChatClientRequest()
    ): CallChatClientResponse =
        this.callUnary("Chat/CallClient", arg)

}
