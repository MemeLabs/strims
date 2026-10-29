// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::Time;

use super::*;
use crate::Muxer;
use crate::units::UnitWriter;
use crate::units::record::{ConfigEntry, write_configs, write_unit_head};
use crate::units::test_support::{audio, unit, video};

/// What a reader made of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Out {
    Configs(Vec<u32>),
    Gap(u32, u64),
    /// Track, DTS, keyframe.
    Unit(u32, i64, bool),
}

fn read(r: &mut UnitReader, record: &[u8], out: &mut Vec<Out>) {
    let unit = r
        .read(record)
        .map(|u| Out::Unit(u.track.0, u.dts, u.keyframe));
    while let Some(e) = r.poll_event() {
        out.push(match e {
            ReaderEvent::Configs => Out::Configs(r.configs().iter().map(|c| c.track.0).collect()),
            ReaderEvent::Gap { track, missing } => Out::Gap(track.0, missing),
        });
    }
    out.extend(unit);
}

fn read_all<'a>(r: &mut UnitReader, records: impl IntoIterator<Item = &'a Vec<u8>>) -> Vec<Out> {
    let mut out = Vec::new();
    for rec in records {
        read(r, rec, &mut out);
    }
    out
}

/// A unit's payload as the writer had it.
struct Push {
    track: u32,
    dts: i64,
    key: bool,
}

const fn v(dts: i64, key: bool) -> Push {
    Push { track: 1, dts, key }
}

const fn a(dts: i64) -> Push {
    Push {
        track: 2,
        dts,
        key: true,
    }
}

/// The records a writer of video track 1 and audio track 2 makes of
/// `pushes`, one per unit or CONFIGS record.
fn records(pushes: &[Push]) -> Vec<Vec<u8>> {
    let mut w = UnitWriter::new(4096).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    for p in pushes {
        w.push(&unit(p.track, p.dts, p.key, 8)).unwrap();
    }
    crate::units::tests::decode(4096, w.output())
}

/// Two GOPs of three pictures with audio between.
fn two_gops() -> Vec<Push> {
    vec![
        v(0, true),
        a(0),
        v(3000, false),
        a(1024),
        v(6000, false),
        v(9000, true),
        a(2048),
        v(12000, false),
        a(3072),
    ]
}

#[test]
fn a_stream_read_from_its_start_gives_every_unit() {
    let recs = records(&two_gops());
    let mut r = UnitReader::new();
    let out = read_all(&mut r, &recs);
    assert_eq!(
        out,
        [
            Out::Configs(vec![1, 2]),
            Out::Unit(1, 0, true),
            Out::Unit(2, 0, true),
            Out::Unit(1, 3000, false),
            Out::Unit(2, 1024, true),
            Out::Unit(1, 6000, false),
            Out::Unit(1, 9000, true),
            Out::Unit(2, 2048, true),
            Out::Unit(1, 12000, false),
            Out::Unit(2, 3072, true),
        ]
    );
    assert_eq!(r.configs(), [video(1), audio(2)]);
    let m = r.metrics();
    assert_eq!((m.records, m.units, m.configs, m.gaps), (11, 9, 2, 0));
}

#[test]
fn a_reader_joining_mid_gop_starts_at_the_next_configs_and_keyframe() {
    let recs = records(&two_gops());
    // From the second GOP's first P: units before a CONFIGS are dropped.
    let mut r = UnitReader::new();
    let out = read_all(&mut r, &recs[3..]);
    assert_eq!(
        out,
        [
            Out::Configs(vec![1, 2]),
            Out::Unit(1, 9000, true),
            Out::Unit(2, 2048, true),
            Out::Unit(1, 12000, false),
            Out::Unit(2, 3072, true),
        ]
    );
    assert_eq!(r.metrics().unknown_tracks, 3);
}

#[test]
fn a_changed_config_mid_gop_lets_audio_start_and_video_wait_for_a_keyframe() {
    let mut w = UnitWriter::new(4096).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    w.push(&unit(1, 0, true, 8)).unwrap();
    w.push(&unit(2, 0, true, 8)).unwrap();
    let mut changed = audio(2);
    if let crate::CodecParams::Aac { asc, .. } = &mut changed.params {
        asc.push(0);
    }
    w.set_config(&changed).unwrap();
    w.push(&unit(1, 3000, false, 8)).unwrap();
    w.push(&unit(2, 1024, true, 8)).unwrap();
    w.push(&unit(1, 6000, true, 8)).unwrap();
    let recs = crate::units::tests::decode(4096, w.output());
    // Joined at the change.
    let mut r = UnitReader::new();
    assert_eq!(
        read_all(&mut r, &recs[3..]),
        [
            Out::Configs(vec![1, 2]),
            Out::Unit(2, 1024, true),
            Out::Unit(1, 6000, true),
        ]
    );
    assert_eq!(r.configs()[1], changed);
    // Read through: the change is a configs event, and nothing waits.
    let mut r = UnitReader::new();
    let out = read_all(&mut r, &recs);
    assert_eq!(out[3], Out::Configs(vec![1, 2]));
    assert_eq!(out[4], Out::Unit(1, 3000, false));
    assert_eq!(r.metrics().awaiting_keyframe, 0);
}

#[test]
fn a_lost_unit_is_a_gap_and_video_resumes_at_its_next_keyframe() {
    let recs = records(&two_gops());
    // Records: C V0 A0 V1 A1 V2 C V3 A2 V4 A3; lose V1 and A1.
    let kept: Vec<&Vec<u8>> = recs
        .iter()
        .enumerate()
        .filter(|(i, _)| ![3, 4].contains(i))
        .map(|(_, r)| r)
        .collect();
    let mut r = UnitReader::new();
    r.lost();
    let out = read_all(&mut r, kept);
    assert_eq!(
        out,
        [
            Out::Configs(vec![1, 2]),
            Out::Unit(1, 0, true),
            Out::Unit(2, 0, true),
            Out::Gap(1, 1),
            Out::Gap(2, 1),
            Out::Unit(1, 9000, true),
            Out::Unit(2, 2048, true),
            Out::Unit(1, 12000, false),
            Out::Unit(2, 3072, true),
        ]
    );
    let m = r.metrics();
    assert_eq!(
        (m.gaps, m.missing, m.awaiting_keyframe, m.lost),
        (2, 2, 1, 1)
    );

    // A lost sync record and keyframe: the next video unit's number tells.
    let kept: Vec<&Vec<u8>> = recs
        .iter()
        .enumerate()
        .filter(|(i, _)| ![6, 7].contains(i))
        .map(|(_, r)| r)
        .collect();
    let out = read_all(&mut UnitReader::new(), kept);
    assert_eq!(
        out[6..],
        [
            Out::Unit(2, 2048, true),
            Out::Gap(1, 1),
            Out::Unit(2, 3072, true)
        ]
    );
}

#[test]
fn a_unit_of_a_config_version_the_reader_lacks_is_dropped() {
    let mut w = UnitWriter::new(4096).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    w.push(&unit(1, 0, true, 8)).unwrap();
    w.push(&unit(2, 0, true, 8)).unwrap();
    let mut v2 = video(1);
    v2.params = crate::CodecParams::Avc {
        avcc: crate::synth::script::avcc(320, 180),
        width: 320,
        height: 180,
    };
    w.set_config(&v2).unwrap();
    w.push(&unit(1, 3000, true, 8)).unwrap();
    w.push(&unit(1, 6000, false, 8)).unwrap();
    w.push(&unit(2, 1024, true, 8)).unwrap();
    let recs = crate::units::tests::decode(4096, w.output());
    // Records: C V0 A0 C' V1 V2 A1; lose C'.
    let kept: Vec<&Vec<u8>> = recs
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 3)
        .map(|(_, r)| r)
        .collect();
    let mut r = UnitReader::new();
    let out = read_all(&mut r, kept);
    assert_eq!(
        out,
        [
            Out::Configs(vec![1, 2]),
            Out::Unit(1, 0, true),
            Out::Unit(2, 0, true),
            Out::Unit(2, 1024, true),
        ]
    );
    assert_eq!(r.metrics().config_mismatches, 2);
}

#[test]
fn a_new_session_starts_its_tracks_over() {
    let first = records(&two_gops());
    let second = records(&[v(0, true), a(0), v(3000, false)]);
    let mut r = UnitReader::new();
    let mut out = read_all(&mut r, &first);
    out.clear();
    r.lost();
    let out2 = read_all(&mut r, &second);
    assert_eq!(
        out2,
        [
            Out::Configs(vec![1, 2]),
            Out::Unit(1, 0, true),
            Out::Unit(2, 0, true),
            Out::Unit(1, 3000, false),
        ]
    );
}

#[test]
fn units_out_of_order_or_repeated_are_dropped() {
    let recs = records(&two_gops());
    let mut r = UnitReader::new();
    let mut out = Vec::new();
    for rec in &recs[..4] {
        read(&mut r, rec, &mut out);
    }
    // V1 again: stale.
    read(&mut r, &recs[3], &mut out);
    assert_eq!(out.len(), 4);
    assert_eq!(r.metrics().stale, 1);
    // A unit whose DTS goes back.
    let mut h = Vec::new();
    write_unit_head(
        0,
        &UnitHeader {
            track: crate::TrackId(2),
            keyframe: true,
            seq: 1,
            config: 0,
            dts: -5,
            pts: -5,
            published: Time::EPOCH,
        },
        &mut h,
    );
    read(&mut r, &h, &mut out);
    assert_eq!(out.len(), 4);
    assert_eq!(r.metrics().out_of_order, 1);
}

#[test]
fn records_the_reader_can_not_use_are_counted() {
    let mut r = UnitReader::new();
    let mut out = Vec::new();
    read(&mut r, &[], &mut out);
    read(&mut r, &[9, 0, 0], &mut out);
    // A config that doesn't parse.
    let mut c = Vec::new();
    write_configs(
        0,
        [ConfigEntry {
            config: b"\x02",
            ..ConfigEntry::of(&video(1), 0, 0)
        }],
        &mut c,
    );
    read(&mut r, &c, &mut out);
    // Configs of a sample entry the reader doesn't know.
    let mut o = Vec::new();
    write_configs(
        0,
        [
            ConfigEntry {
                sample_entry: *b"Opus",
                ..ConfigEntry::of(&audio(3), 0, 0)
            },
            ConfigEntry::of(&audio(2), 0, 0),
        ],
        &mut o,
    );
    read(&mut r, &o, &mut out);
    assert_eq!(out, [Out::Configs(vec![2])]);
    let m = r.metrics();
    assert_eq!(
        (m.malformed, m.unknown_kinds, m.unknown_entries, m.configs),
        (2, 1, 1, 1)
    );
}
