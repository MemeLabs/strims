// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Recorded golden vectors (`golden/vectors.hex`). The encoders must
//! reproduce the recorded frames byte for byte, and a decoder started at
//! each recorded offset must yield the recorded records.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use prost::Message;
use sha2::{Digest, Sha256};
use strims_chunkstream::{
    Frag, FrameDecoder, FrameEncoder, HEADER_LEN, RecordDecoder, RecordEncoder,
};
use strims_proto::network::v1::directory::{EventBroadcast, event};

const VECTORS: &str = include_str!("../golden/vectors.hex");

fn lines(kind: &str) -> Vec<Vec<&'static str>> {
    let lines: Vec<Vec<&str>> = VECTORS
        .lines()
        .map(|l| l.split(' ').collect::<Vec<_>>())
        .filter(|f| f[0] == kind)
        .map(|f| f[1..].to_vec())
        .collect();
    assert!(!lines.is_empty(), "no {kind} vectors");
    lines
}

fn bytes(s: &str) -> Vec<u8> {
    if s == "=" {
        Vec::new()
    } else {
        hex::decode(s).unwrap()
    }
}

fn pattern(n: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 131 + 7) % 256) as u8).collect()
}

fn frame_encode(size: usize, record: &[u8]) -> Vec<u8> {
    let e = FrameEncoder::new(size).unwrap();
    let mut out = vec![0xee; e.encoded_len(record.len()).unwrap()];
    assert_eq!(e.encode(&[record], &mut out), Ok(out.len()));
    out
}

/// The record bytes of the first record in `frames`.
fn frame_decode(size: usize, mut frames: &[u8]) -> Vec<u8> {
    let mut d = FrameDecoder::new(size).unwrap();
    let mut record = Vec::new();
    loop {
        let (n, frag) = d.decode(frames);
        frames = &frames[n..];
        match frag {
            Some(Frag::Data(b)) => record.extend_from_slice(b),
            Some(Frag::End) => return record,
            f => panic!("unexpected {f:?}"),
        }
    }
}

fn record_encode(size: usize, payloads: &[Vec<u8>]) -> Vec<u8> {
    let e = RecordEncoder::new(size).unwrap();
    let mut out = Vec::new();
    for p in payloads {
        let start = out.len();
        out.resize(start + e.encoded_len(p.len()).unwrap(), 0xee);
        assert_eq!(e.encode(p, &mut out[start..]), Ok(out.len() - start));
    }
    out
}

/// The payloads a decoder started at `offset` yields from `input`, fed in
/// `piece` byte pieces.
fn record_decode(size: usize, offset: u64, input: &[u8], piece: usize) -> Vec<Vec<u8>> {
    let mut d = RecordDecoder::new(size, 1 << 20).unwrap();
    d.reset(offset);
    let mut records = Vec::new();
    for mut chunk in input.chunks(piece) {
        loop {
            let r = d.decode(chunk);
            chunk = &chunk[r.consumed..];
            if !r.record {
                break;
            }
            records.push(d.record().to_vec());
        }
    }
    records
}

#[test]
fn frames_match_vectors() {
    for f in lines("frames") {
        let size: usize = f[0].parse().unwrap();
        let (record, frames) = (bytes(f[1]), bytes(f[2]));
        assert_eq!(record, pattern(record.len()));
        assert_eq!(
            hex::encode(frame_encode(size, &record)),
            f[2],
            "size {size}, record {}",
            record.len()
        );
        assert_eq!(frame_decode(size, &frames), record);
    }
}

#[test]
fn large_frames_match_vectors() {
    for f in lines("frames_digest") {
        let size: usize = f[0].parse().unwrap();
        let record = pattern(f[1].parse().unwrap());
        let frames = frame_encode(size, &record);
        let headers: Vec<u8> = frames
            .chunks(size)
            .flat_map(|c| c[..HEADER_LEN].to_vec())
            .collect();
        assert_eq!(hex::encode(headers), f[3], "record {}", record.len());
        assert_eq!(
            hex::encode(Sha256::digest(&frames)),
            f[2],
            "record {}",
            record.len()
        );
        assert_eq!(frame_decode(size, &frames), record);
    }
}

#[test]
fn event_broadcasts_match_vectors() {
    let mut pings = 0;
    for f in lines("records") {
        let (proto, stream) = (bytes(f[0]), bytes(f[1]));
        let m = EventBroadcast::decode(proto.as_slice()).unwrap();
        assert_eq!(m.encode_to_vec(), proto);
        assert_eq!(
            hex::encode(record_encode(1024, std::slice::from_ref(&proto))),
            f[1]
        );
        let records = record_decode(1024, 0, &stream, stream.len());
        assert_eq!(records, std::slice::from_ref(&proto));
        let decoded = EventBroadcast::decode(records[0].as_slice()).unwrap();
        assert_eq!(decoded, m);
        if let Some(event::Body::Ping(p)) = &decoded.events[0].body {
            assert_eq!(p.time, 1_257_894_000_000_000_000);
            pings += 1;
        }
    }
    assert_eq!(pings, 1);
}

struct Stream {
    size: usize,
    bytes: Vec<u8>,
    payloads: Vec<Vec<u8>>,
}

fn streams() -> BTreeMap<&'static str, Stream> {
    lines("stream")
        .into_iter()
        .map(|f| {
            let stream = Stream {
                size: f[1].parse().unwrap(),
                bytes: bytes(f[2]),
                payloads: f[3].split(',').map(bytes).collect(),
            };
            (f[0], stream)
        })
        .collect()
}

#[test]
fn streams_match_vectors() {
    for (id, s) in streams() {
        assert_eq!(
            hex::encode(record_encode(s.size, &s.payloads)),
            hex::encode(&s.bytes),
            "stream {id}"
        );
        assert_eq!(
            record_decode(s.size, 0, &s.bytes, s.bytes.len()),
            s.payloads,
            "stream {id}"
        );
    }
}

/// A read marked `end` lists every record the decoder must yield. One
/// marked `err` stopped early, so its records must be a prefix of them.
#[test]
fn reads_match_vectors() {
    let streams = streams();
    let (mut ended, mut failed) = (0, 0);
    for f in lines("read") {
        let s = &streams[f[0]];
        let offset: usize = f[1].parse().unwrap();
        let want: Vec<Vec<u8>> = match f[3] {
            "-" => Vec::new(),
            items => items
                .split(',')
                .map(|i| match i.strip_prefix('x') {
                    Some(h) => bytes(if h.is_empty() { "=" } else { h }),
                    None => s.payloads[i.parse::<usize>().unwrap()].clone(),
                })
                .collect(),
        };
        let input = &s.bytes[offset..];
        for piece in [input.len().max(1), 7, 1] {
            let got = record_decode(s.size, offset as u64, input, piece);
            match f[2] {
                "end" => assert_eq!(got, want, "stream {} offset {offset}", f[0]),
                "err" => assert!(
                    got.starts_with(&want),
                    "stream {} offset {offset}: the {} recorded aren't a prefix of {}",
                    f[0],
                    want.len(),
                    got.len()
                ),
                s => panic!("status {s}"),
            }
        }
        match f[2] {
            "end" => ended += 1,
            _ => failed += 1,
        }
    }
    assert!(ended > 700 && failed > 0, "{ended} ended, {failed} failed");
}
