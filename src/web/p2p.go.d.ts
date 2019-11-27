import Bus from "./wasmio_bus";
import * as wrtc from "./wrtc";

export default class P2P {
  static init(wrtcBridge: wrtc.Bridge, bus: Bus): Promise<any>;
}
