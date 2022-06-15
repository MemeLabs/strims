import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
} from "../../../type/certificate";

export type IBootstrapPeerGetPublishEnabledRequest = Record<string, any>;

export class BootstrapPeerGetPublishEnabledRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBootstrapPeerGetPublishEnabledRequest) {
  }

  static encode(m: BootstrapPeerGetPublishEnabledRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerGetPublishEnabledRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerGetPublishEnabledRequest();
  }
}

export type IBootstrapPeerGetPublishEnabledResponse = {
  enabled?: boolean;
}

export class BootstrapPeerGetPublishEnabledResponse {
  enabled: boolean;

  constructor(v?: IBootstrapPeerGetPublishEnabledResponse) {
    this.enabled = v?.enabled || false;
  }

  static encode(m: BootstrapPeerGetPublishEnabledResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type IBootstrapPeerListNetworksRequest = Record<string, any>;

export class BootstrapPeerListNetworksRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBootstrapPeerListNetworksRequest) {
  }

  static encode(m: BootstrapPeerListNetworksRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerListNetworksRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerListNetworksRequest();
  }
}

export type IBootstrapPeerListNetworksResponse = Record<string, any>;

export class BootstrapPeerListNetworksResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBootstrapPeerListNetworksResponse) {
  }

  static encode(m: BootstrapPeerListNetworksResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerListNetworksResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerListNetworksResponse();
  }
}

export type IBootstrapPeerPublishRequest = {
  certificate?: strims_type_ICertificate;
}

export class BootstrapPeerPublishRequest {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: IBootstrapPeerPublishRequest) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: BootstrapPeerPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
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

export type IBootstrapPeerPublishResponse = Record<string, any>;

export class BootstrapPeerPublishResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IBootstrapPeerPublishResponse) {
  }

  static encode(m: BootstrapPeerPublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeerPublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapPeerPublishResponse();
  }
}

