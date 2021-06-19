import { WorkerBridge } from "../lib/bridge";

declare const init: (
  baseURI: string,
  wasmio: unknown
) => Promise<{
  init(service: "default" | "broker", api: WorkerBridge, ...args: any[]): Promise<any>;
}>;

export default init;
