// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Fixed-size frames that carry records of any length (see the crate docs
//! for the layout).

/// Bytes at the start of every frame.
pub const HEADER_LEN: usize = 4;

/// Set in a header's first byte on a record's last frame.
pub const EOR_FLAG: u8 = 0x80;

/// The largest frame size. Every end offset then fits in the header's low
/// 31 bits.
pub const MAX_FRAME_SIZE: usize = (u32::MAX >> 1) as usize - HEADER_LEN;

/// [`EOR_FLAG`] as a bit of the big-endian header.
pub(crate) const EOR_BIT: u32 = (EOR_FLAG as u32) << 24;

/// A frame size outside `HEADER_LEN + 1 ..= MAX_FRAME_SIZE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("frame size {0} is outside {min}..={max}", min = HEADER_LEN + 1, max = MAX_FRAME_SIZE)]
pub struct SizeError(pub usize);

/// The output buffer can't hold the encoded frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("output buffer is too small for the encoded frames")]
pub struct BufferTooSmall;

pub(crate) fn check_size(size: usize) -> Result<usize, SizeError> {
    if (HEADER_LEN + 1..=MAX_FRAME_SIZE).contains(&size) {
        Ok(size)
    } else {
        Err(SizeError(size))
    }
}

/// Splits records into frames of one size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameEncoder {
    size: usize,
}

impl FrameEncoder {
    /// An encoder for frames of `size` bytes.
    pub fn new(size: usize) -> Result<Self, SizeError> {
        Ok(Self {
            size: check_size(size)?,
        })
    }

    /// The frame size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The bytes [`encode`](Self::encode) writes for a record of
    /// `record_len` bytes: `(record_len / (size - 4) + 1) * size`. `None`
    /// when that overflows `usize`.
    pub fn encoded_len(&self, record_len: usize) -> Option<usize> {
        (record_len / (self.size - HEADER_LEN))
            .checked_add(1)?
            .checked_mul(self.size)
    }

    /// Writes the record made of `parts`, in order, as whole frames to the
    /// start of `out`, padding included. Returns the bytes written.
    pub fn encode(&self, parts: &[&[u8]], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let record_len = parts
            .iter()
            .try_fold(0usize, |n, p| n.checked_add(p.len()))
            .ok_or(BufferTooSmall)?;
        let len = self.encoded_len(record_len).ok_or(BufferTooSmall)?;
        let out = out.get_mut(..len).ok_or(BufferTooSmall)?;
        let frames = len / self.size;
        let mut src = Parts::new(parts);
        for (i, frame) in out.chunks_mut(self.size).enumerate() {
            let Some((header, data)) = frame.split_at_mut_checked(HEADER_LEN) else {
                continue;
            };
            let n = src.fill(data);
            let value = if i + 1 == frames {
                // invariant: n < size <= MAX_FRAME_SIZE, so the end fits in
                // 31 bits.
                EOR_BIT | (HEADER_LEN + n) as u32
            } else {
                0
            };
            header.copy_from_slice(&value.to_be_bytes());
            if let Some(padding) = data.get_mut(n..) {
                padding.fill(0);
            }
        }
        Ok(len)
    }
}

/// A read cursor over the concatenation of byte slices.
struct Parts<'a> {
    rest: core::slice::Iter<'a, &'a [u8]>,
    current: &'a [u8],
}

impl<'a> Parts<'a> {
    fn new(parts: &'a [&'a [u8]]) -> Self {
        Self {
            rest: parts.iter(),
            current: &[],
        }
    }

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

/// Where a decoder's start offset falls within its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// At a frame start.
    Frame,
    /// Inside a header: the rest of that frame is skipped.
    Header,
    /// Inside a frame's data: the bytes up to the next end-of-record header
    /// are taken as one record, which began before the start offset.
    Data,
}

/// A piece of the frame stream, from [`FrameDecoder::decode`] or
/// [`PackedDecoder::decode`](crate::packed::PackedDecoder::decode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frag<'a> {
    /// Bytes of the current record.
    Data(&'a [u8]),
    /// The current record ended.
    End,
    /// The frame's header flags an end outside `4..=size`: the current
    /// record is lost, and the rest of the frame is skipped.
    Corrupt,
}

/// What follows a record end in its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AfterEnd {
    /// Padding: the next record starts on the next frame.
    Padding,
    /// The next record's bytes.
    Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Reading the header; `pos` of its bytes are in `header`.
    Header,
    /// Record bytes up to `end`; `eor` when the record ends there.
    Data { end: usize, eor: bool },
    /// Discarding the rest of the frame.
    Skip,
}

/// Splits a frame stream into record fragments. Input may arrive in pieces
/// of any size.
#[derive(Debug, Clone)]
pub struct FrameDecoder {
    size: usize,
    /// Offset within the current frame.
    pos: usize,
    state: State,
    header: [u8; HEADER_LEN],
    after_end: AfterEnd,
}

impl FrameDecoder {
    /// A decoder for frames of `size` bytes, starting at stream offset 0.
    pub fn new(size: usize) -> Result<Self, SizeError> {
        Self::with_after_end(size, AfterEnd::Padding)
    }

    pub(crate) fn with_after_end(size: usize, after_end: AfterEnd) -> Result<Self, SizeError> {
        Ok(Self {
            size: check_size(size)?,
            pos: 0,
            state: State::Header,
            header: [0; HEADER_LEN],
            after_end,
        })
    }

    /// The frame size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Restarts at stream offset `offset`: the next input byte is the
    /// stream's byte there.
    pub fn reset(&mut self, offset: u64) -> Start {
        // The remainder is below `size`, which is a usize.
        self.pos = (offset % self.size as u64) as usize;
        let (state, start) = match self.pos {
            0 => (State::Header, Start::Frame),
            pos if pos < HEADER_LEN => (State::Skip, Start::Header),
            _ => (
                State::Data {
                    end: self.size,
                    eor: false,
                },
                Start::Data,
            ),
        };
        self.state = state;
        start
    }

    /// Reads `input` up to the next fragment. Returns the bytes consumed and
    /// the fragment; `None` once `input` is used up. Headers and padding are
    /// consumed without a fragment.
    pub fn decode<'a>(&mut self, input: &'a [u8]) -> (usize, Option<Frag<'a>>) {
        let mut used = 0;
        loop {
            let rest = input.get(used..).unwrap_or_default();
            match self.state {
                State::Header => {
                    let take = HEADER_LEN.saturating_sub(self.pos).min(rest.len());
                    if let (Some(dst), Some(src)) = (
                        self.header.get_mut(self.pos..self.pos + take),
                        rest.get(..take),
                    ) {
                        dst.copy_from_slice(src);
                    }
                    self.pos += take;
                    used += take;
                    if self.pos < HEADER_LEN {
                        return (used, None);
                    }
                    let header = u32::from_be_bytes(self.header);
                    if header & EOR_BIT == 0 {
                        // The other bits of a continuation header carry
                        // nothing.
                        self.state = State::Data {
                            end: self.size,
                            eor: false,
                        };
                        continue;
                    }
                    let end = (header & !EOR_BIT) as usize;
                    if !(HEADER_LEN..=self.size).contains(&end) {
                        self.state = State::Skip;
                        return (used, Some(Frag::Corrupt));
                    }
                    self.state = State::Data { end, eor: true };
                }
                State::Data { end, eor } => {
                    if self.pos >= end {
                        if eor {
                            self.state = match self.after_end {
                                AfterEnd::Padding => State::Skip,
                                AfterEnd::Record => State::Data {
                                    end: self.size,
                                    eor: false,
                                },
                            };
                            return (used, Some(Frag::End));
                        }
                        self.next_frame();
                        continue;
                    }
                    if rest.is_empty() {
                        return (used, None);
                    }
                    let take = (end - self.pos).min(rest.len());
                    self.pos += take;
                    used += take;
                    return (used, Some(Frag::Data(rest.get(..take).unwrap_or_default())));
                }
                State::Skip => {
                    if self.pos >= self.size {
                        self.next_frame();
                        continue;
                    }
                    if rest.is_empty() {
                        return (used, None);
                    }
                    let take = (self.size - self.pos).min(rest.len());
                    self.pos += take;
                    used += take;
                }
            }
        }
    }

    fn next_frame(&mut self) {
        self.pos = 0;
        self.state = State::Header;
    }
}

#[cfg(test)]
#[path = "frame_test.rs"]
mod tests;
