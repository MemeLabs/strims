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
  body: TransferPeerAnnounceSwarmResponse.BodyOneOf;

  constructor(v?: ITransferPeerAnnounceSwarmResponse) {
    this.body = new TransferPeerAnnounceSwarmResponse.BodyOneOf(v?.body);
  }

  static encode(m: TransferPeerAnnounceSwarmResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
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
        m.body.port = r.uint32();
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
  export type IBodyOneOf =
  { port: number }
  ;

  export class BodyOneOf {
    private _port: number = 0;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "port" in v) this.port = v.port;
    }

    public clear() {
      this._port = 0;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set port(v: number) {
      this.clear();
      this._port = v;
      this._case = BodyCase.PORT;
    }

    get port(): number {
      return this._port;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    PORT = 1,
  }

}

