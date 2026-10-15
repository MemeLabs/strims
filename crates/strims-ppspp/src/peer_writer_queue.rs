// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The queue of a peer's channels that have messages to write.
//!
//! A write cycle detaches the queue and works through it; channels queued
//! meanwhile form a new queue, and what the cycle didn't reach is
//! reattached behind them. A key is in the attached queue at most once, so
//! the queue is bounded by the number of the peer's channels.

use std::collections::{BTreeSet, VecDeque};

/// Channel keys in FIFO order, each at most once.
#[derive(Debug)]
pub(crate) struct PeerTaskRunnerQueue<K: Copy + Ord> {
    queue: VecDeque<K>,
    /// The keys in `queue`.
    queued: BTreeSet<K>,
}

impl<K: Copy + Ord> Default for PeerTaskRunnerQueue<K> {
    fn default() -> Self {
        PeerTaskRunnerQueue {
            queue: VecDeque::new(),
            queued: BTreeSet::new(),
        }
    }
}

/// The keys a write cycle took with [`PeerTaskRunnerQueue::detach`].
#[derive(Debug)]
pub(crate) struct DetachedQueue<K>(VecDeque<K>);

impl<K> DetachedQueue<K> {
    pub(crate) fn pop(&mut self) -> Option<K> {
        self.0.pop_front()
    }
}

impl<K: Copy + Ord> PeerTaskRunnerQueue<K> {
    /// Appends `k` unless it is already queued; true if it was added.
    pub(crate) fn push(&mut self, k: K) -> bool {
        if !self.queued.insert(k) {
            return false;
        }
        self.queue.push_back(k);
        true
    }

    pub(crate) fn remove(&mut self, k: K) {
        if self.queued.remove(&k)
            && let Some(i) = self.queue.iter().position(|&x| x == k)
        {
            self.queue.remove(i);
        }
    }

    /// Takes the queued keys; keys pushed afterwards form a new queue.
    pub(crate) fn detach(&mut self) -> DetachedQueue<K> {
        self.queued.clear();
        DetachedQueue(core::mem::take(&mut self.queue))
    }

    /// Appends what is left of a detached queue, skipping keys pushed again
    /// since.
    pub(crate) fn reattach(&mut self, mut d: DetachedQueue<K>) {
        while let Some(k) = d.pop() {
            self.push(k);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop() {
        let mut q = PeerTaskRunnerQueue::default();
        for i in 0..10 {
            q.push(i);
        }
        assert!(!q.is_empty());
        let mut dq = q.detach();
        assert!(q.is_empty());
        for i in 0..10 {
            assert_eq!(dq.pop(), Some(i));
        }
        assert_eq!(dq.pop(), None);
    }

    #[test]
    fn deduplicate() {
        let mut q = PeerTaskRunnerQueue::default();
        for i in 0..10 {
            assert!(q.push(i));
            assert!(!q.push(i));
        }
        assert!(!q.is_empty());
        let mut dq = q.detach();
        assert!(q.is_empty());
        for i in 0..10 {
            assert_eq!(dq.pop(), Some(i));
        }
        assert_eq!(dq.pop(), None);
    }

    #[test]
    fn remove() {
        let mut q = PeerTaskRunnerQueue::default();
        for i in 0..10 {
            q.push(i);
        }
        q.remove(0);
        q.remove(9);
        assert!(!q.is_empty());
        let mut dq = q.detach();
        assert!(q.is_empty());
        for i in 1..9 {
            assert_eq!(dq.pop(), Some(i));
        }
        assert_eq!(dq.pop(), None);
    }

    #[test]
    fn reattach_skips_requeued() {
        let mut q = PeerTaskRunnerQueue::default();
        for i in 0..4 {
            q.push(i);
        }
        let mut dq = q.detach();
        assert_eq!(dq.pop(), Some(0));
        // 2 was pushed again while detached; 0 failed and goes back first.
        q.push(2);
        q.push(0);
        q.reattach(dq);
        let mut dq = q.detach();
        let got: Vec<_> = core::iter::from_fn(|| dq.pop()).collect();
        assert_eq!(got, vec![2, 0, 1, 3]);
    }
}
