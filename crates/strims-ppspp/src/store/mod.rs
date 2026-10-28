// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chunk storage: the live-window [`Buffer`] with its non-blocking
//! [`BufferReader`]s, and the local [`Writer`]'s bin assignment.
//!
//! The store is written only through [`crate::Swarm`], which also queues
//! every stored chunk for the scheduler ([`crate::Swarm::poll_event`]).

mod buffer;
mod writer;

#[cfg(test)]
mod buffer_test;

pub use buffer::{Buffer, BufferLayout, BufferReader, ChunkBytes};
pub use writer::Writer;

use crate::codec;

/// Store errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The window moved past data the reader had not read.
    /// [`BufferReader::recover`] skips ahead.
    #[error("buffer underrun")]
    BufferUnderrun,
    /// The buffer was reset; make a new reader.
    #[error("stream reset")]
    StreamReset,
    /// The bin's chunks aren't stored (or have left the window).
    #[error("bin data not set")]
    BinDataNotSet,
    /// The buffer was closed.
    #[error("cannot read from closed buffer")]
    Closed,
    /// No chunk to start reading from at or after the requested bin.
    #[error("viable read offset not found")]
    ReadOffsetNotFound,
    /// Nothing to read yet; wait for the next
    /// [`SwarmEvent::Readable`](crate::SwarmEvent::Readable).
    #[error("no data available")]
    WouldBlock,
    #[error("buffer size must be power of 2")]
    InvalidSize,
    #[error("unsupported buffer layout")]
    UnsupportedLayout,
    #[error("cannot cache truncated swarm buffer")]
    CacheTruncated,
    #[error("cannot cache empty buffer")]
    CacheEmpty,
    #[error("cache larger than the buffer")]
    CacheTooLarge,
    /// A bin larger than the window or outside the bin space.
    #[error("bin out of bounds")]
    BinOutOfBounds,
    /// Data that isn't exactly the bin's chunks.
    #[error("data length does not match bin")]
    DataLength,
    #[error(transparent)]
    Codec(#[from] codec::Error),
}
