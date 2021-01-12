import Reader from "../../../../lib/pb/reader";
import Writer from "../../../../lib/pb/writer";


export interface ITransfer {
  id?: Uint8Array;
}

export class Transfer {
  id: Uint8Array = new Uint8Array();

  constructor(v?: ITransfer) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: Transfer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(10).bytes(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Transfer {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Transfer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

