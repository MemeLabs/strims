import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_replication_v1_Checkpoint,
  strims_replication_v1_ICheckpoint,
} from "./replication";

export type ICheckpointChangeEvent = {
  checkpoint?: strims_replication_v1_ICheckpoint;
}

export class CheckpointChangeEvent {
  checkpoint: strims_replication_v1_Checkpoint | undefined;

  constructor(v?: ICheckpointChangeEvent) {
    this.checkpoint = v?.checkpoint && new strims_replication_v1_Checkpoint(v.checkpoint);
  }

  static encode(m: CheckpointChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.checkpoint) strims_replication_v1_Checkpoint.encode(m.checkpoint, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CheckpointChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CheckpointChangeEvent();
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

export type ICheckpointDeleteEvent = {
  checkpoint?: strims_replication_v1_ICheckpoint;
}

export class CheckpointDeleteEvent {
  checkpoint: strims_replication_v1_Checkpoint | undefined;

  constructor(v?: ICheckpointDeleteEvent) {
    this.checkpoint = v?.checkpoint && new strims_replication_v1_Checkpoint(v.checkpoint);
  }

  static encode(m: CheckpointDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.checkpoint) strims_replication_v1_Checkpoint.encode(m.checkpoint, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CheckpointDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CheckpointDeleteEvent();
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

/* @internal */
export const strims_replication_v1_CheckpointChangeEvent = CheckpointChangeEvent;
/* @internal */
export type strims_replication_v1_CheckpointChangeEvent = CheckpointChangeEvent;
/* @internal */
export type strims_replication_v1_ICheckpointChangeEvent = ICheckpointChangeEvent;
/* @internal */
export const strims_replication_v1_CheckpointDeleteEvent = CheckpointDeleteEvent;
/* @internal */
export type strims_replication_v1_CheckpointDeleteEvent = CheckpointDeleteEvent;
/* @internal */
export type strims_replication_v1_ICheckpointDeleteEvent = ICheckpointDeleteEvent;
