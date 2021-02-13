import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type INetworkAddress = {
  hostId?: Uint8Array;
  port?: number;
}

export class NetworkAddress {
  hostId: Uint8Array;
  port: number;

  constructor(v?: INetworkAddress) {
    this.hostId = v?.hostId || new Uint8Array();
    this.port = v?.port || 0;
  }

  static encode(m: NetworkAddress, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.hostId) w.uint32(10).bytes(m.hostId);
    if (m.port) w.uint32(16).uint32(m.port);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkAddress {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkAddress();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.hostId = r.bytes();
        break;
        case 2:
        m.port = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

