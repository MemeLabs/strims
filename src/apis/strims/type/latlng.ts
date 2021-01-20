import Reader from "../../../lib/pb/reader";
import Writer from "../../../lib/pb/writer";


export interface ILatLng {
  latitude?: number;
  longitude?: number;
}

export class LatLng {
  latitude: number = 0;
  longitude: number = 0;

  constructor(v?: ILatLng) {
    this.latitude = v?.latitude || 0;
    this.longitude = v?.longitude || 0;
  }

  static encode(m: LatLng, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.latitude) w.uint32(9).double(m.latitude);
    if (m.longitude) w.uint32(17).double(m.longitude);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LatLng {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LatLng();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.latitude = r.double();
        break;
        case 2:
        m.longitude = r.double();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

