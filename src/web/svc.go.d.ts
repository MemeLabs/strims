// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { WorkerBridge, WorkerOptions } from "../lib/bridge";

export const wasmPath: string;

declare const init: (
  baseURI: string,
  wasmio: unknown
) => Promise<{
  init(service: "default" | "broker", api: WorkerBridge, options: WorkerOptions): Promise<any>;
}>;

export default init;
