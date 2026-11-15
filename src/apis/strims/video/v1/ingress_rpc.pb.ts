// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IVideoIngressIsSupportedRequest as strims_video_v1_IVideoIngressIsSupportedRequest,
  VideoIngressIsSupportedRequest as strims_video_v1_VideoIngressIsSupportedRequest,
  VideoIngressIsSupportedResponse as strims_video_v1_VideoIngressIsSupportedResponse,
  type IVideoIngressGetConfigRequest as strims_video_v1_IVideoIngressGetConfigRequest,
  VideoIngressGetConfigRequest as strims_video_v1_VideoIngressGetConfigRequest,
  VideoIngressGetConfigResponse as strims_video_v1_VideoIngressGetConfigResponse,
  type IVideoIngressSetConfigRequest as strims_video_v1_IVideoIngressSetConfigRequest,
  VideoIngressSetConfigRequest as strims_video_v1_VideoIngressSetConfigRequest,
  VideoIngressSetConfigResponse as strims_video_v1_VideoIngressSetConfigResponse,
  type IVideoIngressListStreamsRequest as strims_video_v1_IVideoIngressListStreamsRequest,
  VideoIngressListStreamsRequest as strims_video_v1_VideoIngressListStreamsRequest,
  VideoIngressListStreamsResponse as strims_video_v1_VideoIngressListStreamsResponse,
  type IVideoIngressGetChannelURLRequest as strims_video_v1_IVideoIngressGetChannelURLRequest,
  VideoIngressGetChannelURLRequest as strims_video_v1_VideoIngressGetChannelURLRequest,
  VideoIngressGetChannelURLResponse as strims_video_v1_VideoIngressGetChannelURLResponse,
  type IVideoIngressShareCreateChannelRequest as strims_video_v1_IVideoIngressShareCreateChannelRequest,
  VideoIngressShareCreateChannelRequest as strims_video_v1_VideoIngressShareCreateChannelRequest,
  VideoIngressShareCreateChannelResponse as strims_video_v1_VideoIngressShareCreateChannelResponse,
  type IVideoIngressShareUpdateChannelRequest as strims_video_v1_IVideoIngressShareUpdateChannelRequest,
  VideoIngressShareUpdateChannelRequest as strims_video_v1_VideoIngressShareUpdateChannelRequest,
  VideoIngressShareUpdateChannelResponse as strims_video_v1_VideoIngressShareUpdateChannelResponse,
  type IVideoIngressShareDeleteChannelRequest as strims_video_v1_IVideoIngressShareDeleteChannelRequest,
  VideoIngressShareDeleteChannelRequest as strims_video_v1_VideoIngressShareDeleteChannelRequest,
  VideoIngressShareDeleteChannelResponse as strims_video_v1_VideoIngressShareDeleteChannelResponse,
} from "./ingress.pb";

export interface VideoIngressService {
  isSupported(req: strims_video_v1_VideoIngressIsSupportedRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressIsSupportedResponse> | strims_video_v1_VideoIngressIsSupportedResponse;
  getConfig(req: strims_video_v1_VideoIngressGetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressGetConfigResponse> | strims_video_v1_VideoIngressGetConfigResponse;
  setConfig(req: strims_video_v1_VideoIngressSetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressSetConfigResponse> | strims_video_v1_VideoIngressSetConfigResponse;
  listStreams(req: strims_video_v1_VideoIngressListStreamsRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressListStreamsResponse> | strims_video_v1_VideoIngressListStreamsResponse;
  getChannelURL(req: strims_video_v1_VideoIngressGetChannelURLRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressGetChannelURLResponse> | strims_video_v1_VideoIngressGetChannelURLResponse;
}

export class UnimplementedVideoIngressService implements VideoIngressService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  isSupported(req: strims_video_v1_VideoIngressIsSupportedRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressIsSupportedResponse> | strims_video_v1_VideoIngressIsSupportedResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getConfig(req: strims_video_v1_VideoIngressGetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressGetConfigResponse> | strims_video_v1_VideoIngressGetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setConfig(req: strims_video_v1_VideoIngressSetConfigRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressSetConfigResponse> | strims_video_v1_VideoIngressSetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listStreams(req: strims_video_v1_VideoIngressListStreamsRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressListStreamsResponse> | strims_video_v1_VideoIngressListStreamsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getChannelURL(req: strims_video_v1_VideoIngressGetChannelURLRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressGetChannelURLResponse> | strims_video_v1_VideoIngressGetChannelURLResponse { throw new Error("not implemented"); }
}

export const registerVideoIngressService = (host: strims_rpc_Service, service: VideoIngressService): void => {
  host.registerMethod<strims_video_v1_VideoIngressIsSupportedRequest, strims_video_v1_VideoIngressIsSupportedResponse>("strims.video.v1.VideoIngress.IsSupported", (req, call) => service.isSupported(req, call), strims_video_v1_VideoIngressIsSupportedRequest);
  host.registerMethod<strims_video_v1_VideoIngressGetConfigRequest, strims_video_v1_VideoIngressGetConfigResponse>("strims.video.v1.VideoIngress.GetConfig", (req, call) => service.getConfig(req, call), strims_video_v1_VideoIngressGetConfigRequest);
  host.registerMethod<strims_video_v1_VideoIngressSetConfigRequest, strims_video_v1_VideoIngressSetConfigResponse>("strims.video.v1.VideoIngress.SetConfig", (req, call) => service.setConfig(req, call), strims_video_v1_VideoIngressSetConfigRequest);
  host.registerMethod<strims_video_v1_VideoIngressListStreamsRequest, strims_video_v1_VideoIngressListStreamsResponse>("strims.video.v1.VideoIngress.ListStreams", (req, call) => service.listStreams(req, call), strims_video_v1_VideoIngressListStreamsRequest);
  host.registerMethod<strims_video_v1_VideoIngressGetChannelURLRequest, strims_video_v1_VideoIngressGetChannelURLResponse>("strims.video.v1.VideoIngress.GetChannelURL", (req, call) => service.getChannelURL(req, call), strims_video_v1_VideoIngressGetChannelURLRequest);
}

export class VideoIngressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public isSupported(req?: strims_video_v1_IVideoIngressIsSupportedRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressIsSupportedResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.IsSupported", new strims_video_v1_VideoIngressIsSupportedRequest(req)), strims_video_v1_VideoIngressIsSupportedResponse, opts);
  }

  public getConfig(req?: strims_video_v1_IVideoIngressGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressGetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetConfig", new strims_video_v1_VideoIngressGetConfigRequest(req)), strims_video_v1_VideoIngressGetConfigResponse, opts);
  }

  public setConfig(req?: strims_video_v1_IVideoIngressSetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressSetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.SetConfig", new strims_video_v1_VideoIngressSetConfigRequest(req)), strims_video_v1_VideoIngressSetConfigResponse, opts);
  }

  public listStreams(req?: strims_video_v1_IVideoIngressListStreamsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressListStreamsResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.ListStreams", new strims_video_v1_VideoIngressListStreamsRequest(req)), strims_video_v1_VideoIngressListStreamsResponse, opts);
  }

  public getChannelURL(req?: strims_video_v1_IVideoIngressGetChannelURLRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressGetChannelURLResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngress.GetChannelURL", new strims_video_v1_VideoIngressGetChannelURLRequest(req)), strims_video_v1_VideoIngressGetChannelURLResponse, opts);
  }
}

export interface VideoIngressShareService {
  createChannel(req: strims_video_v1_VideoIngressShareCreateChannelRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressShareCreateChannelResponse> | strims_video_v1_VideoIngressShareCreateChannelResponse;
  updateChannel(req: strims_video_v1_VideoIngressShareUpdateChannelRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressShareUpdateChannelResponse> | strims_video_v1_VideoIngressShareUpdateChannelResponse;
  deleteChannel(req: strims_video_v1_VideoIngressShareDeleteChannelRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressShareDeleteChannelResponse> | strims_video_v1_VideoIngressShareDeleteChannelResponse;
}

export class UnimplementedVideoIngressShareService implements VideoIngressShareService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createChannel(req: strims_video_v1_VideoIngressShareCreateChannelRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressShareCreateChannelResponse> | strims_video_v1_VideoIngressShareCreateChannelResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateChannel(req: strims_video_v1_VideoIngressShareUpdateChannelRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressShareUpdateChannelResponse> | strims_video_v1_VideoIngressShareUpdateChannelResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteChannel(req: strims_video_v1_VideoIngressShareDeleteChannelRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoIngressShareDeleteChannelResponse> | strims_video_v1_VideoIngressShareDeleteChannelResponse { throw new Error("not implemented"); }
}

export const registerVideoIngressShareService = (host: strims_rpc_Service, service: VideoIngressShareService): void => {
  host.registerMethod<strims_video_v1_VideoIngressShareCreateChannelRequest, strims_video_v1_VideoIngressShareCreateChannelResponse>("strims.video.v1.VideoIngressShare.CreateChannel", (req, call) => service.createChannel(req, call), strims_video_v1_VideoIngressShareCreateChannelRequest);
  host.registerMethod<strims_video_v1_VideoIngressShareUpdateChannelRequest, strims_video_v1_VideoIngressShareUpdateChannelResponse>("strims.video.v1.VideoIngressShare.UpdateChannel", (req, call) => service.updateChannel(req, call), strims_video_v1_VideoIngressShareUpdateChannelRequest);
  host.registerMethod<strims_video_v1_VideoIngressShareDeleteChannelRequest, strims_video_v1_VideoIngressShareDeleteChannelResponse>("strims.video.v1.VideoIngressShare.DeleteChannel", (req, call) => service.deleteChannel(req, call), strims_video_v1_VideoIngressShareDeleteChannelRequest);
}

export class VideoIngressShareClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createChannel(req?: strims_video_v1_IVideoIngressShareCreateChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressShareCreateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.CreateChannel", new strims_video_v1_VideoIngressShareCreateChannelRequest(req)), strims_video_v1_VideoIngressShareCreateChannelResponse, opts);
  }

  public updateChannel(req?: strims_video_v1_IVideoIngressShareUpdateChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressShareUpdateChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.UpdateChannel", new strims_video_v1_VideoIngressShareUpdateChannelRequest(req)), strims_video_v1_VideoIngressShareUpdateChannelResponse, opts);
  }

  public deleteChannel(req?: strims_video_v1_IVideoIngressShareDeleteChannelRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoIngressShareDeleteChannelResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoIngressShare.DeleteChannel", new strims_video_v1_VideoIngressShareDeleteChannelRequest(req)), strims_video_v1_VideoIngressShareDeleteChannelResponse, opts);
  }
}

