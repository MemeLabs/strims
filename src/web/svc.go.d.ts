import { WorkerAPI } from "../apis/client";

export default class P2P {
  static init(service: "default" | "broker", api: WorkerAPI, ...any): Promise<any>;
}
