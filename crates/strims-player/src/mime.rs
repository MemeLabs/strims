// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The MediaSource type of an init's tracks.

use strims_video::{DecoderConfig, TrackKind};

/// The type a SourceBuffer of these tracks is created with (or changed
/// to): `video/mp4` with any video track, `audio/mp4` without, and the
/// codecs parameter quoted per RFC 6381, video codecs first, then audio,
/// each in config order. An Opus track's codec is `Opus`, as MP4 names
/// it. The parameter is left out when there are no configs.
///
/// `video/mp4;codecs="avc1.64001f,mp4a.40.2"`
pub fn mse_type(configs: &[DecoderConfig]) -> String {
    let video = configs.iter().any(|c| c.kind == TrackKind::Video);
    let mut mime = String::from(if video { "video/mp4" } else { "audio/mp4" });
    let mut codecs = 0;
    for kind in [TrackKind::Video, TrackKind::Audio] {
        for config in configs.iter().filter(|c| c.kind == kind) {
            mime.push_str(if codecs == 0 { ";codecs=\"" } else { "," });
            mime.push_str(mse_codec(&config.codec));
            codecs += 1;
        }
    }
    if codecs > 0 {
        mime.push('"');
    }
    mime
}

/// A config's codec string as MediaSource takes it in MP4: Opus is
/// `Opus`, which WebKit requires in that case.
pub fn mse_codec(codec: &str) -> &str {
    if codec.eq_ignore_ascii_case("opus") {
        "Opus"
    } else {
        codec
    }
}

#[cfg(test)]
mod tests {
    use strims_video::{CodecParams, TrackId};

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
    fn video_streams_are_video_mp4_with_video_codecs_first() {
        let configs = [
            config(2, TrackKind::Audio, "mp4a.40.2"),
            config(1, TrackKind::Video, "avc1.64001f"),
        ];
        assert_eq!(
            mse_type(&configs),
            r#"video/mp4;codecs="avc1.64001f,mp4a.40.2""#
        );
        assert_eq!(
            mse_type(&[config(1, TrackKind::Video, "avc1.42c01e")]),
            r#"video/mp4;codecs="avc1.42c01e""#
        );
    }

    #[test]
    fn audio_only_streams_are_audio_mp4() {
        assert_eq!(
            mse_type(&[config(2, TrackKind::Audio, "mp4a.40.5")]),
            r#"audio/mp4;codecs="mp4a.40.5""#
        );
        assert_eq!(mse_type(&[]), "audio/mp4");
    }

    #[test]
    fn opus_is_named_as_mp4_names_it() {
        assert_eq!(mse_codec("opus"), "Opus");
        assert_eq!(mse_codec("OPUS"), "Opus");
        assert_eq!(mse_codec("mp4a.40.2"), "mp4a.40.2");
        let configs = [
            config(1, TrackKind::Video, "avc1.64001f"),
            config(2, TrackKind::Audio, "opus"),
        ];
        assert_eq!(mse_type(&configs), r#"video/mp4;codecs="avc1.64001f,Opus""#);
    }
}
