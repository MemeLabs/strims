package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class VideoChannelClient(filepath: String) : RPCClient(filepath) {

    suspend fun list(
        arg: VideoChannelListRequest = VideoChannelListRequest()
    ): VideoChannelListResponse =
        this.callUnary("VideoChannel/List", arg)

    suspend fun create(
        arg: VideoChannelCreateRequest = VideoChannelCreateRequest()
    ): VideoChannelCreateResponse =
        this.callUnary("VideoChannel/Create", arg)

    suspend fun update(
        arg: VideoChannelUpdateRequest = VideoChannelUpdateRequest()
    ): VideoChannelUpdateResponse =
        this.callUnary("VideoChannel/Update", arg)

    suspend fun delete(
        arg: VideoChannelDeleteRequest = VideoChannelDeleteRequest()
    ): VideoChannelDeleteResponse =
        this.callUnary("VideoChannel/Delete", arg)

}
