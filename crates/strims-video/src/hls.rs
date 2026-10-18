// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS media playlists over fMP4 segments, and the routes that serve them.
//!
//! A [`Playlist`] takes finished segments from a source, each an init
//! (`ftyp`+`moov`), its media (`moof`+`mdat`…) and its duration. It keeps
//! the newest few, serves them by number, and renders a live media
//! playlist (RFC 8216, version 7) listing the newest of those. Segments
//! that slid out of the listing stay fetchable for a while, for players
//! that read an earlier playlist.
//!
//! [`HlsRoutes`] maps request paths to the playlists of registered
//! profiles and their streams:
//!
//! - `/hls/<profile key>/<transfer id>/index.m3u8`: the playlist;
//! - `/hls/<profile key>/<transfer id>/init-<id>.mp4`: an init;
//! - `/hls/<profile key>/<transfer id>/<seq>.m4s`: a media segment.
//!
//! Keys are 32 bytes in hex, and the playlist's URIs are relative to it.
//! Registrations are handles, [`HlsProfile`] and [`HlsStream`], whose drop
//! removes them.
//!
//! Every collection is bounded: segments and their bytes by
//! [`PlaylistConfig`], profiles by [`MAX_PROFILES`], streams by
//! [`MAX_STREAMS`], and request paths by [`MAX_PATH_LEN`].

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Write as _};
use std::mem;
use std::rc::{Rc, Weak};

use strims_base::time::{Duration, Time};

/// How many segments a [`Playlist`] lists and keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaylistConfig {
    /// Segments listed, newest last. At most `retained` are.
    pub listed: usize,
    /// Segments kept for fetching, the listed ones among them.
    pub retained: usize,
    /// The most bytes of kept segments and inits.
    pub max_retained_bytes: usize,
}

impl Default for PlaylistConfig {
    fn default() -> Self {
        PlaylistConfig {
            listed: 6,
            retained: 8,
            max_retained_bytes: 64 << 20,
        }
    }
}

struct Segment {
    seq: u64,
    init: u64,
    media: Rc<[u8]>,
    /// The duration in milliseconds, as listed.
    millis: u64,
    /// A discontinuity precedes the segment.
    discontinuity: bool,
}

struct Init {
    id: u64,
    data: Rc<[u8]>,
}

/// A live HLS media playlist and the segments it serves.
///
/// Segment numbers count up from the first, one per pushed segment; init
/// ids count up from the same start, one per distinct init. Kept segments
/// hold consecutive numbers, and the playlist lists the newest of them.
///
/// Bounds: at most [`PlaylistConfig::retained`] segments, and at most
/// [`PlaylistConfig::max_retained_bytes`] of them and the inits they
/// reference, oldest dropped first. An init is kept while a kept segment
/// references it, so there are no more inits than segments.
pub struct Playlist {
    config: PlaylistConfig,
    /// Oldest first, with consecutive numbers.
    segments: VecDeque<Segment>,
    /// The inits the kept segments reference, oldest first.
    inits: VecDeque<Init>,
    retained_bytes: usize,
    next_seq: u64,
    next_init: u64,
    /// Some segment was pushed.
    pushed: bool,
    /// A discontinuity precedes the next pushed segment.
    pending_discontinuity: bool,
    /// Segments ever pushed after a discontinuity.
    discontinuities: u64,
    /// The largest rounded duration pushed, in seconds.
    max_duration: u64,
}

impl Playlist {
    /// An empty playlist numbering segments and inits from 0.
    pub fn new(config: PlaylistConfig) -> Self {
        Playlist::starting_at(config, 0)
    }

    /// An empty playlist numbering segments and inits from `first`.
    pub fn starting_at(config: PlaylistConfig, first: u64) -> Self {
        Playlist {
            config,
            segments: VecDeque::new(),
            inits: VecDeque::new(),
            retained_bytes: 0,
            next_seq: first,
            next_init: first,
            pushed: false,
            pending_discontinuity: false,
            discontinuities: 0,
            max_duration: 0,
        }
    }

    pub fn config(&self) -> &PlaylistConfig {
        &self.config
    }

    /// Appends a segment: `media` decodes with `init` and lasts `duration`
    /// (negative counts as zero). It takes the next segment number.
    ///
    /// An init identical to the newest kept segment's is shared. Any
    /// other gets the next init id, and a discontinuity precedes the
    /// segment unless it's the first. Then the oldest segments are dropped
    /// until the bounds hold. A segment too large to fit is dropped with
    /// all the others, and its init with it, so the next segment starts
    /// a new init.
    pub fn push_segment(&mut self, init: &[u8], media: &[u8], duration: Duration) {
        let shared = self.inits.back().filter(|i| *i.data == *init).map(|i| i.id);
        let init_id = match shared {
            Some(id) => id,
            None => {
                let id = self.next_init;
                self.next_init = self.next_init.saturating_add(1);
                self.retained_bytes = self.retained_bytes.saturating_add(init.len());
                self.inits.push_back(Init {
                    id,
                    data: Rc::from(init),
                });
                self.pending_discontinuity |= self.pushed;
                id
            }
        };

        let millis = duration_millis(duration);
        self.max_duration = self.max_duration.max(rounded_secs(millis));
        let discontinuity = mem::take(&mut self.pending_discontinuity);
        if discontinuity {
            self.discontinuities = self.discontinuities.saturating_add(1);
        }
        self.retained_bytes = self.retained_bytes.saturating_add(media.len());
        self.segments.push_back(Segment {
            seq: self.next_seq,
            init: init_id,
            media: Rc::from(media),
            millis,
            discontinuity,
        });
        self.next_seq = self.next_seq.saturating_add(1);
        self.pushed = true;
        self.trim();
    }

    /// Marks a discontinuity before the next pushed segment: players reset
    /// their decoder and timeline there. Calls before that push mark one.
    pub fn discontinuity(&mut self) {
        self.pending_discontinuity = true;
    }

    /// Drops the oldest segments, and the inits only they referenced,
    /// until the bounds hold.
    fn trim(&mut self) {
        while self.segments.len() > self.config.retained
            || self.retained_bytes > self.config.max_retained_bytes
        {
            let Some(dropped) = self.segments.pop_front() else {
                break;
            };
            self.retained_bytes = self.retained_bytes.saturating_sub(dropped.media.len());
            let oldest = self.segments.front().map(|s| s.init);
            while let Some(init) = self.inits.front() {
                if oldest.is_some_and(|o| init.id >= o) {
                    break;
                }
                self.retained_bytes = self.retained_bytes.saturating_sub(init.data.len());
                self.inits.pop_front();
            }
        }
    }

    /// The listed segments, oldest first.
    fn listed(&self) -> impl Iterator<Item = &Segment> {
        let n = self.config.listed.min(self.segments.len());
        self.segments.iter().skip(self.segments.len() - n)
    }

    /// The playlist's text.
    ///
    /// The first listed segment and every segment whose init differs from
    /// the one before it carry an `EXT-X-MAP` naming their init. The
    /// playlist is live: it never ends.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(160 + 64 * self.config.listed.min(self.segments.len()));
        // Writing to a String can't fail.
        let _ = write!(
            out,
            "#EXTM3U\n\
             #EXT-X-VERSION:7\n\
             #EXT-X-TARGETDURATION:{}\n\
             #EXT-X-MEDIA-SEQUENCE:{}\n\
             #EXT-X-DISCONTINUITY-SEQUENCE:{}\n",
            self.target_duration(),
            self.media_sequence(),
            self.discontinuity_sequence(),
        );
        let mut init = None;
        for s in self.listed() {
            if s.discontinuity {
                out.push_str("#EXT-X-DISCONTINUITY\n");
            }
            if init != Some(s.init) {
                let _ = writeln!(out, "#EXT-X-MAP:URI=\"init-{}.mp4\"", s.init);
                init = Some(s.init);
            }
            let _ = write!(
                out,
                "#EXTINF:{}.{:03},\n{}.m4s\n",
                s.millis / 1000,
                s.millis % 1000,
                s.seq
            );
        }
        out
    }

    /// The bytes of a kept init.
    pub fn init(&self, id: u64) -> Option<Rc<[u8]>> {
        self.inits
            .iter()
            .find(|i| i.id == id)
            .map(|i| Rc::clone(&i.data))
    }

    /// The media of a kept segment.
    pub fn segment(&self, seq: u64) -> Option<Rc<[u8]>> {
        let front = self.segments.front()?;
        let index = usize::try_from(seq.checked_sub(front.seq)?).ok()?;
        self.segments
            .get(index)
            .filter(|s| s.seq == seq)
            .map(|s| Rc::clone(&s.media))
    }

    /// `EXT-X-TARGETDURATION`: the largest segment duration pushed, rounded
    /// to whole seconds, and at least 1.
    ///
    /// Every listed duration must round to at most the target, and a live
    /// playlist learns durations as segments arrive. So the target is the
    /// largest seen, and it only grows.
    pub fn target_duration(&self) -> u64 {
        self.max_duration.max(1)
    }

    /// `EXT-X-MEDIA-SEQUENCE`: the first listed segment's number, or the
    /// next segment's when none is listed.
    pub fn media_sequence(&self) -> u64 {
        self.listed().next().map_or(self.next_seq, |s| s.seq)
    }

    /// `EXT-X-DISCONTINUITY-SEQUENCE`: the discontinuities that preceded
    /// segments no longer listed.
    pub fn discontinuity_sequence(&self) -> u64 {
        let listed = self.listed().filter(|s| s.discontinuity).count();
        self.discontinuities
            .saturating_sub(u64::try_from(listed).unwrap_or(u64::MAX))
    }

    /// The number the next pushed segment takes.
    pub fn next_sequence(&self) -> u64 {
        self.next_seq
    }

    /// The newest kept segment's init id.
    pub fn init_id(&self) -> Option<u64> {
        self.segments.back().map(|s| s.init)
    }

    /// The number of kept segments, listed or not.
    pub fn retained_segments(&self) -> usize {
        self.segments.len()
    }

    /// The bytes of kept segments and inits.
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

impl fmt::Debug for Playlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Playlist")
            .field("config", &self.config)
            .field("media_sequence", &self.media_sequence())
            .field("next_sequence", &self.next_seq)
            .field("retained_segments", &self.segments.len())
            .field("retained_inits", &self.inits.len())
            .field("retained_bytes", &self.retained_bytes)
            .finish_non_exhaustive()
    }
}

/// `d` in whole milliseconds, rounded half up; zero when negative.
fn duration_millis(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).map_or(0, |ns| ns.saturating_add(500_000) / 1_000_000)
}

/// Milliseconds in whole seconds, rounded half up.
fn rounded_secs(millis: u64) -> u64 {
    millis.saturating_add(500) / 1000
}

/// The length of profile keys and transfer ids in paths.
pub const KEY_LEN: usize = 32;

/// The most profiles [`HlsRoutes`] serves. A node factory runs one profile
/// at a time, and each registers once.
pub const MAX_PROFILES: usize = 16;

/// The most streams a profile serves. Callers set their own lower cap;
/// this one bounds the registry.
pub const MAX_STREAMS: usize = 16;

/// The longest request path [`HlsRoutes::get`] parses. Route paths are at
/// most 164 bytes.
pub const MAX_PATH_LEN: usize = 256;

const PLAYLIST_TYPE: &str = "application/vnd.apple.mpegurl";
const INIT_TYPE: &str = "video/mp4";
const SEGMENT_TYPE: &str = "video/iso.segment";
const ERROR_TYPE: &str = "text/plain; charset=utf-8";
const NO_CACHE: &str = "no-cache";
const IMMUTABLE: &str = "max-age=31536000, immutable";
const NO_STORE: &str = "no-store";

/// Why a registration failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RouteError {
    #[error("HLS profile keys are 32 bytes")]
    KeyLength,
    #[error("too many HLS profiles")]
    TooManyProfiles,
    #[error("too many HLS streams")]
    TooManyStreams,
}

/// The answer to a GET.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsResponse {
    /// 200; 400 for a malformed path; 404 for a missing resource.
    pub status: u16,
    pub content_type: &'static str,
    pub cache_control: &'static str,
    pub body: Rc<[u8]>,
}

impl HlsResponse {
    fn ok(content_type: &'static str, cache_control: &'static str, body: Rc<[u8]>) -> Self {
        HlsResponse {
            status: 200,
            content_type,
            cache_control,
            body,
        }
    }

    fn error(status: u16) -> Self {
        let body: &[u8] = match status {
            400 => b"bad request\n",
            _ => b"not found\n",
        };
        HlsResponse {
            status,
            content_type: ERROR_TYPE,
            cache_control: NO_STORE,
            body: Rc::from(body),
        }
    }
}

/// The path of a stream's playlist, from the root of the HLS server.
pub fn playlist_path(profile_key: &[u8], transfer_id: &[u8; KEY_LEN]) -> String {
    let mut path = String::with_capacity(16 + 2 * (profile_key.len() + KEY_LEN));
    path.push_str("/hls/");
    push_hex(&mut path, profile_key);
    path.push('/');
    push_hex(&mut path, transfer_id);
    path.push_str("/index.m3u8");
    path
}

fn push_hex(out: &mut String, bytes: &[u8]) {
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
}

type Profiles = RefCell<BTreeMap<[u8; KEY_LEN], Rc<ProfileState>>>;

struct ProfileState {
    key: [u8; KEY_LEN],
    streams: RefCell<BTreeMap<[u8; KEY_LEN], Rc<StreamState>>>,
}

struct StreamState {
    playlist: RefCell<Playlist>,
    last_access: Cell<Time>,
}

/// The HLS streams of every registered profile, by path.
///
/// Clones share the registry. The driver serving HTTP answers requests with
/// [`get`](HlsRoutes::get), and each profile's video control registers its
/// streams through an [`HlsProfile`].
#[derive(Clone, Default)]
pub struct HlsRoutes {
    profiles: Rc<Profiles>,
}

impl HlsRoutes {
    pub fn new() -> Self {
        HlsRoutes::default()
    }

    /// Registers a profile by its public key, replacing any earlier
    /// registration of the key.
    pub fn profile(&self, profile_key: &[u8]) -> Result<HlsProfile, RouteError> {
        let key: [u8; KEY_LEN] = profile_key.try_into().map_err(|_| RouteError::KeyLength)?;
        let state = Rc::new(ProfileState {
            key,
            streams: RefCell::new(BTreeMap::new()),
        });
        let replaced = {
            let mut profiles = self.profiles.borrow_mut();
            if profiles.len() >= MAX_PROFILES && !profiles.contains_key(&key) {
                return Err(RouteError::TooManyProfiles);
            }
            profiles.insert(key, Rc::clone(&state))
        };
        drop(replaced);
        Ok(HlsProfile {
            routes: Rc::downgrade(&self.profiles),
            state,
        })
    }

    /// The number of registered profiles.
    pub fn profiles(&self) -> usize {
        self.profiles.borrow().len()
    }

    /// Answers a GET of `path` at `now`. A query string is ignored.
    ///
    /// A 200 records `now` as the stream's last access. Playlists can't be
    /// cached; inits and segments can, since their paths never name other
    /// bytes: numbers start at the stream's open time (see
    /// [`HlsProfile::open_stream`]).
    pub fn get(&self, now: Time, path: &str) -> HlsResponse {
        let route = match parse_path(path) {
            Ok(route) => route,
            Err(status) => return HlsResponse::error(status),
        };
        let profile = self.profiles.borrow().get(&route.profile).cloned();
        let Some(profile) = profile else {
            return HlsResponse::error(404);
        };
        let stream = profile.streams.borrow().get(&route.stream).cloned();
        let Some(stream) = stream else {
            return HlsResponse::error(404);
        };
        let response = {
            let playlist = stream.playlist.borrow();
            match route.file {
                File::Playlist => Some(HlsResponse::ok(
                    PLAYLIST_TYPE,
                    NO_CACHE,
                    Rc::from(playlist.render().into_bytes()),
                )),
                File::Init(id) => playlist
                    .init(id)
                    .map(|body| HlsResponse::ok(INIT_TYPE, IMMUTABLE, body)),
                File::Segment(seq) => playlist
                    .segment(seq)
                    .map(|body| HlsResponse::ok(SEGMENT_TYPE, IMMUTABLE, body)),
            }
        };
        match response {
            Some(response) => {
                stream.last_access.set(stream.last_access.get().max(now));
                response
            }
            None => HlsResponse::error(404),
        }
    }
}

impl fmt::Debug for HlsRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HlsRoutes")
            .field("profiles", &self.profiles())
            .finish()
    }
}

/// A profile's registration in [`HlsRoutes`]. Dropping it unregisters the
/// profile and its streams.
#[must_use = "dropping the handle unregisters the profile"]
pub struct HlsProfile {
    routes: Weak<Profiles>,
    state: Rc<ProfileState>,
}

impl HlsProfile {
    pub fn key(&self) -> &[u8; KEY_LEN] {
        &self.state.key
    }

    /// Registers a stream of the transfer, opened at `now`, replacing any
    /// earlier registration of the transfer.
    ///
    /// The playlist numbers segments and inits from `now` in Unix
    /// milliseconds. Segments last far longer than a millisecond, so a
    /// stream reopened at the same path starts past the earlier stream's
    /// numbers, and a cache never holds other bytes under a segment's or
    /// init's path.
    pub fn open_stream(
        &self,
        now: Time,
        transfer_id: [u8; KEY_LEN],
        config: PlaylistConfig,
    ) -> Result<HlsStream, RouteError> {
        let first = u64::try_from(now.unix_millis()).unwrap_or(0);
        let state = Rc::new(StreamState {
            playlist: RefCell::new(Playlist::starting_at(config, first)),
            last_access: Cell::new(now),
        });
        let replaced = {
            let mut streams = self.state.streams.borrow_mut();
            if streams.len() >= MAX_STREAMS && !streams.contains_key(&transfer_id) {
                return Err(RouteError::TooManyStreams);
            }
            streams.insert(transfer_id, Rc::clone(&state))
        };
        drop(replaced);
        Ok(HlsStream {
            profile: Rc::downgrade(&self.state),
            transfer_id,
            path: playlist_path(&self.state.key, &transfer_id),
            state,
        })
    }

    /// The number of registered streams.
    pub fn streams(&self) -> usize {
        self.state.streams.borrow().len()
    }
}

impl Drop for HlsProfile {
    fn drop(&mut self) {
        let Some(profiles) = self.routes.upgrade() else {
            return;
        };
        let removed = {
            let mut profiles = profiles.borrow_mut();
            match profiles.get(&self.state.key) {
                Some(s) if Rc::ptr_eq(s, &self.state) => profiles.remove(&self.state.key),
                _ => None,
            }
        };
        drop(removed);
    }
}

impl fmt::Debug for HlsProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HlsProfile")
            .field("streams", &self.streams())
            .finish_non_exhaustive()
    }
}

/// A stream's registration under an [`HlsProfile`], and its playlist.
/// Dropping it unregisters the stream.
#[must_use = "dropping the handle unregisters the stream"]
pub struct HlsStream {
    profile: Weak<ProfileState>,
    transfer_id: [u8; KEY_LEN],
    path: String,
    state: Rc<StreamState>,
}

impl HlsStream {
    pub fn transfer_id(&self) -> &[u8; KEY_LEN] {
        &self.transfer_id
    }

    /// The playlist's path; see [`playlist_path`].
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The time of the latest successful GET of the stream, or its open
    /// time before any.
    pub fn last_access(&self) -> Time {
        self.state.last_access.get()
    }

    /// See [`Playlist::push_segment`].
    pub fn push_segment(&self, init: &[u8], media: &[u8], duration: Duration) {
        self.state
            .playlist
            .borrow_mut()
            .push_segment(init, media, duration);
    }

    /// See [`Playlist::discontinuity`].
    pub fn discontinuity(&self) {
        self.state.playlist.borrow_mut().discontinuity();
    }

    /// The bytes the playlist keeps; see [`Playlist::retained_bytes`].
    pub fn retained_bytes(&self) -> usize {
        self.state.playlist.borrow().retained_bytes()
    }
}

impl Drop for HlsStream {
    fn drop(&mut self) {
        let Some(profile) = self.profile.upgrade() else {
            return;
        };
        let removed = {
            let mut streams = profile.streams.borrow_mut();
            match streams.get(&self.transfer_id) {
                Some(s) if Rc::ptr_eq(s, &self.state) => streams.remove(&self.transfer_id),
                _ => None,
            }
        };
        drop(removed);
    }
}

impl fmt::Debug for HlsStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HlsStream")
            .field("path", &self.path)
            .field("last_access", &self.last_access())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum File {
    Playlist,
    Init(u64),
    Segment(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Route {
    profile: [u8; KEY_LEN],
    stream: [u8; KEY_LEN],
    file: File,
}

/// A request path's route, or the status of its refusal: 404 outside
/// `/hls/` and for unknown file names, 400 for anything else malformed.
fn parse_path(path: &str) -> Result<Route, u16> {
    if path.len() > MAX_PATH_LEN {
        return Err(400);
    }
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    let rest = path.strip_prefix("/hls/").ok_or(404u16)?;
    let mut parts = rest.split('/');
    let (Some(profile), Some(stream), Some(file), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(400);
    };
    Ok(Route {
        profile: decode_key(profile).ok_or(400u16)?,
        stream: decode_key(stream).ok_or(400u16)?,
        file: parse_file(file)?,
    })
}

fn parse_file(name: &str) -> Result<File, u16> {
    if name == "index.m3u8" {
        return Ok(File::Playlist);
    }
    if let Some(seq) = name.strip_suffix(".m4s") {
        return parse_number(seq).map(File::Segment).ok_or(400);
    }
    if let Some(id) = name
        .strip_prefix("init-")
        .and_then(|n| n.strip_suffix(".mp4"))
    {
        return parse_number(id).map(File::Init).ok_or(400);
    }
    Err(if name.is_empty() { 400 } else { 404 })
}

/// A decimal number of ASCII digits only.
fn parse_number(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// A key of exactly [`KEY_LEN`] bytes in hex, either case.
fn decode_key(s: &str) -> Option<[u8; KEY_LEN]> {
    let (pairs, []) = s.as_bytes().as_chunks::<2>() else {
        return None;
    };
    if pairs.len() != KEY_LEN {
        return None;
    }
    let mut key = [0; KEY_LEN];
    for (b, &[hi, lo]) in key.iter_mut().zip(pairs) {
        *b = (hex_digit(hi)? << 4) | hex_digit(lo)?;
    }
    Some(key)
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[path = "hls_test.rs"]
mod tests;
