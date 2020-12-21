import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Funding {
  constructor(private readonly host: RPCHost) {}

  public test(
    arg: pb.IFundingTestRequest = new pb.FundingTestRequest()
  ): Promise<pb.FundingTestResponse> {
    return this.host.expectOne(this.host.call("Funding/Test", new pb.FundingTestRequest(arg)));
  }
}
