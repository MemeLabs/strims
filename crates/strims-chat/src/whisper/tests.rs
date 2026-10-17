// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Tests panic on failure by design.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_proto::chat::v1::Message;

use super::*;

const SENDER: &[u8] = &[7; 32];

fn t0() -> Time {
    Time::from_unix(1_700_000_000, 0)
}

fn secs(n: i64) -> Duration {
    Duration::from_secs(n)
}

fn check(
    ignore: Option<&UiConfigIgnore>,
    body: &str,
    unread: usize,
    thread_exists: bool,
    threads: usize,
) -> Result<(), WhisperRefusal> {
    check_receive(
        t0(),
        SENDER,
        body,
        ignore,
        unread,
        thread_exists,
        threads,
        &WhisperLimits::default(),
    )
}

fn ignore(deadline: i64) -> UiConfigIgnore {
    UiConfigIgnore {
        id: 1,
        version: None,
        alias: "sender".into(),
        peer_key: SENDER.to_vec(),
        deadline,
    }
}

#[test]
fn ignored_senders_are_refused_until_the_deadline() {
    assert_eq!(
        check(Some(&ignore(0)), "hi", 0, true, 1),
        Err(WhisperRefusal::Ignored)
    );
    let now = t0().unix();
    assert_eq!(
        check(Some(&ignore(now + 60)), "hi", 0, true, 1),
        Err(WhisperRefusal::Ignored)
    );
    assert_eq!(check(Some(&ignore(now)), "hi", 0, true, 1), Ok(()));
    assert_eq!(check(Some(&ignore(now - 1)), "hi", 0, true, 1), Ok(()));

    let other = UiConfigIgnore {
        peer_key: vec![8; 32],
        ..ignore(0)
    };
    assert_eq!(check(Some(&other), "hi", 0, true, 1), Ok(()));
    assert_eq!(check(None, "hi", 0, true, 1), Ok(()));
}

#[test]
fn unread_cap_counts_the_new_whisper() {
    assert_eq!(check(None, "hi", MAX_UNREAD - 1, true, 1), Ok(()));
    assert_eq!(
        check(None, "hi", MAX_UNREAD, true, 1),
        Err(WhisperRefusal::TooManyUnread)
    );
}

#[test]
fn new_threads_are_refused_at_the_cap() {
    assert_eq!(check(None, "hi", 0, false, MAX_WHISPER_THREADS - 1), Ok(()));
    assert_eq!(
        check(None, "hi", 0, false, MAX_WHISPER_THREADS),
        Err(WhisperRefusal::TooManyThreads)
    );
    assert_eq!(check(None, "hi", 0, true, MAX_WHISPER_THREADS), Ok(()));
}

#[test]
fn long_bodies_are_refused() {
    let body = "x".repeat(MAX_WHISPER_BODY_BYTES);
    assert_eq!(check(None, &body, 0, true, 1), Ok(()));
    assert_eq!(
        check(None, &(body + "x"), 0, true, 1),
        Err(WhisperRefusal::TooLong)
    );
}

#[test]
fn every_refusal_answers_with_the_wire_string() {
    for r in [
        WhisperRefusal::Ignored,
        WhisperRefusal::TooLong,
        WhisperRefusal::TooManyUnread,
        WhisperRefusal::TooManyThreads,
    ] {
        assert_eq!(r.to_string(), WHISPER_REFUSED);
    }
}

#[test]
fn threads_record_the_last_whisper() {
    let mut thread = WhisperThread {
        id: 5,
        peer_key: SENDER.to_vec(),
        alias: "old".into(),
        ..Default::default()
    };
    let now = t0() + Duration::from_millis(1234);
    thread_sent(&mut thread, "sender", 41, now);
    assert_eq!(thread.alias, "sender");
    assert_eq!(thread.last_message_time, 1_700_000_001_234);
    assert_eq!(thread.last_message_id, 41);
    assert!(!thread.has_unread);

    thread_received(&mut thread, "renamed", 42, now + secs(1));
    assert_eq!(thread.alias, "renamed");
    assert_eq!(thread.last_message_time, 1_700_000_002_234);
    assert_eq!(thread.last_message_id, 42);
    assert!(thread.has_unread);
}

fn record(id: u64) -> WhisperRecord {
    WhisperRecord {
        id,
        network_key: vec![1; 32],
        server_key: vec![2; 32],
        peer_key: vec![3; 32],
        state: MessageState::Enqueued as i32,
        message: Some(Message {
            body: format!("whisper {id}"),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn deliveries() -> Deliveries {
    Deliveries::new(WhisperLimits::default())
}

#[test]
fn failed_attempts_retry_on_schedule_until_failed() {
    let mut d = deliveries();
    assert!(d.enqueue(t0(), &record(1)));
    assert_eq!(d.poll_timeout(), Some(t0()));

    let mut now = t0();
    let mut due = Vec::new();
    for attempt in 1..=6 {
        assert_eq!(d.poll_send(now - Duration::MILLISECOND), None);
        let s = d.poll_send(now).unwrap();
        assert_eq!(
            s,
            Delivery {
                id: 1,
                network_key: vec![1; 32],
                server_key: vec![2; 32],
                peer_key: vec![3; 32],
                body: "whisper 1".into(),
                attempt,
            }
        );
        assert_eq!(d.poll_timeout(), None);
        let state = d.finished(now, 1, false);
        if attempt < 6 {
            assert_eq!(state, None);
            now = d.poll_timeout().unwrap();
            due.push(now - t0());
        } else {
            assert_eq!(state, Some(MessageState::Failed));
        }
    }
    assert_eq!(
        due,
        vec![secs(10), secs(40), secs(100), secs(220), secs(520)]
    );
    assert!(d.is_empty());
    assert_eq!(d.poll_timeout(), None);
}

#[test]
fn success_delivers() {
    let mut d = deliveries();
    d.enqueue(t0(), &record(1));
    d.poll_send(t0()).unwrap();
    d.finished(t0(), 1, false);
    let now = d.poll_timeout().unwrap();
    assert_eq!(d.poll_send(now).unwrap().attempt, 2);
    assert_eq!(d.finished(now, 1, true), Some(MessageState::Delivered));
    assert!(d.is_empty());
    assert_eq!(d.in_flight(), 0);
    assert_eq!(d.finished(now, 1, true), None);
}

#[test]
fn records_fail_after_ten_minutes() {
    let mut d = deliveries();
    d.enqueue(t0(), &record(1));
    d.poll_send(t0()).unwrap();
    let late = t0() + secs(595);
    assert_eq!(d.finished(late, 1, false), None);
    let deadline = t0() + secs(600);
    assert_eq!(d.poll_timeout(), Some(deadline));
    assert_eq!(d.poll_send(deadline).unwrap().attempt, 2);
    assert_eq!(d.finished(deadline, 1, false), Some(MessageState::Failed));
    assert!(d.is_empty());
}

#[test]
fn attempts_in_flight_are_capped() {
    let mut d = deliveries();
    for id in (1..=10).rev() {
        assert!(d.enqueue(t0(), &record(id)));
    }
    let sent: Vec<u64> = std::iter::from_fn(|| d.poll_send(t0()).map(|s| s.id)).collect();
    assert_eq!(sent, (1..=8).collect::<Vec<_>>());
    assert_eq!(d.in_flight(), MAX_IN_FLIGHT);
    assert_eq!(d.poll_timeout(), None);

    assert_eq!(d.finished(t0(), 3, true), Some(MessageState::Delivered));
    assert_eq!(d.poll_timeout(), Some(t0()));
    assert_eq!(d.poll_send(t0()).unwrap().id, 9);
    assert_eq!(d.poll_send(t0()), None);
}

#[test]
fn the_queue_is_bounded() {
    let mut d = deliveries();
    for id in 0..MAX_PENDING_DELIVERIES as u64 {
        assert!(d.enqueue(t0(), &record(id)));
    }
    assert!(!d.enqueue(t0(), &record(1000)));
    assert!(d.enqueue(t0(), &record(5)));
    assert_eq!(d.len(), MAX_PENDING_DELIVERIES);

    let mut d = deliveries();
    let delivered = WhisperRecord {
        state: MessageState::Delivered as i32,
        ..record(1)
    };
    assert!(!d.enqueue(t0(), &delivered));
    let bodiless = WhisperRecord {
        message: None,
        ..record(2)
    };
    assert!(!d.enqueue(t0(), &bodiless));
    assert!(d.is_empty());
}

#[test]
fn requeued_records_keep_their_schedule() {
    let mut d = deliveries();
    d.enqueue(t0(), &record(1));
    d.poll_send(t0()).unwrap();
    d.finished(t0(), 1, false);
    assert!(d.enqueue(t0() + secs(1), &record(1)));
    assert_eq!(d.poll_timeout(), Some(t0() + secs(10)));
}

#[test]
fn cancel_forgets_records() {
    let mut d = deliveries();
    d.enqueue(t0(), &record(1));
    d.enqueue(t0(), &record(2));
    d.cancel(1);
    assert_eq!(d.poll_send(t0()).unwrap().id, 2);
    assert_eq!(d.poll_send(t0()), None);

    // A cancelled attempt keeps its slot until it finishes.
    d.cancel(2);
    assert_eq!(d.in_flight(), 1);
    assert_eq!(d.finished(t0(), 2, true), None);
    assert_eq!(d.in_flight(), 0);
    assert!(d.is_empty());

    // Enqueued again while in flight, it's reported as usual.
    d.enqueue(t0(), &record(3));
    d.poll_send(t0()).unwrap();
    d.cancel(3);
    assert!(d.enqueue(t0(), &record(3)));
    assert_eq!(d.finished(t0(), 3, true), Some(MessageState::Delivered));

    d.cancel(99);
    assert_eq!(d.finished(t0(), 99, true), None);
    assert_eq!(d.in_flight(), 0);
}

#[test]
fn refused_records_fail_without_a_retry() {
    let mut d = deliveries();
    d.enqueue(t0(), &record(1));
    d.enqueue(t0(), &record(2));
    assert_eq!(d.refused(1), None, "not in flight");
    assert_eq!(d.poll_send(t0()).unwrap().id, 1);
    assert_eq!(d.refused(1), Some(MessageState::Failed));
    assert_eq!(d.in_flight(), 0);
    assert_eq!(d.len(), 1);
    assert_eq!(d.refused(1), None);

    assert_eq!(d.poll_send(t0()).unwrap().id, 2);
    d.cancel(2);
    assert_eq!(d.refused(2), None, "cancelled");
    assert!(d.is_empty());
    assert_eq!(d.in_flight(), 0);
}

#[test]
fn the_lease_key_names_whispers() {
    assert_eq!(lease_key(), "rt:whisper");
}
