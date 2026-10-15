// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sign-all integrity tests: verification across generations of peers,
//! recovery of the pending run, bounds, and caches.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::Time;
use strims_binmap::{Bin, Map};

use super::merkle_test::{TestWriter, random, test_key};
use super::*;

fn verifier(key: &strims_proto::types::Key, window: usize, cs: usize) -> SignAllSwarmVerifier {
    SignAllSwarmVerifier::new(&SignAllOptions {
        live_discard_window: window,
        chunk_size: cs,
        verifier: LiveSignatureAlgorithm::ED25519
            .verifier(&key.public)
            .unwrap(),
    })
    .unwrap()
}

fn write_all(v: &mut SignAllSwarmVerifier, signer: &SignatureSigner, cs: usize, d: &[u8]) {
    for (i, chunk) in d.chunks(cs).enumerate() {
        v.write_chunk(
            signer,
            Time::from_unix_millis(1_700_000_000_000 + i as i64),
            Bin(2 * i as u64),
            chunk,
        );
    }
}

fn apply(cv: &mut ChannelVerifier, b: Bin, w: &TestWriter) {
    let mut v = cv.chunk_verifier(b);
    for (sb, ts, sig) in &w.signed {
        v.set_signed_integrity(*sb, *ts, sig);
    }
}

fn sent(src: &SwarmVerifier, b: Bin) -> TestWriter {
    let mut w = TestWriter::default();
    src.write_integrity(b, &Map::new(), &mut w).unwrap();
    w
}

/// Each generation verifies every chunk using the signatures the previous
/// generation sends. The data is small because ed25519 in a debug build is
/// slow.
#[test]
fn sign_all_verifier() {
    let key = test_key(6);
    let (cs, window) = (1024, 256);
    let n = 1 << 18;

    let mut v0 = verifier(&key, window, cs);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    let d = random(n, 6);
    write_all(&mut v0, &signer, cs, &d);

    let mut src = SwarmVerifier::SignAll(v0);
    for generation in 0..3 {
        let mut dst = SwarmVerifier::SignAll(verifier(&key, window, cs));
        let mut cv = dst.channel_verifier();
        for i in 0..n / cs {
            let b = Bin(2 * i as u64);
            apply(&mut cv, b, &sent(&src, b));
            cv.chunk_verifier(b)
                .verify(&mut dst, b, &d[i * cs..(i + 1) * cs])
                .unwrap_or_else(|e| panic!("gen {generation}: invalid data at bin {b}: {e}"));
        }
        src = dst;
    }
}

struct Fixture {
    key: strims_proto::types::Key,
    src: SwarmVerifier,
    dst: SwarmVerifier,
    data: Vec<u8>,
}

const CS: usize = 32;
const WINDOW: usize = 16;

fn fixture() -> Fixture {
    let key = test_key(7);
    let mut src = verifier(&key, WINDOW, CS);
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&key.private)
        .unwrap();
    let data = random(8 * CS, 7);
    write_all(&mut src, &signer, CS, &data);
    Fixture {
        dst: SwarmVerifier::SignAll(verifier(&key, WINDOW, CS)),
        src: SwarmVerifier::SignAll(src),
        key,
        data,
    }
}

fn chunks(d: &[u8], b: Bin) -> &[u8] {
    let l = b.base_offset() as usize * CS;
    &d[l..l + b.base_length() as usize * CS]
}

#[test]
fn multi_chunk_bins() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin::new(2, 1);
    let w = sent(&f.src, b);
    assert_eq!(
        w.signed.iter().map(|s| s.0).collect::<Vec<_>>(),
        vec![Bin(8), Bin(10), Bin(12), Bin(14)]
    );
    apply(&mut cv, b, &w);
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunks(&f.data, b))
        .unwrap();
    assert_eq!(sent(&f.dst, b).signed, w.signed);
}

/// A failed verification clears the pending run, so later chunks on the
/// channel still verify.
#[test]
fn channel_recovers_after_failure() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin(2);
    apply(&mut cv, b, &sent(&f.src, b));
    let mut bad = chunks(&f.data, b).to_vec();
    bad[0] ^= 1;
    assert_eq!(
        cv.chunk_verifier(b).verify(&mut f.dst, b, &bad),
        Err(Error::InvalidChunkSignature)
    );
    let b = Bin(4);
    apply(&mut cv, b, &sent(&f.src, b));
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunks(&f.data, b))
        .unwrap();
}

/// Signatures for data the channel skipped (not expected by the scheduler)
/// don't block the next run.
#[test]
fn unverified_run_is_replaced() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    apply(&mut cv, Bin(2), &sent(&f.src, Bin(2)));
    // the data for bin 2 is never verified
    let b = Bin(8);
    apply(&mut cv, b, &sent(&f.src, b));
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunks(&f.data, b))
        .unwrap();

    // a consecutive run that starts before the data's bin
    apply(&mut cv, Bin(10), &sent(&f.src, Bin(10)));
    apply(&mut cv, Bin(12), &sent(&f.src, Bin(12)));
    let b = Bin(12);
    cv.chunk_verifier(b)
        .verify(&mut f.dst, b, chunks(&f.data, b))
        .unwrap();
}

#[test]
fn tampered_signature_and_missing_signatures() {
    let mut f = fixture();
    let mut cv = f.dst.channel_verifier();
    let b = Bin(6);
    let mut w = sent(&f.src, b);
    w.signed[0].2[5] ^= 1;
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunks(&f.data, b)),
        Err(Error::InvalidChunkSignature)
    );
    // no signatures
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunks(&f.data, b)),
        Err(Error::MissingChunkSignature)
    );
    // fewer signatures than chunks
    let b = Bin::new(1, 2);
    let mut w = sent(&f.src, b);
    w.signed.pop();
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunks(&f.data, b)),
        Err(Error::MissingChunkSignature)
    );
    // a signature of the wrong length is dropped
    let b = Bin(6);
    let mut w = sent(&f.src, b);
    w.signed[0].2.push(0);
    apply(&mut cv, b, &w);
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, chunks(&f.data, b)),
        Err(Error::MissingChunkSignature)
    );
    // short data
    let b = Bin(6);
    apply(&mut cv, b, &sent(&f.src, b));
    assert_eq!(
        cv.chunk_verifier(b)
            .verify(&mut f.dst, b, &chunks(&f.data, b)[1..]),
        Err(Error::DataLength)
    );
}

#[test]
fn pending_signatures_are_bounded() {
    let f = fixture();
    let mut cv = f.dst.channel_verifier();
    let ChannelVerifier::SignAll(v) = &mut cv else {
        unreachable!()
    };
    for i in 0..10 * WINDOW as u64 {
        v.set_signed_integrity(Bin(2 * i), Time::EPOCH, &[0; 64]);
        assert!(v.timestamps.len() <= WINDOW);
    }
    // huge and sentinel bins are rejected before they are stored
    for b in [Bin::NONE, Bin::ALL, Bin(1)] {
        v.set_signed_integrity(b, Time::EPOCH, &[0; 64]);
        assert!(v.timestamps.is_empty());
    }
    let mut dst = f.dst;
    for b in [Bin::ALL, Bin::new(40, 0), Bin::NONE] {
        assert_eq!(
            cv.chunk_verifier(b).verify(&mut dst, b, &[]),
            Err(Error::BinOutOfBounds)
        );
    }
}

/// A late chunk's signature, from before the tail, never overwrites the
/// slot of a live chunk.
#[test]
fn late_signature_does_not_clobber_live_slot() {
    let mut f = fixture();
    let SwarmVerifier::SignAll(src) = &mut f.src else {
        unreachable!()
    };
    let signer = LiveSignatureAlgorithm::ED25519
        .signer(&f.key.private)
        .unwrap();
    // push the window past chunk 0
    let more = random(WINDOW * CS, 8);
    for (i, c) in more.chunks(CS).enumerate() {
        src.write_chunk(&signer, Time::EPOCH, Bin(2 * (8 + i) as u64), c);
    }
    assert_eq!(src.tail, Bin(16));
    let before = sent(&f.src, Bin(32)).signed;
    let SwarmVerifier::SignAll(src) = &mut f.src else {
        unreachable!()
    };
    src.store_signature(Bin(0), Time::EPOCH, &[0; 64]);
    assert_eq!(sent(&f.src, Bin(32)).signed, before);
    assert_eq!(
        f.src
            .write_integrity(Bin(0), &Map::new(), &mut TestWriter::default()),
        Err(Error::MissingChunkSignature)
    );
}

#[test]
fn cache_round_trip_sets_head() {
    let f = fixture();
    let SwarmVerifier::SignAll(src) = &f.src else {
        unreachable!()
    };
    let c = Cache {
        integrity: Some(src.export_cache()),
        data: f.data.clone(),
        ..Default::default()
    };
    assert_eq!(
        c.integrity
            .as_ref()
            .unwrap()
            .sign_all_integrity
            .as_ref()
            .unwrap()
            .timestamps
            .len(),
        8
    );
    let mut dst = SwarmVerifier::SignAll(verifier(&f.key, WINDOW, CS));
    dst.import_cache(&c).unwrap();
    assert_eq!(dst.export_cache(), src.export_cache());
    // the import moved the head, so the imported signatures can be sent
    assert_eq!(
        sent(&dst, Bin::new(3, 0)).signed,
        sent(&f.src, Bin::new(3, 0)).signed
    );

    let mut bad = c.clone();
    bad.data[3 * CS] ^= 1;
    let mut dst = SwarmVerifier::SignAll(verifier(&f.key, WINDOW, CS));
    assert_eq!(dst.import_cache(&bad), Err(Error::InvalidChunkSignature));
    let mut short = c.clone();
    short.data.truncate(7 * CS);
    assert_eq!(
        dst.import_cache(&short),
        Err(Error::IntegrityCacheIncomplete)
    );

    // a reset verifier exports nothing stale
    dst.import_cache(&c).unwrap();
    assert_eq!(dst.export_cache(), src.export_cache());
    dst.reset();
    assert_eq!(
        dst.export_cache()
            .sign_all_integrity
            .unwrap()
            .timestamps
            .len(),
        0
    );
}
