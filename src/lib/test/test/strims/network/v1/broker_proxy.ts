import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";


export interface IBrokerProxyRequest {
  connMtu?: number;
}

export class BrokerProxyRequest {
  connMtu: number = 0;

  constructor(v?: IBrokerProxyRequest) {
    this.connMtu = v?.connMtu || 0;
  }

  static encode(m: BrokerProxyRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBrokerProxyEvent {
  body?: BrokerProxyEvent.IBodyOneOf
}

export class BrokerProxyEvent {
  body: BrokerProxyEvent.BodyOneOf;

  constructor(v?: IBrokerProxyEvent) {
    this.body = new BrokerProxyEvent.BodyOneOf(v?.body);
  }

  static encode(m: BrokerProxyEvent, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
      BrokerProxyEvent.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case 2:
      BrokerProxyEvent.Data.encode(m.body.data, w.uint32(18).fork()).ldelim();
      break;
      case 3:
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
        m.body.open = BrokerProxyEvent.Open.decode(r, r.uint32());
        break;
        case 2:
        m.body.data = BrokerProxyEvent.Data.decode(r, r.uint32());
        break;
        case 3:
        m.body.read = BrokerProxyEvent.Read.decode(r, r.uint32());
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
  export type IBodyOneOf =
  { open: BrokerProxyEvent.IOpen }
  |{ data: BrokerProxyEvent.IData }
  |{ read: BrokerProxyEvent.IRead }
  ;

  export class BodyOneOf {
    private _open: BrokerProxyEvent.Open | undefined;
    private _data: BrokerProxyEvent.Data | undefined;
    private _read: BrokerProxyEvent.Read | undefined;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "open" in v) this.open = new BrokerProxyEvent.Open(v.open);
      if (v && "data" in v) this.data = new BrokerProxyEvent.Data(v.data);
      if (v && "read" in v) this.read = new BrokerProxyEvent.Read(v.read);
    }

    public clear() {
      this._open = undefined;
      this._data = undefined;
      this._read = undefined;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set open(v: BrokerProxyEvent.Open) {
      this.clear();
      this._open = v;
      this._case = BodyCase.OPEN;
    }

    get open(): BrokerProxyEvent.Open {
      return this._open;
    }

    set data(v: BrokerProxyEvent.Data) {
      this.clear();
      this._data = v;
      this._case = BodyCase.DATA;
    }

    get data(): BrokerProxyEvent.Data {
      return this._data;
    }

    set read(v: BrokerProxyEvent.Read) {
      this.clear();
      this._read = v;
      this._case = BodyCase.READ;
    }

    get read(): BrokerProxyEvent.Read {
      return this._read;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1,
    DATA = 2,
    READ = 3,
  }

  export interface IOpen {
    proxyId?: bigint;
  }

  export class Open {
    proxyId: bigint = BigInt(0);

    constructor(v?: IOpen) {
      this.proxyId = v?.proxyId || BigInt(0);
    }

    static encode(m: Open, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
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

  export interface IData {
    data?: Uint8Array;
  }

  export class Data {
    data: Uint8Array = new Uint8Array();

    constructor(v?: IData) {
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Data, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.data) w.uint32(10).bytes(m.data);
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

  export interface IRead {
  }

  export class Read {

    constructor(v?: IRead) {
      // noop
    }

    static encode(m: Read, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Read {
      if (r instanceof Reader && length) r.skip(length);
      return new Read();
    }
  }

}

export interface IBrokerProxySendKeysRequest {
  proxyId?: bigint;
  keys?: Uint8Array[];
}

export class BrokerProxySendKeysRequest {
  proxyId: bigint = BigInt(0);
  keys: Uint8Array[] = [];

  constructor(v?: IBrokerProxySendKeysRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
    if (v?.keys) this.keys = v.keys;
  }

  static encode(m: BrokerProxySendKeysRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBrokerProxySendKeysResponse {
}

export class BrokerProxySendKeysResponse {

  constructor(v?: IBrokerProxySendKeysResponse) {
    // noop
  }

  static encode(m: BrokerProxySendKeysResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxySendKeysResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxySendKeysResponse();
  }
}

export interface IBrokerProxyReceiveKeysRequest {
  proxyId?: bigint;
  keys?: Uint8Array[];
}

export class BrokerProxyReceiveKeysRequest {
  proxyId: bigint = BigInt(0);
  keys: Uint8Array[] = [];

  constructor(v?: IBrokerProxyReceiveKeysRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
    if (v?.keys) this.keys = v.keys;
  }

  static encode(m: BrokerProxyReceiveKeysRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBrokerProxyReceiveKeysResponse {
  keys?: Uint8Array[];
}

export class BrokerProxyReceiveKeysResponse {
  keys: Uint8Array[] = [];

  constructor(v?: IBrokerProxyReceiveKeysResponse) {
    if (v?.keys) this.keys = v.keys;
  }

  static encode(m: BrokerProxyReceiveKeysResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBrokerProxyDataRequest {
  proxyId?: bigint;
  data?: Uint8Array;
}

export class BrokerProxyDataRequest {
  proxyId: bigint = BigInt(0);
  data: Uint8Array = new Uint8Array();

  constructor(v?: IBrokerProxyDataRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: BrokerProxyDataRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.proxyId) w.uint32(8).uint64(m.proxyId);
    if (m.data) w.uint32(18).bytes(m.data);
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

export interface IBrokerProxyDataResponse {
}

export class BrokerProxyDataResponse {

  constructor(v?: IBrokerProxyDataResponse) {
    // noop
  }

  static encode(m: BrokerProxyDataResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyDataResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxyDataResponse();
  }
}

export interface IBrokerProxyCloseRequest {
  proxyId?: bigint;
}

export class BrokerProxyCloseRequest {
  proxyId: bigint = BigInt(0);

  constructor(v?: IBrokerProxyCloseRequest) {
    this.proxyId = v?.proxyId || BigInt(0);
  }

  static encode(m: BrokerProxyCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBrokerProxyCloseResponse {
}

export class BrokerProxyCloseResponse {

  constructor(v?: IBrokerProxyCloseResponse) {
    // noop
  }

  static encode(m: BrokerProxyCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxyCloseResponse();
  }
}

