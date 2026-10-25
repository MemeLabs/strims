// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The largest or smallest sample over a sliding window of time, in a
//! fixed number of buckets: fixed-size state, and a sample leaves the
//! window up to a bucket's width late.

use strims_base::time::{Duration, Time};

/// Buckets per window.
pub(crate) const BUCKETS: usize = 8;

/// Bucket width in nanoseconds for `window`.
fn width(window: Duration) -> i64 {
    (window.as_nanos() / BUCKETS as i64).max(1)
}

/// The largest sample over a window of time.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WindowedMax {
    /// Per bucket: its index (time / width) and its largest sample.
    buckets: [(i64, i64); BUCKETS],
    /// Bucket width, nanoseconds.
    width: i64,
}

impl WindowedMax {
    pub(crate) fn new(window: Duration) -> WindowedMax {
        WindowedMax {
            buckets: [(i64::MIN, 0); BUCKETS],
            width: width(window),
        }
    }

    fn index(&self, now: Time) -> i64 {
        now.unix_nanos() / self.width
    }

    pub(crate) fn add(&mut self, now: Time, v: i64) {
        let index = self.index(now);
        let slot = usize::try_from(index.rem_euclid(BUCKETS as i64)).unwrap_or(0);
        if let Some(b) = self.buckets.get_mut(slot) {
            if b.0 == index {
                b.1 = b.1.max(v);
            } else {
                *b = (index, v);
            }
        }
    }

    /// The largest sample in the window ending at `now`, or 0.
    pub(crate) fn get(&self, now: Time) -> i64 {
        let current = self.index(now);
        let oldest = current.saturating_sub(BUCKETS as i64 - 1);
        self.buckets
            .iter()
            .filter(|(i, _)| *i >= oldest && *i <= current)
            .map(|&(_, v)| v)
            .max()
            .unwrap_or(0)
    }
}

/// The smallest sample over a window of time.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WindowedMin {
    /// Per bucket: its index (time / width) and its smallest sample.
    buckets: [(i64, i64); BUCKETS],
    /// Bucket width, nanoseconds.
    width: i64,
}

impl WindowedMin {
    pub(crate) fn new(window: Duration) -> WindowedMin {
        WindowedMin {
            buckets: [(i64::MIN, i64::MAX); BUCKETS],
            width: width(window),
        }
    }

    fn index(&self, now: Time) -> i64 {
        now.unix_nanos() / self.width
    }

    pub(crate) fn add(&mut self, now: Time, v: i64) {
        let index = self.index(now);
        let slot = usize::try_from(index.rem_euclid(BUCKETS as i64)).unwrap_or(0);
        if let Some(b) = self.buckets.get_mut(slot) {
            if b.0 == index {
                b.1 = b.1.min(v);
            } else {
                *b = (index, v);
            }
        }
    }

    /// The smallest sample in the window ending at `now`, if any.
    pub(crate) fn get(&self, now: Time) -> Option<i64> {
        let current = self.index(now);
        let oldest = current.saturating_sub(BUCKETS as i64 - 1);
        self.buckets
            .iter()
            .filter(|(i, _)| *i >= oldest && *i <= current)
            .map(|&(_, v)| v)
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    #[test]
    fn extrema_age_out_of_the_window() {
        // 8 buckets of 1 s.
        let mut lo = WindowedMin::new(Duration::from_secs(8));
        let mut hi = WindowedMax::new(Duration::from_secs(8));
        assert_eq!(lo.get(t(0)), None);
        assert_eq!(hi.get(t(0)), 0);
        for (ms, v) in [(0, 50), (1_000, 600), (2_000, 700)] {
            lo.add(t(ms), v);
            hi.add(t(ms), v);
        }
        assert_eq!(lo.get(t(2_000)), Some(50));
        assert_eq!(hi.get(t(2_000)), 700);
        // The first bucket leaves the window 8 buckets on.
        assert_eq!(lo.get(t(7_999)), Some(50));
        assert_eq!(lo.get(t(8_000)), Some(600));
        assert_eq!(hi.get(t(10_000)), 0);
        assert_eq!(lo.get(t(10_000)), None);
    }
}
