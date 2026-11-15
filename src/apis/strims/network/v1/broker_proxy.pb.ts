// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BrokerProxyRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.connMtu = r.int32();
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

export type IBrokerProxyEvent = {
  body?: BrokerProxyEvent_IBody
}

export class BrokerProxyEvent {
  body: BrokerProxyEvent_TBody;

  constructor(v?: IBrokerProxyEvent) {
    this.body = v?.body ? new BrokerProxyEvent_Body(v.body) : new BrokerProxyEvent_Body();
  }

  static encode(m: BrokerProxyEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case BrokerProxyEvent_BodyCase.OPEN:
      BrokerProxyEvent_Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case BrokerProxyEvent_BodyCase.DATA:
      BrokerProxyEvent_Data.encode(m.body.data, w.uint32(18).fork()).ldelim();
      break;
      case BrokerProxyEvent_BodyCase.READ:
      BrokerProxyEvent_Read.encode(m.body.read, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BrokerProxyEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new BrokerProxyEvent_Body({ open: BrokerProxyEvent_Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new BrokerProxyEvent_Body({ data: BrokerProxyEvent_Data.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new BrokerProxyEvent_Body({ read: BrokerProxyEvent_Read.decode(r, r.uint32()) });
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

export enum BrokerProxyEvent_BodyCase {
  NOT_SET = 0,
  OPEN = 1,
  DATA = 2,
  READ = 3,
}

export type BrokerProxyEvent_IBody =
{ case?: BrokerProxyEvent_BodyCase.NOT_SET }
|{ case?: BrokerProxyEvent_BodyCase.OPEN, open: BrokerProxyEvent_IOpen }
|{ case?: BrokerProxyEvent_BodyCase.DATA, data: BrokerProxyEvent_IData }
|{ case?: BrokerProxyEvent_BodyCase.READ, read: BrokerProxyEvent_IRead }
;

export type BrokerProxyEvent_TBody = Readonly<
{ case: BrokerProxyEvent_BodyCase.NOT_SET }
|{ case: BrokerProxyEvent_BodyCase.OPEN, open: BrokerProxyEvent_Open }
|{ case: BrokerProxyEvent_BodyCase.DATA, data: BrokerProxyEvent_Data }
|{ case: BrokerProxyEvent_BodyCase.READ, read: BrokerProxyEvent_Read }
>;

class BrokerProxyEvent_BodyImpl {
  declare open?: BrokerProxyEvent_Open;
  declare data?: BrokerProxyEvent_Data;
  declare read?: BrokerProxyEvent_Read;
  case: BrokerProxyEvent_BodyCase = BrokerProxyEvent_BodyCase.NOT_SET;

  constructor(v?: BrokerProxyEvent_IBody) {
    if (v && "open" in v) {
      this.case = BrokerProxyEvent_BodyCase.OPEN;
      this.open = new BrokerProxyEvent_Open(v.open);
    } else
    if (v && "data" in v) {
      this.case = BrokerProxyEvent_BodyCase.DATA;
      this.data = new BrokerProxyEvent_Data(v.data);
    } else
    if (v && "read" in v) {
      this.case = BrokerProxyEvent_BodyCase.READ;
      this.read = new BrokerProxyEvent_Read(v.read);
    }
  }
}

export const BrokerProxyEvent_Body = BrokerProxyEvent_BodyImpl as {
  new (): Readonly<{ case: BrokerProxyEvent_BodyCase.NOT_SET }>;
  new <T extends BrokerProxyEvent_IBody>(v: T): Readonly<
  T extends { open: BrokerProxyEvent_IOpen } ? { case: BrokerProxyEvent_BodyCase.OPEN, open: BrokerProxyEvent_Open } :
  T extends { data: BrokerProxyEvent_IData } ? { case: BrokerProxyEvent_BodyCase.DATA, data: BrokerProxyEvent_Data } :
  T extends { read: BrokerProxyEvent_IRead } ? { case: BrokerProxyEvent_BodyCase.READ, read: BrokerProxyEvent_Read } :
  never
  >;
};

export type BrokerProxyEvent_IOpen = {
  proxyId?: bigint;
}

export class BrokerProxyEvent_Open {
  proxyId: bigint;

  constructor(v?: BrokerProxyEvent_IOpen) {
    this.proxyId = v?.proxyId || BigInt(0);
  }

  static encode(m: BrokerProxyEvent_Open, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.proxyId) w.uint32(8).uint64(m.proxyId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyEvent_Open {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BrokerProxyEvent_Open();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.proxyId = r.uint64();
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

export type BrokerProxyEvent_IData = {
  data?: Uint8Array;
}

export class BrokerProxyEvent_Data {
  data: Uint8Array;

  constructor(v?: BrokerProxyEvent_IData) {
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: BrokerProxyEvent_Data, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data.length) w.uint32(10).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyEvent_Data {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BrokerProxyEvent_Data();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type BrokerProxyEvent_IRead = object;

export class BrokerProxyEvent_Read {

  constructor(v?: BrokerProxyEvent_IRead) {
    void v; // an empty message has no fields to set
  }

  static encode(m: BrokerProxyEvent_Read, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BrokerProxyEvent_Read {
    if (r instanceof Reader && length) r.skip(length);
    return new BrokerProxyEvent_Read();
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IBrokerProxySendKeysResponse = object;

export class BrokerProxySendKeysResponse {

  constructor(v?: IBrokerProxySendKeysResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BrokerProxyReceiveKeysResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.keys.push(r.bytes())
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IBrokerProxyDataResponse = object;

export class BrokerProxyDataResponse {

  constructor(v?: IBrokerProxyDataResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BrokerProxyCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.proxyId = r.uint64();
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

export type IBrokerProxyCloseResponse = object;

export class BrokerProxyCloseResponse {

  constructor(v?: IBrokerProxyCloseResponse) {
    void v; // an empty message has no fields to set
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

