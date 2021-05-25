import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICreateBootstrapClientRequest,
  CreateBootstrapClientRequest,
  CreateBootstrapClientResponse,
  IUpdateBootstrapClientRequest,
  UpdateBootstrapClientRequest,
  UpdateBootstrapClientResponse,
  IDeleteBootstrapClientRequest,
  DeleteBootstrapClientRequest,
  DeleteBootstrapClientResponse,
  IGetBootstrapClientRequest,
  GetBootstrapClientRequest,
  GetBootstrapClientResponse,
  IListBootstrapClientsRequest,
  ListBootstrapClientsRequest,
  ListBootstrapClientsResponse,
  IListBootstrapPeersRequest,
  ListBootstrapPeersRequest,
  ListBootstrapPeersResponse,
  IPublishNetworkToBootstrapPeerRequest,
  PublishNetworkToBootstrapPeerRequest,
  PublishNetworkToBootstrapPeerResponse,
} from "./bootstrap";

registerType("strims.network.v1.bootstrap.CreateBootstrapClientRequest", CreateBootstrapClientRequest);
registerType("strims.network.v1.bootstrap.CreateBootstrapClientResponse", CreateBootstrapClientResponse);
registerType("strims.network.v1.bootstrap.UpdateBootstrapClientRequest", UpdateBootstrapClientRequest);
registerType("strims.network.v1.bootstrap.UpdateBootstrapClientResponse", UpdateBootstrapClientResponse);
registerType("strims.network.v1.bootstrap.DeleteBootstrapClientRequest", DeleteBootstrapClientRequest);
registerType("strims.network.v1.bootstrap.DeleteBootstrapClientResponse", DeleteBootstrapClientResponse);
registerType("strims.network.v1.bootstrap.GetBootstrapClientRequest", GetBootstrapClientRequest);
registerType("strims.network.v1.bootstrap.GetBootstrapClientResponse", GetBootstrapClientResponse);
registerType("strims.network.v1.bootstrap.ListBootstrapClientsRequest", ListBootstrapClientsRequest);
registerType("strims.network.v1.bootstrap.ListBootstrapClientsResponse", ListBootstrapClientsResponse);
registerType("strims.network.v1.bootstrap.ListBootstrapPeersRequest", ListBootstrapPeersRequest);
registerType("strims.network.v1.bootstrap.ListBootstrapPeersResponse", ListBootstrapPeersResponse);
registerType("strims.network.v1.bootstrap.PublishNetworkToBootstrapPeerRequest", PublishNetworkToBootstrapPeerRequest);
registerType("strims.network.v1.bootstrap.PublishNetworkToBootstrapPeerResponse", PublishNetworkToBootstrapPeerResponse);

export interface BootstrapFrontendService {
  createClient(req: CreateBootstrapClientRequest, call: strims_rpc_Call): Promise<CreateBootstrapClientResponse> | CreateBootstrapClientResponse;
  updateClient(req: UpdateBootstrapClientRequest, call: strims_rpc_Call): Promise<UpdateBootstrapClientResponse> | UpdateBootstrapClientResponse;
  deleteClient(req: DeleteBootstrapClientRequest, call: strims_rpc_Call): Promise<DeleteBootstrapClientResponse> | DeleteBootstrapClientResponse;
  getClient(req: GetBootstrapClientRequest, call: strims_rpc_Call): Promise<GetBootstrapClientResponse> | GetBootstrapClientResponse;
  listClients(req: ListBootstrapClientsRequest, call: strims_rpc_Call): Promise<ListBootstrapClientsResponse> | ListBootstrapClientsResponse;
  listPeers(req: ListBootstrapPeersRequest, call: strims_rpc_Call): Promise<ListBootstrapPeersResponse> | ListBootstrapPeersResponse;
  publishNetworkToPeer(req: PublishNetworkToBootstrapPeerRequest, call: strims_rpc_Call): Promise<PublishNetworkToBootstrapPeerResponse> | PublishNetworkToBootstrapPeerResponse;
}

export const registerBootstrapFrontendService = (host: strims_rpc_Service, service: BootstrapFrontendService): void => {
  host.registerMethod<CreateBootstrapClientRequest, CreateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", service.createClient.bind(service));
  host.registerMethod<UpdateBootstrapClientRequest, UpdateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", service.updateClient.bind(service));
  host.registerMethod<DeleteBootstrapClientRequest, DeleteBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", service.deleteClient.bind(service));
  host.registerMethod<GetBootstrapClientRequest, GetBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", service.getClient.bind(service));
  host.registerMethod<ListBootstrapClientsRequest, ListBootstrapClientsResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", service.listClients.bind(service));
  host.registerMethod<ListBootstrapPeersRequest, ListBootstrapPeersResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", service.listPeers.bind(service));
  host.registerMethod<PublishNetworkToBootstrapPeerRequest, PublishNetworkToBootstrapPeerResponse>("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", service.publishNetworkToPeer.bind(service));
}

export class BootstrapFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createClient(req?: ICreateBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", new CreateBootstrapClientRequest(req)), opts);
  }

  public updateClient(req?: IUpdateBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", new UpdateBootstrapClientRequest(req)), opts);
  }

  public deleteClient(req?: IDeleteBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", new DeleteBootstrapClientRequest(req)), opts);
  }

  public getClient(req?: IGetBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", new GetBootstrapClientRequest(req)), opts);
  }

  public listClients(req?: IListBootstrapClientsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListBootstrapClientsResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", new ListBootstrapClientsRequest(req)), opts);
  }

  public listPeers(req?: IListBootstrapPeersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListBootstrapPeersResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", new ListBootstrapPeersRequest(req)), opts);
  }

  public publishNetworkToPeer(req?: IPublishNetworkToBootstrapPeerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PublishNetworkToBootstrapPeerResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", new PublishNetworkToBootstrapPeerRequest(req)), opts);
  }
}

