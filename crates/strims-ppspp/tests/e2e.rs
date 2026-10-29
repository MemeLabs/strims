// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! End to end through the wire format, without a scheduler: a Rust
//! `Writer` publishes into a seeder swarm; the seeder sends the handshake,
//! integrity and data as encoded frames; a second swarm, acting as the
//! peer, decodes them, verifies, publishes, answers with HAVEs, and reads
//! the original byte stream back through its non-blocking reader. Tampered
//! chunks, signatures and uncle hashes are rejected and the transfer
//! recovers.
//!
//! Then with the scheduler, in [`mesh`]: runners in a full mesh with a
//! link latency and uplinks of fixed rate, where a viewer that joins late
//! moves its reader back a GOP's worth of chunks ([`Runner::rewind`]).
//! The tests check what it reads and that the other viewers' lag holds,
//! and print the time the rewind took and their lag with and without it.
//!
//! [`Runner::rewind`]: strims_ppspp::Runner::rewind

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Map};
use strims_ppspp::codec::{
    ChannelHeader, ChannelWriter, FrameReader, Have, Message, MessageWriter, Reader,
};
use strims_ppspp::integrity::{
    self, ChannelVerifier, LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod,
    VerifierOptions,
};
use strims_ppspp::store;
use strims_ppspp::{Swarm, SwarmError, SwarmEvent, SwarmId, SwarmOptions, Writer, WriterOptions};

const MTU: usize = 1400;
const CHANNEL: u64 = 3;

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

fn options(pm: ProtectionMethod, sa: LiveSignatureAlgorithm) -> SwarmOptions {
    SwarmOptions {
        chunk_size: 128,
        chunks_per_signature: 8,
        live_window: 64,
        integrity: VerifierOptions {
            protection_method: pm,
            merkle_hash_tree_function: MerkleHashTreeFunction::BLAKE2B256,
            live_signature_algorithm: sa,
        },
        ..Default::default()
    }
}

fn stream(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect()
}

struct Seeder {
    writer: Writer,
    swarm: Swarm,
    /// What the seeder knows the peer has, from its HAVEs.
    peer_have: Map,
}

struct Peer {
    swarm: Swarm,
    verifier: ChannelVerifier,
    reader: store::BufferReader,
    received: Vec<u8>,
    published: Vec<Bin>,
    rejected: Vec<(Bin, SwarmError)>,
}

fn setup(pm: ProtectionMethod, sa: LiveSignatureAlgorithm) -> (Seeder, Peer) {
    let key = strims_crypto::key_from_seed(&[77; 32]);
    let (writer, swarm) = Writer::new(
        WriterOptions {
            swarm_options: options(pm, sa),
            key: key.clone(),
        },
        t(0),
    )
    .unwrap();
    let mut peer = Swarm::new(SwarmId::new(&key.public), options(pm, sa)).unwrap();
    let verifier = peer.channel_verifier();
    let reader = peer.reader();
    while peer.poll_event().is_some() {}
    (
        Seeder {
            writer,
            swarm,
            peer_have: Map::new(),
        },
        Peer {
            swarm: peer,
            verifier,
            reader,
            received: vec![],
            published: vec![],
            rejected: vec![],
        },
    )
}

impl Seeder {
    fn handshake_frame(&self) -> Vec<u8> {
        let mut frame = vec![0u8; MTU];
        let mut cw = ChannelWriter::new(&mut frame, CHANNEL);
        cw.write(&Message::Handshake(
            self.swarm.handshake_options().handshake(),
        ))
        .unwrap();
        let n = cw.finish();
        frame.truncate(n);
        frame
    }

    /// Integrity then data for `b`, as the scheduler's `WriteData` does.
    /// `None` if the bin doesn't fit the frame.
    fn data_frame(&self, b: Bin, now: Time) -> Option<Vec<u8>> {
        let mut frame = vec![0u8; MTU];
        let mut cw = ChannelWriter::new(&mut frame, CHANNEL);
        match self.swarm.write_integrity(b, &self.peer_have, &mut cw) {
            Ok(_) => {}
            Err(e) if e.is_not_enough_space() => return None,
            Err(e) => panic!("integrity for {b}: {e}"),
        }
        match self.swarm.write_data(b, now, &mut cw) {
            Ok(_) => {}
            Err(store::Error::Codec(_)) => return None,
            Err(e) => panic!("data for {b}: {e}"),
        }
        let n = cw.finish();
        frame.truncate(n);
        Some(frame)
    }

    fn handle_frame(&mut self, frame: &[u8]) {
        for c in FrameReader::new(frame) {
            let (_, payload) = c.unwrap();
            for m in Reader::new(self.swarm.reader_config(), payload) {
                if let Message::Have(h) = m.unwrap() {
                    self.peer_have.set(h.bin);
                }
            }
        }
    }
}

impl Peer {
    /// The channel layer's message dispatch, minus the scheduler. Returns
    /// the HAVE frame for the chunks it published.
    fn handle_frame(&mut self, frame: &[u8]) -> Vec<u8> {
        for c in FrameReader::new(frame) {
            let (h, payload) = c.unwrap();
            assert_eq!(h.channel, CHANNEL);
            for m in Reader::new(self.swarm.reader_config(), payload) {
                match m.unwrap() {
                    Message::Handshake(hs) => {
                        let o = hs.options.summary();
                        assert_eq!(o.swarm_identifier, Some(self.swarm.id().as_bytes()));
                        assert_eq!(o.chunk_size, Some(self.swarm.options().chunk_size as u32));
                        let e = o.epoch.unwrap();
                        if self.swarm.sync_epoch(e.timestamp, e.signature).unwrap() {
                            // A new epoch restarts the stream; the
                            // scheduler would then pick the read offset.
                            self.swarm.reset();
                            self.swarm.set_offset(Bin(0));
                        }
                    }
                    Message::Integrity(i) => {
                        self.swarm
                            .handle_integrity(&mut self.verifier, i.bin, i.hash)
                    }
                    Message::SignedIntegrity(i) => self.swarm.handle_signed_integrity(
                        &mut self.verifier,
                        i.bin,
                        i.timestamp,
                        i.signature,
                    ),
                    Message::Data(d) => {
                        match self.swarm.handle_data(&mut self.verifier, d.bin, d.data) {
                            Ok(p) => assert!(p.new),
                            Err(e) => self.rejected.push((d.bin, e)),
                        }
                    }
                    m => panic!("unexpected {m:?}"),
                }
            }
        }

        // drain the swarm's queue right away, as the runner must
        let mut haves = vec![0u8; MTU];
        let mut cw = ChannelWriter::new(&mut haves, CHANNEL);
        while let Some(e) = self.swarm.poll_event() {
            match e {
                SwarmEvent::Published(b) => {
                    self.published.push(b);
                    cw.write(&Message::Have(Have { bin: b })).unwrap();
                }
                SwarmEvent::Readable => self.read(),
                SwarmEvent::Reset => {}
            }
        }
        let n = cw.finish();
        haves.truncate(n);
        haves
    }

    fn read(&mut self) {
        let mut buf = [0u8; 100];
        loop {
            match self.swarm.read(&mut self.reader, &mut buf) {
                Ok(n) => self.received.extend_from_slice(&buf[..n]),
                // a reset reader stays reset; the test checks it
                Err(store::Error::WouldBlock | store::Error::StreamReset) => return,
                Err(e) => panic!("read: {e}"),
            }
        }
    }
}

fn exchange(seeder: &mut Seeder, peer: &mut Peer, frame: &[u8]) {
    let haves = peer.handle_frame(frame);
    seeder.handle_frame(&haves);
}

/// Sends `b`, splitting it into halves when it doesn't fit a frame, as
/// the scheduler's `WriteData` does.
fn send(seeder: &mut Seeder, peer: &mut Peer, b: Bin, now: Time) {
    match seeder.data_frame(b, now) {
        Some(f) => exchange(seeder, peer, &f),
        None => {
            assert!(!b.is_base(), "a chunk must fit a frame");
            send(seeder, peer, b.left(), now);
            send(seeder, peer, b.right(), now);
        }
    }
}

fn transfer(pm: ProtectionMethod, sa: LiveSignatureAlgorithm) {
    let (mut seeder, mut peer) = setup(pm, sa);
    let cs = 128;

    // before the handshake there is nothing to read
    assert_eq!(
        peer.swarm.read(&mut peer.reader, &mut [0; 8]),
        Err(store::Error::WouldBlock)
    );

    let data = stream(cs * 8 * 5 + 300);
    // write in odd pieces at increasing times
    for (i, piece) in data.chunks(777).enumerate() {
        seeder
            .writer
            .write(&mut seeder.swarm, t(10 + i as i64), piece)
            .unwrap();
    }
    let published: Vec<_> = core::iter::from_fn(|| seeder.swarm.poll_event()).collect();
    // chunks 0..31, merged into one bin
    assert!(
        published.contains(&SwarmEvent::Published(Bin(31))),
        "{published:?}"
    );

    let hs = seeder.handshake_frame();
    exchange(&mut seeder, &mut peer, &hs);

    // munro 0 chunk by chunk, out of order
    for o in [3u64, 0, 1, 7, 2, 6, 5, 4] {
        send(&mut seeder, &mut peer, Bin::new(0, o), t(100));
    }
    // munro 1 as two halves, munro 2 whole (split if needed)
    send(&mut seeder, &mut peer, Bin::new(2, 2), t(101));
    send(&mut seeder, &mut peer, Bin::new(2, 3), t(101));
    send(&mut seeder, &mut peer, Bin::new(3, 2), t(102));
    // munros 4 then 3, in pairs
    for o in (32..40u64).chain(24..32).step_by(2) {
        send(&mut seeder, &mut peer, Bin::new(1, o / 2), t(103));
    }

    assert!(peer.rejected.is_empty(), "{:?}", peer.rejected);
    peer.read();
    // the writer still stages the partial last munro (for merkle)
    let unit = if pm == ProtectionMethod::MERKLE_TREE {
        cs * 8
    } else {
        cs
    };
    let sent = data.len() / unit * unit;
    assert_eq!(peer.received.len(), sent.min(cs * 40));
    assert_eq!(peer.received, data[..peer.received.len()]);

    // the peer can now serve every chunk it has, with the same integrity
    for o in 0..40u64 {
        let b = Bin::new(0, o);
        let mut a = Recorded::default();
        let mut c = Recorded::default();
        seeder
            .swarm
            .write_integrity(b, &Map::new(), &mut a)
            .unwrap();
        peer.swarm.write_integrity(b, &Map::new(), &mut c).unwrap();
        assert_eq!(a.0, c.0, "integrity for {b}");
        let mut a = Recorded::default();
        let mut c = Recorded::default();
        seeder.swarm.write_data(b, t(0), &mut a).unwrap();
        peer.swarm.write_data(b, t(0), &mut c).unwrap();
        assert_eq!(a.0, c.0, "data for {b}");
    }

    // flushing pads the last unit; the peer gets the rest of the stream
    seeder.writer.flush(&mut seeder.swarm, t(200)).unwrap();
    while seeder.swarm.poll_event().is_some() {}
    let next = Bin::new(0, 40);
    let end = seeder.swarm.store().next();
    let mut b = next;
    while b < end {
        send(&mut seeder, &mut peer, b, t(201));
        b += 2;
    }
    peer.read();
    assert_eq!(peer.received[..data.len()], data);
    assert!(peer.received[data.len()..].iter().all(|&x| x == 0));
}

#[derive(Default)]
struct Recorded(Vec<u8>);

impl MessageWriter for Recorded {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, strims_ppspp::codec::Error> {
        let mut b = vec![0u8; m.encoded_len()];
        let n = m.encode(&mut b)?;
        self.0.extend_from_slice(&b);
        Ok(n)
    }
}

#[test]
fn merkle_ed25519() {
    transfer(
        ProtectionMethod::MERKLE_TREE,
        LiveSignatureAlgorithm::ED25519,
    );
}

#[test]
fn merkle_unsigned() {
    transfer(ProtectionMethod::MERKLE_TREE, LiveSignatureAlgorithm::NONE);
}

#[test]
fn sign_all_ed25519() {
    transfer(ProtectionMethod::SIGN_ALL, LiveSignatureAlgorithm::ED25519);
}

#[test]
fn no_integrity() {
    transfer(ProtectionMethod::NONE, LiveSignatureAlgorithm::NONE);
}

/// Offset of the last byte of the first message after the channel header.
fn first_message_end(frame: &[u8]) -> usize {
    let (_, hn) = ChannelHeader::decode(frame).unwrap();
    let cfg = strims_ppspp::codec::ReaderConfig {
        chunk_size: 128,
        integrity_hash_size: 32,
        integrity_signature_size: 64,
    };
    let (_, n) = Message::decode(&cfg, &frame[hn..]).unwrap();
    hn + n - 1
}

fn tamper_case(pm: ProtectionMethod) {
    let (mut seeder, mut peer) = setup(pm, LiveSignatureAlgorithm::ED25519);
    let data = stream(128 * 16);
    seeder.writer.write(&mut seeder.swarm, t(5), &data).unwrap();
    let hs = seeder.handshake_frame();
    exchange(&mut seeder, &mut peer, &hs);

    // a flipped data byte
    let b = Bin(4);
    let mut f = seeder.data_frame(b, t(6)).unwrap();
    let n = f.len();
    f[n - 3] ^= 0x10;
    exchange(&mut seeder, &mut peer, &f);
    let (rb, err) = peer.rejected.pop().unwrap();
    assert_eq!(rb, b);
    let want = match pm {
        ProtectionMethod::MERKLE_TREE => integrity::Error::InvalidSignature,
        _ => integrity::Error::InvalidChunkSignature,
    };
    assert_eq!(err, SwarmError::Integrity(want));
    assert!(peer.published.is_empty());
    assert!(peer.swarm.store().bins().is_empty());

    // a flipped signature byte
    let mut f = seeder.data_frame(b, t(6)).unwrap();
    let sig_end = first_message_end(&f);
    f[sig_end] ^= 0x01;
    exchange(&mut seeder, &mut peer, &f);
    assert_eq!(
        peer.rejected.pop().unwrap(),
        (b, SwarmError::Integrity(want))
    );

    // the untampered frame is accepted afterwards
    let f = seeder.data_frame(b, t(6)).unwrap();
    exchange(&mut seeder, &mut peer, &f);
    assert!(peer.rejected.is_empty(), "{:?}", peer.rejected);
    assert_eq!(peer.published, vec![b]);

    if pm == ProtectionMethod::MERKLE_TREE {
        // the peer has chunk 2, so chunk 4 comes with uncles 10 and 13
        // only; a bad uncle no longer matches the verified tree
        assert!(seeder.peer_have.filled_at(b));
        let b = Bin(8);
        let mut f = seeder.data_frame(b, t(7)).unwrap();
        let uncle_end = first_message_end(&f);
        f[uncle_end] ^= 0x01;
        exchange(&mut seeder, &mut peer, &f);
        assert_eq!(
            peer.rejected.pop().unwrap(),
            (
                b,
                SwarmError::Integrity(integrity::Error::Merkle(
                    strims_ppspp::merkle::Error::HashMismatch
                ))
            )
        );
        let f = seeder.data_frame(b, t(7)).unwrap();
        exchange(&mut seeder, &mut peer, &f);
        assert!(peer.rejected.is_empty());
    }

    // short data
    let mut f = seeder.data_frame(Bin(8), t(8)).unwrap();
    let n = f.len();
    f.truncate(n - 1);
    let (h, hn) = ChannelHeader::decode(&f).unwrap();
    let fixed = ChannelHeader {
        length: h.length - 1,
        ..h
    };
    fixed.encode(&mut f[..hn]).unwrap();
    exchange(&mut seeder, &mut peer, &f);
    assert_eq!(
        peer.rejected.pop().unwrap(),
        (Bin(8), SwarmError::Integrity(integrity::Error::DataLength))
    );
}

#[test]
fn merkle_rejects_tampering() {
    tamper_case(ProtectionMethod::MERKLE_TREE);
}

#[test]
fn sign_all_rejects_tampering() {
    tamper_case(ProtectionMethod::SIGN_ALL);
}

/// A writer reset starts a new epoch; the peer resets when it sees it and
/// its old reader reports the reset.
#[test]
fn epoch_reset() {
    let (mut seeder, mut peer) = setup(
        ProtectionMethod::MERKLE_TREE,
        LiveSignatureAlgorithm::ED25519,
    );
    let data = stream(128 * 8);
    seeder.writer.write(&mut seeder.swarm, t(1), &data).unwrap();
    let hs = seeder.handshake_frame();
    exchange(&mut seeder, &mut peer, &hs);
    send(&mut seeder, &mut peer, Bin(7), t(2));
    peer.read();
    assert_eq!(peer.received, data);

    seeder
        .writer
        .reset(&mut seeder.swarm, t(1) + Duration::SECOND)
        .unwrap();
    let events: Vec<_> = core::iter::from_fn(|| seeder.swarm.poll_event()).collect();
    assert!(events.contains(&SwarmEvent::Reset));
    let data2 = stream(128 * 9)[128..].to_vec();
    seeder
        .writer
        .write(&mut seeder.swarm, t(1_100), &data2)
        .unwrap();
    seeder.peer_have = Map::new();

    let hs = seeder.handshake_frame();
    let haves = peer.handle_frame(&hs);
    assert!(haves.is_empty());
    assert_eq!(
        peer.swarm.read(&mut peer.reader, &mut [0; 8]),
        Err(store::Error::StreamReset)
    );
    peer.reader = peer.swarm.reader();
    peer.received.clear();
    send(&mut seeder, &mut peer, Bin(7), t(1_200));
    peer.read();
    assert_eq!(peer.received, data2);

    // an old epoch is refused
    let mut old = Swarm::new(
        peer.swarm.id().clone(),
        options(
            ProtectionMethod::MERKLE_TREE,
            LiveSignatureAlgorithm::ED25519,
        ),
    )
    .unwrap();
    assert_eq!(
        old.sync_epoch(t(1) + Duration::SECOND, peer.swarm.epoch().value().1),
        Ok(true)
    );
    let (ts, sig) = (t(1), seeder.swarm.epoch().value().1.to_vec());
    assert!(old.sync_epoch(ts, &sig).is_err());
}

/// Bins from the wire that are too large for the window are refused
/// without walking them.
#[test]
fn huge_bins_are_refused() {
    let (mut seeder, mut peer) = setup(ProtectionMethod::SIGN_ALL, LiveSignatureAlgorithm::ED25519);
    seeder
        .writer
        .write(&mut seeder.swarm, t(1), &stream(128 * 4))
        .unwrap();
    let hs = seeder.handshake_frame();
    exchange(&mut seeder, &mut peer, &hs);
    for b in [Bin::new(40, 0), Bin::ALL, Bin::NONE, strims_ppspp::MAX_BIN] {
        peer.swarm
            .handle_signed_integrity(&mut peer.verifier, b, t(0), &[0; 64]);
        assert!(
            peer.swarm
                .handle_data(&mut peer.verifier, b, &[0; 128])
                .is_err()
        );
        assert!(
            seeder
                .swarm
                .write_integrity(b, &Map::new(), &mut Recorded::default())
                .is_err()
        );
        assert!(
            seeder
                .swarm
                .write_data(b, t(0), &mut Recorded::default())
                .is_err()
        );
    }
}

mod mesh {
    //! A deterministic mesh: one runner per node, every pair of joined
    //! nodes linked, frames delivered after the link latency, each node's
    //! uplink sending one frame at a time at its rate (round robin over
    //! the peers that want to send), downlinks without a limit. The seed
    //! writes the stream one integrity unit at a time at the stream rate.

    use std::collections::BTreeMap;

    use strims_base::rng::det_rng;
    use strims_ppspp::{Event, PeerKey, PushMode, Rewind, RewindError, Runner, SwarmKey};

    use super::*;

    pub const CHUNK: usize = 4096;
    pub const UNIT: usize = 8;
    pub const WINDOW: usize = 1024;
    /// The stream rate: 4 Mbit/s, 122 chunks a second.
    pub const RATE: u64 = 500_000;
    const LATENCY: Duration = Duration::from_millis(30);
    const FRAME: usize = 8 * 1024;

    pub fn byte(o: u64) -> u8 {
        (o.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 56) as u8
    }

    pub fn options(mode: PushMode) -> SwarmOptions {
        SwarmOptions {
            chunk_size: CHUNK,
            chunks_per_signature: UNIT,
            stream_count: 16,
            live_window: WINDOW,
            push_mode: mode,
            integrity: VerifierOptions {
                protection_method: ProtectionMethod::MERKLE_TREE,
                merkle_hash_tree_function: MerkleHashTreeFunction::BLAKE3,
                live_signature_algorithm: LiveSignatureAlgorithm::NONE,
            },
            ..Default::default()
        }
    }

    pub struct Node {
        pub runner: Runner,
        swarm: SwarmKey,
        pub reader: store::BufferReader,
        peers: BTreeMap<usize, PeerKey>,
        up_rate: u64,
        up_free: Time,
        rr: usize,
        pub joined: bool,
        /// The next offset a read should start at, once one was read.
        pub next_read: Option<u64>,
        /// Where reading started.
        pub first_read: Option<u64>,
        /// Reads that started anywhere but where the last one ended.
        pub gaps: u64,
        /// Bytes that differ from the stream.
        pub wrong: u64,
        /// When each chunk's last byte was first read.
        pub read_at: BTreeMap<u64, Time>,
        /// A rewind retried at each `Readable`: its offset and start.
        pub rewinding: Option<(u64, Time)>,
        /// Its outcome, and how long it took.
        pub rewound: Option<(Result<u64, RewindError>, Duration)>,
    }

    pub struct Mesh {
        pub nodes: Vec<Node>,
        writer: Writer,
        pub now: Time,
        queue: BTreeMap<(Time, u64), (usize, usize, Vec<u8>)>,
        seq: u64,
        pub written: u64,
        next_write: Time,
        /// When each chunk was written.
        pub published: Vec<Time>,
    }

    /// The channel id of `from`'s frames at `to`.
    fn channel(to: usize, from: usize) -> u64 {
        (to * 100 + from) as u64 + 1
    }

    impl Mesh {
        /// A seed (node 0, 20 Mbit/s up) and `viewers` viewers (10 Mbit/s
        /// up), which take part once [`Mesh::join`] links them.
        pub fn new(mode: PushMode, viewers: usize) -> Mesh {
            let key = strims_crypto::key_from_seed(&[91; 32]);
            let (writer, seed) = Writer::new(
                WriterOptions {
                    swarm_options: options(mode),
                    key: key.clone(),
                },
                t(0),
            )
            .unwrap();
            let mut seed = Some(seed);
            let nodes = (0..=viewers)
                .map(|i| {
                    let swarm = seed.take().unwrap_or_else(|| {
                        Swarm::new(SwarmId::new(&key.public), options(mode)).unwrap()
                    });
                    let reader = swarm.reader();
                    let mut runner = Runner::new(det_rng(100 + i as u64));
                    let up_rate = if i == 0 { 2_500_000 } else { 1_250_000 };
                    runner.set_upload_capacity(Some(up_rate));
                    let swarm = runner.add_swarm(swarm);
                    Node {
                        runner,
                        swarm,
                        reader,
                        peers: BTreeMap::new(),
                        up_rate,
                        up_free: t(0),
                        rr: 0,
                        joined: i == 0,
                        next_read: None,
                        first_read: None,
                        gaps: 0,
                        wrong: 0,
                        read_at: BTreeMap::new(),
                        rewinding: None,
                        rewound: None,
                    }
                })
                .collect();
            Mesh {
                nodes,
                writer,
                now: t(0),
                queue: BTreeMap::new(),
                seq: 0,
                written: 0,
                next_write: t(0),
                published: vec![],
            }
        }

        /// Links node `i` with every joined node.
        pub fn join(&mut self, i: usize) {
            let now = self.now;
            for j in 0..self.nodes.len() {
                if j == i || !self.nodes[j].joined {
                    continue;
                }
                let pj = self.nodes[i].runner.add_peer(&[j as u8 + 1], now);
                let pi = self.nodes[j].runner.add_peer(&[i as u8 + 1], now);
                let (a, b) = (&mut self.nodes[i], pj);
                a.runner
                    .run_swarm(b, a.swarm, channel(i, j), channel(j, i), now)
                    .unwrap();
                a.peers.insert(j, b);
                let (a, b) = (&mut self.nodes[j], pi);
                a.runner
                    .run_swarm(b, a.swarm, channel(j, i), channel(i, j), now)
                    .unwrap();
                a.peers.insert(i, b);
                self.drain(i);
                self.drain(j);
            }
            self.nodes[i].joined = true;
        }

        /// Starts moving node `i`'s reader back to `offset`; the outcome
        /// lands in [`Node::rewound`].
        pub fn rewind(&mut self, i: usize, offset: u64) {
            self.nodes[i].rewinding = Some((offset, self.now));
            self.retry_rewind(i);
            self.drain(i);
        }

        fn retry_rewind(&mut self, i: usize) {
            let now = self.now;
            let n = &mut self.nodes[i];
            let Some((offset, since)) = n.rewinding else {
                return;
            };
            let r = match n.runner.rewind(n.swarm, &mut n.reader, offset, now) {
                Ok(Rewind::Pending) => return,
                Ok(Rewind::Done(at)) => {
                    n.next_read = Some(at);
                    Ok(at)
                }
                Err(e) => Err(e),
            };
            n.rewinding = None;
            n.rewound = Some((r, now - since));
        }

        /// Runs until `end`.
        pub fn run_until(&mut self, end: Time) {
            let unit = (UNIT * CHUNK) as i64;
            let interval = Duration::from_nanos(unit * 1_000_000_000 / RATE as i64);
            loop {
                let mut next = end.min(self.next_write);
                if let Some((&(at, _), _)) = self.queue.first_key_value() {
                    next = next.min(at);
                }
                for n in self.nodes.iter().filter(|n| n.joined) {
                    if let Some(at) = n.runner.poll_timeout() {
                        next = next.min(at);
                    }
                    let wants = n.peers.values().any(|&p| n.runner.wants_transmit(p));
                    if wants {
                        next = next.min(n.up_free.max(self.now));
                    }
                }
                if next > end || next == end && self.now == end {
                    return;
                }
                self.now = self.now.max(next);
                let now = self.now;

                while let Some(entry) = self.queue.first_entry() {
                    if entry.key().0 > now {
                        break;
                    }
                    let (to, from, frame) = entry.remove();
                    let n = &mut self.nodes[to];
                    let peer = n.peers[&from];
                    n.runner.handle_frame(peer, now, &frame).unwrap();
                    self.drain(to);
                }
                while self.next_write <= now {
                    let data: Vec<u8> = (self.written..self.written + unit as u64)
                        .map(byte)
                        .collect();
                    let n = &mut self.nodes[0];
                    n.runner
                        .write(n.swarm, &mut self.writer, now, &data)
                        .unwrap();
                    self.written += unit as u64;
                    self.published.extend([now; UNIT]);
                    self.next_write += interval;
                    self.drain(0);
                }
                for i in 0..self.nodes.len() {
                    let n = &mut self.nodes[i];
                    if n.joined && n.runner.poll_timeout().is_some_and(|at| at <= now) {
                        n.runner.handle_timeout(now);
                        self.drain(i);
                    }
                }
                for i in 0..self.nodes.len() {
                    self.transmit(i);
                }
            }
        }

        fn transmit(&mut self, i: usize) {
            let now = self.now;
            let mut buf = vec![0u8; FRAME];
            loop {
                let n = &mut self.nodes[i];
                if !n.joined || n.up_free > now {
                    return;
                }
                let peers: Vec<(usize, PeerKey)> = n.peers.iter().map(|(&j, &p)| (j, p)).collect();
                let mut sent = false;
                for k in 0..peers.len() {
                    let (j, p) = peers[(n.rr + k) % peers.len()];
                    if !n.runner.wants_transmit(p) {
                        continue;
                    }
                    let len = n.runner.poll_transmit(p, now, &mut buf);
                    if len == 0 {
                        continue;
                    }
                    n.rr = (n.rr + k + 1) % peers.len();
                    let nanos = len as i64 * 1_000_000_000 / n.up_rate as i64;
                    n.up_free = n.up_free.max(now) + Duration::from_nanos(nanos);
                    let at = n.up_free + LATENCY;
                    self.seq += 1;
                    self.queue
                        .insert((at, self.seq), (j, i, buf[..len].to_vec()));
                    sent = true;
                    break;
                }
                self.drain(i);
                if !sent {
                    return;
                }
            }
        }

        fn drain(&mut self, i: usize) {
            let mut readable = false;
            while let Some(e) = self.nodes[i].runner.poll_event() {
                readable |= matches!(e, Event::Readable(_));
            }
            if readable {
                self.retry_rewind(i);
                self.read(i);
            }
        }

        fn read(&mut self, i: usize) {
            let now = self.now;
            let n = &mut self.nodes[i];
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                let swarm = n.runner.swarm(n.swarm).unwrap();
                match swarm.read(&mut n.reader, &mut buf) {
                    Ok(k) => {
                        let start = n.reader.position() - k as u64;
                        if n.first_read.is_none() {
                            n.first_read = Some(start);
                        } else if n.next_read != Some(start) {
                            n.gaps += 1;
                        }
                        n.next_read = Some(start + k as u64);
                        n.wrong += buf[..k]
                            .iter()
                            .enumerate()
                            .filter(|&(o, &b)| b != byte(start + o as u64))
                            .count() as u64;
                        let cs = CHUNK as u64;
                        let mut c = start.div_ceil(cs);
                        while (c + 1) * cs <= start + k as u64 {
                            n.read_at.entry(c).or_insert(now);
                            c += 1;
                        }
                    }
                    Err(store::Error::WouldBlock) => return,
                    Err(store::Error::BufferUnderrun) => {
                        let mut s = n.runner.swarm_mut(n.swarm, now).unwrap();
                        s.recover_reader(&mut n.reader).unwrap();
                    }
                    Err(e) => panic!("node {i}: {e}"),
                }
            }
        }

        /// The read lags of node `i`'s chunks written from `from` to `to`.
        pub fn lags(&self, i: usize, from: Time, to: Time) -> Vec<Duration> {
            self.nodes[i]
                .read_at
                .iter()
                .filter_map(|(&c, &at)| {
                    let p = *self.published.get(c as usize)?;
                    (p >= from && p < to).then(|| at - p)
                })
                .collect()
        }
    }

    pub fn quantile(v: &mut [Duration], q: f64) -> Duration {
        v.sort();
        v.get(((v.len() as f64 - 1.0) * q).round() as usize)
            .copied()
            .unwrap_or_default()
    }
}

/// A run of the mesh: a seed, six viewers that join in the first second,
/// and a late viewer (node 7) that joins at 10 s, reads its first chunks
/// and, with `back` set, moves back that many bytes behind where it
/// started. Returns the mesh at 20 s.
fn rewind_run(mode: strims_ppspp::PushMode, back: Option<u64>) -> mesh::Mesh {
    let mut m = mesh::Mesh::new(mode, 7);
    for i in 1..7 {
        m.run_until(t(150 * i as i64));
        m.join(i);
    }
    m.run_until(t(10_000));
    m.join(7);
    while m.nodes[7].first_read.is_none() {
        let next = m.now + Duration::from_millis(10);
        m.run_until(next);
    }
    if let Some(back) = back {
        let first = m.nodes[7].first_read.unwrap();
        m.rewind(7, first.saturating_sub(back));
    }
    m.run_until(t(20_000));
    m
}

/// The other viewers' lags of the chunks written from 5 s to 19 s, all
/// together.
fn others_lags(m: &mesh::Mesh) -> Vec<Duration> {
    (1..7)
        .flat_map(|i| m.lags(i, t(5_000), t(19_000)))
        .collect()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// A late viewer moves back two seconds of stream (a GOP of 244 chunks):
/// it reads exactly the stream from the offset's chunk on, and the other
/// viewers read every chunk with their lag as it is without the rewind.
#[test]
fn a_late_viewer_rewinds_a_gop_and_the_others_keep_their_lag() {
    use strims_ppspp::PushMode;
    let back = 2 * mesh::RATE + 1234;
    for mode in [PushMode::Hybrid, PushMode::Off] {
        let control = rewind_run(mode, None);
        let m = rewind_run(mode, Some(back));
        let j = &m.nodes[7];
        let first = j.first_read.unwrap();
        let k = first - back;
        let at = k - k % mesh::CHUNK as u64;
        let (r, took) = j.rewound.unwrap();
        assert_eq!(r, Ok(at), "{mode:?}");
        assert_eq!(j.wrong, 0, "{mode:?}");
        assert_eq!(j.gaps, 0, "{mode:?}: reads go on from the chunk");
        // it caught up with the live edge, as it does without the rewind
        let behind = mesh::RATE / 2;
        assert!(j.next_read.unwrap() + behind >= m.written, "{mode:?}");
        assert_eq!(control.nodes[7].first_read, Some(first), "{mode:?}");
        for i in 1..7 {
            let n = &m.nodes[i];
            assert_eq!((n.gaps, n.wrong), (0, 0), "{mode:?} viewer {i}");
            let read = n.read_at.len();
            let want = control.nodes[i].read_at.len();
            assert!(read + 8 >= want, "{mode:?} viewer {i}: {read} of {want}");
        }
        let (mut with, mut without) = (others_lags(&m), others_lags(&control));
        let (p50, p99) = (
            mesh::quantile(&mut with, 0.5),
            mesh::quantile(&mut with, 0.99),
        );
        let (c50, c99) = (
            mesh::quantile(&mut without, 0.5),
            mesh::quantile(&mut without, 0.99),
        );
        let mut joiner = m.lags(7, t(11_000), t(19_000));
        println!(
            "{mode:?}: rewind of {} chunks took {:.0} ms; others' lag p50 {:.0} ms p99 {:.0} ms \
             (without: p50 {:.0} ms p99 {:.0} ms); the joiner's lag after it p50 {:.0} ms",
            (first - at).div_ceil(mesh::CHUNK as u64),
            ms(took),
            ms(p50),
            ms(p99),
            ms(c50),
            ms(c99),
            ms(mesh::quantile(&mut joiner, 0.5)),
        );
        assert!(took < Duration::from_secs(3), "{mode:?}: {took:?}");
        assert!(
            p99 <= c99 + Duration::from_millis(100) && ms(p99) <= ms(c99) * 1.5 + 20.0,
            "{mode:?}: p99 {p99:?} against {c99:?}"
        );
    }
}

/// A rewind further back than the window reaches fails at once, and the
/// viewer reads on as it would have.
#[test]
fn a_rewind_past_the_window_fails_and_reading_goes_on() {
    use strims_ppspp::{PushMode, RewindError};
    let window = (mesh::WINDOW * mesh::CHUNK) as u64;
    let control = rewind_run(PushMode::Hybrid, None);
    let m = rewind_run(PushMode::Hybrid, Some(window));
    let j = &m.nodes[7];
    let (r, took) = j.rewound.unwrap();
    assert_eq!(r, Err(RewindError::OutsideWindow));
    assert_eq!(took, Duration::ZERO);
    assert_eq!((j.gaps, j.wrong), (0, 0));
    assert_eq!(j.first_read, control.nodes[7].first_read);
    assert_eq!(j.next_read, control.nodes[7].next_read);
}
