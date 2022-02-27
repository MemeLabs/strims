import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IDoubleValue = {
  value?: number;
}

export class DoubleValue {
  value: number;

  constructor(v?: IDoubleValue) {
    this.value = v?.value || 0;
  }

  static encode(m: DoubleValue, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(9).double(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DoubleValue {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DoubleValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.double();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFloatValue = {
  value?: number;
}

export class FloatValue {
  value: number;

  constructor(v?: IFloatValue) {
    this.value = v?.value || 0;
  }

  static encode(m: FloatValue, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(13).float(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FloatValue {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FloatValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.float();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IInt64Value = {
  value?: bigint;
}

export class Int64Value {
  value: bigint;

  constructor(v?: IInt64Value) {
    this.value = v?.value || BigInt(0);
  }

  static encode(m: Int64Value, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(8).int64(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Int64Value {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Int64Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.int64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUInt64Value = {
  value?: bigint;
}

export class UInt64Value {
  value: bigint;

  constructor(v?: IUInt64Value) {
    this.value = v?.value || BigInt(0);
  }

  static encode(m: UInt64Value, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(8).uint64(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UInt64Value {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UInt64Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IInt32Value = {
  value?: number;
}

export class Int32Value {
  value: number;

  constructor(v?: IInt32Value) {
    this.value = v?.value || 0;
  }

  static encode(m: Int32Value, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(8).int32(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Int32Value {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Int32Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.int32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUInt32Value = {
  value?: number;
}

export class UInt32Value {
  value: number;

  constructor(v?: IUInt32Value) {
    this.value = v?.value || 0;
  }

  static encode(m: UInt32Value, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(8).uint32(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UInt32Value {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UInt32Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBoolValue = {
  value?: boolean;
}

export class BoolValue {
  value: boolean;

  constructor(v?: IBoolValue) {
    this.value = v?.value || false;
  }

  static encode(m: BoolValue, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(8).bool(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BoolValue {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BoolValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IStringValue = {
  value?: string;
}

export class StringValue {
  value: string;

  constructor(v?: IStringValue) {
    this.value = v?.value || "";
  }

  static encode(m: StringValue, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value.length) w.uint32(10).string(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StringValue {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new StringValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBytesValue = {
  value?: Uint8Array;
}

export class BytesValue {
  value: Uint8Array;

  constructor(v?: IBytesValue) {
    this.value = v?.value || new Uint8Array();
  }

  static encode(m: BytesValue, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value.length) w.uint32(10).bytes(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BytesValue {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BytesValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

