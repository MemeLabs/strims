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
   * served while a profile runs. `lines`, the testbed's bare node's port to
   * its lines worker. Throws when it can't draw a seed.
   */
  export function start(
    options: unknown,
    frontend: MessagePort,
    signal: MessagePort,
    psi?: MessagePort,
    feeds?: MessagePort,
    lines?: MessagePort
  ): void;

  /** Starts the PSI worker, serving the node's engine on `port`. */
  export function start_psi(options: unknown, port: MessagePort): void;

  /**
   * Starts the testbed's lines worker: it writes each line whose parts the
   * node posts on `port`, and posts the line on `out`. Throws in a build
   * without the e2e feature.
   */
  export function start_lines(options: unknown, port: MessagePort, out: MessagePort): void;
}
