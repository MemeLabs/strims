// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The answers' pace; the push rule and the pace in action are the
//! runner's tests.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

fn ms(n: i64) -> Duration {
    Duration::from_millis(n)
}

/// Three links whose latest round trips stand `excess` ms over their
/// 40 ms bases, as of `at`.
fn three(at: Time, excess: i64) -> [PathReading; 3] {
    [link(at, excess); 3]
}

/// A link whose latest round trip stands `excess` ms over its 40 ms base,
/// as of `at`.
fn link(at: Time, excess: i64) -> PathReading {
    PathReading {
        samples: 10,
        at,
        rtt: ms(40 + excess),
        base: Some(ms(40)),
        recent_min: Some(ms(40 + excess)),
        in_flight: 0,
        rate: None,
        queued: None,
        written_since: 0,
    }
}

/// A pace whose updates cover a span by 1.1 s, answers flowing over an
/// empty path.
fn flowing() -> AnswerRate {
    let mut r = AnswerRate::new();
    for i in 1..=11 {
        r.spend(1024);
        r.update(t(i * 100), three(t(i * 100), 10));
    }
    r
}

#[test]
fn answers_start_unpaced() {
    let mut r = AnswerRate::new();
    r.spend(1 << 20);
    assert!(r.open(t(0)) && !r.exhausted());
}

#[test]
fn bursts_and_jitter_go_through() {
    let mut r = flowing();
    // A burst far past the bucket's depth goes at once.
    r.spend(1 << 20);
    assert!(r.open(t(1_100)) && !r.exhausted());
    // Delay past the target, deep even, that doesn't last: still unpaced.
    for i in 12..40 {
        let excess = if i % 3 == 0 { 10 } else { 300 };
        r.update(t(i * 100), three(t(i * 100), excess));
        r.spend(1 << 16);
        assert!(r.open(t(i * 100)), "{i}");
    }
}

#[test]
fn a_standing_queue_paces_at_what_answers_had_then_lets_go() {
    let mut r = flowing();
    // Twice the target for a second while answers ran at 400 KiB/s: paced
    // at half that.
    for i in 11..=21 {
        r.spend(40 * 1024);
        r.update(t(i * 100), three(t(i * 100), 100));
    }
    assert_eq!(r.rate(), 200 * 1024);
    assert!(r.open(t(2_100)));
    r.spend(1 << 20);
    assert!(!r.open(t(2_100)) && r.exhausted());
    // The queue drains; answers last waited at 2.2 s. Paced until a span
    // after that, then free.
    r.note_limited();
    r.spend(1024);
    r.update(t(2_200), three(t(2_200), 10));
    for i in 23..=31 {
        r.spend(1024);
        r.update(t(i * 100), three(t(i * 100), 10));
    }
    assert!(!r.open(t(3_100)));
    r.spend(1024);
    r.update(t(3_200), three(t(3_200), 10));
    assert!(r.open(t(3_200)) && !r.exhausted());
}

#[test]
fn a_deep_queue_paces_within_a_quarter_span() {
    let mut r = flowing();
    for i in 12..=14 {
        r.spend(40 * 1024);
        r.update(t(i * 100), three(t(i * 100), 300));
        assert!(r.open(t(i * 100)), "{i}");
    }
    r.spend(40 * 1024);
    r.update(t(1_500), three(t(1_500), 300));
    r.spend(1 << 20);
    assert!(!r.open(t(1_500)));
}

#[test]
fn a_gap_in_the_readings_shows_no_standing_queue() {
    let mut r = flowing();
    for i in 11..=21 {
        let at = if i == 16 { t(-5_000) } else { t(i * 100) };
        r.update(t(i * 100), three(at, 100));
    }
    r.spend(1 << 20);
    assert!(r.open(t(2_100)));
}

#[test]
fn one_peer_delaying_its_echoes_doesnt_start_the_pace() {
    let mut r = flowing();
    for i in 11..=21 {
        r.spend(40 * 1024);
        let at = t(i * 100);
        r.update(at, [link(at, 0), link(at, 0), link(at, 0), link(at, 2_000)]);
    }
    r.spend(1 << 20);
    assert!(r.open(t(2_100)));
}

#[test]
fn a_drained_queue_doubles_the_rate_back() {
    let mut r = flowing();
    for i in 11..=21 {
        r.spend(40 * 1024);
        r.update(t(i * 100), three(t(i * 100), 100));
    }
    let paced = r.rate();
    // Drained to a fifth of the target, answers waiting: doubles per probe
    // interval, not a tenth.
    for i in 22..=25 {
        r.note_limited();
        r.spend(1024);
        r.update(t(i * 100), three(t(i * 100), 10));
    }
    assert!(r.rate() >= 2 * paced, "{} {}", r.rate(), paced);
}

#[test]
fn answers_are_cut_by_their_share_of_the_queue() {
    let mut r = flowing();
    // Twice the target for a second while answers were two fifths of what
    // went out, at 400 KiB/s: a fifth off, not half.
    for i in 11..=21 {
        r.spend(40 * 1024);
        r.wrote(100 * 1024);
        r.update(t(i * 100), three(t(i * 100), 100));
    }
    assert_eq!(r.rate(), 400 * 1024 * 80 / 100);
}

#[test]
fn a_queue_of_the_nodes_own_pushes_doesnt_start_the_pace() {
    let mut r = flowing();
    for i in 11..=21 {
        r.spend(4 * 1024);
        r.wrote(400 * 1024);
        r.update(t(i * 100), three(t(i * 100), 300));
    }
    r.spend(1 << 20);
    assert!(r.open(t(2_100)));
}

#[test]
fn the_pace_never_falls_below_its_floor() {
    let mut r = flowing();
    for i in 11..=21 {
        r.update(t(i * 100), three(t(i * 100), 300));
    }
    for i in 22..60 {
        r.note_limited();
        r.update(t(i * 100), three(t(i * 100), 300));
    }
    assert_eq!(r.rate(), ANSWER_RATE_FLOOR);
}

#[test]
fn a_paced_rate_grows_back_while_the_queue_is_the_nodes_pushes() {
    let mut r = flowing();
    for i in 11..=21 {
        r.spend(40 * 1024);
        r.update(t(i * 100), three(t(i * 100), 100));
    }
    let paced = r.rate();
    // The queue stands on, but answers are now a twentieth of what goes
    // out: the pace doubles while they wait.
    for i in 22..=25 {
        r.note_limited();
        r.spend(10 * 1024);
        r.wrote(200 * 1024);
        r.update(t(i * 100), three(t(i * 100), 100));
    }
    assert!(r.rate() >= 2 * paced, "{} {}", r.rate(), paced);
}
