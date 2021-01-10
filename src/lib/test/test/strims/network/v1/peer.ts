import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate
} from "../../type/certificate";

export interface INetworkPeerNegotiateRequest {
  keyCount?: number;
}

export class NetworkPeerNegotiateRequest {
  keyCount: number = 0;

  constructor(v?: INetworkPeerNegotiateRequest) {
    this.keyCount = v?.keyCount || 0;
  }

  static encode(m: NetworkPeerNegotiateRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.keyCount) w.uint32(8).uint32(m.keyCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerNegotiateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerNegotiateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.keyCount = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerNegotiateResponse {
  keyCount?: number;
}

export class NetworkPeerNegotiateResponse {
  keyCount: number = 0;

  constructor(v?: INetworkPeerNegotiateResponse) {
    this.keyCount = v?.keyCount || 0;
  }

  static encode(m: NetworkPeerNegotiateResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.keyCount) w.uint32(8).uint32(m.keyCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerNegotiateResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerNegotiateResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.keyCount = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerBinding {
  port?: number;
  certificate?: strims_type_ICertificate | undefined;
}

export class NetworkPeerBinding {
  port: number = 0;
  certificate: strims_type_Certificate | undefined;

  constructor(v?: INetworkPeerBinding) {
    this.port = v?.port || 0;
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: NetworkPeerBinding, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.port) w.uint32(8).uint32(m.port);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerBinding {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerBinding();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.port = r.uint32();
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerOpenRequest {
  bindings?: INetworkPeerBinding[];
}

export class NetworkPeerOpenRequest {
  bindings: NetworkPeerBinding[] = [];

  constructor(v?: INetworkPeerOpenRequest) {
    if (v?.bindings) this.bindings = v.bindings.map(v => new NetworkPeerBinding(v));
  }

  static encode(m: NetworkPeerOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    for (const v of m.bindings) NetworkPeerBinding.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerOpenRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerOpenRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bindings.push(NetworkPeerBinding.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerOpenResponse {
  bindings?: INetworkPeerBinding[];
}

export class NetworkPeerOpenResponse {
  bindings: NetworkPeerBinding[] = [];

  constructor(v?: INetworkPeerOpenResponse) {
    if (v?.bindings) this.bindings = v.bindings.map(v => new NetworkPeerBinding(v));
  }

  static encode(m: NetworkPeerOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    for (const v of m.bindings) NetworkPeerBinding.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerOpenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerOpenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bindings.push(NetworkPeerBinding.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerCloseRequest {
  key?: Uint8Array;
}

export class NetworkPeerCloseRequest {
  key: Uint8Array = new Uint8Array();

  constructor(v?: INetworkPeerCloseRequest) {
    this.key = v?.key || new Uint8Array();
  }

  static encode(m: NetworkPeerCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.key) w.uint32(10).bytes(m.key);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerCloseRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerCloseRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerCloseResponse {
}

export class NetworkPeerCloseResponse {

  constructor(v?: INetworkPeerCloseResponse) {
    // noop
  }

  static encode(m: NetworkPeerCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new NetworkPeerCloseResponse();
  }
}

export interface INetworkPeerUpdateCertificateRequest {
  certificate?: strims_type_ICertificate | undefined;
}

export class NetworkPeerUpdateCertificateRequest {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: INetworkPeerUpdateCertificateRequest) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: NetworkPeerUpdateCertificateRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerUpdateCertificateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkPeerUpdateCertificateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface INetworkPeerUpdateCertificateResponse {
}

export class NetworkPeerUpdateCertificateResponse {

  constructor(v?: INetworkPeerUpdateCertificateResponse) {
    // noop
  }

  static encode(m: NetworkPeerUpdateCertificateResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerUpdateCertificateResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new NetworkPeerUpdateCertificateResponse();
  }
}

