// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import init, { Demuxer } from "../../crates/strims-player/Cargo.toml";
import { runLowLatencyWorker } from "../lib/media/lowlatency/worker";

declare let self: DedicatedWorkerGlobalScope;

// The low-latency player's worker: strims-player's Demuxer and WebCodecs, one stream each.
runLowLatencyWorker(self, () => init().then(() => () => new Demuxer()));
