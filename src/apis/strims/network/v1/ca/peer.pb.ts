// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  type ICertificate as strims_type_ICertificate,
  CertificateRequest as strims_type_CertificateRequest,
  type ICertificateRequest as strims_type_ICertificateRequest,
} from "../../../type/certificate.pb";

export type ICAPeerRenewRequest = {
  certificate?: strims_type_ICertificate;
  certificateRequest?: strims_type_ICertificateRequest;
}

export class CAPeerRenewRequest {
  certificate: strims_type_Certificate | undefined;
  certificateRequest: strims_type_CertificateRequest | undefined;

  constructor(v?: ICAPeerRenewRequest) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.certificateRequest = v?.certificateRequest && new strims_type_CertificateRequest(v.certificateRequest);
  }

  static encode(m: CAPeerRenewRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    if (m.certificateRequest) strims_type_CertificateRequest.encode(m.certificateRequest, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CAPeerRenewRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CAPeerRenewRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 2:
        m.certificateRequest = strims_type_CertificateRequest.decode(r, r.uint32());
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

export type ICAPeerRenewResponse = {
  certificate?: strims_type_ICertificate;
}

export class CAPeerRenewResponse {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: ICAPeerRenewResponse) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: CAPeerRenewResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CAPeerRenewResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CAPeerRenewResponse();
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

