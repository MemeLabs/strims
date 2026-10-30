// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The fMP4 segmenter: a [`Muxer`] writing one record per segment to a
//! [`RecordSink`].
//!
//! # Records
//!
//! A record is `[u16 BE init length][init segment][media segment]` (see
//! [`crate::fmp4`]). Its media starts at a keyframe of the cut track: the
//! video track with the lowest id. Its first fragment holds that
//! keyframe. Its init declares the segment's tracks, video first, each
//! kind in track id order.
//!
//! # Cut rule
//!
//! - The first segment starts at the cut track's first keyframe. Its
//!   tracks are the configs set by then; units before it are dropped
//!   ([`SegmenterMetrics::dropped_before_start`]).
//! - At a later keyframe of the cut track, the segment ends and the
//!   keyframe starts the next one when the segment holds at least
//!   [`CutRule::min_media_bytes`] of samples and the sink can end the
//!   record after its last fragment. When it can't
//!   ([`RecordSink::end_blocked_after`]), the keyframe's group joins the
//!   segment and the next keyframe decides again
//!   ([`SegmenterMetrics::blocked_cuts`]).
//! - A changed config (new codec parameters for a track, or a new track)
//!   ends the segment at the next keyframe of the cut track whatever its
//!   size. When the sink can't end the record there, a top-level `free`
//!   box after the last `mdat` pads the record to
//!   [`RecordSink::unblocking_len`]. The next segment's init declares the
//!   new configs. Until then, units of the changed or new tracks are
//!   dropped ([`SegmenterMetrics::dropped_pending_config`]): the segment's
//!   init doesn't describe them.
//! - A record that a unit could take past [`CutRule::max_record_bytes`]
//!   is [`SegmentError::RecordTooLarge`]: no keyframe came within the
//!   bound. The segmenter stops; every later call returns
//!   [`SegmentError::Closed`].
//! - [`Muxer::finish`] writes and ends the segment in progress, padded as
//!   for a changed config when the sink can't end it. The next segment
//!   starts at the cut track's next keyframe.
//!
//! # Timing
//!
//! Samples are timed as [`FragmentWriter`] times them. At a cut, the cut
//! track's last sample lasts until the keyframe that starts the next
//! segment; at [`Muxer::finish`], the writer's flush rules time every
//! track's last.
//!
//! An init's edit list moves every track back by its composition delay:
//! the starting keyframe's PTS less DTS (none when not positive), taken
//! when the segment that declares new configs starts. Later segments of
//! the same configs keep it, so their inits stay the same.
//!
//! # Writing
//!
//! A record is written as it goes: the length prefix and init when its
//! segment starts, each fragment once [`FragmentWriter`] has it due
//! ([`CutRule::fragment_duration`] of samples timed, or a stream's first
//! GOP whole when it decodes from before the edit list), and the last
//! fragment and the end at the cut. The cut rule and the end gate apply
//! to the record: only its last fragment is unwritten when the gate is
//! asked. Each fragment's `tfdt` follows on from the previous one's, and
//! the `mfhd` sequence numbers count fragments over the segmenter's life.
//!
//! # Bounds
//!
//! A segment's held samples are part of its record, so they stay within
//! [`CutRule::max_record_bytes`]. A segment has at most [`MAX_TRACKS`]
//! tracks. The buffers are reused from one segment to the next.
//! [`Segmenter::take_segment`] holds the [`MAX_PENDING_SEGMENTS`] latest
//! segments.

use std::collections::{BTreeMap, VecDeque};

use strims_base::time::Duration;

use crate::fmp4::{
    FREE_HEADER_LEN, FragmentWriter, InitError, MAX_TRACKS, Written, write_free, write_init,
};
use crate::media::{AccessUnit, DecoderConfig, Muxer, TrackId, TrackKind};
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

/// When the segmenter ends a segment, and how it writes the fragments
/// within one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutRule {
    /// The least sample payload a segment holds before a keyframe can end
    /// it.
    pub min_media_bytes: usize,
    /// The longest record, prefix, init and padding included.
    pub max_record_bytes: usize,
    /// The media a fragment holds before it is written: its timed samples
    /// of some track last at least this long ([`FragmentWriter`]'s span).
    /// Zero writes a fragment per unit; [`Duration::MAX`] one per record,
    /// at the cut.
    pub fragment_duration: Duration,
}

impl CutRule {
    pub const DEFAULT_MIN_MEDIA_BYTES: usize = 32 * 1024;
    /// Half the ingress swarm writer's 16 MiB window.
    pub const DEFAULT_MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
    /// A few video frames and their audio a fragment. A packed frame is
    /// output once it fills, about 100 ms of a 5 Mbit/s stream and longer
    /// below, so smaller fragments reach the swarm little sooner and
    /// cost more bytes.
    pub const DEFAULT_FRAGMENT_DURATION: Duration = Duration::from_millis(100);
}

impl Default for CutRule {
    fn default() -> Self {
        CutRule {
            min_media_bytes: Self::DEFAULT_MIN_MEDIA_BYTES,
            max_record_bytes: Self::DEFAULT_MAX_RECORD_BYTES,
            fragment_duration: Self::DEFAULT_FRAGMENT_DURATION,
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
    /// Bytes of `free` padding after the last `mdat`.
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

/// Cuts access units into segments, one record each (see the
/// [module docs](self)).
#[derive(Debug)]
pub struct Segmenter<S: RecordSink> {
    rule: CutRule,
    sink: S,
    /// The current configs, at most [`MAX_TRACKS`].
    configs: BTreeMap<TrackId, DecoderConfig>,
    /// The configs the segment in progress (or the last one) declares, in
    /// init order; the writer's tracks.
    segment_configs: Vec<DecoderConfig>,
    /// Per segment config: the current config differs from it.
    stale: [bool; MAX_TRACKS],
    /// The init of `segment_configs`.
    init: Vec<u8>,
    /// Reused for inits being checked, fragments and padding.
    scratch: Vec<u8>,
    writer: FragmentWriter,
    started: bool,
    /// The configs differ from the segment's: the next keyframe cuts.
    changed: bool,
    /// The segment in progress has another init than the last record.
    new_init: bool,
    /// Bytes of the record in progress written to the sink.
    written: usize,
    /// Bytes of the segment's sample payload.
    media_bytes: usize,
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
            stale: [false; MAX_TRACKS],
            init: Vec::new(),
            scratch: Vec::new(),
            writer: FragmentWriter::new(rule.fragment_duration),
            started: false,
            changed: false,
            new_init: false,
            written: 0,
            media_bytes: 0,
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

    /// The length of the record in progress if it ended now.
    fn record_len(&self) -> usize {
        self.written + self.writer.flush_len()
    }

    /// Marks the segment's tracks whose configs changed, and whether any
    /// config differs from the segment's.
    fn update_changed(&mut self) {
        for (config, stale) in self.segment_configs.iter().zip(self.stale.iter_mut()) {
            *stale = self.configs.get(&config.track) != Some(config);
        }
        self.changed =
            self.stale.iter().any(|s| *s) || self.configs.len() != self.segment_configs.len();
    }

    /// Writes `self.scratch` to the sink, counting it in the record.
    fn write_scratch(&mut self) {
        self.sink.write(&self.scratch);
        self.written += self.scratch.len();
    }

    /// Starts a segment at `keyframe`: declares the current configs and
    /// writes the record's prefix and init.
    fn start_segment(&mut self, keyframe: &AccessUnit<'_>) -> Result<(), SegmentError> {
        let same = self.configs.len() == self.segment_configs.len()
            && ordered(&self.configs)
                .zip(&self.segment_configs)
                .all(|(a, b)| a == b);
        if same {
            self.new_init = false;
        } else {
            let configs: Vec<DecoderConfig> = ordered(&self.configs).cloned().collect();
            let delay = composition_delay(keyframe, &self.configs);
            self.scratch.clear();
            write_init(&configs, delay, &mut self.scratch)?;
            if self.scratch.len() > MAX_INIT_LEN {
                return Err(SegmentError::InitTooLong);
            }
            self.writer.set_tracks(&configs, delay)?;
            self.new_init = self.scratch != self.init;
            std::mem::swap(&mut self.init, &mut self.scratch);
            self.segment_configs = configs;
        }
        self.stale = [false; MAX_TRACKS];
        self.changed = false;
        // The length fits: inits are checked against MAX_INIT_LEN.
        let len = u16::try_from(self.init.len()).unwrap_or(u16::MAX);
        self.sink.write(&len.to_be_bytes());
        self.sink.write(&self.init);
        self.written = PREFIX_LEN + self.init.len();
        self.media_bytes = 0;
        self.started = true;
        Ok(())
    }

    /// Adds a unit to the segment in progress, writing the fragment then
    /// due; drops it when its track's config isn't the segment's.
    fn add(&mut self, unit: &AccessUnit<'_>) -> Result<(), SegmentError> {
        let declared = self
            .segment_configs
            .iter()
            .zip(&self.stale)
            .any(|(c, stale)| c.track == unit.track && !stale);
        if !declared {
            self.metrics.dropped_pending_config += 1;
            return Ok(());
        }
        let size = unit.payload.len();
        let grow = self.writer.max_push_growth(unit.track, size);
        let limit = self.rule.max_record_bytes;
        if self.record_len().saturating_add(grow) > limit {
            return self.fail(SegmentError::RecordTooLarge { limit });
        }
        self.scratch.clear();
        if self.writer.push(unit, &mut self.scratch).is_err() {
            return Err(SegmentError::UnknownTrack(unit.track.0));
        }
        self.write_scratch();
        self.media_bytes += size;
        Ok(())
    }

    /// Ends the segment in progress before a keyframe of the cut track at
    /// `cut_dts`, or at the end of the stream when `None`. A blocked end
    /// is padded when `forced`, and otherwise leaves the segment going:
    /// returns whether the record ended.
    fn cut(&mut self, cut_dts: Option<i64>, forced: bool) -> Result<bool, SegmentError> {
        let len = self.writer.flush_len();
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
        let next = cut_dts.and_then(|dts| self.cut_track().map(|t| (t, dts)));
        self.write_record(next, padding)?;
        Ok(true)
    }

    /// Writes the segment's last fragment, its last samples timed by
    /// `next`, and `padding` bytes of `free`, and ends the record.
    fn write_record(
        &mut self,
        next: Option<(TrackId, i64)>,
        padding: usize,
    ) -> Result<(), SegmentError> {
        self.scratch.clear();
        self.writer.flush(next, &mut self.scratch);
        if padding > 0 {
            write_free(padding, &mut self.scratch);
        }
        self.write_scratch();
        let info = segment_info(&self.writer.take_written());
        if self.sink.end_record().is_err() {
            return self.fail(SegmentError::EndRefused);
        }
        let bytes = self.written;
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
        self.started = false;
        Ok(())
    }
}

impl<S: RecordSink> Muxer for Segmenter<S> {
    type Error = SegmentError;

    /// Sets a track's config. Refused, changing nothing, when the configs
    /// can't be declared together ([`fmp4::check_configs`]) or their init,
    /// with the longest edit lists, would be longer than [`MAX_INIT_LEN`].
    ///
    /// [`fmp4::check_configs`]: crate::fmp4::check_configs
    fn set_config(&mut self, config: &DecoderConfig) -> Result<(), SegmentError> {
        self.check_open()?;
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
        self.start_segment(unit)?;
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
pub(crate) fn ordered(
    configs: &BTreeMap<TrackId, DecoderConfig>,
) -> impl Iterator<Item = &DecoderConfig> + Clone {
    let of = move |kind| configs.values().filter(move |c| c.kind == kind);
    of(TrackKind::Video).chain(of(TrackKind::Audio))
}

/// The composition delay a segment starting at `keyframe` declares: its
/// PTS less DTS in its track's timescale, zero when not positive.
pub(crate) fn composition_delay(
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

/// The counts and media time of a segment's written samples: its first
/// video track's time, or its longest track's.
fn segment_info(written: &[Written]) -> SegmentInfo {
    let time = |w: &Written| {
        if w.timescale == 0 {
            return Duration::ZERO;
        }
        let nanos = i128::from(w.units) * 1_000_000_000 / i128::from(w.timescale);
        Duration::from_nanos(i64::try_from(nanos).unwrap_or(i64::MAX))
    };
    let count = |kind| {
        let n: u64 = written
            .iter()
            .filter(|w| w.kind == Some(kind))
            .map(|w| w.samples)
            .sum();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let duration = match written.iter().find(|w| w.kind == Some(TrackKind::Video)) {
        Some(w) => time(w),
        None => written.iter().map(time).max().unwrap_or(Duration::ZERO),
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
