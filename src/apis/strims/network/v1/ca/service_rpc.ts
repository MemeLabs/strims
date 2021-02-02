import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  ICARenewRequest,
  CARenewRequest,
  CARenewResponse,
} from "./service";

registerType("strims.network.v1.ca.CARenewRequest", CARenewRequest);
registerType("strims.network.v1.ca.CARenewResponse", CARenewResponse);

export class CAClient {
  constructor(private readonly host: RPCHost) {}

  public renew(arg: ICARenewRequest = new CARenewRequest()): Promise<CARenewResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.ca.CA.Renew", new CARenewRequest(arg)));
  }
}

