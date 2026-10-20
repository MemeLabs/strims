// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The channel layer between frames and the scheduler: handshake
//! validation, per-direction message counters, and the metered payload
//! writer.
//!
//! - [`validate_handshake`] checks a peer's options in a fixed order;
//!   [`Runner::handle_frame`](crate::Runner::handle_frame) dispatches the
//!   rest of a channel's messages. Our own handshake is
//!   [`Swarm::handshake_options`].
//! - `MeteredWriter` is a [`ChannelWriter`] that counts what it writes. The
//!   counts reach the channel's [`ChannelMetrics`] only when the payload is
//!   finished and non-empty, so a discarded payload counts nothing.

use crate::codec::{
    self, ChannelWriter, HandshakeOptions, Message, MessageWriter, ProtocolOptions,
};
use crate::swarm::Swarm;
use crate::{MINIMUM_PROTOCOL_VERSION, PROTOCOL_VERSION};

/// Handshake validation errors: a required option is missing, or its value
/// differs from ours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("handshake missing VersionOption")]
    NoVersion,
    #[error("handshake missing MinimumVersionOption")]
    NoMinimumVersion,
    #[error("handshake missing LiveWindowOption")]
    NoLiveWindow,
    #[error("handshake missing ChunkSizeOption")]
    NoChunkSize,
    #[error("handshake missing ChunksPerSignatureOption")]
    NoChunksPerSignature,
    #[error("handshake missing StreamCountOption")]
    NoStreamCount,
    #[error("handshake missing SwarmIdentifierOption")]
    NoSwarmIdentifier,
    #[error("handshake missing ContentIntegrityProtectionMethod")]
    NoContentIntegrityProtectionMethod,
    #[error("handshake missing MerkleHashTreeFunction")]
    NoMerkleHashTreeFunction,
    #[error("handshake missing LiveSignatureAlgorithm")]
    NoLiveSignatureAlgorithm,
    #[error("incompatible VersionOption")]
    IncompatibleVersion,
    #[error("incompatible MinimumVersionOption")]
    IncompatibleMinimumVersion,
    #[error("invalid LiveWindowOption")]
    InvalidLiveWindow,
    #[error("incompatible ChunkSizeOption")]
    IncompatibleChunkSize,
    #[error("incompatible ChunksPerSignatureOption")]
    IncompatibleChunksPerSignature,
    #[error("incompatible StreamCountOption")]
    IncompatibleStreamCount,
    #[error("incompatible SwarmIdentifierOption")]
    IncompatibleSwarmIdentifier,
    #[error("incompatible ContentIntegrityProtectionMethod")]
    IncompatibleContentIntegrityProtectionMethod,
    #[error("incompatible MerkleHashTreeFunction")]
    IncompatibleMerkleHashTreeFunction,
    #[error("incompatible LiveSignatureAlgorithm")]
    IncompatibleLiveSignatureAlgorithm,
}

/// Checks a peer's handshake options against `swarm` and returns them (the
/// first value of each option; later duplicates are ignored). The epoch is
/// left to the caller.
///
/// The checks run in a fixed order and the first failure is the error:
/// swarm id, version, minimum version, live window, chunk size, chunks per
/// signature, stream count, then the integrity method, hash function and
/// signature algorithm. A missing option fails as `No*` at its place in
/// that order.
pub fn validate_handshake<'a>(
    swarm: &Swarm,
    options: &ProtocolOptions<'a>,
) -> Result<HandshakeOptions<'a>, HandshakeError> {
    use HandshakeError as E;

    let o = options.summary();
    let so = swarm.options();

    let id = o.swarm_identifier.ok_or(E::NoSwarmIdentifier)?;
    if id != swarm.id().as_bytes() {
        return Err(E::IncompatibleSwarmIdentifier);
    }
    if o.version.ok_or(E::NoVersion)? < MINIMUM_PROTOCOL_VERSION {
        return Err(E::IncompatibleVersion);
    }
    if o.minimum_version.ok_or(E::NoMinimumVersion)? > PROTOCOL_VERSION {
        return Err(E::IncompatibleMinimumVersion);
    }
    if o.live_window.ok_or(E::NoLiveWindow)? == 0 {
        return Err(E::InvalidLiveWindow);
    }
    if o.chunk_size.ok_or(E::NoChunkSize)? != so.chunk_size as u32 {
        return Err(E::IncompatibleChunkSize);
    }
    if o.chunks_per_signature.ok_or(E::NoChunksPerSignature)? != so.chunks_per_signature as u32 {
        return Err(E::IncompatibleChunksPerSignature);
    }
    if o.stream_count.ok_or(E::NoStreamCount)? != so.stream_count as u16 {
        return Err(E::IncompatibleStreamCount);
    }
    let i = &so.integrity;
    if o.content_integrity_protection_method
        .ok_or(E::NoContentIntegrityProtectionMethod)?
        != i.protection_method.0
    {
        return Err(E::IncompatibleContentIntegrityProtectionMethod);
    }
    if o.merkle_hash_tree_function
        .ok_or(E::NoMerkleHashTreeFunction)?
        != i.merkle_hash_tree_function.0
    {
        return Err(E::IncompatibleMerkleHashTreeFunction);
    }
    if o.live_signature_algorithm
        .ok_or(E::NoLiveSignatureAlgorithm)?
        != i.live_signature_algorithm.0
    {
        return Err(E::IncompatibleLiveSignatureAlgorithm);
    }
    Ok(o)
}

/// Message counters for one direction of a channel: one per message type,
/// plus byte and validation counters.
///
/// `chunk` counts the chunks `Data` messages span. `data_bytes` counts
/// `Data` payload bytes and `overhead_bytes` every other byte of a message
/// body; neither counts the type byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelMetrics {
    pub handshake: u64,
    pub restart: u64,
    pub data: u64,
    pub chunk: u64,
    pub integrity: u64,
    pub signed_integrity: u64,
    pub ack: u64,
    pub have: u64,
    pub request: u64,
    pub cancel: u64,
    pub choke: u64,
    pub unchoke: u64,
    pub ping: u64,
    pub pong: u64,
    pub stream_request: u64,
    pub stream_cancel: u64,
    pub stream_open: u64,
    pub stream_close: u64,
    pub announce: u64,
    pub data_bytes: u64,
    pub overhead_bytes: u64,
    /// Inbound `Data` messages that failed verification.
    pub invalid_data: u64,
    /// The chunks those messages spanned.
    pub invalid_chunk: u64,
    /// Their payload bytes.
    pub invalid_bytes: u64,
    /// Inbound messages ignored as invalid: a bin past `MAX_BIN` or wider
    /// than the window, a STREAM_OPEN for a stream we never asked the peer
    /// for, or one far past our request, an ANNOUNCE naming its sender as
    /// its parent, a stream past the count, or more entries than a seed
    /// takes. Honest peers don't send these.
    pub rejected: u64,
    /// Inbound STREAM_OPENs ignored because they answer a STREAM_REQUEST we
    /// cancelled before the answer arrived: a race between honest peers.
    pub stale_stream_open: u64,
    /// Inbound `Data` for chunks already behind our window (or behind the
    /// munros we can still verify), ignored: late answers.
    pub stale_data: u64,
}

impl ChannelMetrics {
    /// Counts one message.
    pub(crate) fn count(&mut self, m: &Message<'_>) {
        let body = m.byte_len() as u64;
        let c = match m {
            Message::Handshake(_) => &mut self.handshake,
            Message::Restart => &mut self.restart,
            Message::Data(d) => {
                add(&mut self.data, 1);
                add(&mut self.chunk, d.bin.base_length());
                let data = d.data.len() as u64;
                add(&mut self.data_bytes, data);
                add(&mut self.overhead_bytes, body.saturating_sub(data));
                return;
            }
            Message::Ack(_) => &mut self.ack,
            Message::Have(_) => &mut self.have,
            Message::Integrity(_) => &mut self.integrity,
            Message::SignedIntegrity(_) => &mut self.signed_integrity,
            Message::Request(_) => &mut self.request,
            Message::Cancel(_) => &mut self.cancel,
            Message::Choke => &mut self.choke,
            Message::Unchoke => &mut self.unchoke,
            Message::Ping(_) => &mut self.ping,
            Message::Pong(_) => &mut self.pong,
            Message::StreamRequest(_) => &mut self.stream_request,
            Message::StreamCancel(_) => &mut self.stream_cancel,
            Message::StreamOpen(_) => &mut self.stream_open,
            Message::StreamClose(_) => &mut self.stream_close,
            Message::Announce(_) => &mut self.announce,
            Message::End => return,
        };
        add(c, 1);
        add(&mut self.overhead_bytes, body);
    }

    fn merge(&mut self, o: &ChannelMetrics) {
        let ChannelMetrics {
            handshake,
            restart,
            data,
            chunk,
            integrity,
            signed_integrity,
            ack,
            have,
            request,
            cancel,
            choke,
            unchoke,
            ping,
            pong,
            stream_request,
            stream_cancel,
            stream_open,
            stream_close,
            announce,
            data_bytes,
            overhead_bytes,
            invalid_data,
            invalid_chunk,
            invalid_bytes,
            rejected,
            stale_stream_open,
            stale_data,
        } = o;
        add(&mut self.handshake, *handshake);
        add(&mut self.restart, *restart);
        add(&mut self.data, *data);
        add(&mut self.chunk, *chunk);
        add(&mut self.integrity, *integrity);
        add(&mut self.signed_integrity, *signed_integrity);
        add(&mut self.ack, *ack);
        add(&mut self.have, *have);
        add(&mut self.request, *request);
        add(&mut self.cancel, *cancel);
        add(&mut self.choke, *choke);
        add(&mut self.unchoke, *unchoke);
        add(&mut self.ping, *ping);
        add(&mut self.pong, *pong);
        add(&mut self.stream_request, *stream_request);
        add(&mut self.stream_cancel, *stream_cancel);
        add(&mut self.stream_open, *stream_open);
        add(&mut self.stream_close, *stream_close);
        add(&mut self.announce, *announce);
        add(&mut self.data_bytes, *data_bytes);
        add(&mut self.overhead_bytes, *overhead_bytes);
        add(&mut self.invalid_data, *invalid_data);
        add(&mut self.invalid_chunk, *invalid_chunk);
        add(&mut self.invalid_bytes, *invalid_bytes);
        add(&mut self.rejected, *rejected);
        add(&mut self.stale_stream_open, *stale_stream_open);
        add(&mut self.stale_data, *stale_data);
    }
}

/// Counters saturate: peers choose bins, so `chunk` grows by up to 2^61 per
/// message.
fn add(c: &mut u64, n: u64) {
    *c = c.saturating_add(n);
}

/// A [`ChannelWriter`] that counts the messages it writes.
#[derive(Debug)]
pub(crate) struct MeteredWriter<'b> {
    cw: ChannelWriter<'b>,
    pending: ChannelMetrics,
}

impl<'b> MeteredWriter<'b> {
    pub(crate) fn new(buf: &'b mut [u8], channel: u64) -> MeteredWriter<'b> {
        MeteredWriter {
            cw: ChannelWriter::new(buf, channel),
            pending: ChannelMetrics::default(),
        }
    }

    /// Payload space left.
    pub(crate) fn available(&self) -> usize {
        self.cw.available()
    }

    /// Discards the payload and its counts.
    pub(crate) fn reset(&mut self) {
        self.cw.reset();
        self.pending = ChannelMetrics::default();
    }

    /// Writes the channel header, merges the counts into `out`, and
    /// returns the bytes used (0 if nothing was written) and the data bytes
    /// written.
    pub(crate) fn finish(self, out: &mut ChannelMetrics) -> (usize, u64) {
        let n = self.cw.finish();
        if n == 0 {
            return (0, 0);
        }
        out.merge(&self.pending);
        (n, self.pending.data_bytes)
    }
}

impl MessageWriter for MeteredWriter<'_> {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, codec::Error> {
        let n = self.cw.write(m)?;
        self.pending.count(m);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use crate::codec::{Have, ProtocolOption};
    use crate::{SwarmId, SwarmOptions};
    use strims_binmap::Bin;

    fn swarm() -> Swarm {
        let key = strims_crypto::key_from_seed(&[3; 32]);
        Swarm::new(SwarmId::new(&key.public), SwarmOptions::default()).unwrap()
    }

    #[test]
    fn own_handshake_validates() {
        let s = swarm();
        let h = s.handshake_options();
        let o = validate_handshake(&s, &h.handshake().options).unwrap();
        assert_eq!(o.live_window, Some(1 << 16));
    }

    #[test]
    fn handshake_reports_the_first_failed_check() {
        let s = swarm();
        let h = s.handshake_options();
        let all = h.options();
        // Drop or change one option at a time; the first problem in check
        // order wins.
        let cases: Vec<(usize, Option<ProtocolOption<'_>>, HandshakeError)> = vec![
            (0, None, HandshakeError::NoSwarmIdentifier),
            (
                0,
                Some(ProtocolOption::SwarmIdentifier(b"other")),
                HandshakeError::IncompatibleSwarmIdentifier,
            ),
            (1, None, HandshakeError::NoVersion),
            (
                1,
                Some(ProtocolOption::Version(1)),
                HandshakeError::IncompatibleVersion,
            ),
            (2, None, HandshakeError::NoMinimumVersion),
            (
                2,
                Some(ProtocolOption::MinimumVersion(3)),
                HandshakeError::IncompatibleMinimumVersion,
            ),
            (3, None, HandshakeError::NoLiveWindow),
            (
                3,
                Some(ProtocolOption::LiveWindow(0)),
                HandshakeError::InvalidLiveWindow,
            ),
            (
                4,
                Some(ProtocolOption::ChunkSize(7)),
                HandshakeError::IncompatibleChunkSize,
            ),
            (
                5,
                Some(ProtocolOption::ContentIntegrityProtectionMethod(9)),
                HandshakeError::IncompatibleContentIntegrityProtectionMethod,
            ),
            (
                6,
                Some(ProtocolOption::MerkleHashTreeFunction(9)),
                HandshakeError::IncompatibleMerkleHashTreeFunction,
            ),
            (
                7,
                Some(ProtocolOption::LiveSignatureAlgorithm(9)),
                HandshakeError::IncompatibleLiveSignatureAlgorithm,
            ),
            (
                8,
                Some(ProtocolOption::ChunksPerSignature(7)),
                HandshakeError::IncompatibleChunksPerSignature,
            ),
            (
                9,
                Some(ProtocolOption::StreamCount(7)),
                HandshakeError::IncompatibleStreamCount,
            ),
        ];
        for (i, replacement, want) in cases {
            let mut opts: Vec<_> = all.to_vec();
            match replacement {
                Some(r) => opts[i] = r,
                None => {
                    opts.remove(i);
                }
            }
            let po = ProtocolOptions::from_slice(&opts);
            assert_eq!(validate_handshake(&s, &po).unwrap_err(), want, "case {i}");
        }
    }

    #[test]
    fn metered_writer_counts_on_finish_only() {
        let mut buf = [0u8; 64];
        let mut out = ChannelMetrics::default();
        let mut w = MeteredWriter::new(&mut buf, 3);
        w.write(&Message::Have(Have { bin: Bin(4) })).unwrap();
        w.reset();
        w.write(&Message::Have(Have { bin: Bin(6) })).unwrap();
        let (n, data) = w.finish(&mut out);
        assert!(n > 0);
        assert_eq!(data, 0);
        assert_eq!(out.have, 1);
        assert_eq!(out.overhead_bytes, 1);
    }
}
