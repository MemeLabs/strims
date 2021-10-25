import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  IPProfRequest,
  PProfRequest,
  PProfResponse,
  IReadMetricsRequest,
  ReadMetricsRequest,
  ReadMetricsResponse,
  IWatchMetricsRequest,
  WatchMetricsRequest,
  WatchMetricsResponse,
} from "./debug";

export interface DebugService {
  pProf(req: PProfRequest, call: strims_rpc_Call): Promise<PProfResponse> | PProfResponse;
  readMetrics(req: ReadMetricsRequest, call: strims_rpc_Call): Promise<ReadMetricsResponse> | ReadMetricsResponse;
  watchMetrics(req: WatchMetricsRequest, call: strims_rpc_Call): GenericReadable<WatchMetricsResponse>;
}

export const registerDebugService = (host: strims_rpc_Service, service: DebugService): void => {
  host.registerMethod<PProfRequest, PProfResponse>("strims.debug.v1.Debug.PProf", service.pProf.bind(service), PProfRequest);
  host.registerMethod<ReadMetricsRequest, ReadMetricsResponse>("strims.debug.v1.Debug.ReadMetrics", service.readMetrics.bind(service), ReadMetricsRequest);
  host.registerMethod<WatchMetricsRequest, WatchMetricsResponse>("strims.debug.v1.Debug.WatchMetrics", service.watchMetrics.bind(service), WatchMetricsRequest);
}

export class DebugClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public pProf(req?: IPProfRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PProfResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.PProf", new PProfRequest(req)), PProfResponse, opts);
  }

  public readMetrics(req?: IReadMetricsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ReadMetricsResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.ReadMetrics", new ReadMetricsRequest(req)), ReadMetricsResponse, opts);
  }

  public watchMetrics(req?: IWatchMetricsRequest): GenericReadable<WatchMetricsResponse> {
    return this.host.expectMany(this.host.call("strims.debug.v1.Debug.WatchMetrics", new WatchMetricsRequest(req)), WatchMetricsResponse);
  }
}

