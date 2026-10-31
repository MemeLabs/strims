// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use crate::media::{TrackId, TrackKind};

use super::*;

/// An AAC-LC AudioSpecificConfig at 48 kHz of `channels` (a channel
/// configuration), with 960-sample frames when `short`.
fn lc(channels: u8, short: bool) -> Vec<u8> {
    vec![0x11, 0x80 | (channels << 3) | (u8::from(short) << 2)]
}

fn aac_config(asc: Vec<u8>) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc,
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

/// An OpusHead of `channels`; a mapping family 1 one past two channels.
fn opus_config(channels: u8) -> DecoderConfig {
    let mut head = b"OpusHead".to_vec();
    head.extend_from_slice(&[1, channels, 0x38, 0x01, 0x80, 0xbb, 0, 0, 0, 0]);
    head.push(u8::from(channels > 2));
    if channels > 2 {
        // 5.1: four streams, two of them coupled.
        head.extend_from_slice(&[4, 2, 0, 4, 1, 2, 3, 5]);
    }
    DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "Opus".into(),
        timescale: 48_000,
        params: CodecParams::Opus {
            head,
            channels,
            pre_skip: 312,
        },
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn aac_frames_empty_every_channel_element_of_their_configuration() {
    // Each decodes to zeros in ffmpeg (tests/ffmpeg.rs decodes them in
    // a stream).
    let expected = [
        "00c80007",
        "2064000190000e",
        "00c800010320000c800070",
        "00c800010320000c80000164000380",
        "00c800010320000c800011320000c80007",
        "00c800010320000c800011320000c8000306400038",
        "00c800010320000c800011320000c800012320000c80003064000380",
    ];
    for (c, want) in (1..=7).zip(expected) {
        let s = Silence::for_config(&aac_config(lc(c, false))).unwrap();
        assert_eq!(s.frames.len(), 1);
        assert_eq!(hex(&s.frames[0].0), want, "channel configuration {c}");
        assert_eq!(s.step(), 1024);
    }
    let s = Silence::for_config(&aac_config(lc(2, true))).unwrap();
    assert_eq!(
        (hex(&s.frames[0].0).as_str(), s.step()),
        ("2064000190000e", 960)
    );
}

#[test]
fn he_aac_and_program_config_channels_have_no_silence() {
    // Explicit SBR: 24 kHz core, 48 kHz out, stereo LC.
    assert_eq!(
        Silence::for_config(&aac_config(vec![0x2b, 0x11, 0x88, 0x00])),
        None
    );
    assert_eq!(Silence::for_config(&aac_config(lc(0, false))), None);
    assert_eq!(Silence::for_config(&aac_config(vec![0x11])), None);
}

#[test]
fn opus_packets_hold_a_silent_celt_frame_per_stream() {
    let mono = Silence::for_config(&opus_config(1)).unwrap();
    let got: Vec<(String, u32)> = mono.frames.iter().map(|(p, n)| (hex(p), *n)).collect();
    assert_eq!(
        got,
        [
            ("f8fffe".into(), 960),
            ("f0fffe".into(), 480),
            ("e8fffe".into(), 240),
            ("e0fffe".into(), 120)
        ]
    );
    let stereo = Silence::for_config(&opus_config(2)).unwrap();
    assert_eq!(hex(&stereo.frames[0].0), "fcfffe");
    let surround = Silence::for_config(&opus_config(6)).unwrap();
    assert_eq!(hex(&surround.frames[0].0), "fc02fffefc02fffef802fffef8fffe");
    for (packet, samples) in &surround.frames {
        assert_eq!(crate::opus::packet_samples(packet), Some(*samples));
    }
    let mut broken = opus_config(2);
    if let CodecParams::Opus { head, .. } = &mut broken.params {
        head.truncate(10);
    }
    assert_eq!(Silence::for_config(&broken), None);
}

#[test]
fn a_cover_rounds_to_the_nearest_step_and_ends_with_its_longest_frames() {
    let aac = Silence::for_config(&aac_config(lc(2, false))).unwrap();
    let lens = |s: &Silence, n: i64| s.cover(n).map(|(_, d)| d).collect::<Vec<_>>();
    assert_eq!(lens(&aac, 511), Vec::<u32>::new());
    assert_eq!(lens(&aac, 512), [1024]);
    assert_eq!(lens(&aac, 1536), [1024, 1024]);
    assert_eq!(lens(&aac, -5000), Vec::<u32>::new());
    let opus = Silence::for_config(&opus_config(2)).unwrap();
    assert_eq!(lens(&opus, 2 * 960 + 480 + 120 + 50), [120, 480, 960, 960]);
    assert_eq!(lens(&opus, 2 * 960 + 480 + 240 + 50), [240, 480, 960, 960]);
    assert_eq!(lens(&opus, 59), Vec::<u32>::new());
    assert_eq!(lens(&opus, 60), [120]);
}
