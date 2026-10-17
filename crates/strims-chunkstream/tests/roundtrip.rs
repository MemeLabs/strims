// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Property tests of the reader contract (see the crate docs): written
//! records come back whole from offset 0, a decoder started at any chunk
//! offset yields a contiguous suffix of them, and arbitrary input never
//! panics or yields a payload above `max_record`. Debug builds check
//! arithmetic overflow, so an overflow is a failure too.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_chunkstream::{
    HEADER_LEN, MAX_UVARINT_LEN, RecordDecoder, RecordEncoder, get_uvarint, put_uvarint,
    uvarint_len,
};

fn config() -> Config {
    Config {
        cases: Config::default().cases.max(512),
        rng_seed: RngSeed::Fixed(0xc4a7_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

fn encode(size: usize, payloads: &[Vec<u8>]) -> Vec<u8> {
    let e = RecordEncoder::new(size).unwrap();
    let mut out = Vec::new();
    for p in payloads {
        let start = out.len();
        out.resize(start + e.encoded_len(p.len()).unwrap(), 0);
        e.encode(p, &mut out[start..]).unwrap();
    }
    out
}

/// Feeds `input` in pieces of the given sizes, cycling through them, and
/// collects the payloads, checking each call's result.
fn decode(d: &mut RecordDecoder, input: &[u8], pieces: &[usize]) -> Vec<Vec<u8>> {
    let mut records = Vec::new();
    let mut rest = input;
    for &piece in pieces.iter().cycle() {
        if rest.is_empty() {
            break;
        }
        let (mut chunk, tail) = rest.split_at(piece.clamp(1, rest.len()));
        rest = tail;
        loop {
            let r = d.decode(chunk);
            assert!(r.consumed <= chunk.len());
            chunk = &chunk[r.consumed..];
            if !r.record {
                assert!(chunk.is_empty(), "input left without a record");
                break;
            }
            assert!(d.record().len() <= d.max_record());
            records.push(d.record().to_vec());
        }
    }
    records
}

fn payloads(max_len: usize) -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop::collection::vec(prop::collection::vec(any::<u8>(), 0..max_len), 0..12)
}

fn pieces() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(1..300usize, 1..6)
}

fn frame_size() -> impl Strategy<Value = usize> {
    prop_oneof![HEADER_LEN + 1..=40, Just(1024)]
}

/// The index of the first payload a decoder started at `offset` must
/// yield, given each record's first and last frame index.
fn first_yielded(spans: &[(usize, usize)], size: usize, offset: usize) -> usize {
    let frame = offset / size;
    if offset.is_multiple_of(size) {
        spans
            .iter()
            .position(|&(first, _)| first >= frame)
            .unwrap_or(spans.len())
    } else {
        // The rest of this frame, and the frames up to the first
        // end-of-record header after it, make one record.
        spans
            .iter()
            .position(|&(_, last)| last > frame)
            .map_or(spans.len(), |i| i + 1)
    }
}

fn spans(e: &RecordEncoder, payloads: &[Vec<u8>]) -> Vec<(usize, usize)> {
    let mut next = 0;
    payloads
        .iter()
        .map(|p| {
            let frames = e.encoded_len(p.len()).unwrap() / e.frame_size();
            let span = (next, next + frames - 1);
            next += frames;
            span
        })
        .collect()
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn any_record_sequence_round_trips(
        size in frame_size(),
        payloads in payloads(300),
        pieces in pieces(),
    ) {
        let stream = encode(size, &payloads);
        let max = payloads.iter().map(Vec::len).max().unwrap_or(0);
        let mut d = RecordDecoder::new(size, max).unwrap();
        prop_assert_eq!(&decode(&mut d, &stream, &pieces), &payloads);
        prop_assert_eq!(d.metrics().records, payloads.len() as u64);
        prop_assert_eq!(d.metrics().length_mismatch + d.metrics().partial, 0);
    }

    /// Frames are a whole number of chunks, and chunks are at least 8 bytes,
    /// so a chunk offset is a frame start or 4 or more bytes into a frame's
    /// data. A record entered partway may pass the length check by chance,
    /// which adds one leading record.
    #[test]
    fn any_chunk_offset_yields_a_contiguous_suffix(
        chunk in prop::sample::select(vec![8usize, 16, 32]),
        chunks_per_frame in 1..=4usize,
        payloads in payloads(200),
        offset_seed in any::<usize>(),
        pieces in pieces(),
    ) {
        let size = chunk * chunks_per_frame;
        let e = RecordEncoder::new(size).unwrap();
        let stream = encode(size, &payloads);
        let offset = offset_seed % (stream.len() / chunk + 1) * chunk;
        let expected = &payloads[first_yielded(&spans(&e, &payloads), size, offset)..];

        let mut d = RecordDecoder::new(size, 256).unwrap();
        d.reset(offset as u64);
        let got = decode(&mut d, &stream[offset..], &pieces);
        prop_assert!(got.ends_with(expected), "offset {offset}: {got:?}");
        prop_assert!(got.len() <= expected.len() + 1, "offset {offset}: {got:?}");
    }

    /// Random bytes, or a valid stream with bytes flipped.
    #[test]
    fn arbitrary_input_never_panics_and_stays_within_max(
        size in frame_size(),
        max in 0..64usize,
        offset in any::<u64>(),
        random in prop::collection::vec(any::<u8>(), 0..2000),
        valid in payloads(100),
        flips in prop::collection::vec((any::<usize>(), any::<u8>()), 0..20),
        mutate in any::<bool>(),
        pieces in pieces(),
    ) {
        let input = if mutate {
            let mut b = encode(size, &valid);
            let n = b.len();
            for (i, x) in flips.into_iter().filter(|_| n > 0) {
                b[i % n] ^= x;
            }
            b
        } else {
            random
        };
        let mut d = RecordDecoder::new(size, max).unwrap();
        d.reset(offset);
        let records = decode(&mut d, &input, &pieces);
        prop_assert!(records.iter().all(|r| r.len() <= max));
        prop_assert_eq!(d.metrics().records, records.len() as u64);
    }

    #[test]
    fn uvarints_round_trip(v in any::<u64>(), tail in prop::collection::vec(any::<u8>(), 0..4)) {
        let mut b = [0; MAX_UVARINT_LEN];
        let n = put_uvarint(&mut b, v);
        prop_assert_eq!(n, uvarint_len(v));
        let mut encoded = b[..n].to_vec();
        encoded.extend_from_slice(&tail);
        prop_assert_eq!(get_uvarint(&encoded), Some((v, n)));
    }

    #[test]
    fn arbitrary_varint_bytes_never_panic(b in prop::collection::vec(any::<u8>(), 0..16)) {
        if let Some((_, n)) = get_uvarint(&b) {
            prop_assert!(n <= MAX_UVARINT_LEN && n <= b.len());
        }
    }
}
