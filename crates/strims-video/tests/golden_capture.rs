// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors of browser captures (`golden/capture.txt`): decoder
//! configs and access units as each engine's WebCodecs encoders gave them,
//! the MIME type and init segment the configs make, and the units read
//! back through the unit format into fMP4 fragments. `STRIMS_GOLDEN=update`
//! rewrites the outputs from the code (see `strims-golden`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use strims_base::time::{Duration, Time};
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_golden::Golden;
use strims_proto::video::v1::CaptureConfig;
use strims_proto::video::v1::capture_config::Codec;
use strims_video::capture::decoder_config;
use strims_video::fmp4::{FragmentWriter, parse_init, write_init};
use strims_video::units::{MAX_RECORD_LEN, UnitReader, UnitWriter};
use strims_video::{AccessUnit, CodecParams, Muxer, TrackId, units_mime};

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/capture.txt"))
}

/// A unit of an entry: track, presentation time in microseconds, keyframe
/// and payload.
fn units(s: &str) -> Vec<(u32, i64, bool, Vec<u8>)> {
    s.split(',')
        .map(|u| {
            let f: Vec<&str> = u.split(':').collect();
            (
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                f[2] == "1",
                hex::decode(f[3]).unwrap(),
            )
        })
        .collect()
}

#[test]
fn engine_captures_mux_into_the_unit_format_and_fmp4() {
    let mut g = open();
    for e in g.all("capture") {
        let engine = g.str(e, "engine").to_owned();
        let configs = [
            decoder_config(&CaptureConfig {
                track: 1,
                codec: Codec::H264 as i32,
                config: g.bytes(e, "avcc"),
            })
            .unwrap(),
            decoder_config(&CaptureConfig {
                track: 2,
                codec: Codec::Opus as i32,
                config: g.bytes(e, "opus_head"),
            })
            .unwrap(),
        ];
        assert!(
            matches!(configs[1].params, CodecParams::Opus { pre_skip: 312, .. }),
            "{engine}"
        );
        g.check(e, "mime", units_mime(&configs));
        let mut init = Vec::new();
        write_init(&configs, Duration::ZERO, &mut init).unwrap();
        assert_eq!(parse_init(&init).unwrap().tracks().len(), 2, "{engine}");
        g.check_hex(e, "init", &init);

        // Through the unit format and back, into fragments.
        let units = units(g.str(e, "units"));
        let mut w = UnitWriter::new(4096).unwrap();
        for c in &configs {
            w.set_config(c).unwrap();
        }
        let ticks = |track: u32, us: i64| {
            let rate = if track == 1 { 90_000 } else { 48_000 };
            (us * rate + 500_000).div_euclid(1_000_000)
        };
        for (track, us, keyframe, payload) in &units {
            let t = ticks(*track, *us);
            w.push(&AccessUnit {
                track: TrackId(*track),
                dts: t,
                pts: t,
                keyframe: *keyframe,
                payload,
                published: Time::from_unix_millis(1_800_000_000_000),
            })
            .unwrap();
        }
        w.end_frame();
        let stream = w.take_output();
        let mut d = ChunkedRecordDecoder::new(4096, MAX_RECORD_LEN).unwrap();
        let mut r = UnitReader::new();
        let mut fragments = FragmentWriter::new(Duration::from_millis(100));
        fragments.set_tracks(&configs, Duration::ZERO).unwrap();
        let mut media = Vec::new();
        let mut read = Vec::new();
        let mut at = 0;
        loop {
            let (n, item) = d.decode(&stream[at..]);
            at += n;
            match item {
                Some(Item::Record { payload, .. }) => {
                    if let Some(u) = r.read(payload) {
                        read.push((u.track.0, u.dts, u.keyframe, u.payload.to_vec()));
                        fragments.push(&u, &mut media).unwrap();
                    }
                }
                Some(Item::Lost) => panic!("{engine}: lost records"),
                None => break,
            }
        }
        let want: Vec<_> = units
            .iter()
            .map(|(track, us, key, p)| (*track, ticks(*track, *us), *key, p.clone()))
            .collect();
        assert_eq!(read, want, "{engine}");
        assert_eq!(r.configs(), configs, "{engine}");
        fragments.flush(None, &mut media);
        let payloads: usize = units.iter().map(|u| u.3.len()).sum();
        assert!(media.len() > payloads, "{engine}");
    }
}
