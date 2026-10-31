// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::time::Time;

use super::*;
use crate::captions::{CcTuple, NalFormat};

const TRACK: TrackId = TrackId(3);

fn ms(n: i64) -> Duration {
    Duration::from_millis(n)
}

fn payload(continues: bool, duration: u64, text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    write_cue(
        &Cue {
            continues,
            duration,
            text,
        },
        &mut out,
    );
    out
}

fn unit(pts: i64, payload: &[u8]) -> AccessUnit<'_> {
    AccessUnit {
        track: TRACK,
        dts: pts,
        pts,
        keyframe: true,
        payload,
        published: Time::from_unix_millis(pts),
    }
}

fn reader() -> CueReader {
    let mut r = CueReader::new();
    r.set_configs(&[config(TRACK)]);
    r
}

fn events(r: &mut CueReader) -> Vec<CueEvent> {
    std::iter::from_fn(|| r.poll_event()).collect()
}

fn show(id: u64, start: i64, end: i64, text: &str) -> CueEvent {
    CueEvent::Show {
        id,
        kind: CueKind::Subtitles,
        start: ms(start),
        end: ms(end),
        text: text.into(),
    }
}

#[test]
fn a_cue_reads_back_as_written() {
    let p = payload(true, 1234, "two\nlines");
    assert_eq!(p[..3], [3, 1, 0xd2]);
    assert_eq!(
        parse_cue(&p).unwrap(),
        Cue {
            continues: true,
            duration: 1234,
            text: "two\nlines",
        }
    );
    assert_eq!(parse_cue(&payload(false, 0, "")).unwrap().text, "");
}

#[test]
fn a_cue_skips_head_fields_it_does_not_know_and_flags_it_does_not() {
    // head_len 4: flags with an unknown bit, duration 5, an unknown field.
    let p = [4, 0xfe, 5, 0x81, 0x01, b'h', b'i'];
    assert_eq!(
        parse_cue(&p).unwrap(),
        Cue {
            continues: false,
            duration: 5,
            text: "hi",
        }
    );
}

#[test]
fn malformed_cues_are_refused() {
    assert_eq!(parse_cue(&[]), Err(CueError::Truncated));
    assert_eq!(parse_cue(&[0]), Err(CueError::Truncated));
    assert_eq!(parse_cue(&[2, 0]), Err(CueError::Truncated));
    assert_eq!(parse_cue(&[2, 0, 0x80]), Err(CueError::Truncated));
    assert_eq!(parse_cue(&[2, 0, 1, 0xff]), Err(CueError::NotUtf8));
    let long = "x".repeat(MAX_CUE_TEXT + 1);
    assert_eq!(parse_cue(&payload(false, 1, &long)), Err(CueError::TooLong));
    let most = "x".repeat(MAX_CUE_TEXT);
    assert!(parse_cue(&payload(false, 1, &most)).is_ok());
}

proptest! {
    #![proptest_config(Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0x7e47),
        failure_persistence: None,
        ..Config::default()
    })]

    #[test]
    fn any_bytes_parse_without_panic(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _ = parse_cue(&bytes);
        let mut r = reader();
        r.push(&unit(0, &bytes));
        let _ = events(&mut r);
    }

    #[test]
    fn written_cues_read_back(continues: bool, duration: u64, text in "\\PC{0,64}") {
        let p = payload(continues, duration, &text);
        prop_assert_eq!(parse_cue(&p).unwrap(), Cue { continues, duration, text: &text });
    }
}

#[test]
fn estimates_count_characters_at_the_reading_rate_within_their_bounds() {
    assert_eq!(estimate(""), MIN_CUE);
    assert_eq!(estimate("Hi"), MIN_CUE);
    // 30 characters, a space among them: 2 s.
    assert_eq!(estimate("The quick brown fox jumps over"), ms(2000));
    // Runs of whitespace and line breaks count once.
    assert_eq!(
        estimate("The quick brown\n  fox jumps over"),
        estimate("The quick brown fox jumps over")
    );
    // 15 Han characters read as 30.
    assert_eq!(estimate(&"字".repeat(15)), ms(2000));
    assert_eq!(estimate(&"한".repeat(15)), ms(2000));
    assert_eq!(estimate(&"x".repeat(500)), MAX_ESTIMATE);
}

#[test]
fn a_one_word_cue_shows_its_floor_beside_one_starting_soon_after() {
    let mut r = reader();
    r.push(&unit(0, &payload(false, 800, "Hi")));
    r.push(&unit(300, &payload(false, 2000, "there, friend")));
    assert_eq!(
        events(&mut r),
        [show(0, 0, 800, "Hi"), show(1, 300, 2300, "there, friend")]
    );
}

#[test]
fn the_next_cue_ends_a_cue_sooner_but_not_before_its_floor() {
    let mut r = reader();
    r.push(&unit(0, &payload(false, 5000, "a long one")));
    r.push(&unit(2000, &payload(false, 3000, "the next")));
    r.push(&unit(2500, &payload(false, 3000, "and another")));
    assert_eq!(
        events(&mut r),
        [
            show(0, 0, 5000, "a long one"),
            CueEvent::End {
                id: 0,
                end: ms(2000)
            },
            show(1, 2000, 5000, "the next"),
            CueEvent::End {
                id: 1,
                end: ms(2800)
            },
            show(2, 2500, 5500, "and another"),
        ]
    );
}

#[test]
fn a_short_duration_shows_its_floor() {
    let mut r = reader();
    r.push(&unit(1000, &payload(false, 100, "blink")));
    assert_eq!(events(&mut r), [show(0, 1000, 1800, "blink")]);
}

#[test]
fn an_empty_cue_ends_the_tracks_cues_after_their_floor() {
    let mut r = reader();
    r.push(&unit(0, &payload(false, 4000, "until cleared")));
    r.push(&unit(500, &payload(false, 0, "")));
    r.push(&unit(1000, &payload(false, 0, "")));
    assert_eq!(
        events(&mut r),
        [
            show(0, 0, 4000, "until cleared"),
            CueEvent::End {
                id: 0,
                end: ms(800)
            },
        ]
    );
}

#[test]
fn a_continued_cue_of_the_cue_showing_is_that_cue() {
    let mut r = reader();
    r.push(&unit(0, &payload(false, 3000, "same")));
    r.push(&unit(1000, &payload(true, 2000, "same")));
    assert_eq!(events(&mut r), [show(0, 0, 3000, "same")]);
    assert_eq!(r.metrics().cues, 1);
}

#[test]
fn a_reader_that_joins_shows_what_is_left_of_a_continued_cue() {
    let mut r = reader();
    r.push(&unit(1000, &payload(true, 300, "joined")));
    assert_eq!(events(&mut r), [show(0, 1000, 1300, "joined")]);
    // Once it ended, the same text again is a cue of its own.
    r.push(&unit(1500, &payload(true, 300, "joined")));
    assert_eq!(events(&mut r), [show(1, 1500, 1800, "joined")]);
}

#[test]
fn cues_of_other_tracks_and_malformed_ones_show_nothing() {
    let mut r = reader();
    let p = payload(false, 1000, "elsewhere");
    r.push(&AccessUnit {
        track: TrackId(1),
        ..unit(0, &p)
    });
    r.push(&unit(0, &[9, 9]));
    assert_eq!(events(&mut r), []);
    assert_eq!(r.metrics().malformed, 1);
    r.set_configs(&[]);
    r.push(&unit(0, &p));
    assert_eq!(events(&mut r), []);
}

#[test]
fn a_reset_forgets_the_cues_showing() {
    let mut r = reader();
    r.push(&unit(0, &payload(false, 3000, "before")));
    r.reset();
    r.push(&unit(500, &payload(false, 3000, "after")));
    assert_eq!(
        events(&mut r),
        [show(0, 0, 3000, "before"), show(1, 500, 3500, "after")]
    );
}

#[test]
fn times_follow_the_tracks_timescale() {
    let mut r = CueReader::new();
    r.set_configs(&[DecoderConfig {
        timescale: 90_000,
        ..config(TRACK)
    }]);
    r.push(&unit(90_000, &payload(false, 180_000, "two seconds")));
    assert_eq!(events(&mut r), [show(0, 1000, 3000, "two seconds")]);
}

#[test]
fn the_cues_showing_are_bounded() {
    let mut r = reader();
    for i in 0..(MAX_SHOWING as i64 + 4) {
        r.push(&unit(i, &payload(false, 5000, "stacked")));
        assert!(r.showing.len() <= MAX_SHOWING);
    }
    assert_eq!(r.metrics().cues, MAX_SHOWING as u64 + 4);
}

// --- captions ---------------------------------------------------------------

const VIDEO: TrackId = TrackId(1);

fn video_config() -> DecoderConfig {
    crate::units::test_support::video(1)
}

/// A video unit's payload carrying `pairs` of field 1.
fn frame(pairs: &[[u8; 2]]) -> Vec<u8> {
    let idr = [0, 0, 0, 4, 0x65, 0x88, 0x84, 0x00];
    if pairs.is_empty() {
        return idr.to_vec();
    }
    let tuples: Vec<CcTuple> = pairs
        .iter()
        .map(|p| crate::synth::captions::field1(*p))
        .collect();
    let mut out = Vec::new();
    NalFormat::avc(4)
        .with_cc_sei(&idr, &tuples, &mut out)
        .unwrap();
    out
}

/// A video unit at 30 fps decoding at frame `dts` and presenting at
/// frame `pts`.
fn video_unit(dts: i64, pts: i64, payload: &[u8]) -> AccessUnit<'_> {
    AccessUnit {
        track: VIDEO,
        dts: dts * 3000,
        pts: pts * 3000,
        keyframe: dts == 0,
        payload,
        published: Time::from_unix_millis(0),
    }
}

/// Frame `n` at 30 fps.
fn frame_at(n: i64) -> Duration {
    Duration::from_nanos(n * 1_000_000_000 / 30)
}

fn caption(id: u64, n: i64, text: &str) -> CueEvent {
    CueEvent::Show {
        id,
        kind: CueKind::Captions,
        start: frame_at(n),
        end: frame_at(n) + CAPTION_TIMEOUT,
        text: text.into(),
    }
}

/// Feeds `pairs` one a frame, in presentation order, the frames in
/// decode order of an IBBP pattern, I0 P3 B1 B2 P6 B4 B5 and on, each
/// decoding a frame before the one before it presents.
fn feed_reordered(r: &mut CueReader, pairs: &[[u8; 2]]) {
    let n = pairs.len() as i64 + 3;
    for d in 0..n {
        let p = match d {
            0 => 0,
            d => {
                let (group, at) = ((d - 1) / 3, (d - 1) % 3);
                3 * group + if at == 0 { 3 } else { at }
            }
        };
        let pair: Vec<[u8; 2]> = pairs.get(p as usize).copied().into_iter().collect();
        let payload = frame(&pair);
        r.push(&video_unit(d - 1, p, &payload));
    }
}

#[test]
fn captions_show_from_the_frame_they_appear_in_across_b_frames() {
    let mut r = CueReader::new();
    r.set_configs(&[video_config()]);
    let mut pairs = crate::synth::captions::pop_on("ONE");
    let one = pairs.len() as i64 - 2;
    pairs.extend(crate::synth::captions::pop_on("TWO"));
    let two = pairs.len() as i64 - 2;
    pairs.push(crate::synth::captions::pair(0x14, 0x2c));
    let gone = pairs.len() as i64 - 1;
    feed_reordered(&mut r, &pairs);
    assert_eq!(
        events(&mut r),
        [
            caption(0, one, "ONE"),
            CueEvent::End {
                id: 0,
                end: frame_at(two)
            },
            caption(1, two, "TWO"),
            CueEvent::End {
                id: 1,
                end: frame_at(gone)
            },
        ]
    );
    assert_eq!(r.metrics().captions, 2);
}

/// Feeds `frames` of pairs one a frame from frame `from`, decoding as
/// they present.
fn feed_in_order(r: &mut CueReader, from: i64, frames: &[&[[u8; 2]]]) {
    for (k, pairs) in frames.iter().enumerate() {
        let n = from + k as i64;
        let payload = frame(pairs);
        r.push(&video_unit(n, n, &payload));
    }
}

#[test]
fn a_caption_without_a_change_shows_for_the_timeout() {
    let mut r = CueReader::new();
    r.set_configs(&[video_config()]);
    let pairs = crate::synth::captions::pop_on("STILL");
    let frames: Vec<&[[u8; 2]]> = pairs.iter().map(std::slice::from_ref).collect();
    feed_in_order(&mut r, 0, &frames);
    let shown = pairs.len() as i64 - 2;
    assert_eq!(events(&mut r), [caption(0, shown, "STILL")]);
    // Past the timeout, the caption swapped off and on again is a cue of
    // its own.
    let after = shown + 16 * 30 + 1;
    let eoc = [crate::synth::captions::pair(0x14, 0x2f)];
    feed_in_order(&mut r, after, &[&eoc, &[], &eoc, &[]]);
    assert_eq!(events(&mut r), [caption(1, after + 2, "STILL")]);
}

#[test]
fn a_reset_or_another_video_track_starts_captions_afresh() {
    let mut r = CueReader::new();
    r.set_configs(&[video_config()]);
    let pairs = crate::synth::captions::pop_on("HALF");
    let frames: Vec<&[[u8; 2]]> = pairs.iter().take(8).map(std::slice::from_ref).collect();
    feed_in_order(&mut r, 0, &frames);
    r.reset();
    let eoc = [crate::synth::captions::pair(0x14, 0x2f)];
    feed_in_order(&mut r, 10, &[&eoc, &[]]);
    assert_eq!(events(&mut r), []);
    // Units of a track that isn't the stream's video carry no captions.
    r.set_configs(&[]);
    feed_in_order(&mut r, 20, &[&eoc, &[], &eoc, &[]]);
    assert_eq!(events(&mut r), []);
}
