// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Version vectors.
//!
//! A replicated record's version maps each replica (device id) to the
//! number of writes it has made to the record, and carries the wall-clock
//! time of the latest write in unix milliseconds. A replica missing from
//! the map counts as zero. Two versions are ordered when one is at least
//! the other at every replica; otherwise they are concurrent.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use strims_base::time::Time;
use strims_proto::dao::v1::VersionVector;

/// A version with one write by `replica` at `now`.
pub fn seed(replica: u64, now: Time) -> VersionVector {
    VersionVector {
        value: BTreeMap::from([(replica, 1)]),
        updated_at: now.unix_millis(),
    }
}

/// Records a write by `replica` at `now`. The write's time is `now`, or
/// a millisecond after the version's time if that is later, so a write is
/// always timed after every write it follows. Picking the latest of
/// concurrent writes by time then agrees with causality.
pub fn increment(v: &mut VersionVector, replica: u64, now: Time) {
    let n = v.value.entry(replica).or_insert(0);
    *n = n.saturating_add(1);
    v.updated_at = v.updated_at.saturating_add(1).max(now.unix_millis());
}

/// Raises `d` to at least `v` at every replica, and to `v`'s time if later.
pub fn upgrade(d: &mut VersionVector, v: &VersionVector) {
    d.updated_at = d.updated_at.max(v.updated_at);
    for (&r, &n) in &v.value {
        let e = d.value.entry(r).or_insert(0);
        *e = (*e).max(n);
    }
}

/// Lowers `d` to at most `v` at every replica, keeping only replicas both
/// hold, and to `v`'s time if earlier. An empty `v` empties `d` and zeroes
/// its time.
pub fn downgrade(d: &mut VersionVector, v: &VersionVector) {
    if v.value.is_empty() {
        d.value.clear();
        d.updated_at = 0;
        return;
    }
    d.updated_at = d.updated_at.min(v.updated_at);
    d.value.retain(|r, n| match v.value.get(r) {
        Some(&m) => {
            *n = (*n).min(m);
            true
        }
        None => false,
    });
}

/// How `a` orders against `b`: `None` when they are concurrent.
pub fn compare(a: &VersionVector, b: &VersionVector) -> Option<Ordering> {
    let mut lt = false;
    let mut gt = false;
    for r in a.value.keys().chain(b.value.keys()) {
        let x = a.value.get(r).copied().unwrap_or(0);
        let y = b.value.get(r).copied().unwrap_or(0);
        lt |= x < y;
        gt |= x > y;
    }
    match (lt, gt) {
        (true, true) => None,
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => Some(Ordering::Equal),
    }
}

/// `v`'s counts without its time.
pub fn without_time(v: &VersionVector) -> VersionVector {
    VersionVector {
        value: v.value.clone(),
        updated_at: 0,
    }
}

/// `replica`'s count in `v`.
pub fn get(v: &VersionVector, replica: u64) -> u64 {
    v.value.get(&replica).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vv(entries: &[(u64, u64)], at: i64) -> VersionVector {
        VersionVector {
            value: entries.iter().copied().collect(),
            updated_at: at,
        }
    }

    #[test]
    fn increment_counts_and_stamps() {
        let mut v = VersionVector::default();
        increment(&mut v, 3, Time::from_unix_millis(10));
        increment(&mut v, 3, Time::from_unix_millis(20));
        increment(&mut v, 4, Time::from_unix_millis(30));
        assert_eq!(v, vv(&[(3, 2), (4, 1)], 30));
        increment(&mut v, 4, Time::from_unix_millis(5));
        assert_eq!(v, vv(&[(3, 2), (4, 2)], 31));
        assert_eq!(seed(5, Time::from_unix_millis(7)), vv(&[(5, 1)], 7));
    }

    #[test]
    fn upgrade_takes_pointwise_max() {
        let mut d = vv(&[(1, 3), (2, 1)], 5);
        upgrade(&mut d, &vv(&[(2, 4), (3, 1)], 9));
        assert_eq!(d, vv(&[(1, 3), (2, 4), (3, 1)], 9));
        upgrade(&mut d, &vv(&[], 1));
        assert_eq!(d, vv(&[(1, 3), (2, 4), (3, 1)], 9));
    }

    #[test]
    fn downgrade_takes_intersection_min() {
        let mut d = vv(&[(1, 3), (2, 5), (4, 1)], 5);
        downgrade(&mut d, &vv(&[(1, 4), (2, 2), (3, 7)], 3));
        assert_eq!(d, vv(&[(1, 3), (2, 2)], 3));
        downgrade(&mut d, &vv(&[], 9));
        assert_eq!(d, vv(&[], 0));
    }

    #[test]
    fn compare_orders_or_reports_concurrency() {
        let a = vv(&[(1, 2), (2, 1)], 0);
        assert_eq!(compare(&a, &a), Some(Ordering::Equal));
        assert_eq!(
            compare(&a, &vv(&[(1, 2), (2, 1), (3, 0)], 0)),
            Some(Ordering::Equal)
        );
        assert_eq!(compare(&a, &vv(&[(1, 3), (2, 1)], 0)), Some(Ordering::Less));
        assert_eq!(compare(&a, &vv(&[(1, 2)], 0)), Some(Ordering::Greater));
        assert_eq!(compare(&a, &vv(&[(1, 3), (2, 0)], 0)), None);
        assert_eq!(compare(&vv(&[], 0), &a), Some(Ordering::Less));
    }
}
