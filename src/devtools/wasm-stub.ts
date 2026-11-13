// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The wasm crates' glue (crates/strims-wasm's and crates/strims-player's) in a build without
 * them: webpack's `console` target (`pnpm run console:build`) puts this module in place of each
 * crate's Cargo.toml, so the devtools app builds without Rust. The console's page never calls
 * it; whatever does fails, saying why.
 */

const missing = (): never => {
  throw new Error("no wasm crates in this build (webpack's console target)");
};

const init = (): Promise<unknown> =>
  Promise.reject(new Error("no wasm crates in this build (webpack's console target)"));

export default init;

export const start = missing;

export const start_psi = missing;

export class Remuxer {
  constructor() {
    missing();
  }
}

export class Demuxer {
  constructor() {
    missing();
  }
}
