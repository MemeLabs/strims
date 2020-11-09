import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class RPCTest {
  constructor(private readonly host: RPCHost) {}

  public callUnary(
    arg: pb.IRPCCallUnaryRequest = new pb.RPCCallUnaryRequest()
  ): Promise<pb.RPCCallUnaryResponse> {
    return this.host.expectOne(
      this.host.call("RPCTest/CallUnary", new pb.RPCCallUnaryRequest(arg))
    );
  }
  public callStream(
    arg: pb.IRPCCallStreamRequest = new pb.RPCCallStreamRequest()
  ): GenericReadable<pb.RPCCallStreamResponse> {
    return this.host.expectMany(
      this.host.call("RPCTest/CallStream", new pb.RPCCallStreamRequest(arg))
    );
  }
}
