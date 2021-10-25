import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
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

export interface VideoIngressService {
  isSupported(req: VideoIngressIsSupportedRequest, call: strims_rpc_Call): Promise<VideoIngressIsSupportedResponse> | VideoIngressIsSupportedResponse;
  getConfig(req: VideoIngressGetConfigRequest, call: strims_rpc_Call): Promise<VideoIngressGetConfigResponse> | VideoIngressGetConfigResponse;
  setConfig(req: VideoIngressSetConfigRequest, call: strims_rpc_Call): Promise<VideoIngressSetConfigResponse> | VideoIngressSetConfigResponse;
  listStreams(req: VideoIngressListStreamsRequest, call: strims_rpc_Call): Promise<VideoIngressListStreamsResponse> | VideoIngressListStreamsResponse;
  getChannelURL(req: VideoIngressGetChannelURLRequest, call: strims_rpc_Call): Promise<VideoIngressGetChannelURLResponse> | VideoIngressGetChannelURLResponse;
}

export const registerVideoIngressService = (host: strims_rpc_Service, service: VideoIngressService): void => {
  host.registerMethod<VideoIngressIsSupportedRequest, VideoIngressIsSupportedResponse>("strims.video.v1.VideoIngress.IsSupported", service.isSupported.bind(service), VideoIngressIsSupportedRequest);
  host.registerMethod<VideoIngressGetConfigRequest, VideoIngressGetConfigResponse>("strims.video.v1.VideoIngress.GetConfig", service.getConfig.bind(service), VideoIngressGetConfigRequest);
  host.registerMethod<VideoIngressSetConfigRequest, VideoIngressSetConfigResponse>("strims.video.v1.VideoIngress.SetConfig", service.setConfig.bind(service), VideoIngressSetConfigRequest);
  host.registerMethod<VideoIngressListStreamsRequest, VideoIngressListStreamsResponse>("strims.video.v1.VideoIngress.ListStreams", service.listStreams.bind(service), VideoIngressListStreamsRequest);
  host.registerMethod<VideoIngressGetChannelURLRequest, VideoIngressGetChannelURLResponse>("strims.video.v1.VideoIngress.GetChannelURL", service.getChannelURL.bind(service), VideoIngressGetChannelURLRequest);
}

export class VideoIngressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public isSupported(req?: IVideoIngressIsSupportedRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.IsSupported", new VideoIngressIsSupportedRequest(req)), VideoIngressIsSupportedResponse, opts);
  }

  public getConfig(req?: IVideoIngressGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressGetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetConfig", new VideoIngressGetConfigRequest(req)), VideoIngressGetConfigResponse, opts);
  }

  public setConfig(req?: IVideoIngressSetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressSetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.SetConfig", new VideoIngressSetConfigRequest(req)), VideoIngressSetConfigResponse, opts);
  }

  public listStreams(req?: IVideoIngressListStreamsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressListStreamsResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.ListStreams", new VideoIngressListStreamsRequest(req)), VideoIngressListStreamsResponse, opts);
  }

  public getChannelURL(req?: IVideoIngressGetChannelURLRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressGetChannelURLResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetChannelURL", new VideoIngressGetChannelURLRequest(req)), VideoIngressGetChannelURLResponse, opts);
  }
}

export interface VideoIngressShareService {
  createChannel(req: VideoIngressShareCreateChannelRequest, call: strims_rpc_Call): Promise<VideoIngressShareCreateChannelResponse> | VideoIngressShareCreateChannelResponse;
  updateChannel(req: VideoIngressShareUpdateChannelRequest, call: strims_rpc_Call): Promise<VideoIngressShareUpdateChannelResponse> | VideoIngressShareUpdateChannelResponse;
  deleteChannel(req: VideoIngressShareDeleteChannelRequest, call: strims_rpc_Call): Promise<VideoIngressShareDeleteChannelResponse> | VideoIngressShareDeleteChannelResponse;
}

export const registerVideoIngressShareService = (host: strims_rpc_Service, service: VideoIngressShareService): void => {
  host.registerMethod<VideoIngressShareCreateChannelRequest, VideoIngressShareCreateChannelResponse>("strims.video.v1.VideoIngressShare.CreateChannel", service.createChannel.bind(service), VideoIngressShareCreateChannelRequest);
  host.registerMethod<VideoIngressShareUpdateChannelRequest, VideoIngressShareUpdateChannelResponse>("strims.video.v1.VideoIngressShare.UpdateChannel", service.updateChannel.bind(service), VideoIngressShareUpdateChannelRequest);
  host.registerMethod<VideoIngressShareDeleteChannelRequest, VideoIngressShareDeleteChannelResponse>("strims.video.v1.VideoIngressShare.DeleteChannel", service.deleteChannel.bind(service), VideoIngressShareDeleteChannelRequest);
}

export class VideoIngressShareClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createChannel(req?: IVideoIngressShareCreateChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressShareCreateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.CreateChannel", new VideoIngressShareCreateChannelRequest(req)), VideoIngressShareCreateChannelResponse, opts);
  }

  public updateChannel(req?: IVideoIngressShareUpdateChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressShareUpdateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.UpdateChannel", new VideoIngressShareUpdateChannelRequest(req)), VideoIngressShareUpdateChannelResponse, opts);
  }

  public deleteChannel(req?: IVideoIngressShareDeleteChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoIngressShareDeleteChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.DeleteChannel", new VideoIngressShareDeleteChannelRequest(req)), VideoIngressShareDeleteChannelResponse, opts);
  }
}

