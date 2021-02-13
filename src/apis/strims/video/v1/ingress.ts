import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  VideoChannel as strims_video_v1_VideoChannel,
  IVideoChannel as strims_video_v1_IVideoChannel,
} from "./channel";
import {
  DirectoryListingSnippet as strims_network_v1_DirectoryListingSnippet,
  IDirectoryListingSnippet as strims_network_v1_IDirectoryListingSnippet,
} from "../../network/v1/directory";

export type IVideoIngressConfig = {
  enabled?: boolean;
  serverAddr?: string;
  publicServerAddr?: string;
  serviceNetworkKeys?: Uint8Array[];
}

export class VideoIngressConfig {
  enabled: boolean;
  serverAddr: string;
  publicServerAddr: string;
  serviceNetworkKeys: Uint8Array[];

  constructor(v?: IVideoIngressConfig) {
    this.enabled = v?.enabled || false;
    this.serverAddr = v?.serverAddr || "";
    this.publicServerAddr = v?.publicServerAddr || "";
    this.serviceNetworkKeys = v?.serviceNetworkKeys ? v.serviceNetworkKeys : [];
  }

  static encode(m: VideoIngressConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enabled) w.uint32(8).bool(m.enabled);
    if (m.serverAddr) w.uint32(18).string(m.serverAddr);
    if (m.publicServerAddr) w.uint32(26).string(m.publicServerAddr);
    for (const v of m.serviceNetworkKeys) w.uint32(34).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enabled = r.bool();
        break;
        case 2:
        m.serverAddr = r.string();
        break;
        case 3:
        m.publicServerAddr = r.string();
        break;
        case 4:
        m.serviceNetworkKeys.push(r.bytes())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressStream = {
  id?: bigint;
  channelId?: bigint;
  createdAt?: bigint;
  updatedAt?: bigint;
}

export class VideoIngressStream {
  id: bigint;
  channelId: bigint;
  createdAt: bigint;
  updatedAt: bigint;

  constructor(v?: IVideoIngressStream) {
    this.id = v?.id || BigInt(0);
    this.channelId = v?.channelId || BigInt(0);
    this.createdAt = v?.createdAt || BigInt(0);
    this.updatedAt = v?.updatedAt || BigInt(0);
  }

  static encode(m: VideoIngressStream, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.channelId) w.uint32(16).uint64(m.channelId);
    if (m.createdAt) w.uint32(24).int64(m.createdAt);
    if (m.updatedAt) w.uint32(32).int64(m.updatedAt);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressStream {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressStream();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.channelId = r.uint64();
        break;
        case 3:
        m.createdAt = r.int64();
        break;
        case 4:
        m.updatedAt = r.int64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressIsSupportedRequest = {
}

export class VideoIngressIsSupportedRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoIngressIsSupportedRequest) {
  }

  static encode(m: VideoIngressIsSupportedRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressIsSupportedRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoIngressIsSupportedRequest();
  }
}

export type IVideoIngressIsSupportedResponse = {
  supported?: boolean;
}

export class VideoIngressIsSupportedResponse {
  supported: boolean;

  constructor(v?: IVideoIngressIsSupportedResponse) {
    this.supported = v?.supported || false;
  }

  static encode(m: VideoIngressIsSupportedResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.supported) w.uint32(8).bool(m.supported);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressIsSupportedResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressIsSupportedResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.supported = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressGetConfigRequest = {
}

export class VideoIngressGetConfigRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoIngressGetConfigRequest) {
  }

  static encode(m: VideoIngressGetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressGetConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoIngressGetConfigRequest();
  }
}

export type IVideoIngressGetConfigResponse = {
  config?: IVideoIngressConfig | undefined;
}

export class VideoIngressGetConfigResponse {
  config: VideoIngressConfig | undefined;

  constructor(v?: IVideoIngressGetConfigResponse) {
    this.config = v?.config && new VideoIngressConfig(v.config);
  }

  static encode(m: VideoIngressGetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) VideoIngressConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressGetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressGetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = VideoIngressConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressSetConfigRequest = {
  config?: IVideoIngressConfig | undefined;
}

export class VideoIngressSetConfigRequest {
  config: VideoIngressConfig | undefined;

  constructor(v?: IVideoIngressSetConfigRequest) {
    this.config = v?.config && new VideoIngressConfig(v.config);
  }

  static encode(m: VideoIngressSetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) VideoIngressConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressSetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressSetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = VideoIngressConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressSetConfigResponse = {
  config?: IVideoIngressConfig | undefined;
}

export class VideoIngressSetConfigResponse {
  config: VideoIngressConfig | undefined;

  constructor(v?: IVideoIngressSetConfigResponse) {
    this.config = v?.config && new VideoIngressConfig(v.config);
  }

  static encode(m: VideoIngressSetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) VideoIngressConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressSetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressSetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = VideoIngressConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressListStreamsRequest = {
}

export class VideoIngressListStreamsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoIngressListStreamsRequest) {
  }

  static encode(m: VideoIngressListStreamsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressListStreamsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoIngressListStreamsRequest();
  }
}

export type IVideoIngressListStreamsResponse = {
  streams?: IVideoIngressStream[];
}

export class VideoIngressListStreamsResponse {
  streams: VideoIngressStream[];

  constructor(v?: IVideoIngressListStreamsResponse) {
    this.streams = v?.streams ? v.streams.map(v => new VideoIngressStream(v)) : [];
  }

  static encode(m: VideoIngressListStreamsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.streams) VideoIngressStream.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressListStreamsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressListStreamsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.streams.push(VideoIngressStream.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressGetChannelURLRequest = {
  id?: bigint;
}

export class VideoIngressGetChannelURLRequest {
  id: bigint;

  constructor(v?: IVideoIngressGetChannelURLRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: VideoIngressGetChannelURLRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressGetChannelURLRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressGetChannelURLRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressGetChannelURLResponse = {
  url?: string;
  serverAddr?: string;
  streamKey?: string;
}

export class VideoIngressGetChannelURLResponse {
  url: string;
  serverAddr: string;
  streamKey: string;

  constructor(v?: IVideoIngressGetChannelURLResponse) {
    this.url = v?.url || "";
    this.serverAddr = v?.serverAddr || "";
    this.streamKey = v?.streamKey || "";
  }

  static encode(m: VideoIngressGetChannelURLResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.url) w.uint32(10).string(m.url);
    if (m.serverAddr) w.uint32(18).string(m.serverAddr);
    if (m.streamKey) w.uint32(26).string(m.streamKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressGetChannelURLResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressGetChannelURLResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.url = r.string();
        break;
        case 2:
        m.serverAddr = r.string();
        break;
        case 3:
        m.streamKey = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressShareCreateChannelRequest = {
  directoryListingSnippet?: strims_network_v1_IDirectoryListingSnippet | undefined;
}

export class VideoIngressShareCreateChannelRequest {
  directoryListingSnippet: strims_network_v1_DirectoryListingSnippet | undefined;

  constructor(v?: IVideoIngressShareCreateChannelRequest) {
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_DirectoryListingSnippet(v.directoryListingSnippet);
  }

  static encode(m: VideoIngressShareCreateChannelRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directoryListingSnippet) strims_network_v1_DirectoryListingSnippet.encode(m.directoryListingSnippet, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressShareCreateChannelRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressShareCreateChannelRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.directoryListingSnippet = strims_network_v1_DirectoryListingSnippet.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressShareCreateChannelResponse = {
  channel?: strims_video_v1_IVideoChannel | undefined;
}

export class VideoIngressShareCreateChannelResponse {
  channel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoIngressShareCreateChannelResponse) {
    this.channel = v?.channel && new strims_video_v1_VideoChannel(v.channel);
  }

  static encode(m: VideoIngressShareCreateChannelResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) strims_video_v1_VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressShareCreateChannelResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressShareCreateChannelResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressShareUpdateChannelRequest = {
  directoryListingSnippet?: strims_network_v1_IDirectoryListingSnippet | undefined;
}

export class VideoIngressShareUpdateChannelRequest {
  directoryListingSnippet: strims_network_v1_DirectoryListingSnippet | undefined;

  constructor(v?: IVideoIngressShareUpdateChannelRequest) {
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_DirectoryListingSnippet(v.directoryListingSnippet);
  }

  static encode(m: VideoIngressShareUpdateChannelRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directoryListingSnippet) strims_network_v1_DirectoryListingSnippet.encode(m.directoryListingSnippet, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressShareUpdateChannelRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressShareUpdateChannelRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.directoryListingSnippet = strims_network_v1_DirectoryListingSnippet.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressShareUpdateChannelResponse = {
  channel?: strims_video_v1_IVideoChannel | undefined;
}

export class VideoIngressShareUpdateChannelResponse {
  channel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoIngressShareUpdateChannelResponse) {
    this.channel = v?.channel && new strims_video_v1_VideoChannel(v.channel);
  }

  static encode(m: VideoIngressShareUpdateChannelResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) strims_video_v1_VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressShareUpdateChannelResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoIngressShareUpdateChannelResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoIngressShareDeleteChannelRequest = {
}

export class VideoIngressShareDeleteChannelRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoIngressShareDeleteChannelRequest) {
  }

  static encode(m: VideoIngressShareDeleteChannelRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressShareDeleteChannelRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoIngressShareDeleteChannelRequest();
  }
}

export type IVideoIngressShareDeleteChannelResponse = {
}

export class VideoIngressShareDeleteChannelResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoIngressShareDeleteChannelResponse) {
  }

  static encode(m: VideoIngressShareDeleteChannelResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoIngressShareDeleteChannelResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoIngressShareDeleteChannelResponse();
  }
}

