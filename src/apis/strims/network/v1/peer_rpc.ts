import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_network_v1_INetworkPeerNegotiateRequest,
  strims_network_v1_NetworkPeerNegotiateRequest,
  strims_network_v1_NetworkPeerNegotiateResponse,
  strims_network_v1_INetworkPeerOpenRequest,
  strims_network_v1_NetworkPeerOpenRequest,
  strims_network_v1_NetworkPeerOpenResponse,
  strims_network_v1_INetworkPeerCloseRequest,
  strims_network_v1_NetworkPeerCloseRequest,
  strims_network_v1_NetworkPeerCloseResponse,
  strims_network_v1_INetworkPeerUpdateCertificateRequest,
  strims_network_v1_NetworkPeerUpdateCertificateRequest,
  strims_network_v1_NetworkPeerUpdateCertificateResponse,
} from "./peer";

export interface NetworkPeerService {
  negotiate(req: strims_network_v1_NetworkPeerNegotiateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerNegotiateResponse> | strims_network_v1_NetworkPeerNegotiateResponse;
  open(req: strims_network_v1_NetworkPeerOpenRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerOpenResponse> | strims_network_v1_NetworkPeerOpenResponse;
  close(req: strims_network_v1_NetworkPeerCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerCloseResponse> | strims_network_v1_NetworkPeerCloseResponse;
  updateCertificate(req: strims_network_v1_NetworkPeerUpdateCertificateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerUpdateCertificateResponse> | strims_network_v1_NetworkPeerUpdateCertificateResponse;
}

export class UnimplementedNetworkPeerService implements NetworkPeerService {
  negotiate(req: strims_network_v1_NetworkPeerNegotiateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerNegotiateResponse> | strims_network_v1_NetworkPeerNegotiateResponse { throw new Error("not implemented"); }
  open(req: strims_network_v1_NetworkPeerOpenRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerOpenResponse> | strims_network_v1_NetworkPeerOpenResponse { throw new Error("not implemented"); }
  close(req: strims_network_v1_NetworkPeerCloseRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerCloseResponse> | strims_network_v1_NetworkPeerCloseResponse { throw new Error("not implemented"); }
  updateCertificate(req: strims_network_v1_NetworkPeerUpdateCertificateRequest, call: strims_rpc_Call): Promise<strims_network_v1_NetworkPeerUpdateCertificateResponse> | strims_network_v1_NetworkPeerUpdateCertificateResponse { throw new Error("not implemented"); }
}

export const registerNetworkPeerService = (host: strims_rpc_Service, service: NetworkPeerService): void => {
  host.registerMethod<strims_network_v1_NetworkPeerNegotiateRequest, strims_network_v1_NetworkPeerNegotiateResponse>("strims.network.v1.NetworkPeer.Negotiate", service.negotiate.bind(service), strims_network_v1_NetworkPeerNegotiateRequest);
  host.registerMethod<strims_network_v1_NetworkPeerOpenRequest, strims_network_v1_NetworkPeerOpenResponse>("strims.network.v1.NetworkPeer.Open", service.open.bind(service), strims_network_v1_NetworkPeerOpenRequest);
  host.registerMethod<strims_network_v1_NetworkPeerCloseRequest, strims_network_v1_NetworkPeerCloseResponse>("strims.network.v1.NetworkPeer.Close", service.close.bind(service), strims_network_v1_NetworkPeerCloseRequest);
  host.registerMethod<strims_network_v1_NetworkPeerUpdateCertificateRequest, strims_network_v1_NetworkPeerUpdateCertificateResponse>("strims.network.v1.NetworkPeer.UpdateCertificate", service.updateCertificate.bind(service), strims_network_v1_NetworkPeerUpdateCertificateRequest);
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

