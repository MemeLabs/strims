// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type ITransferPeerAnnounceRequest as strims_transfer_v1_ITransferPeerAnnounceRequest,
  TransferPeerAnnounceRequest as strims_transfer_v1_TransferPeerAnnounceRequest,
  TransferPeerAnnounceResponse as strims_transfer_v1_TransferPeerAnnounceResponse,
  type ITransferPeerCloseRequest as strims_transfer_v1_ITransferPeerCloseRequest,
  TransferPeerCloseRequest as strims_transfer_v1_TransferPeerCloseRequest,
  TransferPeerCloseResponse as strims_transfer_v1_TransferPeerCloseResponse,
} from "./peer.pb";

export interface TransferPeerService {
  announce(req: strims_transfer_v1_TransferPeerAnnounceRequest, call: strims_rpc_Call): Promise<strims_transfer_v1_TransferPeerAnnounceResponse> | strims_transfer_v1_TransferPeerAnnounceResponse;
  close(req: strims_transfer_v1_TransferPeerCloseRequest, call: strims_rpc_Call): Promise<strims_transfer_v1_TransferPeerCloseResponse> | strims_transfer_v1_TransferPeerCloseResponse;
}

export class UnimplementedTransferPeerService implements TransferPeerService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  announce(req: strims_transfer_v1_TransferPeerAnnounceRequest, call: strims_rpc_Call): Promise<strims_transfer_v1_TransferPeerAnnounceResponse> | strims_transfer_v1_TransferPeerAnnounceResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  close(req: strims_transfer_v1_TransferPeerCloseRequest, call: strims_rpc_Call): Promise<strims_transfer_v1_TransferPeerCloseResponse> | strims_transfer_v1_TransferPeerCloseResponse { throw new Error("not implemented"); }
}

export const registerTransferPeerService = (host: strims_rpc_Service, service: TransferPeerService): void => {
  host.registerMethod<strims_transfer_v1_TransferPeerAnnounceRequest, strims_transfer_v1_TransferPeerAnnounceResponse>("strims.transfer.v1.TransferPeer.Announce", (req, call) => service.announce(req, call), strims_transfer_v1_TransferPeerAnnounceRequest);
  host.registerMethod<strims_transfer_v1_TransferPeerCloseRequest, strims_transfer_v1_TransferPeerCloseResponse>("strims.transfer.v1.TransferPeer.Close", (req, call) => service.close(req, call), strims_transfer_v1_TransferPeerCloseRequest);
}

export class TransferPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public announce(req?: strims_transfer_v1_ITransferPeerAnnounceRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_transfer_v1_TransferPeerAnnounceResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Announce", new strims_transfer_v1_TransferPeerAnnounceRequest(req)), strims_transfer_v1_TransferPeerAnnounceResponse, opts);
  }

  public close(req?: strims_transfer_v1_ITransferPeerCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_transfer_v1_TransferPeerCloseResponse> {
    return this.host.expectOne(this.host.call("strims.transfer.v1.TransferPeer.Close", new strims_transfer_v1_TransferPeerCloseRequest(req)), strims_transfer_v1_TransferPeerCloseResponse, opts);
  }
}

