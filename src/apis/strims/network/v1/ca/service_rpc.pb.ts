// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type ICARenewRequest as strims_network_v1_ca_ICARenewRequest,
  CARenewRequest as strims_network_v1_ca_CARenewRequest,
  CARenewResponse as strims_network_v1_ca_CARenewResponse,
  type ICAFindRequest as strims_network_v1_ca_ICAFindRequest,
  CAFindRequest as strims_network_v1_ca_CAFindRequest,
  CAFindResponse as strims_network_v1_ca_CAFindResponse,
} from "./service.pb";

export interface CAService {
  renew(req: strims_network_v1_ca_CARenewRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CARenewResponse> | strims_network_v1_ca_CARenewResponse;
  find(req: strims_network_v1_ca_CAFindRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CAFindResponse> | strims_network_v1_ca_CAFindResponse;
}

export class UnimplementedCAService implements CAService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  renew(req: strims_network_v1_ca_CARenewRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CARenewResponse> | strims_network_v1_ca_CARenewResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  find(req: strims_network_v1_ca_CAFindRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CAFindResponse> | strims_network_v1_ca_CAFindResponse { throw new Error("not implemented"); }
}

export const registerCAService = (host: strims_rpc_Service, service: CAService): void => {
  host.registerMethod<strims_network_v1_ca_CARenewRequest, strims_network_v1_ca_CARenewResponse>("strims.network.v1.ca.CA.Renew", (req, call) => service.renew(req, call), strims_network_v1_ca_CARenewRequest);
  host.registerMethod<strims_network_v1_ca_CAFindRequest, strims_network_v1_ca_CAFindResponse>("strims.network.v1.ca.CA.Find", (req, call) => service.find(req, call), strims_network_v1_ca_CAFindRequest);
}

export class CAClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public renew(req?: strims_network_v1_ca_ICARenewRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ca_CARenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Renew", new strims_network_v1_ca_CARenewRequest(req)), strims_network_v1_ca_CARenewResponse, opts);
  }

  public find(req?: strims_network_v1_ca_ICAFindRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ca_CAFindResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Find", new strims_network_v1_ca_CAFindRequest(req)), strims_network_v1_ca_CAFindResponse, opts);
  }
}

