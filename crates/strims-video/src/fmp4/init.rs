// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The init segment: `ftyp` and a `moov` declaring one track per decoder
//! config, with `mvex` for fragments.

use crate::aac::{self, AacError};
use crate::fmp4::boxes::{self, end, start, start_full};
use crate::media::{CodecParams, DecoderConfig, TrackKind};

/// The most tracks an init segment declares.
pub const MAX_TRACKS: usize = 8;

/// The movie timescale of `mvhd`; the movie declares no duration, so only
/// the tracks' own timescales time samples.
const MOVIE_TIMESCALE: u32 = 1000;
/// ISO 639-2 "und" packed into 15 bits, 5 bits per letter less 0x60.
const LANGUAGE_UND: u16 = (21 << 10) | (14 << 5) | 4;
/// `tkhd` flags: track_enabled | track_in_movie.
const TKHD_ENABLED_IN_MOVIE: u32 = 0x03;
/// `url ` flags: the media data is in this file.
const URL_SELF_CONTAINED: u32 = 0x01;
/// 72 dpi in 16.16.
const DPI_72: u32 = 72 << 16;
/// `avc1` depth: colour with no alpha.
const DEPTH_COLOUR: u16 = 0x0018;
/// The channel count written when the AudioSpecificConfig leaves the
/// layout to a program config element.
const FALLBACK_CHANNELS: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum InitError {
    #[error("more than {MAX_TRACKS} tracks")]
    TooManyTracks,
    #[error("track id 0 is reserved")]
    TrackIdZero,
    #[error("track {0} is declared twice")]
    DuplicateTrack(u32),
    #[error("track {0} has timescale 0")]
    ZeroTimescale(u32),
    /// The codec parameters are of another kind of track.
    #[error("track {track} is {kind} but its codec parameters are not")]
    KindMismatch { track: u32, kind: TrackKind },
    #[error("track {track}: {error}")]
    Aac { track: u32, error: AacError },
}

/// Checks that `configs` can be declared together: at most
/// [`MAX_TRACKS`], distinct nonzero ids, nonzero timescales, and codec
/// parameters of each track's kind.
pub fn check_configs(configs: &[DecoderConfig]) -> Result<(), InitError> {
    if configs.len() > MAX_TRACKS {
        return Err(InitError::TooManyTracks);
    }
    for (i, c) in configs.iter().enumerate() {
        let track = c.track.0;
        if track == 0 {
            return Err(InitError::TrackIdZero);
        }
        if configs.iter().take(i).any(|o| o.track == c.track) {
            return Err(InitError::DuplicateTrack(track));
        }
        if c.timescale == 0 {
            return Err(InitError::ZeroTimescale(track));
        }
        let params_kind = match c.params {
            CodecParams::Avc { .. } => TrackKind::Video,
            CodecParams::Aac { .. } => TrackKind::Audio,
        };
        if params_kind != c.kind {
            return Err(InitError::KindMismatch {
                track,
                kind: c.kind,
            });
        }
    }
    Ok(())
}

/// Appends the init segment declaring `configs`, one `trak` each in the
/// order given, track id = config track.
///
/// - `ftyp`: major brand `iso5`, minor version 512, compatible `iso5`,
///   `iso6`, `mp41`.
/// - `moov`: `mvhd` (timescale 1000, no duration), the `trak`s, then
///   `mvex` with a `trex` per track (sample description 1, no defaults).
/// - Each `trak` has `tkhd` (enabled, in movie; the picture size for
///   video, full volume for audio) and `mdia`: `mdhd` with the config's
///   timescale and language `und`, `hdlr` (`vide` or `soun`), and `minf`
///   with `vmhd` or `smhd`, a self-contained `dref` and an `stbl` of one
///   sample entry and empty sample tables.
/// - Video's sample entry is `avc1` holding the avcC record as received.
/// - Audio's is `mp4a` holding an `esds` with the AudioSpecificConfig. Its
///   16.16 sample rate field holds 16 bits: a rate above 65535 Hz is
///   halved until it fits. Decoders take the rate from the
///   AudioSpecificConfig, and `mdhd` carries the true rate as the
///   timescale. A channel count of 0 (a layout from a program config
///   element) is written as 2.
///
/// On error `out` is unchanged.
pub fn write_init(configs: &[DecoderConfig], out: &mut Vec<u8>) -> Result<(), InitError> {
    check_configs(configs)?;
    let from = out.len();
    let written = write_checked(configs, out);
    if written.is_err() {
        out.truncate(from);
    }
    written
}

fn write_checked(configs: &[DecoderConfig], out: &mut Vec<u8>) -> Result<(), InitError> {
    let ftyp = start(out, b"ftyp");
    out.extend_from_slice(b"iso5");
    boxes::u32(out, 512);
    out.extend_from_slice(b"iso5iso6mp41");
    end(out, ftyp);

    let moov = start(out, b"moov");
    write_mvhd(configs, out);
    for c in configs {
        write_trak(c, out)?;
    }
    let mvex = start(out, b"mvex");
    for c in configs {
        let trex = start_full(out, b"trex", 0, 0);
        boxes::u32(out, c.track.0);
        // default_sample_description_index
        boxes::u32(out, 1);
        // default_sample_duration, _size, _flags
        boxes::zeros(out, 12);
        end(out, trex);
    }
    end(out, mvex);
    end(out, moov);
    Ok(())
}

fn write_mvhd(configs: &[DecoderConfig], out: &mut Vec<u8>) {
    let next_track = configs
        .iter()
        .map(|c| c.track.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mvhd = start_full(out, b"mvhd", 0, 0);
    // creation_time, modification_time
    boxes::zeros(out, 8);
    boxes::u32(out, MOVIE_TIMESCALE);
    // duration
    boxes::u32(out, 0);
    // rate 1.0, volume 1.0, reserved
    boxes::u32(out, 0x0001_0000);
    boxes::u16(out, 0x0100);
    boxes::zeros(out, 10);
    boxes::unity_matrix(out);
    // pre_defined
    boxes::zeros(out, 24);
    boxes::u32(out, next_track);
    end(out, mvhd);
}

fn write_trak(c: &DecoderConfig, out: &mut Vec<u8>) -> Result<(), InitError> {
    let trak = start(out, b"trak");
    let (width, height) = match c.params {
        CodecParams::Avc { width, height, .. } => (width, height),
        CodecParams::Aac { .. } => (0, 0),
    };
    let tkhd = start_full(out, b"tkhd", 0, TKHD_ENABLED_IN_MOVIE);
    // creation_time, modification_time
    boxes::zeros(out, 8);
    boxes::u32(out, c.track.0);
    // reserved, duration, reserved[2], layer, alternate_group
    boxes::zeros(out, 20);
    boxes::u16(
        out,
        if c.kind == TrackKind::Audio {
            0x0100
        } else {
            0
        },
    );
    // reserved
    boxes::u16(out, 0);
    boxes::unity_matrix(out);
    boxes::u32(out, u32::from(width) << 16);
    boxes::u32(out, u32::from(height) << 16);
    end(out, tkhd);

    let mdia = start(out, b"mdia");
    let mdhd = start_full(out, b"mdhd", 0, 0);
    // creation_time, modification_time
    boxes::zeros(out, 8);
    boxes::u32(out, c.timescale);
    // duration
    boxes::u32(out, 0);
    boxes::u16(out, LANGUAGE_UND);
    // pre_defined
    boxes::u16(out, 0);
    end(out, mdhd);

    let (handler, name): (&[u8; 4], &[u8]) = match c.kind {
        TrackKind::Video => (b"vide", b"VideoHandler\0"),
        TrackKind::Audio => (b"soun", b"SoundHandler\0"),
    };
    let hdlr = start_full(out, b"hdlr", 0, 0);
    // pre_defined
    boxes::u32(out, 0);
    out.extend_from_slice(handler);
    // reserved[3]
    boxes::zeros(out, 12);
    out.extend_from_slice(name);
    end(out, hdlr);

    let minf = start(out, b"minf");
    match c.kind {
        TrackKind::Video => {
            let vmhd = start_full(out, b"vmhd", 0, 1);
            // graphicsmode, opcolor[3]
            boxes::zeros(out, 8);
            end(out, vmhd);
        }
        TrackKind::Audio => {
            let smhd = start_full(out, b"smhd", 0, 0);
            // balance, reserved
            boxes::zeros(out, 4);
            end(out, smhd);
        }
    }
    let dinf = start(out, b"dinf");
    let dref = start_full(out, b"dref", 0, 0);
    boxes::u32(out, 1);
    let url = start_full(out, b"url ", 0, URL_SELF_CONTAINED);
    end(out, url);
    end(out, dref);
    end(out, dinf);

    let stbl = start(out, b"stbl");
    let stsd = start_full(out, b"stsd", 0, 0);
    boxes::u32(out, 1);
    write_sample_entry(c, out)?;
    end(out, stsd);
    for (fourcc, fields) in [(b"stts", 1), (b"stsc", 1), (b"stsz", 2), (b"stco", 1)] {
        let table = start_full(out, fourcc, 0, 0);
        // entry_count; stsz: sample_size, sample_count
        boxes::zeros(out, 4 * fields);
        end(out, table);
    }
    end(out, stbl);
    end(out, minf);
    end(out, mdia);
    end(out, trak);
    Ok(())
}

fn write_sample_entry(c: &DecoderConfig, out: &mut Vec<u8>) -> Result<(), InitError> {
    match &c.params {
        CodecParams::Avc {
            avcc,
            width,
            height,
        } => {
            let avc1 = start(out, b"avc1");
            sample_entry_header(out);
            // pre_defined, reserved, pre_defined[3]
            boxes::zeros(out, 16);
            boxes::u16(out, *width);
            boxes::u16(out, *height);
            boxes::u32(out, DPI_72);
            boxes::u32(out, DPI_72);
            // reserved
            boxes::u32(out, 0);
            // frame_count
            boxes::u16(out, 1);
            // compressorname: an empty counted string
            boxes::zeros(out, 32);
            boxes::u16(out, DEPTH_COLOUR);
            // pre_defined
            boxes::u16(out, 0xffff);
            let avcc_box = start(out, b"avcC");
            out.extend_from_slice(avcc);
            end(out, avcc_box);
            end(out, avc1);
        }
        CodecParams::Aac {
            asc,
            sample_rate,
            channels,
            ..
        } => {
            let mp4a = start(out, b"mp4a");
            sample_entry_header(out);
            // reserved[2]
            boxes::zeros(out, 8);
            boxes::u16(
                out,
                match channels {
                    0 => FALLBACK_CHANNELS,
                    n => u16::from(*n),
                },
            );
            // samplesize
            boxes::u16(out, 16);
            // pre_defined, reserved
            boxes::zeros(out, 4);
            boxes::u32(out, sample_rate_field(*sample_rate));
            let esds = start(out, b"esds");
            aac::write_esds(asc, out).map_err(|error| InitError::Aac {
                track: c.track.0,
                error,
            })?;
            end(out, esds);
            end(out, mp4a);
        }
    }
    Ok(())
}

/// SampleEntry: reserved[6], data_reference_index 1.
fn sample_entry_header(out: &mut Vec<u8>) {
    boxes::zeros(out, 6);
    boxes::u16(out, 1);
}

/// The 16.16 `samplerate` of an AudioSampleEntry: the rate, halved until
/// it fits 16 bits.
fn sample_rate_field(rate: u32) -> u32 {
    let mut rate = rate;
    while rate > u32::from(u16::MAX) {
        rate /= 2;
    }
    rate << 16
}

#[cfg(test)]
#[path = "init_test.rs"]
mod tests;
