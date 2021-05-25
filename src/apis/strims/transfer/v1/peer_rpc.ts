import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

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

export interface TransferPeerService {
  announce(req: TransferPeerAnnounceRequest, call: strims_rpc_Call): Promise<TransferPeerAnnounceResponse> | TransferPeerAnnounceResponse;
  close(req: TransferPeerCloseRequest, call: strims_rpc_Call): Promise<TransferPeerCloseResponse> | TransferPeerCloseResponse;
}

export const registerTransferPeerService = (host: strims_rpc_Service, service: TransferPeerService): void => {
  host.registerMethod<TransferPeerAnnounceRequest, TransferPeerAnnounceResponse>("strims.transfer.v1.TransferPeer.Announce", service.announce.bind(service));
  host.registerMethod<TransferPeerCloseRequest, TransferPeerCloseResponse>("strims.transfer.v1.TransferPeer.Close", service.close.bind(service));
}

export class TransferPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public announce(req?: ITransferPeerAnnounceRequest, opts?: strims_rpc_UnaryCallOptions): Promise<TransferPeerAnnounceResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Announce", new TransferPeerAnnounceRequest(req)), opts);
  }

  public close(req?: ITransferPeerCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<TransferPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Close", new TransferPeerCloseRequest(req)), opts);
  }
}

