// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The export's rows for frames written from known messages and
//! decisions.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use serde_json::{Value, json};
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::{ChannelWriter, Data, Have, Message, MessageWriter, Ping, Pong, Request};
use strims_ppspp_capture::{
    Clock, Dir, Header, Reader, Record, SwarmParams, TapRecord, Writer, export, tap_kind,
};

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

fn code(name: &str) -> u32 {
    (1..64)
        .find(|&c| tap_kind(c).is_some_and(|k| k.name == name))
        .unwrap()
}

/// One frame body: a payload per (channel, messages).
fn frame(payloads: &[(u64, &[Message<'_>])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (channel, msgs) in payloads {
        let mut buf = vec![0u8; 1024];
        let mut cw = ChannelWriter::new(&mut buf, *channel);
        for m in *msgs {
            cw.write(m).unwrap();
        }
        let n = cw.finish();
        out.extend_from_slice(&buf[..n]);
    }
    out
}

fn rows(dump: &[u8]) -> Vec<Value> {
    let mut r = Reader::new(dump).unwrap();
    let mut out = Vec::new();
    let stats = export(&mut r, &mut out, &mut |e| panic!("{e}")).unwrap();
    let rows: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(stats.rows, rows.len() as u64);
    rows
}

#[test]
fn rows_describe_messages_and_decisions() {
    let params = SwarmParams {
        chunk_size: 4,
        integrity_hash_size: 32,
        integrity_signature_size: 64,
        stream_count: 2,
        live_window: 64,
        ..SwarmParams::default()
    };
    let out = frame(&[(
        9,
        &[
            Message::Data(Data {
                bin: Bin(2),
                timestamp: t(10),
                data: &[1, 2, 3, 4],
            }),
            Message::Have(Have { bin: Bin(5) }),
            Message::Request(Request {
                bin: Bin(6),
                timestamp: t(9),
            }),
            Message::Ping(Ping { nonce: 42 }),
        ],
    )]);
    let inbound = frame(&[
        (
            3,
            &[
                Message::Data(Data {
                    bin: Bin(8),
                    timestamp: t(11),
                    data: &[5, 6, 7, 8],
                }),
                Message::Pong(Pong {
                    nonce: 42,
                    delay: 3,
                }),
            ],
        ),
        (77, &[Message::Have(Have { bin: Bin(0) })]),
    ]);
    let mut w = Writer::new(
        Vec::new(),
        &Header {
            clock: Clock::Sim,
            origin: t(0),
            file: 0,
            node: &[5],
            label: "",
        },
    )
    .unwrap();
    let recs = [
        (
            t(0),
            Record::Peer {
                peer: 1,
                name: &[0],
            },
        ),
        (
            t(0),
            Record::Peer {
                peer: 2,
                name: &[7],
            },
        ),
        (
            t(0),
            Record::Swarm {
                swarm: 1,
                id: &[1; 32],
                params,
            },
        ),
        (
            t(0),
            Record::ChannelOpen {
                peer: 1,
                swarm: 1,
                channel: 3,
                peer_channel: 9,
            },
        ),
        (
            t(10),
            Record::Frame {
                dir: Dir::Out,
                peer: 1,
                len: out.len() as u64,
                body: &out,
            },
        ),
        (
            t(10),
            Record::Tap(TapRecord::new(
                code("DataSent"),
                1,
                1,
                &[2, 1, t(5).unix_nanos()],
            )),
        ),
        (
            t(20),
            Record::Frame {
                dir: Dir::In,
                peer: 1,
                len: inbound.len() as u64,
                body: &inbound,
            },
        ),
        (
            t(20),
            Record::Tap(TapRecord::new(
                code("PushHit"),
                1,
                1,
                &[8, Duration::from_millis(7).as_nanos()],
            )),
        ),
        (
            t(20),
            Record::Tap(TapRecord::new(code("PushMiss"), 1, 1, &[10])),
        ),
        (
            t(30),
            Record::Tap(TapRecord::new(code("Switched"), 2, 1, &[1, 1])),
        ),
        (t(30), Record::TapLost { count: 3 }),
    ];
    for (at, r) in &recs {
        w.write(*at, r).unwrap();
    }
    let got = rows(&w.into_inner());
    let n = |ms: i64| t(ms).unix_nanos();
    let base = |ms, peer: &str, dir: &str, frame: u64| json!({"t": n(ms), "node": "05", "peer": peer, "dir": dir, "frame": frame});
    let with = |mut v: Value, extra: Value| {
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    };
    let want = [
        with(
            base(10, "00", "out", 1),
            json!({"type": "FRAME", "len": out.len()}),
        ),
        with(
            base(10, "00", "out", 1),
            json!({
                "swarm": 1, "channel": 9, "type": "DATA", "bin": 2, "first": 1, "last": 1,
                "stream": 1, "ts": n(10), "len": 1 + 1 + 6 + 4, "ann": "push",
                "wait": Duration::from_millis(5).as_nanos(),
            }),
        ),
        with(
            base(10, "00", "out", 1),
            json!({
                "swarm": 1, "channel": 9, "type": "HAVE", "bin": 5, "first": 2, "last": 3,
                "len": 2,
            }),
        ),
        with(
            base(10, "00", "out", 1),
            json!({
                "swarm": 1, "channel": 9, "type": "REQUEST", "bin": 6, "first": 3, "last": 3,
                "stream": 1, "ts": n(9), "len": 1 + 1 + 6,
            }),
        ),
        with(
            base(10, "00", "out", 1),
            json!({"swarm": 1, "channel": 9, "type": "PING", "len": 1 + 8, "nonce": 42}),
        ),
        with(
            base(20, "00", "in", 2),
            json!({"type": "FRAME", "len": inbound.len()}),
        ),
        with(
            base(20, "00", "in", 2),
            json!({
                "swarm": 1, "channel": 3, "type": "DATA", "bin": 8, "first": 4, "last": 4,
                "stream": 0, "ts": n(11), "len": 1 + 1 + 6 + 4, "ann": "hit",
                "lag": Duration::from_millis(7).as_nanos(),
            }),
        ),
        with(
            base(20, "00", "in", 2),
            json!({
                "swarm": 1, "channel": 3, "type": "PONG", "len": 1 + 16, "delay": 3,
                "nonce": 42,
            }),
        ),
        with(
            base(20, "00", "in", 2),
            json!({
                "channel": 77, "type": "UNKNOWN_CHANNEL", "len": 2,
            }),
        ),
        with(
            base(20, "00", "tap", 2),
            json!({
                "swarm": 1, "type": "PushMiss", "bin": 10, "first": 5, "last": 5, "stream": 1,
            }),
        ),
        with(
            base(30, "07", "tap", 2),
            json!({
                "swarm": 1, "type": "Switched", "stream": 1, "from": "00",
            }),
        ),
        json!({"t": n(30), "node": "05", "dir": "tap", "frame": 2, "type": "TapLost", "count": 3}),
    ];
    assert_eq!(got.len(), want.len(), "{got:#?}");
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g, w);
    }
}

#[test]
fn malformed_frames_become_rows() {
    let mut w = Writer::new(
        Vec::new(),
        &Header {
            clock: Clock::Node,
            origin: t(0),
            file: 0,
            node: &[1],
            label: "",
        },
    )
    .unwrap();
    w.write(
        t(1),
        &Record::Frame {
            dir: Dir::In,
            peer: 4,
            len: 3,
            body: &[0xff, 0xff, 0xff],
        },
    )
    .unwrap();
    let got = rows(&w.into_inner());
    let kinds: Vec<&str> = got.iter().map(|r| r["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["FRAME", "MALFORMED"]);
    assert_eq!(got[0]["peer"], "?4");
}
