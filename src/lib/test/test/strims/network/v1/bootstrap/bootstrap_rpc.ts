import { RPCHost } from "../../../../../../rpc/host";
import { registerType } from "../../../../../../pb/registry";

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

registerType(".strims.network.v1.bootstrap.CreateBootstrapClientRequest", CreateBootstrapClientRequest);
registerType(".strims.network.v1.bootstrap.CreateBootstrapClientResponse", CreateBootstrapClientResponse);
registerType(".strims.network.v1.bootstrap.UpdateBootstrapClientRequest", UpdateBootstrapClientRequest);
registerType(".strims.network.v1.bootstrap.UpdateBootstrapClientResponse", UpdateBootstrapClientResponse);
registerType(".strims.network.v1.bootstrap.DeleteBootstrapClientRequest", DeleteBootstrapClientRequest);
registerType(".strims.network.v1.bootstrap.DeleteBootstrapClientResponse", DeleteBootstrapClientResponse);
registerType(".strims.network.v1.bootstrap.GetBootstrapClientRequest", GetBootstrapClientRequest);
registerType(".strims.network.v1.bootstrap.GetBootstrapClientResponse", GetBootstrapClientResponse);
registerType(".strims.network.v1.bootstrap.ListBootstrapClientsRequest", ListBootstrapClientsRequest);
registerType(".strims.network.v1.bootstrap.ListBootstrapClientsResponse", ListBootstrapClientsResponse);
registerType(".strims.network.v1.bootstrap.ListBootstrapPeersRequest", ListBootstrapPeersRequest);
registerType(".strims.network.v1.bootstrap.ListBootstrapPeersResponse", ListBootstrapPeersResponse);
registerType(".strims.network.v1.bootstrap.PublishNetworkToBootstrapPeerRequest", PublishNetworkToBootstrapPeerRequest);
registerType(".strims.network.v1.bootstrap.PublishNetworkToBootstrapPeerResponse", PublishNetworkToBootstrapPeerResponse);

export class BootstrapClient {
  constructor(private readonly host: RPCHost) {}

  public createClient(arg: ICreateBootstrapClientRequest = new CreateBootstrapClientRequest()): Promise<CreateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.CreateClient", new CreateBootstrapClientRequest(arg)));
  }

  public updateClient(arg: IUpdateBootstrapClientRequest = new UpdateBootstrapClientRequest()): Promise<UpdateBootstrapClientResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.UpdateClient", new UpdateBootstrapClientRequest(arg)));
  }

  public deleteClient(arg: IDeleteBootstrapClientRequest = new DeleteBootstrapClientRequest()): Promise<DeleteBootstrapClientResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.DeleteClient", new DeleteBootstrapClientRequest(arg)));
  }

  public getClient(arg: IGetBootstrapClientRequest = new GetBootstrapClientRequest()): Promise<GetBootstrapClientResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.GetClient", new GetBootstrapClientRequest(arg)));
  }

  public listClients(arg: IListBootstrapClientsRequest = new ListBootstrapClientsRequest()): Promise<ListBootstrapClientsResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.ListClients", new ListBootstrapClientsRequest(arg)));
  }

  public listPeers(arg: IListBootstrapPeersRequest = new ListBootstrapPeersRequest()): Promise<ListBootstrapPeersResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.ListPeers", new ListBootstrapPeersRequest(arg)));
  }

  public publishNetworkToPeer(arg: IPublishNetworkToBootstrapPeerRequest = new PublishNetworkToBootstrapPeerRequest()): Promise<PublishNetworkToBootstrapPeerResponse> {
    return this.host.expectOne(this.host.call(".strims.network.v1.bootstrap.Bootstrap.PublishNetworkToPeer", new PublishNetworkToBootstrapPeerRequest(arg)));
  }
}

