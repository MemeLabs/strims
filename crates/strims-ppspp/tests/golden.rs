// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.txt`):
//!
//! - every codec message, a multi-message payload, channel headers and a
//!   frame: the pinned bytes decode to the expected messages and encode
//!   back byte for byte;
//! - Merkle digests of every bin for each supported hash function;
//! - writer scenarios for Merkle (all hash functions, ed25519 and no
//!   signatures), sign-all and no integrity: the `Writer` replays the
//!   scenario's writes at their times, and its epochs, `Data` messages,
//!   integrity messages (with several peer have maps) and swarm caches are
//!   pinned; each cache must import and export again unchanged;
//! - swarm id and URI strings and parsing.
//!
//! `STRIMS_GOLDEN=update` rewrites the epoch signatures, integrity messages
//! and caches from the code (see `strims-golden`). The other vectors are
//! asserted, never rewritten.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::str::FromStr;

use prost::Message as _;
use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Map};
use strims_golden::Golden;
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

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.txt"))
}

/// Byte strings in the vectors are hex, with `-` for none.
fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        vec![]
    } else {
        hex::decode(s).unwrap()
    }
}

fn hx(b: &[u8]) -> String {
    if b.is_empty() {
        "-".to_owned()
    } else {
        hex::encode(b)
    }
}

fn bytes(g: &Golden, e: usize, k: &str) -> Vec<u8> {
    unhex(g.str(e, k))
}

fn num<T: FromStr>(g: &Golden, e: usize, k: &str) -> T
where
    T::Err: Debug,
{
    g.str(e, k).parse().unwrap()
}

/// The payload byte at stream offset `off` in the writer scenarios.
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

    let g = open();
    let msgs: BTreeMap<&str, Vec<u8>> = g
        .all("msg")
        .into_iter()
        .map(|e| (g.str(e, "name"), bytes(&g, e, "hex")))
        .collect();
    assert_eq!(msgs.len(), expected.len());

    for (name, want) in &expected {
        let pinned = &msgs[name];
        // encodes to the pinned bytes
        assert_eq!(&encode(want), pinned, "encode {name}");
        // and decodes them to the same message
        let (got, n) = Message::decode(&config(), pinned).unwrap();
        assert_eq!(n, pinned.len(), "decode length {name}");
        assert_eq!(&got, want, "decode {name}");
        // decode then encode is the identity
        assert_eq!(&encode(&got), pinned, "re-encode {name}");
    }

    // a payload of every message, ended by End
    let payload = bytes(&g, g.all("payload")[0], "hex");
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
    let g = open();
    for l in g.all("header") {
        let h = ChannelHeader {
            channel: num(&g, l, "channel"),
            length: num(&g, l, "length"),
        };
        let pinned = bytes(&g, l, "hex");
        let mut b = vec![0u8; h.byte_len()];
        assert_eq!(h.encode(&mut b), Ok(pinned.len()));
        assert_eq!(b, pinned);
        assert_eq!(ChannelHeader::decode(&pinned), Ok((h, pinned.len())));
    }

    let frame = bytes(&g, g.all("frame")[0], "hex");
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
    let g = open();
    let mut n = 0;
    for l in g.all("merkle") {
        let f = MerkleHashTreeFunction(num(&g, l, "fn"));
        let cs: usize = num(&g, l, "cs");
        let root = Bin(num(&g, l, "root"));
        let mut h = f.hasher().unwrap();
        assert_eq!(h.size(), num::<usize>(&g, l, "size"));
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
            g.str(l, "digests"),
            "fn {} root {root}",
            f.0
        );
        n += 1;
    }
    assert_eq!(n, 40);
    assert_eq!(MerkleHashTreeFunction::MD5.hasher().map(|h| h.size()), None);
}

/// Collects the encoding of every message written to it.
#[derive(Default)]
struct Encoded(Vec<u8>);

impl MessageWriter for Encoded {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, codec::Error> {
        let b = encode(m);
        self.0.extend_from_slice(&b);
        Ok(b.len())
    }
}

/// Compares an error to the pinned one, whose spaces the vectors write as
/// underscores and which may carry a prefix.
fn assert_pinned_err(pinned: &str, err: &dyn std::fmt::Display, what: &str) {
    let pinned = pinned.replace('_', " ");
    assert!(
        pinned.ends_with(&err.to_string()),
        "{what}: pinned {pinned:?}, got {err}"
    );
}

/// The entries of each writer scenario, from its `writer` entry up to its
/// `end`.
fn scenarios(g: &Golden) -> Vec<Vec<usize>> {
    let mut out = vec![];
    let mut cur: Option<Vec<usize>> = None;
    for e in g.prefixed("") {
        match g.name(e) {
            "writer" => cur = Some(vec![e]),
            "end" => out.push(cur.take().unwrap()),
            _ => {
                if let Some(c) = &mut cur {
                    c.push(e);
                }
            }
        }
    }
    out
}

/// Replays each writer scenario through the `Writer`. A `write len=<n>
/// ts=<t>` entry writes the next `n` stream bytes at `t` Unix nanoseconds,
/// or at the previous write's time when `t` isn't a number.
#[test]
fn writer_matches_vectors() {
    let mut g = open();
    let scenarios = scenarios(&g);
    assert_eq!(scenarios.len(), 12);
    let mut checked = BTreeMap::<&str, usize>::new();

    for sc in &scenarios {
        let head = sc[0];
        let name = g.str(head, "name").to_owned();
        let name = name.as_str();
        let seed: [u8; 32] = bytes(&g, head, "seed").try_into().unwrap();
        let salt: u64 = num(&g, head, "salt");
        let key = strims_crypto::key_from_seed(&seed);
        let options = SwarmOptions {
            chunk_size: num(&g, head, "cs"),
            chunks_per_signature: num(&g, head, "cps"),
            live_window: num(&g, head, "lw"),
            stream_count: num(&g, head, "sc"),
            integrity: VerifierOptions {
                protection_method: ProtectionMethod(num(&g, head, "pm")),
                merkle_hash_tree_function: MerkleHashTreeFunction(num(&g, head, "hf")),
                live_signature_algorithm: LiveSignatureAlgorithm(num(&g, head, "sa")),
            },
            ..Default::default()
        };

        let epoch = *sc.iter().find(|&&l| g.name(l) == "epoch").unwrap();
        let epoch_ts = Time::from_unix_nanos(num(&g, epoch, "ts"));
        let (mut w, mut s) = Writer::new(
            WriterOptions {
                swarm_options: options.clone(),
                key: key.clone(),
            },
            epoch_ts,
        )
        .unwrap();
        let (ts, sig) = s.epoch().value();
        assert_eq!(ts, epoch_ts, "{name}: epoch");
        g.check(epoch, "sig", hx(sig));

        let mut off = 0u64;
        let mut now = epoch_ts;
        for &l in &sc[1..] {
            match g.name(l) {
                "uri" => assert_eq!(s.uri().to_string(), g.str(l, "str"), "{name}: uri"),
                "write" => {
                    let n: u64 = num(&g, l, "len");
                    if let Ok(t) = g.str(l, "ts").parse::<i64>() {
                        now = Time::from_unix_nanos(t);
                    }
                    let d = stream_data(off, n, salt);
                    assert_eq!(w.write(&mut s, now, &d), Ok(d.len()));
                    off += n;
                }
                "phase" => {
                    assert_eq!(s.store().tail(), Bin(num(&g, l, "tail")), "{name}: tail");
                    assert_eq!(s.store().next(), Bin(num(&g, l, "next")), "{name}: next");
                }
                "data" => {
                    let b = Bin(num(&g, l, "bin"));
                    let mut e = Encoded::default();
                    match (
                        s.write_data(b, ms(1_700_000_000_999), &mut e),
                        g.opt(l, "hex"),
                    ) {
                        (Ok(_), Some(h)) => assert_eq!(e.0, unhex(h), "{name}: data {b}"),
                        (Err(err), None) => assert_pinned_err(g.str(l, "err"), &err, name),
                        (r, _) => {
                            panic!("{name}: data {b}: got {r:?}, pinned {:?}", g.opt(l, "err"))
                        }
                    }
                    *checked.entry("data").or_default() += 1;
                }
                "integrity" => {
                    let b = Bin(num(&g, l, "bin"));
                    let mut have = Map::new();
                    if g.str(l, "have") != "-" {
                        for h in g.str(l, "have").split(',') {
                            have.set(Bin(h.parse().unwrap()));
                        }
                    }
                    let mut e = Encoded::default();
                    match (
                        s.write_integrity(b, &have, &mut e),
                        g.opt(l, "hex").is_some(),
                    ) {
                        (Ok(_), true) => g.check(l, "hex", hx(&e.0)),
                        (Err(err), false) => assert_pinned_err(g.str(l, "err"), &err, name),
                        (r, _) => panic!(
                            "{name}: integrity {b}: got {r:?}, pinned {:?}",
                            g.opt(l, "err")
                        ),
                    }
                    *checked.entry("integrity").or_default() += 1;
                }
                "cache" => {
                    match (s.export_cache(), g.opt(l, "hex").is_some()) {
                        (Ok(c), true) => {
                            let c = c.encode_to_vec();
                            g.check(l, "hex", hx(&c));
                            reimport_cache(name, &key, &options, &c, off, salt);
                        }
                        (Err(err), false) => assert_pinned_err(g.str(l, "err"), &err, name),
                        (r, _) => panic!("{name}: cache: got {r:?}, pinned {:?}", g.opt(l, "err")),
                    }
                    *checked.entry("cache").or_default() += 1;
                }
                _ => {}
            }
        }
    }
    // every vector was checked
    for kind in ["data", "integrity", "cache"] {
        assert_eq!(checked[kind], g.all(kind).len(), "{kind}");
    }
    assert_eq!(checked["cache"], 16);
}

/// Imports an exported cache into a fresh swarm, exports it again and
/// reads the stream back.
fn reimport_cache(
    name: &str,
    key: &strims_proto::types::Key,
    options: &SwarmOptions,
    exported: &[u8],
    len: u64,
    salt: u64,
) {
    let c = Cache::decode(exported).unwrap();
    let mut s = Swarm::new(SwarmId::new(&key.public), options.clone()).unwrap();
    s.import_cache(&c)
        .unwrap_or_else(|e| panic!("{name}: import: {e}"));
    let again = s.export_cache().unwrap();
    assert_eq!(again.encode_to_vec(), exported, "{name}: cache re-export");

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
    let g = open();
    let mut n = 0;
    for l in g.all("swarmid") {
        assert_eq!(
            SwarmId::new(&bytes(&g, l, "hex")).to_string(),
            g.str(l, "str")
        );
        n += 1;
    }
    for l in g.all("decodeid") {
        let s = String::from_utf8(bytes(&g, l, "str")).unwrap();
        match (SwarmId::decode(&s), g.opt(l, "hex")) {
            (Ok(id), Some(h)) => assert_eq!(id.as_bytes(), unhex(h), "{s:?}"),
            (Err(_), None) => {}
            (r, _) => panic!("{s:?}: got {r:?}, pinned {:?}", g.opt(l, "hex")),
        }
        n += 1;
    }
    assert_eq!(n, 20);
}

#[test]
fn uris() {
    let g = open();
    let id = SwarmId::new(&(0..32).map(|i| 0xa0 + i as u8).collect::<Vec<_>>());
    let strings: Vec<&str> = g
        .all("uristring")
        .into_iter()
        .map(|l| g.str(l, "str"))
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
    for l in g.all("parseuri") {
        let s = String::from_utf8(bytes(&g, l, "str")).unwrap();
        match (Uri::parse(&s), g.opt(l, "id")) {
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
                assert_eq!(opts, g.str(l, "opts"), "{s:?}");
            }
            (Err(_), None) => {}
            (r, _) => panic!("{s:?}: got {r:?}, pinned {:?}", g.opt(l, "id")),
        }
        n += 1;
    }
    assert_eq!(n, 37);
}
