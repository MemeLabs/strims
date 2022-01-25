import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
  CertificateRequest as strims_type_CertificateRequest,
  ICertificateRequest as strims_type_ICertificateRequest,
} from "../../../type/certificate";

export type ICertificateLog = {
  id?: bigint;
  networkId?: bigint;
  certificate?: strims_type_ICertificate;
}

export class CertificateLog {
  id: bigint;
  networkId: bigint;
  certificate: strims_type_Certificate | undefined;

  constructor(v?: ICertificateLog) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: CertificateLog, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CertificateLog {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CertificateLog();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.networkId = r.uint64();
        break;
        case 3:
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

export type ICARenewRequest = {
  certificate?: strims_type_ICertificate;
  certificateRequest?: strims_type_ICertificateRequest;
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
  certificate?: strims_type_ICertificate;
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

export type ICAFindRequest = {
  subject?: string;
  serialNumber?: Uint8Array;
  fullChain?: boolean;
}

export class CAFindRequest {
  subject: string;
  serialNumber: Uint8Array;
  fullChain: boolean;

  constructor(v?: ICAFindRequest) {
    this.subject = v?.subject || "";
    this.serialNumber = v?.serialNumber || new Uint8Array();
    this.fullChain = v?.fullChain || false;
  }

  static encode(m: CAFindRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.subject) w.uint32(10).string(m.subject);
    if (m.serialNumber) w.uint32(18).bytes(m.serialNumber);
    if (m.fullChain) w.uint32(24).bool(m.fullChain);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CAFindRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CAFindRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.subject = r.string();
        break;
        case 2:
        m.serialNumber = r.bytes();
        break;
        case 3:
        m.fullChain = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICAFindResponse = {
  certificate?: strims_type_ICertificate;
}

export class CAFindResponse {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: ICAFindResponse) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: CAFindResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CAFindResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CAFindResponse();
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

