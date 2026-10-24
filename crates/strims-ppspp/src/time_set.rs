// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The time a bin was first seen (bin times) or requested (request times),
//! kept as a binary tree over the bin space.
//!
//! A node's `count` is the number of base chunks under it that have a time.
//! A node whose count equals its base length is "full": a lookup that finds
//! no deeper node returns it. Setting a bin keeps the first time set for it.
//! [`Time::EPOCH`] marks a node without a time of its own.
//!
//! The tree is bounded by its owners: the peer scheduler only sets bins it
//! has validated against the live window and prunes behind the window, so
//! it holds at most a few windows of nodes plus the spine to the root (at
//! most 62 nodes). Bins at or past [`MAX_BIN`] are ignored, which bounds
//! the spine.
//!
//! [`TimeSet::prune`] drops subtrees without fixing their ancestors'
//! counts, so a node straddling the threshold can look full afterwards.
//! Its owners only prune bins they never set again, so the stale counts
//! only make lookups of pruned bins fail, as they should.
//! [`TimeSet::take_before`] drops exactly the bins before its threshold,
//! keeps the counts, and says how many chunks had a time.

use strims_base::time::Time;
use strims_binmap::Bin;

use crate::MAX_BIN;

#[derive(Debug, Default)]
pub(crate) struct TimeSet {
    root: Option<Box<Node>>,
}

#[derive(Debug)]
struct Node {
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
    bin: Bin,
    count: u64,
    time: Time,
}

impl Node {
    fn new(bin: Bin) -> Node {
        Node {
            left: None,
            right: None,
            bin,
            count: 0,
            time: Time::EPOCH,
        }
    }

    fn is_full(&self) -> bool {
        self.bin.base_length() == self.count
    }

    fn size(&self) -> usize {
        1 + self.left.as_ref().map_or(0, |n| n.size()) + self.right.as_ref().map_or(0, |n| n.size())
    }

    /// Returns the time of the node holding `b`.
    fn set(&mut self, b: Bin, t: Time) -> Time {
        if self.bin == b {
            if self.time == Time::EPOCH {
                self.count = self.bin.base_length();
                self.time = t;
            }
            return self.time;
        }

        if self.is_full() {
            return self.get(b).map_or(Time::EPOCH, |n| n.time);
        }

        let go_right = self.bin < b;
        let child_bin = if go_right {
            self.bin.right()
        } else {
            self.bin.left()
        };
        let slot = if go_right {
            &mut self.right
        } else {
            &mut self.left
        };
        match slot {
            Some(c) => self.count = self.count.wrapping_sub(c.count),
            None => *slot = Some(Box::new(Node::new(child_bin))),
        }
        let Some(child) = slot else {
            return Time::EPOCH;
        };
        let r = child.set(b, t);
        self.count = self.count.wrapping_add(child.count);
        r
    }

    /// Returns `(ok, deleted)`: whether `b` was counted under this node,
    /// and whether this node is now empty.
    fn unset(&mut self, b: Bin) -> (bool, bool) {
        let child = if self.bin < b {
            &mut self.right
        } else {
            &mut self.left
        };
        let mut ok = false;
        if let Some(c) = child {
            let (cok, deleted) = c.unset(b);
            ok = cok;
            if deleted {
                *child = None;
            }
        }

        ok = ok || self.is_full();
        if ok {
            self.count = self.count.wrapping_sub(b.base_length());
            if self.count == 0 {
                return (true, true);
            }
        }
        (ok, false)
    }

    fn get(&self, b: Bin) -> Option<&Node> {
        if !self.bin.contains(b) {
            return None;
        }
        if self.bin == b {
            return Some(self);
        }
        let child = if self.bin < b {
            &self.right
        } else {
            &self.left
        };
        let n = child.as_ref().and_then(|c| c.get(b));
        if n.is_none() && self.is_full() {
            return Some(self);
        }
        n
    }

    /// Base chunks with a time under this node.
    fn timed(&self) -> u64 {
        if self.time != Time::EPOCH {
            return self.bin.base_length();
        }
        self.left.as_ref().map_or(0, |n| n.timed()) + self.right.as_ref().map_or(0, |n| n.timed())
    }

    fn prune(&mut self, b: Bin) {
        if self.bin < b {
            self.left = None;
            if let Some(r) = &mut self.right {
                r.prune(b);
            }
        } else if let Some(l) = &mut self.left {
            l.prune(b);
        }
    }
}

impl TimeSet {
    /// The number of nodes.
    pub(crate) fn size(&self) -> usize {
        self.root.as_ref().map_or(0, |n| n.size())
    }

    /// Records `time` for `bin` unless it already has one, and returns the
    /// time it has. Bins at or past [`MAX_BIN`] are ignored.
    pub(crate) fn set(&mut self, bin: Bin, time: Time) -> Time {
        if bin >= MAX_BIN {
            return Time::EPOCH;
        }
        let mut root = self.root.take().unwrap_or_else(|| Box::new(Node::new(bin)));
        while !root.bin.contains(bin) {
            let mut r = Box::new(Node::new(root.bin.parent()));
            r.count = root.count;
            if r.bin < root.bin {
                r.right = Some(root);
            } else {
                r.left = Some(root);
            }
            root = r;
        }
        let t = root.set(bin, time);
        self.root = Some(root);
        t
    }

    /// Removes the time of `bin`. A bin outside every node is ignored, so
    /// the counts of the nodes on its path stay consistent.
    pub(crate) fn unset(&mut self, bin: Bin) {
        if self.root.as_ref().and_then(|r| r.get(bin)).is_none() {
            return;
        }
        if let Some(root) = &mut self.root
            && root.unset(bin).1
        {
            self.root = None;
        }
    }

    /// The bin holding `bin`'s time and the time.
    pub(crate) fn get(&self, bin: Bin) -> Option<(Bin, Time)> {
        let n = self.root.as_ref()?.get(bin)?;
        if n.time == Time::EPOCH {
            return None;
        }
        Some((n.bin, n.time))
    }

    /// Drops the times of every bin wholly before base bin `bin` and
    /// returns how many base chunks had one. A node whose own time covers
    /// chunks on both sides stays. The counts stay exact.
    pub(crate) fn take_before(&mut self, bin: Bin) -> u64 {
        fn take(slot: &mut Option<Box<Node>>, b: Bin) -> u64 {
            let Some(n) = slot else {
                return 0;
            };
            if n.bin.base_right() < b {
                let t = n.timed();
                *slot = None;
                return t;
            }
            if n.bin.base_left() >= b || n.time != Time::EPOCH {
                return 0;
            }
            let t = take(&mut n.left, b) + take(&mut n.right, b);
            n.count =
                n.left.as_ref().map_or(0, |c| c.count) + n.right.as_ref().map_or(0, |c| c.count);
            if n.count == 0 && n.left.is_none() && n.right.is_none() {
                *slot = None;
            }
            t
        }
        take(&mut self.root, bin)
    }

    /// Drops the times of (roughly) the bins before `bin`.
    pub(crate) fn prune(&mut self, bin: Bin) {
        while let Some(root) = self.root.take() {
            if root.bin < bin {
                self.root = root.right;
            } else {
                self.root = Some(root);
                break;
            }
        }
        if let Some(root) = &mut self.root {
            root.prune(bin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(v: i64) -> Time {
        Time::from_unix_nanos(v)
    }

    fn assert_get(s: &TimeSet, b: u64, expected: i64, ok: bool) {
        let got = s.get(Bin(b));
        assert_eq!(
            got.map(|(_, t)| t).unwrap_or(Time::EPOCH),
            ts(expected),
            "value mismatch at bin {b}"
        );
        assert_eq!(got.is_some(), ok, "ok mismatch at bin {b}");
    }

    #[test]
    fn time_set_get() {
        let mut s = TimeSet::default();
        s.set(Bin(1), ts(100));
        s.set(Bin(5), ts(200));
        s.set(Bin(7), ts(300));
        s.set(Bin(15), ts(400));

        for (bin, value, ok) in [
            (0, 100, true),
            (4, 200, true),
            (5, 200, true),
            (7, 300, true),
            (9, 300, true),
            (15, 400, true),
            (19, 400, true),
            (31, 0, false),
            (32, 0, false),
        ] {
            assert_get(&s, bin, value, ok);
        }
    }

    #[test]
    fn time_set_unset() {
        let mut s = TimeSet::default();
        s.set(Bin(3), ts(100));
        s.set(Bin(9), ts(200));
        s.set(Bin(23), ts(300));

        s.unset(Bin(9));
        assert_get(&s, 2, 100, true);
        assert_get(&s, 10, 0, false);
        assert_get(&s, 25, 300, true);

        s.set(Bin(11), ts(400));
        assert_get(&s, 2, 100, true);
        assert_get(&s, 10, 400, true);
        assert_get(&s, 25, 300, true);

        s.unset(Bin(7));
        assert_get(&s, 2, 0, false);
        assert_get(&s, 10, 0, false);
        assert_get(&s, 25, 300, true);
    }

    #[test]
    fn time_set_prune() {
        let mut s = TimeSet::default();
        s.set(Bin(3), ts(100));
        s.set(Bin(9), ts(200));
        s.set(Bin(23), ts(300));

        s.prune(Bin(16));
        assert_get(&s, 2, 0, false);
        assert_get(&s, 10, 0, false);
        assert_get(&s, 25, 300, true);
    }

    #[test]
    fn take_before_counts_exactly_what_it_drops() {
        let mut s = TimeSet::default();
        for b in [0, 2, 6, 8, 12] {
            s.set(Bin(b), ts(b as i64 + 1));
        }
        // Chunks 0, 1 and 3 lie before chunk 4 (bin 8).
        assert_eq!(s.take_before(Bin(8)), 3);
        for b in [0, 2, 6] {
            assert_eq!(s.get(Bin(b)), None);
        }
        assert_eq!(s.get(Bin(8)), Some((Bin(8), ts(9))));
        assert_eq!(s.get(Bin(12)), Some((Bin(12), ts(13))));
        assert_eq!(s.take_before(Bin(8)), 0);
        // The counts stay exact: an unset and a set after the take behave.
        s.unset(Bin(8));
        assert_eq!(s.get(Bin(8)), None);
        assert_eq!(s.take_before(Bin(14)), 1);
        assert_eq!(s.size(), 0);
    }

    #[test]
    fn first_time_wins() {
        let mut s = TimeSet::default();
        assert_eq!(s.set(Bin(4), ts(10)), ts(10));
        assert_eq!(s.set(Bin(4), ts(20)), ts(10));
    }

    #[test]
    fn unset_of_absent_bin_keeps_counts() {
        // Subtracting an absent bin from the nodes on its path would make a
        // full node look partial and lose its time.
        let mut s = TimeSet::default();
        s.set(Bin(0), ts(1));
        s.set(Bin(2), ts(2));
        s.unset(Bin(100));
        assert_get(&s, 0, 1, true);
        assert_get(&s, 2, 2, true);
        assert_get(&s, 1, 0, false);
    }

    #[test]
    fn prune_bounds_size() {
        let mut s = TimeSet::default();
        for i in 0..10_000u64 {
            s.set(Bin(i * 2), ts(i as i64 + 1));
            if i % 1024 == 1023 {
                s.prune(Bin((i * 2).saturating_sub(2048)));
            }
        }
        assert!(s.size() < 6 * 2048, "{}", s.size());
        assert_get(&s, 19_998, 10_000, true);
    }

    #[test]
    fn out_of_range_bins_ignored() {
        let mut s = TimeSet::default();
        s.set(Bin::NONE, ts(1));
        s.set(MAX_BIN, ts(1));
        assert_eq!(s.size(), 0);
        s.set(Bin(MAX_BIN.0 - 2), ts(1));
        assert!(s.size() <= 64);
    }
}
