import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Network as strims_network_v1_Network,
  INetwork as strims_network_v1_INetwork,
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

