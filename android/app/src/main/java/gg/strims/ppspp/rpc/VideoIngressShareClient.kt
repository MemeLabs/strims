package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class VideoIngressShareClient(filepath: String) : RPCClient(filepath) {

    suspend fun createChannel(
        arg: VideoIngressShareCreateChannelRequest = VideoIngressShareCreateChannelRequest()
    ): VideoIngressShareCreateChannelResponse =
        this.callUnary("VideoIngressShare/CreateChannel", arg)

    suspend fun updateChannel(
        arg: VideoIngressShareUpdateChannelRequest = VideoIngressShareUpdateChannelRequest()
    ): VideoIngressShareUpdateChannelResponse =
        this.callUnary("VideoIngressShare/UpdateChannel", arg)

    suspend fun deleteChannel(
        arg: VideoIngressShareDeleteChannelRequest = VideoIngressShareDeleteChannelRequest()
    ): VideoIngressShareDeleteChannelResponse =
        this.callUnary("VideoIngressShare/DeleteChannel", arg)

}
