import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export interface IHLSEgressIsSupportedRequest {
}

export class HLSEgressIsSupportedRequest {

  constructor(v?: IHLSEgressIsSupportedRequest) {
    // noop
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

export interface IHLSEgressIsSupportedResponse {
  supported?: boolean;
}

export class HLSEgressIsSupportedResponse {
  supported: boolean = false;

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

export interface IHLSEgressOpenStreamRequest {
  swarmUri?: string;
}

export class HLSEgressOpenStreamRequest {
  swarmUri: string = "";

  constructor(v?: IHLSEgressOpenStreamRequest) {
    this.swarmUri = v?.swarmUri || "";
  }

  static encode(m: HLSEgressOpenStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmUri) w.uint32(10).string(m.swarmUri);
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
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IHLSEgressOpenStreamResponse {
  playlistUrl?: string;
}

export class HLSEgressOpenStreamResponse {
  playlistUrl: string = "";

  constructor(v?: IHLSEgressOpenStreamResponse) {
    this.playlistUrl = v?.playlistUrl || "";
  }

  static encode(m: HLSEgressOpenStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.playlistUrl) w.uint32(10).string(m.playlistUrl);
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

export interface IHLSEgressCloseStreamRequest {
  transferId?: Uint8Array;
}

export class HLSEgressCloseStreamRequest {
  transferId: Uint8Array = new Uint8Array();

  constructor(v?: IHLSEgressCloseStreamRequest) {
    this.transferId = v?.transferId || new Uint8Array();
  }

  static encode(m: HLSEgressCloseStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.transferId) w.uint32(10).bytes(m.transferId);
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

export interface IHLSEgressCloseStreamResponse {
}

export class HLSEgressCloseStreamResponse {

  constructor(v?: IHLSEgressCloseStreamResponse) {
    // noop
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

