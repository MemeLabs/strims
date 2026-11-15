// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Metrics as strims_debug_v1_Metrics,
  type IMetrics as strims_debug_v1_IMetrics,
  MockStream as strims_debug_v1_MockStream,
  type IMockStream as strims_debug_v1_IMockStream,
} from "./metrics.pb";

export type IReadMetricsRequest = object;

export class ReadMetricsRequest {

  constructor(v?: IReadMetricsRequest) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ReadMetricsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.metrics = strims_debug_v1_Metrics.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IWatchMetricsRequest = {
  /** 0 or less means 1 s. The service clamps it to its bounds. */
  intervalMs?: number;
}

export class WatchMetricsRequest {
  /** 0 or less means 1 s. The service clamps it to its bounds. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchMetricsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.intervalMs = r.int32();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchMetricsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.metrics = strims_debug_v1_Metrics.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IGetConfigRequest = object;

export class GetConfigRequest {

  constructor(v?: IGetConfigRequest) {
    void v; // an empty message has no fields to set
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
  config?: IConfig;
}

export class GetConfigResponse {
  config: Config | undefined;

  constructor(v?: IGetConfigResponse) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: GetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ISetConfigRequest = {
  config?: IConfig;
}

export class SetConfigRequest {
  config: Config | undefined;

  constructor(v?: ISetConfigRequest) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: SetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ISetConfigResponse = {
  config?: IConfig;
}

export class SetConfigResponse {
  config: Config | undefined;

  constructor(v?: ISetConfigResponse) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: SetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/** One record of a mock stream's swarm. */
export type IMockStreamSegment = {
  /** The mock stream's id. */
  id?: bigint;
  /** When it was written, in Unix microseconds. */
  timestamp?: bigint;
  /** Zeros that bring the stream to its bitrate. */
  padding?: Uint8Array;
}

/** One record of a mock stream's swarm. */
export class MockStreamSegment {
  /** The mock stream's id. */
  id: bigint;
  /** When it was written, in Unix microseconds. */
  timestamp: bigint;
  /** Zeros that bring the stream to its bitrate. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IStartMockStreamRequest = {
  bitrateKbps?: number;
  /** 0 means 1 s. */
  segmentIntervalMs?: number;
  /** 0 runs the stream until it's stopped. */
  timeoutMs?: number;
  networkKey?: Uint8Array;
}

export class StartMockStreamRequest {
  bitrateKbps: number;
  /** 0 means 1 s. */
  segmentIntervalMs: number;
  /** 0 runs the stream until it's stopped. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StartMockStreamResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/** Stopping an id no mock stream has does nothing. */
export type IStopMockStreamRequest = {
  id?: bigint;
}

/** Stopping an id no mock stream has does nothing. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StopMockStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IStopMockStreamResponse = object;

export class StopMockStreamResponse {

  constructor(v?: IStopMockStreamResponse) {
    void v; // an empty message has no fields to set
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

export type IListMockStreamsRequest = object;

export class ListMockStreamsRequest {

  constructor(v?: IListMockStreamsRequest) {
    void v; // an empty message has no fields to set
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
  /** In id order. */
  mockStreams?: strims_debug_v1_IMockStream[];
}

export class ListMockStreamsResponse {
  /** In id order. */
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListMockStreamsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mockStreams.push(strims_debug_v1_MockStream.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

