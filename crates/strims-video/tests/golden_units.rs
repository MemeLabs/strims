// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors of the unit format (`golden/units.txt`): UNIT, CONFIGS
//! and END records, the MIME type, and the chunked stream a unit script
//! makes through [`UnitWriter`], signed early once and ended, read back
//! through [`UnitReader`]. Values are hex. `STRIMS_GOLDEN=update` rewrites them
//! from the code (see `strims-golden`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use strims_base::time::Time;
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_golden::Golden;
use strims_video::units::record::{
    ConfigEntry, Record, UnitHeader, parse_record, write_configs, write_end, write_unit_head,
};
use strims_video::units::{MAX_RECORD_LEN, ReaderEvent, UnitReader, UnitWriter};
use strims_video::{AccessUnit, DecoderConfig, Muxer, TrackId, units_mime};

const VIDEO: TrackId = TrackId(1);
const AUDIO: TrackId = TrackId(2);

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/units.txt"))
}

/// The configs of the file's `tracks` entry: an H.264 track 1 at 90 kHz,
/// and an AAC or an Opus track 2 at 48 kHz.
fn configs(g: &Golden) -> (DecoderConfig, DecoderConfig, DecoderConfig) {
    let e = g.all("tracks")[0];
    let entry = |track, sample_entry, timescale, config: &[u8]| {
        ConfigEntry {
            track,
            version: 0,
            next_seq: 0,
            sample_entry,
            timescale,
            config,
        }
        .decoder_config()
        .unwrap()
        .unwrap()
    };
    (
        entry(VIDEO, *b"avc1", 90_000, &g.bytes(e, "avcc")),
        entry(AUDIO, *b"mp4a", 48_000, &g.bytes(e, "asc")),
        entry(AUDIO, *b"Opus", 48_000, &g.bytes(e, "opus_head")),
    )
}

fn tracks(g: &Golden, list: &str) -> Vec<DecoderConfig> {
    let (v, a, o) = configs(g);
    list.chars()
        .map(|c| match c {
            'v' => v.clone(),
            'a' => a.clone(),
            'o' => o.clone(),
            other => panic!("track kind {other}"),
        })
        .collect()
}

fn list<T: std::str::FromStr>(s: &str) -> Vec<T>
where
    T::Err: std::fmt::Debug,
{
    s.split(',').map(|v| v.parse().unwrap()).collect()
}

#[test]
fn units() {
    let mut g = open();
    for e in g.all("unit") {
        let h = UnitHeader {
            track: TrackId(g.str(e, "track").parse().unwrap()),
            keyframe: g.str(e, "key") == "1",
            seq: g.str(e, "seq").parse().unwrap(),
            config: g.str(e, "config").parse().unwrap(),
            dts: g.str(e, "dts").parse().unwrap(),
            pts: g.str(e, "pts").parse().unwrap(),
            published: Time::from_unix_millis(g.str(e, "unix_ms").parse().unwrap()),
        };
        let gop_back = g.str(e, "gop_back").parse().unwrap();
        let payload = g.bytes(e, "payload");
        let mut out = Vec::new();
        write_unit_head(gop_back, &h, &mut out);
        out.extend_from_slice(&payload);
        assert_eq!(
            parse_record(&out).unwrap(),
            Record::Unit {
                gop_back,
                header: h,
                payload: &payload,
            }
        );
        g.check_hex(e, "hex", &out);
    }
}

#[test]
fn configs_records() {
    let mut g = open();
    for e in g.all("configs") {
        let tracks = tracks(&g, g.str(e, "tracks"));
        let versions: Vec<u32> = list(g.str(e, "versions"));
        let next: Vec<u64> = list(g.str(e, "next"));
        let gop_back = g.str(e, "gop_back").parse().unwrap();
        let entries: Vec<ConfigEntry<'_>> = tracks
            .iter()
            .zip(versions.iter().zip(&next))
            .map(|(c, (&v, &n))| ConfigEntry::of(c, v, n))
            .collect();
        let mut out = Vec::new();
        write_configs(gop_back, entries.iter().copied(), &mut out);
        let Record::Configs {
            gop_back: back,
            entries: read,
        } = parse_record(&out).unwrap()
        else {
            panic!("not configs");
        };
        assert_eq!(back, gop_back);
        assert_eq!(read.collect::<Vec<_>>(), entries);
        g.check_hex(e, "hex", &out);
    }
}

#[test]
fn ends() {
    let mut g = open();
    for e in g.all("end") {
        let mut out = Vec::new();
        write_end(&mut out);
        assert_eq!(parse_record(&out).unwrap(), Record::End { gop_back: 0 });
        g.check_hex(e, "hex", &out);
    }
}

#[test]
fn mime() {
    let mut g = open();
    for e in g.all("mime") {
        let tracks = tracks(&g, g.str(e, "tracks"));
        g.check(e, "value", units_mime(&tracks));
    }
}

/// Six frames of 30 fps video, a keyframe every three, and 48 kHz AAC up
/// to each frame, in arrival order: track, DTS, keyframe, payload.
fn script() -> Vec<(TrackId, i64, bool, Vec<u8>)> {
    let mut units = Vec::new();
    let mut n = 0i64;
    for k in 0..6i64 {
        let key = k % 3 == 0;
        let size = if key { 40 } else { 12 };
        let payload = (0..size).map(|i| (k * 16 + i) as u8).collect();
        units.push((VIDEO, k * 3000, key, payload));
        while n * 1024 * 90_000 / 48_000 <= k * 3000 {
            units.push((AUDIO, n * 1024, true, vec![0xa0 | (n as u8 & 0xf); 6]));
            n += 1;
        }
    }
    units
}

fn unit((track, dts, keyframe, payload): &(TrackId, i64, bool, Vec<u8>)) -> AccessUnit<'_> {
    AccessUnit {
        track: *track,
        dts: *dts,
        pts: *dts,
        keyframe: *keyframe,
        payload,
        published: Time::from_unix_millis(1_793_275_200_000 + *dts / 90),
    }
}

/// The script through a writer of `frame`-byte frames, signed early
/// after unit `sign` into units of `unit_frames` frames and ended, then
/// read back whole.
#[test]
fn streams() {
    let mut g = open();
    let (v, a, _) = configs(&g);
    let units = script();
    for e in g.all("stream") {
        let frame: usize = g.str(e, "frame").parse().unwrap();
        let sign: usize = g.str(e, "sign_after").parse().unwrap();
        let unit_bytes = frame * g.str(e, "unit_frames").parse::<usize>().unwrap();
        let mut w = UnitWriter::new(frame).unwrap();
        w.set_config(&v).unwrap();
        w.set_config(&a).unwrap();
        let mut stream = Vec::new();
        for (i, u) in units.iter().enumerate() {
            w.push(&unit(u)).unwrap();
            if i == sign {
                w.end_frame();
                stream.extend(w.take_output());
                let next = stream.len().div_ceil(unit_bytes) * unit_bytes;
                stream.resize(next, 0);
                w.continue_at(next as u64).unwrap();
            }
        }
        w.finish().unwrap();
        stream.extend(w.take_output());
        g.check_hex(e, "hex", &stream);

        let mut d = ChunkedRecordDecoder::new(frame, MAX_RECORD_LEN).unwrap();
        let mut r = UnitReader::new();
        let mut read = Vec::new();
        let mut ends = Vec::new();
        let mut at = 0;
        loop {
            let (n, item) = d.decode(&stream[at..]);
            at += n;
            match item {
                Some(Item::Record { payload, .. }) => {
                    if let Some(u) = r.read(payload) {
                        read.push((u.track, u.dts, u.keyframe, u.payload.to_vec()));
                    }
                    while let Some(e) = r.poll_event() {
                        if e == ReaderEvent::End {
                            ends.push(read.len());
                        }
                    }
                }
                Some(Item::Lost) => panic!("lost records"),
                None => break,
            }
        }
        assert_eq!(read, units);
        assert_eq!(ends, [units.len()], "an END after the last unit");
        assert!(r.configs().is_empty());
    }
}
