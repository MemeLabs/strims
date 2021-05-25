import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICARenewRequest,
  CARenewRequest,
  CARenewResponse,
} from "./service";

registerType("strims.network.v1.ca.CARenewRequest", CARenewRequest);
registerType("strims.network.v1.ca.CARenewResponse", CARenewResponse);

export interface CAService {
  renew(req: CARenewRequest, call: strims_rpc_Call): Promise<CARenewResponse> | CARenewResponse;
}

export const registerCAService = (host: strims_rpc_Service, service: CAService): void => {
  host.registerMethod<CARenewRequest, CARenewResponse>("strims.network.v1.ca.CA.Renew", service.renew.bind(service));
}

export class CAClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public renew(req?: ICARenewRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CARenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Renew", new CARenewRequest(req)), opts);
  }
}

