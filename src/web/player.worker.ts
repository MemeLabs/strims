// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import init, { Remuxer } from "../../crates/strims-player/Cargo.toml";
import { runPlayerWorker } from "../lib/media/worker";

declare let self: DedicatedWorkerGlobalScope;

// A unit-format player's worker: strims-player's Remuxer, one per stream.
runPlayerWorker(self, () => init().then(() => () => new Remuxer()));
