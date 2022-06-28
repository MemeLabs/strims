import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_vnic_v1_IGetConfigRequest,
  strims_vnic_v1_GetConfigRequest,
  strims_vnic_v1_GetConfigResponse,
  strims_vnic_v1_ISetConfigRequest,
  strims_vnic_v1_SetConfigRequest,
  strims_vnic_v1_SetConfigResponse,
} from "./vnic";

export interface VNICFrontendService {
  getConfig(req: strims_vnic_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_GetConfigResponse> | strims_vnic_v1_GetConfigResponse;
  setConfig(req: strims_vnic_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_SetConfigResponse> | strims_vnic_v1_SetConfigResponse;
}

export class UnimplementedVNICFrontendService implements VNICFrontendService {
  getConfig(req: strims_vnic_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_GetConfigResponse> | strims_vnic_v1_GetConfigResponse { throw new Error("not implemented"); }
  setConfig(req: strims_vnic_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_vnic_v1_SetConfigResponse> | strims_vnic_v1_SetConfigResponse { throw new Error("not implemented"); }
}

export const registerVNICFrontendService = (host: strims_rpc_Service, service: VNICFrontendService): void => {
  host.registerMethod<strims_vnic_v1_GetConfigRequest, strims_vnic_v1_GetConfigResponse>("strims.vnic.v1.VNICFrontend.GetConfig", service.getConfig.bind(service), strims_vnic_v1_GetConfigRequest);
  host.registerMethod<strims_vnic_v1_SetConfigRequest, strims_vnic_v1_SetConfigResponse>("strims.vnic.v1.VNICFrontend.SetConfig", service.setConfig.bind(service), strims_vnic_v1_SetConfigRequest);
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

