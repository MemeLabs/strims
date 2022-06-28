import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_video_v1_IHLSEgressIsSupportedRequest,
  strims_video_v1_HLSEgressIsSupportedRequest,
  strims_video_v1_HLSEgressIsSupportedResponse,
  strims_video_v1_IHLSEgressGetConfigRequest,
  strims_video_v1_HLSEgressGetConfigRequest,
  strims_video_v1_HLSEgressGetConfigResponse,
  strims_video_v1_IHLSEgressSetConfigRequest,
  strims_video_v1_HLSEgressSetConfigRequest,
  strims_video_v1_HLSEgressSetConfigResponse,
  strims_video_v1_IHLSEgressOpenStreamRequest,
  strims_video_v1_HLSEgressOpenStreamRequest,
  strims_video_v1_HLSEgressOpenStreamResponse,
  strims_video_v1_IHLSEgressCloseStreamRequest,
  strims_video_v1_HLSEgressCloseStreamRequest,
  strims_video_v1_HLSEgressCloseStreamResponse,
} from "./hls_egress";

export interface HLSEgressService {
  isSupported(req: strims_video_v1_HLSEgressIsSupportedRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressIsSupportedResponse> | strims_video_v1_HLSEgressIsSupportedResponse;
  getConfig(req: strims_video_v1_HLSEgressGetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressGetConfigResponse> | strims_video_v1_HLSEgressGetConfigResponse;
  setConfig(req: strims_video_v1_HLSEgressSetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressSetConfigResponse> | strims_video_v1_HLSEgressSetConfigResponse;
  openStream(req: strims_video_v1_HLSEgressOpenStreamRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressOpenStreamResponse> | strims_video_v1_HLSEgressOpenStreamResponse;
  closeStream(req: strims_video_v1_HLSEgressCloseStreamRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressCloseStreamResponse> | strims_video_v1_HLSEgressCloseStreamResponse;
}

export class UnimplementedHLSEgressService implements HLSEgressService {
  isSupported(req: strims_video_v1_HLSEgressIsSupportedRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressIsSupportedResponse> | strims_video_v1_HLSEgressIsSupportedResponse { throw new Error("not implemented"); }
  getConfig(req: strims_video_v1_HLSEgressGetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressGetConfigResponse> | strims_video_v1_HLSEgressGetConfigResponse { throw new Error("not implemented"); }
  setConfig(req: strims_video_v1_HLSEgressSetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressSetConfigResponse> | strims_video_v1_HLSEgressSetConfigResponse { throw new Error("not implemented"); }
  openStream(req: strims_video_v1_HLSEgressOpenStreamRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressOpenStreamResponse> | strims_video_v1_HLSEgressOpenStreamResponse { throw new Error("not implemented"); }
  closeStream(req: strims_video_v1_HLSEgressCloseStreamRequest, call: strims_rpc_Call): Promise<strims_video_v1_HLSEgressCloseStreamResponse> | strims_video_v1_HLSEgressCloseStreamResponse { throw new Error("not implemented"); }
}

export const registerHLSEgressService = (host: strims_rpc_Service, service: HLSEgressService): void => {
  host.registerMethod<strims_video_v1_HLSEgressIsSupportedRequest, strims_video_v1_HLSEgressIsSupportedResponse>("strims.video.v1.HLSEgress.IsSupported", service.isSupported.bind(service), strims_video_v1_HLSEgressIsSupportedRequest);
  host.registerMethod<strims_video_v1_HLSEgressGetConfigRequest, strims_video_v1_HLSEgressGetConfigResponse>("strims.video.v1.HLSEgress.GetConfig", service.getConfig.bind(service), strims_video_v1_HLSEgressGetConfigRequest);
  host.registerMethod<strims_video_v1_HLSEgressSetConfigRequest, strims_video_v1_HLSEgressSetConfigResponse>("strims.video.v1.HLSEgress.SetConfig", service.setConfig.bind(service), strims_video_v1_HLSEgressSetConfigRequest);
  host.registerMethod<strims_video_v1_HLSEgressOpenStreamRequest, strims_video_v1_HLSEgressOpenStreamResponse>("strims.video.v1.HLSEgress.OpenStream", service.openStream.bind(service), strims_video_v1_HLSEgressOpenStreamRequest);
  host.registerMethod<strims_video_v1_HLSEgressCloseStreamRequest, strims_video_v1_HLSEgressCloseStreamResponse>("strims.video.v1.HLSEgress.CloseStream", service.closeStream.bind(service), strims_video_v1_HLSEgressCloseStreamRequest);
}

export class HLSEgressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public isSupported(req?: strims_video_v1_IHLSEgressIsSupportedRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_HLSEgressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.IsSupported", new strims_video_v1_HLSEgressIsSupportedRequest(req)), strims_video_v1_HLSEgressIsSupportedResponse, opts);
  }

  public getConfig(req?: strims_video_v1_IHLSEgressGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_HLSEgressGetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.GetConfig", new strims_video_v1_HLSEgressGetConfigRequest(req)), strims_video_v1_HLSEgressGetConfigResponse, opts);
  }

  public setConfig(req?: strims_video_v1_IHLSEgressSetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_HLSEgressSetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.SetConfig", new strims_video_v1_HLSEgressSetConfigRequest(req)), strims_video_v1_HLSEgressSetConfigResponse, opts);
  }

  public openStream(req?: strims_video_v1_IHLSEgressOpenStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_HLSEgressOpenStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.OpenStream", new strims_video_v1_HLSEgressOpenStreamRequest(req)), strims_video_v1_HLSEgressOpenStreamResponse, opts);
  }

  public closeStream(req?: strims_video_v1_IHLSEgressCloseStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_HLSEgressCloseStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.CloseStream", new strims_video_v1_HLSEgressCloseStreamRequest(req)), strims_video_v1_HLSEgressCloseStreamResponse, opts);
  }
}

