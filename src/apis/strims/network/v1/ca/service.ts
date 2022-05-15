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
  fullChain?: boolean;
  query?: CAFindRequest.IQuery
}

export class CAFindRequest {
  fullChain: boolean;
  query: CAFindRequest.TQuery;

  constructor(v?: ICAFindRequest) {
    this.fullChain = v?.fullChain || false;
    this.query = new CAFindRequest.Query(v?.query);
  }

  static encode(m: CAFindRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.fullChain) w.uint32(32).bool(m.fullChain);
    switch (m.query.case) {
      case CAFindRequest.QueryCase.SUBJECT:
      w.uint32(8010).string(m.query.subject);
      break;
      case CAFindRequest.QueryCase.SERIAL_NUMBER:
      w.uint32(8018).bytes(m.query.serialNumber);
      break;
      case CAFindRequest.QueryCase.KEY:
      w.uint32(8026).bytes(m.query.key);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CAFindRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CAFindRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.query = new CAFindRequest.Query({ subject: r.string() });
        break;
        case 1002:
        m.query = new CAFindRequest.Query({ serialNumber: r.bytes() });
        break;
        case 1003:
        m.query = new CAFindRequest.Query({ key: r.bytes() });
        break;
        case 4:
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

export namespace CAFindRequest {
  export enum QueryCase {
    NOT_SET = 0,
    SUBJECT = 1001,
    SERIAL_NUMBER = 1002,
    KEY = 1003,
  }

  export type IQuery =
  { case?: QueryCase.NOT_SET }
  |{ case?: QueryCase.SUBJECT, subject: string }
  |{ case?: QueryCase.SERIAL_NUMBER, serialNumber: Uint8Array }
  |{ case?: QueryCase.KEY, key: Uint8Array }
  ;

  export type TQuery = Readonly<
  { case: QueryCase.NOT_SET }
  |{ case: QueryCase.SUBJECT, subject: string }
  |{ case: QueryCase.SERIAL_NUMBER, serialNumber: Uint8Array }
  |{ case: QueryCase.KEY, key: Uint8Array }
  >;

  class QueryImpl {
    subject: string;
    serialNumber: Uint8Array;
    key: Uint8Array;
    case: QueryCase = QueryCase.NOT_SET;

    constructor(v?: IQuery) {
      if (v && "subject" in v) {
        this.case = QueryCase.SUBJECT;
        this.subject = v.subject;
      } else
      if (v && "serialNumber" in v) {
        this.case = QueryCase.SERIAL_NUMBER;
        this.serialNumber = v.serialNumber;
      } else
      if (v && "key" in v) {
        this.case = QueryCase.KEY;
        this.key = v.key;
      }
    }
  }

  export const Query = QueryImpl as {
    new (): Readonly<{ case: QueryCase.NOT_SET }>;
    new <T extends IQuery>(v: T): Readonly<
    T extends { subject: string } ? { case: QueryCase.SUBJECT, subject: string } :
    T extends { serialNumber: Uint8Array } ? { case: QueryCase.SERIAL_NUMBER, serialNumber: Uint8Array } :
    T extends { key: Uint8Array } ? { case: QueryCase.KEY, key: Uint8Array } :
    never
    >;
  };

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

