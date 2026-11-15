// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IBootstrapPeerGetPublishEnabledRequest as strims_network_v1_bootstrap_IBootstrapPeerGetPublishEnabledRequest,
  BootstrapPeerGetPublishEnabledRequest as strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest,
  BootstrapPeerGetPublishEnabledResponse as strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse,
  type IBootstrapPeerListNetworksRequest as strims_network_v1_bootstrap_IBootstrapPeerListNetworksRequest,
  BootstrapPeerListNetworksRequest as strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest,
  BootstrapPeerListNetworksResponse as strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse,
  type IBootstrapPeerPublishRequest as strims_network_v1_bootstrap_IBootstrapPeerPublishRequest,
  BootstrapPeerPublishRequest as strims_network_v1_bootstrap_BootstrapPeerPublishRequest,
  BootstrapPeerPublishResponse as strims_network_v1_bootstrap_BootstrapPeerPublishResponse,
} from "./peer.pb";

export interface PeerServiceService {
  getPublishEnabled(req: strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse> | strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse;
  listNetworks(req: strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse> | strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse;
  publish(req: strims_network_v1_bootstrap_BootstrapPeerPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerPublishResponse> | strims_network_v1_bootstrap_BootstrapPeerPublishResponse;
}

export class UnimplementedPeerServiceService implements PeerServiceService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getPublishEnabled(req: strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse> | strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listNetworks(req: strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse> | strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  publish(req: strims_network_v1_bootstrap_BootstrapPeerPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_bootstrap_BootstrapPeerPublishResponse> | strims_network_v1_bootstrap_BootstrapPeerPublishResponse { throw new Error("not implemented"); }
}

export const registerPeerServiceService = (host: strims_rpc_Service, service: PeerServiceService): void => {
  host.registerMethod<strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest, strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledResponse>("strims.network.v1.bootstrap.PeerService.GetPublishEnabled", (req, call) => service.getPublishEnabled(req, call), strims_network_v1_bootstrap_BootstrapPeerGetPublishEnabledRequest);
  host.registerMethod<strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest, strims_network_v1_bootstrap_BootstrapPeerListNetworksResponse>("strims.network.v1.bootstrap.PeerService.ListNetworks", (req, call) => service.listNetworks(req, call), strims_network_v1_bootstrap_BootstrapPeerListNetworksRequest);
  host.registerMethod<strims_network_v1_bootstrap_BootstrapPeerPublishRequest, strims_network_v1_bootstrap_BootstrapPeerPublishResponse>("strims.network.v1.bootstrap.PeerService.Publish", (req, call) => service.publish(req, call), strims_network_v1_bootstrap_BootstrapPeerPublishRequest);
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

