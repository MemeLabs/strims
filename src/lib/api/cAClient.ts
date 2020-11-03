import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class CA {
  constructor(private readonly host: RPCHost) {}

  public renew(arg: pb.ICARenewRequest = new pb.CARenewRequest()): Promise<pb.CARenewResponse> {
    return this.host.expectOne(this.host.call("CA/Renew", new pb.CARenewRequest(arg)));
  }
}
