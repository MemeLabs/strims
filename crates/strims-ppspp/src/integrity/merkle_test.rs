// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Merkle integrity tests: the writer, verification across generations of
//! peers, tampering, and bounds on peer input.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use rand_core::Rng;
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_binmap::{Bin, Map};

use super::*;
use crate::codec::{self, Message};

/// Records integrity messages.
#[derive(Default)]
pub(super) struct TestWriter {
    /// Bin, timestamp, signature and count of data-less chunks.
    pub signed: Vec<(Bin, Time, Vec<u8>, u64)>,
    pub integrity: Vec<(Bin, Vec<u8>)>,
}

impl MessageWriter for TestWriter {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, codec::Error> {
        match m {
            Message::SignedIntegrity(m) => {
                self.signed
                    .push((m.bin, m.timestamp, m.signature.to_vec(), m.dataless))
            }
            Message::Integrity(m) => self.integrity.push((m.bin, m.hash.to_vec())),
            _ => panic!("unexpected message {m:?}"),
        }
        Ok(0)
    }
}

pub(super) fn test_key(seed: u8) -> strims_proto::types::Key {
    strims_crypto::key_from_seed(&[seed; 32])
}

pub(super) fn random(n: usize, seed: u64) -> Vec<u8> {
    let mut b = vec![0u8; n];
    det_rng(seed).fill_bytes(&mut b);
    b
}

fn verifier(
    key: &strims_proto::types::Key,
    window: usize,
    cps: usize,
    cs: usize,
    hash: MerkleHashTreeFunction,
) -> MerkleSwarmVerifier {
    MerkleSwarmVerifier::new(&MerkleOptions {
        live_discard_window: window,
        chunk_size: cs,
        chunks_per_signature: cps,
        verifier: LiveSignatureAlgorithm::ED25519
            .verifier(&key.public)
            .unwrap(),
        hash,
    })
    .unwrap()
}

/// Feeds every munro of `d` through `write_munro`.
fn write_all(
    v: &mut MerkleSwarmVerifier,
    signer: &SignatureSigner,
    cps: usize,
    cs: usize,
    d: &[u8],
) {
    for (i, munro) in d.chunks(cps * cs).enumerate() {
        v.write_munro(
            signer,
            Time::from_unix_millis(1_700_000_000_000 + i as i64),
            i as u64,
            munro,
            cps as u64,
        )
        .unwrap();
    }
}

/// The writer fills the ring and wraps it.
#[test]
fn merkle_writer() {
    let key = test_key(1);
    let (window, cps, cs) = (1 << 10, 32, 1024);
    let mut v = verifier(&key, window, cps, cs, MerkleHashTreeFunction::SHA256);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    // 2 MiB: twice the live window, so the ring wraps
    write_all(&mut v, &signer, cps, cs, &random(1 << 21, 1));
    assert_eq!(v.ring.head, 64);
    assert_eq!(v.ring.tail, 32);
}

/// Each generation verifies every chunk using the integrity messages the
/// previous generation sends.
#[test]
fn merkle_verifier() {
    let key = test_key(2);
    let (window, cps, cs) = (1 << 10, 32, 1024);
    let n = 1 << 20;
    assert_eq!(n, window * cs);

    let mut src = verifier(&key, window, cps, cs, MerkleHashTreeFunction::BLAKE2B256);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    let d = random(n, 2);
    write_all(&mut src, &signer, cps, cs, &d);

    for generation in 0..3 {
        let src_sv = SwarmVerifier::Merkle(src);
        let mut sv = SwarmVerifier::Merkle(verifier(
            &key,
            window,
            cps,
            cs,
            MerkleHashTreeFunction::BLAKE2B256,
        ));
        let mut cv = sv.channel_verifier();
        for i in 0..n / cs {
            let b = Bin(2 * i as u64);
            let mut w = TestWriter::default();
            src_sv
                .write_integrity(b, &Map::new(), &mut w)
                .unwrap_or_else(|e| panic!("failed to write integrity messages bin {b}: {e}"));

            let mut v = cv.chunk_verifier(b);
            let (_, ts, sig, dataless) = &w.signed[0];
            v.set_signed_integrity(b, *ts, *dataless, sig);
            for (ib, hash) in &w.integrity {
                v.set_integrity(*ib, hash);
            }
            v.verify(&mut sv, b, &d[i * cs..(i + 1) * cs])
                .unwrap_or_else(|e| panic!("gen {generation}: invalid data at bin {b}: {e}"));
        }
        let SwarmVerifier::Merkle(next) = sv else {
            unreachable!()
        };
        src = next;
    }
}

fn sent_integrity(src: &SwarmVerifier, b: Bin, have: &Map) -> TestWriter {
    let mut w = TestWriter::default();
    src.write_integrity(b, have, &mut w).unwrap();
    w
}

fn apply(cv: &mut ChannelVerifier, b: Bin, w: &TestWriter) {
    let mut v = cv.chunk_verifier(b);
    for (sb, ts, sig, dataless) in &w.signed {
        v.set_signed_integrity(*sb, *ts, *dataless, sig);
    }
    for (ib, hash) in &w.integrity {
        v.set_integrity(*ib, hash);
    }
}

struct Fixture {
    key: strims_proto::types::Key,
    src: SwarmVerifier,
    dst: SwarmVerifier,
    data: Vec<u8>,
}

const CS: usize = 64;
const CPS: usize = 8;
const WINDOW: usize = 64;

fn fixture() -> Fixture {
    let key = test_key(3);
    let mut src = verifier(&key, WINDOW, CPS, CS, MerkleHashTreeFunction::SHA256);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    let data = random(4 * CPS * CS, 3);
    write_all(&mut src, &signer, CPS, CS, &data);
    let dst = verifier(&key, WINDOW, CPS, CS, MerkleHashTreeFunction::SHA256);
    Fixture {
        key,
        src: SwarmVerifier::Merkle(src),
        dst: SwarmVerifier::Merkle(dst),
        data,
    }
}

fn chunk(d: &[u8], b: Bin) -> &[u8] {
    let l = b.base_offset() as usize * CS;
    &d[l..l + b.base_length() as usize * CS]
}

#[test]
fn verify_uses_stored_tree_for_later_chunks() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin(2);
    apply(&mut cv, b, &sent_integrity(&f.src, b, &Map::new()));
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();

    // the peer now has chunk 1: the next chunk needs no signature and only
    // the uncles below the first ancestor the peer has data under
    let mut have = Map::new();
    have.set(b);
    let b = Bin(4);
    let w = sent_integrity(&f.src, b, &have);
    assert!(w.signed.is_empty());
    assert_eq!(
        w.integrity.iter().map(|x| x.0).collect::<Vec<_>>(),
        vec![Bin(6)]
    );
    apply(&mut cv, b, &w);
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();

    // and the destination can now serve integrity for both chunks
    let w = sent_integrity(&f.dst, Bin(2), &Map::new());
    assert_eq!(w.signed.len(), 1);
    let _ = f.key;
}

#[test]
fn tampered_chunk_is_rejected() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin(18);
    apply(&mut cv, b, &sent_integrity(&f.src, b, &Map::new()));
    let mut c = chunk(&f.data, b).to_vec();
    c[7] ^= 0x80;
    assert_eq!(
        cv.chunk_verifier(b).verify(&mut f.dst, b, &c),
        Err(Error::InvalidSignature)
    );
    // nothing was stored, and the channel recovers for the next attempt
    assert_eq!(
        f.dst
            .write_integrity(b, &Map::new(), &mut TestWriter::default()),
        Err(Error::MissingHashSubtree)
    );
    apply(&mut cv, b, &sent_integrity(&f.src, b, &Map::new()));
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();
}

#[test]
fn tampered_signature_is_rejected() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin(34);
    let mut w = sent_integrity(&f.src, b, &Map::new());
    w.signed[0].2[10] ^= 1;
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunk(&f.data, b)),
        Err(Error::InvalidSignature)
    );

    // a signature of the wrong length
    let mut w = sent_integrity(&f.src, b, &Map::new());
    w.signed[0].2.truncate(63);
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunk(&f.data, b)),
        Err(Error::SignatureTooShort)
    );

    // no signature at all
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunk(&f.data, b)),
        Err(Error::SignatureTooShort)
    );
}

#[test]
fn tampered_uncle_is_rejected() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin(20);
    let mut w = sent_integrity(&f.src, b, &Map::new());
    w.integrity[1].1[0] ^= 1;
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunk(&f.data, b)),
        Err(Error::InvalidSignature)
    );

    // once munro 1 is verified, a bad uncle mismatches the stored tree
    let b = Bin(22);
    apply(&mut cv, b, &sent_integrity(&f.src, b, &Map::new()));
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();
    let b = Bin(16);
    let mut w = sent_integrity(&f.src, b, &Map::new());
    // the sibling: bin 17 is already verified, so the path stops there
    // and a bad bin 27 would never be read
    w.integrity[0].1[0] ^= 1;
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunk(&f.data, b)),
        Err(Error::Merkle(crate::merkle::Error::HashMismatch))
    );
    let mut w = sent_integrity(&f.src, b, &Map::new());
    w.integrity[2].1[0] ^= 1;
    apply(&mut cv, b, &w);
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();
}

#[test]
fn peer_bins_are_bounds_checked() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();

    for b in [
        Bin::NONE,
        Bin::ALL,
        Bin(u64::MAX - 1),
        Bin::new(40, 3),
        Bin::new(62, 0),
    ] {
        let mut w = TestWriter::default();
        assert!(
            f.src.write_integrity(b, &Map::new(), &mut w).is_err(),
            "{b}"
        );
        let mut v = cv.chunk_verifier(b);
        v.set_signed_integrity(b, Time::EPOCH, 1, &[0; 64]);
        v.set_integrity(b, &[0; 32]);
        let r = v.verify(&mut f.dst, b, &[0; CS]);
        assert!(r.is_err(), "{b}");
    }

    // a bin spanning two munros, and data of the wrong length
    let b = Bin::new(4, 0);
    assert_eq!(
        f.src
            .write_integrity(b, &Map::new(), &mut TestWriter::default()),
        Err(Error::MissingHashSubtree)
    );
    let b = Bin(2);
    apply(&mut cv, b, &sent_integrity(&f.src, b, &Map::new()));
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, &f.data[..CS - 1]),
        Err(Error::DataLength)
    );
    // an uncle outside the munro is ignored
    let mut v = cv.chunk_verifier(b);
    v.set_integrity(Bin(100), &[0; 32]);
}

/// A munro far ahead of the head advances the ring by at most one window.
#[test]
fn ring_jump_is_bounded() {
    let mut f = fixture();
    let SwarmVerifier::Merkle(dst) = &mut f.dst else {
        unreachable!()
    };
    let key = f.key;
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    dst.write_munro(&signer, Time::EPOCH, 0, &f.data[..CPS * CS], CPS as u64)
        .unwrap();
    dst.write_munro(
        &signer,
        Time::EPOCH,
        1 << 50,
        &f.data[..CPS * CS],
        CPS as u64,
    )
    .unwrap();
    assert_eq!(dst.ring.head, (1 << 50) + 1);
    assert_eq!(dst.ring.tail, (1 << 50) + 1 - dst.ring.size);
    assert_eq!(dst.ring.segments.iter().filter(|s| s.occupied).count(), 1);
}

/// With one chunk per signature, chunk 0 is munro 0, and a channel's first
/// message must still select it.
#[test]
fn one_chunk_per_signature() {
    let key = test_key(4);
    let mut src = verifier(&key, 16, 1, CS, MerkleHashTreeFunction::SHA1);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    let data = random(4 * CS, 4);
    write_all(&mut src, &signer, 1, CS, &data);
    let src = SwarmVerifier::Merkle(src);
    let mut dst = SwarmVerifier::Merkle(verifier(&key, 16, 1, CS, MerkleHashTreeFunction::SHA1));
    let mut cv = dst.channel_verifier();
    for i in 0..4u64 {
        let b = Bin(2 * i);
        let w = sent_integrity(&src, b, &Map::new());
        assert!(w.integrity.is_empty());
        apply(&mut cv, b, &w);
        cv.chunk_verifier(b)
            .verify(&mut dst, b, chunk(&data, b))
            .unwrap();
    }
}

#[test]
fn cache_round_trip() {
    let f = fixture();
    let SwarmVerifier::Merkle(src) = &f.src else {
        unreachable!()
    };
    let c = Cache {
        integrity: Some(src.export_cache()),
        data: f.data.clone(),
        ..Default::default()
    };
    let mut dst = verifier(&f.key, WINDOW, CPS, CS, MerkleHashTreeFunction::SHA256);
    dst.import_cache(&c).unwrap();
    assert_eq!(dst.export_cache(), src.export_cache());

    let mut bad = c.clone();
    bad.data[100] ^= 1;
    let mut dst = verifier(&f.key, WINDOW, CPS, CS, MerkleHashTreeFunction::SHA256);
    assert_eq!(dst.import_cache(&bad), Err(Error::InvalidSignature));
    let mut short = c.clone();
    short.data.truncate(short.data.len() - 1);
    assert_eq!(
        dst.import_cache(&short),
        Err(Error::IntegrityCacheIncomplete)
    );
    let mut nosig = c.clone();
    if let Some(i) = &mut nosig.integrity
        && let Some(m) = &mut i.merkle_integrity
    {
        m.signatures.pop();
    }
    assert_eq!(
        dst.import_cache(&nosig),
        Err(Error::IntegrityCacheIncomplete)
    );
    assert_eq!(
        dst.import_cache(&Cache::default()),
        Err(Error::NoIntegrityCache)
    );
}

#[test]
fn unsupported_options() {
    let key = test_key(5);
    let v = LiveSignatureAlgorithm::ED25519
        .verifier(&key.public)
        .unwrap();
    let o = |window, cps, hash| MerkleOptions {
        live_discard_window: window,
        chunk_size: 16,
        chunks_per_signature: cps,
        verifier: v.clone(),
        hash,
    };
    assert_eq!(
        MerkleSwarmVerifier::new(&o(64, 8, MerkleHashTreeFunction::MD5)).err(),
        Some(Error::UnsupportedHashTreeFunction)
    );
    assert_eq!(
        MerkleSwarmVerifier::new(&o(64, 128, MerkleHashTreeFunction::SHA1)).err(),
        Some(Error::InvalidOptions)
    );
    assert_eq!(
        MerkleSwarmVerifier::new(&o(48, 8, MerkleHashTreeFunction::SHA1)).err(),
        Some(Error::InvalidOptions)
    );
    assert_eq!(
        LiveSignatureAlgorithm(9).verifier(&key.public).err(),
        Some(Error::UnsupportedSignatureAlgorithm)
    );
    assert_eq!(
        LiveSignatureAlgorithm::ED25519
            .verifier(&key.public[..31])
            .err(),
        Some(Error::InvalidKey)
    );
    // 32 bytes that aren't a point: every verification fails
    let bad = LiveSignatureAlgorithm::ED25519
        .verifier(&[0xff; 32])
        .unwrap();
    assert!(!bad.verify(Domain::PpsppMunroRoot, Time::EPOCH, b"x", &[0; 64]));
}

/// A source whose munro `i` holds data in its first `counts[i]` chunks and
/// zeros after them, signed with that count.
fn early_fixture(counts: &[u64]) -> Fixture {
    let key = test_key(9);
    let mut src = verifier(&key, WINDOW, CPS, CS, MerkleHashTreeFunction::BLAKE3);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    let mut data = random(counts.len() * CPS * CS, 9);
    for (i, &c) in counts.iter().enumerate() {
        let munro = &mut data[i * CPS * CS..(i + 1) * CPS * CS];
        munro[c as usize * CS..].fill(0);
        let t = Time::from_unix_millis(1_700_000_000_000 + i as i64);
        src.write_munro(&signer, t, i as u64, munro, c).unwrap();
    }
    let dst = verifier(&key, WINDOW, CPS, CS, MerkleHashTreeFunction::BLAKE3);
    Fixture {
        key,
        src: SwarmVerifier::Merkle(src),
        dst: SwarmVerifier::Merkle(dst),
        data,
    }
}

/// The chunk at offset `o` of munro `m`.
fn munro_chunk(m: u64, o: u64) -> Bin {
    Bin(2 * (m * CPS as u64 + o))
}

/// A munro signed with fewer chunks verifies from any of its data chunks,
/// its count carried with the signature: the uncles of data-less subtrees
/// aren't sent, the receiver stores the count, and serves it on.
#[test]
fn an_early_signed_munro_verifies_from_any_data_chunk() {
    let counts = [1, 3, 5, 7, 8];
    for (m, &c) in counts.iter().enumerate() {
        let m = m as u64;
        for o in 0..c {
            let mut f = early_fixture(&counts);
            let mut cv = f.dst.channel_verifier();
            let b = munro_chunk(m, o);
            let w = sent_integrity(&f.src, b, &Map::new());
            assert_eq!(w.signed.len(), 1);
            assert_eq!(w.signed[0].3, CPS as u64 - c, "munro {m}");
            let root = Bin::new(3, m);
            for (u, _) in &w.integrity {
                let first = u.base_offset() - root.base_offset();
                assert!(first < c, "munro {m} chunk {o}: data-less uncle {u} sent");
            }
            apply(&mut cv, b, &w);
            cv.chunk_verifier(b)
                .verify(&mut f.dst, b, chunk(&f.data, b))
                .unwrap_or_else(|e| panic!("munro {m} chunk {o}: {e}"));

            // The receiver now holds the munro with its count and serves the
            // chunk on to a third verifier, and the munro's other data
            // chunks verify against what it stored.
            let mut third = SwarmVerifier::Merkle(verifier(
                &f.key,
                WINDOW,
                CPS,
                CS,
                MerkleHashTreeFunction::BLAKE3,
            ));
            let mut tcv = third.channel_verifier();
            let w = sent_integrity(&f.dst, b, &Map::new());
            assert_eq!(w.signed[0].3, CPS as u64 - c);
            apply(&mut tcv, b, &w);
            tcv.chunk_verifier(b)
                .verify(&mut third, b, chunk(&f.data, b))
                .unwrap();
            let mut have = Map::new();
            have.set(b);
            for x in (0..c).map(|x| munro_chunk(m, x)).filter(|&x| x != b) {
                let w = sent_integrity(&f.src, x, &have);
                assert!(w.signed.is_empty());
                apply(&mut cv, x, &w);
                cv.chunk_verifier(x)
                    .verify(&mut f.dst, x, chunk(&f.data, x))
                    .unwrap_or_else(|e| panic!("munro {m} chunk {o} then {x}: {e}"));
                have.set(x);
            }
        }
    }
}

/// Whatever a peer sends for a data-less subtree, the receiver checks the
/// root with zero digests there.
#[test]
fn zero_leaves_override_peer_digests_for_dataless_subtrees() {
    let mut f = early_fixture(&[3]);
    let mut cv = f.dst.channel_verifier();
    let b = munro_chunk(0, 0);
    let mut w = sent_integrity(&f.src, b, &Map::new());
    // chunk 3 and chunks 4-7 are data-less: bogus digests for them
    w.integrity.push((Bin(6), vec![0xaa; 32]));
    w.integrity.push((Bin(11), vec![0xbb; 32]));
    apply(&mut cv, b, &w);
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();
}

/// Data for a chunk its munro's signed count makes data-less is refused,
/// before and after the munro is stored, and no integrity is written for
/// it.
#[test]
fn data_for_a_dataless_chunk_is_refused() {
    let mut f = early_fixture(&[3]);
    let zeros = vec![0u8; CS];
    let mut cv = f.dst.channel_verifier();
    // before: the count comes with the signature
    let w = sent_integrity(&f.src, munro_chunk(0, 0), &Map::new());
    let x = munro_chunk(0, 5);
    apply(&mut cv, x, &w);
    assert_eq!(
        cv.chunk_verifier(x).verify(&mut f.dst, x, &zeros),
        Err(Error::DataLess)
    );
    // a bin reaching into the data-less chunks
    let wide = Bin::new(2, 0);
    apply(&mut cv, wide, &w);
    assert_eq!(
        cv.chunk_verifier(wide)
            .verify(&mut f.dst, wide, chunk(&f.data, wide)),
        Err(Error::DataLess)
    );
    assert_eq!(
        f.src
            .write_integrity(x, &Map::new(), &mut TestWriter::default()),
        Err(Error::DataLess)
    );
    // after: the stored munro's count
    let b = munro_chunk(0, 1);
    apply(&mut cv, b, &sent_integrity(&f.src, b, &Map::new()));
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunk(&f.data, b))
        .unwrap();
    assert_eq!(
        cv.chunk_verifier(x).verify(&mut f.dst, x, &zeros),
        Err(Error::DataLess)
    );
}

/// A signed count of data-less chunks that leaves the munro no chunk of
/// data verifies nothing, and a munro of no data, or of more than it
/// holds, can't be signed.
#[test]
fn counts_outside_the_munro_are_refused() {
    let mut f = early_fixture(&[8]);
    let b = munro_chunk(0, 0);
    for c in [CPS as u64, CPS as u64 + 1, u64::MAX] {
        let mut cv = f.dst.channel_verifier();
        let mut w = sent_integrity(&f.src, b, &Map::new());
        w.signed[0].3 = c;
        apply(&mut cv, b, &w);
        assert_eq!(
            cv.chunk_verifier(b)
                .verify(&mut f.dst, b, chunk(&f.data, b)),
            Err(Error::InvalidChunkCount),
            "{c}"
        );
    }
    let SwarmVerifier::Merkle(src) = &mut f.src else {
        unreachable!()
    };
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&f.key.private)
        .unwrap();
    for c in [0, CPS as u64 + 1] {
        assert_eq!(
            src.write_munro(&signer, Time::EPOCH, 1, &f.data, c),
            Err(Error::InvalidChunkCount)
        );
    }
}

/// A munro's signature holds only at its own bin with its own count: one
/// replayed at another munro's position, or with its count changed, fails.
#[test]
fn a_munro_verifies_only_at_its_bin_and_count() {
    let mut f = early_fixture(&[8, 8, 5]);
    let SwarmVerifier::Merkle(src) = &f.src else {
        unreachable!()
    };
    // munro 1's data and integrity, presented as munro 3's
    let w = sent_integrity(&f.src, munro_chunk(1, 2), &Map::new());
    let at = munro_chunk(3, 2);
    let shift = |b: Bin| Bin(b.0 + 2 * 2 * CPS as u64);
    let mut moved = TestWriter::default();
    for (b, t, sig, c) in &w.signed {
        moved.signed.push((shift(*b), *t, sig.clone(), *c));
    }
    for (b, h) in &w.integrity {
        moved.integrity.push((shift(*b), h.clone()));
    }
    let mut cv = f.dst.channel_verifier();
    apply(&mut cv, at, &moved);
    assert_eq!(
        cv.chunk_verifier(at)
            .verify(&mut f.dst, at, chunk(&f.data, munro_chunk(1, 2))),
        Err(Error::InvalidSignature)
    );
    let _ = src;

    // munro 2 (5 chunks of data, 3 data-less) with its count raised or
    // lowered
    let b = munro_chunk(2, 1);
    for c in [4, 2, 0] {
        let mut w = sent_integrity(&f.src, b, &Map::new());
        w.signed[0].3 = c;
        let mut cv = f.dst.channel_verifier();
        apply(&mut cv, b, &w);
        assert_eq!(
            cv.chunk_verifier(b)
                .verify(&mut f.dst, b, chunk(&f.data, b)),
            Err(Error::InvalidSignature),
            "{c}"
        );
    }
}

/// A cache covers munros with data throughout: its export stops at the
/// first early-signed munro, and the import takes those it covers.
#[test]
fn cache_export_stops_before_an_early_signed_munro() {
    let f = early_fixture(&[8, 8, 3, 8]);
    let SwarmVerifier::Merkle(src) = &f.src else {
        unreachable!()
    };
    let e = src.export_cache();
    assert_eq!(e.merkle_integrity.as_ref().unwrap().timestamps.len(), 2);
    let c = Cache {
        integrity: Some(e),
        data: f.data.clone(),
        ..Default::default()
    };
    let mut dst = verifier(&f.key, WINDOW, CPS, CS, MerkleHashTreeFunction::BLAKE3);
    assert_eq!(dst.import_cache(&c), Ok(2 * CPS * CS));
}

/// A cache signed under another layout (the root alone, without its bin
/// and count) is refused whole, so it is dropped, never misread.
#[test]
fn a_cache_signed_under_another_layout_is_refused() {
    let f = fixture();
    let SwarmVerifier::Merkle(src) = &f.src else {
        unreachable!()
    };
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&f.key.private)
        .unwrap();
    let mut c = Cache {
        integrity: Some(src.export_cache()),
        data: f.data.clone(),
        ..Default::default()
    };
    let m = c
        .integrity
        .as_mut()
        .and_then(|i| i.merkle_integrity.as_mut())
        .unwrap();
    let h = MerkleHashTreeFunction::SHA256;
    for (i, sig) in m.signatures.iter_mut().enumerate() {
        let b = Bin::new(3, i as u64);
        let mut t = crate::merkle::Tree::new(b, CS, 32);
        t.fill(
            &mut h.hasher().unwrap(),
            b,
            &f.data[i * CPS * CS..(i + 1) * CPS * CS],
        )
        .unwrap();
        let ts = Time::from_unix_nanos(m.timestamps[i]);
        *sig = signer
            .sign(Domain::PpsppMunroRoot, ts, t.root_digest())
            .as_slice()
            .to_vec();
    }
    let mut dst = verifier(&f.key, WINDOW, CPS, CS, h);
    assert_eq!(dst.import_cache(&c), Err(Error::InvalidSignature));
    assert_eq!(
        dst.export_cache()
            .merkle_integrity
            .unwrap()
            .timestamps
            .len(),
        0
    );
}
