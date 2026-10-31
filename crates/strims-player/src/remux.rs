// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`Remuxer`]: a unit-format stream's records in, the fMP4 a
//! SourceBuffer appends out.

use std::collections::VecDeque;
use std::ops::Range;

use strims_base::time::{Duration, Time};
use strims_video::fmp4::{FragmentWriter, MAX_TRACKS, write_init};
use strims_video::silence::Silence;
use strims_video::text::{CueEvent, CueKind, CueReader, CueReaderMetrics};
use strims_video::units::{MAX_RECORD_LEN, ReaderEvent, UnitReader, UnitReaderMetrics};
use strims_video::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

use crate::mime::mse_type;
use crate::webcodecs::opus_packet_samples;

/// The most payload bytes of units held unwritten. A unit that would pass
/// it first has every held unit written out.
pub const MAX_HELD_BYTES: usize = MAX_RECORD_LEN;
/// The most units held unwritten, bounded as [`MAX_HELD_BYTES`].
pub const MAX_HELD_UNITS: usize = 4096;
/// The longest hole in an audio track that silence fills, and the most
/// silence one unit writes. A hole filled once the unit after it comes
/// keeps a player going only while it is shorter than the player's lag
/// behind the live edge, at most 3 s (the UI player's greatest target
/// lag); a longer one stalls the player first either way, so it stays a
/// hole, which the player jumps.
pub const MAX_SILENCE: Duration = Duration::from_secs(3);
/// How far a video unit's decode time may pass the end of an audio track
/// before silence follows it, so a track whose audio stops still plays.
/// Sources' video leads their audio by at most about 20 ms in stream
/// order (ffmpeg's FLV muxer by 2 ms, browser captures by 17 ms), and
/// players play at least 0.5 s behind the live edge: silence that far
/// behind the video stays ahead of their playback.
pub const AUDIO_FOLLOW: Duration = Duration::from_millis(200);
/// The farthest behind the video silence follows it. Audio that comes
/// within silence already written, later than the video by more than
/// the distance followed, moves it to that lateness and [`AUDIO_FOLLOW`]
/// more, so a source whose audio lags its video keeps its audio.
pub const MAX_AUDIO_FOLLOW: Duration = Duration::from_secs(1);

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
    /// `missing` units of `track` never arrived, by sequence numbers. A
    /// video track's unit before them lasts until its unit after them; an
    /// audio track's hole is filled with silence (see the
    /// [crate docs](crate)).
    Gap {
        track: TrackId,
        kind: TrackKind,
        missing: u64,
    },
    /// The input skipped: the record in progress was dropped.
    Discontinuity,
    /// The writer's session started over at earlier timestamps, or a
    /// later session came after an end: units from here on are moved by
    /// `offset` (the whole offset, not its change), so they follow the
    /// output so far.
    Restart { offset: Duration },
    /// The stream ended: every unit before it was written out, the cut
    /// track's last lasting to the end of the output. Media after it is a
    /// later writer session's, which follows a restart.
    End,
    /// A cue shows from `start` to `end` on the SourceBuffer's timeline,
    /// as the media's presentation times are, unless a
    /// [`CueEnd`](Self::CueEnd) of it comes first: a text track's
    /// subtitle or a caption of the video ([`strims_video::text`]).
    Cue {
        /// Unique among the remuxer's cues.
        id: u64,
        kind: CueKind,
        start: Duration,
        end: Duration,
        /// Lines joined by LF.
        text: &'a str,
    },
    /// Cue `id` ends at `end`, sooner than it said.
    CueEnd { id: u64, end: Duration },
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
    /// waits for its init, for going back within an epoch, and for
    /// starting within silence already written ahead of them.
    pub dropped_before_init: u64,
    pub dropped_pending_config: u64,
    pub dropped_backwards: u64,
    pub dropped_under_silence: u64,
    /// Configs no init segment could declare.
    pub refused_inits: u64,
    /// Init segments and fragments output, and their bytes.
    pub inits: u64,
    pub fragments: u64,
    pub output_bytes: u64,
    /// Epochs started at a writer's restart or after an end.
    pub restarts: u64,
    /// Ends output: END records and [`Remuxer::finish`].
    pub ends: u64,
    /// Times the held bounds wrote every held unit out.
    pub held_flushes: u64,
    /// Holes in audio tracks filled with silence, and the silent frames
    /// written.
    pub silence_fills: u64,
    pub silence_units: u64,
    /// Holes in audio tracks left past [`MAX_SILENCE`].
    pub audio_holes: u64,
    /// Payload bytes held unwritten now.
    pub held_bytes: u64,
    pub reader: UnitReaderMetrics,
    pub cues: CueReaderMetrics,
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

/// The init the fragments follow.
#[derive(Debug)]
struct CurrentInit {
    /// In init order.
    configs: Vec<DecoderConfig>,
    bytes: Vec<u8>,
}

/// A declared track's place on the output timeline.
#[derive(Debug, Clone)]
struct TrackOut {
    track: TrackId,
    kind: TrackKind,
    timescale: u32,
    /// Audio samples per frame; 0 for video, and for Opus, whose packets
    /// say their own.
    frame_samples: u32,
    /// The init's edit for the track: the composition delay in its
    /// timescale, as the fragment writer takes it.
    edit_units: i64,
    /// The output decode time of the track's last unit, and its distance
    /// from the one before. Silence written isn't a unit.
    last_dts: Option<i64>,
    last_delta: Option<i64>,
    /// An audio track's silent frames; `None` for video and for configs
    /// without them.
    silence: Option<Silence>,
    /// Where an audio track's output ends on the presentation timeline,
    /// its last unit or silence lasting its frame's samples.
    audio_end: Option<Duration>,
    /// The end of silence written ahead of an audio track's units: a unit
    /// starting before it is dropped.
    silent_to: Option<Duration>,
}

impl TrackOut {
    /// The output place of `config` under an init of `delay`, after the
    /// track that held its SourceBuffer track before, if any: the same
    /// track for its timestamps, and an audio track's end and silence
    /// when `carry`.
    fn new(
        config: &DecoderConfig,
        delay: Duration,
        before: Option<&TrackOut>,
        carry: bool,
    ) -> Self {
        let kept = before.filter(|o| o.track == config.track && o.timescale == config.timescale);
        let carried = before.filter(|o| carry && o.kind == TrackKind::Audio);
        TrackOut {
            track: config.track,
            kind: config.kind,
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
            silence: if config.kind == TrackKind::Audio {
                Silence::for_config(config)
            } else {
                None
            },
            audio_end: carried.and_then(|o| o.audio_end),
            silent_to: carried.and_then(|o| o.silent_to),
        }
    }

    /// Where the track's last unit ends: its decode time plus the distance
    /// from the one before, or an audio frame, or 1/30 s; for an audio
    /// track, no earlier than its silence.
    fn end(&self) -> Option<Duration> {
        let filled = self
            .audio_end
            .map(|e| e + to_duration(self.edit_units, self.timescale));
        let Some(last) = self.last_dts else {
            return filled;
        };
        let fallback = if self.frame_samples > 0 {
            i64::from(self.frame_samples)
        } else {
            i64::from((self.timescale / FALLBACK_FRAME_RATE).max(1))
        };
        let lasts = self.last_delta.filter(|d| *d > 0).unwrap_or(fallback);
        let end = to_duration(last.saturating_add(lasts), self.timescale);
        Some(filled.map_or(end, |f| f.max(end)))
    }

    /// The samples an audio unit of `payload` lasts: an AAC frame's, an
    /// Opus packet's, or the distance from the track's unit before.
    fn samples(&self, payload: &[u8]) -> i64 {
        if self.frame_samples > 0 {
            return i64::from(self.frame_samples);
        }
        opus_packet_samples(payload)
            .map(i64::from)
            .or(self.last_delta)
            .unwrap_or(0)
    }

    /// `units` of output time less the init's edit: where a sample of
    /// that time presents, or a sample decoding then would.
    fn presentation(&self, units: i64) -> Duration {
        to_duration(units.saturating_sub(self.edit_units), self.timescale)
    }

    /// The output time of presentation time `at`, in the track's units.
    fn units_at(&self, at: Duration) -> i64 {
        to_units(at, self.timescale).saturating_add(self.edit_units)
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
/// and [`MAX_HELD_UNITS`], and the silence one unit writes by
/// [`MAX_SILENCE`]. Outputs hold their bytes until taken: the
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
    /// An end was output and no unit written since.
    ended: bool,
    /// The stream ended after output: the next unit starts an epoch.
    after_end: bool,
    /// What every unit's timestamps are moved by.
    offset: Duration,
    /// Where the init's cut keyframe decodes on the presentation timeline:
    /// where silence following the video starts for an audio track that
    /// has had no unit.
    run_start: Duration,
    /// How far behind the latest video unit audio tracks follow it, from
    /// [`AUDIO_FOLLOW`] to [`MAX_AUDIO_FOLLOW`], and where that unit
    /// decodes on the presentation timeline.
    follow: Duration,
    video_at: Option<Duration>,
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
    cues: CueReader,
    /// The init's composition delay, which its edit lists move the
    /// presentation back by.
    delay: Duration,
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
            ended: false,
            after_end: false,
            offset: Duration::ZERO,
            run_start: Duration::ZERO,
            follow: AUDIO_FOLLOW,
            video_at: None,
            outs: Vec::new(),
            held: Vec::new(),
            queue: VecDeque::new(),
            taken: None,
            bytes: Vec::new(),
            units: Vec::new(),
            cues: CueReader::new(),
            delay: Duration::ZERO,
            metrics: RemuxerMetrics::default(),
        }
    }

    pub fn metrics(&self) -> RemuxerMetrics {
        RemuxerMetrics {
            held_bytes: self.writer.held_bytes() as u64,
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
    /// bytes begin a whole record. Nothing held is written out: a video
    /// track's last unit lasts until its next one, so the picture holds,
    /// and an audio track's hole is filled with silence.
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

    /// The stream ended: writes out the units held, the cut track's last
    /// lasting to the end of the output and the others' as
    /// [`FragmentWriter::flush`] times them without a next unit, then an
    /// [`Output::End`], unless the last end came after the last unit. An
    /// END record ends the stream alike. Media after it is a later
    /// session's, which starts an epoch at the end of the output.
    pub fn finish(&mut self) {
        self.reclaim();
        self.end();
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
            Queued::End => Output::End,
            Queued::Cue {
                id,
                kind,
                start,
                end,
                text,
            } => Output::Cue {
                id: *id,
                kind: *kind,
                start: *start,
                end: *end,
                text,
            },
            Queued::CueEnd { id, end } => Output::CueEnd { id: *id, end: *end },
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

    /// Queues the cue reader's events, on the SourceBuffer's timeline.
    fn take_cues(&mut self) {
        let at = |t: Duration| t + self.offset - self.delay;
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
                    start: at(start),
                    end: at(end),
                    text,
                },
                CueEvent::End { id, end } => Queued::CueEnd { id, end: at(end) },
            });
        }
    }

    /// The stream ended: writes out the units held, the cut track's last
    /// lasting to the end of the output, where a later session starts, so
    /// the picture has no hole there; then an end.
    fn end(&mut self) {
        let end = self.output_end();
        let next = self
            .outs
            .first()
            .map(|o| (o.track, to_units(end, o.timescale)));
        self.flush(next);
        if !self.ended {
            self.ended = true;
            self.metrics.ends += 1;
            self.queue.push_back(Queued::End);
        }
        self.after_end = self.init.is_some();
        self.cues.reset();
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
        let after_end = std::mem::take(&mut self.after_end);
        if after_end || last.is_some_and(|last| dts < last) {
            // The reader passes a track's units in decode order but for a
            // writer's restart, which comes with a CONFIGS record. A later
            // session after an end follows the output whatever its
            // timestamps.
            let changed = std::mem::take(&mut self.configs_changed);
            if !changed && !after_end {
                self.metrics.dropped_backwards += 1;
                return;
            }
            self.offset = self.output_end() - to_duration(unit.dts, timescale);
            self.metrics.restarts += 1;
            self.cues.reset();
            self.queue.push_back(Queued::Restart {
                offset: self.offset,
            });
            dts = unit.dts.saturating_add(to_units(self.offset, timescale));
            if let Some(last) = last {
                dts = dts.max(last);
            }
        }

        let cut = cut_track(self.reader.configs());
        if (self.pending || self.init.is_none()) && unit.keyframe && cut == Some(unit.track) {
            // No sample may decode before its init's edit list starts:
            // Firefox's MediaSource drops a GOP whose samples that do are
            // split over fragments, up to the next keyframe. A keyframe
            // decoding before its composition delay, as a stream's first
            // does when its decode times start at zero with B-frames,
            // moves the output later by the difference, every track
            // alike, so fragments stay a unit each from it.
            let lead = delay_units(composition_delay(unit, timescale), timescale) - dts;
            if lead > 0 {
                self.offset += to_duration(lead, timescale);
                dts += lead;
            }
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
        let Some(oi) = self.outs.iter().position(|o| o.track == unit.track) else {
            return;
        };
        let shift = dts.saturating_sub(unit.dts);
        let pts = unit.pts.saturating_add(shift);

        // Silence to write: an audio unit fills the hole before it; a
        // video unit has audio tracks follow it, after it so the init's
        // keyframe comes first.
        let mut fills = [None; MAX_TRACKS];
        let mut room = 0;
        if kind == TrackKind::Audio {
            let out = self.outs.get_mut(oi);
            let start = out.as_ref().map(|o| o.presentation(pts));
            if let (Some(out), Some(start)) = (out, start) {
                if out.silent_to.is_some_and(|to| start < to) {
                    self.metrics.dropped_under_silence += 1;
                    if let Some(v) = self.video_at {
                        let late = (v - start + AUDIO_FOLLOW).min(MAX_AUDIO_FOLLOW);
                        self.follow = self.follow.max(late);
                    }
                    return;
                }
                out.silent_to = None;
                if let Some(from) = out.audio_end
                    && let Some(fill) = plan_fill(out, from, start, false, unit.published)
                {
                    room += fill.frames;
                    fills[0] = Some((oi, fill));
                }
            }
        } else if let Some(out) = self.outs.get(oi) {
            let at = out.presentation(dts);
            self.video_at = Some(self.video_at.map_or(at, |v| v.max(at)));
            let follow = self.follow;
            for ((i, o), slot) in self
                .outs
                .iter()
                .enumerate()
                .filter(|(_, o)| o.kind == TrackKind::Audio)
                .zip(fills.iter_mut())
            {
                let from = o.audio_end.unwrap_or(self.run_start);
                let to = at - follow;
                if to > from
                    && let Some(fill) = plan_fill(o, from, to, true, unit.published - follow)
                {
                    room += fill.frames;
                    *slot = Some((i, fill));
                }
            }
        }

        if self.writer.held_bytes().saturating_add(unit.payload.len()) > MAX_HELD_BYTES
            || self.held.len().saturating_add(room) >= MAX_HELD_UNITS
        {
            self.metrics.held_flushes += 1;
            self.flush(None);
        }
        for (i, fill) in fills.iter().flatten().filter(|(_, f)| !f.ahead) {
            self.write_fill(*i, *fill);
        }
        let Some(out) = self.outs.get_mut(oi) else {
            return;
        };
        let moved = AccessUnit { dts, pts, ..*unit };
        let at = self.bytes.len();
        let Ok(n) = self.writer.push(&moved, &mut self.bytes) else {
            return;
        };
        out.last_delta = out.last_dts.map(|last| dts.saturating_sub(last));
        out.last_dts = Some(dts);
        if kind == TrackKind::Audio {
            let samples = out.samples(unit.payload);
            out.audio_end = Some(out.presentation(pts.saturating_add(samples)));
        }
        let edit = out.edit_units;
        self.held.push(UnitInfo {
            track: unit.track,
            kind,
            keyframe: unit.keyframe,
            pts: to_duration(pts.saturating_sub(edit), timescale),
            published: unit.published,
        });
        self.metrics.units += 1;
        self.ended = false;
        if n > 0 {
            self.wrote(at, n, |_| false);
        }
        for (i, fill) in fills.iter().flatten().filter(|(_, f)| f.ahead) {
            self.write_fill(*i, *fill);
        }
    }

    /// Writes the silence `fill` plans for the audio track at `oi`: its
    /// frames held for the next push to write, or, for a hole too long to
    /// fill, the track's held units ended at its end.
    fn write_fill(&mut self, oi: usize, fill: Fill) {
        let Remuxer {
            outs,
            writer,
            held,
            metrics,
            bytes,
            ..
        } = self;
        let Some(out) = outs.get_mut(oi) else {
            return;
        };
        let Some(silence) = out.silence.as_ref() else {
            return;
        };
        let from = fill.from;
        if fill.hole {
            metrics.audio_holes += 1;
            let at = bytes.len();
            let n = writer
                .end_track(out.track, out.units_at(from), bytes)
                .unwrap_or(0);
            out.audio_end = Some(fill.to);
            if fill.ahead {
                out.silent_to = Some(fill.to);
            }
            if n > 0 {
                let track = out.track;
                self.wrote(at, n, |t| t == track);
            }
            return;
        }
        let mut dts = out.units_at(from);
        let mut count = 0;
        for (payload, samples) in silence.cover(to_units(fill.to - from, out.timescale)) {
            let pts = out.presentation(dts);
            let unit = AccessUnit {
                track: out.track,
                dts,
                pts: dts,
                keyframe: true,
                payload,
                published: fill.published - (fill.to - pts),
            };
            if writer.hold(&unit).is_err() {
                return;
            }
            held.push(UnitInfo {
                track: out.track,
                kind: TrackKind::Audio,
                keyframe: true,
                pts,
                published: unit.published,
            });
            dts = dts.saturating_add(i64::from(samples));
            count += 1;
        }
        out.audio_end = Some(out.presentation(dts));
        if fill.ahead {
            out.silent_to = out.audio_end;
        }
        metrics.silence_fills += 1;
        metrics.silence_units += count;
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
        // A SourceBuffer that takes the new init keeps its track buffers:
        // an audio track goes on from where the one before it ended.
        let carry = compatible;
        let sole = |outs: &[TrackOut]| {
            let mut audio = outs.iter().filter(|o| o.kind == TrackKind::Audio);
            audio
                .next()
                .filter(|_| audio.next().is_none())
                .map(|o| o.track)
        };
        let sole_before = sole(&self.outs);
        let sole_after = {
            let mut audio = configs.iter().filter(|c| c.kind == TrackKind::Audio);
            audio
                .next()
                .filter(|_| audio.next().is_none())
                .map(|c| c.track)
        };
        let outs = configs
            .iter()
            .map(|c| {
                let before = self.outs.iter().find(|o| o.track == c.track).or_else(|| {
                    (sole_after == Some(c.track))
                        .then(|| self.outs.iter().find(|o| Some(o.track) == sole_before))
                        .flatten()
                });
                TrackOut::new(c, delay, before, carry)
            })
            .collect();
        self.outs = outs;
        self.run_start = to_duration(dts, timescale) - delay;
        self.delay = delay;
        self.init = Some(CurrentInit { configs, bytes });
        self.pending = false;
    }

    /// Writes out every unit held, each track's last timed by `next` (its
    /// track and output decode time) or as the writer's flush times it.
    fn flush(&mut self, next: Option<(TrackId, i64)>) {
        let at = self.bytes.len();
        let n = self.writer.flush(next, &mut self.bytes);
        if n > 0 {
            self.wrote(at, n, |_| true);
        }
        self.held.clear();
    }

    /// Queues the fragment the writer appended at `at`, `n` bytes: the
    /// held units, less the last of each track `all` refuses (a push
    /// writes every timed unit, a flush every held one, and ending a
    /// track every unit of it and the timed ones of the others).
    fn wrote(&mut self, at: usize, n: usize, all: impl Fn(TrackId) -> bool) {
        let first = self.units.len();
        let mut lasts = [None; MAX_TRACKS];
        for (out, last) in self.outs.iter().zip(lasts.iter_mut()) {
            let kept = if all(out.track) {
                None
            } else {
                self.held.iter().rposition(|u| u.track == out.track)
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

/// Silence to write in an audio track: from its end to a time on the
/// presentation timeline.
#[derive(Debug, Clone, Copy)]
struct Fill {
    from: Duration,
    to: Duration,
    /// The silent frames it writes; none for a hole.
    frames: usize,
    /// Longer than [`MAX_SILENCE`]: the track's units end at `from`, and
    /// its output goes on from `to`.
    hole: bool,
    /// Written ahead of the track's units, which are dropped up to `to`.
    ahead: bool,
    /// The publish time of the output at `to`, which the silence's are
    /// counted back from.
    published: Time,
}

/// The silence that fills `out`, an audio track, from `from` to `to`:
/// `None` when it has no silent frames or the gap rounds to none of them.
fn plan_fill(
    out: &TrackOut,
    from: Duration,
    to: Duration,
    ahead: bool,
    published: Time,
) -> Option<Fill> {
    let silence = out.silence.as_ref()?;
    let gap = to - from;
    let step = to_duration(i64::from(silence.step()), out.timescale);
    if gap + gap < step {
        return None;
    }
    let mut fill = Fill {
        from,
        to,
        frames: 0,
        hole: gap > MAX_SILENCE,
        ahead,
        published,
    };
    if !fill.hole {
        fill.frames = silence.cover(to_units(gap, out.timescale)).count();
    }
    Some(fill)
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
/// PTS less DTS, zero when not positive.
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
