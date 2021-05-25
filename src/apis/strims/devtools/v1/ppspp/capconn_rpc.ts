import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  ICapConnWatchLogsRequest,
  CapConnWatchLogsRequest,
  CapConnWatchLogsResponse,
  ICapConnLoadLogRequest,
  CapConnLoadLogRequest,
  CapConnLoadLogResponse,
} from "./capconn";

registerType("strims.devtools.v1.ppspp.CapConnWatchLogsRequest", CapConnWatchLogsRequest);
registerType("strims.devtools.v1.ppspp.CapConnWatchLogsResponse", CapConnWatchLogsResponse);
registerType("strims.devtools.v1.ppspp.CapConnLoadLogRequest", CapConnLoadLogRequest);
registerType("strims.devtools.v1.ppspp.CapConnLoadLogResponse", CapConnLoadLogResponse);

export interface CapConnService {
  watchLogs(req: CapConnWatchLogsRequest, call: strims_rpc_Call): GenericReadable<CapConnWatchLogsResponse>;
  loadLog(req: CapConnLoadLogRequest, call: strims_rpc_Call): Promise<CapConnLoadLogResponse> | CapConnLoadLogResponse;
}

export const registerCapConnService = (host: strims_rpc_Service, service: CapConnService): void => {
  host.registerMethod<CapConnWatchLogsRequest, CapConnWatchLogsResponse>("strims.devtools.v1.ppspp.CapConn.WatchLogs", service.watchLogs.bind(service));
  host.registerMethod<CapConnLoadLogRequest, CapConnLoadLogResponse>("strims.devtools.v1.ppspp.CapConn.LoadLog", service.loadLog.bind(service));
}

export class CapConnClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public watchLogs(req?: ICapConnWatchLogsRequest): GenericReadable<CapConnWatchLogsResponse> {
    return this.host.expectMany(this.host.call("strims.devtools.v1.ppspp.CapConn.WatchLogs", new CapConnWatchLogsRequest(req)));
  }

  public loadLog(req?: ICapConnLoadLogRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CapConnLoadLogResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.ppspp.CapConn.LoadLog", new CapConnLoadLogRequest(req)), opts);
  }
}

