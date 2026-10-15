// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! End to end through the wire format, without a scheduler: a Rust
//! `Writer` publishes into a seeder swarm; the seeder sends the handshake,
//! integrity and data as encoded frames; a second swarm, acting as the
//! peer, decodes them, verifies, publishes, answers with HAVEs, and reads
//! the original byte stream back through its non-blocking reader. Tampered
//! chunks, signatures and uncle hashes are rejected and the transfer
//! recovers.

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
