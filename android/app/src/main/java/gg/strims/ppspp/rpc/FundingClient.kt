package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class FundingClient(filepath: String) : RPCClient(filepath) {

    suspend fun test(
        arg: FundingTestRequest = FundingTestRequest()
    ): FundingTestResponse =
        this.callUnary("Funding/Test", arg)

}
