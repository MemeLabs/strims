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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICache = {
  id?: bigint;
  uri?: string;
  integrity?: Cache.IIntegrity;
  data?: Uint8Array;
  epoch?: Cache.IEpoch;
}

export class Cache {
  id: bigint;
  uri: string;
  integrity: Cache.Integrity | undefined;
  data: Uint8Array;
  epoch: Cache.Epoch | undefined;

  constructor(v?: ICache) {
    this.id = v?.id || BigInt(0);
    this.uri = v?.uri || "";
    this.integrity = v?.integrity && new Cache.Integrity(v.integrity);
    this.data = v?.data || new Uint8Array();
    this.epoch = v?.epoch && new Cache.Epoch(v.epoch);
  }

  static encode(m: Cache, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.uri.length) w.uint32(18).string(m.uri);
    if (m.integrity) Cache.Integrity.encode(m.integrity, w.uint32(26).fork()).ldelim();
    if (m.data.length) w.uint32(34).bytes(m.data);
    if (m.epoch) Cache.Epoch.encode(m.epoch, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Cache {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.integrity = Cache.Integrity.decode(r, r.uint32());
        break;
        case 4:
        m.data = r.bytes();
        break;
        case 5:
        m.epoch = Cache.Epoch.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Cache {
  export type ISignAllIntegrity = {
    timestamps?: bigint[];
    signatures?: Uint8Array;
  }

  export class SignAllIntegrity {
    timestamps: bigint[];
    signatures: Uint8Array;

    constructor(v?: ISignAllIntegrity) {
      this.timestamps = v?.timestamps ? v.timestamps : [];
      this.signatures = v?.signatures || new Uint8Array();
    }

    static encode(m: SignAllIntegrity, w?: Writer): Writer {
      if (!w) w = new Writer();
      m.timestamps.reduce((w, v) => w.int64(v), w.uint32(10).fork()).ldelim();
      if (m.signatures.length) w.uint32(18).bytes(m.signatures);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): SignAllIntegrity {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new SignAllIntegrity();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IMerkleIntegrity = {
    timestamps?: bigint[];
    signatures?: Uint8Array[];
  }

  export class MerkleIntegrity {
    timestamps: bigint[];
    signatures: Uint8Array[];

    constructor(v?: IMerkleIntegrity) {
      this.timestamps = v?.timestamps ? v.timestamps : [];
      this.signatures = v?.signatures ? v.signatures : [];
    }

    static encode(m: MerkleIntegrity, w?: Writer): Writer {
      if (!w) w = new Writer();
      m.timestamps.reduce((w, v) => w.int64(v), w.uint32(10).fork()).ldelim();
      for (const v of m.signatures) w.uint32(18).bytes(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): MerkleIntegrity {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new MerkleIntegrity();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IIntegrity = {
    signAllIntegrity?: Cache.ISignAllIntegrity;
    merkleIntegrity?: Cache.IMerkleIntegrity;
  }

  export class Integrity {
    signAllIntegrity: Cache.SignAllIntegrity | undefined;
    merkleIntegrity: Cache.MerkleIntegrity | undefined;

    constructor(v?: IIntegrity) {
      this.signAllIntegrity = v?.signAllIntegrity && new Cache.SignAllIntegrity(v.signAllIntegrity);
      this.merkleIntegrity = v?.merkleIntegrity && new Cache.MerkleIntegrity(v.merkleIntegrity);
    }

    static encode(m: Integrity, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.signAllIntegrity) Cache.SignAllIntegrity.encode(m.signAllIntegrity, w.uint32(8010).fork()).ldelim();
      if (m.merkleIntegrity) Cache.MerkleIntegrity.encode(m.merkleIntegrity, w.uint32(8018).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Integrity {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Integrity();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1001:
          m.signAllIntegrity = Cache.SignAllIntegrity.decode(r, r.uint32());
          break;
          case 1002:
          m.merkleIntegrity = Cache.MerkleIntegrity.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IEpoch = {
    timestamp?: bigint;
    signature?: Uint8Array;
  }

  export class Epoch {
    timestamp: bigint;
    signature: Uint8Array;

    constructor(v?: IEpoch) {
      this.timestamp = v?.timestamp || BigInt(0);
      this.signature = v?.signature || new Uint8Array();
    }

    static encode(m: Epoch, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.timestamp) w.uint32(8).int64(m.timestamp);
      if (m.signature.length) w.uint32(18).bytes(m.signature);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Epoch {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Epoch();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

