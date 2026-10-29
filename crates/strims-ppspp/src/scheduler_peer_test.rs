// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduler tests over a mock peer queue: requests, stream subscriptions,
//! timeouts, the push/pull hybrid, and the bounds on peer input.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::push::seed::PushFate;
use super::*;
use crate::codec::{Announce, AnnounceEntry, Message, Reader};
use crate::{SwarmId, SwarmOptions};
use slotmap::SlotMap;
use std::collections::BTreeMap;
use strims_base::rng::det_rng;

/// A [`PeerTaskQueue`] that records the calls.
#[derive(Debug, Default)]
pub(crate) struct MockQueue {
    pub(crate) enqueued: Vec<(PeerKey, SwarmKey)>,
    pub(crate) enqueued_now: Vec<(PeerKey, SwarmKey)>,
    pub(crate) pushed: Vec<(PeerKey, Bin, PeerPriority)>,
    pub(crate) pushed_front: Vec<(PeerKey, Bin, PeerPriority)>,
    pub(crate) removed: Vec<(PeerKey, Bin, PeerPriority)>,
    pub(crate) removed_runner: Vec<PeerKey>,
}

impl PeerTaskQueue for MockQueue {
    fn enqueue(&mut self, peer: PeerKey, swarm: SwarmKey) {
        self.enqueued.push((peer, swarm));
    }
    fn enqueue_now(&mut self, peer: PeerKey, swarm: SwarmKey) {
        self.enqueued_now.push((peer, swarm));
    }
    fn push_data(
        &mut self,
        peer: PeerKey,
        _: SwarmKey,
        b: Bin,
        _: Time,
        pri: PeerPriority,
    ) -> bool {
        self.pushed.push((peer, b, pri));
        true
    }
    fn push_front_data(&mut self, peer: PeerKey, _: SwarmKey, b: Bin, _: Time, pri: PeerPriority) {
        self.pushed_front.push((peer, b, pri));
    }
    fn remove_data(&mut self, peer: PeerKey, _: SwarmKey, b: Bin, pri: PeerPriority) {
        self.removed.push((peer, b, pri));
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
    fixture_with(Swarm::new(SwarmId::new(&key.public), options).unwrap())
}

/// A scheduler for `swarm`, with four peer keys.
fn fixture_with(swarm: Swarm) -> Fixture {
    fixture_peers(swarm, 4)
}

/// A scheduler for `swarm`, with `n` peer keys.
fn fixture_peers(swarm: Swarm, n: usize) -> Fixture {
    let mut keys: SlotMap<SwarmKey, ()> = SlotMap::with_key();
    let key = keys.insert(());
    let mut peers: SlotMap<PeerKey, ()> = SlotMap::with_key();
    let peers = (0..n).map(|_| peers.insert(())).collect();
    let sched = PeerSwarmScheduler::new(&swarm, key, &mut det_rng(1), now());
    Fixture {
        swarm,
        sched,
        peers,
    }
}

/// Options for the pull scheduler's tests: the mechanisms they test run
/// without push.
fn pull() -> SwarmOptions {
    SwarmOptions {
        push_mode: PushMode::Off,
        ..Default::default()
    }
}

fn eight_streams() -> SwarmOptions {
    SwarmOptions {
        stream_count: 8,
        ..pull()
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
    f.sched.add_channel(p, &[], now(), &mut q);
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
    f.sched.add_channel(p, &[], now(), &mut q);
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
    f.sched.add_channel(p, &[], now(), &mut q);
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
    f.sched.add_channel(p, &[1], now(), q);
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
fn a_far_have_leaves_a_seeds_publish_times() {
    // A seed's bin times are its publish times: a HAVE far past the
    // stream's head prunes none of them.
    let mut f = fixture(SwarmOptions {
        scheduling_method: SchedulingMethod::SEED,
        ..hybrid(1)
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.set_bin_time(Bin::new(6, 0), now());
    assert!(
        f.sched
            .handle_have(p, Bin(1 << 40), now() + Duration::from_millis(10))
    );
    assert!(
        f.sched
            .handle_have(p, Bin(0), now() + Duration::from_millis(80))
    );
    assert_eq!(f.sched.s.bin_time(Bin(0)), now());
    assert_eq!(f.sched.s.bin_times_max, Bin::new(6, 0).base_right());
}

#[test]
fn sent_chunks_count_pushes_and_distinct_chunks() {
    let mut f = fixture(hybrid(1));
    let ms = Duration::from_millis;
    let t = now();
    f.sched
        .s
        .count_sent(Bin::new(1, 0), PeerPriority::High, t, t + ms(30));
    f.sched
        .s
        .count_sent(Bin(2), PeerPriority::Low, t, t + ms(5));
    f.sched
        .s
        .count_sent(Bin(4), PeerPriority::Low, t, t + ms(500));
    let st = f.sched.stats();
    assert_eq!(
        (st.chunks_sent, st.pushed_chunks, st.distinct_chunks_sent),
        (4, 2, 3)
    );
    assert_eq!((st.push_wait.count(), st.answer_wait.count()), (1, 2));
    let p = st.push_wait.percentile(50.0).unwrap();
    assert!((ms(30)..=ms(36)).contains(&p), "{p}");
}

#[test]
fn have_flood_is_bounded() {
    let mut f = fixture(SwarmOptions {
        live_window: 1 << 10,
        ..pull()
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
    let mut f = fixture(pull());
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
    let mut f = fixture(pull());
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
    let mut f = fixture(pull());
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

#[test]
fn pong_needs_an_outstanding_ping() {
    let mut f = fixture(pull());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    // Nonce 0 before any ping isn't a round trip (it would be one since
    // 1970).
    f.sched.handle_pong(p, 0, now());
    let c = &f.sched.channels[&p].controller;
    assert_eq!(c.request_timeout(), Duration::SECOND);
    assert_eq!(c.rtt(), Duration::ZERO);

    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    let Message::Ping(ping) = msgs[0] else {
        panic!("{msgs:?}")
    };
    f.sched
        .handle_pong(p, ping.nonce, now() + Duration::from_millis(40));
    let c = &f.sched.channels[&p].controller;
    assert_eq!(c.rtt(), Duration::from_millis(40));
    // A repeat is ignored.
    f.sched
        .handle_pong(p, ping.nonce, now() + Duration::from_secs(30));
    let c = &f.sched.channels[&p].controller;
    assert_eq!(c.rtt(), Duration::from_millis(40));
}

#[test]
fn a_superseded_ping_skips_the_controller() {
    let mut f = fixture(pull());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let cfg = f.swarm.reader_config();
    let mut nonces = Vec::new();
    for at in [now(), now() + PING_INTERVAL + Duration::from_millis(100)] {
        let (sched, swarm) = (&mut f.sched, &f.swarm);
        let msgs = written(|cw| sched.write(p, at, swarm, cw).unwrap(), cfg);
        let Message::Ping(ping) = msgs[0] else {
            panic!("{msgs:?}")
        };
        nonces.push(ping.nonce);
    }
    let back = now() + Duration::from_millis(2500);
    f.sched.handle_pong(p, nonces[0], back);
    assert_eq!(f.sched.channels[&p].controller.rtt(), Duration::ZERO);
    f.sched.handle_pong(p, nonces[1], back);
    assert_eq!(
        f.sched.channels[&p].controller.rtt(),
        Duration::from_millis(1400)
    );
}

#[test]
fn requests_are_bounded_by_the_window() {
    let mut f = fixture(SwarmOptions {
        live_window: 256,
        ..pull()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    f.sched.handle_have(p, Bin::new(8, 0), now());
    // A huge congestion window.
    for _ in 0..200 {
        f.sched.channels[&p].controller.add_delay_sample(
            Duration::from_millis(-1_000_000_000),
            1 << 40,
            now(),
        );
        f.sched.channels[&p].controller.digest_delay_samples(now());
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
    let mut f = fixture(pull());
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
    let mut f = fixture(pull());
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
    let mut f = fixture(pull());
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
    f.sched.gc(&f.swarm, now());
    assert_eq!(f.sched.channels[&p].request_times.size(), before);
}

#[test]
fn unanswered_requests_filled_elsewhere_leave_the_flight() {
    // A request the peer never answers, for a chunk that arrives from
    // elsewhere first (a push), times out with the chunk already stored.
    // It must leave the flight without a loss: two such requests stuck in
    // flight would fill the minimum window and stop the channel.
    let mut f = fixture(pull());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let other = f.peers[1];
    f.sched.add_channel(other, &[], now(), &mut q);
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
    assert_eq!(c.controller.in_flight(), 0);
    assert!(c.cancel_bins.is_empty());
    assert!(c.backoff_bins.is_empty());
}

#[test]
fn requests_pruned_behind_the_store_leave_the_flight() {
    // A request filled elsewhere whose time GC prunes (the store moved past
    // it) before its deadline: the deadline can't settle it, so the prune
    // must. Stranded flight accumulates until it fills the window, and a
    // channel whose flight never drains stops requesting for good.
    let mut f = fixture(pull());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let other = f.peers[1];
    f.sched.add_channel(other, &[], now(), &mut q);
    f.sched.handle_handshake(other, 1 << 16);
    f.sched.s.first_chunk_set = true;
    f.swarm.set_offset(Bin(0));
    f.sched.handle_have(p, Bin::new(1, 0), now());
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    assert_eq!(
        msgs.iter()
            .filter(|m| matches!(m, Message::Request(_)))
            .count(),
        2
    );
    assert_eq!(f.sched.channels[&p].controller.in_flight(), 2048);

    // Both chunks come from the other peer and fill the store past them.
    let chunk = vec![0u8; f.swarm.options().chunk_size];
    for b in [Bin(0), Bin(2)] {
        f.swarm.publish(b, &chunk).unwrap();
        f.sched.consume(b, now(), &f.swarm, &mut q);
        f.sched
            .handle_data(other, b, now(), true, true, now(), Bin(0), &mut q);
    }
    assert_eq!(f.swarm.store().next(), Bin(4));
    f.sched.gc(&f.swarm, now());
    assert_eq!(f.sched.channels[&p].controller.in_flight(), 0);

    let later = now() + Duration::from_secs(2);
    let mut swarm = f.swarm;
    f.sched.run(later, &mut swarm, &mut q);
    let c = &f.sched.channels[&p];
    assert_eq!(c.controller.in_flight(), 0);
    assert!(c.backoff_bins.is_empty());
}

#[test]
fn channels_ping_once_a_second() {
    let mut f = fixture(SwarmOptions {
        push_mode: PushMode::Hybrid,
        ..Default::default()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let cfg = f.swarm.reader_config();
    let mut n = 0;
    for ms in (0..3000).step_by(100) {
        let (sched, swarm) = (&mut f.sched, &f.swarm);
        let at = now() + Duration::from_millis(ms);
        let msgs = written(|cw| sched.write(p, at, swarm, cw).unwrap(), cfg);
        n += msgs
            .iter()
            .filter(|m| matches!(m, Message::Ping(_)))
            .count();
    }
    assert_eq!(n, 3);
}

#[test]
fn pongs_reach_the_controller_and_path_samples_dont() {
    let mut f = fixture(pull());
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    let Some(Message::Ping(ping)) = msgs.first() else {
        panic!("{msgs:?}")
    };
    f.sched
        .handle_pong(p, ping.nonce, now() + Duration::from_millis(40));
    f.sched
        .handle_path_sample(p, Duration::from_millis(70), None, now());
    assert_eq!(
        f.sched.channels[&p].controller.rtt(),
        Duration::from_millis(40)
    );
}

#[test]
fn path_samples_rank_a_seeds_holders_and_pongs_dont() {
    let mut f = adaptive_seed(4, 2);
    let mut q = MockQueue::default();
    let p = add_ready(&mut f, &mut q, 0);
    let min_rtt = |f: &mut Fixture| {
        let a = f.sched.seed_allocation(now()).unwrap();
        a.neighbours.iter().find(|n| n.peer == p).unwrap().min_rtt
    };
    let cfg = f.swarm.reader_config();
    let (sched, swarm) = (&mut f.sched, &f.swarm);
    let msgs = written(|cw| sched.write(p, now(), swarm, cw).unwrap(), cfg);
    let Some(ping) = msgs.iter().find_map(|m| match m {
        Message::Ping(x) => Some(x.nonce),
        _ => None,
    }) else {
        panic!("{msgs:?}")
    };
    f.sched
        .handle_pong(p, ping, now() + Duration::from_millis(10));
    assert_eq!(min_rtt(&mut f), None);
    f.sched
        .handle_path_sample(p, Duration::from_millis(30), None, now());
    f.sched
        .handle_path_sample(p, Duration::from_millis(20), None, now());
    assert_eq!(min_rtt(&mut f), Some(Duration::from_millis(20)));
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
        ..pull()
    });
    let mut q = MockQueue::default();
    let p = f.peers[0];
    f.sched.add_channel(p, &[], now(), &mut q);
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
        ..pull()
    });
    f2.sched.add_channel(p, &[], now(), &mut q);
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
        ..pull()
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
        ..pull()
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
        ..pull()
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
        let l = &mut f.sched.channels[&p].controller;
        // 10 ms one way, then 150 ms: read as 140 ms of queueing.
        l.add_delay_sample(Duration::from_millis(10), 1024, t);
        l.digest_delay_samples(t);
        for _ in 0..500 {
            l.add_sent(8192);
            l.add_delay_sample(Duration::from_millis(150), 8192, t);
            l.digest_delay_samples(t);
        }
        assert_eq!(l.cwnd(), 2 * 1024);
        l.add_rtt_sample(Duration::from_millis(300), t);
    }
    let requests = requests_at(&mut f, p, t);
    let floor = 2 * rate as i64 * 1024 * 300 / 1000;
    let l = &f.sched.channels[&p].controller;
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
        .add_rtt_sample(Duration::from_secs(600), t);
    let _ = requests_at(&mut f, p, t);
    let cap = 2 * rate as i64 * 1024 * STREAM_WINDOW_FLOOR_RTT.as_millis() / 1000;
    assert!(f.sched.channels[&p].controller.min_cwnd() <= cap);
}

#[test]
fn a_window_at_its_floor_still_asks_for_a_chunk() {
    // Twice a slow stream over a short path is less than a 4 KiB chunk,
    // and so is LEDBAT's minimum: a window held there by queueing delay
    // would ask for nothing, and never get the answer it needs to grow.
    let mut f = fixture(SwarmOptions {
        chunk_size: 4096,
        live_window: 1 << 10,
        chunks_per_signature: 8,
        ..pull()
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    f.sched.s.first_chunk_set = true;
    // 8 chunks a second.
    let mut t = now();
    for i in 0..40u64 {
        t = now() + Duration::from_millis(125 * i as i64);
        assert!(f.sched.handle_have(p, Bin::new(0, i), t));
    }
    {
        let l = &mut f.sched.channels[&p].controller;
        l.add_rtt_sample(Duration::from_millis(5), t);
        l.add_delay_sample(Duration::from_millis(10), 1024, t);
        l.digest_delay_samples(t);
        for _ in 0..500 {
            l.add_sent(8192);
            l.add_delay_sample(Duration::from_millis(150), 8192, t);
            l.digest_delay_samples(t);
        }
        assert!(l.cwnd() < 4096, "{}", l.cwnd());
        assert_eq!(l.in_flight(), 0);
    }
    assert_eq!(requests_at(&mut f, p, t).len(), 1);
    assert_eq!(f.sched.channels[&p].controller.min_cwnd(), 4096);
}

#[test]
fn hybrid_window_floor_reads_the_minimum_round_trip() {
    for (options, on_min) in [(pull(), false), (hybrid(4), true)] {
        let mut f = fixture(SwarmOptions { ..options });
        let mut q = MockQueue::default();
        let p = ready_channel(&mut f, &mut q);
        let l = &f.sched.channels[&p].controller;
        assert_eq!(l.floor_on_min(), on_min);
    }
}

// --- push/pull hybrid ---------------------------------------------------------

/// Hybrid at `streams` streams, a seed admitting subscribers first come
/// (the adaptive seed's tests name their policy).
fn hybrid(streams: usize) -> SwarmOptions {
    SwarmOptions {
        stream_count: streams,
        push_mode: PushMode::Hybrid,
        seed: crate::SeedPolicy::ADMISSION,
        ..Default::default()
    }
}

fn add_ready(f: &mut Fixture, q: &mut MockQueue, i: usize) -> PeerKey {
    let p = f.peers[i];
    f.sched.add_channel(p, &[i as u8 + 1], now(), q);
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
        ..Default::default()
    });
    f.sched
        .handle_stream_request(r, 5, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    assert_eq!(f.sched.push_stats().children, 1);
    // ... measured spare upload accepts it as an extra stream.
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.1,
        ..Default::default()
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
        ..Default::default()
    });
    f.sched.run_push(t, &mut q);
    assert_eq!(f.sched.push_stats().children, 4);

    // Pushes over the interior target: enough subscribers are closed to
    // bring them under it.
    f.sched.set_upload(push::UploadInfo {
        rate: 1_390_000,
        busy: 0.95,
        ..Default::default()
    });
    f.sched.run_push(t, &mut q);
    let st = f.sched.push_stats();
    assert_eq!((st.children, st.sheds_sent), (2, 2));
}

#[test]
fn other_swarms_pushes_count_against_admission() {
    let mut f = fixture(seed_hybrid(4));
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let r = add_ready(&mut f, &mut q, 1);
    // A measured uplink with room ...
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.1,
        ..Default::default()
    });
    f.sched
        .handle_stream_request(p, 0, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    assert_eq!(f.sched.push_stats().children, 1);
    // ... that the node's other swarms have committed.
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.1,
        others: 900_000.0,
        ..Default::default()
    });
    f.sched
        .handle_stream_request(r, 1, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    assert_eq!(f.sched.push_stats().children, 1);
}

#[test]
fn a_swarm_sheds_its_share_of_the_nodes_excess() {
    // Four subscribers at about 410 KB/s each on a saturated 2 MB/s
    // uplink, whose interior target is 1.4 MB/s.
    let shed = |others: f64| {
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
        let mut t = now();
        for i in 0..11 {
            t = now() + Duration::from_millis(100 + 200 * i);
            f.sched.s.push_consumed(Bin(0), 64, t);
        }
        f.sched.set_upload(push::UploadInfo {
            rate: 1_900_000,
            busy: 0.95,
            others,
            ..Default::default()
        });
        f.sched.run_push(t, &mut q);
        f.sched.push_stats().sheds_sent
    };
    // Alone, about 240 KB/s over: one subscriber.
    assert_eq!(shed(0.0), 1);
    // With as much again from the node's other swarms, about 1.9 MB/s
    // over, half of it this swarm's: three.
    assert_eq!(shed(1_638_400.0), 3);
}

fn seed_hybrid(streams: usize) -> SwarmOptions {
    SwarmOptions {
        scheduling_method: SchedulingMethod::SEED,
        ..hybrid(streams)
    }
}

/// Whether `peer` subscribes to `stream`.
fn subscribed(f: &Fixture, peer: PeerKey, stream: usize) -> bool {
    f.sched.s.streams[stream]
        .subscribers
        .iter()
        .any(|x| x.channel == peer)
}

#[test]
fn a_pinned_seed_pushes_each_stream_to_its_pin() {
    let mut f = fixture(seed_hybrid(4));
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let r = add_ready(&mut f, &mut q, 1);
    f.sched
        .handle_stream_request(r, 0, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    assert!(subscribed(&f, r, 0));

    // Pinning closes the subscribers outside the pin.
    assert!(
        f.sched
            .pin_allocation(Some(vec![vec![p], vec![r], vec![], vec![]]), &mut q)
    );
    assert!(!subscribed(&f, r, 0));
    assert!(
        next_messages(&mut f, r, now())
            .iter()
            .any(|m| matches!(m, Message::StreamClose(c) if c.stream == 0))
    );
    // Only the pinned peers' requests are accepted, whatever the upload.
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.1,
        ..Default::default()
    });
    for (peer, stream) in [(p, 0), (r, 0), (p, 1), (r, 1), (p, 2)] {
        f.sched
            .handle_stream_request(peer, stream, Bin(0), Bin(0), now(), &mut q)
            .unwrap();
    }
    assert!(subscribed(&f, p, 0) && subscribed(&f, r, 1));
    assert!(!subscribed(&f, r, 0) && !subscribed(&f, p, 1) && !subscribed(&f, p, 2));
    // Nothing is shed while pinned.
    f.sched.set_upload(push::UploadInfo {
        rate: 1_000,
        busy: 0.99,
        ..Default::default()
    });
    f.sched.run_push(now() + Duration::from_secs(10), &mut q);
    assert_eq!(f.sched.push_stats().children, 2);

    // Lifted, the admission decides again.
    assert!(f.sched.pin_allocation(None, &mut q));
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.1,
        ..Default::default()
    });
    f.sched
        .handle_stream_request(p, 2, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    assert!(subscribed(&f, p, 2));

    // A non-seed can't be pinned.
    let mut f = fixture(hybrid(4));
    assert!(!f.sched.pin_allocation(Some(vec![vec![]; 4]), &mut q));
}

fn adaptive_seed(streams: usize, peers: usize) -> Fixture {
    adaptive_seed_with(streams, peers, crate::SeedQos::ByAllocation)
}

fn adaptive_seed_with(streams: usize, peers: usize, qos: crate::SeedQos) -> Fixture {
    let key = strims_crypto::key_from_seed(&[5; 32]);
    let options = SwarmOptions {
        seed: crate::SeedPolicy {
            allocation: crate::SeedAllocation::Adaptive,
            qos,
        },
        ..seed_hybrid(streams)
    };
    fixture_peers(
        Swarm::new(SwarmId::new(&key.public), options).unwrap(),
        peers,
    )
}

/// The peers a seed's allocation holds copies at.
fn holders(f: &Fixture) -> Vec<PeerKey> {
    let a = f.sched.seed_allocation(now()).unwrap();
    a.neighbours
        .iter()
        .filter(|n| n.holder)
        .map(|n| n.peer)
        .collect()
}

/// Publishes about 320 chunks a second after `from` until `to`, so a
/// stream's subscriber costs about 400 KB/s over the stream count.
fn publish(f: &mut Fixture, from: Time, to: Time) {
    let mut at = from;
    while at < to {
        at += Duration::from_millis(200);
        f.sched.s.push_consumed(Bin(0), 64, at);
    }
}

/// A chunk pushed at `t`: one per millisecond since the fixture's start.
fn pushed_bin(t: Time) -> Bin {
    Bin(2 * (t - now()).as_millis().max(0) as u64)
}

/// A push queued at `queued` written to `p` at `at`, which `p` announces
/// once it arrived, `announce` later (never if `None`).
fn push_to(f: &mut Fixture, p: PeerKey, queued: Time, at: Time, announce: Option<Duration>) {
    let b = pushed_bin(at);
    f.sched.note_push(p, b, queued, PushFate::Written, at);
    if let Some(d) = announce {
        f.sched.handle_have(p, b, at + d);
    }
}

/// One check interval of the adaptive seed from `t`: publishing, a push
/// written to each of `pushed` after `wait` and announced, and the uplink
/// busy `busy` of the time at `rate` bytes a second.
fn alloc_check(
    f: &mut Fixture,
    q: &mut MockQueue,
    t: Time,
    pushed: &[PeerKey],
    wait: Duration,
    (rate, busy): (u64, f64),
) -> Time {
    let next = t + push::seed::ALLOC_CHECK_INTERVAL;
    publish(f, t, next);
    for &p in pushed {
        push_to(f, p, t, t + wait, Some(Duration::from_millis(20)));
    }
    f.sched.set_upload(push::UploadInfo {
        rate,
        busy,
        ..Default::default()
    });
    f.sched.run_push(next, q);
    next
}

/// Six neighbours of an adaptive seed at 4 streams, with round trips of
/// 10, 20, ... 60 ms.
fn adaptive_seed_with_rtts() -> (Fixture, MockQueue, Vec<PeerKey>) {
    let mut f = adaptive_seed(4, 6);
    let mut q = MockQueue::default();
    let peers: Vec<PeerKey> = (0..6).map(|i| add_ready(&mut f, &mut q, i)).collect();
    for (i, &p) in peers.iter().enumerate() {
        let rtt = Duration::from_millis(10 * (i as i64 + 1));
        push::seed::round_trip(&mut f.sched.channels[&p].push, rtt, now());
    }
    (f, q, peers)
}

/// `p` asks the seed for stream `s` at `t`; whether it is subscribed.
fn ask(f: &mut Fixture, q: &mut MockQueue, p: PeerKey, s: u16, t: Time) -> bool {
    f.sched
        .handle_stream_request(p, s, Bin(0), Bin(0), t, q)
        .unwrap();
    subscribed(f, p, usize::from(s))
}

/// The upload a first wave's ramp step reads: no known capacity, a rate
/// whose reading pays for every neighbour's copy, busy `busy` of the last
/// 300 ms, the links' latest excess `delay_ms`.
fn ramp_upload(f: &mut Fixture, busy: f64, delay_ms: i64) {
    f.sched.set_upload(push::UploadInfo {
        rate: 1_000_000,
        busy_recent: busy,
        push_busy_recent: busy,
        latest_delay: Some(Duration::from_millis(delay_ms)),
        ..Default::default()
    });
}

/// Whether `p` was sent a STREAM_CLOSE for `stream`.
fn declined(f: &mut Fixture, p: PeerKey, stream: u16, t: Time) -> bool {
    next_messages(f, p, t)
        .iter()
        .any(|m| matches!(m, Message::StreamClose(c) if c.stream == stream))
}

#[test]
fn without_a_known_capacity_the_first_wave_ramps_and_holds_the_rest() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let step = push::seed::RAMP_STEP;
    publish(&mut f, now() - Duration::SECOND, now());
    // The meter reads 20 times what went out (nothing waited in the runner):
    // a reading that sizes nothing yet.
    ramp_upload(&mut f, 0.0, 0);
    for &p in &peers {
        ask(&mut f, &mut q, p, 0, now());
    }
    // Two copies, to the first two that asked; the rest wait, undeclined.
    assert_eq!(holders(&f), peers[..2].to_vec());
    for &p in &peers[2..] {
        assert!(!subscribed(&f, p, 0) && !declined(&mut f, p, 0, now()));
    }
    // The uplink keeps up: twice the copies a step later, to the next to
    // have asked.
    let mut t = now() + step;
    f.sched.run_push(t, &mut q);
    assert_eq!(holders(&f), peers[..4].to_vec());
    // A queue forms below the runner: a copy more.
    ramp_upload(&mut f, 0.0, 30);
    t += step;
    f.sched.run_push(t, &mut q);
    assert_eq!(holders(&f), peers[..5].to_vec());
    // Past the ramp's queue: it holds a step (a lone delayed echo may be a
    // retransmission), and on the second stops at half; the one still
    // waiting was declined at PENDING_MAX.
    let window = |f: &Fixture, t| f.sched.seed_allocation(t).unwrap().window;
    ramp_upload(&mut f, 0.0, 60);
    t += step;
    f.sched.run_push(t, &mut q);
    assert_eq!(window(&f, t), 5);
    t += step;
    f.sched.run_push(t, &mut q);
    assert_eq!(window(&f, t), 2);
    assert!(declined(&mut f, peers[5], 0, t));
    assert!(!subscribed(&f, peers[5], 0));
}

#[test]
fn the_ramp_grows_only_while_its_copies_are_held() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let step = push::seed::RAMP_STEP;
    let window = |f: &Fixture, t| f.sched.seed_allocation(t).unwrap().window;
    publish(&mut f, now() - Duration::SECOND, now());
    ramp_upload(&mut f, 0.0, 0);
    for &p in &peers[..2] {
        ask(&mut f, &mut q, p, 0, now());
    }
    // Both copies held: it doubles. Then two of four are held, and it
    // holds however idle the uplink, until the rest ask.
    let mut t = now() + step;
    f.sched.run_push(t, &mut q);
    assert_eq!(window(&f, t), 4);
    t += step;
    f.sched.run_push(t, &mut q);
    assert_eq!(window(&f, t), 4);
    for &p in &peers[2..] {
        ask(&mut f, &mut q, p, 0, t);
    }
    assert_eq!(holders(&f), peers[..4].to_vec());
    t += step;
    f.sched.run_push(t, &mut q);
    assert_eq!(holders(&f), peers.to_vec());
}

#[test]
fn the_ramp_stops_at_the_copies_the_reading_pays_for() {
    // The busy-time reading pays for three copies (150 KB/s at no busy
    // time reads 3 MB/s): with no queue to stop it, the ramp stops there
    // rather than doubling to four.
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let step = push::seed::RAMP_STEP;
    let window = |f: &Fixture, t| f.sched.seed_allocation(t).unwrap().window;
    publish(&mut f, now() - Duration::SECOND, now());
    f.sched.set_upload(push::UploadInfo {
        rate: 150_000,
        latest_delay: Some(Duration::ZERO),
        ..Default::default()
    });
    for &p in &peers {
        ask(&mut f, &mut q, p, 0, now());
    }
    let mut t = now();
    for _ in 0..4 {
        publish(&mut f, t, t + step);
        t += step;
        f.sched.run_push(t, &mut q);
    }
    assert_eq!(window(&f, t), 3);
}

#[test]
fn a_held_request_is_declined_inside_the_childs_open_timeout() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    publish(&mut f, now() - Duration::SECOND, now());
    ramp_upload(&mut f, 0.0, 30);
    for &p in &peers {
        ask(&mut f, &mut q, p, 0, now());
    }
    // A copy more per step, as a queue forms; at PENDING_MAX the requests
    // the copies didn't reach are declined, before the child gives up.
    let t = now() + push::seed::PENDING_MAX;
    assert!(t < now() + push::OPEN_TIMEOUT);
    f.sched.run_push(t, &mut q);
    assert_eq!(holders(&f), peers[..3].to_vec());
    for &p in &peers[3..] {
        assert!(declined(&mut f, p, 0, t));
    }
}

#[test]
fn a_known_capacity_sizes_the_first_wave_at_once() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    publish(&mut f, now() - Duration::SECOND, now());
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        hint: Some(10_000_000),
        ..Default::default()
    });
    for &p in &peers {
        ask(&mut f, &mut q, p, 0, now());
    }
    assert_eq!(holders(&f), peers.to_vec());
}

#[test]
fn an_adaptive_seed_pushes_whole_copies_to_its_nearest() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    // With nothing measured the window is one copy: the first to ask
    // holds it, here the farthest.
    for &p in peers.iter().rev() {
        ask(&mut f, &mut q, p, 0, now());
    }
    assert_eq!(holders(&f), vec![peers[5]]);
    // A holder may take every stream; anyone else none.
    assert!(ask(&mut f, &mut q, peers[5], 1, now()));
    assert!(!ask(&mut f, &mut q, peers[0], 1, now()));

    // Its pushes keep up: the window doubles at each check while it leads
    // the copies in use at most twice, up to what the uplink pays for at
    // the store rate so far, and the free copies go to the nearest that
    // asked.
    let ms = Duration::from_millis;
    let t = alloc_check(&mut f, &mut q, now(), &[peers[5]], ms(5), (1_200_000, 0.5));
    assert_eq!(holders(&f), vec![peers[0], peers[5]]);
    let t = alloc_check(&mut f, &mut q, t, &[peers[5]], ms(5), (1_200_000, 0.5));
    assert_eq!(holders(&f), vec![peers[0], peers[5]]);
    assert!(ask(&mut f, &mut q, peers[0], 0, t));
    let mut t = alloc_check(&mut f, &mut q, t, &[peers[5]], ms(5), (1_200_000, 0.5));
    assert_eq!(holders(&f), vec![peers[0], peers[1], peers[5]]);
    t = alloc_check(&mut f, &mut q, t, &[peers[5]], ms(5), (1_200_000, 0.5));
    assert_eq!(holders(&f), vec![peers[0], peers[1], peers[2], peers[5]]);
    for &p in &peers[1..3] {
        assert!(ask(&mut f, &mut q, p, 0, t));
    }
    assert!(!ask(&mut f, &mut q, peers[3], 0, t));

    // Once measured, a budget of two copies (8 subscribers at about
    // 100 KB/s, at 70 % of the capacity): the farthest go, losing their
    // subscriptions.
    t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(5), (1_200_000, 1.0));
    assert_eq!(holders(&f), peers[..2].to_vec());
    assert!(!subscribed(&f, peers[5], 0) && !subscribed(&f, peers[5], 1));
    assert!(ask(&mut f, &mut q, peers[1], 2, t));

    // A holder that gives up every subscription gives its copy back once
    // the grace since it last asked has passed.
    for s in [0, 2] {
        f.sched.handle_stream_cancel(peers[1], s).unwrap();
    }
    t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(5), (1_200_000, 1.0));
    assert_eq!(holders(&f), peers[..2].to_vec());
    let asked = t - push::seed::ALLOC_CHECK_INTERVAL;
    while t <= asked + push::seed::HOLDER_GRACE {
        t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(5), (1_200_000, 1.0));
    }
    assert_eq!(holders(&f), vec![peers[0]]);
    assert!(ask(&mut f, &mut q, peers[2], 0, t));

    // With room for everyone's every stream, the window grows into it, and
    // then everyone is accepted.
    for _ in 0..3 {
        t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(5), (100_000_000, 0.5));
    }
    assert!(ask(&mut f, &mut q, peers[4], 3, t));
}

#[test]
fn a_holder_whose_link_stands_a_queue_spills_its_streams() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let ms = Duration::from_millis;
    for s in 0..4 {
        assert!(ask(&mut f, &mut q, peers[0], s, now()));
    }
    // A check at which peers[0]'s link stood 300 ms of queue, on an uplink
    // with no queue of its own (or with one: `draining`).
    let check = |f: &mut Fixture, q: &mut MockQueue, t: Time, draining: bool| -> Time {
        let next = t + push::seed::ALLOC_CHECK_INTERVAL;
        publish(f, t, next);
        push_to(f, peers[0], t, t + ms(5), Some(ms(20)));
        push::seed::round_trip(&mut f.sched.channels[&peers[0]].push, ms(310), t + ms(500));
        // One copy's budget (600 KB/s at 70 % pays for four subscribers at
        // about 100 KB/s).
        f.sched.set_upload(push::UploadInfo {
            rate: 600_000,
            busy: 1.0,
            draining,
            drained: Some(600_000.0),
            ..Default::default()
        });
        f.sched.run_push(next, q);
        next
    };
    let streams = |f: &Fixture, p: PeerKey| (0..4).filter(|&s| subscribed(f, p, s)).count();
    // A full uplink is no slow link: nothing spills.
    let mut t = now();
    for _ in 0..3 {
        t = check(&mut f, &mut q, t, true);
    }
    assert_eq!(streams(&f, peers[0]), 4);
    // An uplink with room: after two checks behind, half its streams spill,
    // and it keeps them until a satellite took them.
    for _ in 0..2 {
        t = check(&mut f, &mut q, t, false);
    }
    assert_eq!(streams(&f, peers[0]), 4);
    assert!(!ask(&mut f, &mut q, peers[1], 0, t));
    assert!(!ask(&mut f, &mut q, peers[2], 0, t));
    assert!(ask(&mut f, &mut q, peers[1], 3, t));
    // A satellite holds as few of the seed's streams as any neighbour that
    // asked lately: one each here.
    assert!(!ask(&mut f, &mut q, peers[1], 2, t));
    assert!(ask(&mut f, &mut q, peers[2], 2, t));
    t = check(&mut f, &mut q, t, false);
    assert!(!subscribed(&f, peers[0], 3) && !subscribed(&f, peers[0], 2));
    assert!(subscribed(&f, peers[1], 3) && subscribed(&f, peers[2], 2));
    assert_eq!(holders(&f), peers[..3].to_vec());
    // Still behind: it spills again, down to one.
    t = check(&mut f, &mut q, t, false);
    assert!(ask(&mut f, &mut q, peers[3], 1, t));
    check(&mut f, &mut q, t, false);
    assert_eq!(streams(&f, peers[0]), 1);
}

#[test]
fn a_stream_its_holder_cancels_goes_to_a_near_satellite_until_it_asks_again() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let ms = Duration::from_millis;
    // One copy, held by peers[1] (20 ms).
    for s in 0..4 {
        assert!(ask(&mut f, &mut q, peers[1], s, now()));
    }
    let one_copy = (600_000, 1.0);
    let mut t = alloc_check(&mut f, &mut q, now(), &[peers[1]], ms(5), one_copy);
    assert_eq!(holders(&f), vec![peers[1]]);
    // It cancels stream 3 and keeps its copy: no one else takes stream 3
    // from the seed, so a neighbour no farther than the holder may hold it
    // as a satellite, and one farther may not.
    f.sched.handle_stream_cancel(peers[1], 3).unwrap();
    assert!(!ask(&mut f, &mut q, peers[5], 3, t));
    assert!(ask(&mut f, &mut q, peers[0], 3, t));
    t = alloc_check(&mut f, &mut q, t, &[peers[1]], ms(5), one_copy);
    assert!(subscribed(&f, peers[0], 3));
    assert_eq!(holders(&f), vec![peers[0], peers[1]]);
    // The holder asks for it again and takes it back.
    assert!(ask(&mut f, &mut q, peers[1], 3, t));
    alloc_check(&mut f, &mut q, t, &[peers[1]], ms(5), one_copy);
    assert!(subscribed(&f, peers[1], 3));
    assert!(!subscribed(&f, peers[0], 3));
}

#[test]
fn a_nearer_neighbour_takes_a_copy_once_it_subscribed() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let ms = Duration::from_millis;
    let saturated = (300_000, 1.0);
    assert!(ask(&mut f, &mut q, peers[5], 0, now()));
    assert!(!ask(&mut f, &mut q, peers[0], 0, now()));
    assert!(!ask(&mut f, &mut q, peers[4], 0, now()));
    // The nearest that asked is given the copy, and the farthest keeps it
    // until the new holder subscribes ...
    let mut t = alloc_check(&mut f, &mut q, now(), &[peers[5]], ms(5), saturated);
    assert_eq!(holders(&f), vec![peers[0], peers[5]]);
    assert!(subscribed(&f, peers[5], 0));
    assert!(!ask(&mut f, &mut q, peers[4], 0, t));
    assert!(ask(&mut f, &mut q, peers[0], 0, t));
    t = alloc_check(&mut f, &mut q, t, &[peers[5]], ms(5), saturated);
    assert_eq!(holders(&f), vec![peers[0]]);
    assert!(!subscribed(&f, peers[5], 0));
    // ... and a holder left without a subscription gives it back after the
    // grace, for the next to ask.
    f.sched.handle_stream_cancel(peers[0], 0).unwrap();
    let asked = t - push::seed::ALLOC_CHECK_INTERVAL;
    while t <= asked + push::seed::HOLDER_GRACE {
        t = alloc_check(&mut f, &mut q, t, &[], ms(5), saturated);
    }
    assert!(holders(&f).is_empty());
    assert!(ask(&mut f, &mut q, peers[3], 0, t));
}

#[test]
fn an_adaptive_seeds_window_follows_its_pushes() {
    let mut f = adaptive_seed(4, 3);
    let mut q = MockQueue::default();
    let peers: Vec<PeerKey> = (0..3).map(|i| add_ready(&mut f, &mut q, i)).collect();
    for (i, &p) in peers.iter().enumerate() {
        let rtt = Duration::from_millis(10 * (i as i64 + 1));
        push::seed::round_trip(&mut f.sched.channels[&p].push, rtt, now());
    }
    for &p in &peers[..2] {
        ask(&mut f, &mut q, p, 0, now());
    }
    assert_eq!(holders(&f), vec![peers[0]]);
    let window = |f: &Fixture| f.sched.seed_allocation(now()).unwrap().window;
    let ms = Duration::from_millis;
    // A budget of three copies once measured.
    let saturated = (2_000_000, 1.0);

    // No pushes, no evidence.
    let mut t = alloc_check(&mut f, &mut q, now(), &[], ms(0), saturated);
    assert_eq!(window(&f), 1);
    // Pushes that keep up on a saturated uplink: the window leads the
    // copies in use twice at most, and the free copy goes to the one that
    // asked.
    t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(20), saturated);
    assert_eq!(window(&f), 2);
    assert_eq!(holders(&f), peers[..2].to_vec());
    t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(20), saturated);
    assert_eq!(window(&f), 2);
    assert!(ask(&mut f, &mut q, peers[1], 0, t));
    // The second copy in use is a check old before the window grows on it.
    t = alloc_check(&mut f, &mut q, t, &peers[..2], ms(20), saturated);
    assert_eq!(window(&f), 2);
    // Pushes queued before the window changed don't count. Queues that
    // don't empty hold it; queues near going stale at both holders halve
    // it, and the farther goes.
    push_to(&mut f, peers[1], t - ms(1500), t, Some(ms(20)));
    t = alloc_check(&mut f, &mut q, t, &peers[..2], ms(20), saturated);
    assert_eq!(window(&f), 3);
    t = alloc_check(&mut f, &mut q, t, &peers[..2], ms(300), saturated);
    assert_eq!(window(&f), 3);
    t = alloc_check(&mut f, &mut q, t, &peers[..2], ms(600), saturated);
    assert_eq!(window(&f), 1);
    assert_eq!(holders(&f), vec![peers[0]]);
    assert!(!subscribed(&f, peers[1], 0));
    // A stale drop overloads: on an uplink with idle time it neither cuts
    // the window nor lets it grow.
    f.sched
        .note_push(peers[0], pushed_bin(t), t, PushFate::Stale, t + ms(1100));
    t = alloc_check(&mut f, &mut q, t, &[], ms(0), (2_000_000, 0.5));
    assert_eq!(window(&f), 1);
    alloc_check(&mut f, &mut q, t, &[peers[0]], ms(5), (2_000_000, 0.5));
    assert_eq!(window(&f), 2);
}

#[test]
fn an_adaptive_seed_bars_a_holder_that_doesnt_announce_its_pushes() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let ms = Duration::from_millis;
    // Each interval peers[0] announces its push 800 ms after it arrived,
    // peers[1] never.
    let check = |f: &mut Fixture, q: &mut MockQueue, t: Time| -> Time {
        let next = t + push::seed::ALLOC_CHECK_INTERVAL;
        publish(f, t, next);
        push_to(f, peers[0], t, t + ms(5), Some(ms(800)));
        if subscribed(f, peers[1], 0) {
            push_to(f, peers[1], t, t + ms(5), None);
        }
        f.sched.set_upload(push::UploadInfo {
            rate: 2_000_000,
            busy: 1.0,
            ..Default::default()
        });
        f.sched.run_push(next, q);
        next
    };
    let neighbour = |f: &Fixture, p: PeerKey, t: Time| {
        let a = f.sched.seed_allocation(t).unwrap();
        a.neighbours.into_iter().find(|n| n.peer == p).unwrap()
    };
    let mut t = now();
    assert!(ask(&mut f, &mut q, peers[0], 0, t));
    assert!(!ask(&mut f, &mut q, peers[1], 0, t));
    // A check in use later the window doubles, and the free copy goes to
    // the one that asked.
    t = check(&mut f, &mut q, t);
    t = check(&mut f, &mut q, t);
    assert_eq!(holders(&f), peers[..2].to_vec());
    assert!(ask(&mut f, &mut q, peers[1], 0, t));
    // A push is due at the check after next; three due in a row
    // unannounced take its copy and bar it.
    for _ in 0..3 {
        t = check(&mut f, &mut q, t);
        assert_eq!(holders(&f), peers[..2].to_vec());
    }
    t = check(&mut f, &mut q, t);
    assert_eq!(holders(&f), vec![peers[0]]);
    assert!(!subscribed(&f, peers[1], 0));
    assert!(neighbour(&f, peers[1], t).barred);
    assert_eq!(f.sched.seed_allocation(t).unwrap().counters.silenced, 1);
    // While barred it is declined, and it is no asker for a free copy.
    let barred = t;
    while t < barred + push::seed::HOLDER_BAR - ms(1) {
        assert!(!ask(&mut f, &mut q, peers[1], 0, t));
        t = check(&mut f, &mut q, t);
        assert_eq!(holders(&f), vec![peers[0]]);
    }
    // Then it may hold again; barred a second time, for twice as long.
    t = check(&mut f, &mut q, t);
    assert!(!neighbour(&f, peers[1], t).barred);
    assert!(ask(&mut f, &mut q, peers[1], 0, t));
    assert_eq!(holders(&f), peers[..2].to_vec());
    while holders(&f).len() == 2 {
        t = check(&mut f, &mut q, t);
    }
    let barred = t;
    assert!(neighbour(&f, peers[1], barred + push::seed::HOLDER_BAR * 2 - ms(1)).barred);
    assert!(!neighbour(&f, peers[1], barred + push::seed::HOLDER_BAR * 2).barred);
    assert_eq!(f.sched.seed_allocation(t).unwrap().counters.silenced, 2);
}

#[test]
fn an_adaptive_seed_times_cancelled_pushes() {
    // A push its holder cancelled waited in the seed's queue all the same,
    // so its wait drives the window; the holder never got it, so it isn't
    // due to announce it.
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    let ms = Duration::from_millis;
    let cancelled = |f: &mut Fixture, p: PeerKey, t: Time, wait: Duration| {
        f.sched
            .note_push(p, pushed_bin(t + wait), t, PushFate::Cancelled, t + wait);
    };
    // Each interval peers[0] is pushed to and announces it; peers[1]
    // cancels every push.
    let check = |f: &mut Fixture, q: &mut MockQueue, t: Time| -> Time {
        let next = t + push::seed::ALLOC_CHECK_INTERVAL;
        publish(f, t, next);
        push_to(f, peers[0], t, t + ms(5), Some(ms(20)));
        if subscribed(f, peers[1], 0) {
            cancelled(f, peers[1], t, ms(5));
        }
        f.sched.set_upload(push::UploadInfo {
            rate: 2_000_000,
            busy: 1.0,
            ..Default::default()
        });
        f.sched.run_push(next, q);
        next
    };
    let mut t = now();
    assert!(ask(&mut f, &mut q, peers[0], 0, t));
    assert!(!ask(&mut f, &mut q, peers[1], 0, t));
    t = check(&mut f, &mut q, t);
    assert!(ask(&mut f, &mut q, peers[1], 0, t));
    // Past the checks that bar a silent holder, it still holds.
    for _ in 0..push::seed::SILENT_CHECKS + 2 {
        t = check(&mut f, &mut q, t);
        assert_eq!(holders(&f), peers[..2].to_vec());
    }
    assert_eq!(f.sched.seed_allocation(t).unwrap().counters.silenced, 0);

    // Cancelled pushes alone, 600 ms in the queue of a saturated uplink,
    // halve the window.
    let window = |f: &Fixture| f.sched.seed_allocation(now()).unwrap().window;
    let before = window(&f);
    assert!(before >= 2, "{before}");
    let next = t + push::seed::ALLOC_CHECK_INTERVAL;
    publish(&mut f, t, next);
    cancelled(&mut f, peers[1], t, ms(600));
    f.sched.set_upload(push::UploadInfo {
        rate: 2_000_000,
        busy: 1.0,
        ..Default::default()
    });
    f.sched.run_push(next, &mut q);
    assert_eq!(window(&f), before / 2);
}

#[test]
fn an_adaptive_seeds_first_wave_takes_what_its_uplink_pays_for() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    // Publishing for a second on an uplink measured at 2 MB/s: two copies
    // at 70 %, at the store rate since the first chunk.
    publish(&mut f, now(), now() + Duration::SECOND);
    let t = now() + Duration::SECOND;
    f.sched.set_upload(push::UploadInfo {
        rate: 2_000_000,
        busy: 1.0,
        ..Default::default()
    });
    for &p in &peers {
        ask(&mut f, &mut q, p, 0, t);
    }
    assert_eq!(holders(&f), peers[..2].to_vec());
}

#[test]
fn an_adaptive_seeds_first_wave_leaves_room_for_other_swarms() {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    // The uplink of the test above, half of whose 70 % the node's other
    // swarms have committed: one copy.
    publish(&mut f, now(), now() + Duration::SECOND);
    let t = now() + Duration::SECOND;
    f.sched.set_upload(push::UploadInfo {
        rate: 2_000_000,
        busy: 1.0,
        others: 700_000.0,
        ..Default::default()
    });
    for &p in &peers {
        ask(&mut f, &mut q, p, 0, t);
    }
    assert_eq!(holders(&f), peers[..1].to_vec());
}

#[test]
fn without_uplink_shares_an_adaptive_seeds_window_is_its_budget() {
    let mut f = adaptive_seed_with(4, 3, crate::SeedQos::Equal);
    let mut q = MockQueue::default();
    let peers: Vec<PeerKey> = (0..3).map(|i| add_ready(&mut f, &mut q, i)).collect();
    let window = |f: &Fixture| f.sched.seed_allocation(now()).unwrap().window;
    ask(&mut f, &mut q, peers[0], 0, now());
    // Overloaded pushes on a saturated uplink cut nothing: the window is
    // what the uplink pays for (three copies at 2 MB/s once measured).
    let ms = Duration::from_millis;
    let mut t = now();
    for _ in 0..4 {
        t = alloc_check(&mut f, &mut q, t, &[peers[0]], ms(600), (2_000_000, 1.0));
    }
    assert_eq!(window(&f), 3);
}

#[test]
fn under_one_copy_a_holder_takes_what_the_budget_pays_for() {
    let mut f = adaptive_seed(4, 2);
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    for s in 0..4 {
        f.sched
            .handle_stream_request(p, s, Bin(0), Bin(0), now(), &mut q)
            .unwrap();
    }
    assert_eq!(f.sched.push_stats().children, 4);
    // Two subscribers' worth.
    let t = now() + Duration::from_secs(1);
    publish(&mut f, now(), t);
    f.sched.set_upload(push::UploadInfo {
        rate: 300_000,
        busy: 1.0,
        ..Default::default()
    });
    f.sched.run_push(t, &mut q);
    publish(&mut f, t, t + push::seed::ALLOC_MEASURE);
    let t = t + push::seed::ALLOC_MEASURE;
    f.sched.run_push(t, &mut q);
    assert_eq!(f.sched.push_stats().children, 2);
    assert_eq!(holders(&f), vec![p]);
}

#[test]
fn seed_weights_follow_the_streams_pushed() {
    let mut f = fixture(SwarmOptions {
        seed: crate::SeedPolicy {
            allocation: crate::SeedAllocation::Admission,
            qos: crate::SeedQos::ByAllocation,
        },
        ..seed_hybrid(4)
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let r = add_ready(&mut f, &mut q, 1);
    let idle = add_ready(&mut f, &mut q, 2);
    for (peer, stream) in [(p, 0), (p, 1), (p, 2), (p, 3), (r, 0)] {
        f.sched
            .handle_stream_request(peer, stream, Bin(0), Bin(0), now(), &mut q)
            .unwrap();
    }
    let mut w = BTreeMap::new();
    w.insert(r, 600);
    assert!(f.sched.seed_weights(&mut w));
    assert_eq!((w[&p], w[&r], w[&idle]), (1000, 600, 50));

    // Equal shares: the weights are reciprocity's alone.
    let f = fixture(seed_hybrid(4));
    assert!(!f.sched.seed_weights(&mut BTreeMap::new()));
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

/// The decisions the tap recorded since the last call, in order.
fn tapped(f: &mut Fixture) -> Vec<crate::TapEvent> {
    let mut v = Vec::new();
    f.sched.s.tap.drain(&mut |e| v.push(e));
    v
}

#[test]
fn the_tap_gives_misses_and_prunes_their_deadlines_and_rules() {
    let mut f = fixture(hybrid(1));
    f.sched.s.tap.set(true);
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    f.sched
        .handle_stream_open(parent, 0, Bin(0), now(), &mut q)
        .unwrap();
    f.sched.handle_have(other, Bin::new(2, 0), now());
    let deadline = f.sched.next_push_deadline().unwrap();
    tapped(&mut f);
    f.sched.expire_push(deadline, &mut q);
    let misses: Vec<_> = tapped(&mut f)
        .into_iter()
        .filter_map(|e| match e.kind {
            TapKind::PushMiss {
                deadline,
                first_seen,
                counted,
                lag,
                ..
            } => Some((e.peer, deadline, first_seen, counted, lag.deadline)),
            _ => None,
        })
        .collect();
    assert_eq!(
        misses,
        vec![(Some(parent), deadline, now(), true, push::DEADLINE_INITIAL); 4]
    );

    // A parent silent for a second is pruned by the silence rule.
    let later = now() + push::PRUNE_SILENCE;
    f.sched.handle_have(other, Bin::new(2, 1), later);
    let deadline = f.sched.next_push_deadline().unwrap();
    f.sched.expire_push(deadline, &mut q);
    let pruned: Vec<_> = tapped(&mut f)
        .into_iter()
        .filter_map(|e| match e.kind {
            TapKind::Pruned { why, .. } => Some((e.peer, why)),
            _ => None,
        })
        .collect();
    let [(peer, why)] = pruned[..] else {
        panic!("{pruned:?}");
    };
    assert_eq!(peer, Some(parent));
    assert!(why.silent && !why.lossy, "{why:?}");
    // Chunk 3 went to pull unrequested, so the next announcement expects
    // it again.
    assert_eq!((why.silent_misses, why.expected, why.pushes), (9, 9, 0));
    assert_eq!(why.since_hit, Some(deadline - now()));
    assert_eq!(why.age, Some(deadline - now()));
    assert_eq!(why.lag.deadline, push::DEADLINE_INITIAL);

    // The next check passes over the backed-off parent and grafts `other`.
    f.sched
        .run_push(deadline + push::PARENT_CHECK_INTERVAL, &mut q);
    let choices: Vec<_> = tapped(&mut f)
        .into_iter()
        .filter_map(|e| match e.kind {
            TapKind::ParentChoice { choice, .. } => Some((e.peer, choice)),
            _ => None,
        })
        .collect();
    let [(peer, c)] = choices[..] else {
        panic!("{choices:?}");
    };
    assert_eq!(peer, Some(other));
    assert_eq!(c.action, crate::ParentAction::Graft);
    assert_eq!(c.best.map(|b| b.0), Some(other));
    assert_eq!((c.runner_up, c.backed_off, c.stale), (None, 1, 0));
}

#[test]
fn the_tap_gives_admissions_their_rule_and_numbers() {
    let mut f = fixture(hybrid(8));
    f.sched.s.tap.set(true);
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    let r = add_ready(&mut f, &mut q, 1);
    let admissions = |f: &mut Fixture| -> Vec<(bool, crate::Admission)> {
        tapped(f)
            .into_iter()
            .filter_map(|e| match e.kind {
                TapKind::Admitted { why, .. } => Some((true, why)),
                TapKind::Rejected { why, .. } => Some((false, why)),
                _ => None,
            })
            .collect()
    };
    f.sched
        .handle_stream_request(p, 2, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    f.sched
        .handle_stream_request(r, 5, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    let got = admissions(&mut f);
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got[0].0 && got[0].1.rule == crate::AdmitRule::Unmeasured && !got[0].1.extra);
    assert!(!got[1].0 && got[1].1.rule == crate::AdmitRule::Unmeasured && got[1].1.extra);

    // Measured: the rate plus a subscriber against the capacity at the
    // extra target.
    f.sched.set_upload(push::UploadInfo {
        rate: 100_000,
        busy: 0.95,
        ..Default::default()
    });
    f.sched
        .handle_stream_request(r, 5, Bin(0), Bin(0), now(), &mut q)
        .unwrap();
    let got = admissions(&mut f);
    let [(false, a)] = got[..] else {
        panic!("{got:?}");
    };
    assert_eq!(a.rule, crate::AdmitRule::Measured);
    assert_eq!(
        (a.upload_rate, a.capacity, a.target_permille, a.extra),
        (100_000, Some(105_263), 500, true)
    );
    assert_eq!(a.stream_children, 0);
}

#[test]
fn the_tap_gives_a_shed_its_numbers() {
    let mut f = fixture(seed_hybrid(1));
    f.sched.s.tap.set(true);
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
    let mut t = now();
    for i in 0..11 {
        t = now() + Duration::from_millis(100 + 200 * i);
        f.sched.s.push_consumed(Bin(0), 64, t);
    }
    f.sched.set_upload(push::UploadInfo {
        rate: 1_900_000,
        busy: 0.95,
        ..Default::default()
    });
    tapped(&mut f);
    f.sched.run_push(t, &mut q);
    let sheds: Vec<_> = tapped(&mut f)
        .into_iter()
        .filter_map(|e| match e.kind {
            TapKind::Shed { why, .. } => Some(why),
            _ => None,
        })
        .collect();
    let [
        crate::Shed::Saturated {
            busy,
            capacity,
            excess,
            count,
            others,
            ..
        },
    ] = sheds[..]
    else {
        panic!("{sheds:?}");
    };
    assert_eq!((busy, capacity, count, others), (950, 2_000_000, 1, 0));
    assert!(excess > 0, "{excess}");
}

#[test]
fn a_missed_push_is_cancelled_at_its_parent() {
    // What a child gives up on, at a deadline or with the parent, is
    // cancelled at the parent, which drops the pushes it hasn't sent.
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    assert_eq!(
        f.sched.handle_stream_open(parent, 0, Bin(0), now(), &mut q),
        Ok(StreamOpenOutcome::Accepted)
    );
    let cancels = |f: &mut Fixture, p: PeerKey, t: Time| -> Vec<Bin> {
        next_messages(f, p, t)
            .iter()
            .filter_map(|m| match m {
                Message::Cancel(c) => Some(c.bin),
                _ => None,
            })
            .collect()
    };

    // Chunks 0..=3 miss their deadline: the parent gets a CANCEL for them,
    // and the peer pull asks for them none.
    f.sched.handle_have(other, Bin::new(2, 0), now());
    let deadline = f.sched.next_push_deadline().unwrap();
    f.sched.expire_push(deadline, &mut q);
    assert_eq!(cancels(&mut f, parent, deadline), vec![Bin::new(2, 0)]);
    assert!(cancels(&mut f, other, deadline).is_empty());

    // Chunks 4..=7 are still expected when we unsubscribe: they go to pull
    // and are cancelled at the parent with the stream.
    f.sched.handle_have(other, Bin::new(2, 1), deadline);
    assert!(
        f.sched
            .cancel_stream_sub(parent, 0, &f.swarm, deadline, &mut q)
    );
    assert!(f.sched.s.request_bins.empty_at(Bin::new(2, 1)));
    let msgs = next_messages(&mut f, parent, deadline);
    assert!(
        msgs.iter()
            .any(|m| matches!(m, Message::StreamCancel(c) if c.stream == 0))
    );
    assert!(
        msgs.iter()
            .any(|m| matches!(m, Message::Cancel(c) if c.bin == Bin::new(2, 1)))
    );
}

#[test]
fn a_missed_push_is_pulled_from_the_others_first() {
    // The parent's answer would queue behind its pushes, so a chunk it
    // missed is left to the other channels for REQUEST_BACKOFF. Another
    // holder, untimed and so counted as quick, is what releases it at the
    // deadline: with none, the push waits for the parent
    // (`a_late_push_waits_while_pull_cant_win`).
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    assert_eq!(
        f.sched.handle_stream_open(parent, 0, Bin(0), now(), &mut q),
        Ok(StreamOpenOutcome::Accepted)
    );
    f.sched.handle_have(parent, Bin::new(2, 0), now());
    f.sched.handle_have(other, Bin::new(2, 0), now());
    let deadline = f.sched.next_push_deadline().unwrap();
    f.sched.expire_push(deadline, &mut q);
    let requests = |f: &mut Fixture, t: Time| {
        next_messages(f, parent, t)
            .iter()
            .filter(|m| matches!(m, Message::Request(_)))
            .count()
    };
    assert_eq!(requests(&mut f, deadline), 0);
    assert_eq!(requests(&mut f, deadline + REQUEST_BACKOFF), 4);
}

/// A late push waits for the parent while no other holder is expected to
/// answer before [`push::DEADLINE_MAX`] past the chunk's first
/// announcement, the parent's last chance, looking again every
/// [`push::RELEASE_RECHECK`]; at that cap pull takes it.
#[test]
fn a_late_push_waits_while_pull_cant_win() {
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    assert_eq!(
        f.sched.handle_stream_open(parent, 0, Bin(0), now(), &mut q),
        Ok(StreamOpenOutcome::Accepted)
    );
    let t = now();
    f.sched.handle_have(parent, Bin::new(2, 0), t);
    // `other` holds the chunks too, but answers in two seconds.
    f.sched.handle_have(other, Bin::new(2, 0), t);
    f.sched
        .channels
        .get_mut(&other)
        .unwrap()
        .answer_time
        .sample(Duration::from_secs(2));
    let cap = t + push::DEADLINE_MAX;
    let deadline = f.sched.next_push_deadline().unwrap();
    assert!(deadline + push::RELEASE_RECHECK < cap);
    f.sched.expire_push(deadline, &mut q);
    assert_eq!(f.sched.push_stats().misses, 0);
    assert!(f.sched.s.request_bins.filled_at(Bin::new(2, 0)));
    assert_eq!(
        f.sched.next_push_deadline(),
        Some(deadline + push::RELEASE_RECHECK)
    );
    f.sched.expire_push(cap, &mut q);
    assert_eq!(f.sched.push_stats().misses, 4);
    assert!(f.sched.s.request_bins.empty_at(Bin::new(2, 0)));
}

/// A channel leaves a chunk to one expected to deliver it sooner: a peer
/// that answers quickly and, by its announcement lag, will announce the
/// chunk soon. Once that announcement is overdue, the channel asks.
#[test]
fn requests_go_to_whoever_is_expected_to_deliver_first() {
    let ms = Duration::from_millis;
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let slow = add_ready(&mut f, &mut q, 0);
    let fast = add_ready(&mut f, &mut q, 1);
    f.sched.s.first_chunk_set = true;
    let t = now();
    // Both announce chunk 0, `fast` 10 ms later: its announcement lag.
    f.sched.handle_have(slow, Bin(0), t);
    f.sched.handle_have(fast, Bin(0), t + ms(10));
    // `fast` answers in 20 ms; `slow` is untimed, so it expects its
    // request timeout.
    f.sched
        .channels
        .get_mut(&fast)
        .unwrap()
        .answer_time
        .sample(ms(20));
    // `fast`'s request round leaves window room.
    let t1 = t + ms(20);
    assert_eq!(requests_at(&mut f, fast, t1), vec![Bin(0)]);

    // Chunks only `slow` announced, of chunk 0's signature unit, so first
    // seen at `t`: `fast` is expected to announce them 10 ms after that and
    // deliver them 20 ms later.
    f.sched.handle_have(slow, Bin::new(2, 1), t1);
    assert_eq!(f.sched.s.bin_time(Bin(8)), t);
    assert!(requests_at(&mut f, slow, t1).is_empty());
    // Until `fast`'s horizon (its lag's peak or mean plus two deviations,
    // 20 ms) and the slack are past, while `fast` has room; then `slow`
    // asks.
    let horizon = t + ms(20) + PREDICT_SLACK;
    assert!(requests_at(&mut f, fast, horizon).is_empty());
    assert!(requests_at(&mut f, slow, horizon).is_empty());
    let late = horizon + ms(1);
    assert!(requests_at(&mut f, fast, late).is_empty());
    assert_eq!(requests_at(&mut f, slow, late).len(), 4);
}

/// Answers queue below pushes, a hybrid swarm's apart for the answer gate
/// ([`crate::answer_gate`]); a CANCEL takes them from the same queue.
#[test]
fn answers_queue_by_push_mode() {
    for (options, pri) in [
        (pull(), PeerPriority::Low),
        (hybrid(1), PeerPriority::Scavenger),
    ] {
        let mut f = fixture(options);
        let mut q = MockQueue::default();
        let p = add_ready(&mut f, &mut q, 0);
        assert!(f.sched.handle_request(p, Bin(2), now(), &mut q));
        assert_eq!(q.pushed, [(p, Bin(2), pri)]);
        f.sched.handle_cancel(p, Bin(2), Bin(0), &mut q);
        assert_eq!(q.removed, [(p, Bin(2), pri)]);
    }
}

#[test]
fn a_cancelled_push_is_not_sent() {
    // A CANCEL drops its chunk's push at write time, whenever the push was
    // queued; a request after it is still answered. Only bins in our
    // window count.
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let child = ready_channel(&mut f, &mut q);
    let ms = Duration::from_millis;
    let t = now();
    f.sched.handle_cancel(child, Bin(2), Bin(0), &mut q);
    assert!(
        q.removed
            .contains(&(child, Bin(2), PeerPriority::Scavenger))
    );
    assert!(f.sched.push_unwanted(child, Bin(2), t - ms(100), t));
    assert!(
        f.sched
            .push_unwanted(child, Bin(2), t + ms(100), t + ms(100))
    );
    assert!(!f.sched.push_unwanted(child, Bin(4), t, t));
    assert_eq!(f.sched.push_stats().suppressed, 2);
    // A stale push is stale first.
    assert!(
        f.sched
            .push_unwanted(child, Bin(2), t, t + push::PUSH_STALE + ms(1))
    );
    let st = f.sched.push_stats();
    assert_eq!((st.suppressed, st.stale_drops), (2, 1));
    assert!(f.sched.handle_request(child, Bin(2), t, &mut q));
    assert_eq!(
        q.pushed.last(),
        Some(&(child, Bin(2), PeerPriority::Scavenger))
    );

    // Behind the window, past a live window beyond our newest chunk, or
    // wider than a window: ignored.
    let past = Bin(f.sched.s.live_window.0 + 2);
    for (b, window_start) in [(Bin(6), Bin(8)), (past, Bin(0)), (Bin::ALL, Bin(0))] {
        f.sched.handle_cancel(child, b, window_start, &mut q);
    }
    assert!(!f.sched.push_unwanted(child, Bin(6), t, t));
    assert!(!f.sched.push_unwanted(child, past, t, t));
    // GC forgets the cancels behind the window.
    f.sched.gc_push(Bin(4));
    assert!(!f.sched.push_unwanted(child, Bin(2), t, t));
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
        f.sched.add_channel(p, &[], now(), &mut q);
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
        f.sched.add_channel(fresh, &[], now(), &mut q);
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

#[test]
fn bins_whose_integrity_overflows_an_empty_frame_are_split() {
    // SIGN_ALL sends a signature per chunk. The frame holds both chunks of
    // a two-chunk bin but only one signature, so the bin's left chunk goes
    // out and its right chunk goes back to the queue.
    let (mut writer, mut swarm) = crate::Writer::new(
        crate::WriterOptions {
            swarm_options: SwarmOptions {
                integrity: crate::integrity::VerifierOptions {
                    protection_method: ProtectionMethod::SIGN_ALL,
                    ..crate::integrity::VerifierOptions::default_options()
                },
                ..Default::default()
            },
            key: strims_crypto::key_from_seed(&[5; 32]),
        },
        now(),
    )
    .unwrap();
    let chunk_size = swarm.options().chunk_size;
    writer
        .write(&mut swarm, now(), &vec![7; 2 * chunk_size])
        .unwrap();
    let mut f = fixture_with(swarm);
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);

    let mut scratch = vec![0u8; 2 * chunk_size];
    let mut w = crate::codec::Writer::new(&mut scratch);
    let one_chunk = f
        .swarm
        .write_integrity(Bin(0), &Map::new(), &mut w)
        .unwrap()
        + f.swarm.write_data(Bin(0), now(), &mut w).unwrap();
    let header = crate::codec::ChannelHeader {
        channel: 1,
        length: 0,
    }
    .byte_len();
    let mut buf = vec![0u8; header + one_chunk + chunk_size];
    let mut cw = MeteredWriter::new(&mut buf, 1);
    let b = Bin::new(1, 0);
    assert_eq!(
        f.sched.write_data(
            p,
            b,
            now(),
            PeerPriority::Low,
            now(),
            &f.swarm,
            &mut cw,
            true,
            &mut q
        ),
        Ok(())
    );
    assert_eq!(q.pushed_front, vec![(p, b.right(), PeerPriority::Low)]);

    let (n, _) = cw.finish(&mut ChannelMetrics::default());
    let cfg = f.swarm.reader_config();
    let msgs: Vec<_> = crate::codec::FrameReader::new(&buf[..n])
        .flat_map(|c| Reader::new(cfg, c.unwrap().1).map(Result::unwrap))
        .collect();
    assert!(
        matches!(
            msgs[..],
            [
                Message::SignedIntegrity(crate::codec::SignedIntegrity { bin: Bin(0), .. }),
                Message::Data(crate::codec::Data { bin: Bin(0), .. }),
            ]
        ),
        "{msgs:?}"
    );
}

// --- reports --------------------------------------------------------------

/// The ANNOUNCE entries `peer`'s channel writes next: stream, hits, late,
/// misses and parent id.
fn reports(f: &mut Fixture, peer: PeerKey, t: Time) -> Vec<(u16, u64, u64, u64, Vec<u8>)> {
    next_messages(f, peer, t)
        .iter()
        .filter_map(|m| match m {
            Message::Announce(a) if !a.is_empty() => Some(
                a.iter()
                    .map(|e| (e.stream, e.hits, e.late, e.misses, e.parent.to_vec()))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn a_viewer_reports_its_parents_to_every_neighbour_that_asked() {
    let mut f = fixture(hybrid(2));
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    let asker = add_ready(&mut f, &mut q, 2);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    assert_eq!(
        f.sched.handle_stream_open(parent, 0, Bin(0), now(), &mut q),
        Ok(StreamOpenOutcome::Accepted)
    );
    // `other` announces chunks `first..first + 4`: the parent pushes the
    // first of stream 0, and the second misses its deadline.
    let batch = |f: &mut Fixture, q: &mut MockQueue, first: u64, t: Time| {
        f.sched.handle_have(other, Bin::new(2, first / 4), t);
        let b = Bin(2 * first);
        f.sched.consume(b, t, &f.swarm, q);
        f.sched.handle_data(parent, b, t, true, true, t, Bin(0), q);
        let deadline = f.sched.next_push_deadline().unwrap();
        f.sched.expire_push(deadline, q);
        deadline
    };
    let ms = Duration::from_millis;
    batch(&mut f, &mut q, 0, now() + ms(10));

    // Nobody asked: the tick sends nothing, and the tallies start over.
    let tick = now() + push::REPORT_INTERVAL;
    f.sched.run_push(tick, &mut q);
    for p in [parent, other, asker] {
        assert!(reports(&mut f, p, tick).is_empty());
    }

    // Once asked, a report per interval to each that asked, leaving out
    // the entries about its receiver: here, all of the parent's.
    for p in [asker, parent] {
        assert!(f.sched.handle_announce(p, Announce::ASK, tick));
    }
    let deadline = batch(&mut f, &mut q, 4, tick + ms(10));
    let tick = tick + push::REPORT_INTERVAL;
    assert!(deadline < tick);
    f.sched.run_push(tick, &mut q);
    assert_eq!(reports(&mut f, asker, tick), vec![(0, 1, 0, 1, vec![1])]);
    assert!(reports(&mut f, parent, tick).is_empty());
    assert!(reports(&mut f, other, tick).is_empty());
    f.sched.run_push(tick + push::REPORT_INTERVAL, &mut q);
    assert!(reports(&mut f, asker, tick + push::REPORT_INTERVAL).is_empty());

    // A parent silent for a second is pruned, and still reported.
    let t = tick + push::REPORT_INTERVAL + ms(100);
    f.sched.handle_have(other, Bin::new(3, 1), t);
    let deadline = f.sched.next_push_deadline().unwrap();
    f.sched.expire_push(deadline, &mut q);
    assert_eq!(f.sched.push_stats().pruned, 1);
    let tick = deadline.max(tick + push::REPORT_INTERVAL * 2);
    f.sched.run_push(tick, &mut q);
    assert_eq!(reports(&mut f, asker, tick), vec![(0, 0, 0, 4, vec![1])]);
}

#[test]
fn an_ask_lapses_after_the_lease() {
    let mut f = fixture(hybrid(1));
    let mut q = MockQueue::default();
    let parent = ready_channel(&mut f, &mut q);
    let other = add_ready(&mut f, &mut q, 1);
    let asker = add_ready(&mut f, &mut q, 2);
    f.sched.s.first_chunk_set = true;
    assert!(f.sched.do_stream_sub(parent, 0, Bin(0)));
    f.sched
        .handle_stream_open(parent, 0, Bin(0), now(), &mut q)
        .unwrap();
    assert!(f.sched.handle_announce(asker, Announce::ASK, now()));
    // A push a second, reported while the ask lasts.
    for i in 0..12 {
        let t = now() + push::REPORT_INTERVAL * (i + 1);
        let b = Bin(2 * i as u64);
        let at = t - Duration::from_millis(500);
        f.sched.handle_have(other, b, at);
        f.sched.consume(b, at, &f.swarm, &mut q);
        f.sched
            .handle_data(parent, b, at, true, true, at, Bin(0), &mut q);
        f.sched.run_push(t, &mut q);
        let reported = !reports(&mut f, asker, t).is_empty();
        assert_eq!(reported, t - now() <= push::REPORT_LEASE, "{i}");
    }
}

/// An adaptive seed's neighbours, with round trips of 10, 20, ... 60 ms
/// and ids 1..=6; the first holds stream 0.
fn report_seed() -> (Fixture, MockQueue, Vec<PeerKey>) {
    let (mut f, mut q, peers) = adaptive_seed_with_rtts();
    assert!(ask(&mut f, &mut q, peers[0], 0, now()));
    (f, q, peers)
}

/// One check interval of an adaptive seed from `t`: unit `u` published at
/// `t`, the last chunk of each of its streams announced by each of `lags`
/// that long after; a push to each of `fed` written `wait` after it was
/// queued; and each of `reports` naming a parent on stream 0 with its
/// hits and misses.
#[allow(clippy::too_many_arguments)]
fn report_check(
    f: &mut Fixture,
    q: &mut MockQueue,
    t: Time,
    u: u64,
    lags: &[(PeerKey, Duration)],
    fed: &[PeerKey],
    wait: Duration,
    reports: &[(PeerKey, &[u8], u64, u64)],
) -> Time {
    let next = t + push::seed::ALLOC_CHECK_INTERVAL;
    publish(f, t, next);
    let unit = 1u64 << f.sched.s.signature_layer;
    let streams = u64::from(f.sched.s.stream_count);
    let last = |s: u64| Bin(2 * (u * unit + unit - streams + s));
    for s in 0..streams {
        f.sched.s.set_bin_time(last(s), t);
        for &(p, lag) in lags {
            f.sched.handle_have(p, last(s), t + lag);
        }
    }
    for &p in fed {
        f.sched
            .note_push(p, last(0), t, PushFate::Written, t + wait);
    }
    for &(r, parent, hits, misses) in reports {
        let e = [AnnounceEntry {
            stream: 0,
            hits,
            late: 0,
            misses,
            delay: Duration::from_millis(30),
            parent,
        }];
        f.sched.handle_announce(
            r,
            Announce::new(&e).unwrap(),
            t + Duration::from_millis(500),
        );
    }
    f.sched.set_upload(push::UploadInfo {
        rate: 2_000_000,
        busy: 1.0,
        ..Default::default()
    });
    f.sched.run_push(next, q);
    next
}

fn neighbour(f: &Fixture, p: PeerKey, t: Time) -> crate::SeedNeighbour {
    let a = f.sched.seed_allocation(t).unwrap();
    a.neighbours.into_iter().find(|n| n.peer == p).unwrap()
}

#[test]
fn an_adaptive_seed_bars_a_holder_its_reporters_say_serves_nothing() {
    let (mut f, mut q, peers) = report_seed();
    let ms = Duration::from_millis;
    let lags = [(peers[0], ms(20)), (peers[2], ms(100)), (peers[3], ms(100))];
    let unserved = [(peers[2], &[1u8][..], 0, 10), (peers[3], &[1u8][..], 0, 10)];
    let mut t = now();
    // It asks every neighbour for reports.
    f.sched.run_push(t, &mut q);
    for &p in &peers {
        let asks = next_messages(&mut f, p, t);
        assert!(asks.contains(&Message::Announce(Announce::ASK)), "{asks:?}");
    }
    // Judged once it held its copy for the probation, and its pushes kept
    // up for three checks.
    for u in 1..3 {
        t = report_check(&mut f, &mut q, t, u, &lags, &peers[..1], ms(5), &unserved);
        assert_eq!(holders(&f), vec![peers[0]]);
    }
    let n = neighbour(&f, peers[0], t);
    assert_eq!((n.reporters, n.served), (2, Some(false)));
    t = report_check(&mut f, &mut q, t, 3, &lags, &peers[..1], ms(5), &unserved);
    assert!(holders(&f).is_empty());
    assert!(!subscribed(&f, peers[0], 0));
    assert!(neighbour(&f, peers[0], t).barred);
    let c = f.sched.seed_allocation(t).unwrap().counters;
    assert_eq!(c.unserved, 1);
    assert_eq!(c.reports.tallied, 6);

    // Reporters that saw it serve prove it.
    let (mut f, mut q, peers) = report_seed();
    let served = [(peers[2], &[1u8][..], 10, 0), (peers[3], &[1u8][..], 9, 1)];
    let mut t = now();
    for u in 1..6 {
        t = report_check(&mut f, &mut q, t, u, &lags, &peers[..1], ms(5), &served);
    }
    assert_eq!(holders(&f), vec![peers[0]]);
    let n = neighbour(&f, peers[0], t);
    assert_eq!((n.reporters, n.served), (2, Some(true)));
    assert_eq!(n.report_delay, Some(ms(30)));
}

#[test]
fn one_reporter_is_no_quorum() {
    let (mut f, mut q, peers) = report_seed();
    let ms = Duration::from_millis;
    let lags = [(peers[0], ms(20)), (peers[2], ms(100))];
    let unserved = [(peers[2], &[1u8][..], 0, 10)];
    let mut t = now();
    for u in 1..8 {
        t = report_check(&mut f, &mut q, t, u, &lags, &peers[..1], ms(5), &unserved);
    }
    assert_eq!(holders(&f), vec![peers[0]]);
    let n = neighbour(&f, peers[0], t);
    assert_eq!((n.reporters, n.served), (1, None));
}

#[test]
fn a_holder_the_seed_starves_is_not_blamed() {
    let (mut f, mut q, peers) = report_seed();
    let ms = Duration::from_millis;
    let lags = [(peers[0], ms(20)), (peers[2], ms(100)), (peers[3], ms(100))];
    let unserved = [(peers[2], &[1u8][..], 0, 10), (peers[3], &[1u8][..], 0, 10)];
    let mut t = now();
    // Every push to it waits past the overload limit.
    for u in 1..8 {
        t = report_check(&mut f, &mut q, t, u, &lags, &peers[..1], ms(600), &unserved);
    }
    assert_eq!(holders(&f), vec![peers[0]]);
    assert!(!neighbour(&f, peers[0], t).kept_up);
    // Once they keep up, the reports from before don't count: three more
    // checks.
    for u in 8..10 {
        t = report_check(&mut f, &mut q, t, u, &lags, &peers[..1], ms(5), &unserved);
        assert_eq!(holders(&f), vec![peers[0]]);
    }
    report_check(&mut f, &mut q, t, 10, &lags, &peers[..1], ms(5), &unserved);
    assert!(holders(&f).is_empty());
}

#[test]
fn reports_the_seeds_observations_rule_out_count_for_nothing() {
    const IDS: [[u8; 1]; 6] = [[1], [2], [3], [4], [5], [6]];
    let (mut f, mut q, peers) = report_seed();
    let ms = Duration::from_millis;
    let (holder, near, far, silent) = (peers[0], peers[2], peers[3], peers[5]);
    let lags = [(holder, ms(200)), (near, ms(50)), (far, ms(210))];
    let t = report_check(&mut f, &mut q, now(), 1, &lags, &[holder], ms(5), &[]);
    let entry = |parent: &'static [u8], hits: u64, misses: u64| AnnounceEntry {
        stream: 0,
        hits,
        late: 0,
        misses,
        delay: Duration::ZERO,
        parent,
    };
    let send = |f: &mut Fixture, from: PeerKey, e: AnnounceEntry<'_>| {
        f.sched
            .handle_announce(from, Announce::new(&[e]).unwrap(), t)
    };
    let ids: Vec<&'static [u8]> = IDS.iter().map(|x| &x[..]).collect();
    // The holder takes stream 0 from the seed: its own report on it, and
    // a report naming a parent that doesn't take it from the seed, count
    // for nothing.
    assert!(send(&mut f, holder, entry(ids[2], 5, 0)));
    assert!(send(&mut f, far, entry(ids[4], 5, 0)));
    // Nor does one from a neighbour that announces nothing.
    assert!(send(&mut f, silent, entry(ids[0], 5, 0)));
    // `near` announces stream 0 sooner than the holder could relay it.
    assert!(send(&mut f, near, entry(ids[0], 5, 0)));
    // `far` reports misses its own HAVEs don't show.
    assert!(send(&mut f, far, entry(ids[0], 0, 5)));
    // ... and its hits count.
    assert!(send(&mut f, far, entry(ids[0], 5, 0)));
    // A parent the seed doesn't know.
    assert!(send(&mut f, far, entry(&[99], 5, 0)));
    // A report about its sender is invalid, and so is one past the
    // stream count.
    assert!(!send(&mut f, far, entry(ids[3], 5, 0)));
    assert!(!send(
        &mut f,
        far,
        AnnounceEntry {
            stream: 4,
            ..entry(ids[0], 5, 0)
        }
    ));
    let c = f.sched.seed_allocation(t).unwrap().counters.reports;
    assert_eq!(
        c,
        crate::ReportCounters {
            tallied: 1,
            unknown: 1,
            invalid: 2,
            reporter_fed: 1,
            subject_unfed: 1,
            reporter_silent: 1,
            too_early: 1,
            misses_unseen: 1,
        }
    );
    assert_eq!(neighbour(&f, far, t).reports.tallied, 1);
    // Past the per-check entry limit, entries are invalid.
    let n = push::seed::report_entry_limit(f.sched.s.stream_count) as usize;
    let many = vec![entry(ids[0], 5, 0); n];
    assert!(
        !f.sched
            .handle_announce(near, Announce::new(&many).unwrap(), t)
    );
    let c = f.sched.seed_allocation(t).unwrap().counters.reports;
    assert_eq!(c.invalid, 3);
}

/// A joiner whose window starts at chunk 512 (1 KiB chunks, a 1024 chunk
/// window): it holds chunks 512..520, its peer announces chunks 0..520,
/// and a backlog of chunks 384..512 is open.
fn rewound(options: SwarmOptions) -> (Fixture, MockQueue, PeerKey) {
    let mut f = fixture(SwarmOptions {
        live_window: 1 << 10,
        integrity: crate::integrity::VerifierOptions {
            protection_method: ProtectionMethod::NONE,
            ..Default::default()
        },
        ..options
    });
    let mut q = MockQueue::default();
    let p = ready_channel(&mut f, &mut q);
    for o in 512..520u64 {
        f.swarm.publish(Bin(2 * o), &[o as u8; 1024]).unwrap();
    }
    f.swarm.set_offset(Bin(2 * 519));
    f.sched.s.first_chunk_set = true;
    f.sched.s.request_bins.fill_before(Bin(2 * 519));
    drain(&mut f, &mut q);
    assert_eq!(f.swarm.store().tail(), Bin(2 * 512));
    assert!(f.sched.handle_have(p, Bin::new(9, 0), now()));
    assert!(f.sched.handle_have(p, Bin::new(3, 64), now()));
    assert!(f.sched.backlog_servable(Bin(2 * 384), Bin(2 * 512), now()));
    let had = f.sched.s.have_bins.clone();
    f.swarm.open_backlog(Bin(2 * 384), &had).unwrap();
    f.sched.open_backlog(Bin(2 * 384), Bin(2 * 512), &mut q);
    (f, q, p)
}

/// Hands the swarm's published chunks to the scheduler.
fn drain(f: &mut Fixture, q: &mut MockQueue) {
    while let Some(e) = f.swarm.poll_event() {
        if let crate::SwarmEvent::Published(b) = e {
            f.sched.consume(b, now(), &f.swarm, q);
        }
    }
}

/// Live chunks are requested first; the room they leave goes to the
/// backlog's oldest group, and nothing below it.
#[test]
fn a_backlog_takes_the_room_live_requests_leave() {
    let (mut f, _, p) = rewound(pull());
    // the peer announces 8 more live chunks; the window allows 32 requests
    assert!(f.sched.handle_have(p, Bin::new(3, 65), now()));
    let requests = requests_at(&mut f, p, now());
    assert_eq!(requests.len(), 32, "{requests:?}");
    let (live, backlog) = requests.split_at(8);
    assert!(live.iter().all(|b| (520..528).contains(&b.base_offset())));
    assert!(
        backlog
            .iter()
            .all(|b| (384..384 + REQUEST_GROUP).contains(&b.base_offset())),
        "{backlog:?}"
    );

    // no backlog: nothing before the live chunks is asked for
    let (mut f, _, p) = rewound(pull());
    let s = &mut f.sched;
    s.close_backlog(&mut f.swarm);
    assert!(s.handle_have(p, Bin::new(3, 65), now()));
    s.s.request_bins.reset(Bin(2 * 400));
    let requests = requests_at(&mut f, p, now());
    assert!(
        requests.iter().all(|b| b.base_offset() >= 520),
        "{requests:?}"
    );
}

/// A backlog's chunks go to no HAVE, push or new channel, and a live
/// chunk's HAVE leaves them out; the store becomes whole and the backlog
/// ends.
#[test]
fn backlog_chunks_are_never_announced_or_pushed() {
    let (mut f, mut q, p) = rewound(pull());
    let sub = f.peers[1];
    f.sched.add_channel(sub, &[2], now(), &mut q);
    f.sched.handle_handshake(sub, 1 << 10);
    f.sched.channels.get_mut(&sub).unwrap().have_bins = Map::new();
    f.sched.s.streams[0].add_subscriber(sub, Bin(0), false, now());
    q.pushed.clear();
    for o in 384..512u64 {
        f.swarm.publish(Bin(2 * o), &[o as u8; 1024]).unwrap();
    }
    drain(&mut f, &mut q);
    assert!(q.pushed.is_empty(), "{:?}", q.pushed);
    let c = &f.sched.channels[&sub];
    assert!(c.have_bins.is_empty(), "{:?}", c.have_bins.debug_string());
    assert_eq!(f.swarm.store().tail(), Bin(2 * 384));
    assert!(!f.sched.s.backlog.unwrap().pending);

    // a live chunk whose cover spans the backlog
    for o in 520..1024u64 {
        f.swarm.publish(Bin(2 * o), &[o as u8; 1024]).unwrap();
    }
    drain(&mut f, &mut q);
    let c = &f.sched.channels[&sub];
    assert!(c.have_bins.filled_at(Bin::new(9, 1)));
    assert!(c.have_bins.empty_at(Bin::new(7, 3)), "chunks 384..512");
    assert_eq!(q.pushed.len(), 1024 - 520);
    // a new channel and a restart announce no backlog chunk either
    let late = f.peers[2];
    f.sched.add_channel(late, &[3], now(), &mut q);
    assert!(f.sched.channels[&late].have_bins.empty_at(Bin::new(7, 3)));
    assert!(f.sched.channels[&late].have_bins.filled_at(Bin(2 * 520)));
    f.sched.channels.get_mut(&sub).unwrap().have_bins = Map::new();
    f.sched.handle_restart(sub);
    assert!(f.sched.channels[&sub].have_bins.empty_at(Bin::new(7, 3)));
    let _ = p;
}

/// A backlog whose holder goes, or that left every holder's window, ends
/// at the next tick: the store's backlog closes.
#[test]
fn a_backlog_no_one_can_serve_ends() {
    let (mut f, mut q, p) = rewound(pull());
    f.sched.run(now(), &mut f.swarm, &mut q);
    assert!(f.swarm.store().backlog().is_some());
    f.sched.close_channel(p, &f.swarm, &mut q);
    f.sched.run(now(), &mut f.swarm, &mut q);
    assert_eq!(f.swarm.store().backlog(), None);
    assert!(!f.sched.s.backlog.unwrap().pending);
}

/// GC keeps a pending backlog's request times, so its answers count as
/// answers.
#[test]
fn gc_keeps_a_backlogs_request_times() {
    let (mut f, _, p) = rewound(pull());
    let requests = requests_at(&mut f, p, now());
    assert!(requests.iter().any(|b| b.base_offset() < 512));
    let before = f.sched.channels[&p].request_times.size();
    f.sched.gc(&f.swarm, now());
    assert_eq!(f.sched.channels[&p].request_times.size(), before);
}

/// A backlog opened again after one ended asks only from its own start,
/// where the store takes chunks.
#[test]
fn a_reopened_backlog_asks_only_from_its_start() {
    let (mut f, mut q, p) = rewound(pull());
    f.sched.close_backlog(&mut f.swarm);
    let had = f.sched.s.have_bins.clone();
    f.swarm.open_backlog(Bin(2 * 448), &had).unwrap();
    f.sched.open_backlog(Bin(2 * 448), Bin(2 * 512), &mut q);
    assert_eq!(f.swarm.store().store_from(), Bin(2 * 448));
    let requests = requests_at(&mut f, p, now());
    assert!(!requests.is_empty());
    assert!(
        requests
            .iter()
            .all(|b| (448..512).contains(&b.base_offset())),
        "{requests:?}"
    );
}
