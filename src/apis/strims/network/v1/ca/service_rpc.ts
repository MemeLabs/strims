import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_network_v1_ca_ICARenewRequest,
  strims_network_v1_ca_CARenewRequest,
  strims_network_v1_ca_CARenewResponse,
  strims_network_v1_ca_ICAFindRequest,
  strims_network_v1_ca_CAFindRequest,
  strims_network_v1_ca_CAFindResponse,
} from "./service";

export interface CAService {
  renew(req: strims_network_v1_ca_CARenewRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CARenewResponse> | strims_network_v1_ca_CARenewResponse;
  find(req: strims_network_v1_ca_CAFindRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CAFindResponse> | strims_network_v1_ca_CAFindResponse;
}

export class UnimplementedCAService implements CAService {
  renew(req: strims_network_v1_ca_CARenewRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CARenewResponse> | strims_network_v1_ca_CARenewResponse { throw new Error("not implemented"); }
  find(req: strims_network_v1_ca_CAFindRequest, call: strims_rpc_Call): Promise<strims_network_v1_ca_CAFindResponse> | strims_network_v1_ca_CAFindResponse { throw new Error("not implemented"); }
}

export const registerCAService = (host: strims_rpc_Service, service: CAService): void => {
  host.registerMethod<strims_network_v1_ca_CARenewRequest, strims_network_v1_ca_CARenewResponse>("strims.network.v1.ca.CA.Renew", service.renew.bind(service), strims_network_v1_ca_CARenewRequest);
  host.registerMethod<strims_network_v1_ca_CAFindRequest, strims_network_v1_ca_CAFindResponse>("strims.network.v1.ca.CA.Find", service.find.bind(service), strims_network_v1_ca_CAFindRequest);
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

