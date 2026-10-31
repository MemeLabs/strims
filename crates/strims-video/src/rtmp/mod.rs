// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A sans-IO RTMP ingest server.
//!
//! [`RtmpServer`] takes the bytes of one TCP connection and yields
//! [`RtmpEvent`]s: the publish request, the stream's metadata, its audio
//! and video message bodies (FLV tag bodies, for
//! [`FlvSource`](crate::FlvSource)) and the end of the publish. The
//! driver owns the socket and the deadlines; the server owns no time.
//!
//! - **Handshake:** version 3 only. A C1 that carries a valid Flash Player
//!   digest gets the digest reply (S1 signed with the server key, S2
//!   signed over the C1 digest); any other C1 gets the plain reply, whose
//!   S2 echoes C1. C2 is read and ignored.
//! - **Chunk streams:** basic header formats 0-3 with 1-, 2- and 3-byte
//!   chunk stream ids, timestamp deltas and extended timestamps (repeated
//!   on format 3 chunks of an extended header). Messages reassemble per
//!   chunk stream into buffers that grow as chunks arrive.
//! - **Control:** Set Chunk Size, Abort, Window Acknowledgement Size (the
//!   server acknowledges the peer's window), Set Peer Bandwidth and
//!   acknowledgements (ignored), and user control pings (answered).
//!   Aggregate messages split into their FLV tags.
//! - **Commands:** `connect`, `releaseStream`, `FCPublish`,
//!   `createStream`, `publish` and `deleteStream` / `FCUnpublish` /
//!   `closeStream`; `@setDataFrame` and `onMetaData` data messages, and
//!   `onTextData` ones, whose `text` is a subtitle cue. Other
//!   commands and data messages are ignored, as are audio and video
//!   before the publish is accepted. AMF3 command and data messages are
//!   taken as AMF0 ones when their format byte is 0 and a name follows in
//!   AMF0; any other is ignored, as are shared objects.
//! - **Stream Begin:** the server sends user control Stream Begin of
//!   message stream 0 in its `connect` reply, and of the publish's stream
//!   before `NetStream.Publish.Start`.
//!
//! Every buffer has a bound in [`RtmpConfig`], and peer input that breaks
//! one fails the connection with an [`RtmpError`].

pub(crate) mod chunk;
pub(crate) mod handshake;
mod server;
#[cfg(test)]
pub(crate) mod test_util;

pub use server::{Phase, RtmpConfig, RtmpEvent, RtmpServer};

use crate::amf0::Amf0Error;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RtmpError {
    #[error("RTMP version {0} is not supported; the handshake must start with version 3")]
    Version(u8),
    /// A format 1-3 chunk on a chunk stream without a previous header.
    #[error("chunk stream {0} continues a message it never started")]
    NoPreviousHeader(u32),
    /// A format 0-2 chunk on a chunk stream with a message in progress.
    #[error("chunk stream {0} starts a message before finishing the last")]
    Interrupted(u32),
    #[error("message type {type_id} of {len} bytes exceeds the {max}-byte limit")]
    MessageTooLarge { type_id: u8, len: u32, max: usize },
    #[error("partial messages exceed {0} bytes")]
    PartialTooLarge(usize),
    #[error("more than {0} chunk streams")]
    TooManyChunkStreams(usize),
    /// A Set Chunk Size of 0 or with its reserved top bit set.
    #[error("chunk size {0:#x} is not valid")]
    ChunkSize(u32),
    #[error("{0} message is truncated")]
    Truncated(&'static str),
    #[error("aggregate message is malformed")]
    Aggregate,
    #[error("invalid AMF0 in a {context}: {error}")]
    Amf {
        context: &'static str,
        error: Amf0Error,
    },
    #[error("command name is not a string")]
    CommandName,
    #[error("{0} before connect")]
    NotConnected(&'static str),
    #[error("unexpected {0}")]
    Unexpected(&'static str),
    #[error("connect names no app")]
    ConnectApp,
    #[error("publish names no stream")]
    PublishName,
    #[error("queued output exceeds {0} bytes")]
    OutputFull(usize),
}
