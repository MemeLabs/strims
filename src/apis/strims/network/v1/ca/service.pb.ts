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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CARenewResponse();
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

export type ICAFindRequest = {
  fullChain?: boolean;
  query?: CAFindRequest_IQuery
}

export class CAFindRequest {
  fullChain: boolean;
  query: CAFindRequest_TQuery;

  constructor(v?: ICAFindRequest) {
    this.fullChain = v?.fullChain || false;
    this.query = v?.query ? new CAFindRequest_Query(v.query) : new CAFindRequest_Query();
  }

  static encode(m: CAFindRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.fullChain) w.uint32(32).bool(m.fullChain);
    switch (m.query.case) {
      case CAFindRequest_QueryCase.SUBJECT:
      w.uint32(8010).string(m.query.subject);
      break;
      case CAFindRequest_QueryCase.SERIAL_NUMBER:
      w.uint32(8018).bytes(m.query.serialNumber);
      break;
      case CAFindRequest_QueryCase.KEY:
      w.uint32(8026).bytes(m.query.key);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CAFindRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CAFindRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.query = new CAFindRequest_Query({ subject: r.string() });
        break;
        case 1002:
        m.query = new CAFindRequest_Query({ serialNumber: r.bytes() });
        break;
        case 1003:
        m.query = new CAFindRequest_Query({ key: r.bytes() });
        break;
        case 4:
        m.fullChain = r.bool();
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

export enum CAFindRequest_QueryCase {
  NOT_SET = 0,
  SUBJECT = 1001,
  SERIAL_NUMBER = 1002,
  KEY = 1003,
}

export type CAFindRequest_IQuery =
{ case?: CAFindRequest_QueryCase.NOT_SET }
|{ case?: CAFindRequest_QueryCase.SUBJECT, subject: string }
|{ case?: CAFindRequest_QueryCase.SERIAL_NUMBER, serialNumber: Uint8Array }
|{ case?: CAFindRequest_QueryCase.KEY, key: Uint8Array }
;

export type CAFindRequest_TQuery = Readonly<
{ case: CAFindRequest_QueryCase.NOT_SET }
|{ case: CAFindRequest_QueryCase.SUBJECT, subject: string }
|{ case: CAFindRequest_QueryCase.SERIAL_NUMBER, serialNumber: Uint8Array }
|{ case: CAFindRequest_QueryCase.KEY, key: Uint8Array }
>;

class CAFindRequest_QueryImpl {
  declare subject?: string;
  declare serialNumber?: Uint8Array;
  declare key?: Uint8Array;
  case: CAFindRequest_QueryCase = CAFindRequest_QueryCase.NOT_SET;

  constructor(v?: CAFindRequest_IQuery) {
    if (v && "subject" in v) {
      this.case = CAFindRequest_QueryCase.SUBJECT;
      this.subject = v.subject;
    } else
    if (v && "serialNumber" in v) {
      this.case = CAFindRequest_QueryCase.SERIAL_NUMBER;
      this.serialNumber = v.serialNumber;
    } else
    if (v && "key" in v) {
      this.case = CAFindRequest_QueryCase.KEY;
      this.key = v.key;
    }
  }
}

export const CAFindRequest_Query = CAFindRequest_QueryImpl as {
  new (): Readonly<{ case: CAFindRequest_QueryCase.NOT_SET }>;
  new <T extends CAFindRequest_IQuery>(v: T): Readonly<
  T extends { subject: string } ? { case: CAFindRequest_QueryCase.SUBJECT, subject: string } :
  T extends { serialNumber: Uint8Array } ? { case: CAFindRequest_QueryCase.SERIAL_NUMBER, serialNumber: Uint8Array } :
  T extends { key: Uint8Array } ? { case: CAFindRequest_QueryCase.KEY, key: Uint8Array } :
  never
  >;
};

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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CAFindResponse();
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

