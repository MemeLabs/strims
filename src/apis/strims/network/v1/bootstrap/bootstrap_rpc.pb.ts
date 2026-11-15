// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IGetConfigRequest as strims_network_v1_bootstrap_IGetConfigRequest,
  GetConfigRequest as strims_network_v1_bootstrap_GetConfigRequest,
  GetConfigResponse as strims_network_v1_bootstrap_GetConfigResponse,
  type ISetConfigRequest as strims_network_v1_bootstrap_ISetConfigRequest,
  SetConfigRequest as strims_network_v1_bootstrap_SetConfigRequest,
  SetConfigResponse as strims_network_v1_bootstrap_SetConfigResponse,
  type ICreateBootstrapClientRequest as strims_network_v1_bootstrap_ICreateBootstrapClientRequest,
  CreateBootstrapClientRequest as strims_network_v1_bootstrap_CreateBootstrapClientRequest,
  CreateBootstrapClientResponse as strims_network_v1_bootstrap_CreateBootstrapClientResponse,
  type IUpdateBootstrapClientRequest as strims_network_v1_bootstrap_IUpdateBootstrapClientRequest,
  UpdateBootstrapClientRequest as strims_network_v1_bootstrap_UpdateBootstrapClientRequest,
  UpdateBootstrapClientResponse as strims_network_v1_bootstrap_UpdateBootstrapClientResponse,
  type IDeleteBootstrapClientRequest as strims_network_v1_bootstrap_IDeleteBootstrapClientRequest,
  DeleteBootstrapClientRequest as strims_network_v1_bootstrap_DeleteBootstrapClientRequest,
  DeleteBootstrapClientResponse as strims_network_v1_bootstrap_DeleteBootstrapClientResponse,
  type IGetBootstrapClientRequest as strims_network_v1_bootstrap_IGetBootstrapClientRequest,
  GetBootstrapClientRequest as strims_network_v1_bootstrap_GetBootstrapClientRequest,
  GetBootstrapClientResponse as strims_network_v1_bootstrap_GetBootstrapClientResponse,
  type IListBootstrapClientsRequest as strims_network_v1_bootstrap_IListBootstrapClientsRequest,
  ListBootstrapClientsRequest as strims_network_v1_bootstrap_ListBootstrapClientsRequest,
  ListBootstrapClientsResponse as strims_network_v1_bootstrap_ListBootstrapClientsResponse,
  type IListBootstrapPeersRequest as strims_network_v1_bootstrap_IListBootstrapPeersRequest,
  ListBootstrapPeersRequest as strims_network_v1_bootstrap_ListBootstrapPeersRequest,
  ListBootstrapPeersResponse as strims_network_v1_bootstrap_ListBootstrapPeersResponse,
  type IPublishNetworkToBootstrapPeerRequest as strims_network_v1_bootstrap_IPublishNetworkToBootstrapPeerRequest,
  PublishNetworkToBootstrapPeerRequest as strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest,
  PublishNetworkToBootstrapPeerResponse as strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse,
} from "./bootstrap.pb";

export interface BootstrapFrontendService {
  getConfig(req: strims_network_v1_bootstrap_GetConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_GetConfigResponse> | strims_network_v1_bootstrap_GetConfigResponse;
  setConfig(req: strims_network_v1_bootstrap_SetConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_SetConfigResponse> | strims_network_v1_bootstrap_SetConfigResponse;
  createClient(req: strims_network_v1_bootstrap_CreateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_CreateBootstrapClientResponse> | strims_network_v1_bootstrap_CreateBootstrapClientResponse;
  updateClient(req: strims_network_v1_bootstrap_UpdateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_UpdateBootstrapClientResponse> | strims_network_v1_bootstrap_UpdateBootstrapClientResponse;
  deleteClient(req: strims_network_v1_bootstrap_DeleteBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_DeleteBootstrapClientResponse> | strims_network_v1_bootstrap_DeleteBootstrapClientResponse;
  getClient(req: strims_network_v1_bootstrap_GetBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_GetBootstrapClientResponse> | strims_network_v1_bootstrap_GetBootstrapClientResponse;
  listClients(req: strims_network_v1_bootstrap_ListBootstrapClientsRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapClientsResponse> | strims_network_v1_bootstrap_ListBootstrapClientsResponse;
  listPeers(req: strims_network_v1_bootstrap_ListBootstrapPeersRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapPeersResponse> | strims_network_v1_bootstrap_ListBootstrapPeersResponse;
  publishNetworkToPeer(req: strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse> | strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse;
}

export class UnimplementedBootstrapFrontendService implements BootstrapFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getConfig(req: strims_network_v1_bootstrap_GetConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_GetConfigResponse> | strims_network_v1_bootstrap_GetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setConfig(req: strims_network_v1_bootstrap_SetConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_SetConfigResponse> | strims_network_v1_bootstrap_SetConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createClient(req: strims_network_v1_bootstrap_CreateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_CreateBootstrapClientResponse> | strims_network_v1_bootstrap_CreateBootstrapClientResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateClient(req: strims_network_v1_bootstrap_UpdateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_UpdateBootstrapClientResponse> | strims_network_v1_bootstrap_UpdateBootstrapClientResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteClient(req: strims_network_v1_bootstrap_DeleteBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_DeleteBootstrapClientResponse> | strims_network_v1_bootstrap_DeleteBootstrapClientResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getClient(req: strims_network_v1_bootstrap_GetBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_GetBootstrapClientResponse> | strims_network_v1_bootstrap_GetBootstrapClientResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listClients(req: strims_network_v1_bootstrap_ListBootstrapClientsRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapClientsResponse> | strims_network_v1_bootstrap_ListBootstrapClientsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listPeers(req: strims_network_v1_bootstrap_ListBootstrapPeersRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapPeersResponse> | strims_network_v1_bootstrap_ListBootstrapPeersResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  publishNetworkToPeer(req: strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse> | strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse { throw new Error("not implemented"); }
}

export const registerBootstrapFrontendService = (host: strims_rpc_Service, service: BootstrapFrontendService): void => {
  host.registerMethod<strims_network_v1_bootstrap_GetConfigRequest, strims_network_v1_bootstrap_GetConfigResponse>("strims.network.v1.bootstrap.BootstrapFrontend.GetConfig", (req, call) => service.getConfig(req, call), strims_network_v1_bootstrap_GetConfigRequest);
  host.registerMethod<strims_network_v1_bootstrap_SetConfigRequest, strims_network_v1_bootstrap_SetConfigResponse>("strims.network.v1.bootstrap.BootstrapFrontend.SetConfig", (req, call) => service.setConfig(req, call), strims_network_v1_bootstrap_SetConfigRequest);
  host.registerMethod<strims_network_v1_bootstrap_CreateBootstrapClientRequest, strims_network_v1_bootstrap_CreateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", (req, call) => service.createClient(req, call), strims_network_v1_bootstrap_CreateBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_UpdateBootstrapClientRequest, strims_network_v1_bootstrap_UpdateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", (req, call) => service.updateClient(req, call), strims_network_v1_bootstrap_UpdateBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_DeleteBootstrapClientRequest, strims_network_v1_bootstrap_DeleteBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", (req, call) => service.deleteClient(req, call), strims_network_v1_bootstrap_DeleteBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_GetBootstrapClientRequest, strims_network_v1_bootstrap_GetBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", (req, call) => service.getClient(req, call), strims_network_v1_bootstrap_GetBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_ListBootstrapClientsRequest, strims_network_v1_bootstrap_ListBootstrapClientsResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", (req, call) => service.listClients(req, call), strims_network_v1_bootstrap_ListBootstrapClientsRequest);
  host.registerMethod<strims_network_v1_bootstrap_ListBootstrapPeersRequest, strims_network_v1_bootstrap_ListBootstrapPeersResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", (req, call) => service.listPeers(req, call), strims_network_v1_bootstrap_ListBootstrapPeersRequest);
  host.registerMethod<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest, strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse>("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", (req, call) => service.publishNetworkToPeer(req, call), strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest);
}

export class BootstrapFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getConfig(req?: strims_network_v1_bootstrap_IGetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_GetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.GetConfig", new strims_network_v1_bootstrap_GetConfigRequest(req)), strims_network_v1_bootstrap_GetConfigResponse, opts);
  }

  public setConfig(req?: strims_network_v1_bootstrap_ISetConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_SetConfigResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.SetConfig", new strims_network_v1_bootstrap_SetConfigRequest(req)), strims_network_v1_bootstrap_SetConfigResponse, opts);
  }

  public createClient(req?: strims_network_v1_bootstrap_ICreateBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_CreateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", new strims_network_v1_bootstrap_CreateBootstrapClientRequest(req)), strims_network_v1_bootstrap_CreateBootstrapClientResponse, opts);
  }

  public updateClient(req?: strims_network_v1_bootstrap_IUpdateBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_UpdateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", new strims_network_v1_bootstrap_UpdateBootstrapClientRequest(req)), strims_network_v1_bootstrap_UpdateBootstrapClientResponse, opts);
  }

  public deleteClient(req?: strims_network_v1_bootstrap_IDeleteBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_DeleteBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", new strims_network_v1_bootstrap_DeleteBootstrapClientRequest(req)), strims_network_v1_bootstrap_DeleteBootstrapClientResponse, opts);
  }

  public getClient(req?: strims_network_v1_bootstrap_IGetBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_GetBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", new strims_network_v1_bootstrap_GetBootstrapClientRequest(req)), strims_network_v1_bootstrap_GetBootstrapClientResponse, opts);
  }

  public listClients(req?: strims_network_v1_bootstrap_IListBootstrapClientsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_ListBootstrapClientsResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", new strims_network_v1_bootstrap_ListBootstrapClientsRequest(req)), strims_network_v1_bootstrap_ListBootstrapClientsResponse, opts);
  }

  public listPeers(req?: strims_network_v1_bootstrap_IListBootstrapPeersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_ListBootstrapPeersResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", new strims_network_v1_bootstrap_ListBootstrapPeersRequest(req)), strims_network_v1_bootstrap_ListBootstrapPeersResponse, opts);
  }

  public publishNetworkToPeer(req?: strims_network_v1_bootstrap_IPublishNetworkToBootstrapPeerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", new strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest(req)), strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse, opts);
  }
}

