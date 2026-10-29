// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Record framing for byte streams that readers join part-way through.
//!
//! A writer appends records to a stream, and a reader may start at any
//! offset of it, such as a live swarm's window tail. The framing lets the
//! reader find the record boundaries after its start offset and reject the
//! record it entered partway. Everything here is synchronous: encoders
//! write into caller buffers, and decoders take input in pieces of any
//! size.
//!
//! Three layouts, all wire formats that must stay byte-exact. Two share
//! one frame format:
//!
//! - zero-padded frames, at the crate root: every record starts on a new
//!   frame, and a length prefix lets the reader check it;
//! - packed frames, in [`packed`]: records back to back, at most one
//!   record end per frame. Video swarms of the fMP4 format use them.
//!
//! The third, chunked frames in [`chunked`], has frames of its own, one
//! per swarm chunk: each frame's 2-byte header gives its first record
//! boundary, and length-prefixed records follow, any number ending per
//! frame.
//!
//! # Frames
//!
//! The stream is a sequence of frames of one fixed `size`, from
//! `HEADER_LEN + 1` to [`MAX_FRAME_SIZE`] bytes. Each frame starts with a
//! [`HEADER_LEN`] byte big-endian header:
//!
//! - on a frame that holds a record end, `0x8000_0000 | end`: the top bit
//!   is the end-of-record flag ([`EOR_FLAG`] in the first byte), and `end`
//!   is the offset within the frame just past the record's last byte,
//!   header included;
//! - on every other frame, `0`.
//!
//! # Zero-padded frames
//!
//! Every record starts on a new frame, right after the header. Its bytes
//! fill frames in order, and the rest of its last frame, the one holding
//! its end, is zero. A record of `n` bytes takes `n / (size - 4) + 1`
//! frames ([`FrameEncoder`]), so a record that exactly fills its frames is
//! followed by a frame holding only the header `0x8000_0004`.
//!
//! ## Records
//!
//! A record is the payload's length as an unsigned LEB128 varint
//! ([`get_uvarint`]), then the payload ([`RecordEncoder`]).
//!
//! ## Reader contract
//!
//! [`RecordDecoder`] reads from the stream offset given to
//! [`RecordDecoder::reset`] (0 when new):
//!
//! - At a frame start, it can't tell a record's first frame from a
//!   continuation frame: both have header `0`.
//! - Inside a frame's data, 4 or more bytes past a frame start, it hasn't
//!   read that frame's header. It takes the rest of the frame and the
//!   frames up to the next end-of-record header as one record.
//! - Inside a header, 1 to 3 bytes past a frame start, it skips to the
//!   next frame.
//!
//! So the first record it assembles, the bytes up to the first
//! end-of-record header it reads, may begin inside an earlier record. A
//! record is accepted only when its varint equals the number of bytes after
//! the varint.
//!
//! On a stream of encoded records whose payloads fit `max_record`:
//!
//! - Every record whose first frame follows the frame of the first
//!   end-of-record header read is yielded whole, in order.
//! - Before that, at most one record is yielded: the first one assembled,
//!   when it passes the length check. A record that starts at the start
//!   offset, a frame start, passes. The tail of an earlier record can pass
//!   by chance, so payloads need their own validation.
//!
//! On any input:
//!
//! - A frame whose flagged `end` is below 4 or above `size` is corrupt: the
//!   record in progress is dropped and decoding resumes at the next frame.
//! - A payload longer than `max_record` is dropped at its end, and the
//!   decoder buffers at most `max_record + MAX_UVARINT_LEN` bytes.
//! - No input panics, and the work per call is linear in its input.
//!
//! # Packed frames
//!
//! The frames' data is the records back to back, so a record starts where
//! the one before it ends, in the same frame, and a frame holds at most one
//! record end. The [`packed`] module docs give the layout and its reader
//! contract.

pub mod chunked;
mod frame;
pub mod packed;
mod record;
mod uvarint;

pub use frame::{
    BufferTooSmall, EOR_FLAG, Frag, FrameDecoder, FrameEncoder, HEADER_LEN, MAX_FRAME_SIZE,
    SizeError, Start,
};
pub use record::{Decoded, DecoderMetrics, RecordDecoder, RecordEncoder};
pub use uvarint::{MAX_UVARINT_LEN, get_uvarint, put_uvarint, uvarint_len};
