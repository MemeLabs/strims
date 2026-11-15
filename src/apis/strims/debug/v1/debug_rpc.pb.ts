// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type IReadMetricsRequest as strims_debug_v1_IReadMetricsRequest,
  ReadMetricsRequest as strims_debug_v1_ReadMetricsRequest,
  ReadMetricsResponse as strims_debug_v1_ReadMetricsResponse,
  type IWatchMetricsRequest as strims_debug_v1_IWatchMetricsRequest,
  WatchMetricsRequest as strims_debug_v1_WatchMetricsRequest,
  WatchMetricsResponse as strims_debug_v1_WatchMetricsResponse,
  type IGetConfigRequest as strims_debug_v1_IGetConfigRequest,
  GetConfigRequest as strims_debug_v1_GetConfigRequest,
  GetConfigResponse as strims_debug_v1_GetConfigResponse,
  type ISetConfigRequest as strims_debug_v1_ISetConfigRequest,
  SetConfigRequest as strims_debug_v1_SetConfigRequest,
  SetConfigResponse as strims_debug_v1_SetConfigResponse,
  type IStartMockStreamRequest as strims_debug_v1_IStartMockStreamRequest,
  StartMockStreamRequest as strims_debug_v1_StartMockStreamRequest,
  StartMockStreamResponse as strims_debug_v1_StartMockStreamResponse,
  type IStopMockStreamRequest as strims_debug_v1_IStopMockStreamRequest,
  StopMockStreamRequest as strims_debug_v1_StopMockStreamRequest,
  StopMockStreamResponse as strims_debug_v1_StopMockStreamResponse,
  type IListMockStreamsRequest as strims_debug_v1_IListMockStreamsRequest,
  ListMockStreamsRequest as strims_debug_v1_ListMockStreamsRequest,
  ListMockStreamsResponse as strims_debug_v1_ListMockStreamsResponse,
} from "./debug.pb";

export interface DebugService {
  readMetrics(req: strims_debug_v1_ReadMetricsRequest, call: strims_rpc_Call): Promise<strims_debug_v1_ReadMetricsResponse> | strims_debug_v1_ReadMetricsResponse;
  watchMetrics(req: strims_debug_v1_WatchMetricsRequest, call: strims_rpc_Call): GenericReadable<strims_debug_v1_WatchMetricsResponse>;
  getConfig(req: strims_debug_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_GetConfigResponse> | strims_debug_v1_GetConfigResponse;
  setConfig(req: strims_debug_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_SetConfigResponse> | strims_debug_v1_SetConfigResponse;
  startMockStream(req: strims_debug_v1_StartMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StartMockStreamResponse> | strims_debug_v1_StartMockStreamResponse;
  stopMockStream(req: strims_debug_v1_StopMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StopMockStreamResponse> | strims_debug_v1_StopMockStreamResponse;
  listMockStreams(req: strims_debug_v1_ListMockStreamsRequest, call: strims_rpc_Call): Promise<strims_debug_v1_ListMockStreamsResponse> | strims_debug_v1_ListMockStreamsResponse;
}

export class UnimplementedDebugService implements DebugService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  readMetrics(req: strims_debug_v1_ReadMetricsRequest, call: strims_rpc_Call): Promise<strims_debug_v1_ReadMetricsResponse> | strims_debug_v1_ReadMetricsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchMetrics(req: strims_debug_v1_WatchMetricsRequest, call: strims_rpc_Call): GenericReadable<strims_debug_v1_WatchMetricsResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getConfig(req: strims_debug_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_GetConfigResponse> | strims_debug_v1_GetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setConfig(req: strims_debug_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_SetConfigResponse> | strims_debug_v1_SetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  startMockStream(req: strims_debug_v1_StartMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StartMockStreamResponse> | strims_debug_v1_StartMockStreamResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  stopMockStream(req: strims_debug_v1_StopMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StopMockStreamResponse> | strims_debug_v1_StopMockStreamResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listMockStreams(req: strims_debug_v1_ListMockStreamsRequest, call: strims_rpc_Call): Promise<strims_debug_v1_ListMockStreamsResponse> | strims_debug_v1_ListMockStreamsResponse { throw new Error("not implemented"); }
}

export const registerDebugService = (host: strims_rpc_Service, service: DebugService): void => {
  host.registerMethod<strims_debug_v1_ReadMetricsRequest, strims_debug_v1_ReadMetricsResponse>("strims.debug.v1.Debug.ReadMetrics", (req, call) => service.readMetrics(req, call), strims_debug_v1_ReadMetricsRequest);
  host.registerMethod<strims_debug_v1_WatchMetricsRequest, strims_debug_v1_WatchMetricsResponse>("strims.debug.v1.Debug.WatchMetrics", (req, call) => service.watchMetrics(req, call), strims_debug_v1_WatchMetricsRequest);
  host.registerMethod<strims_debug_v1_GetConfigRequest, strims_debug_v1_GetConfigResponse>("strims.debug.v1.Debug.GetConfig", (req, call) => service.getConfig(req, call), strims_debug_v1_GetConfigRequest);
  host.registerMethod<strims_debug_v1_SetConfigRequest, strims_debug_v1_SetConfigResponse>("strims.debug.v1.Debug.SetConfig", (req, call) => service.setConfig(req, call), strims_debug_v1_SetConfigRequest);
  host.registerMethod<strims_debug_v1_StartMockStreamRequest, strims_debug_v1_StartMockStreamResponse>("strims.debug.v1.Debug.StartMockStream", (req, call) => service.startMockStream(req, call), strims_debug_v1_StartMockStreamRequest);
  host.registerMethod<strims_debug_v1_StopMockStreamRequest, strims_debug_v1_StopMockStreamResponse>("strims.debug.v1.Debug.StopMockStream", (req, call) => service.stopMockStream(req, call), strims_debug_v1_StopMockStreamRequest);
  host.registerMethod<strims_debug_v1_ListMockStreamsRequest, strims_debug_v1_ListMockStreamsResponse>("strims.debug.v1.Debug.ListMockStreams", (req, call) => service.listMockStreams(req, call), strims_debug_v1_ListMockStreamsRequest);
}

export class DebugClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public readMetrics(req?: strims_debug_v1_IReadMetricsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_ReadMetricsResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.ReadMetrics", new strims_debug_v1_ReadMetricsRequest(req)), strims_debug_v1_ReadMetricsResponse, opts);
  }

  public watchMetrics(req?: strims_debug_v1_IWatchMetricsRequest): GenericReadable<strims_debug_v1_WatchMetricsResponse> {
    return this.host.expectMany(this.host.call("strims.debug.v1.Debug.WatchMetrics", new strims_debug_v1_WatchMetricsRequest(req)), strims_debug_v1_WatchMetricsResponse);
  }

  public getConfig(req?: strims_debug_v1_IGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_GetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.GetConfig", new strims_debug_v1_GetConfigRequest(req)), strims_debug_v1_GetConfigResponse, opts);
  }

  public setConfig(req?: strims_debug_v1_ISetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_SetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.SetConfig", new strims_debug_v1_SetConfigRequest(req)), strims_debug_v1_SetConfigResponse, opts);
  }

  public startMockStream(req?: strims_debug_v1_IStartMockStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_StartMockStreamResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.StartMockStream", new strims_debug_v1_StartMockStreamRequest(req)), strims_debug_v1_StartMockStreamResponse, opts);
  }

  public stopMockStream(req?: strims_debug_v1_IStopMockStreamRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_StopMockStreamResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.StopMockStream", new strims_debug_v1_StopMockStreamRequest(req)), strims_debug_v1_StopMockStreamResponse, opts);
  }

  public listMockStreams(req?: strims_debug_v1_IListMockStreamsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_ListMockStreamsResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.ListMockStreams", new strims_debug_v1_ListMockStreamsRequest(req)), strims_debug_v1_ListMockStreamsResponse, opts);
  }
}

