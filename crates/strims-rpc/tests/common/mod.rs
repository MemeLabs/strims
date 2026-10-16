// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the integration tests share.

use strims_base::time::{Duration, Time};
use strims_rpc::Timer;
use tokio::time::{Instant, Sleep};

/// `Time` on tokio's clock: `epoch` at `base`.
#[derive(Clone)]
pub struct TokioTimer {
    epoch: Time,
    base: Instant,
}

impl TokioTimer {
    /// A timer whose `now()` is `epoch` at the current instant.
    pub fn new(epoch: Time) -> TokioTimer {
        TokioTimer {
            epoch,
            base: Instant::now(),
        }
    }
}

impl Timer for TokioTimer {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        self.epoch + Duration::from(self.base.elapsed())
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        let after = (deadline - self.epoch).to_std().unwrap_or_default();
        tokio::time::sleep_until(self.base + after)
    }
}
