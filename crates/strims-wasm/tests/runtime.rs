// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The worker runtime in a dedicated worker: sleeps, the clock, the
//! macrotask yield, entropy and tasks.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::Cell;
use std::rc::Rc;

use futures_util::FutureExt;
use strims_base::time::{Duration, Time};
use strims_rpc::Timer;
use strims_wasm::clock::{self, WasmClock};
use strims_wasm::{entropy, runtime};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_dedicated_worker);

#[wasm_bindgen_test]
fn tests_run_in_a_dedicated_worker() {
    assert!(
        js_sys::global()
            .dyn_ref::<web_sys::DedicatedWorkerGlobalScope>()
            .is_some()
    );
}

#[wasm_bindgen_test]
fn logging_starts_once() {
    strims_wasm::log::init(-1);
    strims_wasm::log::init(2);
    tracing::debug!("debug");
    tracing::info!(n = 1, "info");
    tracing::warn!("warn");
    tracing::error!("error");
}

async fn sleep(d: Duration) {
    WasmClock.sleep_until(clock::now() + d).await;
}

#[wasm_bindgen_test]
async fn sleeps_never_wake_early() {
    for ms in [0, 1, 2, 3, 5, 8, 13, 21, 50] {
        let deadline = clock::now() + Duration::from_millis(ms);
        WasmClock.sleep_until(deadline).await;
        let woke = clock::now();
        assert!(
            woke >= deadline,
            "{ms} ms: woke at {woke:?}, before {deadline:?}"
        );
    }

    // Moving the deadline of an armed sleep re-arms its timer, earlier or
    // later.
    let mut s = clock::new_sleep();
    let now = clock::now();
    clock::reset_sleep(std::pin::Pin::new(&mut s), now + Duration::from_secs(3600));
    assert!((&mut s).now_or_never().is_none());
    let deadline = now + Duration::from_millis(20);
    clock::reset_sleep(std::pin::Pin::new(&mut s), deadline);
    (&mut s).await;
    assert!(clock::now() >= deadline);
    let deadline = clock::now() + Duration::from_millis(10);
    clock::reset_sleep(std::pin::Pin::new(&mut s), deadline);
    s.await;
    assert!(clock::now() >= deadline);
}

#[wasm_bindgen_test]
async fn a_new_sleep_is_due() {
    clock::new_sleep().await;
}

#[wasm_bindgen_test]
async fn now_is_wall_time() {
    // The clock reads wall time at millisecond precision.
    clock::yield_now().await;
    let now = clock::now();
    assert_eq!(now, now.truncate(Time::PRECISION));
    let wall = Time::from_unix_millis(js_sys::Date::now() as i64);
    let skew = (wall - now).abs();
    assert!(
        skew < Duration::from_secs(1),
        "{now:?} is {skew} from {wall:?}"
    );
}

#[wasm_bindgen_test]
async fn yield_lets_macrotasks_run() {
    // A message posted on another channel before the yield is delivered
    // before the yield's own message.
    let channel = web_sys::MessageChannel::new().unwrap();
    let delivered = Rc::new(Cell::new(false));
    let on_message = {
        let delivered = delivered.clone();
        Closure::<dyn FnMut()>::new(move || delivered.set(true))
    };
    channel
        .port1()
        .set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    channel.port2().post_message(&JsValue::UNDEFINED).unwrap();
    assert!(!delivered.get());
    clock::yield_now().await;
    assert!(delivered.get());

    // Concurrent yields all resume, past one dropped while it waited.
    let a = clock::yield_now();
    let b = clock::yield_now();
    assert!(clock::yield_now().now_or_never().is_none());
    futures_util::future::join(a, b).await;
}

#[wasm_bindgen_test]
fn seeds_differ() {
    let a = entropy::seed().unwrap();
    let b = entropy::seed().unwrap();
    assert_ne!(a, b);
    assert_ne!(a, [0; 32]);
}

#[wasm_bindgen_test]
async fn spawned_tasks_run_until_aborted() {
    let ran = Rc::new(Cell::new(false));
    let task = {
        let ran = ran.clone();
        runtime::spawn(async move { ran.set(true) })
    };
    drop(task);
    clock::yield_now().await;
    assert!(ran.get(), "a detached task runs");

    let woke = Rc::new(Cell::new(false));
    let task = {
        let woke = woke.clone();
        runtime::spawn(async move {
            sleep(Duration::from_millis(20)).await;
            woke.set(true);
        })
    };
    clock::yield_now().await;
    task.abort();
    assert!(task.is_aborted());
    sleep(Duration::from_millis(60)).await;
    assert!(!woke.get(), "an aborted task stops");
}
