import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_auth_v1_PairingToken,
  strims_auth_v1_IPairingToken,
} from "../../auth/v1/auth";
import {
  strims_dao_v1_VersionVector,
  strims_dao_v1_IVersionVector,
} from "../../dao/v1/dao";

export type IVersion = {
  replicaId?: number;
  currentVersion?: strims_dao_v1_IVersionVector;
}

export class Version {
  replicaId: number;
  currentVersion: strims_dao_v1_VersionVector | undefined;

  constructor(v?: IVersion) {
    this.replicaId = v?.replicaId || 0;
    this.currentVersion = v?.currentVersion && new strims_dao_v1_VersionVector(v.currentVersion);
  }

  static encode(m: Version, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.replicaId) w.uint32(8).uint32(m.replicaId);
    if (m.currentVersion) strims_dao_v1_VersionVector.encode(m.currentVersion, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Version {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Version();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.replicaId = r.uint32();
        break;
        case 2:
        m.currentVersion = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEvent = {
  namespace?: bigint;
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  delete?: boolean;
  record?: Uint8Array;
}

export class Event {
  namespace: bigint;
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  delete: boolean;
  record: Uint8Array;

  constructor(v?: IEvent) {
    this.namespace = v?.namespace || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.delete = v?.delete || false;
    this.record = v?.record || new Uint8Array();
  }

  static encode(m: Event, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.namespace) w.uint32(8).int64(m.namespace);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(26).fork()).ldelim();
    if (m.delete) w.uint32(32).bool(m.delete);
    if (m.record.length) w.uint32(42).bytes(m.record);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Event();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.namespace = r.int64();
        break;
        case 2:
        m.id = r.uint64();
        break;
        case 3:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
        case 4:
        m.delete = r.bool();
        break;
        case 5:
        m.record = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEventBundle = {
  events?: strims_replication_v1_IEvent[];
}

export class EventBundle {
  events: strims_replication_v1_Event[];

  constructor(v?: IEventBundle) {
    this.events = v?.events ? v.events.map(v => new strims_replication_v1_Event(v)) : [];
  }

  static encode(m: EventBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) strims_replication_v1_Event.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EventBundle {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EventBundle();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.events.push(strims_replication_v1_Event.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEventLog = {
  id?: bigint;
  replicaId?: bigint;
  events?: strims_replication_v1_IEvent[];
}

export class EventLog {
  id: bigint;
  replicaId: bigint;
  events: strims_replication_v1_Event[];

  constructor(v?: IEventLog) {
    this.id = v?.id || BigInt(0);
    this.replicaId = v?.replicaId || BigInt(0);
    this.events = v?.events ? v.events.map(v => new strims_replication_v1_Event(v)) : [];
  }

  static encode(m: EventLog, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.replicaId) w.uint32(16).uint64(m.replicaId);
    for (const v of m.events) strims_replication_v1_Event.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EventLog {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EventLog();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.replicaId = r.uint64();
        break;
        case 3:
        m.events.push(strims_replication_v1_Event.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreatePairingTokenRequest = {
  networkId?: bigint;
  bootstrapId?: bigint;
}

export class CreatePairingTokenRequest {
  networkId: bigint;
  bootstrapId: bigint;

  constructor(v?: ICreatePairingTokenRequest) {
    this.networkId = v?.networkId || BigInt(0);
    this.bootstrapId = v?.bootstrapId || BigInt(0);
  }

  static encode(m: CreatePairingTokenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.bootstrapId) w.uint32(16).uint64(m.bootstrapId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreatePairingTokenRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreatePairingTokenRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.bootstrapId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreatePairingTokenResponse = {
  token?: strims_auth_v1_IPairingToken;
}

export class CreatePairingTokenResponse {
  token: strims_auth_v1_PairingToken | undefined;

  constructor(v?: ICreatePairingTokenResponse) {
    this.token = v?.token && new strims_auth_v1_PairingToken(v.token);
  }

  static encode(m: CreatePairingTokenResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.token) strims_auth_v1_PairingToken.encode(m.token, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreatePairingTokenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreatePairingTokenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.token = strims_auth_v1_PairingToken.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

/* @internal */
export const strims_replication_v1_Version = Version;
/* @internal */
export type strims_replication_v1_Version = Version;
/* @internal */
export type strims_replication_v1_IVersion = IVersion;
/* @internal */
export const strims_replication_v1_Event = Event;
/* @internal */
export type strims_replication_v1_Event = Event;
/* @internal */
export type strims_replication_v1_IEvent = IEvent;
/* @internal */
export const strims_replication_v1_EventBundle = EventBundle;
/* @internal */
export type strims_replication_v1_EventBundle = EventBundle;
/* @internal */
export type strims_replication_v1_IEventBundle = IEventBundle;
/* @internal */
export const strims_replication_v1_EventLog = EventLog;
/* @internal */
export type strims_replication_v1_EventLog = EventLog;
/* @internal */
export type strims_replication_v1_IEventLog = IEventLog;
/* @internal */
export const strims_replication_v1_CreatePairingTokenRequest = CreatePairingTokenRequest;
/* @internal */
export type strims_replication_v1_CreatePairingTokenRequest = CreatePairingTokenRequest;
/* @internal */
export type strims_replication_v1_ICreatePairingTokenRequest = ICreatePairingTokenRequest;
/* @internal */
export const strims_replication_v1_CreatePairingTokenResponse = CreatePairingTokenResponse;
/* @internal */
export type strims_replication_v1_CreatePairingTokenResponse = CreatePairingTokenResponse;
/* @internal */
export type strims_replication_v1_ICreatePairingTokenResponse = ICreatePairingTokenResponse;
