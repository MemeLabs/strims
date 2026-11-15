// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type INetworkPeerNegotiateRequest as strims_network_v1_INetworkPeerNegotiateRequest,
  NetworkPeerNegotiateRequest as strims_network_v1_NetworkPeerNegotiateRequest,
  NetworkPeerNegotiateResponse as strims_network_v1_NetworkPeerNegotiateResponse,
  type INetworkPeerOpenRequest as strims_network_v1_INetworkPeerOpenRequest,
  NetworkPeerOpenRequest as strims_network_v1_NetworkPeerOpenRequest,
  NetworkPeerOpenResponse as strims_network_v1_NetworkPeerOpenResponse,
  type INetworkPeerCloseRequest as strims_network_v1_INetworkPeerCloseRequest,
  NetworkPeerCloseRequest as strims_network_v1_NetworkPeerCloseRequest,
  NetworkPeerCloseResponse as strims_network_v1_NetworkPeerCloseResponse,
  type INetworkPeerUpdateCertificateRequest as strims_network_v1_INetworkPeerUpdateCertificateRequest,
  NetworkPeerUpdateCertificateRequest as strims_network_v1_NetworkPeerUpdateCertificateRequest,
  NetworkPeerUpdateCertificateResponse as strims_network_v1_NetworkPeerUpdateCertificateResponse,
} from "./peer.pb";

export interface NetworkPeerService {
  negotiate(req: strims_network_v1_NetworkPeerNegotiateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerNegotiateResponse> | strims_network_v1_NetworkPeerNegotiateResponse;
  open(req: strims_network_v1_NetworkPeerOpenRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerOpenResponse> | strims_network_v1_NetworkPeerOpenResponse;
  close(req: strims_network_v1_NetworkPeerCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerCloseResponse> | strims_network_v1_NetworkPeerCloseResponse;
  updateCertificate(req: strims_network_v1_NetworkPeerUpdateCertificateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerUpdateCertificateResponse> | strims_network_v1_NetworkPeerUpdateCertificateResponse;
}

export class UnimplementedNetworkPeerService implements NetworkPeerService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  negotiate(req: strims_network_v1_NetworkPeerNegotiateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerNegotiateResponse> | strims_network_v1_NetworkPeerNegotiateResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  open(req: strims_network_v1_NetworkPeerOpenRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerOpenResponse> | strims_network_v1_NetworkPeerOpenResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  close(req: strims_network_v1_NetworkPeerCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerCloseResponse> | strims_network_v1_NetworkPeerCloseResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateCertificate(req: strims_network_v1_NetworkPeerUpdateCertificateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerUpdateCertificateResponse> | strims_network_v1_NetworkPeerUpdateCertificateResponse { throw new Error("not implemented"); }
}

export const registerNetworkPeerService = (host: strims_rpc_Service, service: NetworkPeerService): void => {
  host.registerMethod<strims_network_v1_NetworkPeerNegotiateRequest, strims_network_v1_NetworkPeerNegotiateResponse>("strims.network.v1.NetworkPeer.Negotiate", (req, call) => service.negotiate(req, call), strims_network_v1_NetworkPeerNegotiateRequest);
  host.registerMethod<strims_network_v1_NetworkPeerOpenRequest, strims_network_v1_NetworkPeerOpenResponse>("strims.network.v1.NetworkPeer.Open", (req, call) => service.open(req, call), strims_network_v1_NetworkPeerOpenRequest);
  host.registerMethod<strims_network_v1_NetworkPeerCloseRequest, strims_network_v1_NetworkPeerCloseResponse>("strims.network.v1.NetworkPeer.Close", (req, call) => service.close(req, call), strims_network_v1_NetworkPeerCloseRequest);
  host.registerMethod<strims_network_v1_NetworkPeerUpdateCertificateRequest, strims_network_v1_NetworkPeerUpdateCertificateResponse>("strims.network.v1.NetworkPeer.UpdateCertificate", (req, call) => service.updateCertificate(req, call), strims_network_v1_NetworkPeerUpdateCertificateRequest);
}

export class NetworkPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public negotiate(req?: strims_network_v1_INetworkPeerNegotiateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_NetworkPeerNegotiateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Negotiate", new strims_network_v1_NetworkPeerNegotiateRequest(req)), strims_network_v1_NetworkPeerNegotiateResponse, opts);
  }

  public open(req?: strims_network_v1_INetworkPeerOpenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_NetworkPeerOpenResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Open", new strims_network_v1_NetworkPeerOpenRequest(req)), strims_network_v1_NetworkPeerOpenResponse, opts);
  }

  public close(req?: strims_network_v1_INetworkPeerCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_NetworkPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Close", new strims_network_v1_NetworkPeerCloseRequest(req)), strims_network_v1_NetworkPeerCloseResponse, opts);
  }

  public updateCertificate(req?: strims_network_v1_INetworkPeerUpdateCertificateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_NetworkPeerUpdateCertificateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.UpdateCertificate", new strims_network_v1_NetworkPeerUpdateCertificateRequest(req)), strims_network_v1_NetworkPeerUpdateCertificateResponse, opts);
  }
}

