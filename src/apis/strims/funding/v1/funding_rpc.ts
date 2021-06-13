import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IFundingTestRequest,
  FundingTestRequest,
  FundingTestResponse,
} from "./funding";

registerType("strims.funding.v1.FundingTestRequest", FundingTestRequest);
registerType("strims.funding.v1.FundingTestResponse", FundingTestResponse);

export interface FundingService {
  test(req: FundingTestRequest, call: strims_rpc_Call): Promise<FundingTestResponse> | FundingTestResponse;
}

export const registerFundingService = (host: strims_rpc_Service, service: FundingService): void => {
  host.registerMethod<FundingTestRequest, FundingTestResponse>("strims.funding.v1.Funding.Test", service.test.bind(service));
}

export class FundingClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public test(req?: IFundingTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<FundingTestResponse> {
    return this.host.expectOne(this.host.call("strims.funding.v1.Funding.Test", new FundingTestRequest(req)), opts);
  }
}
