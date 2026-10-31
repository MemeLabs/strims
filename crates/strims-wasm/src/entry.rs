// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The worker's entry point.
//!
//! The worker script runs `await init(); start(options, frontend, signal,
//! psi, feeds)` with the ports the window transferred to it. Ports buffer their
//! messages until a handler is attached, so nothing sent before `start` is
//! lost. A `start` that throws is reported to the window as `fatal` by the
//! worker script.
//!
//! `start` opens the profiles in IndexedDB ([`IdbKv`], with no table
//! prefix) through a [`Manager`] on a [`WasmFactory`], and serves the UI's
//! frontend RPC connection on `frontend` ([`frontend::serve`]). A panic
//! posts `{type: "fatal", message}` on `signal`.
//!
//! The PSI worker's script runs `await init(); start_psi(options, port)`
//! instead ([`crate::psi::start_psi`]).

use std::rc::Rc;

use js_sys::{Object, Reflect};
use rand_core::SeedableRng;
use strims_base::rng::DetRng;
use strims_session::Manager;
use wasm_bindgen::prelude::*;
use web_sys::MessagePort;

use crate::clock::WasmClock;
use crate::factory::WasmFactory;
use crate::kv::IdbKv;
use crate::options::Options;
use crate::{frontend, runtime};

/// The operating system a browser's device record reports: the UI's
/// device list shows it.
pub const DEVICE_OS: &str = "web";

/// The worker's profiles and their sessions.
pub type WasmManager = Manager<IdbKv, WasmFactory, WasmClock>;

/// Starts the worker. `options` are the window's (see
/// [`crate::options`]); `frontend` carries the UI's RPC connection and
/// `signal` the WebRTC signalling with the window. `psi` is the port to
/// the PSI worker, where the profiles' PSI sessions compute
/// ([`crate::psi`]); without one (`undefined` or `null`) they compute in
/// this worker. `feeds` is the port players' feeds open on
/// ([`crate::feed`]), served while a profile runs; without one, players
/// read streams over the frontend RPC. With the `e2e` feature,
/// `options.e2e: true` hands `frontend`, `signal` and `feeds` to the
/// browser link tests' and the testbed's bare node instead.
#[wasm_bindgen]
pub fn start(
    options: JsValue,
    frontend: MessagePort,
    signal: MessagePort,
    psi: Option<MessagePort>,
    feeds: Option<MessagePort>,
) -> Result<(), JsValue> {
    let (parsed, ignored) = Options::parse(&options);
    crate::log::init(parsed.log_level);
    // The `e2e` feature's bare node takes its ports here, before anything
    // opens.
    #[cfg(feature = "e2e")]
    if crate::e2e::requested(&options) {
        crate::e2e::start(frontend, signal, feeds);
        return Ok(());
    }
    for line in &ignored {
        tracing::warn!("ignoring an option: {line}");
    }
    set_fatal_hook(&signal);
    let seed = crate::entropy::seed()
        .map_err(|e| js_sys::Error::new(&format!("drawing a seed from entropy: {e}")))?;
    let manager = Rc::new(WasmManager::new(
        IdbKv::new(""),
        WasmFactory::new(&parsed, signal, psi, feeds),
        WasmClock,
        DEVICE_OS,
        DetRng::from_seed(seed),
    ));
    // Detached: the connection lives as long as the window's port.
    drop(runtime::spawn(async move {
        let res = frontend::serve(manager, frontend).await;
        tracing::info!(?res, "frontend disconnected");
    }));
    tracing::info!("strims-wasm worker started");
    Ok(())
}

/// Makes a panic post `{type: "fatal", message}` on `signal`.
fn set_fatal_hook(signal: &MessagePort) {
    let signal = signal.clone();
    crate::log::set_fatal_hook(move |message| {
        let msg = Object::new();
        let _ = Reflect::set(&msg, &"type".into(), &"fatal".into());
        let _ = Reflect::set(&msg, &"message".into(), &message.into());
        let _ = signal.post_message(&msg);
    });
}
