// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Runner tests: two runners wired back to back with instant delivery.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::codec::{ChannelWriter, Have, MessageWriter};
use crate::{SwarmId, SwarmOptions, WriterOptions, store};
use strims_base::rng::det_rng;

const MTU: usize = 16 * 1024 - 4;
const MUNRO: usize = 64 * 1024;

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

fn byte(o: u64) -> u8 {
    (o.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 56) as u8
}

fn options() -> SwarmOptions {
    SwarmOptions {
        live_window: 1 << 10,
        ..Default::default()
    }
}

struct Node {
    runner: Runner,
    swarm: SwarmKey,
    peer: PeerKey,
    reader: store::BufferReader,
}

struct Pair {
    seed: Node,
    leech: Node,
    writer: Writer,
    now: Time,
    written: u64,
    /// Every frame sent, in order.
    frames: Vec<Vec<u8>>,
}

impl Pair {
    fn new(leech_options: SwarmOptions) -> Pair {
        Pair::with(options(), leech_options)
    }

    fn with(seed_options: SwarmOptions, leech_options: SwarmOptions) -> Pair {
        let key = strims_crypto::key_from_seed(&[4; 32]);
        let (writer, swarm) = Writer::new(
            WriterOptions {
                swarm_options: seed_options,
                key: key.clone(),
            },
            t(0),
        )
        .unwrap();
        let mut seed = Runner::new(det_rng(1));
        let seed_reader = swarm.reader();
        let ss = seed.add_swarm(swarm);
        let sp = seed.add_peer(b"leech", t(0));
        seed.run_swarm(sp, ss, 1, 2, t(0)).unwrap();

        let swarm = Swarm::new(SwarmId::new(&key.public), leech_options).unwrap();
        let reader = swarm.reader();
        let mut leech = Runner::new(det_rng(2));
        let ls = leech.add_swarm(swarm);
        let lp = leech.add_peer(b"seed", t(0));
        leech.run_swarm(lp, ls, 2, 1, t(0)).unwrap();
        Pair {
            seed: Node {
                runner: seed,
                swarm: ss,
                peer: sp,
                reader: seed_reader,
            },
            leech: Node {
                runner: leech,
                swarm: ls,
                peer: lp,
                reader,
            },
            writer,
            now: t(0),
            written: 0,
            frames: Vec::new(),
        }
    }

    fn pump(&mut self) -> Result<(), RunnerError> {
        let mut buf = vec![0u8; MTU];
        loop {
            let mut moved = false;
            for (from, to) in [(0, 1), (1, 0)] {
                let (a, b) = if from == 0 {
                    (&mut self.seed, &mut self.leech)
                } else {
                    (&mut self.leech, &mut self.seed)
                };
                while a.runner.wants_transmit(a.peer) {
                    let n = a.runner.poll_transmit(a.peer, self.now, &mut buf);
                    if n == 0 {
                        break;
                    }
                    moved = true;
                    self.frames.push(buf[..n].to_vec());
                    b.runner.handle_frame(b.peer, self.now, &buf[..n])?;
                }
                let _ = to;
            }
            if !moved {
                return Ok(());
            }
        }
    }

    fn tick(&mut self, ms: i64) {
        self.now += Duration::from_millis(ms);
        self.seed.runner.handle_timeout(self.now);
        self.leech.runner.handle_timeout(self.now);
        self.pump().unwrap();
    }

    fn write(&mut self, munros: usize) {
        for _ in 0..munros {
            let data: Vec<u8> = (self.written..self.written + MUNRO as u64)
                .map(byte)
                .collect();
            self.seed
                .runner
                .write(self.seed.swarm, &mut self.writer, self.now, &data)
                .unwrap();
            self.written += MUNRO as u64;
        }
    }

    /// Reads what the leech has; returns (start, bytes) of this read run.
    fn read(&mut self) -> Result<Vec<u8>, store::Error> {
        let n = &mut self.leech;
        let swarm = n.runner.swarm(n.swarm).unwrap();
        let mut out = vec![];
        let mut buf = vec![0u8; MUNRO];
        loop {
            match swarm.read(&mut n.reader, &mut buf) {
                Ok(k) => out.extend_from_slice(&buf[..k]),
                Err(store::Error::WouldBlock) => return Ok(out),
                Err(e) => return Err(e),
            }
        }
    }
}

#[test]
fn transfer_between_runners() {
    let mut p = Pair::new(options());
    p.tick(100);
    assert!(
        p.leech
            .runner
            .channel_stats(p.leech.peer, p.leech.swarm)
            .unwrap()
            .scheduler
            .handshake_received
    );
    for _ in 0..20 {
        p.write(1);
        for _ in 0..5 {
            p.tick(50);
        }
    }
    let swarm = p.leech.runner.swarm(p.leech.swarm).unwrap();
    let start = p.leech.reader.offset(swarm.store()).unwrap();
    let got = p.read().unwrap();
    assert_eq!(start + got.len() as u64, p.written);
    assert!(
        got.iter()
            .enumerate()
            .all(|(i, &b)| b == byte(start + i as u64))
    );

    let m = p.leech.runner.peer_metrics(p.leech.peer, p.now).unwrap();
    assert!(m.metrics.read.count >= got.len() as u64);
    assert_eq!(m.swarms.len(), 1);
    let m = p.seed.runner.peer_metrics(p.seed.peer, p.now).unwrap();
    assert!(m.metrics.write.count >= got.len() as u64);
    // The seed's reader is unused; it would start at the seed's tail.
    let _ = p.seed.reader;
}

#[test]
fn writer_reset_restarts_the_stream() {
    // After a writer reset the stream restarts at bin 0 under a new epoch.
    // The leech must forget the seed's old newest HAVE, or it would ask for
    // nothing until the new stream passed the old position.
    let mut p = Pair::new(options());
    p.tick(100);
    p.write(8);
    for _ in 0..20 {
        p.tick(50);
    }
    assert!(!p.read().unwrap().is_empty());

    p.now += Duration::from_millis(10);
    p.seed
        .runner
        .reset_writer(p.seed.swarm, &mut p.writer, p.now)
        .unwrap();
    p.written = 0;
    for _ in 0..10 {
        p.write(1);
        for _ in 0..5 {
            p.tick(50);
        }
    }
    assert_eq!(p.read(), Err(store::Error::StreamReset));
    let swarm = p.leech.runner.swarm(p.leech.swarm).unwrap();
    p.leech.reader = swarm.reader();
    let start = p.leech.reader.offset(swarm.store()).unwrap();
    let got = p.read().unwrap();
    assert!(start < 2 * MUNRO as u64, "{start}");
    assert_eq!(start + got.len() as u64, p.written);
    assert!(
        got.iter()
            .enumerate()
            .all(|(i, &b)| b == byte(start + i as u64))
    );
}

#[test]
fn incompatible_handshake_fails_the_frame() {
    let mut p = Pair::new(SwarmOptions {
        chunk_size: 512,
        ..options()
    });
    p.now = t(100);
    p.seed.runner.handle_timeout(p.now);
    let e = p.pump().unwrap_err();
    assert_eq!(
        e,
        RunnerError::Handshake(HandshakeError::IncompatibleChunkSize)
    );
}

#[test]
fn run_swarm_errors() {
    let mut p = Pair::new(options());
    let r = &mut p.leech.runner;
    let (s, peer) = (p.leech.swarm, p.leech.peer);
    assert_eq!(
        r.run_swarm(peer, s, 9, 9, t(0)),
        Err(RunnerError::ChannelRunning)
    );
    let other = r.add_swarm(Swarm::new(SwarmId::new(&[1; 32]), options()).unwrap());
    assert_eq!(
        r.run_swarm(peer, other, 2, 9, t(0)),
        Err(RunnerError::ChannelInUse)
    );
    r.run_swarm(peer, other, 3, 9, t(0)).unwrap();
    let gone = r.add_peer(b"x", t(0));
    r.remove_peer(gone);
    assert_eq!(
        r.run_swarm(gone, s, 4, 4, t(0)),
        Err(RunnerError::UnknownPeer)
    );
    assert!(r.remove_swarm(other).is_some());
    assert_eq!(
        r.run_swarm(peer, other, 4, 4, t(0)),
        Err(RunnerError::UnknownSwarm)
    );
    assert_eq!(
        r.handle_frame(gone, t(0), &[]),
        Err(RunnerError::UnknownPeer)
    );
}

#[test]
fn scheduler_lives_with_its_channels() {
    let mut p = Pair::new(options());
    p.tick(100);
    assert!(p.leech.runner.swarm_stats(p.leech.swarm).is_some());
    p.leech.runner.stop_swarm(p.leech.peer, p.leech.swarm);
    assert!(p.leech.runner.swarm_stats(p.leech.swarm).is_none());
    assert!(
        p.leech
            .runner
            .channel_stats(p.leech.peer, p.leech.swarm)
            .is_none()
    );
    // Frames for the stopped channel are skipped.
    p.pump().unwrap();
    p.leech.runner.remove_peer(p.leech.peer);
    assert_eq!(
        p.leech
            .runner
            .poll_transmit(p.leech.peer, p.now, &mut [0; 64]),
        0
    );
    assert!(p.leech.runner.peer_stats(p.leech.peer).is_none());
}

#[test]
fn unknown_channels_are_skipped() {
    let mut p = Pair::new(options());
    let mut frame = vec![0u8; 64];
    let mut cw = ChannelWriter::new(&mut frame, 77);
    cw.write(&Message::Have(Have { bin: Bin(4) })).unwrap();
    let n = cw.finish();
    p.leech
        .runner
        .handle_frame(p.leech.peer, t(0), &frame[..n])
        .unwrap();
    // A truncated channel header fails the frame.
    assert!(
        p.leech
            .runner
            .handle_frame(p.leech.peer, t(0), &frame[..n - 1])
            .is_err()
    );
}

#[test]
fn events_tell_the_driver_what_to_do() {
    let mut p = Pair::new(options());
    // The new channel queued its handshake for the next tick.
    assert!(!p.seed.runner.wants_transmit(p.seed.peer));
    assert_eq!(p.seed.runner.poll_timeout(), Some(t(0) + TICK_INTERVAL));
    p.seed.runner.handle_timeout(t(0) + TICK_INTERVAL);
    let events: Vec<_> = core::iter::from_fn(|| p.seed.runner.poll_event()).collect();
    assert!(events.contains(&Event::Transmit(p.seed.peer)), "{events:?}");
    assert!(p.seed.runner.wants_transmit(p.seed.peer));
    let mut buf = vec![0u8; MTU];
    let n = p.seed.runner.poll_transmit(p.seed.peer, t(100), &mut buf);
    assert!(n > 0);
    assert!(!p.seed.runner.wants_transmit(p.seed.peer));

    // The leech answers a frame right away (a pong is due).
    p.leech
        .runner
        .handle_frame(p.leech.peer, t(100), &buf[..n])
        .unwrap();
    let events: Vec<_> = core::iter::from_fn(|| p.leech.runner.poll_event()).collect();
    assert!(
        events.contains(&Event::Transmit(p.leech.peer)),
        "{events:?}"
    );

    // Publishing through the guard reaches the scheduler: HAVEs go out.
    p.now = t(100);
    p.pump().unwrap();
    {
        let mut s = p.seed.runner.swarm_mut(p.seed.swarm, t(150)).unwrap();
        p.writer.write(&mut s, t(150), &vec![1u8; MUNRO]).unwrap();
    }
    p.seed.runner.handle_timeout(t(200));
    assert!(p.seed.runner.wants_transmit(p.seed.peer));
    let n = p.seed.runner.poll_transmit(p.seed.peer, t(200), &mut buf);
    let cfg = p.leech.runner.swarm(p.leech.swarm).unwrap().reader_config();
    let haves: usize = codec::FrameReader::new(&buf[..n])
        .map(|c| {
            let (_, payload) = c.unwrap();
            Reader::new(cfg, payload)
                .map(|m| m.unwrap())
                .filter(|m| matches!(m, Message::Have(_)))
                .count()
        })
        .sum();
    assert!(haves > 0);
}

#[test]
fn weights_follow_useful_data() {
    // A seed with two leeches: only the seed's data is useful to the
    // leeches, so a leech weighs the seed at the maximum.
    let mut p = Pair::new(options());
    p.tick(100);
    for _ in 0..30 {
        p.write(1);
        p.tick(50);
        p.tick(50);
    }
    let w = p.leech.runner.peer_qos_weight(p.leech.peer);
    assert_eq!(w, Some(MAX_PEER_QOS_WEIGHT));
    let mut saw = false;
    while let Some(e) = p.leech.runner.poll_event() {
        saw |= matches!(
            e,
            Event::QosWeight {
                weight: MAX_PEER_QOS_WEIGHT,
                ..
            }
        );
    }
    assert!(saw);
    // The seed gets no useful data: its only peer keeps the maximum (an
    // equal share), set when the peer was added.
    assert_eq!(
        p.seed.runner.peer_qos_weight(p.seed.peer),
        Some(MAX_PEER_QOS_WEIGHT)
    );
}

/// Gives each peer `rates[i]` bytes of useful data per second for 3 s and
/// runs the weight update.
fn weights_for(rates: &[u64]) -> (Runner, Vec<PeerKey>) {
    let mut r = Runner::new(det_rng(9));
    let peers: Vec<PeerKey> = (0..rates.len())
        .map(|i| r.add_peer(&[i as u8], t(0)))
        .collect();
    while r.poll_event().is_some() {}
    for ms in (0..3000).step_by(100) {
        for (i, &rate) in rates.iter().enumerate() {
            r.peers[peers[i]].useful.add(rate / 10, t(ms));
        }
        r.handle_timeout(t(ms));
    }
    (r, peers)
}

#[test]
fn weights_normalize_to_the_top_peer() {
    // Weights are relative to the top contributor, so 30 equal
    // contributors all get the maximum; a share of the total would put
    // each under the floor and erase reciprocity.
    let (r, peers) = weights_for(&[100_000; 30]);
    for &p in &peers {
        assert_eq!(r.peer_qos_weight(p), Some(MAX_PEER_QOS_WEIGHT));
    }

    // Shares follow the rates, relative to the top one, down to the floor.
    let mut rates = vec![200_000, 100_000, 50_000, 1_000, 0];
    rates.extend([150_000; 25]);
    let (r, peers) = weights_for(&rates);
    let w: Vec<u64> = peers
        .iter()
        .map(|&p| r.peer_qos_weight(p).unwrap())
        .collect();
    assert_eq!(
        w[..5],
        [1000, 500, 250, MIN_PEER_QOS_WEIGHT, MIN_PEER_QOS_WEIGHT]
    );
    assert!(w[5..].iter().all(|&x| x == 750), "{w:?}");
}

#[test]
fn new_peers_get_an_initial_weight() {
    // vnic writers start at weight 1 until told otherwise: the runner
    // tells them at once.
    let mut r = Runner::new(det_rng(9));
    let a = r.add_peer(b"a", t(0));
    assert_eq!(r.peer_qos_weight(a), Some(MAX_PEER_QOS_WEIGHT));
    assert_eq!(
        r.poll_event(),
        Some(Event::QosWeight {
            peer: a,
            weight: MAX_PEER_QOS_WEIGHT
        })
    );

    // A newcomer gets the median of the current weights.
    let (mut r, _) = weights_for(&[200_000, 100_000, 10_000]);
    while r.poll_event().is_some() {}
    let n = r.add_peer(b"new", t(3000));
    assert_eq!(r.peer_qos_weight(n), Some(500));
    assert_eq!(
        r.poll_event(),
        Some(Event::QosWeight {
            peer: n,
            weight: 500
        })
    );

    // Without any useful data the update still runs: everyone gets the
    // maximum.
    let (r, peers) = weights_for(&[0, 0, 0]);
    for &p in &peers {
        assert_eq!(r.peer_qos_weight(p), Some(MAX_PEER_QOS_WEIGHT));
    }
    // Contributions that stop fall back to equal shares once they leave
    // the 60 s window.
    let (mut r, peers) = weights_for(&[100_000, 10_000]);
    assert_eq!(r.peer_qos_weight(peers[1]), Some(100));
    r.handle_timeout(t(3000) + Duration::from_secs(61));
    r.handle_timeout(t(3000) + Duration::from_secs(62));
    assert_eq!(r.peer_qos_weight(peers[1]), Some(MAX_PEER_QOS_WEIGHT));
}

#[test]
fn request_stream_waits_for_the_handshake() {
    // A STREAM_OPEN before the seed's handshake would be ignored, so the
    // request waits for the handshake.
    let mut p = Pair::new(options());
    let (peer, swarm) = (p.leech.peer, p.leech.swarm);
    assert_eq!(
        p.leech.runner.request_stream(peer, swarm, 0, Bin(0)),
        Err(RunnerError::HandshakePending)
    );
    p.tick(100);
    assert_eq!(
        p.leech.runner.request_stream(peer, swarm, 0, Bin(0)),
        Ok(())
    );
}

#[test]
fn stream_subscription_pushes_data() {
    // STREAM_REQUEST -> STREAM_OPEN -> pushes, through
    // `Runner::request_stream`.
    let mut p = Pair::new(options());
    p.tick(100);
    p.write(2);
    for _ in 0..10 {
        p.tick(50);
    }
    p.leech
        .runner
        .request_stream(p.leech.peer, p.leech.swarm, 0, Bin(0))
        .unwrap();
    p.tick(100);
    assert_eq!(
        p.seed.runner.swarm_stats(p.seed.swarm).unwrap().subscribers,
        1
    );
    assert_eq!(
        p.leech
            .runner
            .swarm_stats(p.leech.swarm)
            .unwrap()
            .stream_sources,
        1
    );
    let requests_before = p
        .leech
        .runner
        .channel_stats(p.leech.peer, p.leech.swarm)
        .unwrap()
        .outbound
        .request;
    for _ in 0..10 {
        p.write(1);
        for _ in 0..5 {
            p.tick(50);
        }
    }
    let cs = p
        .leech
        .runner
        .channel_stats(p.leech.peer, p.leech.swarm)
        .unwrap();
    // The source's HAVEs mark its stream's bins requested, so the new
    // chunks arrive pushed, not pulled.
    assert_eq!(cs.outbound.request, requests_before, "{cs:?}");
    let swarm = p.leech.runner.swarm(p.leech.swarm).unwrap();
    let start = p.leech.reader.offset(swarm.store()).unwrap();
    let got = p.read().unwrap();
    assert_eq!(start + got.len() as u64, p.written);
    assert!(
        got.iter()
            .enumerate()
            .all(|(i, &b)| b == byte(start + i as u64))
    );

    // Cancelling the subscription returns to pull.
    p.leech
        .runner
        .cancel_stream(p.leech.peer, p.leech.swarm, 0)
        .unwrap();
    p.tick(100);
    assert_eq!(
        p.seed.runner.swarm_stats(p.seed.swarm).unwrap().subscribers,
        0
    );
    assert_eq!(
        p.leech
            .runner
            .swarm_stats(p.leech.swarm)
            .unwrap()
            .stream_sources,
        0
    );
    p.write(2);
    for _ in 0..10 {
        p.tick(50);
    }
    let got2 = p.read().unwrap();
    assert_eq!(got2.len(), 2 * MUNRO);
    assert!(
        p.leech
            .runner
            .channel_stats(p.leech.peer, p.leech.swarm)
            .unwrap()
            .outbound
            .request
            > requests_before
    );
}

#[test]
fn chunks_that_never_fit_are_dropped() {
    // A frame too small for a chunk and its integrity: the chunk is
    // dropped from the queue, so the peer doesn't spin writing empty
    // frames.
    let mut p = Pair::new(options());
    p.tick(100);
    p.tick(100);
    p.write(1);
    let mut buf = vec![0u8; MTU];
    for _ in 0..10 {
        p.now += Duration::from_millis(100);
        p.seed.runner.handle_timeout(p.now);
        p.leech.runner.handle_timeout(p.now);
        while p.seed.runner.wants_transmit(p.seed.peer) {
            let n = p.seed.runner.poll_transmit(p.seed.peer, p.now, &mut buf);
            if n == 0 {
                break;
            }
            p.leech
                .runner
                .handle_frame(p.leech.peer, p.now, &buf[..n])
                .unwrap();
        }
        while p.leech.runner.wants_transmit(p.leech.peer) {
            let n = p.leech.runner.poll_transmit(p.leech.peer, p.now, &mut buf);
            if n == 0 {
                break;
            }
            p.seed
                .runner
                .handle_frame(p.seed.peer, p.now, &buf[..n])
                .unwrap();
        }
        if p.seed.runner.peer_stats(p.seed.peer).unwrap().queued_data[1] > 0 {
            break;
        }
    }
    assert!(p.seed.runner.peer_stats(p.seed.peer).unwrap().queued_data[1] > 0);
    let mut small = [0u8; 300];
    let mut polls = 0;
    while p.seed.runner.wants_transmit(p.seed.peer) {
        p.seed.runner.poll_transmit(p.seed.peer, p.now, &mut small);
        polls += 1;
        assert!(polls < 1000, "spinning");
    }
    assert_eq!(
        p.seed.runner.peer_stats(p.seed.peer).unwrap().queued_data,
        [0, 0]
    );
}

#[test]
fn idle_peers_dont_tick() {
    // Peers without running channels have nothing to tick: no timer.
    let mut r = Runner::new(det_rng(3));
    let a = r.add_peer(b"a", t(0));
    let b = r.add_peer(b"b", t(0));
    assert_eq!(r.poll_timeout(), None);
    assert_eq!(r.peer_swarms(a).count(), 0);

    // A channel starts the clock, a full interval after it starts, even
    // after a long idle spell.
    let key = strims_crypto::key_from_seed(&[4; 32]);
    let s1 = r.add_swarm(Swarm::new(SwarmId::new(&key.public), options()).unwrap());
    let key2 = strims_crypto::key_from_seed(&[6; 32]);
    let s2 = r.add_swarm(Swarm::new(SwarmId::new(&key2.public), options()).unwrap());
    r.run_swarm(a, s1, 7, 1, t(60_000)).unwrap();
    assert_eq!(r.poll_timeout(), Some(t(60_000) + TICK_INTERVAL));
    r.run_swarm(a, s2, 3, 2, t(60_000)).unwrap();
    r.run_swarm(b, s2, 3, 2, t(60_000)).unwrap();
    // In our channel id order.
    assert_eq!(r.peer_swarms(a).collect::<Vec<_>>(), vec![s2, s1]);
    assert_eq!(r.peer_swarms(b).collect::<Vec<_>>(), vec![s2]);

    // When the last channel stops, the timer goes away again.
    r.stop_swarm(a, s1);
    assert_eq!(r.peer_swarms(a).collect::<Vec<_>>(), vec![s2]);
    r.stop_swarm(a, s2);
    r.remove_peer(b);
    assert_eq!(r.poll_timeout(), None);
    assert_eq!(r.peer_swarms(b).count(), 0);
}

#[test]
fn late_data_behind_the_window_is_ignored_as_stale() {
    // A frame of data held back until the leech's window has moved past
    // its chunks: they can't be stored or verified any more. The frame was
    // sent after the leech announced part of the munro, so it carries no
    // munro signature. It is a late answer, and it still answers the
    // request.
    let mut p = Pair::new(SwarmOptions {
        delivery_mode: crate::DeliveryMode::MANDATORY,
        ..options()
    });
    p.tick(100);
    p.tick(100);
    p.write(1);
    let mut held = None;
    let mut buf = vec![0u8; MTU];
    for _ in 0..40 {
        p.now += Duration::from_millis(100);
        p.seed.runner.handle_timeout(p.now);
        p.leech.runner.handle_timeout(p.now);
        while p.leech.runner.wants_transmit(p.leech.peer) {
            let n = p.leech.runner.poll_transmit(p.leech.peer, p.now, &mut buf);
            if n == 0 {
                break;
            }
            p.seed
                .runner
                .handle_frame(p.seed.peer, p.now, &buf[..n])
                .unwrap();
        }
        // One frame per tick: a munro takes several, and the leech's HAVEs
        // reach the seed in between.
        if p.seed.runner.wants_transmit(p.seed.peer) {
            let n = p.seed.runner.poll_transmit(p.seed.peer, p.now, &mut buf);
            if n == 0 {
                continue;
            }
            let has_data =
                crate::codec::FrameReader::new(&buf[..n])
                    .flatten()
                    .any(|(_, payload)| {
                        crate::codec::Reader::new(
                            p.leech.runner.swarm(p.leech.swarm).unwrap().reader_config(),
                            payload,
                        )
                        .flatten()
                        .any(|m| matches!(m, crate::codec::Message::Data(_)))
                    });
            let announced = p
                .seed
                .runner
                .channel_stats(p.seed.peer, p.seed.swarm)
                .unwrap()
                .inbound
                .have
                > 0;
            if has_data && announced && held.is_none() {
                held = Some(buf[..n].to_vec());
            } else {
                p.leech
                    .runner
                    .handle_frame(p.leech.peer, p.now, &buf[..n])
                    .unwrap();
            }
        }
        if held.is_some() {
            break;
        }
    }
    let held = held.unwrap();
    // More than a window of stream (1024 chunks) moves the leech past it.
    for _ in 0..24 {
        p.write(1);
        for _ in 0..4 {
            p.tick(50);
        }
    }
    let stats = |p: &Pair| {
        p.leech
            .runner
            .channel_stats(p.leech.peer, p.leech.swarm)
            .unwrap()
    };
    let before = stats(&p).inbound;
    p.leech
        .runner
        .handle_frame(p.leech.peer, p.now, &held)
        .unwrap();
    let after = stats(&p).inbound;
    assert_eq!(after.invalid_data, 0, "{after:?}");
    assert!(after.stale_data > before.stale_data, "{after:?}");
}

fn hybrid() -> SwarmOptions {
    SwarmOptions {
        push_mode: PushMode::Hybrid,
        ..options()
    }
}

/// A hybrid transfer: pulled at first, pushed once the leech subscribes
/// (the seed's allocation is pinned to it). Returns the pair and the tap
/// events of the seed and the leech.
fn tapped_transfer(tap: bool) -> (Pair, Vec<TapEvent>, Vec<TapEvent>) {
    let mut p = Pair::with(hybrid(), hybrid());
    p.seed
        .runner
        .pin_seed_allocation(p.seed.swarm, Some(vec![vec![p.seed.peer]]))
        .unwrap();
    let level = if tap { TapLevel::All } else { TapLevel::Off };
    p.seed.runner.set_tap(level);
    p.leech.runner.set_tap(level);
    let (mut seed, mut leech) = (Vec::new(), Vec::new());
    let mut step = |p: &mut Pair, ms: i64| {
        p.tick(ms);
        p.seed.runner.drain_tap(|e| seed.push(e));
        p.leech.runner.drain_tap(|e| leech.push(e));
    };
    step(&mut p, 100);
    p.write(2);
    for _ in 0..10 {
        step(&mut p, 50);
    }
    p.leech
        .runner
        .request_stream(p.leech.peer, p.leech.swarm, 0, Bin(0))
        .unwrap();
    for _ in 0..10 {
        p.write(1);
        for _ in 0..5 {
            step(&mut p, 50);
        }
    }
    (p, seed, leech)
}

#[test]
fn a_seeds_shares_follow_its_subscribers_at_once() {
    // A hybrid seed whose uplink shares follow its pushes weighs a
    // subscriber in the call that admitted or lost it, not at the next
    // update.
    let seed = SwarmOptions {
        seed: crate::SeedPolicy {
            allocation: crate::SeedAllocation::Admission,
            qos: crate::SeedQos::ByAllocation,
        },
        ..hybrid()
    };
    let mut p = Pair::with(seed, hybrid());
    for _ in 0..15 {
        p.write(1);
        p.tick(100);
    }
    let weight = |p: &Pair| p.seed.runner.peer_qos_weight(p.seed.peer);
    assert_eq!(weight(&p), Some(MIN_PEER_QOS_WEIGHT));
    while p.seed.runner.poll_event().is_some() {}

    p.leech
        .runner
        .request_stream(p.leech.peer, p.leech.swarm, 0, Bin(0))
        .unwrap();
    p.pump().unwrap();
    assert_eq!(weight(&p), Some(MAX_PEER_QOS_WEIGHT));
    let mut events = Vec::new();
    while let Some(e) = p.seed.runner.poll_event() {
        events.push(e);
    }
    assert!(
        events.contains(&Event::QosWeight {
            peer: p.seed.peer,
            weight: MAX_PEER_QOS_WEIGHT
        }),
        "{events:?}"
    );

    // The leech's cancel goes out at its next tick; the seed only reads it.
    p.leech
        .runner
        .cancel_stream(p.leech.peer, p.leech.swarm, 0)
        .unwrap();
    p.now += Duration::from_millis(100);
    p.leech.runner.handle_timeout(p.now);
    p.pump().unwrap();
    assert_eq!(weight(&p), Some(MIN_PEER_QOS_WEIGHT));
}

#[test]
fn the_tap_changes_no_frame() {
    let (off, seed, leech) = tapped_transfer(false);
    assert!(seed.is_empty() && leech.is_empty());
    let (on, _, _) = tapped_transfer(true);
    assert_eq!(off.frames, on.frames);
}

#[test]
fn the_tap_tells_pushes_from_answers() {
    let (mut p, seed, leech) = tapped_transfer(true);
    let (mut pushed, mut answered) = (0, 0);
    for e in &seed {
        if let TapKind::DataSent { bin, push, .. } = e.kind {
            assert_eq!((e.swarm, e.peer), (Some(p.seed.swarm), Some(p.seed.peer)));
            if push {
                pushed += bin.base_length();
            } else {
                answered += bin.base_length();
            }
        }
    }
    let stats = p.seed.runner.swarm_stats(p.seed.swarm).unwrap();
    assert!(pushed > 0 && answered > 0, "{pushed} {answered}");
    assert_eq!(pushed, stats.pushed_chunks);
    assert_eq!(pushed + answered, stats.chunks_sent);
    assert!(
        seed.iter()
            .any(|e| e.kind == TapKind::Admitted { stream: 0 })
    );
    let hits = leech
        .iter()
        .filter(|e| matches!(e.kind, TapKind::PushHit { .. }))
        .count();
    assert!(hits > 0);

    // channel and peer lifecycle
    p.leech.runner.remove_peer(p.leech.peer);
    let mut gone = Vec::new();
    p.leech.runner.drain_tap(|e| gone.push(e.kind));
    assert_eq!(gone, [TapKind::ChannelClosed, TapKind::PeerRemoved]);
    let peer = p.leech.runner.add_peer(b"again", p.now);
    p.leech
        .runner
        .run_swarm(peer, p.leech.swarm, 7, 8, p.now)
        .unwrap();
    let mut back = Vec::new();
    p.leech.runner.drain_tap(|e| back.push((e.peer, e.kind)));
    assert_eq!(
        back,
        [
            (Some(peer), TapKind::PeerAdded),
            (
                Some(peer),
                TapKind::ChannelOpened {
                    channel: 7,
                    peer_channel: 8
                }
            ),
        ]
    );
    assert_eq!(p.leech.runner.channels().count(), 1);
    assert_eq!(
        p.leech.runner.peers().map(|(_, id)| id).collect::<Vec<_>>(),
        [&b"again"[..]]
    );
}
