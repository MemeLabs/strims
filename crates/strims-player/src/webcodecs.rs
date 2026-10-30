// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What WebCodecs decoders take of a decoder config, and the durations
//! audio payloads carry.

use strims_video::{CodecParams, DecoderConfig};

/// 48 kHz samples in an Opus packet of more is malformed (RFC 6716 3.2.5).
const OPUS_MAX_PACKET_SAMPLES: u32 = 5760;

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

/// A config's WebCodecs `description`: the avcC record, under which the
/// decoder takes length-prefixed NAL units (the unit format's H.264
/// payloads); the AudioSpecificConfig, under which it takes raw AAC
/// frames; or the OpusHead, from which decoders take the channel mapping
/// and drop the pre-skip at their start. Empty for a codec without one.
pub fn description(config: &DecoderConfig) -> &[u8] {
    match &config.params {
        CodecParams::Avc { avcc, .. } => avcc,
        CodecParams::Aac { asc, .. } => asc,
        CodecParams::Opus { head, .. } => head,
        _ => &[],
    }
}

/// The 48 kHz samples an Opus packet decodes to, from its TOC byte and
/// frame count (RFC 6716 3.1 and 3.2); `None` for a packet that is empty,
/// lacks its frame count byte, holds no frames or lasts over 120 ms. A
/// multistream packet's first stream gives its duration, which every
/// stream shares.
pub fn opus_packet_samples(packet: &[u8]) -> Option<u32> {
    let toc = *packet.first()?;
    let config = toc >> 3;
    let frame: u32 = match (config, config % 4) {
        // SILK: 10, 20, 40 and 60 ms.
        (0..=11, 0) => 480,
        (0..=11, 1) => 960,
        (0..=11, 2) => 1920,
        (0..=11, _) => 2880,
        // Hybrid: 10 and 20 ms.
        (12..=15, 0 | 2) => 480,
        (12..=15, _) => 960,
        // CELT: 2.5, 5, 10 and 20 ms.
        (_, 0) => 120,
        (_, 1) => 240,
        (_, 2) => 480,
        (_, _) => 960,
    };
    let frames = match toc & 0x03 {
        0 => 1,
        1 | 2 => 2,
        _ => u32::from(*packet.get(1)? & 0x3f),
    };
    let samples = frame * frames;
    (frames > 0 && samples <= OPUS_MAX_PACKET_SAMPLES).then_some(samples)
}

#[cfg(test)]
mod tests {
    use strims_video::synth::script::{aac_lc_asc, avcc};
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
    }

    #[test]
    fn opus_packets_last_their_frames() {
        // CELT fullband 20 ms, one frame: libopus's default.
        assert_eq!(opus_packet_samples(&[0xfc, 0xff]), Some(960));
        // CELT 2.5 ms, two equal frames.
        assert_eq!(opus_packet_samples(&[(16 << 3) | 1]), Some(240));
        // SILK 60 ms, two frames of different sizes.
        assert_eq!(opus_packet_samples(&[(3 << 3) | 2, 1, 2]), Some(5760));
        // Hybrid 10 ms, code 3 with six frames.
        assert_eq!(opus_packet_samples(&[(12 << 3) | 3, 6]), Some(2880));
        // SILK 40 ms, code 3 with three frames: 120 ms, the most.
        assert_eq!(opus_packet_samples(&[(2 << 3) | 3, 0x83]), Some(5760));
    }

    #[test]
    fn malformed_opus_packets_have_no_duration() {
        assert_eq!(opus_packet_samples(&[]), None);
        // Code 3 without its frame count, or with none.
        assert_eq!(opus_packet_samples(&[0xff]), None);
        assert_eq!(opus_packet_samples(&[0xff, 0x00]), None);
        // Four 60 ms frames: past 120 ms.
        assert_eq!(opus_packet_samples(&[(3 << 3) | 3, 4]), None);
    }
}
