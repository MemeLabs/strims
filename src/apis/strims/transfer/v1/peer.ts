import Reader from "../../../../lib/pb/reader";
import Writer from "../../../../lib/pb/writer";


export interface ITransferPeerAnnounceSwarmRequest {
  swarmId?: Uint8Array;
  port?: number;
}

export class TransferPeerAnnounceSwarmRequest {
  swarmId: Uint8Array = new Uint8Array();
  port: number = 0;

  constructor(v?: ITransferPeerAnnounceSwarmRequest) {
    this.swarmId = v?.swarmId || new Uint8Array();
    this.port = v?.port || 0;
  }

  static encode(m: TransferPeerAnnounceSwarmRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmId) w.uint32(10).bytes(m.swarmId);
    if (m.port) w.uint32(16).uint32(m.port);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerAnnounceSwarmRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferPeerAnnounceSwarmRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarmId = r.bytes();
        break;
        case 2:
        m.port = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ITransferPeerAnnounceSwarmResponse {
  body?: TransferPeerAnnounceSwarmResponse.IBody
}

export class TransferPeerAnnounceSwarmResponse {
  body: TransferPeerAnnounceSwarmResponse.TBody;

  constructor(v?: ITransferPeerAnnounceSwarmResponse) {
    this.body = new TransferPeerAnnounceSwarmResponse.Body(v?.body);
  }

  static encode(m: TransferPeerAnnounceSwarmResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case TransferPeerAnnounceSwarmResponse.BodyCase.PORT:
      w.uint32(8).uint32(m.body.port);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerAnnounceSwarmResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferPeerAnnounceSwarmResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new TransferPeerAnnounceSwarmResponse.Body({ port: r.uint32() });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace TransferPeerAnnounceSwarmResponse {
  export enum BodyCase {
    NOT_SET = 0,
    PORT = 1,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.PORT, port: number }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.PORT, port: number }
  >;

  class BodyImpl {
    port: number;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "port" in v) {
        this.case = BodyCase.PORT;
        this.port = v.port;
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { port: number } ? { case: BodyCase.PORT, port: number } :
    never
    >;
  };

}

export interface ITransferPeerCloseSwarmRequest {
  swarmId?: Uint8Array;
}

export class TransferPeerCloseSwarmRequest {
  swarmId: Uint8Array = new Uint8Array();

  constructor(v?: ITransferPeerCloseSwarmRequest) {
    this.swarmId = v?.swarmId || new Uint8Array();
  }

  static encode(m: TransferPeerCloseSwarmRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmId) w.uint32(10).bytes(m.swarmId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerCloseSwarmRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferPeerCloseSwarmRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarmId = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ITransferPeerCloseSwarmResponse {
}

export class TransferPeerCloseSwarmResponse {

  constructor(v?: ITransferPeerCloseSwarmResponse) {
    // noop
  }

  static encode(m: TransferPeerCloseSwarmResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferPeerCloseSwarmResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new TransferPeerCloseSwarmResponse();
  }
}

