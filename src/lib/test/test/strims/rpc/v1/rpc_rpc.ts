import { RPCHost } from "../../../../../rpc/host";
import { registerType } from "../../../../../pb/registry";
import { Readable as GenericReadable } from "../../../../../rpc/stream";

import {
  IRPCCallUnaryRequest,
  RPCCallUnaryRequest,
  RPCCallUnaryResponse,
  IRPCCallStreamRequest,
  RPCCallStreamRequest,
  RPCCallStreamResponse,
} from "./rpc";

registerType(".strims.rpc.v1.RPCCallUnaryRequest", RPCCallUnaryRequest);
registerType(".strims.rpc.v1.RPCCallUnaryResponse", RPCCallUnaryResponse);
registerType(".strims.rpc.v1.RPCCallStreamRequest", RPCCallStreamRequest);
registerType(".strims.rpc.v1.RPCCallStreamResponse", RPCCallStreamResponse);

export class RPCTestClient {
  constructor(private readonly host: RPCHost) {}

  public callUnary(arg: IRPCCallUnaryRequest = new RPCCallUnaryRequest()): Promise<RPCCallUnaryResponse> {
    return this.host.expectOne(this.host.call(".strims.rpc.v1.RPCTest.CallUnary", new RPCCallUnaryRequest(arg)));
  }

  public callStream(arg: IRPCCallStreamRequest = new RPCCallStreamRequest()): GenericReadable<RPCCallStreamResponse> {
    return this.host.expectMany(this.host.call(".strims.rpc.v1.RPCTest.CallStream", new RPCCallStreamRequest(arg)));
  }
}

