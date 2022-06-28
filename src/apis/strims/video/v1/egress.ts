import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IEgressOpenStreamRequest = {
  swarmUri?: string;
  networkKeys?: Uint8Array[];
}

export class EgressOpenStreamRequest {
  swarmUri: string;
  networkKeys: Uint8Array[];

  constructor(v?: IEgressOpenStreamRequest) {
    this.swarmUri = v?.swarmUri || "";
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
  }

  static encode(m: EgressOpenStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmUri.length) w.uint32(10).string(m.swarmUri);
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EgressOpenStreamRequest();
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

export type IEgressOpenStreamResponse = {
  body?: EgressOpenStreamResponse.IBody
}

export class EgressOpenStreamResponse {
  body: EgressOpenStreamResponse.TBody;

  constructor(v?: IEgressOpenStreamResponse) {
    this.body = new EgressOpenStreamResponse.Body(v?.body);
  }

  static encode(m: EgressOpenStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case EgressOpenStreamResponse.BodyCase.OPEN:
      strims_video_v1_EgressOpenStreamResponse_Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case EgressOpenStreamResponse.BodyCase.DATA:
      strims_video_v1_EgressOpenStreamResponse_Data.encode(m.body.data, w.uint32(18).fork()).ldelim();
      break;
      case EgressOpenStreamResponse.BodyCase.ERROR:
      strims_video_v1_EgressOpenStreamResponse_Error.encode(m.body.error, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EgressOpenStreamResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new EgressOpenStreamResponse.Body({ open: strims_video_v1_EgressOpenStreamResponse_Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new EgressOpenStreamResponse.Body({ data: strims_video_v1_EgressOpenStreamResponse_Data.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new EgressOpenStreamResponse.Body({ error: strims_video_v1_EgressOpenStreamResponse_Error.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace EgressOpenStreamResponse {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1,
    DATA = 2,
    ERROR = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: strims_video_v1_EgressOpenStreamResponse_IOpen }
  |{ case?: BodyCase.DATA, data: strims_video_v1_EgressOpenStreamResponse_IData }
  |{ case?: BodyCase.ERROR, error: strims_video_v1_EgressOpenStreamResponse_IError }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: strims_video_v1_EgressOpenStreamResponse_Open }
  |{ case: BodyCase.DATA, data: strims_video_v1_EgressOpenStreamResponse_Data }
  |{ case: BodyCase.ERROR, error: strims_video_v1_EgressOpenStreamResponse_Error }
  >;

  class BodyImpl {
    open: strims_video_v1_EgressOpenStreamResponse_Open;
    data: strims_video_v1_EgressOpenStreamResponse_Data;
    error: strims_video_v1_EgressOpenStreamResponse_Error;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new strims_video_v1_EgressOpenStreamResponse_Open(v.open);
      } else
      if (v && "data" in v) {
        this.case = BodyCase.DATA;
        this.data = new strims_video_v1_EgressOpenStreamResponse_Data(v.data);
      } else
      if (v && "error" in v) {
        this.case = BodyCase.ERROR;
        this.error = new strims_video_v1_EgressOpenStreamResponse_Error(v.error);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: strims_video_v1_EgressOpenStreamResponse_IOpen } ? { case: BodyCase.OPEN, open: strims_video_v1_EgressOpenStreamResponse_Open } :
    T extends { data: strims_video_v1_EgressOpenStreamResponse_IData } ? { case: BodyCase.DATA, data: strims_video_v1_EgressOpenStreamResponse_Data } :
    T extends { error: strims_video_v1_EgressOpenStreamResponse_IError } ? { case: BodyCase.ERROR, error: strims_video_v1_EgressOpenStreamResponse_Error } :
    never
    >;
  };

  export type IOpen = {
    transferId?: Uint8Array;
  }

  export class Open {
    transferId: Uint8Array;

    constructor(v?: IOpen) {
      this.transferId = v?.transferId || new Uint8Array();
    }

    static encode(m: Open, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.transferId.length) w.uint32(10).bytes(m.transferId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Open {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Open();
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

  export type IData = {
    data?: Uint8Array;
    segmentEnd?: boolean;
    discontinuity?: boolean;
  }

  export class Data {
    data: Uint8Array;
    segmentEnd: boolean;
    discontinuity: boolean;

    constructor(v?: IData) {
      this.data = v?.data || new Uint8Array();
      this.segmentEnd = v?.segmentEnd || false;
      this.discontinuity = v?.discontinuity || false;
    }

    static encode(m: Data, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.data.length) w.uint32(10).bytes(m.data);
      if (m.segmentEnd) w.uint32(16).bool(m.segmentEnd);
      if (m.discontinuity) w.uint32(24).bool(m.discontinuity);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Data {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Data();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.data = r.bytes();
          break;
          case 2:
          m.segmentEnd = r.bool();
          break;
          case 3:
          m.discontinuity = r.bool();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IError = {
    message?: string;
  }

  export class Error {
    message: string;

    constructor(v?: IError) {
      this.message = v?.message || "";
    }

    static encode(m: Error, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.message.length) w.uint32(10).string(m.message);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Error {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Error();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.message = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

/* @internal */
export const strims_video_v1_EgressOpenStreamRequest = EgressOpenStreamRequest;
/* @internal */
export type strims_video_v1_EgressOpenStreamRequest = EgressOpenStreamRequest;
/* @internal */
export type strims_video_v1_IEgressOpenStreamRequest = IEgressOpenStreamRequest;
/* @internal */
export const strims_video_v1_EgressOpenStreamResponse = EgressOpenStreamResponse;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse = EgressOpenStreamResponse;
/* @internal */
export type strims_video_v1_IEgressOpenStreamResponse = IEgressOpenStreamResponse;
/* @internal */
export const strims_video_v1_EgressOpenStreamResponse_Open = EgressOpenStreamResponse.Open;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse_Open = EgressOpenStreamResponse.Open;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse_IOpen = EgressOpenStreamResponse.IOpen;
/* @internal */
export const strims_video_v1_EgressOpenStreamResponse_Data = EgressOpenStreamResponse.Data;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse_Data = EgressOpenStreamResponse.Data;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse_IData = EgressOpenStreamResponse.IData;
/* @internal */
export const strims_video_v1_EgressOpenStreamResponse_Error = EgressOpenStreamResponse.Error;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse_Error = EgressOpenStreamResponse.Error;
/* @internal */
export type strims_video_v1_EgressOpenStreamResponse_IError = EgressOpenStreamResponse.IError;
