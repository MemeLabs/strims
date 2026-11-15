// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Config as strims_vnic_v1_Config,
  type IConfig as strims_vnic_v1_IConfig,
} from "./vnic.pb";

export type IConfigChangeEvent = {
  config?: strims_vnic_v1_IConfig;
}

export class ConfigChangeEvent {
  config: strims_vnic_v1_Config | undefined;

  constructor(v?: IConfigChangeEvent) {
    this.config = v?.config && new strims_vnic_v1_Config(v.config);
  }

  static encode(m: ConfigChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_vnic_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ConfigChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ConfigChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_vnic_v1_Config.decode(r, r.uint32());
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

