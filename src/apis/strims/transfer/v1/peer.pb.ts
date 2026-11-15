// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type ITransferPeerAnnounceRequest = {
  id?: Uint8Array;
  channel?: bigint;
}

export class TransferPeerAnnounceRequest {
  id: Uint8Array;
  channel: bigint;

  constructor(v?: ITransferPeerAnnounceRequest) {
    this.id = v?.id || new Uint8Array();
    this.channel = v?.channel || BigInt(0);
  }

  static encode(m: TransferPeerAnnounceRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    if (m.channel) w.uint32(16).uint64(m.channel);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerAnnounceRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TransferPeerAnnounceRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        case 2:
        m.channel = r.uint64();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ITransferPeerAnnounceResponse = {
  body?: TransferPeerAnnounceResponse_IBody
}

export class TransferPeerAnnounceResponse {
  body: TransferPeerAnnounceResponse_TBody;

  constructor(v?: ITransferPeerAnnounceResponse) {
    this.body = v?.body ? new TransferPeerAnnounceResponse_Body(v.body) : new TransferPeerAnnounceResponse_Body();
  }

  static encode(m: TransferPeerAnnounceResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case TransferPeerAnnounceResponse_BodyCase.CHANNEL:
      w.uint32(8).uint64(m.body.channel);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerAnnounceResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TransferPeerAnnounceResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new TransferPeerAnnounceResponse_Body({ channel: r.uint64() });
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export enum TransferPeerAnnounceResponse_BodyCase {
  NOT_SET = 0,
  CHANNEL = 1,
}

export type TransferPeerAnnounceResponse_IBody =
{ case?: TransferPeerAnnounceResponse_BodyCase.NOT_SET }
|{ case?: TransferPeerAnnounceResponse_BodyCase.CHANNEL, channel: bigint }
;

export type TransferPeerAnnounceResponse_TBody = Readonly<
{ case: TransferPeerAnnounceResponse_BodyCase.NOT_SET }
|{ case: TransferPeerAnnounceResponse_BodyCase.CHANNEL, channel: bigint }
>;

class TransferPeerAnnounceResponse_BodyImpl {
  declare channel?: bigint;
  case: TransferPeerAnnounceResponse_BodyCase = TransferPeerAnnounceResponse_BodyCase.NOT_SET;

  constructor(v?: TransferPeerAnnounceResponse_IBody) {
    if (v && "channel" in v) {
      this.case = TransferPeerAnnounceResponse_BodyCase.CHANNEL;
      this.channel = v.channel;
    }
  }
}

export const TransferPeerAnnounceResponse_Body = TransferPeerAnnounceResponse_BodyImpl as {
  new (): Readonly<{ case: TransferPeerAnnounceResponse_BodyCase.NOT_SET }>;
  new <T extends TransferPeerAnnounceResponse_IBody>(v: T): Readonly<
  T extends { channel: bigint } ? { case: TransferPeerAnnounceResponse_BodyCase.CHANNEL, channel: bigint } :
  never
  >;
};

export type ITransferPeerCloseRequest = {
  id?: Uint8Array;
}

export class TransferPeerCloseRequest {
  id: Uint8Array;

  constructor(v?: ITransferPeerCloseRequest) {
    this.id = v?.id || new Uint8Array();
  }

  static encode(m: TransferPeerCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id.length) w.uint32(10).bytes(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerCloseRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TransferPeerCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ITransferPeerCloseResponse = object;

export class TransferPeerCloseResponse {

  constructor(v?: ITransferPeerCloseResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: TransferPeerCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new TransferPeerCloseResponse();
  }
}

