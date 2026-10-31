// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`FragmentWriter`]: access units in, the fragments that follow an init
//! segment out.

use strims_base::time::{Duration, Time};

use crate::fmp4::fragment::{
    EMPTY_FRAGMENT_LEN, PRFT_LEN, ProducerTime, Sample, TRAF_FIXED_LEN, TrackRun, entry_len,
    traf_len, write_fragment, write_prft,
};
use crate::fmp4::init::{InitError, MAX_TRACKS, check_configs, delay_units};
use crate::media::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

/// The frame rate of a video sample's duration when nothing times it.
const FALLBACK_FRAME_RATE: u32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FragmentError {
    #[error("track {0} is not declared")]
    UnknownTrack(u32),
}

/// A held sample's presentation time and publish time.
#[derive(Debug, Clone, Copy, Default)]
struct Stamp {
    pts: i64,
    published: Time,
}

#[derive(Debug)]
struct Track {
    track: TrackId,
    kind: TrackKind,
    timescale: u32,
    /// Audio samples per frame; 0 for video, and for Opus, whose packets
    /// last from 2.5 to 120 ms.
    frame_samples: u32,
    /// The latest duration timed by a following sample, kept across runs.
    prev_duration: Option<u32>,
    /// The media time of the init's edit for the track: the composition
    /// delay in its timescale, 0 without one.
    edit_units: i64,
    /// The held samples, the last untimed.
    samples: Vec<Sample>,
    /// Their payloads back to back.
    data: Vec<u8>,
    /// The DTS of the first held sample and of the last.
    base_dts: i64,
    last_dts: i64,
    first: Stamp,
    last: Stamp,
    /// The durations of the timed samples.
    timed_units: u64,
    /// Samples written and their durations, since the last
    /// [`FragmentWriter::take_written`].
    written_samples: u64,
    written_units: u64,
}

impl Track {
    fn new(config: &DecoderConfig, delay: Duration, reuse: Option<Track>) -> Self {
        let frame_samples = match config.params {
            CodecParams::Aac { frame_samples, .. } => frame_samples,
            CodecParams::Avc { .. } | CodecParams::Opus { .. } => 0,
        };
        let (prev_duration, mut samples, mut data) = match reuse {
            Some(t) if t.timescale == config.timescale => (t.prev_duration, t.samples, t.data),
            Some(t) => (None, t.samples, t.data),
            None => (None, Vec::new(), Vec::new()),
        };
        samples.clear();
        data.clear();
        Track {
            track: config.track,
            kind: config.kind,
            timescale: config.timescale,
            frame_samples,
            prev_duration,
            edit_units: if delay > Duration::ZERO {
                delay_units(delay, config.timescale)
            } else {
                0
            },
            samples,
            data,
            base_dts: 0,
            last_dts: 0,
            first: Stamp::default(),
            last: Stamp::default(),
            timed_units: 0,
            written_samples: 0,
            written_units: 0,
        }
    }

    fn timed(&self) -> usize {
        self.samples.len().saturating_sub(1)
    }

    /// Bytes of the timed samples' payloads.
    fn timed_bytes(&self) -> usize {
        let last = self.samples.last().map_or(0, |s| s.size as usize);
        self.data.len().saturating_sub(last)
    }

    fn fallback_duration(&self) -> u32 {
        self.prev_duration
            .unwrap_or(self.timescale / FALLBACK_FRAME_RATE)
    }

    fn clear(&mut self) {
        self.samples.clear();
        self.data.clear();
        self.timed_units = 0;
    }
}

/// Samples a writer wrote, per track, since the last
/// [`FragmentWriter::take_written`].
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Written {
    pub(crate) kind: Option<TrackKind>,
    pub(crate) timescale: u32,
    pub(crate) samples: u64,
    pub(crate) units: u64,
}

/// Writes access units as the fragments that follow one init segment.
///
/// # Fragments
///
/// A unit is held until the next unit of its track gives its duration;
/// it is then *timed*. A fragment is the timed samples, each track's in a
/// `traf` in the order the tracks are declared, after a `prft` naming the
/// fragment's first sample: the first sample of its first `traf`.
///
/// - A fragment is due once the timed samples of some track last at least
///   the writer's span. A span of zero writes each unit as soon as it is
///   timed; [`Duration::MAX`] writes nothing until [`FragmentWriter::flush`].
/// - A *run* is the fragments after [`FragmentWriter::set_tracks`] or a
///   flush. Its first fragment holds the run's first sample, so a run
///   pushed from a keyframe starts with a fragment a decoder can start
///   at: nothing is due until that sample is timed.
/// - A fragment holding a sample that decodes before the init's edit list
///   starts (a DTS below the composition delay in its track's timescale,
///   so below zero on the presentation timeline) ends right before a
///   keyframe of that track: the samples up to the next keyframe stay in
///   that one fragment. Only a stream whose decode times start near zero
///   has such samples, in its first GOP.
/// - [`FragmentWriter::flush`] writes every held sample as one fragment
///   and ends the run.
/// - `mfhd` numbers the fragments from 1, over the writer's life.
///
/// # Timing
///
/// - A sample lasts from its DTS to the next sample's in its track; a
///   decreasing DTS gives 0. Durations saturate at 32 bits.
/// - At a flush, each track's last sample lasts until the DTS the flush
///   names for its track. Without one, an audio track's lasts its frame's
///   samples, and any other repeats its track's previous duration, or
///   lasts 1/30 s without one.
/// - The composition offset is PTS less DTS, signed and saturated to 32
///   bits. A `traf`'s `tfdt` is its first sample's DTS, 0 when negative,
///   so within a run each track's `tfdt` is the previous fragment's plus
///   its durations.
///
/// # Bounds
///
/// The writer holds what it is pushed until it writes it, and what a
/// track holds grows without bound while no fragment is due: callers
/// bound it with [`FragmentWriter::held_bytes`] or
/// [`FragmentWriter::flush_len`] and flush. Its other state is fixed per
/// track, for at most [`MAX_TRACKS`] tracks, and its buffers are reused.
///
/// # Use
///
/// A player muxing a stream for MediaSource writes an init
/// ([`write_init`](crate::fmp4::write_init)) per set of decoder configs,
/// declares the same configs and delay here, pushes each unit and appends
/// what comes out; before a new init, or after a discontinuity, it
/// flushes.
#[derive(Debug)]
pub struct FragmentWriter {
    span: Duration,
    /// The declared tracks, in init order; at most [`MAX_TRACKS`].
    tracks: Vec<Track>,
    /// The next fragment's `mfhd` sequence number.
    sequence: u32,
    /// The run has written a fragment.
    run_written: bool,
    /// The track of the run's first sample, while the run has written
    /// nothing.
    run_first: Option<usize>,
}

impl FragmentWriter {
    /// A writer with no tracks, writing a fragment once the timed samples
    /// of some track last at least `span`.
    pub fn new(span: Duration) -> Self {
        FragmentWriter {
            span,
            tracks: Vec::new(),
            sequence: 1,
            run_written: false,
            run_first: None,
        }
    }

    /// Declares the tracks of the init segment the next fragments follow,
    /// in that init's order, and the composition delay its edit lists
    /// carry (`write_init`'s), and starts a run. Held samples are dropped:
    /// [`flush`](Self::flush) first to keep them. A track whose id and
    /// timescale stay keeps its previous duration. Refused, changing
    /// nothing, when [`check_configs`] refuses `configs`.
    pub fn set_tracks(
        &mut self,
        configs: &[DecoderConfig],
        composition_delay: Duration,
    ) -> Result<(), InitError> {
        check_configs(configs)?;
        let mut old = std::mem::take(&mut self.tracks);
        self.tracks = configs
            .iter()
            .map(|c| {
                let reuse = old
                    .iter()
                    .position(|t| t.track == c.track)
                    .map(|i| old.swap_remove(i));
                Track::new(c, composition_delay, reuse)
            })
            .collect();
        self.run_written = false;
        self.run_first = None;
        Ok(())
    }

    /// Takes the next unit of a declared track, in decode order: it is
    /// held, and the track's last held sample is timed by its DTS.
    /// Appends the fragment then due to `out`; returns its bytes, 0 when
    /// none is due.
    pub fn push(
        &mut self,
        unit: &AccessUnit<'_>,
        out: &mut Vec<u8>,
    ) -> Result<usize, FragmentError> {
        self.take(unit)?;
        Ok(if self.due() { self.write_timed(out) } else { 0 })
    }

    /// Takes the next unit of a declared track as [`push`](Self::push)
    /// does, appending nothing: a fragment it makes due waits for the next
    /// push or flush, which writes it with what is due then.
    pub fn hold(&mut self, unit: &AccessUnit<'_>) -> Result<(), FragmentError> {
        self.take(unit)
    }

    /// Ends the samples `track` holds at decode time `end`: its last
    /// lasts until `end`, and one fragment of every track's timed samples
    /// and all of this track's is appended to `out`, whether or not the
    /// run's first sample is timed. The track's next unit starts a `traf`
    /// at its own decode time, so the track has a hole from `end` to it.
    /// Returns the fragment's bytes, 0 when the track holds nothing.
    pub fn end_track(
        &mut self,
        track: TrackId,
        end: i64,
        out: &mut Vec<u8>,
    ) -> Result<usize, FragmentError> {
        let Some(i) = self.tracks.iter().position(|t| t.track == track) else {
            return Err(FragmentError::UnknownTrack(track.0));
        };
        let Some(t) = self.tracks.get_mut(i) else {
            return Ok(0);
        };
        let last_dts = t.last_dts;
        let Some(last) = t.samples.last_mut() else {
            return Ok(0);
        };
        let d = duration(end.saturating_sub(last_dts));
        last.duration = d;
        t.prev_duration = Some(d);
        Ok(self.write(out, |n| n == i))
    }

    /// Holds `unit` as the next sample of its track, timing the track's
    /// last held one.
    fn take(&mut self, unit: &AccessUnit<'_>) -> Result<(), FragmentError> {
        let Some((i, t)) = self
            .tracks
            .iter_mut()
            .enumerate()
            .find(|(_, t)| t.track == unit.track)
        else {
            return Err(FragmentError::UnknownTrack(unit.track.0));
        };
        if !self.run_written && self.run_first.is_none() {
            self.run_first = Some(i);
        }
        let stamp = Stamp {
            pts: unit.pts,
            published: unit.published,
        };
        match t.samples.last_mut() {
            Some(last) => {
                let d = duration(unit.dts.saturating_sub(t.last_dts));
                last.duration = d;
                t.prev_duration = Some(d);
                t.timed_units = t.timed_units.saturating_add(u64::from(d));
            }
            None => {
                t.base_dts = unit.dts;
                t.first = stamp;
            }
        }
        let cto = unit.pts.saturating_sub(unit.dts);
        t.samples.push(Sample {
            duration: 0,
            size: u32::try_from(unit.payload.len()).unwrap_or(u32::MAX),
            keyframe: unit.keyframe,
            composition_offset: i32::try_from(cto).unwrap_or(if cto < 0 {
                i32::MIN
            } else {
                i32::MAX
            }),
        });
        t.data.extend_from_slice(unit.payload);
        t.last_dts = unit.dts;
        t.last = stamp;
        Ok(())
    }

    /// Appends one fragment of every held sample, each track's last timed
    /// by `next`, the DTS of its track's next unit (see
    /// [Timing](FragmentWriter#timing)), and ends the run. Returns its bytes,
    /// 0 when nothing is held.
    pub fn flush(&mut self, next: Option<(TrackId, i64)>, out: &mut Vec<u8>) -> usize {
        for t in &mut self.tracks {
            let fallback = t.fallback_duration();
            let Some(last) = t.samples.last_mut() else {
                continue;
            };
            let d = match next {
                Some((track, dts)) if track == t.track => {
                    let d = duration(dts.saturating_sub(t.last_dts));
                    t.prev_duration = Some(d);
                    d
                }
                _ if t.kind == TrackKind::Audio && t.frame_samples > 0 => t.frame_samples,
                _ => fallback,
            };
            last.duration = d;
        }
        let n = self.write(out, |_| true);
        self.run_written = false;
        self.run_first = None;
        n
    }

    /// The bytes [`flush`](Self::flush) would append now.
    pub fn flush_len(&self) -> usize {
        if self.tracks.iter().all(|t| t.samples.is_empty()) {
            return 0;
        }
        PRFT_LEN
            + EMPTY_FRAGMENT_LEN
            + self
                .tracks
                .iter()
                .map(|t| traf_len(t.kind, t.samples.len()) + t.data.len())
                .sum::<usize>()
    }

    /// Bytes of the held samples' payloads.
    pub fn held_bytes(&self) -> usize {
        self.tracks.iter().map(|t| t.data.len()).sum()
    }

    /// The `mfhd` sequence number of the next fragment.
    pub fn next_sequence(&self) -> u32 {
        self.sequence
    }

    /// The most a [`push`](Self::push) of a unit of `track` with
    /// `payload` bytes adds to the bytes appended so far plus
    /// [`flush_len`](Self::flush_len): its sample, a `traf` for it, and
    /// the fragment and `traf`s that writing the timed samples splits off.
    pub(crate) fn max_push_growth(&self, track: TrackId, payload: usize) -> usize {
        let kind = self
            .tracks
            .iter()
            .find(|t| t.track == track)
            .map_or(TrackKind::Video, |t| t.kind);
        payload
            + entry_len(kind)
            + TRAF_FIXED_LEN
            + PRFT_LEN
            + EMPTY_FRAGMENT_LEN
            + TRAF_FIXED_LEN * self.tracks.len()
    }

    /// Per declared track, the samples written and their durations since
    /// the last take; clears the counts.
    pub(crate) fn take_written(&mut self) -> [Written; MAX_TRACKS] {
        let mut out = [Written::default(); MAX_TRACKS];
        for (t, w) in self.tracks.iter_mut().zip(out.iter_mut()) {
            *w = Written {
                kind: Some(t.kind),
                timescale: t.timescale,
                samples: std::mem::take(&mut t.written_samples),
                units: std::mem::take(&mut t.written_units),
            };
        }
        out
    }

    /// A fragment of the timed samples is due: some track's last at least
    /// the span, and they hold the run's first sample.
    fn due(&self) -> bool {
        if self.span == Duration::MAX {
            return false;
        }
        if !self.run_written
            && let Some(first) = self.run_first
            && self.tracks.get(first).is_none_or(|t| t.timed() == 0)
        {
            return false;
        }
        // Samples decoding before the edit end their fragment right before
        // a keyframe of their track: the held last one.
        if self.tracks.iter().any(|t| {
            t.timed() > 0
                && t.base_dts < t.edit_units
                && !t.samples.last().is_some_and(|s| s.keyframe)
        }) {
            return false;
        }
        self.tracks
            .iter()
            .any(|t| t.timed() > 0 && units_to_duration(t.timed_units, t.timescale) >= self.span)
    }

    /// Appends the fragment of the timed samples, keeping each track's
    /// last.
    fn write_timed(&mut self, out: &mut Vec<u8>) -> usize {
        self.write(out, |_| false)
    }

    /// Appends a `prft` and the fragment of the held samples, less the
    /// last of each track whose index `all` refuses, and drops what it
    /// wrote.
    fn write(&mut self, out: &mut Vec<u8>, all: impl Fn(usize) -> bool) -> usize {
        let count = |i: usize, t: &Track| {
            if all(i) { t.samples.len() } else { t.timed() }
        };
        let bytes = |i: usize, t: &Track| {
            if all(i) {
                t.data.len()
            } else {
                t.timed_bytes()
            }
        };
        let Some(lead) = self
            .tracks
            .iter()
            .enumerate()
            .find_map(|(i, t)| (count(i, t) > 0).then_some(t))
        else {
            return 0;
        };
        let start = out.len();
        write_prft(
            &ProducerTime {
                track: lead.track,
                media_time: u64::try_from(lead.first.pts).unwrap_or(0),
                published: lead.first.published,
            },
            out,
        );
        let mut runs = [TrackRun::default(); MAX_TRACKS];
        for (i, (t, run)) in self.tracks.iter().zip(runs.iter_mut()).enumerate() {
            *run = TrackRun {
                track: t.track,
                kind: t.kind,
                base_decode_time: u64::try_from(t.base_dts).unwrap_or(0),
                samples: t.samples.get(..count(i, t)).unwrap_or_default(),
                data: t.data.get(..bytes(i, t)).unwrap_or_default(),
            };
        }
        let runs = runs.get(..self.tracks.len()).unwrap_or_default();
        write_fragment(self.sequence, runs, out);
        self.sequence = self.sequence.checked_add(1).unwrap_or(1);
        self.run_written = true;
        for (i, t) in self.tracks.iter_mut().enumerate() {
            let n = count(i, t);
            if n == 0 {
                continue;
            }
            let units: u64 = t
                .samples
                .get(..n)
                .unwrap_or_default()
                .iter()
                .map(|s| u64::from(s.duration))
                .sum();
            t.written_samples += n as u64;
            t.written_units = t.written_units.saturating_add(units);
            if !all(i) {
                let b = t.timed_bytes();
                t.samples.drain(..n);
                t.data.drain(..b);
                t.base_dts = t.last_dts;
                t.first = t.last;
                t.timed_units = 0;
            } else {
                t.clear();
            }
        }
        out.len() - start
    }
}

/// A sample duration from a DTS difference: 0 when negative, saturated
/// to 32 bits.
fn duration(d: i64) -> u32 {
    u32::try_from(d.max(0)).unwrap_or(u32::MAX)
}

/// `units` of a `timescale` as a duration, saturating; zero for timescale 0.
fn units_to_duration(units: u64, timescale: u32) -> Duration {
    if timescale == 0 {
        return Duration::ZERO;
    }
    let nanos = i128::from(units) * 1_000_000_000 / i128::from(timescale);
    Duration::from_nanos(i64::try_from(nanos).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[path = "writer_test.rs"]
mod tests;
