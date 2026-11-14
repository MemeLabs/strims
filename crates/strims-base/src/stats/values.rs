// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;

use super::sketch;

/// Non-negative integers counted in the log sketch's buckets
/// ([`sketch`]: 128 a doubling, one value each under 256), for
/// percentiles of gauges sampled over a run. A percentile takes the
/// integer nearest rank and reads its bucket's lower bound, as
/// [`sketch::percentile`] reads the values sorted: never above the value,
/// at most 1/129 under it. Sparse: it holds the buckets it counted in, at
/// most 7424 over every `u64`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueHistogram {
    /// Counts by bucket, each keyed by its lower bound.
    buckets: BTreeMap<u64, u64>,
    count: u64,
    max: u64,
}

impl ValueHistogram {
    /// Counts `v`.
    pub fn add(&mut self, v: u64) {
        let c = self.buckets.entry(sketch::floor_u64(v)).or_default();
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

    /// The lower bound of the bucket that holds the `p`th percentile (0–100,
    /// the integer nearest rank: [`sketch::rank`]). `None` when empty.
    pub fn percentile(&self, p: u32) -> Option<u64> {
        sketch::ranked(self.buckets.iter().map(|(&b, &c)| (b, c)), self.count, p)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn small_values_count_exactly() {
        let mut h = ValueHistogram::default();
        assert_eq!(h.percentile(50), None);
        assert_eq!(h.max(), None);
        for v in 0..256 {
            h.add(v);
        }
        assert_eq!(h.percentile(0), Some(0));
        assert_eq!(h.percentile(50), Some(127));
        assert_eq!(h.percentile(100), Some(255));
        assert_eq!(h.max(), Some(255));
        assert_eq!(h.count(), 256);
        assert_eq!(h.buckets.len(), 256);
    }

    #[test]
    fn percentiles_read_within_a_129th_below() {
        for v in [256, 257, 300, 1_000, 4_097, 123_456, 1 << 40, u64::MAX] {
            let mut h = ValueHistogram::default();
            h.add(v);
            let got = h.percentile(50).unwrap();
            assert!(got <= v && v - got <= v / 129, "{v}: {got}");
            assert_eq!(h.max(), Some(v));
        }
    }

    #[test]
    fn buckets_hold_a_128th_of_a_doubling() {
        let mut h = ValueHistogram::default();
        for v in 1 << 20..1 << 21 {
            h.add(v);
        }
        assert_eq!(h.buckets.len(), 128);
        h.add(u64::MAX);
        h.add(0);
        let mut all = ValueHistogram::default();
        for e in 0..64 {
            for v in [1u64 << e, (1u64 << e) | ((1u64 << e) - 1)] {
                all.add(v);
            }
        }
        assert!(all.buckets.len() <= 7424);
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
        assert_eq!(a.percentile(50), Some(10));
        assert_eq!(a.max(), Some(5_000));
        assert_eq!(a.percentile(100), Some(4_992));
    }

    /// splitmix64's next value of `s`.
    fn next(s: &mut u64) -> u64 {
        *s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    #[test]
    fn percentiles_read_what_the_sorted_values_read() {
        let mut s = 3;
        for round in 0..300 {
            let n = 1 + next(&mut s) % 400;
            let shift = next(&mut s) % 60;
            let mut v: Vec<u64> = (0..n).map(|_| next(&mut s) >> shift).collect();
            if round % 3 == 0 {
                v.iter_mut().for_each(|x| *x %= 300);
            }
            let mut h = ValueHistogram::default();
            v.iter().for_each(|&x| h.add(x));
            v.sort_unstable();
            for p in 0..=100 {
                assert_eq!(h.percentile(p), sketch::percentile(&v, p), "{round} {p}");
            }
        }
        // And the cases' non-negative integers, as Python and SQL read them.
        let c = sketch::cases::cases();
        let lists = c
            .int_lists
            .iter()
            .filter(|(v, _)| v.iter().all(|&x| x >= 0));
        let mut n = 0;
        for (v, wants) in lists {
            let mut h = ValueHistogram::default();
            v.iter().for_each(|&x| h.add(x as u64));
            for (&p, &want) in c.ps.iter().zip(wants) {
                assert_eq!(h.percentile(p), Some(want as u64), "{p} {v:?}");
            }
            n += 1;
        }
        assert!(n > 10, "{n}");
    }

    #[test]
    fn nonzero_leaves_out_the_zeros() {
        let mut h = ValueHistogram::default();
        for v in [0, 0, 0, 7, 9] {
            h.add(v);
        }
        let n = h.nonzero();
        assert_eq!(
            (n.count(), n.percentile(50), n.max()),
            (2, Some(7), Some(9))
        );
        assert_eq!(h.percentile(50), Some(0));
        let mut zeros = ValueHistogram::default();
        zeros.add(0);
        assert_eq!(zeros.nonzero(), ValueHistogram::default());
    }
}
