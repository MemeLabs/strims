// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type ICacheMeta = {
  id?: bigint;
  swarmId?: Uint8Array;
  swarmSalt?: Uint8Array;
  checksum?: number;
}

export class CacheMeta {
  id: bigint;
  swarmId: Uint8Array;
  swarmSalt: Uint8Array;
  checksum: number;

  constructor(v?: ICacheMeta) {
    this.id = v?.id || BigInt(0);
    this.swarmId = v?.swarmId || new Uint8Array();
    this.swarmSalt = v?.swarmSalt || new Uint8Array();
    this.checksum = v?.checksum || 0;
  }

  static encode(m: CacheMeta, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.swarmId.length) w.uint32(18).bytes(m.swarmId);
    if (m.swarmSalt.length) w.uint32(26).bytes(m.swarmSalt);
    if (m.checksum) w.uint32(32).uint32(m.checksum);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CacheMeta {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CacheMeta();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.swarmId = r.bytes();
        break;
        case 3:
        m.swarmSalt = r.bytes();
        break;
        case 4:
        m.checksum = r.uint32();
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

export type ICache = {
  id?: bigint;
  uri?: string;
  integrity?: Cache_IIntegrity;
  data?: Uint8Array;
  epoch?: Cache_IEpoch;
}

export class Cache {
  id: bigint;
  uri: string;
  integrity: Cache_Integrity | undefined;
  data: Uint8Array;
  epoch: Cache_Epoch | undefined;

  constructor(v?: ICache) {
    this.id = v?.id || BigInt(0);
    this.uri = v?.uri || "";
    this.integrity = v?.integrity && new Cache_Integrity(v.integrity);
    this.data = v?.data || new Uint8Array();
    this.epoch = v?.epoch && new Cache_Epoch(v.epoch);
  }

  static encode(m: Cache, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.uri.length) w.uint32(18).string(m.uri);
    if (m.integrity) Cache_Integrity.encode(m.integrity, w.uint32(26).fork()).ldelim();
    if (m.data.length) w.uint32(34).bytes(m.data);
    if (m.epoch) Cache_Epoch.encode(m.epoch, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cache {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Cache();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.uri = r.string();
        break;
        case 3:
        m.integrity = Cache_Integrity.decode(r, r.uint32());
        break;
        case 4:
        m.data = r.bytes();
        break;
        case 5:
        m.epoch = Cache_Epoch.decode(r, r.uint32());
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

export type Cache_ISignAllIntegrity = {
  timestamps?: bigint[];
  signatures?: Uint8Array;
}

export class Cache_SignAllIntegrity {
  timestamps: bigint[];
  signatures: Uint8Array;

  constructor(v?: Cache_ISignAllIntegrity) {
    this.timestamps = v?.timestamps ? v.timestamps : [];
    this.signatures = v?.signatures || new Uint8Array();
  }

  static encode(m: Cache_SignAllIntegrity, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.timestamps.reduce((w, v) => w.int64(v), w.uint32(10).fork()).ldelim();
    if (m.signatures.length) w.uint32(18).bytes(m.signatures);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cache_SignAllIntegrity {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Cache_SignAllIntegrity();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.timestamps.push(r.int64());
        break;
        case 2:
        m.signatures = r.bytes();
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

export type Cache_IMerkleIntegrity = {
  timestamps?: bigint[];
  signatures?: Uint8Array[];
}

export class Cache_MerkleIntegrity {
  timestamps: bigint[];
  signatures: Uint8Array[];

  constructor(v?: Cache_IMerkleIntegrity) {
    this.timestamps = v?.timestamps ? v.timestamps : [];
    this.signatures = v?.signatures ? v.signatures : [];
  }

  static encode(m: Cache_MerkleIntegrity, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.timestamps.reduce((w, v) => w.int64(v), w.uint32(10).fork()).ldelim();
    for (const v of m.signatures) w.uint32(18).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cache_MerkleIntegrity {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Cache_MerkleIntegrity();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.timestamps.push(r.int64());
        break;
        case 2:
        m.signatures.push(r.bytes())
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

export type Cache_IIntegrity = {
  signAllIntegrity?: Cache_ISignAllIntegrity;
  merkleIntegrity?: Cache_IMerkleIntegrity;
}

export class Cache_Integrity {
  signAllIntegrity: Cache_SignAllIntegrity | undefined;
  merkleIntegrity: Cache_MerkleIntegrity | undefined;

  constructor(v?: Cache_IIntegrity) {
    this.signAllIntegrity = v?.signAllIntegrity && new Cache_SignAllIntegrity(v.signAllIntegrity);
    this.merkleIntegrity = v?.merkleIntegrity && new Cache_MerkleIntegrity(v.merkleIntegrity);
  }

  static encode(m: Cache_Integrity, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.signAllIntegrity) Cache_SignAllIntegrity.encode(m.signAllIntegrity, w.uint32(8010).fork()).ldelim();
    if (m.merkleIntegrity) Cache_MerkleIntegrity.encode(m.merkleIntegrity, w.uint32(8018).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cache_Integrity {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Cache_Integrity();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.signAllIntegrity = Cache_SignAllIntegrity.decode(r, r.uint32());
        break;
        case 1002:
        m.merkleIntegrity = Cache_MerkleIntegrity.decode(r, r.uint32());
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

export type Cache_IEpoch = {
  timestamp?: bigint;
  signature?: Uint8Array;
}

export class Cache_Epoch {
  timestamp: bigint;
  signature: Uint8Array;

  constructor(v?: Cache_IEpoch) {
    this.timestamp = v?.timestamp || BigInt(0);
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: Cache_Epoch, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.timestamp) w.uint32(8).int64(m.timestamp);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cache_Epoch {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Cache_Epoch();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.timestamp = r.int64();
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

