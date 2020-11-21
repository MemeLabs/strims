import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class NetworkPeer {
  constructor(private readonly host: RPCHost) {}

  public negotiate(
    arg: pb.INetworkPeerNegotiateRequest = new pb.NetworkPeerNegotiateRequest()
  ): Promise<pb.NetworkPeerNegotiateResponse> {
    return this.host.expectOne(
      this.host.call("NetworkPeer/Negotiate", new pb.NetworkPeerNegotiateRequest(arg))
    );
  }
  public open(
    arg: pb.INetworkPeerOpenRequest = new pb.NetworkPeerOpenRequest()
  ): Promise<pb.NetworkPeerOpenResponse> {
    return this.host.expectOne(
      this.host.call("NetworkPeer/Open", new pb.NetworkPeerOpenRequest(arg))
    );
  }
  public close(
    arg: pb.INetworkPeerCloseRequest = new pb.NetworkPeerCloseRequest()
  ): Promise<pb.NetworkPeerCloseResponse> {
    return this.host.expectOne(
      this.host.call("NetworkPeer/Close", new pb.NetworkPeerCloseRequest(arg))
    );
  }
  public updateCertificate(
    arg: pb.INetworkPeerUpdateCertificateRequest = new pb.NetworkPeerUpdateCertificateRequest()
  ): Promise<pb.NetworkPeerUpdateCertificateResponse> {
    return this.host.expectOne(
      this.host.call(
        "NetworkPeer/UpdateCertificate",
        new pb.NetworkPeerUpdateCertificateRequest(arg)
      )
    );
  }
}
