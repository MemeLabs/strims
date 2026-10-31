// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS playlists over fMP4 segments and their WebVTT subtitles, the
//! muxer that cuts access units into them, and the routes that serve
//! them.
//!
//! A [`Playlist`] takes finished segments from a source, each an init
//! (`ftyp`+`moov`), its media (`moof`+`mdat`…), its duration, and from a
//! muxer its subtitles and parts. It keeps the newest, serves them by
//! number, and renders a live media playlist (RFC 8216, version 7)
//! listing the newest of those, with a part target their parts too
//! ([Low latency](self#low-latency)), which ends with `EXT-X-ENDLIST`
//! once the stream ended, the subtitles' playlist of the same listing,
//! and the multivariant playlist players open, of one variant
//! ([`render_multivariant`](Playlist::render_multivariant)). Segments
//! that slid out of the listing stay fetchable for a while, for players
//! that read an earlier playlist.
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
//! An audio unit that starts past the end of its track's unit before, by
//! half a silent frame or more, follows a hole. Silent frames
//! ([`silence`](crate::silence)) fill a hole of up to [`MAX_SILENCE`],
//! in the segment of the unit after it: a hole across a cut is filled at
//! the start of the next segment, ahead of its cut track's first sample.
//! A longer hole, or one in a track without silent frames (HE-AAC, AAC
//! channels a program config element defines), is left, the unit before
//! it lasting its own length. No hole is filled across a gap, a reset, the
//! stream's end or a new init.
//!
//! Within a segment, a fragment ends before each keyframe of the cut
//! track, and before the cut track's next unit once the fragment holds
//! [`HlsMuxerConfig::fragment_duration`] of media.
//!
//! # Subtitles
//!
//! A stream's text tracks stay out of its inits. Their cues
//! ([`text`](crate::text)) are the subtitles of the segments they show
//! in: once a stream had a text track, each segment carries a WebVTT
//! file of the cues that show from its start for its duration, each cut
//! to the segment, so a cue across a cut is in both segments, and each
//! part ([Parts](self#parts)) the same of its own time. Cues that show at
//! once are listed as they are, and players stack them. A cue's text has
//! `&`, `<` and `>` escaped and no blank line. A part's file holds the
//! cues known when it ends: a cue that a later one ends sooner keeps its
//! longer end there.
//!
//! A cue's times are the media's: the stream's presentation times less
//! the init's composition delay, as its frames' are after the init's
//! edit. Every file's `X-TIMESTAMP-MAP` maps cue time 0 to media time 0,
//! `MPEGTS:0,LOCAL:00:00:00.000`, so a player that applies the map, one
//! that adds `MPEGTS` alone and one that reads the times as they are all
//! show a cue at its frames. Each cue, the header among them, ends with a
//! blank line, so the files read as one when joined.
//!
//! A segment whose video carries CEA-608 captions
//! ([`captions`](crate::captions)) says so, and the multivariant playlist
//! declares CC1 from then on: players decode the captions from the video.
//!
//! # Parts
//!
//! With a part target ([`HlsMuxerConfig::part_target`]), each segment also
//! comes out as parts ([`HlsPart`]) for low-latency HLS: runs of its
//! fragments that, back to back, are its media, and whose durations add up
//! to its own. A part lasts from its first unit to the next part's first,
//! and ends:
//!
//! - with its segment;
//! - before a unit of the cut track that comes more than the target after
//!   the part's start: the part ends where the unit before it, the cut
//!   track's held one, starts, so it lasts at most the target and at least
//!   the target less a frame. Only a frame longer than the target alone
//!   makes a longer part;
//! - while the cut track sends nothing for two of its frames, or its
//!   units wait for a new init, before a unit of another track that comes
//!   more than the target after the part's start: the part ends at the
//!   cut track's held unit when that came after the part's start, else at
//!   another track's. So audio goes on in parts while the video stalls,
//!   and video that comes behind its audio keeps its timeline.
//!
//! Every sample a part holds is timed by the unit after it: the held unit
//! a part ends at goes into the next part, with each other track's held
//! one. A part whose first sample of the cut track is a keyframe is
//! independent. Keyframes end no part of their own: past
//! [`KEYFRAME_CUT_FLOOR`] they cut the segment.
//!
//! A segment that ends at a gap or the stream's end has no unit after it
//! to time its held samples, which last their own lengths: its last part
//! ends before the cut track's held unit when that unit, lasting the
//! longer of its track's last two steps, would take the part past the
//! target.
//!
//! A frame interval over 15% of the target (under 7 frames a second at a
//! 1 s target) makes parts shorter than 85% of it, the least RFC 8216bis
//! allows a part that neither is independent nor ends its segment.
//!
//! Parts come only as units do: while none arrive, no part finishes, so a
//! playlist goes without the new part RFC 8216bis asks for within each
//! part target.
//!
//! # Low latency
//!
//! With a part target ([`PlaylistConfig::part_target`]), the media
//! playlist is a low-latency one (RFC 8216bis): under `EXT-X-PART-INF`
//! and an `EXT-X-SERVER-CONTROL` whose `PART-HOLD-BACK` is three part
//! targets, it lists each segment's parts (`EXT-X-PART`, `INDEPENDENT=YES`
//! on those that start at a keyframe) as they come, the open segment's
//! after the last whole one, and while live hints at the next part
//! (`EXT-X-PRELOAD-HINT`). Parts are numbered across the stream,
//! `part-<number>.m4s`, so the hint names the next part whichever segment
//! it starts. The segments that end within three targets of the
//! playlist's end list their parts, which stay fetchable while their
//! segment is kept. A part lists its segment's milliseconds up to its
//! end less those before it, so a segment's parts add up to its
//! `EXTINF`.
//!
//! The subtitles' playlist lists the same parts, under the same tags, as
//! WebVTT files (`part-<number>.vtt`): each of the cues that show in its
//! part, cut to it, under the segments' `X-TIMESTAMP-MAP`, and without
//! cues for a part from before the stream had a text track. Players keep
//! to low latency only while every playlist they load has parts. With
//! subtitles, the media and subtitles playlists each carry a rendition
//! report of the other (`EXT-X-RENDITION-REPORT`).
//!
//! Every segment a muxer made is dated (`EXT-X-PROGRAM-DATE-TIME`), in the
//! media and subtitles playlists alike, by its first unit's publish time:
//! a date more than half a second before the end of the segment before,
//! from a writer's clock that stepped back, becomes that end, so each
//! date maps to one segment.
//!
//! # Blocking
//!
//! A low-latency media playlist and its subtitles' playlist offer
//! blocking reloads (`CAN-BLOCK-RELOAD=YES`). A reload of either with an
//! `_HLS_msn` delivery directive, and an `_HLS_part` too, waits
//! ([`HlsRoutes::request`], [`HlsBlock`]) until the playlist lists that
//! segment whole, or that part of it, or something later: a part past a
//! segment's last is the next segment's first. A request of the hinted
//! part, its media or its subtitles, waits for it. A reload of an ended playlist, or of one without
//! parts, answers at once. One that asks for a segment more than two past
//! the next, a part more than the Advance Part Limit past the newest
//! (three part targets' worth, three at least), or a part without a
//! segment answers 400.
//!
//! A blocked request waits at most three targets
//! ([`HlsBlock::timeout`]), after which its caller answers 503, and
//! answers 404 once its stream goes. At most [`MAX_BLOCKED_REQUESTS`] wait
//! on a stream; a request past them answers 503.
//!
//! # Routes
//!
//! [`HlsRoutes`] maps request paths to the playlists of registered
//! profiles and their streams:
//!
//! - `/hls/<profile key>/<transfer id>/multivariant.m3u8`: the
//!   multivariant playlist;
//! - `/hls/<profile key>/<transfer id>/index.m3u8`: the media playlist;
//! - `/hls/<profile key>/<transfer id>/subtitles.m3u8`: the subtitles'
//!   playlist;
//! - `/hls/<profile key>/<transfer id>/init-<id>.mp4`: an init;
//! - `/hls/<profile key>/<transfer id>/<seq>.m4s`: a media segment;
//! - `/hls/<profile key>/<transfer id>/part-<number>.m4s`: a part of a
//!   media segment;
//! - `/hls/<profile key>/<transfer id>/<seq>.vtt`: a segment's subtitles,
//!   a WebVTT file without cues for a segment that has none;
//! - `/hls/<profile key>/<transfer id>/part-<number>.vtt`: a part's
//!   subtitles, the same.
//!
//! Keys are 32 bytes in hex, and the playlist's URIs are relative to it.
//! Registrations are handles, [`HlsProfile`] and [`HlsStream`], whose drop
//! removes them.
//!
//! Every collection is bounded: segments by [`MAX_SEGMENTS`], their parts
//! by [`MAX_SEGMENT_PARTS`] and their bytes by [`PlaylistConfig`], blocked
//! requests by [`MAX_BLOCKED_REQUESTS`], a muxer's segment by
//! [`HlsMuxerConfig::max_segment_bytes`], profiles by [`MAX_PROFILES`],
//! streams by [`MAX_STREAMS`], and request paths by [`MAX_PATH_LEN`].

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Write as _};
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};

use strims_base::time::{Duration, Time};

use crate::captions::NalFormat;
use crate::fmp4::{FragmentWriter, InitError, MAX_TRACKS, Written, write_init};
use crate::media::{AccessUnit, CodecParams, DecoderConfig, Muxer, TrackId, TrackKind};
use crate::opus;
use crate::silence::Silence;
use crate::text::{CueEvent, CueReader};

/// The most segments a [`Playlist`] keeps, listed or not.
pub const MAX_SEGMENTS: usize = 256;

/// The bitrate a multivariant playlist declares before any segment, in
/// bit/s.
pub const DEFAULT_BANDWIDTH: u64 = 1_000_000;

/// A WebVTT file's header, which maps its cues' times to the media's
/// ([module docs](self#subtitles)).
const VTT_HEADER: &str = "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n";
/// A WebVTT file without cues.
const EMPTY_VTT: &str = VTT_HEADER;

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
    /// The most bytes of kept segments, their parts and inits.
    pub max_retained_bytes: usize,
    /// `EXT-X-PART-INF`'s part target, which the segments' parts last at
    /// most ([`HlsMuxerConfig::part_target`]); zero lists no parts. See
    /// [Low latency](self#low-latency).
    pub part_target: Duration,
}

impl Default for PlaylistConfig {
    fn default() -> Self {
        PlaylistConfig {
            target: 4,
            listed: Duration::from_secs(16),
            retained: Duration::from_secs(40),
            max_retained_bytes: 64 << 20,
            part_target: Duration::ZERO,
        }
    }
}

impl PlaylistConfig {
    /// The target, at least 1.
    fn target_at_least_one(&self) -> u64 {
        self.target.max(1)
    }

    /// The part target, within its bounds; `None` lists no parts.
    fn parts(&self) -> Option<Duration> {
        part_target_within(self.part_target, self.target)
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

/// The part target `part_target` comes to under target duration
/// `target`: from [`MIN_PART_TARGET`] to the target, `None` when zero.
fn part_target_within(part_target: Duration, target: u64) -> Option<Duration> {
    if part_target <= Duration::ZERO {
        return None;
    }
    let most = Duration::from_secs(i64::try_from(target.max(1)).unwrap_or(i64::MAX));
    Some(part_target.clamp(MIN_PART_TARGET, most))
}

/// The most parts a [`Playlist`] keeps of a segment; later ones are
/// dropped.
pub const MAX_SEGMENT_PARTS: usize = 128;

/// How far a segment's date may go back over the end of the segment
/// before it, as a writer's clock drifts.
const DATE_OVERLAP: Duration = Duration::from_millis(500);

struct Segment {
    seq: u64,
    init: u64,
    media: Rc<[u8]>,
    /// Its subtitles, as a WebVTT file.
    vtt: Option<Rc<[u8]>>,
    /// The duration in milliseconds, as listed.
    millis: u64,
    /// A discontinuity precedes the segment.
    discontinuity: bool,
    /// `EXT-X-PROGRAM-DATE-TIME`: the date of its start.
    date: Option<Time>,
    /// Its parts, in order. Bound: [`MAX_SEGMENT_PARTS`].
    parts: Vec<Part>,
}

impl Segment {
    /// The bytes its parts hold of their own.
    fn own_part_bytes(&self) -> usize {
        self.parts
            .iter()
            .filter_map(|p| p.own.as_ref())
            .map(|b| b.len())
            .sum()
    }

    /// The bytes of its parts' subtitles.
    fn part_vtt_bytes(&self) -> usize {
        self.parts
            .iter()
            .filter_map(|p| p.vtt.as_ref())
            .map(|v| v.len())
            .sum()
    }
}

/// A part of a segment.
#[derive(Debug, Clone)]
struct Part {
    number: u64,
    /// Its duration in milliseconds, as listed: the parts of its segment
    /// up to it, in milliseconds, less those before it, so a segment's
    /// parts add up to its own.
    millis: u64,
    independent: bool,
    /// Its bytes while its segment is open; once it is kept, its bytes are
    /// the range of the segment's media from `start` for `len`.
    own: Option<Rc<[u8]>>,
    start: usize,
    len: usize,
    /// Its subtitles, as a WebVTT file.
    vtt: Option<Rc<[u8]>>,
}

/// What a multivariant playlist declares of a stream's variant, from its
/// segments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Variant {
    /// The newest segment's RFC 6381 codecs, comma-separated.
    codecs: Option<Rc<str>>,
    /// The newest segment's picture size.
    resolution: Option<(u16, u16)>,
    /// A segment had subtitles: the stream has a text track.
    subtitles: bool,
    /// A segment's video carried CEA-608 captions.
    captions: bool,
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
/// With a part target, a segment comes as its parts first
/// ([`push_part`](Self::push_part)), which make it the open segment:
/// listed by its parts, its number taken, its media to come with the
/// segment itself ([`push`](Self::push)). Part numbers count up from the
/// first segment number, one per pushed part.
///
/// Bounds: at most [`MAX_SEGMENTS`] segments, [`MAX_SEGMENT_PARTS`] parts
/// of each, and at most [`PlaylistConfig::max_retained_bytes`] of them,
/// the open segment's parts and the inits they reference, oldest dropped
/// first. An init is kept while a kept or open segment references it, so
/// there are no more inits than segments.
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
    variant: Variant,
    /// The segment whose parts came, its media to come; its number is
    /// `next_seq`, which its push takes.
    open: Option<Segment>,
    /// The open segment's parts' durations.
    open_duration: Duration,
    next_part: u64,
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
            variant: Variant::default(),
            open: None,
            open_duration: Duration::ZERO,
            next_part: first,
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
        let segment = self.begin(init, false, None);
        self.finish(segment, media, None, duration);
    }

    /// Appends a muxer's segment, as [`push_segment`](Self::push_segment)
    /// does, with its subtitles, its date and the discontinuity it
    /// follows; what it says of its codecs, picture size, subtitles and
    /// captions is what the multivariant playlist declares
    /// ([`render_multivariant`](Self::render_multivariant)). The open
    /// segment, whose parts came, is this one: it keeps its number and
    /// listing, and its parts become ranges of the segment's media, unless
    /// they don't add up to it, when they go.
    pub fn push(&mut self, segment: &HlsSegment) {
        let vtt = segment.vtt.as_deref().map(Rc::from);
        let mut open = match self.open.take() {
            Some(open) => open,
            None => self.begin(
                &segment.init,
                segment.discontinuity,
                Some(segment.published),
            ),
        };
        let own = open.own_part_bytes();
        self.retained_bytes = self.retained_bytes.saturating_sub(own);
        if own == segment.media.len() {
            let mut at = 0;
            for part in &mut open.parts {
                part.own = None;
                part.start = at;
                at = at.saturating_add(part.len);
            }
        } else {
            let vtt = open.part_vtt_bytes();
            self.retained_bytes = self.retained_bytes.saturating_sub(vtt);
            open.parts.clear();
        }
        self.finish(open, &segment.media, vtt, segment.duration);
        let v = &mut self.variant;
        v.codecs = Some(Rc::clone(&segment.codecs));
        v.resolution = segment.resolution;
        v.subtitles |= segment.vtt.is_some();
        v.captions |= segment.captions;
    }

    /// Appends a part of a segment ([`HlsPart`]). A segment's first part
    /// opens it, as a [`push`](Self::push) of it would begin it, and
    /// finishes a segment still open from its parts; a later part goes on
    /// the open segment, unless it isn't the next of it or the segment has
    /// [`MAX_SEGMENT_PARTS`], when it is dropped. It takes the next part
    /// number.
    pub fn push_part(&mut self, part: &HlsPart) {
        if part.index == 0 {
            self.close_open();
            let segment = self.begin(&part.init, part.discontinuity, Some(part.published));
            self.open = Some(segment);
            self.open_duration = Duration::ZERO;
        }
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if open.parts.len() != part.index as usize || open.parts.len() >= MAX_SEGMENT_PARTS {
            return;
        }
        let before = duration_millis(self.open_duration);
        self.open_duration += part.duration.max(Duration::ZERO);
        let millis = duration_millis(self.open_duration).saturating_sub(before);
        open.millis = open.millis.saturating_add(millis);
        open.parts.push(Part {
            number: self.next_part,
            millis,
            independent: part.independent,
            own: Some(Rc::clone(&part.media)),
            start: 0,
            len: part.media.len(),
            vtt: part.vtt.clone(),
        });
        self.next_part = self.next_part.saturating_add(1);
        let vtt = part.vtt.as_ref().map_or(0, |v| v.len());
        self.retained_bytes = self
            .retained_bytes
            .saturating_add(part.media.len())
            .saturating_add(vtt);
        self.trim();
    }

    /// Finishes the open segment from its parts: its media theirs, without
    /// subtitles.
    fn close_open(&mut self) {
        let Some(mut open) = self.open.take() else {
            return;
        };
        let own = open.own_part_bytes();
        self.retained_bytes = self.retained_bytes.saturating_sub(own);
        let mut media = Vec::with_capacity(own);
        for part in &mut open.parts {
            if let Some(bytes) = part.own.take() {
                part.start = media.len();
                media.extend_from_slice(&bytes);
            }
        }
        let duration = self.open_duration;
        self.finish(open, &media, None, duration);
    }

    /// A segment of `init` that follows a discontinuity when
    /// `discontinuity` says so, and dated by `published`: its number is the
    /// next, which its [`finish`](Self::finish) takes.
    fn begin(&mut self, init: &[u8], discontinuity: bool, published: Option<Time>) -> Segment {
        if mem::take(&mut self.ended) {
            self.pending_discontinuity |= self.pushed;
        }
        self.pending_discontinuity |= discontinuity;
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
        let discontinuity = mem::take(&mut self.pending_discontinuity);
        if discontinuity {
            self.discontinuities = self.discontinuities.saturating_add(1);
        }
        self.pushed = true;
        Segment {
            seq: self.next_seq,
            init: init_id,
            media: Rc::from(&[][..]),
            vtt: None,
            millis: 0,
            discontinuity,
            date: published.and_then(|t| self.date_of(t)),
            parts: Vec::new(),
        }
    }

    /// The date of a segment that starts at `published`, after the kept
    /// segments: none before 1970, and the end of the segment before for
    /// one that would start more than [`DATE_OVERLAP`] before it, so each
    /// date maps to one segment.
    fn date_of(&self, published: Time) -> Option<Time> {
        if published.unix_millis() < 0 {
            return None;
        }
        let end = self.segments.back().and_then(|s| {
            let millis = i64::try_from(s.millis).ok()?;
            Some(s.date? + Duration::from_millis(millis))
        });
        match end {
            Some(end) if published + DATE_OVERLAP < end => Some(end),
            _ => Some(published),
        }
    }

    /// Keeps `segment` with its media and subtitles, lasting `duration`
    /// (negative counts as zero): it takes the next segment number.
    fn finish(
        &mut self,
        mut segment: Segment,
        media: &[u8],
        vtt: Option<Rc<[u8]>>,
        duration: Duration,
    ) {
        segment.millis = duration_millis(duration);
        segment.media = Rc::from(media);
        let bytes = media.len() + vtt.as_ref().map_or(0, |v| v.len());
        segment.vtt = vtt;
        self.retained_bytes = self.retained_bytes.saturating_add(bytes);
        self.retained_millis = self.retained_millis.saturating_add(segment.millis);
        self.segments.push_back(segment);
        self.next_seq = self.next_seq.saturating_add(1);
        self.trim();
    }

    /// Marks a discontinuity before the next pushed segment: players reset
    /// their decoder and timeline there. Calls before that push mark one.
    pub fn discontinuity(&mut self) {
        self.pending_discontinuity = true;
    }

    /// The stream ended: the playlist ends with `EXT-X-ENDLIST`, until a
    /// segment or part is pushed. A segment still open is finished from
    /// its parts.
    pub fn end(&mut self) {
        self.close_open();
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
            let bytes = dropped.media.len()
                + dropped.vtt.as_ref().map_or(0, |v| v.len())
                + dropped.part_vtt_bytes();
            self.retained_bytes = self.retained_bytes.saturating_sub(bytes);
            self.retained_millis = self.retained_millis.saturating_sub(dropped.millis);
            let oldest = self.segments.front().or(self.open.as_ref()).map(|s| s.init);
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
        self.render_listing(true)
    }

    /// The subtitles' media playlist: the media playlist's listing, each
    /// segment's WebVTT file in place of its media and no inits.
    pub fn render_subtitles(&self) -> String {
        self.render_listing(false)
    }

    fn render_listing(&self, media: bool) -> String {
        let mut out = String::with_capacity(256 + 96 * self.segments.len());
        // Writing to a String can't fail.
        let _ = write!(
            out,
            "#EXTM3U\n\
             #EXT-X-VERSION:7\n\
             #EXT-X-TARGETDURATION:{}\n",
            self.target_duration(),
        );
        let parts = self.config.parts();
        if let Some(target) = parts {
            let target = duration_millis(target);
            out.push_str("#EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,PART-HOLD-BACK=");
            push_millis(&mut out, target.saturating_mul(3));
            out.push_str("\n#EXT-X-PART-INF:PART-TARGET=");
            push_millis(&mut out, target);
            out.push('\n');
        }
        let _ = write!(
            out,
            "#EXT-X-MEDIA-SEQUENCE:{}\n\
             #EXT-X-DISCONTINUITY-SEQUENCE:{}\n",
            self.media_sequence(),
            self.discontinuity_sequence(),
        );
        let listed: Vec<&Segment> = self.listed().collect();
        // The listed segments that list their parts: those that end within
        // three targets of the playlist's end.
        let window = self.config.target_at_least_one().saturating_mul(3000);
        let mut from_end = self.open.as_ref().map_or(0, |o| o.millis);
        let mut with_parts = 0;
        for s in listed.iter().rev() {
            if from_end >= window {
                break;
            }
            from_end = from_end.saturating_add(s.millis);
            with_parts += 1;
        }
        let first_with_parts = listed.len() - with_parts;
        let ext = if media { "m4s" } else { "vtt" };
        let mut init = None;
        for (i, s) in listed.iter().enumerate() {
            self.render_head(&mut out, s, media, &mut init);
            if i >= first_with_parts {
                render_parts(&mut out, s, ext);
            }
            let _ = write!(
                out,
                "#EXTINF:{}.{:03},\n{}.{}\n",
                s.millis / 1000,
                s.millis % 1000,
                s.seq,
                if media { "m4s" } else { "vtt" }
            );
        }
        if let Some(open) = &self.open {
            self.render_head(&mut out, open, media, &mut init);
            render_parts(&mut out, open, ext);
        }
        if parts.is_some() {
            if !self.ended {
                let _ = writeln!(
                    out,
                    "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-{}.{ext}\"",
                    self.next_part
                );
            }
            self.render_report(&mut out, media);
        }
        if self.ended {
            out.push_str("#EXT-X-ENDLIST\n");
        }
        out
    }

    /// The tags before a segment's parts and its URI: its discontinuity,
    /// in the media playlist its init when it differs from the one before,
    /// and its date.
    fn render_head(&self, out: &mut String, s: &Segment, media: bool, init: &mut Option<u64>) {
        if s.discontinuity {
            out.push_str("#EXT-X-DISCONTINUITY\n");
        }
        if media && *init != Some(s.init) {
            let _ = writeln!(out, "#EXT-X-MAP:URI=\"init-{}.mp4\"", s.init);
            *init = Some(s.init);
        }
        if let Some(date) = s.date {
            out.push_str("#EXT-X-PROGRAM-DATE-TIME:");
            push_date(out, date);
            out.push('\n');
        }
    }

    /// The other media playlist's rendition report, for a stream whose
    /// multivariant playlist names both: its newest part, which the media
    /// and subtitles playlists share.
    fn render_report(&self, out: &mut String, media: bool) {
        if !self.variant.subtitles {
            return;
        }
        let newest = self
            .open
            .as_ref()
            .filter(|o| !o.parts.is_empty())
            .or(self.segments.back());
        let Some(newest) = newest else {
            return;
        };
        let other = if media { "subtitles" } else { "index" };
        let _ = write!(
            out,
            "#EXT-X-RENDITION-REPORT:URI=\"{other}.m3u8\",LAST-MSN={}",
            newest.seq
        );
        if let Some(last) = newest.parts.len().checked_sub(1) {
            let _ = write!(out, ",LAST-PART={last}");
        }
        out.push('\n');
    }

    /// The multivariant playlist of the stream: one variant, the media
    /// playlist, of the newest segment's codecs and picture size and the
    /// peak bitrate of the listed segments ([`DEFAULT_BANDWIDTH`] before
    /// any). Once a segment had subtitles, it names the subtitles'
    /// playlist as a `SUBTITLES` rendition; once a segment's video carried
    /// CEA-608 captions, it declares CC1 as a `CLOSED-CAPTIONS` rendition.
    /// Neither is declared default.
    pub fn render_multivariant(&self) -> String {
        let v = &self.variant;
        let mut out = String::from("#EXTM3U\n#EXT-X-VERSION:7\n");
        if v.subtitles {
            out.push_str(
                "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"Subtitles\",\
                 DEFAULT=NO,AUTOSELECT=YES,FORCED=NO,URI=\"subtitles.m3u8\"\n",
            );
        }
        if v.captions {
            out.push_str(
                "#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"Captions\",\
                 DEFAULT=NO,AUTOSELECT=YES,INSTREAM-ID=\"CC1\"\n",
            );
        }
        let _ = write!(out, "#EXT-X-STREAM-INF:BANDWIDTH={}", self.peak_bandwidth());
        if let Some(codecs) = &v.codecs {
            let _ = write!(out, ",CODECS=\"{codecs}\"");
        }
        if let Some((w, h)) = v.resolution {
            let _ = write!(out, ",RESOLUTION={w}x{h}");
        }
        if v.subtitles {
            out.push_str(",SUBTITLES=\"subs\"");
        }
        if v.captions {
            out.push_str(",CLOSED-CAPTIONS=\"cc\"");
        }
        out.push_str("\nindex.m3u8\n");
        out
    }

    /// The highest bitrate of a listed segment, in bit/s; the default
    /// before any.
    fn peak_bandwidth(&self) -> u64 {
        self.listed()
            .filter(|s| s.millis > 0)
            .map(|s| {
                let bits = u64::try_from(s.media.len())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(8000);
                bits / s.millis
            })
            .max()
            .unwrap_or(DEFAULT_BANDWIDTH)
    }

    /// A kept segment's subtitles: its WebVTT file, or one without cues
    /// for a segment that had none.
    pub fn vtt(&self, seq: u64) -> Option<Rc<[u8]>> {
        let s = self.kept(seq)?;
        Some(
            s.vtt
                .clone()
                .unwrap_or_else(|| Rc::from(EMPTY_VTT.as_bytes())),
        )
    }

    fn kept(&self, seq: u64) -> Option<&Segment> {
        let front = self.segments.front()?;
        let index = usize::try_from(seq.checked_sub(front.seq)?).ok()?;
        self.segments.get(index).filter(|s| s.seq == seq)
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
        self.kept(seq).map(|s| Rc::clone(&s.media))
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

    /// The number the next pushed part takes: the part a live playlist
    /// with parts hints at.
    pub fn next_part(&self) -> u64 {
        self.next_part
    }

    /// What a reload of the playlist asking for segment `msn`, or part
    /// `part` of it, gets: see [Blocking](self#blocking).
    fn reload(&self, msn: u64, part: Option<u64>) -> Reload {
        let Some(target) = self.config.parts() else {
            return Reload::Now;
        };
        if self.ended {
            return Reload::Now;
        }
        if msn > self.next_seq.saturating_add(2) {
            return Reload::Refuse;
        }
        let limit = advance_part_limit(target);
        let finished = self.segments.back().is_some_and(|s| s.seq >= msn);
        let Some(part) = part else {
            return if finished { Reload::Now } else { Reload::Wait };
        };
        if let Some(open) = self.open.as_ref().filter(|o| o.seq == msn) {
            let listed = u64::try_from(open.parts.len()).unwrap_or(u64::MAX);
            return match part.checked_sub(listed) {
                None => Reload::Now,
                Some(ahead) if ahead >= limit => Reload::Refuse,
                Some(_) => Reload::Wait,
            };
        }
        if finished {
            // A part past a segment's last is the next segment's first.
            let parts = self.kept(msn).map_or(0, |s| s.parts.len());
            let past_last = usize::try_from(part).map_or(true, |p| p >= parts);
            let next = msn.saturating_add(1);
            let next_listed = self.segments.back().is_some_and(|s| s.seq >= next)
                || self.open.as_ref().is_some_and(|o| o.seq == next);
            return if !past_last || next_listed || self.kept(msn).is_none() {
                Reload::Now
            } else {
                Reload::Wait
            };
        }
        if part >= limit {
            Reload::Refuse
        } else {
            Reload::Wait
        }
    }

    /// Whether a request of part `number` waits for it: it is the part a
    /// live playlist hints at.
    fn hints(&self, number: u64) -> bool {
        self.config.parts().is_some() && !self.ended && number == self.next_part
    }

    /// How long a blocked request waits: three targets.
    fn block_timeout(&self) -> Duration {
        let target = i64::try_from(self.target_duration()).unwrap_or(i64::MAX);
        Duration::from_secs(target.saturating_mul(3))
    }

    /// A kept part, its segment open or kept, and that segment.
    fn find_part(&self, number: u64) -> Option<(&Segment, &Part)> {
        fn in_segment(s: &Segment, number: u64) -> Option<&Part> {
            let first = s.parts.first()?.number;
            let index = usize::try_from(number.checked_sub(first)?).ok()?;
            s.parts.get(index).filter(|p| p.number == number)
        }
        if let Some(open) = &self.open
            && let Some(part) = in_segment(open, number)
        {
            return Some((open, part));
        }
        let segment = self
            .segments
            .iter()
            .rev()
            .find(|s| s.parts.first().is_some_and(|p| p.number <= number))?;
        Some((segment, in_segment(segment, number)?))
    }

    /// The bytes of a kept part, its segment open or kept.
    pub fn part(&self, number: u64) -> Option<Rc<[u8]>> {
        let (segment, part) = self.find_part(number)?;
        if let Some(own) = &part.own {
            return Some(Rc::clone(own));
        }
        let end = part.start.checked_add(part.len)?;
        segment.media.get(part.start..end).map(Rc::from)
    }

    /// A kept part's subtitles: its WebVTT file, or one without cues for a
    /// part that had none.
    pub fn part_vtt(&self, number: u64) -> Option<Rc<[u8]>> {
        let (_, part) = self.find_part(number)?;
        Some(
            part.vtt
                .clone()
                .unwrap_or_else(|| Rc::from(EMPTY_VTT.as_bytes())),
        )
    }

    /// `EXT-X-DISCONTINUITY-SEQUENCE`: the discontinuities that preceded
    /// segments no longer listed.
    pub fn discontinuity_sequence(&self) -> u64 {
        let listed = self
            .listed()
            .chain(self.open.as_ref())
            .filter(|s| s.discontinuity)
            .count();
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

/// Appends a segment's parts' tags, their files of extension `ext`.
fn render_parts(out: &mut String, s: &Segment, ext: &str) {
    for p in &s.parts {
        out.push_str("#EXT-X-PART:DURATION=");
        push_millis(out, p.millis);
        let _ = write!(out, ",URI=\"part-{}.{ext}\"", p.number);
        if p.independent {
            out.push_str(",INDEPENDENT=YES");
        }
        out.push('\n');
    }
}

/// RFC 8216bis's Advance Part Limit: how many parts past the playlist's
/// last a blocking reload may ask for, three part targets' worth.
fn advance_part_limit(target: Duration) -> u64 {
    let millis = duration_millis(target).max(1);
    3000u64.div_ceil(millis).clamp(1, 3000).max(3)
}

/// What a blocking playlist reload gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reload {
    Now,
    Wait,
    /// 400: it asks too far ahead.
    Refuse,
}

/// Appends `millis` as decimal seconds to the millisecond.
fn push_millis(out: &mut String, millis: u64) {
    let _ = write!(out, "{}.{:03}", millis / 1000, millis % 1000);
}

/// Appends `t`, 1970 or later, as an ISO 8601 UTC date to the
/// millisecond: `YYYY-MM-DDThh:mm:ss.sssZ`.
fn push_date(out: &mut String, t: Time) {
    let ms = t.unix_millis().max(0);
    let (days, of_day) = (ms.div_euclid(86_400_000), ms.rem_euclid(86_400_000));
    let (y, m, d) = civil_from_days(days);
    let _ = write!(
        out,
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        of_day / 3_600_000,
        of_day / 60_000 % 60,
        of_day / 1000 % 60,
        of_day % 1000
    );
}

/// The proleptic Gregorian year, month and day `days` after 1970-01-01
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
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

/// The longest hole in an audio track an [`HlsMuxer`] fills with silence:
/// it bounds the frames a unit writes, about 150 of Opus or 141 of AAC at
/// 48 kHz. A hole longer than it is audio that stopped, not a lost or late
/// frame.
pub const MAX_SILENCE: Duration = Duration::from_secs(3);

/// The media a segment's fragment holds by default
/// ([`HlsMuxerConfig::fragment_duration`]): a few video frames and their
/// audio, so a fragment's `prft`, `moof` and `mdat` headers stay a small
/// share of its bytes.
pub const DEFAULT_FRAGMENT_DURATION: Duration = Duration::from_millis(100);

/// The shortest part target an [`HlsMuxer`] cuts parts to: a shorter
/// [`HlsMuxerConfig::part_target`] counts as it.
pub const MIN_PART_TARGET: Duration = Duration::from_millis(100);

/// The most finished parts an [`HlsMuxer`] holds for
/// [`HlsMuxer::take_part`]; older ones are dropped.
pub const MAX_PENDING_PARTS: usize = 16;

/// How an [`HlsMuxer`] cuts segments, fragments and parts.
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
    /// The most media a part lasts (see [Parts](self#parts)), from
    /// [`MIN_PART_TARGET`] to the target; zero cuts no parts.
    pub part_target: Duration,
}

impl Default for HlsMuxerConfig {
    fn default() -> Self {
        HlsMuxerConfig {
            target: 4,
            fragment_duration: DEFAULT_FRAGMENT_DURATION,
            max_segment_bytes: 16 << 20,
            part_target: Duration::ZERO,
        }
    }
}

impl HlsMuxerConfig {
    /// The part target parts are cut to, within its bounds; `None` cuts
    /// none.
    fn parts(&self) -> Option<Duration> {
        part_target_within(self.part_target, self.target)
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
    /// Its subtitles, the text tracks' cues that show during it, as a
    /// WebVTT file; `None` for a stream that never had a text track.
    pub vtt: Option<Vec<u8>>,
    /// Its init's RFC 6381 codecs, comma-separated, video first.
    pub codecs: Rc<str>,
    /// Its video's picture size.
    pub resolution: Option<(u16, u16)>,
    /// Its video carried CEA-608 captions.
    pub captions: bool,
    /// When the source received its first unit (that unit's
    /// [`published`](AccessUnit::published) time): the date of its start.
    pub published: Time,
}

/// A part of a segment an [`HlsMuxer`] finished: the next run of the
/// segment's fragments. See [Parts](self#parts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsPart {
    /// Its fragments: the next bytes of its segment's media. A segment's
    /// parts, back to back, are its media.
    pub media: Rc<[u8]>,
    /// From its first unit to the next part's first; a segment's last
    /// part lasts to the segment's end, so a segment's parts add up to
    /// its duration.
    pub duration: Duration,
    /// Its first sample of the cut track is a keyframe.
    pub independent: bool,
    /// Its place in its segment, from 0.
    pub index: u32,
    /// Its segment's init, discontinuity and publish time, as the
    /// segment's [`HlsSegment`] has them.
    pub init: Rc<[u8]>,
    pub discontinuity: bool,
    pub published: Time,
    /// Its subtitles, the text tracks' cues that show during it, cut to
    /// it, as a WebVTT file; `None` for a stream that never had a text
    /// track.
    pub vtt: Option<Rc<[u8]>>,
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
    /// Holes in audio tracks filled with silence, and the silent frames
    /// that filled them.
    pub silence_fills: u64,
    pub silence_units: u64,
    /// Holes in audio tracks left: longer than [`MAX_SILENCE`], of a
    /// track without silent frames, or without room in the segment.
    pub audio_holes: u64,
    pub parts: u64,
    /// Parts that last past the part target: a frame alone lasted past
    /// it, or a segment's held samples, timed by their own lengths at a
    /// gap or the end, ran past it.
    pub overlong_parts: u64,
    /// Finished parts dropped untaken.
    pub dropped_parts: u64,
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

/// An audio track the init declares: its silent frames, and where its
/// units end.
#[derive(Debug)]
struct AudioTrack {
    track: TrackId,
    timescale: u32,
    /// `None` for a config without them.
    silence: Option<Silence>,
    /// An AAC frame's samples; 0 for Opus, whose packets carry theirs.
    frame_samples: u32,
    /// The decode time the track's last unit ends at; `None` before its
    /// first unit after the init, a gap, a reset or the stream's end.
    end: Option<i64>,
    /// The samples of its last unit, for a packet that doesn't carry its
    /// own.
    last: i64,
}

impl AudioTrack {
    fn new(config: &DecoderConfig) -> AudioTrack {
        let frame_samples = match config.params {
            CodecParams::Aac { frame_samples, .. } => frame_samples,
            _ => 0,
        };
        AudioTrack {
            track: config.track,
            timescale: config.timescale,
            silence: Silence::for_config(config),
            frame_samples,
            end: None,
            last: 0,
        }
    }

    /// The samples a unit of `payload` lasts: an AAC frame's, an Opus
    /// packet's, or the unit's before.
    fn samples(&self, payload: &[u8]) -> i64 {
        if self.frame_samples > 0 {
            return i64::from(self.frame_samples);
        }
        opus::packet_samples(payload).map_or(self.last, i64::from)
    }

    /// What comes before a unit at `dts`: nothing, silence from the
    /// track's end, or a hole left.
    fn hole_before(&self, dts: i64) -> Hole {
        let Some(end) = self.end else {
            return Hole::None;
        };
        let gap = dts.saturating_sub(end);
        let step = self
            .silence
            .as_ref()
            .map_or(self.last, |s| i64::from(s.step()))
            .max(1);
        if gap.saturating_add(gap) < step {
            return Hole::None;
        }
        if self.silence.is_none() || gap > timescale_units(MAX_SILENCE, self.timescale) {
            return Hole::Left { end };
        }
        Hole::Fill { from: end }
    }
}

/// The hole before an audio unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hole {
    None,
    /// Silent frames fill it, from the track's end at `from`.
    Fill {
        from: i64,
    },
    /// It is left, the unit before it lasting until the track's end at
    /// `end`.
    Left {
        end: i64,
    },
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
    /// Its video carried captions.
    captions: bool,
    /// When the source received its first unit.
    published: Time,
    /// The part in progress, with a part target.
    part: OpenPart,
}

/// The part in progress of a segment.
#[derive(Debug, Clone, Copy)]
struct OpenPart {
    index: u32,
    /// The time of its first unit.
    start: Duration,
    /// Where its bytes start in the segment's media.
    from: usize,
    /// The durations of the segment's parts before it.
    before: Duration,
    /// Whether its first unit of the cut track is a keyframe, once it
    /// holds one.
    first_cut: Option<bool>,
    /// Its units of the cut track: the writer's held one it started
    /// with, and those added since.
    cut_units: u32,
}

impl OpenPart {
    /// The part that starts at `start`, at byte `from` of its segment's
    /// media, after parts lasting `before`; it starts with the writer's
    /// held unit of the cut track, a keyframe or not, if any.
    fn new(
        index: u32,
        start: Duration,
        from: usize,
        before: Duration,
        held_cut: Option<bool>,
    ) -> Self {
        OpenPart {
            index,
            start,
            from,
            before,
            first_cut: held_cut,
            cut_units: u32::from(held_cut.is_some()),
        }
    }

    /// Whether it starts at a keyframe of the cut track, `held` of its
    /// units of the cut track staying held past its end.
    fn independent(&self, held: u32) -> bool {
        self.cut_units > held && self.first_cut == Some(true)
    }
}

/// Where a part ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartEnd {
    /// At the held unit at this time: the timed samples are written, and
    /// each track's last stays held.
    Held(Duration),
    /// Before the cut track's unit at `at`, of decode time `next`: every
    /// held sample is written.
    Before { next: (TrackId, i64), at: Duration },
}

/// A cue a segment's subtitles may hold, on the stream's timeline.
#[derive(Debug, Clone)]
struct VttCue {
    id: u64,
    start: Duration,
    end: Duration,
    text: String,
}

/// The most cues an [`HlsMuxer`] holds for the segments to come; past it
/// the oldest goes.
const MAX_VTT_CUES: usize = 64;

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
/// [`take_segment`](Self::take_segment) hands out, and with a part target
/// their parts, which [`take_part`](Self::take_part) hands out. A text
/// track's config and units go into the segments' subtitles
/// ([module docs](self#subtitles)).
///
/// # Bounds
///
/// A segment's fragments and held samples stay within
/// [`HlsMuxerConfig::max_segment_bytes`]: a unit that would take them past
/// it ends the segment, and one that can't fit even alone is dropped with
/// the units after it up to the next keyframe. It holds at most
/// [`MAX_TRACKS`] media and text configs, [`MAX_PENDING_SEGMENTS`]
/// finished segments, [`MAX_PENDING_PARTS`] finished parts, a cue reader
/// of its text tracks and 64 cues for the segments to come, and reuses
/// its buffers but for each segment's and part's media and each
/// segment's subtitles.
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
    /// The declared audio tracks; at most [`MAX_TRACKS`].
    audio: Vec<AudioTrack>,
    /// A discontinuity precedes the next segment.
    pending_discontinuity: bool,
    /// A segment was finished, so a gap after it is a discontinuity.
    finished: bool,
    /// Where the segment the last cut ended ends, so the next starts no
    /// earlier.
    cut_end: Option<Duration>,
    /// The time of the cut track's unit the writer holds, and whether it
    /// is a keyframe; `None` while it holds none.
    held_cut: Option<(Duration, bool)>,
    out: VecDeque<HlsSegment>,
    /// The text tracks' configs, which the init leaves out. Bound:
    /// [`MAX_TRACKS`].
    text: BTreeMap<TrackId, DecoderConfig>,
    /// The stream had a text track: its segments carry subtitles.
    subtitles: bool,
    cues: CueReader,
    /// Bound: `MAX_VTT_CUES`.
    vtt_cues: VecDeque<VttCue>,
    /// The init's codecs, picture size and composition delay, and its cut
    /// track's NAL format, whose units may carry captions.
    codecs: Rc<str>,
    resolution: Option<(u16, u16)>,
    delay: Duration,
    nals: Option<NalFormat>,
    parts: VecDeque<HlsPart>,
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
            audio: Vec::new(),
            pending_discontinuity: false,
            finished: false,
            cut_end: None,
            held_cut: None,
            out: VecDeque::new(),
            text: BTreeMap::new(),
            subtitles: false,
            cues: CueReader::new(),
            vtt_cues: VecDeque::new(),
            codecs: Rc::from(""),
            resolution: None,
            delay: Duration::ZERO,
            nals: None,
            parts: VecDeque::new(),
            metrics: HlsMuxerMetrics::default(),
        }
    }

    /// Sets the text tracks to `configs`, those of another kind left out:
    /// a track not among them goes.
    pub fn set_text_configs(&mut self, configs: &[DecoderConfig]) {
        self.text = configs
            .iter()
            .filter(|c| c.kind == TrackKind::Text)
            .take(MAX_TRACKS)
            .map(|c| (c.track, c.clone()))
            .collect();
        self.subtitles |= !self.text.is_empty();
        let text: Vec<DecoderConfig> = self.text.values().cloned().collect();
        self.cues.set_configs(&text);
    }

    /// Takes the cue reader's events into the cues held.
    fn take_cues(&mut self) {
        while let Some(e) = self.cues.poll_event() {
            match e {
                CueEvent::Show {
                    id,
                    start,
                    end,
                    text,
                    ..
                } => {
                    if self.vtt_cues.len() >= MAX_VTT_CUES {
                        self.vtt_cues.pop_front();
                    }
                    self.vtt_cues.push_back(VttCue {
                        id,
                        start,
                        end,
                        text,
                    });
                }
                CueEvent::End { id, end } => {
                    if let Some(c) = self.vtt_cues.iter_mut().find(|c| c.id == id) {
                        c.end = end;
                    }
                }
            }
        }
    }

    /// The WebVTT file of the cues that show in a segment presented from
    /// `start` for `duration`, the init's composition delay before the
    /// stream's presentation times; see the
    /// [module docs](self#subtitles).
    fn vtt(&self, start: Duration, duration: Duration) -> Vec<u8> {
        let from = start + self.delay;
        let to = from + duration;
        let mut out = String::from(VTT_HEADER);
        for c in &self.vtt_cues {
            let (s, e) = (c.start.max(from), c.end.min(to));
            if e <= s {
                continue;
            }
            push_vtt_time(&mut out, s - self.delay);
            out.push_str(" --> ");
            push_vtt_time(&mut out, e - self.delay);
            out.push('\n');
            push_vtt_text(&mut out, &c.text);
            out.push_str("\n\n");
        }
        out.into_bytes()
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

    /// The oldest finished part not yet taken. The call that finishes a
    /// segment finishes its last part too, so a caller that takes every
    /// part, then every segment, after each call takes a segment's parts
    /// before it.
    pub fn take_part(&mut self) -> Option<HlsPart> {
        self.parts.pop_front()
    }

    /// Units went missing here: ends the segment in progress, and drops
    /// units up to the cut track's next keyframe, whose segment follows a
    /// discontinuity.
    pub fn gap(&mut self) {
        self.end(None, None);
        self.forget_audio_ends();
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
        self.audio.clear();
        self.text.clear();
        self.cues = CueReader::new();
        self.vtt_cues.clear();
    }

    /// No hole is filled from where the audio tracks' units ended so far.
    fn forget_audio_ends(&mut self) {
        for a in &mut self.audio {
            a.end = None;
        }
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
            self.held_cut = None;
            self.init = Rc::from(&self.scratch[..]);
            let codecs: Vec<&str> = configs.iter().map(|c| c.codec.as_str()).collect();
            self.codecs = Rc::from(codecs.join(","));
            let cut = configs.iter().find(|c| c.kind == TrackKind::Video);
            self.resolution = cut.and_then(|c| c.params.picture_size());
            self.nals = cut.and_then(|c| NalFormat::of(&c.params));
            self.delay = delay;
            self.audio = configs
                .iter()
                .filter(|c| c.kind == TrackKind::Audio)
                .map(AudioTrack::new)
                .collect();
            self.declared = configs;
            self.changed = false;
        }
        if !keyframe {
            self.metrics.mid_gop_segments += 1;
        }
        // The units of a segment's tracks interleave out of time order, so
        // its parts can end past the unit that cuts it: the next segment
        // starts where they end.
        let start = self.cut_end.take().filter(|end| *end > t).unwrap_or(t);
        self.segment = Some(Open {
            start,
            keyframe,
            discontinuity: mem::take(&mut self.pending_discontinuity),
            media: Vec::new(),
            fragment_start: None,
            captions: false,
            published: u.published,
            part: OpenPart::new(0, start, 0, Duration::ZERO, self.held_keyframe()),
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
        self.held_cut = None;
        open.media.extend_from_slice(&self.scratch);
        open.fragment_start = None;
    }

    /// Writes the fragment of the timed held samples, each track's last
    /// staying held, the first of those at `held`.
    fn write_timed(&mut self, held: Duration) {
        let Some(open) = self.segment.as_mut() else {
            return;
        };
        self.scratch.clear();
        self.writer.write_timed(&mut self.scratch);
        open.media.extend_from_slice(&self.scratch);
        open.fragment_start = Some(held);
    }

    /// Whether the cut track's unit the writer holds is a keyframe; `None`
    /// while it holds none.
    fn held_keyframe(&self) -> Option<bool> {
        self.held_cut.map(|(_, key)| key)
    }

    /// Where the part in progress ends before the unit `u` at `t`, whose
    /// track's unit before it came at `prev`; `None` while it goes on. See
    /// [Parts](self#parts).
    fn part_due(&self, u: &AccessUnit<'_>, t: Duration, prev: Option<Duration>) -> Option<PartEnd> {
        let target = self.config.parts()?;
        let open = self.segment.as_ref()?;
        let start = open.part.start;
        if !self.declares(u.track) || t - start <= target {
            return None;
        }
        let cut = self.cut_track()?;
        let own = cut.track == u.track;
        // The cut track is late while it sends nothing, or while its units
        // wait for a new init.
        if !own && !self.cut_track_late(t) && self.declares(cut.track) {
            return None;
        }
        // The held units the part can end at: the cut track's, then the
        // unit's own track's, then the latest of the others'.
        let others = self
            .timing
            .iter()
            .filter(|x| x.track != cut.track && x.track != u.track)
            .map(|x| x.last)
            .max();
        let held = [
            self.held_cut.map(|(at, _)| at),
            prev.filter(|_| !own),
            others,
        ];
        if let Some(at) = held.into_iter().flatten().find(|at| *at > start) {
            return Some(PartEnd::Held(at));
        }
        let holds = open.media.len() > open.part.from || open.fragment_start.is_some();
        (own && holds).then_some(PartEnd::Before {
            next: (u.track, u.dts),
            at: t,
        })
    }

    /// Ends the part in progress at `end`, and starts the next there.
    fn end_part(&mut self, end: PartEnd) {
        let (at, held) = match end {
            PartEnd::Held(at) => {
                self.write_timed(at);
                (at, u32::from(self.held_cut.is_some()))
            }
            PartEnd::Before { next, at } => {
                self.flush_fragment(Some(next));
                (at, 0)
            }
        };
        let held_keyframe = self.held_keyframe();
        let Some(open) = self.segment.as_mut() else {
            return;
        };
        let p = open.part;
        let Some(media) = open.media.get(p.from..).filter(|m| !m.is_empty()) else {
            return;
        };
        let duration = (at - p.start).max(Duration::ZERO);
        let mut part = HlsPart {
            media: Rc::from(media),
            duration,
            independent: p.independent(held),
            index: p.index,
            init: Rc::clone(&self.init),
            discontinuity: open.discontinuity,
            published: open.published,
            vtt: None,
        };
        open.part = OpenPart::new(
            p.index.saturating_add(1),
            at,
            open.media.len(),
            p.before + duration,
            held_keyframe,
        );
        part.vtt = self
            .subtitles
            .then(|| Rc::from(self.vtt(p.start, duration)));
        self.emit_part(part);
    }

    /// Ends the part in progress at the cut track's held unit when that
    /// unit, lasting the longer of its track's last two steps, would take
    /// the part past the target: the samples of a segment that ends with
    /// no unit after it are timed by their own lengths.
    fn end_part_before_held(&mut self) {
        let (Some(target), Some(open), Some((at, _))) =
            (self.config.parts(), self.segment.as_ref(), self.held_cut)
        else {
            return;
        };
        let step = self
            .cut_track()
            .and_then(|c| self.timing(c.track))
            .map_or(Duration::ZERO, Timing::longest_step);
        if at > open.part.start && at + step - open.part.start > target {
            self.end_part(PartEnd::Held(at));
        }
    }

    /// Hands out the last part of the finished segment `open`, which
    /// lasts `duration`: returns the segment's duration, its parts' sum.
    fn end_last_part(&mut self, open: &Open, duration: Duration) -> Duration {
        let p = open.part;
        let Some(media) = open.media.get(p.from..).filter(|m| !m.is_empty()) else {
            return p.before;
        };
        let last = (duration - p.before).max(Duration::ZERO);
        let vtt = self.subtitles.then(|| Rc::from(self.vtt(p.start, last)));
        self.emit_part(HlsPart {
            media: Rc::from(media),
            duration: last,
            independent: p.independent(0),
            index: p.index,
            init: Rc::clone(&self.init),
            discontinuity: open.discontinuity,
            published: open.published,
            vtt,
        });
        p.before + last
    }

    fn emit_part(&mut self, part: HlsPart) {
        if self.config.parts().is_some_and(|t| part.duration > t) {
            self.metrics.overlong_parts += 1;
        }
        if self.parts.len() >= MAX_PENDING_PARTS {
            self.parts.pop_front();
            self.metrics.dropped_parts += 1;
        }
        self.parts.push_back(part);
        self.metrics.parts += 1;
    }

    /// Ends the segment in progress: its last fragment's samples timed by
    /// `next`, and lasting to `end` (or its samples' time without one).
    /// A segment holding nothing passes its discontinuity on.
    fn end(&mut self, next: Option<(TrackId, i64)>, end: Option<Duration>) {
        if end.is_none() {
            self.end_part_before_held();
        }
        self.flush_fragment(next);
        let written = self.writer.take_written();
        self.cut_end = None;
        let Some(open) = self.segment.take() else {
            return;
        };
        if open.media.is_empty() {
            self.pending_discontinuity |= open.discontinuity;
            return;
        }
        let mut duration = match end {
            Some(end) => end - open.start,
            None => samples_time(&written),
        };
        if self.config.parts().is_some() {
            duration = self.end_last_part(&open, duration);
        }
        if end.is_some() {
            self.cut_end = Some(open.start + duration);
        }
        if !fits_target(duration, self.config.target) {
            self.metrics.overlong_segments += 1;
        }
        if self.out.len() >= MAX_PENDING_SEGMENTS {
            self.out.pop_front();
            self.metrics.dropped_segments += 1;
        }
        let vtt = self.subtitles.then(|| self.vtt(open.start, duration));
        let shown_to = open.start + self.delay + duration;
        self.vtt_cues.retain(|c| c.end > shown_to);
        self.out.push_back(HlsSegment {
            init: Rc::clone(&self.init),
            media: open.media,
            duration,
            keyframe: open.keyframe,
            discontinuity: open.discontinuity,
            vtt,
            codecs: Rc::clone(&self.codecs),
            resolution: self.resolution,
            captions: open.captions,
            published: open.published,
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
        let audio = self.audio.iter().position(|a| a.track == u.track);
        let hole = audio
            .and_then(|i| self.audio.get(i))
            .map_or(Hole::None, |a| a.hole_before(u.dts));
        let growth = self.writer.max_push_growth(u.track, u.payload.len());
        let silence = match hole {
            Hole::Fill { from } => self.silence_growth(u, from),
            _ => 0,
        };
        if !self.room_for(growth.saturating_add(silence)) {
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
        match hole {
            Hole::None => {}
            Hole::Fill { from } if self.room_for(growth.saturating_add(silence)) => {
                self.write_silence(u, from);
            }
            Hole::Fill { from: end } | Hole::Left { end } => {
                self.metrics.audio_holes += 1;
                // The track is declared, so ending its units can't fail.
                let _ = self.writer.end_track(u.track, end, &mut self.scratch);
            }
        }
        self.writer
            .push(u, &mut self.scratch)
            .map_err(|_| HlsMuxerError::UnknownTrack(u.track.0))?;
        if let Some(a) = audio.and_then(|i| self.audio.get_mut(i)) {
            let samples = a.samples(u.payload);
            a.last = samples;
            a.end = Some(u.dts.saturating_add(samples));
        }
        let captions = self.nals.is_some_and(|n| {
            self.cut_track().is_some_and(|c| c.track == u.track) && n.has_captions(u.payload)
        });
        let cut = self.cut_track().is_some_and(|c| c.track == u.track);
        if cut {
            self.held_cut = Some((t, u.keyframe));
        }
        if let Some(open) = self.segment.as_mut() {
            // A writer of an unbounded span writes only when flushed or
            // when a track's units end at a hole.
            open.media.extend_from_slice(&self.scratch);
            open.fragment_start.get_or_insert(t);
            open.captions |= captions;
            if cut {
                open.part.first_cut.get_or_insert(u.keyframe);
                open.part.cut_units = open.part.cut_units.saturating_add(1);
            }
        }
        Ok(())
    }

    /// The most bytes the silence filling an audio track from `from` to
    /// the unit `u` adds to the segment.
    fn silence_growth(&self, u: &AccessUnit<'_>, from: i64) -> usize {
        let Some(silence) = self
            .audio
            .iter()
            .find(|a| a.track == u.track)
            .and_then(|a| a.silence.as_ref())
        else {
            return 0;
        };
        silence
            .cover(u.dts.saturating_sub(from))
            .map(|(payload, _)| self.writer.max_push_growth(u.track, payload.len()))
            .fold(0usize, usize::saturating_add)
    }

    /// Holds the silent frames that fill an audio track from `from` up to
    /// the unit `u`, each a keyframe published as long before `u` as it
    /// decodes before it.
    fn write_silence(&mut self, u: &AccessUnit<'_>, from: i64) {
        let HlsMuxer {
            audio,
            writer,
            metrics,
            ..
        } = self;
        let Some(a) = audio.iter().find(|a| a.track == u.track) else {
            return;
        };
        let Some(silence) = a.silence.as_ref() else {
            return;
        };
        let mut dts = from;
        let mut count = 0u64;
        for (payload, samples) in silence.cover(u.dts.saturating_sub(from)) {
            let unit = AccessUnit {
                track: u.track,
                dts,
                pts: dts,
                keyframe: true,
                payload,
                published: u.published - media_time(u.dts.saturating_sub(dts), a.timescale),
            };
            if writer.hold(&unit).is_err() {
                break;
            }
            dts = dts.saturating_add(i64::from(samples));
            count += 1;
        }
        if count > 0 {
            metrics.silence_fills += 1;
            metrics.silence_units += count;
        }
    }
}

impl Muxer for HlsMuxer {
    type Error = HlsMuxerError;

    /// Sets a track's config. Refused, changing nothing, when the configs
    /// can't be declared together or their init, with the longest edit
    /// lists, would be longer than [`MAX_INIT_LEN`].
    fn set_config(&mut self, config: &DecoderConfig) -> Result<(), HlsMuxerError> {
        if config.kind == TrackKind::Text {
            if self.text.len() >= MAX_TRACKS && !self.text.contains_key(&config.track) {
                return Err(InitError::TooManyTracks.into());
            }
            let mut text: Vec<DecoderConfig> = self
                .text
                .values()
                .filter(|c| c.track != config.track)
                .cloned()
                .collect();
            text.push(config.clone());
            self.set_text_configs(&text);
            return Ok(());
        }
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

    /// Takes a unit: a text track's cue goes into the subtitles of the
    /// segments it shows in.
    fn push(&mut self, u: &AccessUnit<'_>) -> Result<(), HlsMuxerError> {
        if self.text.contains_key(&u.track) {
            self.cues.push(u);
            self.take_cues();
            return Ok(());
        }
        let Some(timescale) = self.configs.get(&u.track).map(|c| c.timescale) else {
            return Err(HlsMuxerError::UnknownTrack(u.track.0));
        };
        let t = media_time(u.dts, timescale);
        let prev = self.timing(u.track).map(|x| x.last);
        let step = self.note(u.track, t);
        if let Some(end) = self.part_due(u, t, prev) {
            self.end_part(end);
        }
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
        self.forget_audio_ends();
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

/// Appends `t` as a WebVTT timestamp, `hh:mm:ss.ttt`; zero when negative.
fn push_vtt_time(out: &mut String, t: Duration) {
    let ms = t.as_millis().max(0);
    let _ = write!(
        out,
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    );
}

/// Appends a cue's text as WebVTT cue text: `&`, `<` and `>` escaped, and
/// its lines without blank ones, which would end the cue.
fn push_vtt_text(out: &mut String, text: &str) {
    let mut first = true;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if !first {
            out.push('\n');
        }
        first = false;
        for c in line.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                c => out.push(c),
            }
        }
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

/// The most requests blocked on a stream at once ([`HlsBlock`]); past it,
/// a request that would block answers 503.
pub const MAX_BLOCKED_REQUESTS: usize = 64;

const PLAYLIST_TYPE: &str = "application/vnd.apple.mpegurl";
const INIT_TYPE: &str = "video/mp4";
const SEGMENT_TYPE: &str = "video/iso.segment";
const VTT_TYPE: &str = "text/vtt";
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
    /// 200; 400 for a malformed path or delivery directive; 404 for a
    /// missing resource; 503 for a request that can't block, or blocked
    /// past its timeout.
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

    /// The answer to a blocked request whose wait ran out
    /// ([`HlsBlock::timeout`]).
    pub fn unavailable() -> Self {
        HlsResponse::error(503)
    }

    fn error(status: u16) -> Self {
        let body: &[u8] = match status {
            400 => b"bad request\n",
            503 => b"service unavailable\n",
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

/// The path of a stream's media playlist, from the root of the HLS
/// server.
pub fn playlist_path(profile_key: &[u8], transfer_id: &[u8; KEY_LEN]) -> String {
    stream_path(profile_key, transfer_id, "index.m3u8")
}

/// The path of a stream's multivariant playlist, from the root of the HLS
/// server: what players open.
pub fn multivariant_path(profile_key: &[u8], transfer_id: &[u8; KEY_LEN]) -> String {
    stream_path(profile_key, transfer_id, "multivariant.m3u8")
}

fn stream_path(profile_key: &[u8], transfer_id: &[u8; KEY_LEN], file: &str) -> String {
    let mut path = String::with_capacity(24 + 2 * (profile_key.len() + KEY_LEN));
    path.push_str("/hls/");
    push_hex(&mut path, profile_key);
    path.push('/');
    push_hex(&mut path, transfer_id);
    path.push('/');
    path.push_str(file);
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
    /// The requests blocked on the playlist, by id, and their wakers.
    /// Bound: [`MAX_BLOCKED_REQUESTS`].
    waiters: RefCell<Vec<(u64, Option<Waker>)>>,
    next_waiter: Cell<u64>,
}

impl StreamState {
    fn new(playlist: Playlist, now: Time) -> Self {
        StreamState {
            playlist: RefCell::new(playlist),
            last_access: Cell::new(now),
            waiters: RefCell::new(Vec::new()),
            next_waiter: Cell::new(0),
        }
    }

    fn touch(&self, now: Time) {
        self.last_access.set(self.last_access.get().max(now));
    }

    /// Wakes the blocked requests: the playlist changed, or the stream
    /// went.
    fn changed(&self) {
        let wakers: Vec<Waker> = self
            .waiters
            .borrow_mut()
            .iter_mut()
            .filter_map(|(_, w)| w.take())
            .collect();
        for w in wakers {
            w.wake();
        }
    }

    /// The answer to a request of `route` now, or that it waits.
    fn answer(&self, route: &Route) -> Result<HlsResponse, Waiting> {
        let playlist = self.playlist.borrow();
        let text = |t: String| Rc::from(t.into_bytes());
        let response = match route.file {
            File::Multivariant => Some(HlsResponse::ok(
                PLAYLIST_TYPE,
                NO_CACHE,
                text(playlist.render_multivariant()),
            )),
            File::Playlist | File::Subtitles => {
                let media = route.file == File::Playlist;
                let reload = match route.directives {
                    Some((msn, part)) => playlist.reload(msn, part),
                    None => Reload::Now,
                };
                match reload {
                    Reload::Refuse => return Ok(HlsResponse::error(400)),
                    Reload::Wait => return Err(Waiting),
                    Reload::Now => {}
                }
                let body = if media {
                    playlist.render()
                } else {
                    playlist.render_subtitles()
                };
                Some(HlsResponse::ok(PLAYLIST_TYPE, NO_CACHE, text(body)))
            }
            File::Vtt(seq) => playlist
                .vtt(seq)
                .map(|body| HlsResponse::ok(VTT_TYPE, IMMUTABLE, body)),
            File::Init(id) => playlist
                .init(id)
                .map(|body| HlsResponse::ok(INIT_TYPE, IMMUTABLE, body)),
            File::Segment(seq) => playlist
                .segment(seq)
                .map(|body| HlsResponse::ok(SEGMENT_TYPE, IMMUTABLE, body)),
            File::Part(number) => match playlist.part(number) {
                Some(body) => Some(HlsResponse::ok(SEGMENT_TYPE, IMMUTABLE, body)),
                None if playlist.hints(number) => return Err(Waiting),
                None => None,
            },
            File::PartVtt(number) => match playlist.part_vtt(number) {
                Some(body) => Some(HlsResponse::ok(VTT_TYPE, IMMUTABLE, body)),
                None if playlist.hints(number) => return Err(Waiting),
                None => None,
            },
        };
        Ok(response.unwrap_or_else(|| HlsResponse::error(404)))
    }
}

/// A request waits for its stream's playlist to change.
#[derive(Debug, Clone, Copy)]
struct Waiting;

/// What a request answers: now, or once its stream holds what it asks
/// for ([Blocking](self#blocking)).
#[derive(Debug)]
pub enum HlsAnswer {
    Ready(HlsResponse),
    Blocked(HlsBlock),
}

/// A request blocked on its stream: a future of its answer, once the
/// stream holds what it asks for, or 404 once the stream goes. It waits
/// as long as its caller polls it, which [`timeout`](Self::timeout) bounds:
/// past it, the caller drops it and answers
/// [`HlsResponse::unavailable`]. Dropping it frees its place among the
/// stream's [`MAX_BLOCKED_REQUESTS`].
#[must_use = "a blocked request answers when polled"]
pub struct HlsBlock {
    stream: Weak<StreamState>,
    route: Route,
    waiter: u64,
    timeout: Duration,
}

impl HlsBlock {
    /// How long the request may wait: three target durations, after which
    /// RFC 8216bis has it answer 503.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Future for HlsBlock {
    type Output = HlsResponse;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<HlsResponse> {
        let Some(stream) = self.stream.upgrade() else {
            return Poll::Ready(HlsResponse::error(404));
        };
        match stream.answer(&self.route) {
            Ok(response) => Poll::Ready(response),
            Err(Waiting) => {
                let mut waiters = stream.waiters.borrow_mut();
                if let Some((_, w)) = waiters.iter_mut().find(|(id, _)| *id == self.waiter) {
                    match w {
                        Some(w) if w.will_wake(cx.waker()) => {}
                        _ => *w = Some(cx.waker().clone()),
                    }
                }
                Poll::Pending
            }
        }
    }
}

impl Drop for HlsBlock {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.upgrade() {
            stream
                .waiters
                .borrow_mut()
                .retain(|(id, _)| *id != self.waiter);
        }
    }
}

impl fmt::Debug for HlsBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HlsBlock")
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
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

    /// Answers a GET of `path` at `now`, as [`request`](Self::request)
    /// does, at once: a request that would block answers 503.
    pub fn get(&self, now: Time, path: &str) -> HlsResponse {
        match self.request(now, path) {
            HlsAnswer::Ready(response) => response,
            HlsAnswer::Blocked(_) => HlsResponse::unavailable(),
        }
    }

    /// Answers a GET of `path` at `now`, or blocks it until its stream
    /// holds what it asks for ([Blocking](self#blocking)). Of a query
    /// string, only a playlist's delivery directives are read.
    ///
    /// A 200, and a request that blocks, record `now` as the stream's last
    /// access. Playlists can't be cached; inits, segments, parts and
    /// subtitles can, since their paths never name other bytes: numbers
    /// start at the stream's open time (see [`HlsProfile::open_stream`]).
    pub fn request(&self, now: Time, path: &str) -> HlsAnswer {
        let route = match parse_path(path) {
            Ok(route) => route,
            Err(status) => return HlsAnswer::Ready(HlsResponse::error(status)),
        };
        let profile = self.profiles.borrow().get(&route.profile).cloned();
        let Some(profile) = profile else {
            return HlsAnswer::Ready(HlsResponse::error(404));
        };
        let stream = profile.streams.borrow().get(&route.stream).cloned();
        let Some(stream) = stream else {
            return HlsAnswer::Ready(HlsResponse::error(404));
        };
        match stream.answer(&route) {
            Ok(response) => {
                if response.status == 200 {
                    stream.touch(now);
                }
                HlsAnswer::Ready(response)
            }
            Err(Waiting) => {
                stream.touch(now);
                let waiter = {
                    let mut waiters = stream.waiters.borrow_mut();
                    if waiters.len() >= MAX_BLOCKED_REQUESTS {
                        return HlsAnswer::Ready(HlsResponse::unavailable());
                    }
                    let id = stream.next_waiter.get();
                    stream.next_waiter.set(id.wrapping_add(1));
                    waiters.push((id, None));
                    id
                };
                let timeout = stream.playlist.borrow().block_timeout();
                HlsAnswer::Blocked(HlsBlock {
                    stream: Rc::downgrade(&stream),
                    route,
                    waiter,
                    timeout,
                })
            }
        }
    }

    /// The requests blocked on the stream of `transfer_id` under the
    /// profile of `profile_key`.
    pub fn blocked(&self, profile_key: &[u8; KEY_LEN], transfer_id: &[u8; KEY_LEN]) -> usize {
        let profile = self.profiles.borrow().get(profile_key).cloned();
        let stream = profile.and_then(|p| p.streams.borrow().get(transfer_id).cloned());
        stream.map_or(0, |s| s.waiters.borrow().len())
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
        let state = Rc::new(StreamState::new(Playlist::starting_at(config, first), now));
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

    /// Records a request of the stream at `now`, as a GET that answers
    /// does ([`HlsRoutes::get`]).
    pub fn touch(&self, now: Time) {
        self.state.touch(now);
    }

    /// See [`Playlist::push_segment`].
    pub fn push_segment(&self, init: &[u8], media: &[u8], duration: Duration) {
        self.state
            .playlist
            .borrow_mut()
            .push_segment(init, media, duration);
        self.state.changed();
    }

    /// See [`Playlist::push`].
    pub fn push(&self, segment: &HlsSegment) {
        self.state.playlist.borrow_mut().push(segment);
        self.state.changed();
    }

    /// See [`Playlist::push_part`].
    pub fn push_part(&self, part: &HlsPart) {
        self.state.playlist.borrow_mut().push_part(part);
        self.state.changed();
    }

    /// The number of segments the playlist lists.
    pub fn listed_segments(&self) -> usize {
        self.state.playlist.borrow().listed_segments()
    }

    /// See [`Playlist::discontinuity`].
    pub fn discontinuity(&self) {
        self.state.playlist.borrow_mut().discontinuity();
    }

    /// See [`Playlist::end`].
    pub fn end(&self) {
        self.state.playlist.borrow_mut().end();
        self.state.changed();
    }

    /// The bytes the playlist keeps; see [`Playlist::retained_bytes`].
    pub fn retained_bytes(&self) -> usize {
        self.state.playlist.borrow().retained_bytes()
    }
}

impl Drop for HlsStream {
    fn drop(&mut self) {
        // Its blocked requests answer 404 once it is gone.
        self.state.changed();
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
    Multivariant,
    Playlist,
    Subtitles,
    Init(u64),
    Segment(u64),
    Part(u64),
    Vtt(u64),
    PartVtt(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Route {
    profile: [u8; KEY_LEN],
    stream: [u8; KEY_LEN],
    file: File,
    /// A playlist's delivery directives: the segment, and the part of it,
    /// a blocking reload asks for.
    directives: Option<(u64, Option<u64>)>,
}

/// A request path's route, or the status of its refusal: 404 outside
/// `/hls/` and for unknown file names, 400 for anything else malformed.
fn parse_path(path: &str) -> Result<Route, u16> {
    if path.len() > MAX_PATH_LEN {
        return Err(400);
    }
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let rest = path.strip_prefix("/hls/").ok_or(404u16)?;
    let mut parts = rest.split('/');
    let (Some(profile), Some(stream), Some(file), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(400);
    };
    let file = parse_file(file)?;
    let directives = match file {
        File::Playlist | File::Subtitles => parse_directives(query)?,
        _ => None,
    };
    Ok(Route {
        profile: decode_key(profile).ok_or(400u16)?,
        stream: decode_key(stream).ok_or(400u16)?,
        file,
        directives,
    })
}

/// A playlist query's `_HLS_msn` and `_HLS_part`, or 400 for one that
/// isn't a number, comes twice, or a part without a segment. Other
/// parameters, `_HLS_skip` among them, are ignored.
fn parse_directives(query: &str) -> Result<Option<(u64, Option<u64>)>, u16> {
    let (mut msn, mut part) = (None, None);
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let slot = match key {
            "_HLS_msn" => &mut msn,
            "_HLS_part" => &mut part,
            _ => continue,
        };
        if slot.is_some() {
            return Err(400);
        }
        *slot = Some(parse_number(value).ok_or(400u16)?);
    }
    match (msn, part) {
        (None, Some(_)) => Err(400),
        (None, None) => Ok(None),
        (Some(msn), part) => Ok(Some((msn, part))),
    }
}

fn parse_file(name: &str) -> Result<File, u16> {
    match name {
        "multivariant.m3u8" => return Ok(File::Multivariant),
        "index.m3u8" => return Ok(File::Playlist),
        "subtitles.m3u8" => return Ok(File::Subtitles),
        _ => {}
    }
    if let Some(number) = name
        .strip_prefix("part-")
        .and_then(|n| n.strip_suffix(".m4s"))
    {
        return parse_number(number).map(File::Part).ok_or(400);
    }
    if let Some(seq) = name.strip_suffix(".m4s") {
        return parse_number(seq).map(File::Segment).ok_or(400);
    }
    if let Some(number) = name
        .strip_prefix("part-")
        .and_then(|n| n.strip_suffix(".vtt"))
    {
        return parse_number(number).map(File::PartVtt).ok_or(400);
    }
    if let Some(seq) = name.strip_suffix(".vtt") {
        return parse_number(seq).map(File::Vtt).ok_or(400);
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

#[cfg(test)]
#[path = "hls_parts_test.rs"]
mod parts_tests;

#[cfg(test)]
#[path = "hls_ll_test.rs"]
mod ll_tests;
