package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class VideoClient(filepath: String) : RPCClient(filepath) {

    suspend fun openClient(
        arg: OpenVideoClientRequest = OpenVideoClientRequest()
    ): RPCResponseStream<VideoClientEvent> =
        this.callStreaming("Video/OpenClient", arg)

    suspend fun openServer(
        arg: OpenVideoServerRequest = OpenVideoServerRequest()
    ): VideoServerOpenResponse =
        this.callUnary("Video/OpenServer", arg)

    suspend fun writeToServer(
        arg: WriteToVideoServerRequest = WriteToVideoServerRequest()
    ): WriteToVideoServerResponse =
        this.callUnary("Video/WriteToServer", arg)

    suspend fun publishSwarm(
        arg: PublishSwarmRequest = PublishSwarmRequest()
    ): PublishSwarmResponse =
        this.callUnary("Video/PublishSwarm", arg)

    suspend fun startRTMPIngress(
        arg: StartRTMPIngressRequest = StartRTMPIngressRequest()
    ): StartRTMPIngressResponse =
        this.callUnary("Video/StartRTMPIngress", arg)

    suspend fun startHLSEgress(
        arg: StartHLSEgressRequest = StartHLSEgressRequest()
    ): StartHLSEgressResponse =
        this.callUnary("Video/StartHLSEgress", arg)

    suspend fun stopHLSEgress(
        arg: StopHLSEgressRequest = StopHLSEgressRequest()
    ): StopHLSEgressResponse =
        this.callUnary("Video/StopHLSEgress", arg)

}
