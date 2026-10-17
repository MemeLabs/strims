// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The tests' executor and clock: a current-thread runtime on tokio's
//! paused clock, with one local task set.

use std::future::Future;

use futures_util::StreamExt;
use strims_base::time::{Duration, Time};
use strims_dao::{Recv, Subscription};
use strims_rpc::Timer;
use tokio::task::LocalSet;
use tokio::time::{Instant, Sleep};

/// The time every node's clock reads when the world starts.
pub const T0: Time = Time::from_unix_millis(1_700_000_000_000);

/// A clock reading [`T0`] at `base`.
#[derive(Clone)]
pub struct TokioTimer {
    pub base: Instant,
}

impl Timer for TokioTimer {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        T0 + Duration::from(self.base.elapsed())
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        let after = (deadline - T0).to_std().unwrap_or_default();
        tokio::time::sleep_until(self.base + after)
    }
}

/// Runs a test: `f` and the local tasks it spawns, on a current-thread
/// runtime whose clock is paused: it jumps to the next timer whenever every
/// task waits.
pub fn run(f: impl Future<Output = ()> + 'static) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f));
}

pub async fn sleep_secs(secs: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
}

/// Waits long enough for links, memberships and controls to settle.
pub async fn settle() {
    sleep_secs(30).await;
}

/// Waits up to `secs` seconds, polling every 100 ms, for `cond`.
pub async fn wait_for(secs: u64, what: &str, mut cond: impl FnMut() -> bool) {
    let steps = secs * 10;
    for _ in 0..steps {
        if cond() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(cond(), "timed out waiting for {what}");
}

/// The events a subscription holds now. Panics if it lagged.
pub fn drain<E: Clone>(events: &mut Subscription<E>) -> Vec<E> {
    let mut out = Vec::new();
    while let Some(Some(r)) = futures_util::FutureExt::now_or_never(events.next()) {
        match r {
            Recv::Event(e) => out.push((*e).clone()),
            Recv::Lagged => panic!("lagged"),
        }
    }
    out
}
