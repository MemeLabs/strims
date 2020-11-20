package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class CAPeerClient(filepath: String) : RPCClient(filepath) {

    suspend fun renew(
        arg: CAPeerRenewRequest = CAPeerRenewRequest()
    ): CAPeerRenewResponse =
        this.callUnary("CAPeer/Renew", arg)

}
