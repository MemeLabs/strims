// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The wasm-bindgen glue for crates/strims-wasm, which
// vendor_modules/strims-wasm-loader builds when the worker imports the
// crate's Cargo.toml.
declare module "*/strims-wasm/Cargo.toml" {
  /** Fetches and instantiates the module. */
  export default function init(): Promise<unknown>;

  /**
   * Starts the node with the window's options and the two ports it
   * transferred. Throws when the options are invalid.
   */
  export function start(options: unknown, frontend: MessagePort, signal: MessagePort): void;
}
