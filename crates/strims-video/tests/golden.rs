// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.txt`): init segments with and without
//! edit lists, `prft` boxes, and the fragments a unit script makes through
//! [`FragmentWriter`] at each kind of span. Values are hex, and lists are comma-separated.
//! `STRIMS_GOLDEN=update` rewrites them from the code (see
//! `strims-golden`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use strims_base::time::{Duration, Time};
use strims_golden::Golden;
use strims_video::fmp4::{FragmentWriter, ProducerTime, write_init, write_prft};
use strims_video::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

const VIDEO: TrackId = TrackId(1);
const AUDIO: TrackId = TrackId(2);

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.txt"))
}

fn video_config() -> DecoderConfig {
    DecoderConfig {
        track: VIDEO,
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: vec![1, 0x64, 0, 0x1f, 0xff, 0xe0, 0],
            width: 640,
            height: 360,
        },
    }
}

fn audio_config() -> DecoderConfig {
    DecoderConfig {
        track: AUDIO,
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc: vec![0x11, 0x90],
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

/// A stereo Opus track 2, of the OpusHead WebCodecs encoders give.
fn opus_config() -> DecoderConfig {
    DecoderConfig {
        track: AUDIO,
        kind: TrackKind::Audio,
        codec: "Opus".into(),
        timescale: 48_000,
        params: CodecParams::Opus {
            head: b"OpusHead\x01\x02\x38\x01\x80\xbb\0\0\0\0\0".to_vec(),
            channels: 2,
            pre_skip: 312,
        },
    }
}

fn configs(tracks: &str) -> Vec<DecoderConfig> {
    tracks
        .chars()
        .map(|c| match c {
            'v' => video_config(),
            'a' => audio_config(),
            'o' => opus_config(),
            other => panic!("track kind {other}"),
        })
        .collect()
}

fn span(s: &str) -> Duration {
    match s {
        "max" => Duration::MAX,
        "0" => Duration::ZERO,
        ms => Duration::from_millis(ms.strip_suffix("ms").unwrap().parse().unwrap()),
    }
}

/// A unit of the script: track, DTS, PTS, keyframe, payload.
type Unit = (TrackId, i64, i64, bool, Vec<u8>);

/// Twelve frames of 30 fps video with B-frames (`I P B P B …`, a
/// keyframe every six), and 48 kHz AAC up to each frame, in arrival
/// order; payloads of a few bytes.
fn script() -> Vec<Unit> {
    let mut units = Vec::new();
    let mut n = 0i64;
    for k in 0..12i64 {
        let key = k % 6 == 0;
        // Shown as I B P B P …, two frames after decoding starts.
        let cto = match (key, k % 2) {
            (true, _) => 6000,
            (false, 1) => 9000,
            _ => 3000,
        };
        let size = if key { 6 } else { 3 };
        let payload = (0..size).map(|i| (k * 16 + i) as u8).collect();
        units.push((VIDEO, k * 3000, k * 3000 + cto, key, payload));
        while n * 1024 * 90_000 / 48_000 <= k * 3000 {
            units.push((
                AUDIO,
                n * 1024,
                n * 1024,
                true,
                vec![0xa0 | (n as u8 & 0xf); 2],
            ));
            n += 1;
        }
    }
    units
}

/// The publish time of a unit: 2026-10-29T12:00:00Z plus its DTS in
/// milliseconds, and a millisecond per track id.
fn published(track: TrackId, dts: i64, timescale: i64) -> Time {
    Time::from_unix_millis(1_793_275_200_000 + dts * 1000 / timescale + i64::from(track.0))
}

fn unit((track, dts, pts, keyframe, payload): &Unit) -> AccessUnit<'_> {
    let timescale = if *track == VIDEO { 90_000 } else { 48_000 };
    AccessUnit {
        track: *track,
        dts: *dts,
        pts: *pts,
        keyframe: *keyframe,
        payload,
        published: published(*track, *dts, timescale),
    }
}

#[test]
fn inits() {
    let mut g = open();
    for e in g.all("init") {
        let tracks = configs(g.str(e, "tracks"));
        let delay = Duration::from_nanos(g.str(e, "delay_ns").parse().unwrap());
        let mut out = Vec::new();
        write_init(&tracks, delay, &mut out).unwrap();
        g.check_hex(e, "hex", &out);
    }
}

#[test]
fn prfts() {
    let mut g = open();
    for e in g.all("prft") {
        let p = ProducerTime {
            track: TrackId(g.str(e, "track").parse().unwrap()),
            media_time: g.str(e, "media_time").parse().unwrap(),
            published: Time::from_unix_millis(g.str(e, "unix_ms").parse().unwrap()),
        };
        let mut out = Vec::new();
        write_prft(&p, &mut out);
        g.check_hex(e, "hex", &out);
    }
}

#[test]
fn fragments() {
    let mut g = open();
    let units = script();
    for e in g.all("fragments") {
        let mut w = FragmentWriter::new(span(g.str(e, "span")));
        // The script's keyframes show two frames late.
        w.set_tracks(
            &[video_config(), audio_config()],
            Duration::from_nanos(66_666_666),
        )
        .unwrap();
        let mut out = Vec::new();
        for u in &units {
            w.push(&unit(u), &mut out).unwrap();
        }
        w.flush(None, &mut out);
        g.check_hex(e, "hex", &out);
    }
}
