// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The leaky-bucket rate limiter.
//!
//! The bucket holds up to one second of traffic at the configured rate and
//! drains continuously. The level is an exact integer in nano-bytes
//! (bytes × 10⁹), so draining by `elapsed_ns × limit` never rounds and
//! [`Hlb::ready_at`] can give the exact release time.

use strims_base::time::{Duration, Time};

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// A leaky bucket with a capacity of one second at `limit` bytes per
/// second.
#[derive(Debug, Clone)]
pub struct Hlb {
    /// Bytes per second.
    limit: u64,
    /// Bucket level in nano-bytes.
    level: u128,
    /// Time the level was last drained to.
    last: Time,
}

impl Hlb {
    /// `limit` is in bytes per second; 0 lets nothing through.
    pub fn new(limit: u64) -> Hlb {
        Hlb {
            limit,
            level: 0,
            last: Time::EPOCH,
        }
    }

    /// Bytes per second.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Clamps the current level to the new capacity.
    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
        self.level = self.level.min(Self::capacity(limit));
    }

    /// Time of the last drain. Every release time reported by
    /// [`Hlb::ready_at`] is at or after it.
    pub fn last(&self) -> Time {
        self.last
    }

    fn capacity(limit: u64) -> u128 {
        u128::from(limit) * NANOS_PER_SEC
    }

    /// Drains the bucket up to `now`. A `now` earlier than the last drain is
    /// ignored, so a clock stepping backwards can't refill the bucket.
    pub fn advance(&mut self, now: Time) {
        if now <= self.last {
            return;
        }
        let elapsed = u128::try_from((now - self.last).as_nanos()).unwrap_or(0);
        self.level = self
            .level
            .saturating_sub(elapsed.saturating_mul(u128::from(self.limit)));
        self.last = now;
    }

    /// Whether `n` bytes may pass at the last drained time: `level + n` must
    /// stay below the capacity. An empty bucket passes any packet, so one
    /// at least as large as the limit can't stall forever.
    pub fn admits(&self, n: u64) -> bool {
        if self.limit == 0 {
            return false;
        }
        self.level == 0
            || self.level.saturating_add(u128::from(n) * NANOS_PER_SEC) < Self::capacity(self.limit)
    }

    /// Adds `n` sent bytes to the bucket.
    pub fn charge(&mut self, n: u64) {
        self.level = self.level.saturating_add(u128::from(n) * NANOS_PER_SEC);
    }

    /// Drains, tests, and charges if admitted.
    pub fn check(&mut self, n: u64, now: Time) -> bool {
        self.advance(now);
        if !self.admits(n) {
            return false;
        }
        self.charge(n);
        true
    }

    /// Earliest time at which [`Hlb::admits`] holds for `n` bytes, or `None`
    /// if it never will (limit 0). Returns [`Hlb::last`] when it already
    /// holds.
    pub fn ready_at(&self, n: u64) -> Option<Time> {
        if self.limit == 0 {
            return None;
        }
        if self.admits(n) {
            return Some(self.last);
        }
        let need = u128::from(n) * NANOS_PER_SEC;
        let cap = Self::capacity(self.limit);
        // Highest level at which `n` passes.
        let target = if need < cap { cap - need - 1 } else { 0 };
        let excess = self.level.saturating_sub(target);
        let ns = excess.div_ceil(u128::from(self.limit));
        let ns = i64::try_from(ns).unwrap_or(i64::MAX);
        Some(self.last + Duration::from_nanos(ns))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ms(n: i64) -> Time {
        Time::from_unix_millis(n)
    }

    #[test]
    fn admits_below_capacity() {
        // 1000 B/s: one second of burst, `value + n >= limit` rejects.
        let mut h = Hlb::new(1000);
        assert!(h.check(600, ms(0)));
        assert!(!h.check(400, ms(0)), "600 + 400 >= 1000");
        assert!(h.check(399, ms(0)));
        // 999 queued; after 500ms, 499 remain.
        assert!(!h.check(501, ms(500)));
        assert!(h.check(500, ms(500)));
    }

    #[test]
    fn oversize_packet_passes_empty_bucket() {
        let mut h = Hlb::new(1000);
        assert!(h.check(1500, ms(0)));
        // 500 bytes left after a second: 500 + 600 >= 1000.
        assert!(!h.check(600, ms(1000)));
        assert_eq!(h.ready_at(600), Some(ms(1100) + Duration::NANOSECOND));
        assert!(!h.check(1500, ms(1400)), "only an empty bucket takes it");
        assert_eq!(h.ready_at(1500), Some(ms(1500)));
        assert!(h.check(1500, ms(1500)));
    }

    #[test]
    fn ready_at_is_exact() {
        let mut h = Hlb::new(1000);
        assert!(h.check(900, ms(0)));
        // Needs level + 200 < 1000, so level <= 799.999999999 bytes: 100ms
        // plus one nanosecond.
        let t = h.ready_at(200).unwrap();
        assert_eq!(t, ms(100) + Duration::NANOSECOND);
        let mut early = h.clone();
        early.advance(t - Duration::NANOSECOND);
        assert!(!early.admits(200));
        h.advance(t);
        assert!(h.admits(200));
    }

    #[test]
    fn zero_limit_never_admits() {
        let mut h = Hlb::new(0);
        assert!(!h.check(1, ms(0)));
        assert_eq!(h.ready_at(1), None);
    }

    #[test]
    fn clock_going_backwards_does_not_refill() {
        let mut h = Hlb::new(1000);
        assert!(h.check(900, ms(1000)));
        assert!(!h.check(200, ms(0)));
        assert_eq!(h.last(), ms(1000));
    }

    #[test]
    fn set_limit_clamps() {
        let mut h = Hlb::new(1000);
        assert!(h.check(900, ms(0)));
        h.set_limit(100);
        // Level clamped to 100 bytes, which drains in one second.
        assert!(!h.admits(1));
        assert!(h.check(1, ms(1000)));
    }
}
