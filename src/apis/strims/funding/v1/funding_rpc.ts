import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_funding_v1_IFundingTestRequest,
  strims_funding_v1_FundingTestRequest,
  strims_funding_v1_FundingTestResponse,
} from "./funding";

export interface FundingService {
  test(req: strims_funding_v1_FundingTestRequest, call: strims_rpc_Call): Promise<strims_funding_v1_FundingTestResponse> | strims_funding_v1_FundingTestResponse;
}

export class UnimplementedFundingService implements FundingService {
  test(req: strims_funding_v1_FundingTestRequest, call: strims_rpc_Call): Promise<strims_funding_v1_FundingTestResponse> | strims_funding_v1_FundingTestResponse { throw new Error("not implemented"); }
}

export const registerFundingService = (host: strims_rpc_Service, service: FundingService): void => {
  host.registerMethod<strims_funding_v1_FundingTestRequest, strims_funding_v1_FundingTestResponse>("strims.funding.v1.Funding.Test", service.test.bind(service), strims_funding_v1_FundingTestRequest);
}

export class FundingClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public test(req?: strims_funding_v1_IFundingTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_funding_v1_FundingTestResponse> {
    return this.host.expectOne(this.host.call("strims.funding.v1.Funding.Test", new strims_funding_v1_FundingTestRequest(req)), strims_funding_v1_FundingTestResponse, opts);
  }
}

