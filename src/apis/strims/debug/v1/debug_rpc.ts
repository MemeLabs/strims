import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_debug_v1_IPProfRequest,
  strims_debug_v1_PProfRequest,
  strims_debug_v1_PProfResponse,
  strims_debug_v1_IReadMetricsRequest,
  strims_debug_v1_ReadMetricsRequest,
  strims_debug_v1_ReadMetricsResponse,
  strims_debug_v1_IWatchMetricsRequest,
  strims_debug_v1_WatchMetricsRequest,
  strims_debug_v1_WatchMetricsResponse,
  strims_debug_v1_IGetConfigRequest,
  strims_debug_v1_GetConfigRequest,
  strims_debug_v1_GetConfigResponse,
  strims_debug_v1_ISetConfigRequest,
  strims_debug_v1_SetConfigRequest,
  strims_debug_v1_SetConfigResponse,
  strims_debug_v1_IStartMockStreamRequest,
  strims_debug_v1_StartMockStreamRequest,
  strims_debug_v1_StartMockStreamResponse,
  strims_debug_v1_IStopMockStreamRequest,
  strims_debug_v1_StopMockStreamRequest,
  strims_debug_v1_StopMockStreamResponse,
} from "./debug";

export interface DebugService {
  pProf(req: strims_debug_v1_PProfRequest, call: strims_rpc_Call): Promise<strims_debug_v1_PProfResponse> | strims_debug_v1_PProfResponse;
  readMetrics(req: strims_debug_v1_ReadMetricsRequest, call: strims_rpc_Call): Promise<strims_debug_v1_ReadMetricsResponse> | strims_debug_v1_ReadMetricsResponse;
  watchMetrics(req: strims_debug_v1_WatchMetricsRequest, call: strims_rpc_Call): GenericReadable<strims_debug_v1_WatchMetricsResponse>;
  getConfig(req: strims_debug_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_GetConfigResponse> | strims_debug_v1_GetConfigResponse;
  setConfig(req: strims_debug_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_SetConfigResponse> | strims_debug_v1_SetConfigResponse;
  startMockStream(req: strims_debug_v1_StartMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StartMockStreamResponse> | strims_debug_v1_StartMockStreamResponse;
  stopMockStream(req: strims_debug_v1_StopMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StopMockStreamResponse> | strims_debug_v1_StopMockStreamResponse;
}

export class UnimplementedDebugService implements DebugService {
  pProf(req: strims_debug_v1_PProfRequest, call: strims_rpc_Call): Promise<strims_debug_v1_PProfResponse> | strims_debug_v1_PProfResponse { throw new Error("not implemented"); }
  readMetrics(req: strims_debug_v1_ReadMetricsRequest, call: strims_rpc_Call): Promise<strims_debug_v1_ReadMetricsResponse> | strims_debug_v1_ReadMetricsResponse { throw new Error("not implemented"); }
  watchMetrics(req: strims_debug_v1_WatchMetricsRequest, call: strims_rpc_Call): GenericReadable<strims_debug_v1_WatchMetricsResponse> { throw new Error("not implemented"); }
  getConfig(req: strims_debug_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_GetConfigResponse> | strims_debug_v1_GetConfigResponse { throw new Error("not implemented"); }
  setConfig(req: strims_debug_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_debug_v1_SetConfigResponse> | strims_debug_v1_SetConfigResponse { throw new Error("not implemented"); }
  startMockStream(req: strims_debug_v1_StartMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StartMockStreamResponse> | strims_debug_v1_StartMockStreamResponse { throw new Error("not implemented"); }
  stopMockStream(req: strims_debug_v1_StopMockStreamRequest, call: strims_rpc_Call): Promise<strims_debug_v1_StopMockStreamResponse> | strims_debug_v1_StopMockStreamResponse { throw new Error("not implemented"); }
}

export const registerDebugService = (host: strims_rpc_Service, service: DebugService): void => {
  host.registerMethod<strims_debug_v1_PProfRequest, strims_debug_v1_PProfResponse>("strims.debug.v1.Debug.PProf", service.pProf.bind(service), strims_debug_v1_PProfRequest);
  host.registerMethod<strims_debug_v1_ReadMetricsRequest, strims_debug_v1_ReadMetricsResponse>("strims.debug.v1.Debug.ReadMetrics", service.readMetrics.bind(service), strims_debug_v1_ReadMetricsRequest);
  host.registerMethod<strims_debug_v1_WatchMetricsRequest, strims_debug_v1_WatchMetricsResponse>("strims.debug.v1.Debug.WatchMetrics", service.watchMetrics.bind(service), strims_debug_v1_WatchMetricsRequest);
  host.registerMethod<strims_debug_v1_GetConfigRequest, strims_debug_v1_GetConfigResponse>("strims.debug.v1.Debug.GetConfig", service.getConfig.bind(service), strims_debug_v1_GetConfigRequest);
  host.registerMethod<strims_debug_v1_SetConfigRequest, strims_debug_v1_SetConfigResponse>("strims.debug.v1.Debug.SetConfig", service.setConfig.bind(service), strims_debug_v1_SetConfigRequest);
  host.registerMethod<strims_debug_v1_StartMockStreamRequest, strims_debug_v1_StartMockStreamResponse>("strims.debug.v1.Debug.StartMockStream", service.startMockStream.bind(service), strims_debug_v1_StartMockStreamRequest);
  host.registerMethod<strims_debug_v1_StopMockStreamRequest, strims_debug_v1_StopMockStreamResponse>("strims.debug.v1.Debug.StopMockStream", service.stopMockStream.bind(service), strims_debug_v1_StopMockStreamRequest);
}

export class DebugClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public pProf(req?: strims_debug_v1_IPProfRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_debug_v1_PProfResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.PProf", new strims_debug_v1_PProfRequest(req)), strims_debug_v1_PProfResponse, opts);
  }

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
}

