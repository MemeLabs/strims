import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_KeyType,
} from "./key";

export type ICertificateRequestBody = {
  key?: Uint8Array;
  keyType?: strims_type_KeyType;
  keyUsage?: strims_type_KeyUsage;
  subject?: string;
}

export class CertificateRequestBody {
  key: Uint8Array;
  keyType: strims_type_KeyType;
  keyUsage: strims_type_KeyUsage;
  subject: string;

  constructor(v?: ICertificateRequestBody) {
    this.key = v?.key || new Uint8Array();
    this.keyType = v?.keyType || 0;
    this.keyUsage = v?.keyUsage || 0;
    this.subject = v?.subject || "";
  }

  static encode(m: CertificateRequestBody, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
    if (m.keyType) w.uint32(16).uint32(m.keyType);
    if (m.keyUsage) w.uint32(24).uint32(m.keyUsage);
    if (m.subject.length) w.uint32(34).string(m.subject);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CertificateRequestBody {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CertificateRequestBody();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        case 2:
        m.keyType = r.uint32();
        break;
        case 3:
        m.keyUsage = r.uint32();
        break;
        case 4:
        m.subject = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICertificateRequest = {
  body?: Uint8Array;
  signature?: Uint8Array;
}

export class CertificateRequest {
  body: Uint8Array;
  signature: Uint8Array;

  constructor(v?: ICertificateRequest) {
    this.body = v?.body || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: CertificateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).bytes(m.body);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CertificateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CertificateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = r.bytes();
        break;
        case 2:
        m.signature = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICertificateBody = {
  key?: Uint8Array;
  keyType?: strims_type_KeyType;
  keyUsage?: strims_type_KeyUsage;
  subject?: string;
  notBefore?: bigint;
  notAfter?: bigint;
  serialNumber?: Uint8Array;
  issuerSerialNumber?: Uint8Array;
}

export class CertificateBody {
  key: Uint8Array;
  keyType: strims_type_KeyType;
  keyUsage: strims_type_KeyUsage;
  subject: string;
  notBefore: bigint;
  notAfter: bigint;
  serialNumber: Uint8Array;
  issuerSerialNumber: Uint8Array;

  constructor(v?: ICertificateBody) {
    this.key = v?.key || new Uint8Array();
    this.keyType = v?.keyType || 0;
    this.keyUsage = v?.keyUsage || 0;
    this.subject = v?.subject || "";
    this.notBefore = v?.notBefore || BigInt(0);
    this.notAfter = v?.notAfter || BigInt(0);
    this.serialNumber = v?.serialNumber || new Uint8Array();
    this.issuerSerialNumber = v?.issuerSerialNumber || new Uint8Array();
  }

  static encode(m: CertificateBody, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
    if (m.keyType) w.uint32(16).uint32(m.keyType);
    if (m.keyUsage) w.uint32(24).uint32(m.keyUsage);
    if (m.subject.length) w.uint32(34).string(m.subject);
    if (m.notBefore) w.uint32(40).uint64(m.notBefore);
    if (m.notAfter) w.uint32(48).uint64(m.notAfter);
    if (m.serialNumber.length) w.uint32(58).bytes(m.serialNumber);
    if (m.issuerSerialNumber.length) w.uint32(66).bytes(m.issuerSerialNumber);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CertificateBody {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CertificateBody();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        case 2:
        m.keyType = r.uint32();
        break;
        case 3:
        m.keyUsage = r.uint32();
        break;
        case 4:
        m.subject = r.string();
        break;
        case 5:
        m.notBefore = r.uint64();
        break;
        case 6:
        m.notAfter = r.uint64();
        break;
        case 7:
        m.serialNumber = r.bytes();
        break;
        case 8:
        m.issuerSerialNumber = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICertificate = {
  body?: Uint8Array;
  signature?: Uint8Array;
  parent?: strims_type_ICertificate;
}

export class Certificate {
  body: Uint8Array;
  signature: Uint8Array;
  parent: strims_type_Certificate | undefined;

  constructor(v?: ICertificate) {
    this.body = v?.body || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
    this.parent = v?.parent && new strims_type_Certificate(v.parent);
  }

  static encode(m: Certificate, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).bytes(m.body);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    if (m.parent) strims_type_Certificate.encode(m.parent, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Certificate {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Certificate();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = r.bytes();
        break;
        case 2:
        m.signature = r.bytes();
        break;
        case 3:
        m.parent = strims_type_Certificate.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export enum KeyUsage {
  KEY_USAGE_UNDEFINED = 0,
  KEY_USAGE_PEER = 1,
  KEY_USAGE_BOOTSTRAP = 2,
  KEY_USAGE_SIGN = 4,
  KEY_USAGE_BROKER = 8,
  KEY_USAGE_ENCIPHERMENT = 16,
}
/* @internal */
export const strims_type_CertificateRequestBody = CertificateRequestBody;
/* @internal */
export type strims_type_CertificateRequestBody = CertificateRequestBody;
/* @internal */
export type strims_type_ICertificateRequestBody = ICertificateRequestBody;
/* @internal */
export const strims_type_CertificateRequest = CertificateRequest;
/* @internal */
export type strims_type_CertificateRequest = CertificateRequest;
/* @internal */
export type strims_type_ICertificateRequest = ICertificateRequest;
/* @internal */
export const strims_type_CertificateBody = CertificateBody;
/* @internal */
export type strims_type_CertificateBody = CertificateBody;
/* @internal */
export type strims_type_ICertificateBody = ICertificateBody;
/* @internal */
export const strims_type_Certificate = Certificate;
/* @internal */
export type strims_type_Certificate = Certificate;
/* @internal */
export type strims_type_ICertificate = ICertificate;
/* @internal */
export const strims_type_KeyUsage = KeyUsage;
/* @internal */
export type strims_type_KeyUsage = KeyUsage;
