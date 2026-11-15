// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IGetConfigRequest as strims_vnic_v1_IGetConfigRequest,
  GetConfigRequest as strims_vnic_v1_GetConfigRequest,
  GetConfigResponse as strims_vnic_v1_GetConfigResponse,
  type ISetConfigRequest as strims_vnic_v1_ISetConfigRequest,
  SetConfigRequest as strims_vnic_v1_SetConfigRequest,
  SetConfigResponse as strims_vnic_v1_SetConfigResponse,
} from "./vnic.pb";

export interface VNICFrontendService {
  getConfig(req: strims_vnic_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_GetConfigResponse> | strims_vnic_v1_GetConfigResponse;
  setConfig(req: strims_vnic_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_SetConfigResponse> | strims_vnic_v1_SetConfigResponse;
}

export class UnimplementedVNICFrontendService implements VNICFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getConfig(req: strims_vnic_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_GetConfigResponse> | strims_vnic_v1_GetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setConfig(req: strims_vnic_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_SetConfigResponse> | strims_vnic_v1_SetConfigResponse { throw new Error("not implemented"); }
}

export const registerVNICFrontendService = (host: strims_rpc_Service, service: VNICFrontendService): void => {
  host.registerMethod<strims_vnic_v1_GetConfigRequest, strims_vnic_v1_GetConfigResponse>("strims.vnic.v1.VNICFrontend.GetConfig", (req, call) => service.getConfig(req, call), strims_vnic_v1_GetConfigRequest);
  host.registerMethod<strims_vnic_v1_SetConfigRequest, strims_vnic_v1_SetConfigResponse>("strims.vnic.v1.VNICFrontend.SetConfig", (req, call) => service.setConfig(req, call), strims_vnic_v1_SetConfigRequest);
}

export class VNICFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getConfig(req?: strims_vnic_v1_IGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_vnic_v1_GetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.vnic.v1.VNICFrontend.GetConfig", new strims_vnic_v1_GetConfigRequest(req)), strims_vnic_v1_GetConfigResponse, opts);
  }

  public setConfig(req?: strims_vnic_v1_ISetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_vnic_v1_SetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.vnic.v1.VNICFrontend.SetConfig", new strims_vnic_v1_SetConfigRequest(req)), strims_vnic_v1_SetConfigResponse, opts);
  }
}

