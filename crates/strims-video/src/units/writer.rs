// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The unit-format writer, and when its swarm signs early.

use std::collections::BTreeMap;

use strims_base::time::{Duration, Time};
use strims_chunkstream::chunked::{ChunkedEncoder, ContinueError, EncodeError, FrameSizeError};

use super::record::{ConfigEntry, UnitHeader, write_configs, write_end, write_unit_head};
use super::{AUDIO_SYNC_SPAN, MAX_RECORD_LEN};
use crate::fmp4::{InitError, check_configs};
use crate::media::{
    AccessUnit, CodecParams, DecoderConfig, MAX_CONFIG_LEN, Muxer, TrackId, TrackKind,
};

/// Why a [`UnitWriter`] call failed. A failed call writes nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UnitWriteError {
    #[error("track {0} has no decoder config")]
    UnknownTrack(u32),
    /// The configs can't be declared together.
    #[error("invalid decoder config: {0}")]
    Config(#[from] InitError),
    #[error("a decoder config is longer than {MAX_CONFIG_LEN} bytes")]
    ConfigTooLong,
    /// The unit's record would be over [`MAX_RECORD_LEN`].
    #[error("a {len}-byte access unit is over the {MAX_RECORD_LEN}-byte record bound")]
    RecordTooLarge { len: usize },
    #[error("the stream overflows: {0}")]
    Stream(#[from] EncodeError),
    #[error("can't continue there: {0}")]
    Continue(#[from] ContinueError),
    /// Output not yet taken would land after the jump.
    #[error("the writer holds output not yet taken")]
    OutputPending,
}

/// What a [`UnitWriter`] has written and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitWriterMetrics {
    /// Records written: CONFIGS, UNIT and END.
    pub records: u64,
    pub units: u64,
    /// GOPs started, each with its sync record.
    pub gops: u64,
    /// CONFIGS records written for a changed config, outside a GOP start.
    pub config_changes: u64,
    /// Stream bytes output: records, frame headers and padding.
    pub bytes: u64,
    /// Bytes of frame padding written by [`UnitWriter::end_frame`].
    pub padding: u64,
    /// Units before the first GOP's keyframe.
    pub dropped_before_start: u64,
    /// END records written: sessions finished.
    pub ends: u64,
}

/// A track the writer has a config of.
#[derive(Debug, Clone)]
struct Track {
    config: DecoderConfig,
    version: u32,
    /// The seq of the track's next unit.
    seq: u64,
}

/// The GOP in progress.
#[derive(Debug, Clone, Copy)]
struct Gop {
    /// The stream offset of its sync record.
    offset: u64,
    /// Its keyframe's track and decode time.
    track: TrackId,
    dts: i64,
}

/// Writes access units into a swarm's stream as unit-format records (see
/// the [format](super)), collecting the stream bytes in an output buffer.
///
/// Every byte output is final. The driver writes the output into the
/// swarm and returns the buffer ([`take_output`], [`restore`]). To have the
/// swarm sign its partial integrity unit early, it ends the frame in
/// progress ([`end_frame`]), writes the output, flushes the swarm writer,
/// and continues at the offset the flush answers ([`continue_at`]). A
/// session's deliberate end is [`finish`](Muxer::finish), which writes
/// its END record, signed the same way.
///
/// [`take_output`]: UnitWriter::take_output
/// [`restore`]: UnitWriter::restore
/// [`end_frame`]: UnitWriter::end_frame
/// [`continue_at`]: UnitWriter::continue_at
///
/// # Bounds
///
/// At most [`MAX_TRACKS`](super::MAX_TRACKS) configs
/// ([`check_configs`]), each of at most [`MAX_CONFIG_LEN`] bytes; a record
/// is at most [`MAX_RECORD_LEN`] bytes. The output holds what the driver
/// hasn't taken.
#[derive(Debug)]
pub struct UnitWriter {
    encoder: ChunkedEncoder,
    /// Stream bytes not yet taken.
    out: Vec<u8>,
    /// Bound: `MAX_TRACKS`.
    tracks: BTreeMap<TrackId, Track>,
    /// The tracks' configs, in track id order.
    configs: Vec<DecoderConfig>,
    gop: Option<Gop>,
    /// A config changed since the last CONFIGS record.
    changed: bool,
    /// The records written when the last END was.
    ended_at: u64,
    /// Reused for record heads and CONFIGS records.
    scratch: Vec<u8>,
    metrics: UnitWriterMetrics,
}

impl UnitWriter {
    /// A writer of frames of `frame_size` bytes, the swarm's chunk size, at
    /// stream offset 0.
    pub fn new(frame_size: usize) -> Result<UnitWriter, FrameSizeError> {
        Ok(UnitWriter {
            encoder: ChunkedEncoder::new(frame_size)?,
            out: Vec::new(),
            tracks: BTreeMap::new(),
            configs: Vec::new(),
            gop: None,
            changed: false,
            ended_at: 0,
            scratch: Vec::new(),
            metrics: UnitWriterMetrics::default(),
        })
    }

    /// The frame size.
    pub fn frame_size(&self) -> usize {
        self.encoder.frame_size()
    }

    /// The tracks' configs, in track id order: those of the stream's MIME
    /// type ([`units_mime`](super::units_mime)).
    pub fn configs(&self) -> &[DecoderConfig] {
        &self.configs
    }

    /// The stream offset of the next byte output.
    pub fn offset(&self) -> u64 {
        self.encoder.offset()
    }

    /// The stream bytes not yet taken.
    pub fn output(&self) -> &[u8] {
        &self.out
    }

    /// Takes the stream bytes output so far.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Returns a buffer from [`take_output`](Self::take_output) for reuse.
    /// Its contents are discarded; output written since the take is kept.
    pub fn restore(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        buf.append(&mut self.out);
        self.out = buf;
    }

    pub fn metrics(&self) -> UnitWriterMetrics {
        self.metrics
    }

    /// Ends the frame in progress, padding it to its end in the output, so
    /// the stream output is a whole number of frames.
    pub fn end_frame(&mut self) {
        let at = self.out.len();
        let n = self.encoder.frame_padding();
        self.out.resize(at + n, 0);
        let written = self
            .encoder
            .end_frame(self.out.get_mut(at..).unwrap_or_default())
            .unwrap_or(0);
        self.out.truncate(at + written);
        self.metrics.padding += written as u64;
        self.metrics.bytes += written as u64;
    }

    /// Moves the stream to `offset`, a frame start at or after
    /// [`offset`](Self::offset), leaving the bytes between out of the
    /// stream: where a swarm writer's flush says the next unit starts. The
    /// frame in progress must be ended and the output taken.
    pub fn continue_at(&mut self, offset: u64) -> Result<(), UnitWriteError> {
        if !self.out.is_empty() {
            return Err(UnitWriteError::OutputPending);
        }
        self.encoder.continue_at(offset)?;
        Ok(())
    }

    /// The track whose keyframes start GOPs: the video track with the
    /// lowest id, or the audio track with the lowest id.
    fn cut_track(&self) -> Option<TrackId> {
        let of = |kind| {
            self.configs
                .iter()
                .find(|c| c.kind == kind)
                .map(|c| c.track)
        };
        of(TrackKind::Video).or_else(|| of(TrackKind::Audio))
    }

    /// Whether `unit` of a track of `config` starts a GOP.
    fn starts_gop(&self, unit: &AccessUnit<'_>, config: &DecoderConfig) -> bool {
        if !unit.keyframe || self.cut_track() != Some(unit.track) {
            return false;
        }
        let Some(gop) = self.gop else {
            return true;
        };
        if config.kind == TrackKind::Video || gop.track != unit.track {
            return true;
        }
        let span =
            i128::from(AUDIO_SYNC_SPAN.as_nanos()) * i128::from(config.timescale) / 1_000_000_000;
        i128::from(unit.dts) - i128::from(gop.dts) >= span
    }

    /// Writes the record of `parts` to the output.
    fn encode(&mut self, parts: &[&[u8]]) -> Result<(), UnitWriteError> {
        let payload = parts.iter().map(|p| p.len()).sum::<usize>();
        let len = self
            .encoder
            .encoded_len(payload)
            .ok_or(EncodeError::Overflow)?;
        let at = self.out.len();
        self.out.resize(at + len, 0);
        match self
            .encoder
            .encode(parts, self.out.get_mut(at..).unwrap_or_default())
        {
            Ok(n) => {
                self.out.truncate(at + n);
                self.metrics.records += 1;
                self.metrics.bytes += n as u64;
                Ok(())
            }
            Err(e) => {
                self.out.truncate(at);
                Err(e.into())
            }
        }
    }

    /// Writes a CONFIGS record of every track, `gop_back` from its GOP's
    /// sync record.
    fn write_configs(&mut self, gop_back: u64) -> Result<(), UnitWriteError> {
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        write_configs(
            gop_back,
            self.tracks
                .values()
                .map(|t| ConfigEntry::of(&t.config, t.version, t.seq)),
            &mut scratch,
        );
        let res = self.encode(&[&scratch]);
        self.scratch = scratch;
        res
    }
}

impl Muxer for UnitWriter {
    type Error = UnitWriteError;

    /// Sets a track's config; a changed one gets the next version, and a
    /// CONFIGS record goes before the next unit. Refused, changing
    /// nothing, when the configs can't be declared together
    /// ([`check_configs`]) or the config is over [`MAX_CONFIG_LEN`].
    fn set_config(&mut self, config: &DecoderConfig) -> Result<(), UnitWriteError> {
        let held = self.tracks.get(&config.track);
        if held.is_some_and(|t| t.config == *config) {
            return Ok(());
        }
        let bytes = match &config.params {
            CodecParams::Avc { avcc, .. } => avcc.len(),
            CodecParams::Aac { asc, .. } => asc.len(),
            CodecParams::Opus { head, .. } => head.len(),
        };
        if bytes > MAX_CONFIG_LEN {
            return Err(UnitWriteError::ConfigTooLong);
        }
        let mut configs: Vec<DecoderConfig> = self
            .configs
            .iter()
            .filter(|c| c.track != config.track)
            .cloned()
            .collect();
        configs.push(config.clone());
        configs.sort_by_key(|c| c.track);
        check_configs(&configs)?;
        let (version, seq) = match held {
            Some(t) => (t.version.wrapping_add(1), t.seq),
            None => (0, 0),
        };
        self.tracks.insert(
            config.track,
            Track {
                config: config.clone(),
                version,
                seq,
            },
        );
        self.configs = configs;
        self.changed = true;
        Ok(())
    }

    /// Writes a unit's record, after the sync record when it starts a GOP
    /// or a CONFIGS record when a config changed. Units before the first
    /// GOP are dropped.
    fn push(&mut self, unit: &AccessUnit<'_>) -> Result<(), UnitWriteError> {
        let Some(track) = self.tracks.get(&unit.track) else {
            return Err(UnitWriteError::UnknownTrack(unit.track.0));
        };
        let starts = self.starts_gop(unit, &track.config);
        let header = UnitHeader {
            track: unit.track,
            keyframe: unit.keyframe,
            seq: track.seq,
            config: track.version,
            dts: unit.dts,
            pts: unit.pts,
            published: unit.published,
        };
        let gop = match (starts, self.gop) {
            (true, _) => None,
            (false, Some(gop)) => Some(gop),
            (false, None) => {
                self.metrics.dropped_before_start += 1;
                return Ok(());
            }
        };
        let mut head = std::mem::take(&mut self.scratch);
        head.clear();
        // The head's length doesn't depend on gop_back's value beyond its
        // varint, which is at most MAX_UVARINT_LEN bytes: bound it with
        // the longest.
        write_unit_head(u64::MAX, &header, &mut head);
        let len = head.len().saturating_add(unit.payload.len());
        self.scratch = head;
        if len > MAX_RECORD_LEN {
            return Err(UnitWriteError::RecordTooLarge {
                len: unit.payload.len(),
            });
        }
        let gop = match gop {
            Some(gop) => {
                if self.changed {
                    let back = self.encoder.next_record_offset().saturating_sub(gop.offset);
                    self.write_configs(back)?;
                    self.changed = false;
                    self.metrics.config_changes += 1;
                }
                gop
            }
            None => {
                let offset = self.encoder.next_record_offset();
                self.write_configs(0)?;
                self.changed = false;
                self.metrics.gops += 1;
                let gop = Gop {
                    offset,
                    track: unit.track,
                    dts: unit.dts,
                };
                self.gop = Some(gop);
                gop
            }
        };
        let back = self.encoder.next_record_offset().saturating_sub(gop.offset);
        let mut head = std::mem::take(&mut self.scratch);
        head.clear();
        write_unit_head(back, &header, &mut head);
        let res = self.encode(&[&head, unit.payload]);
        self.scratch = head;
        res?;
        if let Some(t) = self.tracks.get_mut(&unit.track) {
            t.seq = t.seq.wrapping_add(1);
        }
        self.metrics.units += 1;
        Ok(())
    }

    /// Ends the writer's session: writes an END record after the
    /// session's records, and the next unit written starts a GOP, of the
    /// next session. A writer that has written no record since its last
    /// END writes none. The writer holds no units.
    fn finish(&mut self) -> Result<(), UnitWriteError> {
        self.gop = None;
        if self.metrics.records == self.ended_at {
            return Ok(());
        }
        let mut end = std::mem::take(&mut self.scratch);
        end.clear();
        write_end(&mut end);
        let res = self.encode(&[&end]);
        self.scratch = end;
        res?;
        self.metrics.ends += 1;
        self.ended_at = self.metrics.records;
        Ok(())
    }
}

/// When a stream's partial integrity unit is due for signing: an
/// `interval` after its first byte.
///
/// A swarm signs a unit when it fills, so a stream at a bitrate below
/// `unit_bytes / interval` has its partial unit signed at the interval,
/// padded to its end; the padding is sent, so such a stream is sent at
/// about that rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignTimer {
    unit: u64,
    interval: Duration,
    /// The stream offset written up to.
    offset: u64,
    /// When the partial unit's first byte was written.
    since: Option<Time>,
}

impl SignTimer {
    /// A timer for integrity units of `unit_bytes` (at least 1), at stream
    /// offset 0. [`Duration::MAX`] never has a unit signed early.
    pub fn new(unit_bytes: u64, interval: Duration) -> SignTimer {
        SignTimer {
            unit: unit_bytes.max(1),
            interval,
            offset: 0,
            since: None,
        }
    }

    /// The stream was written up to `offset` at `now`.
    pub fn written(&mut self, offset: u64, now: Time) {
        let partial = !offset.is_multiple_of(self.unit);
        let new_unit = offset / self.unit != self.offset / self.unit;
        if !partial {
            self.since = None;
        } else if self.since.is_none() || new_unit {
            self.since = Some(now);
        }
        self.offset = offset;
    }

    /// Whether a unit is partial: written to, and not yet full or signed.
    pub fn is_partial(&self) -> bool {
        self.since.is_some()
    }

    /// When the partial unit is due for signing; `None` without one.
    pub fn due(&self) -> Option<Time> {
        if self.interval == Duration::MAX {
            return None;
        }
        self.since.map(|t| t + self.interval)
    }

    /// The partial unit was signed: the stream goes on at `offset`, where
    /// the next unit starts.
    pub fn flushed(&mut self, offset: u64) {
        self.offset = offset;
        self.since = None;
    }
}

#[cfg(test)]
#[path = "writer_test.rs"]
mod tests;
