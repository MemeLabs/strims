import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type ITimestamp = {
  seconds?: bigint;
  nanos?: number;
}

export class Timestamp {
  seconds: bigint;
  nanos: number;

  constructor(v?: ITimestamp) {
    this.seconds = v?.seconds || BigInt(0);
    this.nanos = v?.nanos || 0;
  }

  static encode(m: Timestamp, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.seconds) w.uint32(8).int64(m.seconds);
    if (m.nanos) w.uint32(16).int32(m.nanos);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Timestamp {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Timestamp();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.seconds = r.int64();
        break;
        case 2:
        m.nanos = r.int32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

