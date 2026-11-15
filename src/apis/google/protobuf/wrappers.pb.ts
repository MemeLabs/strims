// Protocol Buffers - Google's data interchange format
// Copyright 2008 Google Inc.  All rights reserved.
// https://developers.google.com/protocol-buffers/
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
//     * Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above
// copyright notice, this list of conditions and the following disclaimer
// in the documentation and/or other materials provided with the
// distribution.
//     * Neither the name of Google Inc. nor the names of its
// contributors may be used to endorse or promote products derived from
// this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

// Wrappers for primitive (non-message) types. These types are useful
// for embedding primitives in the `google.protobuf.Any` type and for places
// where we need to distinguish between the absence of a primitive
// typed field and its default value.
//
// These wrappers have no meaningful use within repeated fields as they lack
// the ability to detect presence on individual elements.
// These wrappers have no meaningful use within a map or a oneof since
// individual entries of a map or fields of a oneof can already detect presence.

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


/**
 * Wrapper message for `double`.
 *
 * The JSON representation for `DoubleValue` is JSON number.
 */
export type IDoubleValue = {
  /** The double value. */
  value?: number;
}

/**
 * Wrapper message for `double`.
 *
 * The JSON representation for `DoubleValue` is JSON number.
 */
export class DoubleValue {
  /** The double value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DoubleValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.double();
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

/**
 * Wrapper message for `float`.
 *
 * The JSON representation for `FloatValue` is JSON number.
 */
export type IFloatValue = {
  /** The float value. */
  value?: number;
}

/**
 * Wrapper message for `float`.
 *
 * The JSON representation for `FloatValue` is JSON number.
 */
export class FloatValue {
  /** The float value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FloatValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.float();
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

/**
 * Wrapper message for `int64`.
 *
 * The JSON representation for `Int64Value` is JSON string.
 */
export type IInt64Value = {
  /** The int64 value. */
  value?: bigint;
}

/**
 * Wrapper message for `int64`.
 *
 * The JSON representation for `Int64Value` is JSON string.
 */
export class Int64Value {
  /** The int64 value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Int64Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.int64();
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

/**
 * Wrapper message for `uint64`.
 *
 * The JSON representation for `UInt64Value` is JSON string.
 */
export type IUInt64Value = {
  /** The uint64 value. */
  value?: bigint;
}

/**
 * Wrapper message for `uint64`.
 *
 * The JSON representation for `UInt64Value` is JSON string.
 */
export class UInt64Value {
  /** The uint64 value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UInt64Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.uint64();
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

/**
 * Wrapper message for `int32`.
 *
 * The JSON representation for `Int32Value` is JSON number.
 */
export type IInt32Value = {
  /** The int32 value. */
  value?: number;
}

/**
 * Wrapper message for `int32`.
 *
 * The JSON representation for `Int32Value` is JSON number.
 */
export class Int32Value {
  /** The int32 value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Int32Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.int32();
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

/**
 * Wrapper message for `uint32`.
 *
 * The JSON representation for `UInt32Value` is JSON number.
 */
export type IUInt32Value = {
  /** The uint32 value. */
  value?: number;
}

/**
 * Wrapper message for `uint32`.
 *
 * The JSON representation for `UInt32Value` is JSON number.
 */
export class UInt32Value {
  /** The uint32 value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UInt32Value();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.uint32();
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

/**
 * Wrapper message for `bool`.
 *
 * The JSON representation for `BoolValue` is JSON `true` and `false`.
 */
export type IBoolValue = {
  /** The bool value. */
  value?: boolean;
}

/**
 * Wrapper message for `bool`.
 *
 * The JSON representation for `BoolValue` is JSON `true` and `false`.
 */
export class BoolValue {
  /** The bool value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BoolValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.bool();
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

/**
 * Wrapper message for `string`.
 *
 * The JSON representation for `StringValue` is JSON string.
 */
export type IStringValue = {
  /** The string value. */
  value?: string;
}

/**
 * Wrapper message for `string`.
 *
 * The JSON representation for `StringValue` is JSON string.
 */
export class StringValue {
  /** The string value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StringValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.string();
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

/**
 * Wrapper message for `bytes`.
 *
 * The JSON representation for `BytesValue` is JSON string.
 */
export type IBytesValue = {
  /** The bytes value. */
  value?: Uint8Array;
}

/**
 * Wrapper message for `bytes`.
 *
 * The JSON representation for `BytesValue` is JSON string.
 */
export class BytesValue {
  /** The bytes value. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BytesValue();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.bytes();
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

