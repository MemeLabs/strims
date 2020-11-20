import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class CAPeer {
  constructor(private readonly host: RPCHost) {}

  public renew(
    arg: pb.ICAPeerRenewRequest = new pb.CAPeerRenewRequest()
  ): Promise<pb.CAPeerRenewResponse> {
    return this.host.expectOne(this.host.call("CAPeer/Renew", new pb.CAPeerRenewRequest(arg)));
  }
}
