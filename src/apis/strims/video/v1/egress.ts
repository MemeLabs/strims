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
    if (m.swarmUri) w.uint32(10).string(m.swarmUri);
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
      EgressOpenStreamResponse.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case EgressOpenStreamResponse.BodyCase.DATA:
      EgressOpenStreamResponse.Data.encode(m.body.data, w.uint32(18).fork()).ldelim();
      break;
      case EgressOpenStreamResponse.BodyCase.ERROR:
      EgressOpenStreamResponse.Error.encode(m.body.error, w.uint32(26).fork()).ldelim();
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
        m.body = new EgressOpenStreamResponse.Body({ open: EgressOpenStreamResponse.Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new EgressOpenStreamResponse.Body({ data: EgressOpenStreamResponse.Data.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new EgressOpenStreamResponse.Body({ error: EgressOpenStreamResponse.Error.decode(r, r.uint32()) });
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
  |{ case?: BodyCase.OPEN, open: EgressOpenStreamResponse.IOpen }
  |{ case?: BodyCase.DATA, data: EgressOpenStreamResponse.IData }
  |{ case?: BodyCase.ERROR, error: EgressOpenStreamResponse.IError }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: EgressOpenStreamResponse.Open }
  |{ case: BodyCase.DATA, data: EgressOpenStreamResponse.Data }
  |{ case: BodyCase.ERROR, error: EgressOpenStreamResponse.Error }
  >;

  class BodyImpl {
    open: EgressOpenStreamResponse.Open;
    data: EgressOpenStreamResponse.Data;
    error: EgressOpenStreamResponse.Error;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new EgressOpenStreamResponse.Open(v.open);
      } else
      if (v && "data" in v) {
        this.case = BodyCase.DATA;
        this.data = new EgressOpenStreamResponse.Data(v.data);
      } else
      if (v && "error" in v) {
        this.case = BodyCase.ERROR;
        this.error = new EgressOpenStreamResponse.Error(v.error);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: EgressOpenStreamResponse.IOpen } ? { case: BodyCase.OPEN, open: EgressOpenStreamResponse.Open } :
    T extends { data: EgressOpenStreamResponse.IData } ? { case: BodyCase.DATA, data: EgressOpenStreamResponse.Data } :
    T extends { error: EgressOpenStreamResponse.IError } ? { case: BodyCase.ERROR, error: EgressOpenStreamResponse.Error } :
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
      if (m.transferId) w.uint32(10).bytes(m.transferId);
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
    bufferUnderrun?: boolean;
  }

  export class Data {
    data: Uint8Array;
    segmentEnd: boolean;
    bufferUnderrun: boolean;

    constructor(v?: IData) {
      this.data = v?.data || new Uint8Array();
      this.segmentEnd = v?.segmentEnd || false;
      this.bufferUnderrun = v?.bufferUnderrun || false;
    }

    static encode(m: Data, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.data) w.uint32(10).bytes(m.data);
      if (m.segmentEnd) w.uint32(16).bool(m.segmentEnd);
      if (m.bufferUnderrun) w.uint32(24).bool(m.bufferUnderrun);
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
          m.bufferUnderrun = r.bool();
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
      if (m.message) w.uint32(10).string(m.message);
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

