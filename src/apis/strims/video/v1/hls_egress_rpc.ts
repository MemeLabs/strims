import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IHLSEgressIsSupportedRequest,
  HLSEgressIsSupportedRequest,
  HLSEgressIsSupportedResponse,
  IHLSEgressGetConfigRequest,
  HLSEgressGetConfigRequest,
  HLSEgressGetConfigResponse,
  IHLSEgressSetConfigRequest,
  HLSEgressSetConfigRequest,
  HLSEgressSetConfigResponse,
  IHLSEgressOpenStreamRequest,
  HLSEgressOpenStreamRequest,
  HLSEgressOpenStreamResponse,
  IHLSEgressCloseStreamRequest,
  HLSEgressCloseStreamRequest,
  HLSEgressCloseStreamResponse,
} from "./hls_egress";

export interface HLSEgressService {
  isSupported(req: HLSEgressIsSupportedRequest, call: strims_rpc_Call): Promise<HLSEgressIsSupportedResponse> | HLSEgressIsSupportedResponse;
  getConfig(req: HLSEgressGetConfigRequest, call: strims_rpc_Call): Promise<HLSEgressGetConfigResponse> | HLSEgressGetConfigResponse;
  setConfig(req: HLSEgressSetConfigRequest, call: strims_rpc_Call): Promise<HLSEgressSetConfigResponse> | HLSEgressSetConfigResponse;
  openStream(req: HLSEgressOpenStreamRequest, call: strims_rpc_Call): Promise<HLSEgressOpenStreamResponse> | HLSEgressOpenStreamResponse;
  closeStream(req: HLSEgressCloseStreamRequest, call: strims_rpc_Call): Promise<HLSEgressCloseStreamResponse> | HLSEgressCloseStreamResponse;
}

export class UnimplementedHLSEgressService implements HLSEgressService {
  isSupported(req: HLSEgressIsSupportedRequest, call: strims_rpc_Call): Promise<HLSEgressIsSupportedResponse> | HLSEgressIsSupportedResponse { throw new Error("not implemented"); }
  getConfig(req: HLSEgressGetConfigRequest, call: strims_rpc_Call): Promise<HLSEgressGetConfigResponse> | HLSEgressGetConfigResponse { throw new Error("not implemented"); }
  setConfig(req: HLSEgressSetConfigRequest, call: strims_rpc_Call): Promise<HLSEgressSetConfigResponse> | HLSEgressSetConfigResponse { throw new Error("not implemented"); }
  openStream(req: HLSEgressOpenStreamRequest, call: strims_rpc_Call): Promise<HLSEgressOpenStreamResponse> | HLSEgressOpenStreamResponse { throw new Error("not implemented"); }
  closeStream(req: HLSEgressCloseStreamRequest, call: strims_rpc_Call): Promise<HLSEgressCloseStreamResponse> | HLSEgressCloseStreamResponse { throw new Error("not implemented"); }
}

export const registerHLSEgressService = (host: strims_rpc_Service, service: HLSEgressService): void => {
  host.registerMethod<HLSEgressIsSupportedRequest, HLSEgressIsSupportedResponse>("strims.video.v1.HLSEgress.IsSupported", service.isSupported.bind(service), HLSEgressIsSupportedRequest);
  host.registerMethod<HLSEgressGetConfigRequest, HLSEgressGetConfigResponse>("strims.video.v1.HLSEgress.GetConfig", service.getConfig.bind(service), HLSEgressGetConfigRequest);
  host.registerMethod<HLSEgressSetConfigRequest, HLSEgressSetConfigResponse>("strims.video.v1.HLSEgress.SetConfig", service.setConfig.bind(service), HLSEgressSetConfigRequest);
  host.registerMethod<HLSEgressOpenStreamRequest, HLSEgressOpenStreamResponse>("strims.video.v1.HLSEgress.OpenStream", service.openStream.bind(service), HLSEgressOpenStreamRequest);
  host.registerMethod<HLSEgressCloseStreamRequest, HLSEgressCloseStreamResponse>("strims.video.v1.HLSEgress.CloseStream", service.closeStream.bind(service), HLSEgressCloseStreamRequest);
}

export class HLSEgressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public isSupported(req?: IHLSEgressIsSupportedRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.IsSupported", new HLSEgressIsSupportedRequest(req)), HLSEgressIsSupportedResponse, opts);
  }

  public getConfig(req?: IHLSEgressGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressGetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.GetConfig", new HLSEgressGetConfigRequest(req)), HLSEgressGetConfigResponse, opts);
  }

  public setConfig(req?: IHLSEgressSetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressSetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.SetConfig", new HLSEgressSetConfigRequest(req)), HLSEgressSetConfigResponse, opts);
  }

  public openStream(req?: IHLSEgressOpenStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressOpenStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.OpenStream", new HLSEgressOpenStreamRequest(req)), HLSEgressOpenStreamResponse, opts);
  }

  public closeStream(req?: IHLSEgressCloseStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HLSEgressCloseStreamResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.HLSEgress.CloseStream", new HLSEgressCloseStreamRequest(req)), HLSEgressCloseStreamResponse, opts);
  }
}

