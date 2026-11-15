// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import type {
  KeyType as strims_type_KeyType,
} from "./key.pb";

/** What a certificate request asks for. */
export type ICertificateRequestBody = {
  key?: Uint8Array;
  keyType?: strims_type_KeyType;
  keyUsage?: KeyUsage;
  subject?: string;
}

/** What a certificate request asks for. */
export class CertificateRequestBody {
  key: Uint8Array;
  keyType: strims_type_KeyType;
  keyUsage: KeyUsage;
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/**
 * A CertificateRequestBody's encoding, signed by its key in the
 * certificate-request domain.
 */
export type ICertificateRequest = {
  body?: Uint8Array;
  signature?: Uint8Array;
}

/**
 * A CertificateRequestBody's encoding, signed by its key in the
 * certificate-request domain.
 */
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/**
 * What a certificate states. The issuer serial number names the
 * certificate whose key signs this one; it is empty for a self-signed
 * certificate.
 */
export type ICertificateBody = {
  key?: Uint8Array;
  keyType?: strims_type_KeyType;
  keyUsage?: KeyUsage;
  subject?: string;
  notBefore?: bigint;
  notAfter?: bigint;
  serialNumber?: Uint8Array;
  issuerSerialNumber?: Uint8Array;
}

/**
 * What a certificate states. The issuer serial number names the
 * certificate whose key signs this one; it is empty for a self-signed
 * certificate.
 */
export class CertificateBody {
  key: Uint8Array;
  keyType: strims_type_KeyType;
  keyUsage: KeyUsage;
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/**
 * A CertificateBody's encoding, signed in the certificate domain by its
 * issuer's key, with the issuer's certificate as its parent. A certificate
 * stored apart from its chain has no parent.
 */
export type ICertificate = {
  body?: Uint8Array;
  signature?: Uint8Array;
  parent?: ICertificate;
}

/**
 * A CertificateBody's encoding, signed in the certificate domain by its
 * issuer's key, with the issuer's certificate as its parent. A certificate
 * stored apart from its chain has no parent.
 */
export class Certificate {
  body: Uint8Array;
  signature: Uint8Array;
  parent: Certificate | undefined;

  constructor(v?: ICertificate) {
    this.body = v?.body || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
    this.parent = v?.parent && new Certificate(v.parent);
  }

  static encode(m: Certificate, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).bytes(m.body);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    if (m.parent) Certificate.encode(m.parent, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Certificate {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        m.parent = Certificate.decode(r, r.uint32());
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

export enum KeyUsage {
  KEY_USAGE_UNDEFINED = 0,
  /** PEER allows bearer to connect with members of the signator's network */
  KEY_USAGE_PEER = 1,
  /**
   * BOOTSTRAP allows the bearer to connect to a network's signators. Invites
   * including transient keys with bootstrap certs allow new members to request
   * peer certs.
   */
  KEY_USAGE_BOOTSTRAP = 2,
  /** SIGN allows the bearer to sign certificates. */
  KEY_USAGE_SIGN = 4,
  /**
   * BROKER allows the bearer to negotiate connections between a network's
   * members.
   */
  KEY_USAGE_BROKER = 8,
  /** ENCIPHERMENT allows the key to be used for encrypting messages. */
  KEY_USAGE_ENCIPHERMENT = 16,
}

