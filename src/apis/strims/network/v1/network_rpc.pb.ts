// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type ICreateServerRequest as strims_network_v1_ICreateServerRequest,
  CreateServerRequest as strims_network_v1_CreateServerRequest,
  CreateServerResponse as strims_network_v1_CreateServerResponse,
  type IUpdateServerConfigRequest as strims_network_v1_IUpdateServerConfigRequest,
  UpdateServerConfigRequest as strims_network_v1_UpdateServerConfigRequest,
  UpdateServerConfigResponse as strims_network_v1_UpdateServerConfigResponse,
  type IDeleteNetworkRequest as strims_network_v1_IDeleteNetworkRequest,
  DeleteNetworkRequest as strims_network_v1_DeleteNetworkRequest,
  DeleteNetworkResponse as strims_network_v1_DeleteNetworkResponse,
  type IGetNetworkRequest as strims_network_v1_IGetNetworkRequest,
  GetNetworkRequest as strims_network_v1_GetNetworkRequest,
  GetNetworkResponse as strims_network_v1_GetNetworkResponse,
  type IListNetworksRequest as strims_network_v1_IListNetworksRequest,
  ListNetworksRequest as strims_network_v1_ListNetworksRequest,
  ListNetworksResponse as strims_network_v1_ListNetworksResponse,
  type ICreateInvitationRequest as strims_network_v1_ICreateInvitationRequest,
  CreateInvitationRequest as strims_network_v1_CreateInvitationRequest,
  CreateInvitationResponse as strims_network_v1_CreateInvitationResponse,
  type ICreateNetworkFromInvitationRequest as strims_network_v1_ICreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationRequest as strims_network_v1_CreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationResponse as strims_network_v1_CreateNetworkFromInvitationResponse,
  type IWatchNetworksRequest as strims_network_v1_IWatchNetworksRequest,
  WatchNetworksRequest as strims_network_v1_WatchNetworksRequest,
  WatchNetworksResponse as strims_network_v1_WatchNetworksResponse,
  type IUpdateDisplayOrderRequest as strims_network_v1_IUpdateDisplayOrderRequest,
  UpdateDisplayOrderRequest as strims_network_v1_UpdateDisplayOrderRequest,
  UpdateDisplayOrderResponse as strims_network_v1_UpdateDisplayOrderResponse,
  type IUpdateAliasRequest as strims_network_v1_IUpdateAliasRequest,
  UpdateAliasRequest as strims_network_v1_UpdateAliasRequest,
  UpdateAliasResponse as strims_network_v1_UpdateAliasResponse,
  type IGetUIConfigRequest as strims_network_v1_IGetUIConfigRequest,
  GetUIConfigRequest as strims_network_v1_GetUIConfigRequest,
  GetUIConfigResponse as strims_network_v1_GetUIConfigResponse,
  type IListPeersRequest as strims_network_v1_IListPeersRequest,
  ListPeersRequest as strims_network_v1_ListPeersRequest,
  ListPeersResponse as strims_network_v1_ListPeersResponse,
  type IGrantPeerInvitationRequest as strims_network_v1_IGrantPeerInvitationRequest,
  GrantPeerInvitationRequest as strims_network_v1_GrantPeerInvitationRequest,
  GrantPeerInvitationResponse as strims_network_v1_GrantPeerInvitationResponse,
  type ITogglePeerBanRequest as strims_network_v1_ITogglePeerBanRequest,
  TogglePeerBanRequest as strims_network_v1_TogglePeerBanRequest,
  TogglePeerBanResponse as strims_network_v1_TogglePeerBanResponse,
  type IResetPeerRenameCooldownRequest as strims_network_v1_IResetPeerRenameCooldownRequest,
  ResetPeerRenameCooldownRequest as strims_network_v1_ResetPeerRenameCooldownRequest,
  ResetPeerRenameCooldownResponse as strims_network_v1_ResetPeerRenameCooldownResponse,
  type IDeletePeerRequest as strims_network_v1_IDeletePeerRequest,
  DeletePeerRequest as strims_network_v1_DeletePeerRequest,
  DeletePeerResponse as strims_network_v1_DeletePeerResponse,
  type IListAliasReservationsRequest as strims_network_v1_IListAliasReservationsRequest,
  ListAliasReservationsRequest as strims_network_v1_ListAliasReservationsRequest,
  ListAliasReservationsResponse as strims_network_v1_ListAliasReservationsResponse,
  type IResetAliasReservationCooldownRequest as strims_network_v1_IResetAliasReservationCooldownRequest,
  ResetAliasReservationCooldownRequest as strims_network_v1_ResetAliasReservationCooldownRequest,
  ResetAliasReservationCooldownResponse as strims_network_v1_ResetAliasReservationCooldownResponse,
} from "./network.pb";

export interface NetworkFrontendService {
  createServer(req: strims_network_v1_CreateServerRequest, call: strims_rpc_Call): Promise<strims_network_v1_CreateServerResponse> | strims_network_v1_CreateServerResponse;
  updateServerConfig(req: strims_network_v1_UpdateServerConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_UpdateServerConfigResponse> | strims_network_v1_UpdateServerConfigResponse;
  delete(req: strims_network_v1_DeleteNetworkRequest, call: strims_rpc_Call): Promise<strims_network_v1_DeleteNetworkResponse> | strims_network_v1_DeleteNetworkResponse;
  get(req: strims_network_v1_GetNetworkRequest, call: strims_rpc_Call): Promise<strims_network_v1_GetNetworkResponse> | strims_network_v1_GetNetworkResponse;
  list(req: strims_network_v1_ListNetworksRequest, call: strims_rpc_Call): Promise<strims_network_v1_ListNetworksResponse> | strims_network_v1_ListNetworksResponse;
  createInvitation(req: strims_network_v1_CreateInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_CreateInvitationResponse> | strims_network_v1_CreateInvitationResponse;
  createNetworkFromInvitation(req: strims_network_v1_CreateNetworkFromInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_CreateNetworkFromInvitationResponse> | strims_network_v1_CreateNetworkFromInvitationResponse;
  watch(req: strims_network_v1_WatchNetworksRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_WatchNetworksResponse>;
  updateDisplayOrder(req: strims_network_v1_UpdateDisplayOrderRequest, call: strims_rpc_Call): Promise<strims_network_v1_UpdateDisplayOrderResponse> | strims_network_v1_UpdateDisplayOrderResponse;
  updateAlias(req: strims_network_v1_UpdateAliasRequest, call: strims_rpc_Call): Promise<strims_network_v1_UpdateAliasResponse> | strims_network_v1_UpdateAliasResponse;
  getUIConfig(req: strims_network_v1_GetUIConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_GetUIConfigResponse> | strims_network_v1_GetUIConfigResponse;
  listPeers(req: strims_network_v1_ListPeersRequest, call: strims_rpc_Call): Promise<strims_network_v1_ListPeersResponse> | strims_network_v1_ListPeersResponse;
  grantPeerInvitation(req: strims_network_v1_GrantPeerInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_GrantPeerInvitationResponse> | strims_network_v1_GrantPeerInvitationResponse;
  togglePeerBan(req: strims_network_v1_TogglePeerBanRequest, call: strims_rpc_Call): Promise<strims_network_v1_TogglePeerBanResponse> | strims_network_v1_TogglePeerBanResponse;
  resetPeerRenameCooldown(req: strims_network_v1_ResetPeerRenameCooldownRequest, call: strims_rpc_Call): Promise<strims_network_v1_ResetPeerRenameCooldownResponse> | strims_network_v1_ResetPeerRenameCooldownResponse;
  deletePeer(req: strims_network_v1_DeletePeerRequest, call: strims_rpc_Call): Promise<strims_network_v1_DeletePeerResponse> | strims_network_v1_DeletePeerResponse;
  listAliasReservations(req: strims_network_v1_ListAliasReservationsRequest, call: strims_rpc_Call): Promise<strims_network_v1_ListAliasReservationsResponse> | strims_network_v1_ListAliasReservationsResponse;
  resetAliasReservationCooldown(req: strims_network_v1_ResetAliasReservationCooldownRequest, call: strims_rpc_Call): Promise<strims_network_v1_ResetAliasReservationCooldownResponse> | strims_network_v1_ResetAliasReservationCooldownResponse;
}

export class UnimplementedNetworkFrontendService implements NetworkFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createServer(req: strims_network_v1_CreateServerRequest, call: strims_rpc_Call): Promise<strims_network_v1_CreateServerResponse> | strims_network_v1_CreateServerResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateServerConfig(req: strims_network_v1_UpdateServerConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_UpdateServerConfigResponse> | strims_network_v1_UpdateServerConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  delete(req: strims_network_v1_DeleteNetworkRequest, call: strims_rpc_Call): Promise<strims_network_v1_DeleteNetworkResponse> | strims_network_v1_DeleteNetworkResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  get(req: strims_network_v1_GetNetworkRequest, call: strims_rpc_Call): Promise<strims_network_v1_GetNetworkResponse> | strims_network_v1_GetNetworkResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  list(req: strims_network_v1_ListNetworksRequest, call: strims_rpc_Call): Promise<strims_network_v1_ListNetworksResponse> | strims_network_v1_ListNetworksResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createInvitation(req: strims_network_v1_CreateInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_CreateInvitationResponse> | strims_network_v1_CreateInvitationResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createNetworkFromInvitation(req: strims_network_v1_CreateNetworkFromInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_CreateNetworkFromInvitationResponse> | strims_network_v1_CreateNetworkFromInvitationResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watch(req: strims_network_v1_WatchNetworksRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_WatchNetworksResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateDisplayOrder(req: strims_network_v1_UpdateDisplayOrderRequest, call: strims_rpc_Call): Promise<strims_network_v1_UpdateDisplayOrderResponse> | strims_network_v1_UpdateDisplayOrderResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateAlias(req: strims_network_v1_UpdateAliasRequest, call: strims_rpc_Call): Promise<strims_network_v1_UpdateAliasResponse> | strims_network_v1_UpdateAliasResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getUIConfig(req: strims_network_v1_GetUIConfigRequest, call: strims_rpc_Call): Promise<strims_network_v1_GetUIConfigResponse> | strims_network_v1_GetUIConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listPeers(req: strims_network_v1_ListPeersRequest, call: strims_rpc_Call): Promise<strims_network_v1_ListPeersResponse> | strims_network_v1_ListPeersResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  grantPeerInvitation(req: strims_network_v1_GrantPeerInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_GrantPeerInvitationResponse> | strims_network_v1_GrantPeerInvitationResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  togglePeerBan(req: strims_network_v1_TogglePeerBanRequest, call: strims_rpc_Call): Promise<strims_network_v1_TogglePeerBanResponse> | strims_network_v1_TogglePeerBanResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  resetPeerRenameCooldown(req: strims_network_v1_ResetPeerRenameCooldownRequest, call: strims_rpc_Call): Promise<strims_network_v1_ResetPeerRenameCooldownResponse> | strims_network_v1_ResetPeerRenameCooldownResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deletePeer(req: strims_network_v1_DeletePeerRequest, call: strims_rpc_Call): Promise<strims_network_v1_DeletePeerResponse> | strims_network_v1_DeletePeerResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listAliasReservations(req: strims_network_v1_ListAliasReservationsRequest, call: strims_rpc_Call): Promise<strims_network_v1_ListAliasReservationsResponse> | strims_network_v1_ListAliasReservationsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  resetAliasReservationCooldown(req: strims_network_v1_ResetAliasReservationCooldownRequest, call: strims_rpc_Call): Promise<strims_network_v1_ResetAliasReservationCooldownResponse> | strims_network_v1_ResetAliasReservationCooldownResponse { throw new Error("not implemented"); }
}

export const registerNetworkFrontendService = (host: strims_rpc_Service, service: NetworkFrontendService): void => {
  host.registerMethod<strims_network_v1_CreateServerRequest, strims_network_v1_CreateServerResponse>("strims.network.v1.NetworkFrontend.CreateServer", (req, call) => service.createServer(req, call), strims_network_v1_CreateServerRequest);
  host.registerMethod<strims_network_v1_UpdateServerConfigRequest, strims_network_v1_UpdateServerConfigResponse>("strims.network.v1.NetworkFrontend.UpdateServerConfig", (req, call) => service.updateServerConfig(req, call), strims_network_v1_UpdateServerConfigRequest);
  host.registerMethod<strims_network_v1_DeleteNetworkRequest, strims_network_v1_DeleteNetworkResponse>("strims.network.v1.NetworkFrontend.Delete", (req, call) => service.delete(req, call), strims_network_v1_DeleteNetworkRequest);
  host.registerMethod<strims_network_v1_GetNetworkRequest, strims_network_v1_GetNetworkResponse>("strims.network.v1.NetworkFrontend.Get", (req, call) => service.get(req, call), strims_network_v1_GetNetworkRequest);
  host.registerMethod<strims_network_v1_ListNetworksRequest, strims_network_v1_ListNetworksResponse>("strims.network.v1.NetworkFrontend.List", (req, call) => service.list(req, call), strims_network_v1_ListNetworksRequest);
  host.registerMethod<strims_network_v1_CreateInvitationRequest, strims_network_v1_CreateInvitationResponse>("strims.network.v1.NetworkFrontend.CreateInvitation", (req, call) => service.createInvitation(req, call), strims_network_v1_CreateInvitationRequest);
  host.registerMethod<strims_network_v1_CreateNetworkFromInvitationRequest, strims_network_v1_CreateNetworkFromInvitationResponse>("strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation", (req, call) => service.createNetworkFromInvitation(req, call), strims_network_v1_CreateNetworkFromInvitationRequest);
  host.registerMethod<strims_network_v1_WatchNetworksRequest, strims_network_v1_WatchNetworksResponse>("strims.network.v1.NetworkFrontend.Watch", (req, call) => service.watch(req, call), strims_network_v1_WatchNetworksRequest);
  host.registerMethod<strims_network_v1_UpdateDisplayOrderRequest, strims_network_v1_UpdateDisplayOrderResponse>("strims.network.v1.NetworkFrontend.UpdateDisplayOrder", (req, call) => service.updateDisplayOrder(req, call), strims_network_v1_UpdateDisplayOrderRequest);
  host.registerMethod<strims_network_v1_UpdateAliasRequest, strims_network_v1_UpdateAliasResponse>("strims.network.v1.NetworkFrontend.UpdateAlias", (req, call) => service.updateAlias(req, call), strims_network_v1_UpdateAliasRequest);
  host.registerMethod<strims_network_v1_GetUIConfigRequest, strims_network_v1_GetUIConfigResponse>("strims.network.v1.NetworkFrontend.GetUIConfig", (req, call) => service.getUIConfig(req, call), strims_network_v1_GetUIConfigRequest);
  host.registerMethod<strims_network_v1_ListPeersRequest, strims_network_v1_ListPeersResponse>("strims.network.v1.NetworkFrontend.ListPeers", (req, call) => service.listPeers(req, call), strims_network_v1_ListPeersRequest);
  host.registerMethod<strims_network_v1_GrantPeerInvitationRequest, strims_network_v1_GrantPeerInvitationResponse>("strims.network.v1.NetworkFrontend.GrantPeerInvitation", (req, call) => service.grantPeerInvitation(req, call), strims_network_v1_GrantPeerInvitationRequest);
  host.registerMethod<strims_network_v1_TogglePeerBanRequest, strims_network_v1_TogglePeerBanResponse>("strims.network.v1.NetworkFrontend.TogglePeerBan", (req, call) => service.togglePeerBan(req, call), strims_network_v1_TogglePeerBanRequest);
  host.registerMethod<strims_network_v1_ResetPeerRenameCooldownRequest, strims_network_v1_ResetPeerRenameCooldownResponse>("strims.network.v1.NetworkFrontend.ResetPeerRenameCooldown", (req, call) => service.resetPeerRenameCooldown(req, call), strims_network_v1_ResetPeerRenameCooldownRequest);
  host.registerMethod<strims_network_v1_DeletePeerRequest, strims_network_v1_DeletePeerResponse>("strims.network.v1.NetworkFrontend.DeletePeer", (req, call) => service.deletePeer(req, call), strims_network_v1_DeletePeerRequest);
  host.registerMethod<strims_network_v1_ListAliasReservationsRequest, strims_network_v1_ListAliasReservationsResponse>("strims.network.v1.NetworkFrontend.ListAliasReservations", (req, call) => service.listAliasReservations(req, call), strims_network_v1_ListAliasReservationsRequest);
  host.registerMethod<strims_network_v1_ResetAliasReservationCooldownRequest, strims_network_v1_ResetAliasReservationCooldownResponse>("strims.network.v1.NetworkFrontend.ResetAliasReservationCooldown", (req, call) => service.resetAliasReservationCooldown(req, call), strims_network_v1_ResetAliasReservationCooldownRequest);
}

export class NetworkFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: strims_network_v1_ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.CreateServer", new strims_network_v1_CreateServerRequest(req)), strims_network_v1_CreateServerResponse, opts);
  }

  public updateServerConfig(req?: strims_network_v1_IUpdateServerConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_UpdateServerConfigResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.UpdateServerConfig", new strims_network_v1_UpdateServerConfigRequest(req)), strims_network_v1_UpdateServerConfigResponse, opts);
  }

  public delete(req?: strims_network_v1_IDeleteNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_DeleteNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.Delete", new strims_network_v1_DeleteNetworkRequest(req)), strims_network_v1_DeleteNetworkResponse, opts);
  }

  public get(req?: strims_network_v1_IGetNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_GetNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.Get", new strims_network_v1_GetNetworkRequest(req)), strims_network_v1_GetNetworkResponse, opts);
  }

  public list(req?: strims_network_v1_IListNetworksRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.List", new strims_network_v1_ListNetworksRequest(req)), strims_network_v1_ListNetworksResponse, opts);
  }

  public createInvitation(req?: strims_network_v1_ICreateInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_CreateInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.CreateInvitation", new strims_network_v1_CreateInvitationRequest(req)), strims_network_v1_CreateInvitationResponse, opts);
  }

  public createNetworkFromInvitation(req?: strims_network_v1_ICreateNetworkFromInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_CreateNetworkFromInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation", new strims_network_v1_CreateNetworkFromInvitationRequest(req)), strims_network_v1_CreateNetworkFromInvitationResponse, opts);
  }

  public watch(req?: strims_network_v1_IWatchNetworksRequest): GenericReadable<strims_network_v1_WatchNetworksResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.NetworkFrontend.Watch", new strims_network_v1_WatchNetworksRequest(req)), strims_network_v1_WatchNetworksResponse);
  }

  public updateDisplayOrder(req?: strims_network_v1_IUpdateDisplayOrderRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_UpdateDisplayOrderResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.UpdateDisplayOrder", new strims_network_v1_UpdateDisplayOrderRequest(req)), strims_network_v1_UpdateDisplayOrderResponse, opts);
  }

  public updateAlias(req?: strims_network_v1_IUpdateAliasRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_UpdateAliasResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.UpdateAlias", new strims_network_v1_UpdateAliasRequest(req)), strims_network_v1_UpdateAliasResponse, opts);
  }

  public getUIConfig(req?: strims_network_v1_IGetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_GetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.GetUIConfig", new strims_network_v1_GetUIConfigRequest(req)), strims_network_v1_GetUIConfigResponse, opts);
  }

  public listPeers(req?: strims_network_v1_IListPeersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ListPeersResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.ListPeers", new strims_network_v1_ListPeersRequest(req)), strims_network_v1_ListPeersResponse, opts);
  }

  public grantPeerInvitation(req?: strims_network_v1_IGrantPeerInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_GrantPeerInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.GrantPeerInvitation", new strims_network_v1_GrantPeerInvitationRequest(req)), strims_network_v1_GrantPeerInvitationResponse, opts);
  }

  public togglePeerBan(req?: strims_network_v1_ITogglePeerBanRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_TogglePeerBanResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.TogglePeerBan", new strims_network_v1_TogglePeerBanRequest(req)), strims_network_v1_TogglePeerBanResponse, opts);
  }

  public resetPeerRenameCooldown(req?: strims_network_v1_IResetPeerRenameCooldownRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ResetPeerRenameCooldownResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.ResetPeerRenameCooldown", new strims_network_v1_ResetPeerRenameCooldownRequest(req)), strims_network_v1_ResetPeerRenameCooldownResponse, opts);
  }

  public deletePeer(req?: strims_network_v1_IDeletePeerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_DeletePeerResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.DeletePeer", new strims_network_v1_DeletePeerRequest(req)), strims_network_v1_DeletePeerResponse, opts);
  }

  public listAliasReservations(req?: strims_network_v1_IListAliasReservationsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ListAliasReservationsResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.ListAliasReservations", new strims_network_v1_ListAliasReservationsRequest(req)), strims_network_v1_ListAliasReservationsResponse, opts);
  }

  public resetAliasReservationCooldown(req?: strims_network_v1_IResetAliasReservationCooldownRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_ResetAliasReservationCooldownResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.ResetAliasReservationCooldown", new strims_network_v1_ResetAliasReservationCooldownRequest(req)), strims_network_v1_ResetAliasReservationCooldownResponse, opts);
  }
}

