import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_profile_v1_Device,
  strims_profile_v1_IDevice,
} from "./profile";

export type IDeviceChangeEvent = {
  device?: strims_profile_v1_IDevice;
}

export class DeviceChangeEvent {
  device: strims_profile_v1_Device | undefined;

  constructor(v?: IDeviceChangeEvent) {
    this.device = v?.device && new strims_profile_v1_Device(v.device);
  }

  static encode(m: DeviceChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.device) strims_profile_v1_Device.encode(m.device, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeviceChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeviceChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.device = strims_profile_v1_Device.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeviceDeleteEvent = {
  device?: strims_profile_v1_IDevice;
}

export class DeviceDeleteEvent {
  device: strims_profile_v1_Device | undefined;

  constructor(v?: IDeviceDeleteEvent) {
    this.device = v?.device && new strims_profile_v1_Device(v.device);
  }

  static encode(m: DeviceDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.device) strims_profile_v1_Device.encode(m.device, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeviceDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeviceDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.device = strims_profile_v1_Device.decode(r, r.uint32());
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
export const strims_profile_v1_DeviceChangeEvent = DeviceChangeEvent;
/* @internal */
export type strims_profile_v1_DeviceChangeEvent = DeviceChangeEvent;
/* @internal */
export type strims_profile_v1_IDeviceChangeEvent = IDeviceChangeEvent;
/* @internal */
export const strims_profile_v1_DeviceDeleteEvent = DeviceDeleteEvent;
/* @internal */
export type strims_profile_v1_DeviceDeleteEvent = DeviceDeleteEvent;
/* @internal */
export type strims_profile_v1_IDeviceDeleteEvent = IDeviceDeleteEvent;
