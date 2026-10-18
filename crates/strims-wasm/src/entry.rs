// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The worker's entry point.
//!
//! The worker script runs `await init(); start(options, frontend, signal)`
//! with the two ports the window transferred to it. Ports buffer their
//! messages until a handler is attached, so nothing sent before `start` is
//! lost.

use std::cell::RefCell;

use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use web_sys::MessagePort;

thread_local! {
    /// The ports `start` was given, held so they stay open.
    static PORTS: RefCell<Option<(MessagePort, MessagePort)>> = const { RefCell::new(None) };
}

/// Starts the worker. `options.logLevel` is a zap level (see
/// [`crate::log::init`]); `frontend` carries the UI's RPC connection and
/// `signal` the WebRTC signalling with the window. With the `e2e`
/// feature, `options.e2e: true` hands both ports to the browser link
/// tests' bare node (see `crate::e2e`).
#[wasm_bindgen]
pub fn start(options: JsValue, frontend: MessagePort, signal: MessagePort) -> Result<(), JsValue> {
    let log_level = Reflect::get(&options, &JsValue::from_str("logLevel"))
        .ok()
        .and_then(|v| v.as_f64())
        .map_or(0, |l| l as i32);
    crate::log::init(log_level);
    #[cfg(feature = "e2e")]
    if crate::e2e::requested(&options) {
        crate::e2e::start(frontend, signal);
        return Ok(());
    }
    tracing::info!("strims-wasm worker started");
    let old = PORTS.with(|p| p.replace(Some((frontend, signal))));
    drop(old);
    Ok(())
}
