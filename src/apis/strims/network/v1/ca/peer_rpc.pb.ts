// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type ICAPeerRenewRequest as strims_network_v1_ca_ICAPeerRenewRequest,
  CAPeerRenewRequest as strims_network_v1_ca_CAPeerRenewRequest,
  CAPeerRenewResponse as strims_network_v1_ca_CAPeerRenewResponse,
} from "./peer.pb";

export interface CAPeerService {
  renew(req: strims_network_v1_ca_CAPeerRenewRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CAPeerRenewResponse> | strims_network_v1_ca_CAPeerRenewResponse;
}

export class UnimplementedCAPeerService implements CAPeerService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  renew(req: strims_network_v1_ca_CAPeerRenewRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CAPeerRenewResponse> | strims_network_v1_ca_CAPeerRenewResponse { throw new Error("not implemented"); }
}

export const registerCAPeerService = (host: strims_rpc_Service, service: CAPeerService): void => {
  host.registerMethod<strims_network_v1_ca_CAPeerRenewRequest, strims_network_v1_ca_CAPeerRenewResponse>("strims.network.v1.ca.CAPeer.Renew", (req, call) => service.renew(req, call), strims_network_v1_ca_CAPeerRenewRequest);
}

export class CAPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public renew(req?: strims_network_v1_ca_ICAPeerRenewRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ca_CAPeerRenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CAPeer.Renew", new strims_network_v1_ca_CAPeerRenewRequest(req)), strims_network_v1_ca_CAPeerRenewResponse, opts);
  }
}

