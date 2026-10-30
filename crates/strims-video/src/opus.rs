// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Opus decoder configurations: the identification header, OpusHead
//! (RFC 7845 5.1), and the `dOps` box that carries it in MP4
//! ("Encapsulation of Opus in ISO Base Media File Format" 4.3.2).
//!
//! An Opus track always decodes at 48 kHz, so its timestamps count 48 kHz
//! samples. Its codec string is `Opus`, the sample entry's FourCC, which
//! MP4 media sources take; WebCodecs names the codec `opus`.

use crate::media::MAX_CONFIG_LEN;

/// The RFC 6381 codec string of Opus in MP4.
pub const CODEC: &str = "Opus";
/// The rate Opus decodes at, and the timescale of an Opus track.
pub const SAMPLE_RATE: u32 = 48_000;

/// The header's first 8 bytes.
const MAGIC: &[u8; 8] = b"OpusHead";
/// Bytes of a header before its channel mapping table.
const HEAD_LEN: usize = 19;
/// Versions of the header's major version 0: the upper 4 bits name the
/// major version, and minor versions only append fields.
const MAX_VERSION: u8 = 0x0f;
/// Channels of mapping family 1, the Vorbis channel orders.
const MAX_VORBIS_CHANNELS: u8 = 8;
/// A channel mapping table entry for a channel that is silent.
const SILENT: u8 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OpusError {
    #[error("OpusHead is longer than {MAX_CONFIG_LEN} bytes")]
    TooLong,
    #[error("OpusHead is truncated")]
    Truncated,
    #[error("the identification header does not start with OpusHead")]
    Magic,
    /// A major version other than 0.
    #[error("OpusHead version {0} is not supported")]
    Version(u8),
    #[error("OpusHead has no channels")]
    NoChannels,
    /// More channels than the mapping family allows.
    #[error("{channels} channels are too many for channel mapping family {family}")]
    Channels { family: u8, channels: u8 },
    /// A family other than 0 (mono or stereo), 1 (Vorbis orders) or 255
    /// (unspecified).
    #[error("channel mapping family {0} is not supported")]
    Family(u8),
    #[error("the channel mapping has no streams")]
    NoStreams,
    #[error("{coupled} coupled streams are more than the {streams} streams")]
    Coupled { streams: u8, coupled: u8 },
    /// A mapping table entry names a decoded channel past the streams'.
    #[error("channel {channel} maps to decoded channel {index}, past the streams'")]
    Mapping { channel: u8, index: u8 },
}

/// A parsed OpusHead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpusHead<'a> {
    /// Major version 0, minor version in the lower 4 bits.
    pub version: u8,
    /// The output channel count.
    pub channels: u8,
    /// 48 kHz samples to discard from the decoder's output when it starts.
    pub pre_skip: u16,
    /// The encoder's input rate; it doesn't change how the stream decodes.
    pub input_sample_rate: u32,
    /// Q7.8 dB.
    pub output_gain: i16,
    pub mapping_family: u8,
    /// Opus streams in each packet: 1 for family 0.
    pub streams: u8,
    /// Of those, the stereo ones: `channels - 1` for family 0.
    pub coupled: u8,
    /// The channel mapping table, a decoded channel per output channel;
    /// empty for family 0.
    pub mapping: &'a [u8],
}

/// Parses an OpusHead of at most [`MAX_CONFIG_LEN`] bytes. Bytes after its
/// fields, a later minor version's, are ignored.
pub fn parse_opus_head(data: &[u8]) -> Result<OpusHead<'_>, OpusError> {
    if data.len() > MAX_CONFIG_LEN {
        return Err(OpusError::TooLong);
    }
    let head = data.get(..HEAD_LEN).ok_or(OpusError::Truncated)?;
    let field = |at: usize, n: usize| head.get(at..at + n).unwrap_or_default();
    if field(0, 8) != MAGIC {
        return Err(OpusError::Magic);
    }
    let version = head.get(8).copied().unwrap_or_default();
    if version > MAX_VERSION {
        return Err(OpusError::Version(version));
    }
    let channels = head.get(9).copied().unwrap_or_default();
    if channels == 0 {
        return Err(OpusError::NoChannels);
    }
    let le16 = |at| u16::from_le_bytes(field(at, 2).try_into().unwrap_or_default());
    let pre_skip = le16(10);
    let input_sample_rate = u32::from_le_bytes(field(12, 4).try_into().unwrap_or_default());
    let output_gain = i16::from_le_bytes(field(16, 2).try_into().unwrap_or_default());
    let mapping_family = head.get(18).copied().unwrap_or_default();
    let too_many = OpusError::Channels {
        family: mapping_family,
        channels,
    };
    let (streams, coupled, mapping) = match mapping_family {
        0 if channels > 2 => return Err(too_many),
        0 => (1, channels - 1, &[][..]),
        1 if channels > MAX_VORBIS_CHANNELS => return Err(too_many),
        1 | SILENT => {
            let at = HEAD_LEN + 2 + usize::from(channels);
            let Some(&[streams, coupled, ref mapping @ ..]) = data.get(HEAD_LEN..at) else {
                return Err(OpusError::Truncated);
            };
            if streams == 0 {
                return Err(OpusError::NoStreams);
            }
            if coupled > streams {
                return Err(OpusError::Coupled { streams, coupled });
            }
            // Every stream decodes to one channel, a coupled one to two.
            let decoded = u16::from(streams) + u16::from(coupled);
            if decoded > 255 {
                return Err(OpusError::Coupled { streams, coupled });
            }
            for (channel, &index) in (0u8..).zip(mapping) {
                if index != SILENT && u16::from(index) >= decoded {
                    return Err(OpusError::Mapping { channel, index });
                }
            }
            (streams, coupled, mapping)
        }
        family => return Err(OpusError::Family(family)),
    };
    Ok(OpusHead {
        version,
        channels,
        pre_skip,
        input_sample_rate,
        output_gain,
        mapping_family,
        streams,
        coupled,
        mapping,
    })
}

/// Appends the payload of a `dOps` box (everything after its 8-byte
/// header) for the OpusHead `head`: version 0, then the header's fields
/// big-endian, with the stream counts and mapping table when the mapping
/// family isn't 0.
pub fn write_dops(head: &[u8], out: &mut Vec<u8>) -> Result<(), OpusError> {
    let h = parse_opus_head(head)?;
    out.push(0);
    out.push(h.channels);
    out.extend_from_slice(&h.pre_skip.to_be_bytes());
    out.extend_from_slice(&h.input_sample_rate.to_be_bytes());
    out.extend_from_slice(&h.output_gain.to_be_bytes());
    out.push(h.mapping_family);
    if h.mapping_family != 0 {
        out.push(h.streams);
        out.push(h.coupled);
        out.extend_from_slice(h.mapping);
    }
    Ok(())
}

#[cfg(test)]
#[path = "opus_test.rs"]
mod tests;
