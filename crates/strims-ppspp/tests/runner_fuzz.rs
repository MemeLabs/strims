// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Runner-level fuzzing: whatever frames a peer sends (mutated real
//! traffic, or messages of every type with hostile bins, streams, nonces
//! and timestamps), the runner never panics (debug builds check overflow),
//! keeps writing and ticking, and stays within its bounds.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::{
    Ack, Announce, AnnounceEntry, Cancel, ChannelWriter, Data, Have, Integrity, Message,
    MessageWriter, Ping, Pong, Request, SignedIntegrity, StreamCancel, StreamClose, StreamOpen,
    StreamRequest,
};
use strims_ppspp::integrity::{ProtectionMethod, VerifierOptions};
use strims_ppspp::{
    DATA_QUEUE_CAP, MAX_BIN, PeerKey, PushMode, Runner, SeedAllocation, SeedPolicy, SeedQos, Swarm,
    SwarmId, SwarmKey, SwarmOptions, TAP_CAP, TapLevel, Writer, WriterOptions,
};

const MTU: usize = 4096;
const WINDOW: usize = 64;
const STREAMS: usize = 4;

fn config() -> Config {
    Config {
        cases: Config::default().cases.max(128),
        rng_seed: RngSeed::Fixed(0x0dd_ba11),
        failure_persistence: None,
        ..Config::default()
    }
}

fn options_with(pm: ProtectionMethod, push_mode: PushMode) -> SwarmOptions {
    options_seed(pm, push_mode, SeedPolicy::default())
}

fn options_seed(pm: ProtectionMethod, push_mode: PushMode, seed: SeedPolicy) -> SwarmOptions {
    SwarmOptions {
        push_mode,
        seed,
        chunk_size: 64,
        chunks_per_signature: 4,
        live_window: WINDOW,
        stream_count: STREAMS,
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

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

struct Node {
    runner: Runner,
    swarm: SwarmKey,
    peer: PeerKey,
}

/// A seed with data and a leech, each with one peer on channel 1.
fn victims(pm: ProtectionMethod) -> (Node, Node, Writer) {
    victims_with(pm, PushMode::Off)
}

/// Victims in a push mode (the corpus is always Off traffic; the
/// synthetic messages cover every STREAM_* message).
fn victims_with(pm: ProtectionMethod, push: PushMode) -> (Node, Node, Writer) {
    victims_seed(pm, push, SeedPolicy::default())
}

/// Victims whose seed spends its upload by `policy`.
fn victims_seed(pm: ProtectionMethod, push: PushMode, policy: SeedPolicy) -> (Node, Node, Writer) {
    let key = strims_crypto::key_from_seed(&[7; 32]);
    let (mut w, swarm) = Writer::new(
        WriterOptions {
            swarm_options: options_seed(pm, push, policy),
            key: key.clone(),
        },
        t(0),
    )
    .unwrap();
    let mut seed = Runner::new(det_rng(1));
    let ss = seed.add_swarm(swarm);
    let sp = seed.add_peer(b"peer", t(0));
    seed.run_swarm(sp, ss, 1, 1, t(0)).unwrap();
    let data: Vec<u8> = (0..64 * 40).map(|i| (i * 7 % 251) as u8).collect();
    seed.write(ss, &mut w, t(1), &data).unwrap();

    let mut leech = Runner::new(det_rng(2));
    let ls =
        leech.add_swarm(Swarm::new(SwarmId::new(&key.public), options_with(pm, push)).unwrap());
    let lp = leech.add_peer(b"peer", t(0));
    leech.run_swarm(lp, ls, 1, 1, t(0)).unwrap();
    (
        Node {
            runner: seed,
            swarm: ss,
            peer: sp,
        },
        Node {
            runner: leech,
            swarm: ls,
            peer: lp,
        },
        w,
    )
}

/// Real traffic both ways between a seed and a leech, starting with the
/// handshakes. Built once per method.
fn corpus(pm: ProtectionMethod) -> &'static [Vec<u8>] {
    static CORPUS: std::sync::OnceLock<Vec<Vec<Vec<u8>>>> = std::sync::OnceLock::new();
    let all = CORPUS.get_or_init(|| METHODS.iter().map(|&pm| build_corpus(pm)).collect());
    let i = METHODS.iter().position(|&m| m == pm).unwrap();
    &all[i]
}

fn build_corpus(pm: ProtectionMethod) -> Vec<Vec<u8>> {
    let (mut a, mut b, mut w) = victims(pm);
    let mut out = vec![];
    let mut buf = vec![0u8; MTU];
    let mut now = t(0);
    for step in 0..40 {
        now += Duration::from_millis(50);
        if step % 8 == 4 {
            let data: Vec<u8> = (0..64 * 8).map(|i| (i * 3 % 241) as u8).collect();
            a.runner.write(a.swarm, &mut w, now, &data).unwrap();
        }
        for n in [&mut a, &mut b] {
            n.runner.handle_timeout(now);
        }
        for (from, to) in [(0, 1), (1, 0)] {
            let (x, y) = if from == 0 {
                (&mut a, &mut b)
            } else {
                (&mut b, &mut a)
            };
            while x.runner.wants_transmit(x.peer) {
                let n = x.runner.poll_transmit(x.peer, now, &mut buf);
                if n == 0 {
                    break;
                }
                out.push(buf[..n].to_vec());
                y.runner.handle_frame(y.peer, now, &buf[..n]).unwrap();
            }
            let _ = to;
        }
    }
    assert!(out.len() > 4);
    out
}

fn bin() -> impl Strategy<Value = Bin> {
    prop_oneof![
        (0u64..512).prop_map(Bin),
        (0u64..63, 0u64..8).prop_map(|(l, o)| Bin::new(l, o)),
        any::<u64>().prop_map(Bin),
        Just(Bin::ALL),
        Just(Bin::NONE),
        Just(MAX_BIN),
        Just(Bin(MAX_BIN.0 - 2)),
    ]
}

fn stream() -> impl Strategy<Value = u16> {
    prop_oneof![0u16..STREAMS as u16, any::<u16>()]
}

fn time() -> impl Strategy<Value = Time> {
    prop_oneof![
        (-10_000i64..10_000).prop_map(t),
        any::<i64>().prop_map(Time::from_unix_nanos),
    ]
}

/// One message of any type, owned so the strategy can hold it.
#[derive(Clone, Debug)]
enum Msg {
    Have(Bin),
    Request(Bin, Time),
    Cancel(Bin),
    Ack(Bin, i64),
    Data(Bin, Time, Vec<u8>),
    Integrity(Bin, Vec<u8>),
    Signed(Bin, Time, Vec<u8>),
    Ping(u64),
    Pong(u64),
    StreamRequest(u16, Bin),
    StreamCancel(u16),
    StreamOpen(u16, Bin),
    StreamClose(u16),
    /// Entries: stream, hits, late, misses, delay in ms, parent id.
    Announce(Vec<(u16, u64, u64, u64, u64, Vec<u8>)>),
    Restart,
    Choke,
    Unchoke,
}

/// A parent id: the peer's own, an unknown one, or one of any length.
fn parent_id() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        Just(b"peer".to_vec()),
        Just(b"none".to_vec()),
        prop::collection::vec(any::<u8>(), 0..40),
    ]
}

fn msg() -> impl Strategy<Value = Msg> {
    prop_oneof![
        4 => bin().prop_map(Msg::Have),
        3 => (bin(), time()).prop_map(|(b, t)| Msg::Request(b, t)),
        2 => bin().prop_map(Msg::Cancel),
        1 => (bin(), any::<i64>()).prop_map(|(b, d)| Msg::Ack(b, d)),
        2 => (bin(), time(), prop::collection::vec(any::<u8>(), 0..300))
            .prop_map(|(b, t, d)| Msg::Data(b, t, d)),
        1 => (bin(), prop::collection::vec(any::<u8>(), 32)).prop_map(|(b, h)| Msg::Integrity(b, h)),
        1 => (bin(), time(), prop::collection::vec(any::<u8>(), 64))
            .prop_map(|(b, t, s)| Msg::Signed(b, t, s)),
        1 => prop_oneof![Just(0u64), any::<u64>()].prop_map(Msg::Ping),
        1 => prop_oneof![Just(0u64), any::<u64>()].prop_map(Msg::Pong),
        2 => (stream(), bin()).prop_map(|(s, b)| Msg::StreamRequest(s, b)),
        1 => stream().prop_map(Msg::StreamCancel),
        2 => (stream(), bin()).prop_map(|(s, b)| Msg::StreamOpen(s, b)),
        1 => stream().prop_map(Msg::StreamClose),
        2 => prop::collection::vec(
            (stream(), any::<u64>(), any::<u64>(), any::<u64>(), 0u64..100_000, parent_id()),
            0..40,
        )
        .prop_map(Msg::Announce),
        1 => Just(Msg::Restart),
        1 => Just(Msg::Choke),
        1 => Just(Msg::Unchoke),
    ]
}

fn encode(msgs: &[Msg], channel: u64) -> Vec<u8> {
    let mut buf = vec![0u8; MTU];
    let mut cw = ChannelWriter::new(&mut buf, channel);
    for m in msgs {
        let entries: Vec<AnnounceEntry<'_>>;
        let m = match m {
            Msg::Have(b) => Message::Have(Have { bin: *b }),
            Msg::Request(b, t) => Message::Request(Request {
                bin: *b,
                timestamp: *t,
            }),
            Msg::Cancel(b) => Message::Cancel(Cancel { bin: *b }),
            Msg::Ack(b, d) => Message::Ack(Ack {
                bin: *b,
                delay_sample: Duration::from_nanos(*d),
            }),
            Msg::Data(b, t, d) => Message::Data(Data {
                bin: *b,
                timestamp: *t,
                data: d,
            }),
            Msg::Integrity(b, h) => Message::Integrity(Integrity { bin: *b, hash: h }),
            Msg::Signed(b, t, s) => Message::SignedIntegrity(SignedIntegrity {
                bin: *b,
                timestamp: *t,
                signature: s,
            }),
            Msg::Ping(n) => Message::Ping(Ping { nonce: *n }),
            Msg::Pong(n) => Message::Pong(Pong {
                nonce: *n,
                delay: 0,
            }),
            Msg::StreamRequest(s, b) => Message::StreamRequest(StreamRequest {
                stream: *s,
                bin: *b,
            }),
            Msg::StreamCancel(s) => Message::StreamCancel(StreamCancel { stream: *s }),
            Msg::StreamOpen(s, b) => Message::StreamOpen(StreamOpen {
                stream: *s,
                bin: *b,
            }),
            Msg::StreamClose(s) => Message::StreamClose(StreamClose { stream: *s }),
            Msg::Announce(e) => {
                entries = e
                    .iter()
                    .map(|(stream, hits, late, misses, ms, id)| AnnounceEntry {
                        stream: *stream,
                        hits: *hits,
                        late: *late,
                        misses: *misses,
                        delay: Duration::from_millis(*ms as i64),
                        parent: id,
                    })
                    .filter(AnnounceEntry::is_valid)
                    .take(strims_ppspp::codec::MAX_ANNOUNCE_ENTRIES)
                    .collect();
                Message::Announce(Announce::new(&entries).unwrap())
            }
            Msg::Restart => Message::Restart,
            Msg::Choke => Message::Choke,
            Msg::Unchoke => Message::Unchoke,
        };
        if cw.write(&m).is_err() {
            break;
        }
    }
    let n = cw.finish();
    buf.truncate(n);
    buf
}

#[derive(Clone, Debug)]
enum Action {
    /// A corpus frame with mutations: (index, [(position, byte)], cut).
    Mutated(usize, Vec<(usize, u8)>, Option<usize>),
    /// Synthesized messages on our channel (or a random one).
    Synth(Vec<Msg>, bool),
    /// Raw bytes.
    Raw(Vec<u8>),
    /// Advance the clock and poll.
    Tick(i64),
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        3 => (
            any::<usize>(),
            prop::collection::vec((any::<usize>(), any::<u8>()), 0..6),
            prop::option::of(any::<usize>()),
        )
            .prop_map(|(i, m, c)| Action::Mutated(i, m, c)),
        4 => (prop::collection::vec(msg(), 1..40), prop::bool::weighted(0.9))
            .prop_map(|(m, ours)| Action::Synth(m, ours)),
        1 => prop::collection::vec(any::<u8>(), 0..200).prop_map(Action::Raw),
        2 => (0i64..3_000).prop_map(Action::Tick),
    ]
}

/// Feeds the actions to a victim that got the peer's real handshake, and
/// keeps it writing and ticking, with its tap on and drained after every
/// step.
fn run(victim: &mut Node, corpus: &[Vec<u8>], handshake: &[u8], actions: &[Action]) {
    let mut now = t(100);
    let mut buf = vec![0u8; MTU];
    victim.runner.set_tap(TapLevel::All);
    victim
        .runner
        .handle_frame(victim.peer, now, handshake)
        .unwrap();
    for a in actions {
        let frame = match a {
            Action::Mutated(i, muts, cut) => {
                let mut f = corpus[i % corpus.len()].clone();
                for &(pos, b) in muts {
                    if !f.is_empty() {
                        let n = f.len();
                        f[pos % n] ^= b;
                    }
                }
                if let Some(c) = cut {
                    let n = f.len() + 1;
                    f.truncate(c % n);
                }
                f
            }
            Action::Synth(msgs, ours) => encode(msgs, if *ours { 1 } else { 9 }),
            Action::Raw(b) => b.clone(),
            Action::Tick(ms) => {
                now += Duration::from_millis(*ms);
                victim.runner.handle_timeout(now);
                vec![]
            }
        };
        // Errors are fine (the driver drops the peer); panics are not.
        let _ = victim.runner.handle_frame(victim.peer, now, &frame);
        let mut polls = 0;
        while victim.runner.wants_transmit(victim.peer) && polls < 64 {
            victim.runner.poll_transmit(victim.peer, now, &mut buf);
            polls += 1;
        }
        while victim.runner.poll_event().is_some() {}
        let mut events = 0;
        victim.runner.drain_tap(|_| events += 1);
        assert!(events <= 2 * TAP_CAP + 1);
    }

    let ps = victim.runner.peer_stats(victim.peer).unwrap();
    for q in 0..2 {
        assert!(ps.queued_data[q] <= DATA_QUEUE_CAP, "{ps:?}");
        assert!(ps.queue_ring[q] <= 2 * DATA_QUEUE_CAP + 64, "{ps:?}");
    }
    let cs = victim
        .runner
        .channel_stats(victim.peer, victim.swarm)
        .unwrap();
    let s = cs.scheduler;
    assert!(s.pending_requests <= WINDOW, "{s:?}");
    assert!(s.extra_messages <= 4 + 2 * STREAMS, "{s:?}");
    assert!(s.peer_have_map_size < 512, "{s:?}");
    let ss = victim.runner.swarm_stats(victim.swarm).unwrap();
    assert!(ss.bin_time_nodes <= 16 * WINDOW + 128, "{ss:?}");
    assert!(ss.subscribers <= STREAMS, "{ss:?}");
    let ps = victim.runner.push_stats(victim.swarm).unwrap();
    assert!(ps.children <= STREAMS, "{ps:?}");
    // Misses come only from chunks marked as expected.
    assert!(ps.misses <= ps.expected, "{ps:?}");
    if ps.mode == PushMode::Hybrid {
        assert_eq!(ps.parents.len(), STREAMS);
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn seed_survives_peer_input(
        pm in prop::sample::select(METHODS.to_vec()),
        push in prop::sample::select(vec![PushMode::Off, PushMode::Hybrid]),
        actions in prop::collection::vec(action(), 1..30),
    ) {
        let corpus = corpus(pm);
        let (mut seed, _, _) = victims_with(pm, push);
        // The leech's handshake is in the corpus' first leech frame.
        let hs = handshake_from(corpus, seed.runner.swarm(seed.swarm).unwrap());
        run(&mut seed, corpus, &hs, &actions);
    }

    #[test]
    fn adaptive_seed_survives_peer_input(
        pm in prop::sample::select(METHODS.to_vec()),
        actions in prop::collection::vec(action(), 1..30),
    ) {
        let corpus = corpus(pm);
        let policy = SeedPolicy {
            allocation: SeedAllocation::Adaptive,
            qos: SeedQos::ByAllocation,
        };
        let (mut seed, _, _) = victims_seed(pm, PushMode::Hybrid, policy);
        let hs = handshake_from(corpus, seed.runner.swarm(seed.swarm).unwrap());
        run(&mut seed, corpus, &hs, &actions);
    }

    #[test]
    fn leech_survives_peer_input(
        pm in prop::sample::select(METHODS.to_vec()),
        push in prop::sample::select(vec![PushMode::Off, PushMode::Hybrid]),
        actions in prop::collection::vec(action(), 1..30),
    ) {
        let corpus = corpus(pm);
        let (_, mut leech, _) = victims_with(pm, push);
        let hs = handshake_from(corpus, leech.runner.swarm(leech.swarm).unwrap());
        run(&mut leech, corpus, &hs, &actions);
    }
}

/// The first corpus frame holding a handshake with an epoch (the seed's),
/// which every victim accepts.
fn handshake_from(corpus: &[Vec<u8>], swarm: &Swarm) -> Vec<u8> {
    let cfg = swarm.reader_config();
    corpus
        .iter()
        .find(|f| {
            strims_ppspp::codec::FrameReader::new(f).any(|c| {
                c.is_ok_and(|(_, p)| {
                    strims_ppspp::codec::Reader::new(cfg, p).any(|m| {
                        matches!(m, Ok(Message::Handshake(h)) if h.options.summary().epoch.is_some())
                    })
                })
            })
        })
        .cloned()
        .unwrap()
}
