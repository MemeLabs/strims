// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  type ICertificate as strims_type_ICertificate,
} from "../../../type/certificate.pb";

export type IBootstrapPeerGetPublishEnabledRequest = object;

export class BootstrapPeerGetPublishEnabledRequest {

  constructor(v?: IBootstrapPeerGetPublishEnabledRequest) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapPeerGetPublishEnabledResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enabled = r.bool();
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

export type IBootstrapPeerListNetworksRequest = object;

export class BootstrapPeerListNetworksRequest {

  constructor(v?: IBootstrapPeerListNetworksRequest) {
    void v; // an empty message has no fields to set
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

export type IBootstrapPeerListNetworksResponse = object;

export class BootstrapPeerListNetworksResponse {

  constructor(v?: IBootstrapPeerListNetworksResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapPeerPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
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

export type IBootstrapPeerPublishResponse = object;

export class BootstrapPeerPublishResponse {

  constructor(v?: IBootstrapPeerPublishResponse) {
    void v; // an empty message has no fields to set
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

