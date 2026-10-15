// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A min-heap of bins by time: a channel's request (and push) deadlines.
//!
//! Ties between equal times pop in push order, so runs are deterministic.

use core::cmp::Reverse;
use std::collections::BinaryHeap;

use strims_base::time::Time;
use strims_binmap::Bin;

/// Bins by deadline. The owner bounds its length (see
/// [`BinQueue::len`]); the peer scheduler stops requesting at
/// [`max_pending_requests`](crate::scheduler_peer::max_pending_requests).
#[derive(Debug, Default)]
pub(crate) struct BinQueue {
    heap: BinaryHeap<Reverse<(Time, u64, Bin)>>,
    seq: u64,
}

impl BinQueue {
    pub(crate) fn push(&mut self, b: Bin, t: Time) {
        self.seq = self.seq.wrapping_add(1);
        self.heap.push(Reverse((t, self.seq, b)));
    }

    /// Pops the earliest bin whose time is at or before `t`.
    pub(crate) fn pop_le(&mut self, t: Time) -> Option<Bin> {
        let Reverse((bt, _, _)) = self.heap.peek()?;
        if *bt > t {
            return None;
        }
        self.heap.pop().map(|Reverse((_, _, b))| b)
    }

    pub(crate) fn len(&self) -> usize {
        self.heap.len()
    }

    /// The earliest time in the queue.
    pub(crate) fn peek_time(&self) -> Option<Time> {
        self.heap.peek().map(|Reverse((t, _, _))| *t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_queue() {
        let mut q = BinQueue::default();
        for i in 0..=40 {
            q.push(Bin(i), Time::from_unix_nanos(i as i64));
        }

        let mut b = 0;
        for i in [20, 40] {
            while let Some(v) = q.pop_le(Time::from_unix_nanos(i)) {
                assert_eq!(Bin(b), v);
                b += 1;
            }
            assert_eq!(b, i as u64 + 1);
        }

        assert_eq!(q.pop_le(Time::from_unix_nanos(100)), None);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn ties_pop_in_push_order() {
        let mut q = BinQueue::default();
        let t = Time::from_unix_millis(5);
        for b in [8, 2, 6, 4] {
            q.push(Bin(b), t);
        }
        let got: Vec<_> = core::iter::from_fn(|| q.pop_le(t)).collect();
        assert_eq!(got, vec![Bin(8), Bin(2), Bin(6), Bin(4)]);
    }
}
