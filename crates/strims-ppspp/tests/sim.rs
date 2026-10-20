// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic swarm simulation.
//!
//! An event-queue simulator runs one seed and N peers, each a [`Runner`]
//! with one swarm, over a configurable mesh. Every directed link has a
//! latency and a loss rate; every node has an uplink and a downlink of
//! fixed bandwidth (the uplink shared by its links with start-time fair
//! queueing on the runner's QoS weights, the downlink a FIFO, which is where
//! queues build). The virtual clock jumps to the next event: a frame
//! arriving at a downlink or delivered from it, an uplink becoming free, a
//! runner's `poll_timeout`, or the seed's next write. Everything random
//! comes from one seeded `DetRng`, and a run hashes every frame it sends,
//! so two runs with the same seed must produce the same trace.
//!
//! Peers verify the bytes they read against the generator the seed writes
//! from, and record each chunk's delivery lag (read time minus the time the
//! seed published it).
//!
//! In WAN mode ([`Wan`]) every node sits at a [`Site`] of `strims_wan`: a
//! major city and an access link, whose rates its uplink and downlink run
//! at. A link's latency is both last miles plus half the city pair's round
//! trip, and each end adds jitter that delays frames without reordering
//! them. Frames travel as TCP stream data: their packets' headers take
//! uplink and downlink time, and a frame that loses a packet arrives a
//! retransmission later, holding back every frame behind it on its link.
//! Nothing is dropped. The paths draw from an rng of their own, derived
//! from the seed, so they shift no other draw.
//!
//! Adversarial nodes run a real runner (so they hold a real handshake and
//! real data) and mutate what they send: HAVE floods, corrupted data, or
//! huge and sentinel bins in every message type, or (for the push/pull
//! hybrid) accepting every stream subscription and withholding the pushes.
//!
//! The push/pull comparison runs the same scenarios with the same seed in
//! every [`PushMode`] and stream count: `sim_push_*` tests assert the
//! hybrid's behaviour, and the ignored `push_pull_matrix` prints the
//! comparison table (`cargo test --release -p strims-ppspp --test sim
//! -- --ignored --nocapture push_pull_matrix`).
//!
//! The request controller comparison runs the same scenarios with the same
//! seeds under LEDBAT and the BBR-style controller
//! (`SwarmOptions::request_controller`): `sim_bbr_*` tests assert the
//! BBR-style one, and the ignored `controller_ab` prints the table. Cross
//! traffic is a second swarm in the same runners, seeded by a peer, viewed
//! by every other node, sharing every uplink.
//!
//! The seed allocation runs measure how a seed on a constrained uplink
//! spends it, at 16 streams on WAN paths: the ignored
//! `seed_allocation_bounds` pins allocations the seed can't know (random
//! viewers, the fastest uplinks first) against the default admission, each
//! with and without uplink shares by allocation (`SeedQos`), and
//! `seed_allocation_ab` compares pull, the admission, the adaptive seed and
//! two pins across the WAN scenarios. Both default to 4 seeds of 30 s;
//! `SIM_SEEDS=8 SIM_SECS=60` gives the full tables.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use rand_core::Rng;
use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::{
    Cancel, ChannelWriter, Data, FrameReader, Have, Message, MessageWriter, Pong, Reader,
    ReaderConfig, Request, StreamClose, StreamOpen, StreamRequest,
};
use strims_ppspp::store::{self, BufferReader};
use strims_ppspp::{
    DATA_QUEUE_CAP, DeliveryMode, Event, MAX_BIN, PeerKey, PushMode, PushStats,
    RequestControllerKind, Runner, SeedAllocation, SeedPolicy, SeedQos, SwarmId, SwarmKey,
    SwarmOptions, Writer, WriterOptions,
};
use strims_ppspp_capture::{Capture, Clock, Dir, Options as CaptureOptions};
use strims_wan::sample::pick;
use strims_wan::wire::tcp_wire;
use strims_wan::{
    Access, Carrier, City, EndStats, GLOBAL_AUDIENCE, Network, RESIDENTIAL, Region, Site,
};

/// A frame body: a 16 KiB link MTU minus vnic's 4 byte frame header.
const MTU: usize = 16 * 1024 - 4;
const CHUNK: usize = 1024;
const MUNRO: usize = 64 * CHUNK;

fn t0() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

/// The stream the seed writes: a byte per offset.
fn stream_byte(o: u64) -> u8 {
    let x = o.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (o >> 7);
    (x >> 56) as u8
}

/// The cross-traffic swarm's stream.
fn cross_byte(o: u64) -> u8 {
    stream_byte(o ^ 0x5bd1_e995_0000_0000)
}

/// A directory name for a run's captures: its seed and a hash of its
/// config.
fn run_tag(cfg: &Config) -> String {
    let mut h = 0xcbf2_9ce4_8422_2325;
    fnv(&mut h, format!("{cfg:?}").as_bytes());
    format!("s{}-{:08x}", cfg.seed, h as u32)
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= u64::from(b);
        *h = h.wrapping_mul(0x0100_0000_01b3);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attack {
    /// Thousands of HAVEs per frame: scattered, far ahead, huge.
    HaveFlood,
    /// Serves corrupted chunks for everything it announces.
    Garbage,
    /// Huge and sentinel bins in every message type, unsolicited stream
    /// opens, stream request floods, a pong for nonce 0.
    HugeBins,
    /// A malicious parent: accepts every stream subscription (turns its
    /// STREAM_CLOSEs into STREAM_OPENs) and sends only the data it was
    /// asked for, withholding every push.
    WithholdPush,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Seed,
    Peer,
    Adversary(Attack),
    /// A peer that disappeared mid-stream: it sends and receives nothing.
    Dead,
}

/// A change to the seed's network at a given time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeedStep {
    /// The seed's uplink becomes this many bytes per second.
    Uplink(u64),
    /// Every link to and from the seed gets this one-way latency.
    Latency(Duration),
    /// The seed's channels close and open again on both ends, as after a
    /// dropped and redialed link.
    Reconnect,
}

/// Something that happens to the busiest relay (the peer that uploaded
/// the most so far) mid-stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Incident {
    /// It disappears without a word (no `remove_peer` on its neighbours).
    Dies,
    /// Its uplink drops to this many bytes per second.
    Slows(u64),
}

#[derive(Clone, Debug)]
struct Config {
    seed: u64,
    peers: usize,
    adversaries: Vec<Attack>,
    /// Links per peer in a random mesh; 0 for a full mesh.
    degree: usize,
    /// If non-zero, the mesh instead links every node `i > 0` to
    /// `min(dial, i)` earlier nodes, the seed being node 0.
    dial: usize,
    /// Peers connected to the seed (0 for all).
    seed_degree: usize,
    /// Bytes per second the seed writes, in whole munros.
    stream_rate: u64,
    /// Bytes per chunk.
    chunk_size: usize,
    /// Chunks per munro (one signature, one write).
    munro_chunks: usize,
    /// The largest frame body a link carries.
    mtu: usize,
    /// Uplinks pace frames with a rate limiter in front of a link that has
    /// no bandwidth limit, so the link adds no serialization delay.
    paced_uplinks: bool,
    /// Seconds of stream.
    stream_secs: u64,
    seed_up: u64,
    peer_up: u64,
    peer_down: u64,
    /// Peers (by index from 1) with a slower downlink.
    constrained: Vec<(usize, u64)>,
    delivery_mode: DeliveryMode,
    live_window: usize,
    latency_ms: (i64, i64),
    loss: f64,
    push_mode: PushMode,
    stream_count: usize,
    /// Seconds after the start, and what happens to the busiest relay.
    incident: Option<(i64, Incident)>,
    /// Changes to the seed's network, in seconds after the start (sorted).
    steps: Vec<(i64, SeedStep)>,
    /// Lags count from this many seconds after the start.
    warm_secs: i64,
    /// Print this node's push state every 250 ms.
    trace_node: Option<usize>,
    /// Print every peer's read position every this many seconds.
    progress: Option<i64>,
    /// The swarm's request controller.
    controller: RequestControllerKind,
    /// A second swarm sharing the nodes.
    cross: Option<Cross>,
    /// How the seed spends its upload in Hybrid.
    seed_policy: SeedPolicy,
    /// A fixed seed allocation: per stream, the nodes the seed pushes it
    /// to (`Runner::pin_seed_allocation`).
    pin: Option<Vec<Vec<usize>>>,
    /// Real-world paths. With them `latency_ms`, `loss`, `seed_up`,
    /// `peer_up`, `peer_down` and `constrained` go unused, and `loss` must
    /// stay zero.
    wan: Option<Wan>,
    /// Capture every node's frames (see [`CaptureTo`]); `SIM_CAPTURE`
    /// sets it for any run.
    capture: Option<CaptureTo>,
}

/// Where a run's captures go: `node-<i>.ppcap` in `dir`, one per node,
/// named by node index like the peers in its runner.
#[derive(Clone, Debug)]
struct CaptureTo {
    dir: PathBuf,
    /// Capture the runners' decisions too.
    tap: bool,
}

type SimCapture = Capture<Box<dyn FnMut(u64) -> io::Result<BufWriter<std::fs::File>>>>;

/// Real-world paths ([`strims_wan`]): node `i` sits at `sites[i]`, the seed
/// being node 0.
#[derive(Clone, Debug)]
struct Wan {
    sites: Vec<Site>,
    options: strims_wan::Options,
}

/// Cross traffic: a second swarm in every honest node's runner, at the
/// same rate, seeded by peer `seed` and viewed by every other honest node
/// (the main seed too), over the same links and uplinks.
#[derive(Clone, Copy, Debug)]
struct Cross {
    seed: usize,
    controller: RequestControllerKind,
    push_mode: PushMode,
}

impl Config {
    fn new(seed: u64, peers: usize) -> Config {
        Config {
            seed,
            peers,
            adversaries: vec![],
            degree: 0,
            dial: 0,
            seed_degree: 0,
            stream_rate: 256 * 1024,
            chunk_size: CHUNK,
            munro_chunks: MUNRO / CHUNK,
            mtu: MTU,
            paced_uplinks: false,
            stream_secs: 12,
            seed_up: 16 * 256 * 1024,
            peer_up: 4 * 256 * 1024,
            peer_down: 12 * 256 * 1024,
            constrained: vec![],
            delivery_mode: DeliveryMode::LOW_LATENCY,
            live_window: 1 << 12,
            latency_ms: (5, 80),
            loss: 0.0,
            push_mode: PushMode::Off,
            stream_count: 1,
            incident: None,
            steps: vec![],
            warm_secs: 4,
            trace_node: None,
            progress: None,
            controller: RequestControllerKind::Default,
            cross: None,
            seed_policy: SeedPolicy::default(),
            pin: None,
            wan: None,
            capture: None,
        }
    }
}

/// A node's side of the cross-traffic swarm: what the main swarm keeps in
/// `Node` itself.
struct Side {
    key: SwarmKey,
    writer: Option<Writer>,
    reader: BufferReader,
    reads: bool,
    start: Option<u64>,
    pos: u64,
    mismatches: u64,
    underruns: u64,
    resets: u64,
    lags: Vec<(Time, Duration)>,
    read_chunks: Vec<bool>,
}

struct Node {
    role: Role,
    runner: Runner,
    swarm: SwarmKey,
    writer: Option<Writer>,
    reader: BufferReader,
    /// Outgoing link per remote node.
    links: BTreeMap<usize, usize>,
    up_bw: u64,
    down_bw: u64,
    up_free: Time,
    down_free: Time,
    /// Start-time fair queueing over the uplink, per link.
    vtime: BTreeMap<usize, u128>,
    weights: BTreeMap<PeerKey, u64>,
    start: Option<u64>,
    pos: u64,
    mismatches: u64,
    underruns: u64,
    resets: u64,
    lags: Vec<(Time, Duration)>,
    /// The chunks it read, by index.
    read_chunks: Vec<bool>,
    /// When each chunk's data first reached it from an honest node, by
    /// index (`NIL` until then).
    arrived: Vec<Time>,
    max_queue_delay: Duration,
    errors: u64,
    removed: BTreeSet<PeerKey>,
    /// The cross-traffic swarm, if any.
    cross: Option<Side>,
    /// Bytes through its uplink and downlink (TCP framing included in WAN
    /// mode) and its downlink's longest queue. The path's counters are the
    /// network's ([`Sim::wan_stats`]).
    wire: EndStats,
    cap: Option<SimCapture>,
}

struct Link {
    from: usize,
    to: usize,
    latency: Duration,
    loss: f64,
    /// The peer key of `to` on `from`.
    peer: PeerKey,
    /// The channel id `to` expects on `from`'s frames.
    channel: u64,
    /// The peer key of `from` on `to`.
    rpeer: PeerKey,
    /// In WAN mode, the latest arrival on the link: a frame waits for every
    /// frame ahead of it, retransmissions included.
    last_arrival: Time,
}

#[derive(Debug)]
enum Ev {
    /// A frame reaches the receiver's downlink queue.
    Arrive { link: usize, frame: Vec<u8> },
    /// A frame leaves the downlink and is handled.
    Deliver { link: usize, frame: Vec<u8> },
}

struct Sim {
    cfg: Config,
    nodes: Vec<Node>,
    links: Vec<Link>,
    rng: DetRng,
    events: BTreeMap<(Time, u64), Ev>,
    seq: u64,
    now: Time,
    publish_times: Vec<Time>,
    /// The REQUESTs a `WithholdPush` adversary got, per link into it.
    requested: BTreeMap<usize, BTreeSet<Bin>>,
    /// STREAM_OPENs `WithholdPush` adversaries sent.
    withheld_opens: u64,
    /// Copies of each chunk the seed sent, for the debugging aids.
    seed_sent: BTreeMap<u64, u32>,
    /// When the incident happened, and to whom.
    incident: Option<(Time, usize)>,
    /// The steps applied so far.
    steps_done: usize,
    /// Bytes per munro.
    munro: usize,
    munros: usize,
    written: usize,
    next_write: Time,
    trace: u64,
    frames: u64,
    bytes: u64,
    buf: Vec<u8>,
    read_buf: Vec<u8>,
    rcfg: ReaderConfig,
    end: Time,
    /// The paths of a WAN run.
    wan: Option<Network>,
    /// The seed's allocation, once a second.
    alloc: Vec<AllocSample>,
    next_alloc: Time,
}

/// The seed's allocation at one moment (`Runner::seed_allocation`).
struct AllocSample {
    at: Time,
    /// Per neighbour (node index): the streams pushed to it.
    held: BTreeMap<usize, usize>,
    /// The neighbours an adaptive seed holds copies at, or (any other
    /// seed) those it pushes anything to.
    holders: BTreeSet<usize>,
    /// The seed's objective: the p90 over neighbours of the latest stream's
    /// HAVE lag after publishing, less half the round trip. A stream not
    /// announced lately counts as [`STALE`].
    objective: Duration,
}

/// What a stream a neighbour stopped announcing counts as in the seed's
/// objective (`push::CANDIDATE_FRESHNESS`).
const STALE: Duration = Duration::from_secs(2);

// Every field is printed through `Debug`.
#[allow(dead_code)]
#[derive(Debug)]
struct Report {
    trace: u64,
    frames: u64,
    bytes: u64,
    total: u64,
    /// Per honest peer: start offset, final position.
    positions: Vec<(u64, u64)>,
    /// The share of all chunks that honest peers read.
    delivery: f64,
    mismatches: u64,
    underruns: u64,
    resets: u64,
    lag_p50: Duration,
    lag_p99: Duration,
    lag_max: Duration,
    /// Worst-stripe lag ([`Sim::munro_lags`]): p90 and p99.
    stripe_p90: Duration,
    stripe_p99: Duration,
    /// In WAN mode, the lag p99 of each region's peers.
    regions: Vec<(Region, Duration)>,
    max_queue_delay: Duration,
    finish: Time,
    /// The seed's share of all data bytes sent.
    seed_share: f64,
    /// Data bytes sent by everyone.
    data_bytes: u64,
    /// Chunks honest peers received that they already had.
    wasted: u64,
    /// Push counters summed over honest peers.
    push: PushStats,
    /// The seed's push subscribers, all streams, at the end.
    seed_children: usize,
    /// The seed's push subscriptions (stream slots), mean over the samples
    /// after the warm-up.
    seed_slots: f64,
    /// The seed's objective ([`AllocSample::objective`]), median over the
    /// samples after the warm-up.
    seed_objective: Duration,
    /// When the seed's holders ([`AllocSample::holders`]) last changed,
    /// after the stream started.
    settled: Duration,
    /// Samples after the warm-up at which they changed.
    holder_changes: usize,
    /// Of the chunks the seed sent: request answers, and distinct chunks.
    seed_pull_share: f64,
    seed_distinct: f64,
    /// The incident's victim, and the p99 and worst lag of the chunks read
    /// in the 3 s after it.
    recovery: Option<(usize, Duration, Duration)>,
    /// The cross-traffic swarm's results.
    cross: Option<CrossReport>,
}

/// The cross-traffic swarm, measured like the main one.
#[allow(dead_code)]
#[derive(Debug)]
struct CrossReport {
    delivery: f64,
    lag_p50: Duration,
    lag_p99: Duration,
    seed_share: f64,
    wasted: u64,
    incomplete: usize,
}

impl Sim {
    fn new(cfg: Config) -> Sim {
        let mut rng = det_rng(cfg.seed);
        let key = strims_crypto::key_from_seed(&[9; 32]);
        let options = SwarmOptions {
            chunk_size: cfg.chunk_size,
            chunks_per_signature: cfg.munro_chunks,
            live_window: cfg.live_window,
            delivery_mode: cfg.delivery_mode,
            stream_count: cfg.stream_count,
            push_mode: cfg.push_mode,
            request_controller: cfg.controller,
            seed: cfg.seed_policy,
            ..Default::default()
        };
        let cross_key = strims_crypto::key_from_seed(&[10; 32]);
        let cross_options = cfg.cross.map(|c| SwarmOptions {
            push_mode: c.push_mode,
            request_controller: c.controller,
            ..options.clone()
        });

        let mut roles = vec![Role::Seed];
        roles.extend((0..cfg.peers).map(|_| Role::Peer));
        roles.extend(cfg.adversaries.iter().map(|&a| Role::Adversary(a)));
        if let Some(w) = &cfg.wan {
            assert_eq!(w.sites.len(), roles.len(), "a site per node");
            assert!(cfg.loss <= 0.0, "WAN paths lose packets of their own");
        }

        let mut nodes = Vec::new();
        for (i, &role) in roles.iter().enumerate() {
            let mut runner = Runner::new(fork(&mut rng));
            let (writer, swarm) = if role == Role::Seed {
                let (w, s) = Writer::new(
                    WriterOptions {
                        swarm_options: options.clone(),
                        key: key.clone(),
                    },
                    t0(),
                )
                .unwrap();
                (Some(w), s)
            } else {
                (
                    None,
                    strims_ppspp::Swarm::new(SwarmId::new(&key.public), options.clone()).unwrap(),
                )
            };
            let reader = swarm.reader();
            let swarm = runner.add_swarm(swarm);
            let cross = match (&cross_options, cfg.cross, role) {
                (Some(o), Some(c), Role::Seed | Role::Peer) => {
                    let (writer, swarm) = if i == c.seed {
                        let (w, s) = Writer::new(
                            WriterOptions {
                                swarm_options: o.clone(),
                                key: cross_key.clone(),
                            },
                            t0(),
                        )
                        .unwrap();
                        (Some(w), s)
                    } else {
                        let id = SwarmId::new(&cross_key.public);
                        (None, strims_ppspp::Swarm::new(id, o.clone()).unwrap())
                    };
                    let reader = swarm.reader();
                    Some(Side {
                        key: runner.add_swarm(swarm),
                        writer,
                        reader,
                        reads: i != c.seed,
                        start: None,
                        pos: 0,
                        mismatches: 0,
                        underruns: 0,
                        resets: 0,
                        lags: vec![],
                        read_chunks: vec![],
                    })
                }
                _ => None,
            };
            let (mut up, mut down) = match role {
                Role::Seed => (cfg.seed_up, cfg.peer_down),
                _ => (cfg.peer_up, cfg.peer_down),
            };
            if let Some(&(_, d)) = cfg.constrained.iter().find(|(n, _)| *n == i) {
                down = d;
            }
            if let Some(w) = &cfg.wan {
                let p = w.sites[i].profile;
                (up, down) = (p.up, p.down);
            }
            nodes.push(Node {
                role,
                runner,
                swarm,
                writer,
                reader,
                links: BTreeMap::new(),
                up_bw: up,
                down_bw: down,
                up_free: t0(),
                down_free: t0(),
                vtime: BTreeMap::new(),
                weights: BTreeMap::new(),
                start: None,
                pos: 0,
                mismatches: 0,
                underruns: 0,
                resets: 0,
                lags: vec![],
                read_chunks: vec![],
                arrived: vec![],
                max_queue_delay: Duration::ZERO,
                errors: 0,
                removed: BTreeSet::new(),
                cross,
                wire: EndStats::default(),
                cap: None,
            });
        }

        // The mesh: the seed's peers, then each peer's.
        let n = nodes.len();
        let mut edges = BTreeSet::new();
        let honest: Vec<usize> = (1..=cfg.peers).collect();
        let mut seed_peers = honest.clone();
        if cfg.seed_degree > 0 && cfg.seed_degree < seed_peers.len() {
            shuffle(&mut seed_peers, &mut rng);
            seed_peers.truncate(cfg.seed_degree);
        }
        for &p in &seed_peers {
            edges.insert((0, p));
        }
        if cfg.dial > 0 {
            edges.clear();
            for i in 1..n {
                let mut targets: Vec<usize> = Vec::new();
                while targets.len() < cfg.dial.min(i) {
                    let j = (rng.next_u64() % i as u64) as usize;
                    if !targets.contains(&j) {
                        targets.push(j);
                    }
                }
                for j in targets {
                    edges.insert((j, i));
                }
            }
        }
        for i in (1..n).filter(|_| cfg.dial == 0) {
            let mut others: Vec<usize> = (1..n).filter(|&j| j != i).collect();
            if cfg.degree > 0 {
                shuffle(&mut others, &mut rng);
                others.truncate(cfg.degree);
            }
            for j in others {
                edges.insert((i.min(j), i.max(j)));
            }
        }
        // Adversaries also reach the seed so they hold real data.
        for (i, &role) in roles.iter().enumerate() {
            if matches!(role, Role::Adversary(_)) {
                edges.insert((0, i));
            }
        }

        let mut links = Vec::new();
        for &(a, b) in &edges {
            let span = (cfg.latency_ms.1 - cfg.latency_ms.0).max(1) as u64;
            let latency = Duration::from_millis(cfg.latency_ms.0 + (rng.next_u64() % span) as i64);
            let pa = nodes[a].runner.add_peer(&[b as u8], t0());
            let pb = nodes[b].runner.add_peer(&[a as u8], t0());
            let (ca, cb) = (a as u64 + 1, b as u64 + 1);
            let sa = nodes[a].swarm;
            let sb = nodes[b].swarm;
            nodes[a].runner.run_swarm(pa, sa, cb, ca, t0()).unwrap();
            nodes[b].runner.run_swarm(pb, sb, ca, cb, t0()).unwrap();
            if let (Some(xa), Some(xb)) = (
                nodes[a].cross.as_ref().map(|x| x.key),
                nodes[b].cross.as_ref().map(|x| x.key),
            ) {
                let (ka, kb) = (ca + 1000, cb + 1000);
                nodes[a].runner.run_swarm(pa, xa, kb, ka, t0()).unwrap();
                nodes[b].runner.run_swarm(pb, xb, ka, kb, t0()).unwrap();
            }
            let la = links.len();
            links.push(Link {
                from: a,
                to: b,
                latency,
                loss: cfg.loss,
                peer: pa,
                channel: ca,
                rpeer: pb,
                last_arrival: Time::NIL,
            });
            links.push(Link {
                from: b,
                to: a,
                latency,
                loss: cfg.loss,
                peer: pb,
                channel: cb,
                rpeer: pa,
                last_arrival: Time::NIL,
            });
            nodes[a].links.insert(b, la);
            nodes[b].links.insert(a, la + 1);
            nodes[a].vtime.insert(la, 0);
            nodes[b].vtime.insert(la + 1, 0);
        }
        let wan = cfg.wan.as_ref().map(|w| {
            let net = Network::new(&w.sites, w.options, &mut det_rng(cfg.seed ^ 0x5741_4e00));
            for l in &mut links {
                l.latency = net.one_way(l.from, l.to);
            }
            net
        });

        if let Some(pin) = &cfg.pin {
            let peers = pin
                .iter()
                .map(|ns| {
                    ns.iter()
                        .filter_map(|n| nodes[0].links.get(n).map(|&l| links[l].peer))
                        .collect()
                })
                .collect();
            let swarm = nodes[0].swarm;
            nodes[0]
                .runner
                .pin_seed_allocation(swarm, Some(peers))
                .unwrap();
        }

        let capture = cfg.capture.clone().or_else(|| {
            knob::<String>("SIM_CAPTURE").map(|dir| CaptureTo {
                dir: Path::new(&dir).join(run_tag(&cfg)),
                tap: knob::<String>("SIM_CAPTURE_TAP").as_deref() != Some("0"),
            })
        });
        if let Some(to) = &capture {
            std::fs::create_dir_all(&to.dir).unwrap();
            for (i, n) in nodes.iter_mut().enumerate() {
                let label = match &cfg.wan {
                    Some(w) => {
                        format!("{:?} {:?} {:?}", n.role, w.sites[i].city, w.sites[i].access)
                    }
                    None => format!("{:?}", n.role),
                };
                let path = to.dir.join(format!("node-{i}.ppcap"));
                let sink: Box<dyn FnMut(u64) -> io::Result<BufWriter<std::fs::File>>> =
                    Box::new(move |_| Ok(BufWriter::new(std::fs::File::create(&path)?)));
                let opts = CaptureOptions {
                    clock: Clock::Sim,
                    node: &[i as u8],
                    label: &label,
                    max_file_bytes: 0,
                    tap: to.tap,
                };
                n.cap = Some(Capture::new(sink, opts, t0(), &mut n.runner).unwrap());
            }
        }

        let munro = cfg.munro_chunks * cfg.chunk_size;
        let munros = (cfg.stream_rate * cfg.stream_secs) as usize / munro;
        for node in &mut nodes {
            node.read_chunks = vec![false; munros * cfg.munro_chunks];
            node.arrived = vec![Time::NIL; munros * cfg.munro_chunks];
            if let Some(x) = node.cross.as_mut() {
                x.read_chunks = vec![false; munros * cfg.munro_chunks];
            }
        }
        let rcfg = nodes[0]
            .runner
            .swarm(nodes[0].swarm)
            .unwrap()
            .reader_config();
        let end = t0() + Duration::from_secs(cfg.stream_secs as i64 + 20);
        let mtu = cfg.mtu;
        Sim {
            cfg,
            nodes,
            links,
            rng,
            events: BTreeMap::new(),
            seq: 0,
            now: t0(),
            publish_times: vec![],
            requested: BTreeMap::new(),
            withheld_opens: 0,
            seed_sent: BTreeMap::new(),
            incident: None,
            steps_done: 0,
            munro,
            munros,
            written: 0,
            next_write: t0() + Duration::SECOND,
            trace: 0xcbf2_9ce4_8422_2325,
            frames: 0,
            bytes: 0,
            buf: vec![0; mtu],
            read_buf: vec![0; 64 * 1024],
            rcfg,
            end,
            wan,
            alloc: vec![],
            next_alloc: t0() + Duration::SECOND,
        }
    }

    fn push(&mut self, t: Time, e: Ev) {
        self.seq += 1;
        self.events.insert((t, self.seq), e);
    }

    fn done(&self) -> bool {
        let total = (self.munros * self.munro) as u64;
        self.written == self.munros
            && self
                .nodes
                .iter()
                .filter(|n| n.role == Role::Peer)
                .all(|n| n.pos >= total)
            && self
                .nodes
                .iter()
                .filter_map(|n| n.cross.as_ref())
                .all(|x| !x.reads || x.pos >= total)
    }

    fn next_time(&self) -> Time {
        let mut t = Time::MAX;
        if let Some((&(et, _), _)) = self.events.first_key_value() {
            t = t.min(et);
        }
        if self.written < self.munros {
            t = t.min(self.next_write);
        }
        if let Some((secs, _)) = self.cfg.incident
            && self.incident.is_none()
        {
            t = t.min(t0() + Duration::from_secs(secs));
        }
        if let Some(&(secs, _)) = self.cfg.steps.get(self.steps_done) {
            t = t.min(t0() + Duration::from_secs(secs));
        }
        for n in &self.nodes {
            if n.role == Role::Dead {
                continue;
            }
            if let Some(rt) = n.runner.poll_timeout() {
                t = t.min(rt);
            }
            if n.links
                .values()
                .any(|&l| n.runner.wants_transmit(self.links[l].peer))
            {
                t = t.min(n.up_free.max(self.now));
            }
        }
        t
    }

    fn run(&mut self) -> Report {
        let trace = self.cfg.trace_node;
        let mut next_trace = t0();
        let progress = self.cfg.progress;
        let mut next_progress = t0();
        while !self.done() {
            let t = self.next_time();
            if t > self.end {
                break;
            }
            self.now = self.now.max(t);
            self.step();
            if self.now >= self.next_alloc {
                self.next_alloc += Duration::SECOND;
                self.sample_alloc();
            }
            if let Some(every) = progress
                && self.now >= next_progress
            {
                next_progress += Duration::from_secs(every);
                let pos: Vec<u64> = self
                    .nodes
                    .iter()
                    .skip(1)
                    .map(|n| n.pos / self.cfg.chunk_size as u64)
                    .collect();
                let copies: u64 = self.seed_sent.values().map(|&c| u64::from(c)).sum();
                eprintln!(
                    "t={} written {} pos {pos:?} seed sent {copies} chunks, {} distinct",
                    ms(self.now - t0()) / 1000,
                    self.written * self.cfg.munro_chunks,
                    self.seed_sent.len()
                );
                if knob::<String>("SIM_DUMP").is_some() {
                    self.dump();
                }
            }
            if let Some(i) = trace
                && self.now >= next_trace
            {
                next_trace += Duration::from_millis(250);
                let n = &self.nodes[i];
                if let Some(p) = n.runner.push_stats(n.swarm) {
                    let parents: Vec<String> = p
                        .parents
                        .iter()
                        .map(|x| {
                            x.and_then(|k| n.runner.peer_id(k).map(|id| id[0].to_string()))
                                .unwrap_or("-".into())
                        })
                        .collect();
                    eprintln!(
                        "t={} node {i} parents {parents:?} lag {:?} hits {} misses {} late {} pruned {} opens {} declined {} pos {} up {} busy {}",
                        ms(self.now - t0()),
                        p.parent_lag
                            .iter()
                            .map(|x| x.map(|(a, b)| (ms(a), ms(b))))
                            .collect::<Vec<_>>(),
                        p.hits,
                        p.misses,
                        p.late,
                        p.pruned,
                        p.opens,
                        p.declined,
                        n.pos,
                        p.upload_rate / 1024,
                        p.upload_busy_permille
                    );
                }
            }
        }
        for n in &mut self.nodes {
            if let Some(c) = n.cap.as_mut() {
                c.drain(self.now, &mut n.runner);
                c.flush().unwrap();
                assert!(c.error().is_none(), "capture: {:?}", c.error());
            }
        }
        self.report()
    }

    fn step(&mut self) {
        let now = self.now;
        while let Some(entry) = self.events.first_entry() {
            if entry.key().0 > now {
                break;
            }
            match entry.remove() {
                Ev::Arrive { link, frame } => {
                    let to = self.links[link].to;
                    let wire = match self.wan {
                        Some(_) => tcp_wire(frame.len()).0,
                        None => frame.len(),
                    };
                    let node = &mut self.nodes[to];
                    let start = node.down_free.max(now);
                    let qd = start - now;
                    if now > t0() + Duration::from_secs(3) && qd > node.max_queue_delay {
                        node.max_queue_delay = qd;
                    }
                    node.down_free = start + tx_time(wire, node.down_bw);
                    node.wire.down_bytes += wire as u64;
                    node.wire.down_queue_max = node.wire.down_queue_max.max(qd);
                    let at = node.down_free;
                    self.push(at, Ev::Deliver { link, frame });
                }
                Ev::Deliver { link, frame } => self.deliver(link, &frame),
            }
        }

        for i in 0..self.nodes.len() {
            let n = &mut self.nodes[i];
            if n.role != Role::Dead && n.runner.poll_timeout().is_some_and(|t| t <= now) {
                n.runner.handle_timeout(now);
            }
        }

        if self.written < self.munros && now >= self.next_write {
            self.write();
        }

        if let Some((secs, what)) = self.cfg.incident
            && self.incident.is_none()
            && now >= t0() + Duration::from_secs(secs)
        {
            let victim = self.busiest_relay();
            match what {
                Incident::Dies => self.nodes[victim].role = Role::Dead,
                Incident::Slows(bw) => self.nodes[victim].up_bw = bw,
            }
            self.incident = Some((now, victim));
        }

        while let Some(&(secs, step)) = self.cfg.steps.get(self.steps_done) {
            if now < t0() + Duration::from_secs(secs) {
                break;
            }
            self.steps_done += 1;
            match step {
                SeedStep::Uplink(bw) => self.nodes[0].up_bw = bw,
                SeedStep::Latency(d) => {
                    for l in &mut self.links {
                        if l.from == 0 || l.to == 0 {
                            l.latency = d;
                            if let Some(net) = self.wan.as_mut() {
                                net.set_one_way(l.from, l.to, Some(d));
                            }
                        }
                    }
                }
                SeedStep::Reconnect => {
                    for l in 0..self.links.len() {
                        let (from, to, peer) =
                            (self.links[l].from, self.links[l].to, self.links[l].peer);
                        if from != 0 && to != 0 {
                            continue;
                        }
                        let rchannel = self.links[l ^ 1].channel;
                        let channel = self.links[l].channel;
                        // A redialed link is a new TCP connection: nothing
                        // on it waits for the old one's frames.
                        self.links[l].last_arrival = Time::NIL;
                        let n = &mut self.nodes[from];
                        n.runner.stop_swarm(peer, n.swarm);
                        n.runner
                            .run_swarm(peer, n.swarm, rchannel, channel, now)
                            .unwrap();
                    }
                }
            }
        }

        for i in 0..self.nodes.len() {
            self.drain(i);
        }

        for i in 0..self.nodes.len() {
            self.transmit(i);
        }

        for n in &mut self.nodes {
            if let Some(c) = n.cap.as_mut() {
                c.drain(now, &mut n.runner);
            }
        }
    }

    fn write(&mut self) {
        let now = self.now;
        let munro = self.munro;
        let off = self.written * munro;
        let data: Vec<u8> = (off..off + munro).map(|o| stream_byte(o as u64)).collect();
        let seed = &mut self.nodes[0];
        let mut w = seed.writer.take().unwrap();
        seed.runner.write(seed.swarm, &mut w, now, &data).unwrap();
        seed.writer = Some(w);
        if let Some(c) = self.cfg.cross {
            let data: Vec<u8> = (off..off + munro).map(|o| cross_byte(o as u64)).collect();
            let Node { runner, cross, .. } = &mut self.nodes[c.seed];
            if let Some(x) = cross.as_mut() {
                let mut w = x.writer.take().unwrap();
                runner.write(x.key, &mut w, now, &data).unwrap();
                x.writer = Some(w);
            }
        }
        self.publish_times
            .extend(std::iter::repeat_n(now, munro / self.cfg.chunk_size));
        self.written += 1;
        self.next_write += Duration::from_nanos(
            (munro as i64).saturating_mul(1_000_000_000) / self.cfg.stream_rate as i64,
        );
    }

    fn deliver(&mut self, link: usize, frame: &[u8]) {
        let (to, rpeer) = (self.links[link].to, self.links[link].rpeer);
        let from = self.links[link].from;
        let now = self.now;
        if self.nodes[to].role == Role::Dead || self.nodes[from].role == Role::Dead {
            return;
        }
        if self.nodes[to].role == Role::Adversary(Attack::WithholdPush) {
            let requested = self.requested.entry(link).or_default();
            for c in FrameReader::new(frame) {
                let Ok((_, payload)) = c else { break };
                for m in Reader::new(self.rcfg, payload) {
                    match m {
                        Ok(Message::Request(r)) => {
                            requested.insert(r.bin);
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
        }
        if self.nodes[to].role == Role::Peer
            && matches!(self.nodes[from].role, Role::Seed | Role::Peer)
            && !self.nodes[to].removed.contains(&rpeer)
        {
            self.record_arrivals(to, frame);
        }
        let node = &mut self.nodes[to];
        if node.removed.contains(&rpeer) {
            return;
        }
        let res = match node.cap.as_mut() {
            Some(c) => c.handle_frame(&mut node.runner, rpeer, now, frame),
            None => node.runner.handle_frame(rpeer, now, frame),
        };
        if let Err(e) = res {
            // The driver drops a peer whose frame fails.
            assert!(
                matches!(node.role, Role::Adversary(_))
                    || matches!(self.nodes[self.links[link].from].role, Role::Adversary(_)),
                "honest frame failed: {e}"
            );
            let node = &mut self.nodes[to];
            node.errors += 1;
            node.runner.remove_peer(rpeer);
            node.removed.insert(rpeer);
        }
    }

    /// Records the seed's allocation and objective.
    fn sample_alloc(&mut self) {
        let seed = &self.nodes[0];
        let Some(a) = seed.runner.seed_allocation(seed.swarm, self.now) else {
            return;
        };
        let mut held = BTreeMap::new();
        let mut holders = BTreeSet::new();
        let mut worst = Vec::new();
        for nb in &a.neighbours {
            let Some(id) = seed.runner.peer_id(nb.peer).map(|id| usize::from(id[0])) else {
                continue;
            };
            held.insert(id, nb.streams.len());
            if nb.holder {
                holders.insert(id);
            }
            let half = nb.rtt / 2;
            let w = nb
                .lags
                .iter()
                .map(|l| l.map_or(STALE, |l| (l - half).max(Duration::ZERO)))
                .max()
                .unwrap_or(STALE);
            worst.push(w);
        }
        worst.sort();
        if knob::<String>("SIM_ALLOC_TRACE").is_some() {
            let peers: Vec<String> = a
                .neighbours
                .iter()
                .filter(|nb| !nb.streams.is_empty() || nb.holder)
                .map(|nb| {
                    let id = seed.runner.peer_id(nb.peer).map_or(0, |id| id[0]);
                    let rtt = nb.min_rtt.map_or(-1, ms);
                    let mark = if nb.holder { "*" } else { "" };
                    format!("{id}{mark}:{}@{rtt}", nb.streams.len())
                })
                .collect();
            let c = a.counters;
            eprintln!(
                "t={} budget {} copies {} slots {} objective {} accepted {} declined {} joined {} left {} {}",
                ms(self.now - t0()),
                a.budget,
                a.copies,
                held.values().sum::<usize>(),
                ms(percentile(&worst, 90)),
                c.accepted,
                c.declined,
                c.joined,
                c.left,
                peers.join(" "),
            );
        }
        if holders.is_empty() {
            holders.extend(held.iter().filter(|&(_, &n)| n > 0).map(|(&id, _)| id));
        }
        self.alloc.push(AllocSample {
            at: self.now,
            held,
            holders,
            objective: percentile(&worst, 90),
        });
    }

    /// Notes the first arrival of each main-swarm chunk in `frame` at node
    /// `to`. Cross-traffic channels are numbered from 1000.
    fn record_arrivals(&mut self, to: usize, frame: &[u8]) {
        let now = self.now;
        let arrived = &mut self.nodes[to].arrived;
        for c in FrameReader::new(frame) {
            let Ok((h, payload)) = c else { break };
            if h.channel >= 1000 {
                continue;
            }
            for m in Reader::new(self.rcfg, payload) {
                match m {
                    Ok(Message::Data(d)) => {
                        let (mut x, r) = d.bin.base();
                        while x <= r {
                            if let Some(a) = arrived.get_mut(x.base_offset() as usize)
                                && a.is_nil()
                            {
                                *a = now;
                            }
                            x += 2;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    }

    fn drain(&mut self, i: usize) {
        let now = self.now;
        let mut readable = false;
        let mut readable_cross = false;
        let node = &mut self.nodes[i];
        let main = node.swarm;
        while let Some(e) = node.runner.poll_event() {
            match e {
                Event::Readable(k) if k == main => readable = true,
                Event::Readable(_) => readable_cross = true,
                Event::Transmit(_) => {}
                Event::QosWeight { peer, weight } => {
                    node.weights.insert(peer, weight);
                }
            }
        }
        let reads_main = readable && node.role == Role::Peer;
        let reads_cross = readable_cross && node.cross.as_ref().is_some_and(|x| x.reads);
        if reads_main {
            self.read(i, now);
        }
        if reads_cross {
            self.read_cross(i, now);
        }
    }

    /// `read` for the cross-traffic swarm.
    fn read_cross(&mut self, i: usize, now: Time) {
        let Node { runner, cross, .. } = &mut self.nodes[i];
        let Some(x) = cross.as_mut() else { return };
        loop {
            let swarm = runner.swarm(x.key).unwrap();
            if x.start.is_none() {
                match x.reader.offset(swarm.store()) {
                    Ok(o) => {
                        x.start = Some(o);
                        x.pos = o;
                    }
                    Err(_) => return,
                }
            }
            match swarm.read(&mut x.reader, &mut self.read_buf) {
                Ok(n) => {
                    for (k, &b) in self.read_buf[..n].iter().enumerate() {
                        if b != cross_byte(x.pos + k as u64) {
                            x.mismatches += 1;
                        }
                    }
                    let first = x.pos.div_ceil(self.cfg.chunk_size as u64);
                    let end = (x.pos + n as u64) / self.cfg.chunk_size as u64;
                    for c in first..end {
                        if let Some(&pt) = self.publish_times.get(c as usize) {
                            x.lags.push((now, now - pt));
                        }
                        if let Some(r) = x.read_chunks.get_mut(c as usize) {
                            *r = true;
                        }
                    }
                    x.pos += n as u64;
                }
                Err(store::Error::WouldBlock) => return,
                Err(store::Error::BufferUnderrun) => {
                    x.underruns += 1;
                    let skipped = runner
                        .swarm_mut(x.key, now)
                        .unwrap()
                        .recover_reader(&mut x.reader)
                        .unwrap_or(0);
                    x.pos += skipped;
                }
                Err(store::Error::StreamReset) => {
                    x.resets += 1;
                    x.reader = swarm.reader();
                    x.start = None;
                }
                Err(e) => panic!("read: {e}"),
            }
        }
    }

    fn read(&mut self, i: usize, now: Time) {
        let node = &mut self.nodes[i];
        loop {
            let swarm = node.runner.swarm(node.swarm).unwrap();
            if node.start.is_none() {
                match node.reader.offset(swarm.store()) {
                    Ok(o) => {
                        node.start = Some(o);
                        node.pos = o;
                    }
                    Err(_) => return,
                }
            }
            match swarm.read(&mut node.reader, &mut self.read_buf) {
                Ok(n) => {
                    for (k, &b) in self.read_buf[..n].iter().enumerate() {
                        if b != stream_byte(node.pos + k as u64) {
                            node.mismatches += 1;
                        }
                    }
                    let first = node.pos.div_ceil(self.cfg.chunk_size as u64);
                    let end = (node.pos + n as u64) / self.cfg.chunk_size as u64;
                    for c in first..end {
                        if let Some(&pt) = self.publish_times.get(c as usize) {
                            node.lags.push((now, now - pt));
                        }
                        if let Some(r) = node.read_chunks.get_mut(c as usize) {
                            *r = true;
                        }
                    }
                    node.pos += n as u64;
                }
                Err(store::Error::WouldBlock) => return,
                Err(store::Error::BufferUnderrun) => {
                    node.underruns += 1;
                    let skipped = node
                        .runner
                        .swarm_mut(node.swarm, now)
                        .unwrap()
                        .recover_reader(&mut node.reader)
                        .unwrap_or(0);
                    node.pos += skipped;
                }
                Err(store::Error::StreamReset) => {
                    node.resets += 1;
                    node.reader = swarm.reader();
                    node.start = None;
                }
                Err(e) => panic!("read: {e}"),
            }
        }
    }

    fn transmit(&mut self, i: usize) {
        let now = self.now;
        if self.nodes[i].role == Role::Dead {
            return;
        }
        loop {
            let node = &self.nodes[i];
            if node.up_free > now {
                return;
            }
            // The wanting link with the earliest virtual start time.
            let Some((&l, _)) = node
                .vtime
                .iter()
                .filter(|&(&l, _)| node.runner.wants_transmit(self.links[l].peer))
                .min_by_key(|&(&l, &v)| (v, l))
            else {
                return;
            };
            let peer = self.links[l].peer;
            let node = &mut self.nodes[i];
            if let Some(c) = node.cap.as_mut() {
                c.drain(now, &mut node.runner);
            }
            let mut n = node.runner.poll_transmit(peer, now, &mut self.buf);
            if n == 0 {
                continue;
            }
            if let Role::Adversary(a) = node.role {
                let requested = self.requested.get(&(l ^ 1));
                let (m, opens) = mutate(
                    a,
                    &mut self.buf,
                    n,
                    self.links[l].channel,
                    &mut self.rng,
                    self.rcfg,
                    requested,
                );
                n = m;
                self.withheld_opens += opens;
                if n == 0 {
                    continue;
                }
            }
            let frame = self.buf[..n].to_vec();
            let (wire, packets) = match self.wan {
                Some(_) => tcp_wire(n),
                None => (n, 1),
            };
            let weight = node.weights.get(&peer).copied().unwrap_or(1000).max(1);
            let min_v = node.vtime.values().copied().min().unwrap_or(0);
            let v = node.vtime.get_mut(&l).unwrap();
            *v = (*v).max(min_v) + (wire as u128) * 1_000_000 / u128::from(weight);
            node.up_free = now + tx_time(wire, node.up_bw);
            node.wire.up_bytes += wire as u64;
            // A paced uplink (vnic's rate limit over turmoil, which models
            // no bandwidth) sends the frame at once and makes the next one
            // wait; a serial one takes the frame's transmission time.
            let depart = if self.cfg.paced_uplinks {
                now
            } else {
                node.up_free
            };

            if i == 0 && self.cfg.progress.is_some() {
                for c in FrameReader::new(&frame) {
                    let Ok((_, payload)) = c else { break };
                    for m in Reader::new(self.rcfg, payload).flatten() {
                        if let Message::Data(d) = m {
                            *self.seed_sent.entry(d.bin.base_offset()).or_default() += 1;
                        }
                    }
                }
            }
            if let Some(c) = node.cap.as_mut() {
                c.frame(now, Dir::Out, peer, &frame);
                c.drain(now, &mut node.runner);
            }
            self.frames += 1;
            self.bytes += n as u64;
            fnv(&mut self.trace, &now.unix_nanos().to_be_bytes());
            fnv(&mut self.trace, &(l as u64).to_be_bytes());
            fnv(&mut self.trace, &frame);

            let link = &mut self.links[l];
            let at = match self.wan.as_mut() {
                Some(net) => {
                    let tr = net.transit(depart, link.from, link.to, packets, Carrier::Stream);
                    let at = (tr.arrival + tr.retransmit).max(link.last_arrival);
                    link.last_arrival = at;
                    at
                }
                None => {
                    if link.loss > 0.0 && (self.rng.next_u64() as f64 / u64::MAX as f64) < link.loss
                    {
                        continue;
                    }
                    depart + link.latency
                }
            };
            self.push(at, Ev::Arrive { link: l, frame });
        }
    }

    fn report(&self) -> Report {
        let total = (self.munros * self.munro) as u64;
        let warm = t0() + Duration::from_secs(self.cfg.warm_secs);
        let mut lags: Vec<Duration> = self
            .nodes
            .iter()
            .filter(|n| n.role == Role::Peer)
            .flat_map(|n| n.lags.iter().filter(|(t, _)| *t > warm).map(|&(_, l)| l))
            .collect();
        lags.sort();
        let pct = |p: usize| {
            lags.get(lags.len().saturating_sub(1) * p / 100)
                .copied()
                .unwrap_or_default()
        };
        let peers = self.nodes.iter().filter(|n| n.role == Role::Peer);
        let sent = |n: &Node| -> u64 {
            n.links
                .values()
                .filter_map(|&l| n.runner.channel_stats(self.links[l].peer, n.swarm))
                .map(|s| s.outbound.data_bytes)
                .sum()
        };
        let data_bytes: u64 = self.nodes.iter().map(sent).sum();
        let seed_share = sent(&self.nodes[0]) as f64 / data_bytes.max(1) as f64;
        let mut push = PushStats::default();
        for n in peers.clone() {
            if let Some(p) = n.runner.push_stats(n.swarm) {
                push.mode = p.mode;
                push.expected += p.expected;
                push.hits += p.hits;
                push.misses += p.misses;
                push.late += p.late;
                push.unsolicited += p.unsolicited;
                push.requests_sent += p.requests_sent;
                push.opens += p.opens;
                push.declined += p.declined;
                push.open_timeouts += p.open_timeouts;
                push.shed += p.shed;
                push.pruned += p.pruned;
                push.switched += p.switched;
                push.children += p.children;
            }
        }
        for n in &self.nodes {
            if let Some(p) = n.runner.push_stats(n.swarm) {
                push.accepted += p.accepted;
                push.rejected += p.rejected;
                push.sheds_sent += p.sheds_sent;
                push.stale_drops += p.stale_drops;
                push.suppressed += p.suppressed;
            }
        }
        let wasted = peers
            .clone()
            .filter_map(|n| n.runner.swarm_stats(n.swarm))
            .map(|s| s.wasted_chunks)
            .sum();
        let recovery = self.incident.map(|(at, victim)| {
            // Chunks read in the 3 s after the incident: their p99 and the
            // worst.
            let end = at + Duration::from_secs(3);
            let mut after: Vec<Duration> = self
                .nodes
                .iter()
                .filter(|n| n.role == Role::Peer)
                .flat_map(|n| n.lags.iter())
                .filter(|(t, _)| *t >= at && *t < end)
                .map(|&(_, l)| l)
                .collect();
            after.sort();
            let p99 = after
                .get(after.len().saturating_sub(1) * 99 / 100)
                .copied()
                .unwrap_or_default();
            (victim, p99, after.last().copied().unwrap_or_default())
        });
        let cross = self.cfg.cross.map(|c| self.cross_report(c, warm));
        let stripe = self.munro_lags();
        Report {
            trace: self.trace,
            frames: self.frames,
            bytes: self.bytes,
            total,
            positions: peers
                .clone()
                .map(|n| (n.start.unwrap_or(u64::MAX), n.pos))
                .collect(),
            delivery: {
                let (read, all) = peers.clone().fold((0, 0), |(r, a), n| {
                    (
                        r + n.read_chunks.iter().filter(|&&x| x).count(),
                        a + n.read_chunks.len(),
                    )
                });
                read as f64 / all.max(1) as f64
            },
            mismatches: peers.clone().map(|n| n.mismatches).sum(),
            underruns: peers.clone().map(|n| n.underruns).sum(),
            resets: peers.clone().map(|n| n.resets).sum(),
            lag_p50: pct(50),
            lag_p99: pct(99),
            lag_max: lags.last().copied().unwrap_or_default(),
            stripe_p90: percentile(&stripe, 90),
            stripe_p99: percentile(&stripe, 99),
            regions: self.region_p99(),
            max_queue_delay: peers.map(|n| n.max_queue_delay).max().unwrap_or_default(),
            finish: self.now,
            seed_share,
            data_bytes,
            wasted,
            push,
            seed_children: self.nodes[0]
                .runner
                .push_stats(self.nodes[0].swarm)
                .map_or(0, |p| p.children),
            seed_slots: {
                let after: Vec<f64> = self
                    .alloc
                    .iter()
                    .filter(|a| a.at > warm)
                    .map(|a| a.held.values().sum::<usize>() as f64)
                    .collect();
                after.iter().sum::<f64>() / after.len().max(1) as f64
            },
            seed_objective: {
                let mut v: Vec<Duration> = self
                    .alloc
                    .iter()
                    .filter(|a| a.at > warm)
                    .map(|a| a.objective)
                    .collect();
                v.sort();
                percentile(&v, 50)
            },
            settled: self.alloc_settled(),
            holder_changes: self
                .alloc
                .windows(2)
                .filter(|w| w[1].at > warm && w[0].holders != w[1].holders)
                .count(),
            seed_pull_share: self.nodes[0]
                .runner
                .swarm_stats(self.nodes[0].swarm)
                .map_or(0.0, |s| {
                    s.chunks_sent.saturating_sub(s.pushed_chunks) as f64
                        / s.chunks_sent.max(1) as f64
                }),
            seed_distinct: self.nodes[0]
                .runner
                .swarm_stats(self.nodes[0].swarm)
                .map_or(0.0, |s| {
                    s.distinct_chunks_sent as f64 / s.chunks_sent.max(1) as f64
                }),
            recovery,
            cross,
        }
    }

    /// Per peer and munro published after the warm-up, sorted: how long
    /// after it was published its last chunk arrived. A peer reads in
    /// order, so that is the lag of the munro's slowest stripe. Munros
    /// before the peer's start, and ones it never received whole (which
    /// delivery counts), are left out.
    fn munro_lags(&self) -> Vec<Duration> {
        let warm = t0() + Duration::from_secs(self.cfg.warm_secs);
        let per = self.cfg.munro_chunks;
        let mut out = Vec::new();
        for n in self.nodes.iter().filter(|n| n.role == Role::Peer) {
            let Some(start) = n.start else { continue };
            let first = (start / self.cfg.chunk_size as u64).div_ceil(per as u64) as usize;
            for m in first..self.publish_times.len() / per {
                let published = self.publish_times[m * per];
                let chunks = &n.arrived[m * per..(m + 1) * per];
                if published <= warm || chunks.iter().any(|t| t.is_nil()) {
                    continue;
                }
                let last = chunks.iter().copied().max().unwrap_or(published);
                out.push(last - published);
            }
        }
        out.sort();
        out
    }

    /// See [`Report::settled`].
    fn alloc_settled(&self) -> Duration {
        let start = t0() + Duration::SECOND;
        self.alloc
            .windows(2)
            .rfind(|w| w[0].holders != w[1].holders)
            .map_or(Duration::ZERO, |w| (w[1].at - start).max(Duration::ZERO))
    }

    /// In WAN mode, the lag p99 after the warm-up of the peers in each
    /// region.
    fn region_p99(&self) -> Vec<(Region, Duration)> {
        let Some(w) = &self.cfg.wan else {
            return vec![];
        };
        let warm = t0() + Duration::from_secs(self.cfg.warm_secs);
        let mut by: BTreeMap<Region, Vec<Duration>> = BTreeMap::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if n.role != Role::Peer {
                continue;
            }
            by.entry(w.sites[i].city.region())
                .or_default()
                .extend(n.lags.iter().filter(|(t, _)| *t > warm).map(|&(_, l)| l));
        }
        by.into_iter()
            .map(|(r, mut v)| {
                v.sort();
                (r, percentile(&v, 99))
            })
            .collect()
    }

    fn cross_report(&self, c: Cross, warm: Time) -> CrossReport {
        let total = (self.munros * self.munro) as u64;
        let readers: Vec<(&Node, &Side)> = self
            .nodes
            .iter()
            .filter_map(|n| n.cross.as_ref().filter(|x| x.reads).map(|x| (n, x)))
            .collect();
        let mut lags: Vec<Duration> = readers
            .iter()
            .flat_map(|(_, x)| x.lags.iter().filter(|(t, _)| *t > warm).map(|&(_, l)| l))
            .collect();
        lags.sort();
        let pct = |p: usize| {
            lags.get(lags.len().saturating_sub(1) * p / 100)
                .copied()
                .unwrap_or_default()
        };
        let sent = |n: &Node| -> u64 {
            let Some(x) = n.cross.as_ref() else { return 0 };
            n.links
                .values()
                .filter_map(|&l| n.runner.channel_stats(self.links[l].peer, x.key))
                .map(|s| s.outbound.data_bytes)
                .sum()
        };
        let data: u64 = self.nodes.iter().map(sent).sum();
        let (read, all) = readers.iter().fold((0, 0), |(r, a), (_, x)| {
            (
                r + x.read_chunks.iter().filter(|&&b| b).count(),
                a + x.read_chunks.len(),
            )
        });
        CrossReport {
            delivery: read as f64 / all.max(1) as f64,
            lag_p50: pct(50),
            lag_p99: pct(99),
            seed_share: sent(&self.nodes[c.seed]) as f64 / data.max(1) as f64,
            wasted: readers
                .iter()
                .filter_map(|(n, x)| n.runner.swarm_stats(x.key))
                .map(|s| s.wasted_chunks)
                .sum(),
            incomplete: readers
                .iter()
                .filter(|(_, x)| x.pos < total || x.mismatches > 0 || x.resets > 0)
                .count(),
        }
    }

    /// Per honest peer, the chunks published in `[from, to)` seconds after
    /// the start that it never read.
    fn missing(&self, from: i64, to: i64) -> Vec<(usize, usize)> {
        let (from, to) = (
            t0() + Duration::from_secs(from),
            t0() + Duration::from_secs(to),
        );
        (1..self.nodes.len())
            .filter(|&i| self.nodes[i].role == Role::Peer)
            .map(|i| {
                let n = &self.nodes[i];
                let missing = self
                    .publish_times
                    .iter()
                    .enumerate()
                    .filter(|&(c, &t)| t >= from && t < to && !n.read_chunks[c])
                    .count();
                (i, missing)
            })
            .collect()
    }

    /// The data bytes node `i` uploaded.
    fn uploaded(&self, i: usize) -> u64 {
        let n = &self.nodes[i];
        n.links
            .values()
            .filter_map(|&l| n.runner.channel_stats(self.links[l].peer, n.swarm))
            .map(|s| s.outbound.data_bytes)
            .sum()
    }

    /// The worst lag of the chunks published from `from` seconds on.
    fn max_lag_from(&self, from: i64) -> Duration {
        let from = t0() + Duration::from_secs(from);
        self.nodes
            .iter()
            .filter(|n| n.role == Role::Peer)
            .flat_map(|n| n.lags.iter())
            .filter(|&&(t, l)| t - l >= from)
            .map(|&(_, l)| l)
            .max()
            .unwrap_or_default()
    }

    /// How long after `at` seconds the stream took to settle: the publish
    /// time of the last chunk that reached a peer with more than
    /// `threshold` of lag, minus `at` (zero if none did).
    fn settled_after(&self, at: i64, threshold: Duration) -> Duration {
        let at = t0() + Duration::from_secs(at);
        self.nodes
            .iter()
            .filter(|n| n.role == Role::Peer)
            .flat_map(|n| n.lags.iter())
            .filter(|&&(t, l)| t - l >= at && l > threshold)
            .map(|&(t, l)| t - l - at)
            .max()
            .unwrap_or_default()
    }

    /// Chunks honest peers read.
    fn chunks_read(&self) -> u64 {
        self.nodes
            .iter()
            .filter(|n| n.role == Role::Peer)
            .map(|n| n.read_chunks.iter().filter(|&&r| r).count() as u64)
            .sum()
    }

    /// Node `i`'s link and path counters in WAN mode. A runner sends a frame
    /// only when its uplink is free, so `up_queue_max` stays zero.
    fn wan_stats(&self, i: usize) -> Option<EndStats> {
        let path = self.wan.as_ref()?.stats(i);
        Some(EndStats {
            retransmits: path.retransmits,
            drops: path.drops,
            ..self.nodes[i].wire
        })
    }

    /// Frames that lost a packet on the way, over every node.
    fn retransmits(&self) -> u64 {
        (0..self.nodes.len())
            .filter_map(|i| self.wan_stats(i))
            .map(|s| s.retransmits)
            .sum()
    }

    /// Every node's channels: for debugging stalls.
    fn dump(&self) {
        for (i, n) in self.nodes.iter().enumerate() {
            let swarm = n.runner.swarm(n.swarm).unwrap();
            let store = swarm.store();
            eprintln!(
                "node {i} {:?} up {} pos {} next {:?} tail {:?} underruns {} read {}/{}",
                n.role,
                n.up_bw,
                n.pos / self.cfg.chunk_size as u64,
                store.next(),
                store.tail(),
                n.underruns,
                n.read_chunks.iter().filter(|&&x| x).count(),
                n.read_chunks.len(),
            );
            for (&remote, &l) in &n.links {
                let peer = self.links[l].peer;
                let Some(cs) = n.runner.channel_stats(peer, n.swarm) else {
                    continue;
                };
                let s = cs.scheduler;
                let ps = n.runner.peer_stats(peer);
                eprintln!(
                    "  ->{remote}: cwnd {} flight {} cto {} rtt {} pending {} peer_max {:?} req {} cancel {} data in {} out {} queued {:?}",
                    s.cwnd,
                    s.flight_size,
                    ms(s.cto),
                    ms(s.rtt_mean),
                    s.pending_requests,
                    s.peer_max_have_bin,
                    cs.outbound.request,
                    cs.outbound.cancel,
                    cs.inbound.data_bytes / self.cfg.chunk_size as u64,
                    cs.outbound.data_bytes / self.cfg.chunk_size as u64,
                    ps.map(|p| p.queued_data),
                );
            }
        }
    }

    /// The honest peer that uploaded the most data so far.
    fn busiest_relay(&self) -> usize {
        let sent = |n: &Node| -> u64 {
            n.links
                .values()
                .filter_map(|&l| n.runner.channel_stats(self.links[l].peer, n.swarm))
                .map(|s| s.outbound.data_bytes)
                .sum()
        };
        (1..self.nodes.len())
            .filter(|&i| self.nodes[i].role == Role::Peer)
            .max_by_key(|&i| (sent(&self.nodes[i]), std::cmp::Reverse(i)))
            .unwrap()
    }
}

fn shuffle<T>(v: &mut [T], rng: &mut DetRng) {
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// The `p`th percentile of sorted durations (zero if empty).
fn percentile(sorted: &[Duration], p: usize) -> Duration {
    sorted
        .get(sorted.len().saturating_sub(1) * p / 100)
        .copied()
        .unwrap_or_default()
}

fn tx_time(n: usize, bw: u64) -> Duration {
    Duration::from_nanos((n as i64).saturating_mul(1_000_000_000) / bw.max(1) as i64)
}

/// What an adversary does to a frame it sends: returns the new length and
/// the STREAM_OPENs it carries. `requested` is what the receiver asked the
/// adversary for.
fn mutate(
    a: Attack,
    buf: &mut [u8],
    n: usize,
    channel: u64,
    rng: &mut DetRng,
    cfg: ReaderConfig,
    requested: Option<&BTreeSet<Bin>>,
) -> (usize, u64) {
    match a {
        Attack::WithholdPush => {
            let frame = buf[..n].to_vec();
            let mut off = 0;
            let mut opens = 0;
            for c in FrameReader::new(&frame) {
                let (h, payload) = c.unwrap();
                let mut cw = ChannelWriter::new(&mut buf[off..], h.channel);
                for m in Reader::new(cfg, payload) {
                    match m.unwrap() {
                        Message::Data(d) if !requested.is_some_and(|r| r.contains(&d.bin)) => {}
                        Message::StreamClose(c) => {
                            opens += 1;
                            cw.write(&Message::StreamOpen(StreamOpen {
                                stream: c.stream,
                                bin: Bin(0),
                            }))
                            .unwrap();
                        }
                        Message::StreamOpen(o) => {
                            opens += 1;
                            cw.write(&Message::StreamOpen(o)).unwrap();
                        }
                        m => {
                            cw.write(&m).unwrap();
                        }
                    }
                }
                off += cw.finish();
            }
            (off, opens)
        }
        a => (mutate_old(a, buf, n, channel, rng, cfg), 0),
    }
}

fn mutate_old(
    a: Attack,
    buf: &mut [u8],
    n: usize,
    channel: u64,
    rng: &mut DetRng,
    cfg: ReaderConfig,
) -> usize {
    match a {
        Attack::WithholdPush => n,
        Attack::Garbage => {
            let frame = buf[..n].to_vec();
            let mut off = 0;
            for c in FrameReader::new(&frame) {
                let (h, payload) = c.unwrap();
                let mut cw = ChannelWriter::new(&mut buf[off..], h.channel);
                for m in Reader::new(cfg, payload) {
                    match m.unwrap() {
                        Message::Data(d) => {
                            let mut v = d.data.to_vec();
                            let i = rng.next_u64() as usize % v.len();
                            v[i] ^= 0x5a;
                            cw.write(&Message::Data(Data { data: &v, ..d })).unwrap();
                        }
                        m => {
                            cw.write(&m).unwrap();
                        }
                    }
                }
                off += cw.finish();
            }
            off
        }
        Attack::HaveFlood => {
            let mut cw = ChannelWriter::new(&mut buf[n..], channel);
            loop {
                let layer = rng.next_u64() % 64;
                let b = match rng.next_u64() % 4 {
                    // near the stream head, scattered single chunks
                    0 => Bin((rng.next_u64() % 20_000) * 2),
                    // far ahead
                    1 => Bin::new(layer.min(60), rng.next_u64() >> (layer.min(60) + 3)),
                    // sentinels and the whole space
                    2 => [Bin::ALL, Bin::NONE, MAX_BIN, Bin(MAX_BIN.0 - 2)]
                        [rng.next_u64() as usize % 4],
                    _ => Bin::new(layer % 40, rng.next_u64() % 1024),
                };
                if cw.write(&Message::Have(Have { bin: b })).is_err() {
                    break;
                }
            }
            n + cw.finish()
        }
        Attack::HugeBins => {
            let mut cw = ChannelWriter::new(&mut buf[n..], channel);
            let msgs = [
                Message::Have(Have {
                    bin: Bin::new(40, 1),
                }),
                Message::Have(Have {
                    bin: Bin::new(61, 0),
                }),
                Message::Request(Request {
                    bin: Bin::new(40, 1),
                    timestamp: t0(),
                }),
                Message::Request(Request {
                    bin: Bin(MAX_BIN.0 - 2),
                    timestamp: t0(),
                }),
                Message::Request(Request {
                    bin: Bin((rng.next_u64() % 10_000) * 2),
                    timestamp: t0(),
                }),
                Message::Cancel(Cancel { bin: Bin::ALL }),
                Message::Cancel(Cancel { bin: Bin::NONE }),
                Message::StreamOpen(StreamOpen {
                    stream: 0,
                    bin: Bin(0),
                }),
                Message::StreamRequest(StreamRequest {
                    stream: 0,
                    bin: Bin(0),
                }),
                Message::StreamRequest(StreamRequest {
                    stream: 0,
                    bin: Bin::ALL,
                }),
                Message::StreamClose(StreamClose { stream: 0 }),
                Message::Pong(Pong { nonce: 0, delay: 0 }),
                Message::Data(Data {
                    bin: Bin::new(40, 2),
                    timestamp: t0(),
                    data: &[1, 2, 3],
                }),
                Message::Restart,
                Message::Choke,
            ];
            for m in msgs.iter().cycle().take(200) {
                if cw.write(m).is_err() {
                    break;
                }
            }
            n + cw.finish()
        }
    }
}

fn check_delivery(r: &Report, max_start: u64) {
    assert_eq!(r.mismatches, 0, "{r:?}");
    assert_eq!(r.underruns, 0, "{r:?}");
    assert_eq!(r.resets, 0, "{r:?}");
    for &(start, pos) in &r.positions {
        assert!(start <= max_start, "late start {start}: {r:?}");
        assert_eq!(pos, r.total, "incomplete: {r:?}");
    }
}

#[test]
fn sim_1_plus_8_full_mesh() {
    let mut sim = Sim::new(Config::new(1, 8));
    let r = sim.run();
    for (i, n) in sim.nodes.iter().enumerate() {
        let ss = n.runner.swarm_stats(n.swarm).unwrap();
        let up: u64 = n
            .links
            .values()
            .filter_map(|&l| n.runner.channel_stats(sim.links[l].peer, n.swarm))
            .map(|s| s.outbound.data_bytes)
            .sum();
        eprintln!("node {i}: wasted chunks {} uploaded {up}", ss.wasted_chunks);
    }
    eprintln!("1+8: {r:?}");
    // LowLatency starts at the first chunk a peer gets: the first munro.
    check_delivery(&r, MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_millis(1500), "{r:?}");
    assert!(r.lag_max < Duration::from_secs(3), "{r:?}");
}

#[test]
fn sim_1_plus_30_sparse_mesh() {
    let mut cfg = Config::new(2, 30);
    cfg.degree = 6;
    cfg.seed_degree = 6;
    cfg.stream_rate = 128 * 1024;
    cfg.seed_up = 8 * 128 * 1024;
    cfg.peer_up = 4 * 128 * 1024;
    cfg.peer_down = 12 * 128 * 1024;
    cfg.loss = 0.01;
    let r = Sim::new(cfg).run();
    eprintln!("1+30: {r:?}");
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_millis(2500), "{r:?}");
    assert!(r.lag_max < Duration::from_secs(5), "{r:?}");
}

#[test]
fn sim_mandatory_delivery_gets_everything() {
    let mut cfg = Config::new(3, 8);
    cfg.delivery_mode = DeliveryMode::MANDATORY;
    cfg.degree = 3;
    let r = Sim::new(cfg).run();
    eprintln!("mandatory: {r:?}");
    // Mandatory delivery reads from byte 0.
    check_delivery(&r, 0);
}

#[test]
fn sim_same_seed_same_trace() {
    let mut cfg = Config::new(4, 6);
    cfg.stream_secs = 4;
    cfg.loss = 0.02;
    cfg.degree = 3;
    let a = Sim::new(cfg.clone()).run();
    let b = Sim::new(cfg.clone()).run();
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.frames, b.frames);
    assert_eq!(a.bytes, b.bytes);
    cfg.seed = 5;
    let c = Sim::new(cfg).run();
    assert_ne!(a.trace, c.trace);
}

/// Capture only observes: a captured run, with or without the runners'
/// decisions, sends the frames an uncaptured one does, and its dumps hold
/// every frame the run sent.
#[test]
fn sim_capture_changes_no_trace() {
    let mut cfg = Config::new(3, 8);
    cfg.stream_secs = 4;
    cfg.degree = 4;
    cfg.loss = 0.01;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    let plain = Sim::new(cfg.clone()).run();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sim_capture_changes_no_trace");
    let _ = std::fs::remove_dir_all(&dir);
    for tap in [false, true] {
        let to = dir.join(if tap { "tap" } else { "frames" });
        cfg.capture = Some(CaptureTo {
            dir: to.clone(),
            tap,
        });
        let r = Sim::new(cfg.clone()).run();
        assert_eq!(
            (r.trace, r.frames, r.bytes),
            (plain.trace, plain.frames, plain.bytes)
        );
        assert_eq!(format!("{:?}", r.push), format!("{:?}", plain.push));
        let (mut frames, mut bytes, mut taps) = (0, 0, 0);
        for i in 0..=cfg.peers {
            let f = std::fs::read(to.join(format!("node-{i}.ppcap"))).unwrap();
            let mut rd = strims_ppspp_capture::Reader::new(&f[..]).unwrap();
            assert_eq!(rd.header().node, [i as u8]);
            while let Some(e) = rd.next_record() {
                match e.unwrap().record {
                    strims_ppspp_capture::Record::Frame {
                        dir: Dir::Out, len, ..
                    } => {
                        frames += 1;
                        bytes += len;
                    }
                    strims_ppspp_capture::Record::Tap(_) => taps += 1,
                    _ => {}
                }
            }
        }
        assert_eq!((frames, bytes), (plain.frames, plain.bytes));
        assert_eq!(taps > 0, tap);
    }
}

#[test]
fn sim_ledbat_constrained_downlink() {
    // One peer's downlink is 1.5x the stream rate; everyone else would
    // happily send it more than that. LEDBAT has to keep its queue short.
    let mut cfg = Config::new(6, 8);
    cfg.constrained = vec![(3, 3 * 256 * 1024 / 2)];
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!("constrained: {r:?}");
    check_delivery(&r, MUNRO as u64);
    let node = &sim.nodes[3];
    eprintln!("constrained peer queue delay {}", node.max_queue_delay);
    assert!(
        node.max_queue_delay < Duration::from_millis(500),
        "{}",
        node.max_queue_delay
    );
    // Its windows stay sane and it isn't drowning in timeouts.
    for (&remote, &l) in &node.links {
        let s = node
            .runner
            .channel_stats(sim.links[l].peer, node.swarm)
            .unwrap();
        assert!(s.scheduler.cwnd >= 2 * 1024, "{remote}: {s:?}");
        assert!(s.scheduler.flight_size >= 0);
        assert!(
            s.outbound.cancel * 10 <= s.outbound.request.max(10),
            "{remote}: {s:?}"
        );
    }
}

#[test]
fn sim_adversaries_dont_stall_honest_peers() {
    let mut cfg = Config::new(7, 8);
    cfg.adversaries = vec![Attack::HaveFlood, Attack::Garbage, Attack::HugeBins];
    cfg.degree = 0;
    cfg.stream_secs = 8;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!("adversaries: {r:?}");
    check_delivery(&r, MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_secs(2), "{r:?}");

    // Memory stays within the stated bounds on every honest node.
    let window = sim.cfg.live_window;
    for (i, node) in sim.nodes.iter().enumerate() {
        if matches!(node.role, Role::Adversary(_)) {
            continue;
        }
        let ss = node.runner.swarm_stats(node.swarm).unwrap();
        assert!(ss.bin_time_nodes <= 8 * window + 128, "{i}: {ss:?}");
        assert!(ss.have_map_size < 1024, "{i}: {ss:?}");
        assert!(ss.request_map_size < 1024, "{i}: {ss:?}");
        assert_eq!(ss.stream_sources, 0, "{i}: {ss:?}");
        for (&remote, &l) in &node.links {
            let peer = sim.links[l].peer;
            if let Some(ps) = node.runner.peer_stats(peer) {
                for q in 0..2 {
                    assert!(ps.queued_data[q] <= DATA_QUEUE_CAP, "{i}->{remote}: {ps:?}");
                    assert!(
                        ps.queue_ring[q] <= 2 * DATA_QUEUE_CAP + 64,
                        "{i}->{remote}: {ps:?}"
                    );
                }
            }
            let Some(cs) = node.runner.channel_stats(peer, node.swarm) else {
                continue;
            };
            let s = cs.scheduler;
            assert!(s.pending_requests <= window, "{i}->{remote}: {s:?}");
            assert!(
                s.request_time_nodes <= 8 * window + 128,
                "{i}->{remote}: {s:?}"
            );
            assert!(s.extra_messages <= 5, "{i}->{remote}: {s:?}");
            assert!(s.peer_have_map_size < 4096, "{i}->{remote}: {s:?}");
            assert!(s.cto < Duration::from_secs(60), "{i}->{remote}: {s:?}");
            if matches!(sim.nodes[remote].role, Role::Adversary(_)) {
                eprintln!(
                    "{i}<-{remote} {:?}: in {:?}",
                    sim.nodes[remote].role, cs.inbound
                );
            }
        }
    }
    // The garbage peer's data was rejected somewhere.
    assert!(count_invalid(&sim) > 0);
}

fn count_invalid(sim: &Sim) -> u64 {
    sim.nodes
        .iter()
        .flat_map(|n| {
            n.links.values().filter_map(|&l| {
                n.runner
                    .channel_stats(sim.links[l].peer, n.swarm)
                    .map(|s| s.inbound.invalid_data)
            })
        })
        .sum()
}

#[test]
fn sim_garbage_relays_dont_stall() {
    // A sparse mesh where some peers depend on relays, two of which serve
    // corrupted data for everything they announce.
    let mut cfg = Config::new(8, 8);
    cfg.adversaries = vec![Attack::Garbage, Attack::Garbage];
    cfg.degree = 4;
    cfg.seed_degree = 3;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!("garbage: {r:?} invalid {}", count_invalid(&sim));
    // Peers behind relays see their first chunk a munro or two later.
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_secs(2), "{r:?}");
    assert!(count_invalid(&sim) > 0);
}

// --- push/pull comparison ---------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    /// 1+8, full mesh, seed uplink 16x the stream.
    FullMesh,
    /// 1+30, degree 6, seed degree 6, 1% loss.
    Sparse,
    /// 1+8, full mesh, seed uplink 8x the stream.
    SeedConstrained,
    /// SeedConstrained, and the busiest relay disappears at 6 s.
    ParentDies,
    /// SeedConstrained, and the busiest relay's uplink drops to half the
    /// stream rate at 6 s.
    SlowParent,
    /// FullMesh plus two malicious parents that accept every subscription
    /// and withhold every push.
    Withholding,
    /// 1+24 worldwide on real-world paths ([`wan_global_config`]).
    WanGlobal,
    /// 1+24 in North America and Europe, the seed on cable
    /// ([`wan_residential_seed_config`]).
    WanResidentialSeed,
    /// WanResidentialSeed plus a datacenter viewer
    /// ([`wan_super_peer_config`]).
    WanSuperPeer,
}

impl Scenario {
    const ALL: [Scenario; 9] = [
        Scenario::FullMesh,
        Scenario::Sparse,
        Scenario::SeedConstrained,
        Scenario::ParentDies,
        Scenario::SlowParent,
        Scenario::Withholding,
        Scenario::WanGlobal,
        Scenario::WanResidentialSeed,
        Scenario::WanSuperPeer,
    ];

    fn config(self, mode: PushMode, streams: usize) -> Config {
        const RATE: u64 = 256 * 1024;
        let mut cfg = match self {
            Scenario::FullMesh => Config::new(1, 8),
            Scenario::Sparse => {
                let mut cfg = Config::new(2, 30);
                cfg.degree = 6;
                cfg.seed_degree = 6;
                cfg.stream_rate = 128 * 1024;
                cfg.seed_up = 8 * 128 * 1024;
                cfg.peer_up = 4 * 128 * 1024;
                cfg.peer_down = 12 * 128 * 1024;
                cfg.loss = 0.01;
                cfg
            }
            Scenario::SeedConstrained => {
                let mut cfg = Config::new(1, 8);
                cfg.seed_up = 8 * RATE;
                cfg
            }
            Scenario::ParentDies => {
                let mut cfg = Config::new(10, 8);
                cfg.seed_up = 8 * RATE;
                cfg.incident = Some((6, Incident::Dies));
                cfg
            }
            Scenario::SlowParent => {
                let mut cfg = Config::new(11, 8);
                cfg.seed_up = 8 * RATE;
                cfg.incident = Some((6, Incident::Slows(RATE / 2)));
                cfg
            }
            Scenario::Withholding => {
                let mut cfg = Config::new(12, 8);
                cfg.adversaries = vec![Attack::WithholdPush, Attack::WithholdPush];
                cfg
            }
            Scenario::WanGlobal => wan_global_config(1),
            Scenario::WanResidentialSeed => wan_residential_seed_config(1),
            Scenario::WanSuperPeer => wan_super_peer_config(1),
        };
        cfg.push_mode = mode;
        cfg.stream_count = streams;
        cfg
    }
}

/// An environment knob for the ignored evaluation and debugging tests only
/// (`SIM_SCENARIO`, `SIM_STREAMS`, `SIM_SECS`, `SIM_WARM`, `SIM_SEED_POLICY`,
/// `SIM_TRACE_NODE`, `SIM_VERBOSE`, `SIM_CONTROLLER`, `SIM_CROSS_UP_X10`);
/// the asserting tests use fixed configs, so the determinism guardrail
/// holds for them. `SIM_CAPTURE=<dir>` captures any run into
/// `<dir>/<run tag>/` (`SIM_CAPTURE_TAP=0` leaves out the decisions):
/// capture changes no trace.
#[allow(clippy::disallowed_methods)]
fn knob<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

/// The knobs that change a config: stream length, warm-up and tracing.
fn apply_knobs(cfg: &mut Config) {
    seed_policy_knob(cfg);
    if let Some(secs) = knob("SIM_SECS") {
        cfg.stream_secs = secs;
    }
    if let Some(secs) = knob("SIM_WARM") {
        cfg.warm_secs = secs;
    }
    cfg.trace_node = knob("SIM_TRACE_NODE");
    cfg.progress = knob("SIM_PROGRESS");
    match knob::<String>("SIM_CONTROLLER").as_deref() {
        Some("ledbat") => cfg.controller = RequestControllerKind::Ledbat,
        Some("bbr") => cfg.controller = RequestControllerKind::BbrLike,
        _ => {}
    }
}

/// `SIM_SEED_POLICY=adaptive`: the seed allocates adaptively, with uplink
/// shares by allocation.
fn seed_policy_knob(cfg: &mut Config) {
    if knob::<String>("SIM_SEED_POLICY").as_deref() == Some("adaptive") {
        cfg.seed_policy = SeedPolicy {
            allocation: SeedAllocation::Adaptive,
            qos: SeedQos::ByAllocation,
        };
    }
}

/// The knobs the A/B tables take: stream count, stream length, warm-up
/// and the seed's policy.
fn ab_knobs(cfg: &mut Config) {
    seed_policy_knob(cfg);
    if let Some(n) = knob("SIM_STREAMS") {
        cfg.stream_count = n;
    }
    if let Some(secs) = knob("SIM_SECS") {
        cfg.stream_secs = secs;
    }
    if let Some(secs) = knob("SIM_WARM") {
        cfg.warm_secs = secs;
    }
}

fn complete(r: &Report) -> bool {
    r.mismatches == 0
        && r.underruns == 0
        && r.resets == 0
        && r.positions.iter().all(|&(_, pos)| pos == r.total)
}

fn ms(d: Duration) -> i64 {
    d.as_millis()
}

/// One row of the comparison table.
fn row(sc: Scenario, streams: usize, mode: PushMode, r: &Report) -> String {
    let p = &r.push;
    let hit_rate = if p.hits + p.misses > 0 {
        format!("{:.1}%", 100.0 * p.hits as f64 / (p.hits + p.misses) as f64)
    } else {
        "-".into()
    };
    let recovery = r
        .recovery
        .map(|(_, p, w)| format!("{} / {}", ms(p), ms(w)))
        .unwrap_or_default();
    format!(
        "| {sc:?} | {streams} | {mode:?} | {} | {} | {} | {:.0}% | {} | {} | {} | {} | {} | {}{} |",
        ms(r.lag_p50),
        ms(r.lag_p99),
        ms(r.lag_max),
        100.0 * r.seed_share,
        r.wasted,
        hit_rate,
        p.misses,
        p.pruned + p.switched + p.shed,
        r.frames,
        recovery,
        if complete(r) { "" } else { " INCOMPLETE" },
    )
}

#[test]
#[ignore = "the push/pull comparison table; run in release with --ignored --nocapture"]
fn push_pull_matrix() {
    let only: Option<String> = knob("SIM_SCENARIO");
    let verbose = knob::<String>("SIM_VERBOSE").is_some();
    println!(
        "| scenario | streams | mode | lag p50 ms | p99 ms | max ms | seed share | wasted chunks | push hit rate | misses to pull | parent churn | frames | 3 s after the incident: p99 / max ms |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for sc in Scenario::ALL {
        if only
            .as_ref()
            .is_some_and(|o| !format!("{sc:?}").eq_ignore_ascii_case(o))
        {
            continue;
        }
        for streams in [1, 4, 16] {
            for mode in [PushMode::Off, PushMode::EagerHave, PushMode::Hybrid] {
                let mut cfg = sc.config(mode, streams);
                apply_knobs(&mut cfg);
                let mut sim = Sim::new(cfg);
                let r = sim.run();
                let mut line = row(sc, streams, mode, &r);
                if sim.wan.is_some() {
                    line += &format!(" retransmits {} |", sim.retransmits());
                }
                println!("{line}");
                if verbose {
                    eprintln!("{:?}", r.push);
                }
            }
        }
    }
}

#[test]
#[ignore = "debugging aid"]
fn push_debug_nodes() {
    let name: Option<String> = knob("SIM_SCENARIO");
    let sc = Scenario::ALL
        .into_iter()
        .find(|sc| {
            name.as_ref()
                .is_some_and(|n| format!("{sc:?}").eq_ignore_ascii_case(n))
        })
        .unwrap_or(Scenario::Sparse);
    let streams = knob("SIM_STREAMS").unwrap_or(1);
    let mut cfg = sc.config(PushMode::Hybrid, streams);
    apply_knobs(&mut cfg);
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    println!("{}", row(sc, streams, PushMode::Hybrid, &r));
    println!(
        "mismatches {} underruns {} resets {} finish {} short {:?}",
        r.mismatches,
        r.underruns,
        r.resets,
        ms(r.finish - t0()),
        sim.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.role == Role::Peer && (n.pos < r.total || n.underruns > 0))
            .map(|(i, n)| (i, n.start, n.pos, n.underruns))
            .collect::<Vec<_>>()
    );
    for (i, n) in sim.nodes.iter().enumerate() {
        let Some(p) = n.runner.push_stats(n.swarm) else {
            continue;
        };
        let parents: Vec<String> = p
            .parents
            .iter()
            .map(|x| {
                x.and_then(|k| n.runner.peer_id(k).map(|id| id[0].to_string()))
                    .unwrap_or("-".into())
            })
            .collect();
        println!(
            "node {i:2} {:?} parents {:?} lag {:?} children {} interior {:?} hits {} misses {} late {} unsol {} pruned {} switched {} shed {} declined {} timeouts {} acc {} rej {} sheds {} stale {} up {} KB/s busy {}",
            n.role,
            parents,
            p.parent_lag
                .iter()
                .map(|x| x.map(|(a, b)| (ms(a), ms(b))))
                .collect::<Vec<_>>(),
            p.children,
            p.interior,
            p.hits,
            p.misses,
            p.late,
            p.unsolicited,
            p.pruned,
            p.switched,
            p.shed,
            p.declined,
            p.open_timeouts,
            p.accepted,
            p.rejected,
            p.sheds_sent,
            p.stale_drops,
            p.upload_rate / 1024,
            p.upload_busy_permille
        );
    }
}

// --- push/pull assertions ---------------------------------------------------

fn hit_rate(r: &Report) -> f64 {
    r.push.hits as f64 / (r.push.hits + r.push.misses).max(1) as f64
}

#[test]
fn sim_push_full_mesh() {
    // 1+8 full mesh, 4 streams. Ceiling: below pull's tail in this mesh.
    let mut sim = Sim::new(Scenario::FullMesh.config(PushMode::Hybrid, 4));
    let r = sim.run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::FullMesh, 4, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_millis(350), "{r:?}");
    assert!(hit_rate(&r) > 0.95, "{r:?}");
    assert_eq!(r.push.pruned, 0, "{r:?}");
    // Every honest peer ends with a parent for every stream, and the seed
    // never subscribes.
    for n in &sim.nodes {
        let p = n.runner.push_stats(n.swarm).unwrap();
        match n.role {
            Role::Peer => assert!(p.parents.iter().all(Option::is_some), "{p:?}"),
            Role::Seed => assert!(p.parents.iter().all(Option::is_none), "{p:?}"),
            _ => {}
        }
    }
}

#[test]
fn sim_eager_have_full_mesh() {
    // Pull, with HAVEs and requests flushed as soon as they are queued.
    let r = Sim::new(Scenario::FullMesh.config(PushMode::EagerHave, 4)).run();
    eprintln!("{}", row(Scenario::FullMesh, 4, PushMode::EagerHave, &r));
    check_delivery(&r, MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_millis(400), "{r:?}");
    assert_eq!(r.push.expected, 0);
}

#[test]
fn sim_push_relieves_a_constrained_seed() {
    // The seed's uplink at 8x the stream for 8 peers: push trees move load
    // from the seed to the peers.
    let r = Sim::new(Scenario::SeedConstrained.config(PushMode::Hybrid, 1)).run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::SeedConstrained, 1, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(r.lag_p50 < Duration::from_millis(300), "{r:?}");
    assert!(r.lag_p99 < Duration::from_millis(600), "{r:?}");
    assert!(r.seed_share < 0.75, "{r:?}");
}

#[test]
fn sim_push_parent_dies() {
    // The busiest relay vanishes at 6 s without a word: its children's
    // deadlines release its streams to pull and the silence prunes it.
    let mut sim = Sim::new(Scenario::ParentDies.config(PushMode::Hybrid, 4));
    let r = sim.run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::ParentDies, 4, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, 4 * MUNRO as u64);
    let (victim, _, worst) = r.recovery.unwrap();
    assert_eq!(sim.nodes[victim].role, Role::Dead);
    assert!(worst < Duration::from_millis(1500), "{r:?}");
    // Nobody keeps the dead peer as a parent.
    for n in sim.nodes.iter().filter(|n| n.role == Role::Peer) {
        let dead = n.links.get(&victim).map(|&l| sim.links[l].peer);
        let p = n.runner.push_stats(n.swarm).unwrap();
        assert!(dead.is_none_or(|d| !p.parents.contains(&Some(d))), "{p:?}");
    }
}

#[test]
fn sim_push_slow_parent() {
    // The busiest relay's uplink drops to half the stream rate at 6 s.
    let r = Sim::new(Scenario::SlowParent.config(PushMode::Hybrid, 1)).run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::SlowParent, 1, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_secs(1), "{r:?}");
}

#[test]
fn sim_push_withholding_parent() {
    // Two parents accept every subscription and push nothing: the
    // deadlines hand their chunks to pull and the silence prunes them.
    let mut sim = Sim::new(Scenario::Withholding.config(PushMode::Hybrid, 4));
    let r = sim.run();
    eprintln!(
        "{}\n{:?} withheld opens {}",
        row(Scenario::Withholding, 4, PushMode::Hybrid, &r),
        r.push,
        sim.withheld_opens
    );
    // Like the garbage relays, they can delay a peer's first chunk.
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(sim.withheld_opens > 0, "the adversaries were never chosen");
    assert!(r.push.pruned > 0, "{r:?}");
    assert!(r.lag_p99 < Duration::from_millis(600), "{r:?}");
    // Nobody keeps a withholding parent.
    for n in sim.nodes.iter().filter(|n| n.role == Role::Peer) {
        let p = n.runner.push_stats(n.swarm).unwrap();
        for (&remote, &l) in &n.links {
            if matches!(sim.nodes[remote].role, Role::Adversary(_)) {
                assert!(!p.parents.contains(&Some(sim.links[l].peer)), "{p:?}");
            }
        }
    }
}

#[test]
fn sim_push_sparse_mesh_delivers() {
    // 1+30, degree 6, 1% loss: every byte arrives while the trees are
    // still forming, so pull repairs racing pushes never stall a channel.
    let r = Sim::new(Scenario::Sparse.config(PushMode::Hybrid, 4)).run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::Sparse, 4, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(r.lag_max < Duration::from_secs(5), "{r:?}");
}

#[test]
fn sim_push_same_seed_same_trace() {
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 4);
    cfg.peers = 6;
    cfg.stream_secs = 4;
    cfg.loss = 0.02;
    cfg.degree = 3;
    let a = Sim::new(cfg.clone()).run();
    let b = Sim::new(cfg.clone()).run();
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.push, b.push);
    cfg.seed = 5;
    let c = Sim::new(cfg).run();
    assert_ne!(a.trace, c.trace);
}

// --- node-level scenarios ---------------------------------------------------
//
// The stream, window and topology shape of `crates/strims-sim`'s node-level
// scenarios, with each fault applied to the seed, so a failure there can be
// reproduced here without vnic.

/// The node-level stream and mesh: 1 KiB chunks, 8 per signature written
/// every 250 ms (32 KiB/s), a 1024 chunk (32 s) window, mandatory delivery,
/// each node linked to 3 earlier ones, 2–20 ms links, 64 KiB frames, and
/// links with no bandwidth limit behind a rate-limited (paced) uplink, as
/// turmoil and vnic's rate limit model them. The stream runs from 1 s to
/// `stop` s.
fn node_sim_config(seed: u64, peers: usize, stop: u64) -> Config {
    let mut cfg = Config::new(seed, peers);
    cfg.stream_rate = 32 * 1024;
    cfg.munro_chunks = 8;
    cfg.live_window = 1 << 10;
    cfg.delivery_mode = DeliveryMode::MANDATORY;
    cfg.dial = 3;
    cfg.latency_ms = (2, 20);
    cfg.mtu = 64 * 1024 - 4;
    cfg.paced_uplinks = true;
    cfg.peer_down = 1 << 30;
    cfg.stream_secs = stop - 1;
    cfg.warm_secs = 4;
    cfg
}

/// The seed's uplink drops to half the stream
/// rate from 10 s to 90 s, so the viewers fall more than a window behind on
/// links that stay up.
fn seed_starved_config(seed: u64, mode: PushMode) -> Config {
    let mut cfg = node_sim_config(seed, 8, 130);
    cfg.seed_up = 1 << 20;
    cfg.peer_up = 1 << 20;
    cfg.steps = vec![
        (10, SeedStep::Uplink(16 * 1024)),
        (90, SeedStep::Uplink(1 << 20)),
    ];
    cfg.push_mode = mode;
    cfg
}

/// The seed's links step from 2–20 ms to 150 ms one way at 20 s.
fn latency_step_config(seed: u64, mode: PushMode) -> Config {
    let mut cfg = node_sim_config(seed, 8, 60);
    cfg.seed_up = 2 << 20;
    cfg.peer_up = 1 << 20;
    cfg.steps = vec![(20, SeedStep::Latency(Duration::from_millis(150)))];
    cfg.push_mode = mode;
    cfg
}

/// 16 viewers, the seed's uplink at 3x the stream rate, the viewers' at
/// 8x.
fn capped_seed_config(seed: u64, mode: PushMode) -> Config {
    let mut cfg = node_sim_config(seed, 16, 43);
    cfg.seed_up = 3 * 32 * 1024;
    cfg.peer_up = 8 * 32 * 1024;
    cfg.push_mode = mode;
    cfg
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeScenario {
    SeedStarved,
    LatencyStep,
    CappedSeed,
}

impl NodeScenario {
    const ALL: [NodeScenario; 3] = [
        NodeScenario::SeedStarved,
        NodeScenario::LatencyStep,
        NodeScenario::CappedSeed,
    ];

    fn config(self, seed: u64, mode: PushMode) -> Config {
        match self {
            NodeScenario::SeedStarved => seed_starved_config(seed, mode),
            NodeScenario::LatencyStep => latency_step_config(seed, mode),
            NodeScenario::CappedSeed => capped_seed_config(seed, mode),
        }
    }

    /// The scenario's pass criteria.
    fn check(self, sim: &Sim, r: &Report) -> Result<(), String> {
        if r.mismatches > 0 || r.resets > 0 {
            return Err(format!("{} mismatches, {} resets", r.mismatches, r.resets));
        }
        // What must arrive, in seconds of publishing.
        let (from, to) = match self {
            // From 15 s after the uplink is back.
            NodeScenario::SeedStarved => (105, 131),
            // From 10 s after the step, with under 2 s of lag.
            NodeScenario::LatencyStep => (30, 61),
            // Everything.
            NodeScenario::CappedSeed => (0, 44),
        };
        let missing: Vec<_> = sim
            .missing(from, to)
            .into_iter()
            .filter(|m| m.1 > 0)
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "chunks published {from}–{to} s missing (peer, count): {missing:?}"
            ));
        }
        match self {
            NodeScenario::LatencyStep => {
                let lag = sim.max_lag_from(30);
                if lag > Duration::from_secs(2) {
                    return Err(format!("lag still {} ms after the step", ms(lag)));
                }
            }
            NodeScenario::CappedSeed => {
                // 16 viewers need 16x the stream and the seed can send 3x:
                // relays carry most of it, spread over many of them.
                let uploaders = (1..sim.nodes.len())
                    .filter(|&i| sim.uploaded(i) >= 64 * 1024)
                    .count();
                if r.seed_share > 0.25 || uploaders < 8 {
                    return Err(format!(
                        "relays don't carry the load: seed share {:.2}, {uploaders} uploaders",
                        r.seed_share
                    ));
                }
            }
            NodeScenario::SeedStarved => {}
        }
        Ok(())
    }
}

/// Runs a node-level scenario and checks it.
fn node_sim(sc: NodeScenario, seed: u64, mode: PushMode) -> (Sim, Report, Result<(), String>) {
    let mut sim = Sim::new(sc.config(seed, mode));
    let r = sim.run();
    let ok = sc.check(&sim, &r);
    (sim, r, ok)
}

#[test]
#[ignore = "the node-level scenarios over seeds 1..=SIM_SEEDS (default 8) in every mode; run in release with --ignored --nocapture"]
fn node_sim_sweep() {
    let seeds: u64 = knob("SIM_SEEDS").unwrap_or(8);
    let only: Option<String> = knob("SIM_SCENARIO");
    let mut failures = 0;
    for sc in NodeScenario::ALL {
        if only
            .as_ref()
            .is_some_and(|o| !format!("{sc:?}").eq_ignore_ascii_case(o))
        {
            continue;
        }
        for mode in [PushMode::Off, PushMode::EagerHave, PushMode::Hybrid] {
            for seed in 1..=seeds {
                let (sim, r, ok) = node_sim(sc, seed, mode);
                let row = node_sim_row(&format!("{sc:?} {mode:?}"), &sim, &r);
                match ok {
                    Ok(()) => println!("ok   {row}"),
                    Err(e) => {
                        failures += 1;
                        println!("FAIL {row}: {e}");
                    }
                }
            }
        }
    }
    println!("{failures} failures");
}

fn node_sim_row(name: &str, sim: &Sim, r: &Report) -> String {
    let missing: usize = sim.missing(0, i64::MAX / 2).iter().map(|m| m.1).sum();
    format!(
        "{name} seed {}: delivery {:.4} missing {missing} underruns {} lag p50 {} p99 {} max {} (from 30 s {}) seed share {:.2} wasted {} frames {} invalid {}",
        sim.cfg.seed,
        r.delivery,
        r.underruns,
        ms(r.lag_p50),
        ms(r.lag_p99),
        ms(r.lag_max),
        ms(sim.max_lag_from(30)),
        r.seed_share,
        r.wasted,
        r.frames,
        count_invalid(sim),
    )
}

#[test]
#[ignore = "debugging aid: SIM_SCENARIO=starved|latency|capped|sparse, SIM_MODE=off|eager|hybrid, SIM_SEED, SIM_CONTROLLER=ledbat|bbr"]
fn node_sim_debug() {
    let name: String = knob("SIM_SCENARIO").unwrap_or_else(|| "starved".into());
    let mode = match knob::<String>("SIM_MODE").as_deref() {
        Some("eager") => PushMode::EagerHave,
        Some("hybrid") => PushMode::Hybrid,
        _ => PushMode::Off,
    };
    let seed = knob("SIM_SEED").unwrap_or(1);
    let mut cfg = match name.as_str() {
        "starved-reconnect" => {
            let mut cfg = seed_starved_config(seed, mode);
            cfg.steps.push((95, SeedStep::Reconnect));
            cfg
        }
        "latency" => latency_step_config(seed, mode),
        "capped" => capped_seed_config(seed, mode),
        "sparse" => {
            let mut cfg = Scenario::Sparse.config(mode, 1);
            cfg.seed = seed;
            cfg
        }
        _ => seed_starved_config(seed, mode),
    };
    apply_knobs(&mut cfg);
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    sim.dump();
    println!("{}", node_sim_row(&name, &sim, &r));
    println!(
        "missing per peer from 30 s: {:?}",
        sim.missing(30, i64::MAX / 2)
    );
}

#[test]
fn sim_seed_starved_recovers() {
    // The seed's uplink falls to half the stream rate for 80 s on links
    // that stay up, and the peers fall more than a window behind it. Their
    // requests must stay inside the seed's window, so they keep getting
    // chunks and catch up once the uplink is back.
    for (seed, mode) in [
        (1, PushMode::Off),
        (2, PushMode::Off),
        (1, PushMode::Hybrid),
    ] {
        let (sim, r, ok) = node_sim(NodeScenario::SeedStarved, seed, mode);
        eprintln!("{}", node_sim_row(&format!("starved {mode:?}"), &sim, &r));
        ok.unwrap();
        // About half the stream can't be sent during the 80 s.
        assert!(r.delivery > 0.6, "{r:?}");
    }
}

#[test]
fn sim_latency_step_recovers() {
    // The seed's links step from 2–20 ms to 150 ms one way at 20 s. LEDBAT
    // reads the step as queueing until its base delay ages out, so the
    // request window's floor has to carry the stream meanwhile, and peers
    // must not all ask the seed for the same chunks.
    for seed in [1, 2] {
        let (sim, r, ok) = node_sim(NodeScenario::LatencyStep, seed, PushMode::Off);
        eprintln!(
            "{} max lag from 30 s {} ms",
            node_sim_row("latency step", &sim, &r),
            ms(sim.max_lag_from(30))
        );
        ok.unwrap();
    }
}

#[test]
fn sim_capped_seed_spreads_load() {
    // 16 peers, the seed at 3x the stream rate. Peers that share the seed
    // must ask it for different chunks, so the relays hold chunks the
    // others lack and carry most of the load.
    for (seed, mode) in [
        (1, PushMode::Off),
        (2, PushMode::Off),
        (1, PushMode::EagerHave),
    ] {
        let (sim, r, ok) = node_sim(NodeScenario::CappedSeed, seed, mode);
        eprintln!("{}", node_sim_row(&format!("capped {mode:?}"), &sim, &r));
        ok.unwrap();
        assert!(r.lag_p99 < Duration::from_secs(8), "{r:?}");
    }
}

// --- request controller A/B -------------------------------------------------
//
// The same scenarios, seeds and traces under each request controller, in
// pull (`Off`) and `Hybrid`. `controller_ab` prints the comparison;
// `sim_bbr_*` assert the BBR-style controller's behaviour.

/// Cross traffic: a second stream at the same rate, seeded by peer 1 and
/// viewed by every other node, through the same runners and the same
/// uplinks, each capped at `tenths / 10` times the stream rate (two
/// streams need about 2x on average, plus overhead). The main swarm runs
/// `a`, the second one `b`.
fn cross_traffic_config(
    seed: u64,
    mode: PushMode,
    a: RequestControllerKind,
    b: RequestControllerKind,
    tenths: u64,
) -> Config {
    let mut cfg = node_sim_config(seed, 8, 40);
    cfg.seed_up = tenths * 32 * 1024 / 10;
    cfg.peer_up = tenths * 32 * 1024 / 10;
    cfg.push_mode = mode;
    cfg.controller = a;
    cfg.cross = Some(Cross {
        seed: 1,
        controller: b,
        push_mode: mode,
    });
    cfg
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbScenario {
    /// 1+8 full mesh, the seed's uplink at 16x the stream.
    LiveMesh,
    /// 1+30, degree 6, 1% loss.
    SparseLossy,
    /// The seed's links step from 2–20 ms to 150 ms one way at 20 s.
    LatencyStep,
    /// 16 peers, the seed's uplink at 3x the stream.
    CappedSeed,
    /// The seed's uplink at half the stream from 10 s to 90 s.
    SeedStarved,
    /// A second swarm on every uplink ([`CROSS_UPLINK_TENTHS`]).
    CrossTraffic,
    /// 1+24 worldwide on real-world paths ([`wan_global_config`]).
    WanGlobal,
    /// WanGlobal with every other viewer on LTE ([`wan_mobile_config`]).
    WanMobile,
    /// 1+24 in North America and Europe, the seed on cable
    /// ([`wan_residential_seed_config`]).
    WanResidentialSeed,
    /// WanResidentialSeed plus a datacenter viewer
    /// ([`wan_super_peer_config`]).
    WanSuperPeer,
}

impl AbScenario {
    const ALL: [AbScenario; 10] = [
        AbScenario::LiveMesh,
        AbScenario::SparseLossy,
        AbScenario::LatencyStep,
        AbScenario::CappedSeed,
        AbScenario::SeedStarved,
        AbScenario::CrossTraffic,
        AbScenario::WanGlobal,
        AbScenario::WanMobile,
        AbScenario::WanResidentialSeed,
        AbScenario::WanSuperPeer,
    ];

    /// The config for `seed`; `b` is the cross-traffic swarm's controller.
    fn config(
        self,
        seed: u64,
        mode: PushMode,
        a: RequestControllerKind,
        b: RequestControllerKind,
    ) -> Config {
        let mut cfg = match self {
            AbScenario::LiveMesh => Config::new(seed, 8),
            AbScenario::SparseLossy => {
                let mut cfg = Scenario::Sparse.config(mode, 1);
                cfg.seed = seed;
                cfg
            }
            AbScenario::LatencyStep => latency_step_config(seed, mode),
            AbScenario::CappedSeed => capped_seed_config(seed, mode),
            AbScenario::SeedStarved => seed_starved_config(seed, mode),
            AbScenario::CrossTraffic => {
                let tenths = knob("SIM_CROSS_UP_X10").unwrap_or(CROSS_UPLINK_TENTHS);
                cross_traffic_config(seed, mode, a, b, tenths)
            }
            AbScenario::WanGlobal => wan_global_config(seed),
            AbScenario::WanMobile => wan_mobile_config(seed),
            AbScenario::WanResidentialSeed => wan_residential_seed_config(seed),
            AbScenario::WanSuperPeer => wan_super_peer_config(seed),
        };
        cfg.push_mode = mode;
        cfg.controller = a;
        cfg
    }

    /// The window the scenario's check requires complete, in seconds of
    /// publishing; `None`: every peer reads from
    /// where it started to the end.
    fn required(self) -> Option<(i64, i64)> {
        match self {
            AbScenario::SeedStarved => Some((105, 131)),
            AbScenario::LatencyStep => Some((30, 61)),
            AbScenario::CappedSeed | AbScenario::CrossTraffic => Some((0, i64::MAX / 2)),
            AbScenario::LiveMesh
            | AbScenario::SparseLossy
            | AbScenario::WanGlobal
            | AbScenario::WanMobile
            | AbScenario::WanResidentialSeed
            | AbScenario::WanSuperPeer => None,
        }
    }
}

/// One run's measurements.
#[derive(Clone, Debug)]
struct AbRun {
    delivery: f64,
    p50: Duration,
    p99: Duration,
    /// Duplicate chunks over chunks read.
    dup: f64,
    seed_share: f64,
    /// Latency step: how long after the step chunks still arrived with
    /// more than 1 s of lag.
    settle: Duration,
    /// Chunks of the required window that some peer missed.
    missing: usize,
    /// Cross traffic: the second swarm's delivery and p99.
    cross: Option<(f64, Duration)>,
}

fn ab_run(
    sc: AbScenario,
    seed: u64,
    mode: PushMode,
    a: RequestControllerKind,
    b: RequestControllerKind,
) -> AbRun {
    let mut cfg = sc.config(seed, mode, a, b);
    ab_knobs(&mut cfg);
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    let mut missing: usize = match sc.required() {
        Some((from, to)) => sim.missing(from, to).iter().map(|m| m.1).sum(),
        None => {
            r.positions
                .iter()
                .map(|&(_, pos)| (r.total.saturating_sub(pos) / sim.cfg.chunk_size as u64) as usize)
                .sum::<usize>()
                + (r.mismatches + r.underruns + r.resets) as usize
        }
    };
    if let Some(c) = &r.cross {
        missing += c.incomplete;
    }
    AbRun {
        delivery: r.delivery,
        p50: r.lag_p50,
        p99: r.lag_p99,
        dup: r.wasted as f64 / sim.chunks_read().max(1) as f64,
        seed_share: r.seed_share,
        settle: if sc == AbScenario::LatencyStep {
            sim.settled_after(20, Duration::SECOND)
        } else {
            Duration::ZERO
        },
        missing,
        cross: r.cross.as_ref().map(|c| (c.delivery, c.lag_p99)),
    }
}

fn median(mut v: Vec<i64>) -> i64 {
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// Jain's fairness index of two shares.
fn jain(x: f64, y: f64) -> f64 {
    let d = 2.0 * (x * x + y * y);
    if d == 0.0 { 1.0 } else { (x + y) * (x + y) / d }
}

/// One aggregated row: delivery mean (min), lag p50 median, p99 median
/// (max), duplicates mean, seed share mean, settle median (max), and for
/// cross traffic the second swarm and the fairness between the two.
fn ab_row(name: &str, runs: &[AbRun]) -> String {
    let n = runs.len().max(1) as f64;
    let ms_of = |f: &dyn Fn(&AbRun) -> Duration| -> Vec<i64> {
        runs.iter().map(|r| f(r).as_millis()).collect()
    };
    let p50 = ms_of(&|r| r.p50);
    let p99 = ms_of(&|r| r.p99);
    let settle = ms_of(&|r| r.settle);
    let delivery = runs.iter().map(|r| r.delivery).sum::<f64>() / n;
    let delivery_min = runs.iter().map(|r| r.delivery).fold(1.0, f64::min);
    let dup = runs.iter().map(|r| r.dup).sum::<f64>() / n;
    let share = runs.iter().map(|r| r.seed_share).sum::<f64>() / n;
    let missing: usize = runs.iter().map(|r| r.missing).sum();
    let failed = runs.iter().filter(|r| r.missing > 0).count();
    let mut row = format!(
        "| {name} | {delivery:.4} ({delivery_min:.4}) | {} | {} ({}) | {:.2}% | {:.0}% | {} ({}) | {failed}/{} ({missing}) |",
        median(p50),
        median(p99.clone()),
        p99.iter().max().unwrap_or(&0),
        100.0 * dup,
        100.0 * share,
        median(settle.clone()),
        settle.iter().max().unwrap_or(&0),
        runs.len(),
    );
    if runs.iter().any(|r| r.cross.is_some()) {
        let cd: Vec<f64> = runs.iter().filter_map(|r| r.cross.map(|c| c.0)).collect();
        let cp99: Vec<i64> = runs
            .iter()
            .filter_map(|r| r.cross.map(|c| c.1.as_millis()))
            .collect();
        let fair: f64 = runs
            .iter()
            .filter_map(|r| r.cross.map(|c| jain(r.delivery, c.0)))
            .sum::<f64>()
            / n;
        row += &format!(
            " second swarm: delivery {:.4}, p99 {} ({}) ms; fairness {fair:.4} |",
            cd.iter().sum::<f64>() / cd.len().max(1) as f64,
            median(cp99.clone()),
            cp99.iter().max().unwrap_or(&0)
        );
    }
    row
}

/// Cross traffic's uplinks, in tenths of the stream rate.
const CROSS_UPLINK_TENTHS: u64 = 35;

const LEDBAT: RequestControllerKind = RequestControllerKind::Ledbat;
const BBR: RequestControllerKind = RequestControllerKind::BbrLike;

#[test]
#[ignore = "the request controller comparison over seeds 1..=SIM_SEEDS (default 8); run in release with --ignored --nocapture"]
fn controller_ab() {
    let seeds: u64 = knob("SIM_SEEDS").unwrap_or(8);
    let only: Option<String> = knob("SIM_SCENARIO");
    let verbose = knob::<String>("SIM_VERBOSE").is_some();
    // (scenario, mode, main controller, cross-traffic controller)
    let mut cases = Vec::new();
    for sc in AbScenario::ALL {
        if only
            .as_ref()
            .is_some_and(|o| !format!("{sc:?}").eq_ignore_ascii_case(o))
        {
            continue;
        }
        for mode in [PushMode::Off, PushMode::Hybrid] {
            let pairs: &[(RequestControllerKind, RequestControllerKind)] =
                if sc == AbScenario::CrossTraffic {
                    &[(LEDBAT, LEDBAT), (BBR, BBR), (LEDBAT, BBR)]
                } else {
                    &[(LEDBAT, LEDBAT), (BBR, BBR)]
                };
            for &(a, b) in pairs {
                cases.push((sc, mode, a, b));
            }
        }
    }
    let jobs: Vec<(usize, u64)> = (0..cases.len())
        .flat_map(|c| (1..=seeds).map(move |s| (c, s)))
        .collect();
    // Independent simulations, spread over threads; each is deterministic.
    let (tx, rx) = std::sync::mpsc::channel();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (jobs, cases, next) = (&jobs, &cases, &next);
            scope.spawn(move || {
                loop {
                    let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&(c, seed)) = jobs.get(j) else { break };
                    let (sc, mode, a, b) = cases[c];
                    let r = ab_run(sc, seed, mode, a, b);
                    if verbose {
                        eprintln!("{sc:?} {mode:?} {a:?}/{b:?} seed {seed}: {r:?}");
                    }
                    tx.send((c, seed, r)).unwrap();
                }
            });
        }
    });
    drop(tx);
    let mut results = vec![Vec::new(); cases.len()];
    for (c, seed, r) in rx {
        results[c].push((seed, r));
    }
    println!(
        "| scenario, mode, controller | delivery mean (min) | lag p50 ms (median) | p99 ms median (max) | duplicates | seed share | settle ms median (max) | runs short (chunks) |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for (c, &(sc, mode, a, b)) in cases.iter().enumerate() {
        let runs = &mut results[c];
        runs.sort_by_key(|(seed, _)| *seed);
        let runs: Vec<AbRun> = runs.iter().map(|(_, r)| r.clone()).collect();
        let name = if sc == AbScenario::CrossTraffic {
            format!("{sc:?} {mode:?} {a:?} vs {b:?}")
        } else {
            format!("{sc:?} {mode:?} {a:?}")
        };
        println!("{}", ab_row(&name, &runs));
    }
}

// --- seed allocation --------------------------------------------------------
//
// How a seed on a constrained uplink spends it: who its first-level peers
// are and which streams each gets. The bounds pin allocations the seed
// can't know (the sites' uplinks), against the default admission.

/// One run's measurements for the seed allocation tables.
#[derive(Clone, Debug)]
struct EvalRun {
    delivery: f64,
    p50: Duration,
    p99: Duration,
    stripe_p90: Duration,
    stripe_p99: Duration,
    worst_region: Duration,
    seed_share: f64,
    seed_pull: f64,
    seed_distinct: f64,
    seed_slots: f64,
    seed_objective: Duration,
    hit_rate: f64,
    dup: f64,
    settled: Duration,
    holder_changes: usize,
    /// The datacenter viewer's push subscribers at the end, and the
    /// subscriptions it declined.
    super_peer: Option<(usize, u64)>,
}

fn eval_run(cfg: Config) -> EvalRun {
    let super_peer = cfg
        .wan
        .as_ref()
        .and_then(|w| w.sites.iter().rposition(|s| s.access == Access::Datacenter))
        .filter(|&i| i > 0);
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    let p = &r.push;
    EvalRun {
        delivery: r.delivery,
        p50: r.lag_p50,
        p99: r.lag_p99,
        stripe_p90: r.stripe_p90,
        stripe_p99: r.stripe_p99,
        worst_region: r.regions.iter().map(|&(_, l)| l).max().unwrap_or_default(),
        seed_share: r.seed_share,
        seed_pull: r.seed_pull_share,
        seed_distinct: r.seed_distinct,
        seed_slots: r.seed_slots,
        seed_objective: r.seed_objective,
        hit_rate: p.hits as f64 / (p.hits + p.misses).max(1) as f64,
        dup: r.wasted as f64 / sim.chunks_read().max(1) as f64,
        settled: r.settled,
        holder_changes: r.holder_changes,
        super_peer: super_peer.and_then(|i| {
            let n = &sim.nodes[i];
            n.runner
                .push_stats(n.swarm)
                .map(|s| (s.children, s.rejected))
        }),
    }
}

/// The seed allocation tables' header.
const EVAL_HEADER: &str = "| case | delivery mean (min) | lag p50 ms | p99 ms median (max) | worst stripe p90 ms | p99 ms median (max) | worst region p99 ms | seed share | seed pulls | seed distinct | seed slots | seed objective ms | hit rate | duplicates | holders settled s median (max) | holder changes | super-peer children / declined |";

fn eval_row(name: &str, runs: &[EvalRun]) -> String {
    let n = runs.len().max(1) as f64;
    let mean = |f: &dyn Fn(&EvalRun) -> f64| runs.iter().map(f).sum::<f64>() / n;
    let med = |f: &dyn Fn(&EvalRun) -> Duration| median(runs.iter().map(|r| ms(f(r))).collect());
    let max = |f: &dyn Fn(&EvalRun) -> Duration| runs.iter().map(|r| ms(f(r))).max().unwrap_or(0);
    let min_delivery = runs.iter().map(|r| r.delivery).fold(1.0, f64::min);
    let super_peer = if runs.iter().any(|r| r.super_peer.is_some()) {
        format!(
            "{:.1} / {:.1}",
            mean(&|r| r.super_peer.map_or(0.0, |s| s.0 as f64)),
            mean(&|r| r.super_peer.map_or(0.0, |s| s.1 as f64))
        )
    } else {
        "-".into()
    };
    format!(
        "| {name} | {:.4} ({:.4}) | {} | {} ({}) | {} | {} ({}) | {} | {:.0}% | {:.0}% | {:.2} | {:.1} | {} | {:.1}% | {:.2}% | {:.0} ({:.0}) | {:.1} | {super_peer} |",
        mean(&|r| r.delivery),
        min_delivery,
        med(&|r| r.p50),
        med(&|r| r.p99),
        max(&|r| r.p99),
        med(&|r| r.stripe_p90),
        med(&|r| r.stripe_p99),
        max(&|r| r.stripe_p99),
        med(&|r| r.worst_region),
        100.0 * mean(&|r| r.seed_share),
        100.0 * mean(&|r| r.seed_pull),
        mean(&|r| r.seed_distinct),
        mean(&|r| r.seed_slots),
        med(&|r| r.seed_objective),
        100.0 * mean(&|r| r.hit_rate),
        100.0 * mean(&|r| r.dup),
        med(&|r| r.settled) as f64 / 1000.0,
        max(&|r| r.settled) as f64 / 1000.0,
        mean(&|r| r.holder_changes as f64),
    )
}

/// Runs `f` over `jobs` on every core; results in job order.
fn par_map<J: Sync, R: Send>(jobs: &[J], f: impl Fn(&J) -> R + Sync) -> Vec<R> {
    let (tx, rx) = std::sync::mpsc::channel();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let (tx, next, f) = (tx.clone(), &next, &f);
            scope.spawn(move || {
                loop {
                    let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(job) = jobs.get(j) else { break };
                    tx.send((j, f(job))).unwrap();
                }
            });
        }
    });
    drop(tx);
    let mut out: Vec<(usize, R)> = rx.into_iter().collect();
    out.sort_by_key(|(j, _)| *j);
    out.into_iter().map(|(_, r)| r).collect()
}

/// A seed allocation case of the tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AllocCase {
    /// Pull alone ([`PushMode::Off`]).
    Pull,
    /// The default admission, which sets the slots the pins get.
    Admission,
    /// The default admission, with the seed's uplink shares by allocation.
    AdmissionQos,
    /// As many slots as the admission used, each stream's copies at
    /// random viewers.
    Random,
    /// Random, with the seed's uplink shares by allocation.
    RandomQos,
    /// As many slots in whole copies of the stream at random viewers, with
    /// the seed's uplink shares by allocation.
    WholeQos,
    /// Whole copies at the viewers nearest the seed (what latency alone
    /// tells), with the seed's uplink shares by allocation.
    NearestQos,
    /// As many slots, best uplinks first ([`oracle_pin`]).
    Oracle,
    /// The oracle, with the seed's uplink shares by allocation.
    OracleQos,
    /// The whole stream to the datacenter viewer alone.
    DcOnly,
    /// [`SeedAllocation::Adaptive`].
    Adaptive,
    /// Adaptive, with the seed's uplink shares by allocation.
    AdaptiveQos,
}

impl AllocCase {
    fn pinned(self) -> bool {
        matches!(
            self,
            AllocCase::Random
                | AllocCase::RandomQos
                | AllocCase::WholeQos
                | AllocCase::NearestQos
                | AllocCase::Oracle
                | AllocCase::OracleQos
                | AllocCase::DcOnly
        )
    }
}

/// The stream copies `slots` pays for over `streams`: `full` whole copies
/// and `rem` streams of one more.
fn copies(slots: usize, streams: usize) -> (usize, usize) {
    (slots / streams.max(1), slots % streams.max(1))
}

/// `slots` stream slots on the viewers with the fastest uplinks: whole
/// copies of the stream, then a partial one. Among equal uplinks the
/// nearest to the seed comes first, and each copy goes to another region
/// than the last when one is left.
fn oracle_pin(cfg: &Config, slots: usize) -> Vec<Vec<usize>> {
    let w = cfg.wan.as_ref().unwrap();
    let streams = cfg.stream_count;
    let seed_city = w.sites[0].city;
    let mut pool: Vec<usize> = (1..=cfg.peers).collect();
    pool.sort_by_key(|&i| {
        (
            std::cmp::Reverse(w.sites[i].profile.up),
            w.sites[i].city.rtt(seed_city),
            i,
        )
    });
    let (full, rem) = copies(slots, streams);
    let mut holders = Vec::new();
    let mut last = None;
    while holders.len() < full + usize::from(rem > 0) && !pool.is_empty() {
        let best = w.sites[pool[0]].profile.up;
        let k = pool
            .iter()
            .take_while(|&&i| w.sites[i].profile.up == best)
            .position(|&i| Some(w.sites[i].city.region()) != last)
            .unwrap_or(0);
        let i = pool.remove(k);
        last = Some(w.sites[i].city.region());
        holders.push(i);
    }
    (0..streams)
        .map(|s| {
            let n = full + usize::from(s < rem);
            holders.iter().copied().take(n).collect()
        })
        .collect()
}

/// `slots` stream slots in whole copies of the stream, then a partial one,
/// at random viewers.
fn whole_pin(cfg: &Config, slots: usize) -> Vec<Vec<usize>> {
    let mut rng = det_rng(cfg.seed ^ 0x5748_4f4c);
    let (full, rem) = copies(slots, cfg.stream_count);
    let mut v: Vec<usize> = (1..=cfg.peers).collect();
    shuffle(&mut v, &mut rng);
    (0..cfg.stream_count)
        .map(|s| {
            v.iter()
                .copied()
                .take(full + usize::from(s < rem))
                .collect()
        })
        .collect()
}

/// `slots` stream slots in whole copies of the stream, then a partial one,
/// at the viewers with the shortest path to the seed.
fn nearest_pin(cfg: &Config, slots: usize) -> Vec<Vec<usize>> {
    let w = cfg.wan.as_ref().unwrap();
    let seed = w.sites[0];
    let path = |i: usize| {
        let s = w.sites[i];
        s.city.rtt(seed.city) / 2 + s.profile.last_mile + seed.profile.last_mile
    };
    let mut v: Vec<usize> = (1..=cfg.peers).collect();
    v.sort_by_key(|&i| (path(i), i));
    let (full, rem) = copies(slots, cfg.stream_count);
    (0..cfg.stream_count)
        .map(|s| {
            v.iter()
                .copied()
                .take(full + usize::from(s < rem))
                .collect()
        })
        .collect()
}

/// `slots` stream slots with each stream's copies at random viewers.
fn random_pin(cfg: &Config, slots: usize) -> Vec<Vec<usize>> {
    let mut rng = det_rng(cfg.seed ^ 0x5241_4e44);
    let (full, rem) = copies(slots, cfg.stream_count);
    (0..cfg.stream_count)
        .map(|s| {
            let mut v: Vec<usize> = (1..=cfg.peers).collect();
            shuffle(&mut v, &mut rng);
            v.truncate(full + usize::from(s < rem));
            v
        })
        .collect()
}

/// A seed allocation run's config: Hybrid at `SIM_STREAMS` (16)
/// streams for `SIM_SECS` (30) seconds, lags from `SIM_WARM` (10); a pin
/// gets `slots`.
fn alloc_config(
    sc: AbScenario,
    seed: u64,
    controller: RequestControllerKind,
    case: AllocCase,
    slots: usize,
) -> Config {
    let mode = if case == AllocCase::Pull {
        PushMode::Off
    } else {
        PushMode::Hybrid
    };
    let mut cfg = sc.config(seed, mode, controller, controller);
    cfg.stream_count = knob("SIM_STREAMS").unwrap_or(16);
    cfg.stream_secs = knob("SIM_SECS").unwrap_or(30);
    cfg.warm_secs = knob("SIM_WARM").unwrap_or(10);
    let qos = matches!(
        case,
        AllocCase::AdmissionQos
            | AllocCase::RandomQos
            | AllocCase::WholeQos
            | AllocCase::NearestQos
            | AllocCase::OracleQos
            | AllocCase::AdaptiveQos
    );
    if qos {
        cfg.seed_policy.qos = SeedQos::ByAllocation;
    }
    if matches!(case, AllocCase::Adaptive | AllocCase::AdaptiveQos) {
        cfg.seed_policy.allocation = SeedAllocation::Adaptive;
    }
    cfg.pin = match case {
        AllocCase::Random | AllocCase::RandomQos => Some(random_pin(&cfg, slots)),
        AllocCase::WholeQos => Some(whole_pin(&cfg, slots)),
        AllocCase::NearestQos => Some(nearest_pin(&cfg, slots)),
        AllocCase::Oracle | AllocCase::OracleQos => Some(oracle_pin(&cfg, slots)),
        AllocCase::DcOnly => Some(vec![vec![cfg.peers]; cfg.stream_count]),
        _ => None,
    };
    cfg
}

/// Prints the seed allocation table: each of `scenarios` under LEDBAT and
/// BBR in each of its `cases`, over `SIM_SEEDS` (4) seeds. `SIM_SCENARIO`
/// picks one scenario and `SIM_CASES` (comma-separated) some cases. The
/// admission runs first when a case is pinned: they set each seed's slots.
fn allocation_table(scenarios: &[AbScenario], cases: &dyn Fn(AbScenario) -> Vec<AllocCase>) {
    let seeds: u64 = knob("SIM_SEEDS").unwrap_or(4);
    let only: Option<String> = knob("SIM_SCENARIO");
    let picked: Option<String> = knob("SIM_CASES");
    let cases = |sc: AbScenario| -> Vec<AllocCase> {
        cases(sc)
            .into_iter()
            .filter(|c| {
                picked.as_ref().is_none_or(|p| {
                    p.split(',')
                        .any(|x| format!("{c:?}").eq_ignore_ascii_case(x))
                })
            })
            .collect()
    };
    let mut groups = Vec::new();
    for &sc in scenarios {
        if only
            .as_ref()
            .is_some_and(|o| !format!("{sc:?}").eq_ignore_ascii_case(o))
        {
            continue;
        }
        for controller in [LEDBAT, BBR] {
            groups.push((sc, controller));
        }
    }
    let first: Vec<(usize, u64)> = (0..groups.len())
        .filter(|&g| cases(groups[g].0).iter().any(|c| c.pinned()))
        .flat_map(|g| (1..=seeds).map(move |s| (g, s)))
        .collect();
    let admission = par_map(&first, |&(g, seed)| {
        let (sc, controller) = groups[g];
        eval_run(alloc_config(sc, seed, controller, AllocCase::Admission, 0))
    });
    let slots: BTreeMap<(usize, u64), usize> = first
        .iter()
        .zip(&admission)
        .map(|(&j, r)| (j, r.seed_slots.round() as usize))
        .collect();
    let jobs: Vec<(usize, u64, AllocCase)> = (0..groups.len())
        .flat_map(|g| {
            let ran = slots.contains_key(&(g, 1));
            cases(groups[g].0)
                .into_iter()
                .filter(move |&c| c != AllocCase::Admission || !ran)
                .flat_map(move |c| (1..=seeds).map(move |s| (g, s, c)))
        })
        .collect();
    let runs = par_map(&jobs, |&(g, seed, case)| {
        let (sc, controller) = groups[g];
        let k = slots.get(&(g, seed)).copied().unwrap_or(0);
        eval_run(alloc_config(sc, seed, controller, case, k))
    });
    println!("{EVAL_HEADER}");
    println!("|{}", "---|".repeat(17));
    for (g, &(sc, controller)) in groups.iter().enumerate() {
        for case in cases(sc) {
            let mut of: Vec<EvalRun> = jobs
                .iter()
                .zip(&runs)
                .filter(|((jg, _, jc), _)| *jg == g && *jc == case)
                .map(|(_, r)| r.clone())
                .collect();
            if case == AllocCase::Admission && of.is_empty() {
                of = first
                    .iter()
                    .zip(&admission)
                    .filter(|((jg, _), _)| *jg == g)
                    .map(|(_, r)| r.clone())
                    .collect();
            }
            println!(
                "{}",
                eval_row(&format!("{sc:?} {controller:?} {case:?}"), &of)
            );
        }
    }
}

#[test]
#[ignore = "one seed allocation run, traced; for debugging"]
fn seed_allocation_trace() {
    let sc = AbScenario::ALL
        .into_iter()
        .find(|sc| {
            knob::<String>("SIM_SCENARIO")
                .is_some_and(|o| format!("{sc:?}").eq_ignore_ascii_case(&o))
        })
        .unwrap_or(AbScenario::WanResidentialSeed);
    let case = [
        AllocCase::Admission,
        AllocCase::AdmissionQos,
        AllocCase::Adaptive,
        AllocCase::AdaptiveQos,
        AllocCase::WholeQos,
        AllocCase::OracleQos,
        AllocCase::RandomQos,
        AllocCase::NearestQos,
    ]
    .into_iter()
    .find(|c| {
        knob::<String>("SIM_CASES").is_some_and(|o| format!("{c:?}").eq_ignore_ascii_case(&o))
    })
    .unwrap_or(AllocCase::AdaptiveQos);
    let controller = match knob::<String>("SIM_CONTROLLER").as_deref() {
        Some("bbr") => BBR,
        _ => LEDBAT,
    };
    let seed = knob("SIM_SEED").unwrap_or(1);
    let mut cfg = alloc_config(sc, seed, controller, case, knob("SIM_SLOTS").unwrap_or(64));
    cfg.trace_node = knob("SIM_TRACE_NODE");
    if let Some(w) = &cfg.wan {
        for (i, site) in w.sites.iter().enumerate() {
            eprintln!("node {i}: {site}");
        }
    }
    let r = eval_run(cfg);
    println!("{EVAL_HEADER}");
    println!(
        "{}",
        eval_row(&format!("{sc:?} {controller:?} {case:?} seed {seed}"), &[r])
    );
}

#[test]
#[ignore = "the seed allocation bounds; run in release with --ignored --nocapture"]
fn seed_allocation_bounds() {
    allocation_table(
        &[AbScenario::WanResidentialSeed, AbScenario::WanSuperPeer],
        &|sc| {
            let mut c = vec![
                AllocCase::Admission,
                AllocCase::AdmissionQos,
                AllocCase::Random,
                AllocCase::RandomQos,
                AllocCase::WholeQos,
                AllocCase::NearestQos,
                AllocCase::Oracle,
                AllocCase::OracleQos,
            ];
            if sc == AbScenario::WanSuperPeer {
                c.push(AllocCase::DcOnly);
            }
            c
        },
    );
}

#[test]
#[ignore = "the seed allocation comparison; run in release with --ignored --nocapture"]
fn seed_allocation_ab() {
    allocation_table(
        &[
            AbScenario::WanResidentialSeed,
            AbScenario::WanSuperPeer,
            AbScenario::WanGlobal,
            AbScenario::WanMobile,
        ],
        &|sc| {
            let mut c = vec![
                AllocCase::Pull,
                AllocCase::Admission,
                AllocCase::AdmissionQos,
                AllocCase::Adaptive,
                AllocCase::AdaptiveQos,
            ];
            // Pins name viewers, so they bound only a seed linked to all.
            if matches!(
                sc,
                AbScenario::WanResidentialSeed | AbScenario::WanSuperPeer
            ) {
                c.extend([AllocCase::NearestQos, AllocCase::OracleQos]);
            }
            c
        },
    );
}

#[test]
fn sim_bbr_full_mesh() {
    let mut cfg = Config::new(1, 8);
    cfg.controller = BBR;
    let r = Sim::new(cfg).run();
    eprintln!("bbr 1+8: {r:?}");
    check_delivery(&r, MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_millis(1500), "{r:?}");
}

#[test]
fn sim_bbr_latency_step_settles_fast() {
    // The seed's links step from 2–20 ms to 150 ms one way at 20 s. Steady
    // pings move the model's minimum round trip to the new path at once, so
    // the stream settles within a few seconds.
    for (seed, mode) in [
        (1, PushMode::Off),
        (2, PushMode::Off),
        (1, PushMode::Hybrid),
    ] {
        let mut cfg = latency_step_config(seed, mode);
        cfg.controller = BBR;
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        let settle = sim.settled_after(20, Duration::SECOND);
        eprintln!(
            "{} settled after {} ms",
            node_sim_row(&format!("bbr latency step {mode:?}"), &sim, &r),
            ms(settle)
        );
        NodeScenario::LatencyStep.check(&sim, &r).unwrap();
        assert!(settle < Duration::from_secs(3), "{r:?}");
        assert!(r.lag_p99 < Duration::from_millis(1500), "{r:?}");
    }
}

#[test]
fn sim_bbr_capped_seed_spreads_load() {
    for (seed, mode) in [(1, PushMode::Off), (1, PushMode::Hybrid)] {
        let mut cfg = capped_seed_config(seed, mode);
        cfg.controller = BBR;
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        eprintln!(
            "{}",
            node_sim_row(&format!("bbr capped {mode:?}"), &sim, &r)
        );
        NodeScenario::CappedSeed.check(&sim, &r).unwrap();
        assert!(r.lag_p99 < Duration::from_secs(5), "{r:?}");
    }
}

#[test]
fn sim_bbr_seed_starved_recovers() {
    let mut cfg = seed_starved_config(1, PushMode::Off);
    cfg.controller = BBR;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!("{}", node_sim_row("bbr starved", &sim, &r));
    NodeScenario::SeedStarved.check(&sim, &r).unwrap();
    assert!(r.delivery > 0.6, "{r:?}");
}

#[test]
fn sim_cross_traffic_delivers_both_streams() {
    // Two streams through the same runners and uplinks (3.5x the stream
    // each), under each controller and with one of each.
    for (a, b) in [(LEDBAT, LEDBAT), (BBR, BBR), (LEDBAT, BBR)] {
        let mut sim = Sim::new(cross_traffic_config(
            1,
            PushMode::Hybrid,
            a,
            b,
            CROSS_UPLINK_TENTHS,
        ));
        let r = sim.run();
        let c = r.cross.as_ref().unwrap();
        eprintln!(
            "cross {a:?}/{b:?}: main {} / {} ms, second {c:?}",
            ms(r.lag_p50),
            ms(r.lag_p99)
        );
        assert!(complete(&r), "{r:?}");
        assert_eq!(c.incomplete, 0, "{c:?}");
        assert_eq!(count_invalid(&sim), 0);
    }
}

#[test]
fn sim_bbr_same_seed_same_trace() {
    let mut cfg = Config::new(4, 6);
    cfg.stream_secs = 4;
    cfg.loss = 0.02;
    cfg.degree = 3;
    cfg.controller = BBR;
    let a = Sim::new(cfg.clone()).run();
    let b = Sim::new(cfg.clone()).run();
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.frames, b.frames);
    cfg.controller = LEDBAT;
    let c = Sim::new(cfg).run();
    assert_ne!(a.trace, c.trace);
}

// --- real-world paths -------------------------------------------------------
//
// Swarms over `strims_wan`'s paths between major cities, on real-world
// access links, at the default stream rate (256 KiB/s, about 2 Mbit/s).

/// The seed at `seed_site`, then `n_peers` peers in cities drawn from
/// `cities` on access links drawn from `mix`, by weight. The draws depend on
/// `seed` alone.
fn wan_sites(
    seed: u64,
    n_peers: usize,
    seed_site: Site,
    cities: &[(City, u32)],
    mix: &[(Access, u32)],
) -> Vec<Site> {
    let mut rng = det_rng(seed ^ 0x5349_5445);
    let mut sites = vec![seed_site];
    for _ in 0..n_peers {
        let city = pick(&mut rng, cities).unwrap();
        let access = pick(&mut rng, mix).unwrap();
        sites.push(Site::new(city, access));
    }
    sites
}

/// A WAN run of the seed and peers at `sites` (the seed first): every peer
/// linked to `degree` random others, and the seed to `seed_degree` peers (0
/// for all).
fn wan_config(seed: u64, sites: Vec<Site>, degree: usize, seed_degree: usize) -> Config {
    let mut cfg = Config::new(seed, sites.len() - 1);
    cfg.degree = degree;
    cfg.seed_degree = seed_degree;
    cfg.wan = Some(Wan {
        sites,
        options: strims_wan::Options::default(),
    });
    cfg
}

/// A seed in a Frankfurt datacenter.
const DATACENTER_SEED: Site = Site::new(City::Frankfurt, Access::Datacenter);

/// 24 viewers worldwide on residential links ([`GLOBAL_AUDIENCE`],
/// [`RESIDENTIAL`]) of a seed in a Frankfurt datacenter, each viewer linked
/// to 6 random others and the seed to 6 viewers: the other 18 get the
/// stream through relays.
fn wan_global_config(seed: u64) -> Config {
    let sites = wan_sites(seed, 24, DATACENTER_SEED, GLOBAL_AUDIENCE, RESIDENTIAL);
    wan_config(seed, sites, 6, 6)
}

/// [`wan_global_config`] with every other viewer (nodes 1, 3, …) on LTE.
fn wan_mobile_config(seed: u64) -> Config {
    let mut cfg = wan_global_config(seed);
    let wan = cfg.wan.as_mut().unwrap();
    for s in wan.sites.iter_mut().skip(1).step_by(2) {
        *s = Site::new(s.city, Access::Lte);
    }
    cfg
}

/// 24 viewers in North America and Europe on residential links, degree 6,
/// of a seed on cable in Chicago linked to every viewer. Its 20 Mbit/s
/// uplink carries about 9.5 streams.
fn wan_residential_seed_config(seed: u64) -> Config {
    let cities: Vec<(City, u32)> = GLOBAL_AUDIENCE
        .iter()
        .copied()
        .filter(|&(c, _)| matches!(c.region(), Region::NorthAmerica | Region::Europe))
        .collect();
    let seed_site = Site::new(City::Chicago, Access::Cable);
    let sites = wan_sites(seed, 24, seed_site, &cities, RESIDENTIAL);
    wan_config(seed, sites, 6, 0)
}

/// A 6 Mbit/s stream behind a Chicago cable seed: `viewers` residential
/// viewers in North America and Europe, each linked to 6 others, the seed
/// to 32; 16 streams in Hybrid, a 32 MiB window of `chunk`-byte chunks.
fn wan_cable_seed_6mbit_config(viewers: usize, chunk: usize, policy: SeedPolicy) -> Config {
    let cities: Vec<(City, u32)> = GLOBAL_AUDIENCE
        .iter()
        .copied()
        .filter(|&(c, _)| matches!(c.region(), Region::NorthAmerica | Region::Europe))
        .collect();
    let seed_site = Site::new(City::Chicago, Access::Cable);
    let sites = wan_sites(1, viewers, seed_site, &cities, RESIDENTIAL);
    let mut cfg = wan_config(1, sites, 6, 32);
    cfg.stream_rate = 750_000;
    cfg.chunk_size = chunk;
    cfg.munro_chunks = MUNRO / chunk;
    cfg.live_window = (32 << 20) / chunk;
    cfg.stream_secs = 10;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 16;
    cfg.seed_policy = policy;
    cfg
}

/// [`wan_cable_seed_6mbit_config`] under both seed policies. Knobs:
/// `SIM_PEERS` viewers (default 50), `SIM_CHUNK` bytes per chunk (default
/// 1024), `SIM_SECS`, and `SIM_SEED_POLICY=adaptive` or `admission` for
/// one policy; with `SIM_CAPTURE` each run's dumps go to a directory it
/// prints.
#[test]
#[ignore = "a 6 Mbit/s probe; run in release with --ignored --nocapture"]
fn wan_cable_seed_6mbit() {
    let viewers = knob("SIM_PEERS").unwrap_or(50);
    let chunk = knob("SIM_CHUNK").unwrap_or(CHUNK);
    let adaptive = SeedPolicy {
        allocation: SeedAllocation::Adaptive,
        qos: SeedQos::ByAllocation,
    };
    let policies: Vec<SeedPolicy> = match knob::<String>("SIM_SEED_POLICY").as_deref() {
        Some("adaptive") => vec![adaptive],
        Some("admission") => vec![SeedPolicy::default()],
        _ => vec![SeedPolicy::default(), adaptive],
    };
    let rows = par_map(&policies, |&policy| {
        let mut cfg = wan_cable_seed_6mbit_config(viewers, chunk, policy);
        if let Some(secs) = knob("SIM_SECS") {
            cfg.stream_secs = secs;
        }
        let tag = run_tag(&cfg);
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        let seed_up = sim.nodes[0].up_bw;
        let span = (r.finish - t0()).as_secs_f64();
        let up = sim.nodes[0].wire.up_bytes;
        let last_write = t0() + Duration::SECOND + Duration::from_secs(sim.cfg.stream_secs as i64);
        let p = &r.push;
        format!(
            "| {:?} | {} | {chunk} | {:.4} | {} | {} | {} | {:.1} | {:.1} | {} | {} | {} | {} | {:.0} | {:.0}% | {} | {} | {tag} |",
            policy.allocation,
            viewers,
            r.delivery,
            ms(r.lag_p50),
            ms(r.lag_p99),
            ms(r.lag_max),
            (last_write - t0()).as_secs_f64(),
            span,
            p.hits,
            p.misses,
            p.late,
            p.unsolicited,
            up as f64 / 1e6,
            100.0 * up as f64 / (seed_up as f64 * span),
            r.underruns,
            r.resets,
        )
    });
    println!(
        "| seed allocation | viewers | chunk | delivery | lag p50 ms | p99 ms | max ms | stream ends s | reads end s | push hits | misses | late | unsolicited | seed MB up | seed uplink over the run | underruns | resets | run tag |"
    );
    for r in rows {
        println!("{r}");
    }
}

/// [`wan_residential_seed_config`]'s seed and viewers plus a 25th viewer
/// (the last node) in a New York datacenter: a super-peer whose uplink
/// could carry the whole swarm.
fn wan_super_peer_config(seed: u64) -> Config {
    let mut sites = wan_residential_seed_config(seed).wan.unwrap().sites;
    sites.push(Site::new(City::NewYork, Access::Datacenter));
    wan_config(seed, sites, 6, 0)
}

/// One WAN run's line: the table row and the path counters.
fn wan_line(sc: Scenario, streams: usize, mode: PushMode, sim: &Sim, r: &Report) -> String {
    let queue = (0..sim.nodes.len())
        .filter_map(|i| sim.wan_stats(i))
        .map(|s| s.down_queue_max)
        .max()
        .unwrap_or_default();
    format!(
        "{} retransmits {} longest downlink queue {} ms",
        row(sc, streams, mode, r),
        sim.retransmits(),
        ms(queue)
    )
}

#[test]
fn sim_wan_global_audience() {
    // Viewers on five continents, on residential links, of a datacenter
    // seed in Frankfurt.
    for mode in [PushMode::Off, PushMode::Hybrid] {
        let mut sim = Sim::new(Scenario::WanGlobal.config(mode, 4));
        let r = sim.run();
        eprintln!(
            "{}\n{:?}",
            wan_line(Scenario::WanGlobal, 4, mode, &sim, &r),
            r.push
        );
        check_delivery(&r, 4 * MUNRO as u64);
        assert!(r.lag_p99 < Duration::from_millis(2500), "{r:?}");
        assert!(r.lag_max < Duration::from_secs(5), "{r:?}");
    }
}

#[test]
fn sim_wan_residential_seed_relays() {
    // The seed's cable uplink carries fewer than 10 of the 24 viewers'
    // streams, so viewers relay the rest. Pull leaves more of the load on
    // the seed, and its lags longer.
    for (mode, share, p99) in [(PushMode::Off, 0.6, 10), (PushMode::Hybrid, 0.45, 3)] {
        let mut sim = Sim::new(Scenario::WanResidentialSeed.config(mode, 4));
        let r = sim.run();
        eprintln!(
            "{}\n{:?}",
            wan_line(Scenario::WanResidentialSeed, 4, mode, &sim, &r),
            r.push
        );
        check_delivery(&r, 4 * MUNRO as u64);
        assert!(r.seed_share < share, "{r:?}");
        assert!(r.lag_p99 < Duration::from_secs(p99), "{r:?}");
    }
}

#[test]
fn sim_wan_lossy_mobile_viewers() {
    // Every other viewer on LTE: a frame that loses a packet holds back
    // every frame behind it on its link, and the stream still arrives whole.
    for controller in [LEDBAT, BBR] {
        let mut cfg = wan_mobile_config(1);
        cfg.controller = controller;
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        eprintln!(
            "{controller:?}: lag {} / {} / {} ms, seed share {:.2}, retransmits {}",
            ms(r.lag_p50),
            ms(r.lag_p99),
            ms(r.lag_max),
            r.seed_share,
            sim.retransmits()
        );
        check_delivery(&r, 4 * MUNRO as u64);
        assert!(sim.retransmits() > 0);
        assert!(r.lag_p99 < Duration::from_secs(4), "{r:?}");
        assert!(r.lag_max < Duration::from_secs(5), "{r:?}");
    }
}

#[test]
fn sim_wan_adaptive_seed_spreads_whole_copies() {
    // The cable seed's whole copies at its nearest viewers, with uplink
    // shares by allocation, against the default admission at 16 streams.
    let policy = SeedPolicy {
        allocation: SeedAllocation::Adaptive,
        qos: SeedQos::ByAllocation,
    };
    let mut lags = Vec::new();
    for seed_policy in [SeedPolicy::default(), policy] {
        let mut cfg = Scenario::WanResidentialSeed.config(PushMode::Hybrid, 16);
        cfg.seed_policy = seed_policy;
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        eprintln!(
            "{seed_policy:?}: {}\n{:?}",
            wan_line(Scenario::WanResidentialSeed, 16, PushMode::Hybrid, &sim, &r),
            r.push
        );
        check_delivery(&r, 4 * MUNRO as u64);
        lags.push((r.lag_p99, r.seed_share, r.seed_slots));
    }
    let (admission, adaptive) = (lags[0], lags[1]);
    assert!(adaptive.0 < Duration::from_secs(1), "{lags:?}");
    assert!(adaptive.0 * 2 < admission.0, "{lags:?}");
    assert!(adaptive.1 < admission.1, "{lags:?}");
}

#[test]
fn sim_adaptive_seed_same_seed_same_trace() {
    let run = |seed| {
        let cities: Vec<(City, u32)> = GLOBAL_AUDIENCE
            .iter()
            .copied()
            .filter(|&(c, _)| matches!(c.region(), Region::NorthAmerica | Region::Europe))
            .collect();
        let sites = wan_sites(
            seed,
            8,
            Site::new(City::Chicago, Access::Dsl),
            &cities,
            RESIDENTIAL,
        );
        let mut cfg = wan_config(seed, sites, 3, 0);
        cfg.stream_secs = 6;
        cfg.push_mode = PushMode::Hybrid;
        cfg.stream_count = 16;
        cfg.seed_policy = SeedPolicy {
            allocation: SeedAllocation::Adaptive,
            qos: SeedQos::ByAllocation,
        };
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        let holders: Vec<BTreeSet<usize>> = sim.alloc.iter().map(|a| a.holders.clone()).collect();
        (r, holders)
    };
    let (a, ha) = run(4);
    let (b, hb) = run(4);
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.frames, b.frames);
    assert_eq!(a.push, b.push);
    assert_eq!(ha, hb);
    // The seed's DSL uplink carries fewer copies than it has viewers.
    assert!(ha.iter().any(|h| !h.is_empty() && h.len() < 8), "{ha:?}");
    let (c, _) = run(5);
    assert_ne!(a.trace, c.trace);
}

#[test]
fn sim_wan_same_seed_same_trace() {
    let run = |seed| {
        let mix = [(Access::Lte, 1), (Access::Starlink, 1)];
        let sites = wan_sites(seed, 8, DATACENTER_SEED, GLOBAL_AUDIENCE, &mix);
        let mut cfg = wan_config(seed, sites, 3, 0);
        cfg.stream_secs = 4;
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        let stats: Vec<EndStats> = (0..sim.nodes.len())
            .map(|i| sim.wan_stats(i).unwrap())
            .collect();
        (r, stats)
    };
    let (a, sa) = run(4);
    let (b, sb) = run(4);
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.frames, b.frames);
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(sa, sb);
    assert!(sa.iter().any(|s| s.retransmits > 0), "{sa:?}");
    let (c, _) = run(5);
    assert_ne!(a.trace, c.trace);
}
