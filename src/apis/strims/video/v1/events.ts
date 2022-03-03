import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  VideoIngressConfig as strims_video_v1_VideoIngressConfig,
  IVideoIngressConfig as strims_video_v1_IVideoIngressConfig,
} from "./ingress";
import {
  VideoChannel as strims_video_v1_VideoChannel,
  IVideoChannel as strims_video_v1_IVideoChannel,
} from "./channel";

export type IVideoIngressConfigChangeEvent = {
  ingressConfig?: strims_video_v1_IVideoIngressConfig;
}

export class VideoIngressConfigChangeEvent {
  ingressConfig: strims_video_v1_VideoIngressConfig | undefined;

  constructor(v?: IVideoIngressConfigChangeEvent) {
    this.ingressConfig = v?.ingressConfig && new strims_video_v1_VideoIngressConfig(v.ingressConfig);
  }

  static encode(m: VideoIngressConfigChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.ingressConfig) strims_video_v1_VideoIngressConfig.encode(m.ingressConfig, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressConfigChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressConfigChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.ingressConfig = strims_video_v1_VideoIngressConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoChannelChangeEvent = {
  videoChannel?: strims_video_v1_IVideoChannel;
}

export class VideoChannelChangeEvent {
  videoChannel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoChannelChangeEvent) {
    this.videoChannel = v?.videoChannel && new strims_video_v1_VideoChannel(v.videoChannel);
  }

  static encode(m: VideoChannelChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.videoChannel) strims_video_v1_VideoChannel.encode(m.videoChannel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.videoChannel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoChannelDeleteEvent = {
  videoChannel?: strims_video_v1_IVideoChannel;
}

export class VideoChannelDeleteEvent {
  videoChannel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoChannelDeleteEvent) {
    this.videoChannel = v?.videoChannel && new strims_video_v1_VideoChannel(v.videoChannel);
  }

  static encode(m: VideoChannelDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.videoChannel) strims_video_v1_VideoChannel.encode(m.videoChannel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.videoChannel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

