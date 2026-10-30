// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The encoding of unit-format records (see the [format](super#records)).
//!
//! Writers append to caller buffers; [`parse_record`] borrows from the
//! record it reads.

use strims_base::time::Time;
use strims_chunkstream::{MAX_UVARINT_LEN, get_uvarint, put_uvarint, uvarint_len};

use super::{MAX_RECORD_LEN, MAX_TRACKS};
use crate::aac::{AacError, parse_asc};
use crate::avc::{AvcError, parse_avcc};
use crate::media::{CodecParams, DecoderConfig, MAX_CONFIG_LEN, TrackId, TrackKind};
use crate::opus::{self, OpusError, parse_opus_head};

/// The kind of a CONFIGS record.
pub const KIND_CONFIGS: u8 = 1;
/// The kind of a UNIT record.
pub const KIND_UNIT: u8 = 2;
/// A UNIT's flag of a keyframe.
pub const FLAG_KEYFRAME: u8 = 0x01;
/// The sample entry of an H.264 track, whose config is an avcC record.
pub const SAMPLE_ENTRY_AVC: [u8; 4] = *b"avc1";
/// The sample entry of an AAC track, whose config is an
/// AudioSpecificConfig.
pub const SAMPLE_ENTRY_AAC: [u8; 4] = *b"mp4a";
/// The sample entry of an Opus track, whose config is an OpusHead.
pub const SAMPLE_ENTRY_OPUS: [u8; 4] = *b"Opus";
/// The bytes of a record that hold its kind and `gop_back`: a record's
/// first `GOP_HEAD_LEN` bytes, or all of a shorter one, are enough for
/// [`gop_back`].
pub const GOP_HEAD_LEN: usize = 1 + MAX_UVARINT_LEN;

/// Why a record was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RecordError {
    #[error("the record is longer than {MAX_RECORD_LEN} bytes")]
    TooLong,
    #[error("the record has kind 0")]
    KindZero,
    #[error("the record is truncated")]
    Truncated,
    /// A varint doesn't fit its field.
    #[error("{0} is out of range")]
    Range(&'static str),
    #[error("track id 0 is reserved")]
    TrackZero,
    #[error("a CONFIGS record holds no tracks")]
    NoTracks,
    #[error("a CONFIGS record holds more than {MAX_TRACKS} tracks")]
    TooManyTracks,
    #[error("track {0} is in the CONFIGS record twice")]
    DuplicateTrack(u32),
    #[error("track {0} has timescale 0")]
    ZeroTimescale(u32),
    #[error("a config is longer than {MAX_CONFIG_LEN} bytes")]
    ConfigTooLong,
    #[error("track {track}: {error}")]
    Avc { track: u32, error: AvcError },
    #[error("track {track}: {error}")]
    Aac { track: u32, error: AacError },
    #[error("track {track}: {error}")]
    Opus { track: u32, error: OpusError },
    /// An AAC track's timescale isn't its output sample rate, or an Opus
    /// track's isn't 48 kHz.
    #[error("track {0}'s timescale is not its sample rate")]
    Timescale(u32),
}

/// A UNIT record's header: an access unit without its payload, and its
/// place in its track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitHeader {
    pub track: TrackId,
    pub keyframe: bool,
    /// The unit's number in its track.
    pub seq: u64,
    /// The version of the track's config the unit decodes with.
    pub config: u32,
    pub dts: i64,
    pub pts: i64,
    pub published: Time,
}

/// A track's entry in a CONFIGS record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigEntry<'a> {
    pub track: TrackId,
    /// The config's version.
    pub version: u32,
    /// The seq of the track's next unit.
    pub next_seq: u64,
    pub sample_entry: [u8; 4],
    pub timescale: u32,
    pub config: &'a [u8],
}

impl<'a> ConfigEntry<'a> {
    /// The entry of `config`, at `version`, before the unit numbered
    /// `next_seq`.
    pub fn of(config: &'a DecoderConfig, version: u32, next_seq: u64) -> ConfigEntry<'a> {
        let (sample_entry, bytes) = match &config.params {
            CodecParams::Avc { avcc, .. } => (SAMPLE_ENTRY_AVC, avcc.as_slice()),
            CodecParams::Aac { asc, .. } => (SAMPLE_ENTRY_AAC, asc.as_slice()),
            CodecParams::Opus { head, .. } => (SAMPLE_ENTRY_OPUS, head.as_slice()),
        };
        ConfigEntry {
            track: config.track,
            version,
            next_seq,
            sample_entry,
            timescale: config.timescale,
            config: bytes,
        }
    }

    /// Whether the entry's sample entry is one [`decoder_config`] reads.
    ///
    /// [`decoder_config`]: ConfigEntry::decoder_config
    pub fn is_known(&self) -> bool {
        [SAMPLE_ENTRY_AVC, SAMPLE_ENTRY_AAC, SAMPLE_ENTRY_OPUS].contains(&self.sample_entry)
    }

    /// Whether `config` is the decoder config this entry describes,
    /// without parsing the entry's config.
    pub fn describes(&self, config: &DecoderConfig) -> bool {
        let of = ConfigEntry::of(config, self.version, self.next_seq);
        of == *self
    }

    /// The decoder config the entry describes, its codec string and the
    /// properties a container declares derived from its config bytes.
    /// `None` for a sample entry it doesn't read ([`is_known`]).
    ///
    /// [`is_known`]: ConfigEntry::is_known
    pub fn decoder_config(&self) -> Option<Result<DecoderConfig, RecordError>> {
        let track = self.track.0;
        let config = match self.sample_entry {
            SAMPLE_ENTRY_AVC => parse_avcc(self.config)
                .map(|p| DecoderConfig {
                    track: self.track,
                    kind: TrackKind::Video,
                    codec: p.codec(),
                    timescale: self.timescale,
                    params: CodecParams::Avc {
                        avcc: self.config.to_vec(),
                        width: p.sps.width,
                        height: p.sps.height,
                    },
                })
                .map_err(|error| RecordError::Avc { track, error }),
            SAMPLE_ENTRY_AAC => parse_asc(self.config)
                .map_err(|error| RecordError::Aac { track, error })
                .and_then(|p| {
                    if p.sample_rate != self.timescale {
                        return Err(RecordError::Timescale(track));
                    }
                    Ok(DecoderConfig {
                        track: self.track,
                        kind: TrackKind::Audio,
                        codec: p.codec(),
                        timescale: self.timescale,
                        params: CodecParams::Aac {
                            asc: self.config.to_vec(),
                            sample_rate: p.sample_rate,
                            channels: p.channels,
                            frame_samples: p.frame_samples,
                        },
                    })
                }),
            SAMPLE_ENTRY_OPUS => parse_opus_head(self.config)
                .map_err(|error| RecordError::Opus { track, error })
                .and_then(|h| {
                    if self.timescale != opus::SAMPLE_RATE {
                        return Err(RecordError::Timescale(track));
                    }
                    Ok(DecoderConfig {
                        track: self.track,
                        kind: TrackKind::Audio,
                        codec: opus::CODEC.into(),
                        timescale: self.timescale,
                        params: CodecParams::Opus {
                            head: self.config.to_vec(),
                            channels: h.channels,
                            pre_skip: h.pre_skip,
                        },
                    })
                }),
            _ => return None,
        };
        Some(config)
    }
}

/// A record, read by [`parse_record`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Record<'a> {
    /// Every track's decoder config; a GOP's sync record when `gop_back`
    /// is 0.
    Configs {
        gop_back: u64,
        entries: ConfigEntries<'a>,
    },
    /// An access unit.
    Unit {
        gop_back: u64,
        header: UnitHeader,
        payload: &'a [u8],
    },
    /// A record of a kind this reader doesn't know.
    Other { kind: u8, gop_back: u64 },
}

impl Record<'_> {
    /// The record's offset less its GOP's sync record's.
    pub fn gop_back(&self) -> u64 {
        match self {
            Record::Configs { gop_back, .. }
            | Record::Unit { gop_back, .. }
            | Record::Other { gop_back, .. } => *gop_back,
        }
    }
}

/// The entries of a CONFIGS record, in record order. [`parse_record`]
/// checked every one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigEntries<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for ConfigEntries<'a> {
    type Item = ConfigEntry<'a>;

    fn next(&mut self) -> Option<ConfigEntry<'a>> {
        let mut c = Cursor(self.rest);
        let entry = c.entry().ok()?;
        self.rest = c.0;
        Some(entry)
    }
}

/// A read position in a record.
struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn u8(&mut self) -> Result<u8, RecordError> {
        let (&b, rest) = self.0.split_first().ok_or(RecordError::Truncated)?;
        self.0 = rest;
        Ok(b)
    }

    fn uvarint(&mut self) -> Result<u64, RecordError> {
        let (v, n) = get_uvarint(self.0).ok_or(RecordError::Truncated)?;
        self.0 = self.0.get(n..).unwrap_or_default();
        Ok(v)
    }

    fn u32(&mut self, field: &'static str) -> Result<u32, RecordError> {
        u32::try_from(self.uvarint()?).map_err(|_| RecordError::Range(field))
    }

    fn zigzag(&mut self) -> Result<i64, RecordError> {
        Ok(unzigzag(self.uvarint()?))
    }

    /// The next `n` bytes, `n` read as a varint first.
    fn bytes(&mut self) -> Result<&'a [u8], RecordError> {
        let n = usize::try_from(self.uvarint()?).map_err(|_| RecordError::Truncated)?;
        self.take(n)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], RecordError> {
        if n > self.0.len() {
            return Err(RecordError::Truncated);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }

    fn track(&mut self) -> Result<TrackId, RecordError> {
        match self.u32("track")? {
            0 => Err(RecordError::TrackZero),
            t => Ok(TrackId(t)),
        }
    }

    /// A length-prefixed CONFIGS entry.
    fn entry(&mut self) -> Result<ConfigEntry<'a>, RecordError> {
        let mut e = Cursor(self.bytes()?);
        let track = e.track()?;
        let version = e.u32("version")?;
        let next_seq = e.uvarint()?;
        let sample_entry: [u8; 4] = e.take(4)?.try_into().map_err(|_| RecordError::Truncated)?;
        let timescale = e.u32("timescale")?;
        if timescale == 0 {
            return Err(RecordError::ZeroTimescale(track.0));
        }
        let config = e.bytes()?;
        if config.len() > MAX_CONFIG_LEN {
            return Err(RecordError::ConfigTooLong);
        }
        Ok(ConfigEntry {
            track,
            version,
            next_seq,
            sample_entry,
            timescale,
            config,
        })
    }
}

/// `v` zigzag encoded: 0, -1, 1, -2, … as 0, 1, 2, 3, ….
pub fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

/// The inverse of [`zigzag`].
pub fn unzigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

/// Appends `v`'s varint.
fn put(out: &mut Vec<u8>, v: u64) {
    let mut b = [0; MAX_UVARINT_LEN];
    let n = put_uvarint(&mut b, v);
    out.extend_from_slice(b.get(..n).unwrap_or_default());
}

/// Reads a record. A record of an unknown kind reads as [`Record::Other`].
///
/// # Errors
///
/// The record is over [`MAX_RECORD_LEN`], truncated, of kind 0, or a field
/// is out of its range. A CONFIGS record must hold 1 to [`MAX_TRACKS`]
/// well-formed entries of distinct tracks; their configs aren't parsed
/// ([`ConfigEntry::decoder_config`] does).
pub fn parse_record(record: &[u8]) -> Result<Record<'_>, RecordError> {
    if record.len() > MAX_RECORD_LEN {
        return Err(RecordError::TooLong);
    }
    let mut c = Cursor(record);
    let kind = c.u8()?;
    if kind == 0 {
        return Err(RecordError::KindZero);
    }
    let gop_back = c.uvarint()?;
    if ![KIND_CONFIGS, KIND_UNIT].contains(&kind) {
        return Ok(Record::Other { kind, gop_back });
    }
    let mut header = Cursor(c.bytes()?);
    let body = c.0;
    match kind {
        KIND_UNIT => {
            let track = header.track()?;
            let flags = header.u8()?;
            let seq = header.uvarint()?;
            let config = header.u32("config")?;
            let dts = header.zigzag()?;
            let cts = header.zigzag()?;
            let published = Time::from_unix_millis(header.zigzag()?);
            Ok(Record::Unit {
                gop_back,
                header: UnitHeader {
                    track,
                    keyframe: flags & FLAG_KEYFRAME != 0,
                    seq,
                    config,
                    dts,
                    pts: dts.saturating_add(cts),
                    published,
                },
                payload: body,
            })
        }
        _ => {
            let mut tracks = [TrackId(0); MAX_TRACKS];
            let mut n = 0;
            let mut c = Cursor(body);
            while !c.0.is_empty() {
                let e = c.entry()?;
                let seen = tracks.get(..n).unwrap_or_default();
                if seen.contains(&e.track) {
                    return Err(RecordError::DuplicateTrack(e.track.0));
                }
                let slot = tracks.get_mut(n).ok_or(RecordError::TooManyTracks)?;
                *slot = e.track;
                n += 1;
            }
            if n == 0 {
                return Err(RecordError::NoTracks);
            }
            Ok(Record::Configs {
                gop_back,
                entries: ConfigEntries { rest: body },
            })
        }
    }
}

/// The `gop_back` of the record whose first bytes are `head`: at least
/// [`GOP_HEAD_LEN`] bytes, or the whole record. `None` when they don't
/// hold one: kind 0, or a varint that doesn't end within them.
pub fn gop_back(head: &[u8]) -> Option<u64> {
    let mut c = Cursor(head);
    match c.u8() {
        Ok(0) | Err(_) => None,
        Ok(_) => c.uvarint().ok(),
    }
}

/// The longest UNIT header: the track and config as `u32` varints, the
/// flags, and four 64-bit varints.
const MAX_UNIT_HEADER_LEN: usize = 2 * 5 + 1 + 4 * MAX_UVARINT_LEN;

/// Appends a UNIT record's bytes up to its payload: its kind,
/// `gop_back` and header.
pub fn write_unit_head(gop_back: u64, h: &UnitHeader, out: &mut Vec<u8>) {
    let mut header = [0; MAX_UNIT_HEADER_LEN];
    let mut n = 0;
    let mut field = |v: u64| {
        let mut b = [0; MAX_UVARINT_LEN];
        let len = put_uvarint(&mut b, v);
        if let (Some(dst), Some(src)) = (header.get_mut(n..n + len), b.get(..len)) {
            dst.copy_from_slice(src);
        }
        n += len;
    };
    field(u64::from(h.track.0));
    // A flags byte is a varint below 0x80.
    field(u64::from(if h.keyframe { FLAG_KEYFRAME } else { 0 }));
    field(h.seq);
    field(u64::from(h.config));
    field(zigzag(h.dts));
    field(zigzag(h.pts.saturating_sub(h.dts)));
    field(zigzag(h.published.unix_millis()));
    out.push(KIND_UNIT);
    put(out, gop_back);
    put(out, n as u64);
    out.extend_from_slice(header.get(..n).unwrap_or_default());
}

/// The bytes of an entry after its length.
fn entry_len(e: &ConfigEntry<'_>) -> usize {
    uvarint_len(u64::from(e.track.0))
        + uvarint_len(u64::from(e.version))
        + uvarint_len(e.next_seq)
        + e.sample_entry.len()
        + uvarint_len(u64::from(e.timescale))
        + uvarint_len(e.config.len() as u64)
        + e.config.len()
}

/// Appends a CONFIGS record of `entries`, in their order.
pub fn write_configs<'a>(
    gop_back: u64,
    entries: impl IntoIterator<Item = ConfigEntry<'a>>,
    out: &mut Vec<u8>,
) {
    out.push(KIND_CONFIGS);
    put(out, gop_back);
    // An empty header.
    put(out, 0);
    for e in entries {
        put(out, entry_len(&e) as u64);
        put(out, u64::from(e.track.0));
        put(out, u64::from(e.version));
        put(out, e.next_seq);
        out.extend_from_slice(&e.sample_entry);
        put(out, u64::from(e.timescale));
        put(out, e.config.len() as u64);
        out.extend_from_slice(e.config);
    }
}

#[cfg(test)]
#[path = "record_test.rs"]
mod tests;
