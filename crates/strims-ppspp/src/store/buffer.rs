// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The live-window chunk buffer and its readers.
//!
//! [`Buffer`] holds the live window: the last `size` chunks before `head`.
//! With the circular layout the chunk at base offset `o` lives at
//! `(o & mask) * chunk_size`, and the whole window is allocated when the
//! buffer is created. The elastic layout grows by 4/3 up to the window and
//! never wraps. Nothing allocates per chunk. `next` is the first chunk not
//! yet received after the read position; everything from the tail up to
//! `next` is readable.
//!
//! The bin map stays bounded: whenever the head advances, everything before
//! the window collapses into one filled prefix, so the map tracks at most a
//! window of gaps.
//!
//! With a [`ChunkCodec`] ([`ChunkBytes::Codec`]) the buffer keeps no bytes,
//! only a ring of the codec's few bytes of state per chunk beside the bin
//! map (nothing at all when the state is empty): a chunk's bytes are made
//! again from its offset and state whenever it is sent, read or exported.
//! Everything else (the window, `next`, the readers and their errors) is
//! the same either way.
//!
//! A [`BufferReader`] is a cursor that is passed the buffer on every call and
//! never blocks: it returns data, [`Error::WouldBlock`], or the sticky
//! underrun, reset and close errors. Errors reach readers as broadcasts: each
//! one bumps a sequence number and replaces any broadcast not yet seen, and a
//! reader that hasn't seen the latest one picks up its error on its next
//! call. The buffer also counts every change a reader could observe
//! ([`Buffer::readable_seq`]) so a driver knows when to retry reads.
//!
//! # Moving back
//!
//! Everything from the tail up to `next` is stored, so a reader can move
//! back to any chunk from the tail up to its position
//! ([`BufferReader::rewind`]) and read on from there; no other reader moves.
//!
//! Chunks before the tail come back as a *backlog* (`open_backlog`):
//! chunks `[from, to)`, `to` being the tail, that the buffer stores as they
//! arrive while the tail, `next` and every reader stay where they were. A
//! chunk's ring slot is shared with the chunk a window on, so a backlog
//! fits only while nothing is stored from `from` plus a window on: a chunk
//! that arrives there closes it. Once all of `[from, to)` is stored the
//! window starts at `from` and readers may move back into it.

use core::fmt;
use std::sync::Arc;

use strims_base::time::Time;
use strims_binmap::{Bin, Map};

use super::{Error, RewindError};
use crate::codec::{Data, Message, MessageWriter};

/// The buffer layout: a `u8`, with 0 unset, so any value from options
/// can be represented.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferLayout(pub u8);

impl BufferLayout {
    /// A ring of the whole window, allocated up front.
    pub const CIRCULAR: BufferLayout = BufferLayout(1);
    /// Grows as chunks arrive, up to the window, and never wraps: chunks
    /// past the window are dropped. For swarms that mostly stay much smaller
    /// than their window, such as chat and directory assets.
    pub const ELASTIC: BufferLayout = BufferLayout(2);
}

/// What a [`Buffer`] keeps of the chunks it holds. Local to the node:
/// peers see the same data either way.
#[derive(Clone, Debug, Default)]
pub enum ChunkBytes {
    /// Their bytes, in the window's allocation ([`BufferLayout`]).
    #[default]
    Stored,
    /// Which chunks it holds and each one's state, which the codec makes
    /// their bytes from whenever they are sent, read or exported.
    Codec(Arc<dyn ChunkCodec>),
}

/// Codecs are equal if they are the same codec ([`Arc::ptr_eq`]).
impl PartialEq for ChunkBytes {
    fn eq(&self, other: &ChunkBytes) -> bool {
        match (self, other) {
            (ChunkBytes::Stored, ChunkBytes::Stored) => true,
            (ChunkBytes::Codec(a), ChunkBytes::Codec(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for ChunkBytes {}

/// Turns each chunk a [`Buffer`] stores into a few bytes of state, and
/// that state back into the chunk ([`ChunkBytes::Codec`]).
///
/// For simulations, whose streams are made from a little state per chunk
/// (or from the offset alone): a node then holds that state instead of the
/// stream, and as long as [`decode`](ChunkCodec::decode) gives back the
/// bytes [`encode`](ChunkCodec::encode) was given, it sends and reads what
/// storing them would. Nothing checks that it does. Offsets count bytes from
/// the stream's first (a chunk's offset is its index times the chunk size)
/// and start over when the stream does.
pub trait ChunkCodec: fmt::Debug + Send + Sync {
    /// The bytes of state per chunk; 0 if a chunk's bytes follow from its
    /// offset alone, and then [`encode`](ChunkCodec::encode) is never
    /// called.
    fn state_len(&self) -> usize;

    /// Writes into `state` ([`state_len`](ChunkCodec::state_len) bytes)
    /// what [`decode`](ChunkCodec::decode) needs to make `chunk`, the chunk
    /// at `offset`, again.
    fn encode(&self, offset: u64, chunk: &[u8], state: &mut [u8]);

    /// Writes into `out`, one chunk long, the bytes of the chunk at
    /// `offset` whose state is `state`.
    fn decode(&self, offset: u64, state: &[u8], out: &mut [u8]);
}

/// The byte offset of bin `b`'s base chunk.
fn bin_byte(b: Bin, chunk_size: u64) -> u64 {
    (b.0 / 2).wrapping_mul(chunk_size)
}

/// The bin at byte offset `n`; a base bin at chunk boundaries.
fn byte_bin(n: u64, chunk_size: u64) -> Bin {
    Bin(n.wrapping_mul(2) / chunk_size)
}

/// The live window of a swarm's chunks.
#[derive(Debug)]
pub struct Buffer {
    chunk_size: u64,
    mask: u64,
    /// The window in bins.
    size: Bin,
    head: Bin,
    /// Bound: chunks before the window are collapsed into one filled
    /// prefix, so the map tracks at most a window of gaps.
    pub(super) bins: Map,
    /// Bound: `window * chunk_size` bytes; empty with a codec.
    pub(super) buf: Vec<u8>,
    bytes: ChunkBytes,
    /// The codec's state of each chunk, laid out as `buf` lays out the
    /// chunks. Bound: `window * state_len` bytes.
    pub(super) states: Vec<u8>,
    state_len: usize,
    layout: BufferLayout,
    window_bytes: usize,
    ready: bool,
    next: Bin,
    sem: u64,
    /// The current error, which new and resynced readers start with.
    err: Option<Error>,
    /// The number of error broadcasts, and the last one's error.
    signal_seq: u64,
    signal: Option<Error>,
    readable_seq: u64,
    /// The backlog being stored: base bins `[from, to)` before the window.
    /// Bound: the ring's slots of the chunks from `from` plus a window on,
    /// none of which is stored while it lasts.
    backlog: Option<(Bin, Bin)>,
}

impl Buffer {
    /// A circular buffer of `size` chunks.
    pub fn new(size: usize, chunk_size: usize) -> Result<Buffer, Error> {
        Buffer::with_layout(size, chunk_size, BufferLayout::CIRCULAR)
    }

    /// A buffer of `size` chunks with `layout`. `size` must be a power of
    /// two.
    pub fn with_layout(
        size: usize,
        chunk_size: usize,
        layout: BufferLayout,
    ) -> Result<Buffer, Error> {
        Buffer::with_chunk_bytes(size, chunk_size, layout, ChunkBytes::Stored)
    }

    /// A buffer of `size` chunks with `layout` that keeps `bytes`. `size`
    /// must be a power of two. With a codec, the ring of states takes the
    /// place of the window's bytes.
    pub fn with_chunk_bytes(
        size: usize,
        chunk_size: usize,
        layout: BufferLayout,
        bytes: ChunkBytes,
    ) -> Result<Buffer, Error> {
        if size == 0 || !size.is_power_of_two() || chunk_size == 0 {
            return Err(Error::InvalidSize);
        }
        let window_bytes = size.checked_mul(chunk_size).ok_or(Error::InvalidSize)?;
        let state_len = match &bytes {
            ChunkBytes::Stored => 0,
            ChunkBytes::Codec(c) => c.state_len(),
        };
        let window_states = size.checked_mul(state_len).ok_or(Error::InvalidSize)?;
        let (buf, states) = match (layout, &bytes) {
            (BufferLayout::CIRCULAR, ChunkBytes::Codec(_)) => (Vec::new(), vec![0; window_states]),
            (BufferLayout::ELASTIC, ChunkBytes::Codec(_)) => (Vec::new(), Vec::new()),
            (BufferLayout::CIRCULAR, ChunkBytes::Stored) => (vec![0; window_bytes], Vec::new()),
            (BufferLayout::ELASTIC, ChunkBytes::Stored) => {
                (vec![0; chunk_size.max(1024).min(window_bytes)], Vec::new())
            }
            _ => return Err(Error::UnsupportedLayout),
        };
        let size = Bin((size as u64).checked_mul(2).ok_or(Error::InvalidSize)?);
        Ok(Buffer {
            chunk_size: chunk_size as u64,
            mask: size.0 / 2 - 1,
            size,
            head: size,
            bins: Map::new(),
            buf,
            bytes,
            states,
            state_len,
            layout,
            window_bytes,
            ready: false,
            next: Bin::NONE,
            sem: 0,
            err: None,
            signal_seq: 0,
            signal: None,
            readable_seq: 0,
            backlog: None,
        })
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size as usize
    }

    /// The window in chunks.
    pub fn window(&self) -> u64 {
        self.size.0 / 2
    }

    /// Counts the changes readers can observe: new data, errors, resets,
    /// recovery and readiness. A driver that sees it change retries its
    /// readers.
    pub fn readable_seq(&self) -> u64 {
        self.readable_seq
    }

    /// True once a read offset is set ([`Buffer::set_offset`], a cache
    /// import or close).
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// The first bin of the window, whether or not a read offset is set.
    pub(crate) fn window_start(&self) -> Bin {
        Bin(self.head.0.saturating_sub(self.size.0))
    }

    /// The first bin the buffer stores: the window's start, or the
    /// backlog's while one is open.
    pub(crate) fn store_from(&self) -> Bin {
        match self.backlog {
            Some((from, _)) => from.min(self.window_start()),
            None => self.window_start(),
        }
    }

    /// The backlog being stored for a rewind
    /// ([`Runner::rewind`](crate::Runner::rewind)): base bins `[from, to)`
    /// before the window.
    pub fn backlog(&self) -> Option<(Bin, Bin)> {
        self.backlog
    }

    /// Opens a backlog from base bin `from` up to the tail, or extends the
    /// open one down to `from`. Its chunks `had` holds count as stored:
    /// they were stored before the window moved past them, and their slots
    /// still hold them while the backlog fits. A backlog `had` fills is
    /// complete at once.
    ///
    /// Fails as [`Buffer::backlog_fits`] does. Costs O(runs of `had` in
    /// the backlog).
    pub(crate) fn open_backlog(&mut self, from: Bin, had: &Map) -> Result<(), RewindError> {
        let tail = self.tail();
        let from = from.base_left();
        if !tail.is_none() && from >= tail {
            return Ok(());
        }
        self.backlog_fits(from)?;
        let (lim, to) = match self.backlog {
            Some((f, _)) if from >= f => return Ok(()),
            Some((f, t)) => (f, t),
            None => (tail, tail),
        };
        self.bins.reset_before(lim);
        self.bins.fill_before(from);
        crate::bin_range::copy_filled(&mut self.bins, had, from, lim);
        self.backlog = Some((from, to));
        self.complete_backlog();
        Ok(())
    }

    /// Whether a backlog from base bin `from` fits the ring: nothing is
    /// stored from `from` plus a window on (always so once the window has
    /// slid past the first read position), and the layout moves its
    /// window. [`Error::WouldBlock`] before the first read offset.
    pub(crate) fn backlog_fits(&self, from: Bin) -> Result<(), RewindError> {
        if self.tail().is_none() {
            return Err(RewindError::Read(Error::WouldBlock));
        }
        if self.layout != BufferLayout::CIRCULAR {
            return Err(RewindError::OutsideWindow);
        }
        // the prefix before the window is filled, so a `from` a window or
        // more behind the tail finds it too
        let shared = Bin(from.base_left().0.saturating_add(self.size.0));
        if self.bins.find_filled_after(shared).is_none() {
            Ok(())
        } else {
            Err(RewindError::OutsideWindow)
        }
    }

    /// Forgets the backlog: its chunks count as before the window again.
    pub(crate) fn close_backlog(&mut self) {
        if self.backlog.take().is_some() {
            self.bins.fill_before(self.window_start());
            self.readable_seq += 1;
        }
    }

    /// Once every chunk of the backlog is stored, starts the window at its
    /// first: nothing is stored from there plus a window on.
    fn complete_backlog(&mut self) {
        let Some((from, to)) = self.backlog else {
            return;
        };
        let gap = self.bins.find_empty_after(from);
        if gap.is_none() || gap >= to {
            self.head = Bin(from.0.saturating_add(self.size.0));
            self.backlog = None;
            self.readable_seq += 1;
        }
    }

    /// Drops everything and waits for a new offset. Readers get
    /// [`Error::StreamReset`].
    pub fn reset(&mut self) {
        self.backlog = None;
        if self.bins.is_empty() {
            return;
        }

        self.head = self.size;
        self.bins = Map::new();
        self.next = Bin(0);
        self.sem += 1;
        self.ready = false;

        self.swap_readable(Some(Error::StreamReset));
        self.push_readable();
    }

    /// Stores the data of `b`. Chunks before the window are ignored; `b`
    /// must fit in the window and `d` must be exactly its chunks.
    pub fn consume(&mut self, b: Bin, d: &[u8]) -> Result<(), Error> {
        if !crate::peer_bin_ok(b, self.window()) {
            return Err(Error::BinOutOfBounds);
        }
        let len = b.base_length().saturating_mul(self.chunk_size);
        if d.len() as u64 != len {
            return Err(Error::DataLength);
        }
        self.set(b, d);
        Ok(())
    }

    /// Closes the buffer: readers get [`Error::Closed`].
    pub fn close(&mut self) {
        self.close_backlog();
        self.swap_readable(Some(Error::Closed));
        self.set_ready();
    }

    fn set_ready(&mut self) {
        if !self.ready {
            self.ready = true;
            self.readable_seq += 1;
        }
    }

    /// Broadcasts `err`: every reader's pending signal becomes `err`.
    fn swap_readable(&mut self, err: Option<Error>) {
        self.err = err;
        self.signal = err;
        self.signal_seq += 1;
        self.readable_seq += 1;
    }

    /// Wakes readers without replacing a pending error.
    fn push_readable(&mut self) {
        self.err = None;
        self.readable_seq += 1;
    }

    fn set(&mut self, b: Bin, d: &[u8]) {
        let (l, r) = b.base();
        // a chunk in the slot of a backlog chunk: the backlog stops fitting
        if let Some((from, _)) = self.backlog
            && r >= Bin(from.0.saturating_add(self.size.0))
        {
            self.close_backlog();
        }
        if l < self.store_from() {
            return;
        }
        if self.layout == BufferLayout::ELASTIC {
            // the layout never wraps: a chunk at or past the window would
            // land over chunk 0
            if r >= self.size {
                return;
            }
            let n = usize::try_from((r.base_offset() + 1) * self.chunk_size).unwrap_or(usize::MAX);
            if n > self.buf.len() && self.bytes == ChunkBytes::Stored {
                let grown = n.max(self.buf.len() * 4 / 3).min(self.window_bytes);
                self.buf.resize(grown, 0);
            }
        }
        if self.head < r + 2 {
            self.head = r + 2;
            self.bins.fill_before(self.window_start());
        }

        if self.bytes == ChunkBytes::Stored {
            let i = self.index(b);
            if let Some(dst) = self.buf.get_mut(i..) {
                let n = dst.len().min(d.len());
                if let (Some(dst), Some(src)) = (dst.get_mut(..n), d.get(..n)) {
                    dst.copy_from_slice(src);
                }
            }
        } else {
            self.encode(b.base_offset(), d);
        }
        self.bins.set(b);
        if self.backlog.is_some_and(|(_, to)| l < to) {
            self.complete_backlog();
        }

        if self.next < self.tail() {
            self.swap_readable(Some(Error::BufferUnderrun));
            return;
        }

        if b.contains(self.next) {
            self.next = self.find_empty_after(self.next);
            self.push_readable();
        }
    }

    /// Stores the codec's state of each chunk of `d`, whose first chunk is
    /// at base offset `o`.
    fn encode(&mut self, o: u64, d: &[u8]) {
        let ChunkBytes::Codec(codec) = &self.bytes else {
            return;
        };
        if self.state_len == 0 {
            return;
        }
        let codec = Arc::clone(codec);
        for (k, chunk) in d.chunks_exact(self.chunk_size as usize).enumerate() {
            let o = o + k as u64;
            let off = o.wrapping_mul(self.chunk_size);
            if let Some(state) = self.state_mut(o) {
                codec.encode(off, chunk, state);
            }
        }
    }

    /// The state of the chunk at base offset `o`; an elastic ring grows to
    /// hold it.
    fn state_mut(&mut self, o: u64) -> Option<&mut [u8]> {
        let i = self.state_index(o);
        let end = i.saturating_add(self.state_len);
        if end > self.states.len() && self.layout == BufferLayout::ELASTIC {
            let full = (self.window() as usize).saturating_mul(self.state_len);
            let grown = end.max(self.states.len() * 4 / 3).min(full);
            self.states.resize(grown, 0);
        }
        self.states.get_mut(i..end)
    }

    /// The state of the chunk at base offset `o`: empty if the codec keeps
    /// none.
    fn state(&self, o: u64) -> &[u8] {
        let i = self.state_index(o);
        self.states
            .get(i..i.saturating_add(self.state_len))
            .unwrap_or_default()
    }

    fn state_index(&self, o: u64) -> usize {
        ((o & self.mask) as usize).saturating_mul(self.state_len)
    }

    /// Makes the bytes from offset `off` on into `out` with `codec`, a
    /// chunk at a time; a chunk `out` takes only part of is made whole
    /// first. Every chunk must be stored.
    fn decode(&self, codec: &dyn ChunkCodec, off: u64, out: &mut [u8]) {
        let cs = self.chunk_size;
        let mut whole = Vec::new();
        let (mut pos, mut done) = (off, 0);
        while done < out.len() {
            let (o, skip) = (pos / cs, (pos % cs) as usize);
            let take = (cs as usize - skip).min(out.len() - done);
            let Some(dst) = out.get_mut(done..done + take) else {
                return;
            };
            if take == cs as usize {
                codec.decode(o * cs, self.state(o), dst);
            } else {
                whole.resize(cs as usize, 0);
                codec.decode(o * cs, self.state(o), &mut whole);
                if let Some(src) = whole.get(skip..skip + take) {
                    dst.copy_from_slice(src);
                }
            }
            pos += take as u64;
            done += take;
        }
    }

    /// The first missing chunk after `b`. A full map has none, so it is the
    /// first chunk past the map's root.
    fn find_empty_after(&self, b: Bin) -> Bin {
        let next = self.bins.find_empty_after(b);
        if next.is_none() {
            return self.bins.root_bin().base_right() + 2;
        }
        next
    }

    /// Writes a `Data` message for `b`, which must be stored and in the
    /// window.
    pub fn write_data(&self, b: Bin, t: Time, w: &mut impl MessageWriter) -> Result<usize, Error> {
        if !self.contains(b) {
            return Err(Error::BinDataNotSet);
        }
        let n =
            usize::try_from(b.base_length() * self.chunk_size).map_err(|_| Error::BinDataNotSet)?;
        let generated;
        let data = match &self.bytes {
            ChunkBytes::Stored => {
                let i = self.index(b);
                self.buf
                    .get(i..i.saturating_add(n))
                    .ok_or(Error::BinDataNotSet)?
            }
            ChunkBytes::Codec(codec) => {
                let mut d = vec![0; n];
                self.decode(
                    codec.as_ref(),
                    b.base_offset().wrapping_mul(self.chunk_size),
                    &mut d,
                );
                generated = d;
                &generated
            }
        };
        Ok(w.write(&Message::Data(Data {
            bin: b,
            timestamp: t,
            data,
        }))?)
    }

    /// Sets the read offset to the start of the run of stored chunks that
    /// contains `b` (or `b` itself if it isn't stored), and `next` to the
    /// first missing chunk after `b`.
    ///
    /// The walk back over stored chunks stops at the tail, so the window
    /// never moves back over chunks the ring has overwritten.
    pub fn set_offset(&mut self, b: Bin) {
        self.backlog = None;
        self.sem += 1;

        let tail = self.window_start();
        let mut b = b.base_left();
        let next = self.find_empty_after(b);
        let mut prev = next;
        while b >= tail && self.bins.filled_at(b) {
            b = self.bins.cover(b).base_left().max(tail);
            prev = b;
            b = b.layer_left();
        }

        self.next = next;
        self.head = prev + self.size.0;
        self.bins.fill_before(prev);
        self.set_ready();
    }

    /// After an underrun, moves the window to the first stored chunk after
    /// the tail.
    fn recover(&mut self) -> Result<(), Error> {
        if self.err != Some(Error::BufferUnderrun) {
            return self.err.map_or(Ok(()), Err);
        }

        let next = self.bins.find_filled_after(self.tail());
        if next.is_none() {
            return Err(Error::ReadOffsetNotFound);
        }

        // the window moves on past any backlog
        self.backlog = None;
        self.head = next + self.size.0;
        self.bins.fill_before(next);
        self.next = self.find_empty_after(next);

        self.sem += 1;
        self.swap_readable(None);
        Ok(())
    }

    /// True if every chunk of `b` is stored or before the window.
    pub(crate) fn bins_filled_at(&self, b: Bin) -> bool {
        self.bins.filled_at(b)
    }

    /// True if nothing is stored.
    pub fn is_empty(&self) -> bool {
        self.bins.is_empty()
    }

    /// The stored chunks in the window. Allocates a copy.
    pub fn bins(&self) -> Map {
        let mut b = self.bins.clone();
        b.reset_before(self.tail());
        b
    }

    fn index(&self, b: Bin) -> usize {
        ((b.base_offset() & self.mask) * self.chunk_size) as usize
    }

    /// The first bin of the window, or `NONE` before a read offset is set.
    pub fn tail(&self) -> Bin {
        if self.next.is_none() {
            return Bin::NONE;
        }
        self.window_start()
    }

    fn contains(&self, b: Bin) -> bool {
        self.tail() <= b.base_left() && b.base_right() < self.head && self.bins.filled_at(b)
    }

    /// The first missing chunk after the read offset.
    pub fn next(&self) -> Bin {
        self.next
    }

    /// Loads cached data starting at chunk 0. A trailing partial chunk is
    /// stored but not readable.
    pub fn import_cache(&mut self, b: &[u8]) -> Result<(), Error> {
        if b.len() > self.window_bytes {
            return Err(Error::CacheTooLarge);
        }
        if self.bytes == ChunkBytes::Stored {
            if self.layout == BufferLayout::ELASTIC {
                self.buf.clear();
                self.buf.resize(b.len(), 0);
            }
            if let Some(dst) = self.buf.get_mut(..b.len()) {
                dst.copy_from_slice(b);
            }
        } else {
            // a trailing partial chunk is never read, so it gets no state
            self.encode(0, b);
        }

        self.next = Bin(2 * (b.len() as u64 / self.chunk_size));
        self.bins.fill_before(self.next);
        self.set_ready();
        Ok(())
    }

    /// The chunks from 0 up to `next`, if the window still starts at 0.
    pub fn export_cache(&self) -> Result<Vec<u8>, Error> {
        if self.tail() != Bin(0) {
            return Err(Error::CacheTruncated);
        }
        if self.bins.is_empty() {
            return Err(Error::CacheEmpty);
        }
        let n = usize::try_from(bin_byte(self.next, self.chunk_size)).unwrap_or(usize::MAX);
        let mut b = vec![0; n];
        match &self.bytes {
            ChunkBytes::Stored => {
                let m = n.min(self.buf.len());
                if let (Some(dst), Some(src)) = (b.get_mut(..m), self.buf.get(..m)) {
                    dst.copy_from_slice(src);
                }
            }
            ChunkBytes::Codec(codec) => self.decode(codec.as_ref(), 0, &mut b),
        }
        Ok(b)
    }
}

/// A read position in a [`Buffer`].
///
/// Every method takes the buffer the reader was created from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferReader {
    sem: u64,
    prev: Bin,
    off: u64,
    err: Option<Error>,
    seen_signal: u64,
}

impl BufferReader {
    /// A reader at the buffer's tail.
    pub fn new(buf: &Buffer) -> BufferReader {
        BufferReader {
            sem: buf.sem,
            prev: buf.tail(),
            off: bin_byte(buf.tail(), buf.chunk_size),
            err: buf.err,
            seen_signal: buf.signal_seq,
        }
    }

    /// Picks up the latest broadcast if this reader hasn't seen it.
    fn take_signal(&mut self, buf: &Buffer) {
        if self.seen_signal != buf.signal_seq {
            self.seen_signal = buf.signal_seq;
            if self.err.is_none() {
                self.err = buf.signal;
            }
        }
    }

    /// After a reset, offset change or recovery, restarts at the tail.
    fn sync(&mut self, buf: &Buffer) {
        if self.sem != buf.sem {
            self.sem = buf.sem;
            self.prev = buf.tail();
            self.off = bin_byte(buf.tail(), buf.chunk_size);
            self.err = buf.err;
        }
    }

    /// Moves back to the tail.
    pub fn unread(&mut self, buf: &Buffer) {
        if buf.next.is_none() {
            return;
        }
        self.prev = buf.tail();
        self.off = bin_byte(buf.tail(), buf.chunk_size);
        self.seen_signal = buf.signal_seq;
    }

    /// The byte offset of the next read as of the last call, without
    /// syncing with the buffer. A read resyncs before it copies, so a read
    /// that returned `n` bytes started at `position() - n`.
    pub fn position(&self) -> u64 {
        self.off
    }

    /// The byte offset of the next read, or [`Error::WouldBlock`] until the
    /// buffer has a read offset.
    pub fn offset(&mut self, buf: &Buffer) -> Result<u64, Error> {
        if !buf.ready {
            return Err(Error::WouldBlock);
        }
        self.sync(buf);
        Ok(self.off)
    }

    /// Moves the reader back to the chunk holding stream offset `offset`
    /// and returns the chunk's offset, where the next read starts. The
    /// bytes from there come again in stream order, those read before
    /// included. No other reader moves.
    ///
    /// The chunk must be at or after the tail, which holds everything up
    /// to `next`, and `offset` at or before the reader's next byte
    /// ([`RewindError::Ahead`]); a chunk before the tail is
    /// [`RewindError::OutsideWindow`] here. A reader whose next read
    /// returns an error stays where it is and gets it as
    /// [`RewindError::Read`]: the read still returns it. Before the buffer
    /// has a read offset that is [`Error::WouldBlock`].
    pub fn rewind(&mut self, buf: &Buffer, offset: u64) -> Result<u64, RewindError> {
        if let Some(e) = self.err {
            return Err(RewindError::Read(e));
        }
        // a broadcast not seen yet stays for the next read
        if self.seen_signal != buf.signal_seq
            && let Some(e) = buf.signal
        {
            return Err(RewindError::Read(e));
        }
        if !buf.ready {
            return Err(RewindError::Read(Error::WouldBlock));
        }
        // as the next read does: its position is where that read starts
        self.sync(buf);
        if let Some(e) = self.err {
            return Err(RewindError::Read(e));
        }
        if offset > self.off {
            return Err(RewindError::Ahead);
        }
        let cs = buf.chunk_size;
        let at = offset - offset % cs;
        let b = byte_bin(at, cs);
        let tail = buf.tail();
        if tail.is_none() || b < tail {
            return Err(RewindError::OutsideWindow);
        }
        self.off = at;
        self.prev = b;
        Ok(at)
    }

    /// After [`Error::BufferUnderrun`], skips to the new tail and returns
    /// the number of bytes skipped.
    ///
    /// A reader that falls behind the window on its own (the window moved
    /// on while the buffer itself kept up) also gets an underrun and
    /// recovers by jumping to the tail, so it never reads overwritten bytes.
    pub fn recover(&mut self, buf: &mut Buffer) -> Result<u64, Error> {
        self.take_signal(buf);
        match self.err {
            None => return Ok(0),
            Some(Error::BufferUnderrun) => {}
            Some(e) => return Err(e),
        }

        if self.sem == buf.sem && buf.err == Some(Error::BufferUnderrun) {
            buf.recover()?;
        }

        let off = self.off;
        if self.sem != buf.sem {
            self.sync(buf);
        } else {
            self.prev = buf.tail();
            self.off = bin_byte(buf.tail(), buf.chunk_size);
            self.err = buf.err;
        }
        self.seen_signal = buf.signal_seq;
        Ok(self.off.wrapping_sub(off))
    }

    /// Copies readable bytes into `p`. Returns the number copied, or
    /// [`Error::WouldBlock`] if there is nothing new, or a sticky error.
    pub fn read(&mut self, buf: &Buffer, p: &mut [u8]) -> Result<usize, Error> {
        if let Some(e) = self.err {
            return Err(e);
        }
        // the pending broadcast comes first: a reset leaves the buffer's
        // own error clear, so resyncing first would hide its StreamReset
        self.take_signal(buf);
        if let Some(e) = self.err {
            return Err(e);
        }
        self.sync(buf);
        if let Some(e) = self.err {
            return Err(e);
        }

        if buf.next == self.prev || buf.next.is_none() {
            return Err(Error::WouldBlock);
        }

        let cs = buf.chunk_size;
        let tail = buf.tail();
        let base = bin_byte(tail, cs);
        if self.off < base {
            self.err = Some(Error::BufferUnderrun);
            return Err(Error::BufferUnderrun);
        }
        let l = usize::try_from(self.off - base).unwrap_or(usize::MAX);
        let h = usize::try_from(bin_byte(Bin(buf.next.0.wrapping_sub(tail.0)), cs)).unwrap_or(0);
        if l >= h {
            return Err(Error::WouldBlock);
        }

        let n = match &buf.bytes {
            ChunkBytes::Stored => {
                // the ring rotated to start at the tail
                let i = buf.index(tail).min(buf.buf.len());
                let (second, first) = buf.buf.split_at(i);
                copy_range(p, first, second, l, h)
            }
            ChunkBytes::Codec(codec) => {
                let n = (h - l).min(p.len());
                if let Some(dst) = p.get_mut(..n) {
                    buf.decode(codec.as_ref(), self.off, dst);
                }
                n
            }
        };

        self.off += n as u64;
        self.prev = byte_bin(self.off, cs);
        Ok(n)
    }

    /// True if [`BufferReader::read`] would return data or an error rather
    /// than [`Error::WouldBlock`].
    pub fn is_readable(&self, buf: &Buffer) -> bool {
        let mut r = *self;
        !matches!(r.read(buf, &mut [0u8; 1]), Err(Error::WouldBlock))
    }

    /// Later reads fail with [`Error::Closed`].
    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(e) = self.err {
            return Err(e);
        }
        self.err = Some(Error::Closed);
        Ok(())
    }
}

/// Copies bytes `l..h` of `first ‖ second` into `dst`, up to its length.
fn copy_range(dst: &mut [u8], first: &[u8], second: &[u8], l: usize, h: usize) -> usize {
    let h = h.min(first.len() + second.len());
    let want = h.saturating_sub(l).min(dst.len());
    let mut n = 0;
    let mut pos = l;
    while n < want {
        let (part, start) = if pos < first.len() {
            (first, pos)
        } else {
            (second, pos - first.len())
        };
        let m = part.len().saturating_sub(start).min(want - n);
        if m == 0 {
            break;
        }
        if let (Some(d), Some(s)) = (dst.get_mut(n..n + m), part.get(start..start + m)) {
            d.copy_from_slice(s);
        }
        n += m;
        pos += m;
    }
    n
}
