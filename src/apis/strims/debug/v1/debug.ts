import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_debug_v1_Metrics,
  strims_debug_v1_IMetrics,
  strims_debug_v1_MockStream,
  strims_debug_v1_IMockStream,
} from "./metrics";

export type IReadMetricsRequest = Record<string, any>;

export class ReadMetricsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IReadMetricsRequest) {
  }

  static encode(m: ReadMetricsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ReadMetricsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ReadMetricsRequest();
  }
}

export type IReadMetricsResponse = {
  metrics?: strims_debug_v1_IMetrics;
}

export class ReadMetricsResponse {
  metrics: strims_debug_v1_Metrics | undefined;

  constructor(v?: IReadMetricsResponse) {
    this.metrics = v?.metrics && new strims_debug_v1_Metrics(v.metrics);
  }

  static encode(m: ReadMetricsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.metrics) strims_debug_v1_Metrics.encode(m.metrics, w.uint32(10).fork()).ldelim();
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
        m.metrics = strims_debug_v1_Metrics.decode(r, r.uint32());
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
  intervalMs?: number;
}

export class WatchMetricsRequest {
  intervalMs: number;

  constructor(v?: IWatchMetricsRequest) {
    this.intervalMs = v?.intervalMs || 0;
  }

  static encode(m: WatchMetricsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.intervalMs) w.uint32(8).int32(m.intervalMs);
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
  metrics?: strims_debug_v1_IMetrics;
}

export class WatchMetricsResponse {
  metrics: strims_debug_v1_Metrics | undefined;

  constructor(v?: IWatchMetricsResponse) {
    this.metrics = v?.metrics && new strims_debug_v1_Metrics(v.metrics);
  }

  static encode(m: WatchMetricsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.metrics) strims_debug_v1_Metrics.encode(m.metrics, w.uint32(10).fork()).ldelim();
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
        m.metrics = strims_debug_v1_Metrics.decode(r, r.uint32());
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

export type IListMockStreamsRequest = Record<string, any>;

export class ListMockStreamsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListMockStreamsRequest) {
  }

  static encode(m: ListMockStreamsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListMockStreamsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListMockStreamsRequest();
  }
}

export type IListMockStreamsResponse = {
  mockStreams?: strims_debug_v1_IMockStream[];
}

export class ListMockStreamsResponse {
  mockStreams: strims_debug_v1_MockStream[];

  constructor(v?: IListMockStreamsResponse) {
    this.mockStreams = v?.mockStreams ? v.mockStreams.map(v => new strims_debug_v1_MockStream(v)) : [];
  }

  static encode(m: ListMockStreamsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.mockStreams) strims_debug_v1_MockStream.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListMockStreamsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListMockStreamsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mockStreams.push(strims_debug_v1_MockStream.decode(r, r.uint32()));
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
export const strims_debug_v1_ListMockStreamsRequest = ListMockStreamsRequest;
/* @internal */
export type strims_debug_v1_ListMockStreamsRequest = ListMockStreamsRequest;
/* @internal */
export type strims_debug_v1_IListMockStreamsRequest = IListMockStreamsRequest;
/* @internal */
export const strims_debug_v1_ListMockStreamsResponse = ListMockStreamsResponse;
/* @internal */
export type strims_debug_v1_ListMockStreamsResponse = ListMockStreamsResponse;
/* @internal */
export type strims_debug_v1_IListMockStreamsResponse = IListMockStreamsResponse;
