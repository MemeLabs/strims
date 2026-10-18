// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Egress: the records of a video swarm, read for a viewer.
//!
//! [`VideoControl::open_stream`] reads the swarm a magnet URI names, as the
//! transfer of its id under the empty salt:
//!
//! - A swarm the node already runs, a capture's or another stream's, is
//!   read where it is: the stream holds its slot in the control's swarm
//!   registry. Streams of a swarm the node adds also publish its transfer
//!   on their networks the node runs, each once joined.
//! - Any other swarm is added with the URI's options and a live window of
//!   [`EgressSettings::window_bytes`], its transfer added and published.
//!   The last stream of it to close removes the transfer, then the swarm.
//!
//! Its URI is at most [`EgressSettings::max_uri_bytes`] and its options
//! within the settings' bounds: the chunk size, chunks per signature (a
//! power of two that fits the window, under a Merkle tree) and stream
//! count, and integrity a node supports. A stream names at most
//! [`EgressSettings::max_network_keys`] networks. The node reads at most
//! [`EgressSettings::max_swarms`] added swarms and
//! [`EgressSettings::max_streams`] streams at once.
//!
//! # Reading
//!
//! A video swarm carries records in the packed chunkstream layout
//! ([`strims_chunkstream::packed`], [`DEFAULT_SIZE`] byte frames). A
//! [`PackedReader`] yields their bytes and ends ([`EgressPiece`]):
//!
//! - It reads from the swarm reader's start, which may fall inside a
//!   record: unless it starts at offset 0, it drops the bytes up to the
//!   first record end.
//! - A discontinuity (the swarm reader's, a gap between the offsets of two
//!   reads, a corrupt frame, or a new swarm) drops the record in
//!   progress and the bytes up to the next whole record. Once something
//!   was yielded, it yields [`EgressPiece::Discontinuity`] once before that
//!   record.
//!
//! So the bytes between two record ends, or between a discontinuity and
//! the next record end, are one whole record as its writer wrote it.
//!
//! An [`EgressStream`] whose swarm closes goes on, after a discontinuity,
//! with the swarm the registry holds in its place under the same transfer
//! id, waiting while that one is being added. With none, it ends with
//! [`VideoError::StreamClosed`].

use std::collections::BTreeSet;
use std::future::poll_fn;
use std::rc::Rc;

use futures_util::FutureExt;
use strims_chunkstream::Frag;
use strims_chunkstream::packed::{DEFAULT_SIZE, PackedDecoder};
use strims_crypto::ED25519_PUBLIC_KEY_SIZE;
use strims_directory::Host;
use strims_kv::Kv;
use strims_ppspp::codec::ProtocolOptionType;
use strims_ppspp::integrity::{LiveSignatureAlgorithm, ProtectionMethod};
use strims_ppspp::{SwarmId, SwarmOptions, Uri};
use strims_rpc::Timer;
use strims_transfer::{ReadFailure, SwarmHost, SwarmRead, TransferId};

use crate::config::EgressSettings;
use crate::control::{Shared, Swarm};
use crate::registry::{Added, Claim, SwarmKind};
use crate::{VideoControl, VideoError, VideoHost};

/// A piece of a video swarm's records, in stream order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EgressPiece<'a> {
    /// Bytes of the record in progress.
    Data(&'a [u8]),
    /// The record in progress ended: its bytes since the last end or
    /// discontinuity are one whole record.
    RecordEnd,
    /// The stream skipped: bytes yielded since the last end are no record.
    /// The next piece begins a whole record.
    Discontinuity,
}

/// What a [`PackedReader`] has done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReaderMetrics {
    /// Discontinuities: the swarm reader's, gaps between read offsets,
    /// failed reads and new swarms.
    pub discontinuities: u64,
    /// Frames whose header flagged an end outside the frame.
    pub corrupt_frames: u64,
    /// Records yielded whole.
    pub records: u64,
}

/// What [`PackedReader::advance`] found, before it is borrowed as an
/// [`EgressPiece`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// The bytes `buf[start..end]`.
    Data(usize, usize),
    End,
    Discontinuity,
}

/// Reads the records of a video swarm's stream; see the
/// [module docs](self).
pub struct PackedReader<R> {
    reader: R,
    decoder: PackedDecoder,
    /// Bytes read and not yet decoded are `buf[start..end]`. Bound: the
    /// size given to [`PackedReader::new`].
    buf: Box<[u8]>,
    start: usize,
    end: usize,
    /// The stream offset just past the last read. `None` before the first
    /// read and after a discontinuity: the decoder restarts at the next
    /// read's offset.
    next: Option<u64>,
    /// The fragments up to the next end are dropped: the tail of a record
    /// whose start wasn't read.
    skip: bool,
    /// Data of the record in progress was yielded.
    in_record: bool,
    /// Something was yielded, so a later discontinuity is reported.
    started: bool,
    /// A discontinuity to report before the next piece.
    gap: bool,
    /// The piece found while the discontinuity before it was reported.
    held: Option<Step>,
    metrics: ReaderMetrics,
}

impl<R: SwarmRead> PackedReader<R> {
    /// A reader of `reader`'s stream that takes up to `buffer_bytes` bytes
    /// (at least 1) per read.
    pub fn new(reader: R, buffer_bytes: usize) -> Self {
        // invariant: DEFAULT_SIZE is a valid frame size.
        #[allow(clippy::expect_used)]
        let decoder = PackedDecoder::new(DEFAULT_SIZE).expect("the default frame size");
        PackedReader {
            reader,
            decoder,
            buf: vec![0; buffer_bytes.max(1)].into_boxed_slice(),
            start: 0,
            end: 0,
            next: None,
            skip: false,
            in_record: false,
            started: false,
            gap: false,
            held: None,
            metrics: ReaderMetrics::default(),
        }
    }

    /// The next piece of the records; a data piece holds at most `max`
    /// bytes (at least 1).
    ///
    /// # Errors
    ///
    /// [`VideoError::StreamClosed`] once the swarm reader closes; then
    /// every call fails so. [`VideoError::StreamRead`] for a failed read,
    /// after which reading goes on as after a discontinuity.
    pub async fn next(&mut self, max: usize) -> Result<EgressPiece<'_>, VideoError> {
        let step = self.advance(max).await?;
        Ok(self.piece(step))
    }

    /// Reads from `reader` from now on, as after a discontinuity.
    pub fn replace(&mut self, reader: R) {
        self.reader = reader;
        self.start = 0;
        self.end = 0;
        self.next = None;
        self.held = None;
        self.discontinuity();
    }

    pub fn metrics(&self) -> ReaderMetrics {
        self.metrics
    }

    pub(crate) fn piece(&self, step: Step) -> EgressPiece<'_> {
        match step {
            Step::Data(start, end) => {
                EgressPiece::Data(self.buf.get(start..end).unwrap_or_default())
            }
            Step::End => EgressPiece::RecordEnd,
            Step::Discontinuity => EgressPiece::Discontinuity,
        }
    }

    /// [`next`](Self::next), without the borrow.
    pub(crate) async fn advance(&mut self, max: usize) -> Result<Step, VideoError> {
        if let Some(step) = self.held.take() {
            return Ok(self.yielded(step));
        }
        let max = max.max(1);
        loop {
            // With the input used up, a decode of nothing still yields the
            // end of a record whose last byte was read.
            let limit = self.end.min(self.start.saturating_add(max));
            let input = self.buf.get(self.start..limit).unwrap_or_default();
            let (used, frag) = self.decoder.decode(input);
            self.start += used;
            let step = match frag {
                None if self.start < self.end => continue,
                None => {
                    self.read().await?;
                    continue;
                }
                // A data fragment is the last of the bytes the call used.
                Some(Frag::Data(d)) if !self.skip => {
                    Step::Data(self.start.saturating_sub(d.len()), self.start)
                }
                Some(Frag::Data(_)) => continue,
                Some(Frag::End) if self.skip => {
                    self.skip = false;
                    continue;
                }
                Some(Frag::End) => Step::End,
                Some(Frag::Corrupt) => {
                    self.metrics.corrupt_frames += 1;
                    self.lose();
                    self.skip = true;
                    continue;
                }
            };
            if self.gap {
                self.gap = false;
                self.held = Some(step);
                return Ok(Step::Discontinuity);
            }
            return Ok(self.yielded(step));
        }
    }

    fn yielded(&mut self, step: Step) -> Step {
        self.started = true;
        match step {
            Step::Data(..) => self.in_record = true,
            Step::End => {
                self.in_record = false;
                self.metrics.records += 1;
            }
            Step::Discontinuity => {}
        }
        step
    }

    /// The record in progress is lost.
    fn lose(&mut self) {
        if self.started {
            self.gap = true;
        }
        self.in_record = false;
    }

    fn discontinuity(&mut self) {
        self.metrics.discontinuities += 1;
        self.lose();
    }

    /// Reads into the empty buffer.
    async fn read(&mut self) -> Result<(), VideoError> {
        match self.reader.read_at(&mut self.buf).await {
            Ok((offset, n)) => {
                let n = n.min(self.buf.len());
                if n == 0 {
                    // A swarm read returns data or fails, so a read of
                    // nothing ends the stream.
                    return Err(VideoError::StreamClosed);
                }
                self.start = 0;
                self.end = n;
                let expected = self.next.replace(offset.saturating_add(n as u64));
                if expected != Some(offset) {
                    if expected.is_some() {
                        self.discontinuity();
                    }
                    self.decoder.reset(offset);
                    self.skip = offset != 0;
                }
                Ok(())
            }
            Err(ReadFailure::Discontinuity) => {
                self.next = None;
                self.discontinuity();
                Ok(())
            }
            Err(ReadFailure::Closed) => Err(VideoError::StreamClosed),
            Err(ReadFailure::Failed(e)) => {
                self.next = None;
                self.discontinuity();
                Err(VideoError::StreamRead(e.to_string()))
            }
        }
    }
}

/// Checks a stream's swarm URI and network keys against `s`: the URI, and
/// the options to add its swarm with. See the [module docs](self).
pub(crate) fn check_stream(
    s: &EgressSettings,
    swarm_uri: &str,
    network_keys: &[Vec<u8>],
) -> Result<(Uri, SwarmOptions), VideoError> {
    if swarm_uri.len() > s.max_uri_bytes {
        return Err(VideoError::UriTooLong);
    }
    let uri = Uri::parse(swarm_uri)?;
    if network_keys.len() > s.max_network_keys {
        return Err(VideoError::NetworkKeyCount);
    }
    if network_keys
        .iter()
        .any(|k| k.len() > s.max_network_key_bytes)
    {
        return Err(VideoError::NetworkKeyTooLong);
    }
    let options = stream_options(s, &uri)?;
    Ok((uri, options))
}

/// The options to add the swarm of `uri` with, if they are within `s`.
fn stream_options(s: &EgressSettings, uri: &Uri) -> Result<SwarmOptions, VideoError> {
    // The URI's values of these options are truncated to a byte.
    for (t, name) in [
        (
            ProtocolOptionType::ContentIntegrityProtectionMethod,
            "integrity protection method",
        ),
        (
            ProtocolOptionType::MerkleHashTreeFunction,
            "merkle hash tree function",
        ),
        (
            ProtocolOptionType::LiveSignatureAlgorithm,
            "live signature algorithm",
        ),
    ] {
        if uri.options.get(t).is_some_and(|v| v > u64::from(u8::MAX)) {
            return Err(VideoError::SwarmOption(name));
        }
    }
    let o = uri
        .options
        .swarm_options()
        .assign_defaults(&SwarmOptions::default_options());
    let within = |v: usize, min: u32, max: u32| {
        usize::try_from(min).is_ok_and(|min| v >= min)
            && usize::try_from(max).is_ok_and(|max| v <= max)
    };
    if !within(o.chunk_size, s.min_chunk_size, s.max_chunk_size) {
        return Err(VideoError::SwarmOption("chunk size"));
    }
    if !within(o.chunks_per_signature, 1, s.max_chunks_per_signature) {
        return Err(VideoError::SwarmOption("chunks per signature"));
    }
    if !within(
        o.stream_count,
        1,
        s.max_stream_count.min(u32::from(u16::MAX)),
    ) {
        return Err(VideoError::SwarmOption("stream count"));
    }
    let i = o.integrity;
    if ![
        ProtectionMethod::NONE,
        ProtectionMethod::MERKLE_TREE,
        ProtectionMethod::SIGN_ALL,
    ]
    .contains(&i.protection_method)
    {
        return Err(VideoError::SwarmOption("integrity protection method"));
    }
    if i.merkle_hash_tree_function.hash_size().is_none() {
        return Err(VideoError::SwarmOption("merkle hash tree function"));
    }
    if i.live_signature_algorithm.signature_size().is_none() {
        return Err(VideoError::SwarmOption("live signature algorithm"));
    }
    if i.live_signature_algorithm == LiveSignatureAlgorithm::ED25519
        && uri.id.as_bytes().len() != ED25519_PUBLIC_KEY_SIZE
    {
        return Err(VideoError::SwarmOption("swarm id"));
    }
    let live_window = live_window(s.window_bytes, o.chunk_size);
    if i.protection_method == ProtectionMethod::MERKLE_TREE
        && (!o.chunks_per_signature.is_power_of_two() || o.chunks_per_signature > live_window)
    {
        return Err(VideoError::SwarmOption("chunks per signature"));
    }
    Ok(SwarmOptions { live_window, ..o })
}

/// The live window of a swarm of `chunk_size` byte chunks: the most
/// chunks, a power of two up to 2^31, that fit in `window_bytes`, and at
/// least one.
pub(crate) fn live_window(window_bytes: usize, chunk_size: usize) -> usize {
    let n = (window_bytes / chunk_size.max(1)).clamp(1, 1 << 31);
    1 << n.ilog2()
}

/// A stream's place among the node's streams, and its claim on its swarm's
/// slot. Dropping it releases both.
struct StreamHold<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
    transfer_id: TransferId,
    claim: Option<Claim>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> StreamHold<K, H, V, T> {
    /// Counts a stream of `transfer_id`, within the bound.
    fn new(shared: &Rc<Shared<K, H, V, T>>, transfer_id: TransferId) -> Result<Self, VideoError> {
        let max = shared.config.egress.max_streams;
        shared.with_swarms(|r| {
            if r.streams >= max {
                return Err(VideoError::TooManyStreams);
            }
            r.streams += 1;
            Ok(())
        })?;
        Ok(StreamHold {
            shared: shared.clone(),
            transfer_id,
            claim: None,
        })
    }

    /// The swarm the stream reads next: its slot's, once it has one other
    /// than the one the stream read. `None` once there's none.
    async fn next_swarm(&mut self) -> Option<Swarm<H>> {
        let t = self.transfer_id;
        let shared = &self.shared;
        let claim = self.claim.as_mut()?;
        poll_fn(|cx| shared.with_swarms(|r| r.poll_swarm(&t, claim, cx))).await
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for StreamHold<K, H, V, T> {
    fn drop(&mut self) {
        let t = self.transfer_id;
        let claim = self.claim;
        let removed = self.shared.with_swarms(|r| {
            r.streams = r.streams.saturating_sub(1);
            claim.and_then(|c| r.release(&t, c))
        });
        if let Some(e) = removed {
            self.shared.drop_swarm(t, e);
        }
    }
}

/// A viewer's stream of a video swarm's records; see the
/// [module docs](self). Dropping it releases the swarm.
pub struct EgressStream<K: Kv, H: Host, V: VideoHost, T: Timer> {
    reader: PackedReader<<H as SwarmHost>::Reader>,
    hold: StreamHold<K, H, V, T>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> EgressStream<K, H, V, T> {
    /// The transfer the stream reads: its swarm id under the empty salt.
    pub fn transfer_id(&self) -> TransferId {
        self.hold.transfer_id
    }

    /// The next piece of the records; a data piece holds at most `max`
    /// bytes (at least 1). When the swarm closes, this reads the swarm
    /// that replaced it, after a [`EgressPiece::Discontinuity`].
    ///
    /// # Errors
    ///
    /// [`VideoError::StreamClosed`] once the swarm is gone with none in
    /// its place; [`VideoError::StreamRead`] for a failed read, after
    /// which reading may go on; [`VideoError::Host`] when a reader of a
    /// new swarm can't be opened.
    pub async fn next(&mut self, max: usize) -> Result<EgressPiece<'_>, VideoError> {
        loop {
            match self.reader.advance(max).await {
                Ok(step) => return Ok(self.reader.piece(step)),
                Err(VideoError::StreamClosed) => {
                    let Some(swarm) = self.hold.next_swarm().await else {
                        return Err(VideoError::StreamClosed);
                    };
                    let s = &self.hold.shared;
                    let reader = s
                        .host
                        .open_reader(swarm)
                        .await
                        .map_err(|e| s.host_failed("opening a video stream reader", &e))?;
                    self.reader.replace(reader);
                    s.metrics.borrow_mut().reopens += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn metrics(&self) -> ReaderMetrics {
        self.reader.metrics()
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Takes a stream's claim on the slot of `t`: the slot there is, or a
    /// new egress slot whose swarm a task adds.
    fn claim_stream(
        self: &Rc<Self>,
        t: TransferId,
        id: SwarmId,
        options: SwarmOptions,
        network_keys: &[Vec<u8>],
    ) -> Result<Claim, VideoError> {
        let networks: BTreeSet<Vec<u8>> = network_keys
            .iter()
            .filter(|k| self.network.network(k).is_some())
            .cloned()
            .collect();
        let max = self.config.egress.max_swarms;
        let (claim, added) = self.with_swarms(|r| {
            if let Some(e) = r.get_mut(&t) {
                if e.kind == SwarmKind::Egress {
                    e.networks.extend(networks);
                }
                return r
                    .claim(&t)
                    .map(|c| (c, false))
                    .ok_or(VideoError::StreamClosed);
            }
            if r.count(SwarmKind::Egress) >= max {
                return Err(VideoError::TooManySwarms);
            }
            let c = r.insert(
                t,
                None,
                id.clone(),
                options.clone(),
                SwarmKind::Egress,
                networks,
                1,
            );
            Ok((c, true))
        })?;
        if added {
            self.spawn(
                self.clone()
                    .add_stream_swarm(t, claim.slot, id, options)
                    .boxed_local(),
            );
        } else {
            self.publish_joined(&t);
        }
        Ok(claim)
    }

    /// Adds the swarm of egress slot `slot` of `t`, and its transfer.
    async fn add_stream_swarm(
        self: Rc<Self>,
        t: TransferId,
        slot: u64,
        id: SwarmId,
        options: SwarmOptions,
    ) {
        let swarm = match self.host.add_swarm(id.clone(), options).await {
            Ok(swarm) => swarm,
            Err(e) => {
                self.host_failed("adding a video swarm", &e);
                self.with_swarms(|r| r.remove_slot(&t, slot));
                return;
            }
        };
        match self.with_swarms(|r| r.set_swarm(&t, slot, swarm)) {
            Added::Kept => {
                self.transfers.add(swarm, &id, Vec::new());
                self.publish_joined(&t);
            }
            Added::Unwanted => self.remove_host_swarm(swarm).await,
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// A stream of the records of the swarm `swarm_uri` names, read on the
    /// networks `network_keys` names; see the [module docs](self).
    ///
    /// # Errors
    ///
    /// The URI or network keys are refused (see the module docs), the node
    /// has its most swarms or streams, or the swarm couldn't be added or
    /// read.
    pub async fn open_stream(
        &self,
        swarm_uri: &str,
        network_keys: &[Vec<u8>],
    ) -> Result<EgressStream<K, H, V, T>, VideoError> {
        let s = &self.shared;
        let (uri, options) = check_stream(&s.config.egress, swarm_uri, network_keys)?;
        let t = TransferId::new(&uri.id, b"");
        let mut hold = StreamHold::new(s, t)?;
        hold.claim = Some(s.claim_stream(t, uri.id, options, network_keys)?);
        let swarm = hold.next_swarm().await.ok_or(VideoError::StreamClosed)?;
        let reader = s
            .host
            .open_reader(swarm)
            .await
            .map_err(|e| s.host_failed("opening a video stream reader", &e))?;
        Ok(EgressStream {
            reader: PackedReader::new(reader, s.config.egress.read_buffer_bytes),
            hold,
        })
    }
}

#[cfg(test)]
#[path = "egress_test.rs"]
mod tests;
