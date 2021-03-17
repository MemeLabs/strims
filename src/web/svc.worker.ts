import { WorkerBridge } from "../lib/bridge";
import svc from "./svc.go";

onmessage = async ({ data: { service, baseURI, args = [] } }) => {
  const proxy = await svc(baseURI);
  void proxy.init(service, new WorkerBridge(), ...args);
};

// let ts know what to expect from worker-loader.
export default null as new () => Worker;
