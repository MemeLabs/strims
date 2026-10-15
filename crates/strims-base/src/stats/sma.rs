// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::time::{Duration, Time};

/// Simple moving average over a sliding window of `n` intervals of `d`.
///
/// Every method that reads the window takes `now` and first expires the
/// intervals that ended before it, so readers take `&mut self`. The sums
/// wrap on overflow; rates and intervals saturate.
#[derive(Clone, Debug)]
pub struct Sma {
    /// Start of the current interval.
    t: Time,
    d: Duration,
    /// Sum of the values in the window.
    v: u64,
    /// Number of samples in the window.
    n: u64,
    /// Index of the current interval in `w`.
    i: usize,
    /// Number of completed intervals, up to `w.len()`.
    wl: usize,
    /// Bound: `n` intervals, fixed at construction.
    w: Box<[SmaSample]>,
}

#[derive(Clone, Copy, Debug, Default)]
struct SmaSample {
    v: u64,
    n: u64,
}

impl Sma {
    /// A window of `n` intervals of `d` starting at `now`. `n` is raised to
    /// at least 1 and `d` to at least 1 ns.
    pub fn new(n: usize, d: Duration, now: Time) -> Sma {
        Sma {
            t: now,
            d: d.max(Duration::NANOSECOND),
            v: 0,
            n: 0,
            i: 0,
            wl: 0,
            w: vec![SmaSample::default(); n.max(1)].into_boxed_slice(),
        }
    }

    /// Expires the intervals that ended before `end`. A long idle gap costs
    /// at most one pass over the window.
    fn advance(&mut self, end: Time) {
        let d = i128::from(self.d.as_nanos());
        let delta = i128::from(end.unix_nanos()) - i128::from(self.t.unix_nanos());
        if delta <= d {
            return;
        }
        // The number of whole intervals: the largest k with t + k*d < end.
        let k = (delta - 1) / d;
        // invariant: t + k*d < end, which fits in an i64.
        self.t = Time::from_unix_nanos((i128::from(self.t.unix_nanos()) + k * d) as i64);

        let len = self.w.len();
        if k >= len as i128 {
            self.i = ((self.i as i128 + k) % len as i128) as usize;
            self.wl = len;
            self.v = 0;
            self.n = 0;
            self.w.fill(SmaSample::default());
            return;
        }

        for _ in 0..k {
            self.i += 1;
            if self.i == len {
                self.i = 0;
            }
            if self.wl < len {
                self.wl += 1;
            }
            if let Some(s) = self.w.get_mut(self.i) {
                self.v = self.v.wrapping_sub(s.v);
                self.n = self.n.wrapping_sub(s.n);
                *s = SmaSample::default();
            }
        }
    }

    /// Empties the window and restarts it at `now`.
    pub fn reset(&mut self, now: Time) {
        self.t = now;
        self.v = 0;
        self.n = 0;
        self.i = 0;
        self.wl = 0;
        self.w.fill(SmaSample::default());
    }

    /// Adds one sample of `v`.
    pub fn add(&mut self, v: u64, now: Time) {
        self.add_n(1, v, now);
    }

    /// Adds `c` samples of `v`.
    pub fn add_n(&mut self, c: u64, v: u64, now: Time) {
        self.advance(now);

        let cv = c.wrapping_mul(v);
        if let Some(s) = self.w.get_mut(self.i) {
            s.v = s.v.wrapping_add(cv);
            s.n = s.n.wrapping_add(c);
        }
        self.v = self.v.wrapping_add(cv);
        self.n = self.n.wrapping_add(c);
    }

    /// The mean sample value in the window.
    pub fn value(&mut self, now: Time) -> u64 {
        self.advance(now);
        self.v.checked_div(self.n).unwrap_or(0)
    }

    /// The number of samples in the window.
    pub fn sample_count(&mut self, now: Time) -> u64 {
        self.advance(now);
        self.n
    }

    /// The completed window length per unit of value.
    pub fn interval(&mut self, now: Time) -> Duration {
        self.advance(now);
        self.window_per(self.v)
    }

    /// The completed window length per sample.
    pub fn sample_interval(&mut self, now: Time) -> Duration {
        self.advance(now);
        self.window_per(self.n)
    }

    /// The value summed over the window, scaled to a rate per `d`.
    pub fn rate(&mut self, d: Duration, now: Time) -> u64 {
        self.advance(now);
        self.per(self.v, d)
    }

    /// The sample count over the window, scaled to a rate per `d`.
    pub fn sample_rate(&mut self, d: Duration, now: Time) -> u64 {
        self.advance(now);
        self.per(self.n, d)
    }

    /// `wl * d / x`, or 0 if `x` is 0. `x` may exceed `i64::MAX`, so this
    /// needs 128 bits.
    fn window_per(&self, x: u64) -> Duration {
        if x == 0 {
            return Duration::ZERO;
        }
        let r = self.wl as i128 * i128::from(self.d.as_nanos()) / i128::from(x);
        Duration::from_nanos(i64::try_from(r).unwrap_or(i64::MAX))
    }

    /// `x * d / (wl * self.d)`, or 0 before the first interval completes.
    /// Needs 128 bits: for a byte rate per second, `x * d` exceeds `u64`
    /// once the window holds about 18 GB. Saturates at the bounds.
    fn per(&self, x: u64, d: Duration) -> u64 {
        if self.wl == 0 {
            return 0;
        }
        let r = i128::from(x) * i128::from(d.as_nanos())
            / (self.wl as i128 * i128::from(self.d.as_nanos()));
        u64::try_from(r.max(0)).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::det_rng;
    use rand_core::Rng;

    fn ms(n: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + n)
    }

    /// Reference model that expires one interval per step.
    struct SteppedSma {
        t: i64,
        d: i64,
        v: u64,
        n: u64,
        i: usize,
        wl: usize,
        w: Vec<(u64, u64)>,
    }

    // Test reference: an out-of-range index is a test failure.
    #[allow(clippy::indexing_slicing)]
    impl SteppedSma {
        fn advance(&mut self, end: i64) {
            let mut t = self.t + self.d;
            while t < end {
                self.t = t;
                self.i += 1;
                if self.i == self.w.len() {
                    self.i = 0;
                }
                if self.wl < self.w.len() {
                    self.wl += 1;
                }
                self.v -= self.w[self.i].0;
                self.n -= self.w[self.i].1;
                self.w[self.i] = (0, 0);
                t += self.d;
            }
        }
    }

    #[test]
    fn averages_within_window() {
        let mut s = Sma::new(10, Duration::SECOND, ms(0));
        s.add(10, ms(100));
        s.add(20, ms(200));
        s.add_n(2, 30, ms(1_500));
        assert_eq!(s.sample_count(ms(1_600)), 4);
        assert_eq!(s.value(ms(1_600)), (10 + 20 + 60) / 4);
    }

    #[test]
    fn samples_expire() {
        let mut s = Sma::new(3, Duration::SECOND, ms(0));
        s.add(5, ms(0));
        // The interval holding the sample is reused after 3 more intervals.
        assert_eq!(s.sample_count(ms(3_000)), 1);
        assert_eq!(s.sample_count(ms(3_001)), 0);
        assert_eq!(s.value(ms(3_001)), 0);
    }

    #[test]
    fn rates() {
        let mut s = Sma::new(4, Duration::SECOND, ms(0));
        assert_eq!(s.rate(Duration::SECOND, ms(0)), 0);
        for i in 0..8 {
            s.add(1000, ms(i * 500));
        }
        // An interval ends once `now` is past `t + d`, so the samples at
        // 0, 0.5 and 1 s share the first. The window holds all 8 samples,
        // and the rates divide by the 3 completed intervals.
        let now = ms(3_600);
        assert_eq!(s.sample_count(now), 8);
        assert_eq!(s.rate(Duration::SECOND, now), 8000 / 3);
        assert_eq!(s.sample_rate(Duration::SECOND, now), 8 / 3);
        assert_eq!(s.interval(now), Duration::from_nanos(3_000_000_000 / 8000));
        assert_eq!(
            s.sample_interval(now),
            Duration::from_nanos(3_000_000_000 / 8)
        );
    }

    /// The window sum times `d` exceeds `u64` here.
    #[test]
    fn rate_does_not_overflow() {
        let mut s = Sma::new(60, Duration::SECOND, ms(0));
        for i in 0..60 {
            s.add(1 << 30, ms(i * 1_000));
        }
        // The first interval (2 samples) has expired by now.
        let now = ms(60_500);
        assert_eq!(s.sample_count(now), 58);
        assert_eq!(s.rate(Duration::SECOND, now), 58 * (1 << 30) / 60);
    }

    #[test]
    fn reset_restarts_window() {
        let mut s = Sma::new(4, Duration::SECOND, ms(0));
        s.add(7, ms(2_500));
        s.reset(ms(10_000));
        assert_eq!(s.sample_count(ms(10_000)), 0);
        assert_eq!(s.rate(Duration::SECOND, ms(10_000)), 0);
        s.add(7, ms(10_100));
        assert_eq!(s.value(ms(10_200)), 7);
    }

    #[test]
    fn degenerate_parameters_are_clamped() {
        // One interval of 1 ns: each sample expires the previous one.
        let mut s = Sma::new(0, Duration::ZERO, ms(0));
        s.add(3, ms(0));
        s.add(5, ms(10));
        assert_eq!(s.value(ms(10)), 5);
        assert_eq!(s.sample_count(ms(20)), 0);
    }

    /// The skip-ahead in `advance` leaves exactly the state of the stepwise
    /// model, for random gaps short and long.
    #[test]
    #[allow(clippy::indexing_slicing)]
    fn advance_skip_ahead_matches_stepwise_expiry() {
        let mut rng = det_rng(9);
        for case in 0..200 {
            let len = 1 + (rng.next_u32() % 12) as usize;
            let d = 1 + i64::from(rng.next_u32() % 1_000);
            let start = 1_000_000;
            let mut s = Sma::new(len, Duration::from_nanos(d), Time::from_unix_nanos(start));
            let mut g = SteppedSma {
                t: start,
                d,
                v: 0,
                n: 0,
                i: 0,
                wl: 0,
                w: vec![(0, 0); len],
            };

            let mut now = start;
            for _ in 0..50 {
                now += match rng.next_u32() % 4 {
                    0 => 0,
                    1 => i64::from(rng.next_u32()) % d,
                    2 => i64::from(rng.next_u32()) % (d * len as i64 * 2),
                    _ => i64::from(rng.next_u32()) % (d * 1_000),
                };
                let v = u64::from(rng.next_u32() % 100);
                s.add(v, Time::from_unix_nanos(now));
                g.advance(now);
                g.w[g.i].0 += v;
                g.w[g.i].1 += 1;
                g.v += v;
                g.n += 1;

                let state = (s.t.unix_nanos(), s.v, s.n, s.i, s.wl);
                assert_eq!(state, (g.t, g.v, g.n, g.i, g.wl), "case {case}");
                let w: Vec<_> = s.w.iter().map(|x| (x.v, x.n)).collect();
                assert_eq!(w, g.w, "case {case}");
            }
        }
    }
}
