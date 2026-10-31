// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser worker driver: a strims node in a dedicated worker.
//!
//! The crate builds for `wasm32-unknown-unknown` and is empty on every
//! other target. `wasm-bindgen --target web` turns it into an ES module
//! that a module worker loads; the window starts the worker and hands it
//! the frontend RPC port, the signalling port, the port to the PSI
//! worker and the port players' feeds open on ([`entry::start`]). The PSI worker loads the same module and
//! runs [`psi::start_psi`].
//!
//! # The worker contract
//!
//! - Everything runs on the worker's one thread. Tasks are local
//!   ([`runtime::spawn`]), and wasm-bindgen-futures polls a woken task in
//!   a microtask.
//! - IndexedDB deactivates a transaction once the event that completed
//!   its last request has been dispatched. Microtasks queued by the event's
//!   handler run before that, so a task woken from a request's callback
//!   issues its next request while the transaction is still active. A
//!   transaction therefore survives any number of awaited operations of
//!   its own, and no await on anything else ([`kv`]).
//! - A task that wakes itself in a microtask runs again before any JS
//!   event: timers, socket and port messages and IndexedDB callbacks wait
//!   until it suspends on something else. A task that must let events in
//!   without suspending yields with [`clock::yield_now`], a
//!   `MessageChannel` message, which is a macrotask: the events queued
//!   before it run first.
//! - [`clock::now`] is the node's only clock read. The `e2e` testbed reads
//!   `Date.now()` for its own measurements.
//!
//! # Modules
//!
//! - [`runtime`]: local tasks that can be aborted.
//! - [`clock`]: the clock, `setTimeout` sleeps and the macrotask yield;
//!   [`clock::WasmClock`] is the services' `strims_rpc::Timer`.
//! - [`entropy`]: seeds from `crypto.getRandomValues`.
//! - [`log`]: `tracing` output on the console, and the panic hook.
//! - [`kv`]: [`kv::IdbKv`], the `strims_kv::Kv` on IndexedDB.
//! - [`node`]: starting a core node in the worker.
//! - [`io`]: [`io::WasmIo`], the node's driver: dials, links and timers.
//! - [`ws`]: WebSocket links.
//! - [`rtc`]: the signal port to the window's peer connections, and the
//!   data channels it hands over.
//! - `inbox`: a link's read queue, filled by its socket's callbacks.
//! - [`fetch`]: [`fetch::BrowserFetch`], the directory's HTTPS requests
//!   with the browser's `fetch`.
//! - [`factory`]: the app layer on the browser driver: the platform and
//!   session factory a profile runs on.
//! - [`frontend`]: the UI's frontend RPC connection over a message port.
//! - [`feed`]: players' feeds, a stream's records straight from the
//!   swarm to a player worker over a port of its own.
//! - [`options`]: the options the window starts the worker with.
//! - [`psi`]: the PSI worker ([`psi::start_psi`]), and the node's engine
//!   on it.
//! - [`memory`]: the worker's counting allocator.
//! - [`entry`]: `start`, the function the worker script calls.
//! - `e2e` (feature `e2e`): the bare node the browser link tests and the
//!   testbed's browser nodes drive.

#![cfg(target_arch = "wasm32")]

pub mod clock;
#[cfg(feature = "e2e")]
pub mod e2e;
pub mod entropy;
pub mod entry;
pub mod factory;
pub mod feed;
pub mod fetch;
pub mod frontend;
mod idb;
mod inbox;
pub mod io;
mod js;
pub mod kv;
pub mod log;
pub mod memory;
pub mod node;
pub mod options;
pub mod psi;
pub mod rtc;
pub mod runtime;
pub mod ws;

pub use kv::IdbKv;

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
