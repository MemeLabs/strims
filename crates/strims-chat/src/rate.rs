// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-key token buckets.
//!
//! Each key's bucket holds up to `burst` tokens and regains one every
//! `interval`; a take spends one. A bucket is held as the time it is full
//! again, so a full bucket is the same as no bucket, and the bucket that
//! fills first is the one whose loss matters least. The limiter holds at
//! most `max_keys` buckets: full ones are forgotten, and a new key beyond
//! the bound evicts the bucket that fills first.

use std::collections::{BTreeMap, BTreeSet};

use strims_base::time::{Duration, Time};

/// The most keys a [`RateLimiter`] holds by default.
pub const MAX_RATE_KEYS: usize = 4096;

/// A [`RateLimiter`]'s buckets and bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateConfig {
    /// The tokens of a full bucket; 0 counts as 1.
    pub burst: u32,
    /// The time a bucket takes to regain one token; at least 1 ns.
    pub interval: Duration,
    /// The most keys held; 0 counts as 1.
    pub max_keys: usize,
}

impl Default for RateConfig {
    /// A burst of 5, then 1 per second, for [`MAX_RATE_KEYS`] keys.
    fn default() -> RateConfig {
        RateConfig {
            burst: 5,
            interval: Duration::SECOND,
            max_keys: MAX_RATE_KEYS,
        }
    }
}

/// Token buckets by key.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    config: RateConfig,
    /// When each bucket is full again; at most `config.max_keys`.
    full_at: BTreeMap<Vec<u8>, Time>,
    /// `full_at` ordered by time.
    by_time: BTreeSet<(Time, Vec<u8>)>,
}

impl RateLimiter {
    pub fn new(config: RateConfig) -> Self {
        Self {
            config,
            full_at: BTreeMap::new(),
            by_time: BTreeSet::new(),
        }
    }

    pub fn config(&self) -> RateConfig {
        self.config
    }

    /// Spends one of `key`'s tokens at `now`. False, spending nothing,
    /// when its bucket is empty.
    pub fn try_take(&mut self, now: Time, key: &[u8]) -> bool {
        self.forget_full(now);

        let interval = self.config.interval.max(Duration::NANOSECOND);
        let slack = interval * i64::from(self.config.burst.max(1) - 1);
        let full_at = match self.full_at.get(key) {
            Some(&t) => {
                if t - now > slack {
                    return false;
                }
                self.by_time.remove(&(t, key.to_vec()));
                t
            }
            None => {
                while self.full_at.len() >= self.config.max_keys.max(1) {
                    let Some((_, k)) = self.by_time.pop_first() else {
                        break;
                    };
                    self.full_at.remove(&k);
                }
                now
            }
        };
        let next = full_at.max(now) + interval;
        self.full_at.insert(key.to_vec(), next);
        self.by_time.insert((next, key.to_vec()));
        true
    }

    /// Forgets the buckets that are full at `now`.
    fn forget_full(&mut self, now: Time) {
        while let Some((t, _)) = self.by_time.first() {
            if t.after(now) {
                break;
            }
            if let Some((_, k)) = self.by_time.pop_first() {
                self.full_at.remove(&k);
            }
        }
    }

    /// The keys held: those whose buckets aren't full.
    pub fn len(&self) -> usize {
        self.full_at.len()
    }

    pub fn is_empty(&self) -> bool {
        self.full_at.is_empty()
    }

    pub fn clear(&mut self) {
        self.full_at.clear();
        self.by_time.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: Time = Time::from_unix(1_000, 0);

    fn takes(r: &mut RateLimiter, now: Time, key: &[u8], n: usize) -> usize {
        (0..n).filter(|_| r.try_take(now, key)).count()
    }

    #[test]
    fn a_full_bucket_allows_a_burst() {
        let mut r = RateLimiter::new(RateConfig::default());
        assert_eq!(takes(&mut r, T0, b"a", 10), 5);
        assert!(!r.try_take(T0, b"a"));
    }

    #[test]
    fn a_bucket_regains_one_token_per_interval() {
        let mut r = RateLimiter::new(RateConfig::default());
        assert_eq!(takes(&mut r, T0, b"a", 5), 5);
        let almost = T0 + Duration::SECOND - Duration::NANOSECOND;
        assert!(!r.try_take(almost, b"a"));
        assert!(r.try_take(T0 + Duration::SECOND, b"a"));
        assert!(!r.try_take(T0 + Duration::SECOND, b"a"));
        assert_eq!(takes(&mut r, T0 + Duration::SECOND * 3, b"a", 5), 2);
        assert_eq!(takes(&mut r, T0 + Duration::SECOND * 60, b"a", 10), 5);
    }

    #[test]
    fn a_steady_rate_within_the_refill_never_runs_dry() {
        let mut r = RateLimiter::new(RateConfig::default());
        for i in 0..100 {
            assert!(r.try_take(T0 + Duration::SECOND * i, b"a"));
        }
    }

    #[test]
    fn keys_have_their_own_buckets() {
        let mut r = RateLimiter::new(RateConfig::default());
        assert_eq!(takes(&mut r, T0, b"a", 5), 5);
        assert!(!r.try_take(T0, b"a"));
        assert!(r.try_take(T0, b"b"));
    }

    #[test]
    fn the_config_sets_burst_and_interval() {
        let mut r = RateLimiter::new(RateConfig {
            burst: 2,
            interval: Duration::MILLISECOND * 100,
            max_keys: 8,
        });
        assert_eq!(takes(&mut r, T0, b"a", 5), 2);
        assert!(r.try_take(T0 + Duration::MILLISECOND * 100, b"a"));

        let mut zero = RateLimiter::new(RateConfig {
            burst: 0,
            interval: Duration::ZERO,
            max_keys: 0,
        });
        assert_eq!(takes(&mut zero, T0, b"a", 3), 1);
        assert!(zero.try_take(T0 + Duration::NANOSECOND, b"a"));
    }

    #[test]
    fn full_buckets_are_forgotten() {
        let mut r = RateLimiter::new(RateConfig::default());
        r.try_take(T0, b"a");
        r.try_take(T0, b"b");
        assert_eq!(r.len(), 2);
        r.try_take(T0 + Duration::SECOND, b"c");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn a_new_key_past_the_bound_evicts_the_bucket_that_fills_first() {
        let mut r = RateLimiter::new(RateConfig {
            max_keys: 3,
            ..RateConfig::default()
        });
        assert_eq!(takes(&mut r, T0, b"a", 5), 5);
        assert_eq!(takes(&mut r, T0, b"b", 1), 1);
        assert_eq!(takes(&mut r, T0, b"c", 5), 5);
        assert!(r.try_take(T0, b"d"));
        assert_eq!(r.len(), 3);
        assert!(!r.try_take(T0, b"a"), "a keeps its empty bucket");
        assert!(!r.try_take(T0, b"c"), "c keeps its empty bucket");
        assert_eq!(takes(&mut r, T0, b"b", 5), 5, "b lost its bucket");
    }

    #[test]
    fn the_default_bound_holds() {
        let mut r = RateLimiter::new(RateConfig::default());
        for i in 0..(MAX_RATE_KEYS as u32 + 100) {
            assert!(r.try_take(T0, &i.to_be_bytes()));
        }
        assert_eq!(r.len(), MAX_RATE_KEYS);
    }
}
