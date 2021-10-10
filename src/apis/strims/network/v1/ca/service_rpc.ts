import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICARenewRequest,
  CARenewRequest,
  CARenewResponse,
  ICAFindRequest,
  CAFindRequest,
  CAFindResponse,
} from "./service";

registerType("strims.network.v1.ca.CARenewRequest", CARenewRequest);
registerType("strims.network.v1.ca.CARenewResponse", CARenewResponse);
registerType("strims.network.v1.ca.CAFindRequest", CAFindRequest);
registerType("strims.network.v1.ca.CAFindResponse", CAFindResponse);

export interface CAService {
  renew(req: CARenewRequest, call: strims_rpc_Call): Promise<CARenewResponse> | CARenewResponse;
  find(req: CAFindRequest, call: strims_rpc_Call): Promise<CAFindResponse> | CAFindResponse;
}

export const registerCAService = (host: strims_rpc_Service, service: CAService): void => {
  host.registerMethod<CARenewRequest, CARenewResponse>("strims.network.v1.ca.CA.Renew", service.renew.bind(service));
  host.registerMethod<CAFindRequest, CAFindResponse>("strims.network.v1.ca.CA.Find", service.find.bind(service));
}

export class CAClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public renew(req?: ICARenewRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CARenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Renew", new CARenewRequest(req)), opts);
  }

  public find(req?: ICAFindRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CAFindResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Find", new CAFindRequest(req)), opts);
  }
}

