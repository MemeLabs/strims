import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export interface IAny {
  typeUrl?: string;
  value?: Uint8Array;
}

export class Any {
  typeUrl: string = "";
  value: Uint8Array = new Uint8Array();

  constructor(v?: IAny) {
    this.typeUrl = v?.typeUrl || "";
    this.value = v?.value || new Uint8Array();
  }

  static encode(m: Any, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.typeUrl) w.uint32(10).string(m.typeUrl);
    if (m.value) w.uint32(18).bytes(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Any {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Any();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.typeUrl = r.string();
        break;
        case 2:
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

