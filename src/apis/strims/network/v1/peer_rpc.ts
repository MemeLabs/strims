import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  INetworkPeerNegotiateRequest,
  NetworkPeerNegotiateRequest,
  NetworkPeerNegotiateResponse,
  INetworkPeerOpenRequest,
  NetworkPeerOpenRequest,
  NetworkPeerOpenResponse,
  INetworkPeerCloseRequest,
  NetworkPeerCloseRequest,
  NetworkPeerCloseResponse,
  INetworkPeerUpdateCertificateRequest,
  NetworkPeerUpdateCertificateRequest,
  NetworkPeerUpdateCertificateResponse,
} from "./peer";

export interface NetworkPeerService {
  negotiate(req: NetworkPeerNegotiateRequest, call: strims_rpc_Call): Promise<NetworkPeerNegotiateResponse> | NetworkPeerNegotiateResponse;
  open(req: NetworkPeerOpenRequest, call: strims_rpc_Call): Promise<NetworkPeerOpenResponse> | NetworkPeerOpenResponse;
  close(req: NetworkPeerCloseRequest, call: strims_rpc_Call): Promise<NetworkPeerCloseResponse> | NetworkPeerCloseResponse;
  updateCertificate(req: NetworkPeerUpdateCertificateRequest, call: strims_rpc_Call): Promise<NetworkPeerUpdateCertificateResponse> | NetworkPeerUpdateCertificateResponse;
}

export const registerNetworkPeerService = (host: strims_rpc_Service, service: NetworkPeerService): void => {
  host.registerMethod<NetworkPeerNegotiateRequest, NetworkPeerNegotiateResponse>("strims.network.v1.NetworkPeer.Negotiate", service.negotiate.bind(service), NetworkPeerNegotiateRequest);
  host.registerMethod<NetworkPeerOpenRequest, NetworkPeerOpenResponse>("strims.network.v1.NetworkPeer.Open", service.open.bind(service), NetworkPeerOpenRequest);
  host.registerMethod<NetworkPeerCloseRequest, NetworkPeerCloseResponse>("strims.network.v1.NetworkPeer.Close", service.close.bind(service), NetworkPeerCloseRequest);
  host.registerMethod<NetworkPeerUpdateCertificateRequest, NetworkPeerUpdateCertificateResponse>("strims.network.v1.NetworkPeer.UpdateCertificate", service.updateCertificate.bind(service), NetworkPeerUpdateCertificateRequest);
}

export class NetworkPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public negotiate(req?: INetworkPeerNegotiateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerNegotiateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Negotiate", new NetworkPeerNegotiateRequest(req)), NetworkPeerNegotiateResponse, opts);
  }

  public open(req?: INetworkPeerOpenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerOpenResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Open", new NetworkPeerOpenRequest(req)), NetworkPeerOpenResponse, opts);
  }

  public close(req?: INetworkPeerCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Close", new NetworkPeerCloseRequest(req)), NetworkPeerCloseResponse, opts);
  }

  public updateCertificate(req?: INetworkPeerUpdateCertificateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerUpdateCertificateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.UpdateCertificate", new NetworkPeerUpdateCertificateRequest(req)), NetworkPeerUpdateCertificateResponse, opts);
  }
}

