import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  IDevToolsTestRequest,
  DevToolsTestRequest,
  DevToolsTestResponse,
} from "./devtools";

registerType("strims.devtools.v1.DevToolsTestRequest", DevToolsTestRequest);
registerType("strims.devtools.v1.DevToolsTestResponse", DevToolsTestResponse);

export class DevToolsClient {
  constructor(private readonly host: RPCHost) {}

  public test(arg: IDevToolsTestRequest = new DevToolsTestRequest()): Promise<DevToolsTestResponse> {
    return this.host.expectOne(this.host.call("strims.devtools.v1.DevTools.Test", new DevToolsTestRequest(arg)));
  }
}

