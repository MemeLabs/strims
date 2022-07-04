import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_network_v1_Network,
  strims_network_v1_INetwork,
  strims_network_v1_UIConfig,
  strims_network_v1_IUIConfig,
} from "./network";

export type INetworkChangeEvent = {
  network?: strims_network_v1_INetwork;
}

export class NetworkChangeEvent {
  network: strims_network_v1_Network | undefined;

  constructor(v?: INetworkChangeEvent) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: NetworkChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = strims_network_v1_Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkDeleteEvent = {
  network?: strims_network_v1_INetwork;
}

export class NetworkDeleteEvent {
  network: strims_network_v1_Network | undefined;

  constructor(v?: INetworkDeleteEvent) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: NetworkDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = strims_network_v1_Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUIConfigChangeEvent = {
  uiConfig?: strims_network_v1_IUIConfig;
}

export class UIConfigChangeEvent {
  uiConfig: strims_network_v1_UIConfig | undefined;

  constructor(v?: IUIConfigChangeEvent) {
    this.uiConfig = v?.uiConfig && new strims_network_v1_UIConfig(v.uiConfig);
  }

  static encode(m: UIConfigChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfig) strims_network_v1_UIConfig.encode(m.uiConfig, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfigChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfig = strims_network_v1_UIConfig.decode(r, r.uint32());
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
export const strims_network_v1_NetworkChangeEvent = NetworkChangeEvent;
/* @internal */
export type strims_network_v1_NetworkChangeEvent = NetworkChangeEvent;
/* @internal */
export type strims_network_v1_INetworkChangeEvent = INetworkChangeEvent;
/* @internal */
export const strims_network_v1_NetworkDeleteEvent = NetworkDeleteEvent;
/* @internal */
export type strims_network_v1_NetworkDeleteEvent = NetworkDeleteEvent;
/* @internal */
export type strims_network_v1_INetworkDeleteEvent = INetworkDeleteEvent;
/* @internal */
export const strims_network_v1_UIConfigChangeEvent = UIConfigChangeEvent;
/* @internal */
export type strims_network_v1_UIConfigChangeEvent = UIConfigChangeEvent;
/* @internal */
export type strims_network_v1_IUIConfigChangeEvent = IUIConfigChangeEvent;
