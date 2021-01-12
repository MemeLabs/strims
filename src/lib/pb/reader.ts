import { readFloat32, readFloat64 } from "./float";

export default class Reader {
  buf: Uint8Array;
  pos: number = 0;

  constructor(buf: Uint8Array) {
    this.buf = buf;
  }

  get len(): number {
    return this.buf.byteLength;
  }

  skip(n: number) {
    if (this.pos + n > this.len) {
      throw new Error("index out of range");
    }
    this.pos += n;
  }

  skipType(wireType: number) {
    switch (wireType) {
      case 0:
        do {
          this.skip(1);
        } while (this.buf[this.pos] & 0x80);
        break;
      case 1:
        this.skip(8);
        break;
      case 2:
        this.skip(this.uint32());
        break;
      case 5:
        this.skip(4);
        break;
    }
  }

  int32(): number {
    return this.uint32() | 0;
  }

  int64(): bigint {
    const v = this.uint64();
    return v >> BigInt(63) === BigInt(0) ? v : -(v & BigInt(0x7fffffffffffffff));
  }

  uint32(): number {
    let v = (this.buf[this.pos] & 127) >>> 0;
    if (this.buf[this.pos++] < 128) return v;
    v = (v | ((this.buf[this.pos] & 127) << 7)) >>> 0;
    if (this.buf[this.pos++] < 128) return v;
    v = (v | ((this.buf[this.pos] & 127) << 14)) >>> 0;
    if (this.buf[this.pos++] < 128) return v;
    v = (v | ((this.buf[this.pos] & 127) << 21)) >>> 0;
    if (this.buf[this.pos++] < 128) return v;
    v = (v | ((this.buf[this.pos] & 15) << 28)) >>> 0;
    return v;
  }

  uint64(): bigint {
    let b = BigInt(0);
    let v = BigInt(0);
    let s = BigInt(0);
    do {
      b = BigInt(this.buf[this.pos]);
      v |= (b & BigInt(0x7f)) << s;
      s += BigInt(7);
      this.pos++;
    } while (this.pos < this.len && b >= BigInt(0x80));
    return v;
  }

  sint32(): number {
    const v = this.uint32();
    return (v >> 1) ^ -(v & 1);
  }

  sint64(): bigint {
    const v = this.uint64();
    return (v >> BigInt(1)) ^ -(v & BigInt(1));
  }

  bool(): boolean {
    return this.uint32() !== 0;
  }

  enum(): number {
    return this.uint32();
  }

  fixed64(): BigInt {
    return (
      BigInt(this.buf[this.pos]) |
      (BigInt(this.buf[this.pos++]) << BigInt(8)) |
      (BigInt(this.buf[this.pos++]) << BigInt(16)) |
      (BigInt(this.buf[this.pos++]) << BigInt(24)) |
      (BigInt(this.buf[this.pos++]) << BigInt(32)) |
      (BigInt(this.buf[this.pos++]) << BigInt(40)) |
      (BigInt(this.buf[this.pos++]) << BigInt(48)) |
      (BigInt(this.buf[this.pos++]) << BigInt(56))
    );
  }

  sfixed64(): BigInt {
    return this.fixed64();
  }

  double(): number {
    const v = readFloat64(this.buf, this.pos);
    this.pos += 8;
    return v;
  }

  string(): string {
    const decoder = new TextDecoder();
    return decoder.decode(this.bytes());
  }

  bytes(): Uint8Array {
    const len = this.uint32();
    const v = this.buf.slice(this.pos, this.pos + len);
    this.pos += len;
    return v;
  }

  fixed32(): number {
    return (
      this.buf[this.pos] |
      (this.buf[this.pos++] << 8) |
      (this.buf[this.pos++] << 16) |
      (this.buf[this.pos++] << 24)
    );
  }

  sfixed32(): number {
    return this.fixed32();
  }

  float(): number {
    const v = readFloat32(this.buf, this.pos);
    this.pos += 4;
    return v;
  }
}
