import { RPCHost } from "../../../../../lib/rpc/host";
import { registerType } from "../../../../../lib/rpc/registry";

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

export class PeerServiceClient {
  constructor(private readonly host: RPCHost) {}

  public getPublishEnabled(arg: IBootstrapPeerGetPublishEnabledRequest = new BootstrapPeerGetPublishEnabledRequest()): Promise<BootstrapPeerGetPublishEnabledResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", new BootstrapPeerGetPublishEnabledRequest(arg)));
  }

  public listNetworks(arg: IBootstrapPeerListNetworksRequest = new BootstrapPeerListNetworksRequest()): Promise<BootstrapPeerListNetworksResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.ListNetworks", new BootstrapPeerListNetworksRequest(arg)));
  }

  public publish(arg: IBootstrapPeerPublishRequest = new BootstrapPeerPublishRequest()): Promise<BootstrapPeerPublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.bootstrap.PeerService.Publish", new BootstrapPeerPublishRequest(arg)));
  }
}

