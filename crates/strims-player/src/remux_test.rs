// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Unit-format streams written by the real writer, read back through the
//! remuxer, against what the segmenter writes for the same units.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_video::fmp4::{media_duration, ntp_timestamp, parse_init, split_record};
use strims_video::synth::script::{Script, ScriptConfig, TagKind, aac_lc_asc, avcc};
use strims_video::units::record::{Record, parse_record};
use strims_video::{CutRule, FlvSource, Muxer, Segmenter, SourceEvent, UnitWriter, VecSink};

use super::*;

/// The swarm's chunk size: the chunked layout's frame.
const FRAME: usize = 4096;
/// The Egress RPC's largest `Data` message.
const MAX_DATA: usize = 32 * 1024;
/// The writer's clock when a script starts.
const EPOCH_MS: i64 = 1_793_275_200_000;

/// An access unit that owns its payload.
#[derive(Debug, Clone)]
struct Unit {
    track: TrackId,
    dts: i64,
    pts: i64,
    keyframe: bool,
    payload: Vec<u8>,
    published: Time,
}

impl Unit {
    fn unit(&self) -> AccessUnit<'_> {
        AccessUnit {
            track: self.track,
            dts: self.dts,
            pts: self.pts,
            keyframe: self.keyframe,
            payload: &self.payload,
            published: self.published,
        }
    }
}

/// What a source gives its muxers.
#[derive(Debug, Clone)]
enum Event {
    Config(DecoderConfig),
    Unit(Unit),
}

/// A script's tags through the FLV source, received at their tag times.
fn source_events(script: &Script) -> Vec<Event> {
    let mut source = FlvSource::new();
    let mut out = Vec::new();
    for tag in &script.tags {
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

fn script(config: ScriptConfig) -> Vec<Event> {
    source_events(&Script::new(&config))
}

fn feed<M: Muxer>(muxer: &mut M, events: &[Event])
where
    M::Error: std::fmt::Debug,
{
    for e in events {
        match e {
            Event::Config(c) => muxer.set_config(c).unwrap(),
            Event::Unit(u) => muxer.push(&u.unit()).unwrap(),
        }
    }
    muxer.finish().unwrap();
}

/// The unit-format records a writer session makes of `events`.
fn unit_records(events: &[Event]) -> Vec<Vec<u8>> {
    let mut w = UnitWriter::new(FRAME).unwrap();
    feed(&mut w, events);
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

/// The segmenter's records of `events`, a fragment per unit and a record
/// per set of configs.
fn segmenter_records(events: &[Event]) -> Vec<Vec<u8>> {
    let rule = CutRule {
        min_media_bytes: usize::MAX,
        max_record_bytes: usize::MAX,
        fragment_duration: Duration::ZERO,
    };
    let mut s = Segmenter::new(rule, VecSink::new());
    feed(&mut s, events);
    s.into_sink().records
}

/// `events` moved as the remuxer moves a stream whose first keyframe
/// decodes before its composition delay: later by the difference.
fn moved_to_the_edit(events: &[Event]) -> Vec<Event> {
    let mut timescales = BTreeMap::new();
    let mut lead = None;
    for e in events {
        match e {
            Event::Config(c) => {
                timescales.insert(c.track, c.timescale);
            }
            Event::Unit(u) if lead.is_none() && u.keyframe && u.pts > u.dts => {
                let ts = timescales[&u.track];
                let by = (u.pts - u.dts - u.dts).max(0);
                lead = Some(to_duration(by, ts));
            }
            _ => {}
        }
    }
    let lead = lead.unwrap_or(Duration::ZERO);
    events
        .iter()
        .map(|e| match e {
            Event::Unit(u) => {
                let by = to_units(lead, timescales[&u.track]);
                Event::Unit(Unit {
                    dts: u.dts + by,
                    pts: u.pts + by,
                    ..u.clone()
                })
            }
            e => e.clone(),
        })
        .collect()
}

/// The first video keyframe's composition delay, in its track's units.
fn first_delay(events: &[Event]) -> i64 {
    events
        .iter()
        .find_map(|e| match e {
            Event::Unit(u) if u.track.0 == 1 && u.keyframe => Some(u.pts - u.dts),
            _ => None,
        })
        .unwrap()
}

/// An output, owned.
#[derive(Debug, Clone, PartialEq)]
enum Out {
    Init {
        data: Vec<u8>,
        mime: String,
        tracks: Vec<DecoderConfig>,
        compatible: bool,
    },
    Media {
        data: Vec<u8>,
        units: Vec<UnitInfo>,
    },
    Gap {
        track: TrackId,
        kind: TrackKind,
        missing: u64,
    },
    Discontinuity,
    Restart(Duration),
    End,
}

fn drain(r: &mut Remuxer, out: &mut Vec<Out>) {
    while let Some(o) = r.poll_output() {
        out.push(match o {
            Output::Init {
                data,
                mime,
                tracks,
                compatible,
            } => Out::Init {
                data: data.to_vec(),
                mime: mime.into(),
                tracks: tracks.to_vec(),
                compatible,
            },
            Output::Media { data, units } => Out::Media {
                data: data.to_vec(),
                units: units.to_vec(),
            },
            Output::Gap {
                track,
                kind,
                missing,
            } => Out::Gap {
                track,
                kind,
                missing,
            },
            Output::Discontinuity => Out::Discontinuity,
            Output::Restart { offset } => Out::Restart(offset),
            Output::End => Out::End,
        });
    }
}

/// Passes `record` as the Egress RPC sends it: `Data` messages of at most
/// `MAX_DATA` bytes, the last with `segment_end`, the first with
/// `discontinuity` when it follows a skip.
fn send(r: &mut Remuxer, record: &[u8], discontinuity: bool, out: &mut Vec<Out>) {
    let chunks: Vec<&[u8]> = if record.is_empty() {
        vec![record]
    } else {
        record.chunks(MAX_DATA).collect()
    };
    let n = chunks.len();
    for (i, chunk) in chunks.into_iter().enumerate() {
        r.message(chunk, i + 1 == n, discontinuity && i == 0);
        drain(r, out);
    }
}

fn remux(records: &[Vec<u8>]) -> (Vec<Out>, Remuxer) {
    let mut r = Remuxer::new();
    let mut out = Vec::new();
    for rec in records {
        send(&mut r, rec, false, &mut out);
    }
    r.finish();
    drain(&mut r, &mut out);
    (out, r)
}

/// The inits and the media after each, concatenated.
fn segments(out: &[Out]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut segs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for o in out {
        match o {
            Out::Init { data, .. } => segs.push((data.clone(), Vec::new())),
            Out::Media { data, .. } => segs.last_mut().unwrap().1.extend_from_slice(data),
            _ => {}
        }
    }
    segs
}

/// A fragment's track run: its track, base decode time, and per sample
/// its duration and composition offset.
#[derive(Debug, Clone, PartialEq)]
struct Run {
    track: u32,
    base: u64,
    samples: Vec<(u32, i32)>,
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(b[at..at + 4].try_into().unwrap())
}

/// A run's track, base decode time and samples.
type RunOf = (u32, u64, Vec<(u32, i32)>);

/// The fragments of `media`: each a `prft`, `moof` and `mdat`.
fn fragments(media: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let (mut at, mut start, mut n) = (0, 0, 0);
    while at + 8 <= media.len() {
        at += be32(media, at) as usize;
        n += 1;
        if n % 3 == 0 {
            out.push(&media[start..at]);
            start = at;
        }
    }
    out
}

/// The boxes of `data`, each its type and content; the writers write
/// 32-bit sizes.
fn boxes(data: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 8 <= data.len() {
        let size = be32(data, at) as usize;
        out.push((
            data[at + 4..at + 8].try_into().unwrap(),
            &data[at + 8..at + size],
        ));
        at += size;
    }
    out
}

/// The runs of a fragment's `moof`, in `traf` order, and its `prft`'s
/// track, NTP timestamp and media time.
fn runs(fragment: &[u8]) -> (Vec<Run>, (u32, u64, u64)) {
    let mut runs = Vec::new();
    let mut prft = None;
    for (fourcc, content) in boxes(fragment) {
        match &fourcc {
            b"prft" => {
                let ntp = u64::from_be_bytes(content[8..16].try_into().unwrap());
                let media = u64::from_be_bytes(content[16..24].try_into().unwrap());
                prft = Some((be32(content, 4), ntp, media));
            }
            b"moof" => {
                for (fourcc, traf) in boxes(content) {
                    if fourcc != *b"traf" {
                        continue;
                    }
                    let mut run = Run {
                        track: 0,
                        base: 0,
                        samples: Vec::new(),
                    };
                    for (fourcc, c) in boxes(traf) {
                        match &fourcc {
                            b"tfhd" => run.track = be32(c, 4),
                            b"tfdt" => run.base = u64::from_be_bytes(c[4..12].try_into().unwrap()),
                            b"trun" => {
                                let flags = be32(c, 0) & 0xff_ffff;
                                let count = be32(c, 4) as usize;
                                let mut at = 8 + if flags & 1 != 0 { 4 } else { 0 };
                                for _ in 0..count {
                                    let duration = be32(c, at);
                                    at += 8;
                                    let mut cto = 0;
                                    if flags & 0x400 != 0 {
                                        at += 4;
                                    }
                                    if flags & 0x800 != 0 {
                                        cto = be32(c, at) as i32;
                                        at += 4;
                                    }
                                    run.samples.push((duration, cto));
                                }
                            }
                            _ => {}
                        }
                    }
                    runs.push(run);
                }
            }
            _ => {}
        }
    }
    (runs, prft.unwrap())
}

/// Every written sample of `track`: its decode time and duration.
fn samples(out: &[Out], track: u32) -> Vec<(i64, i64)> {
    let mut all = Vec::new();
    for o in out {
        if let Out::Media { data, .. } = o {
            for run in runs(data).0.into_iter().filter(|r| r.track == track) {
                let mut dts = run.base as i64;
                for (d, _) in run.samples {
                    all.push((dts, i64::from(d)));
                    dts += i64::from(d);
                }
            }
        }
    }
    all
}

/// The silent frame of AAC LC stereo.
const LC_STEREO_SILENCE: &[u8] = &[0x20, 0x64, 0x00, 0x01, 0x90, 0x00, 0x0e];

/// Every written sample's bytes of `track`, in decode order.
fn payloads(out: &[Out], track: u32) -> Vec<Vec<u8>> {
    let mut all = Vec::new();
    for o in out {
        let Out::Media { data, .. } = o else {
            continue;
        };
        let mut sizes: Vec<(u32, Vec<usize>)> = Vec::new();
        let mut mdat: &[u8] = &[];
        for (fourcc, content) in boxes(data) {
            match &fourcc {
                b"moof" => {
                    for (fourcc, traf) in boxes(content) {
                        if fourcc != *b"traf" {
                            continue;
                        }
                        let (mut id, mut list) = (0, Vec::new());
                        for (fourcc, c) in boxes(traf) {
                            match &fourcc {
                                b"tfhd" => id = be32(c, 4),
                                b"trun" => {
                                    let flags = be32(c, 0) & 0xff_ffff;
                                    let entry = 8
                                        + if flags & 0x400 != 0 { 4 } else { 0 }
                                        + if flags & 0x800 != 0 { 4 } else { 0 };
                                    let count = be32(c, 4) as usize;
                                    let base = 8 + if flags & 1 != 0 { 4 } else { 0 };
                                    for k in 0..count {
                                        list.push(be32(c, base + k * entry + 4) as usize);
                                    }
                                }
                                _ => {}
                            }
                        }
                        sizes.push((id, list));
                    }
                }
                b"mdat" => mdat = content,
                _ => {}
            }
        }
        let mut at = 0;
        for (id, list) in sizes {
            for n in list {
                if id == track {
                    all.push(mdat[at..at + n].to_vec());
                }
                at += n;
            }
        }
    }
    all
}

/// Samples that each start where the one before ended.
fn assert_contiguous(samples: &[(i64, i64)]) {
    for w in samples.windows(2) {
        assert_eq!(
            w[0].0 + w[0].1,
            w[1].0,
            "a hole or overlap after {:?}",
            w[0]
        );
    }
}

fn b_frames() -> ScriptConfig {
    ScriptConfig {
        b_frames: true,
        duration_secs: 8,
        ..ScriptConfig::default()
    }
}

#[test]
fn output_equals_the_segmenters_records_for_one_session() {
    for config in [
        b_frames(),
        ScriptConfig {
            resize_at_gop: Some(2),
            ..b_frames()
        },
        ScriptConfig {
            start_ms: 3_600_000,
            gop_frames: 25,
            fps: 25,
            ..ScriptConfig::default()
        },
        ScriptConfig {
            audio: false,
            ..b_frames()
        },
    ] {
        let events = script(config.clone());
        let (out, r) = remux(&unit_records(&events));
        let expected: Vec<(Vec<u8>, Vec<u8>)> = segmenter_records(&moved_to_the_edit(&events))
            .iter()
            .map(|rec| {
                let (init, media) = split_record(rec).unwrap();
                (init.to_vec(), media.to_vec())
            })
            .collect();
        let got = segments(&out);
        assert_eq!(got.len(), expected.len(), "{config:?}");
        let last = got.len() - 1;
        for (i, (g, e)) in got.iter().zip(&expected).enumerate() {
            assert!(g.0 == e.0, "init {i} differs for {config:?}");
            let info = parse_init(&g.0).unwrap();
            assert!(media_duration(&info, &g.1).unwrap() > Duration::ZERO);
            if i < last {
                assert!(g.1 == e.1, "media {i} differs for {config:?}");
                continue;
            }
            // The last fragment differs only in the video's last sample,
            // which lasts to the end of the output.
            let (gf, ef) = (fragments(&g.1), fragments(&e.1));
            assert_eq!(gf.len(), ef.len(), "{config:?}");
            let n = gf.len() - 1;
            assert!(gf[..n] == ef[..n], "media {i} differs for {config:?}");
            let (mut gr, mut er) = (runs(gf[n]).0, runs(ef[n]).0);
            let mdat = |f: &[u8]| {
                boxes(f)
                    .into_iter()
                    .find(|(t, _)| t == b"mdat")
                    .unwrap()
                    .1
                    .to_vec()
            };
            assert_eq!(mdat(gf[n]), mdat(ef[n]));
            let video = |runs: &mut Vec<Run>| {
                runs.iter_mut()
                    .find(|r| r.track == 1)
                    .unwrap()
                    .samples
                    .pop()
                    .unwrap()
            };
            let (gv, ev) = (video(&mut gr), video(&mut er));
            assert!(gv.0 >= ev.0 && gv.1 == ev.1, "{gv:?} {ev:?} for {config:?}");
            let samples = |runs: &[Run]| -> Vec<RunOf> {
                runs.iter()
                    .map(|r| (r.track, r.base, r.samples.clone()))
                    .collect()
            };
            assert_eq!(samples(&gr), samples(&er), "{config:?}");
        }
        let m = r.metrics();
        assert_eq!(m.inits, expected.len() as u64);
        assert_eq!(m.dropped_backwards + m.restarts + m.held_flushes, 0);
        assert_eq!(m.held_bytes, 0);
    }
}

#[test]
fn a_resize_is_a_compatible_init_at_its_keyframe() {
    let events = script(ScriptConfig {
        resize_at_gop: Some(2),
        ..b_frames()
    });
    let (out, _) = remux(&unit_records(&events));
    let inits: Vec<_> = out
        .iter()
        .filter_map(|o| match o {
            Out::Init {
                mime,
                tracks,
                compatible,
                ..
            } => Some((mime.clone(), tracks.clone(), *compatible)),
            _ => None,
        })
        .collect();
    assert_eq!(inits.len(), 2);
    assert_eq!(inits[0].0, r#"video/mp4;codecs="avc1.64001f,mp4a.40.2""#);
    assert!(!inits[0].2);
    assert!(inits[1].2);
    assert_eq!(inits[1].1[0].kind, TrackKind::Video);
    // The first media after the second init holds the GOP's keyframe.
    let at = out
        .iter()
        .rposition(|o| matches!(o, Out::Init { .. }))
        .unwrap();
    let Some(Out::Media { units, .. }) = out.get(at + 1) else {
        panic!("no media after the init");
    };
    assert!(units[0].keyframe && units[0].kind == TrackKind::Video);
}

#[test]
fn media_lists_the_units_each_fragment_holds() {
    let events = script(b_frames());
    let (out, r) = remux(&unit_records(&events));
    let mut timescales = BTreeMap::new();
    let mut listed = 0;
    for o in &out {
        match o {
            Out::Init { data, tracks, .. } => {
                // The edit list's media time: the keyframe's composition delay.
                let info = parse_init(data).unwrap();
                for t in tracks {
                    timescales.insert(t.track.0, info.track(t.track.0).unwrap().timescale);
                }
            }
            Out::Media { data, units } => {
                let (runs, (prft_track, ntp, media_time)) = runs(data);
                let mut at = 0;
                for run in &runs {
                    let timescale = timescales[&run.track];
                    let mut dts = run.base as i64;
                    for &(d, cto) in &run.samples {
                        let u = units[at];
                        assert_eq!(u.track.0, run.track);
                        let pts = to_duration(dts + i64::from(cto), timescale);
                        // Every track presents a composition delay earlier.
                        let delay = pts - u.pts;
                        assert!(delay >= Duration::ZERO && delay < Duration::from_millis(100));
                        dts += i64::from(d);
                        at += 1;
                    }
                }
                assert_eq!(at, units.len(), "units listed past the fragment's samples");
                assert_eq!(prft_track, units[0].track.0);
                assert_eq!(ntp, ntp_timestamp(units[0].published));
                let first = to_duration(media_time as i64, timescales[&prft_track]);
                assert!(first >= units[0].pts);
                listed += units.len();
            }
            _ => {}
        }
    }
    assert_eq!(listed as u64, r.metrics().units);
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

fn remux_marked(records: &[(Vec<u8>, bool)]) -> (Vec<Out>, Remuxer) {
    let mut r = Remuxer::new();
    let mut out = Vec::new();
    for (rec, skip) in records {
        send(&mut r, rec, *skip, &mut out);
    }
    r.finish();
    drain(&mut r, &mut out);
    (out, r)
}

#[test]
fn a_lost_picture_holds_the_last_one_until_the_next_keyframe() {
    // GOPs of 60 frames at 30 fps: keyframes every 180_000 at 90 kHz.
    let events = script(b_frames());
    let records = unit_records(&events);
    let (out, r) = remux_marked(&without(&records, 1, 200_000, 230_000, true));
    assert!(out.contains(&Out::Discontinuity));
    assert!(out.iter().any(|o| matches!(
        o,
        Out::Gap {
            track: TrackId(1),
            kind: TrackKind::Video,
            ..
        }
    )));
    let video = samples(&out, 1);
    assert_contiguous(&video);
    // The picture before the loss lasts until the keyframe at 360_000,
    // which the output moves on by the first keyframe's composition delay.
    let shift = first_delay(&events);
    let held = video
        .iter()
        .find(|(dts, d)| *dts < 200_000 + shift && dts + d == 360_000 + shift);
    assert!(held.is_some(), "no picture held across the loss: {video:?}");
    assert_contiguous(&samples(&out, 2));
    let m = r.metrics();
    assert!(m.discontinuities >= 1);
    assert!(m.reader.awaiting_keyframe > 0);
}

#[test]
fn lost_audio_is_filled_with_silence() {
    let events = script(b_frames());
    let records = unit_records(&events);
    // About 0.2 s of AAC at 48 kHz from 3 s on.
    let (out, r) = remux_marked(&without(&records, 2, 144_000, 153_600, false));
    assert!(out.iter().any(|o| matches!(
        o,
        Out::Gap {
            track: TrackId(2),
            kind: TrackKind::Audio,
            ..
        }
    )));
    let audio = samples(&out, 2);
    assert_contiguous(&audio);
    // Frames of 1024 samples, FLV's millisecond tag times making 1008 to
    // 1056, and silence rounding the hole to whole frames.
    assert!(audio.iter().all(|(_, d)| *d < 1536), "{audio:?}");
    let silent = payloads(&out, 2)
        .iter()
        .filter(|p| p.as_slice() == LC_STEREO_SILENCE)
        .count();
    let m = r.metrics();
    assert_eq!(silent as u64, m.silence_units);
    assert!((8..=10).contains(&silent), "{silent} silent frames");
    assert!(m.silence_fills >= 1);
    assert_eq!(m.audio_holes, 0);
    // Video loses nothing and goes on frame by frame (FLV's millisecond
    // tag times make frames of 33 or 34 ms).
    let video = samples(&out, 1);
    assert_contiguous(&video);
    assert!(video.iter().all(|(_, d)| *d <= 3060));
}

#[test]
fn a_stream_from_decode_time_zero_plays_from_its_first_keyframe() {
    // Two B-frames: the first keyframe decodes two frames before it
    // presents, at decode time zero.
    let events = script(b_frames());
    let records = unit_records(&events);
    let mut r = Remuxer::new();
    let mut out = Vec::new();
    let mut before_second_gop = None;
    for rec in &records {
        if let Ok(Record::Unit { header, .. }) = parse_record(rec)
            && header.track.0 == 1
            && header.keyframe
            && header.dts > 0
            && before_second_gop.is_none()
        {
            before_second_gop = Some(out.len());
        }
        send(&mut r, rec, false, &mut out);
    }
    let first = &out[..before_second_gop.unwrap()];
    let video: Vec<UnitInfo> = first
        .iter()
        .filter_map(|o| match o {
            Out::Media { units, .. } => Some(units.clone()),
            _ => None,
        })
        .flatten()
        .filter(|u| u.kind == TrackKind::Video)
        .collect();
    // The first GOP comes out a unit a fragment, from its keyframe, which
    // presents at the delay it was moved by.
    let delay = first_delay(&events);
    assert!(delay > 0);
    assert!(
        video.len() > 50,
        "{} pictures before the second GOP",
        video.len()
    );
    assert!(video[0].keyframe);
    assert_eq!(video[0].pts, to_duration(delay, 90_000));
    let fragments = first
        .iter()
        .filter(|o| matches!(o, Out::Media { .. }))
        .count();
    assert!(fragments > video.len());
    // No sample decodes before the edit.
    for o in &out {
        if let Out::Media { data, .. } = o {
            for run in runs(data).0.iter().filter(|r| r.track == 1) {
                assert!(run.base >= delay as u64, "{run:?}");
            }
        }
    }
    assert_eq!(r.metrics().held_flushes, 0);
}

#[test]
fn audio_that_stops_follows_the_video_with_silence() {
    let events = script(b_frames());
    let records = unit_records(&events);
    // No audio from 3 s on, to the end at 8 s.
    let (out, r) = remux_marked(&without(&records, 2, 144_000, i64::MAX, false));
    let audio = samples(&out, 2);
    assert_contiguous(&audio);
    let video = samples(&out, 1);
    let end = |s: &[(i64, i64)], rate: i64| {
        let (dts, d) = s.last().unwrap();
        Duration::from_nanos((dts + d) * 1_000_000_000 / rate)
    };
    // Silence reaches within AUDIO_FOLLOW and a frame of the video's end.
    let gap = end(&video, 90_000) - end(&audio, 48_000);
    assert!(
        gap <= AUDIO_FOLLOW + Duration::from_millis(50),
        "audio ends {gap:?} before the video"
    );
    let m = r.metrics();
    assert!(m.silence_units > 200, "{m:?}");
    assert_eq!(m.audio_holes, 0);
}

#[test]
fn audio_later_than_the_silence_following_the_video_moves_the_follow() {
    let events = script(b_frames());
    // From 3 s on, audio comes half a second after the video of its time.
    let mut late: Vec<Event> = Vec::new();
    let mut queue: Vec<Unit> = Vec::new();
    for e in events {
        match e {
            Event::Unit(u) if u.track.0 == 2 && u.dts >= 144_000 => queue.push(u),
            Event::Unit(u) if u.track.0 == 1 => {
                let now = u.dts * 48_000 / 90_000;
                while queue.first().is_some_and(|a| a.dts + 24_000 <= now) {
                    late.push(Event::Unit(queue.remove(0)));
                }
                late.push(Event::Unit(u));
            }
            e => late.push(e),
        }
    }
    late.extend(queue.into_iter().map(Event::Unit));
    let (out, r) = remux(&unit_records(&late));
    let audio = samples(&out, 2);
    assert_contiguous(&audio);
    // Silence follows the video until the late audio comes; the audio
    // within it is dropped, the follow moves past its lateness, and the
    // rest plays: of 5 s of late audio, 0.3 s goes.
    let m = r.metrics();
    assert!(m.silence_units > 0, "{m:?}");
    assert!((1..=16).contains(&m.dropped_under_silence), "{m:?}");
    let payloads = payloads(&out, 2);
    assert_eq!(payloads.len(), audio.len());
    assert!(
        payloads
            .iter()
            .rev()
            .take(200)
            .all(|p| p != LC_STEREO_SILENCE)
    );
}

#[test]
fn an_audio_hole_past_the_bound_stays_a_hole() {
    // Audio alone, with five seconds missing: no video to follow, and the
    // frame after the hole comes too late to fill it.
    let mut events = vec![Event::Config(audio(2, 48_000))];
    for n in (0..200).chain(434..500) {
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: n * 1024,
            pts: n * 1024,
            keyframe: true,
            payload: vec![2; 50],
            published: Time::from_unix_millis(EPOCH_MS + n * 1024 / 48),
        }));
    }
    let (out, r) = remux(&unit_records(&events));
    let frames = samples(&out, 2);
    let at = frames
        .iter()
        .position(|(dts, _)| *dts == 434 * 1024)
        .unwrap();
    assert_eq!(frames[at - 1], (199 * 1024, 1024));
    assert_contiguous(&frames[..at]);
    assert_contiguous(&frames[at..]);
    let m = r.metrics();
    assert_eq!((m.audio_holes, m.silence_units), (1, 0));

    // A shorter one is filled when the frame after it comes.
    let mut events = vec![Event::Config(audio(2, 48_000))];
    for n in (0..200).chain(300..400) {
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: n * 1024,
            pts: n * 1024,
            keyframe: true,
            payload: vec![2; 50],
            published: Time::from_unix_millis(EPOCH_MS + n * 1024 / 48),
        }));
    }
    let (out, r) = remux(&unit_records(&events));
    assert_contiguous(&samples(&out, 2));
    assert_eq!(r.metrics().silence_units, 100);
}

#[test]
fn opus_holes_are_filled_with_silent_packets_of_their_length() {
    let mut head = b"OpusHead".to_vec();
    head.extend_from_slice(&[1, 2, 0x38, 0x01, 0x80, 0xbb, 0, 0, 0, 0, 0]);
    let opus = DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "Opus".into(),
        timescale: 48_000,
        params: CodecParams::Opus {
            head,
            channels: 2,
            pre_skip: 312,
        },
    };
    let mut events = vec![Event::Config(opus)];
    // 20 ms CELT packets, with 135 ms missing after the hundredth.
    let mut dts = 0;
    for n in 0..200 {
        if n == 100 {
            dts += 6480;
        }
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts,
            pts: dts,
            keyframe: true,
            payload: vec![0xfc, 1, 2, 3],
            published: Time::from_unix_millis(EPOCH_MS + dts / 48),
        }));
        dts += 960;
    }
    let (out, r) = remux(&unit_records(&events));
    let audio = samples(&out, 2);
    assert_contiguous(&audio);
    let silent: Vec<Vec<u8>> = payloads(&out, 2)
        .into_iter()
        .filter(|p| p[1..] == [0xff, 0xfe])
        .collect();
    // 6480 samples: 2.5 ms short of 135, rounded to 2.5 ms steps.
    let lens: Vec<u32> = silent
        .iter()
        .map(|p| crate::webcodecs::opus_packet_samples(p).unwrap())
        .collect();
    assert_eq!(lens.iter().sum::<u32>(), 6480, "{lens:?}");
    assert_eq!(*lens.last().unwrap(), 960);
    assert_eq!(r.metrics().silence_units, lens.len() as u64);
}

#[test]
fn a_track_without_silent_frames_lasts_its_last_unit_until_its_next() {
    // HE-AAC: SBR at 48 kHz over a 24 kHz core.
    let he = DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "mp4a.40.5".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc: vec![0x2b, 0x11, 0x88, 0x00],
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 2048,
        },
    };
    let mut events = vec![Event::Config(he)];
    for n in (0..50).chain(60..100) {
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: n * 2048,
            pts: n * 2048,
            keyframe: true,
            payload: vec![2; 50],
            published: Time::from_unix_millis(EPOCH_MS + n * 2048 / 48),
        }));
    }
    let (out, r) = remux(&unit_records(&events));
    let audio = samples(&out, 2);
    assert_contiguous(&audio);
    assert!(audio.contains(&(49 * 2048, 11 * 2048)));
    assert_eq!(r.metrics().silence_units, 0);
}

#[test]
fn a_restarted_session_follows_the_output() {
    let first = script(b_frames());
    for second in [
        b_frames(),
        ScriptConfig {
            width: 640,
            height: 360,
            ..b_frames()
        },
    ] {
        // The first session ends without its END.
        let mut marked: Vec<(Vec<u8>, bool)> = unit_records(&first)
            .into_iter()
            .filter(|r| !is_end(r))
            .map(|r| (r, false))
            .collect();
        let b = unit_records(&script(second.clone()));
        marked.extend(b.into_iter().enumerate().map(|(i, r)| (r, i == 0)));
        let (out, r) = remux_marked(&marked);
        let restarts: Vec<_> = out
            .iter()
            .filter_map(|o| match o {
                Out::Restart(offset) => Some(*offset),
                _ => None,
            })
            .collect();
        assert_eq!(restarts.len(), 1, "{second:?}");
        // The second session starts where the first ended, 8 s in.
        assert!((restarts[0] - Duration::from_secs(8)).abs() < Duration::from_millis(50));
        let inits = out.iter().filter(|o| matches!(o, Out::Init { .. })).count();
        let resized = second.width != b_frames().width;
        assert_eq!(inits, if resized { 2 } else { 1 });
        if resized {
            let at = out
                .iter()
                .position(|o| matches!(o, Out::Restart(_)))
                .unwrap();
            assert!(out[at..].iter().any(|o| matches!(
                o,
                Out::Init {
                    compatible: true,
                    ..
                }
            )));
        }
        for track in [1, 2] {
            let s = samples(&out, track);
            for w in s.windows(2) {
                assert!(w[1].0 >= w[0].0, "track {track} goes back at {w:?}");
            }
        }
        if !resized {
            // One run across the sessions: the last units of the first
            // last until the second's first.
            assert_contiguous(&samples(&out, 1));
        }
        // Audio goes on across the splice, its gap filled with silence
        // where a new init flushed the run.
        assert_contiguous(&samples(&out, 2));
        // Presentation times keep going.
        let pts: Vec<Duration> = out
            .iter()
            .filter_map(|o| match o {
                Out::Media { units, .. } => Some(units.clone()),
                _ => None,
            })
            .flatten()
            .filter(|u| u.kind == TrackKind::Audio)
            .map(|u| u.pts)
            .collect();
        assert!(pts.windows(2).all(|w| w[1] > w[0]));
        assert!(*pts.last().unwrap() > Duration::from_secs(15));
        assert_eq!(r.metrics().restarts, 1);
    }
}

fn is_end(record: &[u8]) -> bool {
    matches!(parse_record(record), Ok(Record::End { .. }))
}

/// An END writes out what the remuxer holds and outputs an end, once for
/// the record and the input's end after it, the picture's last unit lasting
/// to the end of the output; the session after it starts an epoch there,
/// whatever its timestamps, and plays on with no hole in the picture and
/// every track going forward.
#[test]
fn a_session_after_an_end_follows_the_output() {
    let first = unit_records(&script(b_frames()));
    assert!(is_end(first.last().unwrap()));
    // A later session whose timestamps go on from far ahead.
    let later = unit_records(&script(ScriptConfig {
        start_ms: 600_000,
        ..b_frames()
    }));
    let (alone, _) = remux(&first);
    let ended = alone.iter().position(|o| *o == Out::End).unwrap();
    assert_eq!(ended, alone.len() - 1, "one end, after the media");

    let mut marked: Vec<(Vec<u8>, bool)> = first.iter().map(|r| (r.clone(), false)).collect();
    marked.extend(later.iter().enumerate().map(|(i, r)| (r.clone(), i == 0)));
    let (out, r) = remux_marked(&marked);
    let ends: Vec<usize> = (0..out.len()).filter(|&i| out[i] == Out::End).collect();
    assert_eq!(ends.len(), 2);
    assert_eq!(&out[..=ends[0]], &alone[..]);
    let restarts: Vec<Duration> = out
        .iter()
        .filter_map(|o| match o {
            Out::Restart(offset) => Some(*offset),
            _ => None,
        })
        .collect();
    assert_eq!(restarts.len(), 1);
    // The later session's first unit decodes where the output ends, 8 s
    // in, its 600 s less.
    let at = Duration::from_secs(8) - Duration::from_secs(600);
    assert!(
        (restarts[0] - at).abs() < Duration::from_millis(50),
        "{restarts:?}"
    );
    assert_contiguous(&samples(&out, 1));
    // Audio goes on from where the first session's ends.
    let audio = samples(&out, 2);
    let split = samples(&alone, 2).len();
    let (last, next) = (audio[split - 1], audio[split]);
    assert!(next.0 >= last.0 + last.1, "{last:?} {next:?}");
    assert!(next.0 <= last.0 + last.1 + 1024, "{last:?} {next:?}");
    assert!(audio.windows(2).all(|w| w[1].0 >= w[0].0 + w[0].1));
    let m = r.metrics();
    assert_eq!((m.ends, m.restarts, m.dropped_backwards), (2, 1, 0));
    assert_eq!(m.reader.ends, 2);
}

/// A remuxer that joins a stream at its END outputs the end alone.
#[test]
fn a_join_at_the_end_outputs_only_the_end() {
    let records = unit_records(&script(b_frames()));
    let (out, r) = remux(&records[records.len() - 1..]);
    assert_eq!(out, [Out::End]);
    assert_eq!(r.metrics().ends, 1);
}

/// An H.264 track of `width` by `height` at 90 kHz.
fn video(track: u32, width: u16, height: u16) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: avcc(width, height),
            width,
            height,
        },
    }
}

/// An AAC LC stereo track at `rate`.
fn audio(track: u32, rate: u32) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: rate,
        params: CodecParams::Aac {
            asc: aac_lc_asc(rate),
            sample_rate: rate,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

fn unit(track: u32, dts: i64, cto: i64, keyframe: bool, len: usize) -> Event {
    Event::Unit(Unit {
        track: TrackId(track),
        dts,
        pts: dts + cto,
        keyframe,
        payload: vec![track as u8; len],
        published: Time::from_unix_millis(EPOCH_MS + dts / 90),
    })
}

/// Video at 30 fps from `from` (a keyframe every `gop`) and AAC frames at
/// `rate` between, `frames` pictures long, with a new audio config at
/// picture `change`.
fn av(frames: i64, gop: i64, rate: u32, change: Option<(i64, u32)>) -> Vec<Event> {
    let mut out = vec![
        Event::Config(video(1, 640, 360)),
        Event::Config(audio(2, rate)),
    ];
    let (mut rate, mut a, mut a_base) = (rate, 0i64, 0i64);
    for k in 0..frames {
        if let Some((at, to)) = change
            && at == k
        {
            out.push(Event::Config(audio(2, to)));
            // The new track's timestamps continue in its own timescale.
            a_base = k * 3000 * i64::from(to) / 90_000;
            a = 0;
            rate = to;
        }
        out.push(unit(1, k * 3000, 0, k % gop == 0, 200));
        while (a_base + a * 1024) * 90_000 < (k + 1) * 3000 * i64::from(rate) {
            let dts = a_base + a * 1024;
            out.push(Event::Unit(Unit {
                track: TrackId(2),
                dts,
                pts: dts,
                keyframe: true,
                payload: vec![2; 40],
                published: Time::from_unix_millis(EPOCH_MS + dts * 1000 / i64::from(rate)),
            }));
            a += 1;
        }
    }
    out
}

#[test]
fn a_config_changed_mid_gop_waits_for_the_cut_tracks_keyframe() {
    let events = av(150, 60, 48_000, Some((75, 44_100)));
    let (out, r) = remux(&unit_records(&events));
    let inits: Vec<_> = out
        .iter()
        .filter_map(|o| match o {
            Out::Init {
                tracks, compatible, ..
            } => Some((tracks.clone(), *compatible)),
            _ => None,
        })
        .collect();
    assert_eq!(inits.len(), 2);
    assert!(inits[1].1);
    assert_eq!(inits[1].0[1].timescale, 44_100);
    // The new audio waits for the keyframe at picture 120; video goes on.
    assert!(r.metrics().dropped_pending_config > 0);
    let video = samples(&out, 1);
    assert_eq!(video.len(), 150);
    assert_contiguous(&video);
    let at = out
        .iter()
        .rposition(|o| matches!(o, Out::Init { .. }))
        .unwrap();
    let Some(Out::Media { units, .. }) = out.get(at + 1) else {
        panic!("no media after the init");
    };
    assert!(units[0].keyframe && units[0].pts == Duration::from_secs(4));
}

#[test]
fn audio_only_streams_start_at_their_first_frame() {
    let mut events = vec![Event::Config(audio(2, 48_000))];
    for n in 0..200 {
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: n * 1024,
            pts: n * 1024,
            keyframe: true,
            payload: vec![2; 50],
            published: Time::from_unix_millis(EPOCH_MS + n * 1024 / 48),
        }));
    }
    let (out, _) = remux(&unit_records(&events));
    let Some(Out::Init { mime, tracks, .. }) = out.first() else {
        panic!("no init first: {:?}", out.first());
    };
    assert_eq!(mime, r#"audio/mp4;codecs="mp4a.40.2""#);
    assert_eq!(tracks.len(), 1);
    let audio = samples(&out, 2);
    assert_eq!(audio.len(), 200);
    assert_contiguous(&audio);
}

#[test]
fn a_join_at_a_sync_record_starts_with_its_init_and_a_join_elsewhere_waits() {
    let events = script(b_frames());
    let records = unit_records(&events);
    let syncs: Vec<usize> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(parse_record(r), Ok(Record::Configs { gop_back: 0, .. })))
        .map(|(i, _)| i)
        .collect();
    assert!(syncs.len() >= 3);
    let (out, r) = remux(&records[syncs[1]..]);
    let Some(Out::Init { .. }) = out.first() else {
        panic!("no init first");
    };
    let Some(Out::Media { units, .. }) = out.get(1) else {
        panic!("no media after the init");
    };
    assert!(units[0].keyframe && units[0].pts == Duration::from_secs(2));
    assert_eq!(r.metrics().dropped_before_init, 0);

    // Joined mid-GOP, nothing comes until the next GOP's sync record.
    let (out, r) = remux(&records[syncs[1] + 5..]);
    let Some(Out::Media { units, .. }) = out.get(1) else {
        panic!("no media after the init");
    };
    assert!(units[0].pts == Duration::from_secs(4));
    assert!(r.metrics().reader.unknown_tracks > 0);
}

#[test]
fn a_discontinuity_drops_the_record_in_progress() {
    let records = unit_records(&script(b_frames()));
    let mut r = Remuxer::new();
    let mut out = Vec::new();
    for rec in &records[..40] {
        send(&mut r, rec, false, &mut out);
    }
    r.message(&records[40][..records[40].len() / 2], false, false);
    for (i, rec) in records[41..].iter().enumerate() {
        send(&mut r, rec, i == 0, &mut out);
    }
    let m = r.metrics();
    assert_eq!((m.partial_records, m.discontinuities), (1, 1));
    assert_eq!(m.reader.malformed, 0);
}

#[test]
fn records_over_the_bound_are_dropped_as_lost() {
    let mut r = Remuxer::new();
    let chunk = vec![0x02; MAX_DATA];
    for _ in 0..=MAX_RECORD_LEN / MAX_DATA {
        r.message(&chunk, false, false);
    }
    r.message(&chunk, true, false);
    assert!(r.poll_output().is_none());
    assert!(r.record.capacity() <= MAX_RECORD_LEN + MAX_DATA);
    let records = unit_records(&script(b_frames()));
    let mut out = Vec::new();
    for rec in &records {
        send(&mut r, rec, false, &mut out);
    }
    let m = r.metrics();
    assert_eq!(
        (m.oversize_records, m.reader.lost, m.reader.malformed),
        (1, 1, 0)
    );
    assert!(m.fragments > 100);
}

#[test]
fn held_units_are_bounded() {
    // A run's first fragment waits for its keyframe to be timed, which a
    // video track that stops after it never is, while audio goes on.
    let mut events = vec![
        Event::Config(video(1, 640, 360)),
        Event::Config(audio(2, 48_000)),
    ];
    events.push(unit(1, 0, 0, true, 10));
    for n in 0..2 * MAX_HELD_UNITS as i64 {
        events.push(Event::Unit(Unit {
            track: TrackId(2),
            dts: n * 1024,
            pts: n * 1024,
            keyframe: true,
            payload: vec![2; 10],
            published: Time::from_unix_millis(EPOCH_MS + n * 1024 / 48),
        }));
    }
    let records = unit_records(&events);
    let mut r = Remuxer::new();
    let mut out = Vec::new();
    for rec in &records {
        send(&mut r, rec, false, &mut out);
        assert!(r.held.len() <= MAX_HELD_UNITS);
    }
    r.finish();
    drain(&mut r, &mut out);
    assert!(r.metrics().held_flushes >= 1);
    assert_eq!(samples(&out, 2).len(), 2 * MAX_HELD_UNITS);
}

#[test]
fn outputs_wait_until_taken() {
    let records = unit_records(&script(ScriptConfig::default()));
    let mut r = Remuxer::new();
    for rec in &records[..60] {
        r.message(rec, true, false);
    }
    let mut out = Vec::new();
    drain(&mut r, &mut out);
    let (all, _) = remux(&records[..60]);
    assert_eq!(out, all[..out.len()].to_vec());
    assert!(out.len() > 10);
}

fn proptest_config() -> Config {
    Config {
        cases: 64,
        rng_seed: RngSeed::Fixed(0x5e1a_7e00),
        failure_persistence: None,
        ..Config::default()
    }
}

/// Every output parses: inits as init segments, media against the init
/// before it.
fn check_outputs(r: &mut Remuxer, info: &mut Option<strims_video::fmp4::InitInfo>) -> usize {
    let mut n = 0;
    while let Some(o) = r.poll_output() {
        n += 1;
        match o {
            Output::Init { data, .. } => *info = Some(parse_init(data).unwrap()),
            Output::Media { data, units } => {
                media_duration(info.as_ref().unwrap(), data).unwrap();
                assert!(units.len() <= MAX_HELD_UNITS);
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
            (prop::collection::vec(any::<u8>(), 0..512), any::<bool>(), prop::bool::weighted(0.1)),
            0..64,
        ),
    ) {
        let mut r = Remuxer::new();
        let mut info = None;
        for (data, end, skip) in &messages {
            r.message(data, *end, *skip);
            prop_assert!(check_outputs(&mut r, &mut info) <= 2 * MAX_TRACKS + 4);
        }
        r.finish();
        check_outputs(&mut r, &mut info);
    }

    #[test]
    fn damaged_streams_never_panic_and_their_output_parses(
        flips in prop::collection::vec((any::<prop::sample::Index>(), any::<prop::sample::Index>(), 1u8..), 0..24),
        dropped in prop::collection::btree_set(0usize..400, 0..16),
        skips in prop::collection::btree_set(0usize..400, 0..8),
        cut in any::<prop::sample::Index>(),
    ) {
        let mut records = damaged_base().to_vec();
        for (rec, byte, xor) in &flips {
            let i = rec.index(records.len());
            if !records[i].is_empty() {
                let at = byte.index(records[i].len());
                records[i][at] ^= xor;
            }
        }
        let mut r = Remuxer::new();
        let mut info = None;
        let start = cut.index(records.len() / 2);
        for (i, rec) in records.iter().enumerate().skip(start) {
            if dropped.contains(&i) {
                continue;
            }
            r.message(rec, true, skips.contains(&i));
            check_outputs(&mut r, &mut info);
        }
        r.finish();
        check_outputs(&mut r, &mut info);
        prop_assert!(r.metrics().held_bytes <= MAX_HELD_BYTES as u64);
    }
}

/// A short stream's records, with B-frames and a resize.
fn damaged_base() -> &'static [Vec<u8>] {
    static BASE: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        unit_records(&script(ScriptConfig {
            duration_secs: 4,
            gop_frames: 30,
            resize_at_gop: Some(2),
            video_bitrate: 200_000,
            ..b_frames()
        }))
    })
}
