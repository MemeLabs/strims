import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

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

export class DebugClient {
  constructor(private readonly host: RPCHost) {}

  public pProf(arg?: IPProfRequest): Promise<PProfResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.PProf", new PProfRequest(arg)));
  }

  public readMetrics(arg?: IReadMetricsRequest): Promise<ReadMetricsResponse> {
    return this.host.expectOne(this.host.call("strims.debug.v1.Debug.ReadMetrics", new ReadMetricsRequest(arg)));
  }
}

