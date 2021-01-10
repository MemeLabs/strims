import Reader from "../../../../../pb/reader";
import Writer from "../../../../../pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate
} from "../../../type/certificate";

export interface IBootstrapPeerGetPublishEnabledRequest {
}

export class BootstrapPeerGetPublishEnabledRequest {

  constructor(v?: IBootstrapPeerGetPublishEnabledRequest) {
    // noop
  }

  static encode(m: BootstrapPeerGetPublishEnabledRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerGetPublishEnabledRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerGetPublishEnabledRequest();
  }
}

export interface IBootstrapPeerGetPublishEnabledResponse {
  enabled?: boolean;
}

export class BootstrapPeerGetPublishEnabledResponse {
  enabled: boolean = false;

  constructor(v?: IBootstrapPeerGetPublishEnabledResponse) {
    this.enabled = v?.enabled || false;
  }

  static encode(m: BootstrapPeerGetPublishEnabledResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.enabled) w.uint32(8).bool(m.enabled);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerGetPublishEnabledResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapPeerGetPublishEnabledResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enabled = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IBootstrapPeerListNetworksRequest {
}

export class BootstrapPeerListNetworksRequest {

  constructor(v?: IBootstrapPeerListNetworksRequest) {
    // noop
  }

  static encode(m: BootstrapPeerListNetworksRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerListNetworksRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerListNetworksRequest();
  }
}

export interface IBootstrapPeerListNetworksResponse {
}

export class BootstrapPeerListNetworksResponse {

  constructor(v?: IBootstrapPeerListNetworksResponse) {
    // noop
  }

  static encode(m: BootstrapPeerListNetworksResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerListNetworksResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerListNetworksResponse();
  }
}

export interface IBootstrapPeerPublishRequest {
  certificate?: strims_type_ICertificate;
}

export class BootstrapPeerPublishRequest {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: IBootstrapPeerPublishRequest) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: BootstrapPeerPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerPublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapPeerPublishRequest();
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

export interface IBootstrapPeerPublishResponse {
}

export class BootstrapPeerPublishResponse {

  constructor(v?: IBootstrapPeerPublishResponse) {
    // noop
  }

  static encode(m: BootstrapPeerPublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerPublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerPublishResponse();
  }
}

