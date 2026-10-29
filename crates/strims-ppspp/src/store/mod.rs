// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chunk storage: the live-window [`Buffer`] with its non-blocking
//! [`BufferReader`]s, which can move back within the window
//! ([`BufferReader::rewind`]), and the local [`Writer`]'s bin assignment.
//!
//! The store is written only through [`crate::Swarm`], which also queues
//! every stored chunk for the scheduler ([`crate::Swarm::poll_event`]).

mod buffer;
mod writer;

#[cfg(test)]
mod buffer_test;

pub use buffer::{Buffer, BufferLayout, BufferReader, ChunkBytes, ChunkCodec};
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

/// Why a reader stays where it was instead of moving back
/// ([`BufferReader::rewind`], [`crate::Runner::rewind`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RewindError {
    /// The offset is past the reader's next byte.
    #[error("offset past the read position")]
    Ahead,
    /// The offset's chunk is outside the live window: its slot in the ring
    /// holds a newer chunk, or its munro is behind the verifier's ring, or
    /// the buffer never moves its window ([`BufferLayout::ELASTIC`]).
    #[error("offset outside the live window")]
    OutsideWindow,
    /// No peer can serve the chunks from the offset's on: none announced
    /// one of them, they left every holder's window before they arrived,
    /// or the swarm has no channels.
    #[error("no peer can serve the offset")]
    Unavailable,
    /// The reader's next read returns this error first (a reset, an
    /// underrun, a close), or the reader has no position yet
    /// ([`Error::WouldBlock`]). The rewind left it for that read.
    #[error(transparent)]
    Read(Error),
}
