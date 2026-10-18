// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Packed frames: records back to back, at most one record end per frame.
//!
//! Video swarms carry this layout. It is a wire format and must stay
//! byte-exact.
//!
//! # Layout
//!
//! The stream is a sequence of frames of one fixed `size`, from
//! `HEADER_LEN + 1` to [`MAX_FRAME_SIZE`](crate::MAX_FRAME_SIZE) bytes
//! ([`DEFAULT_SIZE`] on video swarms). Each frame starts with a
//! [`HEADER_LEN`] byte big-endian header, as in the
//! [crate's frames](crate#frames):
//!
//! - on a frame that holds a record end, `0x8000_0000 | end`, where `end`
//!   is the offset within the frame just past the record's last byte,
//!   header included;
//! - on every other frame, `0`.
//!
//! The frames' data, the bytes after their headers, is the records back to
//! back. The first record starts at stream offset 4, and each later one
//! starts where the one before it ends, in the same frame. A header holds
//! one end offset, so a frame holds at most one record end. A record whose
//! last byte ends a frame has its end in the next frame's header,
//! `0x8000_0004`.
//!
//! # Writer
//!
//! [`PackedEncoder`] holds the frame it is filling and outputs its bytes
//! once they are final: up to the end when a record ends in the frame, and
//! the rest when the frame fills. The bytes after an end stay held until
//! their frame fills. A record that begins after an end can't end in the
//! same frame: [`PackedEncoder::end_record`] refuses,
//! [`PackedEncoder::end_blocked_after`] tells whether it would, and
//! [`PackedEncoder::abandon_record`] drops the record, none of whose bytes
//! are output yet.
//!
//! # Reader contract
//!
//! [`PackedDecoder`] reads from the stream offset given to
//! [`PackedDecoder::reset`] (0 when new):
//!
//! - At a frame start, it reads that frame's header.
//! - Inside a frame's data, 4 or more bytes past a frame start, it hasn't
//!   read that frame's header. It takes the rest of the frame as bytes of
//!   one record, which the next end it reads closes.
//! - Inside a header, 1 to 3 bytes past a frame start, it skips to the
//!   next frame.
//!
//! After an [`End`](Frag::End), the next record's bytes follow in the same
//! frame. Unless the start offset is 0, the caller drops the fragments up
//! to the first `End`: on a stream from [`PackedEncoder`], they are always
//! the tail of a record that began before the start offset. Every later
//! `End` closes a whole record, in stream order.
//!
//! On any input:
//!
//! - A frame whose flagged `end` is below 4 or above `size` is corrupt: the
//!   decoder returns [`Corrupt`](Frag::Corrupt) and skips the rest of the
//!   frame. The caller drops the record in progress and the fragments up
//!   to the next `End`, the tail of a record whose start was skipped.
//! - No input panics, the work per call is linear in its input, and the
//!   decoder holds no record bytes.

use core::fmt;

use crate::frame::{
    AfterEnd, EOR_BIT, Frag, FrameDecoder, HEADER_LEN, SizeError, Start, check_size,
};

/// The frame size on video swarms.
pub const DEFAULT_SIZE: usize = u16::MAX as usize - HEADER_LEN;

/// [`PackedEncoder::end_record`] refused: the frame already holds a record
/// end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the frame already holds a record end")]
pub struct FrameHasEnd;

/// Packs records into frames of one size (see the [module docs](self)).
///
/// Bytes go to the caller's `Vec` as they become final, so the stream it
/// collects is always a prefix of the frames.
#[derive(Clone)]
pub struct PackedEncoder {
    /// The frame being filled, `size` bytes.
    frame: Box<[u8]>,
    /// Bytes of `frame` in use, header included: `HEADER_LEN..size`.
    len: usize,
    /// The record end `frame` holds. The bytes before it are output.
    end: Option<usize>,
    /// Where the record in progress began in `frame`; `None` when it began
    /// in an earlier frame.
    record_start: Option<usize>,
    /// The stream offset of `frame`'s first byte.
    frame_offset: u64,
}

impl PackedEncoder {
    /// An encoder for frames of `size` bytes, at stream offset 0. It holds
    /// one frame.
    pub fn new(size: usize) -> Result<Self, SizeError> {
        Ok(Self {
            frame: vec![0; check_size(size)?].into_boxed_slice(),
            len: HEADER_LEN,
            end: None,
            record_start: Some(HEADER_LEN),
            frame_offset: 0,
        })
    }

    /// The frame size.
    pub fn size(&self) -> usize {
        self.frame.len()
    }

    /// The stream offset of the next byte written.
    pub fn offset(&self) -> u64 {
        self.frame_offset + self.len as u64
    }

    /// Appends `data` to the record in progress. Each frame it fills goes
    /// to `out`, from the frame's end if it holds one.
    pub fn write(&mut self, mut data: &[u8], out: &mut Vec<u8>) {
        while !data.is_empty() {
            let room = self.frame.get_mut(self.len..).unwrap_or_default();
            let (head, tail) = data.split_at(room.len().min(data.len()));
            if let Some(dst) = room.get_mut(..head.len()) {
                dst.copy_from_slice(head);
            }
            data = tail;
            self.len += head.len();
            if self.len >= self.frame.len() {
                self.next_frame(out);
            }
        }
    }

    /// Ends the record in progress, and appends the frame up to the end to
    /// `out`. Refused when the frame already holds an end; the record then
    /// continues.
    pub fn end_record(&mut self, out: &mut Vec<u8>) -> Result<(), FrameHasEnd> {
        if self.end.is_some() {
            return Err(FrameHasEnd);
        }
        // invariant: len < size <= MAX_FRAME_SIZE, so the end fits in the
        // header's low 31 bits.
        let header = EOR_BIT | self.len as u32;
        if let Some(h) = self.frame.get_mut(..HEADER_LEN) {
            h.copy_from_slice(&header.to_be_bytes());
        }
        out.extend_from_slice(self.frame.get(..self.len).unwrap_or_default());
        self.end = Some(self.len);
        self.record_start = Some(self.len);
        Ok(())
    }

    /// Whether [`end_record`](Self::end_record) would be refused after
    /// `len` more bytes: the frame holds an end, and they don't fill it.
    pub fn end_blocked_after(&self, len: usize) -> bool {
        self.end.is_some() && len < self.room()
    }

    /// The least `n >= len` for which
    /// [`end_blocked_after`](Self::end_blocked_after) is false: `len`, or
    /// the bytes that fill the frame.
    pub fn unblocking_len(&self, len: usize) -> usize {
        if self.end.is_some() {
            len.max(self.room())
        } else {
            len
        }
    }

    /// Drops the record in progress when it began in the current frame, at
    /// its end or at the stream start: none of its bytes are output. Returns
    /// false, changing nothing, when it began in an earlier frame.
    pub fn abandon_record(&mut self) -> bool {
        match self.record_start {
            Some(start) => {
                self.len = start;
                true
            }
            None => false,
        }
    }

    /// Bytes left in the frame.
    fn room(&self) -> usize {
        self.frame.len() - self.len
    }

    /// Outputs the full frame from its end, or whole with header `0`.
    fn next_frame(&mut self, out: &mut Vec<u8>) {
        let from = match self.end {
            Some(end) => end,
            None => {
                if let Some(h) = self.frame.get_mut(..HEADER_LEN) {
                    h.fill(0);
                }
                0
            }
        };
        out.extend_from_slice(self.frame.get(from..).unwrap_or_default());
        self.frame_offset += self.frame.len() as u64;
        self.len = HEADER_LEN;
        self.end = None;
        self.record_start = None;
    }
}

impl fmt::Debug for PackedEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackedEncoder")
            .field("size", &self.size())
            .field("len", &self.len)
            .field("end", &self.end)
            .field("record_start", &self.record_start)
            .field("frame_offset", &self.frame_offset)
            .finish_non_exhaustive()
    }
}

/// Splits a packed stream into record fragments (see the
/// [module docs](self) for the reader contract). Input may arrive in pieces
/// of any size.
#[derive(Debug, Clone)]
pub struct PackedDecoder {
    frames: FrameDecoder,
}

impl PackedDecoder {
    /// A decoder for frames of `size` bytes, starting at stream offset 0.
    pub fn new(size: usize) -> Result<Self, SizeError> {
        Ok(Self {
            frames: FrameDecoder::with_after_end(size, AfterEnd::Record)?,
        })
    }

    /// The frame size.
    pub fn size(&self) -> usize {
        self.frames.size()
    }

    /// Restarts at stream offset `offset`: the next input byte is the
    /// stream's byte there. Unless `offset` is 0, the fragments up to the
    /// first [`End`](Frag::End) are the tail of a record that began before
    /// it.
    pub fn reset(&mut self, offset: u64) -> Start {
        self.frames.reset(offset)
    }

    /// Reads `input` up to the next fragment. Returns the bytes consumed and
    /// the fragment; `None` once `input` is used up. Headers and skipped
    /// bytes are consumed without a fragment.
    pub fn decode<'a>(&mut self, input: &'a [u8]) -> (usize, Option<Frag<'a>>) {
        self.frames.decode(input)
    }
}

#[cfg(test)]
#[path = "packed_test.rs"]
mod tests;
