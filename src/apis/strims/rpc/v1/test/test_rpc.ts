import { RPCHost } from "../../../../../lib/rpc/host";
import { registerType } from "../../../../../lib/rpc/registry";
import { Readable as GenericReadable } from "../../../../../lib/rpc/stream";

import {
  IRPCCallUnaryRequest,
  RPCCallUnaryRequest,
  RPCCallUnaryResponse,
  IRPCCallStreamRequest,
  RPCCallStreamRequest,
  RPCCallStreamResponse,
} from "./test";

registerType("strims.rpc.v1.test.RPCCallUnaryRequest", RPCCallUnaryRequest);
registerType("strims.rpc.v1.test.RPCCallUnaryResponse", RPCCallUnaryResponse);
registerType("strims.rpc.v1.test.RPCCallStreamRequest", RPCCallStreamRequest);
registerType("strims.rpc.v1.test.RPCCallStreamResponse", RPCCallStreamResponse);

export class RPCTestClient {
  constructor(private readonly host: RPCHost) {}

  public callUnary(arg: IRPCCallUnaryRequest = new RPCCallUnaryRequest()): Promise<RPCCallUnaryResponse> {
    return this.host.expectOne(this.host.call("strims.rpc.v1.test.RPCTest.CallUnary", new RPCCallUnaryRequest(arg)));
  }

  public callStream(arg: IRPCCallStreamRequest = new RPCCallStreamRequest()): GenericReadable<RPCCallStreamResponse> {
    return this.host.expectMany(this.host.call("strims.rpc.v1.test.RPCTest.CallStream", new RPCCallStreamRequest(arg)));
  }
}

