// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS egress: video swarms served to external players as HLS playlists.
//!
//! [`VideoControl::open_hls`] reads the swarm a magnet URI names as an
//! [`EgressStream`], counted among the egress bounds
//! ([`EgressSettings`](crate::EgressSettings)), and serves its records on
//! the driver's [`HlsRoutes`] under the profile's public key: the
//! multivariant playlist at [`multivariant_path`]`(profile key, transfer
//! id)`, the media playlist at [`playlist_path`](crate::hls::playlist_path)
//! beside it, and its inits,
//! segments and subtitles. The control registers the profile when it is
//! built; a profile whose key isn't [`KEY_LEN`](crate::hls::KEY_LEN)
//! bytes has no HLS egress.
//!
//! # Opening
//!
//! A stream opens only while the driver serves HLS
//! ([`HlsSettings::serve`](crate::HlsSettings::serve)) and the device's
//! HLS config is enabled. Its URL is the config's public server address,
//! or [`HlsSettings::base_url`](crate::HlsSettings::base_url) without
//! one, less a trailing `/`, followed by the multivariant playlist's
//! path. The URL is made at each open, so a changed address names the
//! same streams. The open of a new stream answers once the stream lists
//! its first segment, or after
//! [`HlsSettings::open_wait`](crate::HlsSettings::open_wait), so a player
//! given the URL finds media to play. A transfer has one HLS stream:
//! opening it again answers that stream's URL at once, and counts as a
//! request of it for its idle timeout. An open whose
//! URI names other swarm options than the stream reads with, or than the
//! swarm's listing does, opens the swarm as [`VideoControl::open_stream`]
//! does ([`egress`](crate::egress#options)), which refuses what the
//! listing disagrees with; a stream that opens so with other options
//! reads on from the swarm of those after a discontinuity. The node
//! serves at most
//! [`HlsSettings::max_streams`](crate::HlsSettings::max_streams) streams.
//!
//! # Segments
//!
//! A stream reads its swarm in the [unit format](crate::units), which the
//! open's MIME type, the listing's, must name, and its playlist's target
//! duration is
//! [`HlsSettings::target_duration`](crate::HlsSettings::target_duration)
//! for its life. Each whole record of at most [`units::MAX_RECORD_LEN`]
//! goes through a [`UnitReader`] into an [`HlsMuxer`], whose segments the
//! playlist lists (see [`hls`](crate::hls#segments)), and with
//! [`HlsSettings::part_target`](crate::HlsSettings::part_target) their
//! parts as they come ([`hls`](crate::hls#low-latency)), its text tracks
//! among its configs, whose cues are the segments' subtitles
//! ([`hls`](crate::hls#subtitles)). The reader's gaps
//! in a video track are the muxer's, so a lost chunk costs the units up
//! to the next keyframe, and the muxer fills the holes that lost units
//! leave in an audio track alone with silence. New configs, which also
//! come when a writer starts the stream over, reset the muxer. An END
//! record ends the segment in progress and the playlist
//! (`EXT-X-ENDLIST`); a later session's segment takes the end back off
//! (see [`Playlist::push_segment`](crate::hls::Playlist::push_segment)).
//! A record over the bound is dropped.
//!
//! A gap or reset marks the next segment discontinuous.
//! [`Metrics`](crate::Metrics) counts the segments and skipped records.
//!
//! # Lifetime
//!
//! A stream is the node's, not its opener's: it lasts until
//! [`VideoControl::close_hls`] (`CloseStream`) closes it, no request reaches
//! it for [`HlsSettings::idle_timeout`](crate::HlsSettings::idle_timeout)
//! (looked for every
//! [`HlsSettings::sweep_interval`](crate::HlsSettings::sweep_interval) by
//! [`VideoControl::run`]), or the HLS config is disabled. Closing
//! unregisters its routes, so requests of it answer 404, and releases its
//! swarm. [`VideoEvent::HlsStreamOpened`] and
//! [`VideoEvent::HlsStreamClosed`] report each stream.
//!
//! # Recovery
//!
//! A stream outlasts the failures of what it reads. After a failed read
//! it reads on, and when its egress stream ends (its swarm gone with none
//! in its place, as when the node's ingest session of it ends) it opens
//! one again, as [`VideoControl::open_stream`] does, of the URI the
//! swarm's listing names when the node has one. Each waits
//! [`HlsSettings::reopen_interval`](crate::HlsSettings::reopen_interval)
//! first, twice as long after each failure in a row up to
//! [`HlsSettings::max_reopen_interval`](crate::HlsSettings::max_reopen_interval),
//! and the first piece read sets the wait back. Meanwhile the playlist
//! stays as it was, ended or live, and the records read after the stream
//! opened again follow a discontinuity.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll};

use futures_util::FutureExt;
use futures_util::future::{AbortHandle, abortable};
use strims_base::time::{Duration, Time};
use strims_directory::Host;
use strims_kv::Kv;
use strims_ppspp::{SwarmOptions, Uri};
use strims_proto::profile::v1::Profile;
use strims_proto::video::v1::HlsEgressConfig;
use strims_rpc::Timer;
use strims_transfer::TransferId;

use crate::control::Shared;
use crate::egress::{EgressPiece, EgressStream, check_format, check_stream};
use crate::hls::{
    HlsMuxer, HlsMuxerConfig, HlsProfile, HlsRoutes, HlsStream, PlaylistConfig, RouteError,
    multivariant_path, target_secs,
};
use crate::media::{Muxer, TrackKind};
use crate::registry::same_options;
use crate::units::{self, ReaderEvent, UnitReader};
use crate::{HlsSettings, VideoControl, VideoError, VideoEvent, VideoHost};

/// How often an open looks for its new stream's first segment.
const FIRST_SEGMENT_POLL: Duration = Duration::from_millis(50);

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
}

/// An HLS stream of the node, as [`VideoControl::hls_streams`] lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsStreamInfo {
    /// The transfer it reads: its swarm id under the empty salt.
    pub transfer_id: TransferId,
    /// Its media playlist's path on the HLS routes
    /// ([`playlist_path`](crate::hls::playlist_path)).
    pub path: String,
    /// The time of its latest successful request, or its open time.
    pub last_access: Time,
    /// Its records are being read: false while its swarm is opened.
    pub reading: bool,
    /// Times its egress stream ended and was opened again.
    pub reopens: u64,
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
    /// What its reader opens its egress stream with, shared with the task.
    open: Rc<RefCell<OpenArgs>>,
    reopens: u64,
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

/// Turns a stream's pieces into the segments of an [`HlsStream`]; see the
/// [module docs](self#segments).
pub(crate) struct UnitFeed {
    held: Held,
    reader: UnitReader,
    muxer: HlsMuxer,
    /// The record read last was an END: the playlist ends once its last
    /// segment is pushed.
    ended: bool,
}

impl UnitFeed {
    /// A feed of a stream under `settings`.
    pub(crate) fn of(settings: &HlsSettings) -> UnitFeed {
        UnitFeed::new(HlsMuxerConfig {
            target: target_secs(settings.target_duration),
            fragment_duration: settings.fragment_duration,
            max_segment_bytes: settings.max_segment_bytes,
            part_target: settings.part_target,
        })
    }

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
        // A segment's parts come before it, and its first part says what it
        // follows.
        while let Some(p) = self.muxer.take_part() {
            stream.push_part(&p);
        }
        while let Some(s) = self.muxer.take_segment() {
            stream.push(&s);
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
                    self.muxer.set_text_configs(self.reader.text_configs());
                }
                ReaderEvent::TextConfigs => {
                    self.muxer.set_text_configs(self.reader.text_configs());
                }
                ReaderEvent::Gap { track, .. } => {
                    // The muxer fills an audio track's hole when the unit
                    // after it comes; a lost video unit costs the units up
                    // to the next keyframe.
                    let audio = self
                        .reader
                        .configs()
                        .iter()
                        .any(|c| c.track == track && c.kind == TrackKind::Audio);
                    if !audio {
                        self.muxer.gap();
                    }
                }
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
    /// Waits for the first segment of the new stream `id` of `t`, up to
    /// [`HlsSettings::open_wait`], or for the stream to close.
    async fn first_segment(&self, t: TransferId, id: u64) {
        let deadline = self.timer.now() + self.config.hls.open_wait;
        loop {
            let listed = self
                .hls
                .borrow()
                .streams
                .get(&t)
                .filter(|e| e.id == id)
                .map(|e| e.stream.listed_segments() > 0);
            if listed != Some(false) {
                return;
            }
            let now = self.timer.now();
            if now >= deadline {
                return;
            }
            let poll = (now + FIRST_SEGMENT_POLL).min(deadline);
            self.timer.sleep_until(poll).await;
        }
    }

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

    /// Has entry `id` of `t` read `egress`, of the swarm `open` names, in
    /// place of what it read: the playlist goes on after a discontinuity.
    fn reread_hls(
        self: &Rc<Self>,
        t: TransferId,
        id: u64,
        egress: EgressStream<K, H, V, T>,
        open: OpenArgs,
    ) -> Result<(), VideoError> {
        let task = {
            let mut h = self.hls.borrow_mut();
            let entry = h
                .streams
                .get_mut(&t)
                .filter(|e| e.id == id)
                .ok_or(VideoError::StreamClosed)?;
            if let Some(reader) = entry.reader.take() {
                reader.abort();
            }
            *entry.open.borrow_mut() = open;
            entry.stream.discontinuity();
            let stream = Rc::downgrade(&entry.stream);
            let (task, abort) =
                abortable(
                    self.clone()
                        .read_hls(t, id, egress, entry.open.clone(), stream),
                );
            entry.reader = Some(abort);
            task
        };
        self.spawn(task.map(drop).boxed_local());
        tracing::debug!(transfer = %t, "HLS stream reads other swarm options");
        Ok(())
    }

    /// Feeds the pieces of `egress`, and of the streams `open` opens once
    /// it ends, into the playlist of entry `id` of `t` while it lasts; see
    /// the [module docs](self#recovery).
    async fn read_hls(
        self: Rc<Self>,
        t: TransferId,
        id: u64,
        egress: EgressStream<K, H, V, T>,
        open: Rc<RefCell<OpenArgs>>,
        stream: Weak<HlsStream>,
    ) {
        let settings = &self.config.hls;
        let backoff = Backoff::new(settings.reopen_interval, settings.max_reopen_interval);
        let mut feed = UnitFeed::of(settings);
        let mut source = HostSource {
            control: VideoControl {
                shared: self.clone(),
            },
            open,
        };
        serve(
            &mut source,
            Some(egress),
            &mut feed,
            &stream,
            backoff,
            |e| match e {
                ServeEvent::Fed(fed) => {
                    let mut m = self.metrics.borrow_mut();
                    m.hls_segments += fed.segments;
                    m.hls_skipped_records += fed.skipped;
                }
                ServeEvent::Reopened => {
                    tracing::debug!(transfer = %t, "HLS stream opened again");
                    let mut h = self.hls.borrow_mut();
                    if let Some(e) = h.streams.get_mut(&t).filter(|e| e.id == id) {
                        e.reopens += 1;
                    }
                }
                ServeEvent::Failed(error) => {
                    tracing::debug!(transfer = %t, %error, "HLS stream read failed");
                }
            },
        )
        .await;
    }
}

/// What an HLS stream's egress stream opens with again.
pub(crate) struct OpenArgs {
    swarm_uri: String,
    /// The options its URI names, as egress reads them.
    options: SwarmOptions,
    network_keys: Vec<Vec<u8>>,
    mime_type: String,
}

/// Where an HLS stream's pieces come from: an egress stream, opened again
/// when it ends.
pub(crate) trait PieceSource {
    type Stream;

    /// Opens the stream again.
    async fn open(&mut self) -> Result<Self::Stream, VideoError>;

    /// The stream's next piece.
    async fn next<'a>(
        &mut self,
        stream: &'a mut Self::Stream,
    ) -> Result<EgressPiece<'a>, VideoError>;

    /// Resolves after `d`.
    async fn wait(&mut self, d: Duration);
}

/// The node's egress stream of a swarm.
struct HostSource<K: Kv, H: Host, V: VideoHost, T: Timer> {
    control: VideoControl<K, H, V, T>,
    open: Rc<RefCell<OpenArgs>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> PieceSource for HostSource<K, H, V, T> {
    type Stream = EgressStream<K, H, V, T>;

    /// Opens the swarm of the URI its listing names, when the node has
    /// one, else of the URI the stream last opened.
    async fn open(&mut self) -> Result<Self::Stream, VideoError> {
        let s = &self.control.shared;
        let (uri, keys, mime) = {
            let o = self.open.borrow();
            (
                o.swarm_uri.clone(),
                o.network_keys.clone(),
                o.mime_type.clone(),
            )
        };
        let listed = Uri::parse(&uri)
            .ok()
            .and_then(|u| s.listed_swarm_uri(&u.id, &keys))
            .filter(|l| *l != uri);
        let uri = match listed.map(|l| (check_stream(&s.config.egress, &l, &keys), l)) {
            Some((Ok((_, options)), listed)) => {
                let mut o = self.open.borrow_mut();
                o.swarm_uri = listed.clone();
                o.options = options;
                listed
            }
            _ => uri,
        };
        self.control.open_stream(&uri, &keys, &mime).await
    }

    async fn next<'a>(
        &mut self,
        stream: &'a mut Self::Stream,
    ) -> Result<EgressPiece<'a>, VideoError> {
        stream.next(usize::MAX).await
    }

    async fn wait(&mut self, d: Duration) {
        let timer = &self.control.shared.timer;
        timer.sleep_until(timer.now() + d).await;
    }
}

/// The waits of a stream that fails: from the least, doubling after each
/// to the most.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Backoff {
    least: Duration,
    most: Duration,
    next: Duration,
}

impl Backoff {
    pub(crate) fn new(least: Duration, most: Duration) -> Backoff {
        let least = least.max(Duration::MILLISECOND);
        Backoff {
            least,
            most: most.max(least),
            next: least,
        }
    }

    /// The next wait, doubling the one after.
    fn take(&mut self) -> Duration {
        let d = self.next;
        self.next = (d + d).min(self.most);
        d
    }

    fn reset(&mut self) {
        self.next = self.least;
    }
}

/// What [`serve`] did.
#[derive(Debug)]
pub(crate) enum ServeEvent {
    Fed(Fed),
    /// The stream ended and was opened again.
    Reopened,
    /// A read or an open failed.
    Failed(VideoError),
}

/// What a read came to, without its piece's borrow.
enum Read {
    Piece,
    Failed(VideoError),
    Ended(VideoError),
}

/// Feeds the pieces of `current`, and of the streams `source` opens again
/// once it ends, into `stream`'s playlist until `stream` is dropped: see
/// the [module docs](self#recovery). Each piece, reopen and failure goes
/// to `on`.
pub(crate) async fn serve<P: PieceSource>(
    source: &mut P,
    mut current: Option<P::Stream>,
    feed: &mut UnitFeed,
    stream: &Weak<HlsStream>,
    mut backoff: Backoff,
    mut on: impl FnMut(ServeEvent),
) {
    loop {
        let Some(s) = current.as_mut() else {
            source.wait(backoff.take()).await;
            if stream.strong_count() == 0 {
                return;
            }
            match source.open().await {
                Ok(s) => {
                    current = Some(s);
                    if let Some(h) = stream.upgrade() {
                        feed.feed(EgressPiece::Discontinuity, &h);
                    }
                    on(ServeEvent::Reopened);
                }
                Err(e) => on(ServeEvent::Failed(e)),
            }
            continue;
        };
        let read = match source.next(s).await {
            Ok(piece) => {
                let Some(h) = stream.upgrade() else {
                    return;
                };
                on(ServeEvent::Fed(feed.feed(piece, &h)));
                Read::Piece
            }
            // The egress stream reads on after a discontinuity.
            Err(e @ VideoError::StreamRead(_)) => Read::Failed(e),
            Err(e) => Read::Ended(e),
        };
        match read {
            Read::Piece => backoff.reset(),
            Read::Failed(e) => {
                on(ServeEvent::Failed(e));
                source.wait(backoff.take()).await;
            }
            Read::Ended(e) => {
                on(ServeEvent::Failed(e));
                current = None;
            }
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// Serves the swarm `swarm_uri` names, read on the networks
    /// `network_keys` names, of the format its listing's MIME type
    /// `mime_type` names, as an HLS stream: its playlist URL. See the
    /// [module docs](self). A transfer already served answers its stream's
    /// URL.
    ///
    /// # Errors
    ///
    /// The driver doesn't serve HLS, the profile has no HLS egress, the
    /// MIME type names another format than the unit format
    /// ([`check_format`]), the HLS config is disabled or no server address
    /// is set, the node serves its most HLS streams, or the stream doesn't
    /// open ([`VideoControl::open_stream`]). [`VideoError::StreamClosed`]
    /// when the stream closed while it opened.
    pub async fn open_hls(
        &self,
        swarm_uri: &str,
        network_keys: &[Vec<u8>],
        mime_type: &str,
    ) -> Result<String, VideoError> {
        let s = &self.shared;
        let settings = &s.config.hls;
        if !settings.serve {
            return Err(VideoError::HlsNotSupported);
        }
        check_format(mime_type)?;
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
        let (uri, options) = check_stream(&s.config.egress, swarm_uri, network_keys)?;
        let t = TransferId::new(&uri.id, b"");
        let url = format!("{base}{}", multivariant_path(&profile_key, t.as_bytes()));
        let open = || OpenArgs {
            swarm_uri: swarm_uri.to_owned(),
            options: options.clone(),
            network_keys: network_keys.to_vec(),
            mime_type: mime_type.to_owned(),
        };

        // A stream served already: its entry, and whether it reads other
        // options than the open names.
        let served = s.hls.borrow().streams.get(&t).map(|e| {
            e.stream.touch(s.timer.now());
            let other = e.reader.is_some() && !same_options(&e.open.borrow().options, &options);
            (e.id, other)
        });
        if let Some((id, other)) = served {
            let listed = s
                .listed_swarm_uri(&uri.id, network_keys)
                .and_then(|l| check_stream(&s.config.egress, &l, network_keys).ok());
            let disagrees = listed.is_some_and(|(_, l)| !same_options(&l, &options));
            if !other && !disagrees {
                return Ok(url);
            }
            // The open applies egress's rule of options, refusing what the
            // listing disagrees with.
            let egress = self.open_stream(swarm_uri, network_keys, mime_type).await?;
            if other {
                s.reread_hls(t, id, egress, open())?;
            }
            return Ok(url);
        }

        let id = {
            let mut h = s.hls.borrow_mut();
            if h.streams.len() >= settings.max_streams {
                return Err(VideoError::TooManyHlsStreams);
            }
            let playlist = PlaylistConfig {
                target: target_secs(settings.target_duration),
                listed: settings.listed_duration,
                retained: settings.retained_duration,
                max_retained_bytes: settings.max_retained_bytes,
                part_target: settings.part_target,
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
                    open: Rc::new(RefCell::new(open())),
                    reopens: 0,
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
        let egress = self.open_stream(swarm_uri, network_keys, mime_type).await?;
        let task = {
            let mut h = s.hls.borrow_mut();
            let entry = h
                .streams
                .get_mut(&t)
                .filter(|e| e.id == id)
                .ok_or(VideoError::StreamClosed)?;
            let stream = Rc::downgrade(&entry.stream);
            let open = entry.open.clone();
            let (task, abort) = abortable(s.clone().read_hls(t, id, egress, open, stream));
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
        s.first_segment(t, id).await;
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
                reopens: e.reopens,
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "hls_egress_test.rs"]
mod tests;
