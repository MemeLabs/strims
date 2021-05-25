import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IBootstrapPeerGetPublishEnabledRequest,
  BootstrapPeerGetPublishEnabledRequest,
  BootstrapPeerGetPublishEnabledResponse,
  IBootstrapPeerListNetworksRequest,
  BootstrapPeerListNetworksRequest,
  BootstrapPeerListNetworksResponse,
  IBootstrapPeerPublishRequest,
  BootstrapPeerPublishRequest,
  BootstrapPeerPublishResponse,
} from "./peer";

registerType("strims.network.v1.bootstrap.BootstrapPeerGetPublishEnabledRequest", BootstrapPeerGetPublishEnabledRequest);
registerType("strims.network.v1.bootstrap.BootstrapPeerGetPublishEnabledResponse", BootstrapPeerGetPublishEnabledResponse);
registerType("strims.network.v1.bootstrap.BootstrapPeerListNetworksRequest", BootstrapPeerListNetworksRequest);
registerType("strims.network.v1.bootstrap.BootstrapPeerListNetworksResponse", BootstrapPeerListNetworksResponse);
registerType("strims.network.v1.bootstrap.BootstrapPeerPublishRequest", BootstrapPeerPublishRequest);
registerType("strims.network.v1.bootstrap.BootstrapPeerPublishResponse", BootstrapPeerPublishResponse);

export interface PeerServiceService {
  getPublishEnabled(req: BootstrapPeerGetPublishEnabledRequest, call: strims_rpc_Call): Promise<BootstrapPeerGetPublishEnabledResponse> | BootstrapPeerGetPublishEnabledResponse;
  listNetworks(req: BootstrapPeerListNetworksRequest, call: strims_rpc_Call): Promise<BootstrapPeerListNetworksResponse> | BootstrapPeerListNetworksResponse;
  publish(req: BootstrapPeerPublishRequest, call: strims_rpc_Call): Promise<BootstrapPeerPublishResponse> | BootstrapPeerPublishResponse;
}

export const registerPeerServiceService = (host: strims_rpc_Service, service: PeerServiceService): void => {
  host.registerMethod<BootstrapPeerGetPublishEnabledRequest, BootstrapPeerGetPublishEnabledResponse>("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", service.getPublishEnabled.bind(service));
  host.registerMethod<BootstrapPeerListNetworksRequest, BootstrapPeerListNetworksResponse>("strims.network.v1.bootstrap.PeerService.ListNetworks", service.listNetworks.bind(service));
  host.registerMethod<BootstrapPeerPublishRequest, BootstrapPeerPublishResponse>("strims.network.v1.bootstrap.PeerService.Publish", service.publish.bind(service));
}

export class PeerServiceClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getPublishEnabled(req?: IBootstrapPeerGetPublishEnabledRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BootstrapPeerGetPublishEnabledResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", new BootstrapPeerGetPublishEnabledRequest(req)), opts);
  }

  public listNetworks(req?: IBootstrapPeerListNetworksRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BootstrapPeerListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.ListNetworks", new BootstrapPeerListNetworksRequest(req)), opts);
  }

  public publish(req?: IBootstrapPeerPublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<BootstrapPeerPublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.Publish", new BootstrapPeerPublishRequest(req)), opts);
  }
}

