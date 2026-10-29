// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The writer's streams read back through the chunked layout, whole, from
//! a GOP's sync record, and with chunks lost.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};

use super::record::{Record, parse_record};
use super::test_support::{audio, unit, video};
use super::*;
use crate::{AccessUnit, Muxer};

/// The records of the chunked stream `stream` in frames of `frame`.
pub(crate) fn decode(frame: usize, stream: &[u8]) -> Vec<Vec<u8>> {
    decode_from(frame, stream, 0)
        .into_iter()
        .map(|(_, r)| r)
        .collect()
}

/// The records of `stream` read from stream offset `from`: offset and
/// payload.
fn decode_from(frame: usize, stream: &[u8], from: u64) -> Vec<(u64, Vec<u8>)> {
    let mut d = ChunkedRecordDecoder::new(frame, MAX_RECORD_LEN).unwrap();
    d.reset(from);
    let mut out = Vec::new();
    let mut at = from as usize;
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { offset, payload }) => out.push((offset, payload.to_vec())),
            Some(Item::Lost) => panic!("lost records at {at}"),
            None => return out,
        }
    }
}

fn proptest_config() -> Config {
    Config {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x0a11_0000),
        failure_persistence: None,
        ..Config::default()
    }
}

const FRAME: usize = 256;
/// Frames per integrity unit.
const UNIT_FRAMES: usize = 4;

/// What the writer was given: per unit, its track, DTS, keyframe flag and
/// payload length.
#[derive(Debug, Clone, Copy)]
struct Step {
    track: u32,
    dts: i64,
    key: bool,
    len: usize,
    /// The stream is signed early after the unit.
    sign: bool,
}

/// A stream of video at 30 fps, a keyframe every `gop` pictures, and AAC
/// frames between, with payloads of `lens`.
fn steps(gop: usize, lens: &[(usize, bool)]) -> Vec<Step> {
    let mut out = Vec::new();
    let (mut video_n, mut audio_n) = (0i64, 0i64);
    for &(len, sign) in lens {
        // Video at 3000 per picture of 90 kHz, audio at 1024 per frame of
        // 48 kHz: in time order, about three audio frames per two pictures.
        let video_due = video_n * 3000 * 48_000 <= audio_n * 1024 * 90_000;
        let step = if video_due {
            let key = (video_n as usize).is_multiple_of(gop);
            video_n += 1;
            Step {
                track: 1,
                dts: (video_n - 1) * 3000,
                key,
                len: len.max(1),
                sign,
            }
        } else {
            audio_n += 1;
            Step {
                track: 2,
                dts: (audio_n - 1) * 1024,
                key: true,
                len: (len % 400).max(1),
                sign,
            }
        };
        out.push(step);
    }
    out
}

/// The stream the writer makes of `steps`, signing early where they say:
/// the frame in progress ends, and the swarm pads the integrity unit with
/// zeros.
fn write(steps: &[Step]) -> Vec<u8> {
    let mut w = UnitWriter::new(FRAME).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    let mut stream = Vec::new();
    for s in steps {
        w.push(&unit(s.track, s.dts, s.key, s.len)).unwrap();
        if s.sign {
            w.end_frame();
            stream.extend(w.take_output());
            let unit = (FRAME * UNIT_FRAMES) as u64;
            let next = (stream.len() as u64).div_ceil(unit) * unit;
            stream.resize(next as usize, 0);
            w.continue_at(next).unwrap();
        }
    }
    stream.extend(w.take_output());
    stream
}

/// Reads `stream` through a reader, leaving out the chunks in `lost`:
/// the layout decoder resets at the chunk after each, as a swarm reader's
/// gap makes it. The units read, by track and DTS, and the reader.
fn read(stream: &[u8], lost: &BTreeSet<usize>) -> (Vec<(u32, i64, bool)>, UnitReader) {
    let mut d = ChunkedRecordDecoder::new(FRAME, MAX_RECORD_LEN).unwrap();
    let mut r = UnitReader::new();
    let mut out = Vec::new();
    let mut gap = false;
    for (i, chunk) in stream.chunks(FRAME).enumerate() {
        if lost.contains(&i) {
            gap = true;
            continue;
        }
        if gap {
            d.reset((i * FRAME) as u64);
            r.lost();
            gap = false;
        }
        let mut at = 0;
        loop {
            let (n, item) = d.decode(&chunk[at..]);
            at += n;
            match item {
                Some(Item::Record { payload, .. }) => {
                    if let Some(u) = r.read(payload) {
                        out.push((u.track.0, u.dts, u.keyframe));
                        check_unit(&u);
                    }
                }
                Some(Item::Lost) => r.lost(),
                None => break,
            }
        }
    }
    (out, r)
}

/// A unit read is the one written: its payload and publish time follow
/// from its track and DTS.
fn check_unit(u: &AccessUnit<'_>) {
    let len = u.payload.len();
    let want = unit(u.track.0, u.dts, u.keyframe, len);
    assert_eq!(*u, want);
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Every record points at a sync record before it, and the stream
    /// reads back whole.
    #[test]
    fn a_stream_reads_back_whole_and_every_record_points_at_its_gop(
        gop in 1usize..20,
        lens in proptest::collection::vec((1usize..1500, prop::bool::weighted(0.1)), 1..150),
    ) {
        let steps = steps(gop, &lens);
        let stream = write(&steps);
        let recs = decode_from(FRAME, &stream, 0);
        let syncs: BTreeSet<u64> = recs
            .iter()
            .filter(|(_, r)| matches!(parse_record(r).unwrap(), Record::Configs { gop_back: 0, .. }))
            .map(|(o, _)| *o)
            .collect();
        for (offset, r) in &recs {
            let k = offset - parse_record(r).unwrap().gop_back();
            prop_assert!(syncs.contains(&k), "record at {offset} points at {k}");
        }
        let (out, r) = read(&stream, &BTreeSet::new());
        let written: Vec<(u32, i64, bool)> = steps
            .iter()
            .skip_while(|s| !(s.track == 1 && s.key))
            .map(|s| (s.track, s.dts, s.key))
            .collect();
        prop_assert_eq!(out, written);
        prop_assert_eq!(r.metrics().gaps, 0);
    }

    /// With chunks lost, the units read are some of those written, in
    /// order per track, every gap is reported, and video resumes at
    /// keyframes.
    #[test]
    fn lost_chunks_cost_units_and_video_up_to_the_next_keyframe(
        gop in 1usize..20,
        lens in proptest::collection::vec((1usize..1500, prop::bool::weighted(0.1)), 1..150),
        lost in proptest::collection::btree_set(0usize..400, 0..6),
    ) {
        let steps = steps(gop, &lens);
        let stream = write(&steps);
        let (out, r) = read(&stream, &lost);
        let written: BTreeMap<(u32, i64), bool> =
            steps.iter().map(|s| ((s.track, s.dts), s.key)).collect();
        let mut last: BTreeMap<u32, i64> = BTreeMap::new();
        // The DTS of each track's units in write order.
        let order: BTreeMap<u32, Vec<i64>> = [1, 2]
            .into_iter()
            .map(|t| (t, steps.iter().filter(|s| s.track == t).map(|s| s.dts).collect()))
            .collect();
        for &(track, dts, key) in &out {
            prop_assert_eq!(written.get(&(track, dts)), Some(&key));
            if let Some(&prev) = last.get(&track) {
                prop_assert!(dts > prev);
                let o = &order[&track];
                let skipped = o.iter().filter(|&&d| d > prev && d < dts).count();
                // Video that skipped a picture resumes at a keyframe.
                if track == 1 && skipped > 0 {
                    prop_assert!(key, "video resumed at {dts} without a keyframe");
                }
            }
            last.insert(track, dts);
        }
        if lost.iter().all(|&c| c * FRAME >= stream.len()) {
            prop_assert_eq!(r.metrics().gaps, 0);
        }
    }
}

#[test]
fn a_reader_restarting_at_a_records_gop_reads_from_its_keyframe() {
    let lens: Vec<(usize, bool)> = (0..120)
        .map(|i: usize| (300 + i * 37 % 900, i.is_multiple_of(17)))
        .collect();
    let steps = steps(10, &lens);
    let stream = write(&steps);
    // Join at a chunk in the middle: the first whole record there names
    // its GOP's sync record K.
    let join = (stream.len() / FRAME / 2 * FRAME) as u64;
    let (first, rec) = decode_from(FRAME, &stream, join).remove(0);
    let k = first - parse_record(&rec).unwrap().gop_back();
    assert!(k < first);
    // Restart at K's frame and drop the records before K.
    let recs: Vec<Vec<u8>> = decode_from(FRAME, &stream, k - k % FRAME as u64)
        .into_iter()
        .filter(|(o, _)| *o >= k)
        .map(|(_, r)| r)
        .collect();
    assert!(matches!(
        parse_record(&recs[0]).unwrap(),
        Record::Configs { gop_back: 0, .. }
    ));
    let mut r = UnitReader::new();
    let first = recs
        .iter()
        .find_map(|rec| r.read(rec).map(|u| (u.track.0, u.keyframe)));
    assert_eq!(first, Some((1, true)));
    assert_eq!(r.metrics().awaiting_keyframe, 0);
}
