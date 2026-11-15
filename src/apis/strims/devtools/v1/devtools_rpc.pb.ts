// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IDevToolsTestRequest as strims_devtools_v1_IDevToolsTestRequest,
  DevToolsTestRequest as strims_devtools_v1_DevToolsTestRequest,
  DevToolsTestResponse as strims_devtools_v1_DevToolsTestResponse,
} from "./devtools.pb";

export interface DevToolsService {
  test(req: strims_devtools_v1_DevToolsTestRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_DevToolsTestResponse> | strims_devtools_v1_DevToolsTestResponse;
}

export class UnimplementedDevToolsService implements DevToolsService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  test(req: strims_devtools_v1_DevToolsTestRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_DevToolsTestResponse> | strims_devtools_v1_DevToolsTestResponse { throw new Error("not implemented"); }
}

export const registerDevToolsService = (host: strims_rpc_Service, service: DevToolsService): void => {
  host.registerMethod<strims_devtools_v1_DevToolsTestRequest, strims_devtools_v1_DevToolsTestResponse>("strims.devtools.v1.DevTools.Test", (req, call) => service.test(req, call), strims_devtools_v1_DevToolsTestRequest);
}

export class DevToolsClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public test(req?: strims_devtools_v1_IDevToolsTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_devtools_v1_DevToolsTestResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.DevTools.Test", new strims_devtools_v1_DevToolsTestRequest(req)), strims_devtools_v1_DevToolsTestResponse, opts);
  }
}

