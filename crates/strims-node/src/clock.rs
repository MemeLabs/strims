// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node's clock.
//!
//! The core crates take `now: Time` as an argument; the driver reads it from
//! a [`Clock`]. Timers are tokio timers, so a clock also maps a deadline to
//! the runtime's [`tokio::time::Instant`].
//!
//! - [`SystemClock`] reads the wall clock (production). This module is the
//!   only place in the crate that does.
//! - [`SimClock`] is a fixed epoch plus the time elapsed on the runtime's
//!   clock. Under turmoil (or a paused tokio runtime) that clock is
//!   simulated, so the same seed gives the same times.

use strims_base::time::{Duration, Time};
use tokio::time::Instant;

/// A source of `now` and the bridge from deadlines to tokio timers.
pub trait Clock: Clone + Send + Sync + 'static {
    /// The current time, at [`Time::PRECISION`].
    fn now(&self) -> Time;

    /// The runtime instant at which [`Clock::now`] reaches `t`. Never
    /// earlier than `t`, so a timer set for it fires at or after the
    /// deadline.
    fn deadline(&self, t: Time) -> Instant;
}

/// How far ahead an unreachable deadline is parked (a year).
const FAR_FUTURE: std::time::Duration = std::time::Duration::from_secs(365 * 24 * 3600);

/// `base + d`, saturating at a year from `base` for deadlines too far away
/// for `Instant`.
fn add(base: Instant, d: Duration) -> Instant {
    let d = d.to_std().unwrap_or_default().min(FAR_FUTURE);
    base.checked_add(d).unwrap_or(base)
}

/// The wall clock, for production nodes.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Time {
        // The production clock: the one place the driver reads the wall
        // clock. Core crates only ever see the `Time` it returns.
        #[allow(clippy::disallowed_methods)]
        let t = std::time::SystemTime::now();
        Time::from_system(t)
    }

    fn deadline(&self, t: Time) -> Instant {
        // `now()` truncates to milliseconds, so `t - now()` overshoots the
        // real remaining time by less than a millisecond: the timer fires at
        // or after `t`.
        add(Instant::now(), t - self.now())
    }
}

/// A fixed epoch plus the runtime clock's elapsed time: deterministic under
/// turmoil and under a paused tokio runtime.
#[derive(Clone, Copy, Debug)]
pub struct SimClock {
    epoch: Time,
    base: Instant,
}

impl SimClock {
    /// `now()` is `epoch` at the current runtime instant. Must be called
    /// inside a tokio runtime.
    pub fn new(epoch: Time) -> SimClock {
        SimClock {
            epoch,
            base: Instant::now(),
        }
    }

    /// `now()` is `epoch` at `base`.
    pub fn with_base(epoch: Time, base: Instant) -> SimClock {
        SimClock { epoch, base }
    }

    /// The simulation clock of a turmoil host: `epoch` at the start of the
    /// simulation, so hosts started (or restarted) later agree on the time.
    #[cfg(feature = "turmoil")]
    pub fn turmoil(epoch: Time) -> SimClock {
        let elapsed = turmoil::sim_elapsed().unwrap_or_default();
        SimClock {
            epoch: epoch + Duration::from(elapsed),
            base: Instant::now(),
        }
    }

    /// The epoch this clock started at.
    pub fn epoch(&self) -> Time {
        self.epoch
    }
}

impl Clock for SimClock {
    fn now(&self) -> Time {
        let elapsed = Instant::now().saturating_duration_since(self.base);
        (self.epoch + Duration::from(elapsed)).truncate(Time::PRECISION)
    }

    fn deadline(&self, t: Time) -> Instant {
        add(self.base, t - self.epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sim_clock_follows_the_runtime() {
        let epoch = Time::from_unix_millis(1_700_000_000_000);
        let c = SimClock::new(epoch);
        assert_eq!(c.now(), epoch);
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        assert_eq!(c.now(), epoch + Duration::from_millis(1500));
        let t = epoch + Duration::from_secs(3);
        tokio::time::sleep_until(c.deadline(t)).await;
        assert_eq!(c.now(), t);
        // Deadlines in the past fire at once; far deadlines don't overflow.
        assert!(c.deadline(epoch - Duration::from_secs(10)) <= Instant::now());
        assert!(c.deadline(Time::MAX) > Instant::now());
    }

    #[tokio::test]
    async fn system_clock_deadlines_are_not_early() {
        let c = SystemClock;
        let t = c.now() + Duration::from_millis(20);
        tokio::time::sleep_until(c.deadline(t)).await;
        assert!(c.now() >= t);
        assert!(c.deadline(Time::MAX) > Instant::now());
    }
}
