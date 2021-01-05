package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class FundingClient(filepath: String) : RPCClient(filepath) {

    suspend fun test(
        arg: FundingTestRequest = FundingTestRequest()
    ): FundingTestResponse =
        this.callUnary("Funding/Test", arg)

    suspend fun getSummary(
        arg: FundingGetSummaryRequest = FundingGetSummaryRequest()
    ): FundingGetSummaryResponse =
        this.callUnary("Funding/GetSummary", arg)

    suspend fun createSubPlan(
        arg: FundingCreateSubPlanRequest = FundingCreateSubPlanRequest()
    ): FundingCreateSubPlanResponse =
        this.callUnary("Funding/CreateSubPlan", arg)

}
