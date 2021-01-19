import { RPCHost } from "../../../../lib/rpc/host";
import { registerType } from "../../../../lib/rpc/registry";

import {
  IGetHistoryRequest,
  GetHistoryRequest,
  GetHistoryResponse,
} from "./infra";

registerType("strims.infra.v1.GetHistoryRequest", GetHistoryRequest);
registerType("strims.infra.v1.GetHistoryResponse", GetHistoryResponse);

export class InfraClient {
  constructor(private readonly host: RPCHost) {}

  public getHistory(arg: IGetHistoryRequest = new GetHistoryRequest()): Promise<GetHistoryResponse> {
    return this.host.expectOne(this.host.call("strims.infra.v1.Infra.GetHistory", new GetHistoryRequest(arg)));
  }
}

