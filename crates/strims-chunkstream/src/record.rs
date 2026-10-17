// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Length-prefixed records over frames: `uvarint(len) ‖ payload`.

use crate::frame::{BufferTooSmall, Frag, FrameDecoder, FrameEncoder, SizeError, Start};
use crate::uvarint::{MAX_UVARINT_LEN, get_uvarint, put_uvarint, uvarint_len};

/// Writes payloads as framed records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordEncoder {
    frames: FrameEncoder,
}

impl RecordEncoder {
    /// An encoder for frames of `frame_size` bytes.
    pub fn new(frame_size: usize) -> Result<Self, SizeError> {
        Ok(Self {
            frames: FrameEncoder::new(frame_size)?,
        })
    }

    /// The frame size.
    pub fn frame_size(&self) -> usize {
        self.frames.size()
    }

    /// The bytes [`encode`](Self::encode) writes for a payload of
    /// `payload_len` bytes; `None` when that overflows `usize`.
    pub fn encoded_len(&self, payload_len: usize) -> Option<usize> {
        let record_len = uvarint_len(payload_len as u64).checked_add(payload_len)?;
        self.frames.encoded_len(record_len)
    }

    /// Writes `payload`'s record as whole frames to the start of `out`.
    /// Returns the bytes written, [`encoded_len`](Self::encoded_len).
    pub fn encode(&self, payload: &[u8], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let mut prefix = [0; MAX_UVARINT_LEN];
        let n = put_uvarint(&mut prefix, payload.len() as u64);
        self.frames
            .encode(&[prefix.get(..n).unwrap_or_default(), payload], out)
    }
}

/// What a [`RecordDecoder::decode`] call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoded {
    /// Input bytes consumed.
    pub consumed: usize,
    /// A record ended right before input byte `consumed`, and
    /// [`RecordDecoder::record`] holds its payload. When `false`, all the
    /// input was consumed.
    pub record: bool,
}

/// Counters of a [`RecordDecoder`]'s records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecoderMetrics {
    /// Records yielded.
    pub records: u64,
    /// Records dropped by the length check after the decoder started inside
    /// a frame's data: their start preceded the start offset.
    pub partial: u64,
    /// Other records dropped by the length check: their start preceded the
    /// start offset, or they are malformed.
    pub length_mismatch: u64,
    /// Records dropped because their payload exceeded `max_record`.
    pub oversized: u64,
    /// Frames dropped for a header that flags an end outside `4..=size`.
    /// Each drops the record in progress with it.
    pub corrupt: u64,
}

/// Reassembles records from a frame stream that starts at any offset (see
/// the crate docs for the reader contract).
///
/// Payloads are at most `max_record` bytes; a longer record is dropped at
/// its end, and the decoder buffers at most `max_record + MAX_UVARINT_LEN`
/// bytes.
#[derive(Debug, Clone)]
pub struct RecordDecoder {
    frames: FrameDecoder,
    /// The first bytes of the record in progress, or the ready record.
    /// Bound: `limit` bytes.
    buf: Vec<u8>,
    max: usize,
    limit: usize,
    /// Bytes of the record in progress, those beyond `limit` included.
    len: usize,
    /// The record in progress began inside a frame's data.
    partial: bool,
    /// The start of the ready record's payload in `buf`.
    ready: Option<usize>,
    metrics: DecoderMetrics,
}

impl RecordDecoder {
    /// A decoder for frames of `frame_size` bytes and payloads of at most
    /// `max_record` bytes, starting at stream offset 0.
    pub fn new(frame_size: usize, max_record: usize) -> Result<Self, SizeError> {
        Ok(Self {
            frames: FrameDecoder::new(frame_size)?,
            buf: Vec::new(),
            max: max_record,
            limit: max_record.saturating_add(MAX_UVARINT_LEN),
            len: 0,
            partial: false,
            ready: None,
            metrics: DecoderMetrics::default(),
        })
    }

    /// The frame size.
    pub fn frame_size(&self) -> usize {
        self.frames.size()
    }

    /// The largest payload yielded.
    pub fn max_record(&self) -> usize {
        self.max
    }

    /// Restarts at stream offset `offset`, dropping the record in progress:
    /// the next input byte is the stream's byte there.
    pub fn reset(&mut self, offset: u64) {
        let start = self.frames.reset(offset);
        self.discard();
        self.ready = None;
        self.partial = start == Start::Data;
    }

    /// Reads `input` until a record ends or the input runs out. Stops right
    /// after the record's last byte, so the next call starts with the rest
    /// of the input. Each call releases the previous call's record.
    pub fn decode(&mut self, input: &[u8]) -> Decoded {
        if self.ready.take().is_some() {
            self.buf.clear();
        }
        let mut consumed = 0;
        loop {
            let (n, frag) = self
                .frames
                .decode(input.get(consumed..).unwrap_or_default());
            consumed += n;
            match frag {
                None => {
                    return Decoded {
                        consumed,
                        record: false,
                    };
                }
                Some(Frag::Data(data)) => self.push(data),
                Some(Frag::Corrupt) => {
                    self.metrics.corrupt += 1;
                    self.discard();
                    self.partial = false;
                }
                Some(Frag::End) => {
                    if self.finish() {
                        return Decoded {
                            consumed,
                            record: true,
                        };
                    }
                }
            }
        }
    }

    /// The payload of the record the last [`decode`](Self::decode) call
    /// ended at; empty when that call ended at none.
    pub fn record(&self) -> &[u8] {
        self.ready
            .and_then(|start| self.buf.get(start..))
            .unwrap_or_default()
    }

    /// The decoder's counters.
    pub fn metrics(&self) -> &DecoderMetrics {
        &self.metrics
    }

    fn push(&mut self, data: &[u8]) {
        self.len = self.len.saturating_add(data.len());
        let room = self.limit.saturating_sub(self.buf.len());
        let data = data.get(..room.min(data.len())).unwrap_or_default();
        let needed = self.buf.len() + data.len();
        if needed > self.buf.capacity() {
            // Doubling, capped at `limit`, keeps the allocation within the
            // bound.
            let target = needed
                .max(self.buf.capacity().saturating_mul(2))
                .min(self.limit);
            self.buf.reserve_exact(target - self.buf.len());
        }
        self.buf.extend_from_slice(data);
    }

    /// Checks the record that just ended. Returns whether it's ready.
    fn finish(&mut self) -> bool {
        let len = self.len;
        let partial = self.partial;
        self.len = 0;
        self.partial = false;
        match get_uvarint(&self.buf) {
            // `buf` holds the first `min(len, limit)` bytes, and `limit`
            // covers any varint, so the check sees the whole varint.
            Some((declared, n)) if declared == len.saturating_sub(n) as u64 => {
                if declared > self.max as u64 {
                    self.metrics.oversized += 1;
                } else {
                    self.metrics.records += 1;
                    self.ready = Some(n);
                    return true;
                }
            }
            _ if partial => self.metrics.partial += 1,
            _ => self.metrics.length_mismatch += 1,
        }
        self.buf.clear();
        false
    }

    fn discard(&mut self) {
        self.buf.clear();
        self.len = 0;
    }
}

#[cfg(test)]
#[path = "record_test.rs"]
mod tests;
