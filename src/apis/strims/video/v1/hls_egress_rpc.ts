import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

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

export interface HLSEgressService {
  isSupported(req: HLSEgressIsSupportedRequest, call: strims_rpc_Call): Promise<HLSEgressIsSupportedResponse> | HLSEgressIsSupportedResponse;
  openStream(req: HLSEgressOpenStreamRequest, call: strims_rpc_Call): Promise<HLSEgressOpenStreamResponse> | HLSEgressOpenStreamResponse;
  closeStream(req: HLSEgressCloseStreamRequest, call: strims_rpc_Call): Promise<HLSEgressCloseStreamResponse> | HLSEgressCloseStreamResponse;
}

export const registerHLSEgressService = (host: strims_rpc_Service, service: HLSEgressService): void => {
  host.registerMethod<HLSEgressIsSupportedRequest, HLSEgressIsSupportedResponse>("strims.video.v1.HLSEgress.IsSupported", service.isSupported.bind(service), HLSEgressIsSupportedRequest);
  host.registerMethod<HLSEgressOpenStreamRequest, HLSEgressOpenStreamResponse>("strims.video.v1.HLSEgress.OpenStream", service.openStream.bind(service), HLSEgressOpenStreamRequest);
  host.registerMethod<HLSEgressCloseStreamRequest, HLSEgressCloseStreamResponse>("strims.video.v1.HLSEgress.CloseStream", service.closeStream.bind(service), HLSEgressCloseStreamRequest);
}

export class HLSEgressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public isSupported(req?: IHLSEgressIsSupportedRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.IsSupported", new HLSEgressIsSupportedRequest(req)), HLSEgressIsSupportedResponse, opts);
  }

  public openStream(req?: IHLSEgressOpenStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressOpenStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.OpenStream", new HLSEgressOpenStreamRequest(req)), HLSEgressOpenStreamResponse, opts);
  }

  public closeStream(req?: IHLSEgressCloseStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressCloseStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.CloseStream", new HLSEgressCloseStreamRequest(req)), HLSEgressCloseStreamResponse, opts);
  }
}

