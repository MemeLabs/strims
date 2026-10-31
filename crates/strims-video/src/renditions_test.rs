// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::media::TrackId;
use crate::synth::script::{av1c, avcc, hvcc};

fn video(params: CodecParams) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(1),
        kind: TrackKind::Video,
        codec: String::new(),
        timescale: 90_000,
        params,
    }
}

fn h264(width: u16, height: u16) -> DecoderConfig {
    video(CodecParams::Avc {
        avcc: avcc(width, height),
        width,
        height,
    })
}

fn h265(width: u16, height: u16) -> DecoderConfig {
    video(CodecParams::Hevc {
        hvcc: hvcc(width, height),
        width,
        height,
    })
}

fn ladder() -> TranscodeSettings {
    TranscodeSettings {
        ladder: vec![rung(720, 60, 3_000_000), rung(480, 30, 1_200_000)],
        ..TranscodeSettings::default()
    }
}

fn ids(p: &Planned) -> Vec<&str> {
    p.outputs.iter().map(|(id, _)| id.as_str()).collect()
}

#[test]
fn an_h264_stream_gets_the_rungs_below_its_shorter_side() {
    let p = plan(&h264(1920, 1080), &ladder());
    assert!(!p.main_transcoded);
    assert_eq!(ids(&p), ["720p60", "480p30"]);
    assert_eq!(p.transcode.unwrap().renditions, ladder().ladder);

    let p = plan(&h264(1280, 720), &ladder());
    assert_eq!(ids(&p), ["480p30"]);
    // a portrait picture's shorter side is its width
    let p = plan(&h264(720, 1280), &ladder());
    assert_eq!(ids(&p), ["480p30"]);
    let p = plan(&h264(854, 480), &ladder());
    assert_eq!((p.transcode, p.outputs.len()), (None, 0));
    // no ladder, no transcode
    let p = plan(&h264(1920, 1080), &TranscodeSettings::default());
    assert_eq!((p.transcode, p.main_transcoded), (None, false));
}

#[test]
fn hevc_and_av1_streams_get_h264_and_keep_their_own_as_a_rendition() {
    let defaults = TranscodeSettings::default();
    for (source, side, bitrate) in [
        (h265(3840, 2160), 1080, 6_000_000),
        (h265(1920, 1080), 1080, 6_000_000),
        (h265(1280, 720), 720, 4_000_000),
        (h265(640, 360), 360, 1_500_000),
        (
            video(CodecParams::Av1 {
                av1c: av1c(1280, 720),
                width: 1280,
                height: 720,
            }),
            720,
            4_000_000,
        ),
    ] {
        let p = plan(&source, &defaults);
        assert!(p.main_transcoded);
        assert_eq!(ids(&p), [SOURCE_ID]);
        assert_eq!(
            p.transcode.unwrap().renditions,
            [rung(side, 60, bitrate)],
            "{:?}",
            source.params.picture_size()
        );
    }
    // the ladder's rungs below the H.264 stream follow it
    let p = plan(&h265(1920, 1080), &ladder());
    assert_eq!(ids(&p), [SOURCE_ID, "720p60", "480p30"]);
    let renditions = p.transcode.unwrap().renditions;
    assert_eq!(renditions[0], rung(1080, 60, 6_000_000));
    assert_eq!(&renditions[1..], &ladder().ladder[..]);
    let p = plan(&h265(1280, 720), &ladder());
    assert_eq!(ids(&p), [SOURCE_ID, "480p30"]);
}

#[test]
fn a_transcode_makes_at_most_max_renditions() {
    let settings = TranscodeSettings {
        ladder: (0..20).map(|i| rung(100 + i, 30, 100_000)).collect(),
        ..TranscodeSettings::default()
    };
    let p = plan(&h264(1920, 1080), &settings);
    assert_eq!(
        p.transcode.unwrap().renditions.len(),
        crate::codec::MAX_RENDITIONS
    );
    let p = plan(&h265(1920, 1080), &settings);
    assert_eq!(
        p.transcode.unwrap().renditions.len(),
        crate::codec::MAX_RENDITIONS
    );
    assert_eq!(p.outputs.len(), crate::codec::MAX_RENDITIONS);
}

#[test]
fn rendition_keys_are_the_channel_keys_hmac_of_the_label_and_id() {
    let channel = strims_crypto::key_from_seed(&[7; 32]);
    let other = strims_crypto::key_from_seed(&[8; 32]);
    let key = rendition_key(&channel, "480p30");
    assert_eq!(key, rendition_key(&channel, "480p30"));
    assert_ne!(key.public, rendition_key(&channel, "720p60").public);
    assert_ne!(key.public, rendition_key(&other, "480p30").public);
    assert_ne!(key.public, channel.public);
    let mut mac = Hmac::<Sha256>::new_from_slice(&channel.private).unwrap();
    mac.update(b"strims-video-rendition:480p30");
    let seed: [u8; 32] = mac.finalize().into_bytes().into();
    assert_eq!(key, strims_crypto::key_from_seed(&seed));
    assert_eq!(rung_id(&rung(1080, 60, 0)), "1080p60");
}

#[test]
fn transcode_places_are_capped_and_freed_when_dropped() {
    let running = Rc::new(Cell::new(0));
    let a = TranscodePlace::take(&running, 2).unwrap();
    let b = TranscodePlace::take(&running, 2).unwrap();
    assert!(TranscodePlace::take(&running, 2).is_none());
    drop(a);
    assert_eq!(running.get(), 1);
    let _c = TranscodePlace::take(&running, 2).unwrap();
    drop(b);
    assert_eq!(running.get(), 1);
}

#[test]
fn listings_name_their_codecs_size_and_rates() {
    let configs = [DecoderConfig {
        codec: "avc1.64001f".into(),
        ..h264(854, 480)
    }];
    let l = listing(&configs, "magnet:?xt=urn:ppspp:x", 1_328_000, 30.0);
    assert_eq!(
        (
            l.mime_type.as_str(),
            l.width,
            l.height,
            l.bitrate,
            l.frame_rate
        ),
        (
            r#"video/x.strims-units;codecs="avc1.64001f""#,
            854,
            480,
            1_328_000,
            30.0
        )
    );
    assert_eq!(listing(&[], "u", 0, 0.0).width, 0);
}
