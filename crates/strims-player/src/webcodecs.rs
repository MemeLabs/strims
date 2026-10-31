// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What WebCodecs decoders take of a decoder config, and the durations
//! audio payloads carry.

use strims_video::{CodecParams, DecoderConfig};

/// A config's codec string as a WebCodecs `VideoDecoderConfig` or
/// `AudioDecoderConfig` takes it: Opus is `opus`, the WebCodecs codec
/// registry's name; other codecs keep their RFC 6381 string.
pub fn codec(config: &DecoderConfig) -> &str {
    if config.codec.eq_ignore_ascii_case("opus") {
        "opus"
    } else {
        &config.codec
    }
}

/// A config's WebCodecs `description`: the avcC or hvcC record, under
/// which the decoder takes length-prefixed NAL units (the unit format's
/// H.264 and H.265 payloads); the AudioSpecificConfig, under which it
/// takes raw AAC frames; or the OpusHead, from which decoders take the
/// channel mapping and drop the pre-skip at their start. Empty for a codec
/// without one, AV1 among them.
pub fn description(config: &DecoderConfig) -> &[u8] {
    match &config.params {
        CodecParams::Avc { avcc, .. } => avcc,
        CodecParams::Hevc { hvcc, .. } => hvcc,
        CodecParams::Aac { asc, .. } => asc,
        CodecParams::Opus { head, .. } => head,
        _ => &[],
    }
}

/// The 48 kHz samples an Opus packet decodes to; see
/// [`strims_video::opus::packet_samples`].
pub use strims_video::opus::packet_samples as opus_packet_samples;

#[cfg(test)]
mod tests {
    use strims_video::synth::script::{aac_lc_asc, av1c, avcc, hvcc};
    use strims_video::{TrackId, TrackKind};

    use super::*;

    fn config(codec: &str, params: CodecParams) -> DecoderConfig {
        DecoderConfig {
            track: TrackId(1),
            kind: TrackKind::Audio,
            codec: codec.into(),
            timescale: 48_000,
            params,
        }
    }

    #[test]
    fn opus_is_named_as_webcodecs_names_it() {
        let opus = config(
            "Opus",
            CodecParams::Opus {
                head: b"OpusHead".to_vec(),
                channels: 2,
                pre_skip: 312,
            },
        );
        assert_eq!(codec(&opus), "opus");
        let aac = config(
            "mp4a.40.2",
            CodecParams::Aac {
                asc: aac_lc_asc(48_000),
                sample_rate: 48_000,
                channels: 2,
                frame_samples: 1024,
            },
        );
        assert_eq!(codec(&aac), "mp4a.40.2");
    }

    #[test]
    fn descriptions_are_the_configuration_records() {
        let avc = avcc(640, 360);
        let video = DecoderConfig {
            kind: TrackKind::Video,
            codec: "avc1.64001f".into(),
            timescale: 90_000,
            ..config(
                "",
                CodecParams::Avc {
                    avcc: avc.clone(),
                    width: 640,
                    height: 360,
                },
            )
        };
        assert_eq!(description(&video), avc.as_slice());
        let asc = aac_lc_asc(44_100);
        let aac = config(
            "mp4a.40.2",
            CodecParams::Aac {
                asc: asc.clone(),
                sample_rate: 44_100,
                channels: 2,
                frame_samples: 1024,
            },
        );
        assert_eq!(description(&aac), asc.as_slice());
        let head = b"OpusHead\x01\x02\x38\x01\x80\xbb\0\0\0\0\0".to_vec();
        let opus = config(
            "Opus",
            CodecParams::Opus {
                head: head.clone(),
                channels: 2,
                pre_skip: 312,
            },
        );
        assert_eq!(description(&opus), head.as_slice());
        let hevc = hvcc(1280, 720);
        let h265 = DecoderConfig {
            params: CodecParams::Hevc {
                hvcc: hevc.clone(),
                width: 1280,
                height: 720,
            },
            ..video.clone()
        };
        assert_eq!(description(&h265), hevc.as_slice());
        let av1 = DecoderConfig {
            params: CodecParams::Av1 {
                av1c: av1c(1280, 720),
                width: 1280,
                height: 720,
            },
            ..video
        };
        assert!(description(&av1).is_empty());
    }
}
