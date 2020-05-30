import { WorkerAPI } from "../lib/api";

export default class P2P {
  static init(service: "default" | "broker", api: WorkerAPI, ...any): Promise<any>;
}
