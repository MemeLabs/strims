// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::time::Time;

use super::*;

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
