// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors from the Go implementation (`golden/main.go`).
//!
//! - every codec message, a multi-message payload, channel headers and a
//!   frame: Rust decodes Go's bytes to the expected messages and encodes
//!   them back byte for byte;
//! - Merkle digests of every bin for each supported hash function;
//! - the Go `ppspp.Writer` for Merkle (all hash functions, ed25519 and no
//!   signatures), sign-all and no integrity: Rust's `Writer`, replaying the
//!   same writes at the times Go used, must produce identical epochs,
//!   `Data` messages, integrity messages (with several peer have maps) and
//!   swarm caches; Go's caches must import into Rust and export again
//!   unchanged;
//! - swarm id and URI strings and parsing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use prost::Message as _;
use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Map};
use strims_ppspp::codec::{
    self, Ack, Cancel, ChannelHeader, ChannelWriter, Data, EpochOption, FrameReader, Handshake,
    Have, Integrity, Message, MessageWriter, Ping, Pong, ProtocolOption, ProtocolOptionType,
    ProtocolOptions, Reader, ReaderConfig, Request, SignedIntegrity, StreamCancel, StreamClose,
    StreamOpen, StreamRequest, Writer as CodecWriter,
};
use strims_ppspp::integrity::{
    LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod, VerifierOptions,
};
use strims_ppspp::merkle::Tree;
use strims_ppspp::store;
use strims_ppspp::{Swarm, SwarmId, SwarmOptions, Uri, UriOptions, Writer, WriterOptions};
use strims_proto::types::Cache;

const VECTORS: &str = include_str!("../golden/vectors.txt");

struct Line<'a> {
    kind: &'a str,
    kv: BTreeMap<&'a str, &'a str>,
}

impl<'a> Line<'a> {
    fn get(&self, k: &str) -> &'a str {
        self.kv
            .get(k)
            .copied()
            .unwrap_or_else(|| panic!("{} has no {k}", self.kind))
    }

    fn opt(&self, k: &str) -> Option<&'a str> {
        self.kv.get(k).copied()
    }

    fn hex(&self, k: &str) -> Vec<u8> {
        unhex(self.get(k))
    }

    fn num<T: std::str::FromStr>(&self, k: &str) -> T
    where
        T::Err: std::fmt::Debug,
    {
        self.get(k).parse().unwrap()
    }
}

fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        vec![]
    } else {
        hex::decode(s).unwrap()
    }
}

fn lines() -> impl Iterator<Item = Line<'static>> {
    VECTORS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let mut it = l.split(' ');
            let kind = it.next().unwrap();
            let kv = it.map(|t| t.split_once('=').unwrap()).collect();
            Line { kind, kv }
        })
}

/// `dataByte` in `golden/main.go`.
fn data_byte(off: u64, salt: u64) -> u8 {
    ((off.wrapping_add(salt)).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as u8
}

fn stream_data(off: u64, n: u64, salt: u64) -> Vec<u8> {
    (0..n).map(|i| data_byte(off + i, salt)).collect()
}

fn ms(n: i64) -> Time {
    Time::from_unix_millis(n)
}

fn config() -> ReaderConfig {
    ReaderConfig {
        chunk_size: 16,
        integrity_hash_size: 32,
        integrity_signature_size: 64,
    }
}

fn encode(m: &Message<'_>) -> Vec<u8> {
    let mut b = vec![0u8; m.encoded_len()];
    assert_eq!(m.encode(&mut b), Ok(b.len()));
    b
}

#[test]
fn codec_messages() {
    let sig64: Vec<u8> = (0..64).map(|i| (i * 3) as u8).collect();
    let hash32: Vec<u8> = (0..32).map(|i| 0xff - i as u8).collect();
    let id: Vec<u8> = (0..32).map(|i| 0xa0 + i as u8).collect();
    let ff32 = [0xffu8; 32];
    let d1 = stream_data(0, 16, 1);
    let d2 = stream_data(0, 16, 2);

    let all_options = [
        ProtocolOption::SwarmIdentifier(&id),
        ProtocolOption::Version(2),
        ProtocolOption::MinimumVersion(2),
        ProtocolOption::LiveWindow(1 << 16),
        ProtocolOption::ChunkSize(1024),
        ProtocolOption::ContentIntegrityProtectionMethod(2),
        ProtocolOption::MerkleHashTreeFunction(4),
        ProtocolOption::LiveSignatureAlgorithm(2),
        ProtocolOption::ChunksPerSignature(64),
        ProtocolOption::StreamCount(16),
        ProtocolOption::Epoch(EpochOption::new(ms(1_700_000_000_123), &sig64).unwrap()),
    ];
    let odd_options = [
        ProtocolOption::StreamCount(0xffff),
        ProtocolOption::ChunkSize(0xffff_ffff),
        ProtocolOption::SwarmIdentifier(&[]),
        ProtocolOption::Version(0xff),
        ProtocolOption::Version(1),
        ProtocolOption::Epoch(EpochOption::new(ms(-5), &sig64).unwrap()),
    ];

    let expected: Vec<(&str, Message<'_>)> = vec![
        (
            "handshake_full",
            Message::Handshake(Handshake {
                channel_id: 0x0102_0304,
                options: ProtocolOptions::from_slice(&all_options),
            }),
        ),
        (
            "handshake_empty",
            Message::Handshake(Handshake {
                channel_id: 0,
                options: ProtocolOptions::from_slice(&[]),
            }),
        ),
        (
            "handshake_odd",
            Message::Handshake(Handshake {
                channel_id: 0xffff_ffff,
                options: ProtocolOptions::from_slice(&odd_options),
            }),
        ),
        ("restart", Message::Restart),
        (
            "data",
            Message::Data(Data {
                bin: Bin(21),
                timestamp: ms(1234),
                data: &ff32,
            }),
        ),
        (
            "data_base",
            Message::Data(Data {
                bin: Bin(6),
                timestamp: ms(1_700_000_000_000),
                data: &d1,
            }),
        ),
        (
            "data_neg_time",
            Message::Data(Data {
                bin: Bin(0),
                timestamp: ms(-1),
                data: &d2,
            }),
        ),
        (
            "ack",
            Message::Ack(Ack {
                bin: Bin(22),
                delay_sample: Duration::from_millis(1234),
            }),
        ),
        (
            "ack_neg",
            Message::Ack(Ack {
                bin: Bin(1 << 40),
                delay_sample: Duration::from_millis(-3),
            }),
        ),
        ("have", Message::Have(Have { bin: Bin(1234) })),
        ("have_big", Message::Have(Have { bin: Bin::ALL })),
        ("have_none", Message::Have(Have { bin: Bin::NONE })),
        (
            "integrity",
            Message::Integrity(Integrity {
                bin: Bin(22),
                hash: &hash32,
            }),
        ),
        (
            "signed_integrity",
            Message::SignedIntegrity(SignedIntegrity {
                bin: Bin(15),
                timestamp: ms(1_700_000_000_456),
                signature: &sig64,
            }),
        ),
        (
            "request",
            Message::Request(Request {
                bin: Bin(1234),
                timestamp: ms(1234),
            }),
        ),
        ("cancel", Message::Cancel(Cancel { bin: Bin(1234) })),
        ("choke", Message::Choke),
        ("unchoke", Message::Unchoke),
        ("ping", Message::Ping(Ping { nonce: 1234 })),
        (
            "pong",
            Message::Pong(Pong {
                nonce: 0xfedc_ba98_7654_3210,
                delay: 77,
            }),
        ),
        (
            "stream_request",
            Message::StreamRequest(StreamRequest {
                stream: 16,
                bin: Bin(1234),
            }),
        ),
        (
            "stream_cancel",
            Message::StreamCancel(StreamCancel { stream: 16 }),
        ),
        (
            "stream_open",
            Message::StreamOpen(StreamOpen {
                stream: 0xffff,
                bin: Bin(0),
            }),
        ),
        (
            "stream_close",
            Message::StreamClose(StreamClose { stream: 3 }),
        ),
        ("end", Message::End),
    ];

    let msgs: BTreeMap<&str, Vec<u8>> = lines()
        .filter(|l| l.kind == "msg")
        .map(|l| (l.get("name"), l.hex("hex")))
        .collect();
    assert_eq!(msgs.len(), expected.len());

    for (name, want) in &expected {
        let go = &msgs[name];
        // Rust encodes what Go encoded
        assert_eq!(&encode(want), go, "encode {name}");
        // and decodes Go's bytes to the same message
        let (got, n) = Message::decode(&config(), go).unwrap();
        assert_eq!(n, go.len(), "decode length {name}");
        assert_eq!(&got, want, "decode {name}");
        // decode then encode is the identity
        assert_eq!(&encode(&got), go, "re-encode {name}");
    }

    // a payload of every message, ended by End
    let payload = lines().find(|l| l.kind == "payload").unwrap().hex("hex");
    let decoded: Vec<Message<'_>> = Reader::new(config(), &payload)
        .map(Result::unwrap)
        .collect();
    let want: Vec<Message<'_>> = expected
        .iter()
        .map(|x| x.1)
        .filter(|m| *m != Message::End)
        .collect();
    assert_eq!(decoded, want);
    let mut buf = vec![0u8; payload.len()];
    let mut w = CodecWriter::new(&mut buf);
    for m in decoded.iter().chain([&Message::End]) {
        w.write(m).unwrap();
    }
    assert_eq!(w.as_bytes(), payload);
}

#[test]
fn channel_headers_and_frames() {
    for l in lines().filter(|l| l.kind == "header") {
        let h = ChannelHeader {
            channel: l.num("channel"),
            length: l.num("length"),
        };
        let go = l.hex("hex");
        let mut b = vec![0u8; h.byte_len()];
        assert_eq!(h.encode(&mut b), Ok(go.len()));
        assert_eq!(b, go);
        assert_eq!(ChannelHeader::decode(&go), Ok((h, go.len())));
    }

    let frame = lines().find(|l| l.kind == "frame").unwrap().hex("hex");
    let want = [
        (
            0u64,
            vec![
                Message::Have(Have { bin: Bin(7) }),
                Message::Ping(Ping { nonce: 9 }),
            ],
        ),
        (
            300,
            vec![
                Message::Choke,
                Message::Request(Request {
                    bin: Bin(3),
                    timestamp: ms(5),
                }),
            ],
        ),
    ];
    let got: Vec<_> = FrameReader::new(&frame)
        .map(|c| {
            let (h, p) = c.unwrap();
            (
                h.channel,
                Reader::new(config(), p)
                    .map(Result::unwrap)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(got, want);

    let mut out = [0u8; 64];
    let mut off = 0;
    for (channel, msgs) in &want {
        let mut cw = ChannelWriter::new(&mut out[off..], *channel);
        for m in msgs {
            cw.write(m).unwrap();
        }
        off += cw.finish();
    }
    assert_eq!(out[..off], frame);
}

#[test]
fn merkle_digests() {
    let mut n = 0;
    for l in lines().filter(|l| l.kind == "merkle") {
        let f = MerkleHashTreeFunction(l.num("fn"));
        let cs: usize = l.num("cs");
        let root = Bin(l.num("root"));
        let mut h = f.hasher().unwrap();
        assert_eq!(h.size(), l.num::<usize>("size"));
        assert_eq!(f.hash_size(), Some(h.size()));

        let data = stream_data(0, root.base_length() * cs as u64, u64::from(f.0));
        let mut t = Tree::new(root, cs, h.size());
        t.fill(&mut h, root, &data).unwrap();
        let mut digests = vec![];
        for b in root.base_left().0..=root.base_right().0 {
            digests.extend_from_slice(t.get(Bin(b)).unwrap());
        }
        assert_eq!(
            hex::encode(digests),
            l.get("digests"),
            "fn {} root {root}",
            f.0
        );
        n += 1;
    }
    assert_eq!(n, 40);
    assert_eq!(MerkleHashTreeFunction::MD5.hasher().map(|h| h.size()), None);
}

/// Records encoded messages, like the generator's `encoder`.
#[derive(Default)]
struct Encoded(Vec<u8>);

impl MessageWriter for Encoded {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, codec::Error> {
        let b = encode(m);
        self.0.extend_from_slice(&b);
        Ok(b.len())
    }
}

/// Compares an error to Go's, whose spaces the generator made underscores
/// (Go also wraps some errors with a prefix).
fn assert_go_err(go: &str, rust: &dyn std::fmt::Display, what: &str) {
    let go = go.replace('_', " ");
    assert!(
        go.ends_with(&rust.to_string()),
        "{what}: go {go:?}, rust {rust}"
    );
}

struct Scenario<'a> {
    lines: Vec<Line<'a>>,
}

fn scenarios() -> Vec<Scenario<'static>> {
    let mut out = vec![];
    let mut cur: Option<Vec<Line<'static>>> = None;
    for l in lines() {
        match l.kind {
            "writer" => cur = Some(vec![l]),
            "end" => out.push(Scenario {
                lines: cur.take().unwrap(),
            }),
            _ => {
                if let Some(c) = &mut cur {
                    c.push(l);
                }
            }
        }
    }
    out
}

/// Replays each Go writer scenario through the Rust `Writer`.
#[test]
fn writer_matches_go() {
    let scenarios = scenarios();
    assert_eq!(scenarios.len(), 12);
    let mut checked = BTreeMap::<&str, usize>::new();

    for sc in &scenarios {
        let head = &sc.lines[0];
        let name = head.get("name");
        let seed: [u8; 32] = head.hex("seed").try_into().unwrap();
        let salt: u64 = head.num("salt");
        let key = strims_crypto::key_from_seed(&seed);
        let options = SwarmOptions {
            chunk_size: head.num("cs"),
            chunks_per_signature: head.num("cps"),
            live_window: head.num("lw"),
            stream_count: head.num("sc"),
            integrity: VerifierOptions {
                protection_method: ProtectionMethod(head.num("pm")),
                merkle_hash_tree_function: MerkleHashTreeFunction(head.num("hf")),
                live_signature_algorithm: LiveSignatureAlgorithm(head.num("sa")),
            },
            ..Default::default()
        };

        let epoch = sc.lines.iter().find(|l| l.kind == "epoch").unwrap();
        let epoch_ts = Time::from_unix_nanos(epoch.num("ts"));
        let (mut w, mut s) = Writer::new(
            WriterOptions {
                swarm_options: options.clone(),
                key: key.clone(),
            },
            epoch_ts,
        )
        .unwrap();
        assert_eq!(
            s.epoch().value(),
            (epoch_ts, &epoch.hex("sig")[..]),
            "{name}: epoch"
        );

        let mut off = 0u64;
        let mut now = epoch_ts;
        for l in &sc.lines[1..] {
            match l.kind {
                "uri" => assert_eq!(s.uri().to_string(), l.get("str"), "{name}: uri"),
                "write" => {
                    let n: u64 = l.num("len");
                    if let Ok(t) = l.get("ts").parse::<i64>() {
                        now = Time::from_unix_nanos(t);
                    }
                    let d = stream_data(off, n, salt);
                    assert_eq!(w.write(&mut s, now, &d), Ok(d.len()));
                    off += n;
                }
                "phase" => {
                    assert_eq!(s.store().tail(), Bin(l.num("tail")), "{name}: tail");
                    assert_eq!(s.store().next(), Bin(l.num("next")), "{name}: next");
                }
                "data" => {
                    let b = Bin(l.num("bin"));
                    let mut e = Encoded::default();
                    match (s.write_data(b, ms(1_700_000_000_999), &mut e), l.opt("hex")) {
                        (Ok(_), Some(h)) => assert_eq!(e.0, unhex(h), "{name}: data {b}"),
                        (Err(err), None) => assert_go_err(l.get("err"), &err, name),
                        (r, _) => panic!("{name}: data {b}: rust {r:?}, go {:?}", l.kv),
                    }
                    *checked.entry("data").or_default() += 1;
                }
                "integrity" => {
                    let b = Bin(l.num("bin"));
                    let mut have = Map::new();
                    if l.get("have") != "-" {
                        for h in l.get("have").split(',') {
                            have.set(Bin(h.parse().unwrap()));
                        }
                    }
                    let mut e = Encoded::default();
                    match (s.write_integrity(b, &have, &mut e), l.opt("hex")) {
                        (Ok(_), Some(h)) => {
                            assert_eq!(
                                e.0,
                                unhex(h),
                                "{name}: integrity {b} have {}",
                                l.get("have")
                            )
                        }
                        (Err(err), None) => assert_go_err(l.get("err"), &err, name),
                        (r, _) => panic!("{name}: integrity {b}: rust {r:?}, go {:?}", l.kv),
                    }
                    *checked.entry("integrity").or_default() += 1;
                }
                "cache" => {
                    match (s.export_cache(), l.opt("hex")) {
                        (Ok(c), Some(h)) => {
                            assert_eq!(hex::encode(c.encode_to_vec()), h, "{name}: cache");
                            import_go_cache(name, &key, &options, &unhex(h), off, salt);
                        }
                        (Err(err), None) => assert_go_err(l.get("err"), &err, name),
                        (r, _) => panic!("{name}: cache: rust {r:?}, go {:?}", l.kv),
                    }
                    *checked.entry("cache").or_default() += 1;
                }
                _ => {}
            }
        }
    }
    // every vector was checked
    for kind in ["data", "integrity", "cache"] {
        assert_eq!(
            checked[kind],
            lines().filter(|l| l.kind == kind).count(),
            "{kind}"
        );
    }
    assert_eq!(checked["cache"], 16);
}

/// Imports a cache Go exported into a fresh Rust swarm, exports it again
/// and reads the stream back.
fn import_go_cache(
    name: &str,
    key: &strims_proto::types::Key,
    options: &SwarmOptions,
    go: &[u8],
    len: u64,
    salt: u64,
) {
    let c = Cache::decode(go).unwrap();
    let mut s = Swarm::new(SwarmId::new(&key.public), options.clone()).unwrap();
    s.import_cache(&c)
        .unwrap_or_else(|e| panic!("{name}: import: {e}"));
    let again = s.export_cache().unwrap();
    assert_eq!(again.encode_to_vec(), go, "{name}: cache re-export");

    let mut r = s.reader();
    let mut out = vec![0u8; len as usize + 1];
    let mut n = 0;
    loop {
        match s.read(&mut r, &mut out[n..]) {
            Ok(m) => n += m,
            Err(store::Error::WouldBlock) => break,
            Err(e) => panic!("{name}: read: {e}"),
        }
    }
    // the cache holds whole integrity units; the rest is still staged
    let unit = if options.integrity.protection_method == ProtectionMethod::MERKLE_TREE {
        options.chunk_size * options.chunks_per_signature
    } else {
        options.chunk_size
    } as u64;
    assert_eq!(n as u64, len / unit * unit, "{name}: cached bytes");
    assert_eq!(
        out[..n],
        stream_data(0, n as u64, salt),
        "{name}: cached data"
    );
}

#[test]
fn swarm_ids() {
    let mut n = 0;
    for l in lines() {
        match l.kind {
            "swarmid" => {
                assert_eq!(SwarmId::new(&l.hex("hex")).to_string(), l.get("str"));
                n += 1;
            }
            "decodeid" => {
                let s = String::from_utf8(l.hex("str")).unwrap();
                match (SwarmId::decode(&s), l.opt("hex")) {
                    (Ok(id), Some(h)) => assert_eq!(id.as_bytes(), unhex(h), "{s:?}"),
                    (Err(_), None) => {}
                    (r, _) => panic!("{s:?}: rust {r:?}, go {:?}", l.kv),
                }
                n += 1;
            }
            _ => {}
        }
    }
    assert_eq!(n, 20);
}

#[test]
fn uris() {
    let id = SwarmId::new(&(0..32).map(|i| 0xa0 + i as u8).collect::<Vec<_>>());
    let strings: Vec<&str> = lines()
        .filter(|l| l.kind == "uristring")
        .map(|l| l.get("str"))
        .collect();
    let mut partial = UriOptions::default();
    partial.insert(ProtocolOptionType::StreamCount, 32);
    partial.insert(ProtocolOptionType::ChunkSize, 1 << 31);
    assert_eq!(
        strings,
        vec![
            Uri::new(id.clone(), SwarmOptions::default_options().uri_options()).to_string(),
            Uri::new(id, UriOptions::default()).to_string(),
            Uri::new(SwarmId::default(), partial).to_string(),
        ]
    );

    let mut n = 0;
    for l in lines().filter(|l| l.kind == "parseuri") {
        let s = String::from_utf8(l.hex("str")).unwrap();
        match (Uri::parse(&s), l.opt("id")) {
            (Ok(u), Some(id)) => {
                assert_eq!(u.id.as_bytes(), unhex(id), "{s:?}");
                let opts: Vec<String> = u
                    .options
                    .0
                    .iter()
                    .map(|(k, v)| format!("{}:{v}", k.as_u8()))
                    .collect();
                let opts = if opts.is_empty() {
                    "-".to_string()
                } else {
                    opts.join(",")
                };
                assert_eq!(opts, l.get("opts"), "{s:?}");
            }
            (Err(_), None) => {}
            (r, _) => panic!("{s:?}: rust {r:?}, go {:?}", l.kv),
        }
        n += 1;
    }
    assert_eq!(n, 37);
}
