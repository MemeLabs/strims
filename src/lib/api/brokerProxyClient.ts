import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class BrokerProxy {
  constructor(private readonly host: RPCHost) {}

  public open(
    arg: pb.IBrokerProxyRequest = new pb.BrokerProxyRequest()
  ): GenericReadable<pb.BrokerProxyEvent> {
    return this.host.expectMany(this.host.call("BrokerProxy/Open", new pb.BrokerProxyRequest(arg)));
  }
  public sendKeys(
    arg: pb.IBrokerProxySendKeysRequest = new pb.BrokerProxySendKeysRequest()
  ): Promise<pb.BrokerProxySendKeysResponse> {
    return this.host.expectOne(
      this.host.call("BrokerProxy/SendKeys", new pb.BrokerProxySendKeysRequest(arg))
    );
  }
  public receiveKeys(
    arg: pb.IBrokerProxyReceiveKeysRequest = new pb.BrokerProxyReceiveKeysRequest()
  ): Promise<pb.BrokerProxyReceiveKeysResponse> {
    return this.host.expectOne(
      this.host.call("BrokerProxy/ReceiveKeys", new pb.BrokerProxyReceiveKeysRequest(arg))
    );
  }
  public data(
    arg: pb.IBrokerProxyDataRequest = new pb.BrokerProxyDataRequest()
  ): Promise<pb.BrokerProxyDataResponse> {
    return this.host.expectOne(
      this.host.call("BrokerProxy/Data", new pb.BrokerProxyDataRequest(arg))
    );
  }
}
