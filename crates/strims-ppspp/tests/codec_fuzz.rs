// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Peer input never panics: random and mutated frames go through the
//! decoder, and every decoded message goes into a receiving swarm's
//! verifier, store and send paths, as the channel layer would feed them.
//! Debug builds check arithmetic overflow, so an overflow is a failure too.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Map};
use strims_ppspp::codec::{
    ChannelWriter, FrameReader, Message, MessageWriter, Reader, ReaderConfig, Writer as CodecWriter,
};
use strims_ppspp::integrity::{
    ChannelVerifier, LiveSignatureAlgorithm, ProtectionMethod, VerifierOptions,
};
use strims_ppspp::{Swarm, SwarmId, SwarmOptions, Writer, WriterOptions};

/// 1024 cases, or more with `PROPTEST_CASES`.
fn config() -> Config {
    Config {
        cases: Config::default().cases.max(1024),
        rng_seed: RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..Config::default()
    }
}

fn options(pm: ProtectionMethod) -> SwarmOptions {
    SwarmOptions {
        chunk_size: 32,
        chunks_per_signature: 4,
        live_window: 16,
        integrity: VerifierOptions {
            protection_method: pm,
            ..Default::default()
        },
        ..Default::default()
    }
}

const METHODS: [ProtectionMethod; 3] = [
    ProtectionMethod::MERKLE_TREE,
    ProtectionMethod::SIGN_ALL,
    ProtectionMethod::NONE,
];

fn key() -> strims_proto::types::Key {
    strims_crypto::key_from_seed(&[42; 32])
}

/// Valid traffic to mutate: the handshake, then integrity and data for a
/// few bins, from a writer of each protection method. Built once.
fn corpus() -> &'static [Vec<u8>] {
    static CORPUS: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    CORPUS.get_or_init(build_corpus)
}

fn build_corpus() -> Vec<Vec<u8>> {
    let mut out = vec![];
    for pm in METHODS {
        let (mut w, mut s) = Writer::new(
            WriterOptions {
                swarm_options: options(pm),
                key: key(),
            },
            Time::from_unix_millis(1_000),
        )
        .unwrap();
        let data: Vec<u8> = (0..32 * 12).map(|i| (i * 13 % 256) as u8).collect();
        w.write(&mut s, Time::from_unix_millis(2_000), &data)
            .unwrap();

        for bins in [
            &[Bin(0)][..],
            &[Bin(2), Bin(5)],
            &[Bin(19), Bin(8)],
            &[Bin(3)],
        ] {
            let mut frame = vec![0u8; 2048];
            let mut cw = ChannelWriter::new(&mut frame, 7);
            cw.write(&Message::Handshake(s.handshake_options().handshake()))
                .unwrap();
            for &b in bins {
                let _ = s.write_integrity(b, &Map::new(), &mut cw);
                let _ = s.write_data(b, Time::from_unix_millis(3_000), &mut cw);
            }
            cw.write(&Message::Have(strims_ppspp::codec::Have { bin: Bin(7) }))
                .unwrap();
            let n = cw.finish();
            frame.truncate(n);
            out.push(frame);
        }
    }
    out
}

fn receivers() -> Vec<(Swarm, ChannelVerifier)> {
    METHODS
        .iter()
        .map(|&pm| {
            let mut s = Swarm::new(SwarmId::new(&key().public), options(pm)).unwrap();
            s.set_offset(Bin(0));
            let cv = s.channel_verifier();
            (s, cv)
        })
        .collect()
}

#[derive(Default)]
struct Sink(usize);

impl MessageWriter for Sink {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, strims_ppspp::codec::Error> {
        self.0 += m.encoded_len();
        Ok(m.encoded_len())
    }
}

/// What the channel layer does with each message, without the scheduler.
fn feed(s: &mut Swarm, cv: &mut ChannelVerifier, m: &Message<'_>) {
    match *m {
        Message::Handshake(h) => {
            let o = h.options.summary();
            if let Some(e) = o.epoch
                && let Ok(true) = s.sync_epoch(e.timestamp, e.signature)
            {
                s.reset();
                s.set_offset(Bin(0));
            }
            let _ = o.swarm_identifier.map(SwarmId::new);
        }
        Message::Integrity(i) => s.handle_integrity(cv, i.bin, i.hash),
        Message::SignedIntegrity(i) => {
            s.handle_signed_integrity(cv, i.bin, i.timestamp, i.signature)
        }
        Message::Data(d) => {
            let _ = s.handle_data(cv, d.bin, d.data);
        }
        // bins a peer asks for or announces reach the send paths
        Message::Request(strims_ppspp::codec::Request { bin, .. })
        | Message::Have(strims_ppspp::codec::Have { bin })
        | Message::Cancel(strims_ppspp::codec::Cancel { bin })
        | Message::Ack(strims_ppspp::codec::Ack { bin, .. }) => {
            let mut have = Map::new();
            if strims_ppspp::peer_bin_ok(bin, 16) {
                have.set(bin);
            }
            let _ = s.write_integrity(bin, &have, &mut Sink::default());
            let _ = s.write_data(bin, Time::EPOCH, &mut Sink::default());
            let _ = s.store().bins().filled_at(bin);
        }
        _ => {}
    }
    while s.poll_event().is_some() {}
}

fn check_message(cfg: &ReaderConfig, buf: &[u8]) {
    if let Ok((m, n)) = Message::decode(cfg, buf) {
        assert!(n <= buf.len());
        // re-encoding is stable: decoding what was encoded gives a
        // message that encodes to the same bytes. (Not always the same
        // value: a millisecond timestamp past ±292 years wraps when scaled
        // to nanoseconds, so it changes once on the way through.)
        let mut out = vec![0u8; m.encoded_len()];
        if m.encode(&mut out).is_ok() {
            let (again, n2) = Message::decode(cfg, &out).unwrap();
            assert_eq!(n2, out.len());
            let mut out2 = vec![0u8; again.encoded_len()];
            again.encode(&mut out2).unwrap();
            assert_eq!(out2, out, "{m:?}");
            if !matches!(
                m,
                Message::Data(_)
                    | Message::SignedIntegrity(_)
                    | Message::Request(_)
                    | Message::Ack(_)
                    | Message::Handshake(_)
            ) {
                assert_eq!(again, m);
            }
        }
    }
}

fn run(frame: &[u8], cfgs: &[ReaderConfig], rx: &mut [(Swarm, ChannelVerifier)]) {
    for cfg in cfgs {
        check_message(cfg, frame);
        let mut used = 0;
        let mut r = Reader::new(*cfg, frame);
        for m in r.by_ref().take(10_000) {
            let Ok(m) = m else { break };
            used += 1;
            let _ = m.byte_len();
        }
        assert!(r.position() <= frame.len());
        assert!(used <= frame.len());
    }

    for c in FrameReader::new(frame).take(10_000) {
        let Ok((_, payload)) = c else { break };
        for (s, cv) in rx.iter_mut() {
            for m in Reader::new(s.reader_config(), payload).take(10_000) {
                let Ok(m) = m else { break };
                feed(s, cv, &m);
            }
        }
    }
    // readers never panic either
    for (s, _) in rx.iter_mut() {
        let mut r = s.reader();
        let mut buf = [0u8; 64];
        for _ in 0..8 {
            if s.read(&mut r, &mut buf).is_err() {
                let _ = s.recover_reader(&mut r);
            }
        }
    }
}

fn cfgs() -> Vec<ReaderConfig> {
    let mut v = vec![];
    for chunk_size in [1, 16, 1024] {
        for integrity_hash_size in [0, 20, 32, 64] {
            for integrity_signature_size in [0, 64] {
                v.push(ReaderConfig {
                    chunk_size,
                    integrity_hash_size,
                    integrity_signature_size,
                });
            }
        }
    }
    v
}

#[test]
fn corpus_is_valid() {
    let mut rx = receivers();
    for frame in corpus() {
        run(frame, &cfgs(), &mut rx);
    }
    // the unmutated traffic verified and published something
    for (s, _) in &rx {
        assert!(!s.store().is_empty(), "{:?}", s.options().integrity);
    }
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

fn apply(frame: &mut Vec<u8>, m: &Mutation) {
    if frame.is_empty() {
        return;
    }
    let len = frame.len();
    match m {
        Mutation::Flip(p, x) => frame[p % len] ^= x,
        Mutation::Truncate(p) => frame.truncate(p % len),
        Mutation::Insert(p, b) => {
            let p = p % len;
            frame.splice(p..p, b.iter().copied());
        }
        Mutation::Overwrite(p, b) => {
            let p = p % len;
            for (i, x) in b.iter().enumerate() {
                if let Some(d) = frame.get_mut(p + i) {
                    *d = *x;
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn random_bytes_never_panic(frame in prop::collection::vec(any::<u8>(), 0..600)) {
        run(&frame, &cfgs(), &mut receivers());
    }

    #[test]
    fn mutated_traffic_never_panics(
        which in any::<usize>(),
        muts in prop::collection::vec(mutation(), 1..6),
    ) {
        let corpus = corpus();
        let mut frame = corpus[which % corpus.len()].clone();
        for m in &muts {
            apply(&mut frame, m);
        }
        run(&frame, &cfgs(), &mut receivers());
    }

    /// Payloads built from arbitrary messages: whatever the writer accepts,
    /// the reader gives back.
    #[test]
    fn written_payloads_read_back(
        bins in prop::collection::vec(any::<u64>(), 1..20),
        ts in any::<i64>(),
        stream in any::<u16>(),
    ) {
        let cfg = ReaderConfig { chunk_size: 0, integrity_hash_size: 0, integrity_signature_size: 0 };
        let t = Time::from_unix_millis(ts / 1_000_000);
        let ids: Vec<[u8; 4]> = bins.iter().map(|b| (*b as u32).to_be_bytes()).collect();
        let entries: Vec<strims_ppspp::codec::AnnounceEntry<'_>> = bins
            .iter()
            .zip(&ids)
            .map(|(&b, id)| strims_ppspp::codec::AnnounceEntry {
                stream,
                hits: b,
                late: b >> 7,
                misses: b.rotate_left(3),
                delay: Duration::from_millis((b % 100_000) as i64),
                parent: &id[..1 + (b % 4) as usize],
            })
            .collect();
        let msgs: Vec<Message<'_>> = bins.iter().enumerate().map(|(i, &b)| {
            let bin = Bin(b);
            match i % 7 {
                5 => Message::Announce(
                    strims_ppspp::codec::Announce::new(std::slice::from_ref(&entries[i])).unwrap(),
                ),
                0 => Message::Have(strims_ppspp::codec::Have { bin }),
                1 => Message::Request(strims_ppspp::codec::Request { bin, timestamp: t }),
                2 => Message::StreamOpen(strims_ppspp::codec::StreamOpen { stream, bin }),
                3 => Message::Cancel(strims_ppspp::codec::Cancel { bin }),
                4 => Message::Pong(strims_ppspp::codec::Pong { nonce: b, delay: b.rotate_left(7) }),
                _ => Message::Data(strims_ppspp::codec::Data { bin, timestamp: t, data: &[] }),
            }
        }).collect();
        let mut buf = vec![0u8; 1024];
        let mut w = CodecWriter::new(&mut buf);
        for m in &msgs {
            w.write(m).unwrap();
        }
        let n = w.len();
        let got: Vec<_> = Reader::new(cfg, &buf[..n]).map(Result::unwrap).collect();
        prop_assert_eq!(got, msgs);
    }
}

/// The epoch in a peer's handshake must verify against the swarm key.
#[test]
fn forged_epoch_is_rejected() {
    let mut rx = receivers();
    let (s, _) = &mut rx[0];
    let sig = LiveSignatureAlgorithm::ED25519
        .signer(&strims_crypto::key_from_seed(&[1; 32]).private)
        .unwrap()
        .sign(
            strims_crypto::Domain::PpsppEpoch,
            Time::from_unix_millis(5),
            &[],
        );
    assert!(
        s.sync_epoch(Time::from_unix_millis(5), sig.as_slice())
            .is_err()
    );
}
