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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITransferPeerAnnounceResponse = {
  body?: TransferPeerAnnounceResponse.IBody
}

export class TransferPeerAnnounceResponse {
  body: TransferPeerAnnounceResponse.TBody;

  constructor(v?: ITransferPeerAnnounceResponse) {
    this.body = new TransferPeerAnnounceResponse.Body(v?.body);
  }

  static encode(m: TransferPeerAnnounceResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case TransferPeerAnnounceResponse.BodyCase.CHANNEL:
      w.uint32(8).uint64(m.body.channel);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerAnnounceResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferPeerAnnounceResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new TransferPeerAnnounceResponse.Body({ channel: r.uint64() });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace TransferPeerAnnounceResponse {
  export enum BodyCase {
    NOT_SET = 0,
    CHANNEL = 1,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.CHANNEL, channel: bigint }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.CHANNEL, channel: bigint }
  >;

  class BodyImpl {
    channel: bigint;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "channel" in v) {
        this.case = BodyCase.CHANNEL;
        this.channel = v.channel;
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { channel: bigint } ? { case: BodyCase.CHANNEL, channel: bigint } :
    never
    >;
  };

}

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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferPeerCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITransferPeerCloseResponse = {
}

export class TransferPeerCloseResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ITransferPeerCloseResponse) {
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

