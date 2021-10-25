import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

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

export interface BrokerProxyService {
  open(req: BrokerProxyRequest, call: strims_rpc_Call): GenericReadable<BrokerProxyEvent>;
  sendKeys(req: BrokerProxySendKeysRequest, call: strims_rpc_Call): Promise<BrokerProxySendKeysResponse> | BrokerProxySendKeysResponse;
  receiveKeys(req: BrokerProxyReceiveKeysRequest, call: strims_rpc_Call): Promise<BrokerProxyReceiveKeysResponse> | BrokerProxyReceiveKeysResponse;
  data(req: BrokerProxyDataRequest, call: strims_rpc_Call): Promise<BrokerProxyDataResponse> | BrokerProxyDataResponse;
  close(req: BrokerProxyCloseRequest, call: strims_rpc_Call): Promise<BrokerProxyCloseResponse> | BrokerProxyCloseResponse;
}

export const registerBrokerProxyService = (host: strims_rpc_Service, service: BrokerProxyService): void => {
  host.registerMethod<BrokerProxyRequest, BrokerProxyEvent>("strims.network.v1.BrokerProxy.Open", service.open.bind(service), BrokerProxyRequest);
  host.registerMethod<BrokerProxySendKeysRequest, BrokerProxySendKeysResponse>("strims.network.v1.BrokerProxy.SendKeys", service.sendKeys.bind(service), BrokerProxySendKeysRequest);
  host.registerMethod<BrokerProxyReceiveKeysRequest, BrokerProxyReceiveKeysResponse>("strims.network.v1.BrokerProxy.ReceiveKeys", service.receiveKeys.bind(service), BrokerProxyReceiveKeysRequest);
  host.registerMethod<BrokerProxyDataRequest, BrokerProxyDataResponse>("strims.network.v1.BrokerProxy.Data", service.data.bind(service), BrokerProxyDataRequest);
  host.registerMethod<BrokerProxyCloseRequest, BrokerProxyCloseResponse>("strims.network.v1.BrokerProxy.Close", service.close.bind(service), BrokerProxyCloseRequest);
}

export class BrokerProxyClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: IBrokerProxyRequest): GenericReadable<BrokerProxyEvent> {
    return this.host.expectMany(this.host.call("strims.network.v1.BrokerProxy.Open", new BrokerProxyRequest(req)), BrokerProxyEvent);
  }

  public sendKeys(req?: IBrokerProxySendKeysRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxySendKeysResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.SendKeys", new BrokerProxySendKeysRequest(req)), BrokerProxySendKeysResponse, opts);
  }

  public receiveKeys(req?: IBrokerProxyReceiveKeysRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxyReceiveKeysResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.ReceiveKeys", new BrokerProxyReceiveKeysRequest(req)), BrokerProxyReceiveKeysResponse, opts);
  }

  public data(req?: IBrokerProxyDataRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxyDataResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.Data", new BrokerProxyDataRequest(req)), BrokerProxyDataResponse, opts);
  }

  public close(req?: IBrokerProxyCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxyCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.Close", new BrokerProxyCloseRequest(req)), BrokerProxyCloseResponse, opts);
  }
}

