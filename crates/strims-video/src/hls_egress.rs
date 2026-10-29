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
//! # Records
//!
//! Each whole record ([`EgressPiece::RecordEnd`]) is an fMP4 record
//! ([`split_record`]): its init and media become an HLS segment lasting the
//! media's time ([`media_duration`]). A record of at most
//! [`HlsSettings::max_record_bytes`](crate::HlsSettings::max_record_bytes)
//! is held while it arrives. A record that isn't fMP4 with media time,
//! such as a capture's WebM, is skipped, and one over the bound is dropped.
//! Once a segment was served, a skipped or dropped record, or a
//! discontinuity of the stream, marks the next segment discontinuous.
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
use crate::hls::{HlsProfile, HlsRoutes, HlsStream, PlaylistConfig, RouteError, playlist_path};
use crate::{VideoControl, VideoError, VideoEvent, VideoHost};

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

/// What a piece did to an HLS stream ([`RecordFeed::feed`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fed {
    Nothing,
    /// A segment was pushed.
    Segment,
    /// A record was skipped or dropped.
    Skipped,
}

/// Turns a stream's pieces into the segments of an [`HlsStream`]; see the
/// [module docs](self#records).
pub(crate) struct RecordFeed {
    /// The record in progress. Bound: `max_record_bytes`, also of its
    /// capacity, which is kept from record to record.
    record: Vec<u8>,
    max_record_bytes: usize,
    /// The record in progress went over the bound, so its bytes up to its
    /// end are dropped.
    dropping: bool,
    init: InitCache,
    /// A segment was pushed, so a gap after it is a discontinuity.
    pushed: bool,
}

impl RecordFeed {
    pub(crate) fn new(max_record_bytes: usize) -> RecordFeed {
        RecordFeed {
            record: Vec::new(),
            max_record_bytes,
            dropping: false,
            init: InitCache::default(),
            pushed: false,
        }
    }

    /// Takes the stream's next piece into `stream`.
    pub(crate) fn feed(&mut self, piece: EgressPiece<'_>, stream: &HlsStream) -> Fed {
        match piece {
            EgressPiece::Data(d) => self.data(d, stream),
            EgressPiece::RecordEnd => self.end(stream),
            EgressPiece::Discontinuity => {
                self.record.clear();
                self.dropping = false;
                self.gap(stream);
                Fed::Nothing
            }
        }
    }

    fn data(&mut self, d: &[u8], stream: &HlsStream) -> Fed {
        if self.dropping {
            return Fed::Nothing;
        }
        let need = self.record.len().saturating_add(d.len());
        if need > self.max_record_bytes {
            self.record.clear();
            self.dropping = true;
            self.gap(stream);
            return Fed::Skipped;
        }
        if need > self.record.capacity() {
            let grown = self.record.capacity().saturating_mul(2);
            let target = need.max(grown).min(self.max_record_bytes);
            self.record.reserve_exact(target - self.record.len());
        }
        self.record.extend_from_slice(d);
        Fed::Nothing
    }

    fn end(&mut self, stream: &HlsStream) -> Fed {
        if mem::take(&mut self.dropping) {
            return Fed::Nothing;
        }
        let fed = match segment(&self.record, &mut self.init) {
            Some((init, media, duration)) => {
                stream.push_segment(init, media, duration);
                self.pushed = true;
                Fed::Segment
            }
            None => {
                self.gap(stream);
                Fed::Skipped
            }
        };
        self.record.clear();
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

/// A record's init, media and media time, if it is an fMP4 record whose
/// media lasts.
fn segment<'a>(record: &'a [u8], cache: &mut InitCache) -> Option<(&'a [u8], &'a [u8], Duration)> {
    let (init, media) = split_record(record).ok()?;
    let info = cache.info(init)?;
    let duration = media_duration(&info, media).ok()?;
    (duration > Duration::ZERO).then_some((init, media, duration))
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

    /// Feeds the pieces of `egress` into `stream` until the stream fails,
    /// then closes entry `id` of `t`.
    async fn read_hls(
        self: Rc<Self>,
        t: TransferId,
        id: u64,
        mut egress: EgressStream<K, H, V, T>,
        stream: Weak<HlsStream>,
    ) {
        let mut feed = RecordFeed::new(self.config.hls.max_record_bytes);
        let error = loop {
            let piece = match egress.next(usize::MAX).await {
                Ok(piece) => piece,
                Err(e) => break e,
            };
            let Some(stream) = stream.upgrade() else {
                return;
            };
            match feed.feed(piece, &stream) {
                Fed::Nothing => {}
                Fed::Segment => self.metrics.borrow_mut().hls_segments += 1,
                Fed::Skipped => self.metrics.borrow_mut().hls_skipped_records += 1,
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
    /// `network_keys` names, as an HLS stream: its playlist URL. See the
    /// [module docs](self).
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
                listed: settings.listed_segments,
                retained: settings.retained_segments,
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
        let egress = self
            .open_stream(swarm_uri, network_keys, StreamFormat::Packed)
            .await?;
        let task = {
            let mut h = s.hls.borrow_mut();
            let entry = h
                .streams
                .get_mut(&t)
                .filter(|e| e.id == id)
                .ok_or(VideoError::StreamClosed)?;
            let stream = Rc::downgrade(&entry.stream);
            let (task, abort) = abortable(s.clone().read_hls(t, id, egress, stream));
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
