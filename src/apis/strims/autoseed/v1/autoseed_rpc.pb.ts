// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IGetConfigRequest as strims_autoseed_v1_IGetConfigRequest,
  GetConfigRequest as strims_autoseed_v1_GetConfigRequest,
  GetConfigResponse as strims_autoseed_v1_GetConfigResponse,
  type ISetConfigRequest as strims_autoseed_v1_ISetConfigRequest,
  SetConfigRequest as strims_autoseed_v1_SetConfigRequest,
  SetConfigResponse as strims_autoseed_v1_SetConfigResponse,
  type IListRulesRequest as strims_autoseed_v1_IListRulesRequest,
  ListRulesRequest as strims_autoseed_v1_ListRulesRequest,
  ListRulesResponse as strims_autoseed_v1_ListRulesResponse,
  type IGetRuleRequest as strims_autoseed_v1_IGetRuleRequest,
  GetRuleRequest as strims_autoseed_v1_GetRuleRequest,
  GetRuleResponse as strims_autoseed_v1_GetRuleResponse,
  type ICreateRuleRequest as strims_autoseed_v1_ICreateRuleRequest,
  CreateRuleRequest as strims_autoseed_v1_CreateRuleRequest,
  CreateRuleResponse as strims_autoseed_v1_CreateRuleResponse,
  type IUpdateRuleRequest as strims_autoseed_v1_IUpdateRuleRequest,
  UpdateRuleRequest as strims_autoseed_v1_UpdateRuleRequest,
  UpdateRuleResponse as strims_autoseed_v1_UpdateRuleResponse,
  type IDeleteRuleRequest as strims_autoseed_v1_IDeleteRuleRequest,
  DeleteRuleRequest as strims_autoseed_v1_DeleteRuleRequest,
  DeleteRuleResponse as strims_autoseed_v1_DeleteRuleResponse,
} from "./autoseed.pb";

export interface AutoseedFrontendService {
  getConfig(req: strims_autoseed_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_GetConfigResponse> | strims_autoseed_v1_GetConfigResponse;
  setConfig(req: strims_autoseed_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_SetConfigResponse> | strims_autoseed_v1_SetConfigResponse;
  listRules(req: strims_autoseed_v1_ListRulesRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_ListRulesResponse> | strims_autoseed_v1_ListRulesResponse;
  getRule(req: strims_autoseed_v1_GetRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_GetRuleResponse> | strims_autoseed_v1_GetRuleResponse;
  createRule(req: strims_autoseed_v1_CreateRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_CreateRuleResponse> | strims_autoseed_v1_CreateRuleResponse;
  updateRule(req: strims_autoseed_v1_UpdateRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_UpdateRuleResponse> | strims_autoseed_v1_UpdateRuleResponse;
  deleteRule(req: strims_autoseed_v1_DeleteRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_DeleteRuleResponse> | strims_autoseed_v1_DeleteRuleResponse;
}

export class UnimplementedAutoseedFrontendService implements AutoseedFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getConfig(req: strims_autoseed_v1_GetConfigRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_GetConfigResponse> | strims_autoseed_v1_GetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setConfig(req: strims_autoseed_v1_SetConfigRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_SetConfigResponse> | strims_autoseed_v1_SetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listRules(req: strims_autoseed_v1_ListRulesRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_ListRulesResponse> | strims_autoseed_v1_ListRulesResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getRule(req: strims_autoseed_v1_GetRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_GetRuleResponse> | strims_autoseed_v1_GetRuleResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createRule(req: strims_autoseed_v1_CreateRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_CreateRuleResponse> | strims_autoseed_v1_CreateRuleResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateRule(req: strims_autoseed_v1_UpdateRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_UpdateRuleResponse> | strims_autoseed_v1_UpdateRuleResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteRule(req: strims_autoseed_v1_DeleteRuleRequest, call: strims_rpc_Call): Promise<strims_autoseed_v1_DeleteRuleResponse> | strims_autoseed_v1_DeleteRuleResponse { throw new Error("not implemented"); }
}

export const registerAutoseedFrontendService = (host: strims_rpc_Service, service: AutoseedFrontendService): void => {
  host.registerMethod<strims_autoseed_v1_GetConfigRequest, strims_autoseed_v1_GetConfigResponse>("strims.autoseed.v1.AutoseedFrontend.GetConfig", (req, call) => service.getConfig(req, call), strims_autoseed_v1_GetConfigRequest);
  host.registerMethod<strims_autoseed_v1_SetConfigRequest, strims_autoseed_v1_SetConfigResponse>("strims.autoseed.v1.AutoseedFrontend.SetConfig", (req, call) => service.setConfig(req, call), strims_autoseed_v1_SetConfigRequest);
  host.registerMethod<strims_autoseed_v1_ListRulesRequest, strims_autoseed_v1_ListRulesResponse>("strims.autoseed.v1.AutoseedFrontend.ListRules", (req, call) => service.listRules(req, call), strims_autoseed_v1_ListRulesRequest);
  host.registerMethod<strims_autoseed_v1_GetRuleRequest, strims_autoseed_v1_GetRuleResponse>("strims.autoseed.v1.AutoseedFrontend.GetRule", (req, call) => service.getRule(req, call), strims_autoseed_v1_GetRuleRequest);
  host.registerMethod<strims_autoseed_v1_CreateRuleRequest, strims_autoseed_v1_CreateRuleResponse>("strims.autoseed.v1.AutoseedFrontend.CreateRule", (req, call) => service.createRule(req, call), strims_autoseed_v1_CreateRuleRequest);
  host.registerMethod<strims_autoseed_v1_UpdateRuleRequest, strims_autoseed_v1_UpdateRuleResponse>("strims.autoseed.v1.AutoseedFrontend.UpdateRule", (req, call) => service.updateRule(req, call), strims_autoseed_v1_UpdateRuleRequest);
  host.registerMethod<strims_autoseed_v1_DeleteRuleRequest, strims_autoseed_v1_DeleteRuleResponse>("strims.autoseed.v1.AutoseedFrontend.DeleteRule", (req, call) => service.deleteRule(req, call), strims_autoseed_v1_DeleteRuleRequest);
}

export class AutoseedFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getConfig(req?: strims_autoseed_v1_IGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_GetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.GetConfig", new strims_autoseed_v1_GetConfigRequest(req)), strims_autoseed_v1_GetConfigResponse, opts);
  }

  public setConfig(req?: strims_autoseed_v1_ISetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_SetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.SetConfig", new strims_autoseed_v1_SetConfigRequest(req)), strims_autoseed_v1_SetConfigResponse, opts);
  }

  public listRules(req?: strims_autoseed_v1_IListRulesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_ListRulesResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.ListRules", new strims_autoseed_v1_ListRulesRequest(req)), strims_autoseed_v1_ListRulesResponse, opts);
  }

  public getRule(req?: strims_autoseed_v1_IGetRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_GetRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.GetRule", new strims_autoseed_v1_GetRuleRequest(req)), strims_autoseed_v1_GetRuleResponse, opts);
  }

  public createRule(req?: strims_autoseed_v1_ICreateRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_CreateRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.CreateRule", new strims_autoseed_v1_CreateRuleRequest(req)), strims_autoseed_v1_CreateRuleResponse, opts);
  }

  public updateRule(req?: strims_autoseed_v1_IUpdateRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_UpdateRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.UpdateRule", new strims_autoseed_v1_UpdateRuleRequest(req)), strims_autoseed_v1_UpdateRuleResponse, opts);
  }

  public deleteRule(req?: strims_autoseed_v1_IDeleteRuleRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_autoseed_v1_DeleteRuleResponse> {
    return this.host.expectOne(this.host.call("strims.autoseed.v1.AutoseedFrontend.DeleteRule", new strims_autoseed_v1_DeleteRuleRequest(req)), strims_autoseed_v1_DeleteRuleResponse, opts);
  }
}

