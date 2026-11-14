// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The quantile method of the sims and the lab: the integer nearest rank,
//! read through a log sketch. hack/simlab's `sketch.py` and scoring.sql's
//! `sketch_quantile` are the same method in Python and SQL, value for
//! value (`sketch_cases.tsv`, which all three check).
//!
//! - The `p`th percentile (an integer from 0 to 100) of `n` values is the
//!   one of rank max(1, ⌈p·n/100⌉) in their order, computed in integers
//!   ([`rank`]): a float rank takes the 8th of 100 values for p7.
//! - A value reads as the greatest point of the sketch's grid not above it
//!   ([`floor_f64`], [`floor_u64`]). Each doubling splits into
//!   2^[`SUB_BITS`] buckets, a positive value's bucket its binary exponent
//!   and the top [`SUB_BITS`] bits after its leading 1, read at its lower
//!   bound: never above the value, at most 1/129 under it, and integers
//!   under 256 exactly. Zero is a bucket of its own. A negative value's
//!   bucket mirrors its magnitude's, its least member at the magnitude's far
//!   edge: it reads as the magnitude rounded up to the grid, negated, so it
//!   too is never above the value, and a point of the grid (an integer
//!   above -256) reads as itself.
//!
//! Reading the values a quantile picks from, or the quantile it picks, is
//! one reading: the floor keeps their order. So a histogram of the
//! sketch's buckets ([`ValueHistogram`](super::ValueHistogram)) reads what
//! [`percentile`] reads of the values sorted. A quantile reads in the unit
//! its values are written in: the grid scales by powers of two alone.

use super::WaitHistogram;
use crate::time::Duration;

/// Each doubling splits into `2^SUB_BITS` buckets.
pub const SUB_BITS: u32 = 7;

/// The rank (from 1) of the `p`th percentile (0–100, more reading as 100)
/// of `n` values: max(1, ⌈p·n/100⌉).
pub fn rank(p: u32, n: u64) -> u64 {
    let r = (u128::from(p.min(100)) * u128::from(n)).div_ceil(100);
    u64::try_from(r).unwrap_or(u64::MAX).max(1)
}

/// The value of sorted `v` at the `p`th percentile's [`rank`], as it is.
/// `None` when empty.
pub fn nth<T>(sorted: &[T], p: u32) -> Option<&T> {
    let r = rank(p, sorted.len() as u64);
    sorted.get(usize::try_from(r - 1).ok()?)
}

/// The `p`th percentile of sorted `v`, read through the sketch. `None`
/// when empty.
pub fn percentile<T: Sketched>(sorted: &[T], p: u32) -> Option<T> {
    nth(sorted, p).map(|&v| v.sketched())
}

/// Of bucket counts in their buckets' order, the bucket (its key) that
/// holds the `p`th percentile's [`rank`] of `n`, the counts' sum. `None`
/// when `n` is 0.
pub fn ranked<K>(counts: impl IntoIterator<Item = (K, u64)>, n: u64, p: u32) -> Option<K> {
    if n == 0 {
        return None;
    }
    let r = rank(p, n);
    let mut seen = 0u64;
    for (k, c) in counts {
        seen = seen.saturating_add(c);
        if seen >= r {
            return Some(k);
        }
    }
    None
}

/// A non-negative integer read through the sketch: its bucket's lower
/// bound, `v` itself under 256.
pub const fn floor_u64(v: u64) -> u64 {
    let drop = dropped(u64::BITS - v.leading_zeros());
    v >> drop << drop
}

/// A float read through the sketch: the greatest point of the grid not
/// above `x` (see the module docs); NaN and the infinities as they are,
/// either zero as 0.
pub fn floor_f64(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if x == 0.0 {
        return 0.0;
    }
    let (lo, hi) = bounds(x.abs().to_bits());
    if x > 0.0 {
        f64::from_bits(lo)
    } else if lo == x.abs().to_bits() {
        x
    } else {
        -f64::from_bits(hi)
    }
}

/// The low bits a value of `bits` significant bits drops to its bucket.
const fn dropped(bits: u32) -> u32 {
    bits.saturating_sub(SUB_BITS + 1)
}

/// The bit patterns of the bounds `[lo, hi)` of the bucket of a positive
/// finite float of bit pattern `bits`: a normal one's significand has 53
/// bits (its leading 1 implied), a subnormal's those of its mantissa
/// field; `hi` carries into the exponent at the doubling's last bucket.
fn bounds(bits: u64) -> (u64, u64) {
    let significant = if bits >> 52 == 0 {
        u64::BITS - bits.leading_zeros()
    } else {
        53
    };
    let drop = dropped(significant);
    let lo = bits >> drop << drop;
    (lo, lo + (1 << drop))
}

/// A value the sketch reads ([`floor_u64`], [`floor_f64`]).
pub trait Sketched: Copy {
    /// The value read through the sketch.
    fn sketched(self) -> Self;
}

impl Sketched for u64 {
    fn sketched(self) -> u64 {
        floor_u64(self)
    }
}

impl Sketched for u32 {
    fn sketched(self) -> u32 {
        floor_u64(u64::from(self)) as u32
    }
}

impl Sketched for usize {
    fn sketched(self) -> usize {
        floor_u64(self as u64) as usize
    }
}

impl Sketched for i64 {
    /// A negative one reads as its magnitude rounded up to the grid (an
    /// integer: the grid's points from 256 up are), negated.
    fn sketched(self) -> i64 {
        let a = self.unsigned_abs();
        let lo = floor_u64(a);
        if self >= 0 {
            return lo as i64;
        }
        let up = if lo == a {
            a
        } else {
            lo + (1 << dropped(u64::BITS - a.leading_zeros()))
        };
        0i64.saturating_sub_unsigned(up)
    }
}

impl Sketched for f64 {
    fn sketched(self) -> f64 {
        floor_f64(self)
    }
}

/// The `p`th percentile of the waits `h` counts, as the sims and the lab
/// read a [`WaitHistogram`]: the [`rank`] in integers, read as its
/// bucket's lower bound, 0 for bucket 0 (under 1 ms) and 2^((i−1)/4) ms
/// for bucket i, to the nanosecond below. Never above the wait, and at
/// most a sixth (1 − 2^(−1/4)) under it but in the last bucket, which
/// holds every wait from about 46 s. `None` when empty.
pub fn wait_percentile(h: &WaitHistogram, p: u32) -> Option<Duration> {
    let i = ranked(h.buckets.iter().copied().enumerate(), h.count(), p)?;
    Some(wait_floor(i))
}

/// The lower bound of a [`WaitHistogram`]'s bucket `i`.
fn wait_floor(i: usize) -> Duration {
    if i == 0 {
        return Duration::ZERO;
    }
    let ms = ((i - 1) as f64 / 4.0).exp2();
    Duration::from_nanos((ms * 1e6).floor() as i64)
}

/// `sketch_cases.tsv`, which hack/simlab/tests/test_sketch.py writes from
/// the grid's definition (in fractions) and holds Python and SQL to.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
pub(crate) mod cases {
    /// The cases: floats and integers with their readings, and lists of
    /// each with their percentiles' readings at `ps`.
    #[derive(Default)]
    pub(crate) struct Cases {
        pub ps: Vec<u32>,
        pub floats: Vec<(f64, f64)>,
        pub ints: Vec<(i64, i64)>,
        pub float_lists: Vec<(Vec<f64>, Vec<f64>)>,
        pub int_lists: Vec<(Vec<i64>, Vec<i64>)>,
    }

    pub(crate) fn cases() -> Cases {
        let hex = |s: &str| f64::from_bits(u64::from_str_radix(s, 16).unwrap());
        let int = |s: &str| s.parse::<i64>().unwrap();
        let mut c = Cases::default();
        for line in include_str!("sketch_cases.tsv").lines() {
            if let Some(ps) = line
                .strip_prefix('#')
                .and_then(|l| l.trim().strip_prefix("p = "))
            {
                c.ps = ps.split(", ").map(|p| p.parse().unwrap()).collect();
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            match f[0] {
                "f" => c.floats.push((hex(f[1]), hex(f[2]))),
                "i" => c.ints.push((int(f[1]), int(f[2]))),
                "qf" => c.float_lists.push((
                    f[1].split(',').map(hex).collect(),
                    f[2].split(',').map(hex).collect(),
                )),
                "qi" => c.int_lists.push((
                    f[1].split(',').map(int).collect(),
                    f[2].split(',').map(int).collect(),
                )),
                _ => {}
            }
        }
        c
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::stats::WAIT_BUCKETS;

    /// splitmix64's next value of `s`.
    fn next(s: &mut u64) -> u64 {
        *s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// The grid's points by its definition: 2^e · (1 + m/128).
    fn point(e: i32, m: u32) -> f64 {
        2f64.powi(e) * (1.0 + f64::from(m) / 128.0)
    }

    #[test]
    fn ranks_are_the_integer_nearest_rank() {
        assert_eq!(rank(7, 100), 7);
        assert_eq!(rank(55, 100), 55);
        assert_eq!(rank(50, 4), 2);
        assert_eq!(rank(50, 5), 3);
        assert_eq!(rank(0, 9), 1);
        assert_eq!(rank(100, 9), 9);
        assert_eq!(rank(250, 9), 9);
        assert_eq!(rank(99, 1), 1);
        assert_eq!(rank(1, 0), 1);
        for n in 1..=1000u64 {
            for p in 0..=100u32 {
                let r = rank(p, n);
                // The least rank r with r/n >= p/100.
                assert!(r >= 1 && r <= n, "{p} {n}");
                assert!(100 * r >= u64::from(p) * n, "{p} {n}");
                assert!(r == 1 || 100 * (r - 1) < u64::from(p) * n, "{p} {n}");
            }
        }
    }

    #[test]
    fn the_grid_is_its_definition() {
        for e in [
            -1022, -1000, -30, -1, 0, 1, 7, 8, 30, 52, 53, 60, 1000, 1023,
        ] {
            for m in 0..128 {
                let g = point(e, m);
                assert_eq!(floor_f64(g), g, "{e} {m}");
                assert_eq!(floor_f64(-g), -g, "{e} {m}");
                let below = f64::from_bits(g.to_bits() - 1);
                let prev = if m == 0 {
                    point(e - 1, 127)
                } else {
                    point(e, m - 1)
                };
                assert_eq!(floor_f64(below), prev, "{e} {m}");
                assert_eq!(floor_f64(-below), -g, "{e} {m}");
                let above = f64::from_bits(g.to_bits() + 1);
                assert_eq!(floor_f64(above), g, "{e} {m}");
                let next = if m == 127 {
                    point(e + 1, 0)
                } else {
                    point(e, m + 1)
                };
                if next.is_finite() {
                    assert_eq!(floor_f64(-above), -next, "{e} {m}");
                }
            }
        }
        assert_eq!(floor_f64(0.0), 0.0);
        assert_eq!(floor_f64(-0.0).to_bits(), 0);
        assert!(floor_f64(f64::NAN).is_nan());
        assert_eq!(floor_f64(f64::INFINITY), f64::INFINITY);
        assert_eq!(floor_f64(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert_eq!(floor_f64(f64::MAX), point(1023, 127));
        assert_eq!(floor_f64(-f64::MAX), f64::NEG_INFINITY);
        // Subnormals: their mantissa's top 8 significant bits.
        let tiny = f64::from_bits(0b1_0110_1101);
        assert_eq!(floor_f64(tiny), f64::from_bits(0b1_0110_1100));
        assert_eq!(floor_f64(-tiny), -f64::from_bits(0b1_0110_1110));
        assert_eq!(floor_f64(f64::from_bits(1)), f64::from_bits(1));
    }

    #[test]
    fn floats_read_never_above_and_within_a_129th() {
        let mut s = 7;
        for _ in 0..200_000 {
            let x = f64::from_bits(next(&mut s));
            if !x.is_finite() || x == 0.0 {
                continue;
            }
            let f = floor_f64(x);
            assert!(f <= x, "{x:e}");
            assert_eq!(floor_f64(f), f, "{x:e}");
            if x.abs() >= f64::MIN_POSITIVE && f.is_finite() {
                assert!((x - f).abs() <= x.abs() / 128.0, "{x:e} {f:e}");
                if x > 0.0 {
                    assert!(x - f < x / 129.0 || x - f == 0.0, "{x:e} {f:e}");
                }
            }
        }
    }

    #[test]
    fn integers_read_as_their_floats_do() {
        let mut s = 11;
        let mut vals: Vec<u64> = (0..2048).collect();
        for k in 8..=53 {
            for d in [-2i64, -1, 0, 1, 2] {
                vals.push(((1u64 << k) as i64 + d) as u64);
            }
        }
        vals.extend((0..100_000).map(|_| next(&mut s) >> (11 + next(&mut s) % 53)));
        // Floats hold every integer to 2^53.
        for v in vals.into_iter().filter(|&v| v <= 1 << 53) {
            assert_eq!(floor_u64(v) as f64, floor_f64(v as f64), "{v}");
            let i = i64::try_from(v).unwrap();
            assert_eq!(i.sketched() as f64, floor_f64(v as f64), "{v}");
            assert_eq!((-i).sketched() as f64, floor_f64(-(v as f64)), "-{v}");
        }
        for v in 0..256 {
            assert_eq!(floor_u64(v), v);
        }
        assert_eq!(floor_u64(256), 256);
        assert_eq!(floor_u64(257), 256);
        assert_eq!(floor_u64(1_000), 1_000);
        assert_eq!(floor_u64(1_001), 1_000);
        assert_eq!(floor_u64(u64::MAX), 0xff << 56);
        assert_eq!(i64::MIN.sketched(), i64::MIN);
        assert_eq!((-257i64).sketched(), -258);
    }

    #[test]
    fn percentiles_pick_by_the_integer_rank() {
        let v: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&v, 7), Some(7));
        assert_eq!(percentile(&v, 50), Some(50));
        assert_eq!(percentile(&v, 100), Some(100));
        assert_eq!(percentile::<u64>(&[], 50), None);
        let w = [1_001u64, 2_000, 3_000];
        assert_eq!(percentile(&w, 0), Some(1_000));
        assert_eq!(percentile(&w, 34), Some(2_000));
        assert_eq!(percentile(&[-5.5f64, 0.5, 300.7], 1), Some(-5.5));
        assert_eq!(percentile(&[-5.51f64], 1), Some(-5.531_25));
        assert_eq!(ranked([(10, 2), (20, 0), (30, 2)], 4, 50), Some(10));
        assert_eq!(ranked([(10, 2), (20, 0), (30, 2)], 4, 51), Some(30));
        assert_eq!(ranked([(10, 0)], 0, 50), None);
    }

    #[test]
    fn every_case_reads_as_python_and_sql_read_it() {
        let c = cases::cases();
        assert_eq!(c.ps, [0, 1, 5, 7, 10, 25, 50, 55, 75, 90, 95, 99, 100]);
        assert!(c.floats.len() > 1000 && c.ints.len() > 1000);
        for &(x, want) in &c.floats {
            assert_eq!(floor_f64(x).to_bits(), want.to_bits(), "{x:e}");
        }
        for &(x, want) in &c.ints {
            assert_eq!(x.sketched(), want, "{x}");
            if let Ok(u) = u64::try_from(x) {
                assert_eq!(floor_u64(u) as i64, want, "{x}");
            }
        }
        for (v, wants) in &c.float_lists {
            let mut v = v.clone();
            v.sort_by(f64::total_cmp);
            for (&p, want) in c.ps.iter().zip(wants) {
                assert_eq!(
                    percentile(&v, p).unwrap().to_bits(),
                    want.to_bits(),
                    "{p} {v:?}"
                );
            }
        }
        for (v, wants) in &c.int_lists {
            let mut v = v.clone();
            v.sort_unstable();
            for (&p, &want) in c.ps.iter().zip(wants) {
                assert_eq!(percentile(&v, p), Some(want), "{p} {v:?}");
            }
        }
    }

    #[test]
    fn waits_read_their_buckets_lower_bound() {
        let ms = |x: f64| Duration::from_nanos((x * 1e6) as i64);
        for i in 1..WAIT_BUCKETS {
            let lo = wait_floor(i);
            let mut h = WaitHistogram::default();
            h.add(lo + Duration::from_nanos(1));
            assert_eq!(h.buckets[i], 1, "{i}");
            assert_eq!(wait_percentile(&h, 50), Some(lo), "{i}");
            let mut below = WaitHistogram::default();
            below.add(lo - Duration::from_nanos(1));
            assert_eq!(below.buckets[i - 1], 1, "{i}");
        }
        let mut h = WaitHistogram::default();
        assert_eq!(wait_percentile(&h, 50), None);
        for w in 1..=100 {
            h.add(ms(f64::from(w)));
        }
        // 50 ms is in the bucket from 2^(22/4) ≈ 45.25 ms.
        assert_eq!(wait_percentile(&h, 50), Some(wait_floor(23)));
        assert_eq!(wait_percentile(&h, 0), Some(Duration::from_millis(1)));
        h.add(ms(0.3));
        assert_eq!(wait_percentile(&h, 0), Some(Duration::ZERO));
    }
}
