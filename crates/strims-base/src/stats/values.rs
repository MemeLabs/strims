// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;

/// Values under `2^EXACT_BITS` count exactly.
const EXACT_BITS: u64 = 6;
const EXACT: u64 = 1 << EXACT_BITS;
/// Above [`EXACT`], each doubling splits into `2^SUB_BITS` buckets.
const SUB_BITS: u64 = 5;
const SUB_BUCKETS: u64 = 1 << SUB_BITS;

/// Non-negative integers counted in buckets no wider than 1/32 of the
/// values they hold (one value each under 64), for percentiles of gauges
/// sampled over a run. A percentile reads its bucket's lower bound: never
/// above the value, at most 1/32 under it. Sparse: it holds the buckets it
/// counted in, at most 1920 over every `u64`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueHistogram {
    buckets: BTreeMap<u16, u64>,
    count: u64,
    max: u64,
}

impl ValueHistogram {
    /// Counts `v`.
    pub fn add(&mut self, v: u64) {
        let c = self.buckets.entry(bucket(v)).or_default();
        *c = c.saturating_add(1);
        self.count = self.count.saturating_add(1);
        self.max = self.max.max(v);
    }

    /// Values counted.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// The largest value counted, exactly. `None` when empty.
    pub fn max(&self) -> Option<u64> {
        (self.count > 0).then_some(self.max)
    }

    /// Adds `o`'s counts.
    pub fn merge(&mut self, o: &ValueHistogram) {
        for (&b, &n) in &o.buckets {
            let c = self.buckets.entry(b).or_default();
            *c = c.saturating_add(n);
        }
        self.count = self.count.saturating_add(o.count);
        self.max = self.max.max(o.max);
    }

    /// The values above zero alone.
    pub fn nonzero(&self) -> ValueHistogram {
        let mut h = self.clone();
        if let Some(zeros) = h.buckets.remove(&0) {
            h.count = h.count.saturating_sub(zeros);
        }
        if h.count == 0 {
            h.max = 0;
        }
        h
    }

    /// The lower bound of the bucket that holds the `p`th percentile
    /// (0–100, nearest rank). `None` when empty.
    pub fn percentile(&self, p: f64) -> Option<u64> {
        let n = self.count;
        if n == 0 {
            return None;
        }
        let rank = ((p / 100.0) * n as f64).ceil().clamp(1.0, n as f64) as u64;
        let mut seen = 0u64;
        for (&b, &c) in &self.buckets {
            seen = seen.saturating_add(c);
            if seen >= rank {
                return Some(lower(b));
            }
        }
        None
    }
}

/// The bucket of `v`.
fn bucket(v: u64) -> u16 {
    if v < EXACT {
        return v as u16;
    }
    let e = u64::from(63 - v.leading_zeros());
    let sub = (v >> (e - SUB_BITS)) - SUB_BUCKETS;
    (EXACT + (e - EXACT_BITS) * SUB_BUCKETS + sub) as u16
}

/// The least value in bucket `b`.
fn lower(b: u16) -> u64 {
    let b = u64::from(b);
    if b < EXACT {
        return b;
    }
    let k = b - EXACT;
    let e = EXACT_BITS + k / SUB_BUCKETS;
    (SUB_BUCKETS + k % SUB_BUCKETS) << (e - SUB_BITS)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn small_values_count_exactly() {
        let mut h = ValueHistogram::default();
        assert_eq!(h.percentile(50.0), None);
        assert_eq!(h.max(), None);
        for v in 0..EXACT {
            h.add(v);
        }
        assert_eq!(h.percentile(0.0), Some(0));
        assert_eq!(h.percentile(50.0), Some(31));
        assert_eq!(h.percentile(100.0), Some(63));
        assert_eq!(h.max(), Some(63));
        assert_eq!(h.count(), EXACT);
    }

    #[test]
    fn percentiles_read_within_a_thirty_second_below() {
        for v in [64, 65, 100, 1_000, 4_096, 123_456, 1 << 40, u64::MAX] {
            let mut h = ValueHistogram::default();
            h.add(v);
            let got = h.percentile(50.0).unwrap();
            assert!(got <= v && v - got <= v / 32, "{v}: {got}");
            assert_eq!(h.max(), Some(v));
        }
    }

    #[test]
    fn buckets_are_ordered_and_bound_their_values() {
        for v in 1..100_000u64 {
            assert!(bucket(v) >= bucket(v - 1), "{v}");
            assert!(lower(bucket(v)) <= v, "{v}");
            assert_eq!(bucket(lower(bucket(v))), bucket(v), "{v}");
        }
        assert_eq!(bucket(u64::MAX), 1919);
    }

    #[test]
    fn merged_counts_add() {
        let mut a = ValueHistogram::default();
        let mut b = ValueHistogram::default();
        a.add(10);
        b.add(10);
        b.add(5_000);
        a.merge(&b);
        assert_eq!(a.count(), 3);
        assert_eq!(a.percentile(50.0), Some(10));
        assert_eq!(a.max(), Some(5_000));
        let p = a.percentile(100.0).unwrap();
        assert!((4_844..=5_000).contains(&p), "{p}");
    }

    #[test]
    fn nonzero_leaves_out_the_zeros() {
        let mut h = ValueHistogram::default();
        for v in [0, 0, 0, 7, 9] {
            h.add(v);
        }
        let n = h.nonzero();
        assert_eq!(
            (n.count(), n.percentile(50.0), n.max()),
            (2, Some(7), Some(9))
        );
        assert_eq!(h.percentile(50.0), Some(0));
        let mut zeros = ValueHistogram::default();
        zeros.add(0);
        assert_eq!(zeros.nonzero(), ValueHistogram::default());
    }
}
