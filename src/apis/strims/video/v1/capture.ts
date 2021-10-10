import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  ListingSnippet as strims_network_v1_directory_ListingSnippet,
  IListingSnippet as strims_network_v1_directory_IListingSnippet,
} from "../../network/v1/directory/directory";

export type ICaptureOpenRequest = {
  directorySnippet?: strims_network_v1_directory_IListingSnippet | undefined;
  mimeType?: string;
  networkKeys?: Uint8Array[];
}

export class CaptureOpenRequest {
  directorySnippet: strims_network_v1_directory_ListingSnippet | undefined;
  mimeType: string;
  networkKeys: Uint8Array[];

  constructor(v?: ICaptureOpenRequest) {
    this.directorySnippet = v?.directorySnippet && new strims_network_v1_directory_ListingSnippet(v.directorySnippet);
    this.mimeType = v?.mimeType || "";
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
  }

  static encode(m: CaptureOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directorySnippet) strims_network_v1_directory_ListingSnippet.encode(m.directorySnippet, w.uint32(10).fork()).ldelim();
    if (m.mimeType) w.uint32(18).string(m.mimeType);
    for (const v of m.networkKeys) w.uint32(26).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureOpenRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureOpenRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.directorySnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
        break;
        case 2:
        m.mimeType = r.string();
        break;
        case 3:
        m.networkKeys.push(r.bytes())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureOpenResponse = {
  id?: Uint8Array;
}

export class CaptureOpenResponse {
  id: Uint8Array;

  constructor(v?: ICaptureOpenResponse) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: CaptureOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(10).bytes(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureOpenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureOpenResponse();
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

export type ICaptureUpdateRequest = {
  id?: Uint8Array;
  directorySnippet?: strims_network_v1_directory_IListingSnippet | undefined;
}

export class CaptureUpdateRequest {
  id: Uint8Array;
  directorySnippet: strims_network_v1_directory_ListingSnippet | undefined;

  constructor(v?: ICaptureUpdateRequest) {
    this.id = v?.id || new Uint8Array();
    this.directorySnippet = v?.directorySnippet && new strims_network_v1_directory_ListingSnippet(v.directorySnippet);
  }

  static encode(m: CaptureUpdateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(10).bytes(m.id);
    if (m.directorySnippet) strims_network_v1_directory_ListingSnippet.encode(m.directorySnippet, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureUpdateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureUpdateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        case 2:
        m.directorySnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureUpdateResponse = {
}

export class CaptureUpdateResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICaptureUpdateResponse) {
  }

  static encode(m: CaptureUpdateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureUpdateResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CaptureUpdateResponse();
  }
}

export type ICaptureAppendRequest = {
  id?: Uint8Array;
  data?: Uint8Array;
  segmentEnd?: boolean;
}

export class CaptureAppendRequest {
  id: Uint8Array;
  data: Uint8Array;
  segmentEnd: boolean;

  constructor(v?: ICaptureAppendRequest) {
    this.id = v?.id || new Uint8Array();
    this.data = v?.data || new Uint8Array();
    this.segmentEnd = v?.segmentEnd || false;
  }

  static encode(m: CaptureAppendRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(10).bytes(m.id);
    if (m.data) w.uint32(18).bytes(m.data);
    if (m.segmentEnd) w.uint32(24).bool(m.segmentEnd);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureAppendRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureAppendRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        case 2:
        m.data = r.bytes();
        break;
        case 3:
        m.segmentEnd = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICaptureAppendResponse = {
}

export class CaptureAppendResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICaptureAppendResponse) {
  }

  static encode(m: CaptureAppendResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureAppendResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CaptureAppendResponse();
  }
}

export type ICaptureCloseRequest = {
  id?: Uint8Array;
}

export class CaptureCloseRequest {
  id: Uint8Array;

  constructor(v?: ICaptureCloseRequest) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: CaptureCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(10).bytes(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureCloseRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CaptureCloseRequest();
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

export type ICaptureCloseResponse = {
}

export class CaptureCloseResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICaptureCloseResponse) {
  }

  static encode(m: CaptureCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CaptureCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CaptureCloseResponse();
  }
}

