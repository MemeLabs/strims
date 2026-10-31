// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The HLS muxer's parts: where they end, how long they last, which are
//! independent, and how they make up their segments.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::fmp4::test_dump::{TrafDump, dump_media};
use crate::fmp4::{media_duration, parse_init};
use crate::media::CodecParams;
use crate::synth::script::avcc;
use crate::units::test_support::{audio, unit, video};

const VIDEO: u32 = 1;
const AUDIO: u32 = 2;
/// 90 kHz ticks of a frame at 30 frames a second.
const FRAME: i64 = 3000;
/// 48 kHz samples of an AAC frame.
const AAC_FRAME: i64 = 1024;
/// A `trun` entry's flag of a sample that isn't a sync sample.
const NON_SYNC: u32 = 0x0001_0000;

fn ms(n: i64) -> Duration {
    Duration::from_millis(n)
}

/// A muxer of 4 s segments cut into parts of `part_target`.
fn muxer(part_target: Duration, with_audio: bool) -> HlsMuxer {
    let mut m = HlsMuxer::new(HlsMuxerConfig {
        part_target,
        ..HlsMuxerConfig::default()
    });
    m.set_config(&video(VIDEO)).unwrap();
    if with_audio {
        m.set_config(&audio(AUDIO)).unwrap();
    }
    m
}

/// Video frames at the 90 kHz decode times `dts` gives frame numbers,
/// keyframes at `keyframes`.
fn frames(n: i64, dts: impl Fn(i64) -> i64, keyframes: &[i64]) -> Vec<AccessUnit<'static>> {
    (0..n)
        .map(|f| {
            let key = keyframes.contains(&f);
            unit(VIDEO, dts(f), key, if key { 4000 } else { 1500 })
        })
        .collect()
}

/// AAC frames from 0 up to the 90 kHz time `end`.
fn aac(end: i64) -> Vec<AccessUnit<'static>> {
    (0..end * 48_000 / 90_000 / AAC_FRAME)
        .map(|i| unit(AUDIO, i * AAC_FRAME, true, 300))
        .collect()
}

/// Video and audio units by time, video first at a tie, the video coming
/// `lead` (90 kHz) behind the audio of its time.
fn merge(
    video: Vec<AccessUnit<'static>>,
    audio: Vec<AccessUnit<'static>>,
    lead: i64,
) -> Vec<AccessUnit<'static>> {
    let mut out = Vec::with_capacity(video.len() + audio.len());
    let (mut v, mut a) = (video.into_iter().peekable(), audio.into_iter().peekable());
    loop {
        let take_video = match (v.peek(), a.peek()) {
            (Some(x), Some(y)) => (x.dts + lead) * 48_000 <= y.dts * 90_000,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return out,
        };
        out.extend(if take_video { v.next() } else { a.next() });
    }
}

/// `n` frames at 30 a second, keyframes at `keyframes`, with AAC over
/// their time when `with_audio`.
fn av(n: i64, keyframes: &[i64], with_audio: bool) -> Vec<AccessUnit<'static>> {
    let video = frames(n, |f| f * FRAME, keyframes);
    if !with_audio {
        return video;
    }
    merge(video, aac(n * FRAME), 0)
}

/// A segment and the parts taken before it.
struct Cut {
    parts: Vec<HlsPart>,
    segment: HlsSegment,
}

/// Pushes `units` and finishes, taking every part and then every segment
/// after each call: each segment with the parts taken since the one
/// before.
fn run(m: &mut HlsMuxer, units: &[AccessUnit<'_>]) -> Vec<Cut> {
    let mut out = Vec::new();
    let mut parts = Vec::new();
    let mut take = |m: &mut HlsMuxer| {
        parts.extend(std::iter::from_fn(|| m.take_part()));
        while let Some(segment) = m.take_segment() {
            out.push(Cut {
                parts: std::mem::take(&mut parts),
                segment,
            });
        }
    };
    for u in units {
        m.push(u).unwrap();
        take(m);
    }
    m.finish().unwrap();
    take(m);
    assert!(parts.is_empty(), "parts after the last segment");
    out
}

/// The `traf`s of `track` in `media`.
fn trafs(media: &[u8], track: u32) -> Vec<TrafDump> {
    dump_media(media)
        .trafs(track)
        .into_iter()
        .cloned()
        .collect()
}

/// A track's samples over `cuts` in order, as (decode time, duration).
fn samples(cuts: &[Cut], track: u32) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for c in cuts {
        for t in trafs(&c.segment.media, track) {
            let mut dts = t.tfdt;
            for s in &t.samples {
                out.push((dts, u64::from(s.duration)));
                dts += u64::from(s.duration);
            }
        }
    }
    out
}

/// Whether each sample starts where the one before ends.
fn contiguous(samples: &[(u64, u64)]) -> bool {
    samples.windows(2).all(|w| w[0].0 + w[0].1 == w[1].0)
}

/// What every stream's parts keep: a segment's parts, numbered from 0,
/// are its media back to back and add up to its duration; each carries
/// its segment's init, discontinuity and publish time, is fragments
/// after `prft`s, and is independent when its first video sample is a
/// keyframe.
fn check(cuts: &[Cut]) {
    for (i, c) in cuts.iter().enumerate() {
        let s = &c.segment;
        assert!(!c.parts.is_empty(), "segment {i} has no parts");
        let media: Vec<u8> = c
            .parts
            .iter()
            .flat_map(|p| p.media.iter().copied())
            .collect();
        assert!(media == s.media, "segment {i}'s parts aren't its media");
        let sum = c
            .parts
            .iter()
            .fold(Duration::ZERO, |sum, p| sum + p.duration);
        assert_eq!(sum, s.duration, "segment {i}");
        for (j, p) in c.parts.iter().enumerate() {
            assert_eq!(p.index as usize, j, "segment {i}");
            assert!(Rc::ptr_eq(&p.init, &s.init), "segment {i} part {j}");
            assert_eq!(
                (p.discontinuity, p.published),
                (s.discontinuity, s.published)
            );
            assert!(p.duration >= Duration::ZERO);
            let d = dump_media(&p.media);
            for (k, (b, _)) in d.boxes.iter().enumerate() {
                assert_eq!(b, ["prft", "moof", "mdat"][k % 3], "segment {i} part {j}");
            }
            let first = d
                .trafs(VIDEO)
                .first()
                .and_then(|t| t.samples.first().cloned());
            let key = first.is_some_and(|s| s.flags.unwrap() & NON_SYNC == 0);
            assert_eq!(p.independent, key, "segment {i} part {j}");
        }
    }
}

fn millis(d: Duration) -> i64 {
    (d.as_nanos() + 500_000) / 1_000_000
}

fn durations(c: &Cut) -> Vec<i64> {
    c.parts.iter().map(|p| millis(p.duration)).collect()
}

#[test]
fn a_segments_parts_are_its_media_and_add_up_to_its_duration() {
    let keyframes: Vec<i64> = (0..10).map(|g| g * 60).collect();
    let mut m = muxer(ms(1000), true);
    let cuts = run(&mut m, &av(600, &keyframes, true));
    check(&cuts);
    assert_eq!(cuts.len(), 10);
    for (i, c) in cuts.iter().enumerate() {
        assert_eq!(durations(c), [1000, 1000], "segment {i}");
        let independent: Vec<bool> = c.parts.iter().map(|p| p.independent).collect();
        assert_eq!(independent, [true, false], "segment {i}");
        assert_eq!(
            c.segment.published,
            Time::from_unix_millis(1_793_275_200_000 + 2000 * 90 * i as i64)
        );
    }
    let met = m.metrics();
    assert_eq!(
        (met.parts, met.overlong_parts, met.dropped_parts),
        (20, 0, 0)
    );
}

#[test]
fn no_part_target_makes_no_parts() {
    let mut m = muxer(Duration::ZERO, true);
    let units = av(120, &[0, 60], true);
    for u in &units {
        m.push(u).unwrap();
    }
    m.finish().unwrap();
    assert!(m.take_part().is_none());
    assert_eq!(m.metrics().parts, 0);
    assert!(m.take_segment().is_some());
}

#[test]
fn parts_last_at_most_the_target_and_at_least_it_less_a_frame() {
    // Frame rates as fractions, the decode times exact or from FLV's
    // millisecond tag times.
    let rates = [
        ("30", 30, 1, false),
        ("30 flv", 30, 1, true),
        ("29.97 flv", 30_000, 1001, true),
        ("25", 25, 1, false),
        ("24 flv", 24, 1, true),
        ("60 flv", 60, 1, true),
    ];
    for (name, num, den, flv) in rates {
        let dts = move |f: i64| {
            if flv {
                (f * 1000 * den + num / 2) / num * 90
            } else {
                f * 90_000 * den / num
            }
        };
        let fps = (num + den - 1) / den;
        for target in [ms(1000), ms(500), ms(333)] {
            // 2 s GOPs.
            let n = fps * 12;
            let keyframes: Vec<i64> = (0..n).step_by(2 * fps as usize).collect();
            let video = frames(n, dts, &keyframes);
            let end = video.last().unwrap().dts;
            let mut m = muxer(target, true);
            let cuts = run(&mut m, &merge(video, aac(end), 0));
            check(&cuts);
            let frame = Duration::from_nanos(1_000_000_000 / fps) + ms(1);
            for c in &cuts {
                for (j, p) in c.parts.iter().enumerate() {
                    assert!(p.duration <= target, "{name} {target:?}: {:?}", p.duration);
                    let last = j + 1 == c.parts.len();
                    if !last {
                        assert!(
                            p.duration + frame > target,
                            "{name} {target:?}: {:?}",
                            p.duration
                        );
                    }
                    if !last && !p.independent {
                        assert!(p.duration * 100 >= target * 85, "{name} {target:?}");
                    }
                }
            }
            assert_eq!(m.metrics().overlong_parts, 0, "{name} {target:?}");
        }
    }
}

#[test]
fn parts_of_a_split_gop_start_mid_gop() {
    // A 10 s GOP, split into segments within the 4 s target.
    let mut m = muxer(ms(1000), true);
    let cuts = run(&mut m, &av(330, &[0, 300], true));
    check(&cuts);
    let starts: Vec<(bool, bool)> = cuts
        .iter()
        .map(|c| (c.segment.keyframe, c.parts[0].independent))
        .collect();
    assert_eq!(starts[0], (true, true));
    assert!(starts.contains(&(false, false)), "{starts:?}");
    assert_eq!(cuts.iter().filter(|c| c.parts[0].independent).count(), 2);
}

#[test]
fn a_keyframe_within_the_floor_stays_inside_a_part() {
    // Keyframes 0.233 s apart, then 1.767 s later.
    let mut m = muxer(ms(1000), true);
    let cuts = run(&mut m, &av(120, &[0, 7, 60], true));
    check(&cuts);
    assert_eq!(durations(&cuts[0]), [1000, 1000]);
}

#[test]
fn audio_goes_on_in_parts_while_the_video_stalls() {
    // No video for 6 s from frame 60, the audio going on.
    let units: Vec<AccessUnit<'static>> = av(330, &[0, 300], true)
        .into_iter()
        .filter(|u| u.track.0 != VIDEO || !(60 * FRAME..240 * FRAME).contains(&u.dts))
        .collect();
    let mut m = muxer(ms(1000), true);
    let cuts = run(&mut m, &units);
    check(&cuts);
    let parts: Vec<&HlsPart> = cuts.iter().flat_map(|c| &c.parts).collect();
    assert!(parts.iter().all(|p| p.duration <= ms(1000)));
    let audio_only = parts
        .iter()
        .filter(|p| trafs(&p.media, VIDEO).is_empty())
        .count();
    assert!(audio_only >= 4, "{audio_only}");
    assert!(contiguous(&samples(&cuts, AUDIO)));
    let met = m.metrics();
    assert_eq!(met.overlong_parts, 0, "{met:?}");
}

#[test]
fn video_behind_its_audio_keeps_its_timeline_in_parts() {
    let keyframes: Vec<i64> = (0..600).step_by(60).collect();
    for lead in [ms(30), ms(150), ms(400)] {
        let video = frames(600, |f| f * FRAME, &keyframes);
        let units = merge(video, aac(600 * FRAME), lead.as_millis() * 90);
        let mut m = muxer(ms(1000), true);
        let cuts = run(&mut m, &units);
        check(&cuts);
        let v = samples(&cuts, VIDEO);
        assert!(contiguous(&v), "{lead:?}: {v:?}");
        assert!(v.iter().all(|s| s.1 == FRAME as u64), "{lead:?}");
        assert!(contiguous(&samples(&cuts, AUDIO)), "{lead:?}");
        assert!(
            cuts.iter()
                .flat_map(|c| &c.parts)
                .all(|p| p.duration <= ms(1000)),
            "{lead:?}"
        );
        // The segments' times don't run ahead of the media's.
        let total = cuts
            .iter()
            .fold(Duration::ZERO, |sum, c| sum + c.segment.duration);
        assert!(total <= ms(20_000) + lead, "{lead:?}: {total:?}");
        assert!(total >= ms(20_000) - lead, "{lead:?}: {total:?}");
    }
}

#[test]
fn a_frame_longer_than_the_target_makes_a_part_of_its_own() {
    // One frame a second under a 0.5 s target.
    let video = frames(12, |f| f * 90_000, &[0, 4, 8]);
    let mut m = muxer(ms(500), false);
    let cuts = run(&mut m, &video);
    check(&cuts);
    for c in &cuts {
        assert_eq!(durations(c), [1000; 4][..c.parts.len()]);
    }
    let met = m.metrics();
    assert_eq!(met.overlong_parts, met.parts);
}

#[test]
fn a_gap_a_new_init_and_the_end_close_their_segments_last_parts() {
    let units = av(240, &[0, 60, 120, 180], true);
    let mut m = muxer(ms(1000), true);
    let mut smaller = video(VIDEO);
    smaller.params = CodecParams::Avc {
        avcc: avcc(320, 180),
        width: 320,
        height: 180,
    };
    let mut cuts = Vec::new();
    let mut parts = Vec::new();
    let mut take = |m: &mut HlsMuxer, cuts: &mut Vec<Cut>| {
        parts.extend(std::iter::from_fn(|| m.take_part()));
        while let Some(segment) = m.take_segment() {
            cuts.push(Cut {
                parts: std::mem::take(&mut parts),
                segment,
            });
        }
    };
    for u in &units {
        if u.track.0 == VIDEO && u.dts == 75 * FRAME {
            m.gap();
            take(&mut m, &mut cuts);
        }
        if u.track.0 == VIDEO && u.dts == 150 * FRAME {
            m.set_config(&smaller).unwrap();
        }
        if (75 * FRAME..80 * FRAME).contains(&u.dts) && u.track.0 == VIDEO {
            continue;
        }
        m.push(u).unwrap();
        take(&mut m, &mut cuts);
    }
    m.finish().unwrap();
    take(&mut m, &mut cuts);
    check(&cuts);
    let summary: Vec<(i64, bool)> = cuts
        .iter()
        .map(|c| (millis(c.segment.duration), c.segment.discontinuity))
        .collect();
    assert_eq!(
        summary,
        [(2000, false), (500, false), (2000, true), (2000, false)]
    );
    assert_eq!(durations(&cuts[0]), [1000, 1000]);
    assert_eq!(durations(&cuts[1]), [500]);
    assert_eq!(durations(&cuts[3]), [1000, 1000]);
    // The video waits a second for the new init; the audio goes on in
    // parts within the target.
    assert!(cuts[2].parts.iter().all(|p| p.duration <= ms(1000)));
    assert!(!Rc::ptr_eq(&cuts[2].parts[0].init, &cuts[3].parts[0].init));
    assert_eq!(m.metrics().overlong_parts, 0);
}

#[test]
fn b_frames_make_parts_of_their_segments() {
    let units: Vec<AccessUnit<'static>> = av(300, &[0, 60, 120, 180, 240], true)
        .into_iter()
        .map(|mut u| {
            if u.track.0 == VIDEO {
                u.pts = u.dts + 2 * FRAME;
            }
            u
        })
        .collect();
    let mut m = muxer(ms(1000), true);
    let cuts = run(&mut m, &units);
    check(&cuts);
    for c in &cuts[1..] {
        let s = &c.segment;
        assert_eq!(
            media_duration(&parse_init(&s.init).unwrap(), &s.media).unwrap(),
            s.duration
        );
    }
}

#[test]
fn silence_fills_a_hole_across_parts() {
    // The audio from 0.9 s to 1.1 s is missing.
    let units: Vec<AccessUnit<'static>> = av(180, &[0, 60, 120], true)
        .into_iter()
        .filter(|u| u.track.0 != AUDIO || !(43 * AAC_FRAME..52 * AAC_FRAME).contains(&u.dts))
        .collect();
    let mut m = muxer(ms(1000), true);
    let cuts = run(&mut m, &units);
    check(&cuts);
    assert!(contiguous(&samples(&cuts, AUDIO)));
    assert_eq!(m.metrics().silence_units, 9);
}

#[test]
fn part_targets_are_bounded() {
    let keyframes: Vec<i64> = (0..300).step_by(150).collect();
    for (target, most) in [(ms(1), MIN_PART_TARGET), (ms(10_000), ms(4000))] {
        let mut m = muxer(target, true);
        let cuts = run(&mut m, &av(300, &keyframes, true));
        check(&cuts);
        let longest = cuts
            .iter()
            .flat_map(|c| &c.parts)
            .map(|p| p.duration)
            .max()
            .unwrap();
        assert!(longest <= most, "{target:?}: {longest:?}");
        assert!(longest + ms(34) > most, "{target:?}: {longest:?}");
    }
}

#[test]
fn finished_parts_left_untaken_are_bounded() {
    let mut m = muxer(ms(100), true);
    for u in &av(150, &[0, 60, 120], true) {
        m.push(u).unwrap();
    }
    let left: Vec<HlsPart> = std::iter::from_fn(|| m.take_part()).collect();
    assert_eq!(left.len(), MAX_PENDING_PARTS);
    let met = m.metrics();
    assert_eq!(met.dropped_parts, met.parts - MAX_PENDING_PARTS as u64);
}

/// A unit of a random stream: its track, its step in frames after its
/// track's unit before, keyframe and size; or a gap.
#[derive(Debug, Clone)]
enum Op {
    Unit {
        video: bool,
        step: i64,
        keyframe: bool,
        len: usize,
    },
    Gap,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        40 => (
            any::<bool>(),
            prop_oneof![8 => Just(1i64), 2 => 0i64..4, 1 => 0i64..400],
            prop::bool::weighted(0.05),
            0usize..4096,
        )
            .prop_map(|(video, step, keyframe, len)| Op::Unit { video, step, keyframe, len }),
        1 => Just(Op::Gap),
    ]
}

fn proptest_config() -> Config {
    Config {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x9a27_5e11),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Whatever the stream, a segment's parts are its media and add up to
    /// its duration, are independent where they start at a keyframe, and
    /// fit the target unless counted overlong.
    #[test]
    fn arbitrary_streams_make_valid_parts(
        ops in proptest::collection::vec(op(), 0..400),
        target in 100i64..2000,
    ) {
        let mut m = HlsMuxer::new(HlsMuxerConfig {
            max_segment_bytes: 64 * 1024,
            part_target: ms(target),
            ..HlsMuxerConfig::default()
        });
        m.set_config(&video(VIDEO)).unwrap();
        m.set_config(&audio(AUDIO)).unwrap();
        let (mut vt, mut at) = (0i64, 0i64);
        let mut units = Vec::new();
        let mut gaps = Vec::new();
        for op in ops {
            match op {
                Op::Gap => gaps.push(units.len()),
                Op::Unit { video: true, step, keyframe, len } => {
                    vt += step * FRAME;
                    units.push(unit(VIDEO, vt, keyframe, len));
                }
                Op::Unit { video: false, step, len, .. } => {
                    at += step * AAC_FRAME;
                    units.push(unit(AUDIO, at, true, len.min(600)));
                }
            }
        }
        let mut cuts = Vec::new();
        let mut parts = Vec::new();
        let mut take = |m: &mut HlsMuxer, cuts: &mut Vec<Cut>| {
            parts.extend(std::iter::from_fn(|| m.take_part()));
            while let Some(segment) = m.take_segment() {
                cuts.push(Cut { parts: std::mem::take(&mut parts), segment });
            }
        };
        for (i, u) in units.iter().enumerate() {
            if gaps.contains(&i) {
                m.gap();
                take(&mut m, &mut cuts);
            }
            m.push(u).unwrap();
            take(&mut m, &mut cuts);
        }
        m.finish().unwrap();
        take(&mut m, &mut cuts);
        prop_assert!(parts.is_empty());
        check(&cuts);
        let met = m.metrics();
        let all: Vec<&HlsPart> = cuts.iter().flat_map(|c| &c.parts).collect();
        prop_assert_eq!(all.len() as u64, met.parts);
        let overlong = all.iter().filter(|p| p.duration > ms(target)).count();
        prop_assert_eq!(overlong as u64, met.overlong_parts);
        prop_assert_eq!(met.dropped_parts, 0);
    }
}

#[test]
fn a_part_carries_the_cues_that_show_in_it_cut_to_it() {
    const TEXT: u32 = 3;
    let mut m = muxer(ms(1000), false);
    m.set_config(&crate::text::config(TrackId(TEXT))).unwrap();
    let cue = |duration: u64, text: &str| {
        let mut out = Vec::new();
        crate::text::write_cue(
            &crate::text::Cue {
                continues: false,
                duration,
                text,
            },
            &mut out,
        );
        out
    };
    let cues = [(500, cue(1000, "one")), (1500, cue(1500, "across"))];
    let mut parts = Vec::new();
    let mut next = 0;
    for f in 0..90 {
        let at = f * 1000 / 30;
        while let Some((start, payload)) = cues.get(next).filter(|c| c.0 <= at) {
            m.push(&AccessUnit {
                track: TrackId(TEXT),
                dts: *start,
                pts: *start,
                keyframe: true,
                payload,
                published: Time::from_unix_millis(*start),
            })
            .unwrap();
            next += 1;
        }
        m.push(&unit(VIDEO, f * FRAME, f % 60 == 0, 100)).unwrap();
        parts.extend(std::iter::from_fn(|| m.take_part()));
    }
    m.finish().unwrap();
    parts.extend(std::iter::from_fn(|| m.take_part()));
    let vtt = |p: &HlsPart| String::from_utf8(p.vtt.as_deref().unwrap().to_vec()).unwrap();
    let head = "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n";
    assert_eq!(
        vtt(&parts[0]),
        format!("{head}00:00:00.500 --> 00:00:01.000\none\n\n")
    );
    assert_eq!(
        vtt(&parts[1]),
        format!(
            "{head}00:00:01.000 --> 00:00:01.500\none\n\n00:00:01.500 --> 00:00:02.000\nacross\n\n"
        )
    );
    assert_eq!(
        vtt(&parts[2]),
        format!("{head}00:00:02.000 --> 00:00:03.000\nacross\n\n")
    );
    // A stream without a text track has none.
    let mut plain = muxer(ms(1000), false);
    let cuts = run(&mut plain, &av(90, &[0, 60], false));
    assert!(cuts.iter().flat_map(|c| &c.parts).all(|p| p.vtt.is_none()));
}
