// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Egress: the records of a video swarm, read for a viewer.
//!
//! [`VideoControl::open_stream`] reads the swarm a magnet URI names, as the
//! transfer of its id under the empty salt:
//!
//! - A swarm the node already runs, a capture's, a seed's or another
//!   stream's, is read where it is: the stream holds its slot in the
//!   control's swarm registry. Streams of a swarm the node adds also
//!   publish its transfer on their networks the node runs, each once
//!   joined.
//! - Any other swarm is added with the URI's options and a live window of
//!   [`EgressSettings::window_bytes`], its transfer added and published.
//!   The last stream of it to close removes the transfer, then the swarm,
//!   unless autoseed seeds it ([`autoseed`](crate::autoseed)).
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
//! A stream reads its swarm in the layout of its format
//! ([`StreamFormat`], from the listing's MIME type) and yields the bytes
//! and ends of its records ([`EgressPiece`]):
//!
//! - **Packed** ([`PackedReader`]): the fMP4 format's records, in the
//!   packed chunkstream layout
//!   ([`strims_chunkstream::packed`], [`DEFAULT_SIZE`] byte frames). Unless
//!   it starts at offset 0, it drops the bytes up to the first record end.
//! - **Units** ([`ChunkedReader`]): the [unit format](crate::units), in the
//!   chunked layout ([`strims_chunkstream::chunked`]) in frames of the
//!   swarm's chunk size. It yields the records from the first frame
//!   boundary it reads, after joining at a GOP: the first whole record names
//!   its GOP's sync record, and when that is behind it, a second swarm
//!   reader of the stream, its spare, rewinds there ([`Rewind`]) while the
//!   first reads on to the next sync record. The join yields from the sync
//!   record a player plays from sooner ([`ChunkedReader`]): the GOP's when
//!   the rewind lands in time, else the next. It drops the records before
//!   it, which no reader decodes without their GOP's start.
//!
//! A reader reads from the swarm reader's start, which may fall inside a
//! record. A discontinuity (the swarm reader's, a gap between the offsets
//! of two reads, records the layout lost, or a new swarm) drops the record
//! in progress and the bytes up to the next whole record. Once something
//! was yielded, it yields [`EgressPiece::Discontinuity`] once before that
//! record.
//!
//! So the bytes between two record ends, or between a discontinuity and
//! the next record end, are one whole record as its writer wrote it.
//!
//! An [`EgressStream`] whose swarm closes goes on, after a discontinuity,
//! with the swarm the registry holds in its place under the same transfer
//! id, waiting while that one is being added, and joins it again. With
//! none, it ends with [`VideoError::StreamClosed`].

use std::collections::BTreeSet;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::Poll;

use futures_util::FutureExt;
use futures_util::future::{Either, select};
use strims_base::time::{Duration, Time};
use strims_chunkstream::Frag;
use strims_chunkstream::chunked::{ChunkedDecoder, FrameSizeError, Piece};
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
use crate::media::mime_essence;
use crate::registry::{Added, Claim, SwarmKind};
use crate::units::{self, record::GOP_HEAD_LEN};
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

/// What a [`PackedReader`] or [`ChunkedReader`] has done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReaderMetrics {
    /// Discontinuities: the swarm reader's, gaps between read offsets,
    /// failed reads and new swarms.
    pub discontinuities: u64,
    /// Frames whose header disagreed with the stream: a packed frame's end
    /// outside the frame, or a chunked frame the layout skipped as corrupt.
    pub corrupt_frames: u64,
    /// Places the chunked layout lost records: a record cut short, a
    /// corrupt frame, or a record over the bound.
    pub losses: u64,
    /// Records yielded whole.
    pub records: u64,
    /// Rewinds to a GOP's sync record: those that landed, those the swarm
    /// reader refused or didn't answer in time, and those a later sync
    /// record overtook (dropped once its grace passed).
    pub rewinds: u64,
    pub refused_rewinds: u64,
    pub overtaken_rewinds: u64,
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

/// The layout of a video swarm's stream, which its listing's MIME type
/// names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamFormat {
    /// Records in the packed layout: the fMP4 format's.
    #[default]
    Packed,
    /// The [unit format](crate::units): records in the chunked layout, in
    /// frames of the swarm's chunk size, joined at a GOP.
    Units,
}

impl StreamFormat {
    /// The format of a stream listed with MIME type `mime`: the unit
    /// format's ([`units::MIME_TYPE`]), compared without case, or else
    /// packed records.
    pub fn from_mime(mime: &str) -> StreamFormat {
        if mime_essence(mime).eq_ignore_ascii_case(units::MIME_TYPE) {
            StreamFormat::Units
        } else {
            StreamFormat::Packed
        }
    }
}

/// A swarm reader that can move back within its live window.
pub trait Rewind {
    /// Moves reading back to stream offset `offset`: the offset the next
    /// read starts at, `offset`'s chunk start. `None` when it can't, within
    /// its own deadline; reading then goes on where it was.
    fn rewind(&mut self, offset: u64) -> impl Future<Output = Option<u64>>;

    /// Resolves once a join that races a rewind against the next GOP
    /// ([`ChunkedReader::set_spare`]) has given the rewind long enough
    /// since that GOP's sync record arrived. It starts when called. By
    /// default it resolves at once: the sync record wins as it arrives.
    fn grace(&self) -> impl Future<Output = ()> + 'static {
        std::future::ready(())
    }
}

/// What a [`ChunkedReader`] found, before it is borrowed as an
/// [`EgressPiece`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChunkedStep {
    /// The bytes `buf[start..end]`.
    Data(usize, usize),
    /// The bytes `head[start..end]` of the record a join starts at, held
    /// while it was joined.
    Head(usize, usize),
    End,
    Discontinuity,
}

/// Where a [`ChunkedReader`] is in joining its stream at a GOP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Join {
    /// Waiting for the first whole record.
    Waiting,
    /// Holding a record's head: the record's offset and the bytes of it to
    /// hold. `seeking`: the record may be a sync record the join starts at
    /// ([`Join::Seeking`]); otherwise it is the first whole record.
    Holding {
        offset: u64,
        want: usize,
        seeking: bool,
    },
    /// The reader itself is rewinding to the GOP's sync record.
    Rewinding,
    /// Dropping records until one whose head says it is a sync record:
    /// while a spare's rewind races it, or after a rewind failed.
    Seeking,
    /// Reading on.
    Joined,
}

/// A spare reader's rewind to a GOP's sync record, racing the next one.
struct Race<R> {
    /// The rewind, which owns the spare and answers with it.
    rewind: Pin<Box<dyn Future<Output = (R, Option<u64>)>>>,
    /// The GOP's sync record.
    to: u64,
    /// Once the next sync record's head is held: the rewind's grace.
    grace: Option<Pin<Box<dyn Future<Output = ()>>>>,
}

/// How a read went while a spare's rewind raced it.
enum Raced<R, E> {
    Read(Result<(u64, usize), ReadFailure<E>>),
    /// The rewind moved the spare to this offset first.
    Landed(R, u64),
}

/// Reads the records of a unit-format swarm's stream; see the
/// [module docs](self).
///
/// It joins after it starts and after [`replace`](Self::replace): it holds
/// the first whole record's head, at most [`GOP_HEAD_LEN`] bytes, which
/// names the offset `K` of its GOP's sync record. When `K` is before the
/// record, the join moves back to `K` or waits for a later sync record (a
/// record whose `gop_back` is 0), and yields from there; the records before
/// it are dropped, as no reader decodes them without their GOP's start:
///
/// - With a spare ([`set_spare`](Self::set_spare)), the spare rewinds to
///   `K` while the reader reads on to the next sync record. The join takes
///   the spare when its rewind lands before that record arrives or within
///   the rewind's grace after it ([`Rewind::grace`]): a player starting at
///   a later keyframe waits for media past it before it plays, while a
///   rewound GOP plays at once. Otherwise it drops the spare, which drops
///   its rewind, and starts at that record.
/// - Without one, the join waits for the reader's rewind, and when it
///   fails, for the next sync record.
///
/// It rewinds once a join.
pub struct ChunkedReader<R> {
    reader: R,
    decoder: ChunkedDecoder,
    /// Bytes read and not yet decoded are `buf[start..end]`. Bound: the
    /// size given to [`ChunkedReader::new`].
    buf: Box<[u8]>,
    start: usize,
    end: usize,
    /// The stream offset just past the last read. `None` before the first
    /// read and after a discontinuity: the decoder restarts at the next
    /// read's offset.
    next: Option<u64>,
    join: Join,
    /// The next join's spare reader, until the join starts its rewind.
    spare: Option<Box<R>>,
    /// A join's spare rewind, racing the next sync record. Kept across
    /// calls, so a call dropped while it races loses neither.
    race: Option<Box<Race<R>>>,
    /// The held record's head while joining. Its yielded bytes are
    /// `head[..head_out]` of `head[..head_len]`.
    head: [u8; GOP_HEAD_LEN],
    head_len: usize,
    head_out: usize,
    /// Records whose offset is below it are dropped: those before the sync
    /// record a rewind went back to.
    skip_below: u64,
    /// The record in progress is dropped.
    skipping: bool,
    /// Something was yielded, so a later discontinuity is reported.
    started: bool,
    /// A discontinuity to report before the next piece.
    gap: bool,
    /// The piece found while the discontinuity before it was reported.
    held: Option<ChunkedStep>,
    metrics: ReaderMetrics,
}

impl<R: SwarmRead + Rewind> ChunkedReader<R> {
    /// A reader of `reader`'s stream in frames of `frame_size` bytes, the
    /// swarm's chunk size, that takes up to `buffer_bytes` bytes (at least
    /// 1) per read. Records are at most [`units::MAX_RECORD_LEN`].
    pub fn new(reader: R, frame_size: usize, buffer_bytes: usize) -> Result<Self, FrameSizeError> {
        Ok(ChunkedReader {
            reader,
            decoder: ChunkedDecoder::new(frame_size, units::MAX_RECORD_LEN)?,
            buf: vec![0; buffer_bytes.max(1)].into_boxed_slice(),
            start: 0,
            end: 0,
            next: None,
            join: Join::Waiting,
            spare: None,
            race: None,
            head: [0; GOP_HEAD_LEN],
            head_len: 0,
            head_out: 0,
            skip_below: 0,
            skipping: false,
            started: false,
            gap: false,
            held: None,
            metrics: ReaderMetrics::default(),
        })
    }

    /// Gives the next join `spare`, a second reader of the swarm opened
    /// with the reader and not read since, to race its rewind with (see
    /// the [type docs](Self)). The join ends with it dropped or reading in
    /// the reader's place; [`replace`](Self::replace) drops it too.
    pub fn set_spare(&mut self, spare: R) {
        self.spare = Some(Box::new(spare));
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

    /// Reads from `reader` from now on, as after a discontinuity, and joins
    /// again.
    pub fn replace(&mut self, reader: R) {
        self.reader = reader;
        self.spare = None;
        self.race = None;
        self.start = 0;
        self.end = 0;
        self.next = None;
        self.held = None;
        self.rejoin();
        self.discontinuity();
    }

    pub fn metrics(&self) -> ReaderMetrics {
        ReaderMetrics {
            corrupt_frames: self.decoder.metrics().corrupt,
            ..self.metrics
        }
    }

    fn piece(&self, step: ChunkedStep) -> EgressPiece<'_> {
        match step {
            ChunkedStep::Data(start, end) => {
                EgressPiece::Data(self.buf.get(start..end).unwrap_or_default())
            }
            ChunkedStep::Head(start, end) => {
                EgressPiece::Data(self.head.get(start..end).unwrap_or_default())
            }
            ChunkedStep::End => EgressPiece::RecordEnd,
            ChunkedStep::Discontinuity => EgressPiece::Discontinuity,
        }
    }

    /// [`next`](Self::next), without the borrow.
    async fn advance(&mut self, max: usize) -> Result<ChunkedStep, VideoError> {
        if let Some(step) = self.held.take() {
            return Ok(self.yielded(step));
        }
        let max = max.max(1);
        if self.join == Join::Rewinding {
            // A rewind whose call was dropped: wait for the next sync record.
            self.metrics.refused_rewinds += 1;
            self.seek();
        }
        if let Join::Holding {
            offset,
            want,
            seeking,
        } = self.join
            && self.head_len >= want
        {
            // A call dropped while the join waited on a racing rewind.
            if !self.decide(offset, seeking).await {
                return Ok(self.head(max));
            }
        }
        if self.join == Join::Joined && self.head_out < self.head_len {
            let end = self.head_len.min(self.head_out.saturating_add(max));
            let step = ChunkedStep::Head(self.head_out, end);
            self.head_out = end;
            return Ok(self.report(step));
        }
        loop {
            let room = match self.join {
                Join::Holding { want, .. } => want.saturating_sub(self.head_len).max(1),
                _ => max,
            };
            let limit = self.end.min(self.start.saturating_add(room));
            let input = self.buf.get(self.start..limit).unwrap_or_default();
            let (used, piece) = self.decoder.decode(input);
            self.start += used;
            let step = match piece {
                None if self.start < self.end => continue,
                None => {
                    self.read().await?;
                    continue;
                }
                Some(Piece::Start { offset, len }) => {
                    self.skipping = offset < self.skip_below;
                    let seeking = match self.join {
                        _ if self.skipping => None,
                        Join::Waiting => Some(false),
                        Join::Seeking => Some(true),
                        _ => None,
                    };
                    if let Some(seeking) = seeking {
                        self.join = Join::Holding {
                            offset,
                            want: len.min(GOP_HEAD_LEN),
                            seeking,
                        };
                        self.head_len = 0;
                        self.head_out = 0;
                    }
                    continue;
                }
                Some(Piece::Data(_)) if self.skipping => continue,
                Some(Piece::Data(d)) => {
                    let Join::Holding {
                        offset,
                        want,
                        seeking,
                    } = self.join
                    else {
                        // A data piece is the last of the bytes the call used.
                        let step =
                            ChunkedStep::Data(self.start.saturating_sub(d.len()), self.start);
                        return Ok(self.report(step));
                    };
                    let take = d.len().min(want.saturating_sub(self.head_len));
                    if let (Some(dst), Some(src)) = (
                        self.head.get_mut(self.head_len..self.head_len + take),
                        d.get(..take),
                    ) {
                        dst.copy_from_slice(src);
                    }
                    self.head_len += take;
                    if self.head_len < want || self.decide(offset, seeking).await {
                        continue;
                    }
                    self.head(max)
                }
                Some(Piece::End) if self.skipping => {
                    self.skipping = false;
                    continue;
                }
                Some(Piece::End) => {
                    self.metrics.records += 1;
                    ChunkedStep::End
                }
                Some(Piece::Lost) => {
                    self.metrics.losses += 1;
                    self.skipping = false;
                    self.rejoin_held();
                    self.lose();
                    continue;
                }
            };
            return Ok(self.report(step));
        }
    }

    /// A held head is whole: whether the join drops its record.
    async fn decide(&mut self, offset: u64, seeking: bool) -> bool {
        if seeking {
            self.candidate().await
        } else {
            self.first(offset).await
        }
    }

    /// The first piece of the record a join starts at: its head.
    fn head(&mut self, max: usize) -> ChunkedStep {
        let end = self.head_len.min(max);
        self.head_out = end;
        ChunkedStep::Head(0, end)
    }

    /// The first whole record's head is held: joins there when it is a
    /// sync record, or else moves back to its GOP's sync record or waits
    /// for a later one. Whether the record was dropped.
    async fn first(&mut self, offset: u64) -> bool {
        let head = self.head.get(..self.head_len).unwrap_or_default();
        let to = units::record::gop_back(head)
            .and_then(|back| offset.checked_sub(back))
            .filter(|&k| k < offset);
        let Some(to) = to else {
            self.join = Join::Joined;
            self.spare = None;
            return false;
        };
        if let Some(spare) = self.spare.take() {
            let mut spare = *spare;
            self.race = Some(Box::new(Race {
                rewind: Box::pin(async move {
                    let at = Rewind::rewind(&mut spare, to).await;
                    (spare, at)
                }),
                to,
                grace: None,
            }));
            self.seek();
            return true;
        }
        self.join = Join::Rewinding;
        match Rewind::rewind(&mut self.reader, to).await {
            Some(at) if at <= to => {
                self.metrics.rewinds += 1;
                self.resume(at, to);
            }
            _ => {
                self.metrics.refused_rewinds += 1;
                self.seek();
            }
        }
        true
    }

    /// A seeking join holds a record's head: joins there when it is a sync
    /// record, unless a racing rewind lands within its grace first. Whether
    /// the record was dropped.
    async fn candidate(&mut self) -> bool {
        let head = self.head.get(..self.head_len).unwrap_or_default();
        if units::record::gop_back(head) != Some(0) {
            self.seek();
            return true;
        }
        if let Some(race) = self.race.as_mut() {
            let grace = race
                .grace
                .get_or_insert_with(|| Box::pin(self.reader.grace()));
            let ended = poll_fn(|cx| {
                if let Poll::Ready(r) = race.rewind.as_mut().poll(cx) {
                    return Poll::Ready(Some(r));
                }
                grace.as_mut().poll(cx).map(|()| None)
            })
            .await;
            let to = race.to;
            self.race = None;
            match ended {
                Some((spare, Some(at))) if at <= to => {
                    self.metrics.rewinds += 1;
                    self.reader = spare;
                    self.resume(at, to);
                    return true;
                }
                Some(_) => self.metrics.refused_rewinds += 1,
                None => self.metrics.overtaken_rewinds += 1,
            }
        }
        self.join = Join::Joined;
        false
    }

    /// Reads on from `at`, where a rewind to the sync record at `to` left
    /// the reader.
    fn resume(&mut self, at: u64, to: u64) {
        self.start = 0;
        self.end = 0;
        self.next = Some(at);
        self.decoder.reset(at);
        self.skip_below = to;
        self.skipping = false;
        self.head_len = 0;
        self.head_out = 0;
        self.join = Join::Joined;
    }

    /// Drops the record whose head is held, and waits for a sync record.
    fn seek(&mut self) {
        self.join = Join::Seeking;
        self.skipping = true;
        self.head_len = 0;
        self.head_out = 0;
    }

    /// A step to yield, after the discontinuity before it.
    fn report(&mut self, step: ChunkedStep) -> ChunkedStep {
        if self.gap {
            self.gap = false;
            self.held = Some(step);
            return ChunkedStep::Discontinuity;
        }
        self.yielded(step)
    }

    fn yielded(&mut self, step: ChunkedStep) -> ChunkedStep {
        self.started = true;
        step
    }

    /// A join that was holding a record's head waits for the next record:
    /// the one it held is lost.
    fn rejoin_held(&mut self) {
        if let Join::Holding { seeking, .. } = self.join {
            self.join = if seeking {
                Join::Seeking
            } else {
                Join::Waiting
            };
            self.head_len = 0;
            self.head_out = 0;
        }
    }

    /// Joins again at the next whole record.
    fn rejoin(&mut self) {
        self.join = Join::Waiting;
        self.head_len = 0;
        self.head_out = 0;
        self.skip_below = 0;
        self.skipping = false;
    }

    /// The record in progress is lost.
    fn lose(&mut self) {
        if self.started {
            self.gap = true;
        }
    }

    fn discontinuity(&mut self) {
        self.metrics.discontinuities += 1;
        self.lose();
    }

    /// Reads into the empty buffer. While a spare's rewind races, it
    /// waits for the rewind too: one that lands puts the spare in the
    /// reader's place, as of its offset.
    async fn read(&mut self) -> Result<(), VideoError> {
        let read = match self.race.as_mut() {
            None => Raced::Read(self.reader.read_at(&mut self.buf).await),
            Some(race) => {
                let to = race.to;
                let mut read = pin!(self.reader.read_at(&mut self.buf));
                match select(race.rewind.as_mut(), read.as_mut()).await {
                    Either::Right((r, _)) => Raced::Read(r),
                    Either::Left(((spare, Some(at)), _)) if at <= to => Raced::Landed(spare, at),
                    Either::Left(_) => {
                        // Refused: the join waits for the next sync record.
                        self.race = None;
                        self.metrics.refused_rewinds += 1;
                        Raced::Read(read.await)
                    }
                }
            }
        };
        let r = match read {
            Raced::Landed(spare, at) => {
                let to = self.race.take().map_or(at, |race| race.to);
                self.metrics.rewinds += 1;
                self.reader = spare;
                self.resume(at, to);
                return Ok(());
            }
            Raced::Read(r) => r,
        };
        match r {
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
                    self.skipping = false;
                    self.rejoin_held();
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

/// A host's swarm reader, rewinding within `timeout` and giving a racing
/// rewind `grace` ([`EgressSettings`]).
struct HostReader<R, T> {
    reader: R,
    timer: T,
    timeout: Duration,
    grace: Duration,
}

impl<R: SwarmRead, T: Timer> SwarmRead for HostReader<R, T> {
    type Error = R::Error;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<R::Error>> {
        self.reader.read_at(out).await
    }
}

impl<R: SwarmRead, T: Timer> Rewind for HostReader<R, T> {
    async fn rewind(&mut self, offset: u64) -> Option<u64> {
        let deadline = self.timer.now() + self.timeout;
        within(
            &self.timer,
            deadline,
            rewind_reader(&mut self.reader, offset),
        )
        .await
    }

    fn grace(&self) -> impl Future<Output = ()> + 'static {
        self.timer.sleep_until(self.timer.now() + self.grace)
    }
}

/// `rewind`'s answer, or `None` once `deadline` passes, which drops it.
async fn within<T: Timer>(
    timer: &T,
    deadline: Time,
    rewind: impl Future<Output = Option<u64>>,
) -> Option<u64> {
    let rewind = pin!(rewind);
    let sleep = pin!(timer.sleep_until(deadline));
    match select(rewind, sleep).await {
        Either::Left((at, _)) => at,
        Either::Right(_) => None,
    }
}

/// Moves `reader` back to stream offset `offset`: the offset reading
/// resumes from, or `None` when the swarm refuses, in which case the join
/// waits for the next sync record.
async fn rewind_reader<R: SwarmRead>(reader: &mut R, offset: u64) -> Option<u64> {
    SwarmRead::rewind(reader, offset).await.ok()
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

/// A stream's reader, in its format's layout.
enum Reader<R, T> {
    Packed(PackedReader<R>),
    Units(ChunkedReader<HostReader<R, T>>),
}

/// A step of either reader.
#[derive(Clone, Copy, Debug)]
enum AnyStep {
    Packed(Step),
    Units(ChunkedStep),
}

impl<R: SwarmRead, T: Timer> Reader<R, T> {
    async fn advance(&mut self, max: usize) -> Result<AnyStep, VideoError> {
        match self {
            Reader::Packed(r) => r.advance(max).await.map(AnyStep::Packed),
            Reader::Units(r) => r.advance(max).await.map(AnyStep::Units),
        }
    }

    fn piece(&self, step: AnyStep) -> EgressPiece<'_> {
        match (self, step) {
            (Reader::Packed(r), AnyStep::Packed(step)) => r.piece(step),
            (Reader::Units(r), AnyStep::Units(step)) => r.piece(step),
            // invariant: a reader's steps are its own; the arm is unreachable.
            _ => EgressPiece::Discontinuity,
        }
    }
}

/// A viewer's stream of a video swarm's records; see the
/// [module docs](self). Dropping it releases the swarm.
pub struct EgressStream<K: Kv, H: Host, V: VideoHost, T: Timer> {
    reader: Reader<<H as SwarmHost>::Reader, T>,
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
                    match &mut self.reader {
                        Reader::Packed(r) => r.replace(reader),
                        Reader::Units(r) => {
                            r.replace(s.host_reader(reader));
                            if let Some(spare) = s.spare_reader(swarm).await {
                                r.set_spare(spare);
                            }
                        }
                    }
                    s.metrics.borrow_mut().reopens += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn metrics(&self) -> ReaderMetrics {
        match &self.reader {
            Reader::Packed(r) => r.metrics(),
            Reader::Units(r) => r.metrics(),
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// `reader`, rewinding within the egress settings' timeout and grace.
    fn host_reader(
        &self,
        reader: <H as SwarmHost>::Reader,
    ) -> HostReader<<H as SwarmHost>::Reader, T> {
        HostReader {
            reader,
            timer: self.timer.clone(),
            timeout: self.config.egress.rewind_timeout,
            grace: self.config.egress.rewind_grace,
        }
    }

    /// A unit-format stream's spare reader of `swarm`
    /// ([`ChunkedReader::set_spare`]), opened with its reader. `None` when
    /// it can't be opened: the join then waits for the reader's rewind.
    async fn spare_reader(
        &self,
        swarm: Swarm<H>,
    ) -> Option<HostReader<<H as SwarmHost>::Reader, T>> {
        match self.host.open_reader(swarm).await {
            Ok(r) => Some(self.host_reader(r)),
            Err(e) => {
                let _ = self.host_failed("opening a video stream's spare reader", &e);
                None
            }
        }
    }

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
                if matches!(e.kind, SwarmKind::Egress | SwarmKind::Seed) {
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

    /// Adds the swarm of egress or seed slot `slot` of `t`, and its
    /// transfer.
    pub(crate) async fn add_stream_swarm(
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
    /// networks `network_keys` names in the layout of `format`; see the
    /// [module docs](self).
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
        format: StreamFormat,
    ) -> Result<EgressStream<K, H, V, T>, VideoError> {
        let s = &self.shared;
        let (uri, options) = check_stream(&s.config.egress, swarm_uri, network_keys)?;
        let t = TransferId::new(&uri.id, b"");
        let mut hold = StreamHold::new(s, t)?;
        hold.claim = Some(s.claim_stream(t, uri.id, options.clone(), network_keys)?);
        let swarm = hold.next_swarm().await.ok_or(VideoError::StreamClosed)?;
        let reader = s
            .host
            .open_reader(swarm)
            .await
            .map_err(|e| s.host_failed("opening a video stream reader", &e))?;
        let buffer = s.config.egress.read_buffer_bytes;
        let reader = match format {
            StreamFormat::Packed => Reader::Packed(PackedReader::new(reader, buffer)),
            StreamFormat::Units => {
                let mut r = ChunkedReader::new(s.host_reader(reader), options.chunk_size, buffer)
                    .map_err(|_| VideoError::SwarmOption("chunk size"))?;
                if let Some(spare) = s.spare_reader(swarm).await {
                    r.set_spare(spare);
                }
                Reader::Units(r)
            }
        };
        Ok(EgressStream { reader, hold })
    }
}

#[cfg(test)]
#[path = "egress_test.rs"]
mod tests;
