// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type IBrokerProxyRequest as strims_network_v1_IBrokerProxyRequest,
  BrokerProxyRequest as strims_network_v1_BrokerProxyRequest,
  BrokerProxyEvent as strims_network_v1_BrokerProxyEvent,
  type IBrokerProxySendKeysRequest as strims_network_v1_IBrokerProxySendKeysRequest,
  BrokerProxySendKeysRequest as strims_network_v1_BrokerProxySendKeysRequest,
  BrokerProxySendKeysResponse as strims_network_v1_BrokerProxySendKeysResponse,
  type IBrokerProxyReceiveKeysRequest as strims_network_v1_IBrokerProxyReceiveKeysRequest,
  BrokerProxyReceiveKeysRequest as strims_network_v1_BrokerProxyReceiveKeysRequest,
  BrokerProxyReceiveKeysResponse as strims_network_v1_BrokerProxyReceiveKeysResponse,
  type IBrokerProxyDataRequest as strims_network_v1_IBrokerProxyDataRequest,
  BrokerProxyDataRequest as strims_network_v1_BrokerProxyDataRequest,
  BrokerProxyDataResponse as strims_network_v1_BrokerProxyDataResponse,
  type IBrokerProxyCloseRequest as strims_network_v1_IBrokerProxyCloseRequest,
  BrokerProxyCloseRequest as strims_network_v1_BrokerProxyCloseRequest,
  BrokerProxyCloseResponse as strims_network_v1_BrokerProxyCloseResponse,
} from "./broker_proxy.pb";

export interface BrokerProxyService {
  open(req: strims_network_v1_BrokerProxyRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_BrokerProxyEvent>;
  sendKeys(req: strims_network_v1_BrokerProxySendKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxySendKeysResponse> | strims_network_v1_BrokerProxySendKeysResponse;
  receiveKeys(req: strims_network_v1_BrokerProxyReceiveKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyReceiveKeysResponse> | strims_network_v1_BrokerProxyReceiveKeysResponse;
  data(req: strims_network_v1_BrokerProxyDataRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyDataResponse> | strims_network_v1_BrokerProxyDataResponse;
  close(req: strims_network_v1_BrokerProxyCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyCloseResponse> | strims_network_v1_BrokerProxyCloseResponse;
}

export class UnimplementedBrokerProxyService implements BrokerProxyService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  open(req: strims_network_v1_BrokerProxyRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_BrokerProxyEvent> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  sendKeys(req: strims_network_v1_BrokerProxySendKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxySendKeysResponse> | strims_network_v1_BrokerProxySendKeysResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  receiveKeys(req: strims_network_v1_BrokerProxyReceiveKeysRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyReceiveKeysResponse> | strims_network_v1_BrokerProxyReceiveKeysResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  data(req: strims_network_v1_BrokerProxyDataRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyDataResponse> | strims_network_v1_BrokerProxyDataResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  close(req: strims_network_v1_BrokerProxyCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_BrokerProxyCloseResponse> | strims_network_v1_BrokerProxyCloseResponse { throw new Error("not implemented"); }
}

export const registerBrokerProxyService = (host: strims_rpc_Service, service: BrokerProxyService): void => {
  host.registerMethod<strims_network_v1_BrokerProxyRequest, strims_network_v1_BrokerProxyEvent>("strims.network.v1.BrokerProxy.Open", (req, call) => service.open(req, call), strims_network_v1_BrokerProxyRequest);
  host.registerMethod<strims_network_v1_BrokerProxySendKeysRequest, strims_network_v1_BrokerProxySendKeysResponse>("strims.network.v1.BrokerProxy.SendKeys", (req, call) => service.sendKeys(req, call), strims_network_v1_BrokerProxySendKeysRequest);
  host.registerMethod<strims_network_v1_BrokerProxyReceiveKeysRequest, strims_network_v1_BrokerProxyReceiveKeysResponse>("strims.network.v1.BrokerProxy.ReceiveKeys", (req, call) => service.receiveKeys(req, call), strims_network_v1_BrokerProxyReceiveKeysRequest);
  host.registerMethod<strims_network_v1_BrokerProxyDataRequest, strims_network_v1_BrokerProxyDataResponse>("strims.network.v1.BrokerProxy.Data", (req, call) => service.data(req, call), strims_network_v1_BrokerProxyDataRequest);
  host.registerMethod<strims_network_v1_BrokerProxyCloseRequest, strims_network_v1_BrokerProxyCloseResponse>("strims.network.v1.BrokerProxy.Close", (req, call) => service.close(req, call), strims_network_v1_BrokerProxyCloseRequest);
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

