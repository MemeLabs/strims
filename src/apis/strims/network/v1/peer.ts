import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Certificate,
  strims_type_ICertificate,
} from "../../type/certificate";

export type INetworkPeerNegotiateRequest = {
  keyCount?: number;
}

export class NetworkPeerNegotiateRequest {
  keyCount: number;

  constructor(v?: INetworkPeerNegotiateRequest) {
    this.keyCount = v?.keyCount || 0;
  }

  static encode(m: NetworkPeerNegotiateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type INetworkPeerNegotiateResponse = {
  keyCount?: number;
}

export class NetworkPeerNegotiateResponse {
  keyCount: number;

  constructor(v?: INetworkPeerNegotiateResponse) {
    this.keyCount = v?.keyCount || 0;
  }

  static encode(m: NetworkPeerNegotiateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type INetworkPeerBinding = {
  port?: number;
  certificate?: strims_type_ICertificate;
}

export class NetworkPeerBinding {
  port: number;
  certificate: strims_type_Certificate | undefined;

  constructor(v?: INetworkPeerBinding) {
    this.port = v?.port || 0;
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: NetworkPeerBinding, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type INetworkPeerOpenRequest = {
  bindings?: strims_network_v1_INetworkPeerBinding[];
}

export class NetworkPeerOpenRequest {
  bindings: strims_network_v1_NetworkPeerBinding[];

  constructor(v?: INetworkPeerOpenRequest) {
    this.bindings = v?.bindings ? v.bindings.map(v => new strims_network_v1_NetworkPeerBinding(v)) : [];
  }

  static encode(m: NetworkPeerOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.bindings) strims_network_v1_NetworkPeerBinding.encode(v, w.uint32(10).fork()).ldelim();
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
        m.bindings.push(strims_network_v1_NetworkPeerBinding.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkPeerOpenResponse = {
  bindings?: strims_network_v1_INetworkPeerBinding[];
}

export class NetworkPeerOpenResponse {
  bindings: strims_network_v1_NetworkPeerBinding[];

  constructor(v?: INetworkPeerOpenResponse) {
    this.bindings = v?.bindings ? v.bindings.map(v => new strims_network_v1_NetworkPeerBinding(v)) : [];
  }

  static encode(m: NetworkPeerOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.bindings) strims_network_v1_NetworkPeerBinding.encode(v, w.uint32(10).fork()).ldelim();
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
        m.bindings.push(strims_network_v1_NetworkPeerBinding.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkPeerCloseRequest = {
  key?: Uint8Array;
}

export class NetworkPeerCloseRequest {
  key: Uint8Array;

  constructor(v?: INetworkPeerCloseRequest) {
    this.key = v?.key || new Uint8Array();
  }

  static encode(m: NetworkPeerCloseRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
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

export type INetworkPeerCloseResponse = Record<string, any>;

export class NetworkPeerCloseResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: INetworkPeerCloseResponse) {
  }

  static encode(m: NetworkPeerCloseResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerCloseResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new NetworkPeerCloseResponse();
  }
}

export type INetworkPeerUpdateCertificateRequest = {
  certificate?: strims_type_ICertificate;
}

export class NetworkPeerUpdateCertificateRequest {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: INetworkPeerUpdateCertificateRequest) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: NetworkPeerUpdateCertificateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type INetworkPeerUpdateCertificateResponse = Record<string, any>;

export class NetworkPeerUpdateCertificateResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: INetworkPeerUpdateCertificateResponse) {
  }

  static encode(m: NetworkPeerUpdateCertificateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkPeerUpdateCertificateResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new NetworkPeerUpdateCertificateResponse();
  }
}

/* @internal */
export const strims_network_v1_NetworkPeerNegotiateRequest = NetworkPeerNegotiateRequest;
/* @internal */
export type strims_network_v1_NetworkPeerNegotiateRequest = NetworkPeerNegotiateRequest;
/* @internal */
export type strims_network_v1_INetworkPeerNegotiateRequest = INetworkPeerNegotiateRequest;
/* @internal */
export const strims_network_v1_NetworkPeerNegotiateResponse = NetworkPeerNegotiateResponse;
/* @internal */
export type strims_network_v1_NetworkPeerNegotiateResponse = NetworkPeerNegotiateResponse;
/* @internal */
export type strims_network_v1_INetworkPeerNegotiateResponse = INetworkPeerNegotiateResponse;
/* @internal */
export const strims_network_v1_NetworkPeerBinding = NetworkPeerBinding;
/* @internal */
export type strims_network_v1_NetworkPeerBinding = NetworkPeerBinding;
/* @internal */
export type strims_network_v1_INetworkPeerBinding = INetworkPeerBinding;
/* @internal */
export const strims_network_v1_NetworkPeerOpenRequest = NetworkPeerOpenRequest;
/* @internal */
export type strims_network_v1_NetworkPeerOpenRequest = NetworkPeerOpenRequest;
/* @internal */
export type strims_network_v1_INetworkPeerOpenRequest = INetworkPeerOpenRequest;
/* @internal */
export const strims_network_v1_NetworkPeerOpenResponse = NetworkPeerOpenResponse;
/* @internal */
export type strims_network_v1_NetworkPeerOpenResponse = NetworkPeerOpenResponse;
/* @internal */
export type strims_network_v1_INetworkPeerOpenResponse = INetworkPeerOpenResponse;
/* @internal */
export const strims_network_v1_NetworkPeerCloseRequest = NetworkPeerCloseRequest;
/* @internal */
export type strims_network_v1_NetworkPeerCloseRequest = NetworkPeerCloseRequest;
/* @internal */
export type strims_network_v1_INetworkPeerCloseRequest = INetworkPeerCloseRequest;
/* @internal */
export const strims_network_v1_NetworkPeerCloseResponse = NetworkPeerCloseResponse;
/* @internal */
export type strims_network_v1_NetworkPeerCloseResponse = NetworkPeerCloseResponse;
/* @internal */
export type strims_network_v1_INetworkPeerCloseResponse = INetworkPeerCloseResponse;
/* @internal */
export const strims_network_v1_NetworkPeerUpdateCertificateRequest = NetworkPeerUpdateCertificateRequest;
/* @internal */
export type strims_network_v1_NetworkPeerUpdateCertificateRequest = NetworkPeerUpdateCertificateRequest;
/* @internal */
export type strims_network_v1_INetworkPeerUpdateCertificateRequest = INetworkPeerUpdateCertificateRequest;
/* @internal */
export const strims_network_v1_NetworkPeerUpdateCertificateResponse = NetworkPeerUpdateCertificateResponse;
/* @internal */
export type strims_network_v1_NetworkPeerUpdateCertificateResponse = NetworkPeerUpdateCertificateResponse;
/* @internal */
export type strims_network_v1_INetworkPeerUpdateCertificateResponse = INetworkPeerUpdateCertificateResponse;
