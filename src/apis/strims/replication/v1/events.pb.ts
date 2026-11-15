// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Checkpoint as strims_replication_v1_Checkpoint,
  type ICheckpoint as strims_replication_v1_ICheckpoint,
} from "./replication.pb";

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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CheckpointChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.checkpoint = strims_replication_v1_Checkpoint.decode(r, r.uint32());
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

