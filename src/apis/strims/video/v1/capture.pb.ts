// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  ListingSnippet as strims_network_v1_directory_ListingSnippet,
  type IListingSnippet as strims_network_v1_directory_IListingSnippet,
} from "../../network/v1/directory/directory.pb";

export type ICaptureOpenRequest = {
  directorySnippet?: strims_network_v1_directory_IListingSnippet;
  networkKeys?: Uint8Array[];
  /**
   * The capture's nominal bitrate, its encoders' configured bitrates
   * together, in bits per second, and whether it has a video track: they
   * pick how the swarm signs.
   */
  bitrate?: bigint;
  video?: boolean;
}

export class CaptureOpenRequest {
  directorySnippet: strims_network_v1_directory_ListingSnippet | undefined;
  networkKeys: Uint8Array[];
  /**
   * The capture's nominal bitrate, its encoders' configured bitrates
   * together, in bits per second, and whether it has a video track: they
   * pick how the swarm signs.
   */
  bitrate: bigint;
  video: boolean;

  constructor(v?: ICaptureOpenRequest) {
    this.directorySnippet = v?.directorySnippet && new strims_network_v1_directory_ListingSnippet(v.directorySnippet);
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
    this.bitrate = v?.bitrate || BigInt(0);
    this.video = v?.video || false;
  }

  static encode(m: CaptureOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directorySnippet) strims_network_v1_directory_ListingSnippet.encode(m.directorySnippet, w.uint32(10).fork()).ldelim();
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    if (m.bitrate) w.uint32(24).uint64(m.bitrate);
    if (m.video) w.uint32(32).bool(m.video);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureOpenRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 4:
        m.video = r.bool();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CaptureOpenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ICaptureUpdateResponse = object;

export class CaptureUpdateResponse {

  constructor(v?: ICaptureUpdateResponse) {
    void v; // an empty message has no fields to set
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

/** A track's decoder config, as its encoder gave it. */
export type ICaptureConfig = {
  /** Nonzero. */
  track?: number;
  codec?: CaptureConfig_Codec;
  config?: Uint8Array;
}

/** A track's decoder config, as its encoder gave it. */
export class CaptureConfig {
  /** Nonzero. */
  track: number;
  codec: CaptureConfig_Codec;
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export enum CaptureConfig_Codec {
  CODEC_UNSPECIFIED = 0,
  /**
   * config is an AVCDecoderConfigurationRecord; units are NAL units
   * with the record's length prefixes. Timed at 90 kHz.
   */
  CODEC_H264 = 1,
  /**
   * config is an OpusHead (RFC 7845 5.1); units are Opus packets. Timed
   * at 48 kHz.
   */
  CODEC_OPUS = 2,
}

/**
 * An access unit. A track's units come in decode order, which is their
 * presentation order.
 */
export type ICaptureUnit = {
  track?: number;
  /**
   * Presentation time in microseconds on the capture's clock, after the
   * track's last unit's.
   */
  timestampUs?: bigint;
  keyframe?: boolean;
  data?: Uint8Array;
}

/**
 * An access unit. A track's units come in decode order, which is their
 * presentation order.
 */
export class CaptureUnit {
  track: number;
  /**
   * Presentation time in microseconds on the capture's clock, after the
   * track's last unit's.
   */
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/** Configs apply from the request's units on. */
export type ICaptureAppendRequest = {
  id?: Uint8Array;
  configs?: ICaptureConfig[];
  units?: ICaptureUnit[];
}

/** Configs apply from the request's units on. */
export class CaptureAppendRequest {
  id: Uint8Array;
  configs: CaptureConfig[];
  units: CaptureUnit[];

  constructor(v?: ICaptureAppendRequest) {
    this.id = v?.id || new Uint8Array();
    this.configs = v?.configs ? v.configs.map(v => new CaptureConfig(v)) : [];
    this.units = v?.units ? v.units.map(v => new CaptureUnit(v)) : [];
  }

  static encode(m: CaptureAppendRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    for (const v of m.configs) CaptureConfig.encode(v, w.uint32(18).fork()).ldelim();
    for (const v of m.units) CaptureUnit.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureAppendRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CaptureAppendRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        case 2:
        m.configs.push(CaptureConfig.decode(r, r.uint32()));
        break;
        case 3:
        m.units.push(CaptureUnit.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ICaptureAppendResponse = object;

export class CaptureAppendResponse {

  constructor(v?: ICaptureAppendResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CaptureCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ICaptureCloseResponse = object;

export class CaptureCloseResponse {

  constructor(v?: ICaptureCloseResponse) {
    void v; // an empty message has no fields to set
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

