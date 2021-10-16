import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IImage = {
  type?: ImageType;
  height?: number;
  width?: number;
  data?: Uint8Array;
}

export class Image {
  type: ImageType;
  height: number;
  width: number;
  data: Uint8Array;

  constructor(v?: IImage) {
    this.type = v?.type || 0;
    this.height = v?.height || 0;
    this.width = v?.width || 0;
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: Image, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.type) w.uint32(8).uint32(m.type);
    if (m.height) w.uint32(16).uint32(m.height);
    if (m.width) w.uint32(24).uint32(m.width);
    if (m.data) w.uint32(34).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Image {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Image();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.type = r.uint32();
        break;
        case 2:
        m.height = r.uint32();
        break;
        case 3:
        m.width = r.uint32();
        break;
        case 4:
        m.data = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export enum ImageType {
  IMAGE_TYPE_UNDEFINED = 0,
  IMAGE_TYPE_APNG = 1,
  IMAGE_TYPE_AVIF = 2,
  IMAGE_TYPE_GIF = 3,
  IMAGE_TYPE_JPEG = 4,
  IMAGE_TYPE_PNG = 5,
  IMAGE_TYPE_WEBP = 6,
}
