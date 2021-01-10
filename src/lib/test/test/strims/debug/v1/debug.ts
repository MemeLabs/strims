import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";


export interface IPProfRequest {
  name?: string;
  debug?: boolean;
  gc?: boolean;
}

export class PProfRequest {
  name: string = "";
  debug: boolean = false;
  gc: boolean = false;

  constructor(v?: IPProfRequest) {
    this.name = v?.name || "";
    this.debug = v?.debug || false;
    this.gc = v?.gc || false;
  }

  static encode(m: PProfRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.name) w.uint32(10).string(m.name);
    if (m.debug) w.uint32(16).bool(m.debug);
    if (m.gc) w.uint32(24).bool(m.gc);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PProfRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PProfRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.debug = r.bool();
        break;
        case 3:
        m.gc = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IPProfResponse {
  name?: string;
  data?: Uint8Array;
}

export class PProfResponse {
  name: string = "";
  data: Uint8Array = new Uint8Array();

  constructor(v?: IPProfResponse) {
    this.name = v?.name || "";
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: PProfResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.name) w.uint32(10).string(m.name);
    if (m.data) w.uint32(18).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PProfResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PProfResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
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

export interface IReadMetricsRequest {
  format?: MetricsFormat;
}

export class ReadMetricsRequest {
  format: MetricsFormat = 0;

  constructor(v?: IReadMetricsRequest) {
    this.format = v?.format || 0;
  }

  static encode(m: ReadMetricsRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.format) w.uint32(8).uint32(m.format);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ReadMetricsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ReadMetricsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.format = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IReadMetricsResponse {
  data?: Uint8Array;
}

export class ReadMetricsResponse {
  data: Uint8Array = new Uint8Array();

  constructor(v?: IReadMetricsResponse) {
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: ReadMetricsResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.data) w.uint32(10).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ReadMetricsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ReadMetricsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export enum MetricsFormat {
  METRICS_FORMAT_TEXT = 0,
  METRICS_FORMAT_PROTO_DELIM = 1,
  METRICS_FORMAT_PROTO_TEXT = 2,
  METRICS_FORMAT_PROTO_COMPACT = 3,
  METRICS_FORMAT_OPEN_METRICS = 4,
}
