// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::media::{CodecParams, TrackKind};
use crate::synth::script::avcc;

/// The OpusHead every WebCodecs encoder tested gives for stereo.
const OPUS_HEAD: [u8; 19] = *b"OpusHead\x01\x02\x38\x01\x80\xbb\0\0\0\0\0";

fn h264(track: u32) -> CaptureConfig {
    CaptureConfig {
        track,
        codec: Codec::H264 as i32,
        config: avcc(1280, 720),
    }
}

fn opus_config(track: u32) -> CaptureConfig {
    CaptureConfig {
        track,
        codec: Codec::Opus as i32,
        config: OPUS_HEAD.to_vec(),
    }
}

fn unit(track: u32, timestamp_us: i64, keyframe: bool) -> CaptureUnit {
    CaptureUnit {
        track,
        timestamp_us,
        keyframe,
        data: vec![track as u8; 10],
    }
}

fn writer() -> MediaWriter {
    MediaWriter {
        units: UnitWriter::new(4096).unwrap(),
        sign: SignTimer::new(32 * 1024, Duration::from_millis(250)),
        last: BTreeMap::new(),
        dropped: 0,
        early_signs: 0,
    }
}

fn refusal<T>(r: Result<T, VideoError>) -> String {
    match r {
        Err(VideoError::CaptureMedia(m)) => m,
        Err(e) => panic!("refused otherwise: {e}"),
        Ok(_) => panic!("not refused"),
    }
}

#[test]
fn a_capture_uri_carries_its_options() {
    let o = capture_options();
    let uri = Uri::new(SwarmId::new(&[0; 32]), o.uri_options()).to_string();
    assert!(
        uri.ends_with("&x.im=2&x.hf=7&x.sa=2&x.cs=4096&x.ps=8&x.sc=16"),
        "{uri}"
    );
    assert_eq!(o.live_window, 8 * 1024);
}

#[test]
fn configs_derive_from_their_records() {
    let v = decoder_config(&h264(1)).unwrap();
    assert_eq!(
        (v.kind, v.codec.as_str(), v.timescale),
        (TrackKind::Video, "avc1.64001f", 90_000)
    );
    assert!(matches!(
        v.params,
        CodecParams::Avc {
            width: 1280,
            height: 720,
            ..
        }
    ));
    let a = decoder_config(&opus_config(2)).unwrap();
    assert_eq!(
        (a.kind, a.codec.as_str(), a.timescale),
        (TrackKind::Audio, "Opus", 48_000)
    );
    assert_eq!(
        a.params,
        CodecParams::Opus {
            head: OPUS_HEAD.to_vec(),
            channels: 2,
            pre_skip: 312
        }
    );
    assert_eq!(
        refusal(decoder_config(&CaptureConfig {
            codec: 0,
            ..h264(1)
        })),
        "track 1: codec 0 is not supported"
    );
    assert_eq!(
        refusal(decoder_config(&CaptureConfig {
            codec: 9,
            ..h264(1)
        })),
        "track 1: codec 9 is not supported"
    );
    assert!(
        refusal(decoder_config(&CaptureConfig {
            config: vec![1],
            ..h264(1)
        }))
        .starts_with("track 1: ")
    );
    assert_eq!(
        refusal(decoder_config(&CaptureConfig {
            config: b"OpusTags".to_vec(),
            ..opus_config(2)
        })),
        "track 2: OpusHead is truncated"
    );
}

#[test]
fn microseconds_round_to_the_nearest_tick() {
    assert_eq!(ticks(0, 90_000), 0);
    assert_eq!(ticks(33_333, 90_000), 3_000);
    assert_eq!(ticks(33_334, 90_000), 3_000);
    assert_eq!(ticks(20_000, 48_000), 960);
    assert_eq!(ticks(-20_000, 48_000), -960);
    // 5.55 us is half a 90 kHz tick: halves round up.
    assert_eq!(ticks(5, 90_000), 0);
    assert_eq!(ticks(6, 90_000), 1);
    assert_eq!(ticks(MAX_TIMESTAMP_US, 90_000), 810_647_932_926_689);
    assert_eq!(ticks(-MAX_TIMESTAMP_US, 90_000), -810_647_932_926_689);
}

#[test]
fn the_rate_bucket_fills_up_to_its_burst() {
    let t0 = Time::from_unix_millis(1_000);
    let mut b = Bucket {
        tokens: 100,
        at: t0,
    };
    assert!(b.take(t0, 60, 1_000, 100));
    assert!(!b.take(t0, 60, 1_000, 100));
    // 20 ms at 1000 B/s refills 20.
    assert!(b.take(t0 + Duration::from_millis(20), 60, 1_000, 100));
    assert_eq!(b.tokens, 0);
    // Never past the burst, however long it waits.
    assert!(!b.take(t0 + Duration::from_secs(60), 101, 1_000, 100));
    assert_eq!(b.tokens, 100);
    // A clock that steps back takes no tokens away.
    assert!(b.take(t0, 100, 1_000, 100));
}

#[test]
fn an_append_is_checked_whole_before_it_is_written() {
    let mut w = writer();
    let configs = [h264(1), opus_config(2)];
    let units = [unit(1, 0, true), unit(2, 0, true), unit(2, 20_000, true)];
    let c = w.check(&configs, &units).unwrap();
    assert_eq!(c.times, [0, 0, 960]);
    w.apply(c, &units, Time::from_unix_millis(5)).unwrap();
    assert_eq!(w.units.metrics().units, 3);
    let before = w.units.output().len();

    let cases: Vec<(Vec<CaptureConfig>, Vec<CaptureUnit>, &str)> = vec![
        (
            vec![],
            vec![unit(3, 1, true)],
            "track 3 has no decoder config",
        ),
        (
            vec![],
            vec![unit(2, 40_000, true), unit(2, 40_000, true)],
            "track 2: timestamp 40000 us is not after the track's last unit",
        ),
        (
            vec![],
            vec![unit(2, 20_010, true)],
            "track 2: timestamp 20010 us is not after the track's last unit",
        ),
        (
            vec![],
            vec![unit(1, MAX_TIMESTAMP_US + 1, false)],
            "track 1: timestamp 9007199254740993 us is out of range",
        ),
        (
            vec![],
            vec![CaptureUnit {
                data: Vec::new(),
                ..unit(1, 40_000, false)
            }],
            "track 1: a unit has no data",
        ),
        (
            vec![],
            vec![CaptureUnit {
                data: vec![0; MAX_RECORD_LEN - MAX_UNIT_HEAD_LEN + 1],
                ..unit(1, 40_000, false)
            }],
            "track 1: a 8388545-byte unit is over the record bound",
        ),
        (
            vec![h264(1), h264(1)],
            vec![],
            "track 1 is configured twice",
        ),
        (vec![h264(0)], vec![], "track id 0 is reserved"),
        (
            (3..=9).map(opus_config).collect(),
            vec![],
            "more than 8 tracks",
        ),
    ];
    for (configs, units, reason) in cases {
        assert_eq!(refusal(w.check(&configs, &units)), reason);
    }
    assert_eq!(w.units.output().len(), before);

    // The record bound's payload fits; a config change applies to the
    // append's units.
    let big = CaptureUnit {
        data: vec![0; MAX_RECORD_LEN - MAX_UNIT_HEAD_LEN],
        ..unit(1, 40_000, true)
    };
    let units = [big];
    let c = w.check(&[h264(1)], &units).unwrap();
    w.apply(c, &units, Time::from_unix_millis(6)).unwrap();
    assert_eq!(w.units.metrics().units, 4);
}

#[test]
fn units_before_the_first_keyframe_are_dropped() {
    let mut w = writer();
    let units = [unit(2, 0, true), unit(1, 0, false), unit(1, 33_333, true)];
    let c = w.check(&[h264(1), opus_config(2)], &units).unwrap();
    w.apply(c, &units, Time::from_unix_millis(5)).unwrap();
    let m = w.units.metrics();
    assert_eq!((m.units, m.dropped_before_start, m.gops), (1, 2, 1));
}
