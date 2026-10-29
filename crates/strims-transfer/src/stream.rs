// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Protobuf records over swarm streams; the crate docs' Streams section
//! states the contracts.

use std::fmt;
use std::future::Future;

use prost::Message;
use strims_chunkstream::{DecoderMetrics, RecordDecoder, RecordEncoder, SizeError};
use strims_ppspp::{SwarmId, SwarmOptions, WriterOptions};
use strims_proto::types::Cache;

use crate::host::Host;

/// The most bytes a [`RecordReader`] takes from one swarm read.
const READ_BUFFER: usize = 64 * 1024;

/// Why a swarm read returned no data.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReadFailure<E> {
    /// The stream started over or skipped data the reader fell behind on.
    /// The next read continues at the new position.
    #[error("stream discontinuity")]
    Discontinuity,
    /// The swarm was removed or the reader closed; later reads fail too.
    #[error("stream closed")]
    Closed,
    /// The read failed; the next read may go on.
    #[error("read failed: {0}")]
    Failed(E),
}

/// Why a [`RecordWriter`] write failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WriteFailure<E> {
    /// The message's encoding of `len` bytes exceeds the writer's
    /// `max_record`. Nothing was written.
    #[error("record of {len} bytes is too large")]
    TooLarge { len: usize },
    /// The swarm writer failed, so the stream may end inside the record.
    /// The next write starts the stream over first.
    #[error("write failed: {0}")]
    Failed(E),
}

/// A reader of a swarm's stream.
pub trait SwarmRead: 'static {
    /// A failed read.
    type Error: fmt::Display;

    /// Reads at least one byte into `out`, which isn't empty, waiting for
    /// data. Returns the stream offset of `out[0]` and the bytes read.
    ///
    /// A reader starts at the swarm's window tail, and each read starts
    /// where the last one ended, except across a discontinuity. When the
    /// stream starts over, a read fails with
    /// [`ReadFailure::Discontinuity`]. When the reader skips data it fell
    /// behind on, a read fails so too, or the skip shows only as a gap
    /// between offsets. Reading then continues at the new position.
    fn read_at(
        &mut self,
        out: &mut [u8],
    ) -> impl Future<Output = Result<(u64, usize), ReadFailure<Self::Error>>>;
}

/// A writer of a swarm's stream.
pub trait SwarmWrite: 'static {
    /// A failed write.
    type Error: fmt::Display;

    /// Appends `data` to the stream. The integrity units it completes are
    /// published; a partial unit waits for the bytes that complete it, or
    /// for [`SwarmWrite::flush`].
    fn write(&mut self, data: &[u8]) -> impl Future<Output = Result<(), Self::Error>>;

    /// Publishes the partial unit at once (`strims_ppspp::Writer::flush`),
    /// padded with zeros to a whole unit, which readers read as zeros.
    /// Returns the stream offset at which the next unit starts, where the
    /// next write lands: the bytes written so far plus the padding. With
    /// nothing staged it publishes nothing and returns the current offset.
    fn flush(&mut self) -> impl Future<Output = Result<u64, Self::Error>>;

    /// Starts the stream over at offset 0 under a new epoch, dropping a
    /// partial unit. Readers see a discontinuity.
    fn reset(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

/// The node's swarms, for services that publish and read streams.
pub trait SwarmHost: Host {
    /// A writer of a swarm this node publishes.
    type Writer: SwarmWrite;
    /// A reader of a swarm's stream.
    type Reader: SwarmRead;

    /// Creates a swarm this node publishes, signed with `options.key`:
    /// its handle, its id and its writer.
    fn create_swarm(
        &self,
        options: WriterOptions,
    ) -> impl Future<Output = Result<(Self::Swarm, SwarmId, Self::Writer), Self::Error>>;

    /// Adds the swarm `id` to download and relay. `options` must match the
    /// publisher's (chunk size, integrity, stream count).
    fn add_swarm(
        &self,
        id: SwarmId,
        options: SwarmOptions,
    ) -> impl Future<Output = Result<Self::Swarm, Self::Error>>;

    /// Adds the swarm `id` as [`add_swarm`](SwarmHost::add_swarm) does,
    /// starting from `cache`, a cache of its stream from
    /// [`export_cache`](SwarmHost::export_cache). Resolves to the swarm and
    /// whether it took the cache. A swarm that refused the cache starts
    /// empty, as an added one does; so does every swarm of a host that
    /// keeps no caches, which is the default.
    fn add_cached_swarm(
        &self,
        id: SwarmId,
        options: SwarmOptions,
        cache: Cache,
    ) -> impl Future<Output = Result<(Self::Swarm, bool), Self::Error>> {
        drop(cache);
        async move { Ok((self.add_swarm(id, options).await?, false)) }
    }

    /// The swarm's stream as a cache: its data from offset 0, its
    /// signatures and its epoch. `None` once the window has moved past
    /// offset 0, while the swarm holds nothing, when the swarm is gone, and
    /// on a host that keeps no caches, which is the default.
    fn export_cache(&self, swarm: Self::Swarm) -> impl Future<Output = Option<Cache>> {
        let _ = swarm;
        std::future::ready(None)
    }

    /// Removes a swarm; its writer and readers close. Remove its transfer
    /// first.
    fn remove_swarm(&self, swarm: Self::Swarm) -> impl Future<Output = Result<(), Self::Error>>;

    /// A reader of the swarm's stream from its window tail.
    fn open_reader(
        &self,
        swarm: Self::Swarm,
    ) -> impl Future<Output = Result<Self::Reader, Self::Error>>;
}

/// Writes protobuf messages into a swarm's stream, one record each, with
/// one [`SwarmWrite::write`] of whole frames per record.
pub struct RecordWriter<W> {
    writer: W,
    encoder: RecordEncoder,
    max_record: usize,
    /// The message being written. Bound: `max_record` bytes.
    payload: Vec<u8>,
    /// Its frames. Bound: the frames of a `max_record` byte payload.
    frames: Vec<u8>,
    /// A write didn't finish, so the stream may end inside a record.
    torn: bool,
}

impl<W: SwarmWrite> RecordWriter<W> {
    /// A writer of `frame_size` byte frames and messages of at most
    /// `max_record` encoded bytes.
    pub fn new(writer: W, frame_size: usize, max_record: usize) -> Result<Self, SizeError> {
        Ok(RecordWriter {
            writer,
            encoder: RecordEncoder::new(frame_size)?,
            max_record,
            payload: Vec::new(),
            frames: Vec::new(),
            torn: false,
        })
    }

    /// Appends `message` as one record. After a write that failed or was
    /// cancelled, it first starts the stream over, so readers see a
    /// discontinuity in place of a torn record.
    pub async fn write<M: Message>(&mut self, message: &M) -> Result<(), WriteFailure<W::Error>> {
        let len = message.encoded_len();
        if len > self.max_record {
            return Err(WriteFailure::TooLarge { len });
        }
        self.payload.clear();
        message.encode_raw(&mut self.payload);
        let frames = self
            .encoder
            .encoded_len(self.payload.len())
            .ok_or(WriteFailure::TooLarge { len })?;
        self.frames.resize(frames, 0);
        // `frames` holds exactly the encoding, so this can't fail.
        let frames = self
            .encoder
            .encode(&self.payload, &mut self.frames)
            .map_err(|_| WriteFailure::TooLarge { len })?;

        if self.torn {
            self.writer.reset().await.map_err(WriteFailure::Failed)?;
            self.torn = false;
        }
        self.torn = true;
        self.writer
            .write(self.frames.get(..frames).unwrap_or_default())
            .await
            .map_err(WriteFailure::Failed)?;
        self.torn = false;
        Ok(())
    }

    /// Starts the stream over at offset 0 under a new epoch. Readers see a
    /// discontinuity.
    pub async fn reset(&mut self) -> Result<(), W::Error> {
        self.writer.reset().await?;
        self.torn = false;
        Ok(())
    }
}

/// Counters of a [`RecordReader`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReaderMetrics {
    /// The record decoder's counters.
    pub decoder: DecoderMetrics,
    /// Discontinuities reported, gaps between read offsets included.
    pub discontinuities: u64,
    /// Records [`RecordReader::next_message`] skipped because they didn't
    /// decode.
    pub malformed: u64,
}

/// Reads the records of a swarm's stream from wherever its reader starts.
pub struct RecordReader<R> {
    reader: R,
    decoder: RecordDecoder,
    /// Bytes read and not yet decoded are `buf[start..end]`. Bound:
    /// [`READ_BUFFER`] bytes.
    buf: Box<[u8]>,
    start: usize,
    end: usize,
    /// The stream offset just past the last read. `None` before the first
    /// read and after a reported discontinuity: the decoder restarts at the
    /// next read's offset.
    next: Option<u64>,
    discontinuities: u64,
    malformed: u64,
}

impl<R: SwarmRead> RecordReader<R> {
    /// A reader of `frame_size` byte frames and payloads of at most
    /// `max_record` bytes; longer records are skipped.
    pub fn new(reader: R, frame_size: usize, max_record: usize) -> Result<Self, SizeError> {
        Ok(RecordReader {
            reader,
            decoder: RecordDecoder::new(frame_size, max_record)?,
            buf: vec![0; READ_BUFFER].into_boxed_slice(),
            start: 0,
            end: 0,
            next: None,
            discontinuities: 0,
            malformed: 0,
        })
    }

    /// The next whole record's payload.
    ///
    /// A discontinuity, reported by the swarm reader or seen as a gap
    /// between read offsets, fails one call with
    /// [`ReadFailure::Discontinuity`]. The next call goes on at the new
    /// position with the decoder restarted at its offset, so its first
    /// record may be the tail of an earlier one.
    pub async fn next(&mut self) -> Result<&[u8], ReadFailure<R::Error>> {
        loop {
            if let Some(input) = self.buf.get(self.start..self.end)
                && !input.is_empty()
            {
                let d = self.decoder.decode(input);
                self.start += d.consumed;
                if d.record {
                    return Ok(self.decoder.record());
                }
                continue;
            }

            let (offset, n) = match self.reader.read_at(&mut self.buf).await {
                Ok(r) => r,
                Err(ReadFailure::Discontinuity) => {
                    self.next = None;
                    self.discontinuities += 1;
                    return Err(ReadFailure::Discontinuity);
                }
                Err(e) => return Err(e),
            };
            let n = n.min(self.buf.len());
            if n == 0 {
                // A swarm read returns data or fails, so a read of nothing
                // ends the stream.
                return Err(ReadFailure::Closed);
            }
            self.start = 0;
            self.end = n;
            let expected = self.next.replace(offset.wrapping_add(n as u64));
            if expected != Some(offset) {
                self.decoder.reset(offset);
                if expected.is_some() {
                    self.discontinuities += 1;
                    return Err(ReadFailure::Discontinuity);
                }
            }
        }
    }

    /// The next record that decodes as `M`. Records that don't are skipped
    /// and counted; failures are those of [`next`](Self::next).
    pub async fn next_message<M: Message + Default>(&mut self) -> Result<M, ReadFailure<R::Error>> {
        loop {
            match M::decode(self.next().await?) {
                Ok(m) => return Ok(m),
                Err(_) => self.malformed += 1,
            }
        }
    }

    /// The reader's counters.
    pub fn metrics(&self) -> ReaderMetrics {
        ReaderMetrics {
            decoder: *self.decoder.metrics(),
            discontinuities: self.discontinuities,
            malformed: self.malformed,
        }
    }
}

#[cfg(test)]
#[path = "stream_test.rs"]
mod tests;
