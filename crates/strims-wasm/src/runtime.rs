// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Local tasks on the worker's executor.
//!
//! The browser's futures aren't `Send`, and the worker has one thread, so
//! every task is local. [`spawn`] starts one with
//! `wasm_bindgen_futures::spawn_local`; the returned [`TaskHandle`] cancels
//! it.

use std::future::Future;

use futures_util::future::{AbortHandle, abortable};

/// A spawned task. [`TaskHandle::abort`] cancels it; dropping the handle
/// detaches the task, which then runs to completion.
#[derive(Debug)]
pub struct TaskHandle(AbortHandle);

impl TaskHandle {
    /// Cancels the task. Its future is dropped the next time the executor
    /// polls it, without running further.
    pub fn abort(&self) {
        self.0.abort();
    }

    /// Whether [`TaskHandle::abort`] has been called.
    pub fn is_aborted(&self) -> bool {
        self.0.is_aborted()
    }
}

/// Runs `f` as a local task. It first runs in a microtask after the
/// current one.
pub fn spawn<F>(f: F) -> TaskHandle
where
    F: Future<Output = ()> + 'static,
{
    let (f, handle) = abortable(f);
    wasm_bindgen_futures::spawn_local(async move {
        // `Err(Aborted)` only says the task was cancelled.
        let _ = f.await;
    });
    TaskHandle(handle)
}
