// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduler tests over a mock peer queue: requests, stream subscriptions,
//! timeouts, the push/pull hybrid, and the bounds on peer input.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::codec::{Message, Reader};
use crate::{SwarmId, SwarmOptions};
use slotmap::SlotMap;
use strims_base::rng::det_rng;

/// A [`PeerTaskQueue`] that records the calls.
#[derive(Debug, Default)]
pub(crate) struct MockQueue {
    pub(crate) enqueued: Vec<(PeerKey, SwarmKey)>,
    pub(crate) enqueued_now: Vec<(PeerKey, SwarmKey)>,
    pub(crate) pushed: Vec<(PeerKey, Bin, PeerPriority)>,
    pub(crate) pushed_front: Vec<(PeerKey, Bin, PeerPriority)>,
    pub(crate) removed: Vec<(PeerKey, Bin)>,
    pub(crate) removed_runner: Vec<PeerKey>,
}

impl PeerTaskQueue for MockQueue {
    fn enqueue(&mut self, peer: PeerKey, swarm: SwarmKey) {
        self.enqueued.push((peer, swarm));
    }
    fn enqueue_now(&mut self, peer: PeerKey, swarm: SwarmKey) {
        self.enqueued_now.push((peer, swarm));
    }
    fn push_data(&mut self, peer: PeerKey, _: SwarmKey, b: Bin, _: Time, pri: PeerPriority) {
        self.pushed.push((peer, b, pri));
    }
    fn push_front_data(&mut self, peer: PeerKey, _: SwarmKey, b: Bin, _: Time, pri: PeerPriority) {
        self.pushed_front.push((peer, b, pri));
    }
    fn remove_data(&mut self, peer: PeerKey, _: SwarmKey, b: Bin, _: PeerPriority) {
        self.removed.push((peer, b));
    }
    fn remove_runner(&mut self, peer: PeerKey, _: SwarmKey) {
        self.removed_runner.push(peer);
    }
}

pub(crate) fn now() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

pub(crate) struct Fixture {
    pub(crate) swarm: Swarm,
    pub(crate) sched: PeerSwarmScheduler,
    pub(crate) peers: Vec<PeerKey>,
}

/// A fresh swarm with `options` and a scheduler for it, with four peer
/// keys.
pub(crate) fn fixture(options: SwarmOptions) -> Fixture {
    let key = strims_crypto::key_from_seed(&[5; 32]);
    let swarm = Swarm::new(SwarmId::new(&key.public), options).unwrap();
    let mut keys: SlotMap<SwarmKey, ()> = SlotMap::with_key();
    let key = keys.insert(());
    let mut peers: SlotMap<PeerKey, ()> = SlotMap::with_key();
    let peers = (0..4).map(|_| peers.insert(())).collect();
    let sched = PeerSwarmScheduler::new(&swarm, key, &mut det_rng(1), now());
    Fixture {
        swarm,
        sched,
        peers,
    }
}

fn eight_streams() -> SwarmOptions {
    SwarmOptions {
        stream_count: 8,
        ..Default::default()
    }
}

/// The messages in a channel payload written by `f`.
fn written(
    f: impl FnOnce(&mut MeteredWriter<'_>),
    cfg: crate::codec::ReaderConfig,
) -> Vec<Message<'static>> {
    let mut buf = vec![0u8; 16 * 1024];
    let mut cw = MeteredWriter::new(&mut buf, 1);
    f(&mut cw);
    let mut m = ChannelMetrics::default();
    let (n, _) = cw.finish(&mut m);
    let buf: &'static [u8] = Box::leak(buf.into_boxed_slice());
    let mut out = vec![];
    for c in crate::codec::FrameReader::new(&buf[..n]) {
        let (_, payload) = c.unwrap();
        for msg in Reader::new(cfg, payload) {
            out.push(msg.unwrap());
        }
    }
    out
}

use crate::channel::ChannelMetrics;

#[test]
fn channel_start_queues_it_and_close_removes_it() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = f.peers[0];
    f.sched.add_channel(p, now(), &mut q);
    assert!(f.sched.channels.contains_key(&p));
    assert_eq!(q.enqueued.len(), 1);

    f.sched.close_channel(p, &f.swarm, &mut q);
    assert_eq!(
        q.removed_runner,
        vec![p],
        "expected peer removeRunner to be called"
    );
    assert!(f.sched.channels.is_empty());
}

#[test]
fn stream_subscription_marks_its_bins_requested() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);

    let peer_have_bin = Bin::new(6, 0);
    let request_bin = Bin::new(5, 0);
    let have_bin = Bin::new(5, 0);
    let stream = 0;
    let start_bin = request_bin.base_right().layer_right();

    f.sched.channels[&p].peer_have_bins.set(peer_have_bin);
    f.sched.s.request_bins.set(request_bin);
    f.sched.s.have_bins.set(have_bin);
    f.sched.s.peer_max_have_bin = peer_have_bin.base_right();
    f.sched.s.have_bin_max = have_bin.base_right();

    assert!(f.sched.do_stream_sub(p, stream, start_bin));

    assert_eq!(
        f.sched.channels[&p].request_streams[0], start_bin,
        "expected channel scheduler start stream to be set after sub"
    );
    let filled: Vec<Bin> = f.sched.s.request_bins.iterate_filled().collect();
    assert_eq!(
        filled,
        [31, 64, 80, 96, 112].map(Bin).to_vec(),
        "expected requested bins to be filled for stream bins after sub"
    );

    f.sched.consume(Bin(64), now(), &f.swarm, &mut q);
    f.sched
        .handle_data(p, Bin(64), Time::NIL, true, true, now(), Bin(0), &mut q);

    assert!(
        f.sched
            .cancel_stream_sub(p, stream, &f.swarm, now(), &mut q)
    );

    let filled: Vec<Bin> = f.sched.s.request_bins.iterate_filled().collect();
    assert_eq!(
        filled,
        [31, 64].map(Bin).to_vec(),
        "expected unreceived bins to be filled for stream bins after unsub"
    );
}

#[test]
fn channel_requests_peer_have() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = f.peers[0];
    f.sched.add_channel(p, now(), &mut q);
    f.sched.handle_handshake(p, 16 * 1024);

    let have_bin = Bin::new(6, 0);
    assert!(f.sched.handle_have(p, have_bin, now()));

    q.enqueued.clear();
    f.sched.handle_message_end(p, now(), &mut q);
    assert!(!q.enqueued.is_empty());

    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    let requests: Vec<Bin> = msgs
        .iter()
        .filter_map(|m| match m {
            Message::Request(r) => Some(r.bin),
            _ => None,
        })
        .collect();
    assert!(!requests.is_empty(), "{msgs:?}");
    assert!(requests.iter().all(|&b| have_bin.contains(b)));
}

#[test]
fn first_write_sends_handshake_and_ping() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = f.peers[0];
    f.sched.add_channel(p, now(), &mut q);
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    assert!(matches!(msgs[0], Message::Ping(_)));
    assert!(matches!(msgs[1], Message::Handshake(_)));
    // Pings at most once a second.
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    assert!(msgs.is_empty(), "{msgs:?}");
}

fn ready_channel(f: &mut Fixture, q: &mut MockQueue) -> PeerKey {
    let p = f.peers[0];
    f.sched.add_channel(p, now(), q);
    f.sched.handle_handshake(p, 1 << 16);
    p
}

#[test]
fn huge_have_costs_a_bounded_walk() {
    // One HAVE for a layer-40 bin must not walk its 2^40 chunks. The
    // hybrid walks a HAVE's new bins for its per-stream lag; an unbounded
    // walk would not finish.
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);

    assert!(f.sched.handle_have(p, Bin::new(40, 3), now()));
    let c = &f.sched.channels[&p];
    assert_eq!(c.peer_max_have_bin, Bin::new(40, 3).base_right());
    // Only the last windows are tracked.
    let lo = f.sched.s.history_start(c.peer_max_have_bin);
    assert!(c.peer_have_bins.filled_at(Bin(lo.0)));
    assert!(c.peer_have_bins.empty_at(Bin(lo.0 - 2)));

    // Bins outside the bin space are refused.
    assert!(!f.sched.handle_have(p, Bin::ALL, now()));
    assert!(!f.sched.handle_have(p, Bin::NONE, now()));
    assert!(!f.sched.handle_have(p, MAX_BIN, now()));
}

#[test]
fn have_flood_is_bounded() {
    let mut f = fixture(SwarmOptions {
        live_window: 1 << 10,
        ..Default::default()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let mut rng = det_rng(7);
    use rand_core::Rng;
    for i in 0..20_000u64 {
        // Scattered bins, far jumps forward and back, all layers.
        let layer = rng.next_u64() % 60;
        let off = rng.next_u64() >> (layer + 3);
        let b = Bin::new(layer, off);
        f.sched
            .handle_have(p, b, now() + Duration::from_millis(i as i64));
    }
    let stats = f.sched.channel_stats(p).unwrap();
    let s = f.sched.stats();
    assert!(s.bin_time_nodes < 16 * 1024, "{s:?}");
    assert!(stats.peer_have_map_size < 4096, "{stats:?}");
}

#[test]
fn unsolicited_stream_open_is_ignored() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    // An accepted source has every bin of the stream the peers announce
    // expected from it and never requested, so only a peer we asked may
    // become one.
    assert_eq!(
        f.sched
            .handle_stream_open(p, 0, Bin(0), now(), &mut MockQueue::default()),
        Ok(StreamOpenOutcome::Rejected)
    );
    assert_eq!(f.sched.stats().stream_sources, 0);

    // A solicited open is accepted.
    assert!(f.sched.do_stream_sub(p, 0, Bin(0)));
    assert_eq!(
        f.sched
            .handle_stream_open(p, 0, Bin(16), now(), &mut MockQueue::default()),
        Ok(StreamOpenOutcome::Accepted)
    );
    assert_eq!(f.sched.stats().stream_sources, 1);
    // ... and the source's HAVEs mark the stream's bins requested.
    f.sched.handle_have(p, Bin::new(4, 1), now());
    assert!(f.sched.s.request_bins.filled_at(Bin(16)));
    assert!(f.sched.s.request_bins.empty_at(Bin(18)));
    // Closing it releases them.
    assert_eq!(
        f.sched
            .handle_stream_close(p, 0, Bin(0), now(), &mut MockQueue::default()),
        Ok(())
    );
    assert!(f.sched.s.request_bins.empty_at(Bin(16)));
    assert_eq!(f.sched.stats().stream_sources, 0);
}

#[test]
fn stream_close_keeps_chunks_requested_from_other_channels() {
    let mut f = fixture(SwarmOptions::default());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let o = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    f.sched.handle_have(o, Bin::new(1, 0), now());
    assert_eq!(requests_at(&mut f, o, now()), vec![Bin(0), Bin(2)]);

    // The source gets only what nobody was asked for.
    assert!(f.sched.do_stream_sub(p, 0, Bin(0)));
    f.sched.handle_have(p, Bin::new(2, 0), now());
    assert!(f.sched.s.request_bins.filled_at(Bin::new(2, 0)));

    f.sched
        .handle_stream_close(p, 0, Bin(0), now(), &mut q)
        .unwrap();
    let r = &f.sched.s.request_bins;
    assert!(r.filled_at(Bin(0)) && r.filled_at(Bin(2)));
    assert!(r.empty_at(Bin(4)) && r.empty_at(Bin(6)));
    assert_eq!(f.sched.stats().stream_sources, 0);
}

#[test]
fn stream_close_from_a_replaced_source_keeps_the_new_one() {
    let mut f = fixture(SwarmOptions::default());
    let mut q = MockQueue::default();
    let a = ready_channel(&mut f, &mut q);
    let b = add_ready(&mut f, &mut q, 1);
    assert!(f.sched.do_stream_sub(a, 0, Bin(0)));
    assert!(f.sched.do_stream_sub(b, 0, Bin(4)));
    f.sched.handle_have(b, Bin::new(2, 0), now());
    assert!(f.sched.s.request_bins.filled_at(Bin::new(1, 1)));

    f.sched
        .handle_stream_close(a, 0, Bin(0), now(), &mut q)
        .unwrap();
    assert!(f.sched.s.request_bins.filled_at(Bin::new(1, 1)));
    assert_eq!(f.sched.s.streams[0].source.map(|src| src.channel), Some(b));
    assert_eq!(f.sched.stats().stream_sources, 1);

    f.sched
        .handle_stream_close(b, 0, Bin(0), now(), &mut q)
        .unwrap();
    assert!(f.sched.s.request_bins.empty_at(Bin::new(1, 1)));
    assert_eq!(f.sched.stats().stream_sources, 0);
}

#[test]
fn invalid_data_hands_a_source_chunk_to_pull() {
    let mut f = fixture(SwarmOptions::default());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let o = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(p, 0, Bin(0)));
    f.sched.handle_have(p, Bin::new(1, 0), now());
    f.sched
        .handle_data(p, Bin(2), now(), false, false, now(), Bin(0), &mut q);
    assert!(f.sched.s.request_bins.empty_at(Bin(2)));
    f.sched.handle_have(o, Bin::new(1, 0), now());
    assert_eq!(requests_at(&mut f, o, now()), vec![Bin(2)]);

    // The source's close leaves the pull request alone.
    f.sched
        .handle_stream_close(p, 0, Bin(0), now(), &mut q)
        .unwrap();
    assert!(f.sched.s.request_bins.filled_at(Bin(2)));
    assert!(f.sched.s.request_bins.empty_at(Bin(0)));
}

#[test]
fn invalid_streams_are_errors() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let e = Err(SchedulerError::InvalidStream(8));
    assert_eq!(
        f.sched
            .handle_stream_request(p, 8, Bin(0), Bin(0), now(), &mut q)
            .map(|_| ()),
        e
    );
    assert_eq!(f.sched.handle_stream_cancel(p, 8), e);
    assert_eq!(
        f.sched
            .handle_stream_open(p, 8, Bin(0), now(), &mut MockQueue::default())
            .map(|_| ()),
        e
    );
    assert_eq!(
        f.sched
            .handle_stream_close(p, 8, Bin(0), now(), &mut MockQueue::default()),
        e
    );
}

#[test]
fn stream_request_pushes_stream_bins() {
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.have_bins.set(Bin::new(5, 0));
    f.sched.s.have_bin_max = Bin::new(5, 0).base_right();
    assert_eq!(
        f.sched
            .handle_stream_request(p, 3, Bin(0), Bin(0), now(), &mut q),
        Ok(true)
    );
    let pushed: Vec<Bin> = q.pushed.iter().map(|&(_, b, _)| b).collect();
    // chunks 3, 11, 19, 27
    assert_eq!(pushed, [6, 22, 38, 54].map(Bin).to_vec());
    assert!(
        q.pushed
            .iter()
            .all(|&(_, _, pri)| pri == PeerPriority::High)
    );
    // A repeated request from the same start pushes nothing again.
    q.pushed.clear();
    assert_eq!(
        f.sched
            .handle_stream_request(p, 3, Bin(0), Bin(0), now(), &mut q),
        Ok(true)
    );
    assert!(q.pushed.is_empty(), "{:?}", q.pushed);
    // New chunks of the stream are pushed as they arrive.
    q.pushed.clear();
    f.sched.consume(Bin::new(3, 4), now(), &f.swarm, &mut q);
    let pushed: Vec<Bin> = q.pushed.iter().map(|&(_, b, _)| b).collect();
    assert_eq!(pushed, [70].map(Bin).to_vec());
}

#[test]
fn a_parent_with_nothing_stored_opens_where_asked() {
    // Before a read offset the tail is NONE; the open starts at the
    // requested bin.
    for options in [eight_streams(), hybrid(8)] {
        let mut f = fixture(options);
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        assert_eq!(
            f.sched
                .handle_stream_request(p, 3, Bin(6), Bin::NONE, now(), &mut q),
            Ok(true)
        );
        let opens: Vec<Message> = next_messages(&mut f, p, now())
            .into_iter()
            .filter(|m| matches!(m, Message::StreamOpen(_)))
            .collect();
        assert_eq!(
            opens,
            vec![Message::StreamOpen(crate::codec::StreamOpen {
                stream: 3,
                bin: Bin(6)
            })]
        );
    }
}

#[test]
fn stream_requests_walk_at_most_a_window_without_a_read_offset() {
    // A node holding chunks far into the stream but no read offset (its
    // tail is NONE) gets a request from bin 0: the open starts a window
    // behind its newest chunk, and pushes at most a window of the stream.
    let mut f = fixture(eight_streams());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let far = Bin::new(0, 1 << 40);
    f.sched.s.have_bins.set(Bin::new(10, (1 << 40) >> 10));
    f.sched.s.have_bin_max = far;
    assert_eq!(
        f.sched
            .handle_stream_request(p, 3, Bin(0), Bin::NONE, now(), &mut q),
        Ok(true)
    );
    let window = f.sched.s.window as usize;
    assert!(q.pushed.len() <= window / 8 + 1, "{}", q.pushed.len());
    let opens: Vec<Bin> = next_messages(&mut f, p, now())
        .into_iter()
        .filter_map(|m| match m {
            Message::StreamOpen(o) => Some(o.bin),
            _ => None,
        })
        .collect();
    assert_eq!(opens, vec![Bin(far.0 - f.sched.s.live_window.0)]);
}

#[test]
fn an_open_at_none_starts_where_we_asked() {
    // A parent with nothing stored opens at its tail, NONE: honest input,
    // read as an open from where we asked.
    for options in [eight_streams(), hybrid(8)] {
        let mut f = fixture(options);
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        assert!(f.sched.do_stream_sub(p, 0, Bin(16)));
        assert_eq!(
            f.sched.handle_stream_open(p, 0, Bin::NONE, now(), &mut q),
            Ok(StreamOpenOutcome::Accepted)
        );
        assert_eq!(f.sched.channels[&p].request_streams[0], Bin(16));
        // Unrequested, it is still rejected.
        assert_eq!(
            f.sched.handle_stream_open(p, 1, Bin::NONE, now(), &mut q),
            Ok(StreamOpenOutcome::Rejected)
        );
    }
}

/// Options with request controller `kind`.
fn with_controller(o: SwarmOptions, kind: RequestControllerKind) -> SwarmOptions {
    SwarmOptions {
        request_controller: kind,
        ..o
    }
}

const CONTROLLERS: [RequestControllerKind; 2] = [
    RequestControllerKind::Ledbat,
    RequestControllerKind::BbrLike,
];

#[test]
fn pong_needs_an_outstanding_ping() {
    for kind in CONTROLLERS {
        let mut f = fixture(with_controller(SwarmOptions::default(), kind));
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        assert_eq!(f.sched.channels[&p].controller.kind(), kind);
        // Nonce 0 before any ping isn't a round trip (it would be one since
        // 1970).
        f.sched.handle_pong(p, 0, now());
        let c = &f.sched.channels[&p].controller;
        assert_eq!(c.request_timeout(), Duration::SECOND, "{kind:?}");
        assert_eq!(c.rtt(), Duration::ZERO, "{kind:?}");

        let cfg = f.swarm.reader_config();
        let (sched, swarm) = (&mut f.sched, &f.swarm);
        let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
        let Message::Ping(ping) = msgs[0] else {
            panic!("{msgs:?}")
        };
        f.sched
            .handle_pong(p, ping.nonce, now() + Duration::from_millis(40));
        let c = &f.sched.channels[&p].controller;
        assert_eq!(c.rtt(), Duration::from_millis(40), "{kind:?}");
        // A repeat is ignored.
        f.sched
            .handle_pong(p, ping.nonce, now() + Duration::from_secs(30));
        let c = &f.sched.channels[&p].controller;
        assert_eq!(c.rtt(), Duration::from_millis(40), "{kind:?}");
    }
}

#[test]
fn requests_are_bounded_by_the_window() {
    let mut f = fixture(SwarmOptions {
        live_window: 256,
        request_controller: RequestControllerKind::Ledbat,
        ..Default::default()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    f.sched.handle_have(p, Bin::new(8, 0), now());
    // A huge congestion window.
    for _ in 0..200 {
        f.sched.channels[&p]
            .controller
            .ledbat_mut()
            .unwrap()
            .add_delay_sample(Duration::from_millis(-1_000_000_000), 1 << 40, now());
        f.sched.channels[&p]
            .controller
            .ledbat_mut()
            .unwrap()
            .digest_delay_samples(now());
    }
    let cfg = f.swarm.reader_config();
    let mut total = 0;
    for i in 0..4 {
        let (sched, swarm) = (&mut f.sched, &f.swarm);
        let msgs = written(
            |cw| {
                let _ = sched.write(p, now() + Duration::from_millis(i), swarm, cw);
            },
            cfg,
        );
        total += msgs
            .iter()
            .filter(|m| matches!(m, Message::Request(_)))
            .count();
        // Bins come back as un-requested.
        f.sched.s.request_bins = Map::new();
    }
    assert!(total <= 256, "{total}");
    assert!(f.sched.channels[&p].request_bins.len() <= 256);
}

#[test]
fn reset_forgets_peer_positions() {
    // A reset forgets the peers' newest HAVEs and stream requests: a new
    // stream starts at bin 0 and must be requested from there.
    let mut f = fixture(SwarmOptions::default());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.handle_have(p, Bin::new(10, 50), now());
    f.sched.reset(now(), &mut q);
    assert_eq!(f.sched.channels[&p].peer_max_have_bin, Bin(0));
    assert_eq!(f.sched.s.peer_max_have_bin, Bin(0));
    assert_eq!(f.sched.channels[&p].request_streams[0], Bin::NONE);
    assert!(!f.sched.channels[&p].handshake_received);
    assert_eq!(q.enqueued_now.len(), 1);
}

#[test]
fn time_outs_cancel_and_release() {
    let mut f = fixture(SwarmOptions::default());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    f.sched.handle_have(p, Bin::new(2, 0), now());
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    let n = msgs
        .iter()
        .filter(|m| matches!(m, Message::Request(_)))
        .count();
    assert_eq!(n, 4);
    assert!(f.sched.s.request_bins.filled_at(Bin::new(2, 0)));

    // Past the congestion timeout the bins are cancelled and requestable.
    let later = now() + Duration::from_secs(2);
    let mut swarm = f.swarm;
    f.sched.run(later, &mut swarm, &mut q);
    assert!(f.sched.s.request_bins.empty_at(Bin::new(2, 0)));
    let msgs = written(|cw| f.sched.write(p, later, &swarm, cw).unwrap(), cfg);
    let cancels: Vec<_> = msgs
        .iter()
        .filter_map(|m| match m {
            Message::Cancel(c) => Some(c.bin),
            _ => None,
        })
        .collect();
    assert_eq!(cancels, vec![Bin::new(2, 0)]);
}

#[test]
fn gc_keeps_request_times_before_the_read_offset() {
    // Before the read offset is set `store.next()` is NONE; GC must not
    // prune request times there, or the answers would no longer count as
    // acks.
    let mut f = fixture(SwarmOptions::default());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.handle_have(p, Bin::new(2, 0), now());
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    assert!(msgs.iter().any(|m| matches!(m, Message::Request(_))));
    let before = f.sched.channels[&p].request_times.size();
    assert!(before > 0);
    assert_eq!(f.swarm.store().next(), Bin::NONE);
    f.sched.gc(&f.swarm);
    assert_eq!(f.sched.channels[&p].request_times.size(), before);
}

#[test]
fn unanswered_requests_filled_elsewhere_leave_the_flight() {
    // A request the peer never answers, for a chunk that arrives from
    // elsewhere first (a push), times out with the chunk already stored.
    // It must leave the flight without a loss: two such requests stuck in
    // flight would fill the minimum window and stop the channel.
    for kind in CONTROLLERS {
        let mut f = fixture(with_controller(SwarmOptions::default(), kind));
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        let other = f.peers[1];
        f.sched.add_channel(other, now(), &mut q);
        f.sched.handle_handshake(other, 1 << 16);
        f.sched.s.first_chunk_set = true;
        f.sched.handle_have(p, Bin::new(1, 0), now());
        let cfg = f.swarm.reader_config();
        let (sched, swarm) = (&mut f.sched, &f.swarm);
        let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
        let requested: Vec<Bin> = msgs
            .iter()
            .filter_map(|m| match m {
                Message::Request(r) => Some(r.bin),
                _ => None,
            })
            .collect();
        assert_eq!(requested, vec![Bin(0), Bin(2)]);
        assert_eq!(f.sched.channels[&p].controller.in_flight(), 2048);

        // Bin 0 is answered by `p` (acked); bin 2 comes from the other peer.
        f.sched.consume(Bin(0), now(), &f.swarm, &mut q);
        f.sched
            .handle_data(p, Bin(0), now(), true, true, now(), Bin(0), &mut q);
        f.sched.handle_message_end(p, now(), &mut q);
        f.sched.consume(Bin(2), now(), &f.swarm, &mut q);
        f.sched
            .handle_data(other, Bin(2), now(), true, true, now(), Bin(0), &mut q);
        // A duplicate answer from `p` doesn't ack twice.
        f.sched
            .handle_data(p, Bin(0), now(), true, false, now(), Bin(0), &mut q);
        f.sched.handle_message_end(p, now(), &mut q);
        assert_eq!(f.sched.channels[&p].controller.in_flight(), 1024);

        // Past the timeout nothing is left in flight, and nothing was lost:
        // no CANCEL, no backoff.
        let later = now() + Duration::from_secs(2);
        let mut swarm = f.swarm;
        f.sched.run(later, &mut swarm, &mut q);
        let c = &f.sched.channels[&p];
        assert_eq!(c.controller.in_flight(), 0, "{kind:?}");
        assert!(c.cancel_bins.is_empty());
        assert!(c.backoff_bins.is_empty());
    }
}

#[test]
fn bbr_like_measures_with_local_times_only() {
    // The BBR-style window takes its round trip from our request to the
    // data, on our clock: data stamped by a peer clock hours off changes
    // nothing, and pushed data counts toward the peer's delivery rate.
    let mut caps = Vec::new();
    for skew in [
        Duration::ZERO,
        Duration::from_secs(-7200),
        Duration::from_secs(36_000),
    ] {
        let mut f = fixture(with_controller(
            SwarmOptions::default(),
            RequestControllerKind::BbrLike,
        ));
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        f.sched.s.first_chunk_set = true;
        let cfg = f.swarm.reader_config();
        let mut t = now();
        for i in 0..40u64 {
            f.sched.handle_have(p, Bin::new(1, i), t);
            let (sched, swarm) = (&mut f.sched, &f.swarm);
            let msgs = written(|cw| sched.write(p, t, swarm, cw).unwrap(), cfg);
            let at = t + Duration::from_millis(30);
            for m in msgs {
                if let Message::Request(r) = m {
                    f.sched.consume(r.bin, at, &f.swarm, &mut q);
                    f.sched
                        .handle_data(p, r.bin, at + skew, true, true, at, Bin(0), &mut q);
                }
            }
            f.sched.handle_message_end(p, at, &mut q);
            t += Duration::from_millis(100);
        }
        let s = f.sched.channel_stats(p).unwrap();
        assert_eq!(s.controller, RequestControllerKind::BbrLike);
        assert_eq!(s.min_rtt, Duration::from_millis(30));
        assert!(s.delivery_rate > 0, "{s:?}");
        caps.push((s.cwnd, s.cto, s.rtt_mean));
    }
    assert!(caps.windows(2).all(|w| w[0] == w[1]), "{caps:?}");
}

/// The REQUESTs in what `p` writes at `t`.
fn requests_at(f: &mut Fixture, p: PeerKey, t: Time) -> Vec<Bin> {
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    written(
        |cw| {
            let _ = sched.write(p, t, swarm, cw);
        },
        cfg,
    )
    .iter()
    .filter_map(|m| match m {
        Message::Request(r) => Some(r.bin),
        _ => None,
    })
    .collect()
}

#[test]
fn requests_stay_inside_the_peers_window() {
    // Requests start inside the peer's live window even when our own next
    // missing chunk is further back. Chunks the peer evicted (its HAVE
    // history still claims them) are never answered, so a channel more
    // than a window behind that asked for them would time out on them
    // forever.
    let window = 256u64;
    let mut f = fixture(SwarmOptions {
        live_window: window as usize,
        chunks_per_signature: 16,
        ..Default::default()
    });
    let mut q = MockQueue::default();
    let p = f.peers[0];
    f.sched.add_channel(p, now(), &mut q);
    f.sched.handle_handshake(p, window as u32);
    f.sched.s.first_chunk_set = true;
    // We read from chunk 0 and have nothing.
    f.swarm.set_offset(Bin(0));
    assert_eq!(f.swarm.store().next(), Bin(0));

    // The peer announces chunks 0..1024: it holds 768..1024.
    assert!(f.sched.handle_have(p, Bin::new(10, 0), now()));
    let requests = requests_at(&mut f, p, now());
    assert!(!requests.is_empty());
    // Nothing before its window, nor in the window's first signature unit
    // (gone by the time the request arrives).
    let lowest = requests.iter().map(|b| b.base_offset()).min().unwrap();
    assert!(lowest >= 1024 - window + 16, "{lowest}");

    // The lead grows with the stream rate: at 64 chunks a second and a 1 s
    // timeout, the oldest 64 or so chunks of the window are left alone too.
    let mut f2 = fixture(SwarmOptions {
        live_window: window as usize,
        chunks_per_signature: 16,
        ..Default::default()
    });
    f2.sched.add_channel(p, now(), &mut q);
    f2.sched.handle_handshake(p, window as u32);
    f2.sched.s.first_chunk_set = true;
    f2.swarm.set_offset(Bin(0));
    let mut t = now();
    for i in 0..64u64 {
        t = now() + Duration::from_millis(250 * i as i64);
        assert!(f2.sched.handle_have(p, Bin::new(4, i), t));
    }
    let rate = f2.sched.s.peer_have_chunk_rate.rate(Duration::SECOND, t);
    assert!(rate >= 32, "{rate}");
    let requests = requests_at(&mut f2, p, t);
    let lowest = requests.iter().map(|b| b.base_offset()).min().unwrap();
    assert!(lowest >= 1024 - window + rate, "{lowest} {rate}");
}

#[test]
fn nodes_ask_a_shared_peer_for_different_chunks() {
    // Within a request group each node asks in its own order, so a shared
    // peer that can't serve every node sends them different chunks, which
    // they can then trade.
    let window = 1u64 << 10;
    let options = SwarmOptions {
        live_window: window as usize,
        ..Default::default()
    };
    let mut asked = vec![];
    for seed in [1, 2, 3] {
        let mut f = fixture(options.clone());
        f.sched = PeerSwarmScheduler::new(&f.swarm, f.sched.s.key, &mut det_rng(seed), now());
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        f.sched.s.first_chunk_set = true;
        // Two groups announced; the window allows 32 requests.
        assert!(f.sched.handle_have(p, Bin::new(7, 0), now()));
        let requests = requests_at(&mut f, p, now());
        assert_eq!(requests.len(), 32);
        // Groups in order: all from the first.
        assert!(
            requests.iter().all(|b| b.base_offset() < REQUEST_GROUP),
            "{requests:?}"
        );
        let mut set = requests.clone();
        set.sort();
        set.dedup();
        assert_eq!(set.len(), 32);
        asked.push(set);
    }
    assert_ne!(asked[0], asked[1]);
    assert_ne!(asked[1], asked[2]);

    // With room for every candidate, all are asked for.
    let mut f = fixture(options);
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.handle_have(p, Bin::new(3, 0), now()));
    let mut requests = requests_at(&mut f, p, now());
    requests.sort();
    assert_eq!(requests, (0..8).map(|i| Bin(2 * i)).collect::<Vec<_>>());
}

#[test]
fn requests_take_the_oldest_groups_across_streams() {
    // A chunk's deadline follows its position, whatever its stream: a
    // round that can't take every candidate asks for the oldest group's
    // chunks of all streams before any later group's.
    let mut f = fixture(SwarmOptions {
        live_window: 1 << 10,
        stream_count: 8,
        ..Default::default()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    // Four groups announced, 32 chunks per stream; the window allows 32
    // requests.
    assert!(f.sched.handle_have(p, Bin::new(8, 0), now()));
    let requests = requests_at(&mut f, p, now());
    assert_eq!(requests.len(), 32);
    assert!(
        requests.iter().all(|b| b.base_offset() < REQUEST_GROUP),
        "{requests:?}"
    );
    let mut streams: Vec<_> = requests.iter().map(|&b| f.sched.s.bin_stream(b)).collect();
    streams.sort();
    streams.dedup();
    assert!(streams.len() > 1, "{streams:?}");
}

#[test]
fn window_floor_carries_the_stream() {
    // After the path's latency steps up, LEDBAT reads the rise as queueing
    // until the old base delay ages out of its history (minutes) and holds
    // its window at 2 chunks per round trip, a fraction of the stream. The
    // window's floor, twice the stream's bandwidth-delay product, has to
    // carry the stream meanwhile.
    let mut f = fixture(SwarmOptions {
        live_window: 1 << 10,
        chunks_per_signature: 8,
        request_controller: RequestControllerKind::Ledbat,
        ..Default::default()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    // 32 chunks a second for 20 s.
    let mut t = now();
    for i in 0..80u64 {
        t = now() + Duration::from_millis(250 * i as i64);
        assert!(f.sched.handle_have(p, Bin::new(3, i), t));
    }
    let rate = f.sched.s.peer_have_chunk_rate.rate(Duration::SECOND, t);
    assert!((24..=40).contains(&rate), "{rate}");
    {
        let l = f.sched.channels[&p].controller.ledbat_mut().unwrap();
        // 10 ms one way, then 150 ms: read as 140 ms of queueing.
        l.add_delay_sample(Duration::from_millis(10), 1024, t);
        l.digest_delay_samples(t);
        for _ in 0..500 {
            l.add_sent(8192);
            l.add_delay_sample(Duration::from_millis(150), 8192, t);
            l.digest_delay_samples(t);
        }
        assert_eq!(l.cwnd(), 2 * 1024);
        l.add_rtt_sample(Duration::from_millis(300));
    }
    let requests = requests_at(&mut f, p, t);
    let floor = 2 * rate as i64 * 1024 * 300 / 1000;
    let l = f.sched.channels[&p].controller.ledbat().unwrap();
    assert!(l.min_cwnd() >= floor - 1024, "{} {floor}", l.min_cwnd());
    assert!(l.cwnd() >= l.min_cwnd());
    assert!(
        requests.len() as i64 >= floor / 1024 - 1,
        "{} {floor}",
        requests.len()
    );

    // Round trips past STREAM_WINDOW_FLOOR_RTT count as that.
    f.sched.channels[&p]
        .controller
        .ledbat_mut()
        .unwrap()
        .add_rtt_sample(Duration::from_secs(600));
    let _ = requests_at(&mut f, p, t);
    let cap = 2 * rate as i64 * 1024 * STREAM_WINDOW_FLOOR_RTT.as_millis() / 1000;
    assert!(f.sched.channels[&p].controller.ledbat().unwrap().min_cwnd() <= cap);
}

// --- push/pull hybrid ---------------------------------------------------------

fn hybrid(streams: usize) -> SwarmOptions {
    SwarmOptions {
        stream_count: streams,
        push_mode: PushMode::Hybrid,
        ..Default::default()
    }
}

fn add_ready(f: &mut Fixture, q: &mut MockQueue, i: usize) -> PeerKey {
    let p = f.peers[i];
    f.sched.add_channel(p, now(), q);
    f.sched.handle_handshake(p, 1 << 16);
    p
}

/// The messages `peer`'s channel writes next.
fn next_messages(f: &mut Fixture, peer: PeerKey, t: Time) -> Vec<Message<'static>> {
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    written(|cw| sched.write(peer, t, swarm, cw).unwrap(), cfg)
}

#[test]
fn hybrid_thicket_constraint_and_admission() {
    let mut f = fixture(hybrid(8));
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let r = add_ready(&mut f, &mut q, 1);

    // The first subscription a non-seed node accepts makes that stream its
    // interior stream.
    assert_eq!(
        f.sched
            .handle_stream_request(p, 2, Bin(0), Bin(0), now(), &mut q),
        Ok(true)
    );
    // Another stream is declined while no spare upload was measured.
    assert_eq!(
        f.sched
            .handle_stream_request(r, 5, Bin(0), Bin(0), now(), &mut q),
        Ok(true)
    );
    let st = f.sched.push_stats();
    assert_eq!((st.accepted, st.rejected, st.children), (1, 1, 1));
    assert_eq!(st.interior, Some(2));
    assert!(
        next_messages(&mut f, p, now())
            .iter()
            .any(|m| matches!(m, Message::StreamOpen(o) if o.stream == 2))
    );
    assert!(
        next_messages(&mut f, r, now())
            .iter()
            .any(|m| matches!(m, Message::StreamClose(c) if c.stream == 5))
    );

    // A saturated uplink still declines it ...
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.95,
    });
    f.sched
        .handle_stream_request(r, 5, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    assert_eq!(f.sched.push_stats().children, 1);
    // ... measured spare upload accepts it as an extra stream.
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.1,
    });
    f.sched
        .handle_stream_request(r, 5, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    let st = f.sched.push_stats();
    assert_eq!((st.accepted, st.rejected, st.children), (2, 2, 2));
    assert!(f.sched.s.streams[5].subscribers[0].extra);
    assert!(!f.sched.s.streams[2].subscribers[0].extra);

    // The seed is interior for every stream.
    let mut f = fixture(SwarmOptions {
        scheduling_method: SchedulingMethod::SEED,
        ..hybrid(8)
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    for s in 0..4 {
        f.sched
            .handle_stream_request(p, s, Bin(0), Bin(0), now(), &mut q)
            .unwrap();
    }
    let st = f.sched.push_stats();
    assert_eq!((st.accepted, st.rejected, st.interior), (4, 0, None));
}

#[test]
fn pull_load_does_not_shed_subscribers() {
    let mut f = fixture(SwarmOptions {
        scheduling_method: SchedulingMethod::SEED,
        ..hybrid(1)
    });
    let mut q = MockQueue::default();
    ready_channel(&mut f, &mut q);
    for i in 1..4 {
        add_ready(&mut f, &mut q, i);
    }
    for i in 0..4 {
        f.sched
            .handle_stream_request(f.peers[i], 0, Bin(0), Bin(0), now(), &mut q)
            .unwrap();
    }
    assert_eq!(f.sched.push_stats().children, 4);
    // About 320 chunks a second: a subscriber costs about 400 KB/s.
    let mut t = now();
    for i in 0..11 {
        t = now() + Duration::from_millis(100 + 200 * i);
        f.sched.s.push_consumed(Bin(0), 64, t);
    }

    // An uplink saturated by pull answers, with room for the pushes.
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000_000,
        busy: 0.95,
    });
    f.sched.run_push(t, &mut q);
    assert_eq!(f.sched.push_stats().children, 4);

    // Pushes over the interior target: enough subscribers are closed to
    // bring them under it.
    f.sched.set_upload(push::UploadInfo {
        rate: 1_390_000,
        busy: 0.95,
    });
    f.sched.run_push(t, &mut q);
    let st = f.sched.push_stats();
    assert_eq!((st.children, st.sheds_sent), (2, 2));
}

#[test]
fn hybrid_deadlines_release_to_pull_and_silence_prunes() {
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;

    // Subscribed to `parent`, which opened the stream.
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    assert_eq!(
        f.sched.handle_stream_open(parent, 0, Bin(0), now(), &mut q),
        Ok(StreamOpenOutcome::Accepted)
    );
    assert_eq!(f.sched.push_stats().parents, vec![Some(parent)]);

    // `other` announces chunks 0..=3: they are expected from the parent
    // (marked requested, with a deadline), so nobody pulls them.
    f.sched.handle_have(other, Bin::new(2, 0), now());
    assert!(f.sched.s.request_bins.filled_at(Bin::new(2, 0)));
    let deadline = f.sched.next_push_deadline().unwrap();
    assert!(deadline >= now() + push::DEADLINE_INITIAL, "{deadline:?}");
    assert!(
        !next_messages(&mut f, other, now())
            .iter()
            .any(|m| matches!(m, Message::Request(_)))
    );

    // At the deadline they go to pull, as misses, and every channel gets a
    // write cycle at once.
    f.sched
        .expire_push(deadline - Duration::MILLISECOND, &mut q);
    assert!(f.sched.s.request_bins.filled_at(Bin::new(2, 0)));
    q.enqueued_now.clear();
    f.sched.expire_push(deadline, &mut q);
    assert!(f.sched.s.request_bins.empty_at(Bin::new(2, 0)));
    assert_eq!(f.sched.push_stats().misses, 4);
    assert!(q.enqueued_now.iter().any(|&(p, _)| p == other));
    let requests = next_messages(&mut f, other, deadline)
        .iter()
        .filter(|m| matches!(m, Message::Request(_)))
        .count();
    assert_eq!(requests, 4);
    // Four misses in 300 ms don't prune ...
    assert_eq!(f.sched.push_stats().parents, vec![Some(parent)]);

    // ... but a parent silent for a second does: STREAM_CANCEL, and its
    // later chunks are pulled.
    let later = now() + push::PRUNE_SILENCE;
    f.sched.handle_have(other, Bin::new(2, 1), later);
    let deadline = f.sched.next_push_deadline().unwrap();
    f.sched.expire_push(deadline, &mut q);
    let st = f.sched.push_stats();
    assert_eq!((st.parents, st.pruned, st.misses), (vec![None], 1, 8));
    assert!(
        next_messages(&mut f, parent, deadline)
            .iter()
            .any(|m| matches!(m, Message::StreamCancel(c) if c.stream == 0))
    );
    // The pruned parent is backed off: the next check picks `other`.
    f.sched
        .run_push(deadline + push::PARENT_CHECK_INTERVAL, &mut q);
    assert!(
        next_messages(&mut f, other, deadline)
            .iter()
            .any(|m| matches!(m, Message::StreamRequest(r) if r.stream == 0))
    );
}

#[test]
fn hybrid_cancels_unsolicited_pushes() {
    // A peer pushing a stream we don't take from it (a lost STREAM_CANCEL,
    // an old parent) gets another STREAM_CANCEL, at most once a second.
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    for (i, t) in [(0u64, now()), (1, now()), (2, now() + Duration::SECOND)] {
        let b = Bin(i * 2);
        f.sched.consume(b, t, &f.swarm, &mut q);
        f.sched.handle_data(p, b, t, true, true, t, Bin(0), &mut q);
        let cancels = next_messages(&mut f, p, t)
            .iter()
            .filter(|m| matches!(m, Message::StreamCancel(_)))
            .count();
        assert_eq!(cancels, usize::from(i != 1), "{i}");
    }
    assert_eq!(f.sched.push_stats().unsolicited, 3);
}

#[test]
fn hybrid_rejects_opens_far_past_the_request() {
    // An open whose start is far past the request would never be expected
    // to deliver, so it could squat on the stream: it counts as a decline.
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    assert!(f.sched.do_stream_sub(p, 0, Bin(0)));
    let far = Bin(f.sched.s.live_window.0 + 2);
    assert_eq!(
        f.sched.handle_stream_open(p, 0, far, now(), &mut q),
        Ok(StreamOpenOutcome::Rejected)
    );
    let st = f.sched.push_stats();
    assert_eq!((st.parents, st.declined, st.opens), (vec![None], 1, 0));
    assert_eq!(f.sched.channels[&p].request_streams[0], Bin::NONE);
    assert!(
        next_messages(&mut f, p, now())
            .iter()
            .any(|m| matches!(m, Message::StreamCancel(_)))
    );
    // A later open from the same peer is unsolicited now.
    assert_eq!(
        f.sched.handle_stream_open(p, 0, Bin(0), now(), &mut q),
        Ok(StreamOpenOutcome::Rejected)
    );
}

#[test]
fn an_open_that_crossed_our_cancel_is_stale_not_rejected() {
    // A STREAM_REQUEST goes unanswered for the open timeout, we cancel it,
    // and the parent's STREAM_OPEN arrives after the cancel. Honest peers
    // race there; it isn't rejected input.
    for options in [eight_streams(), hybrid(8)] {
        let hybrid = options.push_mode == PushMode::Hybrid;
        let mut f = fixture(options);
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        let mut swarm = Swarm::new(f.swarm.id().clone(), f.swarm.options().clone()).unwrap();
        assert!(f.sched.do_stream_sub(p, 0, Bin(0)));
        assert!(f.sched.cancel_stream_sub(p, 0, &swarm, now(), &mut q));
        assert_eq!(
            f.sched.handle_stream_open(p, 0, Bin(0), now(), &mut q),
            Ok(StreamOpenOutcome::Stale),
            "hybrid {hybrid}"
        );
        // Only one answer can cross the cancel; a second open is
        // unsolicited.
        assert_eq!(
            f.sched.handle_stream_open(p, 0, Bin(0), now(), &mut q),
            Ok(StreamOpenOutcome::Rejected)
        );
        // An open for a stream we never asked for stays rejected.
        assert_eq!(
            f.sched.handle_stream_open(p, 1, Bin(0), now(), &mut q),
            Ok(StreamOpenOutcome::Rejected)
        );
        // A request the peer answered (OPEN, then our cancel) leaves no
        // open to race.
        assert!(f.sched.do_stream_sub(p, 2, Bin(0)));
        assert_eq!(
            f.sched.handle_stream_open(p, 2, Bin(0), now(), &mut q),
            Ok(StreamOpenOutcome::Accepted)
        );
        assert!(f.sched.cancel_stream_sub(p, 2, &swarm, now(), &mut q));
        assert_eq!(
            f.sched.handle_stream_open(p, 2, Bin(0), now(), &mut q),
            Ok(StreamOpenOutcome::Rejected)
        );
        // Nor does one it declined (CLOSE) before our cancel.
        assert!(f.sched.do_stream_sub(p, 3, Bin(0)));
        f.sched
            .handle_stream_close(p, 3, Bin(0), now(), &mut q)
            .unwrap();
        f.sched.cancel_stream_sub(p, 3, &swarm, now(), &mut q);
        assert_eq!(
            f.sched.handle_stream_open(p, 3, Bin(0), now(), &mut q),
            Ok(StreamOpenOutcome::Rejected)
        );
        f.sched.run(now(), &mut swarm, &mut q);
    }
}

#[test]
fn a_stream_sub_waits_for_the_handshake() {
    // A STREAM_OPEN before the peer's handshake is ignored, so a request
    // sent before it could go unanswered.
    for options in [eight_streams(), hybrid(8)] {
        let hybrid = options.push_mode == PushMode::Hybrid;
        let mut f = fixture(options);
        let mut q = MockQueue::default();
        let p = f.peers[0];
        f.sched.add_channel(p, now(), &mut q);
        let requests = |f: &mut Fixture| {
            next_messages(f, p, now())
                .iter()
                .filter(|m| matches!(m, Message::StreamRequest(_)))
                .count()
        };

        assert!(!f.sched.do_stream_sub(p, 0, Bin(0)), "hybrid {hybrid}");
        assert_eq!(f.sched.channels[&p].request_streams[0], Bin::NONE);
        assert_eq!(f.sched.stats().stream_sources, 0);
        assert_eq!(f.sched.push_stats().requests_sent, 0);
        assert_eq!(requests(&mut f), 0, "hybrid {hybrid}");

        f.sched.handle_handshake(p, 1 << 16);
        assert!(f.sched.do_stream_sub(p, 0, Bin(0)), "hybrid {hybrid}");
        assert_eq!(requests(&mut f), 1, "hybrid {hybrid}");
    }
}

#[test]
fn an_open_before_the_handshake_is_ignored() {
    // Without the peer's handshake no request of ours can be waiting for
    // the open. After a restart, opens answering the old stream's requests
    // arrive here, so they don't count as rejected either.
    for options in [eight_streams(), hybrid(8)] {
        let hybrid = options.push_mode == PushMode::Hybrid;
        let mut f = fixture(options);
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        assert!(f.sched.do_stream_sub(p, 0, Bin(0)));
        f.sched.reset(now(), &mut q);

        // A channel with no handshake yet, and a peer with no channel.
        let fresh = f.peers[1];
        f.sched.add_channel(fresh, now(), &mut q);
        for peer in [p, fresh, f.peers[2]] {
            assert_eq!(
                f.sched.handle_stream_open(peer, 0, Bin(0), now(), &mut q),
                Ok(StreamOpenOutcome::BeforeHandshake),
                "hybrid {hybrid}"
            );
        }
        assert_eq!(f.sched.channels[&p].request_streams[0], Bin::NONE);
        assert_eq!(f.sched.stats().stream_sources, 0);
        assert!(f.sched.push_stats().parents.iter().all(Option::is_none));
    }
}
