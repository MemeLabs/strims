import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_replication_v1_Checkpoint,
  strims_replication_v1_ICheckpoint,
  strims_replication_v1_Event,
  strims_replication_v1_IEvent,
  strims_replication_v1_EventLog,
  strims_replication_v1_IEventLog,
} from "./replication";
import {
  strims_profile_v1_ProfileID,
  strims_profile_v1_IProfileID,
} from "../../profile/v1/profile";

export type IPeerOpenRequest = Record<string, any>;

export class PeerOpenRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IPeerOpenRequest) {
  }

  static encode(m: PeerOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerOpenRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new PeerOpenRequest();
  }
}

export type IPeerOpenResponse = {
  storeVersion?: number;
  replicaId?: bigint;
  checkpoint?: strims_replication_v1_ICheckpoint;
}

export class PeerOpenResponse {
  storeVersion: number;
  replicaId: bigint;
  checkpoint: strims_replication_v1_Checkpoint | undefined;

  constructor(v?: IPeerOpenResponse) {
    this.storeVersion = v?.storeVersion || 0;
    this.replicaId = v?.replicaId || BigInt(0);
    this.checkpoint = v?.checkpoint && new strims_replication_v1_Checkpoint(v.checkpoint);
  }

  static encode(m: PeerOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.storeVersion) w.uint32(8).uint32(m.storeVersion);
    if (m.replicaId) w.uint32(16).uint64(m.replicaId);
    if (m.checkpoint) strims_replication_v1_Checkpoint.encode(m.checkpoint, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerOpenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerOpenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.storeVersion = r.uint32();
        break;
        case 2:
        m.replicaId = r.uint64();
        break;
        case 3:
        m.checkpoint = strims_replication_v1_Checkpoint.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerBootstrapRequest = {
  events?: strims_replication_v1_IEvent[];
  logs?: strims_replication_v1_IEventLog[];
}

export class PeerBootstrapRequest {
  events: strims_replication_v1_Event[];
  logs: strims_replication_v1_EventLog[];

  constructor(v?: IPeerBootstrapRequest) {
    this.events = v?.events ? v.events.map(v => new strims_replication_v1_Event(v)) : [];
    this.logs = v?.logs ? v.logs.map(v => new strims_replication_v1_EventLog(v)) : [];
  }

  static encode(m: PeerBootstrapRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) strims_replication_v1_Event.encode(v, w.uint32(10).fork()).ldelim();
    for (const v of m.logs) strims_replication_v1_EventLog.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerBootstrapRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerBootstrapRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(strims_replication_v1_Event.decode(r, r.uint32()));
        break;
        case 2:
        m.logs.push(strims_replication_v1_EventLog.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerBootstrapResponse = {
  checkpoint?: strims_replication_v1_ICheckpoint;
}

export class PeerBootstrapResponse {
  checkpoint: strims_replication_v1_Checkpoint | undefined;

  constructor(v?: IPeerBootstrapResponse) {
    this.checkpoint = v?.checkpoint && new strims_replication_v1_Checkpoint(v.checkpoint);
  }

  static encode(m: PeerBootstrapResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.checkpoint) strims_replication_v1_Checkpoint.encode(m.checkpoint, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerBootstrapResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerBootstrapResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.checkpoint = strims_replication_v1_Checkpoint.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerSyncRequest = {
  logs?: strims_replication_v1_IEventLog[];
}

export class PeerSyncRequest {
  logs: strims_replication_v1_EventLog[];

  constructor(v?: IPeerSyncRequest) {
    this.logs = v?.logs ? v.logs.map(v => new strims_replication_v1_EventLog(v)) : [];
  }

  static encode(m: PeerSyncRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.logs) strims_replication_v1_EventLog.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerSyncRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerSyncRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.logs.push(strims_replication_v1_EventLog.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerSyncResponse = {
  checkpoint?: strims_replication_v1_ICheckpoint;
}

export class PeerSyncResponse {
  checkpoint: strims_replication_v1_Checkpoint | undefined;

  constructor(v?: IPeerSyncResponse) {
    this.checkpoint = v?.checkpoint && new strims_replication_v1_Checkpoint(v.checkpoint);
  }

  static encode(m: PeerSyncResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.checkpoint) strims_replication_v1_Checkpoint.encode(m.checkpoint, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerSyncResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerSyncResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.checkpoint = strims_replication_v1_Checkpoint.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerAllocateProfileIDsRequest = Record<string, any>;

export class PeerAllocateProfileIDsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IPeerAllocateProfileIDsRequest) {
  }

  static encode(m: PeerAllocateProfileIDsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerAllocateProfileIDsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new PeerAllocateProfileIDsRequest();
  }
}

export type IPeerAllocateProfileIDsResponse = {
  profileId?: strims_profile_v1_IProfileID;
}

export class PeerAllocateProfileIDsResponse {
  profileId: strims_profile_v1_ProfileID | undefined;

  constructor(v?: IPeerAllocateProfileIDsResponse) {
    this.profileId = v?.profileId && new strims_profile_v1_ProfileID(v.profileId);
  }

  static encode(m: PeerAllocateProfileIDsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) strims_profile_v1_ProfileID.encode(m.profileId, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerAllocateProfileIDsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerAllocateProfileIDsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = strims_profile_v1_ProfileID.decode(r, r.uint32());
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
export const strims_replication_v1_PeerOpenRequest = PeerOpenRequest;
/* @internal */
export type strims_replication_v1_PeerOpenRequest = PeerOpenRequest;
/* @internal */
export type strims_replication_v1_IPeerOpenRequest = IPeerOpenRequest;
/* @internal */
export const strims_replication_v1_PeerOpenResponse = PeerOpenResponse;
/* @internal */
export type strims_replication_v1_PeerOpenResponse = PeerOpenResponse;
/* @internal */
export type strims_replication_v1_IPeerOpenResponse = IPeerOpenResponse;
/* @internal */
export const strims_replication_v1_PeerBootstrapRequest = PeerBootstrapRequest;
/* @internal */
export type strims_replication_v1_PeerBootstrapRequest = PeerBootstrapRequest;
/* @internal */
export type strims_replication_v1_IPeerBootstrapRequest = IPeerBootstrapRequest;
/* @internal */
export const strims_replication_v1_PeerBootstrapResponse = PeerBootstrapResponse;
/* @internal */
export type strims_replication_v1_PeerBootstrapResponse = PeerBootstrapResponse;
/* @internal */
export type strims_replication_v1_IPeerBootstrapResponse = IPeerBootstrapResponse;
/* @internal */
export const strims_replication_v1_PeerSyncRequest = PeerSyncRequest;
/* @internal */
export type strims_replication_v1_PeerSyncRequest = PeerSyncRequest;
/* @internal */
export type strims_replication_v1_IPeerSyncRequest = IPeerSyncRequest;
/* @internal */
export const strims_replication_v1_PeerSyncResponse = PeerSyncResponse;
/* @internal */
export type strims_replication_v1_PeerSyncResponse = PeerSyncResponse;
/* @internal */
export type strims_replication_v1_IPeerSyncResponse = IPeerSyncResponse;
/* @internal */
export const strims_replication_v1_PeerAllocateProfileIDsRequest = PeerAllocateProfileIDsRequest;
/* @internal */
export type strims_replication_v1_PeerAllocateProfileIDsRequest = PeerAllocateProfileIDsRequest;
/* @internal */
export type strims_replication_v1_IPeerAllocateProfileIDsRequest = IPeerAllocateProfileIDsRequest;
/* @internal */
export const strims_replication_v1_PeerAllocateProfileIDsResponse = PeerAllocateProfileIDsResponse;
/* @internal */
export type strims_replication_v1_PeerAllocateProfileIDsResponse = PeerAllocateProfileIDsResponse;
/* @internal */
export type strims_replication_v1_IPeerAllocateProfileIDsResponse = IPeerAllocateProfileIDsResponse;
