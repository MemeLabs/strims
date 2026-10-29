// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Property tests of the chunked layout (see `strims_chunkstream::chunked`).
//! A writer's records, frame ends and skipped frames decode whole from
//! offset 0; a decoder joining at any offset yields the records from the
//! next frame start on; lost chunks, as gaps or zeros, and a corrupt header
//! cost exactly the records whose bytes touch their frame; and arbitrary
//! input never panics, with the work linear in it. Debug builds check
//! arithmetic overflow, so an overflow is a failure too.

// Test code: a panic or out-of-range index is a test failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_chunkstream::chunked::{
    ChunkedDecoder, ChunkedEncoder, ChunkedMetrics, ChunkedRecordDecoder, HEADER_LEN, Item,
    MAX_FRAME_SIZE, NO_BOUNDARY, PADDING_FRAME, Piece,
};

fn config() -> Config {
    Config {
        cases: Config::default().cases.max(256),
        rng_seed: RngSeed::Fixed(0xc4a2_6b1d),
        failure_persistence: None,
        ..Config::default()
    }
}

#[derive(Debug, Clone, Copy)]
enum Op {
    /// A record of this many payload bytes.
    Record(usize),
    /// Ends the frame in progress.
    EndFrame,
    /// Ends the frame in progress and leaves this many frames out.
    Skip(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rec {
    offset: u64,
    /// The stream offset just past its last byte.
    end: u64,
    payload: Vec<u8>,
}

struct Written {
    /// The stream as a reader sees it, the frames left out as zeros.
    stream: Vec<u8>,
    records: Vec<Rec>,
}

fn pattern(n: usize, seed: usize) -> Vec<u8> {
    (0..n)
        .map(|i| ((i * 131 + seed * 17 + 7) % 256) as u8)
        .collect()
}

fn write(size: usize, ops: &[Op]) -> Written {
    let mut e = ChunkedEncoder::new(size).unwrap();
    let mut w = Written {
        stream: Vec::new(),
        records: Vec::new(),
    };
    let end_frame = |e: &mut ChunkedEncoder, out: &mut Vec<u8>| {
        let start = out.len();
        out.resize(start + e.frame_padding(), 0xee);
        assert_eq!(e.end_frame(&mut out[start..]), Ok(out.len() - start));
        assert_eq!(e.offset() % size as u64, 0);
    };
    for (i, &op) in ops.iter().enumerate() {
        match op {
            Op::Record(len) => {
                let payload = pattern(len, i);
                let offset = e.next_record_offset();
                let n = e.encoded_len(len).unwrap();
                let start = w.stream.len();
                w.stream.resize(start + n, 0xee);
                assert_eq!(e.encode(&[&payload], &mut w.stream[start..]), Ok(n));
                assert_eq!(e.offset(), w.stream.len() as u64);
                w.records.push(Rec {
                    offset,
                    end: e.offset(),
                    payload,
                });
            }
            Op::EndFrame => end_frame(&mut e, &mut w.stream),
            Op::Skip(frames) => {
                end_frame(&mut e, &mut w.stream);
                e.continue_at(e.offset() + (frames * size) as u64).unwrap();
                w.stream.resize(e.offset() as usize, 0);
            }
        }
    }
    w
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ev {
    Record(u64, Vec<u8>),
    Lost,
}

impl Ev {
    fn of(r: &Rec) -> Ev {
        Ev::Record(r.offset, r.payload.clone())
    }
}

/// Feeds each segment, a stream offset and the bytes from it, in pieces of
/// the given sizes, cycling through them, after a reset at its offset.
/// Returns what the piece decoder yields, checking each call, that the
/// record decoder agrees, and that the work is linear in the input.
fn decode(size: usize, max: usize, segments: &[(u64, &[u8])], pieces: &[usize]) -> Vec<Ev> {
    let mut d = ChunkedDecoder::new(size, max).unwrap();
    let mut evs = Vec::new();
    let mut current: Option<(u64, usize, Vec<u8>)>;
    let mut last_start = 0;
    let mut calls = 0;
    let mut piece = pieces.iter().cycle();
    for &(offset, input) in segments {
        d.reset(offset);
        current = None;
        let mut lost = false;
        let mut rest = input;
        while !rest.is_empty() {
            let take = (*piece.next().unwrap()).clamp(1, rest.len());
            let (mut chunk, after) = rest.split_at(take);
            rest = after;
            loop {
                let (n, p) = d.decode(chunk);
                assert!(n <= chunk.len());
                chunk = &chunk[n..];
                calls += 1;
                match p {
                    None => {
                        assert!(chunk.is_empty(), "input left without a piece");
                        break;
                    }
                    Some(Piece::Start { offset, len }) => {
                        assert!(current.is_none(), "a start inside a record");
                        assert!((1..=max).contains(&len));
                        assert!(offset >= last_start);
                        last_start = offset;
                        current = Some((offset, len, Vec::new()));
                    }
                    Some(Piece::Data(b)) => {
                        assert!(!b.is_empty());
                        let r = current.as_mut().expect("data outside a record");
                        r.2.extend_from_slice(b);
                        assert!(r.2.len() <= r.1);
                    }
                    Some(Piece::End) => {
                        let (offset, len, payload) =
                            current.take().expect("an end outside a record");
                        assert_eq!(payload.len(), len);
                        evs.push(Ev::Record(offset, payload));
                        lost = false;
                    }
                    Some(Piece::Lost) => {
                        assert!(!lost || current.is_some(), "losses in a row");
                        current = None;
                        evs.push(Ev::Lost);
                        lost = true;
                    }
                }
            }
        }
    }
    let input: usize = segments.iter().map(|(_, b)| b.len()).sum();
    // Each piece but `End` and `Lost` consumes input; an `End` follows a
    // record of at least 2 bytes, and a `Lost` follows a record or input.
    assert!(calls <= 3 * input + 2 * segments.len() + 1, "{calls} calls");

    let mut r = ChunkedRecordDecoder::new(size, max).unwrap();
    let mut items = Vec::new();
    let mut piece = pieces.iter().cycle();
    for &(offset, input) in segments {
        r.reset(offset);
        let mut rest = input;
        while !rest.is_empty() {
            let take = (*piece.next().unwrap()).clamp(1, rest.len());
            let (mut chunk, after) = rest.split_at(take);
            rest = after;
            loop {
                let (n, item) = r.decode(chunk);
                chunk = &chunk[n..];
                match item {
                    None => break,
                    Some(Item::Record { offset, payload }) => {
                        items.push(Ev::Record(offset, payload.to_vec()))
                    }
                    Some(Item::Lost) => items.push(Ev::Lost),
                }
            }
        }
    }
    assert_eq!(evs, items, "the record decoder disagrees");
    assert_eq!(d.metrics(), r.metrics());
    evs
}

fn records(evs: Vec<Ev>) -> Vec<Ev> {
    evs.into_iter().filter(|e| *e != Ev::Lost).collect()
}

fn ops(max_len: usize) -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(
        prop_oneof![
            8 => (1..=max_len).prop_map(Op::Record),
            1 => Just(Op::EndFrame),
            1 => (0..3usize).prop_map(Op::Skip),
        ],
        0..16,
    )
}

fn size() -> impl Strategy<Value = usize> {
    prop_oneof![3 => 3..=40usize, 1 => Just(4096)]
}

fn pieces() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(prop_oneof![1..20usize, 1..5000usize], 1..6)
}

/// Whether a record's bytes touch frame `f`.
fn touches(r: &Rec, size: usize, f: usize) -> bool {
    let (start, end) = ((f * size) as u64, ((f + 1) * size) as u64);
    r.offset < end && r.end > start
}

fn header(stream: &[u8], size: usize, f: usize) -> u16 {
    u16::from_be_bytes([stream[f * size], stream[f * size + 1]])
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn records_decode_whole_from_offset_zero(
        size in size(),
        ops in ops(200),
        pieces in pieces(),
    ) {
        let max_len = if size == 4096 { 20_000 } else { 4 * size };
        let ops: Vec<Op> = ops
            .into_iter()
            .map(|op| match op {
                Op::Record(n) => Op::Record(n * max_len / 200 + 1),
                op => op,
            })
            .collect();
        let w = write(size, &ops);
        for f in 0..w.stream.len().div_ceil(size) {
            let h = header(&w.stream, size, f);
            prop_assert!(h <= NO_BOUNDARY || (HEADER_LEN..size).contains(&usize::from(h)));
        }
        let want: Vec<Ev> = w.records.iter().map(Ev::of).collect();
        let got = decode(size, max_len + 1, &[(0, &w.stream)], &pieces);
        prop_assert_eq!(&got, &want);

        let mut d = ChunkedRecordDecoder::new(size, max_len + 1).unwrap();
        let mut rest = &w.stream[..];
        while !rest.is_empty() {
            let (n, _) = d.decode(rest);
            rest = &rest[n..];
        }
        let m = ChunkedMetrics { records: want.len() as u64, ..ChunkedMetrics::default() };
        prop_assert_eq!(d.metrics(), &m);
    }

    #[test]
    fn a_join_yields_the_records_from_the_next_frame_start_on(
        size in size(),
        ops in ops(150),
        frames in prop::collection::vec(any::<prop::sample::Index>(), 24),
        offsets in prop::collection::vec(any::<prop::sample::Index>(), 8),
        pieces in pieces(),
    ) {
        let w = write(size, &ops);
        let len = w.stream.len();
        let count = len.div_ceil(size).max(1);
        let mut starts: Vec<usize> = frames.iter().map(|i| i.index(count) * size).collect();
        starts.extend(offsets.iter().map(|i| i.index(len.max(1))));
        for offset in starts.into_iter().filter(|&o| o < len) {
            let next = offset.next_multiple_of(size) as u64;
            let want: Vec<Ev> = w
                .records
                .iter()
                .filter(|r| r.offset >= next)
                .map(Ev::of)
                .collect();
            let got = decode(size, 200, &[(offset as u64, &w.stream[offset..])], &pieces);
            prop_assert_eq!(got, want, "offset {}", offset);
        }
    }

    /// Each lost frame is a gap (the decoder resets after it) or zeros.
    #[test]
    fn lost_chunks_cost_the_records_that_touch_them(
        size in size(),
        ops in ops(150),
        lost in prop::collection::vec((any::<prop::sample::Index>(), any::<bool>()), 1..4),
        pieces in pieces(),
    ) {
        let mut w = write(size, &ops);
        let frames = w.stream.len().div_ceil(size);
        prop_assume!(frames > 0);
        let mut gaps = vec![false; frames];
        let mut damaged = vec![false; frames];
        for (i, gap) in lost {
            let f = i.index(frames);
            damaged[f] = true;
            gaps[f] |= gap;
            if !gaps[f] {
                let end = ((f + 1) * size).min(w.stream.len());
                w.stream[f * size..end].fill(0);
            }
        }
        let mut segments: Vec<(u64, &[u8])> = Vec::new();
        let mut f = 0;
        while f < frames {
            if gaps[f] {
                f += 1;
                continue;
            }
            let start = f;
            while f < frames && !gaps[f] {
                f += 1;
            }
            let end = (f * size).min(w.stream.len());
            segments.push(((start * size) as u64, &w.stream[start * size..end]));
        }
        let want: Vec<Ev> = w
            .records
            .iter()
            .filter(|r| (0..frames).all(|f| !damaged[f] || !touches(r, size, f)))
            .map(Ev::of)
            .collect();
        let got = records(decode(size, 200, &segments, &pieces));
        prop_assert_eq!(got, want);
    }

    #[test]
    fn a_corrupt_header_costs_the_records_that_touch_its_frame(
        size in size(),
        ops in ops(150),
        frame in any::<prop::sample::Index>(),
        value in any::<u16>(),
        pieces in pieces(),
    ) {
        let mut w = write(size, &ops);
        let frames = w.stream.len().div_ceil(size);
        prop_assume!(frames > 0);
        let f = frame.index(frames);
        let original = header(&w.stream, size, f);
        prop_assume!(value != original);
        // After a padding frame the decoder joins, and trusts the header.
        prop_assume!(f == 0 || header(&w.stream, size, f - 1) != PADDING_FRAME);
        w.stream[f * size..f * size + HEADER_LEN].copy_from_slice(&value.to_be_bytes());
        let want: Vec<Ev> = w
            .records
            .iter()
            .filter(|r| !touches(r, size, f))
            .map(Ev::of)
            .collect();
        let got = records(decode(size, 200, &[(0, &w.stream)], &pieces));
        prop_assert_eq!(got, want);
    }

    #[test]
    fn records_longer_than_max_record_are_skipped_in_sync(
        size in 3..=40usize,
        ops in ops(100),
        max in 1..100usize,
        pieces in pieces(),
    ) {
        let w = write(size, &ops);
        let want: Vec<Ev> = w
            .records
            .iter()
            .filter(|r| r.payload.len() <= max)
            .map(Ev::of)
            .collect();
        let got = decode(size, max, &[(0, &w.stream)], &pieces);
        let oversized = w.records.iter().any(|r| r.payload.len() > max);
        prop_assert_eq!(got.contains(&Ev::Lost), oversized);
        prop_assert_eq!(records(got), want);
    }

    /// Random bytes, or a valid stream with bytes flipped.
    #[test]
    fn arbitrary_input_never_panics(
        size in prop_oneof![3..=40usize, Just(4096), Just(MAX_FRAME_SIZE)],
        max in prop_oneof![0..64usize, Just(usize::MAX)],
        offset in prop_oneof![Just(0u64), any::<u64>()],
        random in prop::collection::vec(any::<u8>(), 0..3000),
        valid in ops(100),
        flips in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 0..20),
        mutate in any::<bool>(),
        pieces in pieces(),
    ) {
        let input = if mutate {
            let mut b = write(size, &valid).stream;
            if !b.is_empty() {
                for (i, x) in flips {
                    let at = i.index(b.len());
                    b[at] ^= x;
                }
            }
            b
        } else {
            random
        };
        let evs = decode(size, max, &[(offset, &input)], &pieces);
        let decoded: usize = evs
            .iter()
            .map(|e| match e {
                Ev::Record(_, p) => p.len(),
                Ev::Lost => 0,
            })
            .sum();
        prop_assert!(decoded <= input.len());
    }
}
