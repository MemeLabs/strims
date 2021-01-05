import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class TransferPeer {
  constructor(private readonly host: RPCHost) {}

  public announceSwarm(
    arg: pb.ITransferPeerAnnounceSwarmRequest = new pb.TransferPeerAnnounceSwarmRequest()
  ): Promise<pb.TransferPeerAnnounceSwarmResponse> {
    return this.host.expectOne(
      this.host.call("TransferPeer/AnnounceSwarm", new pb.TransferPeerAnnounceSwarmRequest(arg))
    );
  }
}
