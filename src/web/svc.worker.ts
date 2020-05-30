import { WorkerBridge } from "../lib/bridge";
import p2p from "./svc.go";

onmessage = ({ data: { service, args = [] } }) => {
  p2p.init(service, new WorkerBridge(), ...args);
};
