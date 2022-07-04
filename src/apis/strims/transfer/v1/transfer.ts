import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type ITransfer = {
  id?: Uint8Array;
}

export class Transfer {
  id: Uint8Array;

  constructor(v?: ITransfer) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: Transfer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
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

/* @internal */
export const strims_transfer_v1_Transfer = Transfer;
/* @internal */
export type strims_transfer_v1_Transfer = Transfer;
/* @internal */
export type strims_transfer_v1_ITransfer = ITransfer;
