import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
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

registerType("strims.network.v1.BrokerProxyRequest", BrokerProxyRequest);
registerType("strims.network.v1.BrokerProxyEvent", BrokerProxyEvent);
registerType("strims.network.v1.BrokerProxySendKeysRequest", BrokerProxySendKeysRequest);
registerType("strims.network.v1.BrokerProxySendKeysResponse", BrokerProxySendKeysResponse);
registerType("strims.network.v1.BrokerProxyReceiveKeysRequest", BrokerProxyReceiveKeysRequest);
registerType("strims.network.v1.BrokerProxyReceiveKeysResponse", BrokerProxyReceiveKeysResponse);
registerType("strims.network.v1.BrokerProxyDataRequest", BrokerProxyDataRequest);
registerType("strims.network.v1.BrokerProxyDataResponse", BrokerProxyDataResponse);
registerType("strims.network.v1.BrokerProxyCloseRequest", BrokerProxyCloseRequest);
registerType("strims.network.v1.BrokerProxyCloseResponse", BrokerProxyCloseResponse);

export interface BrokerProxyService {
  open(req: BrokerProxyRequest, call: strims_rpc_Call): GenericReadable<BrokerProxyEvent>;
  sendKeys(req: BrokerProxySendKeysRequest, call: strims_rpc_Call): Promise<BrokerProxySendKeysResponse> | BrokerProxySendKeysResponse;
  receiveKeys(req: BrokerProxyReceiveKeysRequest, call: strims_rpc_Call): Promise<BrokerProxyReceiveKeysResponse> | BrokerProxyReceiveKeysResponse;
  data(req: BrokerProxyDataRequest, call: strims_rpc_Call): Promise<BrokerProxyDataResponse> | BrokerProxyDataResponse;
  close(req: BrokerProxyCloseRequest, call: strims_rpc_Call): Promise<BrokerProxyCloseResponse> | BrokerProxyCloseResponse;
}

export const registerBrokerProxyService = (host: strims_rpc_Service, service: BrokerProxyService): void => {
  host.registerMethod<BrokerProxyRequest, BrokerProxyEvent>("strims.network.v1.BrokerProxy.Open", service.open.bind(service));
  host.registerMethod<BrokerProxySendKeysRequest, BrokerProxySendKeysResponse>("strims.network.v1.BrokerProxy.SendKeys", service.sendKeys.bind(service));
  host.registerMethod<BrokerProxyReceiveKeysRequest, BrokerProxyReceiveKeysResponse>("strims.network.v1.BrokerProxy.ReceiveKeys", service.receiveKeys.bind(service));
  host.registerMethod<BrokerProxyDataRequest, BrokerProxyDataResponse>("strims.network.v1.BrokerProxy.Data", service.data.bind(service));
  host.registerMethod<BrokerProxyCloseRequest, BrokerProxyCloseResponse>("strims.network.v1.BrokerProxy.Close", service.close.bind(service));
}

export class BrokerProxyClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: IBrokerProxyRequest): GenericReadable<BrokerProxyEvent> {
    return this.host.expectMany(this.host.call("strims.network.v1.BrokerProxy.Open", new BrokerProxyRequest(req)));
  }

  public sendKeys(req?: IBrokerProxySendKeysRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxySendKeysResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.SendKeys", new BrokerProxySendKeysRequest(req)), opts);
  }

  public receiveKeys(req?: IBrokerProxyReceiveKeysRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxyReceiveKeysResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.ReceiveKeys", new BrokerProxyReceiveKeysRequest(req)), opts);
  }

  public data(req?: IBrokerProxyDataRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxyDataResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.Data", new BrokerProxyDataRequest(req)), opts);
  }

  public close(req?: IBrokerProxyCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BrokerProxyCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.BrokerProxy.Close", new BrokerProxyCloseRequest(req)), opts);
  }
}

