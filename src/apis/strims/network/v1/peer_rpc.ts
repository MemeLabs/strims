import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
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

registerType("strims.network.v1.NetworkPeerNegotiateRequest", NetworkPeerNegotiateRequest);
registerType("strims.network.v1.NetworkPeerNegotiateResponse", NetworkPeerNegotiateResponse);
registerType("strims.network.v1.NetworkPeerOpenRequest", NetworkPeerOpenRequest);
registerType("strims.network.v1.NetworkPeerOpenResponse", NetworkPeerOpenResponse);
registerType("strims.network.v1.NetworkPeerCloseRequest", NetworkPeerCloseRequest);
registerType("strims.network.v1.NetworkPeerCloseResponse", NetworkPeerCloseResponse);
registerType("strims.network.v1.NetworkPeerUpdateCertificateRequest", NetworkPeerUpdateCertificateRequest);
registerType("strims.network.v1.NetworkPeerUpdateCertificateResponse", NetworkPeerUpdateCertificateResponse);

export interface NetworkPeerService {
  negotiate(req: NetworkPeerNegotiateRequest, call: strims_rpc_Call): Promise<NetworkPeerNegotiateResponse> | NetworkPeerNegotiateResponse;
  open(req: NetworkPeerOpenRequest, call: strims_rpc_Call): Promise<NetworkPeerOpenResponse> | NetworkPeerOpenResponse;
  close(req: NetworkPeerCloseRequest, call: strims_rpc_Call): Promise<NetworkPeerCloseResponse> | NetworkPeerCloseResponse;
  updateCertificate(req: NetworkPeerUpdateCertificateRequest, call: strims_rpc_Call): Promise<NetworkPeerUpdateCertificateResponse> | NetworkPeerUpdateCertificateResponse;
}

export const registerNetworkPeerService = (host: strims_rpc_Service, service: NetworkPeerService): void => {
  host.registerMethod<NetworkPeerNegotiateRequest, NetworkPeerNegotiateResponse>("strims.network.v1.NetworkPeer.Negotiate", service.negotiate.bind(service));
  host.registerMethod<NetworkPeerOpenRequest, NetworkPeerOpenResponse>("strims.network.v1.NetworkPeer.Open", service.open.bind(service));
  host.registerMethod<NetworkPeerCloseRequest, NetworkPeerCloseResponse>("strims.network.v1.NetworkPeer.Close", service.close.bind(service));
  host.registerMethod<NetworkPeerUpdateCertificateRequest, NetworkPeerUpdateCertificateResponse>("strims.network.v1.NetworkPeer.UpdateCertificate", service.updateCertificate.bind(service));
}

export class NetworkPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public negotiate(req?: INetworkPeerNegotiateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerNegotiateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Negotiate", new NetworkPeerNegotiateRequest(req)), opts);
  }

  public open(req?: INetworkPeerOpenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerOpenResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Open", new NetworkPeerOpenRequest(req)), opts);
  }

  public close(req?: INetworkPeerCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Close", new NetworkPeerCloseRequest(req)), opts);
  }

  public updateCertificate(req?: INetworkPeerUpdateCertificateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<NetworkPeerUpdateCertificateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.UpdateCertificate", new NetworkPeerUpdateCertificateRequest(req)), opts);
  }
}

