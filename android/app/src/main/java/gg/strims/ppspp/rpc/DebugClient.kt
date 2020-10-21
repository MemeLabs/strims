package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class DebugClient(filepath: String) : RPCClient(filepath) {

    suspend fun pProf(
        arg: PProfRequest = PProfRequest()
    ): PProfResponse =
        this.callUnary("Debug/PProf", arg)

    suspend fun readMetrics(
        arg: ReadMetricsRequest = ReadMetricsRequest()
    ): ReadMetricsResponse =
        this.callUnary("Debug/ReadMetrics", arg)

}
