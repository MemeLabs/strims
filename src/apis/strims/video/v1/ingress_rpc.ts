import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IVideoIngressIsSupportedRequest,
  VideoIngressIsSupportedRequest,
  VideoIngressIsSupportedResponse,
  IVideoIngressGetConfigRequest,
  VideoIngressGetConfigRequest,
  VideoIngressGetConfigResponse,
  IVideoIngressSetConfigRequest,
  VideoIngressSetConfigRequest,
  VideoIngressSetConfigResponse,
  IVideoIngressListStreamsRequest,
  VideoIngressListStreamsRequest,
  VideoIngressListStreamsResponse,
  IVideoIngressGetChannelURLRequest,
  VideoIngressGetChannelURLRequest,
  VideoIngressGetChannelURLResponse,
  IVideoIngressShareCreateChannelRequest,
  VideoIngressShareCreateChannelRequest,
  VideoIngressShareCreateChannelResponse,
  IVideoIngressShareUpdateChannelRequest,
  VideoIngressShareUpdateChannelRequest,
  VideoIngressShareUpdateChannelResponse,
  IVideoIngressShareDeleteChannelRequest,
  VideoIngressShareDeleteChannelRequest,
  VideoIngressShareDeleteChannelResponse,
} from "./ingress";

registerType("strims.video.v1.VideoIngressIsSupportedRequest", VideoIngressIsSupportedRequest);
registerType("strims.video.v1.VideoIngressIsSupportedResponse", VideoIngressIsSupportedResponse);
registerType("strims.video.v1.VideoIngressGetConfigRequest", VideoIngressGetConfigRequest);
registerType("strims.video.v1.VideoIngressGetConfigResponse", VideoIngressGetConfigResponse);
registerType("strims.video.v1.VideoIngressSetConfigRequest", VideoIngressSetConfigRequest);
registerType("strims.video.v1.VideoIngressSetConfigResponse", VideoIngressSetConfigResponse);
registerType("strims.video.v1.VideoIngressListStreamsRequest", VideoIngressListStreamsRequest);
registerType("strims.video.v1.VideoIngressListStreamsResponse", VideoIngressListStreamsResponse);
registerType("strims.video.v1.VideoIngressGetChannelURLRequest", VideoIngressGetChannelURLRequest);
registerType("strims.video.v1.VideoIngressGetChannelURLResponse", VideoIngressGetChannelURLResponse);
registerType("strims.video.v1.VideoIngressShareCreateChannelRequest", VideoIngressShareCreateChannelRequest);
registerType("strims.video.v1.VideoIngressShareCreateChannelResponse", VideoIngressShareCreateChannelResponse);
registerType("strims.video.v1.VideoIngressShareUpdateChannelRequest", VideoIngressShareUpdateChannelRequest);
registerType("strims.video.v1.VideoIngressShareUpdateChannelResponse", VideoIngressShareUpdateChannelResponse);
registerType("strims.video.v1.VideoIngressShareDeleteChannelRequest", VideoIngressShareDeleteChannelRequest);
registerType("strims.video.v1.VideoIngressShareDeleteChannelResponse", VideoIngressShareDeleteChannelResponse);

export interface VideoIngressService {
  isSupported(req: VideoIngressIsSupportedRequest, call: strims_rpc_Call): Promise<VideoIngressIsSupportedResponse> | VideoIngressIsSupportedResponse;
  getConfig(req: VideoIngressGetConfigRequest, call: strims_rpc_Call): Promise<VideoIngressGetConfigResponse> | VideoIngressGetConfigResponse;
  setConfig(req: VideoIngressSetConfigRequest, call: strims_rpc_Call): Promise<VideoIngressSetConfigResponse> | VideoIngressSetConfigResponse;
  listStreams(req: VideoIngressListStreamsRequest, call: strims_rpc_Call): Promise<VideoIngressListStreamsResponse> | VideoIngressListStreamsResponse;
  getChannelURL(req: VideoIngressGetChannelURLRequest, call: strims_rpc_Call): Promise<VideoIngressGetChannelURLResponse> | VideoIngressGetChannelURLResponse;
}

export const registerVideoIngressService = (host: strims_rpc_Service, service: VideoIngressService): void => {
  host.registerMethod<VideoIngressIsSupportedRequest, VideoIngressIsSupportedResponse>("strims.video.v1.VideoIngress.IsSupported", service.isSupported.bind(service));
  host.registerMethod<VideoIngressGetConfigRequest, VideoIngressGetConfigResponse>("strims.video.v1.VideoIngress.GetConfig", service.getConfig.bind(service));
  host.registerMethod<VideoIngressSetConfigRequest, VideoIngressSetConfigResponse>("strims.video.v1.VideoIngress.SetConfig", service.setConfig.bind(service));
  host.registerMethod<VideoIngressListStreamsRequest, VideoIngressListStreamsResponse>("strims.video.v1.VideoIngress.ListStreams", service.listStreams.bind(service));
  host.registerMethod<VideoIngressGetChannelURLRequest, VideoIngressGetChannelURLResponse>("strims.video.v1.VideoIngress.GetChannelURL", service.getChannelURL.bind(service));
}

export class VideoIngressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public isSupported(req?: IVideoIngressIsSupportedRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.IsSupported", new VideoIngressIsSupportedRequest(req)), opts);
  }

  public getConfig(req?: IVideoIngressGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressGetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetConfig", new VideoIngressGetConfigRequest(req)), opts);
  }

  public setConfig(req?: IVideoIngressSetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressSetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.SetConfig", new VideoIngressSetConfigRequest(req)), opts);
  }

  public listStreams(req?: IVideoIngressListStreamsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressListStreamsResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.ListStreams", new VideoIngressListStreamsRequest(req)), opts);
  }

  public getChannelURL(req?: IVideoIngressGetChannelURLRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressGetChannelURLResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetChannelURL", new VideoIngressGetChannelURLRequest(req)), opts);
  }
}

export interface VideoIngressShareService {
  createChannel(req: VideoIngressShareCreateChannelRequest, call: strims_rpc_Call): Promise<VideoIngressShareCreateChannelResponse> | VideoIngressShareCreateChannelResponse;
  updateChannel(req: VideoIngressShareUpdateChannelRequest, call: strims_rpc_Call): Promise<VideoIngressShareUpdateChannelResponse> | VideoIngressShareUpdateChannelResponse;
  deleteChannel(req: VideoIngressShareDeleteChannelRequest, call: strims_rpc_Call): Promise<VideoIngressShareDeleteChannelResponse> | VideoIngressShareDeleteChannelResponse;
}

export const registerVideoIngressShareService = (host: strims_rpc_Service, service: VideoIngressShareService): void => {
  host.registerMethod<VideoIngressShareCreateChannelRequest, VideoIngressShareCreateChannelResponse>("strims.video.v1.VideoIngressShare.CreateChannel", service.createChannel.bind(service));
  host.registerMethod<VideoIngressShareUpdateChannelRequest, VideoIngressShareUpdateChannelResponse>("strims.video.v1.VideoIngressShare.UpdateChannel", service.updateChannel.bind(service));
  host.registerMethod<VideoIngressShareDeleteChannelRequest, VideoIngressShareDeleteChannelResponse>("strims.video.v1.VideoIngressShare.DeleteChannel", service.deleteChannel.bind(service));
}

export class VideoIngressShareClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createChannel(req?: IVideoIngressShareCreateChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressShareCreateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.CreateChannel", new VideoIngressShareCreateChannelRequest(req)), opts);
  }

  public updateChannel(req?: IVideoIngressShareUpdateChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressShareUpdateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.UpdateChannel", new VideoIngressShareUpdateChannelRequest(req)), opts);
  }

  public deleteChannel(req?: IVideoIngressShareDeleteChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressShareDeleteChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.DeleteChannel", new VideoIngressShareDeleteChannelRequest(req)), opts);
  }
}

