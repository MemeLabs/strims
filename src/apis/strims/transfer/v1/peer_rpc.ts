import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  ITransferPeerAnnounceSwarmRequest,
  TransferPeerAnnounceSwarmRequest,
  TransferPeerAnnounceSwarmResponse,
  ITransferPeerCloseSwarmRequest,
  TransferPeerCloseSwarmRequest,
  TransferPeerCloseSwarmResponse,
} from "./peer";

registerType("strims.transfer.v1.TransferPeerAnnounceSwarmRequest", TransferPeerAnnounceSwarmRequest);
registerType("strims.transfer.v1.TransferPeerAnnounceSwarmResponse", TransferPeerAnnounceSwarmResponse);
registerType("strims.transfer.v1.TransferPeerCloseSwarmRequest", TransferPeerCloseSwarmRequest);
registerType("strims.transfer.v1.TransferPeerCloseSwarmResponse", TransferPeerCloseSwarmResponse);

export class TransferPeerClient {
  constructor(private readonly host: RPCHost) {}

  public announceSwarm(arg: ITransferPeerAnnounceSwarmRequest = new TransferPeerAnnounceSwarmRequest()): Promise<TransferPeerAnnounceSwarmResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.AnnounceSwarm", new TransferPeerAnnounceSwarmRequest(arg)));
  }

  public closeSwarm(arg: ITransferPeerCloseSwarmRequest = new TransferPeerCloseSwarmRequest()): Promise<TransferPeerCloseSwarmResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.CloseSwarm", new TransferPeerCloseSwarmRequest(arg)));
  }
}

