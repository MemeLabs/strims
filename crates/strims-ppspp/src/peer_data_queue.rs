// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A peer's queue of bins to send, one per priority (pushes to stream
//! subscribers, then answers to requests).
//!
//! Peers drive it with REQUEST and CANCEL, so every operation is bounded:
//!
//! - entries pop in FIFO order (`push_front` aside), and duplicates are
//!   kept. Entries are numbered as they are queued. A per-channel index
//!   keeps each bin's live count and the number its live entries start
//!   at, so removing the entries a bin contains (a CANCEL) is one range
//!   query (the bins inside `b` are exactly the bin numbers in
//!   `b.base_left()..=b.base_right()`), O(log n) per removed bin. Ring
//!   entries the index doesn't count are dead and are skipped when
//!   popped. A removed bin that is queued again starts a new run of
//!   numbers, so its old entries stay dead.
//! - the ring is compacted whenever dead entries outnumber live ones by
//!   64, so it holds at most `2 * len + 64` entries;
//! - [`PeerDataQueue::push`] refuses entries past [`DATA_QUEUE_CAP`] live
//!   ones, and [`PeerDataQueue::push_capped`] past a channel's own cap. A
//!   refused request is never answered; the requester times it out and
//!   asks again, from us or another peer.
//! - the index counts each channel's live entries, tells whether they
//!   cover a bin ([`PeerDataQueue::covers`]) in a lookup per layer and a
//!   range query per live bin inside it, and drops a channel's entries for
//!   the bins numbered before a bin ([`PeerDataQueue::remove_before`]) in
//!   one range query, O(log n) per removed bin.
//!
//! Each entry carries a time the scheduler gets back on pop: when a
//! request arrived, or when a push was queued. The answer gate reads the
//! oldest live entry's ([`PeerDataQueue::front_time`]).

use core::ops::RangeBounds;
use std::collections::{BTreeMap, VecDeque};

use strims_base::time::Time;
use strims_binmap::Bin;

/// The most live entries a peer's queue holds per priority. An honest
/// requester has at most its congestion window of requests outstanding,
/// typically tens to a few hundred chunks.
pub const DATA_QUEUE_CAP: usize = 8192;

#[derive(Clone, Copy, Debug)]
struct Item<K> {
    k: K,
    b: Bin,
    t: Time,
    /// Queue order number.
    seq: u64,
}

/// One bin's live entries.
#[derive(Clone, Copy, Debug)]
struct Live {
    n: u32,
    /// The lowest number a live entry can have: the number of the entry
    /// queued while the bin had none.
    since: u64,
    /// That entry's time.
    t: Time,
}

/// One channel's live entries: `n` in all, by bin, none above layer `top`.
#[derive(Clone, Debug, Default)]
struct Lives {
    n: usize,
    bins: BTreeMap<Bin, Live>,
    top: u64,
}

impl Lives {
    /// Whether the live bins inside `b` cover it.
    fn cover(&self, b: Bin) -> bool {
        if self.bins.contains_key(&b) {
            return true;
        }
        if b.is_base()
            || self
                .bins
                .range(b.base_left()..=b.base_right())
                .next()
                .is_none()
        {
            return false;
        }
        self.cover(b.left()) && self.cover(b.right())
    }
}

/// One priority's queue of `(channel, bin, time)` entries.
#[derive(Debug)]
pub(crate) struct PeerDataQueue<K: Copy + Ord> {
    /// Bound: `2 * len + 64` (compacted in `push`).
    ring: VecDeque<Item<K>>,
    /// The live entries per channel and bin. Bound: `len` entries.
    /// Invariant: for each (channel, bin) here, the ring holds exactly `n`
    /// of its entries numbered `since` or later, and those are the live
    /// ones. Every other ring entry is dead. A channel's `n` sums its
    /// bins'.
    live: BTreeMap<K, Lives>,
    /// Live entries.
    len: usize,
    /// The next entry's number.
    seq: u64,
}

impl<K: Copy + Ord> Default for PeerDataQueue<K> {
    fn default() -> Self {
        PeerDataQueue {
            ring: VecDeque::new(),
            live: BTreeMap::new(),
            len: 0,
            seq: 0,
        }
    }
}

impl<K: Copy + Ord> PeerDataQueue<K> {
    /// Counts a new live entry for `(k, b)`, queued at `t`, and returns its
    /// number.
    fn count(&mut self, k: K, b: Bin, t: Time) -> u64 {
        let seq = self.seq;
        self.seq += 1;
        let c = self.live.entry(k).or_default();
        c.bins
            .entry(b)
            .or_insert(Live {
                n: 0,
                since: seq,
                t,
            })
            .n += 1;
        c.n += 1;
        c.top = c.top.max(b.layer());
        self.len += 1;
        seq
    }

    fn is_live(live: &BTreeMap<K, Lives>, i: &Item<K>) -> bool {
        live.get(&i.k)
            .and_then(|c| c.bins.get(&i.b))
            .is_some_and(|e| i.seq >= e.since)
    }

    /// Appends an entry; false if the queue is full.
    pub(crate) fn push(&mut self, k: K, b: Bin, t: Time) -> bool {
        self.push_capped(k, b, t, usize::MAX)
    }

    /// Appends an entry; false if the queue is full or `k` has `cap` live
    /// entries.
    pub(crate) fn push_capped(&mut self, k: K, b: Bin, t: Time, cap: usize) -> bool {
        if self.len >= DATA_QUEUE_CAP || self.channel_len(k) >= cap {
            return false;
        }
        let seq = self.count(k, b, t);
        self.ring.push_back(Item { k, b, t, seq });
        self.maybe_compact();
        true
    }

    /// Prepends an entry. Used to put back (parts of) a bin
    /// that didn't fit a frame, so it is never refused: it at most restores
    /// what was popped, split into at most 62 parts.
    pub(crate) fn push_front(&mut self, k: K, b: Bin, t: Time) {
        let seq = self.count(k, b, t);
        self.ring.push_front(Item { k, b, t, seq });
        self.maybe_compact();
    }

    /// Drops `k`'s entries for bins inside `b`.
    pub(crate) fn remove(&mut self, k: K, b: Bin) {
        if b.is_none() {
            return;
        }
        let (l, r) = (b.base_left(), b.base_right());
        if l > r {
            return;
        }
        if self.remove_range(k, l..=r, |_, _, _| {}).is_some() {
            self.maybe_clear();
            self.maybe_compact();
        }
    }

    /// Drops `k`'s entries for the bins numbered before `from`, each of
    /// which starts before it, passing each bin, its live entries and the
    /// time of the entry that made it live to `f`.
    pub(crate) fn remove_before(&mut self, k: K, from: Bin, f: impl FnMut(Bin, u32, Time)) {
        if self.remove_range(k, ..from, f) == Some(true) {
            self.maybe_clear();
            self.maybe_compact();
        }
    }

    /// Drops `k`'s entries for the bins numbered in `range`, passing each
    /// bin, its live entries and the time of the entry that made it live
    /// to `f`. `None` if `k` has no entries, else whether any went.
    fn remove_range(
        &mut self,
        k: K,
        range: impl RangeBounds<Bin> + Clone,
        mut f: impl FnMut(Bin, u32, Time),
    ) -> Option<bool> {
        let c = self.live.get_mut(&k)?;
        let mut removed = false;
        while let Some((&bin, &e)) = c.bins.range(range.clone()).next() {
            c.bins.remove(&bin);
            c.n -= e.n as usize;
            self.len -= e.n as usize;
            removed = true;
            f(bin, e.n, e.t);
        }
        if c.bins.is_empty() {
            self.live.remove(&k);
        }
        Some(removed)
    }

    /// Drops all of `k`'s entries.
    pub(crate) fn remove_all(&mut self, k: K) {
        if let Some(c) = self.live.remove(&k) {
            self.len -= c.n;
        }
        self.maybe_clear();
        self.maybe_compact();
    }

    /// Whether `k`'s live entries cover every chunk of `b`: an entry for
    /// `b` or a bin around it, or entries for bins inside it that cover
    /// it. Costs O(log n) per layer up to `k`'s largest bin, and per live
    /// bin inside `b` per layer of `b`.
    pub(crate) fn covers(&self, k: K, b: Bin) -> bool {
        let Some(c) = self.live.get(&k) else {
            return false;
        };
        if b.is_none() {
            return false;
        }
        let mut x = b.parent();
        while x.layer() <= c.top {
            if c.bins.contains_key(&x) {
                return true;
            }
            x = x.parent();
        }
        c.cover(b)
    }

    /// `k`'s live entries.
    pub(crate) fn channel_len(&self, k: K) -> usize {
        self.live.get(&k).map_or(0, |c| c.n)
    }

    /// The time of the oldest live entry: the front's, once the dead
    /// entries ahead of it are dropped.
    pub(crate) fn front_time(&mut self) -> Option<Time> {
        while let Some(i) = self.ring.front() {
            if Self::is_live(&self.live, i) {
                return Some(i.t);
            }
            self.ring.pop_front();
        }
        None
    }

    /// Takes the oldest live entry.
    pub(crate) fn pop(&mut self) -> Option<(K, Bin, Time)> {
        while let Some(i) = self.ring.pop_front() {
            if let Some(c) = self.live.get_mut(&i.k)
                && let Some(e) = c.bins.get_mut(&i.b)
                && i.seq >= e.since
            {
                e.n -= 1;
                c.n -= 1;
                if e.n == 0 {
                    c.bins.remove(&i.b);
                    if c.bins.is_empty() {
                        self.live.remove(&i.k);
                    }
                }
                self.len -= 1;
                self.maybe_clear();
                return Some((i.k, i.b, i.t));
            }
        }
        None
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Live entries.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Ring entries, live or dead.
    pub(crate) fn ring_len(&self) -> usize {
        self.ring.len()
    }

    fn maybe_clear(&mut self) {
        if self.len == 0 {
            self.ring.clear();
        }
    }

    /// Drops the dead entries once they outnumber live ones.
    fn maybe_compact(&mut self) {
        if self.ring.len() <= 2 * self.len + 64 {
            return;
        }
        let live = &self.live;
        self.ring.retain(|i| Self::is_live(live, i));
    }
}

#[cfg(test)]
mod tests {
    //! Queue order, removal, and the bounds.
    use super::*;

    const W: u8 = 1;

    #[test]
    fn push_pop() {
        let mut q = PeerDataQueue::default();
        for i in (0..=8).step_by(2) {
            q.push(W, Bin(i), Time::EPOCH);
        }
        assert!(!q.is_empty());
        for i in (0..=8).step_by(2) {
            assert_eq!(q.pop(), Some((W, Bin(i), Time::EPOCH)));
        }
        assert!(q.is_empty());
    }

    #[test]
    fn push_front_pop() {
        let mut q = PeerDataQueue::default();
        for i in (2..=8).step_by(2) {
            q.push_front(W, Bin(i), Time::EPOCH);
        }
        assert!(!q.is_empty());
        for i in (2..=8).rev().step_by(2) {
            assert_eq!(q.pop(), Some((W, Bin(i), Time::EPOCH)));
        }
        assert!(q.is_empty());
    }

    #[test]
    fn remove() {
        let mut q = PeerDataQueue::default();
        for i in (0..16).step_by(2) {
            q.push(W, Bin(i), Time::EPOCH);
        }
        q.remove(W, Bin(11));
        assert!(!q.is_empty());
        for i in (0..8).step_by(2) {
            assert_eq!(q.pop(), Some((W, Bin(i), Time::EPOCH)));
        }
        assert!(q.is_empty());
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn remove_only_contained_bins_of_one_channel() {
        let mut q = PeerDataQueue::default();
        q.push(1u8, Bin(3), Time::EPOCH);
        q.push(1, Bin(4), Time::EPOCH);
        q.push(2, Bin(4), Time::EPOCH);
        // bin 1 doesn't contain bin 3; bin 5 contains 4
        q.remove(1, Bin(1));
        q.remove(1, Bin(5));
        assert_eq!(q.pop(), Some((1, Bin(3), Time::EPOCH)));
        assert_eq!(q.pop(), Some((2, Bin(4), Time::EPOCH)));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn duplicates_are_kept() {
        let mut q = PeerDataQueue::default();
        q.push(W, Bin(2), Time::EPOCH);
        q.push(W, Bin(2), Time::EPOCH);
        assert_eq!(q.len(), 2);
        assert!(q.pop().is_some());
        assert!(q.pop().is_some());
        assert!(q.is_empty());
    }

    #[test]
    fn remove_all() {
        let mut q = PeerDataQueue::default();
        for i in 0..10 {
            q.push(i % 2, Bin(i as u64 * 2), Time::EPOCH);
        }
        q.remove_all(0);
        let got: Vec<_> = core::iter::from_fn(|| q.pop()).map(|(k, _, _)| k).collect();
        assert_eq!(got, vec![1; 5]);
    }

    #[test]
    fn bounded_under_request_cancel_flood() {
        let mut q = PeerDataQueue::default();
        // A REQUEST/CANCEL flood leaves dead entries behind.
        for round in 0..100_000u64 {
            q.push(W, Bin((round % 512) * 2), Time::EPOCH);
            if round % 3 != 0 {
                q.remove(W, Bin((round % 512) * 2));
            }
            assert!(
                q.ring_len() <= 2 * q.len() + 65,
                "{} {}",
                q.ring_len(),
                q.len()
            );
        }
        // More requests than the cap are refused.
        let mut refused = 0;
        for i in 0..2 * DATA_QUEUE_CAP as u64 {
            if !q.push(W, Bin(i * 2), Time::EPOCH) {
                refused += 1;
            }
        }
        assert!(refused > 0);
        assert_eq!(q.len(), DATA_QUEUE_CAP);
        // One CANCEL of a big bin is one range removal.
        q.remove(W, Bin::ALL);
        assert!(q.is_empty());
        assert_eq!(q.ring_len(), 0);
    }

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(ms)
    }

    #[test]
    fn requeued_bin_pops_in_its_new_place_with_its_new_time() {
        let mut q = PeerDataQueue::default();
        q.push(W, Bin(2), t(0));
        q.push(W, Bin(4), t(0));
        q.remove(W, Bin(2));
        q.push(W, Bin(2), t(1));
        assert_eq!(q.pop(), Some((W, Bin(4), t(0))));
        assert_eq!(q.pop(), Some((W, Bin(2), t(1))));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn requeued_bin_after_remove_all_keeps_its_new_time() {
        let mut q = PeerDataQueue::default();
        q.push(1u8, Bin(2), t(0));
        q.push(2, Bin(4), t(0));
        q.remove_all(1);
        q.push(1, Bin(2), t(1));
        assert_eq!(q.pop(), Some((2, Bin(4), t(0))));
        assert_eq!(q.pop(), Some((1, Bin(2), t(1))));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn compaction_keeps_the_requeued_entry() {
        let mut q = PeerDataQueue::default();
        // Bin 4 stays live, so the ring keeps bin 2's dead entry.
        q.push(W, Bin(4), t(0));
        q.push(W, Bin(2), t(0));
        q.remove(W, Bin(2));
        q.push(W, Bin(2), t(1));
        for i in 0..100u64 {
            let b = Bin(1000 + i * 2);
            q.push(W, b, t(2));
            q.remove(W, b);
        }
        assert!(q.ring_len() < 100, "{}", q.ring_len());
        assert_eq!(q.pop(), Some((W, Bin(4), t(0))));
        assert_eq!(q.pop(), Some((W, Bin(2), t(1))));
        assert_eq!(q.pop(), None);
    }

    mod model {
        //! Model check: random pushes, removals and pops against a plain
        //! FIFO of entries.
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::{Config, RngSeed};

        #[derive(Clone, Copy, Debug)]
        enum Op {
            Push(u8, u64),
            PushCapped(u8, u64, usize),
            PushFront(u8, u64),
            Remove(u8, u64),
            RemoveBefore(u8, u64),
            RemoveAll(u8),
            Pop,
        }

        impl Op {
            fn bin(self) -> Option<(u8, Bin)> {
                match self {
                    Op::Push(k, b)
                    | Op::PushCapped(k, b, _)
                    | Op::PushFront(k, b)
                    | Op::Remove(k, b)
                    | Op::RemoveBefore(k, b) => Some((k, Bin(b))),
                    Op::RemoveAll(_) | Op::Pop => None,
                }
            }
        }

        fn op() -> impl Strategy<Value = Op> {
            // Two channels and bins of up to layer 4 over 16 base bins, so
            // removals hit re-queued bins often.
            let k = 0u8..2;
            let b = 0u64..32;
            prop_oneof![
                4 => (k.clone(), b.clone()).prop_map(|(k, b)| Op::Push(k, b)),
                2 => (k.clone(), b.clone(), 0usize..8)
                    .prop_map(|(k, b, cap)| Op::PushCapped(k, b, cap)),
                1 => (k.clone(), b.clone()).prop_map(|(k, b)| Op::PushFront(k, b)),
                2 => (k.clone(), b.clone()).prop_map(|(k, b)| Op::Remove(k, b)),
                1 => (k.clone(), b).prop_map(|(k, b)| Op::RemoveBefore(k, b)),
                1 => k.prop_map(Op::RemoveAll),
                3 => Just(Op::Pop),
            ]
        }

        fn config() -> Config {
            Config {
                cases: 512,
                rng_seed: RngSeed::Fixed(0x7064_7175_6575),
                failure_persistence: None,
                ..Config::default()
            }
        }

        proptest! {
            #![proptest_config(config())]

            #[test]
            fn matches_a_fifo_model(ops in proptest::collection::vec(op(), 1..400)) {
                let mut q = PeerDataQueue::default();
                let mut model: VecDeque<(u8, Bin, Time)> = VecDeque::new();
                // The time of the entry that made each (channel, bin) live.
                let mut made: BTreeMap<(u8, Bin), Time> = BTreeMap::new();
                let of = |model: &VecDeque<(u8, Bin, Time)>, k: u8| {
                    model.iter().filter(|e| e.0 == k).count()
                };
                for (i, op) in ops.into_iter().enumerate() {
                    let at = t(i as i64);
                    match op {
                        Op::Push(k, b) => {
                            let ok = model.len() < DATA_QUEUE_CAP;
                            prop_assert_eq!(q.push(k, Bin(b), at), ok);
                            if ok {
                                model.push_back((k, Bin(b), at));
                            }
                        }
                        Op::PushCapped(k, b, cap) => {
                            let ok = model.len() < DATA_QUEUE_CAP && of(&model, k) < cap;
                            prop_assert_eq!(q.push_capped(k, Bin(b), at, cap), ok);
                            if ok {
                                model.push_back((k, Bin(b), at));
                            }
                        }
                        Op::PushFront(k, b) => {
                            q.push_front(k, Bin(b), at);
                            model.push_front((k, Bin(b), at));
                        }
                        Op::Remove(k, b) => {
                            q.remove(k, Bin(b));
                            model.retain(|&(mk, mb, _)| mk != k || !Bin(b).contains(mb));
                        }
                        Op::RemoveBefore(k, b) => {
                            let mut got = BTreeMap::new();
                            q.remove_before(k, Bin(b), |x, n, t| {
                                got.insert(x, (n, t));
                            });
                            let mut want = BTreeMap::new();
                            for &(mk, mb, _) in &model {
                                if mk == k && mb < Bin(b) {
                                    let t = made.get(&(mk, mb)).copied();
                                    want.entry(mb).or_insert((0, t.unwrap_or(Time::NIL))).0 += 1;
                                }
                            }
                            prop_assert_eq!(got, want);
                            model.retain(|&(mk, mb, _)| mk != k || mb >= Bin(b));
                        }
                        Op::RemoveAll(k) => {
                            q.remove_all(k);
                            model.retain(|&(mk, _, _)| mk != k);
                        }
                        Op::Pop => {
                            prop_assert_eq!(q.pop(), model.pop_front());
                        }
                    }
                    made.retain(|&(k, b), _| model.iter().any(|e| (e.0, e.1) == (k, b)));
                    for &(k, b, t) in &model {
                        made.entry((k, b)).or_insert(t);
                    }
                    prop_assert_eq!(q.len(), model.len());
                    prop_assert_eq!(q.is_empty(), model.is_empty());
                    prop_assert_eq!(q.front_time(), model.front().map(|e| e.2));
                    for k in 0..2 {
                        prop_assert_eq!(q.channel_len(k), of(&model, k));
                    }
                    if let Some((_, b)) = op.bin() {
                        for k in 0..2 {
                            let (l, r) = b.base();
                            let covered = (l.0..=r.0).step_by(2).all(|x| {
                                model.iter().any(|e| e.0 == k && e.1.contains(Bin(x)))
                            });
                            prop_assert_eq!(q.covers(k, b), covered);
                        }
                    }
                    if !matches!(op, Op::Pop) {
                        prop_assert!(q.ring_len() <= 2 * q.len() + 64);
                    }
                }
                while let Some(e) = model.pop_front() {
                    prop_assert_eq!(q.pop(), Some(e));
                }
                prop_assert_eq!(q.pop(), None);
            }
        }
    }
}
