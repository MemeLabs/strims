// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { WorkerBridge } from "../lib/bridge";
import svc from "./svc.go";

interface Message {
  service: "default" | "broker";
  baseURI: string;
  args: unknown[];
}

onmessage = async ({ data: { service, baseURI, args = [] } }: MessageEvent<Message>) => {
  const bridge = new WorkerBridge();
  const proxy = await svc(baseURI, bridge.wasmio());
  void proxy.init(service, bridge, ...args);
};

// let ts know what to expect from worker-loader.
export default null as new () => Worker;
