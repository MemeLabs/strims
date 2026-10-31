// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The HLS muxer's cut rule, fragments, gaps, config changes and bounds.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
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

/// A stream of `frames` video frames at 30 a second, keyframes at
/// `keyframes` (frame numbers), with AAC frames over the same time when
/// `with_audio`, in decode order. Each B-frame-free unit shows at its
/// decode time.
fn av(frames: i64, keyframes: &[i64], with_audio: bool) -> Vec<AccessUnit<'static>> {
    let video: Vec<AccessUnit<'static>> = (0..frames)
        .map(|f| {
            let key = keyframes.contains(&f);
            unit(VIDEO, f * FRAME, key, if key { 4000 } else { 1500 })
        })
        .collect();
    if !with_audio {
        return video;
    }
    // 48 kHz samples up to the last frame's end.
    let end = frames * FRAME * 48_000 / 90_000;
    let audio: Vec<AccessUnit<'static>> = (0..end / AAC_FRAME)
        .map(|i| unit(AUDIO, i * AAC_FRAME, true, 300))
        .collect();
    merge(video, audio)
}

/// Video and audio units in decode-time order, video first at a tie.
fn merge(
    video: Vec<AccessUnit<'static>>,
    audio: Vec<AccessUnit<'static>>,
) -> Vec<AccessUnit<'static>> {
    let mut out = Vec::with_capacity(video.len() + audio.len());
    let (mut v, mut a) = (video.into_iter().peekable(), audio.into_iter().peekable());
    loop {
        let take_video = match (v.peek(), a.peek()) {
            (Some(x), Some(y)) => x.dts * 48_000 <= y.dts * 90_000,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return out,
        };
        out.extend(if take_video { v.next() } else { a.next() });
    }
}

fn muxer(config: HlsMuxerConfig, with_audio: bool) -> HlsMuxer {
    let mut m = HlsMuxer::new(config);
    m.set_config(&video(VIDEO)).unwrap();
    if with_audio {
        m.set_config(&audio(AUDIO)).unwrap();
    }
    m
}

/// Pushes `units` and finishes: the segments, in order.
fn run(m: &mut HlsMuxer, units: &[AccessUnit<'_>]) -> Vec<HlsSegment> {
    let mut out = Vec::new();
    for u in units {
        m.push(u).unwrap();
        out.extend(std::iter::from_fn(|| m.take_segment()));
    }
    m.finish().unwrap();
    out.extend(std::iter::from_fn(|| m.take_segment()));
    out
}

fn mux(units: &[AccessUnit<'_>]) -> (Vec<HlsSegment>, HlsMuxerMetrics) {
    let mut m = muxer(HlsMuxerConfig::default(), true);
    let segments = run(&mut m, units);
    (segments, m.metrics())
}

/// The media time of a segment's samples, as a player reads it.
fn media_time_of(s: &HlsSegment) -> Duration {
    media_duration(&parse_init(&s.init).unwrap(), &s.media).unwrap()
}

/// The top-level box types of `media`.
fn boxes(media: &[u8]) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < media.len() {
        let size = u32::from_be_bytes(media[at..at + 4].try_into().unwrap()) as usize;
        out.push(media[at + 4..at + 8].try_into().unwrap());
        at += size;
    }
    out
}

fn millis(d: Duration) -> i64 {
    (d.as_nanos() + 500_000) / 1_000_000
}

#[test]
fn one_second_gops_are_one_second_segments_from_their_keyframes() {
    let keyframes: Vec<i64> = (0..10).map(|g| g * 30).collect();
    let (segments, m) = mux(&av(300, &keyframes, true));
    assert_eq!(segments.len(), 10);
    for (i, s) in segments.iter().enumerate() {
        assert!(s.keyframe, "segment {i}");
        assert!(!s.discontinuity, "segment {i}");
        assert_eq!(millis(s.duration), 1000, "segment {i}");
        assert_eq!(media_time_of(s), s.duration, "segment {i}");
        assert!(Rc::ptr_eq(&s.init, &segments[0].init));
    }
    assert_eq!(
        (m.segments, m.mid_gop_segments, m.overlong_segments),
        (10, 0, 0)
    );
}

#[test]
fn a_gop_that_rounds_to_the_target_stays_whole() {
    // 4.4 s, 2 s and 4.467 s GOPs: each rounds to at most 4 s.
    let keyframes = [0, 132, 192, 326];
    let (segments, m) = mux(&av(356, &keyframes, true));
    let lengths: Vec<i64> = segments.iter().map(|s| millis(s.duration)).collect();
    assert_eq!(&lengths[..3], [4400, 2000, 4467]);
    assert!(segments.iter().all(|s| s.keyframe));
    assert_eq!(m.mid_gop_segments, 0);
}

#[test]
fn a_gop_longer_than_the_target_is_split_into_segments_within_it() {
    // Scene-cut keyframes up to 10 s apart.
    let keyframes = [0, 21, 96, 396, 420, 495, 630];
    let (segments, m) = mux(&av(750, &keyframes, true));
    let mut start = Duration::ZERO;
    let mut starts = Vec::new();
    for s in &segments {
        assert!(fits_target(s.duration, 4), "{:?}", s.duration);
        assert_eq!(millis(media_time_of(s)), millis(s.duration));
        // The frame it starts at.
        starts.push((
            (start.as_nanos() * 30 + 500_000_000) / 1_000_000_000,
            s.keyframe,
        ));
        start += s.duration;
    }
    // The 10 s GOP from frame 96 is cut as late as its frames allow, into
    // 134-frame segments (4.467 s) and the rest; the 4.5 s one from frame
    // 495 once, a frame before its end, so the keyframe after it comes
    // within the floor and joins that last frame's segment.
    let want = [
        (0, true),
        (21, true),
        (96, true),
        (96 + 134, false),
        (96 + 268, false),
        (396, true),
        (420, true),
        (495, true),
        (495 + 134, false),
    ];
    assert_eq!(starts, want);
    assert_eq!(millis(segments[8].duration), (121 * 1000 + 15) / 30);
    assert_eq!((m.mid_gop_segments, m.overlong_segments), (3, 0));
}

#[test]
fn a_keyframe_within_the_floor_joins_the_segment_in_a_fragment_of_its_own() {
    // Keyframes 0.233 s apart, then 0.967 s later.
    let (segments, _) = mux(&av(120, &[0, 7, 36], true));
    assert_eq!(millis(segments[0].duration), 1200);
    assert!(segments[1].keyframe);
    // Fragments of three frames, but for the one cut short by the second
    // keyframe, which starts the next.
    let moofs = boxes(&segments[0].media)
        .iter()
        .filter(|b| *b == b"moof")
        .count();
    assert_eq!(moofs, 13);
}

#[test]
fn fragments_hold_the_fragment_duration_each_after_a_prft() {
    let (segments, _) = mux(&av(60, &[0, 30], true));
    let types = boxes(&segments[0].media);
    assert_eq!(types.len(), 30);
    for (i, b) in types.iter().enumerate() {
        let want: &[u8; 4] = [b"prft", b"moof", b"mdat"][i % 3];
        assert_eq!(b, want, "box {i}");
    }
}

#[test]
fn b_frames_decoding_from_zero_keep_long_first_gops_within_the_target() {
    // Each frame shows two frames after it decodes, from decode time 0:
    // samples before the edit list don't hold the GOP in one fragment.
    let units: Vec<AccessUnit<'static>> = av(450, &[0, 300], true)
        .into_iter()
        .map(|mut u| {
            if u.track.0 == VIDEO {
                u.pts = u.dts + 2 * FRAME;
            }
            u
        })
        .collect();
    let (segments, m) = mux(&units);
    assert!(segments.len() >= 4);
    assert!(segments.iter().all(|s| fits_target(s.duration, 4)));
    assert_eq!(m.overlong_segments, 0);
    let fragments = boxes(&segments[0].media)
        .iter()
        .filter(|b| *b == b"moof")
        .count();
    assert!(fragments > 30, "{fragments}");
}

#[test]
fn a_gap_ends_the_segment_and_drops_units_to_the_next_keyframe() {
    let units = av(150, &[0, 30, 60, 90, 120], true);
    let mut m = muxer(HlsMuxerConfig::default(), true);
    let mut segments = Vec::new();
    // Before any segment is finished, a gap marks nothing.
    m.gap();
    for u in &units {
        if u.track.0 == VIDEO && u.dts == 45 * FRAME {
            m.gap();
            segments.extend(std::iter::from_fn(|| m.take_segment()));
        }
        // The video frames between are lost.
        if (45 * FRAME..50 * FRAME).contains(&u.dts) && u.track.0 == VIDEO {
            continue;
        }
        m.push(u).unwrap();
        segments.extend(std::iter::from_fn(|| m.take_segment()));
    }
    m.finish().unwrap();
    segments.extend(std::iter::from_fn(|| m.take_segment()));
    let summary: Vec<(i64, bool, bool)> = segments
        .iter()
        .map(|s| (millis(s.duration), s.keyframe, s.discontinuity))
        .collect();
    assert_eq!(
        summary,
        [
            (1000, true, false),
            (500, true, false),
            (1000, true, true),
            (1000, true, false),
            (1000, true, false),
        ]
    );
    let met = m.metrics();
    // Ten video frames, and the audio over them.
    assert!(met.dropped_awaiting_keyframe > 10, "{met:?}");
}

#[test]
fn changed_configs_start_a_new_init_at_the_next_keyframe() {
    let units = av(120, &[0, 30, 60, 90], true);
    let mut m = muxer(HlsMuxerConfig::default(), true);
    let mut segments = Vec::new();
    let mut smaller = video(VIDEO);
    smaller.params = CodecParams::Avc {
        avcc: avcc(320, 180),
        width: 320,
        height: 180,
    };
    for u in &units {
        if u.track.0 == VIDEO && u.dts == 40 * FRAME {
            m.set_config(&smaller).unwrap();
        }
        m.push(u).unwrap();
        segments.extend(std::iter::from_fn(|| m.take_segment()));
    }
    m.finish().unwrap();
    segments.extend(std::iter::from_fn(|| m.take_segment()));
    assert_eq!(segments.len(), 4);
    assert!(Rc::ptr_eq(&segments[0].init, &segments[1].init));
    assert_ne!(segments[1].init, segments[2].init);
    assert!(Rc::ptr_eq(&segments[2].init, &segments[3].init));
    // The changed track's units wait for the new init; the audio goes on.
    assert_eq!(m.metrics().dropped_pending_config, 20);
    assert!(segments.iter().all(|s| s.keyframe));
}

#[test]
fn a_reset_forgets_the_tracks_the_new_configs_leave_out() {
    let mut m = muxer(HlsMuxerConfig::default(), true);
    let first = run(&mut m, &av(60, &[0, 30], true));
    assert_eq!(parse_init(&first[0].init).unwrap().tracks().len(), 2);
    m.reset();
    m.set_config(&video(VIDEO)).unwrap();
    let later: Vec<AccessUnit<'static>> = av(60, &[0, 30], false);
    let next = run(&mut m, &later);
    assert!(next[0].discontinuity);
    assert_eq!(parse_init(&next[0].init).unwrap().tracks().len(), 1);
    assert_eq!(
        m.push(&unit(AUDIO, 0, true, 10)),
        Err(HlsMuxerError::UnknownTrack(AUDIO))
    );
}

#[test]
fn audio_ends_a_segment_while_the_video_stalls() {
    // No video for 6 s from frame 60, the audio going on.
    let units: Vec<AccessUnit<'static>> = av(330, &[0, 300], true)
        .into_iter()
        .filter(|u| u.track.0 != VIDEO || !(60 * FRAME..240 * FRAME).contains(&u.dts))
        .collect();
    let (segments, m) = mux(&units);
    for s in &segments {
        assert!(fits_target(s.duration, 4), "{:?}", s.duration);
    }
    assert!(m.mid_gop_segments >= 1, "{m:?}");
    assert_eq!(m.overlong_segments, 0);
}

#[test]
fn a_video_only_stall_past_the_target_is_overlong() {
    let units: Vec<AccessUnit<'static>> = av(330, &[0, 300], false)
        .into_iter()
        .filter(|u| !(60 * FRAME..240 * FRAME).contains(&u.dts))
        .collect();
    let mut m = muxer(HlsMuxerConfig::default(), false);
    let segments = run(&mut m, &units);
    assert_eq!(m.metrics().overlong_segments, 1);
    assert!(segments.iter().any(|s| !fits_target(s.duration, 4)));
}

#[test]
fn a_full_segment_ends_early_and_an_oversized_unit_waits_for_a_keyframe() {
    let config = HlsMuxerConfig {
        max_segment_bytes: 20_000,
        ..HlsMuxerConfig::default()
    };
    let mut m = muxer(config, false);
    let segments = run(&mut m, &av(60, &[0, 30], false));
    assert!(segments.len() > 2);
    assert!(segments.iter().all(|s| s.media.len() <= 20_000));
    assert!(segments[1..].iter().any(|s| !s.keyframe));

    let config = HlsMuxerConfig {
        max_segment_bytes: 3000,
        ..HlsMuxerConfig::default()
    };
    let mut m = muxer(config, false);
    let segments = run(&mut m, &av(60, &[0, 30], false));
    // Keyframes don't fit; the frames after them wait for the next.
    assert!(segments.is_empty());
    assert_eq!(m.metrics().dropped_oversized, 2);
}

#[test]
fn a_stream_without_video_makes_no_segments() {
    let mut m = HlsMuxer::new(HlsMuxerConfig::default());
    m.set_config(&audio(AUDIO)).unwrap();
    let units: Vec<AccessUnit<'static>> = (0..100)
        .map(|i| unit(AUDIO, i * AAC_FRAME, true, 300))
        .collect();
    assert!(run(&mut m, &units).is_empty());
    assert_eq!(m.metrics().dropped_awaiting_keyframe, 100);
}

#[test]
fn refused_configs_change_nothing() {
    let mut m = muxer(HlsMuxerConfig::default(), false);
    let mut bad = audio(AUDIO);
    bad.timescale = 0;
    assert!(matches!(m.set_config(&bad), Err(HlsMuxerError::Config(_))));
    let mut huge = video(3);
    huge.params = CodecParams::Avc {
        avcc: vec![0; 70_000],
        width: 16,
        height: 16,
    };
    assert!(m.set_config(&huge).is_err());
    let segments = run(&mut m, &av(60, &[0, 30], false));
    assert_eq!(parse_init(&segments[0].init).unwrap().tracks().len(), 1);
}

#[test]
fn finished_segments_left_untaken_are_bounded() {
    let mut m = muxer(HlsMuxerConfig::default(), true);
    let keyframes: Vec<i64> = (0..10).map(|g| g * 30).collect();
    for u in &av(300, &keyframes, true) {
        m.push(u).unwrap();
    }
    m.finish().unwrap();
    let left: Vec<HlsSegment> = std::iter::from_fn(|| m.take_segment()).collect();
    assert_eq!(left.len(), MAX_PENDING_SEGMENTS);
    assert_eq!(
        m.metrics().dropped_segments,
        10 - MAX_PENDING_SEGMENTS as u64
    );
    assert_eq!(millis(left[0].duration), 1000);
}

/// A unit of a random stream: track, decode-time step after the last
/// unit of its track, keyframe, payload size; or a gap.
/// An audio sample of a segment: its decode time, duration and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sample {
    dts: i64,
    duration: i64,
    size: usize,
}

/// The samples of `track` in `media`, from each `traf`'s `tfdt` and the
/// durations and sizes its `trun` lists.
fn samples_of(media: &[u8], track: u32) -> Vec<Sample> {
    fn children(data: &[u8]) -> Vec<([u8; 4], &[u8])> {
        let mut out = Vec::new();
        let mut at = 0;
        while at + 8 <= data.len() {
            let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
            out.push((
                data[at + 4..at + 8].try_into().unwrap(),
                &data[at + 8..at + size],
            ));
            at += size;
        }
        out
    }
    let be32 = |b: &[u8], at: usize| u32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    let mut out = Vec::new();
    for (kind, moof) in children(media) {
        if kind != *b"moof" {
            continue;
        }
        for (kind, traf) in children(moof) {
            if kind != *b"traf" {
                continue;
            }
            let boxes = children(traf);
            let tfhd = boxes.iter().find(|(k, _)| k == b"tfhd").unwrap().1;
            if be32(tfhd, 4) != track {
                continue;
            }
            let tfdt = boxes.iter().find(|(k, _)| k == b"tfdt").unwrap().1;
            let mut dts = if tfdt[0] == 1 {
                i64::from_be_bytes(tfdt[4..12].try_into().unwrap())
            } else {
                i64::from(be32(tfdt, 4))
            };
            for (_, trun) in boxes.iter().filter(|(k, _)| k == b"trun") {
                let flags = be32(trun, 0) & 0x00ff_ffff;
                assert!(flags & 0x300 == 0x300, "durations and sizes listed");
                let count = be32(trun, 4) as usize;
                let mut at = 8
                    + if flags & 0x01 != 0 { 4 } else { 0 }
                    + if flags & 0x04 != 0 { 4 } else { 0 };
                let entry = 4 * (flags & 0xf00).count_ones() as usize;
                for _ in 0..count {
                    let duration = i64::from(be32(trun, at));
                    let size = be32(trun, at + 4) as usize;
                    out.push(Sample {
                        dts,
                        duration,
                        size,
                    });
                    dts += duration;
                    at += entry;
                }
            }
        }
    }
    out
}

/// Whether each sample starts where the one before ends.
fn contiguous(samples: &[Sample]) -> bool {
    samples
        .windows(2)
        .all(|w| w[0].dts + w[0].duration == w[1].dts)
}

/// An AAC stream of `frames` video frames with keyframes every second,
/// its audio missing from `from` to `to` (48 kHz samples).
fn av_without_audio(frames: i64, from: i64, to: i64) -> Vec<AccessUnit<'static>> {
    let keyframes: Vec<i64> = (0..frames).step_by(30).collect();
    av(frames, &keyframes, true)
        .into_iter()
        .filter(|u| u.track.0 != AUDIO || !(from..to).contains(&u.dts))
        .collect()
}

/// A stereo AAC-LC frame of no spectrum: the silence of the default
/// audio config.
const SILENT_AAC: usize = 7;

#[test]
fn a_hole_in_an_audio_track_is_filled_with_silent_frames() {
    // Frames 20 to 29 of the first second's audio are missing.
    let units = av_without_audio(90, 20 * AAC_FRAME, 30 * AAC_FRAME);
    let (segments, m) = mux(&units);
    assert_eq!(segments.len(), 3);
    assert_eq!(
        (m.silence_fills, m.silence_units, m.audio_holes),
        (1, 10, 0)
    );
    let audio = samples_of(&segments[0].media, AUDIO);
    assert!(contiguous(&audio), "{audio:?}");
    let silent: Vec<i64> = audio
        .iter()
        .filter(|s| s.size == SILENT_AAC)
        .map(|s| s.dts / AAC_FRAME)
        .collect();
    assert_eq!(silent, (20..30).collect::<Vec<_>>());
    assert!(audio.iter().all(|s| s.duration == AAC_FRAME));
    for s in &segments {
        assert!(!s.discontinuity);
        assert_eq!(media_time_of(s), s.duration);
    }
}

#[test]
fn opus_holes_are_filled_with_silent_celt_frames() {
    let mut m = muxer(HlsMuxerConfig::default(), false);
    m.set_config(&opus_config()).unwrap();
    // 20 ms packets, 0.3 s of them missing from 0.5 s.
    static PACKET: [u8; 40] = {
        let mut p = [0u8; 40];
        p[0] = 0xfc;
        p
    };
    let video: Vec<AccessUnit<'static>> = av(60, &[0, 30], false);
    let audio: Vec<AccessUnit<'static>> = (0..100)
        .map(|i| i * 960)
        .filter(|t| !(24_000..38_400).contains(t))
        .map(|t| AccessUnit {
            payload: &PACKET,
            ..unit(AUDIO, t, true, 0)
        })
        .collect();
    let segments = run(&mut m, &merge(video, audio));
    let met = m.metrics();
    assert_eq!((met.silence_fills, met.silence_units), (1, 15), "{met:?}");
    let audio = samples_of(&segments[0].media, AUDIO);
    assert!(contiguous(&audio), "{audio:?}");
    let silent: Vec<&Sample> = audio.iter().filter(|s| s.size == 3).collect();
    assert_eq!(silent.len(), 15);
    assert!(silent.iter().all(|s| s.duration == 960));
    assert_eq!(silent[0].dts, 24_000);
}

#[test]
fn a_hole_across_a_cut_is_filled_at_the_start_of_the_next_segment() {
    // The audio from 0.9 s to 1.1 s is missing; a keyframe cuts at 1 s.
    let units = av_without_audio(90, 43 * AAC_FRAME, 52 * AAC_FRAME);
    let (segments, m) = mux(&units);
    assert_eq!((m.silence_fills, m.silence_units), (1, 9));
    let first = samples_of(&segments[0].media, AUDIO);
    let second = samples_of(&segments[1].media, AUDIO);
    // The first segment's audio ends at its last frame's own end.
    assert_eq!(first.last().unwrap().dts + AAC_FRAME, 43 * AAC_FRAME);
    assert_eq!(first.last().unwrap().duration, AAC_FRAME);
    // The second's starts with the silence from there, before its video.
    assert_eq!(second[0].dts, 43 * AAC_FRAME);
    assert!(second[..9].iter().all(|s| s.size == SILENT_AAC));
    assert!(contiguous(&second));
    let video = samples_of(&segments[1].media, VIDEO);
    assert_eq!(video[0].dts, 30 * FRAME);
}

#[test]
fn a_hole_past_max_silence_is_left_and_the_unit_before_lasts_its_frame() {
    // 3.5 s of audio missing from 1 s, inside a 6 s GOP.
    let from = 47 * AAC_FRAME;
    let to = from + 164 * AAC_FRAME;
    let units: Vec<AccessUnit<'static>> = av(240, &[0, 180], true)
        .into_iter()
        .filter(|u| u.track.0 != AUDIO || !(from..to).contains(&u.dts))
        .collect();
    let (segments, m) = mux(&units);
    assert_eq!((m.silence_fills, m.audio_holes), (0, 1), "{m:?}");
    let audio: Vec<Sample> = segments
        .iter()
        .flat_map(|s| samples_of(&s.media, AUDIO))
        .collect();
    let before = audio.iter().find(|s| s.dts == from - AAC_FRAME).unwrap();
    assert_eq!(before.duration, AAC_FRAME);
    assert!(audio.iter().any(|s| s.dts == to));
    assert!(audio.iter().all(|s| s.size != SILENT_AAC));
}

#[test]
fn a_hole_in_a_track_without_silent_frames_is_left() {
    let mut m = HlsMuxer::new(HlsMuxerConfig::default());
    m.set_config(&video(VIDEO)).unwrap();
    let mut he = audio(AUDIO);
    // Explicit SBR: a 24 kHz core under 48 kHz out.
    if let CodecParams::Aac { asc, .. } = &mut he.params {
        *asc = vec![0x2b, 0x11, 0x88, 0x00];
    }
    he.codec = "mp4a.40.5".into();
    m.set_config(&he).unwrap();
    let units = av_without_audio(60, 20 * AAC_FRAME, 30 * AAC_FRAME);
    let segments = run(&mut m, &units);
    let met = m.metrics();
    assert_eq!((met.silence_fills, met.audio_holes), (0, 1), "{met:?}");
    let audio = samples_of(&segments[0].media, AUDIO);
    let before = audio.iter().find(|s| s.dts == 19 * AAC_FRAME).unwrap();
    assert_eq!(before.duration, AAC_FRAME);
    assert!(audio.iter().any(|s| s.dts == 30 * AAC_FRAME));
}

#[test]
fn no_hole_is_filled_across_a_gap() {
    let units = av(120, &[0, 30, 60, 90], true);
    let mut m = muxer(HlsMuxerConfig::default(), true);
    let mut segments = Vec::new();
    let lost = 40 * FRAME..60 * FRAME;
    let lost_audio = 40 * FRAME * 48_000 / 90_000..70 * FRAME * 48_000 / 90_000;
    let mut gapped = false;
    for u in &units {
        let missing = if u.track.0 == VIDEO {
            lost.contains(&u.dts)
        } else {
            lost_audio.contains(&u.dts)
        };
        if missing {
            if !gapped {
                m.gap();
                gapped = true;
            }
            continue;
        }
        m.push(u).unwrap();
        segments.extend(std::iter::from_fn(|| m.take_segment()));
    }
    m.finish().unwrap();
    segments.extend(std::iter::from_fn(|| m.take_segment()));
    assert_eq!(m.metrics().silence_fills, 0);
    let after = segments.iter().position(|s| s.discontinuity).unwrap();
    let audio = samples_of(&segments[after].media, AUDIO);
    assert!(audio[0].dts >= lost_audio.end, "{audio:?}");
}

/// An Opus stereo config on track [`AUDIO`].
fn opus_config() -> DecoderConfig {
    DecoderConfig {
        track: TrackId(AUDIO),
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
        rng_seed: RngSeed::Fixed(0x415_3c75),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Whatever the stream, segments parse, fit the target unless counted
    /// overlong, stay within the bytes bound, and start at a keyframe
    /// after a discontinuity.
    #[test]
    fn arbitrary_streams_make_valid_segments(ops in proptest::collection::vec(op(), 0..400)) {
        let config = HlsMuxerConfig {
            max_segment_bytes: 64 * 1024,
            ..HlsMuxerConfig::default()
        };
        let mut m = muxer(config, true);
        let (mut vt, mut at) = (0i64, 0i64);
        let mut segments = Vec::new();
        for op in ops {
            match op {
                Op::Gap => m.gap(),
                Op::Unit { video: true, step, keyframe, len } => {
                    vt += step * FRAME;
                    m.push(&unit(VIDEO, vt, keyframe, len)).unwrap();
                }
                Op::Unit { video: false, step, len, .. } => {
                    at += step * AAC_FRAME;
                    m.push(&unit(AUDIO, at, true, len.min(600))).unwrap();
                }
            }
            segments.extend(std::iter::from_fn(|| m.take_segment()));
        }
        m.finish().unwrap();
        segments.extend(std::iter::from_fn(|| m.take_segment()));
        let met = m.metrics();
        let overlong = segments.iter().filter(|s| !fits_target(s.duration, 4)).count();
        prop_assert_eq!(overlong as u64, met.overlong_segments);
        prop_assert_eq!(segments.len() as u64, met.segments);
        for s in &segments {
            prop_assert!(s.media.len() <= 64 * 1024);
            prop_assert!(media_duration(&parse_init(&s.init).unwrap(), &s.media).is_ok());
            if s.discontinuity {
                prop_assert!(s.keyframe);
            }
        }
        let mid = segments.iter().filter(|s| !s.keyframe).count();
        prop_assert!(mid as u64 <= met.mid_gop_segments);
    }
}

// --- subtitles and captions ---------------------------------------------------

const TEXT: u32 = 3;

/// A cue unit of the text track at `ms`, lasting `duration` ms.
fn cue_payload(duration: u64, text: &str) -> Vec<u8> {
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
}

fn cue_unit(ms: i64, payload: &[u8]) -> AccessUnit<'_> {
    AccessUnit {
        track: TrackId(TEXT),
        dts: ms,
        pts: ms,
        keyframe: true,
        payload,
        published: Time::from_unix_millis(ms),
    }
}

/// Two-second GOPs of 30 fps video, frames from `from`, with `cues` (ms,
/// duration, text) each sent before the first frame at or after it.
fn with_cues(frames: i64, cues: &[(i64, u64, &str)]) -> Vec<HlsSegment> {
    let mut m = muxer(HlsMuxerConfig::default(), false);
    m.set_config(&crate::text::config(TrackId(TEXT))).unwrap();
    let payloads: Vec<Vec<u8>> = cues.iter().map(|c| cue_payload(c.1, c.2)).collect();
    let mut next = 0;
    let mut out = Vec::new();
    for f in 0..frames {
        let ms = f * 1000 / 30;
        while let Some(c) = cues.get(next).filter(|c| c.0 <= ms) {
            m.push(&cue_unit(c.0, &payloads[next])).unwrap();
            next += 1;
        }
        m.push(&unit(VIDEO, f * FRAME, f % 60 == 0, 100)).unwrap();
        out.extend(std::iter::from_fn(|| m.take_segment()));
    }
    m.finish().unwrap();
    out.extend(std::iter::from_fn(|| m.take_segment()));
    out
}

fn vtt(s: &HlsSegment) -> String {
    String::from_utf8(s.vtt.clone().unwrap()).unwrap()
}

#[test]
fn a_streams_cues_are_its_segments_webvtt_clipped_to_each_and_repeated_across() {
    let segments = with_cues(
        240,
        &[
            (500, 1000, "one"),
            (1500, 3000, "across & <b>"),
            (2500, 0, ""),
        ],
    );
    assert_eq!(segments.len(), 4);
    assert_eq!(
        vtt(&segments[0]),
        "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n\
         00:00:00.500 --> 00:00:01.500\none\n\n\
         00:00:01.500 --> 00:00:02.000\nacross &amp; &lt;b&gt;\n\n"
    );
    // The empty cue ends it at 2.5 s, in the second segment.
    assert_eq!(
        vtt(&segments[1]),
        "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n\
         00:00:02.000 --> 00:00:02.500\nacross &amp; &lt;b&gt;\n\n"
    );
    assert_eq!(
        vtt(&segments[2]),
        "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n"
    );
    for s in &segments {
        assert_eq!(&*s.codecs, "avc1.64001f");
        assert_eq!(s.resolution, Some((640, 360)));
        assert!(!s.captions);
    }
}

#[test]
fn close_cues_show_stacked_in_one_segment() {
    let segments = with_cues(120, &[(100, 100, "Hi"), (400, 2000, "there")]);
    assert_eq!(
        vtt(&segments[0]),
        "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n\
         00:00:00.100 --> 00:00:00.900\nHi\n\n\
         00:00:00.400 --> 00:00:02.000\nthere\n\n"
    );
}

#[test]
fn a_stream_without_a_text_track_has_no_subtitles() {
    let (segments, _) = mux(&av(120, &[0, 60], true));
    assert!(segments.iter().all(|s| s.vtt.is_none()));
    assert_eq!(&*segments[0].codecs, "avc1.64001f,mp4a.40.2");
}

#[test]
fn cue_times_follow_the_inits_composition_delay() {
    // B-frames: each frame presents two frames after it decodes.
    let mut m = muxer(HlsMuxerConfig::default(), false);
    m.set_config(&crate::text::config(TrackId(TEXT))).unwrap();
    let payload = cue_payload(1000, "late");
    let mut out = Vec::new();
    for f in 0..121 {
        if f == 30 {
            m.push(&cue_unit(1000, &payload)).unwrap();
        }
        let u = AccessUnit {
            pts: (f + 2) * FRAME,
            ..unit(VIDEO, f * FRAME, f % 60 == 0, 100)
        };
        m.push(&u).unwrap();
        out.extend(std::iter::from_fn(|| m.take_segment()));
    }
    // Presented at 1 s, a frame of 2/30 s presents 2/30 s early after the
    // edit: the cue too.
    assert_eq!(
        vtt(&out[0]),
        "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n\
         00:00:00.933 --> 00:00:01.933\nlate\n\n"
    );
}

#[test]
fn a_segment_whose_video_carries_608_says_so() {
    use crate::captions::NalFormat;
    use crate::synth::captions::{field1, pair};
    let mut m = muxer(HlsMuxerConfig::default(), false);
    let idr = [0, 0, 0, 4, 0x65, 0x88, 0x84, 0x00];
    let mut with_cc = Vec::new();
    NalFormat::avc(4)
        .with_cc_sei(&idr, &[field1(pair(0x14, 0x2c))], &mut with_cc)
        .unwrap();
    let mut out = Vec::new();
    for f in 0..181 {
        let payload: &[u8] = if (60..120).contains(&f) && f == 90 {
            &with_cc
        } else {
            &idr
        };
        let u = AccessUnit {
            payload,
            ..unit(VIDEO, f * FRAME, f % 60 == 0, 0)
        };
        m.push(&u).unwrap();
        out.extend(std::iter::from_fn(|| m.take_segment()));
    }
    assert_eq!(
        out.iter().map(|s| s.captions).collect::<Vec<_>>(),
        [false, true, false]
    );
}
