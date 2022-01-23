import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  BootstrapClient as strims_network_v1_bootstrap_BootstrapClient,
  IBootstrapClient as strims_network_v1_bootstrap_IBootstrapClient,
} from "./bootstrap";

export type IBootstrapClientChange = {
  bootstrapClient?: strims_network_v1_bootstrap_IBootstrapClient;
}

export class BootstrapClientChange {
  bootstrapClient: strims_network_v1_bootstrap_BootstrapClient | undefined;

  constructor(v?: IBootstrapClientChange) {
    this.bootstrapClient = v?.bootstrapClient && new strims_network_v1_bootstrap_BootstrapClient(v.bootstrapClient);
  }

  static encode(m: BootstrapClientChange, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) strims_network_v1_bootstrap_BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapClientChange {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapClientChange();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = strims_network_v1_bootstrap_BootstrapClient.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

