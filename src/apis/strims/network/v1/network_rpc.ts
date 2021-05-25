import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICreateNetworkRequest,
  CreateNetworkRequest,
  CreateNetworkResponse,
  IUpdateNetworkRequest,
  UpdateNetworkRequest,
  UpdateNetworkResponse,
  IDeleteNetworkRequest,
  DeleteNetworkRequest,
  DeleteNetworkResponse,
  IGetNetworkRequest,
  GetNetworkRequest,
  GetNetworkResponse,
  IListNetworksRequest,
  ListNetworksRequest,
  ListNetworksResponse,
  ICreateNetworkInvitationRequest,
  CreateNetworkInvitationRequest,
  CreateNetworkInvitationResponse,
  ICreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationResponse,
} from "./network";

registerType("strims.network.v1.CreateNetworkRequest", CreateNetworkRequest);
registerType("strims.network.v1.CreateNetworkResponse", CreateNetworkResponse);
registerType("strims.network.v1.UpdateNetworkRequest", UpdateNetworkRequest);
registerType("strims.network.v1.UpdateNetworkResponse", UpdateNetworkResponse);
registerType("strims.network.v1.DeleteNetworkRequest", DeleteNetworkRequest);
registerType("strims.network.v1.DeleteNetworkResponse", DeleteNetworkResponse);
registerType("strims.network.v1.GetNetworkRequest", GetNetworkRequest);
registerType("strims.network.v1.GetNetworkResponse", GetNetworkResponse);
registerType("strims.network.v1.ListNetworksRequest", ListNetworksRequest);
registerType("strims.network.v1.ListNetworksResponse", ListNetworksResponse);
registerType("strims.network.v1.CreateNetworkInvitationRequest", CreateNetworkInvitationRequest);
registerType("strims.network.v1.CreateNetworkInvitationResponse", CreateNetworkInvitationResponse);
registerType("strims.network.v1.CreateNetworkFromInvitationRequest", CreateNetworkFromInvitationRequest);
registerType("strims.network.v1.CreateNetworkFromInvitationResponse", CreateNetworkFromInvitationResponse);

export interface NetworkServiceService {
  create(req: CreateNetworkRequest, call: strims_rpc_Call): Promise<CreateNetworkResponse> | CreateNetworkResponse;
  update(req: UpdateNetworkRequest, call: strims_rpc_Call): Promise<UpdateNetworkResponse> | UpdateNetworkResponse;
  delete(req: DeleteNetworkRequest, call: strims_rpc_Call): Promise<DeleteNetworkResponse> | DeleteNetworkResponse;
  get(req: GetNetworkRequest, call: strims_rpc_Call): Promise<GetNetworkResponse> | GetNetworkResponse;
  list(req: ListNetworksRequest, call: strims_rpc_Call): Promise<ListNetworksResponse> | ListNetworksResponse;
  createInvitation(req: CreateNetworkInvitationRequest, call: strims_rpc_Call): Promise<CreateNetworkInvitationResponse> | CreateNetworkInvitationResponse;
  createFromInvitation(req: CreateNetworkFromInvitationRequest, call: strims_rpc_Call): Promise<CreateNetworkFromInvitationResponse> | CreateNetworkFromInvitationResponse;
}

export const registerNetworkServiceService = (host: strims_rpc_Service, service: NetworkServiceService): void => {
  host.registerMethod<CreateNetworkRequest, CreateNetworkResponse>("strims.network.v1.NetworkService.Create", service.create.bind(service));
  host.registerMethod<UpdateNetworkRequest, UpdateNetworkResponse>("strims.network.v1.NetworkService.Update", service.update.bind(service));
  host.registerMethod<DeleteNetworkRequest, DeleteNetworkResponse>("strims.network.v1.NetworkService.Delete", service.delete.bind(service));
  host.registerMethod<GetNetworkRequest, GetNetworkResponse>("strims.network.v1.NetworkService.Get", service.get.bind(service));
  host.registerMethod<ListNetworksRequest, ListNetworksResponse>("strims.network.v1.NetworkService.List", service.list.bind(service));
  host.registerMethod<CreateNetworkInvitationRequest, CreateNetworkInvitationResponse>("strims.network.v1.NetworkService.CreateInvitation", service.createInvitation.bind(service));
  host.registerMethod<CreateNetworkFromInvitationRequest, CreateNetworkFromInvitationResponse>("strims.network.v1.NetworkService.CreateFromInvitation", service.createFromInvitation.bind(service));
}

export class NetworkServiceClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public create(req?: ICreateNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Create", new CreateNetworkRequest(req)), opts);
  }

  public update(req?: IUpdateNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Update", new UpdateNetworkRequest(req)), opts);
  }

  public delete(req?: IDeleteNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Delete", new DeleteNetworkRequest(req)), opts);
  }

  public get(req?: IGetNetworkRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Get", new GetNetworkRequest(req)), opts);
  }

  public list(req?: IListNetworksRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.List", new ListNetworksRequest(req)), opts);
  }

  public createInvitation(req?: ICreateNetworkInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateNetworkInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.CreateInvitation", new CreateNetworkInvitationRequest(req)), opts);
  }

  public createFromInvitation(req?: ICreateNetworkFromInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateNetworkFromInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.CreateFromInvitation", new CreateNetworkFromInvitationRequest(req)), opts);
  }
}

