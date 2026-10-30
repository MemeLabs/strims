// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The media model shared by sources and muxers: tracks, their decoder
//! configs and access units.

use std::fmt;

use strims_base::time::Time;

/// The largest avcC record, AudioSpecificConfig or OpusHead accepted.
pub const MAX_CONFIG_LEN: usize = 64 * 1024;

/// A track of a stream; the fMP4 track id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrackKind {
    Video,
    Audio,
}

impl fmt::Display for TrackKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TrackKind::Video => "video",
            TrackKind::Audio => "audio",
        })
    }
}

/// What a decoder needs to decode a track, and the timescale of the
/// track's timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoderConfig {
    pub track: TrackId,
    pub kind: TrackKind,
    /// The RFC 6381 codec string, such as `avc1.64001f`, `mp4a.40.2` or
    /// `Opus`.
    pub codec: String,
    /// Timestamp units per second.
    pub timescale: u32,
    pub params: CodecParams,
}

/// A codec's configuration record and the properties a container declares
/// from it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecParams {
    /// H.264.
    Avc {
        /// The AVCDecoderConfigurationRecord as received.
        avcc: Vec<u8>,
        /// The picture size after cropping, in pixels.
        width: u16,
        height: u16,
    },
    /// AAC.
    Aac {
        /// The AudioSpecificConfig as received.
        asc: Vec<u8>,
        /// The output sample rate: the SBR rate for explicitly signalled
        /// HE-AAC. It is the track's timescale.
        sample_rate: u32,
        /// The output channel count; 0 when a program config element in
        /// the stream defines the channels.
        channels: u8,
        /// Output samples per frame.
        frame_samples: u32,
    },
    /// Opus, at [`opus::SAMPLE_RATE`](crate::opus::SAMPLE_RATE), the
    /// track's timescale.
    Opus {
        /// The OpusHead identification header as received.
        head: Vec<u8>,
        /// The output channel count.
        channels: u8,
        /// 48 kHz samples the decoder's output drops when it starts.
        pre_skip: u16,
    },
}

/// One coded frame of a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessUnit<'a> {
    pub track: TrackId,
    /// Decode time in the track's timescale.
    pub dts: i64,
    /// Presentation time in the track's timescale.
    pub pts: i64,
    /// Decoding can start at this unit.
    pub keyframe: bool,
    pub payload: &'a [u8],
    /// When the source received the unit, on the writer's Unix clock.
    pub published: Time,
}

/// A consumer of a source's decoder configs and access units.
///
/// A track's config comes before its units, and a later config for the
/// track applies from the next unit on. Each track's units come in decode
/// order, with non-decreasing DTS.
pub trait Muxer {
    type Error;

    /// Sets or replaces the decoder config of `config.track`.
    fn set_config(&mut self, config: &DecoderConfig) -> Result<(), Self::Error>;

    /// Takes the next access unit of a track whose config is set.
    fn push(&mut self, unit: &AccessUnit<'_>) -> Result<(), Self::Error>;

    /// Ends the stream, writing out what the muxer still holds.
    fn finish(&mut self) -> Result<(), Self::Error>;
}

/// The MIME type of an fMP4 stream of these tracks, with the codecs
/// parameter quoted per RFC 6381: video codecs first, then audio, each in
/// config order. The parameter is left out when there are no configs.
///
/// `video/mp4;codecs="avc1.64001f,mp4a.40.2"`
pub fn fmp4_mime(configs: &[DecoderConfig]) -> String {
    with_codecs("video/mp4", configs)
}

/// The MIME type of a unit-format stream of these tracks
/// ([`units`](crate::units)), with the codecs parameter as
/// [`fmp4_mime`] writes it.
///
/// `video/x.strims-units;codecs="avc1.64001f,mp4a.40.2"`
pub fn units_mime(configs: &[DecoderConfig]) -> String {
    with_codecs(crate::units::MIME_TYPE, configs)
}

/// `essence` with the codecs parameter of `configs`, if any.
fn with_codecs(essence: &str, configs: &[DecoderConfig]) -> String {
    let mut mime = String::from(essence);
    let mut codecs = 0;
    for kind in [TrackKind::Video, TrackKind::Audio] {
        for config in configs.iter().filter(|c| c.kind == kind) {
            mime.push_str(if codecs == 0 { ";codecs=\"" } else { "," });
            mime.push_str(&config.codec);
            codecs += 1;
        }
    }
    if codecs > 0 {
        mime.push('"');
    }
    mime
}

/// The essence of MIME type `mime`, its type and subtype without
/// parameters or surrounding whitespace.
pub fn mime_essence(mime: &str) -> &str {
    mime.split(';').next().unwrap_or_default().trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(track: u32, kind: TrackKind, codec: &str) -> DecoderConfig {
        DecoderConfig {
            track: TrackId(track),
            kind,
            codec: codec.into(),
            timescale: 90_000,
            params: CodecParams::Avc {
                avcc: Vec::new(),
                width: 16,
                height: 16,
            },
        }
    }

    #[test]
    fn fmp4_mime_lists_video_codecs_first_quoted() {
        let configs = [
            config(2, TrackKind::Audio, "mp4a.40.2"),
            config(1, TrackKind::Video, "avc1.64001f"),
        ];
        assert_eq!(
            fmp4_mime(&configs),
            r#"video/mp4;codecs="avc1.64001f,mp4a.40.2""#
        );
    }

    #[test]
    fn fmp4_mime_of_one_track() {
        assert_eq!(
            fmp4_mime(&[config(1, TrackKind::Video, "avc1.42c01e")]),
            r#"video/mp4;codecs="avc1.42c01e""#
        );
        assert_eq!(
            fmp4_mime(&[config(2, TrackKind::Audio, "mp4a.40.5")]),
            r#"video/mp4;codecs="mp4a.40.5""#
        );
    }

    #[test]
    fn fmp4_mime_without_configs_has_no_codecs_parameter() {
        assert_eq!(fmp4_mime(&[]), "video/mp4");
    }

    #[test]
    fn units_mime_names_the_unit_format_with_the_codecs() {
        let configs = [
            config(2, TrackKind::Audio, "mp4a.40.2"),
            config(1, TrackKind::Video, "avc1.64001f"),
        ];
        assert_eq!(
            units_mime(&configs),
            r#"video/x.strims-units;codecs="avc1.64001f,mp4a.40.2""#
        );
        assert_eq!(units_mime(&[]), "video/x.strims-units");
    }

    #[test]
    fn opus_keeps_its_sample_entry_case() {
        let configs = [
            config(2, TrackKind::Audio, crate::opus::CODEC),
            config(1, TrackKind::Video, "avc1.640c1f"),
        ];
        assert_eq!(
            units_mime(&configs),
            r#"video/x.strims-units;codecs="avc1.640c1f,Opus""#
        );
    }

    #[test]
    fn mime_essence_drops_the_parameters() {
        assert_eq!(
            mime_essence(r#"video/mp4;codecs="avc1.64001f""#),
            "video/mp4"
        );
        assert_eq!(
            mime_essence(" video/x.strims-units ; codecs=x"),
            "video/x.strims-units"
        );
        assert_eq!(mime_essence(""), "");
    }

    #[test]
    fn fmp4_mime_keeps_config_order_within_a_kind() {
        let configs = [
            config(3, TrackKind::Video, "avc1.4d401f"),
            config(2, TrackKind::Audio, "mp4a.40.2"),
            config(1, TrackKind::Video, "avc1.64001f"),
        ];
        assert_eq!(
            fmp4_mime(&configs),
            r#"video/mp4;codecs="avc1.4d401f,avc1.64001f,mp4a.40.2""#
        );
    }
}
