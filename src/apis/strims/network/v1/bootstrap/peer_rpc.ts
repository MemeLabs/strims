import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_network_v1_bootstrap_IBootstrapPeerGetPublishEnabledRequest,
  strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest,
  strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse,
  strims_network_v1_bootstrap_IBootstrapPeerListNetworksRequest,
  strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest,
  strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse,
  strims_network_v1_bootstrap_IBootstrapPeerPublishRequest,
  strims_network_v1_bootstrap_BootstrapPeerPublishRequest,
  strims_network_v1_bootstrap_BootstrapPeerPublishResponse,
} from "./peer";

export interface PeerServiceService {
  getPublishEnabled(req: strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse> | strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse;
  listNetworks(req: strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse> | strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse;
  publish(req: strims_network_v1_bootstrap_BootstrapPeerPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerPublishResponse> | strims_network_v1_bootstrap_BootstrapPeerPublishResponse;
}

export class UnimplementedPeerServiceService implements PeerServiceService {
  getPublishEnabled(req: strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse> | strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse { throw new Error("not implemented"); }
  listNetworks(req: strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse> | strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse { throw new Error("not implemented"); }
  publish(req: strims_network_v1_bootstrap_BootstrapPeerPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerPublishResponse> | strims_network_v1_bootstrap_BootstrapPeerPublishResponse { throw new Error("not implemented"); }
}

export const registerPeerServiceService = (host: strims_rpc_Service, service: PeerServiceService): void => {
  host.registerMethod<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest, strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse>("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", service.getPublishEnabled.bind(service), strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest);
  host.registerMethod<strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest, strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse>("strims.network.v1.bootstrap.PeerService.ListNetworks", service.listNetworks.bind(service), strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest);
  host.registerMethod<strims_network_v1_bootstrap_BootstrapPeerPublishRequest, strims_network_v1_bootstrap_BootstrapPeerPublishResponse>("strims.network.v1.bootstrap.PeerService.Publish", service.publish.bind(service), strims_network_v1_bootstrap_BootstrapPeerPublishRequest);
}

export class PeerServiceClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getPublishEnabled(req?: strims_network_v1_bootstrap_IBootstrapPeerGetPublishEnabledRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", new strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest(req)), strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse, opts);
  }

  public listNetworks(req?: strims_network_v1_bootstrap_IBootstrapPeerListNetworksRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.ListNetworks", new strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest(req)), strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse, opts);
  }

  public publish(req?: strims_network_v1_bootstrap_IBootstrapPeerPublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_bootstrap_BootstrapPeerPublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.Publish", new strims_network_v1_bootstrap_BootstrapPeerPublishRequest(req)), strims_network_v1_bootstrap_BootstrapPeerPublishResponse, opts);
  }
}

