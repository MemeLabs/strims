// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::test_util::{Av1SeqSpec, av1c, av1c_720p, avcc_720p, hvcc_720p, obu};

const NOW: Time = Time::from_unix_millis(1_700_000_000_000);
const LC_44100: [u8; 2] = [0x12, 0x10];
const LC_48000: [u8; 2] = [0x11, 0x90];
const HE_48000: [u8; 4] = [0x2b, 0x11, 0x88, 0x00];

fn legacy_video(frame_type: u8, packet_type: u8, cts: i32, data: &[u8]) -> Vec<u8> {
    let [_, a, b, c] = cts.to_be_bytes();
    let mut body = vec![(frame_type << 4) | CODEC_AVC, packet_type, a, b, c];
    body.extend_from_slice(data);
    body
}

fn ex_video(frame_type: u8, packet_type: u8, fourcc: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = vec![VIDEO_EX_HEADER | (frame_type << 4) | packet_type];
    body.extend_from_slice(fourcc);
    body.extend_from_slice(data);
    body
}

fn legacy_audio(packet_type: u8, data: &[u8]) -> Vec<u8> {
    let mut body = vec![0xaf, packet_type];
    body.extend_from_slice(data);
    body
}

fn ex_audio(packet_type: u8, fourcc: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = vec![(SOUND_EX_HEADER << 4) | packet_type];
    body.extend_from_slice(fourcc);
    body.extend_from_slice(data);
    body
}

/// An [`AccessUnit`] holding a copy of its payload.
#[derive(Debug, PartialEq, Eq)]
struct Unit {
    track: TrackId,
    dts: i64,
    pts: i64,
    keyframe: bool,
    payload: Vec<u8>,
    published: Time,
}

fn unit(e: Result<Option<SourceEvent<'_>>, FlvError>) -> Unit {
    match e {
        Ok(Some(SourceEvent::Unit(u))) => Unit {
            track: u.track,
            dts: u.dts,
            pts: u.pts,
            keyframe: u.keyframe,
            payload: u.payload.to_vec(),
            published: u.published,
        },
        other => panic!("not a unit: {other:?}"),
    }
}

fn config(e: Result<Option<SourceEvent<'_>>, FlvError>) -> DecoderConfig {
    match e {
        Ok(Some(SourceEvent::Config(c))) => c,
        other => panic!("not a config: {other:?}"),
    }
}

/// A source with the 720p video config and an audio config, both sent at
/// `ts`.
fn source_at(ts: u32, asc: &[u8]) -> FlvSource {
    let mut s = FlvSource::new();
    config(s.video(ts, &legacy_video(1, 0, 0, &avcc_720p()), NOW));
    config(s.audio(ts, &legacy_audio(0, asc), NOW));
    s
}

fn source(asc: &[u8]) -> FlvSource {
    source_at(0, asc)
}

#[test]
fn legacy_avc_sequence_header_and_frames() {
    let mut s = FlvSource::new();
    let avcc = avcc_720p();
    let c = config(s.video(0, &legacy_video(1, 0, 0, &avcc), NOW));
    assert_eq!(
        c,
        DecoderConfig {
            track: VIDEO_TRACK,
            kind: TrackKind::Video,
            codec: "avc1.640028".into(),
            timescale: 90_000,
            params: CodecParams::Avc {
                avcc,
                width: 1280,
                height: 720,
            },
        }
    );

    let key = legacy_video(1, 1, 66, b"key");
    assert_eq!(
        unit(s.video(1000, &key, NOW)),
        Unit {
            track: VIDEO_TRACK,
            dts: 90_000,
            pts: 1066 * 90,
            keyframe: true,
            payload: b"key".to_vec(),
            published: NOW,
        }
    );
    let inter = legacy_video(2, 1, 0, b"inter");
    let u = unit(s.video(1033, &inter, NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (1033 * 90, 1033 * 90, false));
    assert_eq!(u.payload, b"inter");

    assert_eq!(s.video(1066, &legacy_video(1, 2, 0, &[]), NOW), Ok(None));
    assert_eq!(s.metrics().skipped, 1);
}

#[test]
fn only_frame_type_one_is_a_keyframe() {
    let mut s = source(&LC_44100);
    for (i, frame_type) in [1u8, 2, 3, 4].into_iter().enumerate() {
        let body = legacy_video(frame_type, 1, 0, b"x");
        let u = unit(s.video(u32::try_from(i).unwrap(), &body, NOW));
        assert_eq!(u.keyframe, frame_type == 1, "{frame_type}");
    }
}

#[test]
fn enhanced_avc_sequence_start_and_frames() {
    let mut s = FlvSource::new();
    let avcc = avcc_720p();
    let c = config(s.video(0, &ex_video(1, VIDEO_SEQUENCE_START, b"avc1", &avcc), NOW));
    assert_eq!(c.codec, "avc1.640028");
    assert_eq!(
        c.params,
        CodecParams::Avc {
            avcc,
            width: 1280,
            height: 720
        }
    );

    // CodedFrames carries a composition time
    let mut coded = vec![0x00, 0x00, 0x21];
    coded.extend_from_slice(b"key");
    let u = unit(s.video(40, &ex_video(1, VIDEO_CODED_FRAMES, b"avc1", &coded), NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (40 * 90, 73 * 90, true));
    assert_eq!(u.payload, b"key");

    // CodedFramesX doesn't
    let u = unit(s.video(80, &ex_video(2, VIDEO_CODED_FRAMES_X, b"avc1", b"p"), NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (80 * 90, 80 * 90, false));
    assert_eq!(u.payload, b"p");

    assert_eq!(
        s.video(90, &ex_video(1, VIDEO_SEQUENCE_END, b"avc1", &[]), NOW),
        Ok(None)
    );
    assert_eq!(
        s.video(90, &ex_video(1, VIDEO_METADATA, b"avc1", b"\x02"), NOW),
        Ok(None)
    );
    assert_eq!(s.metrics().skipped, 2);
}

#[test]
fn enhanced_hevc_sequence_start_and_frames() {
    let mut s = FlvSource::new();
    let hvcc = hvcc_720p();
    let c = config(s.video(0, &ex_video(1, VIDEO_SEQUENCE_START, b"hvc1", &hvcc), NOW));
    assert_eq!(c.codec, "hvc1.1.6.L93.90");
    assert_eq!(
        (c.track, c.kind, c.timescale),
        (VIDEO_TRACK, TrackKind::Video, 90_000)
    );
    assert_eq!(
        c.params,
        CodecParams::Hevc {
            hvcc,
            width: 1280,
            height: 720
        }
    );

    let mut coded = vec![0x00, 0x00, 0x42];
    coded.extend_from_slice(b"key");
    let u = unit(s.video(40, &ex_video(1, VIDEO_CODED_FRAMES, b"hvc1", &coded), NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (40 * 90, 106 * 90, true));
    assert_eq!(u.payload, b"key");
    let u = unit(s.video(80, &ex_video(2, VIDEO_CODED_FRAMES_X, b"hvc1", b"p"), NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (80 * 90, 80 * 90, false));

    // ffmpeg sends its colour info as a metadata command frame
    let metadata = ex_video(
        FRAME_COMMAND,
        VIDEO_METADATA,
        b"hvc1",
        b"\x02\x00\x09colorInfo",
    );
    assert_eq!(s.video(80, &metadata, NOW), Ok(None));
    assert_eq!(s.metrics().skipped, 1);
}

#[test]
fn enhanced_av1_sequence_start_and_frames() {
    let mut s = FlvSource::new();
    let av1c = av1c_720p();
    let c = config(s.video(0, &ex_video(1, VIDEO_SEQUENCE_START, b"av01", &av1c), NOW));
    assert_eq!(c.codec, "av01.0.08M.08");
    assert_eq!(
        c.params,
        CodecParams::Av1 {
            av1c,
            width: 1280,
            height: 720
        }
    );

    // CodedFrames carries no composition time, and the unit loses the
    // temporal delimiter it starts with
    let frame = obu(6, b"key frame");
    let coded = [obu(2, &[]), frame.clone()].concat();
    let u = unit(s.video(40, &ex_video(1, VIDEO_CODED_FRAMES, b"av01", &coded), NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (40 * 90, 40 * 90, true));
    assert_eq!(u.payload, frame);
    let u = unit(s.video(73, &ex_video(2, VIDEO_CODED_FRAMES_X, b"av01", &frame), NOW));
    assert_eq!((u.dts, u.pts, u.keyframe), (73 * 90, 73 * 90, false));

    // a temporal unit of a delimiter alone carries nothing
    let td = ex_video(2, VIDEO_CODED_FRAMES, b"av01", &obu(2, &[]));
    assert_eq!(s.video(90, &td, NOW), Ok(None));
    let ts_start = ex_video(1, VIDEO_MPEG2TS_SEQUENCE_START, b"av01", b"\x80");
    assert_eq!(s.video(90, &ts_start, NOW), Ok(None));
    assert_eq!(s.metrics().skipped, 2);
    // an MPEG-2 TS sequence start of another codec is malformed
    let avc_ts_start = ex_video(1, VIDEO_MPEG2TS_SEQUENCE_START, b"avc1", b"\x80");
    assert!(matches!(
        s.video(90, &avc_ts_start, NOW),
        Err(FlvError::PacketType { packet_type: 5, .. })
    ));
}

#[test]
fn frames_of_another_codec_than_the_config_are_dropped() {
    let mut s = FlvSource::new();
    config(s.video(0, &legacy_video(1, 0, 0, &avcc_720p()), NOW));
    let hevc = ex_video(1, VIDEO_CODED_FRAMES_X, b"hvc1", b"frame");
    assert_eq!(s.video(10, &hevc, NOW), Ok(None));
    assert_eq!(s.metrics().before_config, 1);
    unit(s.video(20, &legacy_video(1, 1, 0, b"avc"), NOW));

    // a sequence start of another codec changes the track's codec
    let c = config(s.video(
        30,
        &ex_video(1, VIDEO_SEQUENCE_START, b"hvc1", &hvcc_720p()),
        NOW,
    ));
    assert_eq!(c.codec, "hvc1.1.6.L93.90");
    assert_eq!(s.video(40, &legacy_video(1, 1, 0, b"avc"), NOW), Ok(None));
    assert_eq!(s.metrics().before_config, 2);
    // and keeps the DTS clamp
    let u = unit(s.video(15, &hevc, NOW));
    assert_eq!(u.dts, 20 * 90);
    assert_eq!(s.metrics().dts_clamped, 1);
}

#[test]
fn composition_time_is_sign_extended() {
    let mut s = source(&LC_44100);
    for (cts, pts_ms) in [
        (-10, 990),
        (0x7f_ffff, 1000 + 0x7f_ffff),
        (-0x80_0000, 1000 - 0x80_0000),
    ] {
        let u = unit(s.video(1000, &legacy_video(2, 1, cts, b"x"), NOW));
        assert_eq!((u.dts, u.pts), (90_000, pts_ms * 90), "{cts}");
    }
    let mut coded = vec![0xff, 0xff, 0xfe];
    coded.push(0);
    let u = unit(s.video(1000, &ex_video(2, VIDEO_CODED_FRAMES, b"avc1", &coded), NOW));
    assert_eq!(u.pts, 998 * 90);
}

#[test]
fn tag_times_unwrap_across_the_32_bit_wrap() {
    let mut s = source_at(u32::MAX - 20, &LC_48000);
    let wrap = 1i64 << 32;
    let u = unit(s.video(u32::MAX - 10, &legacy_video(1, 1, 0, b"a"), NOW));
    assert_eq!(u.dts, (wrap - 11) * 90);
    let u = unit(s.video(5, &legacy_video(2, 1, 20, b"b"), NOW));
    assert_eq!((u.dts, u.pts), ((wrap + 5) * 90, (wrap + 25) * 90));
    // the clock is shared: audio from before the wrap stays before it
    let u = unit(s.audio(u32::MAX - 1, &legacy_audio(1, b"c"), NOW));
    assert_eq!(u.dts, (wrap - 2) * 48);
    let u = unit(s.video(40, &legacy_video(2, 1, 0, b"d"), NOW));
    assert_eq!(u.dts, (wrap + 40) * 90);
}

/// The tag time of audio frame `n` at 44.1 kHz, rounded down.
fn frame_ms(n: i64) -> u32 {
    u32::try_from(n * 1024 * 1000 / 44_100).unwrap()
}

/// An encoder an hour into its stream that starts its clock over: both
/// tracks' tags restart at 0.
#[test]
fn an_encoder_clock_restart_continues_both_tracks_after_the_latest_time() {
    let hour = 3_600_000u32;
    let mut s = source_at(hour - 100, &LC_48000);
    for k in 0..3 {
        unit(s.video(hour - 66 + k * 33, &legacy_video(1, 1, 0, b"v"), NOW));
    }
    let a = unit(s.audio(hour - 10, &legacy_audio(1, b"a"), NOW));
    // the restart: video first, at 0, then audio
    let v = unit(s.video(0, &legacy_video(1, 1, 66, b"v"), NOW));
    assert_eq!(
        v.dts,
        i64::from(hour + 33) * 90,
        "a video step after the latest time"
    );
    assert_eq!(v.pts - v.dts, 66 * 90);
    // audio restarted with video: its tag lands at the same time, two
    // frames on, so its sample count restarts there
    let next = unit(s.audio(0, &legacy_audio(1, b"a"), NOW));
    assert_eq!(next.dts, i64::from(hour + 33) * 48);
    assert!(next.dts > a.dts);
    let v = unit(s.video(33, &legacy_video(2, 1, 0, b"v"), NOW));
    assert_eq!(v.dts, i64::from(hour + 66) * 90);
    let m = s.metrics();
    assert_eq!(
        (m.clock_restarts, m.dts_clamped, m.audio_resyncs),
        (1, 0, 1)
    );

    // a later restart moves the clock again
    unit(s.video(2_000, &legacy_video(1, 1, 0, b"v"), NOW));
    unit(s.video(2_033, &legacy_video(2, 1, 0, b"v"), NOW));
    let v = unit(s.video(0, &legacy_video(1, 1, 0, b"v"), NOW));
    assert_eq!(v.dts, i64::from(hour + 2_066 + 33) * 90);
    assert_eq!(s.metrics().clock_restarts, 2);
}

#[test]
fn steps_back_within_the_bound_and_the_32_bit_wrap_are_not_restarts() {
    let mut s = source_at(u32::MAX - 500, &LC_48000);
    unit(s.video(u32::MAX - 400, &legacy_video(1, 1, 0, b"v"), NOW));
    // the clamp holds a step back of under a second
    let u = unit(s.video(u32::MAX - 1_399, &legacy_video(2, 1, 0, b"v"), NOW));
    assert_eq!(u.dts, (i64::from(u32::MAX) - 400) * 90);
    let u = unit(s.video(100, &legacy_video(2, 1, 0, b"v"), NOW));
    assert_eq!(u.dts, ((1i64 << 32) + 100) * 90);
    let m = s.metrics();
    assert_eq!((m.clock_restarts, m.dts_clamped), (0, 1));
}

#[test]
fn audio_dts_counts_samples() {
    let mut s = source(&LC_44100);
    for n in 0..200 {
        let u = unit(s.audio(frame_ms(n), &legacy_audio(1, b"aac"), NOW));
        assert_eq!(
            u,
            Unit {
                track: AUDIO_TRACK,
                dts: n * 1024,
                pts: n * 1024,
                keyframe: true,
                payload: b"aac".to_vec(),
                published: NOW,
            }
        );
    }
    assert_eq!(s.metrics().audio_resyncs, 0);
}

#[test]
fn audio_resyncs_to_the_tag_time_after_drifting_a_frame() {
    let mut s = source(&LC_44100);
    unit(s.audio(0, &legacy_audio(1, b"a"), NOW));
    // jitter within a frame keeps the count
    let u = unit(s.audio(40, &legacy_audio(1, b"a"), NOW));
    assert_eq!(u.dts, 1024);
    // a gap of more than a frame restarts it at the tag time
    let u = unit(s.audio(10_000, &legacy_audio(1, b"a"), NOW));
    assert_eq!(u.dts, 441_000);
    assert_eq!(s.metrics().audio_resyncs, 1);
    let u = unit(s.audio(10_030, &legacy_audio(1, b"a"), NOW));
    assert_eq!(u.dts, 442_024);
    // a jump back within the restart bound resyncs, and the clamp holds
    // the DTS
    let u = unit(s.audio(9_500, &legacy_audio(1, b"a"), NOW));
    assert_eq!(u.dts, 442_024);
    assert_eq!(s.metrics().audio_resyncs, 2);
    assert_eq!(s.metrics().dts_clamped, 1);
}

#[test]
fn decreasing_video_dts_is_clamped_and_counted() {
    let mut s = source(&LC_44100);
    assert_eq!(
        unit(s.video(100, &legacy_video(1, 1, 0, b"a"), NOW)).dts,
        9000
    );
    let u = unit(s.video(90, &legacy_video(2, 1, 10, b"b"), NOW));
    assert_eq!((u.dts, u.pts), (9000, 9000));
    assert_eq!(s.metrics().dts_clamped, 1);
    assert_eq!(
        unit(s.video(120, &legacy_video(2, 1, 0, b"c"), NOW)).dts,
        10_800
    );
    assert_eq!(s.metrics().dts_clamped, 1);
}

#[test]
fn units_before_their_tracks_config_are_dropped() {
    let mut s = FlvSource::new();
    assert_eq!(s.video(0, &legacy_video(1, 1, 0, b"a"), NOW), Ok(None));
    assert_eq!(s.audio(0, &legacy_audio(1, b"a"), NOW), Ok(None));
    config(s.audio(0, &legacy_audio(0, &LC_44100), NOW));
    unit(s.audio(20, &legacy_audio(1, b"a"), NOW));
    assert_eq!(
        s.video(40, &ex_video(1, VIDEO_CODED_FRAMES_X, b"avc1", b"a"), NOW),
        Ok(None)
    );
    assert_eq!(s.metrics().before_config, 3);
}

#[test]
fn a_repeated_sequence_header_yields_nothing() {
    let mut s = source(&LC_44100);
    let avcc = avcc_720p();
    assert_eq!(s.video(0, &legacy_video(1, 0, 0, &avcc), NOW), Ok(None));
    assert_eq!(s.video(0, &ex_video(1, 0, b"avc1", &avcc), NOW), Ok(None));
    assert_eq!(s.audio(0, &legacy_audio(0, &LC_44100), NOW), Ok(None));
    assert_eq!(s.metrics().skipped, 3);
}

#[test]
fn a_changed_video_config_keeps_the_dts_clamp() {
    let mut s = source(&LC_44100);
    unit(s.video(1000, &legacy_video(1, 1, 0, b"a"), NOW));
    let mut avcc = avcc_720p();
    avcc[3] = 0x1f;
    let c = config(s.video(1000, &legacy_video(1, 0, 0, &avcc), NOW));
    assert_eq!(c.codec, "avc1.64001f");
    assert_eq!(
        unit(s.video(900, &legacy_video(1, 1, 0, b"b"), NOW)).dts,
        90_000
    );
    assert_eq!(s.metrics().dts_clamped, 1);
}

#[test]
fn a_new_audio_rate_restarts_the_sample_clock() {
    let mut s = source(&LC_44100);
    unit(s.audio(1000, &legacy_audio(1, b"a"), NOW));
    let c = config(s.audio(1000, &legacy_audio(0, &LC_48000), NOW));
    assert_eq!(c.timescale, 48_000);
    let u = unit(s.audio(500, &legacy_audio(1, b"b"), NOW));
    assert_eq!(u.dts, 24_000);
    let m = s.metrics();
    assert_eq!((m.audio_resyncs, m.dts_clamped), (0, 0));
}

#[test]
fn he_aac_runs_at_the_sbr_rate() {
    let mut s = FlvSource::new();
    let c = config(s.audio(0, &legacy_audio(0, &HE_48000), NOW));
    assert_eq!(
        c,
        DecoderConfig {
            track: AUDIO_TRACK,
            kind: TrackKind::Audio,
            codec: "mp4a.40.5".into(),
            timescale: 48_000,
            params: CodecParams::Aac {
                asc: HE_48000.to_vec(),
                sample_rate: 48_000,
                channels: 2,
                frame_samples: 2048,
            },
        }
    );
    unit(s.audio(1000, &legacy_audio(1, b"a"), NOW));
    assert_eq!(
        unit(s.audio(1043, &legacy_audio(1, b"b"), NOW)).dts,
        48_000 + 2048
    );
}

#[test]
fn enhanced_aac_sequence_start_and_frames() {
    let mut s = FlvSource::new();
    let c = config(s.audio(0, &ex_audio(AUDIO_SEQUENCE_START, b"mp4a", &LC_48000), NOW));
    assert_eq!((c.codec.as_str(), c.timescale), ("mp4a.40.2", 48_000));
    let u = unit(s.audio(1000, &ex_audio(AUDIO_CODED_FRAMES, b"mp4a", b"aac"), NOW));
    assert_eq!((u.dts, u.payload.as_slice()), (48_000, &b"aac"[..]));
    for skipped in [AUDIO_SEQUENCE_END, AUDIO_MULTICHANNEL_CONFIG] {
        assert_eq!(
            s.audio(1000, &ex_audio(skipped, b"mp4a", b"\x01\x02"), NOW),
            Ok(None)
        );
    }
    assert_eq!(s.metrics().skipped, 2);
}

#[test]
fn command_frames_and_empty_tags_are_skipped() {
    let mut s = source(&LC_44100);
    // legacy command frames, of any codec
    assert_eq!(s.video(0, &[0x57, 0x00], NOW), Ok(None));
    assert_eq!(s.video(0, &[0x5c, 0x01], NOW), Ok(None));
    // an Enhanced RTMP command frame has no FourCC
    assert_eq!(s.video(0, &[0xd1, 0x00], NOW), Ok(None));
    assert_eq!(s.video(0, &[], NOW), Ok(None));
    assert_eq!(s.audio(0, &[], NOW), Ok(None));
    assert_eq!(s.video(0, &legacy_video(1, 1, 0, &[]), NOW), Ok(None));
    assert_eq!(s.audio(0, &legacy_audio(1, &[]), NOW), Ok(None));
    assert_eq!(s.metrics().skipped, 7);
}

fn video_refusal(body: &[u8]) -> String {
    let mut s = FlvSource::new();
    match s.video(0, body, NOW) {
        Err(e @ FlvError::Unsupported(_)) => e.to_string(),
        other => panic!("not refused: {other:?}"),
    }
}

fn audio_refusal(body: &[u8]) -> String {
    let mut s = FlvSource::new();
    match s.audio(0, body, NOW) {
        Err(e @ FlvError::Unsupported(_)) => e.to_string(),
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn enhanced_video_fourccs_other_than_avc1_hvc1_and_av01_are_refused() {
    let sequence_start = |fourcc| ex_video(1, VIDEO_SEQUENCE_START, fourcc, b"\x01");
    assert_eq!(
        video_refusal(&sequence_start(b"vp09")),
        "VP9 (vp09) is not supported; send H.264 with AAC"
    );
    assert_eq!(
        video_refusal(&sequence_start(b"vp08")),
        "VP8 (vp08) is not supported; send H.264 with AAC"
    );
    assert_eq!(
        video_refusal(&ex_video(2, VIDEO_CODED_FRAMES_X, b"x\x00\xff1", &[])),
        r"video codec 'x\x00\xff1' is not supported; send H.264 with AAC"
    );
}

#[test]
fn multitrack_and_mod_ex_are_refused() {
    assert_eq!(
        video_refusal(&[0x80 | 0x10 | VIDEO_MULTITRACK, 0x00, b'a', b'v', b'c', b'1']),
        "multitrack video is not supported; send H.264 with AAC"
    );
    assert_eq!(
        video_refusal(&[0x80 | 0x10 | VIDEO_MOD_EX, 0x00]),
        "Enhanced RTMP ModEx video packets are not supported; send H.264 with AAC"
    );
    assert_eq!(
        audio_refusal(&[0x90 | AUDIO_MULTITRACK, 0x00]),
        "multitrack audio is not supported; send H.264 with AAC"
    );
    assert_eq!(
        audio_refusal(&[0x90 | AUDIO_MOD_EX]),
        "Enhanced RTMP ModEx audio packets are not supported; send H.264 with AAC"
    );
}

#[test]
fn legacy_video_codecs_other_than_avc_are_refused() {
    let cases = [
        (1, "FLV video codec id 1 is not supported"),
        (2, "Sorenson H.263 video is not supported"),
        (3, "Screen video is not supported"),
        (4, "VP6 video is not supported"),
        (5, "VP6 video is not supported"),
        (6, "Screen video is not supported"),
        (12, "HEVC (FLV codec id 12) is not supported"),
        (13, "AV1 (FLV codec id 13) is not supported"),
        (15, "FLV video codec id 15 is not supported"),
    ];
    for (codec, message) in cases {
        assert_eq!(
            video_refusal(&[0x10 | codec, 0x00, 0, 0, 0]),
            format!("{message}; send H.264 with AAC")
        );
    }
}

#[test]
fn legacy_sound_formats_other_than_aac_are_refused() {
    let cases = [
        (0, "PCM audio is not supported"),
        (1, "ADPCM audio is not supported"),
        (2, "MP3 audio is not supported"),
        (3, "PCM audio is not supported"),
        (4, "Nellymoser audio is not supported"),
        (6, "Nellymoser audio is not supported"),
        (7, "G.711 audio is not supported"),
        (8, "G.711 audio is not supported"),
        (11, "Speex audio is not supported"),
        (13, "FLV sound format 13 is not supported"),
        (14, "MP3 audio is not supported"),
        (15, "FLV sound format 15 is not supported"),
    ];
    for (format, message) in cases {
        assert_eq!(
            audio_refusal(&[(format << 4) | 0x0f, 0x00]),
            format!("{message}; send H.264 with AAC")
        );
    }
}

#[test]
fn enhanced_audio_fourccs_other_than_mp4a_are_refused() {
    let cases: [(&[u8; 4], &str); 6] = [
        (b"Opus", "Opus audio is not supported"),
        (b".mp3", "MP3 audio is not supported"),
        (b"fLaC", "FLAC audio is not supported"),
        (b"ac-3", "AC-3 audio is not supported"),
        (b"ec-3", "E-AC-3 audio is not supported"),
        (b"mp4A", "audio codec 'mp4A' is not supported"),
    ];
    for (fourcc, message) in cases {
        assert_eq!(
            audio_refusal(&ex_audio(AUDIO_SEQUENCE_START, fourcc, &[0x01])),
            format!("{message}; send H.264 with AAC")
        );
    }
}

#[test]
fn aac_object_types_other_than_lc_and_he_are_refused() {
    // AAC-LD (23), 48 kHz, stereo
    assert_eq!(
        audio_refusal(&legacy_audio(0, &[0xb9, 0x90])),
        "AAC object type 23 is not supported; send H.264 with AAC-LC or HE-AAC"
    );
}

#[test]
fn malformed_tags_are_errors() {
    let mut s = source(&LC_44100);
    let video = TrackKind::Video;
    let audio = TrackKind::Audio;
    assert_eq!(s.video(0, &[0x17], NOW), Err(FlvError::Truncated(video)));
    assert_eq!(
        s.video(0, &[0x17, 0x01, 0x00], NOW),
        Err(FlvError::Truncated(video))
    );
    assert_eq!(
        s.video(0, &[0x17, 0x00], NOW),
        Err(FlvError::Truncated(video))
    );
    assert_eq!(
        s.video(0, &[0x91, b'a', b'v'], NOW),
        Err(FlvError::Truncated(video))
    );
    assert_eq!(
        s.video(0, &ex_video(1, VIDEO_CODED_FRAMES, b"avc1", &[0]), NOW),
        Err(FlvError::Truncated(video))
    );
    assert_eq!(
        s.video(0, &legacy_video(1, 3, 0, b"x"), NOW),
        Err(FlvError::PacketType {
            kind: video,
            packet_type: 3
        })
    );
    for packet_type in [5, 8, 15] {
        assert_eq!(
            s.video(0, &ex_video(1, packet_type, b"avc1", b"x"), NOW),
            Err(FlvError::PacketType {
                kind: video,
                packet_type
            })
        );
    }
    assert_eq!(s.audio(0, &[0xaf], NOW), Err(FlvError::Truncated(audio)));
    assert_eq!(
        s.audio(0, &[0x90, b'm'], NOW),
        Err(FlvError::Truncated(audio))
    );
    assert_eq!(
        s.audio(0, &legacy_audio(2, b"x"), NOW),
        Err(FlvError::PacketType {
            kind: audio,
            packet_type: 2
        })
    );
    for packet_type in [3, 6, 8] {
        assert_eq!(
            s.audio(0, &ex_audio(packet_type, b"mp4a", b"x"), NOW),
            Err(FlvError::PacketType {
                kind: audio,
                packet_type
            })
        );
    }
    assert_eq!(
        FlvError::Truncated(video).to_string(),
        "video tag is truncated"
    );
    assert_eq!(
        FlvError::PacketType {
            kind: audio,
            packet_type: 3
        }
        .to_string(),
        "audio packet type 3 is not valid"
    );
}

#[test]
fn invalid_sequence_headers_are_errors() {
    let mut s = FlvSource::new();
    let mut avcc = avcc_720p();
    avcc[0] = 0;
    let e = s.video(0, &legacy_video(1, 0, 0, &avcc), NOW).unwrap_err();
    assert_eq!(e, FlvError::Avc(AvcError::Version(0)));
    assert_eq!(
        e.to_string(),
        "invalid AVC sequence header: avcC version 0 is not 1"
    );
    let e = s.audio(0, &legacy_audio(0, &[0x12]), NOW).unwrap_err();
    assert_eq!(e, FlvError::Aac(AacError::Truncated));
    assert_eq!(
        e.to_string(),
        "invalid AAC sequence header: AudioSpecificConfig is truncated"
    );
    let mut hvcc = hvcc_720p();
    hvcc[0] = 2;
    let e = s
        .video(0, &ex_video(1, VIDEO_SEQUENCE_START, b"hvc1", &hvcc), NOW)
        .unwrap_err();
    assert_eq!(e, FlvError::Hevc(HevcError::Version(2)));
    assert_eq!(
        e.to_string(),
        "invalid HEVC sequence header: hvcC version 2 is not 1"
    );
    let e = s
        .video(
            0,
            &ex_video(1, VIDEO_SEQUENCE_START, b"av01", &[0x81, 0x08, 0x0c, 0]),
            NOW,
        )
        .unwrap_err();
    assert_eq!(e, FlvError::Av1(Av1Error::NoSequenceHeader));
    assert_eq!(
        e.to_string(),
        "invalid AV1 sequence header: av1C holds no sequence header OBU"
    );
    let mut long = avcc_720p();
    long.resize(crate::MAX_CONFIG_LEN + 1, 0);
    assert_eq!(
        s.video(0, &ex_video(1, VIDEO_SEQUENCE_START, b"avc1", &long), NOW),
        Err(FlvError::Avc(AvcError::TooLong))
    );
}

#[derive(Debug, Clone)]
enum Tag {
    Video(u32, Vec<u8>),
    Audio(u32, Vec<u8>),
}

/// Bodies that reach each layout's deeper fields, then arbitrary bytes.
fn body(prefixes: Vec<Vec<u8>>) -> impl Strategy<Value = Vec<u8>> {
    (
        prop::sample::select(prefixes),
        proptest::collection::vec(any::<u8>(), 0..48),
    )
        .prop_map(|(mut prefix, tail)| {
            prefix.extend_from_slice(&tail);
            prefix
        })
}

fn tag() -> impl Strategy<Value = Tag> {
    let video = vec![
        vec![],
        legacy_video(1, 0, 0, &avcc_720p()),
        ex_video(1, VIDEO_SEQUENCE_START, b"avc1", &avcc_720p()),
        vec![0x17, 0x00, 0, 0, 0, 1],
        vec![0x17, 0x01],
        vec![0x27, 0x01],
        vec![0x91, b'a', b'v', b'c', b'1'],
        vec![0x93, b'a', b'v', b'c', b'1'],
        vec![0x90, b'a', b'v', b'c', b'1', 1],
        ex_video(1, VIDEO_SEQUENCE_START, b"hvc1", &hvcc_720p()),
        ex_video(1, VIDEO_SEQUENCE_START, b"av01", &av1c_720p()),
        ex_video(
            1,
            VIDEO_SEQUENCE_START,
            b"av01",
            &av1c(&Av1SeqSpec::main(64, 64).build()),
        ),
        vec![0x91, b'h', b'v', b'c', b'1'],
        vec![0x93, b'h', b'v', b'c', b'1'],
        vec![0x91, b'a', b'v', b'0', b'1', 0x12, 0x00],
        vec![0x93, b'a', b'v', b'0', b'1'],
        vec![0x95, b'a', b'v', b'0', b'1'],
        vec![0x80],
    ];
    let audio = vec![
        vec![],
        legacy_audio(0, &LC_44100),
        legacy_audio(0, &HE_48000),
        ex_audio(0, b"mp4a", &LC_48000),
        vec![0xaf, 0x00],
        vec![0xaf, 0x01],
        vec![0x91, b'm', b'p', b'4', b'a'],
        vec![0x90],
    ];
    let ts = prop_oneof![any::<u32>(), 0u32..2000, (u32::MAX - 2000)..=u32::MAX];
    prop_oneof![
        (ts.clone(), body(video)).prop_map(|(t, b)| Tag::Video(t, b)),
        (ts, body(audio)).prop_map(|(t, b)| Tag::Audio(t, b)),
    ]
}

fn proptest_config() -> Config {
    Config {
        cases: 512,
        rng_seed: RngSeed::Fixed(0xf1f0_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    // Each track's units come with non-decreasing DTS while its timescale
    // holds, and with a payload taken from the end of the tag's body.
    #[test]
    fn arbitrary_tags_never_panic(tags in proptest::collection::vec(tag(), 1..64)) {
        let mut s = FlvSource::new();
        let mut last = [None::<i64>; 2];
        let mut timescale = [0; 2];
        for tag in &tags {
            let (track, body, result) = match tag {
                Tag::Video(t, b) => (0, b, s.video(*t, b, NOW)),
                Tag::Audio(t, b) => (1, b, s.audio(*t, b, NOW)),
            };
            match result {
                Ok(Some(SourceEvent::Unit(u))) => {
                    prop_assert!(last[track].is_none_or(|l| u.dts >= l));
                    last[track] = Some(u.dts);
                    prop_assert!(!u.payload.is_empty());
                    prop_assert!(body.ends_with(u.payload));
                }
                Ok(Some(SourceEvent::Config(c))) => {
                    prop_assert!(c.timescale > 0);
                    if c.timescale != timescale[track] {
                        timescale[track] = c.timescale;
                        last[track] = None;
                    }
                }
                Ok(None) | Err(_) => {}
            }
        }
    }

    // An encoder that starts its clock over at any frame, to any time at
    // least two seconds back, never stalls a track: each one's DTS rises
    // with every frame, and every restart is counted.
    #[test]
    fn an_encoder_restarting_its_clock_never_stalls_a_track(
        start: u32,
        frames in 2usize..240,
        restarts in proptest::collection::vec((0usize..240, 2_000u32..u32::MAX / 2), 0..4),
    ) {
        let mut s = source_at(start, &LC_48000);
        let mut t = start;
        let mut last = [None::<i64>; 2];
        let mut expected = 0;
        for frame in 0..frames {
            if let Some(&(_, back)) = restarts.iter().find(|(at, _)| *at == frame) {
                t = t.wrapping_sub(back);
                expected += 1;
            }
            let video = unit(s.video(t, &legacy_video(2, 1, 0, b"v"), NOW));
            let audio = unit(s.audio(t.wrapping_add(5), &legacy_audio(1, b"a"), NOW));
            for (i, dts) in [video.dts, audio.dts].into_iter().enumerate() {
                prop_assert!(last[i].is_none_or(|l| dts > l), "frame {frame}");
                last[i] = Some(dts);
            }
            t = t.wrapping_add(33);
        }
        prop_assert_eq!(s.metrics().clock_restarts, expected);
        prop_assert_eq!(s.metrics().dts_clamped, 0);
    }
}
