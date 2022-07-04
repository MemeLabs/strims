import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_devtools_v1_IDevToolsTestRequest,
  strims_devtools_v1_DevToolsTestRequest,
  strims_devtools_v1_DevToolsTestResponse,
} from "./devtools";

export interface DevToolsService {
  test(req: strims_devtools_v1_DevToolsTestRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_DevToolsTestResponse> | strims_devtools_v1_DevToolsTestResponse;
}

export class UnimplementedDevToolsService implements DevToolsService {
  test(req: strims_devtools_v1_DevToolsTestRequest, call: strims_rpc_Call): Promise<strims_devtools_v1_DevToolsTestResponse> | strims_devtools_v1_DevToolsTestResponse { throw new Error("not implemented"); }
}

export const registerDevToolsService = (host: strims_rpc_Service, service: DevToolsService): void => {
  host.registerMethod<strims_devtools_v1_DevToolsTestRequest, strims_devtools_v1_DevToolsTestResponse>("strims.devtools.v1.DevTools.Test", service.test.bind(service), strims_devtools_v1_DevToolsTestRequest);
}

export class DevToolsClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public test(req?: strims_devtools_v1_IDevToolsTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_devtools_v1_DevToolsTestResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.DevTools.Test", new strims_devtools_v1_DevToolsTestRequest(req)), strims_devtools_v1_DevToolsTestResponse, opts);
  }
}

