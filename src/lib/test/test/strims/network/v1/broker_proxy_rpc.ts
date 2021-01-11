import { RPCHost } from "../../../../../rpc/host";
import { registerType } from "../../../../../pb/registry";
import { Readable as GenericReadable } from "../../../../../rpc/stream";

import {
  IBrokerProxyRequest,
  BrokerProxyRequest,
  BrokerProxyEvent,
  IBrokerProxySendKeysRequest,
  BrokerProxySendKeysRequest,
  BrokerProxySendKeysResponse,
  IBrokerProxyReceiveKeysRequest,
  BrokerProxyReceiveKeysRequest,
  BrokerProxyReceiveKeysResponse,
  IBrokerProxyDataRequest,
  BrokerProxyDataRequest,
  BrokerProxyDataResponse,
  IBrokerProxyCloseRequest,
  BrokerProxyCloseRequest,
  BrokerProxyCloseResponse,
} from "./broker_proxy";

registerType(".strims.network.v1.BrokerProxyRequest", BrokerProxyRequest);
registerType(".strims.network.v1.BrokerProxyEvent", BrokerProxyEvent);
registerType(".strims.network.v1.BrokerProxySendKeysRequest", BrokerProxySendKeysRequest);
registerType(".strims.network.v1.BrokerProxySendKeysResponse", BrokerProxySendKeysResponse);
registerType(".strims.network.v1.BrokerProxyReceiveKeysRequest", BrokerProxyReceiveKeysRequest);
registerType(".strims.network.v1.BrokerProxyReceiveKeysResponse", BrokerProxyReceiveKeysResponse);
registerType(".strims.network.v1.BrokerProxyDataRequest", BrokerProxyDataRequest);
registerType(".strims.network.v1.BrokerProxyDataResponse", BrokerProxyDataResponse);
registerType(".strims.network.v1.BrokerProxyCloseRequest", BrokerProxyCloseRequest);
registerType(".strims.network.v1.BrokerProxyCloseResponse", BrokerProxyCloseResponse);

export class BrokerProxyClient {
  constructor(private readonly host: RPCHost) {}

  public open(arg: IBrokerProxyRequest = new BrokerProxyRequest()): GenericReadable<BrokerProxyEvent> {
    return this.host.expectMany(this.host.call(".strims.network.v1.BrokerProxy.Open", new BrokerProxyRequest(arg)));
  }

  public sendKeys(arg: IBrokerProxySendKeysRequest = new BrokerProxySendKeysRequest()): Promise<BrokerProxySendKeysResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.BrokerProxy.SendKeys", new BrokerProxySendKeysRequest(arg)));
  }

  public receiveKeys(arg: IBrokerProxyReceiveKeysRequest = new BrokerProxyReceiveKeysRequest()): Promise<BrokerProxyReceiveKeysResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.BrokerProxy.ReceiveKeys", new BrokerProxyReceiveKeysRequest(arg)));
  }

  public data(arg: IBrokerProxyDataRequest = new BrokerProxyDataRequest()): Promise<BrokerProxyDataResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.BrokerProxy.Data", new BrokerProxyDataRequest(arg)));
  }

  public close(arg: IBrokerProxyCloseRequest = new BrokerProxyCloseRequest()): Promise<BrokerProxyCloseResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.BrokerProxy.Close", new BrokerProxyCloseRequest(arg)));
  }
}

