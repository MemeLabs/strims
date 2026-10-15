// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Keys ordered by deadline.
//!
//! An ordered set of `(deadline, key)`. The owner removes the old entry
//! whenever a key's deadline changes or its item goes: the hash table store
//! deletes records by key when their entry expires, so a stale entry would
//! delete a record stored again under the same key. [`TimeoutQueue::next`]
//! is the owner's `poll_timeout` deadline.

use std::collections::BTreeSet;

use strims_base::time::Time;

/// Keys ordered by deadline. Bounded by its owner's collection.
#[derive(Debug, Clone)]
pub(crate) struct TimeoutQueue<K> {
    items: BTreeSet<(Time, K)>,
}

impl<K: Ord + Copy> Default for TimeoutQueue<K> {
    fn default() -> Self {
        TimeoutQueue {
            items: BTreeSet::new(),
        }
    }
}

impl<K: Ord + Copy> TimeoutQueue<K> {
    /// Adds `key` at `deadline`.
    pub(crate) fn push(&mut self, deadline: Time, key: K) {
        self.items.insert((deadline, key));
    }

    pub(crate) fn remove(&mut self, deadline: Time, key: K) {
        self.items.remove(&(deadline, key));
    }

    /// The earliest deadline.
    pub(crate) fn next(&self) -> Option<Time> {
        self.items.first().map(|(t, _)| *t)
    }

    /// Removes and returns an item whose deadline is before `now`.
    pub(crate) fn pop_expired(&mut self, now: Time) -> Option<K> {
        let &(t, k) = self.items.first()?;
        if now.after(t) {
            self.items.pop_first();
            return Some(k);
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_deadline_order_after_expiry() {
        let mut q = TimeoutQueue::default();
        let t = |s| Time::from_unix(s, 0);
        q.push(t(30), 3u32);
        q.push(t(10), 1);
        q.push(t(20), 2);
        q.remove(t(20), 2);
        assert_eq!(q.next(), Some(t(10)));
        assert_eq!(q.pop_expired(t(10)), None);
        assert_eq!(q.pop_expired(t(31)), Some(1));
        assert_eq!(q.pop_expired(t(31)), Some(3));
        assert_eq!(q.pop_expired(t(31)), None);
        assert_eq!(q.len(), 0);
    }
}
