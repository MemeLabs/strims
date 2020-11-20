import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class BootstrapPeer {
  constructor(private readonly host: RPCHost) {}

  public getPublishEnabled(
    arg: pb.IBootstrapPeerGetPublishEnabledRequest = new pb.BootstrapPeerGetPublishEnabledRequest()
  ): Promise<pb.BootstrapPeerGetPublishEnabledResponse> {
    return this.host.expectOne(
      this.host.call(
        "BootstrapPeer/GetPublishEnabled",
        new pb.BootstrapPeerGetPublishEnabledRequest(arg)
      )
    );
  }
  public listNetworks(
    arg: pb.IBootstrapPeerListNetworksRequest = new pb.BootstrapPeerListNetworksRequest()
  ): Promise<pb.BootstrapPeerListNetworksResponse> {
    return this.host.expectOne(
      this.host.call("BootstrapPeer/ListNetworks", new pb.BootstrapPeerListNetworksRequest(arg))
    );
  }
  public publish(
    arg: pb.IBootstrapPeerPublishRequest = new pb.BootstrapPeerPublishRequest()
  ): Promise<pb.BootstrapPeerPublishResponse> {
    return this.host.expectOne(
      this.host.call("BootstrapPeer/Publish", new pb.BootstrapPeerPublishRequest(arg))
    );
  }
}
