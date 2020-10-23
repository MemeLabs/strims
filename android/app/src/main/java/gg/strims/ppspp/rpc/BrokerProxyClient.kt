package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class BrokerProxyClient(filepath: String) : RPCClient(filepath) {

    suspend fun open(
        arg: BrokerProxyRequest = BrokerProxyRequest()
    ): RPCResponseStream<BrokerProxyEvent> =
        this.callStreaming("BrokerProxy/Open", arg)

    suspend fun sendKeys(
        arg: BrokerProxySendKeysRequest = BrokerProxySendKeysRequest()
    ): BrokerProxySendKeysResponse =
        this.callUnary("BrokerProxy/SendKeys", arg)

    suspend fun receiveKeys(
        arg: BrokerProxyReceiveKeysRequest = BrokerProxyReceiveKeysRequest()
    ): BrokerProxyReceiveKeysResponse =
        this.callUnary("BrokerProxy/ReceiveKeys", arg)

    suspend fun data(
        arg: BrokerProxyDataRequest = BrokerProxyDataRequest()
    ): BrokerProxyDataResponse =
        this.callUnary("BrokerProxy/Data", arg)

    suspend fun close(
        arg: BrokerProxyCloseRequest = BrokerProxyCloseRequest()
    ): BrokerProxyCloseResponse =
        this.callUnary("BrokerProxy/Close", arg)

}
