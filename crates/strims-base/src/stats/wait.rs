// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::time::Duration;

/// Buckets in a [`WaitHistogram`].
pub const WAIT_BUCKETS: usize = 64;
/// Buckets per doubling of the wait.
const PER_OCTAVE: f64 = 4.0;

/// Waits counted in quarter-octave buckets: bucket 0 holds waits under
/// 1 ms, bucket `i` those from 2^((i−1)/4) up to 2^(i/4) ms, and the last
/// every wait from about 46 s. A percentile reads within a fifth of the
/// true wait. `Copy`, so diagnostics can carry it by value; counts only
/// grow, so two snapshots subtract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitHistogram {
    pub buckets: [u64; WAIT_BUCKETS],
}

impl Default for WaitHistogram {
    fn default() -> WaitHistogram {
        WaitHistogram {
            buckets: [0; WAIT_BUCKETS],
        }
    }
}

impl WaitHistogram {
    /// Counts one wait; a negative one counts as none.
    pub fn add(&mut self, wait: Duration) {
        let ms = wait.as_nanos().max(0) as f64 / 1e6;
        let i = if ms < 1.0 {
            0
        } else {
            (1.0 + (ms.log2() * PER_OCTAVE).floor()).min((WAIT_BUCKETS - 1) as f64) as usize
        };
        if let Some(b) = self.buckets.get_mut(i) {
            *b = b.saturating_add(1);
        }
    }

    /// Waits counted.
    pub fn count(&self) -> u64 {
        self.buckets.iter().sum()
    }

    /// Adds `o`'s counts.
    pub fn merge(&mut self, o: &WaitHistogram) {
        for (a, b) in self.buckets.iter_mut().zip(o.buckets) {
            *a = a.saturating_add(b);
        }
    }

    /// The upper bound of the bucket that holds the `p`th percentile
    /// (0–100): no more than a fifth above the wait. `None` when empty.
    pub fn percentile(&self, p: f64) -> Option<Duration> {
        let n = self.count();
        if n == 0 {
            return None;
        }
        let rank = ((p / 100.0) * n as f64).ceil().clamp(1.0, n as f64) as u64;
        let mut seen = 0;
        for (i, &c) in self.buckets.iter().enumerate() {
            seen += c;
            if seen >= rank {
                let ms = (i as f64 / PER_OCTAVE).exp2();
                return Some(Duration::from_micros((ms * 1e3).round() as i64));
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn ms(n: i64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn percentiles_read_within_a_fifth_above() {
        let mut h = WaitHistogram::default();
        assert_eq!(h.percentile(50.0), None);
        for w in 1..=100 {
            h.add(ms(w));
        }
        let p50 = h.percentile(50.0).unwrap();
        assert!((ms(50)..=ms(60)).contains(&p50), "{p50}");
        let p99 = h.percentile(99.0).unwrap();
        assert!((ms(99)..=ms(119)).contains(&p99), "{p99}");
        assert_eq!(h.count(), 100);
    }

    #[test]
    fn short_and_long_waits_land_at_the_ends() {
        let mut h = WaitHistogram::default();
        h.add(Duration::from_micros(300));
        h.add(ms(-5));
        h.add(Duration::from_secs(3600));
        assert_eq!(h.buckets[0], 2);
        assert_eq!(h.buckets[WAIT_BUCKETS - 1], 1);
        assert_eq!(h.percentile(50.0), Some(ms(1)));
    }

    #[test]
    fn merged_counts_add() {
        let mut a = WaitHistogram::default();
        let mut b = WaitHistogram::default();
        a.add(ms(10));
        b.add(ms(10));
        b.add(ms(1000));
        a.merge(&b);
        assert_eq!(a.count(), 3);
        let p = a.percentile(100.0).unwrap();
        assert!((ms(1000)..=ms(1190)).contains(&p), "{p}");
    }
}
