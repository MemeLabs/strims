import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Debug {
  constructor(private readonly host: RPCHost) {}

  public pProf(arg: pb.IPProfRequest = new pb.PProfRequest()): Promise<pb.PProfResponse> {
    return this.host.expectOne(this.host.call("Debug/PProf", new pb.PProfRequest(arg)));
  }
  public readMetrics(
    arg: pb.IReadMetricsRequest = new pb.ReadMetricsRequest()
  ): Promise<pb.ReadMetricsResponse> {
    return this.host.expectOne(this.host.call("Debug/ReadMetrics", new pb.ReadMetricsRequest(arg)));
  }
}
