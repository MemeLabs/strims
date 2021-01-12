import Reader from "../../../../lib/pb/reader";
import Writer from "../../../../lib/pb/writer";

import {
  Any as google_protobuf_Any,
  IAny as google_protobuf_IAny,
} from "../../../google/protobuf/any";

export interface ICall {
  id?: bigint;
  parentId?: bigint;
  method?: string;
  argument?: google_protobuf_IAny | undefined;
  headers?: Map<string, Uint8Array> | { [key: string]: Uint8Array };
}

export class Call {
  id: bigint = BigInt(0);
  parentId: bigint = BigInt(0);
  method: string = "";
  argument: google_protobuf_Any | undefined;
  headers: Map<string, Uint8Array> = new Map();

  constructor(v?: ICall) {
    this.id = v?.id || BigInt(0);
    this.parentId = v?.parentId || BigInt(0);
    this.method = v?.method || "";
    this.argument = v?.argument && new google_protobuf_Any(v.argument);
    if (v?.headers) this.headers = v.headers instanceof Map ? v.headers : new Map(Object.entries(v.headers));
  }

  static encode(m: Call, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.parentId) w.uint32(16).uint64(m.parentId);
    if (m.method) w.uint32(26).string(m.method);
    if (m.argument) google_protobuf_Any.encode(m.argument, w.uint32(34).fork()).ldelim();
    for (const [k, v] of m.headers) w.uint32(42).fork().uint32(10).string(k).uint32(18).bytes(v).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Call {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Call();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.parentId = r.uint64();
        break;
        case 3:
        m.method = r.string();
        break;
        case 4:
        m.argument = google_protobuf_Any.decode(r, r.uint32());
        break;
        case 5:
        {
          const flen = r.uint32();
          const fend = r.pos + flen;
          let key: string;
          let value: Uint8Array;
          while (r.pos < fend) {
            const ftag = r.uint32();
            switch (ftag >> 3) {
              case 1:
              key = r.string()
              break;
              case 2:
              value = r.bytes();
              break;
            }
          }
          m.headers.set(key, value)
        }
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IError {
  message?: string;
}

export class Error {
  message: string = "";

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

export interface ICancel {
}

export class Cancel {

  constructor(v?: ICancel) {
    // noop
  }

  static encode(m: Cancel, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cancel {
    if (r instanceof Reader && length) r.skip(length);
    return new Cancel();
  }
}

export interface IUndefined {
}

export class Undefined {

  constructor(v?: IUndefined) {
    // noop
  }

  static encode(m: Undefined, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Undefined {
    if (r instanceof Reader && length) r.skip(length);
    return new Undefined();
  }
}

export interface IClose {
}

export class Close {

  constructor(v?: IClose) {
    // noop
  }

  static encode(m: Close, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Close {
    if (r instanceof Reader && length) r.skip(length);
    return new Close();
  }
}

