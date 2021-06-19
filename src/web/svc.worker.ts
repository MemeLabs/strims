import { WorkerBridge } from "../lib/bridge";
import svc from "./svc.go";

onmessage = async ({ data: { service, baseURI, args = [] } }) => {
  const bridge = new WorkerBridge();
  const proxy = await svc(baseURI, bridge.wasmio());
  void proxy.init(service, bridge, ...args);
};

// let ts know what to expect from worker-loader.
export default null as new () => Worker;
