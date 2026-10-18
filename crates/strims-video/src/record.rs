// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Where a muxer's records go: [`RecordSink`], with [`PackedSink`] packing
//! them into a video swarm's frames and [`VecSink`] collecting them whole.

use strims_chunkstream::SizeError;
use strims_chunkstream::packed::{DEFAULT_SIZE, FrameHasEnd, PackedEncoder};

/// A destination for records written in pieces.
///
/// Some layouts can't end a record at every position: a packed frame
/// holds one record end. A writer asks [`end_blocked_after`] before it
/// commits to ending a record, and pads to [`unblocking_len`] when it
/// must end one.
///
/// [`end_blocked_after`]: RecordSink::end_blocked_after
/// [`unblocking_len`]: RecordSink::unblocking_len
pub trait RecordSink {
    /// Whether [`end_record`](Self::end_record) would be refused after
    /// `len` more bytes of the record in progress.
    fn end_blocked_after(&self, len: usize) -> bool;

    /// The least `n >= len` for which
    /// [`end_blocked_after`](Self::end_blocked_after) is false.
    fn unblocking_len(&self, len: usize) -> usize;

    /// Appends `data` to the record in progress.
    fn write(&mut self, data: &[u8]);

    /// Ends the record in progress; the next write starts the next one.
    /// Refused when the layout can't end a record here; the record then
    /// continues.
    fn end_record(&mut self) -> Result<(), FrameHasEnd>;
}

/// Packs records into frames of a video swarm's packed chunkstream
/// ([`strims_chunkstream::packed`]), collecting the stream bytes that are
/// final in an output buffer.
///
/// The driver hands the output on and returns the buffer, so the stream
/// is written without an allocation per segment:
///
/// ```
/// # use strims_video::record::{PackedSink, RecordSink};
/// # let mut writer: Vec<u8> = Vec::new();
/// let mut sink = PackedSink::new();
/// sink.write(b"record");
/// sink.end_record().unwrap();
/// let out = sink.take_output();
/// writer.extend_from_slice(&out);
/// sink.restore(out);
/// ```
#[derive(Debug, Clone)]
pub struct PackedSink {
    encoder: PackedEncoder,
    /// Stream bytes not yet taken.
    out: Vec<u8>,
}

impl PackedSink {
    /// A sink for frames of [`DEFAULT_SIZE`], at stream offset 0.
    pub fn new() -> Self {
        #[allow(clippy::expect_used)]
        // invariant: DEFAULT_SIZE is a valid frame size.
        Self::with_size(DEFAULT_SIZE).expect("DEFAULT_SIZE is a valid frame size")
    }

    /// A sink for frames of `size` bytes, at stream offset 0.
    pub fn with_size(size: usize) -> Result<Self, SizeError> {
        Ok(PackedSink {
            encoder: PackedEncoder::new(size)?,
            out: Vec::new(),
        })
    }

    /// The stream bytes that are final and not yet taken.
    pub fn output(&self) -> &[u8] {
        &self.out
    }

    /// Takes the stream bytes that are final. The bytes after a frame's
    /// record end stay held until the frame fills.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Returns a buffer from [`take_output`](Self::take_output) for reuse.
    /// Its contents are discarded; output written since the take is kept.
    pub fn restore(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        buf.append(&mut self.out);
        self.out = buf;
    }

    /// The stream offset of the next byte written.
    pub fn offset(&self) -> u64 {
        self.encoder.offset()
    }

    pub fn encoder(&self) -> &PackedEncoder {
        &self.encoder
    }
}

impl Default for PackedSink {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordSink for PackedSink {
    fn end_blocked_after(&self, len: usize) -> bool {
        self.encoder.end_blocked_after(len)
    }

    fn unblocking_len(&self, len: usize) -> usize {
        self.encoder.unblocking_len(len)
    }

    fn write(&mut self, data: &[u8]) {
        self.encoder.write(data, &mut self.out);
    }

    fn end_record(&mut self) -> Result<(), FrameHasEnd> {
        self.encoder.end_record(&mut self.out)
    }
}

/// Collects whole records; any position can end one. The caller drains
/// [`records`](Self::records).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VecSink {
    /// The records ended so far, oldest first.
    pub records: Vec<Vec<u8>>,
    /// The record in progress.
    current: Vec<u8>,
}

impl VecSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// The bytes of the record in progress.
    pub fn current(&self) -> &[u8] {
        &self.current
    }
}

impl RecordSink for VecSink {
    fn end_blocked_after(&self, _len: usize) -> bool {
        false
    }

    fn unblocking_len(&self, len: usize) -> usize {
        len
    }

    fn write(&mut self, data: &[u8]) {
        self.current.extend_from_slice(data);
    }

    fn end_record(&mut self) -> Result<(), FrameHasEnd> {
        self.records.push(std::mem::take(&mut self.current));
        Ok(())
    }
}

impl<S: RecordSink + ?Sized> RecordSink for &mut S {
    fn end_blocked_after(&self, len: usize) -> bool {
        (**self).end_blocked_after(len)
    }

    fn unblocking_len(&self, len: usize) -> usize {
        (**self).unblocking_len(len)
    }

    fn write(&mut self, data: &[u8]) {
        (**self).write(data);
    }

    fn end_record(&mut self) -> Result<(), FrameHasEnd> {
        (**self).end_record()
    }
}

#[cfg(test)]
#[path = "record_test.rs"]
mod tests;
