import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

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

export class VideoIngressClient {
  constructor(private readonly host: RPCHost) {}

  public isSupported(arg?: IVideoIngressIsSupportedRequest): Promise<VideoIngressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.IsSupported", new VideoIngressIsSupportedRequest(arg)));
  }

  public getConfig(arg?: IVideoIngressGetConfigRequest): Promise<VideoIngressGetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetConfig", new VideoIngressGetConfigRequest(arg)));
  }

  public setConfig(arg?: IVideoIngressSetConfigRequest): Promise<VideoIngressSetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.SetConfig", new VideoIngressSetConfigRequest(arg)));
  }

  public listStreams(arg?: IVideoIngressListStreamsRequest): Promise<VideoIngressListStreamsResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.ListStreams", new VideoIngressListStreamsRequest(arg)));
  }

  public getChannelURL(arg?: IVideoIngressGetChannelURLRequest): Promise<VideoIngressGetChannelURLResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetChannelURL", new VideoIngressGetChannelURLRequest(arg)));
  }
}

export class VideoIngressShareClient {
  constructor(private readonly host: RPCHost) {}

  public createChannel(arg?: IVideoIngressShareCreateChannelRequest): Promise<VideoIngressShareCreateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.CreateChannel", new VideoIngressShareCreateChannelRequest(arg)));
  }

  public updateChannel(arg?: IVideoIngressShareUpdateChannelRequest): Promise<VideoIngressShareUpdateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.UpdateChannel", new VideoIngressShareUpdateChannelRequest(arg)));
  }

  public deleteChannel(arg?: IVideoIngressShareDeleteChannelRequest): Promise<VideoIngressShareDeleteChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.DeleteChannel", new VideoIngressShareDeleteChannelRequest(arg)));
  }
}

