import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

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

export class NetworkPeerClient {
  constructor(private readonly host: RPCHost) {}

  public negotiate(arg: INetworkPeerNegotiateRequest = new NetworkPeerNegotiateRequest()): Promise<NetworkPeerNegotiateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Negotiate", new NetworkPeerNegotiateRequest(arg)));
  }

  public open(arg: INetworkPeerOpenRequest = new NetworkPeerOpenRequest()): Promise<NetworkPeerOpenResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Open", new NetworkPeerOpenRequest(arg)));
  }

  public close(arg: INetworkPeerCloseRequest = new NetworkPeerCloseRequest()): Promise<NetworkPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.Close", new NetworkPeerCloseRequest(arg)));
  }

  public updateCertificate(arg: INetworkPeerUpdateCertificateRequest = new NetworkPeerUpdateCertificateRequest()): Promise<NetworkPeerUpdateCertificateResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.NetworkPeer.UpdateCertificate", new NetworkPeerUpdateCertificateRequest(arg)));
  }
}

