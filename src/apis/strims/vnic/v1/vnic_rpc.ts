import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IGetConfigRequest,
  GetConfigRequest,
  GetConfigResponse,
  ISetConfigRequest,
  SetConfigRequest,
  SetConfigResponse,
} from "./vnic";

export interface VNICFrontendService {
  getConfig(req: GetConfigRequest, call: strims_rpc_Call): Promise<GetConfigResponse> | GetConfigResponse;
  setConfig(req: SetConfigRequest, call: strims_rpc_Call): Promise<SetConfigResponse> | SetConfigResponse;
}

export class UnimplementedVNICFrontendService implements VNICFrontendService {
  getConfig(req: GetConfigRequest, call: strims_rpc_Call): Promise<GetConfigResponse> | GetConfigResponse { throw new Error("not implemented"); }
  setConfig(req: SetConfigRequest, call: strims_rpc_Call): Promise<SetConfigResponse> | SetConfigResponse { throw new Error("not implemented"); }
}

export const registerVNICFrontendService = (host: strims_rpc_Service, service: VNICFrontendService): void => {
  host.registerMethod<GetConfigRequest, GetConfigResponse>("strims.vnic.v1.VNICFrontend.GetConfig", service.getConfig.bind(service), GetConfigRequest);
  host.registerMethod<SetConfigRequest, SetConfigResponse>("strims.vnic.v1.VNICFrontend.SetConfig", service.setConfig.bind(service), SetConfigRequest);
}

export class VNICFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getConfig(req?: IGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.vnic.v1.VNICFrontend.GetConfig", new GetConfigRequest(req)), GetConfigResponse, opts);
  }

  public setConfig(req?: ISetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.vnic.v1.VNICFrontend.SetConfig", new SetConfigRequest(req)), SetConfigResponse, opts);
  }
}

