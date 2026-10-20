// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A dump is untrusted input: random, mutated and truncated dumps go
//! through the reader, which never panics, yields at most one record per
//! input byte, and resumes after a malformed record. Debug builds check
//! arithmetic overflow, so an overflow is a failure too.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::time::{Duration, Time};
use strims_ppspp_capture::{
    Clock, Dir, Header, MAX_FRAME, Reader, Record, SwarmParams, TapRecord, Writer,
};

/// 1024 cases, or more with `PROPTEST_CASES`.
fn config() -> Config {
    Config {
        cases: Config::default().cases.max(1024),
        rng_seed: RngSeed::Fixed(0x0ca9_70e5),
        failure_persistence: None,
        ..Config::default()
    }
}

fn header() -> Header<'static> {
    Header {
        clock: Clock::Node,
        origin: Time::from_unix_millis(1_700_000_000_000),
        file: 0,
        node: &[0xab; 32],
        label: "node",
    }
}

/// A dump with every kind of record, built once.
fn corpus() -> &'static [u8] {
    static CORPUS: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    CORPUS.get_or_init(|| {
        let mut w = Writer::new(Vec::new(), &header()).unwrap();
        let mut t = header().origin;
        let frame: Vec<u8> = (0..200u32).map(|i| (i * 7) as u8).collect();
        let params = SwarmParams {
            chunk_size: 1024,
            integrity_hash_size: 32,
            integrity_signature_size: 64,
            stream_count: 4,
            chunks_per_signature: 64,
            live_window: 1024,
            ..SwarmParams::default()
        };
        for i in 1..4u32 {
            t += Duration::from_micros(1500);
            let recs = [
                Record::Peer {
                    peer: i,
                    name: &[i as u8; 32],
                },
                Record::Swarm {
                    swarm: i,
                    id: &[1; 32],
                    params,
                },
                Record::ChannelOpen {
                    peer: i,
                    swarm: i,
                    channel: u64::from(i),
                    peer_channel: 1000,
                },
                Record::Frame {
                    dir: Dir::In,
                    peer: i,
                    len: frame.len() as u64,
                    body: &frame,
                },
                Record::Tap(TapRecord::new(16, i, i, &[1 << 40, -1, 0, 7])),
                Record::Frame {
                    dir: Dir::Out,
                    peer: i,
                    len: 9,
                    body: &frame[..9],
                },
                Record::FrameError {
                    peer: i,
                    message: b"error",
                },
                Record::TapLost { count: 3 },
                Record::ChannelClose { peer: i, swarm: i },
                Record::SwarmGone { swarm: i },
                Record::PeerGone { peer: i },
            ];
            for r in &recs {
                w.write(t, r).unwrap();
            }
        }
        w.into_inner()
    })
}

/// Reads `input` to its end; returns the records and errors seen.
fn run(input: &[u8]) -> (usize, usize) {
    let Ok(mut r) = Reader::new(input) else {
        return (0, 0);
    };
    let (mut ok, mut bad) = (0, 0);
    for _ in 0..=input.len() {
        match r.next_record() {
            None => break,
            Some(Ok(e)) => {
                ok += 1;
                if let Record::Frame { len, body, .. } = e.record {
                    assert!(body.len() <= MAX_FRAME);
                    assert!(body.len() as u64 <= len);
                }
                let _ = e.time + Duration::SECOND;
            }
            Some(Err(e)) => {
                bad += 1;
                if !e.is_recoverable() {
                    assert!(r.next_record().is_none());
                    break;
                }
            }
        }
    }
    assert!(r.next_record().is_none(), "the reader made no progress");
    assert!(ok + bad <= input.len());
    (ok, bad)
}

#[test]
fn corpus_is_valid() {
    let (ok, bad) = run(corpus());
    assert_eq!((ok, bad), (33, 0));
}

#[derive(Clone, Debug)]
enum Mutation {
    Flip(usize, u8),
    Truncate(usize),
    Insert(usize, Vec<u8>),
    Overwrite(usize, Vec<u8>),
}

fn mutation() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        4 => (any::<usize>(), 1..=255u8).prop_map(|(p, x)| Mutation::Flip(p, x)),
        1 => any::<usize>().prop_map(Mutation::Truncate),
        2 => (any::<usize>(), prop::collection::vec(any::<u8>(), 1..12)).prop_map(|(p, b)| Mutation::Insert(p, b)),
        2 => (any::<usize>(), prop::collection::vec(
            prop_oneof![Just(0xffu8), Just(0x80), Just(0x00), Just(0x7f), any::<u8>()], 1..12,
        )).prop_map(|(p, b)| Mutation::Overwrite(p, b)),
    ]
}

fn apply(input: &mut Vec<u8>, m: &Mutation) {
    if input.is_empty() {
        return;
    }
    let len = input.len();
    match m {
        Mutation::Flip(p, x) => input[p % len] ^= x,
        Mutation::Truncate(p) => input.truncate(p % len),
        Mutation::Insert(p, b) => {
            let p = p % len;
            input.splice(p..p, b.iter().copied());
        }
        Mutation::Overwrite(p, b) => {
            let p = p % len;
            for (i, x) in b.iter().enumerate() {
                if let Some(d) = input.get_mut(p + i) {
                    *d = *x;
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn random_bytes_never_panic(input in prop::collection::vec(any::<u8>(), 0..600)) {
        run(&input);
    }

    #[test]
    fn random_records_never_panic(tail in prop::collection::vec(any::<u8>(), 0..600)) {
        // a valid header, then garbage
        let mut input = Writer::new(Vec::new(), &header()).unwrap().into_inner();
        input.extend_from_slice(&tail);
        run(&input);
    }

    #[test]
    fn mutated_dumps_never_panic(ms in prop::collection::vec(mutation(), 1..6)) {
        let mut input = corpus().to_vec();
        for m in &ms {
            apply(&mut input, m);
        }
        run(&input);
    }
}
