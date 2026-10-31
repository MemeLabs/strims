// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! H.265 decoder configurations: the HEVCDecoderConfigurationRecord (hvcC,
//! ISO/IEC 14496-15 8.3.3.1) and the picture size from its first sequence
//! parameter set (ITU-T H.265 7.3.2.2).

use crate::bits::{BitReader, BitsError};
use crate::media::MAX_CONFIG_LEN;

/// The NAL unit type of a sequence parameter set.
const NAL_TYPE_SPS: u8 = 33;
/// The bytes of an hvcC record before its arrays of NAL units.
const HEADER_LEN: usize = 23;
/// The most sub-layers an SPS declares (H.265 7.4.3.2.1).
const MAX_SUB_LAYERS: u32 = 7;
/// The largest bit_depth_*_minus8 (H.265 7.4.3.2.1).
const MAX_BIT_DEPTH_MINUS8: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HevcError {
    #[error("hvcC is longer than {MAX_CONFIG_LEN} bytes")]
    TooLong,
    #[error("hvcC is truncated")]
    Truncated,
    #[error("hvcC version {0} is not 1")]
    Version(u8),
    #[error("NAL length size {0} is not 1, 2 or 4")]
    NalLengthSize(u8),
    #[error("hvcC holds no SPS")]
    NoSps,
    #[error("the SPS array holds a NAL unit that is not an SPS")]
    NotSps,
    #[error("SPS is truncated")]
    SpsTruncated,
    /// An SPS field outside its range; names the field.
    #[error("SPS {0} is out of range")]
    SpsField(&'static str),
}

/// A parsed hvcC record: the fields of its header that name the codec, and
/// its first SPS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HevcConfig {
    /// general_profile_space, 0 to 3.
    pub profile_space: u8,
    /// general_tier_flag: the High tier.
    pub high_tier: bool,
    /// general_profile_idc.
    pub profile_idc: u8,
    /// general_profile_compatibility_flags, flag 0 in the most significant
    /// bit.
    pub compatibility: u32,
    /// general_constraint_indicator_flags.
    pub constraints: [u8; 6],
    /// general_level_idc.
    pub level_idc: u8,
    /// Bytes in each NAL unit's length prefix: 1, 2 or 4.
    pub nal_length_size: u8,
    /// The first SPS.
    pub sps: HevcSps,
}

impl HevcConfig {
    /// The RFC 6381 codec string of an `hvc1` sample entry (ISO/IEC
    /// 14496-15 E.3), such as `hvc1.1.6.L93.90`: the profile space and
    /// profile, the compatibility flags in reverse bit order, the tier and
    /// level, and the constraint bytes without the trailing zero ones, in
    /// upper-case hexadecimal.
    pub fn codec(&self) -> String {
        let space = match self.profile_space {
            1 => "A",
            2 => "B",
            3 => "C",
            _ => "",
        };
        let tier = if self.high_tier { 'H' } else { 'L' };
        let mut s = format!(
            "hvc1.{space}{}.{:X}.{tier}{}",
            self.profile_idc,
            self.compatibility.reverse_bits(),
            self.level_idc
        );
        let kept = self
            .constraints
            .iter()
            .rposition(|&b| b != 0)
            .map_or(0, |i| i + 1);
        for b in self.constraints.iter().take(kept) {
            s.push_str(&format!(".{b:02X}"));
        }
        s
    }
}

/// The fields of a sequence parameter set that size and sample the
/// picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HevcSps {
    /// 0 monochrome, 1 4:2:0, 2 4:2:2, 3 4:4:4.
    pub chroma_format_idc: u8,
    pub bit_depth_luma: u8,
    pub bit_depth_chroma: u8,
    /// The picture size inside the conformance window, in pixels.
    pub width: u16,
    pub height: u16,
}

/// Parses an hvcC record of at most [`MAX_CONFIG_LEN`] bytes. Its arrays
/// of NAL units are read through, and the first NAL unit of an SPS array
/// is parsed as the SPS; an hvcC without one is refused. Bytes after the
/// arrays are not read.
pub fn parse_hvcc(data: &[u8]) -> Result<HevcConfig, HevcError> {
    if data.len() > MAX_CONFIG_LEN {
        return Err(HevcError::TooLong);
    }
    let (header, mut rest) = data
        .split_first_chunk::<HEADER_LEN>()
        .ok_or(HevcError::Truncated)?;
    let [
        version,
        ptl,
        c0,
        c1,
        c2,
        c3,
        k0,
        k1,
        k2,
        k3,
        k4,
        k5,
        level_idc,
        ..,
        layout,
        arrays,
    ] = *header;
    if version != 1 {
        return Err(HevcError::Version(version));
    }
    let nal_length_size = (layout & 0x03) + 1;
    if nal_length_size == 3 {
        return Err(HevcError::NalLengthSize(nal_length_size));
    }
    let mut sps = None;
    for _ in 0..arrays {
        let (&kind, after) = rest.split_first().ok_or(HevcError::Truncated)?;
        let (count, after) = after.split_first_chunk::<2>().ok_or(HevcError::Truncated)?;
        rest = after;
        for i in 0..u16::from_be_bytes(*count) {
            let (len, after) = rest.split_first_chunk::<2>().ok_or(HevcError::Truncated)?;
            let (nal, after) = after
                .split_at_checked(usize::from(u16::from_be_bytes(*len)))
                .ok_or(HevcError::Truncated)?;
            rest = after;
            if kind & 0x3f == NAL_TYPE_SPS && i == 0 && sps.is_none() {
                sps = Some(parse_sps(nal)?);
            }
        }
    }
    Ok(HevcConfig {
        profile_space: ptl >> 6,
        high_tier: ptl & 0x20 != 0,
        profile_idc: ptl & 0x1f,
        compatibility: u32::from_be_bytes([c0, c1, c2, c3]),
        constraints: [k0, k1, k2, k3, k4, k5],
        level_idc,
        nal_length_size,
        sps: sps.ok_or(HevcError::NoSps)?,
    })
}

impl From<BitsError> for HevcError {
    fn from(e: BitsError) -> Self {
        match e {
            BitsError::End => HevcError::SpsTruncated,
            BitsError::CodeTooLong => HevcError::SpsField("exp-Golomb code"),
        }
    }
}

/// Parses an SPS NAL unit, its two header bytes included, as far as the
/// bit depths.
pub fn parse_sps(nal: &[u8]) -> Result<HevcSps, HevcError> {
    let (header, payload) = nal
        .split_first_chunk::<2>()
        .ok_or(HevcError::SpsTruncated)?;
    if (header[0] >> 1) & 0x3f != NAL_TYPE_SPS {
        return Err(HevcError::NotSps);
    }
    let mut r = BitReader::rbsp(payload);
    r.bits(4)?; // sps_video_parameter_set_id
    let sub_layers = r.bits(3)? + 1;
    if sub_layers > MAX_SUB_LAYERS {
        return Err(HevcError::SpsField("sps_max_sub_layers_minus1"));
    }
    r.bit()?; // sps_temporal_id_nesting_flag
    skip_profile_tier_level(&mut r, sub_layers - 1)?;
    r.ue()?; // sps_seq_parameter_set_id
    let chroma_format_idc = r.ue()?;
    if chroma_format_idc > 3 {
        return Err(HevcError::SpsField("chroma_format_idc"));
    }
    let separate_colour_plane = chroma_format_idc == 3 && r.bit()?;
    let coded_width = u64::from(r.ue()?);
    let coded_height = u64::from(r.ue()?);
    let [left, right, top, bottom] = if r.bit()? {
        [
            u64::from(r.ue()?),
            u64::from(r.ue()?),
            u64::from(r.ue()?),
            u64::from(r.ue()?),
        ]
    } else {
        [0; 4]
    };
    let bit_depth_luma = r.ue()?;
    let bit_depth_chroma = r.ue()?;
    if bit_depth_luma > MAX_BIT_DEPTH_MINUS8 || bit_depth_chroma > MAX_BIT_DEPTH_MINUS8 {
        return Err(HevcError::SpsField("bit depth"));
    }

    // H.265 7.4.3.2.1: conformance window offsets count in chroma samples.
    let chroma_array_type = if separate_colour_plane {
        0
    } else {
        chroma_format_idc
    };
    let (sub_width, sub_height) = match chroma_array_type {
        1 => (2, 2),
        2 => (2, 1),
        _ => (1, 1),
    };
    // Each value is below 2^32, so the products can't overflow.
    let width = coded_width.checked_sub(sub_width * (left + right));
    let height = coded_height.checked_sub(sub_height * (top + bottom));
    let size = |v: Option<u64>, field: &'static str| {
        v.and_then(|v| u16::try_from(v).ok())
            .filter(|&v| v > 0)
            .ok_or(HevcError::SpsField(field))
    };
    Ok(HevcSps {
        // Each is at most 3, or 8 + 8.
        chroma_format_idc: chroma_format_idc as u8,
        bit_depth_luma: (bit_depth_luma + 8) as u8,
        bit_depth_chroma: (bit_depth_chroma + 8) as u8,
        width: size(width, "picture width")?,
        height: size(height, "picture height")?,
    })
}

/// Reads past a profile_tier_level(1, `max_sub_layers_minus1`) (H.265
/// 7.3.3).
fn skip_profile_tier_level(
    r: &mut BitReader<'_>,
    max_sub_layers_minus1: u32,
) -> Result<(), HevcError> {
    // general profile space, tier, profile, compatibility flags,
    // constraint flags and level: 2 + 1 + 5 + 32 + 48 + 8 bits.
    for n in [8, 32, 32, 16, 8] {
        r.bits(n)?;
    }
    let mut present = [(false, false); 8];
    for p in present.iter_mut().take(max_sub_layers_minus1 as usize) {
        *p = (r.bit()?, r.bit()?);
    }
    if max_sub_layers_minus1 > 0 {
        for _ in max_sub_layers_minus1..8 {
            r.bits(2)?; // reserved_zero_2bits
        }
    }
    for &(profile, level) in present.iter().take(max_sub_layers_minus1 as usize) {
        if profile {
            // sub-layer profile space, tier, profile, compatibility and
            // constraint flags: 2 + 1 + 5 + 32 + 48 bits.
            for n in [8, 32, 32, 16] {
                r.bits(n)?;
            }
        }
        if level {
            r.bits(8)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "hevc_test.rs"]
mod tests;
