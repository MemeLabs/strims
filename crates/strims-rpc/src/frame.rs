// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Splitting a byte stream into uvarint-length-delimited frames.

use crate::wire::{MalformedVarint, get_uvarint};

/// Why a stream can't be split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// A frame announced a length over the limit.
    #[error("frame of {len} bytes exceeds the {max}-byte limit")]
    TooLarge {
        /// The announced length.
        len: u64,
        /// The decoder's limit.
        max: usize,
    },
    /// A length prefix isn't a valid uvarint.
    #[error("malformed frame length")]
    MalformedLength,
}

impl From<MalformedVarint> for FrameError {
    fn from(_: MalformedVarint) -> FrameError {
        FrameError::MalformedLength
    }
}

/// Splits a byte stream, cut anywhere, into frames of at most `max` bytes.
///
/// Whole frames are lent straight from the input; only a frame cut across
/// inputs is copied. The buffer holds at most one partial frame and its
/// prefix: `max` + [`MAX_VARINT_LEN`](crate::wire::MAX_VARINT_LEN) bytes. After an error the stream
/// can't be resynchronised, and later input fails too.
#[derive(Debug)]
pub struct FrameDecoder {
    max: usize,
    /// A partial frame, from its prefix's first byte.
    buf: Vec<u8>,
    failed: Option<FrameError>,
}

impl FrameDecoder {
    /// A decoder for frames of at most `max` bytes.
    pub fn new(max: usize) -> FrameDecoder {
        FrameDecoder {
            max,
            buf: Vec::new(),
            failed: None,
        }
    }

    /// The largest frame accepted.
    pub fn max(&self) -> usize {
        self.max
    }

    /// Bytes of a partial frame held for the next input.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// Feeds `data`, passing each completed frame to `f`.
    pub fn decode(&mut self, mut data: &[u8], f: &mut impl FnMut(&[u8])) -> Result<(), FrameError> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        let res = self.decode_inner(&mut data, f);
        if let Err(e) = res {
            self.failed = Some(e);
            self.buf = Vec::new();
        }
        res
    }

    fn decode_inner(
        &mut self,
        data: &mut &[u8],
        f: &mut impl FnMut(&[u8]),
    ) -> Result<(), FrameError> {
        while !data.is_empty() {
            if self.buf.is_empty() {
                // lend whole frames from the input
                match self.prefix(data)? {
                    Some((n, len)) if data.len() - n >= len => {
                        let (frame, rest) = data.split_at(n + len);
                        f(frame.get(n..).unwrap_or_default());
                        *data = rest;
                        continue;
                    }
                    _ => {}
                }
            }
            // a partial frame: complete its prefix a byte at a time, then
            // its body
            let need = match self.prefix(&self.buf)? {
                Some((n, len)) => n + len - self.buf.len(),
                None => 1,
            };
            let (head, rest) = data.split_at(need.min(data.len()));
            self.buf.extend_from_slice(head);
            *data = rest;
            if let Some((n, len)) = self.prefix(&self.buf)?
                && self.buf.len() == n + len
            {
                f(self.buf.get(n..).unwrap_or_default());
                self.buf.clear();
            }
        }
        Ok(())
    }

    /// The prefix length and frame length at the start of `buf`, or `None`
    /// if the prefix is incomplete.
    fn prefix(&self, buf: &[u8]) -> Result<Option<(usize, usize)>, FrameError> {
        let Some((len, n)) = get_uvarint(buf)? else {
            return Ok(None);
        };
        match usize::try_from(len) {
            Ok(l) if l <= self.max => Ok(Some((n, l))),
            _ => Err(FrameError::TooLarge { len, max: self.max }),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use crate::wire::{MAX_VARINT_LEN, put_uvarint};

    fn stream(frames: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        for f in frames {
            put_uvarint(&mut out, f.len() as u64);
            out.extend_from_slice(f);
        }
        out
    }

    fn decode_in_chunks(d: &mut FrameDecoder, data: &[u8], chunk: usize) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for c in data.chunks(chunk.max(1)) {
            d.decode(c, &mut |f| out.push(f.to_vec())).unwrap();
        }
        out
    }

    #[test]
    fn frames_survive_any_cut() {
        let frames: Vec<Vec<u8>> = vec![
            vec![],
            vec![1],
            (0..200).map(|i| i as u8).collect(),
            vec![7; 300],
            vec![],
        ];
        let data = stream(&frames);
        for chunk in 1..=data.len() {
            let mut d = FrameDecoder::new(300);
            assert_eq!(
                decode_in_chunks(&mut d, &data, chunk),
                frames,
                "chunk {chunk}"
            );
            assert_eq!(d.buffered(), 0);
        }
    }

    #[test]
    fn a_frame_of_exactly_max_passes_and_one_more_fails() {
        let mut d = FrameDecoder::new(4);
        let mut n = 0;
        d.decode(&stream(&[vec![0; 4]]), &mut |_| n += 1).unwrap();
        assert_eq!(n, 1);
        let err = d.decode(&stream(&[vec![0; 5]]), &mut |_| n += 1);
        assert_eq!(err, Err(FrameError::TooLarge { len: 5, max: 4 }));
        // the stream is lost
        assert_eq!(
            d.decode(&stream(&[vec![0; 1]]), &mut |_| n += 1),
            Err(FrameError::TooLarge { len: 5, max: 4 })
        );
        assert_eq!(n, 1);
    }

    #[test]
    fn an_oversized_prefix_fails_before_its_body_arrives() {
        let mut d = FrameDecoder::new(1024);
        let mut prefix = Vec::new();
        put_uvarint(&mut prefix, 1 << 40);
        assert!(matches!(
            d.decode(&prefix, &mut |_| {}),
            Err(FrameError::TooLarge { .. })
        ));
    }

    #[test]
    fn malformed_prefixes_fail() {
        let mut d = FrameDecoder::new(1024);
        // split across inputs too
        d.decode(&[0x80; 5], &mut |_| {}).unwrap();
        assert_eq!(
            d.decode(&[0x80; 6], &mut |_| {}),
            Err(FrameError::MalformedLength)
        );
    }

    #[test]
    fn the_buffer_holds_one_partial_frame() {
        let mut d = FrameDecoder::new(1000);
        let data = stream(&[vec![1; 1000]]);
        d.decode(&data[..data.len() - 1], &mut |_| panic!("incomplete"))
            .unwrap();
        assert_eq!(d.buffered(), data.len() - 1);
        assert!(d.buffered() <= d.max() + MAX_VARINT_LEN);
    }
}
