import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class SwarmPeer {
  constructor(private readonly host: RPCHost) {}

  public announceSwarm(
    arg: pb.ISwarmPeerAnnounceSwarmRequest = new pb.SwarmPeerAnnounceSwarmRequest()
  ): Promise<pb.SwarmPeerAnnounceSwarmResponse> {
    return this.host.expectOne(
      this.host.call("SwarmPeer/AnnounceSwarm", new pb.SwarmPeerAnnounceSwarmRequest(arg))
    );
  }
}
