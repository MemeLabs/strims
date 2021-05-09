import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

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

export class NetworkServiceClient {
  constructor(private readonly host: RPCHost) {}

  public create(arg?: ICreateNetworkRequest): Promise<CreateNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Create", new CreateNetworkRequest(arg)));
  }

  public update(arg?: IUpdateNetworkRequest): Promise<UpdateNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Update", new UpdateNetworkRequest(arg)));
  }

  public delete(arg?: IDeleteNetworkRequest): Promise<DeleteNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Delete", new DeleteNetworkRequest(arg)));
  }

  public get(arg?: IGetNetworkRequest): Promise<GetNetworkResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.Get", new GetNetworkRequest(arg)));
  }

  public list(arg?: IListNetworksRequest): Promise<ListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.List", new ListNetworksRequest(arg)));
  }

  public createInvitation(arg?: ICreateNetworkInvitationRequest): Promise<CreateNetworkInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.CreateInvitation", new CreateNetworkInvitationRequest(arg)));
  }

  public createFromInvitation(arg?: ICreateNetworkFromInvitationRequest): Promise<CreateNetworkFromInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkService.CreateFromInvitation", new CreateNetworkFromInvitationRequest(arg)));
  }
}

