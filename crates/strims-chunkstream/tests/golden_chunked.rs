// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors of the chunked layout (`golden/chunked.txt`).
//!
//! - `write`: a `ChunkedEncoder` replays `ops`: `r<n>` writes a record of
//!   `n` payload bytes, from a byte pattern that runs on across records;
//!   `f` ends the frame; `s<n>` ends the frame and continues `n` frames
//!   later. Pinned: the bytes written per op (`lens`), the stream a reader
//!   sees, the left-out frames as zeros (`stream`, or its SHA-256 `digest`
//!   and frame `headers` when large), the record offsets (`starts`), and,
//!   for a decoder joining at each frame start, the index of the first
//!   record it yields (`joins`), all the later ones following.
//! - `decode`: a decoder started at `offset` reads `in`, fed whole, in 7
//!   byte and in 1 byte pieces. Pinned: its records (`r<offset>:<payload>`)
//!   and losses, and its counters (`metrics`: records, lost, oversized,
//!   corrupt).
//!
//! `STRIMS_GOLDEN=update` rewrites the pinned fields from the code (see
//! `strims-golden`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use sha2::{Digest, Sha256};
use strims_chunkstream::chunked::{ChunkedEncoder, ChunkedRecordDecoder, HEADER_LEN, Item};
use strims_golden::Golden;

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/chunked.txt"))
}

fn num<T: std::str::FromStr>(g: &Golden, e: usize, k: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    g.str(e, k).parse().unwrap()
}

fn list<T: ToString>(items: &[T]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.iter().map(T::to_string).collect::<Vec<_>>().join(",")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ev {
    Record(u64, Vec<u8>),
    Lost,
}

impl std::fmt::Display for Ev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ev::Record(offset, payload) => write!(f, "r{offset}:{}", hex::encode(payload)),
            Ev::Lost => f.write_str("lost"),
        }
    }
}

/// The records and losses a decoder started at `offset` yields from
/// `input`, fed in `piece` byte pieces, and its counters.
fn decode(size: usize, max: usize, offset: u64, input: &[u8], piece: usize) -> (Vec<Ev>, String) {
    let mut d = ChunkedRecordDecoder::new(size, max).unwrap();
    d.reset(offset);
    let mut evs = Vec::new();
    for mut chunk in input.chunks(piece.max(1)) {
        loop {
            let (n, item) = d.decode(chunk);
            chunk = &chunk[n..];
            match item {
                None => break,
                Some(Item::Record { offset, payload }) => {
                    evs.push(Ev::Record(offset, payload.to_vec()))
                }
                Some(Item::Lost) => evs.push(Ev::Lost),
            }
        }
    }
    let m = d.metrics();
    let metrics = format!("{},{},{},{}", m.records, m.lost, m.oversized, m.corrupt);
    (evs, metrics)
}

/// [`decode`] fed whole, in 7 byte and in 1 byte pieces, which must agree.
fn decode_pieces(size: usize, max: usize, offset: u64, input: &[u8]) -> (Vec<Ev>, String) {
    let whole = decode(size, max, offset, input, input.len());
    for piece in [7, 1] {
        assert_eq!(
            decode(size, max, offset, input, piece),
            whole,
            "piece {piece}"
        );
    }
    whole
}

struct Replay {
    stream: Vec<u8>,
    lens: Vec<usize>,
    records: Vec<Ev>,
}

fn replay(size: usize, ops: &str) -> Replay {
    let mut e = ChunkedEncoder::new(size).unwrap();
    let mut r = Replay {
        stream: Vec::new(),
        lens: Vec::new(),
        records: Vec::new(),
    };
    let mut written = 0;
    for op in ops.split(',') {
        let before = r.stream.len();
        let end_frame = |e: &mut ChunkedEncoder, out: &mut Vec<u8>| {
            let start = out.len();
            out.resize(start + e.frame_padding(), 0xee);
            assert_eq!(e.end_frame(&mut out[start..]), Ok(out.len() - start));
        };
        match op.split_at(1) {
            ("r", n) => {
                let n: usize = n.parse().unwrap();
                let payload: Vec<u8> = (written..written + n)
                    .map(|i| ((i * 131 + 7) % 256) as u8)
                    .collect();
                written += n;
                let offset = e.next_record_offset();
                let len = e.encoded_len(n).unwrap();
                r.stream.resize(before + len, 0xee);
                assert_eq!(e.encode(&[&payload], &mut r.stream[before..]), Ok(len));
                r.records.push(Ev::Record(offset, payload));
            }
            ("f", "") => end_frame(&mut e, &mut r.stream),
            ("s", n) => {
                let n: usize = n.parse().unwrap();
                end_frame(&mut e, &mut r.stream);
                e.continue_at(e.offset() + (n * size) as u64).unwrap();
            }
            _ => panic!("op {op}"),
        }
        r.lens.push(r.stream.len() - before);
        // The left-out frames read as zeros.
        r.stream.resize(e.offset() as usize, 0);
    }
    r
}

#[test]
fn writes_match_vectors() {
    let mut g = open();
    let entries = g.all("write");
    assert!(entries.len() >= 4, "{} write vectors", entries.len());
    for e in entries {
        let name = g.str(e, "name").to_owned();
        let size: usize = num(&g, e, "size");
        let r = replay(size, g.str(e, "ops"));
        g.check(e, "lens", list(&r.lens));
        if size <= 64 {
            g.check_hex(e, "stream", &r.stream);
        } else {
            let headers: Vec<u8> = r
                .stream
                .chunks(size)
                .flat_map(|c| c[..HEADER_LEN].to_vec())
                .collect();
            g.check_hex(e, "headers", headers);
            g.check_hex(e, "digest", Sha256::digest(&r.stream));
        }
        let starts: Vec<u64> = r
            .records
            .iter()
            .map(|ev| match ev {
                Ev::Record(offset, _) => *offset,
                Ev::Lost => unreachable!(),
            })
            .collect();
        g.check(e, "starts", list(&starts));

        let max = 1 << 20;
        let (got, _) = if size <= 64 {
            decode_pieces(size, max, 0, &r.stream)
        } else {
            decode(size, max, 0, &r.stream, 1000)
        };
        assert_eq!(got, r.records, "{name}");

        let mut joins = Vec::new();
        for start in (0..r.stream.len()).step_by(size) {
            let (got, _) = decode(size, max, start as u64, &r.stream[start..], 4096);
            let first = starts.partition_point(|&s| s < start as u64);
            assert_eq!(got, r.records[first..], "{name}: join at {start}");
            joins.push(first);
        }
        g.check(e, "joins", list(&joins));
    }
}

#[test]
fn decodes_match_vectors() {
    let mut g = open();
    let entries = g.all("decode");
    assert!(entries.len() >= 10, "{} decode vectors", entries.len());
    for e in entries {
        let size: usize = num(&g, e, "size");
        let max: usize = num(&g, e, "max");
        let offset: u64 = num(&g, e, "offset");
        let input = g.bytes(e, "in");
        let (evs, metrics) = decode_pieces(size, max, offset, &input);
        g.check(e, "items", list(&evs));
        g.check(e, "metrics", metrics);
    }
}
