import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";


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
    if (!w) w = new Writer(1024);
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
  body?: TransferPeerAnnounceSwarmResponse.IBodyOneOf
}

export class TransferPeerAnnounceSwarmResponse {
  body: TransferPeerAnnounceSwarmResponse.TBodyOneOf;

  constructor(v?: ITransferPeerAnnounceSwarmResponse) {
    this.body = new TransferPeerAnnounceSwarmResponse.BodyOneOf(v?.body);
  }

  static encode(m: TransferPeerAnnounceSwarmResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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
        m.body = new TransferPeerAnnounceSwarmResponse.BodyOneOf({ port: r.uint32() });
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

  export type IBodyOneOf =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.PORT, port: number }
  ;

  export type TBodyOneOf = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.PORT, port: number }
  >;

  class BodyOneOfImpl {
    port: number;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBodyOneOf) {
      if (v && "port" in v) {
        this.case = BodyCase.PORT;
        this.port = v.port;
      }
    }
  }

  export const BodyOneOf = BodyOneOfImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBodyOneOf>(v: T): Readonly<
    T extends { port: number } ? { case: BodyCase.PORT, port: number } :
    never
    >;
  };

}

