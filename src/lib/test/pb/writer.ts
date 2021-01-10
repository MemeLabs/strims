import { writeFloat32, writeFloat64 } from "./float";

const maxVarintLen32 = 5;

export default class Writer {
  buf: Uint8Array;
  pos: number = 0;
  offsets: number[];

  constructor(size: number) {
    this.buf = new Uint8Array(size);
    this.offsets = [];
  }

  fork(): Writer {
    this.offsets.push(this.pos);
    this.pos += maxVarintLen32;
    return this;
  }

  ldelim(): Writer {
    const pos = this.offsets.pop();
    const len = this.pos - pos - maxVarintLen32;

    this.pos = pos;
    this.uint32(len);
    this.buf.copyWithin(this.pos, pos + maxVarintLen32, pos + maxVarintLen32 + len);
    this.pos += len;

    return this;
  }

  int32(v: number): Writer {
    return this.uint32(v >> 0);
  }

  int64(v: bigint): Writer {
    if (v < 0) {
      v |= BigInt(1) << BigInt(63);
    }
    return this.uint64(v);
  }

  uint32(v: number): Writer {
    while (v >= 0x80) {
      this.buf[this.pos] = Number(v & 0xff) | 0x80;
      v >>= 7;
      this.pos++;
    }
    this.buf[this.pos] = Number(v & 0xff);
    this.pos++;
    return this;
  }

  uint64(v: bigint): Writer {
    while (v >= BigInt(0x80)) {
      this.buf[this.pos] = Number(v & BigInt(0xff)) | 0x80;
      v >>= BigInt(7);
      this.pos++;
    }
    this.buf[this.pos] = Number(v & BigInt(0xff));
    this.pos++;
    return this;
  }

  sint32(v: number): Writer {
    return this.uint32((v << 1) ^ (v >> 31));
  }

  sint64(v: bigint): Writer {
    v <<= BigInt(1);
    if (v < 0) {
      v |= BigInt(1);
    }
    return this.uint64(v);
  }

  bool(v: boolean): Writer {
    return this.uint32(v ? 1 : 0);
  }

  enum(v: number): Writer {
    return this.uint32(v);
  }

  fixed64(v: bigint): Writer {
    this.buf[this.pos] = Number(v & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(8)) & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(16)) & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(24)) & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(32)) & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(40)) & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(48)) & BigInt(0xff));
    this.buf[this.pos++] = Number((v >> BigInt(56)) & BigInt(0xff));
    return this;
  }

  sfixed64(v: bigint): Writer {
    return this.fixed64(v);
  }

  double(v: number): Writer {
    writeFloat64(this.buf, this.pos, v);
    this.pos += 8;
    return this;
  }

  string(v: string): Writer {
    const encoder = new TextEncoder();
    return this.bytes(encoder.encode(v));
  }

  bytes(v: Uint8Array): Writer {
    this.uint32(v.byteLength);
    this.buf.set(v, this.pos);
    this.pos += v.byteLength;
    return this;
  }

  fixed32(v: number): Writer {
    this.buf[this.pos] = v & 0xff;
    this.buf[this.pos++] = (v >> 8) & 0xff;
    this.buf[this.pos++] = (v >> 16) & 0xff;
    this.buf[this.pos++] = (v >> 24) & 0xff;
    return this;
  }

  sfixed32(v: number): Writer {
    return this.fixed32(v);
  }

  float(v: number): Writer {
    writeFloat32(this.buf, this.pos, v);
    this.pos += 4;
    return this;
  }
}
