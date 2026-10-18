// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! H.264 decoder configurations: the AVCDecoderConfigurationRecord (avcC,
//! ISO/IEC 14496-15 5.3.3.1) and the picture size from its first sequence
//! parameter set (ITU-T H.264 7.3.2.1.1).

use crate::bits::{BitReader, BitsError};
use crate::media::MAX_CONFIG_LEN;

/// profile_idc values whose SPS carries the chroma format, bit depths and
/// scaling matrices.
const HIGH_PROFILES: [u32; 13] = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];

/// The bound on num_ref_frames_in_pic_order_cnt_cycle (H.264 7.4.2.1.1).
const MAX_POC_CYCLE: u32 = 255;

const NAL_TYPE_SPS: u8 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AvcError {
    #[error("avcC is longer than {MAX_CONFIG_LEN} bytes")]
    TooLong,
    #[error("avcC is truncated")]
    Truncated,
    #[error("avcC version {0} is not 1")]
    Version(u8),
    #[error("NAL length size {0} is not 1, 2 or 4")]
    NalLengthSize(u8),
    #[error("avcC holds no SPS")]
    NoSps,
    #[error("the first parameter set is not an SPS")]
    NotSps,
    #[error("SPS is truncated")]
    SpsTruncated,
    /// An SPS field outside its range; names the field.
    #[error("SPS {0} is out of range")]
    SpsField(&'static str),
}

/// A parsed avcC record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvcConfig {
    /// AVCProfileIndication.
    pub profile: u8,
    /// profile_compatibility: the constraint flags.
    pub compatibility: u8,
    /// AVCLevelIndication.
    pub level: u8,
    /// Bytes in each NAL unit's length prefix: 1, 2 or 4.
    pub nal_length_size: u8,
    /// The first SPS.
    pub sps: Sps,
}

impl AvcConfig {
    /// The RFC 6381 codec string, `avc1.PPCCLL` from the record's profile,
    /// compatibility and level bytes.
    pub fn codec(&self) -> String {
        format!(
            "avc1.{:02x}{:02x}{:02x}",
            self.profile, self.compatibility, self.level
        )
    }
}

/// The fields of a sequence parameter set that size the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sps {
    pub profile_idc: u8,
    pub level_idc: u8,
    /// 0 monochrome, 1 4:2:0, 2 4:2:2, 3 4:4:4.
    pub chroma_format_idc: u8,
    pub frame_mbs_only: bool,
    /// The picture size after cropping, in pixels.
    pub width: u16,
    pub height: u16,
}

/// Parses an avcC record of at most [`MAX_CONFIG_LEN`] bytes holding
/// 1..=31 SPS and up to 255 PPS. Bytes after the PPS (the high profile
/// extension) are not read.
pub fn parse_avcc(data: &[u8]) -> Result<AvcConfig, AvcError> {
    if data.len() > MAX_CONFIG_LEN {
        return Err(AvcError::TooLong);
    }
    let mut r = Bytes(data);
    let version = r.u8()?;
    if version != 1 {
        return Err(AvcError::Version(version));
    }
    let profile = r.u8()?;
    let compatibility = r.u8()?;
    let level = r.u8()?;
    let nal_length_size = (r.u8()? & 0x03) + 1;
    if nal_length_size == 3 {
        return Err(AvcError::NalLengthSize(nal_length_size));
    }
    // A 5-bit count: at most 31.
    let sps_count = r.u8()? & 0x1f;
    if sps_count == 0 {
        return Err(AvcError::NoSps);
    }
    let first = r.parameter_set()?;
    for _ in 1..sps_count {
        r.parameter_set()?;
    }
    for _ in 0..r.u8()? {
        r.parameter_set()?;
    }
    Ok(AvcConfig {
        profile,
        compatibility,
        level,
        nal_length_size,
        sps: parse_sps(first)?,
    })
}

/// A cursor over avcC bytes.
struct Bytes<'a>(&'a [u8]);

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], AvcError> {
        let (head, rest) = self.0.split_at_checked(n).ok_or(AvcError::Truncated)?;
        self.0 = rest;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, AvcError> {
        let (&b, rest) = self.0.split_first().ok_or(AvcError::Truncated)?;
        self.0 = rest;
        Ok(b)
    }

    /// A parameter set NAL unit after its 16-bit length.
    fn parameter_set(&mut self) -> Result<&'a [u8], AvcError> {
        let len = u16::from_be_bytes([self.u8()?, self.u8()?]);
        self.take(usize::from(len))
    }
}

impl From<BitsError> for AvcError {
    fn from(e: BitsError) -> Self {
        match e {
            BitsError::End => AvcError::SpsTruncated,
            BitsError::CodeTooLong => AvcError::SpsField("exp-Golomb code"),
        }
    }
}

/// Parses an SPS NAL unit, header byte included, as far as the frame
/// cropping fields.
pub fn parse_sps(nal: &[u8]) -> Result<Sps, AvcError> {
    let (&header, payload) = nal.split_first().ok_or(AvcError::SpsTruncated)?;
    if header & 0x1f != NAL_TYPE_SPS {
        return Err(AvcError::NotSps);
    }
    let mut r = BitReader::rbsp(payload);
    let profile_idc = r.bits(8)?;
    r.bits(8)?; // constraint_set flags
    let level_idc = r.bits(8)?;
    r.ue()?; // seq_parameter_set_id

    let mut chroma_format_idc = 1;
    let mut separate_colour_plane = false;
    if HIGH_PROFILES.contains(&profile_idc) {
        chroma_format_idc = r.ue()?;
        if chroma_format_idc > 3 {
            return Err(AvcError::SpsField("chroma_format_idc"));
        }
        if chroma_format_idc == 3 {
            separate_colour_plane = r.bit()?;
        }
        r.ue()?; // bit_depth_luma_minus8
        r.ue()?; // bit_depth_chroma_minus8
        r.bit()?; // qpprime_y_zero_transform_bypass_flag
        if r.bit()? {
            let lists = if chroma_format_idc == 3 { 12 } else { 8 };
            for i in 0..lists {
                if r.bit()? {
                    skip_scaling_list(&mut r, if i < 6 { 16 } else { 64 })?;
                }
            }
        }
    }

    r.ue()?; // log2_max_frame_num_minus4
    match r.ue()? {
        0 => {
            r.ue()?; // log2_max_pic_order_cnt_lsb_minus4
        }
        1 => {
            r.bit()?; // delta_pic_order_always_zero_flag
            r.se()?; // offset_for_non_ref_pic
            r.se()?; // offset_for_top_to_bottom_field
            let cycle = r.ue()?;
            if cycle > MAX_POC_CYCLE {
                return Err(AvcError::SpsField("num_ref_frames_in_pic_order_cnt_cycle"));
            }
            for _ in 0..cycle {
                r.se()?; // offset_for_ref_frame
            }
        }
        2 => {}
        _ => return Err(AvcError::SpsField("pic_order_cnt_type")),
    }
    r.ue()?; // max_num_ref_frames
    r.bit()?; // gaps_in_frame_num_value_allowed_flag
    let width_mbs = u64::from(r.ue()?) + 1;
    let height_map_units = u64::from(r.ue()?) + 1;
    let frame_mbs_only = r.bit()?;
    if !frame_mbs_only {
        r.bit()?; // mb_adaptive_frame_field_flag
    }
    r.bit()?; // direct_8x8_inference_flag
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

    // H.264 7.4.2.1.1: crop offsets count in chroma samples, and in field
    // pairs when the picture can be coded as fields.
    let fields = if frame_mbs_only { 1 } else { 2 };
    let chroma_array_type = if separate_colour_plane {
        0
    } else {
        chroma_format_idc
    };
    let (crop_x, crop_y) = match chroma_array_type {
        1 => (2, 2 * fields),
        2 => (2, fields),
        _ => (1, fields),
    };
    // Each factor is below 2^34, so the products can't overflow.
    let width = (width_mbs * 16).checked_sub(crop_x * (left + right));
    let height = (fields * height_map_units * 16).checked_sub(crop_y * (top + bottom));
    let size = |v: Option<u64>, field: &'static str| {
        v.and_then(|v| u16::try_from(v).ok())
            .filter(|&v| v > 0)
            .ok_or(AvcError::SpsField(field))
    };
    Ok(Sps {
        // Read from 8-bit fields; chroma_format_idc is at most 3.
        profile_idc: profile_idc as u8,
        level_idc: level_idc as u8,
        chroma_format_idc: chroma_format_idc as u8,
        frame_mbs_only,
        width: size(width, "picture width")?,
        height: size(height, "picture height")?,
    })
}

/// Reads past a scaling_list() of `size` coefficients (H.264 7.3.2.1.1.1).
fn skip_scaling_list(r: &mut BitReader<'_>, size: usize) -> Result<(), AvcError> {
    let mut last: i64 = 8;
    let mut next: i64 = 8;
    for _ in 0..size {
        if next != 0 {
            let delta = r.se()?;
            if !(-128..=127).contains(&delta) {
                return Err(AvcError::SpsField("delta_scale"));
            }
            next = (last + delta).rem_euclid(256);
        }
        if next != 0 {
            last = next;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "avc_test.rs"]
mod tests;
