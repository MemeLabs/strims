import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
} from "../../type/certificate";

export interface INetworkAddress {
  hostId?: Uint8Array;
  port?: number;
}

export class NetworkAddress {
  hostId: Uint8Array = new Uint8Array();
  port: number = 0;

  constructor(v?: INetworkAddress) {
    this.hostId = v?.hostId || new Uint8Array();
    this.port = v?.port || 0;
  }

  static encode(m: NetworkAddress, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.hostId) w.uint32(10).bytes(m.hostId);
    if (m.port) w.uint32(16).uint32(m.port);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkAddress {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkAddress();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.hostId = r.bytes();
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

export interface IPeerInit {
  protocolVersion?: number;
  certificate?: strims_type_ICertificate | undefined;
  nodePlatform?: string;
  nodeVersion?: string;
}

export class PeerInit {
  protocolVersion: number = 0;
  certificate: strims_type_Certificate | undefined;
  nodePlatform: string = "";
  nodeVersion: string = "";

  constructor(v?: IPeerInit) {
    this.protocolVersion = v?.protocolVersion || 0;
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.nodePlatform = v?.nodePlatform || "";
    this.nodeVersion = v?.nodeVersion || "";
  }

  static encode(m: PeerInit, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.protocolVersion) w.uint32(8).uint32(m.protocolVersion);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.nodePlatform) w.uint32(26).string(m.nodePlatform);
    if (m.nodeVersion) w.uint32(34).string(m.nodeVersion);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerInit {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerInit();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.protocolVersion = r.uint32();
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 3:
        m.nodePlatform = r.string();
        break;
        case 4:
        m.nodeVersion = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

