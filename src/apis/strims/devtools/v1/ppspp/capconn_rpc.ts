import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_devtools_v1_ppspp_ICapConnWatchLogsRequest,
  strims_devtools_v1_ppspp_CapConnWatchLogsRequest,
  strims_devtools_v1_ppspp_CapConnWatchLogsResponse,
  strims_devtools_v1_ppspp_ICapConnLoadLogRequest,
  strims_devtools_v1_ppspp_CapConnLoadLogRequest,
  strims_devtools_v1_ppspp_CapConnLoadLogResponse,
} from "./capconn";

export interface CapConnService {
  watchLogs(req: strims_devtools_v1_ppspp_CapConnWatchLogsRequest, call: strims_rpc_Call): GenericReadable<strims_devtools_v1_ppspp_CapConnWatchLogsResponse>;
  loadLog(req: strims_devtools_v1_ppspp_CapConnLoadLogRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_ppspp_CapConnLoadLogResponse> | strims_devtools_v1_ppspp_CapConnLoadLogResponse;
}

export class UnimplementedCapConnService implements CapConnService {
  watchLogs(req: strims_devtools_v1_ppspp_CapConnWatchLogsRequest, call: strims_rpc_Call): GenericReadable<strims_devtools_v1_ppspp_CapConnWatchLogsResponse> { throw new Error("not implemented"); }
  loadLog(req: strims_devtools_v1_ppspp_CapConnLoadLogRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_ppspp_CapConnLoadLogResponse> | strims_devtools_v1_ppspp_CapConnLoadLogResponse { throw new Error("not implemented"); }
}

export const registerCapConnService = (host: strims_rpc_Service, service: CapConnService): void => {
  host.registerMethod<strims_devtools_v1_ppspp_CapConnWatchLogsRequest, strims_devtools_v1_ppspp_CapConnWatchLogsResponse>("strims.devtools.v1.ppspp.CapConn.WatchLogs", service.watchLogs.bind(service), strims_devtools_v1_ppspp_CapConnWatchLogsRequest);
  host.registerMethod<strims_devtools_v1_ppspp_CapConnLoadLogRequest, strims_devtools_v1_ppspp_CapConnLoadLogResponse>("strims.devtools.v1.ppspp.CapConn.LoadLog", service.loadLog.bind(service), strims_devtools_v1_ppspp_CapConnLoadLogRequest);
}

export class CapConnClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public watchLogs(req?: strims_devtools_v1_ppspp_ICapConnWatchLogsRequest): GenericReadable<strims_devtools_v1_ppspp_CapConnWatchLogsResponse> {
    return this.host.expectMany(this.host.call("strims.devtools.v1.ppspp.CapConn.WatchLogs", new strims_devtools_v1_ppspp_CapConnWatchLogsRequest(req)), strims_devtools_v1_ppspp_CapConnWatchLogsResponse);
  }

  public loadLog(req?: strims_devtools_v1_ppspp_ICapConnLoadLogRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_devtools_v1_ppspp_CapConnLoadLogResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.ppspp.CapConn.LoadLog", new strims_devtools_v1_ppspp_CapConnLoadLogRequest(req)), strims_devtools_v1_ppspp_CapConnLoadLogResponse, opts);
  }
}

