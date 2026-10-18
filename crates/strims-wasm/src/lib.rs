// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser worker driver: a strims node in a dedicated worker.
//!
//! The crate builds for `wasm32-unknown-unknown` and is empty on every
//! other target. `wasm-bindgen --target web` turns it into an ES module
//! that a module worker loads; the window starts the worker and hands it
//! the frontend RPC port and the signalling port ([`entry::start`]).
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
//! - [`clock::now`] is the crate's only clock read.
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
//! - [`entry`]: `start`, the function the worker script calls.
//! - `e2e` (feature `e2e`): the bare node the browser link tests drive.

#![cfg(target_arch = "wasm32")]

pub mod clock;
#[cfg(feature = "e2e")]
pub mod e2e;
pub mod entropy;
pub mod entry;
mod idb;
mod inbox;
pub mod io;
mod js;
pub mod kv;
pub mod log;
pub mod node;
pub mod rtc;
pub mod runtime;
pub mod ws;

pub use kv::IdbKv;

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
