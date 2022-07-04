import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_network_v1_IBrokerProxyRequest,
  strims_network_v1_BrokerProxyRequest,
  strims_network_v1_BrokerProxyEvent,
  strims_network_v1_IBrokerProxySendKeysRequest,
  strims_network_v1_BrokerProxySendKeysRequest,
  strims_network_v1_BrokerProxySendKeysResponse,
  strims_network_v1_IBrokerProxyReceiveKeysRequest,
  strims_network_v1_BrokerProxyReceiveKeysRequest,
  strims_network_v1_BrokerProxyReceiveKeysResponse,
  strims_network_v1_IBrokerProxyDataRequest,
  strims_network_v1_BrokerProxyDataRequest,
  strims_network_v1_BrokerProxyDataResponse,
  strims_network_v1_IBrokerProxyCloseRequest,
  strims_network_v1_BrokerProxyCloseRequest,
  strims_network_v1_BrokerProxyCloseResponse,
} from "./broker_proxy";

export interface BrokerProxyService {
  open(req: strims_network_v1_BrokerProxyRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_BrokerProxyEvent>;
  sendKeys(req: strims_network_v1_BrokerProxySendKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxySendKeysResponse> | strims_network_v1_BrokerProxySendKeysResponse;
  receiveKeys(req: strims_network_v1_BrokerProxyReceiveKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyReceiveKeysResponse> | strims_network_v1_BrokerProxyReceiveKeysResponse;
  data(req: strims_network_v1_BrokerProxyDataRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyDataResponse> | strims_network_v1_BrokerProxyDataResponse;
  close(req: strims_network_v1_BrokerProxyCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyCloseResponse> | strims_network_v1_BrokerProxyCloseResponse;
}

export class UnimplementedBrokerProxyService implements BrokerProxyService {
  open(req: strims_network_v1_BrokerProxyRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_BrokerProxyEvent> { throw new Error("not implemented"); }
  sendKeys(req: strims_network_v1_BrokerProxySendKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxySendKeysResponse> | strims_network_v1_BrokerProxySendKeysResponse { throw new Error("not implemented"); }
  receiveKeys(req: strims_network_v1_BrokerProxyReceiveKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyReceiveKeysResponse> | strims_network_v1_BrokerProxyReceiveKeysResponse { throw new Error("not implemented"); }
  data(req: strims_network_v1_BrokerProxyDataRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyDataResponse> | strims_network_v1_BrokerProxyDataResponse { throw new Error("not implemented"); }
  close(req: strims_network_v1_BrokerProxyCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyCloseResponse> | strims_network_v1_BrokerProxyCloseResponse { throw new Error("not implemented"); }
}

export const registerBrokerProxyService = (host: strims_rpc_Service, service: BrokerProxyService): void => {
  host.registerMethod<strims_network_v1_BrokerProxyRequest, strims_network_v1_BrokerProxyEvent>("strims.network.v1.BrokerProxy.Open", service.open.bind(service), strims_network_v1_BrokerProxyRequest);
  host.registerMethod<strims_network_v1_BrokerProxySendKeysRequest, strims_network_v1_BrokerProxySendKeysResponse>("strims.network.v1.BrokerProxy.SendKeys", service.sendKeys.bind(service), strims_network_v1_BrokerProxySendKeysRequest);
  host.registerMethod<strims_network_v1_BrokerProxyReceiveKeysRequest, strims_network_v1_BrokerProxyReceiveKeysResponse>("strims.network.v1.BrokerProxy.ReceiveKeys", service.receiveKeys.bind(service), strims_network_v1_BrokerProxyReceiveKeysRequest);
  host.registerMethod<strims_network_v1_BrokerProxyDataRequest, strims_network_v1_BrokerProxyDataResponse>("strims.network.v1.BrokerProxy.Data", service.data.bind(service), strims_network_v1_BrokerProxyDataRequest);
  host.registerMethod<strims_network_v1_BrokerProxyCloseRequest, strims_network_v1_BrokerProxyCloseResponse>("strims.network.v1.BrokerProxy.Close", service.close.bind(service), strims_network_v1_BrokerProxyCloseRequest);
}

export class BrokerProxyClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: strims_network_v1_IBrokerProxyRequest): GenericReadable<strims_network_v1_BrokerProxyEvent> {
    return this.host.expectMany(this.host.call("strims.network.v1.BrokerProxy.Open", new strims_network_v1_BrokerProxyRequest(req)), strims_network_v1_BrokerProxyEvent);
  }

  public sendKeys(req?: strims_network_v1_IBrokerProxySendKeysRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_BrokerProxySendKeysResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.SendKeys", new strims_network_v1_BrokerProxySendKeysRequest(req)), strims_network_v1_BrokerProxySendKeysResponse, opts);
  }

  public receiveKeys(req?: strims_network_v1_IBrokerProxyReceiveKeysRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_BrokerProxyReceiveKeysResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.ReceiveKeys", new strims_network_v1_BrokerProxyReceiveKeysRequest(req)), strims_network_v1_BrokerProxyReceiveKeysResponse, opts);
  }

  public data(req?: strims_network_v1_IBrokerProxyDataRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_BrokerProxyDataResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.Data", new strims_network_v1_BrokerProxyDataRequest(req)), strims_network_v1_BrokerProxyDataResponse, opts);
  }

  public close(req?: strims_network_v1_IBrokerProxyCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_BrokerProxyCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.Close", new strims_network_v1_BrokerProxyCloseRequest(req)), strims_network_v1_BrokerProxyCloseResponse, opts);
  }
}

