// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Property tests of the packed layout (see `strims_chunkstream::packed`).
//! A writer merges each record whose end is blocked into the next one,
//! pads it, or abandons it. The stream it writes holds one end per frame
//! and decodes whole from offset 0; from any other offset, a decoder yields
//! the records after the one it entered, in order. Arbitrary input never
//! panics, and the work stays linear in it. Debug builds check arithmetic
//! overflow, so an overflow is a failure too.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_chunkstream::packed::{DEFAULT_SIZE, FrameHasEnd, PackedDecoder, PackedEncoder};
use strims_chunkstream::{EOR_FLAG, Frag, HEADER_LEN};

fn config() -> Config {
    Config {
        cases: Config::default().cases.max(256),
        rng_seed: RngSeed::Fixed(0x9ac4_ed00),
        failure_persistence: None,
        ..Config::default()
    }
}

/// What a writer does with a record whose end is blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blocked {
    /// Leaves it open, so the next record's bytes extend it.
    Merge,
    /// Pads it to the unblocking length, then ends it.
    Pad,
    /// Drops it.
    Abandon,
}

fn blocked() -> impl Strategy<Value = Blocked> {
    prop_oneof![
        Just(Blocked::Merge),
        Just(Blocked::Pad),
        Just(Blocked::Abandon)
    ]
}

struct Written {
    stream: Vec<u8>,
    /// The ended records, each with the stream offset just past it.
    records: Vec<(Vec<u8>, u64)>,
}

/// Writes `data` in pieces of the given sizes, cycling through them.
fn write_pieces(e: &mut PackedEncoder, mut data: &[u8], pieces: &[usize], out: &mut Vec<u8>) {
    for &piece in pieces.iter().cycle() {
        if data.is_empty() {
            break;
        }
        let (head, tail) = data.split_at(piece.min(data.len()));
        e.write(head, out);
        data = tail;
    }
}

/// Writes each record and ends it, handling blocked ends as chosen, then
/// writes `trailing` without an end.
fn write(
    size: usize,
    records: &[(Vec<u8>, Blocked)],
    trailing: &[u8],
    pieces: &[usize],
) -> Written {
    let mut e = PackedEncoder::new(size).unwrap();
    let mut w = Written {
        stream: Vec::new(),
        records: Vec::new(),
    };
    let mut pending = Vec::new();
    for (bytes, choice) in records {
        let blocked = e.end_blocked_after(bytes.len());
        let mut bytes = bytes.clone();
        if blocked && *choice == Blocked::Pad {
            bytes.resize(e.unblocking_len(bytes.len()), 0);
            assert!(!e.end_blocked_after(bytes.len()));
        }
        write_pieces(&mut e, &bytes, pieces, &mut w.stream);
        pending.extend_from_slice(&bytes);
        match e.end_record(&mut w.stream) {
            Ok(()) => {
                assert!(!blocked || *choice == Blocked::Pad);
                w.records.push((std::mem::take(&mut pending), e.offset()));
            }
            Err(FrameHasEnd) => {
                assert!(blocked && *choice != Blocked::Pad);
                if *choice == Blocked::Abandon {
                    let offset = e.offset() - pending.len() as u64;
                    assert!(e.abandon_record(), "a blocked record is held whole");
                    assert_eq!(e.offset(), offset);
                    pending.clear();
                }
            }
        }
    }
    write_pieces(&mut e, trailing, pieces, &mut w.stream);
    w
}

/// Feeds `input` in pieces of the given sizes, cycling through them, and
/// returns the records ended and the bytes after the last, checking each
/// call's result and that the work is linear in the input.
fn decode(d: &mut PackedDecoder, input: &[u8], pieces: &[usize]) -> (Vec<Vec<u8>>, Vec<u8>) {
    let (mut records, mut tail) = (Vec::new(), Vec::new());
    let mut fragments = 0;
    let mut rest = input;
    for &piece in pieces.iter().cycle() {
        if rest.is_empty() {
            break;
        }
        let (mut chunk, after) = rest.split_at(piece.clamp(1, rest.len()));
        rest = after;
        loop {
            let (n, frag) = d.decode(chunk);
            assert!(n <= chunk.len());
            chunk = &chunk[n..];
            match frag {
                None => {
                    assert!(chunk.is_empty(), "input left without a fragment");
                    break;
                }
                Some(Frag::Data(b)) => {
                    assert!(!b.is_empty());
                    tail.extend_from_slice(b);
                }
                Some(Frag::End) => records.push(std::mem::take(&mut tail)),
                // The record in progress is lost.
                Some(Frag::Corrupt) => tail.clear(),
            }
            fragments += 1;
        }
    }
    // Data consumes input, and each frame has at most one End or Corrupt.
    let frames = input.len() / d.size() + 2;
    assert!(fragments <= input.len() + frames, "{fragments} fragments");
    (records, tail)
}

fn records(max_len: usize) -> impl Strategy<Value = Vec<(Vec<u8>, Blocked)>> {
    prop::collection::vec(
        (prop::collection::vec(any::<u8>(), 0..max_len), blocked()),
        0..10,
    )
}

fn pieces() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(1..100usize, 1..6)
}

/// The first frame whose header a decoder started at `offset` reads.
fn first_header(size: usize, offset: usize) -> usize {
    offset.div_ceil(size)
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn records_decode_from_every_offset_to_an_in_order_suffix(
        size in HEADER_LEN + 1..=40usize,
        records in records(64),
        trailing in prop::collection::vec(any::<u8>(), 0..40),
        pieces in pieces(),
    ) {
        let w = write(size, &records, &trailing, &pieces);
        let stream = &w.stream;
        let ends: Vec<u64> = w.records.iter().map(|&(_, end)| end).collect();
        let records: Vec<Vec<u8>> = w.records.into_iter().map(|(r, _)| r).collect();

        // One end per frame, and each in its frame's header.
        let end_frames: Vec<usize> = ends.iter().map(|&e| e as usize / size).collect();
        prop_assert!(end_frames.windows(2).all(|f| f[0] < f[1]), "ends {ends:?}");
        let tail_len = stream.len() % size;
        prop_assert!(tail_len == 0 || tail_len >= HEADER_LEN);
        for (f, frame) in stream.chunks(size).enumerate() {
            let header = u32::from_be_bytes(frame[..HEADER_LEN].try_into().unwrap());
            let want = match end_frames.iter().position(|&e| e == f) {
                Some(i) => (u32::from(EOR_FLAG) << 24) | (ends[i] as usize % size) as u32,
                None => 0,
            };
            prop_assert_eq!(header, want, "frame {}", f);
        }

        let mut d = PackedDecoder::new(size).unwrap();
        let (got, after_last) = decode(&mut d, stream, &pieces);
        prop_assert_eq!(&got, &records);

        for offset in 1..=stream.len() {
            let mut d = PackedDecoder::new(size).unwrap();
            d.reset(offset as u64);
            let (got, tail) = decode(&mut d, &stream[offset..], &pieces);
            let first = first_header(size, offset);
            match end_frames.iter().position(|&f| f >= first) {
                Some(i) => {
                    // The first record ends at end i and holds the stream's
                    // data from the start offset.
                    let before: Vec<u8> = records[..=i].concat();
                    prop_assert!(before.ends_with(&got[0]), "offset {offset}");
                    prop_assert_eq!(&got[1..], &records[i + 1..], "offset {}", offset);
                    prop_assert_eq!(&tail, &after_last, "offset {}", offset);
                }
                None => prop_assert!(got.is_empty(), "offset {offset}"),
            }
        }
    }

    #[test]
    fn an_end_is_refused_exactly_when_blocked(
        size in HEADER_LEN + 1..=40usize,
        lens in prop::collection::vec(0..100usize, 1..20),
    ) {
        let mut e = PackedEncoder::new(size).unwrap();
        let mut out = Vec::new();
        for len in lens {
            let blocked = e.end_blocked_after(len);
            let n = e.unblocking_len(len);
            prop_assert!(n >= len);
            prop_assert!(!e.end_blocked_after(n));
            prop_assert!(n == len || e.end_blocked_after(n - 1));
            let before = out.len();
            e.write(&vec![1; len], &mut out);
            prop_assert_eq!(e.end_record(&mut out).is_err(), blocked);
            if blocked {
                prop_assert_eq!(out.len(), before, "a blocked record's bytes are held");
                prop_assert!(e.abandon_record());
            }
        }
    }

    /// Random bytes, or a valid stream with bytes flipped.
    #[test]
    fn arbitrary_input_never_panics(
        size in prop_oneof![HEADER_LEN + 1..=40usize, Just(1024), Just(DEFAULT_SIZE)],
        offset in any::<u64>(),
        random in prop::collection::vec(any::<u8>(), 0..2000),
        valid in records(100),
        flips in prop::collection::vec((any::<usize>(), any::<u8>()), 0..20),
        mutate in any::<bool>(),
        pieces in pieces(),
    ) {
        let input = if mutate {
            let mut b = write(size, &valid, &[], &pieces).stream;
            let n = b.len();
            for (i, x) in flips.into_iter().filter(|_| n > 0) {
                b[i % n] ^= x;
            }
            b
        } else {
            random
        };
        let mut d = PackedDecoder::new(size).unwrap();
        d.reset(offset);
        let (records, tail) = decode(&mut d, &input, &pieces);
        let decoded: usize = records.iter().map(Vec::len).sum::<usize>() + tail.len();
        prop_assert!(decoded <= input.len());
    }
}
