// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};

use super::*;
use crate::media::CodecParams;
use crate::units::record::{Record, parse_record};
use crate::units::test_support::{audio, unit, video};

const FRAME: usize = 64;

/// The records of a chunked stream from offset 0: offset and payload.
fn records(stream: &[u8]) -> Vec<(u64, Vec<u8>)> {
    let mut d = ChunkedRecordDecoder::new(FRAME, MAX_RECORD_LEN).unwrap();
    let mut out = Vec::new();
    let mut at = 0;
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

/// A record's kind, gop_back, and for a unit its track, seq, config
/// version and keyframe flag.
#[derive(Debug, PartialEq)]
enum R {
    Configs(u64, Vec<(u32, u32, u64)>),
    Unit(u64, u32, u64, u32, bool),
    End,
}

fn summary(stream: &[u8]) -> Vec<(u64, R)> {
    records(stream)
        .iter()
        .map(|(offset, r)| {
            let r = match parse_record(r).unwrap() {
                Record::Configs { gop_back, entries } => R::Configs(
                    gop_back,
                    entries
                        .map(|e| (e.track.0, e.version, e.next_seq))
                        .collect(),
                ),
                Record::Unit {
                    gop_back, header, ..
                } => R::Unit(
                    gop_back,
                    header.track.0,
                    header.seq,
                    header.config,
                    header.keyframe,
                ),
                Record::End { gop_back } => {
                    assert_eq!(gop_back, 0);
                    R::End
                }
                Record::Other { .. } => panic!("unknown kind"),
            };
            (*offset, r)
        })
        .collect()
}

#[test]
fn gops_start_at_video_keyframes_after_their_sync_records() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    // Before the first keyframe: dropped.
    w.push(&unit(2, 0, true, 10)).unwrap();
    w.push(&unit(1, 0, false, 10)).unwrap();
    w.push(&unit(1, 3000, true, 40)).unwrap();
    w.push(&unit(2, 1024, true, 10)).unwrap();
    w.push(&unit(1, 6000, false, 20)).unwrap();
    w.push(&unit(1, 9000, true, 30)).unwrap();
    w.push(&unit(2, 2048, true, 10)).unwrap();
    assert_eq!(w.metrics().dropped_before_start, 2);
    let s = summary(w.output());
    let offsets: Vec<u64> = s.iter().map(|(o, _)| *o).collect();
    let back = |i: usize, gop: usize| offsets[i] - offsets[gop];
    assert_eq!(
        s.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
        [
            R::Configs(0, vec![(1, 0, 0), (2, 0, 0)]),
            R::Unit(back(1, 0), 1, 0, 0, true),
            R::Unit(back(2, 0), 2, 0, 0, true),
            R::Unit(back(3, 0), 1, 1, 0, false),
            R::Configs(0, vec![(1, 0, 2), (2, 0, 1)]),
            R::Unit(back(5, 4), 1, 2, 0, true),
            R::Unit(back(6, 4), 2, 1, 0, true),
        ]
    );
    assert_eq!(offsets[0], 2);
    let m = w.metrics();
    assert_eq!((m.records, m.units, m.gops, m.config_changes), (7, 5, 2, 0));
    assert_eq!(m.bytes, w.output().len() as u64);
}

#[test]
fn a_changed_config_writes_its_version_before_the_next_unit() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    w.push(&unit(1, 0, true, 10)).unwrap();
    // The same config again changes nothing.
    w.set_config(&audio(2)).unwrap();
    w.push(&unit(2, 0, true, 10)).unwrap();
    let mut changed = audio(2);
    if let CodecParams::Aac { asc, .. } = &mut changed.params {
        asc.push(0);
    }
    w.set_config(&changed).unwrap();
    w.push(&unit(1, 3000, false, 10)).unwrap();
    w.push(&unit(2, 1024, true, 10)).unwrap();
    let s = summary(w.output());
    let rs: Vec<&R> = s.iter().map(|(_, r)| r).collect();
    assert_eq!(rs.len(), 6);
    assert_eq!(*rs[3], R::Configs(s[3].0 - 2, vec![(1, 0, 1), (2, 1, 1)]));
    assert!(matches!(rs[4], R::Unit(_, 1, 1, 0, false)));
    assert!(matches!(rs[5], R::Unit(_, 2, 1, 1, true)));
    assert_eq!(w.metrics().config_changes, 1);
    assert_eq!(w.configs()[1], changed);
}

#[test]
fn an_audio_only_stream_starts_a_gop_a_second() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    w.set_config(&audio(2)).unwrap();
    // 48 kHz, 1024 samples a frame: 47 frames make a second.
    for k in 0..100 {
        w.push(&unit(2, k * 1024, true, 4)).unwrap();
    }
    let gops: Vec<u64> = summary(w.output())
        .iter()
        .filter_map(|(_, r)| match r {
            R::Configs(0, e) => Some(e[0].2),
            _ => None,
        })
        .collect();
    assert_eq!(gops, [0, 47, 94]);
}

#[test]
fn a_unit_over_the_record_bound_is_refused_and_writes_nothing() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    w.set_config(&video(1)).unwrap();
    let big = vec![1; MAX_RECORD_LEN];
    let u = AccessUnit {
        payload: &big,
        ..unit(1, 0, true, 0)
    };
    assert_eq!(
        w.push(&u),
        Err(UnitWriteError::RecordTooLarge {
            len: MAX_RECORD_LEN
        })
    );
    assert!(w.output().is_empty());
    assert_eq!(
        w.push(&unit(9, 0, true, 1)),
        Err(UnitWriteError::UnknownTrack(9))
    );
    w.push(&unit(1, 0, true, 1)).unwrap();
    assert_eq!(w.metrics().gops, 1);
}

#[test]
fn configs_the_init_can_not_declare_are_refused() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    let mut v = video(1);
    v.timescale = 0;
    assert!(matches!(w.set_config(&v), Err(UnitWriteError::Config(_))));
    for t in 1..=8 {
        w.set_config(&audio(t)).unwrap();
    }
    assert!(matches!(
        w.set_config(&audio(9)),
        Err(UnitWriteError::Config(_))
    ));
    assert_eq!(w.configs().len(), 8);
}

#[test]
fn an_early_sign_ends_the_frame_and_continues_at_the_next_unit() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    w.set_config(&video(1)).unwrap();
    w.push(&unit(1, 0, true, 10)).unwrap();
    w.end_frame();
    let first = w.take_output();
    assert_eq!(first.len() % FRAME, 0);
    // A unit of 4 frames: the swarm pads to offset 4 * FRAME.
    assert_eq!(
        w.continue_at(FRAME as u64 + 1),
        Err(UnitWriteError::Continue(ContinueError::Unaligned))
    );
    w.continue_at(4 * FRAME as u64).unwrap();
    w.push(&unit(1, 3000, false, 10)).unwrap();
    assert_eq!(
        w.continue_at(8 * FRAME as u64),
        Err(UnitWriteError::OutputPending)
    );
    // The swarm sends the frames between as zeros.
    let mut stream = first;
    stream.resize(4 * FRAME, 0);
    stream.extend_from_slice(w.output());
    let s = summary(&stream);
    assert_eq!(s.len(), 3);
    assert_eq!(s[2].0, 4 * FRAME as u64 + 2);
    assert_eq!(s[2].1, R::Unit(s[2].0 - 2, 1, 1, 0, false));
    assert_eq!(w.offset(), stream.len() as u64);
    assert!(w.metrics().padding > 0);
}

#[test]
fn the_sign_timer_runs_from_a_partial_units_first_byte() {
    let t0 = Time::from_unix_millis(1000);
    let ms = Duration::from_millis;
    let mut s = SignTimer::new(100, ms(250));
    assert_eq!(s.due(), None);
    s.written(40, t0);
    assert_eq!(s.due(), Some(t0 + ms(250)));
    // More bytes of the same unit keep its time.
    s.written(90, t0 + ms(100));
    assert_eq!(s.due(), Some(t0 + ms(250)));
    // A unit filled and the next begun: the new unit's time.
    s.written(130, t0 + ms(200));
    assert_eq!(s.due(), Some(t0 + ms(450)));
    // A unit filled exactly: nothing partial.
    s.written(200, t0 + ms(300));
    assert_eq!(s.due(), None);
    s.written(210, t0 + ms(400));
    s.flushed(300);
    assert_eq!(s.due(), None);
    s.written(301, t0 + ms(500));
    assert_eq!(s.due(), Some(t0 + ms(750)));
    let mut never = SignTimer::new(100, Duration::MAX);
    never.written(1, t0);
    assert_eq!(never.due(), None);
}

#[test]
fn finishing_ends_the_session_once_and_the_next_starts_a_gop() {
    let mut w = UnitWriter::new(FRAME).unwrap();
    // Nothing written: no END.
    w.finish().unwrap();
    assert!(w.output().is_empty());
    w.set_config(&video(1)).unwrap();
    w.push(&unit(1, 0, true, 10)).unwrap();
    w.push(&unit(1, 3000, false, 10)).unwrap();
    w.finish().unwrap();
    w.finish().unwrap();
    assert_eq!(w.metrics().ends, 1);
    // The next session's first unit, not a keyframe, waits for one.
    w.push(&unit(1, 6000, false, 10)).unwrap();
    w.push(&unit(1, 9000, true, 10)).unwrap();
    w.finish().unwrap();
    // Units without their gop_back.
    let kinds: Vec<R> = summary(w.output())
        .into_iter()
        .map(|(_, r)| match r {
            R::Unit(_, track, seq, config, key) => R::Unit(0, track, seq, config, key),
            r => r,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            R::Configs(0, vec![(1, 0, 0)]),
            R::Unit(0, 1, 0, 0, true),
            R::Unit(0, 1, 1, 0, false),
            R::End,
            R::Configs(0, vec![(1, 0, 2)]),
            R::Unit(0, 1, 2, 0, true),
            R::End,
        ]
    );
    let m = w.metrics();
    assert_eq!((m.ends, m.gops, m.dropped_before_start), (2, 2, 1));
}
