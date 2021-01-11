import { RPCHost } from "../../../../../../rpc/host";
import { registerType } from "../../../../../../pb/registry";

import {
  ICAPeerRenewRequest,
  CAPeerRenewRequest,
  CAPeerRenewResponse,
} from "./peer";

registerType(".strims.network.v1.ca.CAPeerRenewRequest", CAPeerRenewRequest);
registerType(".strims.network.v1.ca.CAPeerRenewResponse", CAPeerRenewResponse);

export class CAPeerClient {
  constructor(private readonly host: RPCHost) {}

  public renew(arg: ICAPeerRenewRequest = new CAPeerRenewRequest()): Promise<CAPeerRenewResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.ca.CAPeer.Renew", new CAPeerRenewRequest(arg)));
  }
}

