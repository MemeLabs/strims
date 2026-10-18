// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The fMP4 segmenter: a [`Muxer`] writing one record per segment to a
//! [`RecordSink`].
//!
//! # Records
//!
//! A record is `[u16 BE init length][init segment][media segment]` (see
//! [`crate::fmp4`]). Its media starts at a keyframe of the cut track: the
//! video track with the lowest id. Its init declares the segment's
//! tracks, video first, each kind in track id order.
//!
//! # Cut rule
//!
//! - The first segment starts at the cut track's first keyframe. Its
//!   tracks are the configs set by then; units before it are dropped
//!   ([`SegmenterMetrics::dropped_before_start`]).
//! - At a later keyframe of the cut track, the segment ends and the
//!   keyframe starts the next one when the segment holds at least
//!   [`CutRule::min_media_bytes`] of samples and the sink can end the
//!   record after its fragment. When it can't
//!   ([`RecordSink::end_blocked_after`]), the keyframe's group joins the
//!   segment and the next keyframe decides again
//!   ([`SegmenterMetrics::blocked_cuts`]).
//! - A changed config (new codec parameters for a track, or a new track)
//!   ends the segment at the next keyframe of the cut track whatever its
//!   size. When the sink can't end the record there, a top-level `free`
//!   box after the `mdat` pads the record to
//!   [`RecordSink::unblocking_len`]. The next segment's init declares the
//!   new configs. Until then, units of the changed or new tracks are
//!   dropped ([`SegmenterMetrics::dropped_pending_config`]): the segment's
//!   init doesn't describe them.
//! - A record longer than [`CutRule::max_record_bytes`] is
//!   [`SegmentError::RecordTooLarge`]: no keyframe came within the bound.
//!   The segmenter stops; every later call returns
//!   [`SegmentError::Closed`].
//! - [`Muxer::finish`] writes and ends the segment in progress, padded as
//!   for a changed config when the sink can't end it. The next segment
//!   starts at the cut track's next keyframe.
//!
//! # Timing
//!
//! - A sample lasts from its DTS to the next sample's in its track; a
//!   decreasing DTS gives 0.
//! - At a cut, the cut track's last sample lasts until the keyframe that
//!   starts the next segment. An audio track's last sample lasts its
//!   frame's samples. Any other last sample, and the cut track's at
//!   [`Muxer::finish`], repeats its track's previous duration, or lasts
//!   1/30 s without one.
//! - The composition offset is PTS less DTS, signed and saturated to 32
//!   bits. A track's `tfdt` is its first DTS in the segment, 0 when
//!   negative.
//!
//! # Writing
//!
//! A record is written in three steps: the length prefix and init when
//! its segment starts, the fragment (`moof` and `mdat`) at the cut, and
//! the end. The samples in between are buffered. A fragment per access
//! unit, or per small group, is a change to when fragments are flushed
//! within the record: the cut rule and the end gate apply to the record
//! as they do here, each fragment's `tfdt` starts where the previous one
//! left off, and the `mfhd` sequence numbers count fragments.
//!
//! # Bounds
//!
//! A segment's buffered samples are part of its record, so they stay
//! within [`CutRule::max_record_bytes`]: at most that many bytes of
//! payload, and, since each sample adds at least 8 bytes of `trun`
//! entry, at most an eighth as many samples of 16 bytes of state. A
//! segment has at most [`MAX_TRACKS`] tracks. The buffers are reused from
//! one segment to the next. [`Segmenter::take_segment`] holds the
//! [`MAX_PENDING_SEGMENTS`] latest segments.

use std::collections::{BTreeMap, VecDeque};

use strims_base::time::Duration;

use crate::fmp4::{
    self, EMPTY_FRAGMENT_LEN, FREE_HEADER_LEN, InitError, MAX_TRACKS, Sample, TrackRun, traf_len,
    write_fragment_head, write_free, write_init,
};
use crate::media::{AccessUnit, CodecParams, DecoderConfig, Muxer, TrackId, TrackKind};
use crate::record::RecordSink;

/// The longest init segment: the record prefix holds its length in 16
/// bits.
pub const MAX_INIT_LEN: usize = u16::MAX as usize;
/// The largest record bound: fragment sizes and data offsets are signed
/// 32-bit. A larger [`CutRule::max_record_bytes`] is lowered to it.
pub const MAX_RECORD_LIMIT: usize = i32::MAX as usize;
/// The most [`SegmentInfo`]s [`Segmenter::take_segment`] holds; older
/// ones are dropped.
pub const MAX_PENDING_SEGMENTS: usize = 16;
/// Bytes of the record's init length prefix.
const PREFIX_LEN: usize = 2;
/// The frame rate of a video sample's duration when nothing times it.
const FALLBACK_FRAME_RATE: u32 = 30;

/// When the segmenter ends a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutRule {
    /// The least sample payload a segment holds before a keyframe can end
    /// it.
    pub min_media_bytes: usize,
    /// The longest record, prefix, init and padding included.
    pub max_record_bytes: usize,
}

impl CutRule {
    pub const DEFAULT_MIN_MEDIA_BYTES: usize = 32 * 1024;
    /// Half the ingress swarm writer's 16 MiB window.
    pub const DEFAULT_MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
}

impl Default for CutRule {
    fn default() -> Self {
        CutRule {
            min_media_bytes: Self::DEFAULT_MIN_MEDIA_BYTES,
            max_record_bytes: Self::DEFAULT_MAX_RECORD_BYTES,
        }
    }
}

/// A record the segmenter ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentInfo {
    /// The media time it covers: the cut track's sample durations, or the
    /// longest track's without one.
    pub duration: Duration,
    pub video_samples: u32,
    pub audio_samples: u32,
    /// The record's length, prefix and padding included.
    pub bytes: usize,
    /// Bytes of `free` padding after the `mdat`.
    pub padding: usize,
    /// Its init differs from the previous record's; true for the first.
    pub new_init: bool,
}

/// What a segmenter has written and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SegmenterMetrics {
    pub records: u64,
    pub record_bytes: u64,
    /// Units before the first segment's keyframe.
    pub dropped_before_start: u64,
    /// Units of a changed or new track before the segment that declares
    /// its config.
    pub dropped_pending_config: u64,
    /// Keyframes at which the sink couldn't end the record.
    pub blocked_cuts: u64,
    pub padded_records: u64,
    pub padding_bytes: u64,
    /// [`SegmentInfo`]s dropped untaken.
    pub dropped_infos: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SegmentError {
    /// The segment grew past the record bound without a keyframe to end
    /// it.
    #[error("no keyframe arrived within the {limit}-byte record bound")]
    RecordTooLarge { limit: usize },
    #[error("invalid decoder config: {0}")]
    Config(#[from] InitError),
    #[error("the init segment is longer than {MAX_INIT_LEN} bytes")]
    InitTooLong,
    #[error("track {0} has no decoder config")]
    UnknownTrack(u32),
    /// The sink refused a record end it said it would take.
    #[error("the record sink refused to end the record")]
    EndRefused,
    #[error("the segmenter stopped after an error")]
    Closed,
}

/// A track of the segment in progress.
#[derive(Debug)]
struct SegTrack {
    track: TrackId,
    kind: TrackKind,
    timescale: u32,
    /// Audio samples per frame; 0 for video.
    frame_samples: u32,
    /// The current config differs from the one the segment declares.
    stale: bool,
    base_dts: i64,
    last_dts: i64,
    /// The latest duration timed by a following sample, kept across
    /// segments.
    prev_duration: Option<u32>,
    samples: Vec<Sample>,
    data: Vec<u8>,
}

impl SegTrack {
    fn new(config: &DecoderConfig, reuse: Option<SegTrack>) -> Self {
        let frame_samples = match config.params {
            CodecParams::Aac { frame_samples, .. } => frame_samples,
            CodecParams::Avc { .. } => 0,
        };
        let (prev_duration, samples, data) = match reuse {
            Some(t) if t.timescale == config.timescale => (t.prev_duration, t.samples, t.data),
            Some(t) => (None, t.samples, t.data),
            None => (None, Vec::new(), Vec::new()),
        };
        SegTrack {
            track: config.track,
            kind: config.kind,
            timescale: config.timescale,
            frame_samples,
            stale: false,
            base_dts: 0,
            last_dts: 0,
            prev_duration,
            samples,
            data,
        }
    }

    fn fallback_duration(&self) -> u32 {
        self.prev_duration
            .unwrap_or(self.timescale / FALLBACK_FRAME_RATE)
    }
}

/// Cuts access units into segments, one record each (see the
/// [module docs](self)).
#[derive(Debug)]
pub struct Segmenter<S: RecordSink> {
    rule: CutRule,
    sink: S,
    /// The current configs, at most [`MAX_TRACKS`].
    configs: BTreeMap<TrackId, DecoderConfig>,
    /// The configs the segment in progress (or the last one) declares, in
    /// init order; `tracks` follows it.
    segment_configs: Vec<DecoderConfig>,
    tracks: Vec<SegTrack>,
    /// The init of `segment_configs`.
    init: Vec<u8>,
    /// Reused for inits being checked, fragment heads and padding.
    scratch: Vec<u8>,
    started: bool,
    /// The configs differ from the segment's: the next keyframe cuts.
    changed: bool,
    /// The segment in progress has another init than the last record.
    new_init: bool,
    /// Bytes of the segment's fragment so far.
    fragment_len: usize,
    /// Bytes of the segment's sample payload.
    media_bytes: usize,
    /// The next fragment's `mfhd` sequence number.
    sequence: u32,
    infos: VecDeque<SegmentInfo>,
    metrics: SegmenterMetrics,
    closed: bool,
}

impl<S: RecordSink> Segmenter<S> {
    pub fn new(rule: CutRule, sink: S) -> Self {
        Segmenter {
            rule: CutRule {
                max_record_bytes: rule.max_record_bytes.min(MAX_RECORD_LIMIT),
                ..rule
            },
            sink,
            configs: BTreeMap::new(),
            segment_configs: Vec::new(),
            tracks: Vec::new(),
            init: Vec::new(),
            scratch: Vec::new(),
            started: false,
            changed: false,
            new_init: false,
            fragment_len: EMPTY_FRAGMENT_LEN,
            media_bytes: 0,
            sequence: 1,
            infos: VecDeque::new(),
            metrics: SegmenterMetrics::default(),
            closed: false,
        }
    }

    pub fn rule(&self) -> CutRule {
        self.rule
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    pub fn into_sink(self) -> S {
        self.sink
    }

    pub fn metrics(&self) -> SegmenterMetrics {
        self.metrics
    }

    /// The configs the segment in progress declares, or the last one's,
    /// in init order. Empty before the first segment.
    pub fn segment_configs(&self) -> &[DecoderConfig] {
        &self.segment_configs
    }

    /// The oldest ended record not yet taken.
    pub fn take_segment(&mut self) -> Option<SegmentInfo> {
        self.infos.pop_front()
    }

    fn check_open(&self) -> Result<(), SegmentError> {
        if self.closed {
            Err(SegmentError::Closed)
        } else {
            Ok(())
        }
    }

    fn fail<T>(&mut self, error: SegmentError) -> Result<T, SegmentError> {
        self.closed = true;
        Err(error)
    }

    /// The track whose keyframes start segments.
    fn cut_track(&self) -> Option<TrackId> {
        self.configs
            .values()
            .find(|c| c.kind == TrackKind::Video)
            .map(|c| c.track)
    }

    fn record_len(&self) -> usize {
        PREFIX_LEN + self.init.len() + self.fragment_len
    }

    /// Marks the segment's tracks whose configs changed, and whether any
    /// config differs from the segment's.
    fn update_changed(&mut self) {
        for (config, track) in self.segment_configs.iter().zip(self.tracks.iter_mut()) {
            track.stale = self.configs.get(&config.track) != Some(config);
        }
        self.changed =
            self.tracks.iter().any(|t| t.stale) || self.configs.len() != self.segment_configs.len();
    }

    /// Starts a segment: declares the current configs and writes the
    /// record's prefix and init.
    fn start_segment(&mut self) -> Result<(), SegmentError> {
        let same = self.configs.len() == self.segment_configs.len()
            && ordered(&self.configs)
                .zip(&self.segment_configs)
                .all(|(a, b)| a == b);
        if same {
            self.new_init = false;
        } else {
            let configs: Vec<DecoderConfig> = ordered(&self.configs).cloned().collect();
            self.scratch.clear();
            write_init(&configs, &mut self.scratch)?;
            if self.scratch.len() > MAX_INIT_LEN {
                return Err(SegmentError::InitTooLong);
            }
            self.new_init = self.scratch != self.init;
            std::mem::swap(&mut self.init, &mut self.scratch);
            let mut old = std::mem::take(&mut self.tracks);
            self.tracks = configs
                .iter()
                .map(|c| {
                    let reuse = old
                        .iter()
                        .position(|t| t.track == c.track)
                        .map(|i| old.swap_remove(i));
                    SegTrack::new(c, reuse)
                })
                .collect();
            self.segment_configs = configs;
        }
        for t in &mut self.tracks {
            t.samples.clear();
            t.data.clear();
            t.stale = false;
        }
        self.changed = false;
        // The length fits: inits are checked against MAX_INIT_LEN.
        let len = u16::try_from(self.init.len()).unwrap_or(u16::MAX);
        self.sink.write(&len.to_be_bytes());
        self.sink.write(&self.init);
        self.fragment_len = EMPTY_FRAGMENT_LEN;
        self.media_bytes = 0;
        self.started = true;
        Ok(())
    }

    /// Adds a unit to the segment in progress; drops it when its track's
    /// config isn't the segment's.
    fn add(&mut self, unit: &AccessUnit<'_>) -> Result<(), SegmentError> {
        let Some(t) = self
            .tracks
            .iter_mut()
            .find(|t| t.track == unit.track && !t.stale)
        else {
            self.metrics.dropped_pending_config += 1;
            return Ok(());
        };
        let size = unit.payload.len();
        let n = t.samples.len();
        let grow = traf_len(t.kind, n + 1) - traf_len(t.kind, n) + size;
        let limit = self.rule.max_record_bytes;
        if PREFIX_LEN + self.init.len() + self.fragment_len + grow > limit {
            return self.fail(SegmentError::RecordTooLarge { limit });
        }
        match t.samples.last_mut() {
            Some(last) => {
                let d = duration(unit.dts.saturating_sub(t.last_dts));
                last.duration = d;
                t.prev_duration = Some(d);
            }
            None => t.base_dts = unit.dts,
        }
        let cto = unit.pts.saturating_sub(unit.dts);
        t.samples.push(Sample {
            duration: 0,
            // Below the record bound.
            size: u32::try_from(size).unwrap_or(u32::MAX),
            keyframe: unit.keyframe,
            composition_offset: i32::try_from(cto).unwrap_or(if cto < 0 {
                i32::MIN
            } else {
                i32::MAX
            }),
        });
        t.data.extend_from_slice(unit.payload);
        t.last_dts = unit.dts;
        self.fragment_len += grow;
        self.media_bytes += size;
        Ok(())
    }

    /// Ends the segment in progress before a keyframe of the cut track at
    /// `cut_dts`, or at the end of the stream when `None`. A blocked end
    /// is padded when `forced`, and otherwise leaves the segment going:
    /// returns whether the record ended.
    fn cut(&mut self, cut_dts: Option<i64>, forced: bool) -> Result<bool, SegmentError> {
        let cut_track = self.cut_track();
        for t in &mut self.tracks {
            let fallback = t.fallback_duration();
            let Some(last) = t.samples.last_mut() else {
                continue;
            };
            last.duration = match (t.kind, cut_dts) {
                (TrackKind::Video, Some(dts)) if cut_track == Some(t.track) => {
                    let d = duration(dts.saturating_sub(t.last_dts));
                    t.prev_duration = Some(d);
                    d
                }
                (TrackKind::Audio, _) if t.frame_samples > 0 => t.frame_samples,
                _ => fallback,
            };
        }
        let len = self.fragment_len;
        let padding = if !self.sink.end_blocked_after(len) {
            0
        } else if forced {
            self.sink
                .unblocking_len(len + FREE_HEADER_LEN)
                .saturating_sub(len)
                .max(FREE_HEADER_LEN)
        } else {
            self.metrics.blocked_cuts += 1;
            return Ok(false);
        };
        let limit = self.rule.max_record_bytes;
        if self.record_len().saturating_add(padding) > limit {
            return self.fail(SegmentError::RecordTooLarge { limit });
        }
        self.write_record(padding)?;
        Ok(true)
    }

    /// Writes the segment's fragment and `padding` bytes of `free`, and
    /// ends the record.
    fn write_record(&mut self, padding: usize) -> Result<(), SegmentError> {
        let mut runs = [TrackRun::default(); MAX_TRACKS];
        let mut n = 0;
        for (t, run) in self.tracks.iter().zip(runs.iter_mut()) {
            *run = TrackRun {
                track: t.track,
                kind: t.kind,
                base_decode_time: u64::try_from(t.base_dts).unwrap_or(0),
                samples: &t.samples,
                data: &t.data,
            };
            n += 1;
        }
        let runs = runs.get(..n).unwrap_or_default();
        debug_assert_eq!(fmp4::fragment_len(runs), self.fragment_len);
        self.scratch.clear();
        write_fragment_head(self.sequence, runs, &mut self.scratch);
        self.sink.write(&self.scratch);
        for run in runs {
            self.sink.write(run.data);
        }
        if padding > 0 {
            self.scratch.clear();
            write_free(padding, &mut self.scratch);
            self.sink.write(&self.scratch);
        }
        let info = segment_info(&self.tracks);
        if self.sink.end_record().is_err() {
            return self.fail(SegmentError::EndRefused);
        }
        let bytes = self.record_len() + padding;
        if self.infos.len() >= MAX_PENDING_SEGMENTS {
            self.infos.pop_front();
            self.metrics.dropped_infos += 1;
        }
        self.infos.push_back(SegmentInfo {
            bytes,
            padding,
            new_init: self.new_init,
            ..info
        });
        self.metrics.records += 1;
        self.metrics.record_bytes += bytes as u64;
        if padding > 0 {
            self.metrics.padded_records += 1;
            self.metrics.padding_bytes += padding as u64;
        }
        self.sequence = self.sequence.checked_add(1).unwrap_or(1);
        self.started = false;
        Ok(())
    }
}

impl<S: RecordSink> Muxer for Segmenter<S> {
    type Error = SegmentError;

    /// Sets a track's config. Refused, changing nothing, when the configs
    /// can't be declared together ([`fmp4::check_configs`]) or their init
    /// would be longer than [`MAX_INIT_LEN`].
    fn set_config(&mut self, config: &DecoderConfig) -> Result<(), SegmentError> {
        self.check_open()?;
        if self.configs.get(&config.track) == Some(config) {
            return Ok(());
        }
        let mut configs = self.configs.clone();
        configs.insert(config.track, config.clone());
        let list: Vec<DecoderConfig> = ordered(&configs).cloned().collect();
        self.scratch.clear();
        write_init(&list, &mut self.scratch)?;
        if self.scratch.len() > MAX_INIT_LEN {
            return Err(SegmentError::InitTooLong);
        }
        self.configs = configs;
        if self.started {
            self.update_changed();
        }
        Ok(())
    }

    fn push(&mut self, unit: &AccessUnit<'_>) -> Result<(), SegmentError> {
        self.check_open()?;
        if !self.configs.contains_key(&unit.track) {
            return Err(SegmentError::UnknownTrack(unit.track.0));
        }
        let cuts = unit.keyframe && self.cut_track() == Some(unit.track);
        if self.started {
            let wanted = cuts && (self.changed || self.media_bytes >= self.rule.min_media_bytes);
            if !wanted || !self.cut(Some(unit.dts), self.changed)? {
                return self.add(unit);
            }
        } else if !cuts {
            self.metrics.dropped_before_start += 1;
            return Ok(());
        }
        self.start_segment()?;
        self.add(unit)
    }

    fn finish(&mut self) -> Result<(), SegmentError> {
        self.check_open()?;
        if self.started {
            self.cut(None, true)?;
        }
        Ok(())
    }
}

/// The configs in init order: video, then audio, each by track id.
fn ordered(
    configs: &BTreeMap<TrackId, DecoderConfig>,
) -> impl Iterator<Item = &DecoderConfig> + Clone {
    let of = move |kind| configs.values().filter(move |c| c.kind == kind);
    of(TrackKind::Video).chain(of(TrackKind::Audio))
}

/// A sample duration from a DTS difference: 0 when negative, saturated
/// to 32 bits.
fn duration(d: i64) -> u32 {
    u32::try_from(d.max(0)).unwrap_or(u32::MAX)
}

/// The counts and media time of the segment in `tracks`: its first video
/// track's time, or its longest track's.
fn segment_info(tracks: &[SegTrack]) -> SegmentInfo {
    let time = |t: &SegTrack| {
        let units: u64 = t.samples.iter().map(|s| u64::from(s.duration)).sum();
        if t.timescale == 0 {
            return Duration::ZERO;
        }
        let nanos = i128::from(units) * 1_000_000_000 / i128::from(t.timescale);
        Duration::from_nanos(i64::try_from(nanos).unwrap_or(i64::MAX))
    };
    let count = |kind| {
        let n: usize = tracks
            .iter()
            .filter(|t| t.kind == kind)
            .map(|t| t.samples.len())
            .sum();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let duration = match tracks.iter().find(|t| t.kind == TrackKind::Video) {
        Some(t) => time(t),
        None => tracks.iter().map(time).max().unwrap_or(Duration::ZERO),
    };
    SegmentInfo {
        duration,
        video_samples: count(TrackKind::Video),
        audio_samples: count(TrackKind::Audio),
        bytes: 0,
        padding: 0,
        new_init: false,
    }
}

#[cfg(test)]
#[path = "segment_test.rs"]
mod tests;
