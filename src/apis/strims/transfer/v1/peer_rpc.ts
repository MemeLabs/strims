import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  ITransferPeerAnnounceRequest,
  TransferPeerAnnounceRequest,
  TransferPeerAnnounceResponse,
  ITransferPeerCloseRequest,
  TransferPeerCloseRequest,
  TransferPeerCloseResponse,
} from "./peer";

registerType("strims.transfer.v1.TransferPeerAnnounceRequest", TransferPeerAnnounceRequest);
registerType("strims.transfer.v1.TransferPeerAnnounceResponse", TransferPeerAnnounceResponse);
registerType("strims.transfer.v1.TransferPeerCloseRequest", TransferPeerCloseRequest);
registerType("strims.transfer.v1.TransferPeerCloseResponse", TransferPeerCloseResponse);

export class TransferPeerClient {
  constructor(private readonly host: RPCHost) {}

  public announce(arg: ITransferPeerAnnounceRequest = new TransferPeerAnnounceRequest()): Promise<TransferPeerAnnounceResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Announce", new TransferPeerAnnounceRequest(arg)));
  }

  public close(arg: ITransferPeerCloseRequest = new TransferPeerCloseRequest()): Promise<TransferPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Close", new TransferPeerCloseRequest(arg)));
  }
}

