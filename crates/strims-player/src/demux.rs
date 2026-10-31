// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`Demuxer`]: a unit-format stream's records in, the decoder configs and
//! access units a WebCodecs player decodes out.

use std::collections::VecDeque;
use std::ops::Range;

use strims_base::time::{Duration, Time};
use strims_video::fmp4::MAX_TRACKS;
use strims_video::text::{CueEvent, CueKind, CueReader, CueReaderMetrics};
use strims_video::units::{MAX_RECORD_LEN, ReaderEvent, UnitReader, UnitReaderMetrics};
use strims_video::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

use crate::webcodecs::opus_packet_samples;

/// The frame rate a video unit lasts at, for the presentation end a
/// restart follows, until its track's units give it a duration.
const FALLBACK_FRAME_RATE: i64 = 30;
const NANOS_PER_SEC: i128 = 1_000_000_000;

/// An access unit as a decoder takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemuxedUnit<'a> {
    pub track: TrackId,
    pub kind: TrackKind,
    /// Decoding can start at it.
    pub keyframe: bool,
    /// Its presentation and decode times on the output timeline: its own,
    /// moved by the demuxer's offset (see [`DemuxOutput::Restart`]).
    pub pts: Duration,
    pub dts: Duration,
    /// How long its payload lasts, where the payload says: an AAC frame's
    /// samples, an Opus packet's from its TOC; zero for video and for an
    /// Opus packet whose TOC is malformed.
    pub duration: Duration,
    /// Its publish time on the writer's Unix clock.
    pub published: Time,
    /// The payload: length-prefixed NAL units for H.264 (as its avcC says),
    /// a raw AAC frame, or an Opus packet.
    pub data: &'a [u8],
}

/// What a [`Demuxer`] gives its decoders, in stream order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemuxOutput<'a> {
    /// The tracks' decoder configs changed: every track's, in track id
    /// order. Units after it decode with them; a video track whose config
    /// changed resumes at a keyframe.
    Configs {
        tracks: &'a [DecoderConfig],
    },
    Unit(DemuxedUnit<'a>),
    /// `missing` units of `track` never arrived, by sequence numbers. A
    /// video track resumes at its next keyframe; an audio track at once,
    /// with a hole in its presentation times.
    Gap {
        track: TrackId,
        kind: TrackKind,
        missing: u64,
    },
    /// The input skipped: the record in progress was dropped.
    Discontinuity,
    /// A writer's new session started its timestamps over, or came after
    /// an end: units from here on are moved by `offset` (the whole offset,
    /// not its change), which puts the session's first unit right after
    /// the latest presentation end of the output so far.
    Restart {
        offset: Duration,
    },
    /// The stream ended: a writer session's END record, or the input's end
    /// ([`Demuxer::finish`]). Every unit before it was output. Units after
    /// it are a later session's, which follows the output so far whatever
    /// its timestamps, with a [`Restart`](Self::Restart) before its first.
    End,
    /// A cue shows from `start` to `end` on the output timeline, as the
    /// units' presentation times are, unless a [`CueEnd`](Self::CueEnd)
    /// of it comes first: a text track's subtitle or a caption of the
    /// video ([`strims_video::text`]).
    Cue {
        /// Unique among the demuxer's cues.
        id: u64,
        kind: CueKind,
        start: Duration,
        end: Duration,
        /// Lines joined by LF.
        text: &'a str,
    },
    /// Cue `id` ends at `end`, sooner than it said.
    CueEnd {
        id: u64,
        end: Duration,
    },
}

/// What a [`Demuxer`] has read and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DemuxerMetrics {
    /// Messages ([`Demuxer::message`]) and the bytes of the records' data.
    pub messages: u64,
    pub bytes: u64,
    /// Records read whole, dropped for passing [`MAX_RECORD_LEN`], and
    /// dropped unfinished at a discontinuity.
    pub records: u64,
    pub oversize_records: u64,
    pub partial_records: u64,
    pub discontinuities: u64,
    /// Units output.
    pub units: u64,
    /// Units dropped for decoding before their track's last within one
    /// writer session.
    pub dropped_backwards: u64,
    /// Writer sessions that started their timestamps over or came after
    /// an end.
    pub restarts: u64,
    /// Ends output: END records and [`Demuxer::finish`].
    pub ends: u64,
    pub reader: UnitReaderMetrics,
    pub cues: CueReaderMetrics,
}

/// An output waiting to be taken; a unit's payload is a range of the
/// demuxer's output buffer.
#[derive(Debug)]
enum Queued {
    Configs(Vec<DecoderConfig>),
    Unit {
        track: TrackId,
        kind: TrackKind,
        keyframe: bool,
        pts: Duration,
        dts: Duration,
        duration: Duration,
        published: Time,
        data: Range<usize>,
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
    End,
    Cue {
        id: u64,
        kind: CueKind,
        start: Duration,
        end: Duration,
        text: String,
    },
    CueEnd {
        id: u64,
        end: Duration,
    },
}

/// A track's place on the output timeline.
#[derive(Debug, Clone, Copy)]
struct TrackOut {
    track: TrackId,
    /// The output decode time of the track's last unit in this session,
    /// and its distance from the one before.
    last_dts: Option<Duration>,
    last_delta: Duration,
    /// Where the track's presentation ends: the latest presentation time
    /// plus that unit's duration.
    end: Duration,
}

/// Demuxes one unit-format stream ([`strims_video::units`]) for decoders
/// that take access units, such as the browsers' WebCodecs: the tracks'
/// decoder configs whenever they change, then each access unit with its
/// timestamps, publish time and payload (see the [crate docs](crate)).
///
/// It takes the stream as the Egress RPC's `Data` messages carry it
/// ([`message`](Self::message)), or as its pieces ([`data`](Self::data),
/// [`record_end`](Self::record_end), [`discontinuity`](Self::discontinuity)),
/// and queues its outputs, which [`poll_output`](Self::poll_output) takes
/// in order. It holds no unit back: a record's outputs are queued once the
/// record ends.
///
/// The reader's rules hold: a video track's units start at a keyframe
/// after its config and after every gap, so a decoder fed these units in
/// order never sees a unit whose references went missing.
///
/// An END record, a writer session's deliberate end, outputs an
/// [`End`](DemuxOutput::End), as [`finish`](Self::finish) does at the end
/// of the input, once for each run of units. The first unit after it, a
/// later session's, starts a session at the latest presentation end of the
/// output so far, whatever its timestamps.
///
/// # Bounds
///
/// A record is at most [`MAX_RECORD_LEN`] bytes; a longer one is dropped
/// as lost. Outputs hold their bytes until taken: the buffer is reused
/// once every output was taken, so a caller that takes them after each
/// input holds at most one record's outputs. The reader holds at most
/// [`MAX_TRACKS`] tracks. No input panics.
#[derive(Debug)]
pub struct Demuxer {
    /// The record in progress. Bound: `MAX_RECORD_LEN`.
    record: Vec<u8>,
    /// The record in progress passed the bound: its bytes are dropped
    /// until its end.
    oversize: bool,
    reader: UnitReader,
    /// A CONFIGS record changed the reader's tracks since the last
    /// session began, so a unit going back starts a new one.
    configs_changed: bool,
    /// An end was output and no unit since.
    ended: bool,
    /// The stream ended after output: the next unit starts a session.
    after_end: bool,
    /// What every unit's timestamps are moved by.
    offset: Duration,
    /// Bound: `MAX_TRACKS`.
    outs: Vec<TrackOut>,
    /// Outputs not yet taken, and the one taken last.
    queue: VecDeque<Queued>,
    taken: Option<Queued>,
    /// The queued units' payloads.
    bytes: Vec<u8>,
    cues: CueReader,
    metrics: DemuxerMetrics,
}

impl Default for Demuxer {
    fn default() -> Self {
        Demuxer::new()
    }
}

impl Demuxer {
    pub fn new() -> Demuxer {
        Demuxer {
            record: Vec::new(),
            oversize: false,
            reader: UnitReader::new(),
            configs_changed: false,
            ended: false,
            after_end: false,
            offset: Duration::ZERO,
            outs: Vec::new(),
            queue: VecDeque::new(),
            taken: None,
            bytes: Vec::new(),
            cues: CueReader::new(),
            metrics: DemuxerMetrics::default(),
        }
    }

    pub fn metrics(&self) -> DemuxerMetrics {
        DemuxerMetrics {
            reader: self.reader.metrics(),
            cues: self.cues.metrics(),
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
    /// bytes begin a whole record.
    pub fn discontinuity(&mut self) {
        self.reclaim();
        if !self.record.is_empty() {
            self.metrics.partial_records += 1;
        }
        self.record.clear();
        self.oversize = false;
        self.reader.lost();
        self.cues.reset();
        self.metrics.discontinuities += 1;
        self.queue.push_back(Queued::Discontinuity);
    }

    /// The input ended: outputs an [`End`](DemuxOutput::End), unless the
    /// last end came after the last unit. Input after it is a later
    /// session's, as after an END record.
    pub fn finish(&mut self) {
        self.reclaim();
        self.end();
    }

    /// The next output, in stream order.
    pub fn poll_output(&mut self) -> Option<DemuxOutput<'_>> {
        self.taken = Some(self.queue.pop_front()?);
        Some(match self.taken.as_ref()? {
            Queued::Configs(tracks) => DemuxOutput::Configs { tracks },
            Queued::Unit {
                track,
                kind,
                keyframe,
                pts,
                dts,
                duration,
                published,
                data,
            } => DemuxOutput::Unit(DemuxedUnit {
                track: *track,
                kind: *kind,
                keyframe: *keyframe,
                pts: *pts,
                dts: *dts,
                duration: *duration,
                published: *published,
                data: self.bytes.get(data.clone()).unwrap_or_default(),
            }),
            Queued::Gap {
                track,
                kind,
                missing,
            } => DemuxOutput::Gap {
                track: *track,
                kind: *kind,
                missing: *missing,
            },
            Queued::Discontinuity => DemuxOutput::Discontinuity,
            Queued::Restart { offset } => DemuxOutput::Restart { offset: *offset },
            Queued::End => DemuxOutput::End,
            Queued::Cue {
                id,
                kind,
                start,
                end,
                text,
            } => DemuxOutput::Cue {
                id: *id,
                kind: *kind,
                start: *start,
                end: *end,
                text,
            },
            Queued::CueEnd { id, end } => DemuxOutput::CueEnd { id: *id, end: *end },
        })
    }

    /// Reuses the output buffer once every output was taken.
    fn reclaim(&mut self) {
        if self.queue.is_empty() {
            self.taken = None;
            self.bytes.clear();
        }
    }

    fn read(&mut self, record: &[u8]) {
        self.metrics.records += 1;
        let unit = self.reader.read(record);
        while let Some(event) = self.reader.poll_event() {
            match event {
                ReaderEvent::Configs => {
                    self.configs_changed = true;
                    self.queue
                        .push_back(Queued::Configs(self.reader.configs().to_vec()));
                    self.cue_configs();
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
                ReaderEvent::TextConfigs => self.cue_configs(),
                ReaderEvent::End => self.end(),
            }
        }
        if let Some(unit) = unit {
            self.unit(&unit);
            // After the unit: a restart it starts moves its cues too.
            self.cues.push(&unit);
            self.take_cues();
        }
    }

    /// The cue reader reads the reader's tracks, text and media.
    fn cue_configs(&mut self) {
        let mut configs = self.reader.configs().to_vec();
        configs.extend_from_slice(self.reader.text_configs());
        self.cues.set_configs(&configs);
    }

    /// Queues the cue reader's events, on the output timeline.
    fn take_cues(&mut self) {
        let offset = self.offset;
        while let Some(e) = self.cues.poll_event() {
            self.queue.push_back(match e {
                CueEvent::Show {
                    id,
                    kind,
                    start,
                    end,
                    text,
                } => Queued::Cue {
                    id,
                    kind,
                    start: start + offset,
                    end: end + offset,
                    text,
                },
                CueEvent::End { id, end } => Queued::CueEnd {
                    id,
                    end: end + offset,
                },
            });
        }
    }

    /// The stream ended: an end output, once for each run of units, and
    /// the next unit, after output, starts a session.
    fn end(&mut self) {
        if !self.ended {
            self.ended = true;
            self.metrics.ends += 1;
            self.queue.push_back(Queued::End);
        }
        self.after_end = !self.outs.is_empty();
        self.cues.reset();
    }

    fn unit(&mut self, unit: &AccessUnit<'_>) {
        let Some(config) = self.reader.configs().iter().find(|c| c.track == unit.track) else {
            return;
        };
        let (kind, timescale) = (config.kind, config.timescale);
        let duration = unit_duration(config, unit.payload);
        let raw_dts = to_duration(unit.dts, timescale);
        let raw_pts = to_duration(unit.pts, timescale);
        let last = self
            .outs
            .iter()
            .find(|o| o.track == unit.track)
            .and_then(|o| o.last_dts);
        let after_end = std::mem::take(&mut self.after_end);
        if after_end || last.is_some_and(|last| raw_dts + self.offset < last) {
            // The reader passes a track's units in decode order but for a
            // writer's restart, which comes with a CONFIGS record. A later
            // session after an end follows the output whatever its
            // timestamps.
            let changed = std::mem::take(&mut self.configs_changed);
            if !changed && !after_end {
                self.metrics.dropped_backwards += 1;
                return;
            }
            let end = self
                .outs
                .iter()
                .map(|o| o.end)
                .max()
                .unwrap_or(Duration::ZERO);
            self.offset = end - raw_pts;
            for o in &mut self.outs {
                o.last_dts = None;
            }
            self.metrics.restarts += 1;
            self.cues.reset();
            self.queue.push_back(Queued::Restart {
                offset: self.offset,
            });
        }
        let (dts, pts) = (raw_dts + self.offset, raw_pts + self.offset);
        let out = match self.outs.iter().position(|o| o.track == unit.track) {
            Some(i) => self.outs.get_mut(i),
            None if self.outs.len() < MAX_TRACKS => {
                self.outs.push(TrackOut {
                    track: unit.track,
                    last_dts: None,
                    last_delta: Duration::ZERO,
                    end: Duration::ZERO,
                });
                self.outs.last_mut()
            }
            None => None,
        };
        if let Some(out) = out {
            if let Some(last) = out.last_dts {
                out.last_delta = dts - last;
            }
            out.last_dts = Some(dts);
            let lasts = if duration > Duration::ZERO {
                duration
            } else if out.last_delta > Duration::ZERO {
                out.last_delta
            } else {
                Duration::from_nanos(1_000_000_000 / FALLBACK_FRAME_RATE)
            };
            out.end = out.end.max(pts + lasts);
        }

        let at = self.bytes.len();
        self.bytes.extend_from_slice(unit.payload);
        self.queue.push_back(Queued::Unit {
            track: unit.track,
            kind,
            keyframe: unit.keyframe,
            pts,
            dts,
            duration,
            published: unit.published,
            data: at..self.bytes.len(),
        });
        self.ended = false;
        self.metrics.units += 1;
    }
}

/// How long a unit's payload lasts, where its codec says.
fn unit_duration(config: &DecoderConfig, payload: &[u8]) -> Duration {
    match &config.params {
        CodecParams::Aac { frame_samples, .. } => {
            to_duration(i64::from(*frame_samples), config.timescale)
        }
        CodecParams::Opus { .. } => to_duration(
            i64::from(opus_packet_samples(payload).unwrap_or(0)),
            strims_video::opus::SAMPLE_RATE,
        ),
        _ => Duration::ZERO,
    }
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
#[path = "demux_test.rs"]
mod tests;
