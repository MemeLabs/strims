// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! An uplink's capacity as a schedule of delivery opportunities.
//!
//! A radio uplink (LTE, 5G, a satellite's) sends when the base station or
//! the satellite grants it, in amounts and at times that follow the radio
//! and the cell's load, not a rate. A [`Capacity`] is a recording of those
//! grants: opportunities at times within a period, each carrying some
//! bytes, the schedule repeating every period from an origin. The
//! recordings are Mahimahi's (Winstein et al., NSDI 2013: a millisecond a
//! line, each line a 1500-byte packet's opportunity), and CellReplay's and
//! LeoReplayer's saturated traces in the same format.
//!
//! An uplink with a capacity keeps a byte cursor, the capacity its
//! messages have taken ([`Cursor`]). A message of `wire` bytes handed to it
//! at `t`:
//!
//! - capacity that passed unused while nothing waited is lost: the cursor
//!   moves up to the bytes of every opportunity before `t`
//!   ([`Capacity::before`]);
//! - the cursor takes `wire` more, and the message's last bit leaves at the
//!   opportunity where the cursor ends ([`Capacity::carries`]).
//!
//! This is `hack/testbed/tracelink.c`'s model, which shapes the Docker
//! testbed's trace-driven uplinks: an opportunity's bytes serve what waits
//! when it comes, a message longer than what is left of one finishes in
//! the next, and none carry over idle time.

use strims_base::time::{Duration, Time};

/// A schedule of delivery opportunities (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capacity {
    /// Each opportunity's time within the period, in ns, non-decreasing.
    at: Vec<i64>,
    /// The bytes of every opportunity up to each, within the period:
    /// increasing, and the last is the period's.
    cum: Vec<u64>,
    /// ns, at least the last opportunity's time and more than 0.
    period: i64,
    /// When the schedule's time 0 falls.
    origin: Time,
}

/// The most opportunities a schedule holds.
pub const MAX_OPPORTUNITIES: usize = 1 << 26;

impl Capacity {
    /// A schedule of `(time within the period, bytes)` opportunities, in
    /// time order, repeating every `period` from the unix epoch; `scale`
    /// multiplies each opportunity's bytes, fractions carrying to the next,
    /// as `tracelink --scale` does.
    pub fn new(
        opportunities: impl IntoIterator<Item = (Duration, f64)>,
        period: Option<Duration>,
        scale: f64,
    ) -> Result<Capacity, String> {
        if !(scale > 0.0 && scale.is_finite()) {
            return Err(format!("scale {scale}: positive"));
        }
        let (mut at, mut cum) = (Vec::new(), Vec::new());
        let (mut total, mut carry, mut last) = (0u64, 0.0f64, 0i64);
        for (t, bytes) in opportunities {
            let t = t.as_nanos();
            if t < last {
                return Err("opportunity times go back".into());
            }
            if !(bytes > 0.0 && bytes <= 1e9) {
                return Err(format!("{bytes} bytes: out of range"));
            }
            last = t;
            carry += bytes * scale;
            let whole = carry.floor();
            carry -= whole;
            if whole >= 1.0 {
                total = total.saturating_add(whole as u64);
                at.push(t);
                cum.push(total);
            }
            if at.len() > MAX_OPPORTUNITIES {
                return Err("too many opportunities".into());
            }
        }
        if at.is_empty() {
            return Err("no opportunities".into());
        }
        let period = period.map_or(last, Duration::as_nanos);
        if period <= 0 || period < last {
            return Err("the period is shorter than the schedule".into());
        }
        // An opportunity at the period's end is the next period's first.
        let k = at.iter().rev().take_while(|&&t| t == period).count();
        if k > 0 {
            let bytes: Vec<u64> = cum
                .iter()
                .scan(0u64, |prev, &c| {
                    let b = c - *prev;
                    *prev = c;
                    Some(b)
                })
                .collect();
            let n = at.len();
            let order = (n - k..n).chain(0..n - k);
            let (mut at2, mut cum2, mut sum) = (Vec::with_capacity(n), Vec::with_capacity(n), 0u64);
            for i in order {
                #[allow(clippy::indexing_slicing)] // invariant: i < n
                {
                    at2.push(if i >= n - k { 0 } else { at[i] });
                    sum += bytes[i];
                }
                cum2.push(sum);
            }
            at = at2;
            cum = cum2;
        }
        Ok(Capacity {
            at,
            cum,
            period,
            origin: Time::from_unix_nanos(0),
        })
    }

    /// Mahimahi's format: a millisecond a line, each line one opportunity
    /// of `opportunity` bytes, the period the last line's time (whose
    /// opportunities are the next period's first, at its time 0).
    pub fn mahimahi(text: &str, opportunity: u64, scale: f64) -> Result<Capacity, String> {
        let mut ops = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let ms: i64 = line.parse().map_err(|e| format!("line {}: {e}", n + 1))?;
            if !(0..1 << 40).contains(&ms) {
                return Err(format!("line {}: {ms} ms out of range", n + 1));
            }
            ops.push((Duration::from_millis(ms), opportunity as f64));
        }
        Capacity::new(ops, None, scale)
    }

    /// `tracelink`'s `us` format: `<t_us> <bytes>` a line, the period the
    /// last line's time unless a `# period_us <p>` line gives it.
    pub fn microseconds(text: &str, scale: f64) -> Result<Capacity, String> {
        let (mut ops, mut period) = (Vec::new(), None);
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if let Some(p) = line.strip_prefix("# period_us ") {
                let us: i64 = p
                    .trim()
                    .parse()
                    .map_err(|e| format!("line {}: {e}", n + 1))?;
                period = Some(Duration::from_micros(us));
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut f = line.split_whitespace();
            let (Some(t), Some(b), None) = (f.next(), f.next(), f.next()) else {
                return Err(format!("line {}: `<t_us> <bytes>`", n + 1));
            };
            let t: i64 = t.parse().map_err(|e| format!("line {}: {e}", n + 1))?;
            let b: f64 = b.parse().map_err(|e| format!("line {}: {e}", n + 1))?;
            if !(0..1 << 50).contains(&t) {
                return Err(format!("line {}: {t} us out of range", n + 1));
            }
            ops.push((Duration::from_micros(t), b));
        }
        Capacity::new(ops, period, scale)
    }

    /// The same schedule with its time 0 at `origin`.
    pub fn starting(mut self, origin: Time) -> Capacity {
        self.origin = origin;
        self
    }

    pub fn period(&self) -> Duration {
        Duration::from_nanos(self.period)
    }

    /// The bytes a period carries.
    pub fn per_period(&self) -> u64 {
        self.cum.last().copied().unwrap_or(0)
    }

    /// The mean rate, bytes per second.
    pub fn mean_rate(&self) -> u64 {
        u64::try_from(i128::from(self.per_period()) * 1_000_000_000 / i128::from(self.period))
            .unwrap_or(u64::MAX)
    }

    /// Of the period's opportunities, how many come before `r` ns into it.
    fn count_before(&self, r: i64) -> usize {
        self.at.partition_point(|&t| t < r)
    }

    /// The bytes of every opportunity before `t`, counted from the origin
    /// (negative before it).
    pub fn before(&self, t: Time) -> i128 {
        let rel = i128::from(t.unix_nanos()) - i128::from(self.origin.unix_nanos());
        let p = i128::from(self.period);
        let (m, r) = (rel.div_euclid(p), rel.rem_euclid(p));
        let i = self.count_before(i64::try_from(r).unwrap_or(i64::MAX));
        let within = i
            .checked_sub(1)
            .and_then(|j| self.cum.get(j))
            .copied()
            .unwrap_or(0);
        m * i128::from(self.per_period()) + i128::from(within)
    }

    /// When the opportunity that carries byte `pos` (counted from the
    /// origin, as [`Capacity::before`] counts) comes.
    pub fn carries(&self, pos: i128) -> Time {
        let total = i128::from(self.per_period().max(1));
        let m = (pos - 1).div_euclid(total);
        let r = pos - m * total;
        let i = self
            .cum
            .partition_point(|&c| i128::from(c) < r)
            .min(self.at.len().saturating_sub(1));
        let t = self.at.get(i).copied().unwrap_or(0);
        let ns = i128::from(self.origin.unix_nanos()) + m * i128::from(self.period) + i128::from(t);
        Time::from_unix_nanos(i64::try_from(ns).unwrap_or(i64::MAX))
    }

    /// The mean rate from `from` to `to`, bytes per second (0 when `to`
    /// isn't later).
    pub fn rate_between(&self, from: Time, to: Time) -> u64 {
        let span = (to - from).as_nanos();
        if span <= 0 {
            return 0;
        }
        let bytes = self.before(to) - self.before(from);
        u64::try_from(bytes * 1_000_000_000 / i128::from(span)).unwrap_or(0)
    }
}

/// An uplink's place in its [`Capacity`]: the bytes its messages have
/// taken, counted from the schedule's origin (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor(pub i128);

impl Cursor {
    /// Takes `wire` bytes handed to the uplink at `at`; returns when their
    /// last bit leaves, never before `at`.
    pub fn take(&mut self, cap: &Capacity, at: Time, wire: usize) -> Time {
        self.0 = self.0.max(cap.before(at)) + i128::try_from(wire).unwrap_or(i128::MAX);
        cap.carries(self.0).max(at)
    }

    /// The bytes taken that the schedule hasn't carried by `at`: what the
    /// uplink holds.
    pub fn held(&self, cap: &Capacity, at: Time) -> u64 {
        u64::try_from(self.0 - cap.before(at)).unwrap_or(0)
    }

    /// Forgets what the uplink holds at `at`: the bytes taken past what
    /// the schedule carried by then.
    pub fn forget(&mut self, cap: &Capacity, at: Time) {
        self.0 = self.0.min(cap.before(at));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    fn origin() -> Time {
        t(0)
    }

    #[test]
    fn mahimahi_lines_are_opportunities_of_a_packet() {
        let c = Capacity::mahimahi("1\n1\n3\n10\n", 1500, 1.0).unwrap();
        assert_eq!(c.per_period(), 6000);
        assert_eq!(c.period(), Duration::from_millis(10));
        assert_eq!(c.mean_rate(), 600_000);
        assert!(Capacity::mahimahi("", 1500, 1.0).is_err());
        assert!(Capacity::mahimahi("5\n3\n", 1500, 1.0).is_err());
        assert!(Capacity::mahimahi("x\n", 1500, 1.0).is_err());
    }

    #[test]
    fn microsecond_lines_take_a_period() {
        let c = Capacity::microseconds("# period_us 2000\n0 100\n500 200.5\n", 1.0).unwrap();
        assert_eq!(c.period(), Duration::from_millis(2));
        assert_eq!(c.per_period(), 300);
        assert!(Capacity::microseconds("# period_us 100\n500 1\n", 1.0).is_err());
    }

    #[test]
    fn a_scale_carries_its_fractions() {
        let c = Capacity::mahimahi("1\n2\n3\n4\n", 1514, 0.5).unwrap();
        assert_eq!(c.per_period(), 4 * 757);
        let c = Capacity::mahimahi("1\n2\n3\n", 3, 0.5).unwrap();
        // 1.5, 3.0, 4.5 bytes taken: opportunities of 1, 2 and 1.
        assert_eq!(c.per_period(), 4);
    }

    #[test]
    fn capacity_repeats_from_its_origin_both_ways() {
        let c = Capacity::mahimahi("2\n2\n5\n10\n", 1000, 1.0)
            .unwrap()
            .starting(origin());
        // The opportunity at 10 ms comes at every period's 0.
        assert_eq!(c.before(t(0)), 0);
        assert_eq!(c.before(t(1)), 1000);
        assert_eq!(c.before(t(2)), 1000);
        assert_eq!(c.before(t(3)), 3000);
        assert_eq!(c.before(t(10)), 4000);
        assert_eq!(c.before(t(11)), 5000);
        assert_eq!(c.before(t(13)), 7000);
        assert_eq!(c.before(t(-1)), 0);
        assert_eq!(c.before(t(-8)), -3000);
        assert_eq!(c.carries(1), t(0));
        assert_eq!(c.carries(1000), t(0));
        assert_eq!(c.carries(1001), t(2));
        assert_eq!(c.carries(3001), t(5));
        assert_eq!(c.carries(4000), t(5));
        assert_eq!(c.carries(4001), t(10));
        assert_eq!(c.carries(0), t(-5));
        assert_eq!(c.carries(-999), t(-5));
    }

    #[test]
    fn a_cursor_loses_idle_capacity_and_queues_the_rest() {
        let c = Capacity::mahimahi("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n", 1000, 1.0)
            .unwrap()
            .starting(origin());
        let mut k = Cursor::default();
        // At 0.5 ms the next opportunity is at 1 ms; 1500 bytes need two.
        assert_eq!(k.take(&c, t(0) + Duration::from_micros(500), 1500), t(2));
        // Half of the 2 ms opportunity is left for what comes before it.
        assert_eq!(k.take(&c, t(1), 500), t(2));
        assert_eq!(k.held(&c, t(1)), 2000);
        // At 5.5 ms the opportunities of 3-5 ms went unused.
        assert_eq!(k.take(&c, t(5) + Duration::from_micros(500), 100), t(6));
        assert_eq!(k.held(&c, t(5) + Duration::from_micros(500)), 100);
        // A message never leaves before it came.
        assert!(k.take(&c, t(20), 1) >= t(20));
    }

    #[test]
    fn the_rate_between_two_times_is_what_passed() {
        let c = Capacity::mahimahi("1\n1\n1\n500\n1000\n", 1500, 1.0)
            .unwrap()
            .starting(origin());
        assert_eq!(c.rate_between(t(0), t(1000)), 5 * 1500);
        assert_eq!(c.rate_between(t(0), t(2)), 4 * 1500 * 500);
        assert_eq!(c.rate_between(t(5), t(5)), 0);
    }
}
