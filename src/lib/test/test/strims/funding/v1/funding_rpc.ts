import { RPCHost } from "../../../../../rpc/host";
import { registerType } from "../../../../../pb/registry";

import {
  IFundingTestRequest,
  FundingTestRequest,
  FundingTestResponse,
} from "./funding";

registerType(".strims.funding.v1.FundingTestRequest", FundingTestRequest);
registerType(".strims.funding.v1.FundingTestResponse", FundingTestResponse);

export class FundingClient {
  constructor(private readonly host: RPCHost) {}

  public test(arg: IFundingTestRequest = new FundingTestRequest()): Promise<FundingTestResponse> {
    return this.host.expectOne(this.host.call(".strims.funding.v1.Funding.Test", new FundingTestRequest(arg)));
  }
}

