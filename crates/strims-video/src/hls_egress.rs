// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS egress: video swarms served to external players as HLS playlists.
//!
//! [`VideoControl::open_hls`] reads the swarm a magnet URI names as an
//! [`EgressStream`], counted among the egress bounds
//! ([`EgressSettings`](crate::EgressSettings)), and serves its records on
//! the driver's [`HlsRoutes`] under the profile's public key: the playlist
//! at [`playlist_path`]`(profile key, transfer id)`, with its inits and
//! segments beside it. The control registers the profile when it is
//! built; a profile whose key isn't [`KEY_LEN`](crate::hls::KEY_LEN)
//! bytes has no HLS egress.
//!
//! # Opening
//!
//! A stream opens only while the driver serves HLS
//! ([`HlsSettings::serve`](crate::HlsSettings::serve)) and the device's
//! HLS config is enabled. Its playlist URL is the config's public server
//! address, or [`HlsSettings::base_url`](crate::HlsSettings::base_url)
//! without one, less a trailing `/`, followed by the playlist path. The
//! URL is made at each open, so a changed address names the same streams.
//! A transfer has one HLS stream: opening it again answers that stream's
//! URL. The node serves at most
//! [`HlsSettings::max_streams`](crate::HlsSettings::max_streams) streams.
//!
//! # Formats
//!
//! A stream reads its swarm in the format its open names
//! ([`StreamFormat`], from the listing's MIME type), and its playlist's
//! target duration is
//! [`HlsSettings::target_duration`](crate::HlsSettings::target_duration)
//! for its life:
//!
//! - **fMP4 records** ([`StreamFormat::Packed`]). A record of at most
//!   [`HlsSettings::max_record_bytes`](crate::HlsSettings::max_record_bytes)
//!   is held while it arrives, and once whole ([`EgressPiece::RecordEnd`]),
//!   if it is an fMP4 record ([`split_record`]) with media time
//!   ([`media_duration`]), its init and media become segments: one
//!   lasting the media's time, or, when that would round above the target
//!   ([`fits_target`]), runs of its fragments within the target, which
//!   after the first start mid-GOP. A record that isn't fMP4 with media
//!   time is skipped, as is one with a fragment longer than the target,
//!   from that fragment on; one over the bound is dropped.
//! - **The unit format** ([`StreamFormat::Units`]). Each whole record of
//!   at most [`units::MAX_RECORD_LEN`] goes through a [`UnitReader`] into
//!   an [`HlsMuxer`], whose segments the playlist lists (see
//!   [`hls`](crate::hls#segments)). The reader's gaps are the muxer's, so
//!   a lost chunk costs the units up to the next keyframe. New configs,
//!   which also come when a writer starts the stream over, reset the
//!   muxer. An END record ends the segment in progress and the playlist
//!   (`EXT-X-ENDLIST`); a later session's segment takes the end back off
//!   (see [`Playlist::push_segment`](crate::hls::Playlist::push_segment)).
//!   A record over the bound is dropped.
//!
//! Once a segment was served, a skipped or dropped fMP4 record, a
//! discontinuity of an fMP4 stream, or a gap or reset of a unit-format
//! stream marks the next segment discontinuous.
//! [`Metrics`](crate::Metrics) counts the segments and skipped records.
//!
//! # Lifetime
//!
//! A stream is the node's, not its opener's: it lasts until
//! [`VideoControl::close_hls`] (`CloseStream`) closes it, no request reaches
//! it for [`HlsSettings::idle_timeout`](crate::HlsSettings::idle_timeout)
//! (looked for every
//! [`HlsSettings::sweep_interval`](crate::HlsSettings::sweep_interval) by
//! [`VideoControl::run`]), the HLS config is disabled, or its stream fails.
//! Closing unregisters its routes, so requests of it answer 404, and
//! releases its swarm. [`VideoEvent::HlsStreamOpened`] and
//! [`VideoEvent::HlsStreamClosed`] report each stream.

use std::collections::BTreeMap;
use std::future::Future;
use std::mem;
use std::ops::Range;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll};

use futures_util::FutureExt;
use futures_util::future::{AbortHandle, abortable};
use strims_base::time::{Duration, Time};
use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::profile::v1::Profile;
use strims_proto::video::v1::HlsEgressConfig;
use strims_rpc::Timer;
use strims_transfer::TransferId;

use crate::control::Shared;
use crate::egress::{EgressPiece, EgressStream, StreamFormat, check_stream};
use crate::fmp4::{InitInfo, media_duration, parse_init, split_record};
use crate::hls::{
    HlsMuxer, HlsMuxerConfig, HlsProfile, HlsRoutes, HlsStream, PlaylistConfig, RouteError,
    fits_target, playlist_path, target_secs,
};
use crate::media::Muxer;
use crate::units::{self, ReaderEvent, UnitReader};
use crate::{HlsSettings, VideoControl, VideoError, VideoEvent, VideoHost};

/// Why an HLS stream closed.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum HlsCloseReason {
    /// [`VideoControl::close_hls`] closed it.
    Closed,
    /// No request reached it for
    /// [`HlsSettings::idle_timeout`](crate::HlsSettings::idle_timeout).
    Idle,
    /// The HLS config was disabled.
    Disabled,
    /// Its stream failed.
    Failed(VideoError),
}

/// An HLS stream of the node, as [`VideoControl::hls_streams`] lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsStreamInfo {
    /// The transfer it reads: its swarm id under the empty salt.
    pub transfer_id: TransferId,
    /// Its playlist's path on the HLS routes ([`playlist_path`]).
    pub path: String,
    /// The time of its latest successful request, or its open time.
    pub last_access: Time,
    /// Its records are being read: false while its swarm is opened.
    pub reading: bool,
}

/// The control's HLS streams, and the profile's registration they go
/// under.
pub(crate) struct HlsEgress {
    /// The profile's registration on the driver's routes, or why it has
    /// none.
    profile: Result<HlsProfile, RouteError>,
    /// Bound: [`HlsSettings::max_streams`](crate::HlsSettings::max_streams).
    streams: BTreeMap<TransferId, HlsEntry>,
    /// Tells an entry from earlier ones of its transfer.
    next_id: u64,
    /// HLS config writes seen, so an open notices one during its read of
    /// the config.
    config_writes: u64,
}

struct HlsEntry {
    id: u64,
    /// Its only strong reference: dropping the entry unregisters it.
    stream: Rc<HlsStream>,
    /// Stops its reader task; `None` while its swarm is opened.
    reader: Option<AbortHandle>,
}

impl HlsEgress {
    /// Registers `profile`'s key on `routes`.
    pub(crate) fn new(routes: &HlsRoutes, profile: &Profile) -> HlsEgress {
        let key = profile.key.as_ref().map_or(&[][..], |k| &k.public[..]);
        let profile = routes.profile(key);
        if let Err(e) = &profile {
            tracing::warn!(error = %e, "the profile has no HLS egress");
        }
        HlsEgress {
            profile,
            streams: BTreeMap::new(),
            next_id: 0,
            config_writes: 0,
        }
    }

    /// Whether the profile is registered on the routes.
    pub(crate) fn routed(&self) -> bool {
        self.profile.is_ok()
    }

    /// The entry of `t` if it is entry `id`, taken out.
    fn take(&mut self, t: &TransferId, id: u64) -> Option<HlsEntry> {
        self.streams.remove_entry(t).and_then(|(t, e)| {
            if e.id == id {
                Some(e)
            } else {
                self.streams.insert(t, e);
                None
            }
        })
    }
}

/// The run loop's wait for its next sweep of idle HLS streams, armed while
/// there are streams.
pub(crate) type HlsSweep<T> = Option<Pin<Box<<T as Timer>::Sleep>>>;

/// What a piece did to an HLS stream ([`StreamFeed::feed`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fed {
    /// Segments pushed.
    pub(crate) segments: u64,
    /// Records skipped or dropped.
    pub(crate) skipped: u64,
}

/// Turns a stream's pieces into the segments of an [`HlsStream`], in its
/// format; see the [module docs](self#formats).
pub(crate) enum StreamFeed {
    Records(Box<RecordFeed>),
    Units(Box<UnitFeed>),
}

impl StreamFeed {
    /// A feed of a stream of `format` under `settings`.
    pub(crate) fn new(format: StreamFormat, settings: &HlsSettings) -> StreamFeed {
        let target = target_secs(settings.target_duration);
        match format {
            StreamFormat::Packed => {
                StreamFeed::Records(Box::new(RecordFeed::new(settings.max_record_bytes, target)))
            }
            StreamFormat::Units => StreamFeed::Units(Box::new(UnitFeed::new(HlsMuxerConfig {
                target,
                fragment_duration: settings.fragment_duration,
                max_segment_bytes: settings.max_record_bytes,
            }))),
        }
    }

    /// Takes the stream's next piece into `stream`.
    pub(crate) fn feed(&mut self, piece: EgressPiece<'_>, stream: &HlsStream) -> Fed {
        match self {
            StreamFeed::Records(f) => f.feed(piece, stream),
            StreamFeed::Units(f) => f.feed(piece, stream),
        }
    }
}

/// A record held while it arrives, up to a bound.
struct Held {
    /// Bound: `max`, also of its capacity, which is kept from record to
    /// record.
    record: Vec<u8>,
    max: usize,
    /// The record in progress went over the bound, so its bytes up to its
    /// end are dropped.
    dropping: bool,
}

impl Held {
    fn new(max: usize) -> Held {
        Held {
            record: Vec::new(),
            max,
            dropping: false,
        }
    }

    /// Appends `d` to the record: false when that takes it over the bound,
    /// which drops it.
    fn data(&mut self, d: &[u8]) -> bool {
        if self.dropping {
            return true;
        }
        let need = self.record.len().saturating_add(d.len());
        if need > self.max {
            self.record.clear();
            self.dropping = true;
            return false;
        }
        if need > self.record.capacity() {
            let grown = self.record.capacity().saturating_mul(2);
            let target = need.max(grown).min(self.max);
            self.record.reserve_exact(target - self.record.len());
        }
        self.record.extend_from_slice(d);
        true
    }

    /// The record ended: whether it is whole, not dropped.
    fn end(&mut self) -> bool {
        !mem::take(&mut self.dropping)
    }

    fn clear(&mut self) {
        self.record.clear();
        self.dropping = false;
    }
}

/// Turns an fMP4 stream's records into segments; see the
/// [module docs](self#formats).
pub(crate) struct RecordFeed {
    held: Held,
    /// The playlist's target duration, in whole seconds.
    target: u64,
    init: InitCache,
    /// A segment was pushed, so a gap after it is a discontinuity.
    pushed: bool,
}

impl RecordFeed {
    pub(crate) fn new(max_record_bytes: usize, target: u64) -> RecordFeed {
        RecordFeed {
            held: Held::new(max_record_bytes),
            target,
            init: InitCache::default(),
            pushed: false,
        }
    }

    /// Takes the stream's next piece into `stream`.
    pub(crate) fn feed(&mut self, piece: EgressPiece<'_>, stream: &HlsStream) -> Fed {
        match piece {
            EgressPiece::Data(d) => {
                if self.held.data(d) {
                    return Fed::default();
                }
                self.gap(stream);
                Fed {
                    segments: 0,
                    skipped: 1,
                }
            }
            EgressPiece::RecordEnd => {
                if !self.held.end() {
                    return Fed::default();
                }
                let fed = self.record(stream);
                self.held.record.clear();
                fed
            }
            EgressPiece::Discontinuity => {
                self.held.clear();
                self.gap(stream);
                Fed::default()
            }
        }
    }

    /// Pushes the segments of the whole record held.
    fn record(&mut self, stream: &HlsStream) -> Fed {
        let mut fed = Fed::default();
        let Some(pieces) = segments(&self.held.record, &mut self.init, self.target) else {
            self.gap(stream);
            fed.skipped = 1;
            return fed;
        };
        if let Ok((init, media)) = split_record(&self.held.record) {
            for p in &pieces.segments {
                stream.push_segment(
                    init,
                    media.get(p.bytes.clone()).unwrap_or_default(),
                    p.duration,
                );
                fed.segments += 1;
            }
        }
        self.pushed |= fed.segments > 0;
        if pieces.cut_short {
            self.gap(stream);
            fed.skipped = 1;
        }
        fed
    }

    /// The stream skipped: once a segment was pushed, the next one is
    /// discontinuous.
    fn gap(&self, stream: &HlsStream) {
        if self.pushed {
            stream.discontinuity();
        }
    }
}

/// The last init that parsed, and its tracks.
#[derive(Default)]
struct InitCache {
    /// Bound: an init's length prefix, 64 KiB.
    init: Vec<u8>,
    info: Option<InitInfo>,
}

impl InitCache {
    /// The tracks of `init`, parsed only when it differs from the last.
    fn info(&mut self, init: &[u8]) -> Option<InitInfo> {
        if self.info.is_none() || self.init != init {
            self.info = parse_init(init).ok();
            self.init.clear();
            if self.info.is_some() {
                self.init.extend_from_slice(init);
            }
        }
        self.info
    }
}

/// A segment cut from a record: its bytes within the record's media, and
/// its media time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Piece {
    bytes: Range<usize>,
    duration: Duration,
}

/// The segments of an fMP4 record.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pieces {
    segments: Vec<Piece>,
    /// A fragment longer than the target ended them: the record's media
    /// from it on is skipped.
    cut_short: bool,
}

/// The segments of `record` under target `target`, if it is an fMP4
/// record whose media lasts and whose first fragment fits the target:
/// its media whole when that fits, or else runs of its fragments, each
/// as long as fits.
fn segments(record: &[u8], cache: &mut InitCache, target: u64) -> Option<Pieces> {
    let (init, media) = split_record(record).ok()?;
    let info = cache.info(init)?;
    let mut pieces = Pieces {
        segments: Vec::new(),
        cut_short: false,
    };
    let mut current: Option<Piece> = None;
    let mut total = Duration::ZERO;
    for range in fragments(media)? {
        let duration = media_duration(&info, media.get(range.clone())?).ok()?;
        if !fits_target(duration, target) {
            pieces.cut_short = true;
            break;
        }
        total += duration;
        match &mut current {
            Some(p) if fits_target(p.duration + duration, target) => {
                p.bytes.end = range.end;
                p.duration += duration;
            }
            _ => {
                pieces.segments.extend(current.take());
                current = Some(Piece {
                    bytes: range,
                    duration,
                });
            }
        }
    }
    pieces.segments.extend(current);
    // A segment that isn't the record's last ends at its fragment's end;
    // the record's last takes the boxes after its last fragment too.
    if !pieces.cut_short
        && let Some(last) = pieces.segments.last_mut()
    {
        last.bytes.end = media.len();
    }
    (total > Duration::ZERO && !pieces.segments.is_empty()).then_some(pieces)
}

/// The byte ranges of `media`'s fragments: its top-level boxes up to and
/// including each `mdat`, the boxes after the last `mdat` with it. `None`
/// when a box header is malformed or past the end, or no box is an `mdat`.
fn fragments(media: &[u8]) -> Option<Vec<Range<usize>>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    let (mut at, mut start) = (0usize, 0usize);
    while at < media.len() {
        let rest = media.get(at..)?;
        let size = u32::from_be_bytes(rest.get(..4)?.try_into().ok()?);
        let len = match size {
            0 => rest.len(),
            1 => usize::try_from(u64::from_be_bytes(rest.get(8..16)?.try_into().ok()?)).ok()?,
            n => usize::try_from(n).ok()?,
        };
        let header = if size == 1 { 16 } else { 8 };
        if len < header || len > rest.len() {
            return None;
        }
        let mdat = rest.get(4..8)? == b"mdat";
        at += len;
        if mdat {
            out.push(start..at);
            start = at;
        }
    }
    let last = out.last_mut()?;
    last.end = media.len();
    Some(out)
}

/// Turns a unit-format stream's records into segments; see the
/// [module docs](self#formats).
pub(crate) struct UnitFeed {
    held: Held,
    reader: UnitReader,
    muxer: HlsMuxer,
    /// The record read last was an END: the playlist ends once its last
    /// segment is pushed.
    ended: bool,
}

impl UnitFeed {
    pub(crate) fn new(config: HlsMuxerConfig) -> UnitFeed {
        UnitFeed {
            held: Held::new(units::MAX_RECORD_LEN),
            reader: UnitReader::new(),
            muxer: HlsMuxer::new(config),
            ended: false,
        }
    }

    /// Takes the stream's next piece into `stream`.
    pub(crate) fn feed(&mut self, piece: EgressPiece<'_>, stream: &HlsStream) -> Fed {
        let mut fed = Fed::default();
        match piece {
            EgressPiece::Data(d) => {
                if !self.held.data(d) {
                    self.reader.lost();
                    fed.skipped = 1;
                }
            }
            EgressPiece::RecordEnd => {
                if self.held.end() {
                    fed.skipped = self.record();
                    self.held.record.clear();
                }
            }
            EgressPiece::Discontinuity => {
                self.held.clear();
                self.reader.lost();
            }
        }
        while let Some(s) = self.muxer.take_segment() {
            if s.discontinuity {
                stream.discontinuity();
            }
            stream.push_segment(&s.init, &s.media, s.duration);
            fed.segments += 1;
        }
        if std::mem::take(&mut self.ended) {
            stream.end();
        }
        fed
    }

    /// Reads the whole record held into the muxer: 1 if the muxer refused
    /// its unit, 0 otherwise.
    fn record(&mut self) -> u64 {
        let unit = self.reader.read(&self.held.record);
        while let Some(e) = self.reader.poll_event() {
            match e {
                ReaderEvent::Configs => {
                    self.muxer.reset();
                    for c in self.reader.configs() {
                        // A refused config leaves its track out: its units
                        // are refused.
                        let _ = self.muxer.set_config(c);
                    }
                }
                ReaderEvent::Gap { .. } => self.muxer.gap(),
                ReaderEvent::End => {
                    // Ending a segment can't fail.
                    let _ = self.muxer.finish();
                    self.ended = true;
                }
            }
        }
        match unit.map(|u| self.muxer.push(&u)) {
            Some(Err(_)) => 1,
            _ => 0,
        }
    }
}

/// An open's entry, removed if the open doesn't finish.
struct Opening<'a, K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: &'a Shared<K, H, V, T>,
    transfer_id: TransferId,
    id: u64,
    done: bool,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for Opening<'_, K, H, V, T> {
    fn drop(&mut self) {
        if !self.done {
            let e = self
                .shared
                .hls
                .borrow_mut()
                .take(&self.transfer_id, self.id);
            drop(e);
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Takes in a write of the HLS config: disabling it closes every
    /// stream.
    pub(crate) fn hls_config_written(&self, c: &HlsEgressConfig) {
        let closed = {
            let mut h = self.hls.borrow_mut();
            h.config_writes = h.config_writes.wrapping_add(1);
            if c.enabled {
                return;
            }
            mem::take(&mut h.streams)
        };
        for (t, e) in closed {
            self.end_hls(t, e, HlsCloseReason::Disabled);
        }
    }

    /// Polls the wait for the next sweep of idle streams, armed while
    /// there are streams; ready when it is time to sweep.
    pub(crate) fn poll_hls_sweep(&self, sweep: &mut HlsSweep<T>, cx: &mut Context<'_>) -> Poll<()> {
        if self.hls.borrow().streams.is_empty() {
            *sweep = None;
            return Poll::Pending;
        }
        let sleep = sweep.get_or_insert_with(|| {
            let interval = self.config.hls.sweep_interval.max(Duration::MILLISECOND);
            Box::pin(self.timer.sleep_until(self.timer.now() + interval))
        });
        if sleep.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        }
        *sweep = None;
        Poll::Ready(())
    }

    /// Closes the streams no request reached for the idle timeout.
    pub(crate) fn sweep_hls(&self) {
        let now = self.timer.now();
        let idle = self.config.hls.idle_timeout;
        let closed: Vec<(TransferId, HlsEntry)> = self
            .hls
            .borrow_mut()
            .streams
            .extract_if(.., |_, e| now - e.stream.last_access() >= idle)
            .collect();
        for (t, e) in closed {
            self.end_hls(t, e, HlsCloseReason::Idle);
        }
    }

    /// Ends a stream taken out of the streams: stops its reader, which
    /// releases its swarm, and unregisters it.
    fn end_hls(&self, t: TransferId, e: HlsEntry, reason: HlsCloseReason) {
        let opened = e.reader.is_some();
        if let Some(reader) = &e.reader {
            reader.abort();
        }
        drop(e);
        if opened {
            tracing::debug!(transfer = %t, ?reason, "HLS stream closed");
            self.events.emit(vec![VideoEvent::HlsStreamClosed {
                transfer_id: t,
                reason,
            }]);
        }
    }

    /// Feeds the pieces of `egress`, a stream of `format`, into `stream`
    /// until the stream fails, then closes entry `id` of `t`.
    async fn read_hls(
        self: Rc<Self>,
        t: TransferId,
        id: u64,
        mut egress: EgressStream<K, H, V, T>,
        format: StreamFormat,
        stream: Weak<HlsStream>,
    ) {
        let mut feed = StreamFeed::new(format, &self.config.hls);
        let error = loop {
            let piece = match egress.next(usize::MAX).await {
                Ok(piece) => piece,
                Err(e) => break e,
            };
            let Some(stream) = stream.upgrade() else {
                return;
            };
            let fed = feed.feed(piece, &stream);
            if fed != Fed::default() {
                let mut m = self.metrics.borrow_mut();
                m.hls_segments += fed.segments;
                m.hls_skipped_records += fed.skipped;
            }
        };
        let e = self.hls.borrow_mut().take(&t, id);
        if let Some(e) = e {
            self.end_hls(t, e, HlsCloseReason::Failed(error));
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// Serves the swarm `swarm_uri` names, read on the networks
    /// `network_keys` names in `format`, as an HLS stream: its playlist
    /// URL. See the [module docs](self). A transfer already served answers
    /// its stream's URL, read in the format its first open named.
    ///
    /// # Errors
    ///
    /// The driver doesn't serve HLS, the profile has no HLS egress, the
    /// HLS config is disabled or no server address is set, the node serves
    /// its most HLS streams, or the stream doesn't open
    /// ([`VideoControl::open_stream`]). [`VideoError::StreamClosed`] when
    /// the stream closed while it opened.
    pub async fn open_hls(
        &self,
        swarm_uri: &str,
        network_keys: &[Vec<u8>],
        format: StreamFormat,
    ) -> Result<String, VideoError> {
        let s = &self.shared;
        let settings = &s.config.hls;
        if !settings.serve {
            return Err(VideoError::HlsNotSupported);
        }
        let profile_key = match &s.hls.borrow().profile {
            Ok(p) => *p.key(),
            Err(e) => return Err(VideoError::HlsUnavailable(*e)),
        };
        let config = loop {
            let writes = s.hls.borrow().config_writes;
            let c = self.hls_config().await?;
            if s.hls.borrow().config_writes == writes {
                break c;
            }
        };
        if !config.enabled {
            return Err(VideoError::HlsDisabled);
        }
        let base = match config.public_server_addr.as_str() {
            "" => settings.base_url.as_str(),
            addr => addr,
        };
        let base = base.trim_end_matches('/');
        if base.is_empty() {
            return Err(VideoError::HlsNoServerAddr);
        }
        let (uri, _) = check_stream(&s.config.egress, swarm_uri, network_keys)?;
        let t = TransferId::new(&uri.id, b"");
        let url = format!("{base}{}", playlist_path(&profile_key, t.as_bytes()));

        let id = {
            let mut h = s.hls.borrow_mut();
            if h.streams.contains_key(&t) {
                return Ok(url);
            }
            if h.streams.len() >= settings.max_streams {
                return Err(VideoError::TooManyHlsStreams);
            }
            let playlist = PlaylistConfig {
                target: target_secs(settings.target_duration),
                listed: settings.listed_duration,
                retained: settings.retained_duration,
                max_retained_bytes: settings.max_retained_bytes,
            };
            let stream = h
                .profile
                .as_ref()
                .map_err(|e| VideoError::HlsUnavailable(*e))?
                .open_stream(s.timer.now(), *t.as_bytes(), playlist)
                .map_err(VideoError::HlsUnavailable)?;
            let id = h.next_id;
            h.next_id += 1;
            h.streams.insert(
                t,
                HlsEntry {
                    id,
                    stream: Rc::new(stream),
                    reader: None,
                },
            );
            id
        };
        let mut opening = Opening {
            shared: s,
            transfer_id: t,
            id,
            done: false,
        };
        let egress = self.open_stream(swarm_uri, network_keys, format).await?;
        let task = {
            let mut h = s.hls.borrow_mut();
            let entry = h
                .streams
                .get_mut(&t)
                .filter(|e| e.id == id)
                .ok_or(VideoError::StreamClosed)?;
            let stream = Rc::downgrade(&entry.stream);
            let (task, abort) = abortable(s.clone().read_hls(t, id, egress, format, stream));
            entry.reader = Some(abort);
            task
        };
        opening.done = true;
        s.spawn(task.map(drop).boxed_local());
        s.metrics.borrow_mut().hls_streams += 1;
        tracing::debug!(transfer = %t, url = %url, "HLS stream opened");
        s.events.emit(vec![VideoEvent::HlsStreamOpened {
            transfer_id: t,
            playlist_url: url.clone(),
        }]);
        Ok(url)
    }

    /// Closes the HLS stream of transfer `id`; see the
    /// [module docs](self#lifetime).
    ///
    /// # Errors
    ///
    /// [`VideoError::HlsStreamNotFound`] when the node has no HLS stream of
    /// the transfer.
    pub fn close_hls(&self, id: &TransferId) -> Result<(), VideoError> {
        let s = &self.shared;
        let e = s.hls.borrow_mut().streams.remove(id);
        let e = e.ok_or(VideoError::HlsStreamNotFound)?;
        s.end_hls(*id, e, HlsCloseReason::Closed);
        Ok(())
    }

    /// The node's HLS streams, in transfer id order.
    pub fn hls_streams(&self) -> Vec<HlsStreamInfo> {
        self.shared
            .hls
            .borrow()
            .streams
            .iter()
            .map(|(t, e)| HlsStreamInfo {
                transfer_id: *t,
                path: e.stream.path().to_owned(),
                last_access: e.stream.last_access(),
                reading: e.reader.is_some(),
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "hls_egress_test.rs"]
mod tests;
