import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
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
  host.registerMethod<CreateBootstrapClientRequest, CreateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", service.createClient.bind(service), CreateBootstrapClientRequest);
  host.registerMethod<UpdateBootstrapClientRequest, UpdateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", service.updateClient.bind(service), UpdateBootstrapClientRequest);
  host.registerMethod<DeleteBootstrapClientRequest, DeleteBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", service.deleteClient.bind(service), DeleteBootstrapClientRequest);
  host.registerMethod<GetBootstrapClientRequest, GetBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", service.getClient.bind(service), GetBootstrapClientRequest);
  host.registerMethod<ListBootstrapClientsRequest, ListBootstrapClientsResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", service.listClients.bind(service), ListBootstrapClientsRequest);
  host.registerMethod<ListBootstrapPeersRequest, ListBootstrapPeersResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", service.listPeers.bind(service), ListBootstrapPeersRequest);
  host.registerMethod<PublishNetworkToBootstrapPeerRequest, PublishNetworkToBootstrapPeerResponse>("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", service.publishNetworkToPeer.bind(service), PublishNetworkToBootstrapPeerRequest);
}

export class BootstrapFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createClient(req?: ICreateBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", new CreateBootstrapClientRequest(req)), CreateBootstrapClientResponse, opts);
  }

  public updateClient(req?: IUpdateBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", new UpdateBootstrapClientRequest(req)), UpdateBootstrapClientResponse, opts);
  }

  public deleteClient(req?: IDeleteBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", new DeleteBootstrapClientRequest(req)), DeleteBootstrapClientResponse, opts);
  }

  public getClient(req?: IGetBootstrapClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetBootstrapClientResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", new GetBootstrapClientRequest(req)), GetBootstrapClientResponse, opts);
  }

  public listClients(req?: IListBootstrapClientsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListBootstrapClientsResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", new ListBootstrapClientsRequest(req)), ListBootstrapClientsResponse, opts);
  }

  public listPeers(req?: IListBootstrapPeersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListBootstrapPeersResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", new ListBootstrapPeersRequest(req)), ListBootstrapPeersResponse, opts);
  }

  public publishNetworkToPeer(req?: IPublishNetworkToBootstrapPeerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PublishNetworkToBootstrapPeerResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", new PublishNetworkToBootstrapPeerRequest(req)), PublishNetworkToBootstrapPeerResponse, opts);
  }
}

