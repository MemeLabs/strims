import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  IEgressOpenStreamRequest,
  EgressOpenStreamRequest,
  EgressOpenStreamResponse,
} from "./egress";

registerType("strims.video.v1.EgressOpenStreamRequest", EgressOpenStreamRequest);
registerType("strims.video.v1.EgressOpenStreamResponse", EgressOpenStreamResponse);

export class EgressClient {
  constructor(private readonly host: RPCHost) {}

  public openStream(arg: IEgressOpenStreamRequest = new EgressOpenStreamRequest()): GenericReadable<EgressOpenStreamResponse> {
    return this.host.expectMany(this.host.call("strims.video.v1.Egress.OpenStream", new EgressOpenStreamRequest(arg)));
  }
}

