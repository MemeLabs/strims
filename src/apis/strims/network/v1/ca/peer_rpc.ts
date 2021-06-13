import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICAPeerRenewRequest,
  CAPeerRenewRequest,
  CAPeerRenewResponse,
} from "./peer";

registerType("strims.network.v1.ca.CAPeerRenewRequest", CAPeerRenewRequest);
registerType("strims.network.v1.ca.CAPeerRenewResponse", CAPeerRenewResponse);

export interface CAPeerService {
  renew(req: CAPeerRenewRequest, call: strims_rpc_Call): Promise<CAPeerRenewResponse> | CAPeerRenewResponse;
}

export const registerCAPeerService = (host: strims_rpc_Service, service: CAPeerService): void => {
  host.registerMethod<CAPeerRenewRequest, CAPeerRenewResponse>("strims.network.v1.ca.CAPeer.Renew", service.renew.bind(service));
}

export class CAPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public renew(req?: ICAPeerRenewRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CAPeerRenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CAPeer.Renew", new CAPeerRenewRequest(req)), opts);
  }
}
