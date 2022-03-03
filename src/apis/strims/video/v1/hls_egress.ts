import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IHLSEgressIsSupportedRequest = {
}

export class HLSEgressIsSupportedRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IHLSEgressIsSupportedRequest) {
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new HLSEgressIsSupportedResponse();
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

export type IHLSEgressOpenStreamRequest = {
  swarmUri?: string;
  networkKeys?: Uint8Array[];
}

export class HLSEgressOpenStreamRequest {
  swarmUri: string;
  networkKeys: Uint8Array[];

  constructor(v?: IHLSEgressOpenStreamRequest) {
    this.swarmUri = v?.swarmUri || "";
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
  }

  static encode(m: HLSEgressOpenStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmUri.length) w.uint32(10).string(m.swarmUri);
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HLSEgressOpenStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new HLSEgressOpenStreamResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.playlistUrl = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new HLSEgressCloseStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.transferId = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IHLSEgressCloseStreamResponse = {
}

export class HLSEgressCloseStreamResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IHLSEgressCloseStreamResponse) {
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

