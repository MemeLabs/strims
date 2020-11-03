package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class CAClient(filepath: String) : RPCClient(filepath) {

    suspend fun renew(
        arg: CARenewRequest = CARenewRequest()
    ): CARenewResponse =
        this.callUnary("CA/Renew", arg)

}
