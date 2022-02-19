import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICARenewRequest,
  CARenewRequest,
  CARenewResponse,
  ICAFindRequest,
  CAFindRequest,
  CAFindResponse,
} from "./service";

export interface CAService {
  renew(req: CARenewRequest, call: strims_rpc_Call): Promise<CARenewResponse> | CARenewResponse;
  find(req: CAFindRequest, call: strims_rpc_Call): Promise<CAFindResponse> | CAFindResponse;
}

export class UnimplementedCAService implements CAService {
  renew(req: CARenewRequest, call: strims_rpc_Call): Promise<CARenewResponse> | CARenewResponse { throw new Error("not implemented"); }
  find(req: CAFindRequest, call: strims_rpc_Call): Promise<CAFindResponse> | CAFindResponse { throw new Error("not implemented"); }
}

export const registerCAService = (host: strims_rpc_Service, service: CAService): void => {
  host.registerMethod<CARenewRequest, CARenewResponse>("strims.network.v1.ca.CA.Renew", service.renew.bind(service), CARenewRequest);
  host.registerMethod<CAFindRequest, CAFindResponse>("strims.network.v1.ca.CA.Find", service.find.bind(service), CAFindRequest);
}

export class CAClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public renew(req?: ICARenewRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CARenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Renew", new CARenewRequest(req)), CARenewResponse, opts);
  }

  public find(req?: ICAFindRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CAFindResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Find", new CAFindRequest(req)), CAFindResponse, opts);
  }
}

