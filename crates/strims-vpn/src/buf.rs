// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Pooled byte buffers and the shared outbound message store.
//!
//! A message is marshaled once for up to five links: it goes into a
//! [`BufArena`] slot with one reference per link queue holding it, and each
//! link's `fill` copies it into its frame and drops a reference. Freed
//! buffers return to a small free list ([`BufPool`]), so steady-state
//! traffic doesn't allocate.

use slotmap::SlotMap;

slotmap::new_key_type! {
    /// A queued outbound message.
    pub(crate) struct BufId;
}

/// Free buffers. Bounded at `cap` buffers; a buffer with more capacity
/// than `max_capacity` is dropped.
#[derive(Debug)]
pub(crate) struct BufPool {
    free: Vec<Vec<u8>>,
    cap: usize,
    max_capacity: usize,
}

impl BufPool {
    pub(crate) fn new(cap: usize, max_capacity: usize) -> BufPool {
        BufPool {
            free: Vec::new(),
            cap,
            max_capacity,
        }
    }

    /// An empty buffer.
    pub(crate) fn take(&mut self) -> Vec<u8> {
        self.free.pop().unwrap_or_default()
    }

    pub(crate) fn put(&mut self, mut b: Vec<u8>) {
        if self.free.len() < self.cap && b.capacity() <= self.max_capacity && b.capacity() > 0 {
            b.clear();
            self.free.push(b);
        }
    }
}

#[derive(Debug)]
struct Shared {
    bytes: Vec<u8>,
    refs: u32,
}

/// Marshaled messages waiting in link queues. Every entry is referenced by
/// at least one link queue, so the arena is bounded by the queues' bounds.
#[derive(Debug, Default)]
pub(crate) struct BufArena {
    slots: SlotMap<BufId, Shared>,
}

impl BufArena {
    /// Stores `bytes` with `refs` references.
    pub(crate) fn insert(&mut self, bytes: Vec<u8>, refs: u32) -> BufId {
        self.slots.insert(Shared { bytes, refs })
    }

    pub(crate) fn get(&self, id: BufId) -> Option<&[u8]> {
        self.slots.get(id).map(|s| s.bytes.as_slice())
    }

    /// Drops one reference, returning the buffer once none are left.
    pub(crate) fn release(&mut self, id: BufId) -> Option<Vec<u8>> {
        let s = self.slots.get_mut(id)?;
        s.refs = s.refs.saturating_sub(1);
        if s.refs == 0 {
            return self.slots.remove(id).map(|s| s.bytes);
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn refcounts_and_pool() {
        let mut pool = BufPool::new(1, 1024);
        let mut a = BufArena::default();
        let mut b = pool.take();
        b.extend_from_slice(b"abc");
        let id = a.insert(b, 2);
        assert_eq!(a.get(id), Some(&b"abc"[..]));
        assert!(a.release(id).is_none());
        let b = a.release(id).unwrap();
        assert_eq!(a.len(), 0);
        assert!(a.release(id).is_none());
        pool.put(b);
        assert!(pool.take().is_empty());
        pool.put(Vec::with_capacity(4096));
        assert!(pool.free.is_empty());
    }
}
