import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  IHLSEgressIsSupportedRequest,
  HLSEgressIsSupportedRequest,
  HLSEgressIsSupportedResponse,
  IHLSEgressOpenStreamRequest,
  HLSEgressOpenStreamRequest,
  HLSEgressOpenStreamResponse,
  IHLSEgressCloseStreamRequest,
  HLSEgressCloseStreamRequest,
  HLSEgressCloseStreamResponse,
} from "./hls_egress";

registerType("strims.video.v1.HLSEgressIsSupportedRequest", HLSEgressIsSupportedRequest);
registerType("strims.video.v1.HLSEgressIsSupportedResponse", HLSEgressIsSupportedResponse);
registerType("strims.video.v1.HLSEgressOpenStreamRequest", HLSEgressOpenStreamRequest);
registerType("strims.video.v1.HLSEgressOpenStreamResponse", HLSEgressOpenStreamResponse);
registerType("strims.video.v1.HLSEgressCloseStreamRequest", HLSEgressCloseStreamRequest);
registerType("strims.video.v1.HLSEgressCloseStreamResponse", HLSEgressCloseStreamResponse);

export class HLSEgressClient {
  constructor(private readonly host: RPCHost) {}

  public isSupported(arg: IHLSEgressIsSupportedRequest = new HLSEgressIsSupportedRequest()): Promise<HLSEgressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.IsSupported", new HLSEgressIsSupportedRequest(arg)));
  }

  public openStream(arg: IHLSEgressOpenStreamRequest = new HLSEgressOpenStreamRequest()): Promise<HLSEgressOpenStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.OpenStream", new HLSEgressOpenStreamRequest(arg)));
  }

  public closeStream(arg: IHLSEgressCloseStreamRequest = new HLSEgressCloseStreamRequest()): Promise<HLSEgressCloseStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.CloseStream", new HLSEgressCloseStreamRequest(arg)));
  }
}

