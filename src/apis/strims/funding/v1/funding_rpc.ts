import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  IFundingTestRequest,
  FundingTestRequest,
  FundingTestResponse,
} from "./funding";

registerType("strims.funding.v1.FundingTestRequest", FundingTestRequest);
registerType("strims.funding.v1.FundingTestResponse", FundingTestResponse);

export class FundingClient {
  constructor(private readonly host: RPCHost) {}

  public test(arg?: IFundingTestRequest): Promise<FundingTestResponse> {
    return this.host.expectOne(this.host.call("strims.funding.v1.Funding.Test", new FundingTestRequest(arg)));
  }
}

