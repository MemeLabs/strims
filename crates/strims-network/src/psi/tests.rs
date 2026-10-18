// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Inline and remote sessions against each other, the remote engine over
//! an in-process queue to a server, and the codec's bounds.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use rand_core::Rng;
use strims_base::rng::{DetRng, det_rng};
use strims_mpc::MAX_PEER_SET_SIZE;

use super::codec::{self, MAX_DATA, MAX_ERROR_LEN, MAX_KEY_LEN, MAX_KEYS, MAX_MESSAGE_SIZE};
use super::*;

/// Messages queued for a server.
#[derive(Clone, Default)]
struct Queue(Rc<RefCell<VecDeque<Vec<u8>>>>);

impl PsiTransport for Queue {
    fn send(&self, msg: Vec<u8>) {
        self.0.borrow_mut().push_back(msg);
    }
}

impl Queue {
    fn take(&self) -> Vec<Vec<u8>> {
        self.0.borrow_mut().drain(..).collect()
    }
}

/// A remote engine and its server, joined by a queue.
struct Served {
    engine: RemotePsi,
    queue: Queue,
    server: RefCell<PsiServer>,
}

impl Served {
    fn new() -> Served {
        let queue = Queue::default();
        Served {
            engine: RemotePsi::new(queue.clone()),
            queue,
            server: RefCell::new(PsiServer::new()),
        }
    }

    /// Delivers what is queued for the server, and its answers. Returns
    /// whether anything moved.
    fn flush(&self) -> bool {
        let mut moved = false;
        loop {
            let msgs = self.queue.take();
            if msgs.is_empty() {
                return moved;
            }
            moved = true;
            for m in msgs {
                let replies = self.server.borrow_mut().handle(&m);
                assert_one_step(&replies);
                for r in replies {
                    self.engine.receive(&r);
                }
            }
        }
    }
}

/// Every message of a server's step but the last says more follows; the
/// last is a transmit that says none does, or the outcome.
fn assert_one_step(replies: &[Vec<u8>]) {
    for (i, r) in replies.iter().enumerate() {
        let last = i + 1 == replies.len();
        match codec::decode_reply(r) {
            Some(codec::Reply::Transmit { more, .. }) => assert_eq!(more, !last, "message {i}"),
            Some(codec::Reply::Outcome { .. }) => assert!(last, "an outcome before the end"),
            None => panic!("a malformed reply"),
        }
    }
}

/// A ready handle that counts its notifications.
fn counter() -> (PsiReady, Rc<Cell<usize>>) {
    let n = Rc::new(Cell::new(0));
    let c = n.clone();
    (PsiReady::new(move || c.set(c.get() + 1)), n)
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    sender: Option<PsiOutcome>,
    receiver: Option<PsiOutcome>,
    sender_bytes: Vec<u8>,
    receiver_bytes: Vec<u8>,
}

/// Runs a sender against a receiver, each on `inline` or a remote engine,
/// moving every byte in one call per direction and round.
fn run(
    sender_remote: bool,
    receiver_remote: bool,
    keys: (Vec<Vec<u8>>, Vec<Vec<u8>>),
    seeds: ([u8; 16], [u8; 16]),
    max_peer_keys: usize,
) -> (Run, usize) {
    let remotes = (Served::new(), Served::new());
    let (ready, notified) = counter();
    let start = |remote: bool, engine: &Served, role, keys, seed| {
        if remote {
            engine
                .engine
                .start(role, keys, seed, max_peer_keys, ready.clone())
        } else {
            InlinePsi.start(role, keys, seed, max_peer_keys, ready.clone())
        }
    };
    let mut s = start(sender_remote, &remotes.0, PsiRole::Sender, keys.0, seeds.0);
    let mut r = start(
        receiver_remote,
        &remotes.1,
        PsiRole::Receiver,
        keys.1,
        seeds.1,
    );
    let mut out = Run {
        sender: None,
        receiver: None,
        sender_bytes: Vec::new(),
        receiver_bytes: Vec::new(),
    };
    let drain = |p: &mut dyn PsiSession, log: &mut Vec<u8>| {
        let start = log.len();
        let mut buf = vec![0u8; 1400];
        while let Some(n) = p.poll_transmit(&mut buf) {
            log.extend_from_slice(&buf[..n]);
        }
        log[start..].to_vec()
    };
    loop {
        let mut moved = remotes.0.flush() | remotes.1.flush();
        let to_r = drain(s.as_mut(), &mut out.sender_bytes);
        if !to_r.is_empty() {
            moved = true;
            r.handle_input(&to_r);
        }
        moved |= remotes.1.flush();
        let to_s = drain(r.as_mut(), &mut out.receiver_bytes);
        if !to_s.is_empty() {
            moved = true;
            s.handle_input(&to_s);
        }
        moved |= remotes.0.flush();
        if out.sender.is_none() {
            out.sender = s.poll_outcome();
        }
        if out.receiver.is_none() {
            out.receiver = r.poll_outcome();
        }
        let done = out.sender.is_some() && out.receiver.is_some();
        if done || !moved {
            // the outcome's last bytes
            out.sender_bytes.extend(drain(s.as_mut(), &mut Vec::new()));
            out.receiver_bytes
                .extend(drain(r.as_mut(), &mut Vec::new()));
            assert!(s.poll_outcome().is_none(), "an outcome returned twice");
            return (out, notified.get());
        }
    }
}

fn keys(rng: &mut DetRng, n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|_| {
            let mut k = vec![0u8; 32];
            rng.fill_bytes(&mut k);
            k
        })
        .collect()
}

/// A sender with `s` keys and a receiver with `r`, `shared` of them in
/// both sets.
fn sets(seed: u64, s: usize, r: usize, shared: usize) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut rng = det_rng(seed);
    let common = keys(&mut rng, shared);
    let mut a = common.clone();
    a.extend(keys(&mut rng, s - shared));
    let mut b = keys(&mut rng, r - shared);
    b.extend(common);
    (a, b)
}

/// Remote sessions, in either role or both, give the same bytes and the
/// same intersection as inline ones with the same seeds, and only remote
/// sessions notify.
#[test]
fn remote_sessions_match_inline_ones() {
    // the last pair's messages are longer than one codec message
    let cases = [
        (1, 1, 1),
        (5, 3, 2),
        (12, 12, 7),
        (40, 9, 0),
        (2000, 1100, 600),
    ];
    for (i, (s, r, shared)) in cases.into_iter().enumerate() {
        let seed = i as u64;
        let mut rng = det_rng(100 + seed);
        let mut seeds = ([0u8; 16], [0u8; 16]);
        rng.fill_bytes(&mut seeds.0);
        rng.fill_bytes(&mut seeds.1);
        let (inline, notified) = run(false, false, sets(seed, s, r, shared), seeds, 4096);
        assert_eq!(notified, 0, "inline sessions notified");
        assert_eq!(inline.sender, Some(PsiOutcome::Sent), "case {i}");
        match &inline.receiver {
            Some(PsiOutcome::Shared(k)) => assert_eq!(k.len(), shared, "case {i}"),
            Some(PsiOutcome::CuckooFailed) => {}
            o => panic!("case {i}: {o:?}"),
        }
        for (sr, rr) in [(true, true), (true, false), (false, true)] {
            let (remote, notified) = run(sr, rr, sets(seed, s, r, shared), seeds, 4096);
            assert_eq!(remote, inline, "case {i}, remote {sr} {rr}");
            assert!(notified > 0, "case {i}: remote sessions never notified");
        }
    }
}

/// A peer announcing more keys than the limit fails the session, inline
/// and remote alike.
#[test]
fn the_peer_key_limit_holds_remotely() {
    let seeds = ([1; 16], [2; 16]);
    let (inline, _) = run(false, false, sets(9, 20, 10, 3), seeds, 15);
    let (remote, _) = run(true, true, sets(9, 20, 10, 3), seeds, 15);
    assert_eq!(remote, inline);
    assert!(
        matches!(inline.receiver, Some(PsiOutcome::Failed(_))),
        "{inline:?}"
    );
}

fn start_msg(id: u64, keys: &[Vec<u8>]) -> Vec<u8> {
    codec::encode_start(id, PsiRole::Sender, &[7; 16], 256, keys).unwrap()
}

/// Bytes that don't decode, or that name no session, change nothing on
/// either end.
#[test]
fn malformed_and_unknown_messages_are_ignored() {
    let mut server = PsiServer::new();
    let mut rng = det_rng(5);
    let valid = start_msg(1, &keys(&mut rng, 3));
    let mut garbage: Vec<Vec<u8>> = vec![
        vec![],
        vec![0x01],
        valid[..valid.len() - 1].to_vec(),
        [valid.clone(), vec![0]].concat(),
        codec::encode_input(9, b"for nobody"),
        codec::encode_close(9),
        // a client's message, sent to the server
        codec::encode_transmit(1, false, b"x"),
        codec::encode_outcome(1, &PsiOutcome::Sent),
        // an unknown role
        {
            let mut m = valid.clone();
            m[9] = 7;
            m
        },
    ];
    for _ in 0..500 {
        let len = (rng.next_u64() % 64) as usize;
        let mut m = vec![0u8; len];
        rng.fill_bytes(&mut m);
        garbage.push(m);
    }
    for g in &garbage {
        assert_eq!(server.handle(g), Vec::<Vec<u8>>::new(), "{g:?}");
    }
    assert_eq!(server.len(), 0);

    // a live session on each end ignores them too
    let queue = Queue::default();
    let engine = RemotePsi::new(queue.clone());
    let (ready, notified) = counter();
    let mut session = engine.start(PsiRole::Sender, keys(&mut rng, 3), [3; 16], 256, ready);
    let start = queue.take();
    assert_eq!(start.len(), 1);
    let replies = server.handle(&start[0]);
    assert!(!replies.is_empty());
    for g in &garbage {
        assert_eq!(server.handle(g), Vec::<Vec<u8>>::new(), "{g:?}");
    }
    assert_eq!(server.len(), 1);
    let replies_garbage = [
        vec![],
        codec::encode_transmit(5, false, b"not yours"),
        codec::encode_outcome(5, &PsiOutcome::Sent),
        // a server's message, sent to the client
        codec::encode_input(0, b"x"),
        start[0].clone(),
        // a transmit with no bytes
        replies[0][..9].to_vec(),
    ];
    for g in replies_garbage.iter().chain(&garbage) {
        engine.receive(g);
    }
    assert_eq!(notified.get(), 0);
    assert_eq!(session.poll_transmit(&mut [0u8; 64]), None);
    assert_eq!(session.poll_outcome(), None);
    for r in replies {
        engine.receive(&r);
    }
    assert!(notified.get() > 0);
    assert!(session.poll_transmit(&mut [0u8; 64]).is_some());
}

/// The codec refuses fields over their bounds.
#[test]
fn codec_bounds_hold() {
    let key = |n: usize| vec![1u8; n];
    // keys
    assert!(codec::encode_start(1, PsiRole::Sender, &[0; 16], 1, &[key(MAX_KEY_LEN)]).is_some());
    assert!(
        codec::encode_start(1, PsiRole::Sender, &[0; 16], 1, &[key(MAX_KEY_LEN + 1)]).is_none()
    );
    let many = vec![key(1); MAX_KEYS + 1];
    assert!(codec::encode_start(1, PsiRole::Sender, &[0; 16], 1, &many).is_none());
    let most = vec![key(MAX_KEY_LEN); MAX_KEYS];
    let m = codec::encode_start(1, PsiRole::Receiver, &[0; 16], 1, &most).unwrap();
    assert_eq!(m.len(), MAX_MESSAGE_SIZE);
    assert!(codec::decode_request(&m).is_some());
    assert!(codec::decode_request(&[m.clone(), vec![0]].concat()).is_none());

    // a forged key count, key length and peer limit
    let m = start_msg(1, &[key(4)]);
    let count_at = 1 + 8 + 1 + 16 + 4;
    let mut forged = m.clone();
    forged[count_at..count_at + 4].copy_from_slice(&(MAX_KEYS as u32 + 1).to_le_bytes());
    assert!(codec::decode_request(&forged).is_none());
    let mut forged = m.clone();
    forged[count_at + 4..count_at + 6].copy_from_slice(&(MAX_KEY_LEN as u16 + 1).to_le_bytes());
    assert!(codec::decode_request(&forged).is_none());
    let mut forged = m.clone();
    forged[count_at - 4..count_at].copy_from_slice(&(MAX_PEER_SET_SIZE as u32 + 1).to_le_bytes());
    assert!(codec::decode_request(&forged).is_none());
    // the peer limit is capped as the sessions cap it
    let m = codec::encode_start(1, PsiRole::Sender, &[0; 16], usize::MAX, &[]).unwrap();
    assert!(matches!(
        codec::decode_request(&m),
        Some(codec::Request::Start {
            max_peer_keys: MAX_PEER_SET_SIZE,
            ..
        })
    ));

    // stream bytes
    for (input, transmit) in [
        (
            codec::encode_input(1, &[]),
            codec::encode_transmit(1, false, &[]),
        ),
        (
            codec::encode_input(1, &vec![0; MAX_DATA + 1]),
            codec::encode_transmit(1, false, &vec![0; MAX_DATA + 1]),
        ),
    ] {
        assert!(codec::decode_request(&input).is_none());
        assert!(codec::decode_reply(&transmit).is_none());
    }
    let mut forged = codec::encode_transmit(1, true, b"x");
    forged[9] = 2;
    assert!(codec::decode_reply(&forged).is_none());
    let full = vec![0; MAX_DATA];
    assert!(codec::decode_request(&codec::encode_input(1, &full)).is_some());
    assert!(codec::decode_reply(&codec::encode_transmit(1, false, &full)).is_some());

    // failure messages: cut on encoding, refused over the bound
    let long = "é".repeat(MAX_ERROR_LEN);
    let m = codec::encode_outcome(1, &PsiOutcome::Failed(long));
    let Some(codec::Reply::Outcome {
        outcome: PsiOutcome::Failed(e),
        ..
    }) = codec::decode_reply(&m)
    else {
        panic!("a cut failure decodes");
    };
    assert_eq!(e, "é".repeat(MAX_ERROR_LEN / 2));
    let mut forged = codec::encode_outcome(1, &PsiOutcome::Failed(String::new()));
    forged.extend(vec![b'a'; MAX_ERROR_LEN + 1]);
    assert!(codec::decode_reply(&forged).is_none());

    // any message over the largest
    let mut huge = codec::encode_close(1);
    huge.resize(MAX_MESSAGE_SIZE + 1, 0);
    assert!(codec::decode_request(&huge).is_none());
}

/// A server runs at most `MAX_SERVER_SESSIONS` sessions: a start beyond
/// them fails at once, and a closed session frees its place.
#[test]
fn the_server_bounds_its_sessions() {
    let mut server = PsiServer::new();
    let mut rng = det_rng(3);
    for id in 0..MAX_SERVER_SESSIONS as u64 {
        let replies = server.handle(&start_msg(id, &keys(&mut rng, 2)));
        assert!(!replies.is_empty());
    }
    assert_eq!(server.len(), MAX_SERVER_SESSIONS);
    let busy = server.handle(&start_msg(99, &keys(&mut rng, 2)));
    assert_eq!(
        busy,
        [codec::encode_outcome(
            99,
            &PsiOutcome::Failed("psi server busy".into())
        )]
    );
    // a second start for a running id is dropped
    assert!(server.handle(&start_msg(0, &keys(&mut rng, 2))).is_empty());
    assert!(server.handle(&codec::encode_close(0)).is_empty());
    assert_eq!(server.len(), MAX_SERVER_SESSIONS - 1);
    assert!(!server.handle(&start_msg(99, &keys(&mut rng, 2))).is_empty());
    assert_eq!(server.len(), MAX_SERVER_SESSIONS);
}

/// A remote session fails, and notifies, when its keys don't fit a
/// message, when the server sends more bytes than it holds, or when a
/// shared outcome holds more keys than it started with.
#[test]
fn a_remote_session_fails_over_its_bounds() {
    let queue = Queue::default();
    let engine = RemotePsi::new(queue.clone());

    let (ready, notified) = counter();
    let mut s = engine.start(
        PsiRole::Sender,
        vec![vec![0; MAX_KEY_LEN + 1]],
        [0; 16],
        8,
        ready,
    );
    assert!(
        queue.take().is_empty(),
        "a start that breaks the bounds went out"
    );
    assert_eq!(notified.get(), 1);
    assert!(matches!(s.poll_outcome(), Some(PsiOutcome::Failed(_))));
    drop(s);
    assert!(queue.take().is_empty(), "a finished session sent a close");

    let (ready, notified) = counter();
    let mut s = engine.start(PsiRole::Receiver, vec![vec![1; 32]], [0; 16], 8, ready);
    let id = 1;
    queue.take();
    let chunk = vec![5u8; MAX_DATA];
    for _ in 0..MAX_PENDING_TRANSMIT / MAX_DATA {
        engine.receive(&codec::encode_transmit(id, false, &chunk));
    }
    assert_eq!(s.poll_outcome(), None);
    engine.receive(&codec::encode_transmit(id, false, &[1]));
    assert!(matches!(s.poll_outcome(), Some(PsiOutcome::Failed(_))));
    assert_eq!(s.poll_transmit(&mut [0; 16]), None);
    assert_eq!(queue.take(), [codec::encode_close(id)]);
    assert_eq!(notified.get(), MAX_PENDING_TRANSMIT / MAX_DATA + 1);
    // the session is finished: more bytes are dropped
    engine.receive(&codec::encode_transmit(id, false, &[1]));
    assert_eq!(notified.get(), MAX_PENDING_TRANSMIT / MAX_DATA + 1);
    drop(s);
    assert!(queue.take().is_empty());

    let (ready, _) = counter();
    let mut s = engine.start(PsiRole::Receiver, vec![vec![1; 32]], [0; 16], 8, ready);
    let id = 2;
    queue.take();
    engine.receive(&codec::encode_outcome(
        id,
        &PsiOutcome::Shared(vec![vec![1; 32], vec![2; 32]]),
    ));
    assert!(matches!(s.poll_outcome(), Some(PsiOutcome::Failed(_))));
    drop(s);
    assert!(queue.take().is_empty());
}

/// A remote session's bytes from one server step become visible with the
/// step's last message, and a last step's with its outcome: the peer
/// can't read a session's last bytes before this side has the outcome.
#[test]
fn a_remote_step_becomes_visible_whole() {
    let queue = Queue::default();
    let engine = RemotePsi::new(queue.clone());
    let (ready, notified) = counter();
    let mut s = engine.start(PsiRole::Sender, vec![vec![1; 32]], [0; 16], 8, ready);
    let mut buf = [0u8; 16];
    engine.receive(&codec::encode_transmit(0, true, b"ab"));
    engine.receive(&codec::encode_transmit(0, true, b"cd"));
    assert_eq!(s.poll_transmit(&mut buf), None);
    assert_eq!(notified.get(), 0);
    engine.receive(&codec::encode_transmit(0, false, b"ef"));
    assert_eq!(notified.get(), 1);
    assert_eq!(s.poll_transmit(&mut buf), Some(6));
    assert_eq!(&buf[..6], b"abcdef");

    engine.receive(&codec::encode_transmit(0, true, b"gh"));
    assert_eq!(s.poll_transmit(&mut buf), None);
    assert_eq!(s.poll_outcome(), None);
    engine.receive(&codec::encode_outcome(0, &PsiOutcome::Sent));
    assert_eq!(notified.get(), 2);
    assert_eq!(s.poll_outcome(), Some(PsiOutcome::Sent));
    assert_eq!(s.poll_transmit(&mut buf), Some(2));
    assert_eq!(&buf[..2], b"gh");
}

/// Dropping a remote session before its outcome closes the server's
/// side; input after the outcome isn't sent.
#[test]
fn a_dropped_remote_session_closes_the_servers_side() {
    let remote = Served::new();
    let (ready, _) = counter();
    let mut rng = det_rng(4);
    let s = remote.engine.start(
        PsiRole::Sender,
        keys(&mut rng, 3),
        [1; 16],
        8,
        ready.clone(),
    );
    remote.flush();
    assert_eq!(remote.server.borrow().len(), 1);
    drop(s);
    remote.flush();
    assert_eq!(remote.server.borrow().len(), 0);

    let mut s = remote
        .engine
        .start(PsiRole::Sender, keys(&mut rng, 3), [1; 16], 8, ready);
    remote.flush();
    // garbage from the peer fails the session on the server
    s.handle_input(&[0xff; 64]);
    remote.flush();
    assert!(matches!(s.poll_outcome(), Some(PsiOutcome::Failed(_))));
    assert_eq!(remote.server.borrow().len(), 0);
    s.handle_input(&[0xff; 64]);
    assert!(remote.queue.take().is_empty());
}

/// The server can move to another thread.
#[test]
fn the_server_is_send() {
    fn send<T: Send>() {}
    send::<PsiServer>();
}
