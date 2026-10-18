// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! AAC decoder configurations: the AudioSpecificConfig (ISO/IEC 14496-3
//! 1.6.2.1) and the `esds` box that carries it in MP4 (ISO/IEC 14496-14
//! 3.1.2, descriptors from ISO/IEC 14496-1 7.2.6).

use crate::bits::{BitReader, BitsError};
use crate::media::MAX_CONFIG_LEN;

/// The object type of explicitly signalled SBR (HE-AAC).
pub const AOT_SBR: u8 = 5;
/// The object type of explicitly signalled parametric stereo (HE-AAC v2).
pub const AOT_PS: u8 = 29;

/// Core object types taken: AAC Main, LC, SSR and LTP, whose frames hold
/// 1024 samples, or 960 under frameLengthFlag.
const CORE_OBJECT_TYPES: std::ops::RangeInclusive<u8> = 1..=4;

/// The rates of samplingFrequencyIndex 0..=12; 15 escapes to an explicit
/// 24-bit rate and 13 and 14 are reserved.
const SAMPLE_RATES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];
const EXPLICIT_RATE: u8 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AacError {
    #[error("AudioSpecificConfig is longer than {MAX_CONFIG_LEN} bytes")]
    TooLong,
    #[error("AudioSpecificConfig is truncated")]
    Truncated,
    /// An object type other than Main, LC, SSR or LTP, with or without
    /// explicit SBR or PS.
    #[error("audio object type {0} is not supported")]
    ObjectType(u8),
    #[error("sampling frequency index {0} is reserved")]
    FrequencyIndex(u8),
    #[error("sampling frequency is 0")]
    ZeroRate,
    #[error("SBR rate {extension} is neither {core} nor twice it")]
    SbrRate { core: u32, extension: u32 },
    #[error("channel configuration {0} is reserved")]
    ChannelConfig(u8),
}

impl From<BitsError> for AacError {
    fn from(_: BitsError) -> Self {
        AacError::Truncated
    }
}

/// A parsed AudioSpecificConfig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asc {
    /// The signalled object type, which names the codec: [`AOT_SBR`] or
    /// [`AOT_PS`] for explicitly signalled HE-AAC.
    pub object_type: u8,
    /// The core coder's object type: 1 Main, 2 LC, 3 SSR or 4 LTP.
    pub core_object_type: u8,
    /// The core coder's sample rate.
    pub core_sample_rate: u32,
    /// The output sample rate: the SBR rate under explicit SBR.
    pub sample_rate: u32,
    /// channelConfiguration; 0 leaves the channels to a program config
    /// element in the stream.
    pub channel_config: u8,
    /// Output channels: 0 under channel configuration 0, and 2 for a
    /// parametric stereo mono core.
    pub channels: u8,
    /// Output samples per frame.
    pub frame_samples: u32,
}

impl Asc {
    /// The RFC 6381 codec string, `mp4a.40.<signalled object type>`.
    pub fn codec(&self) -> String {
        format!("mp4a.40.{}", self.object_type)
    }
}

/// Parses an AudioSpecificConfig of at most [`MAX_CONFIG_LEN`] bytes as far
/// as frameLengthFlag. SBR and PS count as signalled only when the object
/// type says so (hierarchical signalling); implicit and backward-compatible
/// SBR leave the core rate and frame length, which time the frames alike.
pub fn parse_asc(data: &[u8]) -> Result<Asc, AacError> {
    if data.len() > MAX_CONFIG_LEN {
        return Err(AacError::TooLong);
    }
    let mut r = BitReader::new(data);
    let object_type = read_object_type(&mut r)?;
    let core_sample_rate = read_rate(&mut r)?;
    // A 4-bit field.
    let channel_config = r.bits(4)? as u8;
    let (sample_rate, core_object_type) = if object_type == AOT_SBR || object_type == AOT_PS {
        (read_rate(&mut r)?, read_object_type(&mut r)?)
    } else {
        (core_sample_rate, object_type)
    };
    if !CORE_OBJECT_TYPES.contains(&core_object_type) {
        return Err(AacError::ObjectType(core_object_type));
    }
    // GASpecificConfig's first field.
    let core_frame = if r.bit()? { 960 } else { 1024 };
    let frame_samples = if sample_rate == core_sample_rate {
        core_frame
    } else if u64::from(sample_rate) == 2 * u64::from(core_sample_rate) {
        2 * core_frame
    } else {
        return Err(AacError::SbrRate {
            core: core_sample_rate,
            extension: sample_rate,
        });
    };
    let mut channels = match channel_config {
        0..=6 => channel_config,
        7 | 12 | 14 => 8,
        11 => 7,
        13 => 24,
        _ => return Err(AacError::ChannelConfig(channel_config)),
    };
    if object_type == AOT_PS && channels == 1 {
        channels = 2;
    }
    Ok(Asc {
        object_type,
        core_object_type,
        core_sample_rate,
        sample_rate,
        channel_config,
        channels,
        frame_samples,
    })
}

/// GetAudioObjectType(): 5 bits, with 31 escaping to 32 plus 6 more.
fn read_object_type(r: &mut BitReader<'_>) -> Result<u8, AacError> {
    // At most 32 + 63.
    let aot = r.bits(5)? as u8;
    Ok(if aot == 31 {
        32 + r.bits(6)? as u8
    } else {
        aot
    })
}

/// A samplingFrequencyIndex and, under the escape, the explicit rate.
fn read_rate(r: &mut BitReader<'_>) -> Result<u32, AacError> {
    // A 4-bit field.
    let index = r.bits(4)? as u8;
    let rate = if index == EXPLICIT_RATE {
        r.bits(24)?
    } else {
        *SAMPLE_RATES
            .get(usize::from(index))
            .ok_or(AacError::FrequencyIndex(index))?
    };
    if rate == 0 {
        return Err(AacError::ZeroRate);
    }
    Ok(rate)
}

const ES_DESCRIPTOR: u8 = 0x03;
const DECODER_CONFIG_DESCRIPTOR: u8 = 0x04;
const DECODER_SPECIFIC_INFO: u8 = 0x05;
const SL_CONFIG_DESCRIPTOR: u8 = 0x06;
/// objectTypeIndication of MPEG-4 audio (ISO/IEC 14496-3).
const OTI_MPEG4_AUDIO: u8 = 0x40;
/// streamType 0x05 (audio) in the top 6 bits, upStream 0, reserved 1.
const AUDIO_STREAM: u8 = (0x05 << 2) | 1;
/// SLConfigDescriptor predefined 2, reserved for MP4 files.
const SL_MP4: u8 = 0x02;

/// Appends the payload of an `esds` box (everything after its 8-byte
/// header) carrying `asc`: version and flags, then an ES_Descriptor
/// holding a DecoderConfigDescriptor for MPEG-4 audio with `asc` as its
/// DecoderSpecificInfo, and an SLConfigDescriptor. ES_ID, buffer size and
/// bitrates are 0 (unset, variable). Descriptor sizes take the fewest
/// bytes.
pub fn write_esds(asc: &[u8], out: &mut Vec<u8>) -> Result<(), AacError> {
    if asc.len() > MAX_CONFIG_LEN {
        return Err(AacError::TooLong);
    }
    let dsi = asc.len();
    let dcd = 13 + descriptor_len(dsi);
    let es = 3 + descriptor_len(dcd) + descriptor_len(1);
    out.extend_from_slice(&[0, 0, 0, 0]);
    put_header(out, ES_DESCRIPTOR, es);
    // ES_ID 0; no stream dependence, URL or OCR stream; priority 0.
    out.extend_from_slice(&[0, 0, 0]);
    put_header(out, DECODER_CONFIG_DESCRIPTOR, dcd);
    out.extend_from_slice(&[OTI_MPEG4_AUDIO, AUDIO_STREAM]);
    // bufferSizeDB (24 bits), maxBitrate, avgBitrate
    out.extend_from_slice(&[0; 11]);
    put_header(out, DECODER_SPECIFIC_INFO, dsi);
    out.extend_from_slice(asc);
    put_header(out, SL_CONFIG_DESCRIPTOR, 1);
    out.push(SL_MP4);
    Ok(())
}

/// Bytes of a size field: 7 bits per byte, at most 4 bytes.
fn size_len(size: usize) -> usize {
    match size {
        0..0x80 => 1,
        0x80..0x4000 => 2,
        0x4000..0x20_0000 => 3,
        _ => 4,
    }
}

/// Bytes of a descriptor with `size` bytes of content.
fn descriptor_len(size: usize) -> usize {
    1 + size_len(size) + size
}

/// A descriptor's tag and size, the size's high groups first, each but
/// the last with the continuation bit.
fn put_header(out: &mut Vec<u8>, tag: u8, size: usize) {
    out.push(tag);
    for i in (0..size_len(size)).rev() {
        // Masked to 7 bits.
        let group = ((size >> (7 * i)) & 0x7f) as u8;
        out.push(if i > 0 { group | 0x80 } else { group });
    }
}

#[cfg(test)]
#[path = "aac_test.rs"]
mod tests;
