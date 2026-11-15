// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type ICapConnWatchLogsRequest as strims_devtools_v1_ppspp_ICapConnWatchLogsRequest,
  CapConnWatchLogsRequest as strims_devtools_v1_ppspp_CapConnWatchLogsRequest,
  CapConnWatchLogsResponse as strims_devtools_v1_ppspp_CapConnWatchLogsResponse,
  type ICapConnLoadLogRequest as strims_devtools_v1_ppspp_ICapConnLoadLogRequest,
  CapConnLoadLogRequest as strims_devtools_v1_ppspp_CapConnLoadLogRequest,
  CapConnLoadLogResponse as strims_devtools_v1_ppspp_CapConnLoadLogResponse,
} from "./capconn.pb";

export interface CapConnService {
  watchLogs(req: strims_devtools_v1_ppspp_CapConnWatchLogsRequest, call: strims_rpc_Call): GenericReadable<strims_devtools_v1_ppspp_CapConnWatchLogsResponse>;
  loadLog(req: strims_devtools_v1_ppspp_CapConnLoadLogRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_ppspp_CapConnLoadLogResponse> | strims_devtools_v1_ppspp_CapConnLoadLogResponse;
}

export class UnimplementedCapConnService implements CapConnService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchLogs(req: strims_devtools_v1_ppspp_CapConnWatchLogsRequest, call: strims_rpc_Call): GenericReadable<strims_devtools_v1_ppspp_CapConnWatchLogsResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  loadLog(req: strims_devtools_v1_ppspp_CapConnLoadLogRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_ppspp_CapConnLoadLogResponse> | strims_devtools_v1_ppspp_CapConnLoadLogResponse { throw new Error("not implemented"); }
}

export const registerCapConnService = (host: strims_rpc_Service, service: CapConnService): void => {
  host.registerMethod<strims_devtools_v1_ppspp_CapConnWatchLogsRequest, strims_devtools_v1_ppspp_CapConnWatchLogsResponse>("strims.devtools.v1.ppspp.CapConn.WatchLogs", (req, call) => service.watchLogs(req, call), strims_devtools_v1_ppspp_CapConnWatchLogsRequest);
  host.registerMethod<strims_devtools_v1_ppspp_CapConnLoadLogRequest, strims_devtools_v1_ppspp_CapConnLoadLogResponse>("strims.devtools.v1.ppspp.CapConn.LoadLog", (req, call) => service.loadLog(req, call), strims_devtools_v1_ppspp_CapConnLoadLogRequest);
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

