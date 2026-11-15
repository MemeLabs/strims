// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  PairingToken as strims_auth_v1_PairingToken,
  type IPairingToken as strims_auth_v1_IPairingToken,
} from "../../auth/v1/auth.pb";
import {
  VersionVector as strims_dao_v1_VersionVector,
  type IVersionVector as strims_dao_v1_IVersionVector,
} from "../../dao/v1/dao.pb";

export type ICheckpoint = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  deleted?: boolean;
}

export class Checkpoint {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  deleted: boolean;

  constructor(v?: ICheckpoint) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.deleted = v?.deleted || false;
  }

  static encode(m: Checkpoint, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(18).fork()).ldelim();
    if (m.deleted) w.uint32(24).bool(m.deleted);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Checkpoint {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Checkpoint();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
        case 3:
        m.deleted = r.bool();
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IEventLog = {
  id?: bigint;
  checkpoint?: ICheckpoint;
  events?: IEvent[];
}

export class EventLog {
  id: bigint;
  checkpoint: Checkpoint | undefined;
  events: Event[];

  constructor(v?: IEventLog) {
    this.id = v?.id || BigInt(0);
    this.checkpoint = v?.checkpoint && new Checkpoint(v.checkpoint);
    this.events = v?.events ? v.events.map(v => new Event(v)) : [];
  }

  static encode(m: EventLog, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.checkpoint) Checkpoint.encode(m.checkpoint, w.uint32(18).fork()).ldelim();
    for (const v of m.events) Event.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EventLog {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EventLog();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.checkpoint = Checkpoint.decode(r, r.uint32());
        break;
        case 3:
        m.events.push(Event.decode(r, r.uint32()));
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreatePairingTokenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.token = strims_auth_v1_PairingToken.decode(r, r.uint32());
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

export type IListCheckpointsRequest = object;

export class ListCheckpointsRequest {

  constructor(v?: IListCheckpointsRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ListCheckpointsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListCheckpointsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListCheckpointsRequest();
  }
}

export type IListCheckpointsResponse = {
  checkpoints?: ICheckpoint[];
}

export class ListCheckpointsResponse {
  checkpoints: Checkpoint[];

  constructor(v?: IListCheckpointsResponse) {
    this.checkpoints = v?.checkpoints ? v.checkpoints.map(v => new Checkpoint(v)) : [];
  }

  static encode(m: ListCheckpointsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.checkpoints) Checkpoint.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListCheckpointsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListCheckpointsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.checkpoints.push(Checkpoint.decode(r, r.uint32()));
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

