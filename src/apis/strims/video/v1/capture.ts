import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_network_v1_directory_ListingSnippet,
  strims_network_v1_directory_IListingSnippet,
} from "../../network/v1/directory/directory";

export type ICaptureOpenRequest = {
  directorySnippet?: strims_network_v1_directory_IListingSnippet;
  networkKeys?: Uint8Array[];
  bitrate?: bigint;
}

export class CaptureOpenRequest {
  directorySnippet: strims_network_v1_directory_ListingSnippet | undefined;
  networkKeys: Uint8Array[];
  bitrate: bigint;

  constructor(v?: ICaptureOpenRequest) {
    this.directorySnippet = v?.directorySnippet && new strims_network_v1_directory_ListingSnippet(v.directorySnippet);
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
    this.bitrate = v?.bitrate || BigInt(0);
  }

  static encode(m: CaptureOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directorySnippet) strims_network_v1_directory_ListingSnippet.encode(m.directorySnippet, w.uint32(10).fork()).ldelim();
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    if (m.bitrate) w.uint32(24).uint64(m.bitrate);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureOpenRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureOpenRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.directorySnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
        break;
        case 2:
        m.networkKeys.push(r.bytes())
        break;
        case 3:
        m.bitrate = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureOpenResponse = {
  id?: Uint8Array;
}

export class CaptureOpenResponse {
  id: Uint8Array;

  constructor(v?: ICaptureOpenResponse) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: CaptureOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureOpenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureOpenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureUpdateRequest = {
  id?: Uint8Array;
  directorySnippet?: strims_network_v1_directory_IListingSnippet;
}

export class CaptureUpdateRequest {
  id: Uint8Array;
  directorySnippet: strims_network_v1_directory_ListingSnippet | undefined;

  constructor(v?: ICaptureUpdateRequest) {
    this.id = v?.id || new Uint8Array();
    this.directorySnippet = v?.directorySnippet && new strims_network_v1_directory_ListingSnippet(v.directorySnippet);
  }

  static encode(m: CaptureUpdateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    if (m.directorySnippet) strims_network_v1_directory_ListingSnippet.encode(m.directorySnippet, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureUpdateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureUpdateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        case 2:
        m.directorySnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureUpdateResponse = Record<string, any>;

export class CaptureUpdateResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICaptureUpdateResponse) {
  }

  static encode(m: CaptureUpdateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureUpdateResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CaptureUpdateResponse();
  }
}

export type ICaptureConfig = {
  track?: number;
  codec?: strims_video_v1_CaptureConfig_Codec;
  config?: Uint8Array;
}

export class CaptureConfig {
  track: number;
  codec: strims_video_v1_CaptureConfig_Codec;
  config: Uint8Array;

  constructor(v?: ICaptureConfig) {
    this.track = v?.track || 0;
    this.codec = v?.codec || 0;
    this.config = v?.config || new Uint8Array();
  }

  static encode(m: CaptureConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.track) w.uint32(8).uint32(m.track);
    if (m.codec) w.uint32(16).uint32(m.codec);
    if (m.config.length) w.uint32(26).bytes(m.config);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.track = r.uint32();
        break;
        case 2:
        m.codec = r.uint32();
        break;
        case 3:
        m.config = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CaptureConfig {
  export enum Codec {
    CODEC_UNSPECIFIED = 0,
    CODEC_H264 = 1,
    CODEC_OPUS = 2,
  }
}

export type ICaptureUnit = {
  track?: number;
  timestampUs?: bigint;
  keyframe?: boolean;
  data?: Uint8Array;
}

export class CaptureUnit {
  track: number;
  timestampUs: bigint;
  keyframe: boolean;
  data: Uint8Array;

  constructor(v?: ICaptureUnit) {
    this.track = v?.track || 0;
    this.timestampUs = v?.timestampUs || BigInt(0);
    this.keyframe = v?.keyframe || false;
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: CaptureUnit, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.track) w.uint32(8).uint32(m.track);
    if (m.timestampUs) w.uint32(16).int64(m.timestampUs);
    if (m.keyframe) w.uint32(24).bool(m.keyframe);
    if (m.data.length) w.uint32(34).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureUnit {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureUnit();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.track = r.uint32();
        break;
        case 2:
        m.timestampUs = r.int64();
        break;
        case 3:
        m.keyframe = r.bool();
        break;
        case 4:
        m.data = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureAppendRequest = {
  id?: Uint8Array;
  configs?: strims_video_v1_ICaptureConfig[];
  units?: strims_video_v1_ICaptureUnit[];
}

export class CaptureAppendRequest {
  id: Uint8Array;
  configs: strims_video_v1_CaptureConfig[];
  units: strims_video_v1_CaptureUnit[];

  constructor(v?: ICaptureAppendRequest) {
    this.id = v?.id || new Uint8Array();
    this.configs = v?.configs ? v.configs.map(v => new strims_video_v1_CaptureConfig(v)) : [];
    this.units = v?.units ? v.units.map(v => new strims_video_v1_CaptureUnit(v)) : [];
  }

  static encode(m: CaptureAppendRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    for (const v of m.configs) strims_video_v1_CaptureConfig.encode(v, w.uint32(18).fork()).ldelim();
    for (const v of m.units) strims_video_v1_CaptureUnit.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureAppendRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureAppendRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        case 2:
        m.configs.push(strims_video_v1_CaptureConfig.decode(r, r.uint32()));
        break;
        case 3:
        m.units.push(strims_video_v1_CaptureUnit.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureAppendResponse = Record<string, any>;

export class CaptureAppendResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICaptureAppendResponse) {
  }

  static encode(m: CaptureAppendResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureAppendResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CaptureAppendResponse();
  }
}

export type ICaptureCloseRequest = {
  id?: Uint8Array;
}

export class CaptureCloseRequest {
  id: Uint8Array;

  constructor(v?: ICaptureCloseRequest) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: CaptureCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureCloseRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureCloseResponse = Record<string, any>;

export class CaptureCloseResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICaptureCloseResponse) {
  }

  static encode(m: CaptureCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CaptureCloseResponse();
  }
}

/* @internal */
export const strims_video_v1_CaptureOpenRequest = CaptureOpenRequest;
/* @internal */
export type strims_video_v1_CaptureOpenRequest = CaptureOpenRequest;
/* @internal */
export type strims_video_v1_ICaptureOpenRequest = ICaptureOpenRequest;
/* @internal */
export const strims_video_v1_CaptureOpenResponse = CaptureOpenResponse;
/* @internal */
export type strims_video_v1_CaptureOpenResponse = CaptureOpenResponse;
/* @internal */
export type strims_video_v1_ICaptureOpenResponse = ICaptureOpenResponse;
/* @internal */
export const strims_video_v1_CaptureUpdateRequest = CaptureUpdateRequest;
/* @internal */
export type strims_video_v1_CaptureUpdateRequest = CaptureUpdateRequest;
/* @internal */
export type strims_video_v1_ICaptureUpdateRequest = ICaptureUpdateRequest;
/* @internal */
export const strims_video_v1_CaptureUpdateResponse = CaptureUpdateResponse;
/* @internal */
export type strims_video_v1_CaptureUpdateResponse = CaptureUpdateResponse;
/* @internal */
export type strims_video_v1_ICaptureUpdateResponse = ICaptureUpdateResponse;
/* @internal */
export const strims_video_v1_CaptureConfig = CaptureConfig;
/* @internal */
export type strims_video_v1_CaptureConfig = CaptureConfig;
/* @internal */
export type strims_video_v1_ICaptureConfig = ICaptureConfig;
/* @internal */
export const strims_video_v1_CaptureUnit = CaptureUnit;
/* @internal */
export type strims_video_v1_CaptureUnit = CaptureUnit;
/* @internal */
export type strims_video_v1_ICaptureUnit = ICaptureUnit;
/* @internal */
export const strims_video_v1_CaptureAppendRequest = CaptureAppendRequest;
/* @internal */
export type strims_video_v1_CaptureAppendRequest = CaptureAppendRequest;
/* @internal */
export type strims_video_v1_ICaptureAppendRequest = ICaptureAppendRequest;
/* @internal */
export const strims_video_v1_CaptureAppendResponse = CaptureAppendResponse;
/* @internal */
export type strims_video_v1_CaptureAppendResponse = CaptureAppendResponse;
/* @internal */
export type strims_video_v1_ICaptureAppendResponse = ICaptureAppendResponse;
/* @internal */
export const strims_video_v1_CaptureCloseRequest = CaptureCloseRequest;
/* @internal */
export type strims_video_v1_CaptureCloseRequest = CaptureCloseRequest;
/* @internal */
export type strims_video_v1_ICaptureCloseRequest = ICaptureCloseRequest;
/* @internal */
export const strims_video_v1_CaptureCloseResponse = CaptureCloseResponse;
/* @internal */
export type strims_video_v1_CaptureCloseResponse = CaptureCloseResponse;
/* @internal */
export type strims_video_v1_ICaptureCloseResponse = ICaptureCloseResponse;
/* @internal */
export const strims_video_v1_CaptureConfig_Codec = CaptureConfig.Codec;
/* @internal */
export type strims_video_v1_CaptureConfig_Codec = CaptureConfig.Codec;
