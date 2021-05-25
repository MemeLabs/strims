import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IPProfRequest,
  PProfRequest,
  PProfResponse,
  IReadMetricsRequest,
  ReadMetricsRequest,
  ReadMetricsResponse,
} from "./debug";

registerType("strims.debug.v1.PProfRequest", PProfRequest);
registerType("strims.debug.v1.PProfResponse", PProfResponse);
registerType("strims.debug.v1.ReadMetricsRequest", ReadMetricsRequest);
registerType("strims.debug.v1.ReadMetricsResponse", ReadMetricsResponse);

export interface DebugService {
  pProf(req: PProfRequest, call: strims_rpc_Call): Promise<PProfResponse> | PProfResponse;
  readMetrics(req: ReadMetricsRequest, call: strims_rpc_Call): Promise<ReadMetricsResponse> | ReadMetricsResponse;
}

export const registerDebugService = (host: strims_rpc_Service, service: DebugService): void => {
  host.registerMethod<PProfRequest, PProfResponse>("strims.debug.v1.Debug.PProf", service.pProf.bind(service));
  host.registerMethod<ReadMetricsRequest, ReadMetricsResponse>("strims.debug.v1.Debug.ReadMetrics", service.readMetrics.bind(service));
}

export class DebugClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public pProf(req?: IPProfRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PProfResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.PProf", new PProfRequest(req)), opts);
  }

  public readMetrics(req?: IReadMetricsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ReadMetricsResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.ReadMetrics", new ReadMetricsRequest(req)), opts);
  }
}

