// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Record ids.
//!
//! A profile's record ids come from a chain of half-open ranges
//! `[next_id, last_id)` (a `profile.v1.ProfileID`). Devices of one profile
//! hold disjoint ranges: a device hands part of its free ids to a new or
//! starving device, so ids stay unique across the profile without
//! coordination. The chain is consumed from its head.

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::Time;
use strims_proto::profile::v1::ProfileId;

/// The most ranges a chain may hold.
pub const MAX_RANGES: usize = 64;

/// The chain no ids are allocated from yet: every id but 0.
pub fn initial_ranges() -> ProfileId {
    ProfileId {
        next_id: 1,
        last_id: u64::MAX,
        next_range: None,
    }
}

/// The chain has fewer ids than requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("profile ids exhausted")]
pub struct Exhausted;

/// A chain from a peer is malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid profile id ranges")]
pub struct InvalidRanges;

/// Takes up to `n` ids from the head of `p`: the range `[first, end)`,
/// which holds fewer than `n` ids when the head range runs out first.
pub fn incr(p: &mut ProfileId, n: u64) -> Result<(u64, u64), Exhausted> {
    while p.next_id >= p.last_id {
        let next = p.next_range.take().ok_or(Exhausted)?;
        *p = *next;
    }
    let n = n.min(p.last_id - p.next_id);
    let first = p.next_id;
    p.next_id += n;
    Ok((first, p.next_id))
}

/// Takes `n` ids from `p` as a chain of their own. `p` is unchanged on
/// failure.
pub fn pop(p: &mut ProfileId, mut n: u64) -> Result<Option<ProfileId>, Exhausted> {
    let mut rest = p.clone();
    let mut out: Option<ProfileId> = None;
    while n > 0 {
        let (first, end) = incr(&mut rest, n)?;
        n -= end - first;
        out = Some(ProfileId {
            next_id: first,
            last_id: end,
            next_range: out.map(Box::new),
        });
    }
    *p = rest;
    Ok(out)
}

/// Appends `ranges` to the end of `p`'s chain. Fails, leaving `p`
/// unchanged, if the chain would hold more than [`MAX_RANGES`] ranges.
pub fn push(p: &mut ProfileId, more: ProfileId) -> Result<(), InvalidRanges> {
    if depth(p).saturating_add(depth(&more)) > MAX_RANGES {
        return Err(InvalidRanges);
    }
    let all: Vec<(u64, u64)> = ranges(p)
        .chain(ranges(&more))
        .map(|r| (r.next_id, r.last_id))
        .collect();
    if let Some(chain) = from_ranges(&all) {
        *p = chain;
    }
    Ok(())
}

/// The chain of `ranges` in order, `None` if there are none.
pub fn from_ranges(ranges: &[(u64, u64)]) -> Option<ProfileId> {
    let mut out: Option<ProfileId> = None;
    for &(next_id, last_id) in ranges.iter().rev() {
        out = Some(ProfileId {
            next_id,
            last_id,
            next_range: out.map(Box::new),
        });
    }
    out
}

/// The number of ids left in `p`.
pub fn free_count(p: &ProfileId) -> u64 {
    ranges(p).fold(0u64, |n, r| {
        n.saturating_add(r.last_id.saturating_sub(r.next_id))
    })
}

/// Checks a chain from a peer: at most [`MAX_RANGES`] ranges, none
/// inverted, none containing id 0.
pub fn validate(p: &ProfileId) -> Result<(), InvalidRanges> {
    if depth(p) > MAX_RANGES {
        return Err(InvalidRanges);
    }
    if ranges(p).any(|r| r.next_id > r.last_id || r.next_id == 0) {
        return Err(InvalidRanges);
    }
    Ok(())
}

fn depth(p: &ProfileId) -> usize {
    ranges(p).count()
}

fn ranges(p: &ProfileId) -> impl Iterator<Item = &ProfileId> {
    std::iter::successors(Some(p), |r| r.next_range.as_deref())
}

/// Milliseconds from the unix epoch to 2021-01-01T00:00:00Z.
const PROFILE_ID_EPOCH_MS: i64 = 1_609_459_200_000;

/// A new profile id: 41 bits of milliseconds since 2021 above 22 random
/// bits. Ids name profile tables, so callers retry on a collision.
pub fn new_profile_id(now: Time, rng: &mut DetRng) -> u64 {
    let ms = now.unix_millis().saturating_sub(PROFILE_ID_EPOCH_MS).max(1) as u64;
    let random = rng.next_u64() & ((1 << 22) - 1);
    ((ms & ((1 << 41) - 1)) << 22) | random
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use strims_base::rng::det_rng;

    use super::*;

    fn chain(ranges: &[(u64, u64)]) -> ProfileId {
        from_ranges(ranges).unwrap()
    }

    fn flat(p: &ProfileId) -> Vec<(u64, u64)> {
        ranges(p).map(|r| (r.next_id, r.last_id)).collect()
    }

    #[test]
    fn incr_walks_the_chain() {
        let mut p = chain(&[(1, 3), (10, 12)]);
        assert_eq!(incr(&mut p, 5), Ok((1, 3)));
        assert_eq!(incr(&mut p, 1), Ok((10, 11)));
        assert_eq!(incr(&mut p, 1), Ok((11, 12)));
        assert_eq!(incr(&mut p, 1), Err(Exhausted));
    }

    #[test]
    fn pop_takes_ids_across_ranges() {
        let mut p = chain(&[(1, 3), (10, 20)]);
        let popped = pop(&mut p, 5).unwrap().unwrap();
        assert_eq!(flat(&popped), [(10, 13), (1, 3)]);
        assert_eq!(free_count(&popped), 5);
        assert_eq!(flat(&p), [(13, 20)]);

        let before = p.clone();
        assert_eq!(pop(&mut p, 8), Err(Exhausted));
        assert_eq!(p, before);
        assert_eq!(pop(&mut p, 0), Ok(None));
    }

    #[test]
    fn push_appends_and_bounds_depth() {
        let mut p = chain(&[(1, 2)]);
        push(&mut p, chain(&[(5, 6), (8, 9)])).unwrap();
        assert_eq!(flat(&p), [(1, 2), (5, 6), (8, 9)]);
        assert_eq!(free_count(&p), 3);

        let long: Vec<(u64, u64)> = (1..=MAX_RANGES as u64)
            .map(|i| (i * 10, i * 10 + 1))
            .collect();
        assert_eq!(push(&mut p, chain(&long)), Err(InvalidRanges));
        assert_eq!(flat(&p), [(1, 2), (5, 6), (8, 9)]);
    }

    #[test]
    fn free_count_saturates() {
        let p = chain(&[(1, u64::MAX), (1, u64::MAX)]);
        assert_eq!(free_count(&p), u64::MAX);
        assert_eq!(free_count(&initial_ranges()), u64::MAX - 1);
    }

    #[test]
    fn validate_rejects_malformed_chains() {
        assert_eq!(validate(&chain(&[(1, 5), (7, 7)])), Ok(()));
        assert_eq!(validate(&chain(&[(5, 1)])), Err(InvalidRanges));
        assert_eq!(validate(&chain(&[(0, 5)])), Err(InvalidRanges));
        let long: Vec<(u64, u64)> = (1..=MAX_RANGES as u64 + 1).map(|i| (i, i + 1)).collect();
        assert_eq!(validate(&chain(&long)), Err(InvalidRanges));
    }

    #[test]
    fn profile_ids_order_by_time() {
        let mut rng = det_rng(1);
        let t = Time::from_unix_millis(PROFILE_ID_EPOCH_MS + 5_000);
        let a = new_profile_id(t, &mut rng);
        let b = new_profile_id(t + strims_base::time::Duration::MILLISECOND, &mut rng);
        assert!(a < b);
        assert_eq!(a >> 22, 5_000);
        assert!(a < 1 << 63);
    }
}
