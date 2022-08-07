import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IPProfRequest = {
  name?: string;
  debug?: boolean;
  gc?: boolean;
}

export class PProfRequest {
  name: string;
  debug: boolean;
  gc: boolean;

  constructor(v?: IPProfRequest) {
    this.name = v?.name || "";
    this.debug = v?.debug || false;
    this.gc = v?.gc || false;
  }

  static encode(m: PProfRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
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

export type IPProfResponse = {
  name?: string;
  data?: Uint8Array;
}

export class PProfResponse {
  name: string;
  data: Uint8Array;

  constructor(v?: IPProfResponse) {
    this.name = v?.name || "";
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: PProfResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.data.length) w.uint32(18).bytes(m.data);
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

export type IReadMetricsRequest = {
  format?: strims_debug_v1_MetricsFormat;
}

export class ReadMetricsRequest {
  format: strims_debug_v1_MetricsFormat;

  constructor(v?: IReadMetricsRequest) {
    this.format = v?.format || 0;
  }

  static encode(m: ReadMetricsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type IReadMetricsResponse = {
  data?: Uint8Array;
}

export class ReadMetricsResponse {
  data: Uint8Array;

  constructor(v?: IReadMetricsResponse) {
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: ReadMetricsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data.length) w.uint32(10).bytes(m.data);
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

export type IWatchMetricsRequest = {
  format?: strims_debug_v1_MetricsFormat;
  intervalMs?: number;
}

export class WatchMetricsRequest {
  format: strims_debug_v1_MetricsFormat;
  intervalMs: number;

  constructor(v?: IWatchMetricsRequest) {
    this.format = v?.format || 0;
    this.intervalMs = v?.intervalMs || 0;
  }

  static encode(m: WatchMetricsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.format) w.uint32(8).uint32(m.format);
    if (m.intervalMs) w.uint32(16).int32(m.intervalMs);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchMetricsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchMetricsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.format = r.uint32();
        break;
        case 2:
        m.intervalMs = r.int32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWatchMetricsResponse = {
  data?: Uint8Array;
}

export class WatchMetricsResponse {
  data: Uint8Array;

  constructor(v?: IWatchMetricsResponse) {
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: WatchMetricsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data.length) w.uint32(10).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchMetricsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchMetricsResponse();
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

export type IGetConfigRequest = Record<string, any>;

export class GetConfigRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IGetConfigRequest) {
  }

  static encode(m: GetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetConfigRequest();
  }
}

export type IGetConfigResponse = {
  config?: strims_debug_v1_IConfig;
}

export class GetConfigResponse {
  config: strims_debug_v1_Config | undefined;

  constructor(v?: IGetConfigResponse) {
    this.config = v?.config && new strims_debug_v1_Config(v.config);
  }

  static encode(m: GetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_debug_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_debug_v1_Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetConfigRequest = {
  config?: strims_debug_v1_IConfig;
}

export class SetConfigRequest {
  config: strims_debug_v1_Config | undefined;

  constructor(v?: ISetConfigRequest) {
    this.config = v?.config && new strims_debug_v1_Config(v.config);
  }

  static encode(m: SetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_debug_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_debug_v1_Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetConfigResponse = {
  config?: strims_debug_v1_IConfig;
}

export class SetConfigResponse {
  config: strims_debug_v1_Config | undefined;

  constructor(v?: ISetConfigResponse) {
    this.config = v?.config && new strims_debug_v1_Config(v.config);
  }

  static encode(m: SetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_debug_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_debug_v1_Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IConfig = {
  enableMockStreams?: boolean;
  mockStreamNetworkKey?: Uint8Array;
}

export class Config {
  enableMockStreams: boolean;
  mockStreamNetworkKey: Uint8Array;

  constructor(v?: IConfig) {
    this.enableMockStreams = v?.enableMockStreams || false;
    this.mockStreamNetworkKey = v?.mockStreamNetworkKey || new Uint8Array();
  }

  static encode(m: Config, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enableMockStreams) w.uint32(8).bool(m.enableMockStreams);
    if (m.mockStreamNetworkKey.length) w.uint32(18).bytes(m.mockStreamNetworkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Config {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Config();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enableMockStreams = r.bool();
        break;
        case 2:
        m.mockStreamNetworkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMockStreamSegment = {
  id?: bigint;
  timestamp?: bigint;
  padding?: Uint8Array;
}

export class MockStreamSegment {
  id: bigint;
  timestamp: bigint;
  padding: Uint8Array;

  constructor(v?: IMockStreamSegment) {
    this.id = v?.id || BigInt(0);
    this.timestamp = v?.timestamp || BigInt(0);
    this.padding = v?.padding || new Uint8Array();
  }

  static encode(m: MockStreamSegment, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.timestamp) w.uint32(16).int64(m.timestamp);
    if (m.padding.length) w.uint32(26).bytes(m.padding);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MockStreamSegment {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MockStreamSegment();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.timestamp = r.int64();
        break;
        case 3:
        m.padding = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IStartMockStreamRequest = {
  bitrateKbps?: number;
  segmentIntervalMs?: number;
  timeoutMs?: number;
  networkKey?: Uint8Array;
}

export class StartMockStreamRequest {
  bitrateKbps: number;
  segmentIntervalMs: number;
  timeoutMs: number;
  networkKey: Uint8Array;

  constructor(v?: IStartMockStreamRequest) {
    this.bitrateKbps = v?.bitrateKbps || 0;
    this.segmentIntervalMs = v?.segmentIntervalMs || 0;
    this.timeoutMs = v?.timeoutMs || 0;
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: StartMockStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bitrateKbps) w.uint32(8).uint32(m.bitrateKbps);
    if (m.segmentIntervalMs) w.uint32(16).uint32(m.segmentIntervalMs);
    if (m.timeoutMs) w.uint32(24).uint32(m.timeoutMs);
    if (m.networkKey.length) w.uint32(34).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StartMockStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new StartMockStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bitrateKbps = r.uint32();
        break;
        case 2:
        m.segmentIntervalMs = r.uint32();
        break;
        case 3:
        m.timeoutMs = r.uint32();
        break;
        case 4:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IStartMockStreamResponse = {
  id?: bigint;
}

export class StartMockStreamResponse {
  id: bigint;

  constructor(v?: IStartMockStreamResponse) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: StartMockStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StartMockStreamResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new StartMockStreamResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IStopMockStreamRequest = {
  id?: bigint;
}

export class StopMockStreamRequest {
  id: bigint;

  constructor(v?: IStopMockStreamRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: StopMockStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StopMockStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new StopMockStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IStopMockStreamResponse = Record<string, any>;

export class StopMockStreamResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IStopMockStreamResponse) {
  }

  static encode(m: StopMockStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StopMockStreamResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new StopMockStreamResponse();
  }
}

export enum MetricsFormat {
  METRICS_FORMAT_TEXT = 0,
  METRICS_FORMAT_PROTO_DELIM = 1,
  METRICS_FORMAT_PROTO_TEXT = 2,
  METRICS_FORMAT_PROTO_COMPACT = 3,
  METRICS_FORMAT_OPEN_METRICS = 4,
}
/* @internal */
export const strims_debug_v1_PProfRequest = PProfRequest;
/* @internal */
export type strims_debug_v1_PProfRequest = PProfRequest;
/* @internal */
export type strims_debug_v1_IPProfRequest = IPProfRequest;
/* @internal */
export const strims_debug_v1_PProfResponse = PProfResponse;
/* @internal */
export type strims_debug_v1_PProfResponse = PProfResponse;
/* @internal */
export type strims_debug_v1_IPProfResponse = IPProfResponse;
/* @internal */
export const strims_debug_v1_ReadMetricsRequest = ReadMetricsRequest;
/* @internal */
export type strims_debug_v1_ReadMetricsRequest = ReadMetricsRequest;
/* @internal */
export type strims_debug_v1_IReadMetricsRequest = IReadMetricsRequest;
/* @internal */
export const strims_debug_v1_ReadMetricsResponse = ReadMetricsResponse;
/* @internal */
export type strims_debug_v1_ReadMetricsResponse = ReadMetricsResponse;
/* @internal */
export type strims_debug_v1_IReadMetricsResponse = IReadMetricsResponse;
/* @internal */
export const strims_debug_v1_WatchMetricsRequest = WatchMetricsRequest;
/* @internal */
export type strims_debug_v1_WatchMetricsRequest = WatchMetricsRequest;
/* @internal */
export type strims_debug_v1_IWatchMetricsRequest = IWatchMetricsRequest;
/* @internal */
export const strims_debug_v1_WatchMetricsResponse = WatchMetricsResponse;
/* @internal */
export type strims_debug_v1_WatchMetricsResponse = WatchMetricsResponse;
/* @internal */
export type strims_debug_v1_IWatchMetricsResponse = IWatchMetricsResponse;
/* @internal */
export const strims_debug_v1_GetConfigRequest = GetConfigRequest;
/* @internal */
export type strims_debug_v1_GetConfigRequest = GetConfigRequest;
/* @internal */
export type strims_debug_v1_IGetConfigRequest = IGetConfigRequest;
/* @internal */
export const strims_debug_v1_GetConfigResponse = GetConfigResponse;
/* @internal */
export type strims_debug_v1_GetConfigResponse = GetConfigResponse;
/* @internal */
export type strims_debug_v1_IGetConfigResponse = IGetConfigResponse;
/* @internal */
export const strims_debug_v1_SetConfigRequest = SetConfigRequest;
/* @internal */
export type strims_debug_v1_SetConfigRequest = SetConfigRequest;
/* @internal */
export type strims_debug_v1_ISetConfigRequest = ISetConfigRequest;
/* @internal */
export const strims_debug_v1_SetConfigResponse = SetConfigResponse;
/* @internal */
export type strims_debug_v1_SetConfigResponse = SetConfigResponse;
/* @internal */
export type strims_debug_v1_ISetConfigResponse = ISetConfigResponse;
/* @internal */
export const strims_debug_v1_Config = Config;
/* @internal */
export type strims_debug_v1_Config = Config;
/* @internal */
export type strims_debug_v1_IConfig = IConfig;
/* @internal */
export const strims_debug_v1_MockStreamSegment = MockStreamSegment;
/* @internal */
export type strims_debug_v1_MockStreamSegment = MockStreamSegment;
/* @internal */
export type strims_debug_v1_IMockStreamSegment = IMockStreamSegment;
/* @internal */
export const strims_debug_v1_StartMockStreamRequest = StartMockStreamRequest;
/* @internal */
export type strims_debug_v1_StartMockStreamRequest = StartMockStreamRequest;
/* @internal */
export type strims_debug_v1_IStartMockStreamRequest = IStartMockStreamRequest;
/* @internal */
export const strims_debug_v1_StartMockStreamResponse = StartMockStreamResponse;
/* @internal */
export type strims_debug_v1_StartMockStreamResponse = StartMockStreamResponse;
/* @internal */
export type strims_debug_v1_IStartMockStreamResponse = IStartMockStreamResponse;
/* @internal */
export const strims_debug_v1_StopMockStreamRequest = StopMockStreamRequest;
/* @internal */
export type strims_debug_v1_StopMockStreamRequest = StopMockStreamRequest;
/* @internal */
export type strims_debug_v1_IStopMockStreamRequest = IStopMockStreamRequest;
/* @internal */
export const strims_debug_v1_StopMockStreamResponse = StopMockStreamResponse;
/* @internal */
export type strims_debug_v1_StopMockStreamResponse = StopMockStreamResponse;
/* @internal */
export type strims_debug_v1_IStopMockStreamResponse = IStopMockStreamResponse;
/* @internal */
export const strims_debug_v1_MetricsFormat = MetricsFormat;
/* @internal */
export type strims_debug_v1_MetricsFormat = MetricsFormat;
