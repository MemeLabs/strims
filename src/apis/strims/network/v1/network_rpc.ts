import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  ICreateServerRequest,
  CreateServerRequest,
  CreateServerResponse,
  IUpdateServerConfigRequest,
  UpdateServerConfigRequest,
  UpdateServerConfigResponse,
  IDeleteNetworkRequest,
  DeleteNetworkRequest,
  DeleteNetworkResponse,
  IGetNetworkRequest,
  GetNetworkRequest,
  GetNetworkResponse,
  IListNetworksRequest,
  ListNetworksRequest,
  ListNetworksResponse,
  ICreateInvitationRequest,
  CreateInvitationRequest,
  CreateInvitationResponse,
  ICreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationResponse,
  IWatchNetworksRequest,
  WatchNetworksRequest,
  WatchNetworksResponse,
  IUpdateDisplayOrderRequest,
  UpdateDisplayOrderRequest,
  UpdateDisplayOrderResponse,
  IUpdateAliasRequest,
  UpdateAliasRequest,
  UpdateAliasResponse,
  IGetUIConfigRequest,
  GetUIConfigRequest,
  GetUIConfigResponse,
} from "./network";

export interface NetworkFrontendService {
  createServer(req: CreateServerRequest, call: strims_rpc_Call): Promise<CreateServerResponse> | CreateServerResponse;
  updateServerConfig(req: UpdateServerConfigRequest, call: strims_rpc_Call): Promise<UpdateServerConfigResponse> | UpdateServerConfigResponse;
  delete(req: DeleteNetworkRequest, call: strims_rpc_Call): Promise<DeleteNetworkResponse> | DeleteNetworkResponse;
  get(req: GetNetworkRequest, call: strims_rpc_Call): Promise<GetNetworkResponse> | GetNetworkResponse;
  list(req: ListNetworksRequest, call: strims_rpc_Call): Promise<ListNetworksResponse> | ListNetworksResponse;
  createInvitation(req: CreateInvitationRequest, call: strims_rpc_Call): Promise<CreateInvitationResponse> | CreateInvitationResponse;
  createNetworkFromInvitation(req: CreateNetworkFromInvitationRequest, call: strims_rpc_Call): Promise<CreateNetworkFromInvitationResponse> | CreateNetworkFromInvitationResponse;
  watch(req: WatchNetworksRequest, call: strims_rpc_Call): GenericReadable<WatchNetworksResponse>;
  updateDisplayOrder(req: UpdateDisplayOrderRequest, call: strims_rpc_Call): Promise<UpdateDisplayOrderResponse> | UpdateDisplayOrderResponse;
  updateAlias(req: UpdateAliasRequest, call: strims_rpc_Call): Promise<UpdateAliasResponse> | UpdateAliasResponse;
  getUIConfig(req: GetUIConfigRequest, call: strims_rpc_Call): Promise<GetUIConfigResponse> | GetUIConfigResponse;
}

export class UnimplementedNetworkFrontendService implements NetworkFrontendService {
  createServer(req: CreateServerRequest, call: strims_rpc_Call): Promise<CreateServerResponse> | CreateServerResponse { throw new Error("not implemented"); }
  updateServerConfig(req: UpdateServerConfigRequest, call: strims_rpc_Call): Promise<UpdateServerConfigResponse> | UpdateServerConfigResponse { throw new Error("not implemented"); }
  delete(req: DeleteNetworkRequest, call: strims_rpc_Call): Promise<DeleteNetworkResponse> | DeleteNetworkResponse { throw new Error("not implemented"); }
  get(req: GetNetworkRequest, call: strims_rpc_Call): Promise<GetNetworkResponse> | GetNetworkResponse { throw new Error("not implemented"); }
  list(req: ListNetworksRequest, call: strims_rpc_Call): Promise<ListNetworksResponse> | ListNetworksResponse { throw new Error("not implemented"); }
  createInvitation(req: CreateInvitationRequest, call: strims_rpc_Call): Promise<CreateInvitationResponse> | CreateInvitationResponse { throw new Error("not implemented"); }
  createNetworkFromInvitation(req: CreateNetworkFromInvitationRequest, call: strims_rpc_Call): Promise<CreateNetworkFromInvitationResponse> | CreateNetworkFromInvitationResponse { throw new Error("not implemented"); }
  watch(req: WatchNetworksRequest, call: strims_rpc_Call): GenericReadable<WatchNetworksResponse> { throw new Error("not implemented"); }
  updateDisplayOrder(req: UpdateDisplayOrderRequest, call: strims_rpc_Call): Promise<UpdateDisplayOrderResponse> | UpdateDisplayOrderResponse { throw new Error("not implemented"); }
  updateAlias(req: UpdateAliasRequest, call: strims_rpc_Call): Promise<UpdateAliasResponse> | UpdateAliasResponse { throw new Error("not implemented"); }
  getUIConfig(req: GetUIConfigRequest, call: strims_rpc_Call): Promise<GetUIConfigResponse> | GetUIConfigResponse { throw new Error("not implemented"); }
}

export const registerNetworkFrontendService = (host: strims_rpc_Service, service: NetworkFrontendService): void => {
  host.registerMethod<CreateServerRequest, CreateServerResponse>("strims.network.v1.NetworkFrontend.CreateServer", service.createServer.bind(service), CreateServerRequest);
  host.registerMethod<UpdateServerConfigRequest, UpdateServerConfigResponse>("strims.network.v1.NetworkFrontend.UpdateServerConfig", service.updateServerConfig.bind(service), UpdateServerConfigRequest);
  host.registerMethod<DeleteNetworkRequest, DeleteNetworkResponse>("strims.network.v1.NetworkFrontend.Delete", service.delete.bind(service), DeleteNetworkRequest);
  host.registerMethod<GetNetworkRequest, GetNetworkResponse>("strims.network.v1.NetworkFrontend.Get", service.get.bind(service), GetNetworkRequest);
  host.registerMethod<ListNetworksRequest, ListNetworksResponse>("strims.network.v1.NetworkFrontend.List", service.list.bind(service), ListNetworksRequest);
  host.registerMethod<CreateInvitationRequest, CreateInvitationResponse>("strims.network.v1.NetworkFrontend.CreateInvitation", service.createInvitation.bind(service), CreateInvitationRequest);
  host.registerMethod<CreateNetworkFromInvitationRequest, CreateNetworkFromInvitationResponse>("strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation", service.createNetworkFromInvitation.bind(service), CreateNetworkFromInvitationRequest);
  host.registerMethod<WatchNetworksRequest, WatchNetworksResponse>("strims.network.v1.NetworkFrontend.Watch", service.watch.bind(service), WatchNetworksRequest);
  host.registerMethod<UpdateDisplayOrderRequest, UpdateDisplayOrderResponse>("strims.network.v1.NetworkFrontend.UpdateDisplayOrder", service.updateDisplayOrder.bind(service), UpdateDisplayOrderRequest);
  host.registerMethod<UpdateAliasRequest, UpdateAliasResponse>("strims.network.v1.NetworkFrontend.UpdateAlias", service.updateAlias.bind(service), UpdateAliasRequest);
  host.registerMethod<GetUIConfigRequest, GetUIConfigResponse>("strims.network.v1.NetworkFrontend.GetUIConfig", service.getUIConfig.bind(service), GetUIConfigRequest);
}

export class NetworkFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.CreateServer", new CreateServerRequest(req)), CreateServerResponse, opts);
  }

  public updateServerConfig(req?: IUpdateServerConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateServerConfigResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.UpdateServerConfig", new UpdateServerConfigRequest(req)), UpdateServerConfigResponse, opts);
  }

  public delete(req?: IDeleteNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.Delete", new DeleteNetworkRequest(req)), DeleteNetworkResponse, opts);
  }

  public get(req?: IGetNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.Get", new GetNetworkRequest(req)), GetNetworkResponse, opts);
  }

  public list(req?: IListNetworksRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.List", new ListNetworksRequest(req)), ListNetworksResponse, opts);
  }

  public createInvitation(req?: ICreateInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.CreateInvitation", new CreateInvitationRequest(req)), CreateInvitationResponse, opts);
  }

  public createNetworkFromInvitation(req?: ICreateNetworkFromInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateNetworkFromInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation", new CreateNetworkFromInvitationRequest(req)), CreateNetworkFromInvitationResponse, opts);
  }

  public watch(req?: IWatchNetworksRequest): GenericReadable<WatchNetworksResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.NetworkFrontend.Watch", new WatchNetworksRequest(req)), WatchNetworksResponse);
  }

  public updateDisplayOrder(req?: IUpdateDisplayOrderRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateDisplayOrderResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.UpdateDisplayOrder", new UpdateDisplayOrderRequest(req)), UpdateDisplayOrderResponse, opts);
  }

  public updateAlias(req?: IUpdateAliasRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateAliasResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.UpdateAlias", new UpdateAliasRequest(req)), UpdateAliasResponse, opts);
  }

  public getUIConfig(req?: IGetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkFrontend.GetUIConfig", new GetUIConfigRequest(req)), GetUIConfigResponse, opts);
  }
}

