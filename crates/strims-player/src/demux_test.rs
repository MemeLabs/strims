// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Unit-format streams written by the real writer, read back through the
//! demuxer, against the units the writer took.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_video::synth::script::{Script, ScriptConfig, TagKind, aac_lc_asc, avcc};
use strims_video::units::record::{Record, parse_record};
use strims_video::{FlvSource, Muxer, SourceEvent, UnitWriter};

use super::*;
use crate::webcodecs;

/// The swarm's chunk size: the chunked layout's frame.
const FRAME: usize = 4096;
/// The Egress RPC's largest `Data` message.
const MAX_DATA: usize = 32 * 1024;
/// The writer's clock when a script starts.
const EPOCH_MS: i64 = 1_793_275_200_000;
/// A stereo OpusHead with a pre-skip of 312.
const OPUS_HEAD: [u8; 19] = *b"OpusHead\x01\x02\x38\x01\x80\xbb\0\0\0\0\0";

/// An access unit that owns its payload.
#[derive(Debug, Clone, PartialEq)]
struct Unit {
    track: TrackId,
    dts: i64,
    pts: i64,
    keyframe: bool,
    payload: Vec<u8>,
    published: Time,
}

/// What a source gives its muxers.
#[derive(Debug, Clone)]
enum Event {
    Config(DecoderConfig),
    Unit(Unit),
}

/// An output, owned.
#[derive(Debug, Clone, PartialEq)]
enum Out {
    Configs(Vec<DecoderConfig>),
    Unit {
        track: TrackId,
        kind: TrackKind,
        keyframe: bool,
        pts: Duration,
        dts: Duration,
        duration: Duration,
        published: Time,
        data: Vec<u8>,
    },
    Gap {
        track: TrackId,
        kind: TrackKind,
        missing: u64,
    },
    Discontinuity,
    Restart(Duration),
}

/// A script's tags through the FLV source, received at their tag times.
fn script(config: ScriptConfig) -> Vec<Event> {
    let mut source = FlvSource::new();
    let mut out = Vec::new();
    for tag in &Script::new(&config).tags {
        let at = Time::from_unix_millis(EPOCH_MS + i64::from(tag.timestamp));
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, at),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, at),
            TagKind::Metadata => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => out.push(Event::Config(c)),
            Some(SourceEvent::Unit(u)) => out.push(Event::Unit(Unit {
                track: u.track,
                dts: u.dts,
                pts: u.pts,
                keyframe: u.keyframe,
                payload: u.payload.to_vec(),
                published: u.published,
            })),
            None => {}
        }
    }
    out
}

fn b_frames() -> ScriptConfig {
    ScriptConfig {
        duration_secs: 8,
        b_frames: true,
        ..ScriptConfig::default()
    }
}

/// The unit-format records a writer session makes of `events`.
fn unit_records(events: &[Event]) -> Vec<Vec<u8>> {
    let mut w = UnitWriter::new(FRAME).unwrap();
    for e in events {
        match e {
            Event::Config(c) => w.set_config(c).unwrap(),
            Event::Unit(u) => w
                .push(&AccessUnit {
                    track: u.track,
                    dts: u.dts,
                    pts: u.pts,
                    keyframe: u.keyframe,
                    payload: &u.payload,
                    published: u.published,
                })
                .unwrap(),
        }
    }
    w.finish().unwrap();
    let stream = w.output();
    let mut d = ChunkedRecordDecoder::new(FRAME, MAX_RECORD_LEN).unwrap();
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { payload, .. }) => out.push(payload.to_vec()),
            Some(Item::Lost) => panic!("lost records at {at}"),
            None => return out,
        }
    }
}

fn drain(d: &mut Demuxer, out: &mut Vec<Out>) {
    while let Some(o) = d.poll_output() {
        out.push(match o {
            DemuxOutput::Configs { tracks } => Out::Configs(tracks.to_vec()),
            DemuxOutput::Unit(u) => Out::Unit {
                track: u.track,
                kind: u.kind,
                keyframe: u.keyframe,
                pts: u.pts,
                dts: u.dts,
                duration: u.duration,
                published: u.published,
                data: u.data.to_vec(),
            },
            DemuxOutput::Gap {
                track,
                kind,
                missing,
            } => Out::Gap {
                track,
                kind,
                missing,
            },
            DemuxOutput::Discontinuity => Out::Discontinuity,
            DemuxOutput::Restart { offset } => Out::Restart(offset),
        });
    }
}

/// Passes `record` as the Egress RPC sends it: `Data` messages of at most
/// `MAX_DATA` bytes, the last with `segment_end`, the first with
/// `discontinuity` when it follows a skip.
fn send(d: &mut Demuxer, record: &[u8], discontinuity: bool, out: &mut Vec<Out>) {
    let chunks: Vec<&[u8]> = if record.is_empty() {
        vec![record]
    } else {
        record.chunks(MAX_DATA).collect()
    };
    let n = chunks.len();
    for (i, chunk) in chunks.into_iter().enumerate() {
        d.message(chunk, i + 1 == n, discontinuity && i == 0);
        drain(d, out);
    }
}

fn demux_marked(records: &[(Vec<u8>, bool)]) -> (Vec<Out>, Demuxer) {
    let mut d = Demuxer::new();
    let mut out = Vec::new();
    for (rec, skip) in records {
        send(&mut d, rec, *skip, &mut out);
    }
    (out, d)
}

fn demux(records: &[Vec<u8>]) -> (Vec<Out>, Demuxer) {
    let marked: Vec<(Vec<u8>, bool)> = records.iter().map(|r| (r.clone(), false)).collect();
    demux_marked(&marked)
}

/// `records` with the units of `track` decoding in `[from, to)` (in its
/// timescale) left out, keyframes too unless `keep_keys`; the first record
/// after them is marked as following a skip.
fn without(
    records: &[Vec<u8>],
    track: u32,
    from: i64,
    to: i64,
    keep_keys: bool,
) -> Vec<(Vec<u8>, bool)> {
    let mut out: Vec<(Vec<u8>, bool)> = Vec::new();
    let mut skipped = false;
    for rec in records {
        if let Ok(Record::Unit { header, .. }) = parse_record(rec)
            && header.track.0 == track
            && (from..to).contains(&header.dts)
            && !(keep_keys && header.keyframe)
        {
            skipped = true;
            continue;
        }
        out.push((rec.clone(), std::mem::take(&mut skipped)));
    }
    out
}

fn units(events: &[Event]) -> Vec<&Unit> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Unit(u) => Some(u),
            Event::Config(_) => None,
        })
        .collect()
}

/// The output's units of `track`: (dts, pts, keyframe).
fn track_units(out: &[Out], track: u32) -> Vec<(Duration, Duration, bool)> {
    out.iter()
        .filter_map(|o| match o {
            Out::Unit {
                track: t,
                dts,
                pts,
                keyframe,
                ..
            } if t.0 == track => Some((*dts, *pts, *keyframe)),
            _ => None,
        })
        .collect()
}

fn ticks(units: i64, timescale: u32) -> Duration {
    Duration::from_nanos(units * 1_000_000_000 / i64::from(timescale))
}

#[test]
fn units_of_one_session_come_out_as_written() {
    let events = script(b_frames());
    let (out, d) = demux(&unit_records(&events));
    let Some(Out::Configs(configs)) = out.first() else {
        panic!("no configs first: {:?}", out.first());
    };
    let written: Vec<&DecoderConfig> = events
        .iter()
        .filter_map(|e| match e {
            Event::Config(c) => Some(c),
            Event::Unit(_) => None,
        })
        .collect();
    assert_eq!(configs.len(), 2);
    for c in configs {
        assert!(written.contains(&c), "{c:?} wasn't written");
    }
    let timescale = |track: TrackId| configs.iter().find(|c| c.track == track).unwrap().timescale;
    let expected: Vec<Out> = units(&events)
        .into_iter()
        .map(|u| {
            let config = configs.iter().find(|c| c.track == u.track).unwrap();
            let ts = timescale(u.track);
            Out::Unit {
                track: u.track,
                kind: config.kind,
                keyframe: u.keyframe,
                pts: ticks(u.pts, ts),
                dts: ticks(u.dts, ts),
                duration: match config.kind {
                    TrackKind::Audio => ticks(1024, ts),
                    TrackKind::Video => Duration::ZERO,
                },
                published: u.published,
                data: u.payload.clone(),
            }
        })
        .collect();
    assert_eq!(out[1..], expected[..]);
    let m = d.metrics();
    assert_eq!(m.units, expected.len() as u64);
    assert_eq!(m.restarts + m.dropped_backwards + m.oversize_records, 0);
}

#[test]
fn configs_carry_what_webcodecs_decoders_take() {
    let opus = DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "Opus".into(),
        timescale: 48_000,
        params: CodecParams::Opus {
            head: OPUS_HEAD.to_vec(),
            channels: 2,
            pre_skip: 312,
        },
    };
    let mut events = vec![Event::Config(video(1)), Event::Config(opus)];
    for k in 0..30 {
        events.push(frame(1, k * 3000, k % 30 == 0, 300));
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: k * 960,
            pts: k * 960,
            keyframe: true,
            // CELT fullband, 20 ms.
            payload: vec![0xfc, 0x01, 0x02],
            published: Time::from_unix_millis(EPOCH_MS + k * 20),
        }));
    }
    let (out, _) = demux(&unit_records(&events));
    let Some(Out::Configs(configs)) = out.first() else {
        panic!("no configs first");
    };
    assert_eq!(webcodecs::codec(&configs[0]), "avc1.64001f");
    assert_eq!(
        webcodecs::description(&configs[0]),
        avcc(640, 360).as_slice()
    );
    assert_eq!(webcodecs::codec(&configs[1]), "opus");
    assert_eq!(webcodecs::description(&configs[1]), OPUS_HEAD.as_slice());
    assert!(matches!(
        configs[1].params,
        CodecParams::Opus { pre_skip: 312, .. }
    ));
    // Each Opus packet lasts its TOC's 20 ms.
    let durations: Vec<Duration> = out
        .iter()
        .filter_map(|o| match o {
            Out::Unit {
                track: TrackId(2),
                duration,
                ..
            } => Some(*duration),
            _ => None,
        })
        .collect();
    assert_eq!(durations.len(), 30);
    assert!(durations.iter().all(|d| *d == Duration::from_millis(20)));
}

#[test]
fn a_lost_picture_resumes_at_the_next_keyframe() {
    // GOPs of 60 frames at 30 fps: keyframes every 180_000 at 90 kHz.
    let records = unit_records(&script(b_frames()));
    let (out, d) = demux_marked(&without(&records, 1, 200_000, 230_000, true));
    let at = out.iter().position(|o| *o == Out::Discontinuity).unwrap();
    assert!(out[at..].iter().any(|o| matches!(
        o,
        Out::Gap {
            track: TrackId(1),
            kind: TrackKind::Video,
            ..
        }
    )));
    // Video after the loss starts at the keyframe at 4 s, and audio goes
    // on at once.
    let after = track_units(&out[at..], 1);
    assert_eq!(after[0].0, Duration::from_secs(4));
    assert!(after[0].2);
    let audio = track_units(&out[at..], 2);
    assert!(audio[0].0 < Duration::from_millis(2600));
    assert!(d.metrics().reader.awaiting_keyframe > 0);
}

#[test]
fn lost_audio_resumes_at_once_with_a_hole() {
    let records = unit_records(&script(b_frames()));
    // About 0.2 s of AAC at 48 kHz from 3 s on.
    let (out, _) = demux_marked(&without(&records, 2, 144_000, 153_600, false));
    assert!(out.iter().any(|o| matches!(
        o,
        Out::Gap {
            track: TrackId(2),
            kind: TrackKind::Audio,
            ..
        }
    )));
    let audio = track_units(&out, 2);
    let hole = audio.windows(2).map(|w| w[1].0 - w[0].0).max().unwrap();
    assert!(hole > Duration::from_millis(200), "{hole:?}");
    // Video loses nothing.
    let video = track_units(&out, 1);
    assert_eq!(
        video.len(),
        units(&script(b_frames()))
            .iter()
            .filter(|u| u.track.0 == 1)
            .count()
    );
}

#[test]
fn a_restarted_session_presents_after_the_output_so_far() {
    let first = unit_records(&script(b_frames()));
    let second = unit_records(&script(b_frames()));
    let mut marked: Vec<(Vec<u8>, bool)> = first.into_iter().map(|r| (r, false)).collect();
    marked.extend(second.into_iter().enumerate().map(|(i, r)| (r, i == 0)));
    let (out, d) = demux_marked(&marked);
    let restarts: Vec<Duration> = out
        .iter()
        .filter_map(|o| match o {
            Out::Restart(offset) => Some(*offset),
            _ => None,
        })
        .collect();
    assert_eq!(restarts.len(), 1);
    let at = out
        .iter()
        .position(|o| matches!(o, Out::Restart(_)))
        .unwrap();
    // The first session's presentation ends where its last units do: the
    // latest video picture lasting its decode interval, the latest AAC
    // frame its 1024 samples.
    let video = track_units(&out[..at], 1);
    let interval = video[video.len() - 1].0 - video[video.len() - 2].0;
    let end = |track: u32, lasts: Duration| {
        track_units(&out[..at], track)
            .iter()
            .map(|u| u.1 + lasts)
            .max()
            .unwrap()
    };
    let first_end = end(1, interval).max(end(2, ticks(1024, 48_000)));
    // The second's first unit presents right there, and every track's
    // presentation goes on after it.
    let Some(Out::Unit { pts, .. }) = out[at + 1..].iter().find(|o| matches!(o, Out::Unit { .. }))
    else {
        panic!("no unit after the restart");
    };
    assert_eq!(*pts, first_end);
    for track in [1, 2] {
        let before = track_units(&out[..at], track);
        let after = track_units(&out[at..], track);
        assert!(
            after
                .iter()
                .all(|u| u.1 + Duration::from_millis(100) >= first_end)
        );
        assert!(before.last().unwrap().0 < after.first().unwrap().0 + Duration::from_millis(100));
        assert!(after.windows(2).all(|w| w[1].0 >= w[0].0));
    }
    assert_eq!(d.metrics().restarts, 1);
}

#[test]
fn a_discontinuity_drops_the_record_in_progress() {
    let records = unit_records(&script(b_frames()));
    let mut d = Demuxer::new();
    let mut out = Vec::new();
    for rec in &records[..20] {
        send(&mut d, rec, false, &mut out);
    }
    let records_before = d.metrics().records;
    // Half a record, then a skip: the record after it comes whole.
    let half = &records[20][..records[20].len() / 2];
    d.message(half, false, false);
    send(&mut d, &records[21], true, &mut out);
    let m = d.metrics();
    assert_eq!(m.partial_records, 1);
    assert_eq!(m.discontinuities, 1);
    assert_eq!(m.records, records_before + 1);
    assert_eq!(m.reader.malformed, 0);
    assert!(out.contains(&Out::Discontinuity));
}

#[test]
fn records_over_the_bound_are_dropped_as_lost() {
    let mut d = Demuxer::new();
    let mut out = Vec::new();
    let big = vec![0u8; MAX_DATA];
    for _ in 0..=MAX_RECORD_LEN / MAX_DATA {
        d.message(&big, false, false);
    }
    d.message(&[], true, false);
    drain(&mut d, &mut out);
    let m = d.metrics();
    assert_eq!(m.oversize_records, 1);
    assert_eq!(m.records, 0);
    assert_eq!(m.reader.lost, 1);
    assert!(out.is_empty());
    // The next record reads as usual.
    let records = unit_records(&script(b_frames()));
    for rec in &records[..3] {
        send(&mut d, rec, false, &mut out);
    }
    assert!(matches!(out.first(), Some(Out::Configs(_))));
}

#[test]
fn outputs_wait_until_taken() {
    let records = unit_records(&script(b_frames()));
    let mut d = Demuxer::new();
    for rec in &records[..10] {
        d.message(rec, true, false);
    }
    let mut out = Vec::new();
    drain(&mut d, &mut out);
    let (all, _) = demux(&records[..10]);
    assert_eq!(out, all);
}

/// An H.264 track at 90 kHz.
fn video(track: u32) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: avcc(640, 360),
            width: 640,
            height: 360,
        },
    }
}

fn frame(track: u32, dts: i64, keyframe: bool, len: usize) -> Event {
    Event::Unit(Unit {
        track: TrackId(track),
        dts,
        pts: dts,
        keyframe,
        payload: vec![track as u8; len],
        published: Time::from_unix_millis(EPOCH_MS + dts / 90),
    })
}

#[test]
fn aac_units_last_their_frames() {
    let aac = DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 44_100,
        params: CodecParams::Aac {
            asc: aac_lc_asc(44_100),
            sample_rate: 44_100,
            channels: 2,
            frame_samples: 1024,
        },
    };
    let mut events = vec![Event::Config(aac)];
    for k in 0..10 {
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: k * 1024,
            pts: k * 1024,
            keyframe: true,
            payload: vec![1; 50],
            published: Time::from_unix_millis(EPOCH_MS),
        }));
    }
    let (out, _) = demux(&unit_records(&events));
    let n = out
        .iter()
        .filter(|o| matches!(o, Out::Unit { duration, .. } if *duration == ticks(1024, 44_100)))
        .count();
    assert_eq!(n, 10);
}

fn proptest_config() -> Config {
    Config {
        cases: 64,
        rng_seed: RngSeed::Fixed(0x5eed_d3a0),
        failure_persistence: None,
        ..Config::default()
    }
}

/// Checks a demuxer's outputs: units of a track decode in order within a
/// session, and every payload lies within the record bound. Returns how
/// many there were.
fn check_outputs(d: &mut Demuxer, last: &mut Vec<(TrackId, Duration)>) -> usize {
    let mut n = 0;
    while let Some(o) = d.poll_output() {
        n += 1;
        match o {
            DemuxOutput::Restart { .. } => last.clear(),
            DemuxOutput::Unit(u) => {
                assert!(u.data.len() <= MAX_RECORD_LEN);
                if let Some((_, l)) = last.iter_mut().find(|(t, _)| *t == u.track) {
                    assert!(u.dts >= *l, "track {:?} went back", u.track);
                    *l = u.dts;
                } else {
                    last.push((u.track, u.dts));
                }
            }
            _ => {}
        }
    }
    n
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn hostile_messages_never_panic(
        messages in prop::collection::vec(
            (prop::collection::vec(any::<u8>(), 0..512), any::<bool>(), any::<bool>()),
            0..64,
        ),
    ) {
        let mut d = Demuxer::new();
        let mut last = Vec::new();
        for (data, end, disc) in &messages {
            d.message(data, *end, *disc);
            check_outputs(&mut d, &mut last);
        }
        prop_assert!(d.record.len() <= MAX_RECORD_LEN);
    }

    #[test]
    fn damaged_streams_never_panic_and_keep_decode_order(
        damage in prop::collection::vec((0usize..400, 0usize..4096, any::<u8>()), 0..24),
        drops in prop::collection::vec(0usize..400, 0..24),
    ) {
        let mut records = damaged_base().to_vec();
        for (i, at, b) in damage {
            let n = records.len();
            let rec = &mut records[i % n];
            if !rec.is_empty() {
                let k = at % rec.len();
                rec[k] ^= b;
            }
        }
        let n = records.len();
        let marked: Vec<(Vec<u8>, bool)> = records
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !drops.iter().any(|d| d % n == *i))
            .map(|(i, r)| (r, drops.iter().any(|d| (d + 1) % n == i)))
            .collect();
        let mut d = Demuxer::new();
        let mut last = Vec::new();
        for (rec, skip) in &marked {
            d.message(rec, true, *skip);
            check_outputs(&mut d, &mut last);
        }
    }
}

fn damaged_base() -> &'static [Vec<u8>] {
    static BASE: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        unit_records(&script(ScriptConfig {
            duration_secs: 4,
            b_frames: true,
            ..ScriptConfig::default()
        }))
    })
}
