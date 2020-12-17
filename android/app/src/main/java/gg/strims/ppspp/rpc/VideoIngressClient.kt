package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class VideoIngressClient(filepath: String) : RPCClient(filepath) {

    suspend fun isSupported(
        arg: VideoIngressIsSupportedRequest = VideoIngressIsSupportedRequest()
    ): VideoIngressIsSupportedResponse =
        this.callUnary("VideoIngress/IsSupported", arg)

    suspend fun getConfig(
        arg: VideoIngressGetConfigRequest = VideoIngressGetConfigRequest()
    ): VideoIngressGetConfigResponse =
        this.callUnary("VideoIngress/GetConfig", arg)

    suspend fun setConfig(
        arg: VideoIngressSetConfigRequest = VideoIngressSetConfigRequest()
    ): VideoIngressSetConfigResponse =
        this.callUnary("VideoIngress/SetConfig", arg)

    suspend fun listStreams(
        arg: VideoIngressListStreamsRequest = VideoIngressListStreamsRequest()
    ): VideoIngressListStreamsResponse =
        this.callUnary("VideoIngress/ListStreams", arg)

    suspend fun listChannels(
        arg: VideoIngressListChannelsRequest = VideoIngressListChannelsRequest()
    ): VideoIngressListChannelsResponse =
        this.callUnary("VideoIngress/ListChannels", arg)

    suspend fun createChannel(
        arg: VideoIngressCreateChannelRequest = VideoIngressCreateChannelRequest()
    ): VideoIngressCreateChannelResponse =
        this.callUnary("VideoIngress/CreateChannel", arg)

    suspend fun updateChannel(
        arg: VideoIngressUpdateChannelRequest = VideoIngressUpdateChannelRequest()
    ): VideoIngressUpdateChannelResponse =
        this.callUnary("VideoIngress/UpdateChannel", arg)

    suspend fun deleteChannel(
        arg: VideoIngressDeleteChannelRequest = VideoIngressDeleteChannelRequest()
    ): VideoIngressDeleteChannelResponse =
        this.callUnary("VideoIngress/DeleteChannel", arg)

    suspend fun getChannelURL(
        arg: VideoIngressGetChannelURLRequest = VideoIngressGetChannelURLRequest()
    ): VideoIngressGetChannelURLResponse =
        this.callUnary("VideoIngress/GetChannelURL", arg)

}
