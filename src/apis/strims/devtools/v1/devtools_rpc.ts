import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IDevToolsTestRequest,
  DevToolsTestRequest,
  DevToolsTestResponse,
} from "./devtools";

export interface DevToolsService {
  test(req: DevToolsTestRequest, call: strims_rpc_Call): Promise<DevToolsTestResponse> | DevToolsTestResponse;
}

export const registerDevToolsService = (host: strims_rpc_Service, service: DevToolsService): void => {
  host.registerMethod<DevToolsTestRequest, DevToolsTestResponse>("strims.devtools.v1.DevTools.Test", service.test.bind(service), DevToolsTestRequest);
}

export class DevToolsClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public test(req?: IDevToolsTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DevToolsTestResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.DevTools.Test", new DevToolsTestRequest(req)), DevToolsTestResponse, opts);
  }
}

