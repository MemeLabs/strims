// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! AV1 decoder configurations: the AV1CodecConfigurationRecord (av1C, AV1
//! Codec ISO Media File Format Binding 2.3) and the picture size from the
//! sequence header OBU it carries (AV1 5.5), and the OBUs of a temporal
//! unit that ISOBMFF samples leave out.

use crate::bits::{BitReader, BitsError};
use crate::media::MAX_CONFIG_LEN;

/// OBU types (AV1 6.2.2).
const OBU_SEQUENCE_HEADER: u8 = 1;
const OBU_TEMPORAL_DELIMITER: u8 = 2;
/// The bytes of an av1C record before its config OBUs.
const HEADER_LEN: usize = 4;
/// The longest leb128 (AV1 4.10.5).
const MAX_LEB128_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Av1Error {
    #[error("av1C is longer than {MAX_CONFIG_LEN} bytes")]
    TooLong,
    #[error("av1C is truncated")]
    Truncated,
    #[error("av1C marker bit is not set")]
    Marker,
    #[error("av1C version {0} is not 1")]
    Version(u8),
    #[error("an OBU is malformed")]
    Obu,
    #[error("av1C holds no sequence header OBU")]
    NoSequenceHeader,
    #[error("sequence header is truncated")]
    SequenceHeaderTruncated,
    /// A sequence header field outside its range; names the field.
    #[error("sequence header {0} is out of range")]
    SequenceHeaderField(&'static str),
}

/// A parsed av1C record: the fields that name the codec, and the picture
/// size from its sequence header OBU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Av1Config {
    /// seq_profile.
    pub profile: u8,
    /// `seq_level_idx[0]`.
    pub level: u8,
    /// `seq_tier[0]`: the High tier.
    pub high_tier: bool,
    /// 8, 10 or 12.
    pub bit_depth: u8,
    pub monochrome: bool,
    /// The sequence header's largest frame size, in pixels.
    pub width: u16,
    pub height: u16,
}

impl Av1Config {
    /// The RFC 6381 codec string of an `av01` sample entry (AV1 Codec ISO
    /// Media File Format Binding, Codecs Parameter String), in its short
    /// form: `av01.P.LLT.DD`, such as `av01.0.08M.08`.
    pub fn codec(&self) -> String {
        format!(
            "av01.{}.{:02}{}.{:02}",
            self.profile,
            self.level,
            if self.high_tier { 'H' } else { 'M' },
            self.bit_depth
        )
    }
}

/// Parses an av1C record of at most [`MAX_CONFIG_LEN`] bytes whose config
/// OBUs hold a sequence header OBU, which gives the picture size. A record
/// without one is refused.
pub fn parse_av1c(data: &[u8]) -> Result<Av1Config, Av1Error> {
    if data.len() > MAX_CONFIG_LEN {
        return Err(Av1Error::TooLong);
    }
    let (&[marker, profile_level, flags, _delay], mut obus) = data
        .split_first_chunk::<HEADER_LEN>()
        .ok_or(Av1Error::Truncated)?;
    if marker & 0x80 == 0 {
        return Err(Av1Error::Marker);
    }
    if marker & 0x7f != 1 {
        return Err(Av1Error::Version(marker & 0x7f));
    }
    let high_bitdepth = flags & 0x40 != 0;
    let twelve_bit = flags & 0x20 != 0;
    let (width, height) = loop {
        let (obu, rest) = next_obu(obus)?.ok_or(Av1Error::NoSequenceHeader)?;
        obus = rest;
        if obu.kind == OBU_SEQUENCE_HEADER {
            break parse_sequence_header(obu.payload)?;
        }
    };
    Ok(Av1Config {
        profile: profile_level >> 5,
        level: profile_level & 0x1f,
        high_tier: flags & 0x80 != 0,
        bit_depth: match (high_bitdepth, twelve_bit) {
            (false, _) => 8,
            (true, false) => 10,
            (true, true) => 12,
        },
        monochrome: flags & 0x10 != 0,
        width,
        height,
    })
}

/// A temporal unit without the temporal delimiter OBUs it starts with:
/// ISOBMFF samples leave them out. Bytes that don't parse as OBUs are left
/// as they are.
pub fn strip_temporal_delimiters(unit: &[u8]) -> &[u8] {
    let mut rest = unit;
    while let Ok(Some((obu, after))) = next_obu(rest) {
        if obu.kind != OBU_TEMPORAL_DELIMITER || !obu.sized {
            break;
        }
        rest = after;
    }
    rest
}

/// An OBU of a sequence of them.
struct Obu<'a> {
    kind: u8,
    /// Its header carries its size; an OBU without one lasts to the end.
    sized: bool,
    payload: &'a [u8],
}

/// The OBU `data` starts with and the bytes after it; `None` for no bytes.
fn next_obu(data: &[u8]) -> Result<Option<(Obu<'_>, &[u8])>, Av1Error> {
    let Some((&header, mut rest)) = data.split_first() else {
        return Ok(None);
    };
    if header & 0x80 != 0 {
        return Err(Av1Error::Obu);
    }
    if header & 0x04 != 0 {
        // obu_extension_header
        rest = rest.get(1..).ok_or(Av1Error::Obu)?;
    }
    let sized = header & 0x02 != 0;
    let (payload, after) = if sized {
        let (len, after) = leb128(rest)?;
        after.split_at_checked(len).ok_or(Av1Error::Obu)?
    } else {
        (rest, &[][..])
    };
    let obu = Obu {
        kind: (header >> 3) & 0x0f,
        sized,
        payload,
    };
    Ok(Some((obu, after)))
}

/// An unsigned leb128 of at most [`MAX_LEB128_LEN`] bytes and a value below
/// 2^32 (AV1 4.10.5), and the bytes after it.
fn leb128(data: &[u8]) -> Result<(usize, &[u8]), Av1Error> {
    let mut value: u64 = 0;
    for (i, &b) in data.iter().take(MAX_LEB128_LEN).enumerate() {
        value |= u64::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            let value = u32::try_from(value)
                .ok()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(Av1Error::Obu)?;
            let rest = data.get(i + 1..).ok_or(Av1Error::Obu)?;
            return Ok((value, rest));
        }
    }
    Err(Av1Error::Obu)
}

impl From<BitsError> for Av1Error {
    fn from(e: BitsError) -> Self {
        match e {
            BitsError::End => Av1Error::SequenceHeaderTruncated,
            BitsError::CodeTooLong => Av1Error::SequenceHeaderField("uvlc code"),
        }
    }
}

/// The largest frame size a sequence header OBU's payload declares (AV1
/// 5.5.1), read as far as max_frame_height_minus_1.
pub fn parse_sequence_header(payload: &[u8]) -> Result<(u16, u16), Av1Error> {
    let mut r = BitReader::new(payload);
    r.bits(3)?; // seq_profile
    r.bit()?; // still_picture
    if r.bit()? {
        // reduced_still_picture_header: seq_level_idx[0] alone.
        r.bits(5)?;
    } else {
        let mut decoder_model_info = false;
        let mut buffer_delay_bits = 0;
        if r.bit()? {
            // timing_info: num_units_in_display_tick, time_scale
            r.bits(32)?;
            r.bits(32)?;
            if r.bit()? {
                // num_ticks_per_picture_minus_1, uvlc()
                r.ue()?;
            }
            decoder_model_info = r.bit()?;
            if decoder_model_info {
                buffer_delay_bits = r.bits(5)? + 1;
                // num_units_in_decoding_tick
                r.bits(32)?;
                // buffer_removal_time_length_minus_1,
                // frame_presentation_time_length_minus_1
                r.bits(10)?;
            }
        }
        let initial_display_delay = r.bit()?;
        let operating_points = r.bits(5)? + 1;
        for _ in 0..operating_points {
            r.bits(12)?; // operating_point_idc
            if r.bits(5)? > 7 {
                r.bit()?; // seq_tier
            }
            if decoder_model_info && r.bit()? {
                // decoder_buffer_delay, encoder_buffer_delay,
                // low_delay_mode_flag
                r.bits(buffer_delay_bits)?;
                r.bits(buffer_delay_bits)?;
                r.bit()?;
            }
            if initial_display_delay && r.bit()? {
                r.bits(4)?;
            }
        }
    }
    let width_bits = r.bits(4)? + 1;
    let height_bits = r.bits(4)? + 1;
    let size = |v: u32, field: &'static str| {
        u16::try_from(u64::from(v) + 1).map_err(|_| Av1Error::SequenceHeaderField(field))
    };
    let width = size(r.bits(width_bits)?, "max_frame_width_minus_1")?;
    let height = size(r.bits(height_bits)?, "max_frame_height_minus_1")?;
    Ok((width, height))
}

#[cfg(test)]
#[path = "av1_test.rs"]
mod tests;
