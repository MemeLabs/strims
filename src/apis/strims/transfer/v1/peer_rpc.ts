import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ITransferPeerAnnounceRequest,
  TransferPeerAnnounceRequest,
  TransferPeerAnnounceResponse,
  ITransferPeerCloseRequest,
  TransferPeerCloseRequest,
  TransferPeerCloseResponse,
} from "./peer";

export interface TransferPeerService {
  announce(req: TransferPeerAnnounceRequest, call: strims_rpc_Call): Promise<TransferPeerAnnounceResponse> | TransferPeerAnnounceResponse;
  close(req: TransferPeerCloseRequest, call: strims_rpc_Call): Promise<TransferPeerCloseResponse> | TransferPeerCloseResponse;
}

export class UnimplementedTransferPeerService implements TransferPeerService {
  announce(req: TransferPeerAnnounceRequest, call: strims_rpc_Call): Promise<TransferPeerAnnounceResponse> | TransferPeerAnnounceResponse { throw new Error("not implemented"); }
  close(req: TransferPeerCloseRequest, call: strims_rpc_Call): Promise<TransferPeerCloseResponse> | TransferPeerCloseResponse { throw new Error("not implemented"); }
}

export const registerTransferPeerService = (host: strims_rpc_Service, service: TransferPeerService): void => {
  host.registerMethod<TransferPeerAnnounceRequest, TransferPeerAnnounceResponse>("strims.transfer.v1.TransferPeer.Announce", service.announce.bind(service), TransferPeerAnnounceRequest);
  host.registerMethod<TransferPeerCloseRequest, TransferPeerCloseResponse>("strims.transfer.v1.TransferPeer.Close", service.close.bind(service), TransferPeerCloseRequest);
}

export class TransferPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public announce(req?: ITransferPeerAnnounceRequest, opts?: strims_rpc_UnaryCallOptions): Promise<TransferPeerAnnounceResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Announce", new TransferPeerAnnounceRequest(req)), TransferPeerAnnounceResponse, opts);
  }

  public close(req?: ITransferPeerCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<TransferPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Close", new TransferPeerCloseRequest(req)), TransferPeerCloseResponse, opts);
  }
}

