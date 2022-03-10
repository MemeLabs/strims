import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IGetConfigRequest,
  GetConfigRequest,
  GetConfigResponse,
  ISetConfigRequest,
  SetConfigRequest,
  SetConfigResponse,
  IListRulesRequest,
  ListRulesRequest,
  ListRulesResponse,
  IGetRuleRequest,
  GetRuleRequest,
  GetRuleResponse,
  ICreateRuleRequest,
  CreateRuleRequest,
  CreateRuleResponse,
  IUpdateRuleRequest,
  UpdateRuleRequest,
  UpdateRuleResponse,
  IDeleteRuleRequest,
  DeleteRuleRequest,
  DeleteRuleResponse,
} from "./autoseed";

export interface AutoseedFrontendService {
  getConfig(req: GetConfigRequest, call: strims_rpc_Call): Promise<GetConfigResponse> | GetConfigResponse;
  setConfig(req: SetConfigRequest, call: strims_rpc_Call): Promise<SetConfigResponse> | SetConfigResponse;
  listRules(req: ListRulesRequest, call: strims_rpc_Call): Promise<ListRulesResponse> | ListRulesResponse;
  getRule(req: GetRuleRequest, call: strims_rpc_Call): Promise<GetRuleResponse> | GetRuleResponse;
  createRule(req: CreateRuleRequest, call: strims_rpc_Call): Promise<CreateRuleResponse> | CreateRuleResponse;
  updateRule(req: UpdateRuleRequest, call: strims_rpc_Call): Promise<UpdateRuleResponse> | UpdateRuleResponse;
  deleteRule(req: DeleteRuleRequest, call: strims_rpc_Call): Promise<DeleteRuleResponse> | DeleteRuleResponse;
}

export class UnimplementedAutoseedFrontendService implements AutoseedFrontendService {
  getConfig(req: GetConfigRequest, call: strims_rpc_Call): Promise<GetConfigResponse> | GetConfigResponse { throw new Error("not implemented"); }
  setConfig(req: SetConfigRequest, call: strims_rpc_Call): Promise<SetConfigResponse> | SetConfigResponse { throw new Error("not implemented"); }
  listRules(req: ListRulesRequest, call: strims_rpc_Call): Promise<ListRulesResponse> | ListRulesResponse { throw new Error("not implemented"); }
  getRule(req: GetRuleRequest, call: strims_rpc_Call): Promise<GetRuleResponse> | GetRuleResponse { throw new Error("not implemented"); }
  createRule(req: CreateRuleRequest, call: strims_rpc_Call): Promise<CreateRuleResponse> | CreateRuleResponse { throw new Error("not implemented"); }
  updateRule(req: UpdateRuleRequest, call: strims_rpc_Call): Promise<UpdateRuleResponse> | UpdateRuleResponse { throw new Error("not implemented"); }
  deleteRule(req: DeleteRuleRequest, call: strims_rpc_Call): Promise<DeleteRuleResponse> | DeleteRuleResponse { throw new Error("not implemented"); }
}

export const registerAutoseedFrontendService = (host: strims_rpc_Service, service: AutoseedFrontendService): void => {
  host.registerMethod<GetConfigRequest, GetConfigResponse>("strims.autoseed.v1.AutoseedFrontend.GetConfig", service.getConfig.bind(service), GetConfigRequest);
  host.registerMethod<SetConfigRequest, SetConfigResponse>("strims.autoseed.v1.AutoseedFrontend.SetConfig", service.setConfig.bind(service), SetConfigRequest);
  host.registerMethod<ListRulesRequest, ListRulesResponse>("strims.autoseed.v1.AutoseedFrontend.ListRules", service.listRules.bind(service), ListRulesRequest);
  host.registerMethod<GetRuleRequest, GetRuleResponse>("strims.autoseed.v1.AutoseedFrontend.GetRule", service.getRule.bind(service), GetRuleRequest);
  host.registerMethod<CreateRuleRequest, CreateRuleResponse>("strims.autoseed.v1.AutoseedFrontend.CreateRule", service.createRule.bind(service), CreateRuleRequest);
  host.registerMethod<UpdateRuleRequest, UpdateRuleResponse>("strims.autoseed.v1.AutoseedFrontend.UpdateRule", service.updateRule.bind(service), UpdateRuleRequest);
  host.registerMethod<DeleteRuleRequest, DeleteRuleResponse>("strims.autoseed.v1.AutoseedFrontend.DeleteRule", service.deleteRule.bind(service), DeleteRuleRequest);
}

export class AutoseedFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getConfig(req?: IGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.GetConfig", new GetConfigRequest(req)), GetConfigResponse, opts);
  }

  public setConfig(req?: ISetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.SetConfig", new SetConfigRequest(req)), SetConfigResponse, opts);
  }

  public listRules(req?: IListRulesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListRulesResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.ListRules", new ListRulesRequest(req)), ListRulesResponse, opts);
  }

  public getRule(req?: IGetRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.GetRule", new GetRuleRequest(req)), GetRuleResponse, opts);
  }

  public createRule(req?: ICreateRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.CreateRule", new CreateRuleRequest(req)), CreateRuleResponse, opts);
  }

  public updateRule(req?: IUpdateRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.UpdateRule", new UpdateRuleRequest(req)), UpdateRuleResponse, opts);
  }

  public deleteRule(req?: IDeleteRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.DeleteRule", new DeleteRuleRequest(req)), DeleteRuleResponse, opts);
  }
}

