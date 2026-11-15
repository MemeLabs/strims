// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IHLSEgressConfig = {
  enabled?: boolean;
  publicServerAddr?: string;
}

export class HLSEgressConfig {
  enabled: boolean;
  publicServerAddr: string;

  constructor(v?: IHLSEgressConfig) {
    this.enabled = v?.enabled || false;
    this.publicServerAddr = v?.publicServerAddr || "";
  }

  static encode(m: HLSEgressConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enabled) w.uint32(8).bool(m.enabled);
    if (m.publicServerAddr.length) w.uint32(18).string(m.publicServerAddr);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enabled = r.bool();
        break;
        case 2:
        m.publicServerAddr = r.string();
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

export type IHLSEgressIsSupportedRequest = object;

export class HLSEgressIsSupportedRequest {

  constructor(v?: IHLSEgressIsSupportedRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: HLSEgressIsSupportedRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressIsSupportedRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new HLSEgressIsSupportedRequest();
  }
}

export type IHLSEgressIsSupportedResponse = {
  supported?: boolean;
}

export class HLSEgressIsSupportedResponse {
  supported: boolean;

  constructor(v?: IHLSEgressIsSupportedResponse) {
    this.supported = v?.supported || false;
  }

  static encode(m: HLSEgressIsSupportedResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.supported) w.uint32(8).bool(m.supported);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressIsSupportedResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressIsSupportedResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.supported = r.bool();
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

export type IHLSEgressGetConfigRequest = object;

export class HLSEgressGetConfigRequest {

  constructor(v?: IHLSEgressGetConfigRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: HLSEgressGetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressGetConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new HLSEgressGetConfigRequest();
  }
}

export type IHLSEgressGetConfigResponse = {
  config?: IHLSEgressConfig;
}

export class HLSEgressGetConfigResponse {
  config: HLSEgressConfig | undefined;

  constructor(v?: IHLSEgressGetConfigResponse) {
    this.config = v?.config && new HLSEgressConfig(v.config);
  }

  static encode(m: HLSEgressGetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) HLSEgressConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressGetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressGetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = HLSEgressConfig.decode(r, r.uint32());
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

export type IHLSEgressSetConfigRequest = {
  config?: IHLSEgressConfig;
}

export class HLSEgressSetConfigRequest {
  config: HLSEgressConfig | undefined;

  constructor(v?: IHLSEgressSetConfigRequest) {
    this.config = v?.config && new HLSEgressConfig(v.config);
  }

  static encode(m: HLSEgressSetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) HLSEgressConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressSetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressSetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = HLSEgressConfig.decode(r, r.uint32());
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

export type IHLSEgressSetConfigResponse = {
  config?: IHLSEgressConfig;
}

export class HLSEgressSetConfigResponse {
  config: HLSEgressConfig | undefined;

  constructor(v?: IHLSEgressSetConfigResponse) {
    this.config = v?.config && new HLSEgressConfig(v.config);
  }

  static encode(m: HLSEgressSetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) HLSEgressConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressSetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressSetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = HLSEgressConfig.decode(r, r.uint32());
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

export type IHLSEgressOpenStreamRequest = {
  swarmUri?: string;
  networkKeys?: Uint8Array[];
  /** The MIME type of the stream's listing, which names its format. */
  mimeType?: string;
}

export class HLSEgressOpenStreamRequest {
  swarmUri: string;
  networkKeys: Uint8Array[];
  /** The MIME type of the stream's listing, which names its format. */
  mimeType: string;

  constructor(v?: IHLSEgressOpenStreamRequest) {
    this.swarmUri = v?.swarmUri || "";
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
    this.mimeType = v?.mimeType || "";
  }

  static encode(m: HLSEgressOpenStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmUri.length) w.uint32(10).string(m.swarmUri);
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    if (m.mimeType.length) w.uint32(26).string(m.mimeType);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressOpenStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressOpenStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarmUri = r.string();
        break;
        case 2:
        m.networkKeys.push(r.bytes())
        break;
        case 3:
        m.mimeType = r.string();
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

export type IHLSEgressOpenStreamResponse = {
  playlistUrl?: string;
}

export class HLSEgressOpenStreamResponse {
  playlistUrl: string;

  constructor(v?: IHLSEgressOpenStreamResponse) {
    this.playlistUrl = v?.playlistUrl || "";
  }

  static encode(m: HLSEgressOpenStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.playlistUrl.length) w.uint32(10).string(m.playlistUrl);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressOpenStreamResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressOpenStreamResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.playlistUrl = r.string();
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

export type IHLSEgressCloseStreamRequest = {
  transferId?: Uint8Array;
}

export class HLSEgressCloseStreamRequest {
  transferId: Uint8Array;

  constructor(v?: IHLSEgressCloseStreamRequest) {
    this.transferId = v?.transferId || new Uint8Array();
  }

  static encode(m: HLSEgressCloseStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.transferId.length) w.uint32(10).bytes(m.transferId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressCloseStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HLSEgressCloseStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.transferId = r.bytes();
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

export type IHLSEgressCloseStreamResponse = object;

export class HLSEgressCloseStreamResponse {

  constructor(v?: IHLSEgressCloseStreamResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: HLSEgressCloseStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressCloseStreamResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new HLSEgressCloseStreamResponse();
  }
}

