import { RPCHost } from "../../../../lib/rpc/host";
import { registerType } from "../../../../lib/rpc/registry";

import {
  ITransferPeerAnnounceSwarmRequest,
  TransferPeerAnnounceSwarmRequest,
  TransferPeerAnnounceSwarmResponse,
} from "./peer";

registerType("strims.transfer.v1.TransferPeerAnnounceSwarmRequest", TransferPeerAnnounceSwarmRequest);
registerType("strims.transfer.v1.TransferPeerAnnounceSwarmResponse", TransferPeerAnnounceSwarmResponse);

export class TransferPeerClient {
  constructor(private readonly host: RPCHost) {}

  public announceSwarm(arg: ITransferPeerAnnounceSwarmRequest = new TransferPeerAnnounceSwarmRequest()): Promise<TransferPeerAnnounceSwarmResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.AnnounceSwarm", new TransferPeerAnnounceSwarmRequest(arg)));
  }
}

