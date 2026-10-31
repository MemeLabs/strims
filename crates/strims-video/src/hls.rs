// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS media playlists over fMP4 segments, the muxer that cuts access
//! units into them, and the routes that serve them.
//!
//! A [`Playlist`] takes finished segments from a source, each an init
//! (`ftyp`+`moov`), its media (`moof`+`mdat`…) and its duration. It keeps
//! the newest, serves them by number, and renders a live media playlist
//! (RFC 8216, version 7) listing the newest of those, which ends with
//! `EXT-X-ENDLIST` once the stream ended. Segments that slid out of the
//! listing stay fetchable for a while, for players that read an earlier
//! playlist.
//!
//! # Target duration
//!
//! A playlist's `EXT-X-TARGETDURATION` is fixed for its life
//! ([`PlaylistConfig::target`]): a live playlist's target may never
//! change. Every segment's duration, as listed in milliseconds and
//! rounded to the nearest second, must be at most the target; the
//! segment sources keep theirs within it ([`fits_target`]). The listing
//! holds at least three targets of media, which players start at least
//! that far from the end of.
//!
//! A playlist never declares `EXT-X-INDEPENDENT-SEGMENTS`. A live
//! playlist can't drop a tag it declared, and a stream whose GOP outlasts
//! the target has segments that start mid-GOP.
//!
//! # Segments
//!
//! [`HlsMuxer`] cuts a stream's access units into segments of at most the
//! target, each a run of fragments ([`FragmentWriter`]) after an init:
//!
//! - The first segment starts at the first keyframe of the cut track (the
//!   video track with the lowest id). A stream without video has none,
//!   and no segments.
//! - A keyframe of the cut track ends the segment once it has lasted
//!   [`KEYFRAME_CUT_FLOOR`], and starts the next.
//! - A unit of the cut track that, with a frame a quarter longer than the
//!   longer of its last two, would take the segment past the target ends
//!   it mid-GOP, and starts the next segment. So only a GOP that rounds
//!   above the target, or comes within a quarter frame of it, is split;
//!   the margin covers timestamps rounded to milliseconds. While the cut
//!   track sends nothing for two of its frames, a unit of another track
//!   ends the segment the same way, and the cut track's last sample ends
//!   at the cut.
//! - Changed configs end the segment at the next keyframe of the cut
//!   track, whose segment's init declares them; until then, units of the
//!   changed tracks are dropped.
//! - A gap ([`HlsMuxer::gap`]) ends the segment, and units are dropped up
//!   to the next keyframe of the cut track, whose segment follows a
//!   discontinuity.
//!
//! Within a segment, a fragment ends before each keyframe of the cut
//! track, and before the cut track's next unit once the fragment holds
//! [`HlsMuxerConfig::fragment_duration`] of media.
//!
//! # Routes
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
//! Every collection is bounded: segments by [`MAX_SEGMENTS`] and their
//! bytes by [`PlaylistConfig`], a muxer's segment by
//! [`HlsMuxerConfig::max_segment_bytes`], profiles by [`MAX_PROFILES`],
//! streams by [`MAX_STREAMS`], and request paths by [`MAX_PATH_LEN`].

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Write as _};
use std::mem;
use std::rc::{Rc, Weak};

use strims_base::time::{Duration, Time};

use crate::fmp4::{FragmentWriter, InitError, MAX_TRACKS, Written, write_init};
use crate::media::{AccessUnit, DecoderConfig, Muxer, TrackId, TrackKind};

/// The most segments a [`Playlist`] keeps, listed or not.
pub const MAX_SEGMENTS: usize = 256;

/// How long a [`Playlist`]'s segments last, and how many it lists and
/// keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaylistConfig {
    /// `EXT-X-TARGETDURATION`, in whole seconds; 0 counts as 1.
    pub target: u64,
    /// The least media time listed: the newest segments whose durations
    /// add up to it, and to three targets at least.
    pub listed: Duration,
    /// The least media time kept for fetching, the listed segments among
    /// it: the newest segments whose durations add up to it, and to the
    /// listed time at least.
    pub retained: Duration,
    /// The most bytes of kept segments and inits.
    pub max_retained_bytes: usize,
}

impl Default for PlaylistConfig {
    fn default() -> Self {
        PlaylistConfig {
            target: 4,
            listed: Duration::from_secs(16),
            retained: Duration::from_secs(40),
            max_retained_bytes: 64 << 20,
        }
    }
}

impl PlaylistConfig {
    /// The target, at least 1.
    fn target_at_least_one(&self) -> u64 {
        self.target.max(1)
    }

    /// The least listed media time, in milliseconds.
    fn listed_millis(&self) -> u64 {
        duration_millis(self.listed).max(self.target_at_least_one().saturating_mul(3000))
    }

    /// The least kept media time, in milliseconds.
    fn retained_millis(&self) -> u64 {
        duration_millis(self.retained).max(self.listed_millis())
    }
}

/// `d` as a target duration: whole seconds, rounded up, at least 1.
pub fn target_secs(d: Duration) -> u64 {
    let secs = u64::try_from(d.as_nanos())
        .unwrap_or(0)
        .div_ceil(1_000_000_000);
    secs.max(1)
}

/// Whether a segment lasting `d` can be listed under target duration
/// `target`: in milliseconds, rounded to the nearest second, it is at
/// most the target.
pub fn fits_target(d: Duration, target: u64) -> bool {
    rounded_secs(duration_millis(d)) <= target.max(1)
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
/// Bounds: at most [`MAX_SEGMENTS`] segments, and at most
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
    /// The kept segments' durations, in milliseconds.
    retained_millis: u64,
    next_seq: u64,
    next_init: u64,
    /// Some segment was pushed.
    pushed: bool,
    /// A discontinuity precedes the next pushed segment.
    pending_discontinuity: bool,
    /// Segments ever pushed after a discontinuity.
    discontinuities: u64,
    /// The stream ended: the playlist lists `EXT-X-ENDLIST`.
    ended: bool,
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
            retained_millis: 0,
            next_seq: first,
            next_init: first,
            pushed: false,
            pending_discontinuity: false,
            discontinuities: 0,
            ended: false,
        }
    }

    pub fn config(&self) -> &PlaylistConfig {
        &self.config
    }

    /// Appends a segment: `media` decodes with `init` and lasts `duration`
    /// (negative counts as zero), which [`fits_target`] the playlist's
    /// target. It takes the next segment number.
    ///
    /// An init identical to the newest kept segment's is shared. Any
    /// other gets the next init id, and a discontinuity precedes the
    /// segment unless it's the first. Then the oldest segments are dropped
    /// until the bounds hold. A segment too large to fit is dropped with
    /// all the others, and its init with it, so the next segment starts
    /// a new init.
    ///
    /// A segment pushed after the playlist [ended](Self::end), a later
    /// session's, takes the end back off, so the playlist is live again,
    /// and follows a discontinuity.
    pub fn push_segment(&mut self, init: &[u8], media: &[u8], duration: Duration) {
        if mem::take(&mut self.ended) {
            self.pending_discontinuity |= self.pushed;
        }
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
        let discontinuity = mem::take(&mut self.pending_discontinuity);
        if discontinuity {
            self.discontinuities = self.discontinuities.saturating_add(1);
        }
        self.retained_bytes = self.retained_bytes.saturating_add(media.len());
        self.retained_millis = self.retained_millis.saturating_add(millis);
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

    /// The stream ended: the playlist ends with `EXT-X-ENDLIST`, until a
    /// segment is pushed.
    pub fn end(&mut self) {
        self.ended = true;
    }

    /// Whether the playlist ended.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Drops the oldest segments, and the inits only they referenced,
    /// until the bounds hold and the segments after the oldest would last
    /// less than the kept media time.
    fn trim(&mut self) {
        let keep = self.config.retained_millis();
        while let Some(front) = self.segments.front() {
            let rest = self.retained_millis.saturating_sub(front.millis);
            let over = self.segments.len() > MAX_SEGMENTS
                || self.retained_bytes > self.config.max_retained_bytes
                || rest >= keep;
            if !over {
                break;
            }
            let Some(dropped) = self.segments.pop_front() else {
                break;
            };
            self.retained_bytes = self.retained_bytes.saturating_sub(dropped.media.len());
            self.retained_millis = self.retained_millis.saturating_sub(dropped.millis);
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

    /// The listed segments, oldest first: the fewest newest segments that
    /// last the listed media time, or every kept one.
    fn listed(&self) -> impl Iterator<Item = &Segment> {
        let want = self.config.listed_millis();
        let mut n = 0;
        let mut millis = 0u64;
        for s in self.segments.iter().rev() {
            if millis >= want {
                break;
            }
            millis = millis.saturating_add(s.millis);
            n += 1;
        }
        self.segments.iter().skip(self.segments.len() - n)
    }

    /// The playlist's text.
    ///
    /// The first listed segment and every segment whose init differs from
    /// the one before it carry an `EXT-X-MAP` naming their init. A live
    /// playlist lists no end; an [ended](Self::end) one ends with
    /// `EXT-X-ENDLIST`.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(160 + 64 * self.segments.len());
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
        if self.ended {
            out.push_str("#EXT-X-ENDLIST\n");
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

    /// `EXT-X-TARGETDURATION`: the configured target, at least 1.
    pub fn target_duration(&self) -> u64 {
        self.config.target_at_least_one()
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

    /// The number of listed segments.
    pub fn listed_segments(&self) -> usize {
        self.listed().count()
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

/// The least a segment lasts before a keyframe of its cut track ends it.
///
/// It stays under half a second, so a segment that holds less than it
/// and then a GOP rounding to at most the target rounds to at most the
/// target too: a keyframe it didn't end at never makes the next GOP split.
pub const KEYFRAME_CUT_FLOOR: Duration = Duration::from_millis(400);

/// The most finished segments an [`HlsMuxer`] holds for
/// [`HlsMuxer::take_segment`]; older ones are dropped.
pub const MAX_PENDING_SEGMENTS: usize = 4;

/// The longest init segment an [`HlsMuxer`] writes: configs whose init
/// would be longer are refused ([`HlsMuxerError::InitTooLong`]).
pub const MAX_INIT_LEN: usize = u16::MAX as usize;

/// The media a segment's fragment holds by default
/// ([`HlsMuxerConfig::fragment_duration`]): a few video frames and their
/// audio, so a fragment's `prft`, `moof` and `mdat` headers stay a small
/// share of its bytes.
pub const DEFAULT_FRAGMENT_DURATION: Duration = Duration::from_millis(100);

/// How an [`HlsMuxer`] cuts segments and fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HlsMuxerConfig {
    /// The playlist's target duration in whole seconds
    /// ([`PlaylistConfig::target`]); 0 counts as 1.
    pub target: u64,
    /// The media a fragment holds before the cut track's next unit ends
    /// it.
    pub fragment_duration: Duration,
    /// The most bytes of a segment's media, its held samples included.
    pub max_segment_bytes: usize,
}

impl Default for HlsMuxerConfig {
    fn default() -> Self {
        HlsMuxerConfig {
            target: 4,
            fragment_duration: DEFAULT_FRAGMENT_DURATION,
            max_segment_bytes: 16 << 20,
        }
    }
}

/// A segment an [`HlsMuxer`] finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsSegment {
    /// The init its media decodes with, shared by the segments of the
    /// same configs.
    pub init: Rc<[u8]>,
    /// Its fragments, back to back.
    pub media: Vec<u8>,
    /// From its first unit to the unit that ended it; at a gap or the
    /// stream's end, its cut track's samples, or its longest track's
    /// without them.
    pub duration: Duration,
    /// It starts at a keyframe of the cut track; it starts mid-GOP when
    /// not.
    pub keyframe: bool,
    /// A gap came before it: its timestamps don't follow on from the
    /// segment before.
    pub discontinuity: bool,
}

/// What an [`HlsMuxer`] has made and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HlsMuxerMetrics {
    pub segments: u64,
    /// Segments that start mid-GOP.
    pub mid_gop_segments: u64,
    /// Segments that don't [`fit`](fits_target) the target: a sample
    /// lasted past it with no unit to cut at.
    pub overlong_segments: u64,
    /// Units dropped while no segment was open: before the cut track's
    /// first keyframe, and after a gap up to its next.
    pub dropped_awaiting_keyframe: u64,
    /// Units of a changed or new track before the segment that declares
    /// its config.
    pub dropped_pending_config: u64,
    /// Units a segment couldn't hold within
    /// [`HlsMuxerConfig::max_segment_bytes`] even alone; units are then
    /// dropped up to the next keyframe.
    pub dropped_oversized: u64,
    /// Finished segments dropped untaken.
    pub dropped_segments: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HlsMuxerError {
    #[error("invalid decoder config: {0}")]
    Config(#[from] InitError),
    #[error("the init segment is longer than {MAX_INIT_LEN} bytes")]
    InitTooLong,
    #[error("track {0} has no decoder config")]
    UnknownTrack(u32),
}

/// A track's latest unit time, and the times between its last three
/// units.
#[derive(Debug, Clone, Copy)]
struct Timing {
    track: TrackId,
    last: Duration,
    step: Duration,
    previous: Duration,
}

impl Timing {
    /// The longer of the last two steps.
    fn longest_step(&self) -> Duration {
        self.step.max(self.previous)
    }
}

/// The segment in progress.
#[derive(Debug)]
struct Open {
    /// Its first unit's decode time.
    start: Duration,
    keyframe: bool,
    discontinuity: bool,
    /// The fragments written so far.
    media: Vec<u8>,
    /// The decode time of the first unit the writer holds; `None` while
    /// it holds none.
    fragment_start: Option<Duration>,
}

/// What a unit does to the segment in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// No segment is open and the unit can't start one.
    Drop,
    /// The unit starts a segment, at a keyframe or not.
    Start(bool),
    /// The unit ends the segment in progress and starts the next, at a
    /// keyframe or not.
    Cut(bool),
    /// The unit ends the fragment in progress.
    Fragment,
    Add,
}

/// Cuts access units into HLS segments; see the
/// [module docs](self#segments).
///
/// It is a [`Muxer`]: its output is the segments it finishes, which
/// [`take_segment`](Self::take_segment) hands out.
///
/// # Bounds
///
/// A segment's fragments and held samples stay within
/// [`HlsMuxerConfig::max_segment_bytes`]: a unit that would take them past
/// it ends the segment, and one that can't fit even alone is dropped with
/// the units after it up to the next keyframe. It holds at most
/// [`MAX_TRACKS`] configs and [`MAX_PENDING_SEGMENTS`] finished segments,
/// and reuses its buffers but for each segment's media.
#[derive(Debug)]
pub struct HlsMuxer {
    config: HlsMuxerConfig,
    /// The current configs, at most [`MAX_TRACKS`].
    configs: BTreeMap<TrackId, DecoderConfig>,
    /// The configs the init declares, in init order: the writer's tracks.
    declared: Vec<DecoderConfig>,
    /// The configs differ from the declared ones.
    changed: bool,
    init: Rc<[u8]>,
    /// Writes fragments only when flushed.
    writer: FragmentWriter,
    scratch: Vec<u8>,
    segment: Option<Open>,
    /// Per track that had units; at most [`MAX_TRACKS`].
    timing: Vec<Timing>,
    /// A discontinuity precedes the next segment.
    pending_discontinuity: bool,
    /// A segment was finished, so a gap after it is a discontinuity.
    finished: bool,
    out: VecDeque<HlsSegment>,
    metrics: HlsMuxerMetrics,
}

impl HlsMuxer {
    pub fn new(config: HlsMuxerConfig) -> Self {
        HlsMuxer {
            config,
            configs: BTreeMap::new(),
            declared: Vec::new(),
            changed: false,
            init: Rc::from(&[][..]),
            writer: FragmentWriter::new(Duration::MAX),
            scratch: Vec::new(),
            segment: None,
            timing: Vec::new(),
            pending_discontinuity: false,
            finished: false,
            out: VecDeque::new(),
            metrics: HlsMuxerMetrics::default(),
        }
    }

    pub fn config(&self) -> HlsMuxerConfig {
        self.config
    }

    pub fn metrics(&self) -> HlsMuxerMetrics {
        self.metrics
    }

    /// The oldest finished segment not yet taken.
    pub fn take_segment(&mut self) -> Option<HlsSegment> {
        self.out.pop_front()
    }

    /// Units went missing here: ends the segment in progress, and drops
    /// units up to the cut track's next keyframe, whose segment follows a
    /// discontinuity.
    pub fn gap(&mut self) {
        self.end(None, None);
        self.pending_discontinuity |= self.finished;
    }

    /// The stream starts over with configs set anew: a [`gap`](Self::gap),
    /// and every config is forgotten, so tracks the new configs leave out
    /// are gone.
    pub fn reset(&mut self) {
        self.gap();
        self.configs.clear();
        self.declared.clear();
        self.changed = false;
        self.timing.clear();
    }

    /// The track whose keyframes start segments.
    fn cut_track(&self) -> Option<&DecoderConfig> {
        self.configs.values().find(|c| c.kind == TrackKind::Video)
    }

    /// Whether the current configs are the declared ones.
    fn declares_current(&self) -> bool {
        self.configs.len() == self.declared.len()
            && ordered(&self.configs)
                .zip(&self.declared)
                .all(|(a, b)| a == b)
    }

    /// Whether `track`'s units go into the segment: the init declares its
    /// current config.
    fn declares(&self, track: TrackId) -> bool {
        self.declared
            .iter()
            .any(|c| c.track == track && self.configs.get(&track) == Some(c))
    }

    fn timing(&self, track: TrackId) -> Option<&Timing> {
        self.timing.iter().find(|t| t.track == track)
    }

    /// Records a unit of `track` at `t`: the longer of the times between
    /// its last three units, zero before it has two after one another.
    fn note(&mut self, track: TrackId, t: Duration) -> Duration {
        match self.timing.iter_mut().find(|x| x.track == track) {
            Some(x) => {
                if t > x.last {
                    x.previous = x.step;
                    x.step = t - x.last;
                }
                x.last = x.last.max(t);
                x.longest_step()
            }
            None => {
                if self.timing.len() < MAX_TRACKS {
                    self.timing.push(Timing {
                        track,
                        last: t,
                        step: Duration::ZERO,
                        previous: Duration::ZERO,
                    });
                }
                Duration::ZERO
            }
        }
    }

    /// The cut track sent nothing for two of its frames before `t`.
    fn cut_track_late(&self, t: Duration) -> bool {
        let Some(c) = self.cut_track() else {
            return false;
        };
        self.timing(c.track)
            .is_some_and(|x| x.step > Duration::ZERO && t > x.last + x.step + x.step)
    }

    /// What the unit of `track` at `t` does, the longer of its track's last
    /// two steps `step`.
    fn step_of(&self, u: &AccessUnit<'_>, t: Duration, step: Duration) -> Step {
        let cut = self.cut_track().is_some_and(|c| c.track == u.track);
        let key = cut && u.keyframe;
        let Some(open) = &self.segment else {
            return if key { Step::Start(true) } else { Step::Drop };
        };
        let d = t - open.start;
        if key && (self.changed || d >= KEYFRAME_CUT_FLOOR) {
            return Step::Cut(true);
        }
        if !self.declares(u.track) {
            // Dropped: it shapes no fragment or segment.
            return Step::Add;
        }
        let held = !open.media.is_empty() || open.fragment_start.is_some();
        if (cut || self.cut_track_late(t))
            && held
            && d > Duration::ZERO
            && !fits_target(d + step + step / 4, self.config.target)
        {
            return Step::Cut(false);
        }
        let fragment_due = open
            .fragment_start
            .is_some_and(|f| key || t - f >= self.config.fragment_duration);
        if cut && fragment_due {
            return Step::Fragment;
        }
        Step::Add
    }

    /// The decode time `next` a cut before the unit of `track` at `t`
    /// names: the unit's own, or `t` in the cut track's timescale.
    fn cut_at(&self, u: &AccessUnit<'_>, t: Duration) -> Option<(TrackId, i64)> {
        let c = self.cut_track()?;
        if c.track == u.track {
            return Some((u.track, u.dts));
        }
        Some((c.track, timescale_units(t, c.timescale)))
    }

    /// Starts a segment at the unit at `t`. A keyframe of changed configs
    /// declares them in a new init.
    fn start(
        &mut self,
        u: &AccessUnit<'_>,
        t: Duration,
        keyframe: bool,
    ) -> Result<(), HlsMuxerError> {
        if keyframe && (self.changed || self.declared.is_empty()) {
            let configs: Vec<DecoderConfig> = ordered(&self.configs).cloned().collect();
            let delay = composition_delay(u, &self.configs);
            self.scratch.clear();
            write_init(&configs, delay, &mut self.scratch)?;
            if self.scratch.len() > MAX_INIT_LEN {
                return Err(HlsMuxerError::InitTooLong);
            }
            self.writer.set_tracks(&configs, delay)?;
            self.init = Rc::from(&self.scratch[..]);
            self.declared = configs;
            self.changed = false;
        }
        if !keyframe {
            self.metrics.mid_gop_segments += 1;
        }
        self.segment = Some(Open {
            start: t,
            keyframe,
            discontinuity: mem::take(&mut self.pending_discontinuity),
            media: Vec::new(),
            fragment_start: None,
        });
        Ok(())
    }

    /// Writes the fragment of the held samples, each track's last timed
    /// by `next`.
    fn flush_fragment(&mut self, next: Option<(TrackId, i64)>) {
        let Some(open) = self.segment.as_mut() else {
            return;
        };
        self.scratch.clear();
        self.writer.flush(next, &mut self.scratch);
        open.media.extend_from_slice(&self.scratch);
        open.fragment_start = None;
    }

    /// Ends the segment in progress: its last fragment's samples timed by
    /// `next`, and lasting to `end` (or its samples' time without one).
    /// A segment holding nothing passes its discontinuity on.
    fn end(&mut self, next: Option<(TrackId, i64)>, end: Option<Duration>) {
        self.flush_fragment(next);
        let written = self.writer.take_written();
        let Some(open) = self.segment.take() else {
            return;
        };
        if open.media.is_empty() {
            self.pending_discontinuity |= open.discontinuity;
            return;
        }
        let duration = match end {
            Some(end) => end - open.start,
            None => samples_time(&written),
        };
        if !fits_target(duration, self.config.target) {
            self.metrics.overlong_segments += 1;
        }
        if self.out.len() >= MAX_PENDING_SEGMENTS {
            self.out.pop_front();
            self.metrics.dropped_segments += 1;
        }
        self.out.push_back(HlsSegment {
            init: Rc::clone(&self.init),
            media: open.media,
            duration,
            keyframe: open.keyframe,
            discontinuity: open.discontinuity,
        });
        self.metrics.segments += 1;
        self.finished = true;
    }

    /// Whether the segment in progress can take `growth` more bytes.
    fn room_for(&self, growth: usize) -> bool {
        self.segment.as_ref().is_some_and(|o| {
            o.media
                .len()
                .saturating_add(self.writer.flush_len())
                .saturating_add(growth)
                <= self.config.max_segment_bytes
        })
    }

    /// Adds the unit at `t` to the segment in progress, ending the segment
    /// first when it can't hold the unit.
    fn add(&mut self, u: &AccessUnit<'_>, t: Duration) -> Result<(), HlsMuxerError> {
        if self.segment.is_none() {
            return Ok(());
        }
        if !self.declares(u.track) {
            self.metrics.dropped_pending_config += 1;
            return Ok(());
        }
        let growth = self.writer.max_push_growth(u.track, u.payload.len());
        if !self.room_for(growth) {
            let held = self
                .segment
                .as_ref()
                .is_some_and(|o| !o.media.is_empty() || o.fragment_start.is_some());
            if held {
                let key = u.keyframe && self.cut_track().is_some_and(|c| c.track == u.track);
                let next = self.cut_at(u, t);
                self.end(next, Some(t));
                self.start(u, t, key)?;
            }
            if !self.room_for(growth) {
                self.segment = None;
                self.pending_discontinuity |= self.finished;
                self.metrics.dropped_oversized += 1;
                return Ok(());
            }
        }
        self.scratch.clear();
        self.writer
            .push(u, &mut self.scratch)
            .map_err(|_| HlsMuxerError::UnknownTrack(u.track.0))?;
        if let Some(open) = self.segment.as_mut() {
            // A writer of an unbounded span writes only when flushed.
            open.media.extend_from_slice(&self.scratch);
            open.fragment_start.get_or_insert(t);
        }
        Ok(())
    }
}

impl Muxer for HlsMuxer {
    type Error = HlsMuxerError;

    /// Sets a track's config. Refused, changing nothing, when the configs
    /// can't be declared together or their init, with the longest edit
    /// lists, would be longer than [`MAX_INIT_LEN`].
    fn set_config(&mut self, config: &DecoderConfig) -> Result<(), HlsMuxerError> {
        if self.configs.get(&config.track) == Some(config) {
            return Ok(());
        }
        let mut configs = self.configs.clone();
        configs.insert(config.track, config.clone());
        let list: Vec<DecoderConfig> = ordered(&configs).cloned().collect();
        self.scratch.clear();
        // The longest delay writes 64-bit edit lists, the longest there are.
        write_init(&list, Duration::MAX, &mut self.scratch)?;
        if self.scratch.len() > MAX_INIT_LEN {
            return Err(HlsMuxerError::InitTooLong);
        }
        self.configs = configs;
        self.changed = !self.declares_current();
        Ok(())
    }

    fn push(&mut self, u: &AccessUnit<'_>) -> Result<(), HlsMuxerError> {
        let Some(timescale) = self.configs.get(&u.track).map(|c| c.timescale) else {
            return Err(HlsMuxerError::UnknownTrack(u.track.0));
        };
        let t = media_time(u.dts, timescale);
        let step = self.note(u.track, t);
        match self.step_of(u, t, step) {
            Step::Drop => {
                self.metrics.dropped_awaiting_keyframe += 1;
                return Ok(());
            }
            Step::Start(key) => self.start(u, t, key)?,
            Step::Cut(key) => {
                let next = self.cut_at(u, t);
                self.end(next, Some(t));
                self.start(u, t, key)?;
            }
            Step::Fragment => self.flush_fragment(Some((u.track, u.dts))),
            Step::Add => {}
        }
        self.add(u, t)
    }

    /// Ends the segment in progress; the next starts at the cut track's
    /// next keyframe.
    fn finish(&mut self) -> Result<(), HlsMuxerError> {
        self.end(None, None);
        Ok(())
    }
}

/// `units` of a `timescale` as a time; zero for timescale 0.
fn media_time(units: i64, timescale: u32) -> Duration {
    if timescale == 0 {
        return Duration::ZERO;
    }
    let nanos = i128::from(units) * 1_000_000_000 / i128::from(timescale);
    Duration::from_nanos(i64::try_from(nanos).unwrap_or(if nanos < 0 {
        i64::MIN
    } else {
        i64::MAX
    }))
}

/// `t` in units of `timescale`, rounded down.
fn timescale_units(t: Duration, timescale: u32) -> i64 {
    let units = i128::from(t.as_nanos()) * i128::from(timescale) / 1_000_000_000;
    i64::try_from(units).unwrap_or(if units < 0 { i64::MIN } else { i64::MAX })
}

/// The media time of written samples: the first video track's, or the
/// longest track's without video samples.
fn samples_time(written: &[Written]) -> Duration {
    let time = |w: &Written| media_time(i64::try_from(w.units).unwrap_or(i64::MAX), w.timescale);
    let video = written
        .iter()
        .find(|w| w.kind == Some(TrackKind::Video) && w.samples > 0);
    match video {
        Some(w) => time(w),
        None => written.iter().map(time).max().unwrap_or(Duration::ZERO),
    }
}

/// The configs in init order: video, then audio, each by track id.
fn ordered(
    configs: &BTreeMap<TrackId, DecoderConfig>,
) -> impl Iterator<Item = &DecoderConfig> + Clone {
    let of = move |kind| configs.values().filter(move |c| c.kind == kind);
    of(TrackKind::Video).chain(of(TrackKind::Audio))
}

/// The composition delay an init starting at `keyframe` declares: its PTS
/// less DTS in its track's timescale, zero when not positive.
fn composition_delay(
    keyframe: &AccessUnit<'_>,
    configs: &BTreeMap<TrackId, DecoderConfig>,
) -> Duration {
    let units = keyframe.pts.saturating_sub(keyframe.dts);
    match configs.get(&keyframe.track) {
        Some(c) if units > 0 && c.timescale > 0 => {
            let nanos = i128::from(units) * 1_000_000_000 / i128::from(c.timescale);
            Duration::from_nanos(i64::try_from(nanos).unwrap_or(i64::MAX))
        }
        _ => Duration::ZERO,
    }
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

    /// See [`Playlist::end`].
    pub fn end(&self) {
        self.state.playlist.borrow_mut().end();
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

#[cfg(test)]
#[path = "hls_muxer_test.rs"]
mod muxer_tests;
