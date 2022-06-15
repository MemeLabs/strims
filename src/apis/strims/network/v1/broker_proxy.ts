import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IBrokerProxyRequest = {
  connMtu?: number;
}

export class BrokerProxyRequest {
  connMtu: number;

  constructor(v?: IBrokerProxyRequest) {
    this.connMtu = v?.connMtu || 0;
  }

  static encode(m: BrokerProxyRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.connMtu) w.uint32(8).int32(m.connMtu);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxyRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.connMtu = r.int32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBrokerProxyEvent = {
  body?: BrokerProxyEvent.IBody
}

export class BrokerProxyEvent {
  body: BrokerProxyEvent.TBody;

  constructor(v?: IBrokerProxyEvent) {
    this.body = new BrokerProxyEvent.Body(v?.body);
  }

  static encode(m: BrokerProxyEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case BrokerProxyEvent.BodyCase.OPEN:
      BrokerProxyEvent.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case BrokerProxyEvent.BodyCase.DATA:
      BrokerProxyEvent.Data.encode(m.body.data, w.uint32(18).fork()).ldelim();
      break;
      case BrokerProxyEvent.BodyCase.READ:
      BrokerProxyEvent.Read.encode(m.body.read, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxyEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new BrokerProxyEvent.Body({ open: BrokerProxyEvent.Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new BrokerProxyEvent.Body({ data: BrokerProxyEvent.Data.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new BrokerProxyEvent.Body({ read: BrokerProxyEvent.Read.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace BrokerProxyEvent {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1,
    DATA = 2,
    READ = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: BrokerProxyEvent.IOpen }
  |{ case?: BodyCase.DATA, data: BrokerProxyEvent.IData }
  |{ case?: BodyCase.READ, read: BrokerProxyEvent.IRead }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: BrokerProxyEvent.Open }
  |{ case: BodyCase.DATA, data: BrokerProxyEvent.Data }
  |{ case: BodyCase.READ, read: BrokerProxyEvent.Read }
  >;

  class BodyImpl {
    open: BrokerProxyEvent.Open;
    data: BrokerProxyEvent.Data;
    read: BrokerProxyEvent.Read;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new BrokerProxyEvent.Open(v.open);
      } else
      if (v && "data" in v) {
        this.case = BodyCase.DATA;
        this.data = new BrokerProxyEvent.Data(v.data);
      } else
      if (v && "read" in v) {
        this.case = BodyCase.READ;
        this.read = new BrokerProxyEvent.Read(v.read);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: BrokerProxyEvent.IOpen } ? { case: BodyCase.OPEN, open: BrokerProxyEvent.Open } :
    T extends { data: BrokerProxyEvent.IData } ? { case: BodyCase.DATA, data: BrokerProxyEvent.Data } :
    T extends { read: BrokerProxyEvent.IRead } ? { case: BodyCase.READ, read: BrokerProxyEvent.Read } :
    never
    >;
  };

  export type IOpen = {
    proxyId?: bigint;
  }

  export class Open {
    proxyId: bigint;

    constructor(v?: IOpen) {
      this.proxyId = v?.proxyId || BigInt(0);
    }

    static encode(m: Open, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.proxyId) w.uint32(8).uint64(m.proxyId);
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
          m.proxyId = r.uint64();
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
  }

  export class Data {
    data: Uint8Array;

    constructor(v?: IData) {
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Data, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.data.length) w.uint32(10).bytes(m.data);
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
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IRead = Record<string, any>;

  export class Read {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IRead) {
    }

    static encode(m: Read, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Read {
      if (r instanceof Reader && length) r.skip(length);
      return new Read();
    }
  }

}

export type IBrokerProxySendKeysRequest = {
  proxyId?: bigint;
  keys?: Uint8Array[];
}

export class BrokerProxySendKeysRequest {
  proxyId: bigint;
  keys: Uint8Array[];

  constructor(v?: IBrokerProxySendKeysRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
    this.keys = v?.keys ? v.keys : [];
  }

  static encode(m: BrokerProxySendKeysRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.proxyId) w.uint32(8).uint64(m.proxyId);
    for (const v of m.keys) w.uint32(18).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxySendKeysRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxySendKeysRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.proxyId = r.uint64();
        break;
        case 2:
        m.keys.push(r.bytes())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBrokerProxySendKeysResponse = Record<string, any>;

export class BrokerProxySendKeysResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBrokerProxySendKeysResponse) {
  }

  static encode(m: BrokerProxySendKeysResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxySendKeysResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxySendKeysResponse();
  }
}

export type IBrokerProxyReceiveKeysRequest = {
  proxyId?: bigint;
  keys?: Uint8Array[];
}

export class BrokerProxyReceiveKeysRequest {
  proxyId: bigint;
  keys: Uint8Array[];

  constructor(v?: IBrokerProxyReceiveKeysRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
    this.keys = v?.keys ? v.keys : [];
  }

  static encode(m: BrokerProxyReceiveKeysRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.proxyId) w.uint32(8).uint64(m.proxyId);
    for (const v of m.keys) w.uint32(18).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyReceiveKeysRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxyReceiveKeysRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.proxyId = r.uint64();
        break;
        case 2:
        m.keys.push(r.bytes())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBrokerProxyReceiveKeysResponse = {
  keys?: Uint8Array[];
}

export class BrokerProxyReceiveKeysResponse {
  keys: Uint8Array[];

  constructor(v?: IBrokerProxyReceiveKeysResponse) {
    this.keys = v?.keys ? v.keys : [];
  }

  static encode(m: BrokerProxyReceiveKeysResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.keys) w.uint32(10).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyReceiveKeysResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxyReceiveKeysResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.keys.push(r.bytes())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBrokerProxyDataRequest = {
  proxyId?: bigint;
  data?: Uint8Array;
}

export class BrokerProxyDataRequest {
  proxyId: bigint;
  data: Uint8Array;

  constructor(v?: IBrokerProxyDataRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: BrokerProxyDataRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.proxyId) w.uint32(8).uint64(m.proxyId);
    if (m.data.length) w.uint32(18).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyDataRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxyDataRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.proxyId = r.uint64();
        break;
        case 2:
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

export type IBrokerProxyDataResponse = Record<string, any>;

export class BrokerProxyDataResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBrokerProxyDataResponse) {
  }

  static encode(m: BrokerProxyDataResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyDataResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxyDataResponse();
  }
}

export type IBrokerProxyCloseRequest = {
  proxyId?: bigint;
}

export class BrokerProxyCloseRequest {
  proxyId: bigint;

  constructor(v?: IBrokerProxyCloseRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
  }

  static encode(m: BrokerProxyCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.proxyId) w.uint32(8).uint64(m.proxyId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyCloseRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BrokerProxyCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.proxyId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBrokerProxyCloseResponse = Record<string, any>;

export class BrokerProxyCloseResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBrokerProxyCloseResponse) {
  }

  static encode(m: BrokerProxyCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxyCloseResponse();
  }
}

