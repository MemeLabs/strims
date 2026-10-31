// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser captures: the UI's encoded media, written into a video swarm in
//! the unit format ([`units`](crate::units)) and listed on its networks.
//!
//! [`VideoControl::open_capture`] creates a swarm under a fresh ed25519
//! key, signing as the capture's nominal bitrate and tracks have it
//! ([`UnitSigning::for_stream`]), with [`capture_options_for`] that
//! signing: 4 KiB chunks, 16 streams, an 8 Ki-chunk window,
//! Merkle/BLAKE3/Ed25519, and its integrity unit size. It adds the swarm's
//! transfer under the empty salt and publishes it on the capture's
//! networks the node has joined, and on the rest as it joins them. It
//! serves the capture's snippet, signed with the capture key. Once the
//! capture's first record is written, it keeps a media listing on each
//! network's directory of the stream's MIME type ([`units_mime`] of its
//! configs) and the swarm URI; a record of other codecs lists the new MIME
//! type in its place. The capture's id is its transfer id.
//!
//! A capture's 1 to
//! [`CaptureSettings::max_network_keys`](crate::CaptureSettings::max_network_keys)
//! network keys each name a network in the profile's store; a network the
//! node hasn't joined lists and carries the capture once it does. Its
//! snippet passes [`strims_directory::snippet::check`]. The node runs at
//! most [`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures)
//! captures.
//!
//! # Media
//!
//! [`VideoControl::append_capture`] takes decoder configs and access units,
//! as the UI's encoders gave them:
//!
//! - A config names its track, nonzero, and its codec's configuration
//!   record: an AVCDecoderConfigurationRecord for H.264, timed at
//!   [`VIDEO_TIMESCALE`], or an OpusHead for Opus, timed at
//!   [`opus::SAMPLE_RATE`]. What else the track's [`DecoderConfig`] holds,
//!   its kind, codec string and picture size or channels, comes from the
//!   record, as a unit reader derives it
//!   ([`ConfigEntry::decoder_config`]). The capture's configs together are
//!   at most [`MAX_TRACKS`](crate::fmp4::MAX_TRACKS) ([`check_configs`]),
//!   and an append names a track at most once.
//! - A unit names a track whose config is set, and carries its payload,
//!   its keyframe flag and its presentation time in microseconds on the
//!   capture's clock, at most [`MAX_TIMESTAMP_US`] from 0. Its decode time
//!   is its presentation time: a capture's tracks have no reordered
//!   pictures. Rounded to its track's timescale, the time is after the
//!   track's last unit's. A record holds the payload with its head, so the
//!   payload is at most [`MAX_RECORD_LEN`] less [`MAX_UNIT_HEAD_LEN`].
//! - An append's configs apply from its units on, and it holds at most
//!   [`CaptureSettings::max_units_per_append`](crate::CaptureSettings::max_units_per_append)
//!   units.
//! - A unit's publish time is when the capture's writer took its append,
//!   on the node's clock.
//!
//! An append that breaks these rules is refused whole with
//! [`VideoError::CaptureMedia`], and the capture goes on as before it.
//! Appends are written one at a time, in the order they were made, by a
//! task of the capture's own: an append whose caller gives up before its
//! turn is dropped, and one under way finishes. An append is refused
//! before it waits when the capture's waiting appends hold
//! [`CaptureSettings::max_queued_bytes`](crate::CaptureSettings::max_queued_bytes)
//! ([`VideoError::CaptureBusy`]), or when it would take the capture past
//! its rate ([`VideoError::CaptureRate`]): a token bucket of
//! [`CaptureSettings::max_burst`](crate::CaptureSettings::max_burst)
//! bytes filling at
//! [`CaptureSettings::max_rate`](crate::CaptureSettings::max_rate), from
//! which an append takes its configs' and payloads' bytes and
//! [`ITEM_COST`] for each config and unit.
//!
//! The units go through a [`UnitWriter`] in frames of the swarm's chunk
//! size into the swarm, as each append is written. A partial integrity
//! unit is signed its signing's interval after its first byte
//! ([`UnitSigning::timer`]): the writer ends its frame, the swarm writer
//! is flushed, which pads the unit with zeros that read as padding frames,
//! and the writer goes on where the next unit starts. A swarm write that
//! fails ends the capture's writing: every later append fails with
//! [`VideoError::StreamWrite`].
//!
//! # Teardown
//!
//! [`VideoControl::close_capture`] takes the capture out of the node, so
//! its appends fail with [`VideoError::CaptureNotFound`], stops the listing
//! keepers (each unpublishes its listing), and stops serving the snippet.
//! The writer then writes its last appends' output and the capture's END
//! record ([`units`](crate::units#records)), and has the swarm sign the
//! last partial integrity unit, for at most [`CLOSE_FLUSH_TIMEOUT`]. The
//! swarm stays for [`END_LINGER`] after the END was signed, for its peers
//! to fetch the END, and the transfer and then the swarm are removed. A
//! frontend connection that ends closes its captures alike.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::future::poll_fn;
use std::pin::pin;
use std::rc::{Rc, Weak};
use std::task::{Poll, Waker};

use futures_util::FutureExt;
use futures_util::future::{Either, select};
use strims_base::time::{Duration, Time};
use strims_crypto::{Signer, generate_key};
use strims_directory::{Host, ListingKeeper, SignedSnippet, snippet};
use strims_kv::Kv;
use strims_ppspp::{SwarmId, SwarmOptions, Uri, WriterOptions};
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::{Listing, ListingSnippet};
use strims_proto::types::Key;
use strims_proto::video::v1::capture_config::Codec;
use strims_proto::video::v1::{CaptureConfig, CaptureUnit};
use strims_rpc::Timer;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};

use crate::channels::check_network;
use crate::control::{Shared, Slot, Swarm};
use crate::fmp4::check_configs;
use crate::ingress::unit_ingress_options;
use crate::media::{AccessUnit, DecoderConfig, Muxer, TrackId, units_mime};
use crate::opus;
use crate::registry::SwarmKind;
use crate::units::record::{SAMPLE_ENTRY_AVC, SAMPLE_ENTRY_OPUS};
use crate::units::{
    ConfigEntry, END_LINGER, MAX_RECORD_LEN, SignTimer, UnitSigning, UnitWriter, UnitWriterMetrics,
};
use crate::{VideoControl, VideoError, VideoHost};

/// The timescale of a capture's H.264 tracks.
pub const VIDEO_TIMESCALE: u32 = 90_000;
/// The farthest a unit's timestamp is from 0, in microseconds: every such
/// value is exact in a JavaScript number, and none overflows in any
/// timescale.
pub const MAX_TIMESTAMP_US: i64 = 1 << 53;
/// What a config or unit costs against its capture's rate beyond its
/// bytes, so a flood of small units is bounded too.
pub const ITEM_COST: u64 = 64;
/// The longest UNIT record head: kind, `gop_back`, header length and the
/// header ([`units`](crate::units#records)).
pub const MAX_UNIT_HEAD_LEN: usize = 64;
/// How long a closing capture's writer has to write its last output and
/// have the swarm sign its last partial unit.
pub const CLOSE_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// A capture swarm's live window, in chunks: 32 MiB.
const LIVE_WINDOW: usize = 8 * 1024;

/// The options of a capture's swarm signing with `signing`; its URI
/// carries every one but the window. A unit-format ingest session's
/// ([`unit_ingress_options`]), with an 8 Ki-chunk (32 MiB) window.
pub fn capture_options_for(signing: &UnitSigning) -> SwarmOptions {
    SwarmOptions {
        live_window: LIVE_WINDOW,
        ..unit_ingress_options(signing)
    }
}

/// The options of a capture's swarm signing with [`UnitSigning::LARGE`],
/// as a capture with video does.
pub fn capture_options() -> SwarmOptions {
    capture_options_for(&UnitSigning::LARGE)
}

/// A capture the node runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureInfo {
    /// Its transfer id.
    pub id: TransferId,
    pub swarm_uri: String,
    /// The MIME type listed, once a record is written.
    pub mime_type: Option<String>,
    /// Each of its networks' keys, and the id of its listing on that
    /// network's directory once published.
    pub listings: Vec<(Vec<u8>, Option<u64>)>,
    /// How its swarm signs.
    pub signing: UnitSigning,
    /// What its writer has written, as of its last append.
    pub writer: UnitWriterMetrics,
    /// Partial integrity units signed before they filled.
    pub early_signs: u64,
}

/// The decoder config of a capture's config: see the
/// [module docs](self#media).
///
/// # Errors
///
/// [`VideoError::CaptureMedia`] for an unknown codec or a record that
/// doesn't parse.
pub fn decoder_config(c: &CaptureConfig) -> Result<DecoderConfig, VideoError> {
    let track = c.track;
    let (sample_entry, timescale) = match Codec::try_from(c.codec) {
        Ok(Codec::H264) => (SAMPLE_ENTRY_AVC, VIDEO_TIMESCALE),
        Ok(Codec::Opus) => (SAMPLE_ENTRY_OPUS, opus::SAMPLE_RATE),
        _ => {
            return Err(media(format!(
                "track {track}: codec {} is not supported",
                c.codec
            )));
        }
    };
    let entry = ConfigEntry {
        track: TrackId(track),
        version: 0,
        next_seq: 0,
        sample_entry,
        timescale,
        config: &c.config,
    };
    match entry.decoder_config() {
        Some(Ok(config)) => Ok(config),
        Some(Err(e)) => Err(media(e.to_string())),
        None => Err(media(format!("track {track}: no decoder config"))),
    }
}

fn media(reason: String) -> VideoError {
    VideoError::CaptureMedia(reason)
}

/// `us` microseconds in units of `timescale`, rounded to the nearest; `us`
/// is at most [`MAX_TIMESTAMP_US`] from 0.
fn ticks(us: i64, timescale: u32) -> i64 {
    let scaled = i128::from(us) * i128::from(timescale);
    let units = (scaled + 500_000).div_euclid(1_000_000);
    i64::try_from(units).unwrap_or(i64::MAX)
}

/// A capture: its swarm, key and listings, and its appends.
pub(crate) struct Capture<H: Host> {
    swarm: Swarm<H>,
    swarm_id: SwarmId,
    key: Key,
    swarm_uri: String,
    /// Bound: the capture's network keys.
    network_keys: Vec<Vec<u8>>,
    /// A keeper per network key once a record is written. Bound: the
    /// capture's network keys.
    keepers: Vec<(Vec<u8>, ListingKeeper)>,
    mime_type: Option<String>,
    appends: Rc<Appends>,
    rate: Bucket,
    signing: UnitSigning,
    writer: UnitWriterMetrics,
    early_signs: u64,
    /// Set once the writer is done: when it had the swarm sign the
    /// capture's END, if it did.
    done: Rc<Slot<Option<Time>>>,
}

impl<H: Host> Capture<H> {
    pub(crate) fn keepers(&self) -> impl Iterator<Item = &ListingKeeper> {
        self.keepers.iter().map(|(_, k)| k)
    }
}

/// A token bucket of bytes.
#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: u64,
    at: Time,
}

impl Bucket {
    /// Takes `cost` tokens at `now` if the bucket, filled at `rate` a
    /// second up to `burst` since it last took, holds them.
    fn take(&mut self, now: Time, cost: u64, rate: u64, burst: u64) -> bool {
        let elapsed = u64::try_from((now - self.at).as_nanos()).unwrap_or(0);
        let filled = u128::from(elapsed) * u128::from(rate) / 1_000_000_000;
        let filled = u64::try_from(filled).unwrap_or(u64::MAX);
        self.tokens = self.tokens.saturating_add(filled).min(burst);
        self.at = self.at.max(now);
        if cost > self.tokens {
            return false;
        }
        self.tokens -= cost;
        true
    }
}

/// An append waiting for its capture's writer.
struct Append {
    id: u64,
    configs: Vec<CaptureConfig>,
    units: Vec<CaptureUnit>,
    bytes: usize,
    reply: Rc<Slot<Result<(), VideoError>>>,
}

#[derive(Default)]
struct AppendState {
    /// In order. Bound: [`CaptureSettings::max_queued_bytes`] of appends.
    ///
    /// [`CaptureSettings::max_queued_bytes`]: crate::CaptureSettings::max_queued_bytes
    queue: VecDeque<Append>,
    /// The bytes of the appends in the queue.
    queued: usize,
    next_id: u64,
    /// Wakes the writer.
    waker: Option<Waker>,
    closed: bool,
}

/// A capture's appends, between their callers and its writer.
#[derive(Default)]
struct Appends {
    state: RefCell<AppendState>,
}

impl Appends {
    /// Queues an append of `bytes` bytes: its id. Refused once the
    /// capture closed, or when the queue would hold more than `max` bytes.
    fn push(
        &self,
        configs: Vec<CaptureConfig>,
        units: Vec<CaptureUnit>,
        bytes: usize,
        max: usize,
        reply: Rc<Slot<Result<(), VideoError>>>,
    ) -> Result<u64, VideoError> {
        let (id, waker) = {
            let mut s = self.state.borrow_mut();
            if s.closed {
                return Err(VideoError::CaptureNotFound);
            }
            if s.queued.saturating_add(bytes) > max {
                return Err(VideoError::CaptureBusy);
            }
            let id = s.next_id;
            s.next_id += 1;
            s.queued += bytes;
            s.queue.push_back(Append {
                id,
                configs,
                units,
                bytes,
                reply,
            });
            (id, s.waker.take())
        };
        if let Some(w) = waker {
            w.wake();
        }
        Ok(id)
    }

    /// Drops append `id` if it still waits.
    fn cancel(&self, id: u64) {
        let dropped = {
            let mut s = self.state.borrow_mut();
            let at = s.queue.iter().position(|a| a.id == id);
            let a = at.and_then(|at| s.queue.remove(at));
            if let Some(a) = &a {
                s.queued = s.queued.saturating_sub(a.bytes);
            }
            a
        };
        drop(dropped);
    }

    /// The next append for the writer; `None` once the capture closed.
    async fn next(&self) -> Option<Append> {
        poll_fn(|cx| {
            let mut s = self.state.borrow_mut();
            if let Some(a) = s.queue.pop_front() {
                s.queued = s.queued.saturating_sub(a.bytes);
                return Poll::Ready(Some(a));
            }
            if s.closed {
                return Poll::Ready(None);
            }
            s.waker = Some(cx.waker().clone());
            Poll::Pending
        })
        .await
    }

    /// Ends the appends: those waiting fail, and the writer finishes.
    fn close(&self) {
        let (queued, waker) = {
            let mut s = self.state.borrow_mut();
            s.closed = true;
            s.queued = 0;
            (std::mem::take(&mut s.queue), s.waker.take())
        };
        for a in queued {
            a.reply.set(Err(VideoError::CaptureNotFound));
        }
        if let Some(w) = waker {
            w.wake();
        }
    }
}

/// Drops its append from the queue if the caller stops waiting first.
struct Queued<'a> {
    appends: &'a Appends,
    id: u64,
}

impl Drop for Queued<'_> {
    fn drop(&mut self) {
        self.appends.cancel(self.id);
    }
}

/// Sets its slot when dropped: the writer is done, whether it finished or
/// its task was dropped, with when the swarm signed the capture's END, if
/// it did.
struct Done {
    slot: Rc<Slot<Option<Time>>>,
    ended: Option<Time>,
}

impl Drop for Done {
    fn drop(&mut self) {
        self.slot.set(self.ended);
    }
}

/// A capture's writer state: the unit-format writer and what the next
/// append is checked against.
struct MediaWriter {
    units: UnitWriter,
    sign: SignTimer,
    /// The decode time of each track's last unit. Bound: the tracks with
    /// configs, at most `MAX_TRACKS`.
    last: BTreeMap<TrackId, i64>,
    /// Units dropped so far, as counted.
    dropped: u64,
    early_signs: u64,
}

/// An append, checked: its configs and its units' decode times.
struct Checked {
    configs: Vec<DecoderConfig>,
    times: Vec<i64>,
}

impl MediaWriter {
    /// Checks an append against the writer; see the
    /// [module docs](self#media).
    fn check(
        &self,
        configs: &[CaptureConfig],
        units: &[CaptureUnit],
    ) -> Result<Checked, VideoError> {
        let mut merged = self.units.configs().to_vec();
        let mut new = Vec::with_capacity(configs.len());
        for c in configs {
            if configs.iter().filter(|o| o.track == c.track).count() > 1 {
                return Err(media(format!("track {} is configured twice", c.track)));
            }
            let config = decoder_config(c)?;
            merged.retain(|m| m.track != config.track);
            merged.push(config.clone());
            new.push(config);
        }
        merged.sort_by_key(|c| c.track);
        check_configs(&merged).map_err(|e| media(e.to_string()))?;
        let mut last = self.last.clone();
        let mut times = Vec::with_capacity(units.len());
        for u in units {
            let track = TrackId(u.track);
            let Some(config) = merged.iter().find(|c| c.track == track) else {
                return Err(media(format!("track {} has no decoder config", u.track)));
            };
            if u.data.is_empty() {
                return Err(media(format!("track {}: a unit has no data", u.track)));
            }
            if u.data.len() > MAX_RECORD_LEN - MAX_UNIT_HEAD_LEN {
                return Err(media(format!(
                    "track {}: a {}-byte unit is over the record bound",
                    u.track,
                    u.data.len()
                )));
            }
            if u.timestamp_us.unsigned_abs() > MAX_TIMESTAMP_US.unsigned_abs() {
                return Err(media(format!(
                    "track {}: timestamp {} us is out of range",
                    u.track, u.timestamp_us
                )));
            }
            let t = ticks(u.timestamp_us, config.timescale);
            if last.get(&track).is_some_and(|&l| t <= l) {
                return Err(media(format!(
                    "track {}: timestamp {} us is not after the track's last unit",
                    u.track, u.timestamp_us
                )));
            }
            last.insert(track, t);
            times.push(t);
        }
        Ok(Checked {
            configs: new,
            times,
        })
    }

    /// Writes a checked append's configs and units, published at `now`.
    fn apply(
        &mut self,
        checked: Checked,
        units: &[CaptureUnit],
        now: Time,
    ) -> Result<(), VideoError> {
        let failed = |e: crate::units::UnitWriteError| VideoError::StreamWrite(e.to_string());
        for c in &checked.configs {
            self.units.set_config(c).map_err(failed)?;
        }
        let tracks: Vec<TrackId> = self.units.configs().iter().map(|c| c.track).collect();
        self.last.retain(|t, _| tracks.contains(t));
        for (u, &t) in units.iter().zip(&checked.times) {
            let track = TrackId(u.track);
            self.units
                .push(&AccessUnit {
                    track,
                    dts: t,
                    pts: t,
                    keyframe: u.keyframe,
                    payload: &u.data,
                    published: now,
                })
                .map_err(failed)?;
            self.last.insert(track, t);
        }
        Ok(())
    }
}

/// A capture being created, counted against the bound until dropped.
struct Opening<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Opening<K, H, V, T> {
    fn new(shared: &Rc<Shared<K, H, V, T>>) -> Result<Self, VideoError> {
        let open = shared.captures.borrow().len();
        let max = shared.config.capture.max_captures;
        shared.with_swarms(|r| {
            if open + r.opening_captures >= max {
                return Err(VideoError::TooManyCaptures);
            }
            r.opening_captures += 1;
            Ok(())
        })?;
        Ok(Opening {
            shared: shared.clone(),
        })
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for Opening<K, H, V, T> {
    fn drop(&mut self) {
        self.shared.with_swarms(|r| {
            r.opening_captures = r.opening_captures.saturating_sub(1);
        });
    }
}

/// Closes the capture an open made after its caller stopped waiting.
struct OpenReply<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
    reply: Rc<Slot<Result<TransferId, VideoError>>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for OpenReply<K, H, V, T> {
    fn drop(&mut self) {
        if let Some(Ok(id)) = self.reply.take_now() {
            self.shared.queue_close_capture(&id);
        }
    }
}

/// A capture taken out of the node, whose writer, transfer and swarm are
/// still to go.
pub(crate) struct Ended<H: Host> {
    id: TransferId,
    swarm: Swarm<H>,
    done: Rc<Slot<Option<Time>>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Creates a capture for a caller waiting on `reply`, or closes it if
    /// the caller is gone.
    async fn create_capture(
        self: Rc<Self>,
        opening: Opening<K, H, V, T>,
        key: Key,
        network_keys: Vec<Vec<u8>>,
        snippet: ListingSnippet,
        signing: UnitSigning,
        reply: Weak<Slot<Result<TransferId, VideoError>>>,
    ) {
        let res = self
            .start_capture(key, network_keys, snippet, signing)
            .await;
        drop(opening);
        match reply.upgrade() {
            Some(reply) => reply.set(res),
            None => {
                if let Ok(id) = res
                    && let Some(ended) = self.end_capture(&id)
                {
                    self.finish_capture(ended).await;
                }
            }
        }
    }

    async fn start_capture(
        self: &Rc<Self>,
        key: Key,
        network_keys: Vec<Vec<u8>>,
        snippet: ListingSnippet,
        signing: UnitSigning,
    ) -> Result<TransferId, VideoError> {
        let options = capture_options_for(&signing);
        let (swarm, swarm_id, writer) = self
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key: key.clone(),
            })
            .await
            .map_err(|e| self.host_failed("creating a capture swarm", &e))?;
        let t = TransferId::new(&swarm_id, b"");
        let networks = network_keys.iter().cloned().collect();
        let fresh = self.with_swarms(|r| {
            if r.get(&t).is_some() {
                return false;
            }
            r.insert(
                t,
                Some(swarm),
                swarm_id.clone(),
                options.clone(),
                SwarmKind::Capture,
                networks,
                0,
            );
            true
        });
        if !fresh {
            self.remove_host_swarm(swarm).await;
            return Err(VideoError::Host("the capture's swarm id is in use".into()));
        }
        self.transfers.add(swarm, &swarm_id, Vec::new());
        self.publish_joined(&t);
        let pushed = match Signer::new(&key) {
            Ok(signer) => {
                let signed = SignedSnippet::sign(&signer, &snippet, self.timer.now());
                self.directory
                    .push_snippet(&swarm_id, signed)
                    .map_err(VideoError::from)
            }
            Err(e) => Err(e.into()),
        };
        if let Err(e) = pushed {
            self.transfers.remove(t);
            self.with_swarms(|r| r.remove(&t));
            self.remove_host_swarm(swarm).await;
            return Err(e);
        }

        let swarm_uri = Uri::new(swarm_id.clone(), options.uri_options()).to_string();
        let appends = Rc::new(Appends::default());
        let done = Slot::new();
        let config = &self.config.capture;
        // invariant: the capture swarm's chunk size is a valid frame size.
        #[allow(clippy::expect_used)]
        let units =
            UnitWriter::new(options.chunk_size).expect("the capture chunk size is a frame size");
        let media = MediaWriter {
            units,
            sign: signing.timer(options.chunk_size),
            last: BTreeMap::new(),
            dropped: 0,
            early_signs: 0,
        };
        self.spawn(
            self.clone()
                .write_capture(
                    t,
                    appends.clone(),
                    media,
                    writer,
                    Done {
                        slot: done.clone(),
                        ended: None,
                    },
                )
                .boxed_local(),
        );
        let prev = self.captures.borrow_mut().insert(
            t,
            Capture {
                swarm,
                swarm_id,
                key,
                swarm_uri,
                network_keys,
                keepers: Vec::new(),
                mime_type: None,
                appends,
                rate: Bucket {
                    tokens: config.max_burst,
                    at: self.timer.now(),
                },
                signing,
                writer: UnitWriterMetrics::default(),
                early_signs: 0,
                done,
            },
        );
        drop(prev);
        Ok(t)
    }

    /// Writes capture `id`'s appends into its swarm until it closes, then
    /// its last output; see the [module docs](self#media).
    async fn write_capture(
        self: Rc<Self>,
        id: TransferId,
        appends: Rc<Appends>,
        mut media: MediaWriter,
        mut writer: <H as SwarmHost>::Writer,
        mut done: Done,
    ) {
        let mut failed: Option<String> = None;
        loop {
            let next = pin!(appends.next());
            let a = match media.sign.due().filter(|_| failed.is_none()) {
                Some(due) => match select(next, pin!(self.timer.sleep_until(due))).await {
                    Either::Left((a, _)) => a,
                    Either::Right(((), _)) => {
                        if let Err(e) = self.sign_capture(&mut media, &mut writer).await {
                            failed = Some(self.capture_failed(&e));
                        }
                        self.capture_written(&id, &mut media, None);
                        continue;
                    }
                },
                None => next.await,
            };
            let Some(a) = a else {
                break;
            };
            let res = match &failed {
                Some(e) => Err(VideoError::StreamWrite(e.clone())),
                None => {
                    let res = self
                        .write_append(&mut media, &mut writer, &a.configs, &a.units)
                        .await;
                    if let Err(VideoError::StreamWrite(e)) = &res {
                        failed = Some(e.clone());
                    }
                    let mime = units_mime(media.units.configs());
                    let listed = media.units.metrics().records > 0;
                    self.capture_written(&id, &mut media, listed.then_some(mime));
                    res
                }
            };
            a.reply.set(res);
        }
        if failed.is_none() && media.units.finish().is_ok() {
            let now = self.timer.now();
            let written = self.write_output(&mut media, &mut writer, now).await;
            let signed = written.is_ok()
                && (!media.sign.is_partial()
                    || self.sign_capture(&mut media, &mut writer).await.is_ok());
            if signed && media.units.metrics().ends > 0 {
                done.ended = Some(self.timer.now());
            }
        }
        drop(done);
    }

    /// Checks an append and writes it into the swarm.
    async fn write_append(
        &self,
        media: &mut MediaWriter,
        writer: &mut <H as SwarmHost>::Writer,
        configs: &[CaptureConfig],
        units: &[CaptureUnit],
    ) -> Result<(), VideoError> {
        let checked = media.check(configs, units)?;
        let now = self.timer.now();
        media.apply(checked, units, now)?;
        self.write_output(media, writer, now).await.map_err(|e| {
            let e = self.capture_failed(&e);
            VideoError::StreamWrite(e)
        })
    }

    /// Writes the writer's output into the swarm at `now`.
    async fn write_output(
        &self,
        media: &mut MediaWriter,
        writer: &mut <H as SwarmHost>::Writer,
        now: Time,
    ) -> Result<(), String> {
        let out = media.units.take_output();
        let res = if out.is_empty() {
            Ok(())
        } else {
            writer.write(&out).await.map_err(|e| e.to_string())
        };
        media.units.restore(out);
        if res.is_ok() {
            media.sign.written(media.units.offset(), now);
        }
        res
    }

    /// Has the swarm sign the partial integrity unit: ends the frame in
    /// progress, writes it, flushes the swarm writer and goes on where the
    /// next unit starts.
    async fn sign_capture(
        &self,
        media: &mut MediaWriter,
        writer: &mut <H as SwarmHost>::Writer,
    ) -> Result<(), String> {
        media.units.end_frame();
        self.write_output(media, writer, self.timer.now()).await?;
        let next = writer.flush().await.map_err(|e| e.to_string())?;
        media.units.continue_at(next).map_err(|e| e.to_string())?;
        media.sign.flushed(next);
        media.early_signs += 1;
        Ok(())
    }

    /// Counts a failed swarm write; the reason.
    fn capture_failed(&self, e: &str) -> String {
        self.metrics.borrow_mut().write_errors += 1;
        tracing::warn!(error = %e, "writing a capture failed");
        e.to_owned()
    }

    /// Takes capture `id`'s writer metrics after a write, and keeps its
    /// listings of `mime` if set and new.
    fn capture_written(
        self: &Rc<Self>,
        id: &TransferId,
        media: &mut MediaWriter,
        mime: Option<String>,
    ) {
        let m = media.units.metrics();
        let dropped = m.dropped_before_start.saturating_sub(media.dropped);
        media.dropped = m.dropped_before_start;
        self.metrics.borrow_mut().dropped_records += dropped;
        let (keepers, listing, start) = {
            let mut captures = self.captures.borrow_mut();
            let Some(c) = captures.get_mut(id) else {
                return;
            };
            c.writer = m;
            c.early_signs = media.early_signs;
            let Some(mime) = mime.filter(|m| c.mime_type.as_ref() != Some(m)) else {
                return;
            };
            c.mime_type = Some(mime.clone());
            let listing = Listing {
                content: Some(Content::Media(Media {
                    mime_type: mime,
                    swarm_uri: c.swarm_uri.clone(),
                })),
            };
            let start = c.keepers.is_empty();
            if start {
                c.keepers = c
                    .network_keys
                    .iter()
                    .map(|k| (k.clone(), ListingKeeper::new(k.clone(), listing.clone())))
                    .collect();
            }
            let keepers: Vec<ListingKeeper> = c.keepers().cloned().collect();
            (keepers, listing, start)
        };
        let config = self.config.listing.keeper_config();
        for keeper in keepers {
            if start {
                let directory = self.directory.clone();
                self.spawn(
                    async move { keeper.run(&directory, config, |_| {}).await }.boxed_local(),
                );
            } else {
                keeper.set_listing(listing.clone());
            }
        }
    }

    /// Takes a capture out of the node: stops its listings, its snippet
    /// and its appends. What's left goes in [`finish_capture`]; see the
    /// [teardown](self#teardown).
    ///
    /// [`finish_capture`]: Shared::finish_capture
    pub(crate) fn end_capture(&self, id: &TransferId) -> Option<Ended<H>> {
        let c = self.captures.borrow_mut().remove(id)?;
        for k in c.keepers() {
            k.stop();
        }
        self.directory.delete_snippet(&c.swarm_id);
        c.appends.close();
        Some(Ended {
            id: *id,
            swarm: c.swarm,
            done: c.done,
        })
    }

    /// Waits for an ended capture's writer, and [`END_LINGER`] after the
    /// END it had signed, then removes its transfer and swarm.
    pub(crate) async fn finish_capture(&self, ended: Ended<H>) {
        let deadline = self.timer.now() + CLOSE_FLUSH_TIMEOUT;
        let signed = match select(
            pin!(ended.done.take()),
            pin!(self.timer.sleep_until(deadline)),
        )
        .await
        {
            Either::Left((signed, _)) => signed,
            Either::Right(_) => None,
        };
        if let Some(signed) = signed {
            self.timer.sleep_until(signed + END_LINGER).await;
        }
        self.transfers.remove(ended.id);
        self.with_swarms(|r| r.remove(&ended.id));
        self.remove_host_swarm(ended.swarm).await;
    }

    /// Closes capture `id`, finishing it in a task.
    pub(crate) fn queue_close_capture(self: &Rc<Self>, id: &TransferId) {
        if let Some(ended) = self.end_capture(id) {
            let s = self.clone();
            self.spawn(async move { s.finish_capture(ended).await }.boxed_local());
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// Opens a capture listed with `snippet` on the networks
    /// `network_keys` names, its swarm signing with `signing`: its id. See
    /// the [module docs](self).
    ///
    /// # Errors
    ///
    /// The network keys or snippet are refused, the node runs its most
    /// captures, or a node call failed.
    pub async fn open_capture(
        &self,
        network_keys: Vec<Vec<u8>>,
        snippet: ListingSnippet,
        signing: UnitSigning,
    ) -> Result<TransferId, VideoError> {
        let s = &self.shared;
        let c = &s.config.capture;
        if network_keys.is_empty() || network_keys.len() > c.max_network_keys {
            return Err(VideoError::NetworkKeyCount);
        }
        snippet::check(&snippet)?;
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for k in network_keys {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        {
            let mut tx = s.store.read().await?;
            for k in &keys {
                check_network(&mut tx, k).await?;
            }
        }
        let opening = Opening::new(s)?;
        let key = generate_key(&mut s.rng.borrow_mut());
        let reply = OpenReply {
            shared: s.clone(),
            reply: Slot::new(),
        };
        let task = s.clone().create_capture(
            opening,
            key,
            keys,
            snippet,
            signing,
            Rc::downgrade(&reply.reply),
        );
        s.spawn(task.boxed_local());
        reply.reply.take().await
    }

    /// Signs `snippet` with capture `id`'s key and serves it.
    ///
    /// # Errors
    ///
    /// There's no capture `id`, or the snippet is over its bounds.
    pub fn update_capture(
        &self,
        id: &TransferId,
        snippet: ListingSnippet,
    ) -> Result<(), VideoError> {
        snippet::check(&snippet)?;
        let s = &self.shared;
        let (key, swarm_id) = s
            .captures
            .borrow()
            .get(id)
            .map(|c| (c.key.clone(), c.swarm_id.clone()))
            .ok_or(VideoError::CaptureNotFound)?;
        let signed = SignedSnippet::sign(&Signer::new(&key)?, &snippet, s.timer.now());
        s.directory.push_snippet(&swarm_id, signed)?;
        Ok(())
    }

    /// Writes `configs` and then `units` into capture `id`. Resolves once
    /// written; see the [module docs](self#media).
    ///
    /// # Errors
    ///
    /// There's no capture `id` (or it closed first),
    /// [`VideoError::CaptureMedia`] when the append breaks the capture's
    /// rules, [`VideoError::CaptureBusy`] or [`VideoError::CaptureRate`]
    /// over its bounds, or [`VideoError::StreamWrite`] once a swarm write
    /// failed.
    pub async fn append_capture(
        &self,
        id: &TransferId,
        configs: Vec<CaptureConfig>,
        units: Vec<CaptureUnit>,
    ) -> Result<(), VideoError> {
        let s = &self.shared;
        let settings = &s.config.capture;
        if units.len() > settings.max_units_per_append {
            return Err(media(format!(
                "{} units are more than an append's {}",
                units.len(),
                settings.max_units_per_append
            )));
        }
        let bytes = configs.iter().map(|c| c.config.len()).sum::<usize>()
            + units.iter().map(|u| u.data.len()).sum::<usize>();
        let items = (configs.len() + units.len()) as u64;
        let cost = (bytes as u64).saturating_add(items.saturating_mul(ITEM_COST));
        let now = s.timer.now();
        let appends = {
            let mut captures = s.captures.borrow_mut();
            let c = captures.get_mut(id).ok_or(VideoError::CaptureNotFound)?;
            if !c
                .rate
                .take(now, cost, settings.max_rate, settings.max_burst)
            {
                return Err(VideoError::CaptureRate);
            }
            c.appends.clone()
        };
        let reply = Slot::new();
        let id = appends.push(
            configs,
            units,
            bytes,
            settings.max_queued_bytes,
            reply.clone(),
        )?;
        let _queued = Queued {
            appends: &appends,
            id,
        };
        reply.take().await
    }

    /// Closes capture `id`; see the [teardown](self#teardown).
    pub async fn close_capture(&self, id: &TransferId) -> Result<(), VideoError> {
        let s = &self.shared;
        let ended = s.end_capture(id).ok_or(VideoError::CaptureNotFound)?;
        s.finish_capture(ended).await;
        Ok(())
    }

    /// The captures the node runs, in id order.
    pub fn captures(&self) -> Vec<CaptureInfo> {
        self.shared
            .captures
            .borrow()
            .iter()
            .map(|(id, c)| CaptureInfo {
                id: *id,
                swarm_uri: c.swarm_uri.clone(),
                mime_type: c.mime_type.clone(),
                listings: c
                    .network_keys
                    .iter()
                    .map(|k| {
                        let keeper = c.keepers.iter().find(|(key, _)| key == k);
                        (
                            k.clone(),
                            keeper.and_then(|(_, keeper)| keeper.listing_id()),
                        )
                    })
                    .collect(),
                signing: c.signing,
                writer: c.writer,
                early_signs: c.early_signs,
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "capture_test.rs"]
mod tests;
