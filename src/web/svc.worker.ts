// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import init, { start, start_psi } from "../../crates/strims-wasm/Cargo.toml";
import type { InitMessage, PsiWorkerMessage, WorkerSignal } from "./wasm";

declare let self: DedicatedWorkerGlobalScope;

// The worker runs a node, or the PSI server for one: the init message's role
// says which. The ports buffer their messages until Rust attaches its
// handlers.
self.onmessage = ({ data }: MessageEvent<InitMessage>) => {
  self.onmessage = null;
  if (data.role === "psi") {
    const { options, port } = data;
    init()
      .then(() => start_psi(options, port))
      .catch((e) =>
        self.postMessage({ type: "fatal", message: String(e) } satisfies PsiWorkerMessage)
      );
    return;
  }
  const { options, frontend, signal, psi } = data;
  init()
    .then(() => start(options, frontend, signal, psi))
    .catch((e) => signal.postMessage({ type: "fatal", message: String(e) } satisfies WorkerSignal));
};
