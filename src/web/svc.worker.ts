import { WorkerBridge } from "../lib/bridge";
import svc from "./svc.go";

onmessage = async ({ data: { service, baseURI, args = [] } }) => {
  const proxy = await svc(baseURI);
  proxy.init(service, new WorkerBridge(), ...args);
};

export default null as new () => Worker;
