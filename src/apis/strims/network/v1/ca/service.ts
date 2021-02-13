import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
  CertificateRequest as strims_type_CertificateRequest,
  ICertificateRequest as strims_type_ICertificateRequest,
} from "../../../type/certificate";

export type ICARenewRequest = {
  certificate?: strims_type_ICertificate | undefined;
  certificateRequest?: strims_type_ICertificateRequest | undefined;
}

export class CARenewRequest {
  certificate: strims_type_Certificate | undefined;
  certificateRequest: strims_type_CertificateRequest | undefined;

  constructor(v?: ICARenewRequest) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.certificateRequest = v?.certificateRequest && new strims_type_CertificateRequest(v.certificateRequest);
  }

  static encode(m: CARenewRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    if (m.certificateRequest) strims_type_CertificateRequest.encode(m.certificateRequest, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CARenewRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CARenewRequest();
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICARenewResponse = {
  certificate?: strims_type_ICertificate | undefined;
}

export class CARenewResponse {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: ICARenewResponse) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: CARenewResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CARenewResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CARenewResponse();
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

