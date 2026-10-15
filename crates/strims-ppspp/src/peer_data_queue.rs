// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A peer's queue of bins to send, one per priority (pushes to stream
//! subscribers, then answers to requests).
//!
//! Peers drive it with REQUEST and CANCEL, so every operation is bounded:
//!
//! - entries pop in FIFO order (`push_front` aside), and duplicates are
//!   kept. A per-channel index counts the live entries per bin, so
//!   removing the entries a bin contains (a CANCEL) is one range query
//!   (the bins inside `b` are exactly the bin numbers in
//!   `b.base_left()..=b.base_right()`), O(log n) per removed bin. Ring
//!   entries the index no longer counts are skipped when popped.
//! - the ring is compacted whenever dead entries outnumber live ones by
//!   64, so it holds at most `2 * len + 64` entries;
//! - [`PeerDataQueue::push`] refuses entries past [`DATA_QUEUE_CAP`] live
//!   ones. A refused request is never answered; the requester times it out
//!   and asks again, from us or another peer.
//!
//! Each entry carries a time the scheduler gets back on pop: a request's
//! timestamp, or when a push was queued.

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
}

/// One priority's queue of `(channel, bin, time)` entries.
#[derive(Debug)]
pub(crate) struct PeerDataQueue<K: Copy + Ord> {
    /// Bound: `2 * len + 64` (compacted in `push`).
    ring: VecDeque<Item<K>>,
    /// The live entries per channel and bin. Bound: `len` entries.
    live: BTreeMap<K, BTreeMap<Bin, u32>>,
    /// Live entries.
    len: usize,
}

impl<K: Copy + Ord> Default for PeerDataQueue<K> {
    fn default() -> Self {
        PeerDataQueue {
            ring: VecDeque::new(),
            live: BTreeMap::new(),
            len: 0,
        }
    }
}

impl<K: Copy + Ord> PeerDataQueue<K> {
    fn count(&mut self, k: K, b: Bin) {
        *self.live.entry(k).or_default().entry(b).or_insert(0) += 1;
        self.len += 1;
    }

    /// Appends an entry; false if the queue is full.
    pub(crate) fn push(&mut self, k: K, b: Bin, t: Time) -> bool {
        if self.len >= DATA_QUEUE_CAP {
            return false;
        }
        self.ring.push_back(Item { k, b, t });
        self.count(k, b);
        self.maybe_compact();
        true
    }

    /// Prepends an entry. Used to put back (parts of) a bin
    /// that didn't fit a frame, so it is never refused: it at most restores
    /// what was popped, split into at most 62 parts.
    pub(crate) fn push_front(&mut self, k: K, b: Bin, t: Time) {
        self.ring.push_front(Item { k, b, t });
        self.count(k, b);
        self.maybe_compact();
    }

    /// Drops `k`'s entries for bins inside `b`.
    pub(crate) fn remove(&mut self, k: K, b: Bin) {
        if b.is_none() {
            return;
        }
        let Some(m) = self.live.get_mut(&k) else {
            return;
        };
        let (l, r) = (b.base_left(), b.base_right());
        if l > r {
            return;
        }
        while let Some((&bin, &n)) = m.range(l..=r).next() {
            m.remove(&bin);
            self.len -= n as usize;
        }
        if m.is_empty() {
            self.live.remove(&k);
        }
        self.maybe_clear();
        self.maybe_compact();
    }

    /// Drops all of `k`'s entries.
    pub(crate) fn remove_all(&mut self, k: K) {
        if let Some(m) = self.live.remove(&k) {
            self.len -= m.values().map(|&n| n as usize).sum::<usize>();
        }
        self.maybe_clear();
        self.maybe_compact();
    }

    /// Takes the oldest live entry.
    pub(crate) fn pop(&mut self) -> Option<(K, Bin, Time)> {
        while let Some(i) = self.ring.pop_front() {
            if let Some(m) = self.live.get_mut(&i.k)
                && let Some(n) = m.get_mut(&i.b)
            {
                *n -= 1;
                if *n == 0 {
                    m.remove(&i.b);
                    if m.is_empty() {
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

    /// Drops dead entries once they outnumber live ones. The earliest ring
    /// entries of each (channel, bin) are the live ones, as `pop` sees them.
    fn maybe_compact(&mut self) {
        if self.ring.len() <= 2 * self.len + 64 {
            return;
        }
        let mut remaining = self.live.clone();
        self.ring.retain(|i| {
            if let Some(n) = remaining.get_mut(&i.k).and_then(|m| m.get_mut(&i.b))
                && *n > 0
            {
                *n -= 1;
                return true;
            }
            false
        });
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
}
