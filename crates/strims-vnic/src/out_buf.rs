// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A link's bounded, preallocated outbound buffer.
//!
//! Frames are built in place: the owner fills a body slice inside the
//! buffer, vnic writes the header in front of it and encrypts the frame
//! there. The driver drains the pending bytes from the front. Space is
//! reclaimed by moving the pending bytes to the front when a frame doesn't
//! fit behind them, which is rare because drivers usually drain everything.

#[derive(Debug)]
pub(crate) struct OutBuf {
    /// Fixed size, allocated once when the peer is established.
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
}

impl OutBuf {
    pub(crate) fn new(capacity: usize) -> OutBuf {
        OutBuf {
            buf: vec![0u8; capacity].into_boxed_slice(),
            head: 0,
            tail: 0,
        }
    }

    pub(crate) fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub(crate) fn pending(&self) -> &[u8] {
        self.buf.get(self.head..self.tail).unwrap_or_default()
    }

    pub(crate) fn len(&self) -> usize {
        self.tail - self.head
    }

    /// Bytes that could be appended after compaction.
    pub(crate) fn free(&self) -> usize {
        self.capacity() - self.len()
    }

    /// Drops `n` bytes from the front. The caller checks `n <= len()`.
    pub(crate) fn advance(&mut self, n: usize) {
        self.head = (self.head + n).min(self.tail);
        if self.head == self.tail {
            self.head = 0;
            self.tail = 0;
        }
    }

    /// A writable region of `len` bytes after the pending ones, compacting
    /// if needed. `None` if fewer than `len` bytes are free.
    pub(crate) fn slot(&mut self, len: usize) -> Option<&mut [u8]> {
        if self.free() < len {
            return None;
        }
        if self.capacity() - self.tail < len {
            self.buf.copy_within(self.head..self.tail, 0);
            self.tail -= self.head;
            self.head = 0;
        }
        self.buf.get_mut(self.tail..self.tail + len)
    }

    /// Makes the first `n` bytes of the last [`OutBuf::slot`] pending.
    pub(crate) fn commit(&mut self, n: usize) {
        self.tail = (self.tail + n).min(self.capacity());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn compacts_and_bounds() {
        let mut b = OutBuf::new(10);
        b.slot(6).unwrap().copy_from_slice(b"abcdef");
        b.commit(6);
        assert!(b.slot(5).is_none());
        b.advance(4);
        assert_eq!(b.pending(), b"ef");
        // 8 free but only 4 at the tail: compacts.
        b.slot(8).unwrap()[..3].copy_from_slice(b"ghi");
        b.commit(3);
        assert_eq!(b.pending(), b"efghi");
        b.advance(5);
        assert_eq!(b.len(), 0);
        assert_eq!(b.free(), 10);
    }
}
