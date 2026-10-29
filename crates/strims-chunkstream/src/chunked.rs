// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chunked frames: one frame per swarm chunk, any number of record ends
//! per frame.
//!
//! Records far smaller than a chunk, such as a stream's access units, cost
//! a few bytes each, and a reader resyncs at every chunk. It is a wire
//! format and must stay byte-exact.
//!
//! # Layout
//!
//! The stream is a sequence of frames of one fixed `size`, from
//! [`MIN_FRAME_SIZE`] to [`MAX_FRAME_SIZE`] bytes, equal to the swarm's
//! chunk size, so frame `i` is chunk `i`: stream offsets `i * size` up to
//! `(i + 1) * size`. Each frame starts with a [`HEADER_LEN`] byte
//! big-endian header `h`:
//!
//! - [`PADDING_FRAME`] (`0`): a padding frame. It holds no record bytes,
//!   and its other bytes carry nothing. An all-zero frame is one.
//! - [`NO_BOUNDARY`] (`1`): the frame's data, offsets `2..size`, is all
//!   inside one record, which began in an earlier frame and doesn't end
//!   before the frame's end.
//! - `2..size`: the offset within the frame of its first boundary.
//! - `size..=0xffff`: invalid.
//!
//! A boundary is a place where a record length is due: stream offset 2,
//! right after each record's last byte, and offset 2 of the frame after
//! padding. A boundary at a frame's end falls at offset 2 of the next
//! frame, whose header is then `2`.
//!
//! At a boundary, a zero byte is padding to the frame's end. Any other
//! byte begins a record: its payload length `L >= 1` as a minimal unsigned
//! LEB128 varint of at most [`MAX_UVARINT_LEN`] bytes, then the `L`
//! payload bytes. A record's bytes, its length included, run on through
//! the data of the frames after it, past their headers. A record's offset
//! is the stream offset of its length's first byte. A record can't be
//! empty: a zero length reads as padding.
//!
//! # Writer
//!
//! [`ChunkedEncoder`] writes each record whole, so every byte it writes is
//! final: a frame's header is known once the length of the record that
//! reaches the frame is. A header may name a boundary that holds padding.
//!
//! A writer ends the frame in progress with [`ChunkedEncoder::end_frame`],
//! which pads it to its end, so the stream written is a whole number of
//! chunks. [`ChunkedEncoder::continue_at`] then moves the encoder to a
//! later frame start without writing the frames between: they read as
//! padding frames when they arrive as zero bytes, and as a gap otherwise.
//! No record crosses them.
//!
//! # Reader contract
//!
//! [`ChunkedDecoder`] yields records as [`Piece`]s, borrowed from its
//! input; [`ChunkedRecordDecoder`] assembles them, holding at most
//! `max_record` bytes. A decoder reads from the stream offset given to
//! `reset` (0 when new), and its input may arrive in pieces of any size.
//!
//! - From offset 0, it is in sync: it knows where every boundary falls.
//! - From another frame start, it joins: it reads the frame's header and
//!   goes to the frame's first boundary. It yields exactly the records whose
//!   offset is at or after the frame start; the record it entered partway
//!   is dropped. From inside a frame, it joins at the next frame start.
//! - To start at a known record offset `K`, a reader resets at
//!   `K - K % size`, the start of `K`'s frame, and drops the records whose
//!   offset is below `K`.
//!
//! In sync, every header must agree with the records before it: `2` or
//! `0` at a boundary, and inside a record the offset just past its end, or
//! `1` when that isn't before the frame's end. When a record's length
//! crosses into a frame, its header is checked once the length is read. A
//! frame is corrupt when its header disagrees or is invalid, or a length
//! isn't a minimal varint that fits a `u64`. The decoder then drops the
//! record in progress, skips the rest of the frame, and joins at the next
//! one.
//!
//! A padding frame drops the record in progress, if any, and the decoder
//! joins at the next frame. A record longer than `max_record` is skipped,
//! and the decoder stays in sync.
//!
//! [`Piece::Lost`] (and [`Item::Lost`]) marks a place where records may be
//! missing: a record cut short, a corrupt frame, a record skipped for its
//! length, and a join after a padding frame that skips record bytes. It
//! comes at most once between two records. Neither the bytes skipped by a
//! `reset`'s join nor padding frames yield one.
//!
//! Loss detection rests on the input: a reader passes every chunk it has
//! in stream order, resets the decoder at the next offset it reads after
//! a chunk it lost, and passes all-zero bytes only for chunks that carry
//! no data. A lost chunk passed as zeros reads as a padding frame, which
//! loses the records that cross it, but a record that lay wholly inside it
//! goes missing unseen.
//!
//! On any input, the decoders don't panic, the work per call is linear in
//! its input, and stream offsets saturate at `u64::MAX`.

use crate::frame::BufferTooSmall;
use crate::uvarint::{MAX_UVARINT_LEN, put_uvarint, uvarint_len};

/// Bytes at the start of every frame.
pub const HEADER_LEN: usize = 2;

/// The header of a frame that holds no record bytes.
pub const PADDING_FRAME: u16 = 0;

/// The header of a frame whose data is all inside one record.
pub const NO_BOUNDARY: u16 = 1;

/// The smallest frame size: a header and one data byte.
pub const MIN_FRAME_SIZE: usize = HEADER_LEN + 1;

/// The largest frame size: every offset within a frame fits the header.
pub const MAX_FRAME_SIZE: usize = u16::MAX as usize + 1;

/// A frame size outside `MIN_FRAME_SIZE..=MAX_FRAME_SIZE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("frame size {0} is outside {MIN_FRAME_SIZE}..={MAX_FRAME_SIZE}")]
pub struct FrameSizeError(pub usize);

/// Why [`ChunkedEncoder::encode`] wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    /// The record is empty, and a zero length reads as padding.
    #[error("a record can't be empty")]
    Empty,
    /// The output buffer is shorter than
    /// [`encoded_len`](ChunkedEncoder::encoded_len).
    #[error("output buffer is too small for the encoded record")]
    BufferTooSmall,
    /// The record's length overflows `usize`, or its frame would end past
    /// stream offset `u64::MAX`.
    #[error("the record overflows the stream")]
    Overflow,
}

/// Why [`ChunkedEncoder::continue_at`] refused, changing nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ContinueError {
    /// A frame is in progress: [`end_frame`](ChunkedEncoder::end_frame)
    /// ends it.
    #[error("a frame is in progress")]
    FrameInProgress,
    /// The offset isn't a frame start.
    #[error("the offset isn't a frame start")]
    Unaligned,
    /// The offset is before the encoder's.
    #[error("the offset is behind the encoder")]
    Behind,
}

fn check_size(size: usize) -> Result<usize, FrameSizeError> {
    if (MIN_FRAME_SIZE..=MAX_FRAME_SIZE).contains(&size) {
        Ok(size)
    } else {
        Err(FrameSizeError(size))
    }
}

/// The offset within its frame of stream offset `offset`.
fn frame_pos(offset: u64, size: usize) -> usize {
    // The remainder is below `size`, which is a usize.
    (offset % size as u64) as usize
}

/// Writes records into chunked frames (see the [module docs](self)).
///
/// Its whole state is the frame size and the stream offset: it holds no
/// bytes, and every byte it writes is final.
///
/// A record carrying its own offset, or one that comes before it, reads it
/// from [`next_record_offset`](Self::next_record_offset), which doesn't
/// depend on the record's length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkedEncoder {
    size: usize,
    /// The stream offset of the next byte written.
    offset: u64,
}

impl ChunkedEncoder {
    /// An encoder for frames of `frame_size` bytes, at stream offset 0.
    pub fn new(frame_size: usize) -> Result<Self, FrameSizeError> {
        Ok(Self {
            size: check_size(frame_size)?,
            offset: 0,
        })
    }

    /// The frame size.
    pub fn frame_size(&self) -> usize {
        self.size
    }

    /// The stream offset of the next byte written. At a frame start, that
    /// is the frame's header.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// The offset of the next record, the stream offset of its length's
    /// first byte: [`offset`](Self::offset), past the header at a frame
    /// start.
    pub fn next_record_offset(&self) -> u64 {
        if self.pos() == 0 {
            self.offset.saturating_add(HEADER_LEN as u64)
        } else {
            self.offset
        }
    }

    /// The bytes [`encode`](Self::encode) writes for a payload of
    /// `payload_len` bytes from here, headers included. `None` when that
    /// overflows `usize`.
    pub fn encoded_len(&self, payload_len: usize) -> Option<usize> {
        let record = uvarint_len(payload_len as u64).checked_add(payload_len)?;
        let data = self.size - HEADER_LEN;
        let (head, rest) = match self.pos() {
            0 => (0, record),
            pos => {
                let room = self.size - pos;
                if record <= room {
                    return Some(record);
                }
                (room, record - room)
            }
        };
        let headers = rest.div_ceil(data).checked_mul(HEADER_LEN)?;
        head.checked_add(rest)?.checked_add(headers)
    }

    /// Writes the record whose payload is `parts`, in order, to the start
    /// of `out`. Returns the bytes written,
    /// [`encoded_len`](Self::encoded_len). On an error it writes nothing
    /// and the encoder is unchanged.
    pub fn encode(&mut self, parts: &[&[u8]], out: &mut [u8]) -> Result<usize, EncodeError> {
        let payload_len = parts
            .iter()
            .try_fold(0usize, |n, p| n.checked_add(p.len()))
            .ok_or(EncodeError::Overflow)?;
        if payload_len == 0 {
            return Err(EncodeError::Empty);
        }
        let len = self.encoded_len(payload_len).ok_or(EncodeError::Overflow)?;
        let end = self
            .offset
            .checked_add(len as u64)
            .ok_or(EncodeError::Overflow)?;
        // The frame the record ends in must end within the stream too, so
        // `end_frame` can't overflow.
        let tail = (self.size - frame_pos(end, self.size)) % self.size;
        end.checked_add(tail as u64).ok_or(EncodeError::Overflow)?;
        let out = out.get_mut(..len).ok_or(EncodeError::BufferTooSmall)?;

        let mut prefix = [0; MAX_UVARINT_LEN];
        let n = put_uvarint(&mut prefix, payload_len as u64);
        let mut src = Parts {
            rest: core::iter::once(prefix.get(..n).unwrap_or_default())
                .chain(parts.iter().copied()),
            current: &[],
        };
        let mut left = n + payload_len;
        let mut pos = self.pos();
        let mut at = 0;
        while left > 0 {
            if pos == 0 {
                // The record starts at offset 2 of its first frame. In a
                // later frame, the boundary after it is the header's.
                let header = if at == 0 {
                    HEADER_LEN
                } else if HEADER_LEN + left < self.size {
                    HEADER_LEN + left
                } else {
                    usize::from(NO_BOUNDARY)
                };
                // invariant: header < size <= MAX_FRAME_SIZE, so it fits a
                // u16.
                let bytes = (header as u16).to_be_bytes();
                if let Some(dst) = out.get_mut(at..at + HEADER_LEN) {
                    dst.copy_from_slice(&bytes);
                }
                at += HEADER_LEN;
                pos = HEADER_LEN;
            }
            let take = (self.size - pos).min(left);
            let copied = src.fill(out.get_mut(at..at + take).unwrap_or_default());
            debug_assert_eq!(copied, take);
            at += take;
            left -= take;
            pos = (pos + take) % self.size;
        }
        debug_assert_eq!(at, len);
        self.offset = end;
        Ok(len)
    }

    /// The bytes [`end_frame`](Self::end_frame) writes: the rest of the
    /// frame in progress, 0 at a frame start.
    pub fn frame_padding(&self) -> usize {
        match self.pos() {
            0 => 0,
            pos => self.size - pos,
        }
    }

    /// Ends the frame in progress, writing zeros to its end at the start
    /// of `out`. Returns the bytes written,
    /// [`frame_padding`](Self::frame_padding). The encoder is then at a
    /// frame start, and the stream written is a whole number of frames.
    pub fn end_frame(&mut self, out: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let n = self.frame_padding();
        out.get_mut(..n).ok_or(BufferTooSmall)?.fill(0);
        // `encode` checked that the frame ends within the stream.
        self.offset = self.offset.saturating_add(n as u64);
        Ok(n)
    }

    /// Moves the encoder to the frame start `offset`, at or after its own,
    /// writing nothing: the frames between are left out of the stream. It
    /// must be at a frame start.
    pub fn continue_at(&mut self, offset: u64) -> Result<(), ContinueError> {
        if self.pos() != 0 {
            return Err(ContinueError::FrameInProgress);
        }
        if frame_pos(offset, self.size) != 0 {
            return Err(ContinueError::Unaligned);
        }
        if offset < self.offset {
            return Err(ContinueError::Behind);
        }
        self.offset = offset;
        Ok(())
    }

    fn pos(&self) -> usize {
        frame_pos(self.offset, self.size)
    }
}

/// A read cursor over the concatenation of byte slices.
struct Parts<'a, I> {
    rest: I,
    current: &'a [u8],
}

impl<'a, I: Iterator<Item = &'a [u8]>> Parts<'a, I> {
    /// Copies the next bytes into `out` until either runs out. Returns the
    /// bytes copied.
    fn fill(&mut self, out: &mut [u8]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.current.is_empty() {
                match self.rest.next() {
                    Some(part) => self.current = part,
                    None => break,
                }
                continue;
            }
            let take = (out.len() - n).min(self.current.len());
            let (head, tail) = self.current.split_at(take);
            if let Some(dst) = out.get_mut(n..n + take) {
                dst.copy_from_slice(head);
            }
            self.current = tail;
            n += take;
        }
        n
    }
}

/// A piece of a chunked stream, from [`ChunkedDecoder::decode`].
///
/// A record is a `Start`, then `Data` pieces holding its `len` payload
/// bytes in order, then `End`. A `Lost` after a `Start` and before its
/// `End` drops that record: its pieces are no record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece<'a> {
    /// A record begins at stream offset `offset`, with a payload of `len`
    /// bytes, from 1 to `max_record`.
    Start { offset: u64, len: usize },
    /// Payload bytes of the record, never empty.
    Data(&'a [u8]),
    /// The record's payload is whole.
    End,
    /// Records may be missing here (see the [module docs](self)).
    Lost,
}

/// Counters of a chunked decoder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChunkedMetrics {
    /// Records yielded whole.
    pub records: u64,
    /// Records begun and not ended: cut by a padding frame, a corrupt
    /// frame or a reset.
    pub lost: u64,
    /// Records skipped for a length above `max_record`.
    pub oversized: u64,
    /// Frames skipped for an invalid header, a header that disagrees with
    /// the records before it, or a malformed length.
    pub corrupt: u64,
}

/// What a frame continues from the frame before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Carry {
    /// It starts at a boundary: its header is `2` or `0`.
    Boundary,
    /// Nothing known: its header gives the first boundary. `report` when
    /// skipping record bytes there means records were lost.
    Join { report: bool },
    /// A length that crossed the frame's start.
    Length,
    /// A payload, of which `left` bytes remain.
    Payload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Reading the frame's header; `pos` of its bytes are in `header`.
    Header,
    /// Discarding the rest of the frame, then joining at the next one.
    Skip { report: bool },
    /// Discarding up to the frame's first boundary at `to`.
    Seek { to: usize },
    /// Padding to the frame's end.
    Pad,
    /// At a boundary. At the frame's end, the boundary is the next frame's.
    Boundary,
    /// Reading a record's length.
    Length,
    /// Reading a record's payload.
    Payload,
}

/// Splits a chunked stream into [`Piece`]s, borrowed from its input (see
/// the [module docs](self) for the reader contract). Input may arrive in
/// pieces of any size. It holds no record bytes.
///
/// To start at a known record offset `K`, [`reset`](Self::reset) at
/// `K - K % frame_size` and drop the records whose offset is below `K`.
#[derive(Debug, Clone)]
pub struct ChunkedDecoder {
    size: usize,
    max: usize,
    /// The stream offset of the current frame's start.
    frame: u64,
    /// Offset within the current frame.
    pos: usize,
    state: State,
    carry: Carry,
    header: [u8; HEADER_LEN],
    /// The header of a frame a length crossed into, checked once the
    /// length is read.
    pending: Option<u16>,
    /// The record in progress: its offset, and its length read so far.
    record: u64,
    length: u64,
    length_bytes: usize,
    /// Payload bytes left, and whether they are yielded or skipped.
    left: u64,
    keep: bool,
    /// A `Start` was yielded and its `End` not yet.
    started: bool,
    /// A `Lost` was yielded since the last `Start`.
    reported: bool,
    metrics: ChunkedMetrics,
}

impl ChunkedDecoder {
    /// A decoder for frames of `frame_size` bytes and payloads of at most
    /// `max_record` bytes, starting at stream offset 0.
    pub fn new(frame_size: usize, max_record: usize) -> Result<Self, FrameSizeError> {
        let mut d = Self {
            size: check_size(frame_size)?,
            max: max_record,
            frame: 0,
            pos: 0,
            state: State::Header,
            carry: Carry::Boundary,
            header: [0; HEADER_LEN],
            pending: None,
            record: 0,
            length: 0,
            length_bytes: 0,
            left: 0,
            keep: false,
            started: false,
            reported: false,
            metrics: ChunkedMetrics::default(),
        };
        d.reset(0);
        Ok(d)
    }

    /// The frame size.
    pub fn frame_size(&self) -> usize {
        self.size
    }

    /// The largest payload yielded.
    pub fn max_record(&self) -> usize {
        self.max
    }

    /// The decoder's counters.
    pub fn metrics(&self) -> &ChunkedMetrics {
        &self.metrics
    }

    /// Restarts at stream offset `offset`: the next input byte is the
    /// stream's byte there. The record in progress is lost, and the caller
    /// drops its pieces. From 0 the decoder is in sync; from anywhere else
    /// it joins at the first frame start at or after `offset`.
    pub fn reset(&mut self, offset: u64) {
        if self.started {
            self.metrics.lost += 1;
        }
        self.started = false;
        self.reported = false;
        self.pending = None;
        self.pos = frame_pos(offset, self.size);
        self.frame = offset - self.pos as u64;
        (self.state, self.carry) = match (offset, self.pos) {
            (0, _) => (State::Header, Carry::Boundary),
            (_, 0) => (State::Header, Carry::Join { report: false }),
            _ => (State::Skip { report: false }, Carry::Join { report: false }),
        };
    }

    /// Reads `input` up to the next piece. Returns the bytes consumed and
    /// the piece; `None` once `input` is used up. Headers, lengths, padding
    /// and skipped bytes are consumed without a piece.
    pub fn decode<'a>(&mut self, input: &'a [u8]) -> (usize, Option<Piece<'a>>) {
        let mut used = 0;
        loop {
            let rest = input.get(used..).unwrap_or_default();
            let room = self.size - self.pos;
            match self.state {
                State::Header => {
                    let Some(&b) = rest.first() else {
                        return (used, None);
                    };
                    if let Some(h) = self.header.get_mut(self.pos) {
                        *h = b;
                    }
                    self.pos += 1;
                    used += 1;
                    if self.pos == HEADER_LEN
                        && let Some(p) = self.on_header(u16::from_be_bytes(self.header))
                    {
                        return (used, Some(p));
                    }
                }
                State::Skip { .. } | State::Pad => {
                    if room == 0 {
                        self.next_frame();
                        continue;
                    }
                    if rest.is_empty() {
                        return (used, None);
                    }
                    let take = room.min(rest.len());
                    self.pos += take;
                    used += take;
                }
                State::Seek { to } => {
                    if self.pos >= to {
                        self.state = State::Boundary;
                        continue;
                    }
                    if rest.is_empty() {
                        return (used, None);
                    }
                    let take = (to - self.pos).min(rest.len());
                    self.pos += take;
                    used += take;
                }
                State::Boundary => {
                    if room == 0 {
                        self.next_frame();
                        continue;
                    }
                    let Some(&b) = rest.first() else {
                        return (used, None);
                    };
                    if b == 0 {
                        self.state = State::Pad;
                        continue;
                    }
                    self.record = self.offset();
                    self.length = 0;
                    self.length_bytes = 0;
                    self.state = State::Length;
                }
                State::Length => {
                    if room == 0 {
                        // The length crosses the frame's end, so the
                        // frame's data is inside the record.
                        if self.pending.take().is_some_and(|h| h != NO_BOUNDARY) {
                            if let Some(p) = self.corrupt() {
                                return (used, Some(p));
                            }
                            continue;
                        }
                        self.next_frame();
                        continue;
                    }
                    let Some(&b) = rest.first() else {
                        return (used, None);
                    };
                    self.pos += 1;
                    used += 1;
                    if let Some(p) = self.on_length_byte(b) {
                        return (used, Some(p));
                    }
                }
                State::Payload => {
                    if self.left == 0 {
                        self.state = State::Boundary;
                        if self.keep {
                            self.started = false;
                            self.metrics.records += 1;
                            return (used, Some(Piece::End));
                        }
                        continue;
                    }
                    if room == 0 {
                        self.next_frame();
                        continue;
                    }
                    if rest.is_empty() {
                        return (used, None);
                    }
                    let take = usize::try_from(self.left)
                        .unwrap_or(usize::MAX)
                        .min(room)
                        .min(rest.len());
                    self.pos += take;
                    used += take;
                    self.left -= take as u64;
                    if self.keep {
                        return (
                            used,
                            Some(Piece::Data(rest.get(..take).unwrap_or_default())),
                        );
                    }
                }
            }
        }
    }

    /// The stream offset of the next input byte.
    fn offset(&self) -> u64 {
        self.frame.saturating_add(self.pos as u64)
    }

    fn next_frame(&mut self) {
        self.carry = match self.state {
            State::Skip { report } => Carry::Join { report },
            State::Length => Carry::Length,
            State::Payload => Carry::Payload,
            State::Pad | State::Boundary => Carry::Boundary,
            State::Header | State::Seek { .. } => Carry::Join { report: true },
        };
        self.frame = self.frame.saturating_add(self.size as u64);
        self.pos = 0;
        self.state = State::Header;
    }

    fn on_header(&mut self, h: u16) -> Option<Piece<'static>> {
        let at = usize::from(h);
        if h != PADDING_FRAME && h != NO_BOUNDARY && !(HEADER_LEN..self.size).contains(&at) {
            return self.corrupt();
        }
        match self.carry {
            Carry::Boundary => match h {
                PADDING_FRAME => self.padding_frame(),
                _ if at == HEADER_LEN => {
                    self.state = State::Boundary;
                    None
                }
                _ => self.corrupt(),
            },
            Carry::Join { report } => match h {
                PADDING_FRAME => self.padding_frame(),
                NO_BOUNDARY => {
                    self.state = State::Skip { report };
                    report.then(|| self.lose(true)).flatten()
                }
                _ => {
                    self.state = State::Seek { to: at };
                    (report && at > HEADER_LEN)
                        .then(|| self.lose(true))
                        .flatten()
                }
            },
            Carry::Length => match h {
                PADDING_FRAME => self.padding_frame(),
                _ => {
                    self.pending = Some(h);
                    self.state = State::Length;
                    None
                }
            },
            Carry::Payload => {
                let end = self.left.saturating_add(HEADER_LEN as u64);
                let want = if end < self.size as u64 {
                    // `end` is below `size`, so it fits a u16.
                    end as u16
                } else {
                    NO_BOUNDARY
                };
                if h == want {
                    self.state = State::Payload;
                    None
                } else if h == PADDING_FRAME {
                    self.padding_frame()
                } else {
                    self.corrupt()
                }
            }
        }
    }

    fn on_length_byte(&mut self, b: u8) -> Option<Piece<'static>> {
        let last = self.length_bytes + 1 == MAX_UVARINT_LEN;
        if last && b > 1 {
            // It doesn't fit a u64.
            return self.corrupt();
        }
        self.length |= u64::from(b & 0x7f) << (7 * self.length_bytes);
        self.length_bytes += 1;
        if b & 0x80 != 0 {
            return None;
        }
        if self.length_bytes > 1 && b == 0 {
            // Not minimal.
            return self.corrupt();
        }
        if let Some(h) = self.pending.take() {
            let end = self.length.saturating_add(self.pos as u64);
            let want = if end < self.size as u64 {
                // `end` is below `size`, so it fits a u16.
                end as u16
            } else {
                NO_BOUNDARY
            };
            if h != want {
                return self.corrupt();
            }
        }
        self.left = self.length;
        self.state = State::Payload;
        if self.length > self.max as u64 {
            self.keep = false;
            self.metrics.oversized += 1;
            return self.lose(true);
        }
        self.keep = true;
        self.started = true;
        self.reported = false;
        Some(Piece::Start {
            offset: self.record,
            // The length is at most `max`, a usize.
            len: self.length as usize,
        })
    }

    /// Drops the record in progress for a padding frame, then joins at the
    /// next frame.
    fn padding_frame(&mut self) -> Option<Piece<'static>> {
        let in_record = matches!(self.carry, Carry::Length | Carry::Payload);
        self.state = State::Skip { report: true };
        self.lose(in_record)
    }

    /// Drops the record in progress and the rest of the frame, then joins
    /// at the next frame.
    fn corrupt(&mut self) -> Option<Piece<'static>> {
        self.metrics.corrupt += 1;
        self.pending = None;
        self.state = State::Skip { report: true };
        self.lose(true)
    }

    /// Drops the record in progress, and yields `Lost` for it, or for
    /// `missing` records when none was yielded since the last `Start`.
    fn lose(&mut self, missing: bool) -> Option<Piece<'static>> {
        let started = core::mem::take(&mut self.started);
        if started {
            self.metrics.lost += 1;
        }
        if started || (missing && !self.reported) {
            self.reported = true;
            Some(Piece::Lost)
        } else {
            None
        }
    }
}

/// A record or a loss, from [`ChunkedRecordDecoder::decode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item<'a> {
    /// A whole record: its stream offset and payload.
    Record { offset: u64, payload: &'a [u8] },
    /// Records may be missing here (see the [module docs](self)).
    Lost,
}

/// Reassembles the records of a chunked stream (see the
/// [module docs](self) for the reader contract). Input may arrive in
/// pieces of any size.
///
/// A record whose payload lies in one frame and one input piece is
/// borrowed from the input; any other is copied into a buffer of at most
/// `max_record` bytes, which grows only past its largest record so far.
///
/// To start at a known record offset `K`, [`reset`](Self::reset) at
/// `K - K % frame_size` and drop the records whose offset is below `K`.
#[derive(Debug, Clone)]
pub struct ChunkedRecordDecoder {
    pieces: ChunkedDecoder,
    /// The payload of the record in progress, or of the record the last
    /// call yielded. Bound: `max_record` bytes.
    buf: Vec<u8>,
    /// The record in progress: its offset and payload length.
    offset: u64,
    len: usize,
    /// `buf` holds the record the last call yielded.
    yielded: bool,
}

impl ChunkedRecordDecoder {
    /// A decoder for frames of `frame_size` bytes and payloads of at most
    /// `max_record` bytes, starting at stream offset 0.
    pub fn new(frame_size: usize, max_record: usize) -> Result<Self, FrameSizeError> {
        Ok(Self {
            pieces: ChunkedDecoder::new(frame_size, max_record)?,
            buf: Vec::new(),
            offset: 0,
            len: 0,
            yielded: false,
        })
    }

    /// The frame size.
    pub fn frame_size(&self) -> usize {
        self.pieces.frame_size()
    }

    /// The largest payload yielded.
    pub fn max_record(&self) -> usize {
        self.pieces.max_record()
    }

    /// The decoder's counters.
    pub fn metrics(&self) -> &ChunkedMetrics {
        self.pieces.metrics()
    }

    /// Restarts at stream offset `offset`, as [`ChunkedDecoder::reset`],
    /// dropping the record in progress.
    pub fn reset(&mut self, offset: u64) {
        self.pieces.reset(offset);
        self.buf.clear();
        self.yielded = false;
    }

    /// Reads `input` until a record ends or records are lost, or the input
    /// runs out. Returns the bytes consumed and the item; `None` once
    /// `input` is used up. Each call releases the record the last one
    /// yielded.
    pub fn decode<'a>(&'a mut self, input: &'a [u8]) -> (usize, Option<Item<'a>>) {
        if core::mem::take(&mut self.yielded) {
            self.buf.clear();
        }
        let mut used = 0;
        // The payload, when one input piece holds it whole.
        let mut whole: Option<&'a [u8]> = None;
        loop {
            let (n, piece) = self.pieces.decode(input.get(used..).unwrap_or_default());
            used += n;
            match piece {
                None => return (used, None),
                Some(Piece::Start { offset, len }) => {
                    self.offset = offset;
                    self.len = len;
                    self.buf.clear();
                    whole = None;
                }
                Some(Piece::Data(data)) => {
                    if self.buf.is_empty() && data.len() == self.len {
                        whole = Some(data);
                    } else {
                        self.push(data);
                    }
                }
                Some(Piece::End) => break,
                Some(Piece::Lost) => {
                    self.buf.clear();
                    return (used, Some(Item::Lost));
                }
            }
        }
        let payload = match whole {
            Some(data) => data,
            None => {
                self.yielded = true;
                &self.buf
            }
        };
        let item = Item::Record {
            offset: self.offset,
            payload,
        };
        (used, Some(item))
    }

    fn push(&mut self, data: &[u8]) {
        let needed = self.buf.len() + data.len();
        if needed > self.buf.capacity() {
            // The record's length is at most `max_record`; doubling, capped
            // at it, keeps the allocation within the bound.
            let target = self
                .len
                .max(self.buf.capacity().saturating_mul(2))
                .min(self.pieces.max_record())
                .max(needed);
            self.buf.reserve_exact(target - self.buf.len());
        }
        self.buf.extend_from_slice(data);
    }
}

#[cfg(test)]
#[path = "chunked_test.rs"]
mod tests;
