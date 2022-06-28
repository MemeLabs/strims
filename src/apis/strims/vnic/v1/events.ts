import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_vnic_v1_Config,
  strims_vnic_v1_IConfig,
} from "./vnic";

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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ConfigChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_vnic_v1_Config.decode(r, r.uint32());
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
export const strims_vnic_v1_ConfigChangeEvent = ConfigChangeEvent;
/* @internal */
export type strims_vnic_v1_ConfigChangeEvent = ConfigChangeEvent;
/* @internal */
export type strims_vnic_v1_IConfigChangeEvent = IConfigChangeEvent;
