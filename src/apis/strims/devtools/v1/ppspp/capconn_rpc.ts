import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
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

export class CapConnClient {
  constructor(private readonly host: RPCHost) {}

  public watchLogs(arg?: ICapConnWatchLogsRequest): GenericReadable<CapConnWatchLogsResponse> {
    return this.host.expectMany(this.host.call("strims.devtools.v1.ppspp.CapConn.WatchLogs", new CapConnWatchLogsRequest(arg)));
  }

  public loadLog(arg?: ICapConnLoadLogRequest): Promise<CapConnLoadLogResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.ppspp.CapConn.LoadLog", new CapConnLoadLogRequest(arg)));
  }
}

