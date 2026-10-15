// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Frames: a 4-byte header (big-endian `u16` port, big-endian `u16` body
//! length) followed by the body. Links carry a byte stream of frames.
//!
//! - [`FrameHeader`] and [`Frame`] encode and decode one frame.
//! - [`FrameDecoder`] reassembles frames from chunks split at arbitrary
//!   points, and lends frames that arrive whole straight out of the input.
//! - [`FrameWriter`] batches an owner's byte stream into frames, which the
//!   owner copies out one per egress opportunity.

use std::collections::VecDeque;

use strims_base::time::BufferTooShort;

/// Header size.
pub const FRAME_HEADER_LEN: usize = 4;

/// The largest body a header can describe.
pub const MAX_FRAME_BODY_LEN: usize = u16::MAX as usize;

/// A frame header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameHeader {
    pub port: u16,
    pub length: u16,
}

impl FrameHeader {
    pub fn to_bytes(self) -> [u8; FRAME_HEADER_LEN] {
        let [p0, p1] = self.port.to_be_bytes();
        let [l0, l1] = self.length.to_be_bytes();
        [p0, p1, l0, l1]
    }

    pub fn from_bytes(b: [u8; FRAME_HEADER_LEN]) -> FrameHeader {
        let [p0, p1, l0, l1] = b;
        FrameHeader {
            port: u16::from_be_bytes([p0, p1]),
            length: u16::from_be_bytes([l0, l1]),
        }
    }

    /// Writes the header to the start of `buf`.
    pub fn encode(self, buf: &mut [u8]) -> Result<usize, BufferTooShort> {
        let dst = buf.get_mut(..FRAME_HEADER_LEN).ok_or(BufferTooShort)?;
        dst.copy_from_slice(&self.to_bytes());
        Ok(FRAME_HEADER_LEN)
    }

    /// Reads a header from the start of `buf`.
    pub fn decode(buf: &[u8]) -> Result<(FrameHeader, usize), BufferTooShort> {
        let b: [u8; FRAME_HEADER_LEN] = buf
            .get(..FRAME_HEADER_LEN)
            .and_then(|b| b.try_into().ok())
            .ok_or(BufferTooShort)?;
        Ok((FrameHeader::from_bytes(b), FRAME_HEADER_LEN))
    }
}

/// A frame borrowed from a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame<'a> {
    pub port: u16,
    pub body: &'a [u8],
}

/// Errors from [`Frame::encode`] and [`Frame::decode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// The body is longer than a header can describe.
    #[error("frame length out of range")]
    BodyTooLong,
    #[error("buffer too short")]
    BufferTooShort,
}

impl<'a> Frame<'a> {
    /// Header plus body.
    pub fn encoded_len(&self) -> usize {
        FRAME_HEADER_LEN + self.body.len()
    }

    /// Writes header and body to the start of `buf`.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, FrameError> {
        let length = u16::try_from(self.body.len()).map_err(|_| FrameError::BodyTooLong)?;
        let n = self.encoded_len();
        let dst = buf.get_mut(..n).ok_or(FrameError::BufferTooShort)?;
        let (h, b) = dst.split_at_mut(FRAME_HEADER_LEN);
        h.copy_from_slice(
            &FrameHeader {
                port: self.port,
                length,
            }
            .to_bytes(),
        );
        b.copy_from_slice(self.body);
        Ok(n)
    }

    /// Reads one complete frame from the start of `buf`, or
    /// [`FrameError::BufferTooShort`] if it isn't all there.
    pub fn decode(buf: &'a [u8]) -> Result<(Frame<'a>, usize), FrameError> {
        let (h, _) = FrameHeader::decode(buf).map_err(|_| FrameError::BufferTooShort)?;
        let n = FRAME_HEADER_LEN + usize::from(h.length);
        let body = buf
            .get(FRAME_HEADER_LEN..n)
            .ok_or(FrameError::BufferTooShort)?;
        Ok((Frame { port: h.port, body }, n))
    }
}

/// Reassembles frames from a byte stream. Frames that lie entirely inside
/// one input chunk are lent straight from it; only a frame split across
/// chunks is copied, into a buffer of at most [`MAX_FRAME_BODY_LEN`] bytes.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    header: [u8; FRAME_HEADER_LEN],
    header_len: usize,
    /// Body of the split frame. Bounded by [`MAX_FRAME_BODY_LEN`]; its
    /// capacity is kept for reuse.
    body: Vec<u8>,
    /// Body length of the split frame once its header is complete.
    want: Option<usize>,
}

impl FrameDecoder {
    pub fn new() -> FrameDecoder {
        FrameDecoder::default()
    }

    /// Bytes held for a frame split across chunks.
    pub fn buffered(&self) -> usize {
        self.header_len + self.body.len()
    }

    /// Feeds the next chunk of the stream and calls `f` for each frame it
    /// completes, in order.
    pub fn decode(&mut self, mut data: &[u8], f: &mut impl FnMut(Frame<'_>)) {
        if self.header_len > 0 || self.want.is_some() {
            data = self.continue_split(data, f);
        }
        while let Ok((frame, n)) = Frame::decode(data) {
            f(frame);
            data = data.get(n..).unwrap_or_default();
        }
        if !data.is_empty() {
            let rest = self.continue_split(data, f);
            debug_assert!(rest.is_empty());
        }
    }

    /// Moves bytes into the split frame; returns the input left over once it
    /// completes (empty if it doesn't).
    fn continue_split<'d>(
        &mut self,
        mut data: &'d [u8],
        f: &mut impl FnMut(Frame<'_>),
    ) -> &'d [u8] {
        if self.want.is_none() {
            let need = FRAME_HEADER_LEN - self.header_len;
            let k = need.min(data.len());
            let (src, rest) = data.split_at(k);
            if let Some(dst) = self.header.get_mut(self.header_len..self.header_len + k) {
                dst.copy_from_slice(src);
            }
            self.header_len += k;
            data = rest;
            if self.header_len < FRAME_HEADER_LEN {
                return data;
            }
            let h = FrameHeader::from_bytes(self.header);
            let want = usize::from(h.length);
            self.body.clear();
            self.body.reserve(want);
            self.want = Some(want);
        }
        let want = self.want.unwrap_or(0);
        let k = (want - self.body.len()).min(data.len());
        let (src, rest) = data.split_at(k);
        self.body.extend_from_slice(src);
        if self.body.len() == want {
            let h = FrameHeader::from_bytes(self.header);
            f(Frame {
                port: h.port,
                body: &self.body,
            });
            self.body.clear();
            self.header_len = 0;
            self.want = None;
        }
        rest
    }
}

/// Errors from [`FrameWriter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FrameWriterError {
    /// The writer's buffer can't take the whole frame now.
    #[error("frame writer buffer full")]
    WouldBlock,
    /// The writer was closed.
    #[error("write on closed frameReadWriter")]
    Closed,
}

/// Owner-side batching of a byte stream into frames.
///
/// Bytes written between flushes form one frame, and a frame is cut every
/// `max_body` bytes; the golden vectors pin these cuts. An empty flush makes
/// no frame. The owner keeps its writer's QoS backlog set while
/// [`FrameWriter::has_frame`], and in [`crate::Host::poll_egress`]'s `fill`
/// returns `Fill { len: fw.fill(body), more: fw.has_frame() }`.
#[derive(Debug)]
pub struct FrameWriter {
    /// Pending bytes, bounded by `capacity`.
    data: VecDeque<u8>,
    /// Lengths of complete frames at the front of `data`. Bounded by
    /// `capacity` (every frame has at least one byte).
    frames: VecDeque<usize>,
    /// Unflushed bytes at the back of `data`, always below `max_body`.
    open: usize,
    max_body: usize,
    capacity: usize,
    closed: bool,
}

impl FrameWriter {
    /// A writer for frames of at most `max_body` body bytes (the link MTU
    /// minus [`FRAME_HEADER_LEN`]) that buffers at most `capacity` bytes.
    pub fn new(max_body: usize, capacity: usize) -> FrameWriter {
        FrameWriter {
            data: VecDeque::new(),
            frames: VecDeque::new(),
            open: 0,
            max_body: max_body.clamp(1, MAX_FRAME_BODY_LEN),
            capacity,
            closed: false,
        }
    }

    /// The largest frame body.
    pub fn max_body(&self) -> usize {
        self.max_body
    }

    /// Bytes waiting to be sent.
    pub fn buffered(&self) -> usize {
        self.data.len()
    }

    /// Bytes [`FrameWriter::write`] accepts now.
    pub fn available(&self) -> usize {
        self.capacity.saturating_sub(self.data.len())
    }

    /// Whether a complete frame is ready for [`FrameWriter::fill`].
    pub fn has_frame(&self) -> bool {
        !self.frames.is_empty()
    }

    /// Appends as much of `p` as fits and returns the count; full
    /// `max_body` frames are cut as they fill.
    pub fn write(&mut self, p: &[u8]) -> Result<usize, FrameWriterError> {
        if self.closed {
            return Err(FrameWriterError::Closed);
        }
        let n = p.len().min(self.available());
        let mut src = p.get(..n).unwrap_or_default();
        while !src.is_empty() {
            let k = (self.max_body - self.open).min(src.len());
            let (head, rest) = src.split_at(k);
            self.data.extend(head);
            self.open += k;
            src = rest;
            if self.open == self.max_body {
                self.frames.push_back(self.open);
                self.open = 0;
            }
        }
        Ok(n)
    }

    /// Ends the current frame, if it has bytes.
    pub fn flush(&mut self) -> Result<(), FrameWriterError> {
        if self.closed {
            return Err(FrameWriterError::Closed);
        }
        if self.open > 0 {
            self.frames.push_back(self.open);
            self.open = 0;
        }
        Ok(())
    }

    /// Write then flush. All or nothing: if `p` doesn't fit, nothing is
    /// written.
    pub fn write_frame(&mut self, p: &[u8]) -> Result<usize, FrameWriterError> {
        if self.closed {
            return Err(FrameWriterError::Closed);
        }
        if p.len() > self.available() {
            return Err(FrameWriterError::WouldBlock);
        }
        let n = self.write(p)?;
        self.flush()?;
        Ok(n)
    }

    /// Copies the next frame body into `buf` and returns its length, or 0 if
    /// no frame is ready. A frame longer than `buf` is split.
    pub fn fill(&mut self, buf: &mut [u8]) -> usize {
        let Some(len) = self.frames.front_mut() else {
            return 0;
        };
        let n = (*len).min(buf.len());
        for (dst, src) in buf.iter_mut().zip(self.data.drain(..n)) {
            *dst = src;
        }
        *len -= n;
        if *len == 0 {
            self.frames.pop_front();
        }
        n
    }

    /// Closes the writer. Later writes fail; buffered bytes are dropped.
    pub fn close(&mut self) {
        self.closed = true;
        self.data.clear();
        self.frames.clear();
        self.open = 0;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn header_round_trip() {
        let h = FrameHeader {
            port: 0xbeef,
            length: 0x0102,
        };
        assert_eq!(h.to_bytes(), [0xbe, 0xef, 0x01, 0x02]);
        let mut b = [0u8; 4];
        assert_eq!(h.encode(&mut b), Ok(4));
        assert_eq!(FrameHeader::decode(&b), Ok((h, 4)));
        assert_eq!(FrameHeader::decode(&b[..3]), Err(BufferTooShort));
    }

    #[test]
    fn frame_round_trip() {
        let f = Frame {
            port: 7,
            body: b"hello",
        };
        let mut b = [0u8; 16];
        assert_eq!(f.encode(&mut b), Ok(9));
        assert_eq!(&b[..9], b"\x00\x07\x00\x05hello");
        assert_eq!(Frame::decode(&b[..9]), Ok((f, 9)));
        assert_eq!(Frame::decode(&b[..8]), Err(FrameError::BufferTooShort));
        assert_eq!(f.encode(&mut b[..8]), Err(FrameError::BufferTooShort));
        let big = vec![0u8; 65536];
        let f = Frame {
            port: 1,
            body: &big,
        };
        assert_eq!(
            f.encode(&mut vec![0u8; 70000]),
            Err(FrameError::BodyTooLong)
        );
    }

    fn stream(frames: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (port, body) in frames {
            let mut b = vec![0u8; 4 + body.len()];
            Frame { port: *port, body }.encode(&mut b).unwrap();
            out.extend_from_slice(&b);
        }
        out
    }

    #[test]
    fn decoder_reassembles_at_every_split() {
        let frames: Vec<(u16, Vec<u8>)> = vec![
            (1, b"abc".to_vec()),
            (2, vec![]),
            (3, (0..=255u8).collect()),
            (65535, b"z".to_vec()),
        ];
        let wire = stream(&frames);
        for chunk in 1..=wire.len() {
            let mut d = FrameDecoder::new();
            let mut got = Vec::new();
            for c in wire.chunks(chunk) {
                d.decode(c, &mut |f| got.push((f.port, f.body.to_vec())));
            }
            assert_eq!(got, frames, "chunk {chunk}");
            assert_eq!(d.buffered(), 0);
        }
        // Two arbitrary splits.
        for a in 0..wire.len() {
            for b in a..wire.len() {
                let mut d = FrameDecoder::new();
                let mut got = Vec::new();
                for c in [&wire[..a], &wire[a..b], &wire[b..]] {
                    d.decode(c, &mut |f| got.push((f.port, f.body.to_vec())));
                }
                assert_eq!(got, frames);
            }
        }
    }

    #[test]
    fn writer_cuts_frames_at_max_body_and_flushes() {
        // max body 4: writes are cut every 4 bytes and at each flush.
        let mut w = FrameWriter::new(4, 64);
        w.write(b"ab").unwrap();
        assert!(!w.has_frame());
        w.flush().unwrap();
        w.flush().unwrap(); // empty flush: no frame
        w.write(b"cdefghij").unwrap(); // 4 + 4 cut, nothing open
        w.write(b"k").unwrap();
        w.write_frame(b"lm").unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 4];
        loop {
            let n = w.fill(&mut buf);
            if n == 0 {
                break;
            }
            got.push(buf[..n].to_vec());
        }
        let want: Vec<Vec<u8>> = vec![
            b"ab".to_vec(),
            b"cdef".to_vec(),
            b"ghij".to_vec(),
            b"klm".to_vec(),
        ];
        assert_eq!(got, want);
        assert_eq!(w.buffered(), 0);
    }

    #[test]
    fn writer_is_bounded() {
        let mut w = FrameWriter::new(4, 6);
        assert_eq!(w.write(b"abcdefgh"), Ok(6));
        assert_eq!(w.write_frame(b"x"), Err(FrameWriterError::WouldBlock));
        let mut buf = [0u8; 4];
        assert_eq!(w.fill(&mut buf), 4);
        assert_eq!(w.write_frame(b"xyz"), Ok(3));
        w.close();
        assert_eq!(w.write(b"a"), Err(FrameWriterError::Closed));
    }
}
