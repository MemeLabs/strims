import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_network_v1_bootstrap_ICreateBootstrapClientRequest,
  strims_network_v1_bootstrap_CreateBootstrapClientRequest,
  strims_network_v1_bootstrap_CreateBootstrapClientResponse,
  strims_network_v1_bootstrap_IUpdateBootstrapClientRequest,
  strims_network_v1_bootstrap_UpdateBootstrapClientRequest,
  strims_network_v1_bootstrap_UpdateBootstrapClientResponse,
  strims_network_v1_bootstrap_IDeleteBootstrapClientRequest,
  strims_network_v1_bootstrap_DeleteBootstrapClientRequest,
  strims_network_v1_bootstrap_DeleteBootstrapClientResponse,
  strims_network_v1_bootstrap_IGetBootstrapClientRequest,
  strims_network_v1_bootstrap_GetBootstrapClientRequest,
  strims_network_v1_bootstrap_GetBootstrapClientResponse,
  strims_network_v1_bootstrap_IListBootstrapClientsRequest,
  strims_network_v1_bootstrap_ListBootstrapClientsRequest,
  strims_network_v1_bootstrap_ListBootstrapClientsResponse,
  strims_network_v1_bootstrap_IListBootstrapPeersRequest,
  strims_network_v1_bootstrap_ListBootstrapPeersRequest,
  strims_network_v1_bootstrap_ListBootstrapPeersResponse,
  strims_network_v1_bootstrap_IPublishNetworkToBootstrapPeerRequest,
  strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest,
  strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse,
} from "./bootstrap";

export interface BootstrapFrontendService {
  createClient(req: strims_network_v1_bootstrap_CreateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_CreateBootstrapClientResponse> | strims_network_v1_bootstrap_CreateBootstrapClientResponse;
  updateClient(req: strims_network_v1_bootstrap_UpdateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_UpdateBootstrapClientResponse> | strims_network_v1_bootstrap_UpdateBootstrapClientResponse;
  deleteClient(req: strims_network_v1_bootstrap_DeleteBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_DeleteBootstrapClientResponse> | strims_network_v1_bootstrap_DeleteBootstrapClientResponse;
  getClient(req: strims_network_v1_bootstrap_GetBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_GetBootstrapClientResponse> | strims_network_v1_bootstrap_GetBootstrapClientResponse;
  listClients(req: strims_network_v1_bootstrap_ListBootstrapClientsRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapClientsResponse> | strims_network_v1_bootstrap_ListBootstrapClientsResponse;
  listPeers(req: strims_network_v1_bootstrap_ListBootstrapPeersRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapPeersResponse> | strims_network_v1_bootstrap_ListBootstrapPeersResponse;
  publishNetworkToPeer(req: strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse> | strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse;
}

export class UnimplementedBootstrapFrontendService implements BootstrapFrontendService {
  createClient(req: strims_network_v1_bootstrap_CreateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_CreateBootstrapClientResponse> | strims_network_v1_bootstrap_CreateBootstrapClientResponse { throw new Error("not implemented"); }
  updateClient(req: strims_network_v1_bootstrap_UpdateBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_UpdateBootstrapClientResponse> | strims_network_v1_bootstrap_UpdateBootstrapClientResponse { throw new Error("not implemented"); }
  deleteClient(req: strims_network_v1_bootstrap_DeleteBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_DeleteBootstrapClientResponse> | strims_network_v1_bootstrap_DeleteBootstrapClientResponse { throw new Error("not implemented"); }
  getClient(req: strims_network_v1_bootstrap_GetBootstrapClientRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_GetBootstrapClientResponse> | strims_network_v1_bootstrap_GetBootstrapClientResponse { throw new Error("not implemented"); }
  listClients(req: strims_network_v1_bootstrap_ListBootstrapClientsRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapClientsResponse> | strims_network_v1_bootstrap_ListBootstrapClientsResponse { throw new Error("not implemented"); }
  listPeers(req: strims_network_v1_bootstrap_ListBootstrapPeersRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_ListBootstrapPeersResponse> | strims_network_v1_bootstrap_ListBootstrapPeersResponse { throw new Error("not implemented"); }
  publishNetworkToPeer(req: strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse> | strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse { throw new Error("not implemented"); }
}

export const registerBootstrapFrontendService = (host: strims_rpc_Service, service: BootstrapFrontendService): void => {
  host.registerMethod<strims_network_v1_bootstrap_CreateBootstrapClientRequest, strims_network_v1_bootstrap_CreateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.CreateClient", service.createClient.bind(service), strims_network_v1_bootstrap_CreateBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_UpdateBootstrapClientRequest, strims_network_v1_bootstrap_UpdateBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.UpdateClient", service.updateClient.bind(service), strims_network_v1_bootstrap_UpdateBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_DeleteBootstrapClientRequest, strims_network_v1_bootstrap_DeleteBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.DeleteClient", service.deleteClient.bind(service), strims_network_v1_bootstrap_DeleteBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_GetBootstrapClientRequest, strims_network_v1_bootstrap_GetBootstrapClientResponse>("strims.network.v1.bootstrap.BootstrapFrontend.GetClient", service.getClient.bind(service), strims_network_v1_bootstrap_GetBootstrapClientRequest);
  host.registerMethod<strims_network_v1_bootstrap_ListBootstrapClientsRequest, strims_network_v1_bootstrap_ListBootstrapClientsResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListClients", service.listClients.bind(service), strims_network_v1_bootstrap_ListBootstrapClientsRequest);
  host.registerMethod<strims_network_v1_bootstrap_ListBootstrapPeersRequest, strims_network_v1_bootstrap_ListBootstrapPeersResponse>("strims.network.v1.bootstrap.BootstrapFrontend.ListPeers", service.listPeers.bind(service), strims_network_v1_bootstrap_ListBootstrapPeersRequest);
  host.registerMethod<strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest, strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerResponse>("strims.network.v1.bootstrap.BootstrapFrontend.PublishNetworkToPeer", service.publishNetworkToPeer.bind(service), strims_network_v1_bootstrap_PublishNetworkToBootstrapPeerRequest);
}

export class BootstrapFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

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

