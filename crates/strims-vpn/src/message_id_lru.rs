// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Recently seen message ids, for dropping duplicates.
//!
//! `insert` reports whether an id is new and moves it to the back of a
//! recency list. Entries not inserted again for `ttl` are pruned, lazily on
//! insert (at most once per `ttl`) and at [`MessageIdLru::poll_timeout`].
//!
//! The table is a `BTreeMap` index over a slab-allocated doubly linked
//! list, bounded at `cap` entries: inserting into a full table evicts the
//! least recently seen id.

use std::collections::BTreeMap;

use strims_base::time::{Duration, Time};

const NIL: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
struct Slot<K> {
    key: K,
    time: Time,
    /// The fewest hops any copy had (see [`MessageIdLru::insert_hops`]).
    hops: u8,
    prev: u32,
    next: u32,
}

/// Recently seen keys. Generic so that the host can key it by (network,
/// message id).
#[derive(Debug)]
pub struct MessageIdLru<K> {
    ttl: Duration,
    cap: usize,
    /// When the next lazy prune may run; `Time::NIL` until the first insert.
    next_gc: Time,
    /// Bounded by `cap`.
    index: BTreeMap<K, u32>,
    /// Bounded by `cap`.
    slots: Vec<Slot<K>>,
    free: Vec<u32>,
    /// Least recently inserted.
    first: u32,
    /// Most recently inserted.
    last: u32,
    evictions: u64,
}

impl<K: Ord + Copy> MessageIdLru<K> {
    /// A table of at most `cap` keys, each kept for `ttl` after its last
    /// insert.
    pub fn new(cap: usize, ttl: Duration) -> MessageIdLru<K> {
        MessageIdLru {
            ttl,
            cap: cap.clamp(1, NIL as usize - 1),
            next_gc: Time::NIL,
            index: BTreeMap::new(),
            slots: Vec::new(),
            free: Vec::new(),
            first: NIL,
            last: NIL,
            evictions: 0,
        }
    }

    /// Stored keys.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Entries evicted because the table was full.
    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    /// Whether `key` is stored. Expired keys count until the next prune.
    pub fn contains(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// Records `key` at `now`. Returns true if it wasn't present.
    pub fn insert(&mut self, now: Time, key: K) -> bool {
        self.insert_hops(now, key, 0).is_none()
    }

    /// [`MessageIdLru::insert`] for a copy that took `hops` hops: `None`
    /// if the key is new, or else the fewest hops of the earlier copies
    /// (the stored minimum becomes `min(previous, hops)`).
    pub fn insert_hops(&mut self, now: Time, key: K, hops: u8) -> Option<u8> {
        if self.next_gc.is_nil() {
            self.next_gc = now + self.ttl;
        } else if now.after(self.next_gc) {
            self.next_gc = now + self.ttl;
            self.prune(now - self.ttl);
        }

        if let Some(&i) = self.index.get(&key) {
            self.unlink(i);
            let mut prev = hops;
            if let Some(s) = self.slots.get_mut(i as usize) {
                s.time = now;
                prev = s.hops;
                s.hops = s.hops.min(hops);
            }
            self.push_back(i);
            return Some(prev);
        }

        if self.index.len() >= self.cap {
            let oldest = self.first;
            if let Some(s) = self.slots.get(oldest as usize) {
                let k = s.key;
                self.unlink(oldest);
                self.index.remove(&k);
                self.free.push(oldest);
                self.evictions += 1;
            }
        }

        let slot = Slot {
            key,
            time: now,
            hops,
            prev: NIL,
            next: NIL,
        };
        let i = match self.free.pop() {
            Some(i) => {
                if let Some(s) = self.slots.get_mut(i as usize) {
                    *s = slot;
                }
                i
            }
            None => {
                // invariant: len < cap < NIL, so the index fits.
                let i = self.slots.len() as u32;
                self.slots.push(slot);
                i
            }
        };
        self.index.insert(key, i);
        self.push_back(i);
        None
    }

    /// When pruning is next due, while anything is stored.
    pub fn poll_timeout(&self) -> Option<Time> {
        (!self.is_empty() && !self.next_gc.is_nil()).then_some(self.next_gc)
    }

    /// Prunes entries older than `ttl` if a prune is due.
    pub fn handle_timeout(&mut self, now: Time) {
        if !self.next_gc.is_nil() && !now.before(self.next_gc) {
            self.next_gc = now + self.ttl;
            self.prune(now - self.ttl);
        }
    }

    /// Removes entries last inserted before `t`.
    fn prune(&mut self, t: Time) {
        while let Some(s) = self.slots.get(self.first as usize) {
            if !s.time.before(t) {
                break;
            }
            let (i, k) = (self.first, s.key);
            self.unlink(i);
            self.index.remove(&k);
            self.free.push(i);
        }
    }

    fn unlink(&mut self, i: u32) {
        let Some(&Slot { prev, next, .. }) = self.slots.get(i as usize) else {
            return;
        };
        match self.slots.get_mut(prev as usize) {
            Some(p) => p.next = next,
            None => self.first = next,
        }
        match self.slots.get_mut(next as usize) {
            Some(n) => n.prev = prev,
            None => self.last = prev,
        }
        if let Some(s) = self.slots.get_mut(i as usize) {
            s.prev = NIL;
            s.next = NIL;
        }
    }

    fn push_back(&mut self, i: u32) {
        let last = self.last;
        if let Some(s) = self.slots.get_mut(i as usize) {
            s.prev = last;
            s.next = NIL;
        }
        match self.slots.get_mut(last as usize) {
            Some(l) => l.next = i,
            None => self.first = i,
        }
        self.last = i;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use crate::message::MessageId;

    fn id(b: u8) -> MessageId {
        let mut m = MessageId([0; 34]);
        m.0[0] = b;
        m
    }

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_000_000 + ms)
    }

    #[test]
    fn lru() {
        let mut l = MessageIdLru::new(64, Duration::MILLISECOND);
        assert!(l.insert(t(0), id(0)));
        assert!(l.insert(t(0), id(1)));
        assert!(l.insert(t(0), id(2)));
        assert!(!l.insert(t(0), id(1)));
        assert!(!l.insert(t(0), id(2)));
    }

    #[test]
    fn lru_ttl() {
        let mut l = MessageIdLru::new(64, Duration::from_millis(15));
        assert!(l.insert(t(0), id(0)));
        assert!(l.insert(t(0), id(1)));

        assert!(!l.insert(t(10), id(1)));
        assert!(l.insert(t(10), id(2)));

        assert!(l.insert(t(20), id(3)));

        assert!(!l.contains(&id(0)));
        assert!(l.contains(&id(1)));
        assert!(l.contains(&id(2)));
        assert!(l.contains(&id(3)));
    }

    #[test]
    fn recency_order_and_bound() {
        let mut l = MessageIdLru::new(3, Duration::SECOND);
        for i in 0..3 {
            assert!(l.insert(t(i), id(i as u8)));
        }
        // Refresh 0, so 1 is now the oldest.
        assert!(!l.insert(t(5), id(0)));
        assert!(l.insert(t(6), id(3)));
        assert_eq!(l.evictions(), 1);
        assert!(!l.contains(&id(1)));
        assert!(l.contains(&id(0)) && l.contains(&id(2)) && l.contains(&id(3)));
        assert_eq!(l.len(), 3);
        assert_eq!(l.slots.len(), 3);
        // Evicted ids are new again.
        assert!(l.insert(t(7), id(1)));
        assert!(!l.contains(&id(2)));
    }

    #[test]
    fn fewest_hops_are_remembered() {
        let mut l = MessageIdLru::new(4, Duration::SECOND);
        assert_eq!(l.insert_hops(t(0), id(0), 4), None);
        assert_eq!(l.insert_hops(t(1), id(0), 2), Some(4));
        assert_eq!(l.insert_hops(t(2), id(0), 3), Some(2));
        assert_eq!(l.insert_hops(t(3), id(0), 1), Some(2));
        assert!(!l.insert(t(4), id(0)));
    }

    #[test]
    fn timeout_prunes_without_inserts() {
        let mut l = MessageIdLru::new(16, Duration::SECOND);
        assert_eq!(l.poll_timeout(), None);
        l.insert(t(0), id(0));
        l.insert(t(500), id(1));
        assert_eq!(l.poll_timeout(), Some(t(1000)));
        l.handle_timeout(t(999));
        assert_eq!(l.len(), 2);
        l.handle_timeout(t(1200));
        // Entry 0 is older than 1200 - 1000; entry 1 isn't.
        assert_eq!(l.len(), 1);
        assert!(l.contains(&id(1)));
        assert_eq!(l.poll_timeout(), Some(t(2200)));
        l.handle_timeout(t(2200));
        assert!(l.is_empty());
        assert_eq!(l.poll_timeout(), None);
        // Slots are reused.
        l.insert(t(3000), id(5));
        assert_eq!(l.slots.len(), 2);
    }
}
