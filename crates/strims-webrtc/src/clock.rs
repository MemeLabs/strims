// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `Time` ↔ `Instant` for str0m.
//!
//! str0m and dimpl take `std::time::Instant`s, and an `Instant` can only be
//! made from `Instant::now()`. Each [`Clock`] reads the wall clock exactly
//! once, as an anchor, and maps every `Time` to `anchor + (time - base)`.
//! str0m only compares and subtracts instants, so its behaviour depends on
//! the `Time`s fed in and never on the anchor: the same inputs give the same
//! outputs in every run.

// str0m's API is in `Instant`s; this module is the only place they are made.
#![allow(clippy::disallowed_types)]

use std::time::Instant;

use strims_base::time::{Duration, Time};

/// Deadlines further out than this are str0m's "not happening" sentinel
/// (`Instant::now() + 100 years`) and are reported as no deadline.
const NEVER: Duration = Duration::from_secs(10 * 365 * 24 * 60 * 60);

#[derive(Clone, Copy, Debug)]
pub(crate) struct Clock {
    anchor: Instant,
    base: Time,
}

impl Clock {
    pub(crate) fn new(base: Time) -> Clock {
        // The single wall-clock read: an anchor for the mapping, whose value
        // never influences str0m (see the module docs).
        #[allow(clippy::disallowed_methods)]
        let anchor = Instant::now();
        Clock { anchor, base }
    }

    /// The `Instant` for `t`. Times before the base map to the anchor
    /// (str0m ignores time going backwards).
    pub(crate) fn instant(&self, t: Time) -> Instant {
        let d = t - self.base;
        let nanos = u64::try_from(d.as_nanos()).unwrap_or(0);
        self.anchor
            .checked_add(std::time::Duration::from_nanos(nanos))
            .unwrap_or(self.anchor)
    }

    /// The `Time` for an instant str0m returned, or `None` for its "never"
    /// sentinel. Instants before the anchor (str0m's "already happened")
    /// map to the base.
    pub(crate) fn time(&self, i: Instant) -> Option<Time> {
        let d = match i.checked_duration_since(self.anchor) {
            Some(d) => Duration::from(d),
            None => Duration::ZERO,
        };
        if d > NEVER {
            return None;
        }
        Some(self.base + d)
    }
}

/// Rounds a deadline up to [`Time::PRECISION`], so a driver whose clock is
/// truncated to milliseconds never wakes just before a deadline and spins.
pub(crate) fn ceil_precision(t: Time) -> Time {
    let p = Time::PRECISION.as_nanos();
    let n = t.unix_nanos();
    let r = n.rem_euclid(p);
    if r == 0 {
        t
    } else {
        Time::from_unix_nanos(n.saturating_add(p - r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_sentinels() {
        let base = Time::from_unix_millis(1_000_000);
        let c = Clock::new(base);
        let t = base + Duration::from_nanos(123_456_789);
        assert_eq!(c.time(c.instant(t)), Some(t));
        // Before the base: clamped.
        assert_eq!(c.time(c.instant(base - Duration::SECOND)), Some(base));
        // "Never".
        let far = c.instant(base) + std::time::Duration::from_secs(100 * 365 * 24 * 3600);
        assert_eq!(c.time(far), None);
    }

    #[test]
    fn ceil_to_millis() {
        let t = Time::from_unix_nanos(5_000_001);
        assert_eq!(ceil_precision(t), Time::from_unix_millis(6));
        assert_eq!(
            ceil_precision(Time::from_unix_millis(6)),
            Time::from_unix_millis(6)
        );
    }
}
