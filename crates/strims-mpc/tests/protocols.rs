// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Protocol runs between two local sessions: correctness, arbitrary
//! chunking, determinism, tampering and failure cases.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use strims_base::rng::{DetRng, det_rng};
use strims_mpc::mpctest::{Transcript, pump};
use strims_mpc::*;

const SEED: Block = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

/// Values and choices from `AesRng(SEED)`; both parties use the same seed.
fn test_ot(kind: OtKind, n: usize, rng: &mut DetRng, max_chunk: usize) {
    let mut values = Vec::with_capacity(n);
    let mut input = AesRng::new(&SEED);
    for _ in 0..n {
        let v0 = input.block();
        let v1 = input.block();
        values.push([v0, v1]);
    }
    let mut tmp = vec![0u8; n.div_ceil(8)];
    input.fill_bytes(&mut tmp);
    // most significant bit first
    let receive: Vec<bool> = (0..n).map(|i| tmp[i / 8] >> (7 - i % 8) & 1 != 0).collect();

    let mut s = ot_sender(kind, values.clone(), SEED);
    let mut r = ot_receiver(kind, receive.clone(), SEED);
    pump(&mut s, &mut r, rng, max_chunk);
    assert!(s.is_done() && r.is_done(), "{kind:?} n={n} stalled");
    assert_eq!(s.result(), Some(Ok(&())));
    let res = r.take_result().unwrap().unwrap();
    for (i, v) in values.iter().enumerate() {
        assert_eq!(res[i], v[usize::from(receive[i])], "{kind:?} n={n} i={i}");
    }
}

/// Debug builds run the elliptic-curve code ~30x slower than release, so
/// the largest sizes only run optimized.
fn sizes(max: usize) -> Vec<usize> {
    let cap = if cfg!(debug_assertions) { 1000 } else { max };
    let mut v = Vec::new();
    let mut n = 10;
    while n <= max.min(cap) {
        v.push(n);
        n *= 10;
    }
    v
}

#[test]
fn naor_pinkas() {
    let mut rng = det_rng(1);
    for n in sizes(if cfg!(debug_assertions) { 100 } else { 1000 }) {
        test_ot(OtKind::Base(BaseOt::NaorPinkas), n, &mut rng, 4096);
    }
}

#[test]
fn chou_orlandi() {
    let mut rng = det_rng(2);
    for n in sizes(1000) {
        test_ot(OtKind::Base(BaseOt::ChouOrlandi), n, &mut rng, 4096);
    }
}

#[test]
fn alsz() {
    let mut rng = det_rng(3);
    for n in sizes(10000) {
        test_ot(OtKind::Alsz(BaseOt::NaorPinkas), n, &mut rng, 4096);
    }
}

#[test]
fn kos() {
    let mut rng = det_rng(4);
    for n in sizes(10000) {
        test_ot(OtKind::Kos(BaseOt::NaorPinkas), n, &mut rng, 4096);
    }
}

#[test]
fn ot_odd_sizes_and_bases() {
    let mut rng = det_rng(5);
    for n in [0, 1, 7, 8, 9, 63, 65, 129] {
        for kind in [
            OtKind::Alsz(BaseOt::ChouOrlandi),
            OtKind::Kos(BaseOt::ChouOrlandi),
        ] {
            test_ot(kind, n, &mut rng, 97);
        }
    }
}

#[test]
fn kkrt() {
    let mut input = AesRng::new(&SEED);
    let inputs: Vec<Block> = (0..100).map(|_| input.block()).collect();
    let kind = OtKind::Kos(BaseOt::NaorPinkas);
    let mut s = kkrt_sender(kind, inputs.len(), SEED);
    let mut r = kkrt_receiver(kind, inputs.clone(), SEED);
    pump(&mut r, &mut s, &mut det_rng(6), 1500);
    let out = s.take_result().unwrap().unwrap();
    let outputs = r.take_result().unwrap().unwrap();
    for (i, input) in inputs.iter().enumerate() {
        assert_eq!(out.encoder.compute(&out.seeds[i], input), outputs[i]);
    }
    // A different input gives an unrelated output.
    let other = out.encoder.compute(&out.seeds[0], &inputs[1]);
    assert_ne!(other, outputs[0]);
}

/// Sets where `ilen` of the receiver's elements are also in the sender's
/// set, at shuffled positions.
fn psz_sets(alen: usize, blen: usize, ilen: usize, seed: u64) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut rng = AesRng::new(&SEED);
    let mut aset: Vec<Vec<u8>> = (0..alen)
        .map(|_| {
            let mut v = vec![0u8; 16];
            rng.fill_bytes(&mut v);
            v
        })
        .collect();
    let mut bset: Vec<Vec<u8>> = (0..blen)
        .map(|_| {
            let mut v = vec![0u8; 16];
            rng.fill_bytes(&mut v);
            v
        })
        .collect();
    aset[..ilen].clone_from_slice(&bset[..ilen]);
    let mut shuffler = det_rng(seed);
    shuffle(&mut aset, &mut shuffler);
    shuffle(&mut bset, &mut shuffler);
    (aset, bset)
}

fn shuffle<T>(v: &mut [T], rng: &mut DetRng) {
    use rand_core::Rng;
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

fn run_psz(
    aset: &[Vec<u8>],
    bset: &[Vec<u8>],
    seeds: (Block, Block),
    rng: &mut DetRng,
    max_chunk: usize,
) -> (Vec<Vec<u8>>, Transcript) {
    let mut s = PszSender::new(aset.to_vec(), seeds.0);
    let mut r = PszReceiver::new(bset.to_vec(), seeds.1);
    let t = pump(&mut s, &mut r, rng, max_chunk);
    assert!(s.is_done() && r.is_done(), "psz stalled");
    assert_eq!(s.result(), Some(Ok(())));
    (r.take_result().unwrap().unwrap(), t)
}

fn expected(aset: &[Vec<u8>], bset: &[Vec<u8>]) -> BTreeSet<Vec<u8>> {
    let a: BTreeSet<_> = aset.iter().cloned().collect();
    bset.iter().filter(|b| a.contains(*b)).cloned().collect()
}

/// The intersection is exact: no missing elements and no false positives.
#[test]
fn psz() {
    let mut cases = vec![(0, 0, 0), (1, 0, 0), (15, 15, 15), (100, 100, 10)];
    if !cfg!(debug_assertions) {
        cases.push((1000, 1000, 250));
    }
    let mut rng = det_rng(7);
    for (alen, blen, ilen) in cases {
        let (aset, bset) = psz_sets(alen, blen, ilen, alen as u64);
        let (got, _) = run_psz(&aset, &bset, (SEED, SEED), &mut rng, 4096);
        let got: BTreeSet<_> = got.into_iter().collect();
        assert_eq!(got.len(), ilen);
        assert_eq!(got, expected(&aset, &bset), "({alen}, {blen}, {ilen})");
    }
}

/// Network keys are 32 bytes, so the broker always takes the SHA-256 path;
/// short inputs take the zero-padding path.
#[test]
fn psz_mixed_lengths() {
    let elem = |i: u32, len: usize| {
        let mut v = i.to_be_bytes().to_vec();
        v.resize(len, 0xa5);
        v
    };
    let aset: Vec<Vec<u8>> = (0..40).map(|i| elem(i, 4 + (i as usize % 40))).collect();
    let bset: Vec<Vec<u8>> = (20..70).map(|i| elem(i, 4 + (i as usize % 40))).collect();
    let (got, _) = run_psz(&aset, &bset, ([3; 16], [4; 16]), &mut det_rng(8), 33);
    let got: BTreeSet<_> = got.into_iter().collect();
    assert_eq!(got, expected(&aset, &bset));
    assert_eq!(got.len(), 20);
}

/// The transcript doesn't depend on how the byte stream is split, and the
/// same seeds reproduce it exactly.
#[test]
fn psz_chunking_and_determinism() {
    let (aset, bset) = psz_sets(30, 25, 9, 9);
    let seeds = ([5; 16], [6; 16]);
    let (want, base) = run_psz(&aset, &bset, seeds, &mut det_rng(0), 1 << 20);
    for (seed, max_chunk) in [(1, 1), (2, 2), (3, 15), (4, 16), (5, 17), (6, 333)] {
        let (got, t) = run_psz(&aset, &bset, seeds, &mut det_rng(seed), max_chunk);
        assert_eq!(got, want);
        assert_eq!(t.a_to_b, base.a_to_b, "max_chunk {max_chunk}");
        assert_eq!(t.b_to_a, base.b_to_a, "max_chunk {max_chunk}");
        assert!(t.deliveries >= base.deliveries);
    }
    // Same chunking seed, same everything.
    let (_, t1) = run_psz(&aset, &bset, seeds, &mut det_rng(42), 50);
    let (_, t2) = run_psz(&aset, &bset, seeds, &mut det_rng(42), 50);
    assert_eq!(t1, t2);
    // Different protocol seeds, same intersection (in another cuckoo order),
    // different transcript.
    let (got, t3) = run_psz(&aset, &bset, ([7; 16], [8; 16]), &mut det_rng(0), 1 << 20);
    let got: BTreeSet<_> = got.into_iter().collect();
    assert_eq!(got, want.into_iter().collect::<BTreeSet<_>>());
    assert_ne!(t3.a_to_b, base.a_to_b);
}

#[test]
fn psz_from_det_rng() {
    let (aset, bset) = psz_sets(5, 5, 2, 1);
    let mut seeds = det_rng(77);
    let mut s = PszSender::from_rng(aset.clone(), &mut seeds);
    let mut r = PszReceiver::from_rng(bset.clone(), &mut seeds);
    pump(&mut s, &mut r, &mut det_rng(0), 4096);
    let got: BTreeSet<_> = r.take_result().unwrap().unwrap().into_iter().collect();
    assert_eq!(got, expected(&aset, &bset));
}

/// Drives `a` and `b` like `pump`, but lets `tamper` edit each batch of
/// bytes in flight. Returns the results' errors.
fn pump_tampered(
    a: &mut dyn Endpoint,
    b: &mut dyn Endpoint,
    mut tamper: impl FnMut(bool, usize, &mut Vec<u8>),
) {
    let mut buf = vec![0u8; 1 << 16];
    let mut offsets = [0usize; 2];
    loop {
        let mut moved = false;
        for dir in [true, false] {
            let (from, to): (&mut dyn Endpoint, &mut dyn Endpoint) = if dir {
                (&mut *a, &mut *b)
            } else {
                (&mut *b, &mut *a)
            };
            let mut batch = Vec::new();
            while let Some(n) = from.poll_transmit(&mut buf) {
                batch.extend_from_slice(&buf[..n]);
            }
            if batch.is_empty() {
                continue;
            }
            moved = true;
            let off = &mut offsets[usize::from(dir)];
            let len = batch.len();
            tamper(dir, *off, &mut batch);
            *off += len;
            let _ = to.handle_input(&batch);
        }
        if !moved || (a.is_done() && b.is_done()) {
            return;
        }
    }
}

/// Corrupting any single byte never panics, and is either detected or
/// leaves the protocol unfinished or with a wrong result, never a hang in
/// a busy loop.
#[test]
fn psz_tampering_never_panics() {
    let (aset, bset) = psz_sets(6, 6, 3, 3);
    let (_, t) = run_psz(&aset, &bset, ([3; 16], [4; 16]), &mut det_rng(0), 1 << 20);
    let mut errors = BTreeSet::new();
    let mut rng = det_rng(10);
    // (direction is sender-to-receiver, offset): a Chou-Orlandi point from
    // each side, a KOS setup row, the KOS check, then random spots.
    let mut targets = vec![(true, 5), (false, 100), (true, 5000), (true, 16600)];
    for i in 0..10 {
        let to_receiver = i % 2 == 0;
        let len = if to_receiver {
            t.a_to_b.len()
        } else {
            t.b_to_a.len()
        };
        targets.push((
            to_receiver,
            (rand_core::Rng::next_u64(&mut rng) % len as u64) as usize,
        ));
    }
    for (to_receiver, target) in targets {
        let mut s = PszSender::new(aset.clone(), [3; 16]);
        let mut r = PszReceiver::new(bset.clone(), [4; 16]);
        pump_tampered(&mut s, &mut r, |dir, off, batch| {
            if dir == to_receiver && (off..off + batch.len()).contains(&target) {
                batch[target - off] ^= 0x41;
            }
        });
        for e in [
            s.result().and_then(Result::err),
            r.result().and_then(|r| r.err().cloned()),
        ]
        .into_iter()
        .flatten()
        {
            errors.insert(format!("{e:?}"));
        }
    }
    // Both the point check and the KOS consistency check fire.
    assert!(
        errors.iter().any(|e| e.contains("InvalidPoint")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("ConsistencyCheckFailed")),
        "{errors:?}"
    );
}

#[test]
fn psz_commitment_check() {
    // The receiver's coin toss rejects a sender that opens a different seed.
    let (aset, bset) = psz_sets(3, 3, 1, 1);
    let (_, t) = run_psz(&aset, &bset, ([1; 16], [2; 16]), &mut det_rng(0), 1 << 20);
    // KKRT's cointoss: after the CO point (32) and 128 x 2 CO ciphertexts
    // the sender commits to 4 seeds, then opens them after the receiver
    // answers. Flip a bit in the first opened seed.
    let open = 32 + 128 * 32 + 4 * 16;
    let mut s = PszSender::new(aset, [1; 16]);
    let mut r = PszReceiver::new(bset, [2; 16]);
    pump_tampered(&mut s, &mut r, |dir, off, batch| {
        if dir && (off..off + batch.len()).contains(&open) {
            batch[open - off] ^= 1;
        }
    });
    assert!(t.a_to_b.len() > open);
    assert_eq!(
        r.result().unwrap().unwrap_err(),
        &Error::CommitmentCheckFailed
    );
}

/// Offset of the PSZ receiver's set size in its byte stream: after the
/// Chou-Orlandi points for 128 base OTs (128 * 32), the KKRT coin toss
/// seeds (4 * 16), and the KOS sender's coin toss (commitment and opening,
/// 2 * 16) and 512 ciphertext pairs (512 * 32).
const RECEIVER_SIZE_OFFSET: usize = 128 * 32 + 4 * 16 + 2 * 16 + 512 * 32;

#[test]
fn wire_layout() {
    let (aset, bset) = psz_sets(4, 3, 1, 1);
    let (_, t) = run_psz(&aset, &bset, ([1; 16], [2; 16]), &mut det_rng(0), 1 << 20);
    let size = &t.b_to_a[RECEIVER_SIZE_OFFSET..RECEIVER_SIZE_OFFSET + 4];
    assert_eq!(size, 3u32.to_be_bytes());
    // The sender's size follows the Chou-Orlandi point, its 128 x 2
    // ciphertexts, the KKRT coin toss (commitments and openings) and the
    // KOS receiver's setup: 128 rows of 96 bytes, its coin toss seed and
    // the 48 byte consistency check.
    let offset = 32 + 128 * 32 + 2 * 4 * 16 + 128 * 96 + 16 + 48;
    assert_eq!(&t.a_to_b[offset..offset + 4], 4u32.to_be_bytes());
    // It ends with 3 masks of 7 bytes per element.
    // Then the PSZ coin toss and 3 masks of 7 bytes per element; the KKRT
    // sender only reads.
    assert_eq!(t.a_to_b.len() - offset, 4 + 2 * 16 + 3 * 4 * 7);
}

#[test]
fn peer_set_size_is_bounded() {
    // A receiver announcing MAX_PEER_SET_SIZE + 1 elements is refused
    // before the sender allocates anything for it.
    let forged = u32::try_from(MAX_PEER_SET_SIZE + 1).unwrap().to_be_bytes();
    let mut s = PszSender::new(vec![vec![1]], [1; 16]);
    let mut r = PszReceiver::new(vec![vec![1]], [2; 16]);
    pump_tampered(&mut s, &mut r, |dir, off, batch| {
        for (i, b) in forged.iter().enumerate() {
            let at = RECEIVER_SIZE_OFFSET + i;
            if !dir && (off..off + batch.len()).contains(&at) {
                batch[at - off] = *b;
            }
        }
    });
    assert_eq!(
        s.result(),
        Some(Err(Error::PeerSetTooLarge {
            size: MAX_PEER_SET_SIZE + 1,
            max: MAX_PEER_SET_SIZE
        }))
    );
}

#[test]
fn input_after_done_is_rejected() {
    let (aset, bset) = psz_sets(2, 2, 1, 1);
    let mut s = PszSender::new(aset, [1; 16]);
    let mut r = PszReceiver::new(bset, [2; 16]);
    pump(&mut s, &mut r, &mut det_rng(0), 4096);
    assert!(s.is_done() && r.is_done());
    assert_eq!(r.handle_input(&[0]), Err(Error::UnexpectedInput));
    assert_eq!(s.handle_input(&[0]), Err(Error::UnexpectedInput));
    assert_eq!(s.handle_input(&[]), Ok(()));
    // The outcome is unaffected.
    assert_eq!(r.result().unwrap().unwrap().len(), 1);
}

#[test]
fn trailing_input_is_rejected() {
    // A sender that sends one byte more than the protocol reads.
    let (aset, bset) = psz_sets(2, 2, 1, 1);
    let mut s = PszSender::new(aset, [1; 16]);
    let mut r = PszReceiver::new(bset, [2; 16]);
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let mut moved = false;
        let mut batch = Vec::new();
        while let Some(n) = s.poll_transmit(&mut buf) {
            batch.extend_from_slice(&buf[..n]);
        }
        if s.is_done() && !batch.is_empty() {
            batch.push(0);
        }
        if !batch.is_empty() {
            moved = true;
            let _ = r.handle_input(&batch);
        }
        while let Some(n) = r.poll_transmit(&mut buf) {
            moved = true;
            s.handle_input(&buf[..n]).unwrap();
        }
        if !moved {
            break;
        }
    }
    assert_eq!(r.result().unwrap().unwrap_err(), &Error::UnexpectedInput);
}

#[test]
fn poll_transmit_respects_buffer() {
    let mut s = PszSender::new(vec![], [1; 16]);
    assert_eq!(s.poll_transmit(&mut []), None);
    let mut one = [0u8; 1];
    let mut first = Vec::new();
    while let Some(n) = s.poll_transmit(&mut one) {
        assert_eq!(n, 1);
        first.push(one[0]);
    }
    // The Chou-Orlandi point, then the sender waits.
    assert_eq!(first.len(), 32);
    assert!(!s.is_done());
    assert_eq!(s.result(), None);
}

/// The cuckoo table can't be built for a few percent of small sets: the
/// receiver aborts, and the sender is left waiting for KKRT input that never
/// comes. Drivers need a deadline.
#[test]
fn receiver_cuckoo_failure_leaves_sender_waiting() {
    let (aset, bset) = psz_sets(6, 6, 3, 3);
    let mut s = PszSender::new(aset, [1; 16]);
    let mut r = PszReceiver::new(bset, [2; 16]);
    pump(&mut s, &mut r, &mut det_rng(0), 4096);
    assert_eq!(r.result(), Some(Err(&Error::CuckooHashMapFull)));
    assert!(r.is_done());
    assert!(!s.is_done());
    assert_eq!(s.result(), None);
}

/// Drivers can move sessions between threads (e.g. into a spawned task).
#[test]
fn sessions_are_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PszSender>();
    assert_send::<PszReceiver>();
    assert_send::<Session<Vec<Block>>>();
    assert_send::<Session<KkrtSenderOutput>>();
}
