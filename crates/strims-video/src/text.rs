// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Timed text: the cues of a stream's text tracks, and the reader that
//! turns a stream's units into the cues players show, its text tracks'
//! and its video's CEA-608 captions.
//!
//! A text track ([`TrackKind::Text`]) has the codec [`CODEC`] and no
//! configuration record. Each of its access units is a cue
//! ([`Cue`]): what shows from the unit's presentation time, for its
//! duration. Every cue unit is a keyframe. A unit's payload:
//!
//! ```text
//! head_len  uvarint  the length of the head fields that follow
//! flags     u8       bit 0: CONTINUES; the other bits are written 0 and ignored
//! duration  uvarint  how long the cue shows, in the track's timescale
//! text      the rest: UTF-8, lines joined by LF, at most MAX_CUE_TEXT bytes
//! ```
//!
//! A reader skips the head's bytes after the fields it knows, so the head
//! gains fields only at its end. An empty text shows nothing: the track's
//! cues end there. A CONTINUES cue is one already showing, repeated at a
//! GOP's start for readers that join there; a reader showing a cue of the
//! same text takes it as that cue.
//!
//! A cue of a text track shows at least [`MIN_CUE`], and the track's next
//! cue ends it sooner down to that floor: so a cue's end is
//! `max(start + MIN_CUE, min(start + duration, next start))`, and cues
//! closer together than the floor show at once, stacked.
//!
//! A stream's closed captions, CC1 of the A/53 captions its video's SEI
//! carries ([`captions`](crate::captions), [`cea608`](crate::cea608)),
//! are cues too: each shows from the frame where the caption appears on
//! screen to the frame where it changes or goes, or for at most
//! [`CAPTION_TIMEOUT`] without a change.
//!
//! Every function takes hostile input: none panics, and each works in
//! time linear in its input.

use std::collections::VecDeque;

use strims_base::time::Duration;
use strims_chunkstream::{MAX_UVARINT_LEN, get_uvarint, put_uvarint};

use crate::captions::{CcTuple, NalFormat};
use crate::cea608::Cc1Decoder;
use crate::media::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

/// The codec string of a text track's decoder config.
pub const CODEC: &str = "cues";
/// The timescale writers give a text track: milliseconds.
pub const TIMESCALE: u32 = 1000;
/// The most bytes of a cue's text.
pub const MAX_CUE_TEXT: usize = 2048;
/// The least time a text track's cue shows: a readability floor, which a
/// one-word cue needs as much as a long one. It holds when the track's
/// next cue starts sooner, and the two show stacked.
pub const MIN_CUE: Duration = Duration::from_millis(800);
/// The longest duration [`estimate`] gives a cue.
pub const MAX_ESTIMATE: Duration = Duration::from_secs(7);
/// The longest a caption shows without a change: a stream whose captions
/// stop leaves none on screen.
pub const CAPTION_TIMEOUT: Duration = Duration::from_secs(16);
/// The most video frames a [`CueReader`] holds to read their captions in
/// presentation order.
const MAX_REORDER: usize = 16;
/// The reading rate [`estimate`] assumes, in characters a second: a
/// subtitle reading rate for adult viewers.
const READ_RATE: i64 = 15;
/// A cue's flag of a cue already showing, repeated.
const FLAG_CONTINUES: u8 = 0x01;
/// The most cues a [`CueReader`] holds as showing; past it the oldest is
/// forgotten, keeping the end it was shown with.
const MAX_SHOWING: usize = 8;

/// The decoder config of text track `track`, at [`TIMESCALE`].
pub fn config(track: TrackId) -> DecoderConfig {
    DecoderConfig {
        track,
        kind: TrackKind::Text,
        codec: CODEC.into(),
        timescale: TIMESCALE,
        params: CodecParams::Text,
    }
}

/// A text track's access unit: what shows from its presentation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cue<'a> {
    /// The cue was showing before the unit: a repeat at a GOP's start.
    pub continues: bool,
    /// How long it shows, in the track's timescale.
    pub duration: u64,
    /// Lines joined by LF; empty shows nothing.
    pub text: &'a str,
}

/// Why a cue unit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CueError {
    #[error("the cue is truncated")]
    Truncated,
    #[error("the cue's text is longer than {MAX_CUE_TEXT} bytes")]
    TooLong,
    #[error("the cue's text is not UTF-8")]
    NotUtf8,
}

/// Reads a cue unit's payload.
///
/// # Errors
///
/// The head is truncated, or the text is over [`MAX_CUE_TEXT`] bytes or
/// not UTF-8.
pub fn parse_cue(payload: &[u8]) -> Result<Cue<'_>, CueError> {
    let (head_len, n) = get_uvarint(payload).ok_or(CueError::Truncated)?;
    let rest = payload.get(n..).unwrap_or_default();
    let head_len = usize::try_from(head_len).map_err(|_| CueError::Truncated)?;
    if head_len > rest.len() {
        return Err(CueError::Truncated);
    }
    let (head, text) = rest.split_at(head_len);
    let (&flags, head) = head.split_first().ok_or(CueError::Truncated)?;
    let (duration, _) = get_uvarint(head).ok_or(CueError::Truncated)?;
    if text.len() > MAX_CUE_TEXT {
        return Err(CueError::TooLong);
    }
    let text = std::str::from_utf8(text).map_err(|_| CueError::NotUtf8)?;
    Ok(Cue {
        continues: flags & FLAG_CONTINUES != 0,
        duration,
        text,
    })
}

/// Appends `cue`'s unit payload. Its text must be at most
/// [`MAX_CUE_TEXT`] bytes for [`parse_cue`] to read it.
pub fn write_cue(cue: &Cue<'_>, out: &mut Vec<u8>) {
    let mut duration = [0; MAX_UVARINT_LEN];
    let n = put_uvarint(&mut duration, cue.duration);
    let duration = duration.get(..n).unwrap_or_default();
    // head_len is under 0x80: a one-byte varint.
    out.push(1 + duration.len() as u8);
    out.push(if cue.continues { FLAG_CONTINUES } else { 0 });
    out.extend_from_slice(duration);
    out.extend_from_slice(cue.text.as_bytes());
}

/// How long `text` takes to read: 15 characters a second, a
/// character of a CJK script counting as two and a run of whitespace as
/// one, from [`MIN_CUE`] to [`MAX_ESTIMATE`].
pub fn estimate(text: &str) -> Duration {
    let mut chars: i64 = 0;
    let mut space = true;
    for c in text.chars() {
        if c.is_whitespace() {
            if !space {
                chars += 1;
            }
            space = true;
            continue;
        }
        space = false;
        chars += if is_wide(c) { 2 } else { 1 };
    }
    let read = Duration::from_millis(chars.saturating_mul(1000) / READ_RATE);
    read.clamp(MIN_CUE, MAX_ESTIMATE)
}

/// Whether `c` is of a script read about half as fast per character as
/// Latin: Han, kana, Hangul, and their full-width forms.
fn is_wide(c: char) -> bool {
    matches!(
        u32::from(c),
        0x1100..=0x11ff
            | 0x2e80..=0x9fff
            | 0xa960..=0xa97f
            | 0xac00..=0xd7ff
            | 0xf900..=0xfaff
            | 0xfe30..=0xfe4f
            | 0xff00..=0xffef
            | 0x2_0000..=0x3_ffff
    )
}

/// Where a cue comes from, which players show as tracks of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CueKind {
    /// A text track's cue.
    Subtitles,
    /// Closed captions the video carries.
    Captions,
}

/// What a [`CueReader`] read, on the stream's media timeline: its
/// presentation times as durations from zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueEvent {
    /// A cue shows from `start` to `end`, unless an [`End`](Self::End)
    /// for it comes first.
    Show {
        /// Unique among the reader's cues.
        id: u64,
        kind: CueKind,
        start: Duration,
        end: Duration,
        /// Lines joined by LF.
        text: String,
    },
    /// Cue `id` ends at `end`, sooner than its show said.
    End { id: u64, end: Duration },
}

/// What a [`CueReader`] has read and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CueReaderMetrics {
    /// Text tracks' cues shown.
    pub cues: u64,
    /// Captions shown.
    pub captions: u64,
    /// Cue units refused ([`parse_cue`]).
    pub malformed: u64,
}

/// A cue the reader shows, which a later cue of its track may end.
#[derive(Debug, Clone)]
struct Showing {
    id: u64,
    track: TrackId,
    start: Duration,
    end: Duration,
    text: String,
}

/// Turns a stream's units into the cues players show ([`CueEvent`]): its
/// text tracks' cue units, and the CC1 captions of its first video track.
///
/// It takes the stream's configs ([`set_configs`](Self::set_configs)),
/// then its units in decode order ([`push`](Self::push)), and queues
/// events, which [`poll_event`](Self::poll_event) takes. Times are the
/// units' presentation times as durations. A video frame's captions are
/// read once no later frame can present before it: once a frame decodes
/// at or after its presentation time, or 16 frames on.
///
/// # Bounds
///
/// It holds at most [`MAX_TRACKS`](crate::fmp4::MAX_TRACKS) text tracks
/// and 8 cues of at most [`MAX_CUE_TEXT`] bytes, 16 frames' caption tuples
/// and a caption decoder's two memories; a push queues at most 34 events,
/// two of each of 17 frames, and its work is linear in the unit.
#[derive(Debug, Default)]
pub struct CueReader {
    /// The text tracks and their timescales.
    text: Vec<(TrackId, u32)>,
    /// Bound: `MAX_SHOWING`.
    showing: Vec<Showing>,
    /// The video track whose captions are read, its timescale and its
    /// units' NAL format.
    video: Option<(TrackId, u32, NalFormat)>,
    /// Video frames whose captions wait to be read, by presentation time.
    /// Bound: `MAX_REORDER`.
    frames: VecDeque<(i64, Vec<CcTuple>)>,
    cc1: Cc1Decoder,
    /// The caption showing.
    caption: Option<Showing>,
    /// Reused for a unit's tuples.
    tuples: Vec<CcTuple>,
    next_id: u64,
    events: VecDeque<CueEvent>,
    metrics: CueReaderMetrics,
}

impl CueReader {
    pub fn new() -> CueReader {
        CueReader::default()
    }

    pub fn metrics(&self) -> CueReaderMetrics {
        self.metrics
    }

    /// The stream's tracks are `configs`. A text track no longer among
    /// them forgets its cues, and captions start afresh when the video
    /// track changes.
    pub fn set_configs(&mut self, configs: &[DecoderConfig]) {
        self.text = configs
            .iter()
            .filter(|c| c.kind == TrackKind::Text)
            .take(crate::fmp4::MAX_TRACKS)
            .map(|c| (c.track, c.timescale))
            .collect();
        let text = &self.text;
        self.showing
            .retain(|s| text.iter().any(|(track, _)| *track == s.track));
        let video = configs
            .iter()
            .find(|c| c.kind == TrackKind::Video)
            .and_then(|c| NalFormat::of(&c.params).map(|f| (c.track, c.timescale, f)));
        if video != self.video {
            self.video = video;
            self.reset_captions();
        }
    }

    /// The stream skipped or started over: the cues showing are
    /// forgotten, and the next cue of each track starts afresh.
    pub fn reset(&mut self) {
        self.showing.clear();
        self.reset_captions();
    }

    fn reset_captions(&mut self) {
        self.frames.clear();
        self.cc1 = Cc1Decoder::new();
        self.caption = None;
    }

    /// Takes the stream's next unit.
    pub fn push(&mut self, unit: &AccessUnit<'_>) {
        if let Some((track, timescale, nals)) = self.video
            && track == unit.track
        {
            self.video_frame(unit, timescale, nals);
            return;
        }
        let Some(&(track, timescale)) = self.text.iter().find(|(t, _)| *t == unit.track) else {
            return;
        };
        let cue = match parse_cue(unit.payload) {
            Ok(cue) => cue,
            Err(_) => {
                self.metrics.malformed += 1;
                return;
            }
        };
        let at = to_duration(unit.pts, timescale);
        self.showing.retain(|s| s.track != track || s.end > at);
        if cue.continues
            && self
                .showing
                .iter()
                .any(|s| s.track == track && s.start <= at && s.text == cue.text)
        {
            return;
        }
        for s in self
            .showing
            .iter_mut()
            .filter(|s| s.track == track && s.start <= at)
        {
            let end = (s.start + MIN_CUE).max(at);
            if end < s.end {
                s.end = end;
                self.events.push_back(CueEvent::End { id: s.id, end });
            }
        }
        if cue.text.is_empty() {
            return;
        }
        let lasts = to_duration(i64::try_from(cue.duration).unwrap_or(i64::MAX), timescale);
        // A continued cue started before the reader joined: what is left
        // of it is all there is to show.
        let end = at
            + if cue.continues {
                lasts
            } else {
                lasts.max(MIN_CUE)
            };
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.showing.len() >= MAX_SHOWING {
            self.showing.remove(0);
        }
        self.showing.push(Showing {
            id,
            track,
            start: at,
            end,
            text: cue.text.into(),
        });
        self.metrics.cues += 1;
        self.events.push_back(CueEvent::Show {
            id,
            kind: CueKind::Subtitles,
            start: at,
            end,
            text: cue.text.into(),
        });
    }

    /// The next event, in the order the units made them.
    pub fn poll_event(&mut self) -> Option<CueEvent> {
        self.events.pop_front()
    }

    /// Holds a video frame's captions, and reads those of the frames
    /// before it in presentation order that no later frame can precede.
    fn video_frame(&mut self, unit: &AccessUnit<'_>, timescale: u32, nals: NalFormat) {
        self.tuples.clear();
        nals.cc_tuples(unit.payload, &mut self.tuples);
        let at = self
            .frames
            .iter()
            .position(|(pts, _)| *pts > unit.pts)
            .unwrap_or(self.frames.len());
        self.frames.insert(at, (unit.pts, self.tuples.clone()));
        while let Some((pts, _)) = self.frames.front() {
            if *pts > unit.dts && self.frames.len() <= MAX_REORDER {
                break;
            }
            let Some((pts, tuples)) = self.frames.pop_front() else {
                break;
            };
            let at = to_duration(pts, timescale);
            if self.caption.as_ref().is_some_and(|c| c.end <= at) {
                self.caption = None;
            }
            if let Some(text) = self.cc1.frame(at, &tuples) {
                self.caption_changed(at, text);
            }
        }
    }

    /// The caption on screen is `text` from `at`: it ends the one showing,
    /// unless it is that one.
    fn caption_changed(&mut self, at: Duration, text: String) {
        if let Some(c) = self.caption.take() {
            if c.text == text {
                self.caption = Some(c);
                return;
            }
            self.events.push_back(CueEvent::End { id: c.id, end: at });
        }
        if text.is_empty() {
            return;
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let end = at + CAPTION_TIMEOUT;
        self.metrics.captions += 1;
        self.events.push_back(CueEvent::Show {
            id,
            kind: CueKind::Captions,
            start: at,
            end,
            text: text.clone(),
        });
        self.caption = Some(Showing {
            id,
            track: TrackId(0),
            start: at,
            end,
            text,
        });
    }
}

/// `units` of `timescale` as a duration, saturated; zero for timescale 0.
pub(crate) fn to_duration(units: i64, timescale: u32) -> Duration {
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

#[cfg(test)]
#[path = "text_test.rs"]
mod tests;
