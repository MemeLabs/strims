// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The wasm-bindgen glue for crates/strims-wasm, which
// vendor_modules/strims-wasm-loader builds when the worker imports the
// crate's Cargo.toml.
declare module "*/strims-wasm/Cargo.toml" {
  /** Fetches and instantiates the module. */
  export default function init(): Promise<unknown>;

  /**
   * Starts the node with the window's options and the ports it transferred.
   * Without `psi`, the port to the PSI worker, the node computes PSI itself.
   * `feeds` is the port players' feeds open on (src/web/wasm.ts FeedHost),
   * served while a profile runs. Throws when it can't draw a seed.
   */
  export function start(
    options: unknown,
    frontend: MessagePort,
    signal: MessagePort,
    psi?: MessagePort,
    feeds?: MessagePort
  ): void;

  /** Starts the PSI worker, serving the node's engine on `port`. */
  export function start_psi(options: unknown, port: MessagePort): void;
}
