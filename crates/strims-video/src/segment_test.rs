// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::time::Time;
use strims_chunkstream::Frag;
use strims_chunkstream::packed::{DEFAULT_SIZE, FrameHasEnd, PackedDecoder};

use super::*;
use crate::fmp4::parse::{InitInfo, media_duration, parse_init, split_record};
use crate::fmp4::test_dump::{MediaDump, dump_media};
use crate::fmp4::{self, ntp_timestamp};
use crate::media::CodecParams;
use crate::record::{PackedSink, VecSink};
use crate::test_util::{SpsSpec, avcc, avcc_720p};

const VIDEO: TrackId = TrackId(1);
const AUDIO: TrackId = TrackId(2);
/// A frame at 30 fps on 90 kHz.
const FRAME: i64 = 3000;

fn video_config() -> DecoderConfig {
    DecoderConfig {
        track: VIDEO,
        kind: TrackKind::Video,
        codec: "avc1.640028".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: avcc_720p(),
            width: 1280,
            height: 720,
        },
    }
}

/// Another picture size.
fn video_config_small() -> DecoderConfig {
    DecoderConfig {
        codec: "avc1.42c01f".into(),
        params: CodecParams::Avc {
            avcc: avcc(&SpsSpec::baseline(20, 15).build()),
            width: 320,
            height: 240,
        },
        ..video_config()
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

fn rule(min_media_bytes: usize) -> CutRule {
    CutRule {
        min_media_bytes,
        ..CutRule::default()
    }
}

/// A fragment per unit, per 100 ms, and per record.
const SPANS: [Duration; 3] = [Duration::ZERO, Duration::from_millis(100), Duration::MAX];

fn spanned(min_media_bytes: usize, fragment_duration: Duration) -> CutRule {
    CutRule {
        min_media_bytes,
        fragment_duration,
        ..CutRule::default()
    }
}

/// Payload bytes identifying a unit: its track, then its DTS.
fn payload(track: TrackId, dts: i64, size: usize) -> Vec<u8> {
    let seed = (track.0 as u8) ^ (dts / 7) as u8;
    (0..size).map(|i| seed.wrapping_add(i as u8)).collect()
}

fn push<S: RecordSink>(
    s: &mut Segmenter<S>,
    track: TrackId,
    dts: i64,
    pts: i64,
    keyframe: bool,
    size: usize,
) -> Result<(), SegmentError> {
    let data = payload(track, dts, size);
    s.push(&AccessUnit {
        track,
        dts,
        pts,
        keyframe,
        payload: &data,
        published: stamp(track, dts),
    })
}

/// The publish time of a unit at `dts` of `track`.
fn stamp(track: TrackId, dts: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + i64::from(track.0) * 1_000_000 + dts)
}

fn video<S: RecordSink>(s: &mut Segmenter<S>, dts: i64, keyframe: bool, size: usize) {
    push(s, VIDEO, dts, dts, keyframe, size).unwrap();
}

fn audio<S: RecordSink>(s: &mut Segmenter<S>, dts: i64, size: usize) {
    push(s, AUDIO, dts, dts, true, size).unwrap();
}

/// Pushes video frames `from..to` at 30 fps, a keyframe every `gop`,
/// sized `key` and `inter`.
fn frames<S: RecordSink>(
    s: &mut Segmenter<S>,
    from: i64,
    to: i64,
    gop: i64,
    key: usize,
    inter: usize,
) {
    for k in from..to {
        let keyframe = k % gop == 0;
        video(s, k * FRAME, keyframe, if keyframe { key } else { inter });
    }
}

struct Parsed {
    info: InitInfo,
    init: Vec<u8>,
    media: MediaDump,
    duration: Duration,
}

fn parse(record: &[u8]) -> Parsed {
    let (init, media) = split_record(record).unwrap();
    let info = parse_init(init).unwrap();
    Parsed {
        info,
        init: init.to_vec(),
        media: dump_media(media),
        duration: media_duration(&info, media).unwrap(),
    }
}

fn records(s: &Segmenter<VecSink>) -> Vec<Parsed> {
    s.sink().records.iter().map(|r| parse(r)).collect()
}

fn infos<S: RecordSink>(s: &mut Segmenter<S>) -> Vec<SegmentInfo> {
    std::iter::from_fn(|| s.take_segment()).collect()
}

/// A packed sink that also collects the records whole, and the stream
/// offsets of their ends.
struct Tee {
    packed: PackedSink,
    whole: VecSink,
    stream: Vec<u8>,
    ends: Vec<u64>,
}

impl Tee {
    fn new(size: usize) -> Self {
        Tee {
            packed: PackedSink::with_size(size).unwrap(),
            whole: VecSink::new(),
            stream: Vec::new(),
            ends: Vec::new(),
        }
    }

    fn drain(&mut self) {
        let out = self.packed.take_output();
        self.stream.extend_from_slice(&out);
        self.packed.restore(out);
    }

    /// The packed stream decoded from offset 0.
    fn decoded(&mut self) -> Vec<Vec<u8>> {
        self.drain();
        decode(self.packed.encoder().size(), &self.stream)
    }

    /// Asserts each record end is in its own frame.
    fn assert_one_end_per_frame(&self) {
        let size = self.packed.encoder().size() as u64;
        let frames: Vec<u64> = self.ends.iter().map(|e| e / size).collect();
        let mut unique = frames.clone();
        unique.dedup();
        assert_eq!(frames, unique, "two record ends in one frame");
    }
}

impl RecordSink for Tee {
    fn end_blocked_after(&self, len: usize) -> bool {
        self.packed.end_blocked_after(len)
    }

    fn unblocking_len(&self, len: usize) -> usize {
        self.packed.unblocking_len(len)
    }

    fn write(&mut self, data: &[u8]) {
        self.packed.write(data);
        self.whole.write(data);
    }

    fn end_record(&mut self) -> Result<(), FrameHasEnd> {
        self.packed.end_record()?;
        self.whole.end_record()?;
        self.ends.push(self.packed.offset());
        Ok(())
    }
}

fn decode(size: usize, stream: &[u8]) -> Vec<Vec<u8>> {
    let mut decoder = PackedDecoder::new(size).unwrap();
    let (mut records, mut current, mut input) = (Vec::new(), Vec::new(), stream);
    while let (used, Some(frag)) = decoder.decode(input) {
        input = &input[used..];
        match frag {
            Frag::Data(d) => current.extend_from_slice(d),
            Frag::End => records.push(std::mem::take(&mut current)),
            Frag::Corrupt => panic!("corrupt frame"),
        }
    }
    records
}

#[test]
fn the_first_segment_starts_at_a_video_keyframe() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&audio_config()).unwrap();
    audio(&mut s, 0, 10);
    s.set_config(&video_config()).unwrap();
    video(&mut s, 0, false, 10);
    audio(&mut s, 1024, 10);
    video(&mut s, FRAME, true, 10);
    audio(&mut s, 2048, 10);
    video(&mut s, 2 * FRAME, false, 10);
    s.finish().unwrap();
    assert_eq!(s.metrics().dropped_before_start, 3);
    let [r] = &records(&s)[..] else {
        panic!("one record")
    };
    let ids: Vec<u32> = r.info.tracks().iter().map(|t| t.id).collect();
    assert_eq!(ids, [1, 2]);
    let v = r.media.traf(1);
    assert_eq!(v.tfdt, FRAME as u64);
    assert_eq!(v.samples.len(), 2);
    assert_eq!(v.samples[0].flags, Some(fmp4::SYNC_SAMPLE_FLAGS));
    assert_eq!(v.samples[1].flags, Some(fmp4::NON_SYNC_SAMPLE_FLAGS));
    assert_eq!(r.media.traf(2).tfdt, 2048);
}

#[test]
fn a_segment_waits_for_a_video_config() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&audio_config()).unwrap();
    for n in 0..10 {
        audio(&mut s, n * 1024, 10);
    }
    s.finish().unwrap();
    assert!(s.sink().records.is_empty());
    assert_eq!(s.metrics().dropped_before_start, 10);
}

#[test]
fn cuts_at_keyframes_once_the_minimum_is_buffered() {
    let mut s = Segmenter::new(rule(10_000), VecSink::new());
    s.set_config(&video_config()).unwrap();
    // GOPs of 4 frames, 6000 bytes: a segment takes two.
    frames(&mut s, 0, 20, 4, 3000, 1000);
    s.finish().unwrap();
    let counts: Vec<u32> = infos(&mut s).iter().map(|i| i.video_samples).collect();
    assert_eq!(counts, [8, 8, 4]);
    for (i, r) in records(&s).iter().enumerate() {
        let v = r.media.traf(1);
        assert_eq!(v.tfdt, (i as u64) * 8 * FRAME as u64);
        assert_eq!(v.samples[0].flags, Some(fmp4::SYNC_SAMPLE_FLAGS));
    }
}

#[test]
fn a_blocked_end_merges_the_next_group() {
    let mut s = Segmenter::new(rule(1000), Tee::new(DEFAULT_SIZE));
    s.set_config(&video_config()).unwrap();
    // GOPs of 1000 bytes: each reaches the minimum, but a second record
    // can't end in the frame holding the first's end.
    for gop in 0..200 {
        frames(&mut s, gop * 3, gop * 3 + 3, 3, 600, 200);
        s.sink_mut().drain();
    }
    s.finish().unwrap();
    let infos = infos(&mut s);
    assert_eq!(infos[0].video_samples, 3);
    assert!(infos[1].video_samples > 3, "{:?}", infos[1]);
    assert!(s.metrics().blocked_cuts > 0);
    // Only the end at finish is padded.
    assert!(infos[..infos.len() - 1].iter().all(|i| i.padding == 0));
    let tee = s.sink_mut();
    tee.assert_one_end_per_frame();
    assert_eq!(tee.decoded(), tee.whole.records);
    // The second record's end is past the first frame.
    assert!(tee.ends[1] >= DEFAULT_SIZE as u64);
}

#[test]
fn a_forced_cut_pads_a_blocked_end_with_free() {
    let mut s = Segmenter::new(rule(1000), Tee::new(DEFAULT_SIZE));
    s.set_config(&video_config()).unwrap();
    frames(&mut s, 0, 4, 3, 600, 200);
    // The first record has ended; a new config cuts at the next keyframe
    // although the segment is short of the minimum.
    s.set_config(&video_config_small()).unwrap();
    video(&mut s, 4 * FRAME, false, 200);
    video(&mut s, 5 * FRAME, false, 200);
    video(&mut s, 6 * FRAME, true, 600);
    video(&mut s, 7 * FRAME, false, 200);
    s.finish().unwrap();
    assert_eq!(s.metrics().dropped_pending_config, 2);
    let infos = infos(&mut s);
    assert_eq!(infos.len(), 3);
    assert_eq!(infos[1].video_samples, 1);
    assert!(infos[1].padding >= fmp4::FREE_HEADER_LEN);
    assert_eq!(
        (infos[0].new_init, infos[1].new_init, infos[2].new_init),
        (true, false, true)
    );
    let tee = s.sink_mut();
    tee.assert_one_end_per_frame();
    // The padding fills the frame: the end is in the next frame's header.
    assert_eq!(tee.ends[1], DEFAULT_SIZE as u64 + 4);
    assert_eq!(tee.decoded(), tee.whole.records);
    let r = parse(&tee.whole.records[1]);
    let boxes: Vec<&str> = r.media.boxes.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(boxes, ["prft", "moof", "mdat", "free"]);
    assert_eq!(r.media.boxes[3].1, infos[1].padding);
    assert_eq!(tee.whole.records[1].len(), infos[1].bytes);
    // Its media time is still the samples': the cut ends the last at the
    // keyframe.
    assert_eq!(
        r.duration,
        Duration::from_nanos(3 * FRAME * 1_000_000_000 / 90_000)
    );
    let small = parse(&tee.whole.records[2]);
    let mut init = Vec::new();
    write_init(&[video_config_small()], Duration::ZERO, &mut init).unwrap();
    assert_eq!(small.init, init);
    // finish pads the last record too when its end is blocked.
    assert!(infos[2].padding > 0);
}

#[test]
fn a_new_track_cuts_at_the_next_keyframe() {
    let mut s = Segmenter::new(rule(1_000_000), VecSink::new());
    s.set_config(&video_config()).unwrap();
    frames(&mut s, 0, 3, 3, 100, 10);
    s.set_config(&audio_config()).unwrap();
    audio(&mut s, 0, 10);
    audio(&mut s, 1024, 10);
    frames(&mut s, 3, 6, 3, 100, 10);
    audio(&mut s, 2048, 10);
    s.finish().unwrap();
    assert_eq!(s.metrics().dropped_pending_config, 2);
    let r = records(&s);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].info.tracks().len(), 1);
    assert_eq!(r[1].info.tracks().len(), 2);
    assert_eq!(r[1].media.traf(2).samples.len(), 1);
}

#[test]
fn an_unchanged_config_does_not_cut() {
    let mut s = Segmenter::new(rule(1_000_000), VecSink::new());
    s.set_config(&video_config()).unwrap();
    frames(&mut s, 0, 3, 3, 100, 10);
    s.set_config(&video_config()).unwrap();
    frames(&mut s, 3, 9, 3, 100, 10);
    s.finish().unwrap();
    assert_eq!(s.sink().records.len(), 1);
}

#[test]
fn a_record_over_the_bound_is_refused() {
    let limit = 50_000;
    let mut s = Segmenter::new(
        CutRule {
            min_media_bytes: 1 << 20,
            max_record_bytes: limit,
            ..CutRule::default()
        },
        VecSink::new(),
    );
    s.set_config(&video_config()).unwrap();
    video(&mut s, 0, true, 1000);
    let mut k = 1;
    let error = loop {
        if let Err(e) = push(&mut s, VIDEO, k * FRAME, k * FRAME, k % 10 == 0, 1000) {
            break e;
        }
        k += 1;
    };
    assert_eq!(error, SegmentError::RecordTooLarge { limit });
    assert!(
        error.to_string().contains("no keyframe arrived within"),
        "{error}"
    );
    // The record held what fits.
    assert!(k > 40);
    assert_eq!(
        push(&mut s, VIDEO, 0, 0, true, 1),
        Err(SegmentError::Closed)
    );
    assert_eq!(s.set_config(&audio_config()), Err(SegmentError::Closed));
    assert_eq!(s.finish(), Err(SegmentError::Closed));
}

#[test]
fn the_record_bound_is_at_most_the_limit() {
    let s = Segmenter::new(
        CutRule {
            min_media_bytes: 0,
            max_record_bytes: usize::MAX,
            ..CutRule::default()
        },
        VecSink::new(),
    );
    assert_eq!(s.rule().max_record_bytes, MAX_RECORD_LIMIT);
}

#[test]
fn the_last_video_sample_lasts_until_the_cutting_keyframe() {
    let mut s = Segmenter::new(rule(100), VecSink::new());
    s.set_config(&video_config()).unwrap();
    for (dts, key) in [(0, true), (3000, false), (7000, false), (9000, false)] {
        video(&mut s, dts, key, 50);
    }
    video(&mut s, 12_500, true, 50);
    video(&mut s, 15_500, false, 50);
    s.finish().unwrap();
    let r = records(&s);
    assert_eq!(r[0].media.traf(1).durations(), [3000, 4000, 2000, 3500]);
    // finish repeats the previous duration.
    assert_eq!(r[1].media.traf(1).durations(), [3000, 3000]);
    assert_eq!(r[1].media.traf(1).tfdt, 12_500);
    assert_eq!(
        r[0].duration,
        Duration::from_nanos(12_500 * 1_000_000_000 / 90_000)
    );
}

#[test]
fn a_lone_sample_at_finish_lasts_a_thirtieth_of_a_second() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    video(&mut s, 0, true, 50);
    s.finish().unwrap();
    assert_eq!(records(&s)[0].media.traf(1).durations(), [3000]);
}

#[test]
fn a_lone_sample_after_a_cut_repeats_the_previous_segment() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    video(&mut s, 0, true, 50);
    video(&mut s, 3600, true, 50);
    s.finish().unwrap();
    let r = records(&s);
    assert_eq!(r[0].media.traf(1).durations(), [3600]);
    assert_eq!(r[1].media.traf(1).durations(), [3600]);
}

#[test]
fn audio_samples_last_until_the_next_and_the_last_a_frame() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    s.set_config(&audio_config()).unwrap();
    video(&mut s, 0, true, 50);
    for dts in [0, 1024, 2048, 3100] {
        audio(&mut s, dts, 20);
    }
    s.finish().unwrap();
    let r = records(&s);
    let a = r[0].media.traf(2);
    assert_eq!(a.durations(), [1024, 1024, 1052, 1024]);
    assert_eq!(a.trun_flags, 0x301);
    assert!(a.samples.iter().all(|s| s.flags.is_none()));
}

#[test]
fn composition_offsets_are_signed() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    // I P B B, the last shown before its decode time.
    for (dts, pts, key) in [
        (0, 3000, true),
        (3000, 12_000, false),
        (6000, 6000, false),
        (9000, 6000, false),
    ] {
        push(&mut s, VIDEO, dts, pts, key, 40).unwrap();
    }
    s.finish().unwrap();
    let ctos: Vec<Option<i32>> = records(&s)[0]
        .media
        .traf(1)
        .samples
        .iter()
        .map(|s| s.composition_offset)
        .collect();
    assert_eq!(ctos, [Some(3000), Some(9000), Some(0), Some(-3000)]);
}

#[test]
fn decode_times_continue_across_segments() {
    let mut s = Segmenter::new(rule(4000), VecSink::new());
    s.set_config(&video_config()).unwrap();
    s.set_config(&audio_config()).unwrap();
    let mut n = 0;
    for k in 0..300 {
        let dts = 1_000_000 + k * FRAME;
        video(
            &mut s,
            dts,
            k % 15 == 0,
            if k % 15 == 0 { 400 } else { 100 },
        );
        // 48 kHz audio frames up to this video frame's time.
        while (n * 1024) * 90_000 / 48_000 <= k * FRAME {
            audio(&mut s, 555_555 + n * 1024, 30);
            n += 1;
        }
    }
    s.finish().unwrap();
    let r = records(&s);
    assert!(r.len() > 5);
    for pair in r.windows(2) {
        for track in [1, 2] {
            let (a, b) = (pair[0].media.traf(track), pair[1].media.traf(track));
            assert_eq!(a.tfdt + a.duration(), b.tfdt, "track {track}");
        }
    }
    let infos = infos(&mut s);
    for (record, info) in r.iter().zip(&infos) {
        assert_eq!(record.duration, info.duration);
    }
}

#[test]
fn segment_infos_describe_the_records() {
    let mut s = Segmenter::new(rule(500), VecSink::new());
    s.set_config(&video_config()).unwrap();
    s.set_config(&audio_config()).unwrap();
    for k in 0..12 {
        video(&mut s, k * FRAME, k % 3 == 0, 200);
        audio(&mut s, k * 1024, 30);
    }
    s.set_config(&video_config_small()).unwrap();
    video(&mut s, 12 * FRAME, true, 200);
    s.finish().unwrap();
    let infos = infos(&mut s);
    let records = &s.sink().records;
    assert_eq!(infos.len(), records.len());
    for (info, record) in infos.iter().zip(records) {
        assert_eq!(info.bytes, record.len());
        assert_eq!(info.padding, 0);
        let parsed = parse(record);
        assert_eq!(
            info.video_samples as usize,
            parsed.media.traf(1).samples.len()
        );
        let audio = if parsed.media.has_traf(2) {
            parsed.media.traf(2).samples.len()
        } else {
            0
        };
        assert_eq!(info.audio_samples as usize, audio);
    }
    // Cuts at frames 3, 6, 9 and, for the new config, 12.
    let new: Vec<bool> = infos.iter().map(|i| i.new_init).collect();
    assert_eq!(new, [true, false, false, false, true]);
    assert_eq!(infos[4].audio_samples, 0);
    assert_eq!(s.segment_configs(), [video_config_small(), audio_config()]);
    let m = s.metrics();
    assert_eq!(m.records, 5);
    assert_eq!(
        m.record_bytes,
        records.iter().map(|r| r.len() as u64).sum::<u64>()
    );
}

#[test]
fn units_of_unknown_tracks_are_refused() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    assert_eq!(
        push(&mut s, TrackId(5), 0, 0, true, 1),
        Err(SegmentError::UnknownTrack(5))
    );
    video(&mut s, 0, true, 1);
    s.finish().unwrap();
    assert_eq!(s.sink().records.len(), 1);
}

#[test]
fn config_sets_that_cannot_be_declared_are_refused() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    for i in 2..=8 {
        s.set_config(&DecoderConfig {
            track: TrackId(i),
            ..audio_config()
        })
        .unwrap();
    }
    assert_eq!(
        s.set_config(&DecoderConfig {
            track: TrackId(9),
            ..audio_config()
        }),
        Err(SegmentError::Config(InitError::TooManyTracks))
    );
    let mut s = Segmenter::new(rule(0), VecSink::new());
    let big = |track, len| DecoderConfig {
        track: TrackId(track),
        params: CodecParams::Avc {
            avcc: vec![0; len],
            width: 16,
            height: 16,
        },
        ..video_config()
    };
    s.set_config(&big(1, 40_000)).unwrap();
    assert_eq!(
        s.set_config(&big(3, 30_000)),
        Err(SegmentError::InitTooLong)
    );
    // The refused config isn't set.
    video(&mut s, 0, true, 1);
    s.finish().unwrap();
    assert_eq!(parse(&s.sink().records[0]).info.tracks().len(), 1);
}

#[test]
fn the_segmenter_continues_after_finish() {
    let mut s = Segmenter::new(rule(0), VecSink::new());
    s.set_config(&video_config()).unwrap();
    video(&mut s, 0, true, 10);
    s.finish().unwrap();
    s.finish().unwrap();
    video(&mut s, FRAME, false, 10);
    video(&mut s, 2 * FRAME, true, 10);
    s.finish().unwrap();
    let r = records(&s);
    assert_eq!(r.len(), 2);
    assert_eq!(r[1].media.traf(1).tfdt, 2 * FRAME as u64);
    assert_eq!(r[1].media.moofs[0].sequence, 2);
    assert_eq!(s.metrics().dropped_before_start, 1);
}

/// Video at 30 fps with B-frames from `start` s on, a keyframe every
/// `gop`, and 48 kHz audio up to each frame.
fn b_frame_stream<S: RecordSink>(s: &mut Segmenter<S>, start: i64, frames: i64, gop: i64) {
    let first = start * 30;
    let mut n = start * 48_000 / 1024;
    for k in first..first + frames {
        let key = (k - first) % gop == 0;
        // Shown in the order I B P B P …, two frames after decoding
        // starts: a keyframe two frames late, a P three and a B one.
        let cto = if key {
            2 * FRAME
        } else if (k - first) % 2 == 1 {
            3 * FRAME
        } else {
            FRAME
        };
        push(
            s,
            VIDEO,
            k * FRAME,
            k * FRAME + cto,
            key,
            if key { 400 } else { 100 },
        )
        .unwrap();
        while n * 1024 * 90_000 / 48_000 <= k * FRAME {
            audio(s, n * 1024, 30);
            n += 1;
        }
    }
}

fn av_segmenter(rule: CutRule) -> Segmenter<VecSink> {
    let mut s = Segmenter::new(rule, VecSink::new());
    s.set_config(&video_config()).unwrap();
    s.set_config(&audio_config()).unwrap();
    s
}

#[test]
fn a_fragment_per_unit_writes_each_unit_once_its_next_times_it() {
    let mut s = av_segmenter(spanned(2000, Duration::ZERO));
    b_frame_stream(&mut s, 10, 60, 15);
    s.finish().unwrap();
    let records = records(&s);
    assert!(records.len() >= 3);
    let mut sequence = 1;
    for r in &records {
        let last = r.media.moofs.len() - 1;
        for (i, m) in r.media.moofs.iter().enumerate() {
            assert_eq!(m.sequence, sequence);
            sequence += 1;
            let samples: usize = m.trafs.iter().map(|t| t.samples.len()).sum();
            // The first fragment holds the keyframe and the audio timed
            // while it waited; the last, each track's last sample.
            if i > 0 && i < last {
                assert_eq!(samples, 1, "fragment {i}");
            }
        }
        for track in [1, 2] {
            for pair in r.media.trafs(track).windows(2) {
                assert_eq!(
                    pair[0].tfdt + pair[0].duration(),
                    pair[1].tfdt,
                    "track {track}"
                );
            }
        }
        let first = &r.media.moofs[0].trafs[0];
        assert_eq!(
            (first.track, first.samples[0].flags),
            (1, Some(fmp4::SYNC_SAMPLE_FLAGS))
        );
        assert_eq!(r.media.moofs[last].trafs.len(), 2);
    }
}

#[test]
fn records_hold_the_same_samples_whatever_their_fragments() {
    let mut by_span = Vec::new();
    for span in SPANS {
        let mut s = av_segmenter(spanned(2000, span));
        b_frame_stream(&mut s, 10, 90, 15);
        s.finish().unwrap();
        let infos = infos(&mut s);
        let records: Vec<_> = records(&s)
            .into_iter()
            .map(|r| {
                (
                    r.init,
                    r.media.traf(1),
                    r.media.traf(2),
                    r.duration,
                    r.media.moofs.len(),
                )
            })
            .collect();
        by_span.push((infos, records));
    }
    let (infos, whole) = &by_span[2];
    assert!(whole.iter().all(|r| r.4 == 1));
    for (span, (other_infos, records)) in SPANS.iter().zip(&by_span) {
        assert_eq!(records.len(), whole.len(), "{span}");
        for (r, w) in records.iter().zip(whole) {
            assert_eq!((&r.0, &r.1, &r.2, r.3), (&w.0, &w.1, &w.2, w.3), "{span}");
        }
        for (a, b) in other_infos.iter().zip(infos) {
            assert_eq!(
                (a.duration, a.video_samples, a.audio_samples, a.new_init),
                (b.duration, b.video_samples, b.audio_samples, b.new_init)
            );
        }
    }
    // A fragment per unit makes the most fragments.
    assert!(
        by_span[0]
            .1
            .iter()
            .zip(&by_span[1].1)
            .all(|(u, g)| u.4 > g.4)
    );
}

#[test]
fn a_stream_decoding_from_zero_writes_its_first_gop_as_one_fragment() {
    let mut s = av_segmenter(spanned(2000, Duration::ZERO));
    b_frame_stream(&mut s, 0, 45, 15);
    s.finish().unwrap();
    let r = records(&s);
    // The first GOP decodes from before the two-frame edit.
    assert_eq!(r[0].media.moofs.len(), 1);
    assert_eq!(r[0].media.traf(1).samples.len(), 15);
    assert!(r[1].media.moofs.len() > 15);
}

#[test]
fn fragments_reach_the_sink_before_their_record_ends() {
    let mut s = av_segmenter(spanned(1_000_000, Duration::from_millis(100)));
    b_frame_stream(&mut s, 10, 30, 30);
    // No record has ended, but its fragments are written.
    assert!(s.sink().records.is_empty());
    let current = s.sink().current().to_vec();
    let (init, media) = split_record(&current).unwrap();
    let d = dump_media(media);
    assert!(d.moofs.len() >= 8, "{}", d.moofs.len());
    // Only the samples a fragment hasn't timed or reached the span with
    // are held.
    let info = parse_init(init).unwrap();
    let written = media_duration(&info, media).unwrap();
    assert!(written >= Duration::from_millis(866), "{written}");
}

#[test]
fn each_fragment_carries_its_first_units_publish_time() {
    let mut s = av_segmenter(spanned(2000, Duration::ZERO));
    b_frame_stream(&mut s, 0, 30, 15);
    s.finish().unwrap();
    for r in records(&s) {
        assert_eq!(r.media.prfts.len(), r.media.moofs.len());
        for (prft, moof) in r.media.prfts.iter().zip(&r.media.moofs) {
            let prft = prft.unwrap();
            let traf = &moof.trafs[0];
            let dts = traf.tfdt as i64;
            let pts = dts + i64::from(traf.samples[0].composition_offset.unwrap_or(0));
            assert_eq!((prft.version, prft.flags), (1, 1));
            assert_eq!(prft.track, traf.track);
            assert_eq!(prft.media_time, pts as u64);
            assert_eq!(prft.ntp, ntp_timestamp(stamp(TrackId(traf.track), dts)));
        }
    }
}

/// The `elst` media time of each `trak` in `init`, if it has one.
fn edit_media_times(init: &[u8]) -> Vec<Option<i32>> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = init[at..].windows(4).position(|w| w == b"trak") {
        let trak = at + i - 4;
        let len = u32::from_be_bytes(init[trak..trak + 4].try_into().unwrap()) as usize;
        let body = &init[trak..trak + len];
        out.push(body.windows(4).position(|w| w == b"elst").map(|e| {
            // After the header: version and flags, entry_count,
            // segment_duration.
            let t = e + 4 + 12;
            i32::from_be_bytes(body[t..t + 4].try_into().unwrap())
        }));
        at = trak + len;
    }
    out
}

#[test]
fn the_composition_delay_is_the_starting_keyframes_and_stays_with_the_configs() {
    let mut s = av_segmenter(rule(0));
    // The first keyframe shows two frames late; a later one, three.
    push(&mut s, VIDEO, 0, 2 * FRAME, true, 10).unwrap();
    audio(&mut s, 0, 10);
    push(&mut s, VIDEO, FRAME, 4 * FRAME, true, 10).unwrap();
    audio(&mut s, 1024, 10);
    s.set_config(&video_config_small()).unwrap();
    push(&mut s, VIDEO, 2 * FRAME, 3 * FRAME, true, 10).unwrap();
    // Without B-frames, no edit list.
    s.set_config(&video_config()).unwrap();
    push(&mut s, VIDEO, 3 * FRAME, 3 * FRAME, true, 10).unwrap();
    s.finish().unwrap();
    let r = records(&s);
    let edits: Vec<Vec<Option<i32>>> = r.iter().map(|r| edit_media_times(&r.init)).collect();
    assert_eq!(
        edits,
        [
            vec![Some(6000), Some(3200)],
            vec![Some(6000), Some(3200)],
            vec![Some(3000), Some(1600)],
            vec![None, None],
        ]
    );
    let new: Vec<bool> = infos(&mut s).iter().map(|i| i.new_init).collect();
    assert_eq!(new, [true, false, true, true]);
}

#[derive(Debug, Clone)]
enum Op {
    Config(u8),
    Unit {
        track: u32,
        step: i64,
        cto: i64,
        keyframe: bool,
        size: usize,
    },
    Finish,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        1 => (0u8..5).prop_map(Op::Config),
        12 => (1u32..=4, -200i64..6000, -9000i64..9000, any::<bool>(), 0usize..1500).prop_map(
            |(track, step, cto, keyframe, size)| Op::Unit { track, step, cto, keyframe, size }
        ),
        1 => Just(Op::Finish),
    ]
}

fn op_config(n: u8) -> DecoderConfig {
    match n {
        0 => video_config(),
        1 => video_config_small(),
        2 => audio_config(),
        3 => DecoderConfig {
            timescale: 44_100,
            params: CodecParams::Aac {
                asc: vec![0x12, 0x10],
                sample_rate: 44_100,
                channels: 0,
                frame_samples: 1024,
            },
            ..audio_config()
        },
        _ => DecoderConfig {
            track: TrackId(3),
            ..video_config_small()
        },
    }
}

/// Runs `ops`, stopping at a fatal error.
fn run<S: RecordSink>(s: &mut Segmenter<S>, ops: &[Op]) {
    let mut dts = [0i64; 5];
    for op in ops {
        let result = match op {
            Op::Config(n) => s.set_config(&op_config(*n)),
            Op::Unit {
                track,
                step,
                cto,
                keyframe,
                size,
            } => {
                let d = &mut dts[*track as usize];
                *d += step;
                push(s, TrackId(*track), *d, *d + cto, *keyframe, *size)
            }
            Op::Finish => s.finish(),
        };
        if matches!(
            result,
            Err(SegmentError::RecordTooLarge { .. } | SegmentError::Closed)
        ) {
            return;
        }
        let _ = s.take_segment();
    }
    let _ = s.finish();
}

fn proptest_config() -> Config {
    Config {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x5e67_3e47),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn arbitrary_unit_sequences_make_valid_records(ops in proptest::collection::vec(op(), 0..300)) {
        for fragment_duration in SPANS {
            let small = CutRule {
                min_media_bytes: 2000,
                max_record_bytes: 40_000,
                fragment_duration,
            };
            let mut s = Segmenter::new(small, VecSink::new());
            run(&mut s, &ops);
            for record in &s.sink().records {
                let r = parse(record);
                prop_assert!(!r.media.moofs.is_empty());
                let first = &r.media.moofs[0].trafs[0];
                prop_assert_eq!(first.samples[0].flags, Some(fmp4::SYNC_SAMPLE_FLAGS));
                prop_assert!(record.len() <= small.max_record_bytes);
                prop_assert!(r.media.prfts.iter().all(Option::is_some));
            }

            // Small frames block most ends.
            let mut s = Segmenter::new(small, Tee::new(700));
            run(&mut s, &ops);
            let tee = s.sink_mut();
            tee.assert_one_end_per_frame();
            prop_assert_eq!(tee.decoded(), tee.whole.records.clone());
            for record in &tee.whole.records {
                parse(record);
            }
        }
    }
}
