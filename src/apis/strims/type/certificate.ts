import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_KeyType,
} from "./key";

export type ICertificateRequest = {
  key?: Uint8Array;
  keyType?: strims_type_KeyType;
  keyUsage?: strims_type_KeyUsage;
  subject?: string;
  signature?: Uint8Array;
}

export class CertificateRequest {
  key: Uint8Array;
  keyType: strims_type_KeyType;
  keyUsage: strims_type_KeyUsage;
  subject: string;
  signature: Uint8Array;

  constructor(v?: ICertificateRequest) {
    this.key = v?.key || new Uint8Array();
    this.keyType = v?.keyType || 0;
    this.keyUsage = v?.keyUsage || 0;
    this.subject = v?.subject || "";
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: CertificateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
    if (m.keyType) w.uint32(16).uint32(m.keyType);
    if (m.keyUsage) w.uint32(24).uint32(m.keyUsage);
    if (m.subject.length) w.uint32(42).string(m.subject);
    if (m.signature.length) w.uint32(34).bytes(m.signature);
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
        m.key = r.bytes();
        break;
        case 2:
        m.keyType = r.uint32();
        break;
        case 3:
        m.keyUsage = r.uint32();
        break;
        case 5:
        m.subject = r.string();
        break;
        case 4:
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

export type ICertificate = {
  key?: Uint8Array;
  keyType?: strims_type_KeyType;
  keyUsage?: strims_type_KeyUsage;
  subject?: string;
  notBefore?: bigint;
  notAfter?: bigint;
  serialNumber?: Uint8Array;
  signature?: Uint8Array;
  parentOneof?: Certificate.IParentOneof
}

export class Certificate {
  key: Uint8Array;
  keyType: strims_type_KeyType;
  keyUsage: strims_type_KeyUsage;
  subject: string;
  notBefore: bigint;
  notAfter: bigint;
  serialNumber: Uint8Array;
  signature: Uint8Array;
  parentOneof: Certificate.TParentOneof;

  constructor(v?: ICertificate) {
    this.key = v?.key || new Uint8Array();
    this.keyType = v?.keyType || 0;
    this.keyUsage = v?.keyUsage || 0;
    this.subject = v?.subject || "";
    this.notBefore = v?.notBefore || BigInt(0);
    this.notAfter = v?.notAfter || BigInt(0);
    this.serialNumber = v?.serialNumber || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
    this.parentOneof = new Certificate.ParentOneof(v?.parentOneof);
  }

  static encode(m: Certificate, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
    if (m.keyType) w.uint32(16).uint32(m.keyType);
    if (m.keyUsage) w.uint32(24).uint32(m.keyUsage);
    if (m.subject.length) w.uint32(34).string(m.subject);
    if (m.notBefore) w.uint32(40).uint64(m.notBefore);
    if (m.notAfter) w.uint32(48).uint64(m.notAfter);
    if (m.serialNumber.length) w.uint32(58).bytes(m.serialNumber);
    if (m.signature.length) w.uint32(66).bytes(m.signature);
    switch (m.parentOneof.case) {
      case Certificate.ParentOneofCase.PARENT:
      strims_type_Certificate.encode(m.parentOneof.parent, w.uint32(74).fork()).ldelim();
      break;
      case Certificate.ParentOneofCase.PARENT_SERIAL_NUMBER:
      w.uint32(82).bytes(m.parentOneof.parentSerialNumber);
      break;
    }
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
        m.signature = r.bytes();
        break;
        case 9:
        m.parentOneof = new Certificate.ParentOneof({ parent: strims_type_Certificate.decode(r, r.uint32()) });
        break;
        case 10:
        m.parentOneof = new Certificate.ParentOneof({ parentSerialNumber: r.bytes() });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Certificate {
  export enum ParentOneofCase {
    NOT_SET = 0,
    PARENT = 9,
    PARENT_SERIAL_NUMBER = 10,
  }

  export type IParentOneof =
  { case?: ParentOneofCase.NOT_SET }
  |{ case?: ParentOneofCase.PARENT, parent: strims_type_ICertificate }
  |{ case?: ParentOneofCase.PARENT_SERIAL_NUMBER, parentSerialNumber: Uint8Array }
  ;

  export type TParentOneof = Readonly<
  { case: ParentOneofCase.NOT_SET }
  |{ case: ParentOneofCase.PARENT, parent: strims_type_Certificate }
  |{ case: ParentOneofCase.PARENT_SERIAL_NUMBER, parentSerialNumber: Uint8Array }
  >;

  class ParentOneofImpl {
    parent: strims_type_Certificate;
    parentSerialNumber: Uint8Array;
    case: ParentOneofCase = ParentOneofCase.NOT_SET;

    constructor(v?: IParentOneof) {
      if (v && "parent" in v) {
        this.case = ParentOneofCase.PARENT;
        this.parent = new strims_type_Certificate(v.parent);
      } else
      if (v && "parentSerialNumber" in v) {
        this.case = ParentOneofCase.PARENT_SERIAL_NUMBER;
        this.parentSerialNumber = v.parentSerialNumber;
      }
    }
  }

  export const ParentOneof = ParentOneofImpl as {
    new (): Readonly<{ case: ParentOneofCase.NOT_SET }>;
    new <T extends IParentOneof>(v: T): Readonly<
    T extends { parent: strims_type_ICertificate } ? { case: ParentOneofCase.PARENT, parent: strims_type_Certificate } :
    T extends { parentSerialNumber: Uint8Array } ? { case: ParentOneofCase.PARENT_SERIAL_NUMBER, parentSerialNumber: Uint8Array } :
    never
    >;
  };

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
export const strims_type_CertificateRequest = CertificateRequest;
/* @internal */
export type strims_type_CertificateRequest = CertificateRequest;
/* @internal */
export type strims_type_ICertificateRequest = ICertificateRequest;
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
