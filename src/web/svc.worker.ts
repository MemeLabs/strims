// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import init, { start } from "../../crates/strims-wasm/Cargo.toml";
import type { InitMessage, WorkerSignal } from "./wasm";

declare let self: DedicatedWorkerGlobalScope;

// The ports buffer their messages until the node attaches its handlers.
self.onmessage = ({ data: { options, frontend, signal } }: MessageEvent<InitMessage>) => {
  self.onmessage = null;
  init()
    .then(() => start(options, frontend, signal))
    .catch((e) => signal.postMessage({ type: "fatal", message: String(e) } satisfies WorkerSignal));
};
