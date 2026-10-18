// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors from `golden/main.go` (regenerate from the repo root with
//! `go run -tags golden ./crates/strims-chunkstream/golden`; the line
//! formats are documented there). The encoders must reproduce the recorded
//! frames byte for byte, the packed encoder with the recorded output after
//! every write and end, and a decoder started at each recorded offset must
//! yield the recorded records.

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
use strims_chunkstream::packed::{FrameHasEnd, PackedDecoder, PackedEncoder};
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
fn frames_match_go() {
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
fn large_frames_match_go() {
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
fn event_broadcasts_match_go() {
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
fn streams_match_go() {
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
fn reads_match_go() {
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

/// A replayed `packed` script.
struct Script {
    stream: Vec<u8>,
    /// The records whose end Go accepted.
    records: Vec<Vec<u8>>,
    /// The bytes written after the last of them.
    pending: Vec<u8>,
}

/// Replays a `packed` script through a [`PackedEncoder`], checking each
/// end's outcome and the output length after every op.
fn packed_replay(size: usize, ops: &str) -> Script {
    let mut e = PackedEncoder::new(size).unwrap();
    let mut s = Script {
        stream: Vec::new(),
        records: Vec::new(),
        pending: Vec::new(),
    };
    let mut written = 0;
    for op in ops.split(',') {
        let (op, out) = op.split_once(':').unwrap();
        match op {
            "e" => {
                assert_eq!(e.end_record(&mut s.stream), Ok(()), "size {size}");
                s.records.push(std::mem::take(&mut s.pending));
            }
            "E" => assert_eq!(e.end_record(&mut s.stream), Err(FrameHasEnd), "size {size}"),
            w => {
                let n: usize = w.strip_prefix('w').unwrap().parse().unwrap();
                let data: Vec<u8> = (written..written + n)
                    .map(|i| ((i * 131 + 7) % 256) as u8)
                    .collect();
                e.write(&data, &mut s.stream);
                s.pending.extend_from_slice(&data);
                written += n;
            }
        }
        assert_eq!(
            s.stream.len(),
            out.parse::<usize>().unwrap(),
            "size {size}, after {op} in {ops}"
        );
    }
    s
}

/// The records a packed decoder started at `offset` ends in `input`, fed
/// in `piece` byte pieces, and the bytes it yields after the last.
fn packed_decode(size: usize, offset: u64, input: &[u8], piece: usize) -> (Vec<Vec<u8>>, Vec<u8>) {
    let mut d = PackedDecoder::new(size).unwrap();
    d.reset(offset);
    let (mut records, mut tail) = (Vec::new(), Vec::new());
    for mut chunk in input.chunks(piece) {
        loop {
            let (n, frag) = d.decode(chunk);
            chunk = &chunk[n..];
            match frag {
                Some(Frag::Data(b)) => tail.extend_from_slice(b),
                Some(Frag::End) => records.push(std::mem::take(&mut tail)),
                Some(Frag::Corrupt) => panic!("corrupt frame from offset {offset}"),
                None => break,
            }
        }
    }
    (records, tail)
}

/// A stream decoded from offset 0 yields the records Go ended, then the
/// output part of the record in progress.
fn assert_decodes_to_script(size: usize, s: &Script) {
    let (records, tail) = packed_decode(size, 0, &s.stream, s.stream.len().max(1));
    assert_eq!(records, s.records, "size {size}");
    assert!(s.pending.starts_with(&tail), "size {size}");
}

#[test]
fn packed_writes_match_go() {
    let mut refused = 0;
    for f in lines("packed") {
        let size: usize = f[1].parse().unwrap();
        let s = packed_replay(size, f[2]);
        assert_eq!(hex::encode(&s.stream), f[3], "stream {}", f[0]);
        assert_decodes_to_script(size, &s);
        refused += f[2].matches('E').count();
    }
    assert!(refused > 10, "{refused} refused ends");
}

#[test]
fn large_packed_writes_match_go() {
    for f in lines("packed_digest") {
        let size: usize = f[1].parse().unwrap();
        let s = packed_replay(size, f[2]);
        let headers: Vec<u8> = s
            .stream
            .chunks(size)
            .flat_map(|c| c[..HEADER_LEN].to_vec())
            .collect();
        assert_eq!(hex::encode(headers), f[4], "stream {}", f[0]);
        assert_eq!(
            hex::encode(Sha256::digest(&s.stream)),
            f[3],
            "stream {}",
            f[0]
        );
        assert_decodes_to_script(size, &s);
    }
}

#[test]
fn packed_reads_match_go() {
    let streams: BTreeMap<&str, (usize, Vec<u8>)> = lines("packed")
        .into_iter()
        .map(|f| (f[0], (f[1].parse().unwrap(), bytes(f[3]))))
        .collect();
    let reads = lines("packed_read");
    assert!(reads.len() > 1000, "{} reads", reads.len());
    for f in reads {
        let (size, stream) = &streams[f[0]];
        let offset: usize = f[1].parse().unwrap();
        let input = &stream[offset..];
        for piece in [input.len().max(1), 7, 1] {
            let (records, tail) = packed_decode(*size, offset as u64, input, piece);
            let lens: Vec<String> = records.iter().map(|r| r.len().to_string()).collect();
            let lens = if lens.is_empty() {
                "-".to_owned()
            } else {
                lens.join(",")
            };
            let mut sum = Sha256::new();
            for r in &records {
                sum.update(r);
            }
            sum.update(&tail);
            let at = format!("stream {} offset {offset} piece {piece}", f[0]);
            assert_eq!(lens, f[2], "{at}");
            assert_eq!(tail.len().to_string(), f[3], "{at}");
            assert_eq!(hex::encode(&sum.finalize()[..8]), f[4], "{at}");
        }
    }
}
