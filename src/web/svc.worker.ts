// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { WorkerBridge, WorkerOptions } from "../lib/bridge";
import svc from "./svc.go";

interface Message {
  service: "default" | "broker";
  baseURI: string;
  options: WorkerOptions;
}

onmessage = async ({ data: { service, baseURI, options } }: MessageEvent<Message>) => {
  const bridge = new WorkerBridge();
  const proxy = await svc(baseURI, bridge.wasmio());
  void proxy.init(service, bridge, options);
};

// let ts know what to expect from worker-loader.
export default null as new () => Worker;
