// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::synth::captions::{field1, pair, pop_on};

/// Frame `n` at 30 fps.
fn at(n: i64) -> Duration {
    Duration::from_nanos(n * 1_000_000_000 / 30)
}

/// Feeds `pairs` a frame each from frame `from`: each change and its
/// frame.
fn feed(d: &mut Cc1Decoder, from: i64, pairs: &[[u8; 2]]) -> Vec<(i64, String)> {
    let mut out = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        let n = from + i as i64;
        if let Some(t) = d.frame(at(n), &[field1(*p)]) {
            out.push((n, t));
        }
    }
    out
}

/// The pairs of `text`'s characters, a space padding an odd length.
fn chars(text: &str) -> Vec<[u8; 2]> {
    let mut bytes = text.as_bytes().to_vec();
    if bytes.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes.chunks(2).map(|c| pair(c[0], c[1])).collect()
}

/// A control code, twice.
fn twice(a: u8, b: u8) -> Vec<[u8; 2]> {
    vec![pair(a, b), pair(a, b)]
}

#[test]
fn a_pop_on_caption_shows_at_its_end_of_caption() {
    let mut d = Cc1Decoder::new();
    let pairs = pop_on("HELLO WORLD");
    let n = pairs.len() as i64;
    assert_eq!(feed(&mut d, 0, &pairs), [(n - 2, "HELLO WORLD".into())]);
    // The next replaces it at its own; erasing the screen empties it.
    let mut next = pop_on("NEXT");
    next.extend(twice(0x14, 0x2c));
    let m = next.len() as i64;
    assert_eq!(
        feed(&mut d, n, &next),
        [(n + m - 4, "NEXT".into()), (n + m - 2, String::new())]
    );
}

#[test]
fn rows_show_top_to_bottom_at_their_indents() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x20);
    // Row 14 (0x14 0x50 = row 14, indent 0), then row 15 indented 8.
    pairs.extend(twice(0x14, 0x50));
    pairs.extend(chars("TOP"));
    pairs.extend(twice(0x14, 0x74));
    pairs.extend(chars("BOTTOM"));
    pairs.extend(twice(0x14, 0x2f));
    let got = feed(&mut d, 0, &pairs);
    assert_eq!(got.last().unwrap().1, "TOP\nBOTTOM");
    assert_eq!(d.screen[14][8..14].iter().collect::<String>(), "BOTTOM");
}

#[test]
fn roll_up_shows_at_carriage_returns_and_after_typing_pauses() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x25);
    pairs.extend(twice(0x14, 0x70));
    pairs.extend(chars("ONE"));
    pairs.extend(twice(0x14, 0x2d));
    pairs.extend(chars("TWO"));
    pairs.extend(twice(0x14, 0x2d));
    pairs.extend(chars("THREE"));
    let got = feed(&mut d, 0, &pairs);
    // Entering roll-up clears the screen; each CR shows the rows rolled
    // up, the base row empty, two rows of them for RU2.
    assert_eq!(
        got,
        [(0, String::new()), (6, "ONE".into()), (10, "TWO".into())]
    );
    // Typing then a pause: what was typed shows half a second on.
    let n = pairs.len() as i64;
    let mut later = Vec::new();
    for k in 0..20 {
        if let Some(t) = d.frame(at(n + k), &[]) {
            later.push((n + k, t));
        }
    }
    assert_eq!(later, [(12 + 15, "TWO\nTHREE".into())]);
}

#[test]
fn a_roll_up_window_keeps_its_rows() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x25);
    for line in ["A", "B", "C"] {
        pairs.extend(chars(line));
        pairs.extend(twice(0x14, 0x2d));
    }
    // Rows past the window's top go.
    assert_eq!(feed(&mut d, 0, &pairs).last().unwrap().1, "C");
    // Three rows from here.
    let mut more = twice(0x14, 0x26);
    more.extend(chars("D"));
    more.extend(twice(0x14, 0x2d));
    more.extend(chars("E"));
    more.extend(twice(0x14, 0x2d));
    assert_eq!(feed(&mut d, 100, &more).last().unwrap().1, "D\nE");
    let mut last = chars("F");
    last.extend(twice(0x14, 0x2d));
    assert_eq!(feed(&mut d, 200, &last).last().unwrap().1, "E\nF");
}

#[test]
fn paint_on_types_on_the_screen() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x29);
    pairs.extend(twice(0x14, 0x70));
    pairs.extend(chars("PAINT"));
    let mut got = feed(&mut d, 0, &pairs);
    for k in 0..20 {
        if let Some(t) = d.frame(at(7 + k), &[]) {
            got.push((7 + k, t));
        }
    }
    assert_eq!(got, [(4 + 15, "PAINT".into())]);
}

#[test]
fn special_and_extended_characters_and_the_standard_ones_608_changes() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x20);
    pairs.extend(twice(0x14, 0x70));
    // é and ñ among the standard ones.
    pairs.push(pair(0x5c, 0x7e));
    // ♪, sent twice.
    pairs.extend(twice(0x11, 0x37));
    // "e" then the extended ë in its place.
    pairs.extend(chars("e"));
    pairs.extend(twice(0x12, 0x36));
    // A mid-row code is a space; a tab moves two on.
    pairs.extend(twice(0x11, 0x20));
    pairs.extend(chars("x"));
    pairs.extend(twice(0x17, 0x22));
    pairs.extend(chars("y"));
    pairs.extend(twice(0x14, 0x2f));
    assert_eq!(feed(&mut d, 0, &pairs).last().unwrap().1, "éñ♪ë x  y");
}

#[test]
fn backspace_and_delete_to_end_of_row_edit_the_row() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x20);
    pairs.extend(twice(0x14, 0x70));
    pairs.extend(chars("ABCDEF"));
    pairs.extend(twice(0x14, 0x21));
    // Back to column 2 with a preamble's indent 0 and two tabs.
    pairs.extend(twice(0x14, 0x70));
    pairs.extend(twice(0x17, 0x22));
    pairs.extend(twice(0x14, 0x24));
    pairs.extend(twice(0x14, 0x2f));
    assert_eq!(feed(&mut d, 0, &pairs).last().unwrap().1, "AB");
}

#[test]
fn channel_2_and_text_mode_are_dropped() {
    let mut d = Cc1Decoder::new();
    // CC2's resume caption loading and text.
    let mut pairs = twice(0x1c, 0x20);
    pairs.extend(twice(0x1c, 0x70));
    pairs.extend(chars("CC2"));
    pairs.extend(twice(0x1c, 0x2f));
    // Text mode on CC1.
    pairs.extend(twice(0x14, 0x2a));
    pairs.extend(chars("TEXT"));
    assert_eq!(feed(&mut d, 0, &pairs), []);
    let pop = pop_on("CC1");
    assert_eq!(feed(&mut d, 100, &pop).last().unwrap().1, "CC1");
}

#[test]
fn a_control_code_repeated_after_a_frame_without_pairs_is_taken_again() {
    let mut d = Cc1Decoder::new();
    let show = pop_on("ONE");
    feed(&mut d, 0, &show);
    // An EOC, a frame without pairs, and another: two swaps.
    assert_eq!(
        d.frame(at(50), &[field1(pair(0x14, 0x2f))]),
        Some(String::new())
    );
    assert_eq!(d.frame(at(51), &[]), None);
    assert_eq!(
        d.frame(at(52), &[field1(pair(0x14, 0x2f))]),
        Some("ONE".into())
    );
    // Within one frame, the repeat is dropped.
    let eoc = field1(pair(0x14, 0x2f));
    assert_eq!(d.frame(at(53), &[eoc, eoc]), Some(String::new()));
}

#[test]
fn pairs_of_bad_parity_and_field_2_are_dropped() {
    let mut d = Cc1Decoder::new();
    let mut pairs = twice(0x14, 0x20);
    pairs.extend(twice(0x14, 0x70));
    pairs.extend(chars("OK"));
    // 'A' without its parity bit fixed: 0x41 has even parity.
    pairs.push([0x41, 0x80]);
    pairs.extend(twice(0x14, 0x2f));
    let mut got = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        let field2 = [0xfd, p[0], p[1]];
        if let Some(t) = d.frame(at(i as i64), &[field1(*p), field2]) {
            got.push(t);
        }
    }
    assert_eq!(got, ["OK"]);
}

fn proptest_config() -> Config {
    Config {
        cases: 512,
        rng_seed: RngSeed::Fixed(0x6080),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn any_pairs_decode_without_panic_within_the_screen(
        frames in proptest::collection::vec(
            proptest::collection::vec(any::<[u8; 3]>(), 0..4),
            0..400,
        ),
    ) {
        let mut d = Cc1Decoder::new();
        for (i, tuples) in frames.iter().enumerate() {
            if let Some(t) = d.frame(at(i as i64), tuples) {
                prop_assert!(t.lines().count() <= ROWS);
                prop_assert!(t.lines().all(|l| l.chars().count() <= COLUMNS));
            }
        }
    }
}
