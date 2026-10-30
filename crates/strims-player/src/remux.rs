// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`Remuxer`]: a unit-format stream's records in, the fMP4 a
//! SourceBuffer appends out.

use std::collections::VecDeque;
use std::ops::Range;

use strims_base::time::{Duration, Time};
use strims_video::fmp4::{FragmentWriter, MAX_TRACKS, write_init};
use strims_video::units::{MAX_RECORD_LEN, ReaderEvent, UnitReader, UnitReaderMetrics};
use strims_video::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

use crate::mime::mse_type;

/// The most payload bytes of units held unwritten. A unit that would pass
/// it first has every held unit written out.
pub const MAX_HELD_BYTES: usize = MAX_RECORD_LEN;
/// The most units held unwritten, bounded as [`MAX_HELD_BYTES`].
pub const MAX_HELD_UNITS: usize = 4096;

/// The frame rate a video unit lasts at when its track's units have
/// given it no duration: [`FragmentWriter`]'s.
const FALLBACK_FRAME_RATE: u32 = 30;
const NANOS_PER_SEC: i128 = 1_000_000_000;

/// A unit a fragment holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitInfo {
    pub track: TrackId,
    pub kind: TrackKind,
    pub keyframe: bool,
    /// When it presents on the SourceBuffer's timeline: its PTS moved by
    /// the remuxer's offset, less its init's composition delay (the edit
    /// list, which browsers apply).
    pub pts: Duration,
    /// Its publish time on the writer's Unix clock.
    pub published: Time,
}

/// What a [`Remuxer`] gives the SourceBuffer, in the order it is appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output<'a> {
    /// An init segment (`ftyp`, `moov`) declaring `tracks` in its order,
    /// for the media after it.
    Init {
        data: &'a [u8],
        /// The SourceBuffer's type ([`mse_type`]).
        mime: &'a str,
        tracks: &'a [DecoderConfig],
        /// The SourceBuffer of the previous init takes it: the same number
        /// of tracks of each kind, with the same ids for a kind with more
        /// than one. False for the first init.
        compatible: bool,
    },
    /// One fragment (`prft`, `moof`, `mdat`) and the units it holds, per
    /// track in the init's order, each track's in decode order.
    Media {
        data: &'a [u8],
        units: &'a [UnitInfo],
    },
    /// `missing` units of `track` never arrived, by sequence numbers. The
    /// track's unit before them lasts until its unit after them.
    Gap {
        track: TrackId,
        kind: TrackKind,
        missing: u64,
    },
    /// The input skipped: the record in progress was dropped.
    Discontinuity,
    /// The writer's session started over at earlier timestamps: units from
    /// here on are moved by `offset` (the whole offset, not its change),
    /// so they follow the output so far.
    Restart { offset: Duration },
}

/// What a [`Remuxer`] has read, written and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RemuxerMetrics {
    /// Messages ([`Remuxer::message`]) and the bytes of the records' data.
    pub messages: u64,
    pub bytes: u64,
    /// Records read whole, dropped for passing [`MAX_RECORD_LEN`], and
    /// dropped unfinished at a discontinuity.
    pub records: u64,
    pub oversize_records: u64,
    pub partial_records: u64,
    pub discontinuities: u64,
    /// Units given to the fragment writer.
    pub units: u64,
    /// Units dropped before the first init, while their track's config
    /// waits for its init, and for going back within an epoch.
    pub dropped_before_init: u64,
    pub dropped_pending_config: u64,
    pub dropped_backwards: u64,
    /// Configs no init segment could declare.
    pub refused_inits: u64,
    /// Init segments and fragments output, and their bytes.
    pub inits: u64,
    pub fragments: u64,
    pub output_bytes: u64,
    /// Epochs started at a writer's restart.
    pub restarts: u64,
    /// Times the held bounds wrote every held unit out.
    pub held_flushes: u64,
    /// Payload bytes held unwritten now.
    pub held_bytes: u64,
    pub reader: UnitReaderMetrics,
}

/// An output waiting to be taken: its bytes and units as ranges of the
/// remuxer's output buffers.
#[derive(Debug)]
enum Queued {
    Init {
        bytes: Range<usize>,
        mime: String,
        tracks: Vec<DecoderConfig>,
        compatible: bool,
    },
    Media {
        bytes: Range<usize>,
        units: Range<usize>,
    },
    Gap {
        track: TrackId,
        kind: TrackKind,
        missing: u64,
    },
    Discontinuity,
    Restart {
        offset: Duration,
    },
}

/// The init the fragments follow.
#[derive(Debug)]
struct CurrentInit {
    /// In init order.
    configs: Vec<DecoderConfig>,
    bytes: Vec<u8>,
}

/// A declared track's place on the output timeline.
#[derive(Debug, Clone, Copy)]
struct TrackOut {
    track: TrackId,
    timescale: u32,
    /// Audio samples per frame; 0 for video.
    frame_samples: u32,
    /// The init's edit for the track: the composition delay in its
    /// timescale, as the fragment writer takes it.
    edit_units: i64,
    /// The output decode time of the track's last unit, and its distance
    /// from the one before.
    last_dts: Option<i64>,
    last_delta: Option<i64>,
}

impl TrackOut {
    fn new(config: &DecoderConfig, delay: Duration, old: &[TrackOut]) -> TrackOut {
        let kept = old
            .iter()
            .find(|o| o.track == config.track && o.timescale == config.timescale);
        TrackOut {
            track: config.track,
            timescale: config.timescale,
            frame_samples: match config.params {
                CodecParams::Aac { frame_samples, .. } => frame_samples,
                _ => 0,
            },
            edit_units: if delay > Duration::ZERO {
                delay_units(delay, config.timescale)
            } else {
                0
            },
            last_dts: kept.and_then(|o| o.last_dts),
            last_delta: kept.and_then(|o| o.last_delta),
        }
    }

    /// Where the track's last unit ends: its decode time plus the distance
    /// from the one before, or an audio frame, or 1/30 s.
    fn end(&self) -> Option<Duration> {
        let last = self.last_dts?;
        let fallback = if self.frame_samples > 0 {
            i64::from(self.frame_samples)
        } else {
            i64::from((self.timescale / FALLBACK_FRAME_RATE).max(1))
        };
        let lasts = self.last_delta.filter(|d| *d > 0).unwrap_or(fallback);
        Some(to_duration(last.saturating_add(lasts), self.timescale))
    }
}

/// Remuxes one unit-format stream ([`strims_video::units`]) into the fMP4
/// a MediaSource SourceBuffer appends: an init segment per set of decoder
/// configs, then a fragment per unit, with the events a player's playout
/// logic needs (see the [crate docs](crate)).
///
/// It takes the stream as the Egress RPC's `Data` messages carry it
/// ([`message`](Self::message)), or as its pieces ([`data`](Self::data),
/// [`record_end`](Self::record_end), [`discontinuity`](Self::discontinuity)),
/// and queues its outputs, which [`poll_output`](Self::poll_output) takes
/// in order.
///
/// # Bounds
///
/// A record is at most [`MAX_RECORD_LEN`] bytes; a longer one is dropped
/// as lost. The units held unwritten are bounded by [`MAX_HELD_BYTES`]
/// and [`MAX_HELD_UNITS`]. Outputs hold their bytes until taken: the
/// buffers are reused once every output was taken, so a caller that
/// takes them after each input holds at most one input's outputs. The
/// reader and writer hold at most [`MAX_TRACKS`] tracks. No input panics.
#[derive(Debug)]
pub struct Remuxer {
    /// The record in progress. Bound: `MAX_RECORD_LEN`.
    record: Vec<u8>,
    /// The record in progress passed the bound: its bytes are dropped
    /// until its end.
    oversize: bool,
    reader: UnitReader,
    writer: FragmentWriter,
    init: Option<CurrentInit>,
    /// The reader's configs differ from the init's: a new init is due at
    /// the cut track's next keyframe.
    pending: bool,
    /// A CONFIGS record changed the reader's tracks since the epoch began,
    /// so a unit going back starts a new one.
    configs_changed: bool,
    /// What every unit's timestamps are moved by.
    offset: Duration,
    /// The init's tracks, in its order. Bound: `MAX_TRACKS`.
    outs: Vec<TrackOut>,
    /// The units the writer holds, in push order. Bound: `MAX_HELD_UNITS`.
    held: Vec<UnitInfo>,
    /// Outputs not yet taken, and the one taken last.
    queue: VecDeque<Queued>,
    taken: Option<Queued>,
    /// The queued outputs' bytes and units.
    bytes: Vec<u8>,
    units: Vec<UnitInfo>,
    metrics: RemuxerMetrics,
}

impl Default for Remuxer {
    fn default() -> Self {
        Remuxer::new()
    }
}

impl Remuxer {
    pub fn new() -> Remuxer {
        Remuxer {
            record: Vec::new(),
            oversize: false,
            reader: UnitReader::new(),
            writer: FragmentWriter::new(Duration::ZERO),
            init: None,
            pending: true,
            configs_changed: false,
            offset: Duration::ZERO,
            outs: Vec::new(),
            held: Vec::new(),
            queue: VecDeque::new(),
            taken: None,
            bytes: Vec::new(),
            units: Vec::new(),
            metrics: RemuxerMetrics::default(),
        }
    }

    pub fn metrics(&self) -> RemuxerMetrics {
        RemuxerMetrics {
            held_bytes: self.writer.held_bytes() as u64,
            reader: self.reader.metrics(),
            ..self.metrics
        }
    }

    /// Takes one Egress `Data` message: a discontinuity before its bytes
    /// when it has `discontinuity`, then its bytes, then the end of the
    /// record when it has `segment_end`.
    pub fn message(&mut self, data: &[u8], segment_end: bool, discontinuity: bool) {
        self.metrics.messages += 1;
        if discontinuity {
            self.discontinuity();
        }
        self.data(data);
        if segment_end {
            self.record_end();
        }
    }

    /// Takes bytes of the record in progress.
    pub fn data(&mut self, bytes: &[u8]) {
        self.reclaim();
        self.metrics.bytes = self.metrics.bytes.saturating_add(bytes.len() as u64);
        if self.oversize || bytes.is_empty() {
            return;
        }
        if self.record.len().saturating_add(bytes.len()) > MAX_RECORD_LEN {
            self.record.clear();
            self.oversize = true;
            self.metrics.oversize_records += 1;
            self.reader.lost();
            return;
        }
        self.record.extend_from_slice(bytes);
    }

    /// The record in progress ended: its bytes since the last end or
    /// discontinuity are one whole record.
    pub fn record_end(&mut self) {
        self.reclaim();
        if std::mem::take(&mut self.oversize) {
            return;
        }
        let record = std::mem::take(&mut self.record);
        self.read(&record);
        self.record = record;
        self.record.clear();
    }

    /// The stream skipped: the record in progress is dropped, and the next
    /// bytes begin a whole record. Nothing held is written out: each
    /// track's last unit lasts until its next one, so the picture holds
    /// and the timeline has no hole.
    pub fn discontinuity(&mut self) {
        self.reclaim();
        if !self.record.is_empty() {
            self.metrics.partial_records += 1;
        }
        self.record.clear();
        self.oversize = false;
        self.reader.lost();
        self.metrics.discontinuities += 1;
        self.queue.push_back(Queued::Discontinuity);
    }

    /// The stream ended: writes out the units held, each track's last
    /// lasting as [`FragmentWriter::flush`] times it without a next unit.
    pub fn finish(&mut self) {
        self.reclaim();
        self.flush(None);
    }

    /// The next output, in append order.
    pub fn poll_output(&mut self) -> Option<Output<'_>> {
        self.taken = Some(self.queue.pop_front()?);
        let bytes = |r: &Range<usize>| self.bytes.get(r.clone()).unwrap_or_default();
        Some(match self.taken.as_ref()? {
            Queued::Init {
                bytes: b,
                mime,
                tracks,
                compatible,
            } => Output::Init {
                data: bytes(b),
                mime,
                tracks,
                compatible: *compatible,
            },
            Queued::Media { bytes: b, units } => Output::Media {
                data: bytes(b),
                units: self.units.get(units.clone()).unwrap_or_default(),
            },
            Queued::Gap {
                track,
                kind,
                missing,
            } => Output::Gap {
                track: *track,
                kind: *kind,
                missing: *missing,
            },
            Queued::Discontinuity => Output::Discontinuity,
            Queued::Restart { offset } => Output::Restart { offset: *offset },
        })
    }

    /// Reuses the output buffers once every output was taken.
    fn reclaim(&mut self) {
        if self.queue.is_empty() {
            self.taken = None;
            self.bytes.clear();
            self.units.clear();
        }
    }

    fn read(&mut self, record: &[u8]) {
        self.metrics.records += 1;
        let unit = self.reader.read(record);
        while let Some(event) = self.reader.poll_event() {
            match event {
                ReaderEvent::Configs => {
                    self.configs_changed = true;
                    let configs = ordered(self.reader.configs());
                    self.pending = self.init.as_ref().is_none_or(|i| i.configs != configs);
                }
                ReaderEvent::Gap { track, missing } => {
                    let kind = self
                        .reader
                        .configs()
                        .iter()
                        .find(|c| c.track == track)
                        .map_or(TrackKind::Video, |c| c.kind);
                    self.queue.push_back(Queued::Gap {
                        track,
                        kind,
                        missing,
                    });
                }
            }
        }
        if let Some(unit) = unit {
            self.unit(&unit);
        }
    }

    fn unit(&mut self, unit: &AccessUnit<'_>) {
        let Some(config) = self.reader.configs().iter().find(|c| c.track == unit.track) else {
            return;
        };
        let (kind, timescale) = (config.kind, config.timescale);
        let mut dts = unit.dts.saturating_add(to_units(self.offset, timescale));
        let last = self
            .outs
            .iter()
            .find(|o| o.track == unit.track)
            .and_then(|o| o.last_dts);
        if let Some(last) = last
            && dts < last
        {
            // The reader passes a track's units in decode order but for a
            // writer's restart, which comes with a CONFIGS record.
            if !std::mem::take(&mut self.configs_changed) {
                self.metrics.dropped_backwards += 1;
                return;
            }
            self.offset = self.output_end() - to_duration(unit.dts, timescale);
            self.metrics.restarts += 1;
            self.queue.push_back(Queued::Restart {
                offset: self.offset,
            });
            dts = unit
                .dts
                .saturating_add(to_units(self.offset, timescale))
                .max(last);
        }

        let cut = cut_track(self.reader.configs());
        if (self.pending || self.init.is_none()) && unit.keyframe && cut == Some(unit.track) {
            self.start_init(unit, timescale, dts);
        }
        let Some(init) = &self.init else {
            self.metrics.dropped_before_init += 1;
            return;
        };
        let current = self.reader.configs().iter().find(|c| c.track == unit.track);
        if !init
            .configs
            .iter()
            .any(|c| c.track == unit.track && Some(c) == current)
        {
            self.metrics.dropped_pending_config += 1;
            return;
        }

        if self.writer.held_bytes().saturating_add(unit.payload.len()) > MAX_HELD_BYTES
            || self.held.len() >= MAX_HELD_UNITS
        {
            self.metrics.held_flushes += 1;
            self.flush(None);
        }
        let Some(out) = self.outs.iter_mut().find(|o| o.track == unit.track) else {
            return;
        };
        let shift = dts.saturating_sub(unit.dts);
        let pts = unit.pts.saturating_add(shift);
        let moved = AccessUnit { dts, pts, ..*unit };
        let at = self.bytes.len();
        let Ok(n) = self.writer.push(&moved, &mut self.bytes) else {
            return;
        };
        out.last_delta = out.last_dts.map(|last| dts.saturating_sub(last));
        out.last_dts = Some(dts);
        let edit = out.edit_units;
        self.held.push(UnitInfo {
            track: unit.track,
            kind,
            keyframe: unit.keyframe,
            pts: to_duration(pts.saturating_sub(edit), timescale),
            published: unit.published,
        });
        self.metrics.units += 1;
        if n > 0 {
            self.wrote(at, n, true);
        }
    }

    /// Declares the reader's configs from `keyframe` on, a keyframe of the
    /// cut track at output decode time `dts`: writes out the units held,
    /// the last of the cut track timed by `dts`, and queues the init when
    /// it differs from the one before.
    fn start_init(&mut self, keyframe: &AccessUnit<'_>, timescale: u32, dts: i64) {
        let configs = ordered(self.reader.configs());
        let delay = composition_delay(keyframe, timescale);
        let mut bytes = Vec::new();
        if write_init(&configs, delay, &mut bytes).is_err() {
            self.metrics.refused_inits += 1;
            return;
        }
        if self.init.is_some() {
            self.flush(Some((keyframe.track, dts)));
        }
        if self.writer.set_tracks(&configs, delay).is_err() {
            self.metrics.refused_inits += 1;
            return;
        }
        let compatible = self
            .init
            .as_ref()
            .is_some_and(|i| same_layout(&i.configs, &configs));
        if self.init.as_ref().is_none_or(|i| i.bytes != bytes) {
            let at = self.bytes.len();
            self.bytes.extend_from_slice(&bytes);
            self.queue.push_back(Queued::Init {
                bytes: at..self.bytes.len(),
                mime: mse_type(&configs),
                tracks: configs.clone(),
                compatible,
            });
            self.metrics.inits += 1;
            self.metrics.output_bytes += bytes.len() as u64;
        }
        self.outs = configs
            .iter()
            .map(|c| TrackOut::new(c, delay, &self.outs))
            .collect();
        self.init = Some(CurrentInit { configs, bytes });
        self.pending = false;
    }

    /// Writes out every unit held, each track's last timed by `next` (its
    /// track and output decode time) or as the writer's flush times it.
    fn flush(&mut self, next: Option<(TrackId, i64)>) {
        let at = self.bytes.len();
        let n = self.writer.flush(next, &mut self.bytes);
        if n > 0 {
            self.wrote(at, n, false);
        }
        self.held.clear();
    }

    /// Queues the fragment the writer appended at `at`, `n` bytes: the
    /// held units, less each track's last when `keep_last` (a push writes
    /// every timed unit, a flush every held one).
    fn wrote(&mut self, at: usize, n: usize, keep_last: bool) {
        let first = self.units.len();
        let mut lasts = [None; MAX_TRACKS];
        for (out, last) in self.outs.iter().zip(lasts.iter_mut()) {
            let kept = if keep_last {
                self.held.iter().rposition(|u| u.track == out.track)
            } else {
                None
            };
            for (i, u) in self.held.iter().enumerate() {
                if u.track == out.track && Some(i) != kept {
                    self.units.push(*u);
                }
            }
            *last = kept.and_then(|i| self.held.get(i).copied());
        }
        self.held.clear();
        self.held.extend(lasts.iter().flatten());
        self.queue.push_back(Queued::Media {
            bytes: at..at + n,
            units: first..self.units.len(),
        });
        self.metrics.fragments += 1;
        self.metrics.output_bytes += n as u64;
    }

    /// Where the output ends: the latest end of a track's last unit.
    fn output_end(&self) -> Duration {
        self.outs
            .iter()
            .filter_map(TrackOut::end)
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

/// Configs in init order: video, then audio, each in the order given.
fn ordered(configs: &[DecoderConfig]) -> Vec<DecoderConfig> {
    let of = |kind| configs.iter().filter(move |c| c.kind == kind).cloned();
    of(TrackKind::Video).chain(of(TrackKind::Audio)).collect()
}

/// The track whose keyframes start inits: the first video track, or the
/// first audio track when there is no video, of configs in track id
/// order. The unit format's GOPs start at its keyframes.
fn cut_track(configs: &[DecoderConfig]) -> Option<TrackId> {
    configs
        .iter()
        .find(|c| c.kind == TrackKind::Video)
        .or_else(|| configs.iter().find(|c| c.kind == TrackKind::Audio))
        .map(|c| c.track)
}

/// Inits a SourceBuffer takes in turn: as many tracks of each kind, with
/// the same ids for a kind with more than one.
fn same_layout(a: &[DecoderConfig], b: &[DecoderConfig]) -> bool {
    [TrackKind::Video, TrackKind::Audio].iter().all(|&kind| {
        let ids = |c: &[DecoderConfig]| -> Vec<TrackId> {
            c.iter()
                .filter(|c| c.kind == kind)
                .map(|c| c.track)
                .collect()
        };
        let (x, y) = (ids(a), ids(b));
        x.len() == y.len() && (x.len() <= 1 || x == y)
    })
}

/// The composition delay an init starting at `keyframe` declares: its
/// PTS less DTS, zero when not positive, as the segmenter takes it.
fn composition_delay(keyframe: &AccessUnit<'_>, timescale: u32) -> Duration {
    let units = keyframe.pts.saturating_sub(keyframe.dts);
    if units <= 0 || timescale == 0 {
        return Duration::ZERO;
    }
    let nanos = i128::from(units) * NANOS_PER_SEC / i128::from(timescale);
    Duration::from_nanos(i64::try_from(nanos).unwrap_or(i64::MAX))
}

/// `delay` in units of `timescale`, rounded to the nearest: the media time
/// of the edit list `write_init` writes for it.
fn delay_units(delay: Duration, timescale: u32) -> i64 {
    to_units(delay, timescale)
}

/// `d` in units of `timescale`, rounded to the nearest and saturated.
fn to_units(d: Duration, timescale: u32) -> i64 {
    let units = (i128::from(d.as_nanos()) * i128::from(timescale) + NANOS_PER_SEC / 2)
        .div_euclid(NANOS_PER_SEC);
    i64::try_from(units).unwrap_or(if units < 0 { i64::MIN } else { i64::MAX })
}

/// `units` of `timescale` as a duration, saturated; zero for timescale 0.
fn to_duration(units: i64, timescale: u32) -> Duration {
    if timescale == 0 {
        return Duration::ZERO;
    }
    let nanos = i128::from(units) * NANOS_PER_SEC / i128::from(timescale);
    Duration::from_nanos(i64::try_from(nanos).unwrap_or(if nanos < 0 {
        i64::MIN
    } else {
        i64::MAX
    }))
}

// Native only: wasm32 runs the bindings' tests (`tests/wasm.rs`).
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "remux_test.rs"]
mod tests;
