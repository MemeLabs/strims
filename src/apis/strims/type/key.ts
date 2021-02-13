import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IKey = {
  type?: KeyType;
  private?: Uint8Array;
  public?: Uint8Array;
}

export class Key {
  type: KeyType;
  private: Uint8Array;
  public: Uint8Array;

  constructor(v?: IKey) {
    this.type = v?.type || 0;
    this.private = v?.private || new Uint8Array();
    this.public = v?.public || new Uint8Array();
  }

  static encode(m: Key, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.type) w.uint32(8).uint32(m.type);
    if (m.private) w.uint32(18).bytes(m.private);
    if (m.public) w.uint32(26).bytes(m.public);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Key {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Key();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.type = r.uint32();
        break;
        case 2:
        m.private = r.bytes();
        break;
        case 3:
        m.public = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export enum KeyType {
  KEY_TYPE_UNDEFINED = 0,
  KEY_TYPE_ED25519 = 1,
  KEY_TYPE_X25519 = 2,
}
