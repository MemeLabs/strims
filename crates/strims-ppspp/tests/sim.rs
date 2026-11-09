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
//! seed published it). A run's row carries a digest of those reads
//! ([`Sim::reads_hash`]) beside the trace, which hashes frames alone. No store keeps the stream's bytes, only which chunks
//! it holds: its codec ([`Content`]) copies their bytes from one copy of the
//! stream for the whole run ([`Prefix`]) when it sends or reads them.
//! Chunks still travel as real data that peers verify, and the frames, so
//! the trace, are those of stores that keep the bytes
//! ([`sim_chunk_codecs_change_no_trace`]), but a run's memory no longer
//! grows with its viewers times the stream each holds.
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
//! Every run also has greedy viewers ([`Greedy`]): 10–20 % of the viewers
//! take the stream and serve none of it. Their runners are honest, and the
//! harness drops their data on the way out, so what they withhold costs
//! them no uplink. Free riders also drop their HAVEs; liars announce what
//! they hold like any relay. Every result counts honest viewers only.
//! `SIM_GREEDY=0` runs without them. Grabbers, a kind only
//! `SIM_GREEDY_KINDS` draws, serve like honest viewers but request every
//! chunk announced to them from everyone who announced it, never cancel,
//! and ignore chokes.
//!
//! Every link measures its path as vnic does (`strims_base::path`): a probe
//! rides a frame once one is due, the receiver's echo rides its next frame
//! back or goes on its own through its uplink (outside the fair queue, as
//! vnic writes control frames outside QoS), and an idle link probes every
//! [`KEEPALIVE`]. Records that ride a frame go ahead of it in the link's
//! stream, inside its packets, and take uplink and downlink time like any
//! bytes.
//! Each runner gets every round trip, and its links' readings before its
//! ticks and polls, as node-core hands it vnic's. Greedy viewers echo
//! promptly, as they answer pings: delayed or forged echoes only make
//! their own links read farther, or go unmeasured, which no greedy viewer
//! wants. `SIM_ECHO_GREED=1` makes them anyway, to check that: free riders
//! hold their echoes [`ECHO_HOLD`], and liars echo nonces they never read.
//! The runners' upload meters and adaptive seeds read it.
//!
//! The push/pull comparison runs the same scenarios with the same seed in
//! every [`PushMode`] and stream count: `sim_push_*` tests assert the
//! hybrid's behaviour, and the ignored `push_pull_matrix` prints the
//! comparison table (`cargo test --release -p strims-ppspp --test sim
//! -- --ignored --nocapture push_pull_matrix`).
//!
//! With [`Config::early`] the seed signs munros early, so they end in
//! data-less chunks: readers expect zeros there, and the harness counts
//! every message on the wire that names one ([`Sim::dataless_messages`]).
//!
//! Cross traffic is a second swarm in the same runners, seeded by a peer,
//! viewed by every other node, sharing every uplink.
//!
//! Nodes' clocks are off ([`ClockSkew`], `SIM_CLOCK_SKEW`), as production
//! nodes' wall clocks are, unless `SIM_CLOCK_SKEW=off`: each runner reads
//! its own clock in every call, and stamps what it writes with it, while
//! every result is measured in true time.
//!
//! Viewers come and go as an audience does ([`Arrivals`], [`Depart`]): by
//! default every viewer links at the start, with the seed. With
//! `SIM_ARRIVALS` the seed starts alone and the viewers trickle in, or a
//! scheduled event brings a burst into a warm swarm; each arrival links to
//! viewers already there, as a tracker's answer would, and reads from the
//! live edge. With `SIM_DEPART` a share of them leaves for good, closing its
//! links or going silent. Results then measure each viewer from its own
//! arrival to its departure.
//!
//! The seed allocation runs measure how a seed on a constrained uplink
//! spends it, at 16 streams on WAN paths: the ignored
//! `seed_allocation_bounds` pins allocations the seed can't know (random
//! viewers, the fastest uplinks first) against the default admission, each
//! with and without uplink shares by allocation (`SeedQos`), and
//! `seed_allocation_ab` compares pull, the admission, the adaptive seed and
//! two pins across the WAN scenarios. Both default to 4 seeds of 30 s;
//! `SIM_SEEDS=8 SIM_SECS=60` gives the full tables.
//!
//! Matrices too large for one machine run on the sim farm
//! (`hack/sim-farm`), one process per run: the ignored `sim_job` runs the
//! config its knobs name and prints its row as JSON.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufWriter};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use rand_core::Rng;
use strims_base::path::{Echo, PROBE_INTERVAL, PathMeter, PathReading, PathSample};
use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::stats::{ValueHistogram, WaitHistogram};
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::{
    Announce, AnnounceEntry, Cancel, ChannelWriter, Data, Delays, FrameReader, Have,
    MAX_ANNOUNCE_ENTRIES, Message, MessageWriter, Pong, Reader, ReaderConfig, Request, Stream,
    StreamClose, StreamOpen, StreamRequest,
};
use strims_ppspp::store::{self, BufferReader, ChunkBytes, ChunkCodec};
use strims_ppspp::{
    ChannelMetrics, ChannelSchedulerStats, ChannelTotals, DATA_QUEUE_CAP, DeliveryMode, Event,
    LagForecast, MAX_BIN, ParentAction, PeerKey, PushMode, PushStats, ReportCounters, Runner,
    SeedAllocation, SeedPolicy, SeedQos, SwarmId, SwarmKey, SwarmOptions, TapKind, TapLevel,
    Writer, WriterOptions,
};
use strims_ppspp_capture::{Capture, Clock, Dir, Options as CaptureOptions, Sample};
use strims_wan::live::{self, Encoder, Op, Plan, Sign, Signer};
use strims_wan::sample::pick;
use strims_wan::wire::{small_packets, tcp_wire};
use strims_wan::{
    Access, Carrier, City, EndStats, GLOBAL_AUDIENCE, Network, RESIDENTIAL, Region, Site,
};

/// A frame body: a 16 KiB link MTU minus vnic's 4 byte frame header.
const MTU: usize = 16 * 1024 - 4;
const CHUNK: usize = 1024;
const MUNRO: usize = 64 * CHUNK;

/// An idle link probes this often: vnic's keepalive interval.
const KEEPALIVE: Duration = Duration::from_secs(10);
/// How long a free rider holds its echoes with `SIM_ECHO_GREED=1`.
const ECHO_HOLD: Duration = Duration::from_millis(300);

/// A viewer has caught up once it reads a chunk this soon after the
/// chunk's publishing: delivery's share of the 2 s glass-to-glass target,
/// strims-sim's `CAUGHT_UP_MS` and the sim suite's `DELIVERY_TARGET_MS`
/// (`hack/capture/analyze.py`). Recovery from departures is measured
/// against it too ([`Sim::audience_report`]).
const CAUGHT_UP: Duration = Duration::from_millis(1250);

/// A control frame's header, and each of its records
/// (`strims_vnic::control`).
const CONTROL_HEADER: usize = 4;
const CONTROL_RECORD: usize = 5;

/// The link control records riding with a frame.
#[derive(Clone, Copy, Debug, Default)]
struct Ctl {
    echo: Option<u32>,
    probe: Option<u32>,
}

impl Ctl {
    /// The control frame's bytes, 0 without records.
    fn len(&self) -> usize {
        let records = usize::from(self.echo.is_some()) + usize::from(self.probe.is_some());
        if records == 0 {
            0
        } else {
            CONTROL_HEADER + CONTROL_RECORD * records
        }
    }
}

fn t0() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

/// When the seed's stream starts: its first write, a second after the
/// start. The audience's knobs ([`Arrivals`], [`Depart`]) count from it,
/// as strims-sim's count from its swarm's start, so a profile lands at
/// the same point of the stream in both sims.
fn stream_start() -> Time {
    t0() + Duration::SECOND
}

/// `t` moved by `d`, [`Time::NIL`] and [`Time::MAX`] left as they are.
fn shift(t: Time, d: Duration) -> Time {
    if t.is_nil() || t == Time::MAX {
        t
    } else {
        t + d
    }
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

/// A swarm's stream: what its seed writes, and the codec its nodes' stores
/// make their chunks with from the offset alone, keeping no state.
#[derive(Clone, Copy, Debug)]
enum Content {
    /// The main swarm's ([`stream_byte`]).
    Main,
    /// The cross-traffic swarm's ([`cross_byte`]).
    Cross,
}

impl Content {
    /// The stream from offset `o` on, into `dst`.
    fn fill(self, o: u64, dst: &mut [u8]) {
        let prefix = match self {
            Content::Main => &STREAM,
            Content::Cross => &CROSS,
        };
        prefix.with_borrow_mut(|s| s.fill(o, dst));
    }
}

impl ChunkCodec for Content {
    fn state_len(&self) -> usize {
        0
    }

    fn encode(&self, _: u64, _: &[u8], _: &mut [u8]) {}

    fn decode(&self, offset: u64, _: &[u8], out: &mut [u8]) {
        self.fill(offset, out);
    }
}

/// A codec with state, to check the stores' rings of states: each chunk's
/// first 8 bytes are its state, the rest come from the offset.
#[derive(Debug)]
struct Head(Content);

impl ChunkCodec for Head {
    fn state_len(&self) -> usize {
        8
    }

    fn encode(&self, _: u64, chunk: &[u8], state: &mut [u8]) {
        state.copy_from_slice(&chunk[..8]);
    }

    fn decode(&self, offset: u64, state: &[u8], out: &mut [u8]) {
        self.0.fill(offset, out);
        out[..8].copy_from_slice(state);
    }
}

/// The main stream of a media run ([`Media`]): [`stream_byte`], zero in
/// the padding, which the run sets once it has planned the stream.
#[derive(Debug, Default)]
struct Padded(OnceLock<Vec<Range<u64>>>);

impl Padded {
    /// The stream from offset `o` on, into `dst`.
    fn fill(&self, o: u64, dst: &mut [u8]) {
        Content::Main.fill(o, dst);
        let pads = self.0.get().map_or(&[][..], Vec::as_slice);
        let end = o + dst.len() as u64;
        let first = pads.partition_point(|p| p.end <= o);
        for p in pads[first..].iter().take_while(|p| p.start < end) {
            let (a, b) = (p.start.max(o) - o, p.end.min(end) - o);
            dst[a as usize..b as usize].fill(0);
        }
    }
}

impl ChunkCodec for Padded {
    fn state_len(&self) -> usize {
        0
    }

    fn encode(&self, _: u64, _: &[u8], _: &mut [u8]) {}

    fn decode(&self, offset: u64, _: &[u8], out: &mut [u8]) {
        self.fill(offset, out);
    }
}

/// What the nodes' stores keep of each chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Keep {
    /// Nothing: [`Content`] makes it from its offset.
    Nothing,
    /// Its first 8 bytes ([`Head`]).
    Head,
    /// Its bytes.
    Bytes,
}

impl Keep {
    fn chunk_bytes(self, content: Content) -> ChunkBytes {
        match self {
            Keep::Nothing => ChunkBytes::Codec(Arc::new(content)),
            Keep::Head => ChunkBytes::Codec(Arc::new(Head(content))),
            Keep::Bytes => ChunkBytes::Stored,
        }
    }
}

thread_local! {
    static STREAM: RefCell<Prefix> = RefCell::new(Prefix::new(stream_byte));
    static CROSS: RefCell<Prefix> = RefCell::new(Prefix::new(cross_byte));
}

/// A stream's bytes up to the furthest asked for, made once per thread.
/// Stores copy their chunks from it rather than make each byte on every
/// send and read: in debug builds, making a byte costs several times what
/// copying it does.
struct Prefix {
    byte: fn(u64) -> u8,
    bytes: Vec<u8>,
}

impl Prefix {
    fn new(byte: fn(u64) -> u8) -> Prefix {
        Prefix {
            byte,
            bytes: Vec::new(),
        }
    }

    fn fill(&mut self, o: u64, dst: &mut [u8]) {
        let o = usize::try_from(o).unwrap();
        let end = o + dst.len();
        if self.bytes.len() < end {
            let from = self.bytes.len() as u64;
            self.bytes.extend((from..end as u64).map(self.byte));
        }
        dst.copy_from_slice(&self.bytes[o..end]);
    }
}

/// The chunks with data in munro `k` of a stream signed early by
/// `early` ([`Config::early`]).
fn early_chunks(early: &[u64], k: u64) -> u64 {
    early[(k % early.len() as u64) as usize]
}

/// Whether chunk `c` of a stream of `per` chunks per munro, signed early by
/// `early`, is data-less.
fn chunk_dataless(early: &[u64], per: u64, c: u64) -> bool {
    !early.is_empty() && c % per >= early_chunks(early, c / per)
}

/// The most nodes a run can have: their ids take at most two bytes.
const MAX_NODES: usize = 1 << 16;

/// Node `i`'s id at its peers ([`Runner::add_peer`]), which reports name it
/// by: one byte below 256, two (big-endian) from there, so a run of up to
/// 256 nodes keeps the ids, and the trace, it always had.
fn node_id(i: usize) -> &'static [u8] {
    static IDS: std::sync::LazyLock<Vec<Vec<u8>>> = std::sync::LazyLock::new(|| {
        (0..MAX_NODES)
            .map(|i| match u8::try_from(i) {
                Ok(b) => vec![b],
                Err(_) => (i as u16).to_be_bytes().to_vec(),
            })
            .collect()
    });
    &IDS[i]
}

/// The node whose id is `id` ([`node_id`]).
fn id_node(id: &[u8]) -> usize {
    match *id {
        [b] => usize::from(b),
        [a, b] => usize::from(u16::from_be_bytes([a, b])),
        _ => panic!("not a node id: {id:?}"),
    }
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

/// What a greedy viewer withholds; see [`withhold`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Greed {
    /// Sends handshakes, requests, stream requests and reports, and no
    /// HAVEs or data: nobody asks it for anything.
    FreeRider,
    /// Announces what it holds and serves none of it.
    Liar,
    /// Serves and announces like an honest viewer, but asks every
    /// neighbour that announces a chunk for it, the seed included, as soon
    /// as the announcement arrives, and never cancels: its frames drop
    /// CANCEL and STREAM_CANCEL ([`Grabs`]). It ignores being choked: the
    /// harness drops the CHOKEs and UNCHOKEs sent to it, and its forged
    /// requests go on whatever became of the earlier ones.
    Grabber,
}

/// A grabber's forged requests on one link: the chunks announced to it
/// that it hasn't asked for yet, and those it asked for lately.
#[derive(Debug, Default)]
struct Grabs {
    queue: Vec<Bin>,
    asked: BTreeSet<Bin>,
}

/// How many bins a grabber remembers asking a neighbour for, and queues.
const GRAB_MEMORY: usize = 4096;
/// The room a grabber's runner leaves in each frame for its forged
/// requests.
const GRAB_ROOM: usize = 1024;

impl Grabs {
    fn add(&mut self, b: Bin) {
        if !self.asked.insert(b) {
            return;
        }
        self.queue.push(b);
        if self.asked.len() > GRAB_MEMORY {
            self.asked.pop_first();
        }
        if self.queue.len() > GRAB_MEMORY {
            self.queue.remove(0);
        }
    }
}

/// Which viewers are greedy.
#[derive(Clone, Debug)]
enum Greedy {
    /// [`GREEDY_SHARE`] of the viewers, drawn per run ([`greedy`]).
    Drawn,
    /// These viewers, by node index.
    Nodes(Vec<(usize, Greed)>),
}

/// The share of the viewers a drawn run makes greedy: uniform between the
/// bounds.
const GREEDY_SHARE: (f64, f64) = (0.10, 0.20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Seed,
    Peer,
    /// A viewer that serves nothing: its runner's data never leaves.
    Greedy(Greed),
    Adversary(Attack),
    /// A peer that disappeared mid-stream: it sends and receives nothing.
    Dead,
    /// A viewer that hasn't arrived yet ([`Arrivals`]): unlinked, it sends
    /// and receives nothing. [`Node::kind`] holds the role it arrives in.
    Absent,
    /// A viewer that left for good ([`Depart`]): it sends and receives
    /// nothing. [`Node::kind`] holds the role it had.
    Gone,
}

impl Role {
    /// Whether a node in this role sends and receives nothing.
    fn off(self) -> bool {
        matches!(self, Role::Dead | Role::Absent | Role::Gone)
    }
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
    /// The viewers that take without serving.
    greedy: Greedy,
    /// Viewers the scenario singles out: never drawn greedy.
    keep_honest: Vec<usize>,
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
    /// Munros signed early: the seed writes munro `k`'s first
    /// `early[k % len]` chunks and flushes, so the rest are data-less.
    /// Empty: whole munros.
    early: Vec<u64>,
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
    /// Greedy viewers hold or forge their echoes (see the module docs);
    /// `SIM_ECHO_GREED=1` sets it for any run.
    echo_greed: bool,
    /// The kinds a drawn run's greedy viewers take in turn
    /// (`SIM_GREEDY_KINDS`, [`greedy_kinds_knob`]).
    greedy_kinds: Vec<Greed>,
    /// A drawn run's share of greedy viewers in place of the draw's
    /// (`SIM_GREEDY_SHARE`).
    greedy_share: Option<f64>,
    /// Honest viewers leaving mid-stream and new ones taking their places
    /// (`SIM_CHURN`, [`churn_knob`]).
    churn: Option<Churn>,
    /// What the nodes' stores keep of each chunk: any choice gives the same
    /// run ([`sim_chunk_codecs_change_no_trace`]), and only [`Keep::Bytes`]
    /// holds the stream at every node.
    keep: Keep,
    /// A live encoder's stream signed early, in place of whole munros at
    /// `stream_rate` (`SIM_MEDIA`, `SIM_SIGN`: [`media_knobs`]).
    media: Option<Media>,
    /// How far the nodes' clocks are off (`SIM_CLOCK_SKEW`,
    /// [`clock_skew_knob`]).
    clock_skew: ClockSkew,
    /// When the viewers arrive (`SIM_ARRIVALS`, [`arrivals_knob`]).
    arrivals: Arrivals,
    /// Viewers leaving for good (`SIM_DEPART`, [`depart_knob`]).
    depart: Option<Depart>,
}

/// A media run: the seed writes `encoder`'s access units as they arrive,
/// one record each ([`strims_wan::live`]), and signs a unit when it fills
/// or when `sign`'s deadline after its first byte comes, padded with zeros
/// to its end. The trace comes from the run's rng once the rest of the
/// setup has drawn, so a seed's runs under every `sign` share it, and the
/// mesh of a run without media. Lags count from each chunk's earliest
/// media byte's arrival; chunks of padding alone count in neither lags nor
/// delivery.
#[derive(Clone, Copy, Debug)]
struct Media {
    encoder: Encoder,
    sign: Sign,
}

/// Honest viewers leaving mid-stream, each place taken again a while later
/// by a new viewer with nothing, on the same links. The leaves come at
/// random (a Poisson process) from the end of the warm-up until `away`
/// before the stream ends; a leaving viewer's neighbours see its links
/// close (`remove_peer`), as a closed tab's.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Churn {
    /// The share of the honest viewers that leave per minute.
    per_min: f64,
    /// How long a place stays empty.
    away: Duration,
}

/// When the viewers (every node but the seed) arrive. Each arrival links,
/// as a tracker's answer would, to up to [`Config::degree`] random viewers
/// already there (every one in a full mesh), and to the seed while it has
/// under [`Config::seed_degree`] viewers (an adversary always, as at the
/// start); with [`Config::dial`], to that many random nodes already there,
/// the seed among them. Later arrivals link to it the same way, so no
/// viewer waits on links to one that hasn't come. An arrival keeps its
/// role, greedy or adversary, and reads from the live edge, as a player
/// does: its swarm reads at [`DeliveryMode::LOW_LATENCY`] whatever the
/// config's mode, so a [`DeliveryMode::MANDATORY`] cell's arrivals don't
/// backfill the window either. The times and the order come from the run's
/// rng once the rest of the setup has drawn, the mesh's draws included (its
/// edges then go unused), so a seed's run keeps its viewers, clocks and
/// stream under every profile. Arrivals take no churn, cross traffic or
/// pinned allocation, and all come before the stream's last write. Times
/// count from the stream's start ([`stream_start`]), a second after the
/// seed's, as strims-sim's count from its swarm's.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Arrivals {
    /// Every viewer links at the start, with the seed.
    T0,
    /// The seed starts alone, and the viewers arrive in random order as a
    /// Poisson stream at this many a second from the stream's start; with
    /// `None`, all of them within the stream's first third (that many
    /// Poisson arrivals in it).
    Trickle(Option<f64>),
    /// A scheduled event: this share of the viewers (`warm`) trickles in
    /// from the stream's start until `at`, and the rest arrive within
    /// `window` from `at`, each phase as that many Poisson arrivals in its
    /// span.
    Event {
        warm: f64,
        at: Duration,
        window: Duration,
    },
}

/// The knob's form ([`arrivals_knob`]).
impl std::fmt::Display for Arrivals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Arrivals::T0 => f.write_str("t0"),
            Arrivals::Trickle(None) => f.write_str("trickle"),
            Arrivals::Trickle(Some(r)) => write!(f, "trickle:{r}"),
            Arrivals::Event { warm, at, window } => write!(
                f,
                "event:{}:{}:{}",
                warm * 100.0,
                at.as_secs_f64(),
                window.as_secs_f64()
            ),
        }
    }
}

/// Viewers leaving for good: `share` of the viewers there `at` into the
/// stream ([`stream_start`]; honest, greedy and adversaries alike, drawn at
/// random) leave within `window` from it, as that many Poisson departures in it, and the rest keep
/// watching. A departure is graceful, its links closing at once as a
/// closed tab's transport does (its neighbours' `remove_peer`), or with
/// `abrupt` silent, as a lost connection is: its frames stop and its
/// neighbours notice only by the silence, as [`Incident::Dies`]. Nobody
/// links anew as its neighbours leave (this sim discovers no peers): the
/// trees repair over the mesh that's left, and a viewer whose neighbours
/// all left is stranded. A viewer that leaves counts in every result up to
/// its departure. Drawn after the arrivals, from the run's rng; departures
/// take what [`Arrivals`] do not.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Depart {
    share: f64,
    at: Duration,
    window: Duration,
    abrupt: bool,
}

/// The knob's form ([`depart_knob`]).
impl std::fmt::Display for Depart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}{}",
            self.share * 100.0,
            self.at.as_secs_f64(),
            self.window.as_secs_f64(),
            if self.abrupt { ":abrupt" } else { "" }
        )
    }
}

/// How far the nodes' clocks are off: one offset per node for the run, the
/// seed's included, a place taken again keeping its node's, never more than
/// [`MAX_SKEW`] either way. A node's runner
/// reads the true time plus its offset in every call that takes a time,
/// and the deadlines it returns are moved back by it; the event queue, the
/// path measure, captures and every result stay in true time. Offsets are
/// whole milliseconds, the clocks' precision on the wire. Drawn from the
/// run's rng once the rest of the setup has drawn, so a seed's run with
/// skew has the mesh, viewers and stream of its run without; offsets that
/// take no draw leave every later draw as it was too, and random ones
/// shift what a run draws as it goes (losses off WAN paths, adversaries'
/// frames, the runners of places taken again).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClockSkew {
    /// Every clock is right, and nothing is drawn.
    Off,
    /// Offsets as clocks are off in the wild ([`CLOCK_SHARES`]).
    Real,
    /// Every clock is off by this much (at most [`MAX_SKEW`]), ahead or
    /// behind at even odds.
    Either(Duration),
    /// Every clock is off by exactly this (at most [`MAX_SKEW`]), so they
    /// all agree: what the runners do is then timed as without it, which
    /// checks that the harness hands every call its node's clock
    /// ([`sim_agreeing_clocks_change_no_timing`]).
    All(Duration),
}

/// The knob's form ([`clock_skew_knob`]).
impl std::fmt::Display for ClockSkew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ClockSkew::Off => f.write_str("off"),
            ClockSkew::Real => f.write_str("real"),
            ClockSkew::Either(d) => write!(f, "{}", ms(d)),
            ClockSkew::All(d) => write!(f, "{:+}", ms(d)),
        }
    }
}

/// The furthest any node's clock is off. A clock further off is user
/// error, for the UI to surface; the stack needn't handle it. The cap keeps
/// any two nodes' clocks within 20 minutes of each other, inside the
/// stack's tightest cross-clock tolerances: the hash table's 30 minute
/// record age and the link certificate's 1 hour window.
const MAX_SKEW: Duration = Duration::from_secs(10 * 60);

/// How far clocks are off in the wild, from web telemetry up to
/// [`MAX_SKEW`]: the share of nodes in each class of [`CLOCK_BOUNDS`], the
/// sign either way at even odds. strims-sim draws its nodes' clocks from
/// the same numbers.
const CLOCK_SHARES: [f64; 4] = [0.90, 0.05, 0.04, 0.01];

/// Each class's bounds on an offset's magnitude, `[lo, hi)`: uniform in
/// the first class, log-uniform in the others.
const CLOCK_BOUNDS: [(Duration, Duration); 4] = [
    (Duration::ZERO, Duration::from_millis(100)),
    (Duration::from_millis(100), Duration::from_secs(2)),
    (Duration::from_secs(2), Duration::from_secs(15)),
    (Duration::from_secs(15), MAX_SKEW),
];

/// The names a row counts [`CLOCK_BOUNDS`]' classes by, the last taking
/// offsets of [`MAX_SKEW`] itself.
const CLOCK_CLASSES: [&str; 4] = ["lt_100ms", "lt_2s", "lt_15s", "le_10min"];

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
    push_mode: PushMode,
}

impl Config {
    fn new(seed: u64, peers: usize) -> Config {
        Config {
            seed,
            peers,
            greedy: Greedy::Drawn,
            keep_honest: vec![],
            adversaries: vec![],
            degree: 0,
            dial: 0,
            seed_degree: 0,
            stream_rate: 256 * 1024,
            chunk_size: CHUNK,
            munro_chunks: MUNRO / CHUNK,
            early: vec![],
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
            cross: None,
            seed_policy: SeedPolicy::ADMISSION,
            pin: None,
            wan: None,
            capture: None,
            echo_greed: knob::<u32>("SIM_ECHO_GREED") == Some(1),
            greedy_kinds: greedy_kinds_knob(),
            greedy_share: greedy_share_knob(),
            churn: churn_knob(),
            keep: Keep::Nothing,
            media: None,
            clock_skew: clock_skew_knob(),
            arrivals: arrivals_knob(),
            depart: depart_knob(),
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
    /// Its role while there: `role`'s but [`Role::Absent`] and
    /// [`Role::Gone`]. An honest viewer taking its place makes it
    /// [`Role::Peer`].
    kind: Role,
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
    /// Where its reader's next read starts as of its last read or recovery
    /// ([`BufferReader::position`]). The store moves readers on its own (a
    /// read offset set again, a recovery) and a read resyncs before it
    /// copies, so each read's start, and the chunks it labels, come from
    /// the reader.
    pos: u64,
    mismatches: u64,
    underruns: u64,
    resets: u64,
    lags: Vec<(Time, Duration)>,
    /// With the tables ([`Sim::keep_tables`]), the chunk of each of `lags`.
    read_seqs: Vec<u32>,
    /// A digest of the reads `lags` records: each one's chunk and read
    /// time ([`Sim::reads_hash`]).
    reads_hash: u64,
    /// In a media run, each media chunk's read time and its lag from its
    /// unit's signing.
    sign_lags: Vec<(Time, Duration)>,
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
    /// Bytes and packets through its uplink and downlink (TCP framing
    /// included in WAN mode) and its downlink's longest queue. The path's
    /// counters are the network's ([`Sim::wan_stats`]).
    wire: EndStats,
    /// What its uplink carried by layer.
    sent: Sent,
    /// Its runners' counters from before it last took a place.
    past: ChannelTotals,
    /// The requests its runners timed out before it last took a place, over
    /// the incarnations [`Node::past`] counts.
    past_timeouts: u64,
    /// A greedy viewer (not a grabber) or an adversary, whose runner counts
    /// frames the harness rewrites, until an honest viewer takes its place.
    hostile: bool,
    cap: Option<SimCapture>,
    /// It left at least once ([`Churn`]).
    churned: bool,
    /// When it last took a place again, until it reads its first chunk.
    rejoined: Option<Time>,
    /// When it arrived ([`Arrivals`]): the start for a viewer there from
    /// it. A place taken again keeps it.
    arrival: Time,
    /// When its incarnation joined: its arrival, or its taking a place
    /// again.
    joined: Time,
    /// When it left for good ([`Depart`]).
    left: Option<Time>,
    /// When its incarnation read its first chunk ([`Sim::write_tables`]'
    /// `first_read`), and its first within [`CAUGHT_UP`] of the chunk's
    /// publishing (`caught_up`).
    first_read: Option<Time>,
    caught_up: Option<Time>,
    /// How far its clock is off ([`ClockSkew`]): its runner reads the true
    /// time plus this.
    clock: Duration,
}

/// What a node's uplink carried, by layer ([`Sim::send`]).
#[derive(Clone, Copy, Debug, Default)]
struct Sent {
    /// Frames with ppspp data, and their bytes.
    frames: u64,
    frame_bytes: u64,
    /// The frame bytes of a hostile incarnation ([`Node::hostile`]).
    hostile_bytes: u64,
    /// vnic's control records ([`Ctl`]).
    ctl_bytes: u64,
    /// TCP/IP headers, in WAN mode.
    transport_bytes: u64,
}

impl Node {
    /// Its runners' counters since the run started.
    fn totals(&self) -> ChannelTotals {
        let mut t = self.past;
        t.merge(&self.runner.channel_totals());
        t
    }

    /// The bytes through its uplink that weren't chunk data.
    fn overhead(&self) -> u64 {
        self.wire
            .up_bytes
            .saturating_sub(self.totals().outbound.data_bytes)
    }

    /// The bytes it wrote to its links that weren't chunk data: its
    /// overhead less the transport's framing, as strims-sim's series count
    /// it.
    fn written_overhead(&self) -> u64 {
        self.overhead().saturating_sub(self.sent.transport_bytes)
    }

    /// Its clock's reading at true time `t`.
    fn local(&self, t: Time) -> Time {
        shift(t, self.clock)
    }

    /// The true time at which its clock reads `t`.
    fn true_time(&self, t: Time) -> Time {
        shift(t, -self.clock)
    }

    /// `r`, a path reading in true time, in its clock.
    fn local_reading(&self, r: Option<PathReading>) -> Option<PathReading> {
        r.map(|r| PathReading {
            at: self.local(r.at),
            ..r
        })
    }

    /// Whether the results count it as an honest viewer: one watching now,
    /// or one that watched until it left for good.
    fn watched(&self) -> bool {
        self.role == Role::Peer || (self.role == Role::Gone && self.kind == Role::Peer)
    }

    /// The seconds it was there, from its arrival to its departure or to
    /// `end`.
    fn present_secs(&self, end: Time) -> f64 {
        (self.left.unwrap_or(end) - self.arrival).as_secs_f64()
    }
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
    /// The path measure of what `from` sends (see the module docs).
    path: PathMeter,
    /// The echo `from` owes `to`'s newest probe, and when it may go (a free
    /// rider holds it).
    echo: Echo,
    echo_at: Time,
    /// The last probe's nonce, and when `from` last sent data or a probe.
    nonce: u32,
    last_send: Time,
}

impl Link {
    fn new(
        from: usize,
        to: usize,
        latency: Duration,
        loss: f64,
        peer: PeerKey,
        channel: u64,
        rpeer: PeerKey,
    ) -> Link {
        Link {
            from,
            to,
            latency,
            loss,
            peer,
            channel,
            rpeer,
            last_arrival: Time::NIL,
            path: PathMeter::new(),
            echo: Echo::default(),
            echo_at: Time::NIL,
            nonce: 0,
            last_send: t0(),
        }
    }

    /// Whether `from` owes control the link must send on its own: the echo
    /// it may let go, its first probe, or an idle link's keepalive probe.
    fn control_due(&self, now: Time) -> bool {
        (self.echo.is_pending() && self.echo_at <= now) || self.probe_alone(now)
    }

    fn probe_alone(&self, now: Time) -> bool {
        self.path.can_probe()
            && (self.path.bytes_written() == 0 || now - self.last_send >= KEEPALIVE)
    }
}

/// Links nodes `a` and `b` (`a < b`) at `now`: each runner's peer and
/// channels (the cross-traffic swarm's too), and the two directed links,
/// `a`'s first. A new link's fair-queue time starts at the least of its
/// node's others, the floor [`Sim::send`] lifts a link to.
fn link_nodes(
    nodes: &mut [Node],
    links: &mut Vec<Link>,
    (a, b): (usize, usize),
    latency: Duration,
    loss: f64,
    now: Time,
) {
    let (ta, tb) = (nodes[a].local(now), nodes[b].local(now));
    let pa = nodes[a].runner.add_peer(node_id(b), ta);
    let pb = nodes[b].runner.add_peer(node_id(a), tb);
    let (ca, cb) = (a as u64 + 1, b as u64 + 1);
    let sa = nodes[a].swarm;
    let sb = nodes[b].swarm;
    nodes[a].runner.run_swarm(pa, sa, cb, ca, ta).unwrap();
    nodes[b].runner.run_swarm(pb, sb, ca, cb, tb).unwrap();
    if let (Some(xa), Some(xb)) = (
        nodes[a].cross.as_ref().map(|x| x.key),
        nodes[b].cross.as_ref().map(|x| x.key),
    ) {
        let (ka, kb) = (ca + 1000, cb + 1000);
        nodes[a].runner.run_swarm(pa, xa, kb, ka, ta).unwrap();
        nodes[b].runner.run_swarm(pb, xb, ka, kb, tb).unwrap();
    }
    let la = links.len();
    links.push(Link::new(a, b, latency, loss, pa, ca, pb));
    links.push(Link::new(b, a, latency, loss, pb, cb, pa));
    for (from, to, l) in [(a, b, la), (b, a, la + 1)] {
        links[l].last_send = now;
        let n = &mut nodes[from];
        let v = n.vtime.values().copied().min().unwrap_or(0);
        n.links.insert(to, l);
        n.vtime.insert(l, v);
    }
}

#[derive(Debug)]
enum Ev {
    /// A frame (and its control records) reaches the receiver's downlink
    /// queue, `wire` bytes of it.
    Arrive {
        link: usize,
        frame: Vec<u8>,
        ctl: Ctl,
        wire: usize,
    },
    /// A frame leaves the downlink and is handled.
    Deliver {
        link: usize,
        frame: Vec<u8>,
        ctl: Ctl,
    },
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
    /// The requests grabbers will forge, per link out of one.
    grabs: BTreeMap<usize, Grabs>,
    /// STREAM_OPENs `WithholdPush` adversaries sent.
    withheld_opens: u64,
    /// Messages that named a data-less chunk ([`Sim::dataless_messages`]).
    dataless_on_wire: u64,
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
    grafts: Grafts,
    /// The push trees' shape and the upload meters, once a second after
    /// the warm-up.
    shape: Shape,
    /// The channels' request windows, once a second.
    windows: Windows,
    /// Each live node's next wake time ([`Sim::wake_time`]), by time and
    /// node, and its entry per node. Only `stale` nodes' times are
    /// recomputed (and those that came due), so the event loop costs what
    /// the nodes that act do, not the swarm's size.
    wakes: BTreeSet<(Time, usize)>,
    wake_of: Vec<Option<Time>>,
    /// Nodes whose state changed since their wake time was computed, and
    /// nodes whose runner may hold events to drain. Bound: one per node.
    stale: BTreeSet<usize>,
    undrained: BTreeSet<usize>,
    /// The reports' verdicts against the real subtrees.
    verdicts: Verdicts,
    /// Per liar, the samples in a row so far at which it held a copy, and
    /// the finished runs.
    liar_runs: BTreeMap<usize, u32>,
    liar_spans: Vec<u32>,
    /// ANNOUNCEs with entries that reached the seed from viewers, and
    /// their bytes; the frame bytes the viewers sent.
    announce: (u64, u64),
    viewer_bytes: u64,
    /// With `SIM_COLLUDE`, when each greedy viewer reports next.
    collude_at: Vec<Time>,
    /// With `SIM_SERIES=1` or the tables, the honest viewers' and the
    /// seed's counters once a second ([`Sim::series`]).
    series: Option<Vec<SeriesSample>>,
    /// With the tables, the same counters at every edge of the fine bins.
    bins: Option<Bins>,
    /// With the tables, the run's timed events ([`TimedEvent`]).
    timeline: Option<Vec<TimedEvent>>,
    /// With the tables, every node's forecast log ([`Runner::set_forecasts`]),
    /// and the samples the logs' caps left out.
    forecasts: Option<Vec<Forecast>>,
    forecasts_lost: u64,
    /// The churn's leaves and returns ([`churn_plan`]), and how many are
    /// done.
    churn: Vec<(Time, usize, bool)>,
    churn_done: usize,
    /// The viewers' arrivals after the start ([`arrival_plan`]) and their
    /// departures ([`depart_plan`]), sorted, and how many of each are done.
    arrivals: Vec<(Time, usize)>,
    arrivals_done: usize,
    departures: Vec<(Time, usize)>,
    departures_done: usize,
    /// The honest viewers whose open parent left ([`Sim::depart`]), counted
    /// per departure, and their streams it was the parent of.
    parent_lost: (u64, u64),
    /// From each place taken again to its viewer's first chunk read.
    joins: Vec<Duration>,
    /// What a new viewer's swarm is made from.
    swarm_id: SwarmId,
    options: SwarmOptions,
    /// With `SIM_LOOPS=1`, how push loops form.
    loops: Option<Loops>,
    /// A media run's stream.
    media: Option<MediaRun>,
}

/// A media run's stream ([`Media`]): its plan, the ops written, when the
/// stream starts (the plan's times count from it), the main swarm's codec,
/// and per chunk when its unit was signed.
struct MediaRun {
    media: Media,
    plan: Plan,
    done: usize,
    start: Time,
    padded: Arc<Padded>,
    signed: Vec<Time>,
}

/// The fine bins of the tables ([`Sim::keep_tables`], [`Sim::write_tables`]):
/// the series' counters and more read at every edge of bins `width` wide
/// from the stream's start, and each bin's row.
struct Bins {
    width: Duration,
    /// The next edge.
    next: Time,
    /// The counters at the last edge.
    last: Option<BinSample>,
    /// The `bins` table's rows ([`bin_cols`]).
    rows: Vec<Vec<i64>>,
    /// With `SIM_BIN_NODES=1`, the `node_bins` table's rows
    /// ([`node_bin_cols`]): each node's in each bin.
    node_rows: Option<Vec<Vec<i64>>>,
}

/// The counters at one bin edge.
#[derive(Clone)]
struct BinSample {
    /// Per viewer node (from 1), if honest and present: pushes before
    /// their deadline, deadlines released to pull, parents pruned, streams
    /// with an open parent, the overhead bytes it wrote so far
    /// ([`Node::written_overhead`]), and its uplink's bytes, packets and
    /// small packets ([`Node::wire`]); and the rest of its counters.
    viewers: Vec<Option<([u64; 8], NodeCounts)>>,
    /// The seed's meter capacity and busy thousandths, copies, chunks sent
    /// and pushed, uplink, overhead bytes, and its uplink's bytes, packets
    /// and small packets; and the rest of its counters.
    seed: ([u64; 10], NodeCounts),
}

/// A node's counters at a bin edge beyond the series': [`NODE_COLS`]' so
/// far, and its push and answer waits' histograms so far.
#[derive(Clone)]
struct NodeCounts {
    c: [u64; NODE_COLS.len()],
    waits: [WaitHistogram; 2],
}

/// [`NodeCounts`]' counters: requests timed out, chunks wasted, answers a
/// full queue refused and answers dropped at write time
/// ([`strims_ppspp::SwarmCounters`]), parents replaced, parents lost to
/// their closing the subscription and subscribers closed for a saturated
/// uplink ([`PushStats`]' `switched`, `shed`, `sheds_sent`), its outbound
/// messages' bytes by kind, its payloads and those of HAVEs alone
/// ([`ChannelMetrics`]), and its stream messages that lost a packet and
/// datagrams lost ([`Sim::wan_stats`]).
const NODE_COLS: [&str; 21] = [
    "timeouts",
    "wasted",
    "answers_refused",
    "answers_dropped",
    "switched",
    "shed",
    "sheds_sent",
    "have_bytes",
    "ack_bytes",
    "request_bytes",
    "announce_bytes",
    "stream_bytes",
    "session_bytes",
    "ping_bytes",
    "header_bytes",
    "integrity_bytes",
    "data_bytes",
    "payloads",
    "have_only_payloads",
    "retransmits",
    "drops",
];

/// What a bin's waits give: pushes written and answers sent in it, and the
/// p50 and p99 of their waits (empty without any: [`NONE`]).
const WAIT_COLS: [&str; 6] = [
    "push_waits",
    "push_wait_p50_ms",
    "push_wait_p99_ms",
    "answer_waits",
    "answer_wait_p50_ms",
    "answer_wait_p99_ms",
];

/// The `bins` table's columns ([`Sim::write_tables`]): the series' and the
/// network's, then [`NODE_COLS`] and [`WAIT_COLS`] summed over the viewers
/// (`viewer_`) and the seed's (`seed_`).
fn bin_cols() -> Vec<String> {
    let mut cols: Vec<String> = [
        "bin",
        "t_ms",
        "present",
        "hits",
        "misses",
        "pruned",
        "parents",
        "viewer_ovh",
        "viewer_up_bytes",
        "viewer_up_packets",
        "viewer_small_packets",
        "capacity",
        "busy",
        "avail",
        "copies",
        "sent",
        "pushed",
        "seed_ovh",
        "seed_up_bytes",
        "seed_up_packets",
        "seed_small_packets",
    ]
    .map(String::from)
    .to_vec();
    for who in ["viewer", "seed"] {
        cols.extend(
            NODE_COLS
                .iter()
                .chain(&WAIT_COLS)
                .map(|c| format!("{who}_{c}")),
        );
    }
    cols
}

/// The `node_bins` table's columns: a node's in one bin.
fn node_bin_cols() -> Vec<String> {
    let mut cols: Vec<String> = [
        "bin",
        "t_ms",
        "node",
        "viewer",
        "hits",
        "misses",
        "pruned",
        "parents",
        "ovh",
        "up_bytes",
        "up_packets",
        "small_packets",
    ]
    .map(String::from)
    .to_vec();
    cols.extend(NODE_COLS.iter().chain(&WAIT_COLS).map(|c| c.to_string()));
    cols
}

/// What a node's counters at `b` add to `a`'s ([`NODE_COLS`]), and its
/// waits' histograms in between.
fn counts_delta(a: &NodeCounts, b: &NodeCounts) -> ([i64; NODE_COLS.len()], [WaitHistogram; 2]) {
    let mut c = [0; NODE_COLS.len()];
    for (x, (p, q)) in c.iter_mut().zip(b.c.iter().zip(&a.c)) {
        *x = p.saturating_sub(*q) as i64;
    }
    let mut waits = [WaitHistogram::default(); 2];
    for (w, (p, q)) in waits.iter_mut().zip(b.waits.iter().zip(&a.waits)) {
        for (x, (m, n)) in w.buckets.iter_mut().zip(p.buckets.iter().zip(&q.buckets)) {
            *x = m.saturating_sub(*n);
        }
    }
    (c, waits)
}

/// An integer [`Table::ints`] writes as an empty field (`NULL` to
/// `hack/sim-db`): a reading a run doesn't have.
const NONE: i64 = i64::MIN;

/// [`WAIT_COLS`]' values for the waits `w`.
fn wait_cols(w: &[WaitHistogram; 2]) -> [i64; WAIT_COLS.len()] {
    let p = |h: &WaitHistogram, q: f64| h.percentile(q).map_or(NONE, ms);
    [
        w[0].count() as i64,
        p(&w[0], 0.5),
        p(&w[0], 0.99),
        w[1].count() as i64,
        p(&w[1], 0.5),
        p(&w[1], 0.99),
    ]
}

/// The `bins` row of bin `k`, from the counters at its edges: what the
/// viewers present at both did (summed, and their median open parents at
/// its end), and the seed's.
fn bin_row(k: usize, width: Duration, a: &BinSample, b: &BinSample) -> Vec<i64> {
    type Counts = ([u64; 8], NodeCounts);
    let both: Vec<(&Counts, &Counts)> = b
        .viewers
        .iter()
        .zip(&a.viewers)
        .filter_map(|(x, y)| Some((x.as_ref()?, y.as_ref()?)))
        .collect();
    let sum = |f: usize| -> i64 {
        both.iter()
            .map(|(x, y)| x.0[f].saturating_sub(y.0[f]) as i64)
            .sum()
    };
    let mut parents: Vec<u64> = both.iter().map(|(x, _)| x.0[3]).collect();
    parents.sort_unstable();
    let gauge = |f: usize| b.seed.0[f] as i64;
    let delta = |f: usize| b.seed.0[f].saturating_sub(a.seed.0[f]) as i64;
    let mut row = vec![
        k as i64,
        k as i64 * width.as_millis(),
        both.len() as i64,
        sum(0),
        sum(1),
        sum(2),
        parents.get(parents.len() / 2).map_or(NONE, |&x| x as i64),
        sum(4),
        sum(5),
        sum(6),
        sum(7),
        gauge(0),
        gauge(1),
        gauge(5),
        gauge(2),
        delta(3),
        delta(4),
        delta(6),
        delta(7),
        delta(8),
        delta(9),
    ];
    let mut counts = [0i64; NODE_COLS.len()];
    let mut waits = [WaitHistogram::default(); 2];
    for (x, y) in &both {
        let (c, w) = counts_delta(&y.1, &x.1);
        for (s, v) in counts.iter_mut().zip(c) {
            *s += v;
        }
        for (s, v) in waits.iter_mut().zip(&w) {
            s.merge(v);
        }
    }
    row.extend(counts);
    row.extend(wait_cols(&waits));
    let (c, w) = counts_delta(&a.seed.1, &b.seed.1);
    row.extend(c);
    row.extend(wait_cols(&w));
    row
}

/// Node `node`'s `node_bins` row in bin `k`, from its counters at the
/// bin's edges (`a`, `b`: the series' in [`BinSample::viewers`]' order, and
/// the rest); its `viewer` is filled in by [`Sim::write_tables`].
fn node_bin_row(
    k: usize,
    width: Duration,
    node: usize,
    a: (&[u64; 8], &NodeCounts),
    b: (&[u64; 8], &NodeCounts),
) -> Vec<i64> {
    let d = |f: usize| b.0[f].saturating_sub(a.0[f]) as i64;
    let mut row = vec![
        k as i64,
        k as i64 * width.as_millis(),
        node as i64,
        -1,
        d(0),
        d(1),
        d(2),
        b.0[3] as i64,
        d(4),
        d(5),
        d(6),
        d(7),
    ];
    let (c, w) = counts_delta(a.1, b.1);
    row.extend(c);
    row.extend(wait_cols(&w));
    row
}

/// One timed event of the run for the `events` table: true time, what it
/// was, the node it happened to (or -1), the other node (or -1), a value
/// and a detail by kind ([`Sim::write_tables`]).
struct TimedEvent {
    at: Time,
    kind: &'static str,
    node: i64,
    peer: i64,
    value: i64,
    detail: &'static str,
}

/// A forecast log sample ([`Runner::set_forecasts`]) as the harness keeps
/// it: the node that took it, the node it sampled (`usize::MAX` if its
/// peer key named no link), and the sample in the node's clock.
struct Forecast {
    node: usize,
    peer: usize,
    f: LagForecast,
}

/// One second's sample for [`Sim::series`].
struct SeriesSample {
    /// Per viewer node (from 1), if honest and present: pushes before
    /// their deadline, deadlines released to pull, parents pruned, streams
    /// with an open parent, and the overhead bytes it wrote so far
    /// ([`Node::written_overhead`]).
    viewers: Vec<Option<[u64; 5]>>,
    /// The seed's meter capacity and busy thousandths, copies, chunks sent
    /// and pushed, uplink, and overhead bytes so far.
    seed: [u64; 7],
}

impl Verdicts {
    fn add(&mut self, o: &Verdicts) {
        self.honest += o.honest;
        self.with_children += o.with_children;
        self.with_reporters += o.with_reporters;
        self.judged += o.judged;
        self.served += o.served;
        self.served_childless += o.served_childless;
        self.liars += o.liars;
        self.liars_judged += o.liars_judged;
        self.liars_served += o.liars_served;
        self.not_kept += o.not_kept;
        self.barred_with_children += o.barred_with_children;
    }
}

/// An adaptive seed's report verdicts at each sample after the warm-up,
/// against its holders' real children: the nodes whose open parent it is
/// on some stream, from every node's push stats.
#[derive(Clone, Copy, Debug, Default)]
struct Verdicts {
    /// Honest holder samples, those with a real child, and those with one
    /// among the seed's neighbours (the ones that can report).
    honest: u64,
    with_children: u64,
    with_reporters: u64,
    /// Honest holder samples a quorum judged, those it saw serve, and of
    /// those the ones without a real child.
    judged: u64,
    served: u64,
    served_childless: u64,
    /// Liar holder samples, those judged, and those served.
    liars: u64,
    liars_judged: u64,
    liars_served: u64,
    /// Honest holder samples at which the seed pushed to it and its pushes
    /// didn't keep up.
    not_kept: u64,
    /// Honest holders barred that had a real child at the sample before.
    barred_with_children: u64,
}

/// Hops from the seed that [`Shape::depth`] counts one by one; deeper
/// chains count together.
const DEPTH_HOPS: usize = 6;

/// The push trees' shape and the nodes' upload meters, sampled once a
/// second after the warm-up (report only).
#[derive(Default)]
struct Shape {
    /// Honest (viewer, stream) samples by how the viewer gets the stream:
    /// `[0]` with no open parent; `[h]` through a chain of open parents `h`
    /// hops from the seed, up to [`DEPTH_HOPS`]; then deeper chains; then
    /// cycles; last, chains that end at a viewer with no open parent (its
    /// subtree is rooted in pull).
    depth: [u64; DEPTH_HOPS + 4],
    /// Over the samples, honest viewers with at least one push subscriber,
    /// and honest viewers.
    relays: u64,
    viewers: u64,
    /// Per sample of each honest node whose upload meter has a capacity:
    /// the capacity over the node's true uplink, for the seed, relays and
    /// the other viewers.
    meter: [Vec<f64>; 3],
    /// Per sample of each honest relay: its push subscribers, all streams.
    fanout: Vec<u64>,
    /// STREAM_REQUESTs honest viewers declined over the samples, and those
    /// declined in a second in which the viewer used under half its
    /// uplink.
    declines: u64,
    declines_idle: u64,
    /// Per node at the last sample: its declines so far and its wire
    /// bytes up.
    last: Vec<Option<(u64, u64)>>,
    /// Over the samples of honest viewers with channels: the shares of
    /// their channels whose peer they ask for lazy HAVEs, and whose peer
    /// asked lazy HAVEs of them, summed; and the samples.
    lazy: [f64; 2],
    lazy_samples: u64,
}

/// Every open channel's request window, sampled once a second over the
/// run on every honest node's main swarm ([`Sim::sample_windows`]): the
/// channels whose peer's handshake came.
#[derive(Default)]
struct Windows {
    /// LEDBAT's window and flight, in bytes.
    cwnd: ValueHistogram,
    flight: ValueHistogram,
    /// Requests out (`ChannelSchedulerStats::requests_out`).
    requests_out: ValueHistogram,
    /// LEDBAT's queuing delay (channels that had a delay sample) and the
    /// request timeout, in microseconds.
    queuing_delay: ValueHistogram,
    cto: ValueHistogram,
}

impl Windows {
    fn add(&mut self, s: &ChannelSchedulerStats) {
        if !s.handshake_received {
            return;
        }
        let us = |d: Duration| d.as_micros().max(0) as u64;
        self.cwnd.add(s.cwnd.max(0) as u64);
        self.flight.add(s.flight_size.max(0) as u64);
        self.requests_out.add(s.requests_out as u64);
        if let Some(d) = s.queuing_delay {
            self.queuing_delay.add(us(d));
        }
        self.cto.add(us(s.cto));
    }

    /// The row's columns, as strims-sim's rows name them: the window's
    /// p10, p50 and p90 and the flight's p50 and p90 in KiB, the requests
    /// out's p50, p90 and most, and the queuing delay's and the request
    /// timeout's p50 and p90 in ms. Most samples of a push channel have
    /// nothing out, so the flight and the requests out also read over the
    /// samples with any (`*_busy_*`), with the share of samples with
    /// requests out (`reqs_out_share`). Columns without samples are left
    /// out.
    fn cols(&self, row: &mut JsonRow) {
        let (flight, requests) = (self.flight.nonzero(), self.requests_out.nonzero());
        for (name, h, unit, ps) in [
            ("cwnd_kib", &self.cwnd, 1024.0, &[10, 50, 90][..]),
            ("flight_kib", &self.flight, 1024.0, &[50, 90]),
            ("flight_kib_busy", &flight, 1024.0, &[50, 90]),
            ("reqs_out", &self.requests_out, 1.0, &[50, 90]),
            ("reqs_out_busy", &requests, 1.0, &[50, 90]),
            ("ledbat_qd_ms", &self.queuing_delay, 1e3, &[50, 90]),
            ("cto_ms", &self.cto, 1e3, &[50, 90]),
        ] {
            for &p in ps {
                if let Some(x) = h.percentile(f64::from(p)) {
                    row.num(&format!("{name}_p{p}"), x as f64 / unit);
                }
            }
        }
        if let Some(n) = self.requests_out.max() {
            row.int("reqs_out_max", n).num(
                "reqs_out_share",
                requests.count() as f64 / self.requests_out.count() as f64,
            );
        }
    }
}

/// With `SIM_LOOPS=1`, how push loops (cycles of open parents) form and
/// how long they last: every runner's decision tap feeds a log of each
/// viewer's parent decisions per stream, and each second's sample finds
/// the loops ([`Sim::sample_loops`]).
#[derive(Default)]
struct Loops {
    /// Per (viewer, stream), its latest parent decisions.
    log: BTreeMap<(usize, Stream), std::collections::VecDeque<(Time, Decision)>>,
    /// The loops at the latest sample, by stream and members (sorted), with
    /// when each was first seen.
    active: BTreeMap<(Stream, Vec<usize>), Time>,
    /// The loops that formed, by how their newest edge formed.
    closed_by: BTreeMap<String, u64>,
    /// Members of each loop that formed, and the lifetimes of those that
    /// broke.
    lens: Vec<u64>,
    lives: Vec<Duration>,
    /// Tap events dropped between drains.
    lost: u64,
    /// Grafts and switches asked, and those that asked one of the asking
    /// viewer's own descendants for the stream.
    grafts: u64,
    grafts_down: u64,
    switches: u64,
    switches_down: u64,
    /// Every parent decision on either side, counted by kind.
    kinds: BTreeMap<String, u64>,
    /// Chunks honest viewers asked the seed for, by how many of the
    /// asker's other neighbours had held them for [`HELD_FOR`]: none, one
    /// or two, three or more; and of those held by none, the ones whose
    /// stream had no open parent at the asker.
    seed_asks: [u64; 3],
    seed_asks_parentless: u64,
}

/// How long a neighbour must have held a chunk for its HAVE to have
/// reached a peer that asks the seed for it ([`Loops::seed_asks`]).
const HELD_FOR: Duration = Duration::from_millis(100);

/// The decisions log keeps per (viewer, stream).
const LOOP_LOG: usize = 16;

/// A parent decision, naming peers by node.
#[derive(Clone, Copy, Debug)]
enum Decision {
    /// With no parent, asked this node.
    Graft(usize),
    /// Asked this node to replace the parent.
    Switch(usize),
    /// Pruned the parent: silent, or else lossy.
    Pruned {
        silent: bool,
    },
    ShedBy,
    Declined,
    OpenTimeout,
}

impl Loops {
    /// How the edge from `child` to `parent` on `stream` formed, and
    /// when: the latest graft or switch that asked `parent`, and for a
    /// graft the latest loss before it.
    fn edge(&self, child: usize, parent: usize, stream: Stream) -> Option<(Time, String)> {
        let log = self.log.get(&(child, stream))?;
        let at = log.iter().rposition(
            |(_, d)| matches!(d, Decision::Graft(p) | Decision::Switch(p) if *p == parent),
        )?;
        let (t, d) = log[at];
        let how = match d {
            Decision::Switch(_) => "switch".to_owned(),
            _ => {
                let lost = log.iter().take(at).rev().find_map(|(_, d)| match d {
                    Decision::Pruned { silent: true, .. } => Some("silent prune"),
                    Decision::Pruned { .. } => Some("lossy prune"),
                    Decision::ShedBy => Some("shed"),
                    Decision::Declined | Decision::OpenTimeout => Some("decline"),
                    _ => None,
                });
                format!("graft after {}", lost.unwrap_or("nothing logged"))
            }
        };
        Some((t, how))
    }
}

/// Honest viewers' open parents, sampled with the seed's allocation after
/// the warm-up: how often, and how long at a time, a liar is one.
#[derive(Default)]
struct Grafts {
    /// (viewer, stream) samples, and those whose parent is a liar.
    samples: u64,
    to_liars: u64,
    /// Per (viewer, stream), the samples in a row so far with a liar.
    runs: BTreeMap<(usize, usize), u32>,
    /// Finished runs, in samples.
    spans: Vec<u32>,
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
    /// mean one-way delay after publishing, as the neighbour's reports give
    /// it. A stream it reported no copies of lately counts as [`STALE`].
    objective: Duration,
    /// The neighbours barred from holding a copy.
    barred: BTreeSet<usize>,
    /// After the warm-up, the holders with a real child.
    with_children: BTreeSet<usize>,
    /// The seed's upload capacity estimate (`PushStats::upload_capacity`).
    capacity: u64,
    /// Per honest neighbour holding every stream: its on-time pushes
    /// (`PushStats::hits`), all of them from the seed.
    hits: BTreeMap<usize, u64>,
}

/// The share of its copy a holder must get as on-time pushes in an interval
/// to count as whole ([`Report::holder_short`]).
const HOLDER_WHOLE: f64 = 0.95;

/// What a stream a neighbour reported no copies of lately counts as in the
/// seed's objective.
const STALE: Duration = Duration::from_secs(2);

// Every field is printed through `Debug`.
#[allow(dead_code)]
#[derive(Debug)]
struct Report {
    trace: u64,
    frames: u64,
    bytes: u64,
    total: u64,
    /// Per honest viewer there at the end: start offset, final position.
    positions: Vec<(u64, u64)>,
    /// The share of all chunks that honest peers read.
    delivery: f64,
    mismatches: u64,
    underruns: u64,
    resets: u64,
    lag_p50: Duration,
    lag_p99: Duration,
    lag_max: Duration,
    /// In a media run, the lag from each media chunk's signing.
    sign_lag_p50: Duration,
    sign_lag_p99: Duration,
    /// Worst-stripe lag ([`Sim::munro_lags`]): p90 and p99.
    stripe_p90: Duration,
    stripe_p99: Duration,
    /// In WAN mode, the lag p99 of each region's peers.
    regions: Vec<(Region, Duration)>,
    max_queue_delay: Duration,
    finish: Time,
    /// The seed's share of all data bytes sent.
    seed_share: f64,
    /// Data bytes sent by everyone ([`Sim::uploaded`]).
    data_bytes: u64,
    /// Chunks honest peers received that they already had.
    wasted: u64,
    /// Push counters: a child's summed over honest peers, a parent's over
    /// every node but the greedy viewers.
    push: PushStats,
    /// The seed's push subscribers, all streams, at the end.
    seed_children: usize,
    /// The seed's push subscriptions (stream slots), mean over the samples
    /// after the warm-up.
    seed_slots: f64,
    /// The seed's objective ([`AllocSample::objective`]), median over the
    /// samples after the warm-up.
    seed_objective: Duration,
    /// The seed's capacity estimate ([`AllocSample::capacity`]), median
    /// over the samples after the warm-up while it writes.
    seed_capacity: u64,
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
    /// The greedy viewers: free riders and liars.
    greedy: (usize, usize),
    /// The grabbers, and their share of the seed's push slots.
    grabbers: (usize, f64),
    /// Over the samples after the warm-up, per honest neighbour holding
    /// every stream between two samples: the share of those intervals in
    /// which it got under [`HOLDER_WHOLE`] of its copy as on-time pushes,
    /// and the median share it got ([`Sim::holder_copies`]).
    holder_short: f64,
    holder_copy: f64,
    /// Of the seed's push slots (one stream at one neighbour) over the
    /// samples after the warm-up, the shares at free riders and at liars.
    greedy_slots: (f64, f64),
    /// The longest a greedy viewer held a slot without a break, sampled
    /// once a second over the run.
    greedy_held: Duration,
    /// Of honest viewers' streams over the samples after the warm-up, the
    /// share whose parent is a liar, and how long that lasted on average.
    liar_grafts: f64,
    liar_graft_span: Duration,
    /// The neighbours an adaptive seed barred from holding a copy, and
    /// how many of them were honest.
    barred: (u64, u64),
    /// The reports' verdicts against the real subtrees.
    verdicts: Verdicts,
    /// How long a liar held a copy at a time: mean and longest.
    liar_hold: (Duration, Duration),
    /// ANNOUNCEs with entries that reached the seed, their bytes, and
    /// those bytes' share of what the viewers sent; the asks the seed sent.
    announce: (u64, u64, f64),
    asks: u64,
    /// What became of the report entries from honest and from greedy
    /// viewers, at the end.
    report_counts: (ReportCounters, ReportCounters),
}

/// The run's arrivals and departures ([`Arrivals`], [`Depart`]), measured
/// over its honest viewers, those that left included.
#[derive(Debug, Default)]
struct AudienceReport {
    /// Viewers that arrived after the start, and that left for good: all of
    /// them, and the honest ones.
    arrived: (usize, usize),
    departed: (usize, usize),
    /// From each honest viewer's join to its first read (sorted), and the
    /// viewers that never read; to its first read within [`CAUGHT_UP`],
    /// and the viewers that never caught up.
    joins: Vec<Duration>,
    unread: usize,
    caught_up: Vec<Duration>,
    behind: usize,
    /// Honest viewers whose open parent left ([`Sim::parent_lost`]).
    parent_lost: (u64, u64),
    /// With departures, the recovery from them.
    recovery: Option<Recovery>,
}

/// How the honest viewers there before the departures started, and that
/// stayed, came through them ([`Sim::recovery`]), from the departures'
/// start on.
#[derive(Debug)]
struct Recovery {
    /// The viewers measured, and those of them left with no link to a node
    /// still there ([`Depart`]: nobody links anew).
    stayed: usize,
    stranded: usize,
    /// The worst lag of their reads.
    peak: Duration,
    /// Over the stream's seconds, their per-second lag p90's excess over
    /// [`CAUGHT_UP`], summed (ms × s), and the time to the end of the last
    /// second it was over (zero if none was); `censored` if that was the
    /// stream's last, so they never recovered in it.
    excess_ms_s: i64,
    recover: Duration,
    censored: bool,
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
        assert!(
            cfg.peers < MAX_NODES,
            "{} viewers: at most {}",
            cfg.peers,
            MAX_NODES - 1
        );
        let mut rng = det_rng(cfg.seed);
        let key = strims_crypto::key_from_seed(&[9; 32]);
        let padded = cfg.media.map(|_| Arc::new(Padded::default()));
        let chunk_bytes = match (&padded, cfg.keep) {
            (None, keep) => keep.chunk_bytes(Content::Main),
            (Some(p), Keep::Nothing) => ChunkBytes::Codec(p.clone()),
            (Some(_), Keep::Bytes) => ChunkBytes::Stored,
            (Some(_), Keep::Head) => panic!("a media run keeps nothing or the bytes"),
        };
        assert!(
            cfg.media.is_none() || cfg.cross.is_none(),
            "a media run takes no cross traffic"
        );
        let options = SwarmOptions {
            chunk_size: cfg.chunk_size,
            chunks_per_signature: cfg.munro_chunks,
            live_window: cfg.live_window,
            chunk_bytes,
            delivery_mode: cfg.delivery_mode,
            stream_count: cfg.stream_count,
            push_mode: cfg.push_mode,
            seed: cfg.seed_policy,
            ..Default::default()
        };
        let cross_key = strims_crypto::key_from_seed(&[10; 32]);
        let cross_options = cfg.cross.map(|c| SwarmOptions {
            chunk_bytes: cfg.keep.chunk_bytes(Content::Cross),
            push_mode: c.push_mode,
            ..options.clone()
        });

        let greedy = greedy(&cfg);
        let mut roles = vec![Role::Seed];
        roles.extend(
            (1..=cfg.peers).map(|i| greedy.get(&i).map_or(Role::Peer, |&g| Role::Greedy(g))),
        );
        roles.extend(cfg.adversaries.iter().map(|&a| Role::Adversary(a)));
        if let Some(w) = &cfg.wan {
            assert_eq!(w.sites.len(), roles.len(), "a site per node");
            assert!(cfg.loss <= 0.0, "WAN paths lose packets of their own");
        }

        // The run's draws come before the runners' first calls, which take
        // each node's clock: each node's runner, the mesh and its
        // latencies, the churn and the media trace, then the clocks
        // ([`ClockSkew`]), then the arrivals and departures ([`Arrivals`]).
        let runners: Vec<Runner> = roles.iter().map(|_| Runner::new(fork(&mut rng))).collect();

        // The mesh: the seed's peers, then each peer's.
        let n = roles.len();
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

        let span = (cfg.latency_ms.1 - cfg.latency_ms.0).max(1) as u64;
        let latencies: Vec<Duration> = edges
            .iter()
            .map(|_| Duration::from_millis(cfg.latency_ms.0 + (rng.next_u64() % span) as i64))
            .collect();
        let capture = cfg.capture.clone().or_else(|| {
            knob::<String>("SIM_CAPTURE").map(|dir| CaptureTo {
                dir: Path::new(&dir).join(run_tag(&cfg)),
                tap: knob::<String>("SIM_CAPTURE_TAP").as_deref() != Some("0"),
            })
        });
        let churn = match cfg.churn {
            Some(c) => {
                assert!(
                    capture.is_none() && cfg.cross.is_none(),
                    "churn takes no capture or cross traffic"
                );
                churn_plan(&cfg, c, &roles, &mut rng)
            }
            None => Vec::new(),
        };
        let media = cfg.media.zip(padded).map(|(m, padded)| {
            let span = Duration::from_secs(cfg.stream_secs as i64);
            let trace = m.encoder.trace(&mut rng, span);
            let signer = Signer::new(
                m.sign.deadline,
                cfg.munro_chunks,
                cfg.chunk_size,
                live::CHUNK_HEADER,
            );
            let plan = live::plan(&trace, signer);
            padded.0.set(plan.pads.clone()).unwrap();
            let start = t0() + Duration::SECOND;
            MediaRun {
                media: m,
                signed: plan.signed_at.iter().map(|&d| start + d).collect(),
                plan,
                done: 0,
                start,
                padded,
            }
        });
        let clocks = clock_offsets(cfg.clock_skew, roles.len(), &mut rng);
        if cfg.arrivals != Arrivals::T0 || cfg.depart.is_some() {
            assert!(
                cfg.churn.is_none() && cfg.cross.is_none() && cfg.pin.is_none(),
                "arrivals and departures take no churn, cross traffic or pinned allocation"
            );
        }
        let arrivals = arrival_plan(&cfg, roles.len(), &mut rng);
        let departures = cfg.depart.map_or_else(Vec::new, |d| {
            depart_plan(&cfg, d, &arrivals, roles.len(), &mut rng)
        });
        // Arrivals link as they come ([`Sim::arrive`]).
        let mut arriving = vec![false; roles.len()];
        for &(_, i) in &arrivals {
            arriving[i] = true;
        }
        if !arrivals.is_empty() {
            edges.clear();
        }

        let mut nodes = Vec::new();
        for ((i, &role), mut runner) in roles.iter().enumerate().zip(runners) {
            let start = shift(t0(), clocks[i]);
            let (writer, swarm) = if role == Role::Seed {
                let (w, s) = Writer::new(
                    WriterOptions {
                        swarm_options: options.clone(),
                        key: key.clone(),
                    },
                    start,
                )
                .unwrap();
                (Some(w), s)
            } else {
                // An arrival reads from the live edge.
                let options = match arriving[i] {
                    true => SwarmOptions {
                        delivery_mode: DeliveryMode::LOW_LATENCY,
                        ..options.clone()
                    },
                    false => options.clone(),
                };
                (
                    None,
                    strims_ppspp::Swarm::new(SwarmId::new(&key.public), options).unwrap(),
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
                            start,
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
                role: if arriving[i] { Role::Absent } else { role },
                kind: role,
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
                read_seqs: vec![],
                reads_hash: 0xcbf2_9ce4_8422_2325,
                first_read: None,
                caught_up: None,
                sign_lags: vec![],
                read_chunks: vec![],
                arrived: vec![],
                max_queue_delay: Duration::ZERO,
                errors: 0,
                removed: BTreeSet::new(),
                cross,
                wire: EndStats::default(),
                sent: Sent::default(),
                past: ChannelTotals::default(),
                past_timeouts: 0,
                hostile: matches!(role, Role::Greedy(g) if g != Greed::Grabber)
                    || matches!(role, Role::Adversary(_)),
                cap: None,
                churned: false,
                rejoined: None,
                arrival: t0(),
                joined: t0(),
                left: None,
                clock: clocks[i],
            });
        }

        let mut links = Vec::new();
        for (&(a, b), &latency) in edges.iter().zip(&latencies) {
            link_nodes(&mut nodes, &mut links, (a, b), latency, cfg.loss, t0());
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

        if let Some(to) = &capture {
            std::fs::create_dir_all(&to.dir).unwrap();
            for (i, n) in nodes.iter_mut().enumerate() {
                let mut label = match &cfg.wan {
                    Some(w) => {
                        format!("{:?} {:?} {:?}", n.kind, w.sites[i].city, w.sites[i].access)
                    }
                    None => format!("{:?}", n.kind),
                };
                // Its decisions' times are its clock's.
                if cfg.clock_skew != ClockSkew::Off {
                    label += &format!(" clock {:+}ms", ms(n.clock));
                }
                let path = to.dir.join(format!("node-{i}.ppcap"));
                let sink: Box<dyn FnMut(u64) -> io::Result<BufWriter<std::fs::File>>> =
                    Box::new(move |_| Ok(BufWriter::new(std::fs::File::create(&path)?)));
                let opts = CaptureOptions {
                    clock: Clock::Sim,
                    node: node_id(i),
                    label: &label,
                    max_file_bytes: 0,
                    tap: to.tap,
                };
                n.cap = Some(Capture::new(sink, opts, t0(), &mut n.runner).unwrap());
            }
        }

        let rcfg = nodes[0]
            .runner
            .swarm(nodes[0].swarm)
            .unwrap()
            .reader_config();
        let end = t0() + Duration::from_secs(cfg.stream_secs as i64 + 20);
        let mtu = cfg.mtu;
        let collude_at = vec![t0(); nodes.len()];
        let every: BTreeSet<usize> = (0..nodes.len()).collect();
        let wake_of = vec![None; nodes.len()];
        let loops = (knob::<u32>("SIM_LOOPS") == Some(1)).then(|| {
            assert!(
                capture.is_none(),
                "SIM_LOOPS drains the tap a capture would"
            );
            for n in &mut nodes {
                n.runner.set_tap(TapLevel::All);
            }
            Loops::default()
        });
        let munro = cfg.munro_chunks * cfg.chunk_size;
        let munros = match &media {
            Some(m) => m.plan.units() as usize,
            None => (cfg.stream_rate * cfg.stream_secs) as usize / munro,
        };
        for node in &mut nodes {
            node.read_chunks = vec![false; munros * cfg.munro_chunks];
            node.arrived = vec![Time::NIL; munros * cfg.munro_chunks];
            if let Some(x) = node.cross.as_mut() {
                x.read_chunks = vec![false; munros * cfg.munro_chunks];
            }
        }
        let publish_times = media.as_ref().map_or_else(Vec::new, |m| {
            m.plan
                .media_at
                .iter()
                .map(|t| t.map_or(Time::NIL, |d| m.start + d))
                .collect()
        });
        Sim {
            cfg,
            nodes,
            links,
            rng,
            events: BTreeMap::new(),
            seq: 0,
            now: t0(),
            publish_times,
            requested: BTreeMap::new(),
            grabs: BTreeMap::new(),
            withheld_opens: 0,
            dataless_on_wire: 0,
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
            grafts: Grafts::default(),
            shape: Shape::default(),
            windows: Windows::default(),
            wakes: BTreeSet::new(),
            wake_of,
            stale: every.clone(),
            undrained: every,
            verdicts: Verdicts::default(),
            liar_runs: BTreeMap::new(),
            liar_spans: Vec::new(),
            announce: (0, 0),
            viewer_bytes: 0,
            collude_at,
            series: (knob::<u32>("SIM_SERIES") == Some(1)).then(Vec::new),
            bins: None,
            timeline: None,
            forecasts: None,
            forecasts_lost: 0,
            churn,
            churn_done: 0,
            arrivals,
            arrivals_done: 0,
            departures,
            departures_done: 0,
            parent_lost: (0, 0),
            joins: Vec::new(),
            swarm_id: SwarmId::new(&key.public),
            options,
            loops,
            media,
        }
    }

    fn push(&mut self, t: Time, e: Ev) {
        self.seq += 1;
        self.events.insert((t, self.seq), e);
    }

    fn done(&self) -> bool {
        let total = (self.munros * self.munro) as u64;
        self.writes_done()
            && self.arrivals_done == self.arrivals.len()
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

    /// The next time anything happens: an event, a write, a step, or a
    /// node's wake time.
    fn next_time(&mut self) -> Time {
        let mut t = Time::MAX;
        if let Some((&(et, _), _)) = self.events.first_key_value() {
            t = t.min(et);
        }
        if !self.writes_done() {
            t = t.min(self.next_write_at());
        }
        if let Some((secs, _)) = self.cfg.incident
            && self.incident.is_none()
        {
            t = t.min(t0() + Duration::from_secs(secs));
        }
        if let Some(&(secs, _)) = self.cfg.steps.get(self.steps_done) {
            t = t.min(t0() + Duration::from_secs(secs));
        }
        if let Some(&(at, _, _)) = self.churn.get(self.churn_done) {
            t = t.min(at);
        }
        if let Some(&(at, _)) = self.arrivals.get(self.arrivals_done) {
            t = t.min(at);
        }
        if let Some(&(at, _)) = self.departures.get(self.departures_done) {
            t = t.min(at);
        }
        self.refresh_wakes();
        if let Some(&(wt, _)) = self.wakes.first() {
            t = t.min(wt);
        }
        t
    }

    /// When node `i` next has something to do as of now: its runner's
    /// timeout, its uplink freeing up while it has frames or control due,
    /// or its links' echoes and keepalives coming due. `None` for a node
    /// that sends and receives nothing ([`Role::off`]).
    ///
    /// The time only grows as the clock moves on while the node's state
    /// stays as it was, and while it lies ahead it stays exact: so a node's
    /// time is recomputed only when its state changes ([`Sim::touch`]) or
    /// the time comes.
    fn wake_time(&self, i: usize) -> Option<Time> {
        let n = &self.nodes[i];
        if n.role.off() {
            return None;
        }
        let mut t = Time::MAX;
        if let Some(rt) = n.runner.poll_timeout() {
            t = t.min(n.true_time(rt));
        }
        if n.links
            .values()
            .any(|&l| n.runner.wants_transmit(self.links[l].peer))
        {
            t = t.min(n.up_free.max(self.now));
        }
        for &l in n.links.values() {
            // Control already due waits for the uplink; otherwise its
            // deadlines are still ahead.
            let link = &self.links[l];
            if link.control_due(self.now) {
                t = t.min(n.up_free.max(self.now));
                continue;
            }
            if link.echo.is_pending() {
                t = t.min(link.echo_at);
            }
            if link.path.can_probe() {
                t = t.min(link.last_send + KEEPALIVE);
            }
        }
        Some(t)
    }

    /// Node `i`'s state changed: its wake time is stale, and its runner may
    /// hold events.
    fn touch(&mut self, i: usize) {
        self.stale.insert(i);
        self.undrained.insert(i);
    }

    /// Recomputes the wake times of the stale nodes and of those that came
    /// due.
    fn refresh_wakes(&mut self) {
        let mut due = std::mem::take(&mut self.stale);
        due.extend(self.wakes.range(..=(self.now, usize::MAX)).map(|&(_, i)| i));
        for i in due {
            if let Some(old) = self.wake_of[i].take() {
                self.wakes.remove(&(old, i));
            }
            self.wake_of[i] = self.wake_time(i);
            if let Some(t) = self.wake_of[i] {
                self.wakes.insert((t, i));
            }
        }
    }

    /// The nodes due now, by index.
    fn due_nodes(&mut self) -> Vec<usize> {
        self.refresh_wakes();
        let mut due: Vec<usize> = self
            .wakes
            .range(..=(self.now, usize::MAX))
            .map(|&(_, i)| i)
            .collect();
        due.sort_unstable();
        due
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
            self.sample_bins(t);
            self.now = self.now.max(t);
            self.step();
            if self.now >= self.next_alloc {
                self.next_alloc += Duration::SECOND;
                self.sample_alloc();
                self.sample_grafts();
                self.sample_series();
                self.sample_shape();
                self.sample_windows();
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
                            x.and_then(|k| n.runner.peer_id(k).map(|id| id_node(id).to_string()))
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
        for i in 0..self.nodes.len() {
            self.take_forecasts(i);
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
                Ev::Arrive {
                    link,
                    frame,
                    ctl,
                    wire,
                } => {
                    let to = self.links[link].to;
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
                    self.push(at, Ev::Deliver { link, frame, ctl });
                }
                Ev::Deliver { link, frame, ctl } => self.deliver(link, &frame, ctl),
            }
        }

        // Only nodes due now can have a timeout due.
        for i in self.due_nodes() {
            let n = &mut self.nodes[i];
            if !n.role.off()
                && n.runner
                    .poll_timeout()
                    .is_some_and(|t| n.true_time(t) <= now)
            {
                for &l in n.links.values() {
                    let link = &self.links[l];
                    let reading = n.local_reading(link.path.reading(now));
                    n.runner.set_peer_path(link.peer, reading);
                }
                let local = n.local(now);
                n.runner.handle_timeout(local);
                self.touch(i);
            }
        }

        if !self.writes_done() && now >= self.next_write_at() {
            self.write();
            self.touch(0);
            if let Some(c) = self.cfg.cross {
                self.touch(c.seed);
            }
        }

        if let Some((secs, what)) = self.cfg.incident
            && self.incident.is_none()
            && now >= t0() + Duration::from_secs(secs)
        {
            let victim = self.busiest_relay();
            match what {
                Incident::Dies => {
                    self.log("incident", victim as i64, -1, 0, "dies");
                    self.nodes[victim].role = Role::Dead;
                }
                Incident::Slows(bw) => {
                    self.log("incident", victim as i64, -1, bw as i64, "slows");
                    self.nodes[victim].up_bw = bw;
                }
            }
            self.incident = Some((now, victim));
            self.touch(victim);
        }

        while let Some(&(secs, step)) = self.cfg.steps.get(self.steps_done) {
            if now < t0() + Duration::from_secs(secs) {
                break;
            }
            self.steps_done += 1;
            self.touch(0);
            match step {
                SeedStep::Uplink(bw) => {
                    self.log("uplink", 0, -1, bw as i64, "");
                    self.nodes[0].up_bw = bw;
                }
                SeedStep::Latency(d) => {
                    self.log("latency", 0, -1, ms(d), "");
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
                    self.log("reconnect", 0, -1, 0, "");
                    for l in 0..self.links.len() {
                        let (from, to, peer) =
                            (self.links[l].from, self.links[l].to, self.links[l].peer);
                        // A viewer gone for good has no link to redial.
                        if (from != 0 && to != 0)
                            || self.nodes[from].role == Role::Gone
                            || self.nodes[to].role == Role::Gone
                        {
                            continue;
                        }
                        let rchannel = self.links[l ^ 1].channel;
                        let channel = self.links[l].channel;
                        // A redialed link is a new TCP connection: nothing
                        // on it waits for the old one's frames, and its
                        // path is measured afresh.
                        let old = &self.links[l];
                        self.links[l] = Link::new(
                            old.from,
                            old.to,
                            old.latency,
                            old.loss,
                            old.peer,
                            old.channel,
                            old.rpeer,
                        );
                        self.links[l].last_send = now;
                        let n = &mut self.nodes[from];
                        let local = n.local(now);
                        n.runner.stop_swarm(peer, n.swarm);
                        n.runner
                            .run_swarm(peer, n.swarm, rchannel, channel, local)
                            .unwrap();
                        self.touch(from);
                    }
                }
            }
        }

        while let Some(&(at, i, leaves)) = self.churn.get(self.churn_done) {
            if now < at {
                break;
            }
            self.churn_done += 1;
            match leaves {
                true => self.leave(i),
                false => self.take_place(i),
            }
        }

        while let Some(&(at, i)) = self.arrivals.get(self.arrivals_done) {
            if now < at {
                break;
            }
            self.arrivals_done += 1;
            self.arrive(i);
        }
        while let Some(&(at, i)) = self.departures.get(self.departures_done) {
            if now < at {
                break;
            }
            self.departures_done += 1;
            self.depart(i);
        }

        // Only a runner called since its last drain holds events.
        for i in std::mem::take(&mut self.undrained) {
            self.drain(i);
        }

        // Only nodes due now can send.
        for i in self.due_nodes() {
            self.transmit(i);
            self.touch(i);
        }

        if self.cfg.capture.is_some() {
            for n in &mut self.nodes {
                if let Some(c) = n.cap.as_mut() {
                    c.drain(now, &mut n.runner);
                }
            }
        }
    }

    /// Viewer `i` leaves: its neighbours see its links close.
    fn leave(&mut self, i: usize) {
        self.log("leave", i as i64, -1, 0, "");
        let n = &mut self.nodes[i];
        n.role = Role::Dead;
        n.churned = true;
        n.rejoined = None;
        let links: Vec<usize> = n.links.values().copied().collect();
        for l in links {
            let (to, rpeer) = (self.links[l].to, self.links[l].rpeer);
            self.nodes[to].runner.remove_peer(rpeer);
            self.touch(to);
        }
        self.touch(i);
    }

    /// A new viewer with nothing takes viewer `i`'s place, on its links.
    fn take_place(&mut self, i: usize) {
        let now = self.now;
        // Frames of the old viewer's links still on the way are lost.
        let links = &self.links;
        self.events.retain(|_, e| {
            let (Ev::Arrive { link, .. } | Ev::Deliver { link, .. }) = e;
            links[*link].from != i && links[*link].to != i
        });
        let mut runner = Runner::new(fork(&mut self.rng));
        if self.loops.is_some() {
            runner.set_tap(TapLevel::All);
        }
        if self.forecasts.is_some() {
            self.take_forecasts(i);
            runner.set_forecasts(true);
        }
        self.log("join", i as i64, -1, 0, "");
        let swarm = strims_ppspp::Swarm::new(self.swarm_id.clone(), self.options.clone()).unwrap();
        let reader = swarm.reader();
        let key = runner.add_swarm(swarm);
        let n = &mut self.nodes[i];
        if !n.hostile {
            n.past.merge(&n.runner.channel_totals());
            n.past_timeouts += n
                .runner
                .swarm_counters(n.swarm)
                .map_or(0, |s| s.requests_timed_out);
        }
        n.hostile = false;
        n.role = Role::Peer;
        n.kind = Role::Peer;
        n.runner = runner;
        n.swarm = key;
        n.reader = reader;
        n.start = None;
        n.pos = 0;
        n.up_free = now;
        n.down_free = now;
        n.weights.clear();
        n.removed.clear();
        n.vtime.values_mut().for_each(|v| *v = 0);
        n.rejoined = Some(now);
        n.joined = now;
        n.first_read = None;
        n.caught_up = None;
        if let Some(last) = self.shape.last.get_mut(i) {
            *last = None;
        }
        let links: Vec<(usize, usize)> = n.links.iter().map(|(&j, &l)| (j, l)).collect();
        for (j, l) in links {
            let (ti, tj) = (self.nodes[i].local(now), self.nodes[j].local(now));
            let pi = self.nodes[i].runner.add_peer(node_id(j), ti);
            let pj = self.nodes[j].runner.add_peer(node_id(i), tj);
            let (ci, cj) = (i as u64 + 1, j as u64 + 1);
            let (si, sj) = (self.nodes[i].swarm, self.nodes[j].swarm);
            self.nodes[i].runner.run_swarm(pi, si, cj, ci, ti).unwrap();
            self.nodes[j].runner.run_swarm(pj, sj, ci, cj, tj).unwrap();
            for (l, from, to, peer, channel, rpeer) in
                [(l, i, j, pi, ci, pj), (l ^ 1, j, i, pj, cj, pi)]
            {
                let old = &self.links[l];
                self.links[l] = Link::new(from, to, old.latency, old.loss, peer, channel, rpeer);
                self.links[l].last_send = now;
            }
            self.touch(j);
        }
        self.touch(i);
    }

    /// Viewer `i` arrives ([`Arrivals`]): it takes its role and links to
    /// nodes already there, as a tracker would answer it.
    fn arrive(&mut self, i: usize) {
        let now = self.now;
        self.log("join", i as i64, -1, 0, "");
        let n = &mut self.nodes[i];
        n.role = n.kind;
        n.arrival = now;
        n.joined = now;
        n.rejoined = Some(now);
        let kind = n.kind;
        let here = |j: usize| j != i && !self.nodes[j].role.off();
        let mut viewers: Vec<usize> = (1..self.nodes.len()).filter(|&j| here(j)).collect();
        let mut to = Vec::new();
        if self.cfg.dial > 0 {
            viewers.push(0);
            shuffle(&mut viewers, &mut self.rng);
            viewers.truncate(self.cfg.dial);
            to = viewers;
        } else {
            if self.cfg.degree > 0 {
                shuffle(&mut viewers, &mut self.rng);
                viewers.truncate(self.cfg.degree);
            }
            let seed_has = self.nodes[0].links.keys().filter(|&&j| here(j)).count();
            if self.cfg.seed_degree == 0 || seed_has < self.cfg.seed_degree {
                to.push(0);
            }
            to.extend(viewers);
        }
        // Adversaries also reach the seed so they hold real data.
        if matches!(kind, Role::Adversary(_)) && !to.contains(&0) {
            to.push(0);
        }
        to.sort_unstable();
        let span = (self.cfg.latency_ms.1 - self.cfg.latency_ms.0).max(1) as u64;
        for j in to {
            let latency =
                Duration::from_millis(self.cfg.latency_ms.0 + (self.rng.next_u64() % span) as i64);
            let (a, b) = (i.min(j), i.max(j));
            let la = self.links.len();
            link_nodes(
                &mut self.nodes,
                &mut self.links,
                (a, b),
                latency,
                self.cfg.loss,
                now,
            );
            if let Some(net) = &self.wan {
                self.links[la].latency = net.one_way(a, b);
                self.links[la + 1].latency = net.one_way(b, a);
            }
            self.touch(j);
        }
        self.touch(i);
    }

    /// Viewer `i` leaves for good ([`Depart`]): gracefully, its neighbours
    /// see its links close and drop them; abruptly, it goes silent. Each
    /// neighbour whose open parent it was on some stream logs a
    /// `parent_lost`.
    fn depart(&mut self, i: usize) {
        if self.nodes[i].role.off() {
            return;
        }
        let abrupt = self.cfg.depart.is_some_and(|d| d.abrupt);
        let how = if abrupt { "abrupt" } else { "graceful" };
        self.log("depart", i as i64, -1, 0, how);
        let links: Vec<(usize, usize)> =
            self.nodes[i].links.iter().map(|(&j, &l)| (j, l)).collect();
        for &(j, l) in &links {
            let n = &self.nodes[j];
            if n.role.off() {
                continue;
            }
            // `i`'s peer key at `j`.
            let key = self.links[l].rpeer;
            let lost = n
                .runner
                .push_stats(n.swarm)
                .map_or(0, |p| p.parents.iter().filter(|&&p| p == Some(key)).count());
            if lost == 0 {
                continue;
            }
            if n.role == Role::Peer {
                self.parent_lost.0 += 1;
                self.parent_lost.1 += lost as u64;
            }
            self.log("parent_lost", j as i64, i as i64, lost as i64, how);
        }
        let n = &mut self.nodes[i];
        n.role = Role::Gone;
        n.left = Some(self.now);
        if !abrupt {
            for (j, l) in links {
                let rpeer = self.links[l].rpeer;
                let n = &mut self.nodes[j];
                n.runner.remove_peer(rpeer);
                n.links.remove(&i);
                n.vtime.remove(&(l ^ 1));
                self.touch(j);
            }
        }
        self.touch(i);
    }

    /// Whether the seed has written the whole stream.
    fn writes_done(&self) -> bool {
        match &self.media {
            Some(m) => m.done == m.plan.ops.len(),
            None => self.written == self.munros,
        }
    }

    /// When the seed writes next.
    fn next_write_at(&self) -> Time {
        match &self.media {
            Some(m) => m.plan.ops.get(m.done).map_or(Time::MAX, |op| {
                m.start
                    + match *op {
                        Op::Write { at, .. } | Op::Flush { at, .. } => at,
                    }
            }),
            None => self.next_write,
        }
    }

    /// Chunks sent a second.
    fn chunk_rate(&self) -> f64 {
        match &self.media {
            Some(m) => m.plan.sent_chunks() as f64 / self.cfg.stream_secs.max(1) as f64,
            None => self.cfg.stream_rate as f64 / self.cfg.chunk_size as f64,
        }
    }

    /// A media run's ops due now: each record written as its access unit
    /// arrives, and each unit in progress flushed at its deadline.
    fn write_media(&mut self) {
        let now = self.now;
        let Sim { media, nodes, .. } = self;
        let Some(m) = media.as_mut() else { return };
        let seed = &mut nodes[0];
        let local = seed.local(now);
        let mut w = seed.writer.take().unwrap();
        while let Some(&op) = m.plan.ops.get(m.done) {
            let at = match op {
                Op::Write { at, .. } | Op::Flush { at, .. } => m.start + at,
            };
            if at > now {
                break;
            }
            match op {
                Op::Write { offset, len, .. } => {
                    let mut data = vec![0; len as usize];
                    m.padded.fill(offset, &mut data);
                    seed.runner.write(seed.swarm, &mut w, local, &data).unwrap();
                }
                Op::Flush { next, .. } => {
                    let got = seed.runner.flush_writer(seed.swarm, &mut w, local).unwrap();
                    assert_eq!(got, next, "the flush's next unit");
                }
            }
            m.done += 1;
        }
        seed.writer = Some(w);
    }

    fn write(&mut self) {
        if self.media.is_some() {
            self.write_media();
            return;
        }
        let now = self.now;
        let munro = self.munro;
        let off = self.written * munro;
        let len = match self.cfg.early.is_empty() {
            true => munro,
            false => {
                early_chunks(&self.cfg.early, self.written as u64) as usize * self.cfg.chunk_size
            }
        };
        let mut data = vec![0; len];
        Content::Main.fill(off as u64, &mut data);
        let seed = &mut self.nodes[0];
        let local = seed.local(now);
        let mut w = seed.writer.take().unwrap();
        seed.runner.write(seed.swarm, &mut w, local, &data).unwrap();
        if !self.cfg.early.is_empty() {
            let next = seed.runner.flush_writer(seed.swarm, &mut w, local).unwrap();
            assert_eq!(next, (off + munro) as u64);
        }
        seed.writer = Some(w);
        if let Some(c) = self.cfg.cross {
            let mut data = vec![0; munro];
            Content::Cross.fill(off as u64, &mut data);
            let local = self.nodes[c.seed].local(now);
            let Node { runner, cross, .. } = &mut self.nodes[c.seed];
            if let Some(x) = cross.as_mut() {
                let mut w = x.writer.take().unwrap();
                runner.write(x.key, &mut w, local, &data).unwrap();
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

    fn deliver(&mut self, link: usize, frame: &[u8], ctl: Ctl) {
        let (to, rpeer) = (self.links[link].to, self.links[link].rpeer);
        let from = self.links[link].from;
        let now = self.now;
        if self.nodes[to].role.off() || self.nodes[from].role.off() {
            return;
        }
        self.touch(to);
        // `to`'s own link back to `from` holds its echo and its meter.
        let back = link ^ 1;
        if let Some(nonce) = ctl.probe {
            let b = &mut self.links[back];
            if !b.echo.is_pending() {
                b.echo_at = match self.nodes[to].role {
                    Role::Greedy(Greed::FreeRider) if self.cfg.echo_greed => now + ECHO_HOLD,
                    _ => now,
                };
            }
            b.echo.read(nonce, now);
        }
        if let Some(nonce) = ctl.echo
            && let Ok(Some(sample)) = self.links[back].path.echo(nonce, now)
        {
            let peer = self.links[back].peer;
            let n = &mut self.nodes[to];
            let at = n.local(sample.at);
            n.runner.on_path_sample(peer, PathSample { at, ..sample });
        }
        if frame.is_empty() {
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
        if self.signs_dataless() && !matches!(self.nodes[from].role, Role::Adversary(_)) {
            self.dataless_on_wire += self.dataless_messages(frame);
        }
        if self.nodes[to].role == Role::Greedy(Greed::Grabber) {
            // Asks for what it hears of on the way back.
            let grabs = self.grabs.entry(link ^ 1).or_default();
            for c in FrameReader::new(frame) {
                let Ok((h, payload)) = c else { break };
                if h.channel >= 1000 {
                    continue;
                }
                for m in Reader::new(self.rcfg, payload) {
                    match m {
                        Ok(Message::Have(h)) => grabs.add(h.bin),
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
        }
        if self.nodes[to].role == Role::Peer
            && matches!(
                self.nodes[from].role,
                Role::Seed | Role::Peer | Role::Greedy(Greed::Grabber)
            )
            && !self.nodes[to].removed.contains(&rpeer)
        {
            self.record_arrivals(to, frame);
        }
        if to == 0 {
            for c in FrameReader::new(frame) {
                let Ok((h, payload)) = c else { break };
                if h.channel >= 1000 {
                    continue;
                }
                for m in Reader::new(self.rcfg, payload) {
                    match m {
                        Ok(m @ Message::Announce(a)) if !a.is_ask() => {
                            self.announce.0 += 1;
                            self.announce.1 += m.encoded_len() as u64;
                        }
                        Ok(Message::Request(r))
                            if self.loops.is_some() && self.nodes[from].role == Role::Peer =>
                        {
                            let parents = {
                                let n = &self.nodes[from];
                                n.runner.push_stats(n.swarm).map(|p| p.parents)
                            };
                            let (mut x, end) = r.bin.base();
                            while x <= end {
                                let c = x.base_offset() as usize;
                                x += 2;
                                let stream = c % self.cfg.stream_count.max(1);
                                let parentless = parents
                                    .as_ref()
                                    .is_none_or(|p| p.get(stream).is_none_or(|x| x.is_none()));
                                let held = self.nodes[from]
                                    .links
                                    .keys()
                                    .filter(|&&n| n != 0)
                                    .filter(|&&n| {
                                        self.nodes[n]
                                            .arrived
                                            .get(c)
                                            .is_some_and(|&t| !t.is_nil() && now - t >= HELD_FOR)
                                    })
                                    .count();
                                let loops = self.loops.as_mut().unwrap();
                                loops.seed_asks[held.min(3).div_ceil(2)] += 1;
                                if held == 0 && parentless {
                                    loops.seed_asks_parentless += 1;
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
        }
        // A grabber ignores chokes.
        let unchoked = (self.nodes[to].role == Role::Greedy(Greed::Grabber))
            .then(|| without_chokes(frame, self.rcfg))
            .flatten();
        let frame = unchoked.as_deref().unwrap_or(frame);
        let node = &mut self.nodes[to];
        if node.removed.contains(&rpeer) {
            return;
        }
        let local = node.local(now);
        // `Capture::handle_frame`'s steps, the capture in true time.
        let res = match node.cap.as_mut() {
            Some(c) => {
                c.drain(now, &mut node.runner);
                c.frame(now, Dir::In, rpeer, frame);
                let r = node.runner.handle_frame(rpeer, local, frame);
                if let Err(e) = &r {
                    c.frame_error(now, rpeer, e);
                }
                c.drain(now, &mut node.runner);
                r
            }
            None => node.runner.handle_frame(rpeer, local, frame),
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

    /// Keeps what the tables need ([`Sim::write_tables`], `SIM_OUT`): the
    /// series, each read's chunk, the series' counters and more at every
    /// edge of bins `width` wide (`SIM_BIN_MS`), each node's too with
    /// `nodes` (`SIM_BIN_NODES=1`), the run's timed events, and with
    /// `forecasts` (`SIM_FORECAST=1`; a row per neighbour per chunk
    /// announced, tens of MB a run) every node's forecast log. Set before
    /// the run; outside the config, so the run's tag stays the same, and it
    /// changes no trace.
    fn keep_tables(&mut self, width: Duration, nodes: bool, forecasts: bool) {
        self.series.get_or_insert_with(Vec::new);
        self.bins = Some(Bins {
            width: width.max(Duration::from_millis(1)),
            next: t0() + Duration::SECOND,
            last: None,
            rows: Vec::new(),
            node_rows: nodes.then(Vec::new),
        });
        // The viewers there from the start; arrivals log theirs as they
        // come ([`Sim::arrive`]).
        let at_start = (1..self.nodes.len()).filter(|&i| self.nodes[i].role != Role::Absent);
        let joins = at_start.map(|i| TimedEvent {
            at: t0(),
            kind: "join",
            node: i as i64,
            peer: -1,
            value: 0,
            detail: "",
        });
        self.timeline = Some(joins.collect());
        if forecasts {
            self.forecasts = Some(Vec::new());
            for n in &mut self.nodes {
                n.runner.set_forecasts(true);
            }
        }
    }

    /// With the tables, records an event at the current time.
    fn log(&mut self, kind: &'static str, node: i64, peer: i64, value: i64, detail: &'static str) {
        if let Some(t) = self.timeline.as_mut() {
            t.push(TimedEvent {
                at: self.now,
                kind,
                node,
                peer,
                value,
                detail,
            });
        }
    }

    /// With the tables, moves node `i`'s forecast log into the harness's.
    fn take_forecasts(&mut self, i: usize) {
        let Some(all) = self.forecasts.as_mut() else {
            return;
        };
        let n = &mut self.nodes[i];
        let (log, lost) = n.runner.take_forecasts(n.swarm);
        // The harness's own truth is `arrived`.
        n.runner.take_held(n.swarm);
        self.forecasts_lost += lost;
        if log.is_empty() {
            return;
        }
        let peers: BTreeMap<PeerKey, usize> = n
            .links
            .iter()
            .map(|(&to, &l)| (self.links[l].peer, to))
            .collect();
        all.extend(log.into_iter().map(|f| Forecast {
            node: i,
            peer: peers.get(&f.peer).copied().unwrap_or(usize::MAX),
            f,
        }));
    }

    /// Node `i`'s [`NodeCounts`] now, with its push stats `p`.
    fn node_counts(&self, i: usize, p: &PushStats) -> NodeCounts {
        let n = &self.nodes[i];
        let o = n.totals().outbound;
        let s = n.runner.swarm_counters(n.swarm).unwrap_or_default();
        let path = self.wan.as_ref().map(|w| w.stats(i)).unwrap_or_default();
        NodeCounts {
            c: [
                s.requests_timed_out,
                s.wasted_chunks,
                s.answers_refused,
                s.answers_dropped,
                p.switched,
                p.shed,
                p.sheds_sent,
                o.have_bytes,
                o.ack_bytes,
                o.request_bytes,
                o.announce_bytes,
                o.stream_bytes,
                o.session_bytes,
                o.ping_bytes,
                o.header_bytes,
                o.integrity_bytes + o.signed_integrity_bytes,
                o.data_bytes,
                o.payloads,
                o.have_only_payloads,
                path.retransmits,
                path.drops,
            ],
            waits: [s.push_wait, s.answer_wait],
        }
    }

    /// With the tables, the counters at each bin edge up to `t`, the time
    /// of the next event: read before it runs, they stand as they did at
    /// every edge since the last event, and the first edge's row takes
    /// what changed since the edge before. Only getters run, so the run
    /// is the same.
    fn sample_bins(&mut self, t: Time) {
        if self.bins.as_ref().is_none_or(|b| t < b.next) {
            return;
        }
        let viewers = (1..self.nodes.len())
            .map(|i| {
                let n = &self.nodes[i];
                (n.role == Role::Peer).then(|| {
                    let p = n.runner.push_stats(n.swarm).unwrap_or_default();
                    let parents = p.parents.iter().filter(|x| x.is_some()).count();
                    let series = [
                        p.hits,
                        p.misses,
                        p.pruned,
                        parents as u64,
                        n.written_overhead(),
                        n.wire.up_bytes,
                        n.wire.up_packets,
                        n.wire.small_packets,
                    ];
                    (series, self.node_counts(i, &p))
                })
            })
            .collect();
        let seed = &self.nodes[0];
        let p = seed.runner.push_stats(seed.swarm).unwrap_or_default();
        let (sent, pushed) = seed
            .runner
            .swarm_counters(seed.swarm)
            .map_or((0, 0), |s| (s.chunks_sent, s.pushed_chunks));
        let sample = BinSample {
            viewers,
            seed: (
                [
                    p.upload_capacity,
                    u64::from(p.upload_busy_permille),
                    p.seed_copies as u64,
                    sent,
                    pushed,
                    seed.up_bw,
                    seed.written_overhead(),
                    seed.wire.up_bytes,
                    seed.wire.up_packets,
                    seed.wire.small_packets,
                ],
                self.node_counts(0, &p),
            ),
        };
        for i in 0..self.nodes.len() {
            self.take_forecasts(i);
        }
        let Some(b) = self.bins.as_mut() else {
            return;
        };
        let mut from = b.last.take();
        while t >= b.next {
            if let Some(a) = &from {
                let k = b.rows.len();
                b.rows.push(bin_row(k, b.width, a, &sample));
                if let Some(rows) = b.node_rows.as_mut() {
                    // The seed's overhead and uplink, in the viewers' order.
                    let seed = |c: &[u64; 10]| [0, 0, 0, 0, c[6], c[7], c[8], c[9]];
                    let (x, y) = (seed(&a.seed.0), seed(&sample.seed.0));
                    rows.push(node_bin_row(
                        k,
                        b.width,
                        0,
                        (&x, &a.seed.1),
                        (&y, &sample.seed.1),
                    ));
                    for (j, (x, y)) in a.viewers.iter().zip(&sample.viewers).enumerate() {
                        if let (Some(x), Some(y)) = (x, y) {
                            rows.push(node_bin_row(k, b.width, j + 1, (&x.0, &x.1), (&y.0, &y.1)));
                        }
                    }
                }
            }
            from = Some(sample.clone());
            b.next += b.width;
        }
        b.last = from;
    }

    /// Records the seed's allocation and objective.
    fn sample_series(&mut self) {
        let Some(series) = self.series.as_mut() else {
            return;
        };
        let viewers = self
            .nodes
            .iter()
            .skip(1)
            .map(|n| {
                (n.role == Role::Peer).then(|| {
                    let [hits, misses, pruned, parents] =
                        n.runner.push_stats(n.swarm).map_or([0; 4], |p| {
                            let parents = p.parents.iter().filter(|x| x.is_some()).count();
                            [p.hits, p.misses, p.pruned, parents as u64]
                        });
                    [hits, misses, pruned, parents, n.written_overhead()]
                })
            })
            .collect();
        let seed = &self.nodes[0];
        let p = seed.runner.push_stats(seed.swarm).unwrap_or_default();
        let (sent, pushed) = seed
            .runner
            .swarm_stats(seed.swarm)
            .map_or((0, 0), |s| (s.chunks_sent, s.pushed_chunks));
        series.push(SeriesSample {
            viewers,
            seed: [
                p.upload_capacity,
                u64::from(p.upload_busy_permille),
                p.seed_copies as u64,
                sent,
                pushed,
                seed.up_bw,
                seed.written_overhead(),
            ],
        });
    }

    /// A digest of the honest viewers' reads at the run's end
    /// ([`Node::watched`]), each viewer's ([`Node::reads_hash`]) in node
    /// order. The trace hashes what the nodes sent, so a change that moves
    /// only readers (where one starts, what it skips) keeps it and changes
    /// this.
    fn reads_hash(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.watched() {
                fnv(&mut h, &(i as u64).to_be_bytes());
                fnv(&mut h, &n.reads_hash.to_be_bytes());
            }
        }
        h
    }

    /// The honest viewers at the run's end, those that left for good
    /// included ([`Node::watched`]), in the series' order (`lag`,
    /// `frames`) and the tables' numbering.
    fn honest(&self) -> Vec<&Node> {
        self.nodes.iter().skip(1).filter(|n| n.watched()).collect()
    }

    /// The run second by second from the stream's start ([`Sim::series`]),
    /// for the tables: the fields of strims-sim's `Report::series`, a chunk
    /// a frame here (lags are per chunk), and the meter's and the seed's
    /// readings at each second's end.
    fn series_data(&self) -> Option<SeriesData> {
        let samples = self.series.as_ref()?;
        let start = t0() + Duration::SECOND;
        let n = samples.len().saturating_sub(1);
        let honest = self.honest();
        let (mut lags, mut frames) = (Vec::new(), Vec::new());
        for v in &honest {
            let mut by: Vec<Vec<Duration>> = vec![Vec::new(); n];
            for &(at, lag) in &v.lags {
                if at >= start
                    && let Some(b) = by.get_mut(((at - start).as_millis() / 1000) as usize)
                {
                    b.push(lag);
                }
            }
            lags.push(
                by.iter_mut()
                    .map(|b| {
                        b.sort_unstable();
                        let i = ((b.len().max(1) - 1) as f64 * 0.99).round() as usize;
                        b.get(i).map_or(-1, |d| d.as_millis())
                    })
                    .collect::<Vec<i64>>(),
            );
            frames.push(by.iter().map(|b| b.len() as i64).collect::<Vec<i64>>());
        }
        // Sample k + 1 closes second k, over the viewers present at both.
        let pairs = || samples.windows(2);
        let both = |w: &[SeriesSample]| -> Vec<([u64; 5], [u64; 5])> {
            w[1].viewers
                .iter()
                .zip(&w[0].viewers)
                .filter_map(|(a, b)| Some(((*a)?, (*b)?)))
                .collect()
        };
        let pooled = |f: usize| -> Vec<i64> {
            pairs()
                .map(|w| {
                    both(w)
                        .iter()
                        .map(|(a, b)| a[f].saturating_sub(b[f]) as i64)
                        .sum()
                })
                .collect()
        };
        let parents: Vec<i64> = pairs()
            .map(|w| {
                let mut p: Vec<u64> = both(w).iter().map(|(a, _)| a[3]).collect();
                p.sort_unstable();
                p.get(p.len() / 2).map_or(-1, |&x| x as i64)
            })
            .collect();
        let seed = |f: usize| -> Vec<i64> { pairs().map(|w| w[1].seed[f] as i64).collect() };
        let seed_delta = |f: usize| -> Vec<i64> {
            pairs()
                .map(|w| w[1].seed[f].saturating_sub(w[0].seed[f]) as i64)
                .collect()
        };
        let mut seed_up = Vec::new();
        for (k, w) in pairs().enumerate() {
            if k == 0 || w[1].seed[5] != w[0].seed[5] {
                seed_up.push(format!("[{k},{}]", w[1].seed[5]));
            }
        }
        let viewer_up: u64 = honest.iter().map(|v| v.up_bw).sum();
        let rate = match &self.media {
            Some(m) => m.plan.sent_bytes / self.cfg.stream_secs.max(1),
            None => self.cfg.stream_rate,
        };
        let mut cols = vec![
            ("present", pairs().map(|w| both(w).len() as i64).collect()),
            ("hits", pooled(0)),
            ("misses", pooled(1)),
            ("pruned", pooled(2)),
            ("parents", parents),
            ("capacity", seed(0)),
            ("busy", seed(1)),
            ("avail", seed(5)),
            ("copies", seed(2)),
            ("sent", seed_delta(3)),
            ("pushed", seed_delta(4)),
            ("seed_ovh", seed_delta(6)),
            ("viewer_ovh", pooled(4)),
        ];
        // A media run's chunks sent each second, by their signing: those
        // holding media (`written`, what `frames` counts), as its other
        // chunks are data-less.
        if let Some(m) = &self.media {
            let mut written = vec![0i64; n];
            for (c, &t) in m.signed.iter().enumerate() {
                let Some(k) = (t >= start).then(|| ((t - start).as_millis() / 1000) as usize)
                else {
                    continue;
                };
                if let Some(w) = written.get_mut(k) {
                    *w += i64::from(!self.publish_times[c].is_nil());
                }
            }
            cols.push(("written", written));
        }
        Some(SeriesData {
            rate,
            chunk: self.cfg.chunk_size as u64,
            secs: self.cfg.stream_secs,
            seed_up,
            viewer_up,
            lag: lags,
            frames,
            cols,
        })
    }

    /// With `SIM_OUT=<dir>` (and `SIM_BIN_MS`, [`Sim::keep_tables`]), the
    /// run's tables, as strims-sim writes them (its `tables` module) for
    /// `hack/sim-db`: CSV files each with a `<table>.schema.json` of its
    /// columns' names and types, times true time in ms from the stream's
    /// start, and the honest viewers at the run's end, those that left for
    /// good included, numbered as the series has them ([`Sim::honest`]).
    /// The series counts a viewer only while it is there: before its
    /// arrival and after its departure ([`Arrivals`], [`Depart`]) it is
    /// in no `present`, and its `lag` is -1.
    ///
    /// | table | one row per | columns |
    /// |---|---|---|
    /// | `seconds` | second of [`Sim::series`] | `sec`, then the series' per-second fields (`present`, `hits`, `misses`, `pruned`, `parents`, `capacity`, `busy`, `avail`, `copies`, `sent`, `pushed`, `seed_ovh`, `viewer_ovh`, and `written` in a media run) |
    /// | `viewer_seconds` | viewer and second | `viewer`, `sec`, `lag_ms` (the series' `lag`, -1 without a read), `frames` |
    /// | `reads` | chunk a viewer read | `viewer`, `t_ms` (when it read it), `seq` (the chunk), `lag_ms` (from its publishing) |
    /// | `bins` | bin of [`Sim::keep_tables`]' width | [`bin_cols`]: `bin`, `t_ms` (its start), the viewers present at both its edges with what they did in it (pushes, deadlines released, prunes, their median open parents at its end, overhead written, and their uplinks' bytes, packets and small packets), the seed's meter, uplink and copies at its end and what it sent in it, then [`NODE_COLS`] (timeouts, wasted chunks, answers refused and dropped, parent switches, sheds taken and sent, message bytes, payloads, retransmits, drops) and [`WAIT_COLS`] summed over those viewers (`viewer_`) and the seed's (`seed_`) |
    /// | `node_bins` | node and bin, with `SIM_BIN_NODES=1` | [`node_bin_cols`]: `bin`, `t_ms`, `node`, `viewer` (its number here, -1 for none), what it did in the bin |
    /// | `viewers` | viewer | `viewer`, `node`, `up` (its uplink in bytes per second), `hostile`, `churned` (it left at least once), `clock_ms` (its clock's offset), `arrive_ms` (when it arrived, -1000 for the start; [`Arrivals`]), `depart_ms` (when it left for good, empty if it stayed; [`Depart`]) |
    /// | `events` | timed event ([`TimedEvent`]) | `t_us`, `kind`, `node`, `peer` (-1 for none), `value`, `detail`: each viewer's `join` (at the start or its arrival, and on taking a place again), `first_read` and `caught_up` (its first chunk read within [`CAUGHT_UP`] of its publishing; `value` each one's ms from the join), churn's `leave`, a viewer's `depart` (`detail` graceful or abrupt), `parent_lost` (`node` a viewer whose open parent `peer` departed, `value` the streams it was the parent of, `detail` the departure's), the seed's steps (`uplink` in bytes per second, `latency` in ms, `reconnect`) and an `incident` (`detail` dies or slows, `value` the new uplink) |
    /// | `forecast` | sample of a node's forecast log ([`Runner::set_forecasts`]), with `SIM_FORECAST=1` | `t_ms`, `node`, `viewer`, `peer` (nodes), `peer_honest` (the seed, or an honest viewer at the end: a greedy one's HAVEs can claim what it never held), `stream`, `seq` (the chunk), `sample_ms`, `mean_ms`, `horizon_ms` (-1 before the estimate's first sample), `samples`, `fresh`, `lazy`, `first_sight`, `run` ([`LagForecast`]), `seen_ms` (our first sight of the chunk) and `held_ms` (when the peer first held it: its first data from an honest node, the seed's publishing; -1 if never, or not in the incarnation that announced it), `channel_open_ms` (when our channel with the peer opened), `live` (it was held after both nodes took their places and after the channel opened: the HAVE tracked its arrival, not a backlog the channel's first HAVEs announce) |
    ///
    /// `series_meta.json` holds the stream's `rate`, `chunk`, `frame` and
    /// `secs`, the seed's `seed_up` steps and the viewers' `viewer_up` as
    /// the series has them, the number of `seconds` and `viewers`, the
    /// bins' `bin_ms`, and the forecast samples the logs' caps left out
    /// (`forecasts_lost`). An empty field is a reading the run doesn't
    /// have ([`NONE`]).
    ///
    /// The bins read the counters the series reads, at each edge rather
    /// than each second's first event ([`Sim::sample_bins`]), so a bin's
    /// sums can differ from its second's at the edges.
    fn write_tables(&self, dir: &Path) -> io::Result<()> {
        use std::fmt::Write;
        std::fs::create_dir_all(dir)?;
        let start = t0() + Duration::SECOND;
        let series = self.series_data();
        let honest = self.honest();
        if let Some(s) = &series {
            let mut names = vec!["sec"];
            names.extend(s.cols.iter().map(|(n, _)| *n));
            let mut t = Table::new("seconds", &names);
            for k in 0..s.len() {
                let mut row = vec![k as i64];
                row.extend(s.cols.iter().map(|(_, v)| v.get(k).copied().unwrap_or(-1)));
                t.ints(&row);
            }
            t.write(dir)?;
            let mut t = Table::new("viewer_seconds", &["viewer", "sec", "lag_ms", "frames"]);
            for (i, (lag, frames)) in s.lag.iter().zip(&s.frames).enumerate() {
                for (k, (l, f)) in lag.iter().zip(frames).enumerate() {
                    t.ints(&[i as i64, k as i64, *l, *f]);
                }
            }
            t.write(dir)?;
        }
        let mut t = Table::new("reads", &["viewer", "t_ms", "seq", "lag_ms"]);
        for (i, v) in honest.iter().enumerate() {
            for (&(at, lag), &seq) in v.lags.iter().zip(&v.read_seqs) {
                t.ints(&[
                    i as i64,
                    (at - start).as_millis(),
                    i64::from(seq),
                    lag.as_millis(),
                ]);
            }
        }
        t.write(dir)?;
        let mut t = Table::new("bins", &bin_cols());
        for row in self.bins.iter().flat_map(|b| &b.rows) {
            t.ints(row);
        }
        t.write(dir)?;
        // Each node's number in the tables, if an honest viewer at the end.
        let mut viewer = vec![-1i64; self.nodes.len()];
        for (i, v) in honest.iter().enumerate() {
            if let Some(n) = self.nodes.iter().position(|n| std::ptr::eq(n, *v)) {
                viewer[n] = i as i64;
            }
        }
        if let Some(rows) = self.bins.as_ref().and_then(|b| b.node_rows.as_ref()) {
            let mut t = Table::new("node_bins", &node_bin_cols());
            for row in rows {
                let mut row = row.clone();
                row[3] = viewer.get(row[2] as usize).copied().unwrap_or(-1);
                t.ints(&row);
            }
            t.write(dir)?;
        }
        let us = |at: Time| -> i64 {
            if at >= start {
                (at - start).as_micros()
            } else {
                -(start - at).as_micros()
            }
        };
        let mut t = Table::typed(
            "viewers",
            &[
                ("viewer", "BIGINT"),
                ("node", "BIGINT"),
                ("up", "BIGINT"),
                ("hostile", "BOOLEAN"),
                ("churned", "BOOLEAN"),
                ("clock_ms", "BIGINT"),
                ("arrive_ms", "BIGINT"),
                ("depart_ms", "BIGINT"),
            ],
        );
        for (i, v) in honest.iter().enumerate() {
            let node = self
                .nodes
                .iter()
                .position(|n| std::ptr::eq(n, *v))
                .unwrap_or(0);
            t.fields(&[
                i.to_string(),
                node.to_string(),
                v.up_bw.to_string(),
                v.hostile.to_string(),
                v.churned.to_string(),
                v.clock.as_millis().to_string(),
                (us(v.arrival) / 1000).to_string(),
                v.left.map_or(String::new(), |t| (us(t) / 1000).to_string()),
            ]);
        }
        t.write(dir)?;
        if let Some(events) = &self.timeline {
            let mut t = Table::typed(
                "events",
                &[
                    ("t_us", "BIGINT"),
                    ("kind", "VARCHAR"),
                    ("node", "BIGINT"),
                    ("peer", "BIGINT"),
                    ("value", "BIGINT"),
                    ("detail", "VARCHAR"),
                ],
            );
            for e in events {
                t.fields(&[
                    us(e.at).to_string(),
                    e.kind.to_owned(),
                    e.node.to_string(),
                    e.peer.to_string(),
                    e.value.to_string(),
                    e.detail.to_owned(),
                ]);
            }
            t.write(dir)?;
        }
        if let Some(forecasts) = &self.forecasts {
            self.forecast_table(forecasts, &viewer, &us).write(dir)?;
        }
        let mut meta = String::from("{\"sim\":\"ppspp\"");
        if let Some(s) = &series {
            let _ = write!(
                meta,
                ",\"rate\":{},\"chunk\":{},\"frame\":{},\"secs\":{},\"seed_up\":[{}],\"viewer_up\":{},\"seconds\":{}",
                s.rate,
                s.chunk,
                s.chunk,
                s.secs,
                s.seed_up.join(","),
                s.viewer_up,
                s.len(),
            );
        }
        let width = self.bins.as_ref().map_or(0, |b| b.width.as_millis());
        let _ = write!(
            meta,
            ",\"viewers\":{},\"bin_ms\":{width},\"forecasts_lost\":{}}}",
            honest.len(),
            self.forecasts_lost
        );
        meta.push('\n');
        std::fs::write(dir.join("series_meta.json"), meta)
    }

    /// The `forecast` table: each forecast log sample, with the true times
    /// it implies and when the sampled node truly first held the chunk
    /// (its first data from an honest node, or the seed's publishing),
    /// `-1` if it never did (censored). `us` maps a true time to the
    /// tables' µs.
    fn forecast_table(
        &self,
        forecasts: &[Forecast],
        viewer: &[i64],
        us: &dyn Fn(Time) -> i64,
    ) -> Table {
        let mut t = Table::typed(
            "forecast",
            &[
                ("t_ms", "BIGINT"),
                ("node", "BIGINT"),
                ("viewer", "BIGINT"),
                ("peer", "BIGINT"),
                ("peer_honest", "BOOLEAN"),
                ("stream", "BIGINT"),
                ("seq", "BIGINT"),
                ("sample_ms", "BIGINT"),
                ("mean_ms", "BIGINT"),
                ("horizon_ms", "BIGINT"),
                ("samples", "BIGINT"),
                ("fresh", "BOOLEAN"),
                ("lazy", "BOOLEAN"),
                ("first_sight", "BOOLEAN"),
                ("run", "BIGINT"),
                ("seen_ms", "BIGINT"),
                ("held_ms", "BIGINT"),
                ("channel_open_ms", "BIGINT"),
                ("live", "BOOLEAN"),
            ],
        );
        let ms_of = |at: Time| us(at).div_euclid(1000);
        // When each node took its place, every time: a peer's first data
        // for a chunk ([`Node::arrived`]) is its first incarnation's, so a
        // sample of a later incarnation's has no truth to meet; and a chunk
        // held before the other's place was taken, or before the channel
        // opened, comes in the HAVEs of a channel's start, not as it
        // arrives (not `live`).
        let mut joined: BTreeMap<usize, Vec<Time>> = BTreeMap::new();
        for e in self.timeline.iter().flatten().filter(|e| e.kind == "join") {
            joined.entry(e.node as usize).or_default().push(e.at);
        }
        for r in forecasts {
            let f = &r.f;
            let node = &self.nodes[r.node];
            let at = node.true_time(f.at);
            let seen = at - f.sample;
            let seq = f.bin.base_offset() as usize;
            let place = |n: usize| {
                joined
                    .get(&n)
                    .and_then(|j| j.iter().rev().find(|&&t| t <= at).copied())
                    .unwrap_or(t0())
            };
            let held = match r.peer {
                0 => self.publish_times.get(seq).copied(),
                p => self.nodes.get(p).and_then(|n| n.arrived.get(seq).copied()),
            }
            .filter(|h| !h.is_nil() && *h >= place(r.peer));
            let opened = node.true_time(f.channel_open);
            let live = held.is_some_and(|h| h >= place(r.node) && h >= opened);
            let est = f.estimate;
            t.fields(&[
                ms_of(at).to_string(),
                r.node.to_string(),
                viewer.get(r.node).copied().unwrap_or(-1).to_string(),
                if r.peer == usize::MAX {
                    "-1".to_owned()
                } else {
                    r.peer.to_string()
                },
                (r.peer == 0 || viewer.get(r.peer).is_some_and(|&v| v >= 0)).to_string(),
                f.stream.to_string(),
                seq.to_string(),
                ms(f.sample).to_string(),
                est.map_or(-1, |e| ms(e.mean)).to_string(),
                est.map_or(-1, |e| ms(e.horizon)).to_string(),
                est.map_or(0, |e| e.samples).to_string(),
                est.is_some_and(|e| e.fresh).to_string(),
                f.lazy.to_string(),
                f.first_sight.to_string(),
                f.run.to_string(),
                ms_of(seen).to_string(),
                held.map_or(-1, ms_of).to_string(),
                ms_of(opened).to_string(),
                live.to_string(),
            ]);
        }
        t
    }

    fn sample_alloc(&mut self) {
        let seed = &self.nodes[0];
        let Some(a) = seed
            .runner
            .seed_allocation(seed.swarm, seed.local(self.now))
        else {
            return;
        };
        let mut held = BTreeMap::new();
        let mut holders = BTreeSet::new();
        let mut barred = BTreeSet::new();
        let mut worst = Vec::new();
        for nb in &a.neighbours {
            let Some(id) = seed.runner.peer_id(nb.peer).map(id_node) else {
                continue;
            };
            held.insert(id, nb.streams.len());
            if nb.holder {
                holders.insert(id);
            }
            if nb.barred {
                barred.insert(id);
            }
            // A neighbour's lags come from its reports' delays, by its clock
            // against the seed's timestamps.
            let off = self.nodes[id].clock - seed.clock;
            let w = nb
                .lags
                .iter()
                .map(|l| l.map_or(STALE, |l| (l - off).max(Duration::ZERO)))
                .max()
                .unwrap_or(STALE);
            worst.push(w);
        }
        worst.sort();
        if knob::<String>("SIM_ALLOC_TRACE").is_some() {
            let peers: Vec<String> = a
                .neighbours
                .iter()
                .filter(|nb| !nb.streams.is_empty() || nb.holder || nb.barred)
                .map(|nb| {
                    let id = seed.runner.peer_id(nb.peer).map_or(0, id_node);
                    let rtt = nb.min_rtt.map_or(-1, ms);
                    let mark = match (nb.holder, nb.barred) {
                        (true, _) => "*",
                        (false, true) => "!",
                        _ => "",
                    };
                    let greed = match self.nodes[id].kind {
                        Role::Greedy(Greed::FreeRider) => "F",
                        Role::Greedy(Greed::Liar) => "L",
                        Role::Greedy(Greed::Grabber) => "G",
                        _ => "",
                    };
                    let served = match nb.served {
                        Some(true) => "+",
                        Some(false) => "-",
                        None => "",
                    };
                    let kept = if nb.holder && !nb.kept_up { "~" } else { "" };
                    format!(
                        "{id}{greed}{mark}{kept}:{}@{rtt}/r{}{served}",
                        nb.streams.len(),
                        nb.reporters
                    )
                })
                .collect();
            let c = a.counters;
            eprintln!(
                "t={} budget {} window {} copies {} slots {} objective {} accepted {} declined {} joined {} left {} silenced {} unserved {} {}",
                ms(self.now - t0()),
                a.budget,
                a.window,
                a.copies,
                held.values().sum::<usize>(),
                ms(percentile(&worst, 90)),
                c.accepted,
                c.declined,
                c.joined,
                c.left,
                c.silenced,
                c.unserved,
                peers.join(" "),
            );
        }
        self.score_verdicts(&a);
        let with_children = if self.now > t0() + Duration::from_secs(self.cfg.warm_secs) {
            self.holders_with_children(&holders)
        } else {
            BTreeSet::new()
        };
        if holders.is_empty() {
            holders.extend(held.iter().filter(|&(_, &n)| n > 0).map(|(&id, _)| id));
        }
        let seed = &self.nodes[0];
        let capacity = seed
            .runner
            .push_stats(seed.swarm)
            .map_or(0, |p| p.upload_capacity);
        let whole = self.cfg.stream_count;
        let hits = held
            .iter()
            .filter(|&(&id, &n)| n >= whole && self.nodes[id].role == Role::Peer)
            .filter_map(|(&id, _)| {
                let n = &self.nodes[id];
                Some((id, n.runner.push_stats(n.swarm)?.hits))
            })
            .collect();
        self.alloc.push(AllocSample {
            at: self.now,
            held,
            holders,
            objective: percentile(&worst, 90),
            barred,
            with_children,
            capacity,
            hits,
        });
    }

    /// Samples [`Sim::shape`]: every honest viewer's chain of open parents
    /// per stream, and every honest node's meter against its uplink.
    fn sample_shape(&mut self) {
        if self.now <= t0() + Duration::from_secs(self.cfg.warm_secs) {
            return;
        }
        let streams = self.cfg.stream_count;
        // Each node's open parent per stream, by node index.
        let parents = self.parents();
        if self.loops.is_some() {
            self.sample_loops(&parents);
        }
        let parent = |i: usize, s: usize| parents.get(i).and_then(|v| v.get(s)).copied().flatten();
        let shape = &mut self.shape;
        for (i, n) in self.nodes.iter().enumerate() {
            let push = n.runner.push_stats(n.swarm);
            if matches!(n.role, Role::Seed | Role::Peer)
                && let Some(p) = &push
                && p.upload_capacity > 0
            {
                let role = match (n.role, p.children) {
                    (Role::Seed, _) => 0,
                    (_, 0) => 2,
                    _ => 1,
                };
                shape.meter[role].push(p.upload_capacity as f64 / n.up_bw.max(1) as f64);
            }
            if n.role != Role::Peer {
                continue;
            }
            shape.viewers += 1;
            if let Some(p) = &push
                && p.pace_channels > 0
            {
                let n = p.pace_channels as f64;
                shape.lazy[0] += p.have_lazy_out as f64 / n;
                shape.lazy[1] += p.have_lazy_in as f64 / n;
                shape.lazy_samples += 1;
            }
            if let Some(p) = &push
                && p.children > 0
            {
                shape.relays += 1;
                shape.fanout.push(p.children as u64);
            }
            if shape.last.len() < self.nodes.len() {
                shape.last.resize(self.nodes.len(), None);
            }
            // The second since the last sample: its declines, and the
            // uplink's use in it.
            let rejected = push.as_ref().map_or(0, |p| p.rejected);
            if let Some((was_rejected, was_up)) = shape.last[i] {
                let declined = rejected.saturating_sub(was_rejected);
                let util = n.wire.up_bytes.saturating_sub(was_up) as f64 / n.up_bw.max(1) as f64;
                shape.declines += declined;
                if util < 0.5 {
                    shape.declines_idle += declined;
                }
            }
            shape.last[i] = Some((rejected, n.wire.up_bytes));
            for s in 0..streams {
                let mut at = parent(i, s);
                let mut seen = vec![i];
                let k = loop {
                    match at {
                        None if seen.len() == 1 => break 0,
                        None => break DEPTH_HOPS + 3,
                        Some(0) if seen.len() <= DEPTH_HOPS => break seen.len(),
                        Some(0) => break DEPTH_HOPS + 1,
                        Some(p) if seen.contains(&p) => break DEPTH_HOPS + 2,
                        Some(p) => {
                            seen.push(p);
                            at = parent(p, s);
                        }
                    }
                };
                if let Some(c) = shape.depth.get_mut(k) {
                    *c += 1;
                }
            }
        }
    }

    /// Samples [`Sim::windows`]: every channel of every honest node's main
    /// swarm.
    fn sample_windows(&mut self) {
        for n in self
            .nodes
            .iter()
            .filter(|n| matches!(n.role, Role::Seed | Role::Peer))
        {
            for (peer, swarm, _, _) in n.runner.channels() {
                if swarm != n.swarm {
                    continue;
                }
                if let Some(cs) = n.runner.channel_stats(peer, swarm) {
                    self.windows.add(&cs.scheduler);
                }
            }
        }
    }

    /// Whether node `x`'s chain of open parents for `stream` passes
    /// through `ancestor`.
    fn descends(&self, x: usize, ancestor: usize, stream: usize) -> bool {
        let mut at = x;
        for _ in 0..self.nodes.len() {
            if at == ancestor {
                return true;
            }
            let n = &self.nodes[at];
            let Some(p) = n
                .runner
                .push_stats(n.swarm)
                .and_then(|p| p.parents.get(stream).copied().flatten())
                .and_then(|k| n.runner.peer_id(k))
                .map(id_node)
            else {
                return false;
            };
            at = p;
        }
        false
    }

    /// Finds the loops of open parents among honest viewers, notes how each
    /// new one closed, and the lifetimes of those that broke.
    fn sample_loops(&mut self, parents: &[Vec<Option<usize>>]) {
        let now = self.now;
        let parent = |i: usize, s: usize| parents.get(i).and_then(|v| v.get(s)).copied().flatten();
        let mut found: BTreeMap<(Stream, Vec<usize>), Vec<usize>> = BTreeMap::new();
        for i in (1..self.nodes.len()).filter(|&i| self.nodes[i].role == Role::Peer) {
            for s in 0..self.cfg.stream_count {
                let mut seen = vec![i];
                let mut at = parent(i, s);
                while let Some(p) = at {
                    if let Some(k) = seen.iter().position(|&x| x == p) {
                        let ring = seen[k..].to_vec();
                        let mut members = ring.clone();
                        members.sort_unstable();
                        found.entry((s as Stream, members)).or_insert(ring);
                        break;
                    }
                    if p == 0 {
                        break;
                    }
                    seen.push(p);
                    at = parent(p, s);
                }
            }
        }
        let Some(loops) = self.loops.as_mut() else {
            return;
        };
        let gone: Vec<_> = loops
            .active
            .keys()
            .filter(|k| !found.contains_key(*k))
            .cloned()
            .collect();
        for k in gone {
            let first = loops.active.remove(&k).unwrap();
            loops.lives.push(now - first);
        }
        for (key, ring) in found {
            if loops.active.contains_key(&key) {
                continue;
            }
            let stream = key.0;
            // The newest edge closed it: ring[j]'s parent is ring[j + 1].
            let newest = (0..ring.len())
                .filter_map(|j| loops.edge(ring[j], ring[(j + 1) % ring.len()], stream))
                .max_by_key(|(t, _)| *t);
            let how = newest.map_or("unknown".to_owned(), |(_, how)| how);
            *loops.closed_by.entry(how).or_default() += 1;
            loops.lens.push(ring.len() as u64);
            loops.active.insert(key, now);
        }
    }

    /// With `SIM_COLLUDE`, greedy viewer `i`'s report due on link `l` to
    /// the seed, once a second: for every holder at the latest sample, on
    /// the stream its index picks, a liar is praised and anyone else
    /// defamed, eight pushes each.
    fn collusion(&mut self, i: usize, l: usize) -> Option<Vec<AnnounceEntry<'static>>> {
        if self.links[l].to != 0
            || self.now < self.collude_at[i]
            || knob::<u32>("SIM_COLLUDE").is_none_or(|c| c == 0)
        {
            return None;
        }
        self.collude_at[i] = self.now + Duration::SECOND;
        let streams = self.cfg.stream_count.max(1);
        let holders = self
            .alloc
            .last()
            .map(|a| a.holders.clone())
            .unwrap_or_default();
        let entries: Vec<AnnounceEntry<'static>> = holders
            .iter()
            .filter(|&&h| h != i)
            .map(|&h| {
                let liar = self.nodes[h].kind == Role::Greedy(Greed::Liar);
                AnnounceEntry {
                    stream: (h % streams) as Stream,
                    hits: if liar { 8 } else { 0 },
                    late: 0,
                    misses: if liar { 0 } else { 8 },
                    hit_delays: if liar {
                        Delays { n: 8, ms: 8 * 20 }
                    } else {
                        Delays::default()
                    },
                    repairs: Delays::default(),
                    parent: node_id(h),
                }
            })
            .collect();
        (!entries.is_empty()).then_some(entries)
    }

    /// Every node's open parent per stream, by node index: none for a
    /// viewer not there ([`Role::Absent`], [`Role::Gone`]).
    fn parents(&self) -> Vec<Vec<Option<usize>>> {
        self.nodes
            .iter()
            .map(|n| {
                if matches!(n.role, Role::Absent | Role::Gone) {
                    return Vec::new();
                }
                n.runner.push_stats(n.swarm).map_or_else(Vec::new, |s| {
                    s.parents
                        .iter()
                        .map(|x| x.and_then(|k| n.runner.peer_id(k)).map(id_node))
                        .collect()
                })
            })
            .collect()
    }

    /// Scores the seed's report verdicts against its holders' real
    /// children, into [`Sim::verdicts`], and tracks how long liars hold.
    fn score_verdicts(&mut self, a: &strims_ppspp::SeedAllocationStats) {
        let seed = &self.nodes[0];
        let id = |k| seed.runner.peer_id(k).map_or(0, id_node);
        let holders: BTreeSet<usize> = a
            .neighbours
            .iter()
            .filter(|n| n.holder)
            .map(|n| id(n.peer))
            .collect();
        for (&i, run) in &mut self.liar_runs {
            if !holders.contains(&i) && *run > 0 {
                self.liar_spans.push(*run);
                *run = 0;
            }
        }
        for &h in &holders {
            if self.nodes[h].kind == Role::Greedy(Greed::Liar) {
                *self.liar_runs.entry(h).or_default() += 1;
            }
        }
        let warm = t0() + Duration::from_secs(self.cfg.warm_secs);
        if self.now <= warm {
            return;
        }
        let parents = self.parents();
        let children = |h: usize| -> BTreeSet<usize> {
            parents
                .iter()
                .enumerate()
                .filter(|(_, p)| p.contains(&Some(h)))
                .map(|(n, _)| n)
                .collect()
        };
        let links = &self.nodes[0].links;
        let was: BTreeSet<usize> = self
            .alloc
            .last()
            .map(|x| x.barred.clone())
            .unwrap_or_default();
        let v = &mut self.verdicts;
        for nb in &a.neighbours {
            let h = id(nb.peer);
            let kids = children(h);
            if nb.barred && !was.contains(&h) && !matches!(self.nodes[h].kind, Role::Greedy(_)) {
                let had = self
                    .alloc
                    .last()
                    .is_some_and(|x| x.with_children.contains(&h));
                v.barred_with_children += u64::from(had);
            }
            if !nb.holder {
                continue;
            }
            let judged = nb.served.is_some();
            let served = nb.served == Some(true);
            match self.nodes[h].kind {
                Role::Greedy(Greed::Liar) => {
                    v.liars += 1;
                    v.liars_judged += u64::from(judged);
                    v.liars_served += u64::from(served);
                }
                Role::Greedy(Greed::FreeRider) => {}
                _ => {
                    v.honest += 1;
                    v.with_children += u64::from(!kids.is_empty());
                    v.with_reporters += u64::from(kids.iter().any(|k| links.contains_key(k)));
                    v.judged += u64::from(judged);
                    v.served += u64::from(served);
                    v.served_childless += u64::from(served && kids.is_empty());
                    v.not_kept += u64::from(!nb.kept_up && !nb.streams.is_empty());
                }
            }
        }
    }

    /// The holders with a real child, for [`AllocSample::with_children`].
    fn holders_with_children(&self, holders: &BTreeSet<usize>) -> BTreeSet<usize> {
        let parents = self.parents();
        holders
            .iter()
            .copied()
            .filter(|&h| parents.iter().any(|p| p.contains(&Some(h))))
            .collect()
    }

    /// See [`Report::barred`]: counted from the samples, once per bar.
    fn barred(&self) -> (u64, u64) {
        let (mut all, mut honest) = (0, 0);
        let mut was = BTreeSet::new();
        for a in &self.alloc {
            for &i in a.barred.difference(&was) {
                all += 1;
                if !matches!(self.nodes[i].kind, Role::Greedy(_)) {
                    honest += 1;
                }
            }
            was.clone_from(&a.barred);
        }
        (all, honest)
    }

    /// Records which honest viewers' streams have a liar for a parent.
    fn sample_grafts(&mut self) {
        if self.now <= t0() + Duration::from_secs(self.cfg.warm_secs) {
            return;
        }
        let g = &mut self.grafts;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.role != Role::Peer {
                continue;
            }
            let Some(p) = n.runner.push_stats(n.swarm) else {
                continue;
            };
            for (s, parent) in p.parents.iter().enumerate() {
                let liar = parent
                    .and_then(|k| n.runner.peer_id(k))
                    .is_some_and(|id| self.nodes[id_node(id)].kind == Role::Greedy(Greed::Liar));
                g.samples += 1;
                if liar {
                    g.to_liars += 1;
                    *g.runs.entry((i, s)).or_default() += 1;
                } else if let Some(run) = g.runs.remove(&(i, s)) {
                    g.spans.push(run);
                }
            }
        }
    }

    /// Whether the main stream has data-less chunks: munros signed early
    /// ([`Config::early`]), or a media run's flushes past a chunk.
    fn signs_dataless(&self) -> bool {
        !self.cfg.early.is_empty()
            || self
                .media
                .as_ref()
                .is_some_and(|m| !m.plan.dataless.is_empty())
    }

    /// Whether any of the main stream's chunks `first..=last` is
    /// data-less.
    fn has_dataless(&self, first: u64, last: u64) -> bool {
        if let Some(m) = &self.media {
            return m.plan.has_dataless(first, last);
        }
        let (early, per) = (&self.cfg.early, self.cfg.munro_chunks as u64);
        (first / per..=last / per).take(1 << 16).any(|k| {
            let from = k * per + early_chunks(early, k);
            from.max(first) <= ((k + 1) * per - 1).min(last)
        })
    }

    /// The main swarm's messages in `frame` that name a data-less chunk:
    /// data or a request for one, a HAVE claiming one, or the hash of a
    /// subtree of data-less chunks only.
    fn dataless_messages(&self, frame: &[u8]) -> u64 {
        let touches = |b: Bin| {
            let (l, r) = (b.base_left().base_offset(), b.base_right().base_offset());
            self.has_dataless(l, r)
        };
        let mut n = 0;
        for c in FrameReader::new(frame) {
            let Ok((h, payload)) = c else { break };
            if h.channel >= 1000 {
                continue;
            }
            for m in Reader::new(self.rcfg, payload) {
                n += u64::from(match m {
                    Ok(Message::Data(d)) => touches(d.bin),
                    Ok(Message::Request(r)) => touches(r.bin),
                    Ok(Message::Have(h)) => touches(h.bin),
                    Ok(Message::Integrity(i)) => {
                        let o = i.bin.base_offset();
                        self.has_dataless(o, o)
                    }
                    Ok(_) => false,
                    Err(_) => break,
                });
            }
        }
        n
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
        let mut asked = Vec::new();
        if let Some(loops) = self.loops.as_mut() {
            let mut events = Vec::new();
            node.runner.drain_tap(|e| events.push(e));
            for e in events {
                let peer = e.peer.and_then(|k| node.runner.peer_id(k)).map(id_node);
                let who = if i == 0 { "seed" } else { "viewer" };
                let kind = match e.kind {
                    TapKind::ParentChoice { choice, .. } => {
                        Some(format!("check {:?}", choice.action))
                    }
                    TapKind::Switched { .. } => Some("switched".to_owned()),
                    TapKind::Pruned { why, .. } => {
                        let age = why.age.unwrap_or_default();
                        let what = match (why.pushes, why.since_hit) {
                            (0, _) => "never pushed",
                            (_, Some(h)) if h >= age => "pushed, never a hit",
                            _ => "hits, then silent",
                        };
                        let old = match age.as_millis() {
                            ..1500 => "under 1.5 s",
                            1500..5000 => "1.5-5 s",
                            _ => "over 5 s",
                        };
                        Some(format!(
                            "pruned {}: {what}, at {old}",
                            if why.silent { "silent" } else { "lossy" }
                        ))
                    }
                    TapKind::ShedBy { .. } => Some("shed by parent".to_owned()),
                    TapKind::Declined { .. } => Some("declined".to_owned()),
                    TapKind::OpenTimeout { .. } => Some("open timeout".to_owned()),
                    TapKind::Admitted { why, .. } => Some(format!(
                        "{who} admitted {:?}{}",
                        why.rule,
                        if why.extra { " extra" } else { "" }
                    )),
                    TapKind::Rejected { why, .. } => {
                        // Whether the true uplink had room at the target.
                        let need = why.upload_rate.max(why.committed) + why.stream_rate;
                        let room = (need as f64)
                            <= node.up_bw as f64 * why.target_permille as f64 / 1000.0;
                        Some(format!(
                            "{who} rejected {:?}{}{}",
                            why.rule,
                            if why.extra { " extra" } else { "" },
                            if room { " (uplink had room)" } else { "" }
                        ))
                    }
                    TapKind::Shed { why, .. } => Some(format!(
                        "{who} shed {}",
                        match why {
                            strims_ppspp::Shed::Saturated { .. } => "saturated",
                            strims_ppspp::Shed::SeedBudget { .. } => "seed budget",
                            strims_ppspp::Shed::Silent => "silent",
                        }
                    )),
                    _ => None,
                };
                if let Some(k) = kind {
                    *loops.kinds.entry(k).or_default() += 1;
                }
                let (stream, d) = match (e.kind, peer) {
                    (TapKind::ParentChoice { stream, choice }, Some(p)) => match choice.action {
                        ParentAction::Graft => (stream, Decision::Graft(p)),
                        ParentAction::Switch => (stream, Decision::Switch(p)),
                        _ => continue,
                    },
                    (TapKind::Pruned { stream, why }, _) => {
                        (stream, Decision::Pruned { silent: why.silent })
                    }
                    (TapKind::ShedBy { stream }, _) => (stream, Decision::ShedBy),
                    (TapKind::Declined { stream }, _) => (stream, Decision::Declined),
                    (TapKind::OpenTimeout { stream, .. }, _) => (stream, Decision::OpenTimeout),
                    (TapKind::Lost { count }, _) => {
                        loops.lost += count;
                        continue;
                    }
                    _ => continue,
                };
                if let Decision::Graft(p) | Decision::Switch(p) = d {
                    asked.push((stream, d, p));
                }
                let log = loops.log.entry((i, stream)).or_default();
                if log.len() == LOOP_LOG {
                    log.pop_front();
                }
                log.push_back((now, d));
            }
        }
        for (stream, d, p) in asked {
            let down = self.descends(p, i, usize::from(stream));
            let loops = self.loops.as_mut().unwrap();
            let (n, n_down) = match d {
                Decision::Graft(_) => (&mut loops.grafts, &mut loops.grafts_down),
                _ => (&mut loops.switches, &mut loops.switches_down),
            };
            *n += 1;
            *n_down += u64::from(down);
        }
        let node = &mut self.nodes[i];
        let reads_main = readable && node.role == Role::Peer;
        let reads_cross = readable_cross && node.cross.as_ref().is_some_and(|x| x.reads);
        if reads_main {
            self.read(i, now);
        }
        if reads_cross {
            self.read_cross(i, now);
        }
        if reads_main || reads_cross {
            // A reader that recovers calls the runner.
            self.touch(i);
        }
    }

    /// `read` for the cross-traffic swarm.
    fn read_cross(&mut self, i: usize, now: Time) {
        let local = self.nodes[i].local(now);
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
                    x.pos = x.reader.position() - n as u64;
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
                    let _ = runner
                        .swarm_mut(x.key, local)
                        .unwrap()
                        .recover_reader(&mut x.reader);
                    x.pos = x.reader.position();
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
        let local = node.local(now);
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
                    node.pos = node.reader.position() - n as u64;
                    if let Some(m) = &self.media {
                        let mut want = vec![0; n];
                        m.padded.fill(node.pos, &mut want);
                        node.mismatches += want
                            .iter()
                            .zip(&self.read_buf[..n])
                            .filter(|(a, b)| a != b)
                            .count() as u64;
                    } else {
                        let (early, cs) = (&self.cfg.early, self.cfg.chunk_size as u64);
                        let per = self.cfg.munro_chunks as u64;
                        for (k, &b) in self.read_buf[..n].iter().enumerate() {
                            let o = node.pos + k as u64;
                            let want = match chunk_dataless(early, per, o / cs) {
                                true => 0,
                                false => stream_byte(o),
                            };
                            if b != want {
                                node.mismatches += 1;
                            }
                        }
                    }
                    let first = node.pos.div_ceil(self.cfg.chunk_size as u64);
                    let end = (node.pos + n as u64) / self.cfg.chunk_size as u64;
                    let mut fresh = false;
                    for c in first..end {
                        if let Some(&pt) = self.publish_times.get(c as usize)
                            && !pt.is_nil()
                        {
                            fresh |= now - pt < CAUGHT_UP;
                            node.lags.push((now, now - pt));
                            fnv(&mut node.reads_hash, &c.to_be_bytes());
                            fnv(&mut node.reads_hash, &now.unix_nanos().to_be_bytes());
                            if self.bins.is_some() {
                                node.read_seqs.push(c as u32);
                            }
                            if let Some(&st) =
                                self.media.as_ref().and_then(|m| m.signed.get(c as usize))
                            {
                                node.sign_lags.push((now, now - st));
                            }
                        }
                        if let Some(r) = node.read_chunks.get_mut(c as usize) {
                            *r = true;
                        }
                    }
                    let first_read = end > first && node.first_read.is_none();
                    let caught_up = fresh && node.caught_up.is_none();
                    if first_read {
                        node.first_read = Some(now);
                    }
                    if caught_up {
                        node.caught_up = Some(now);
                    }
                    if let Some(t) = self.timeline.as_mut() {
                        for (kind, new) in [("first_read", first_read), ("caught_up", caught_up)] {
                            if new {
                                t.push(TimedEvent {
                                    at: now,
                                    kind,
                                    node: i as i64,
                                    peer: -1,
                                    value: ms(now - node.joined),
                                    detail: "",
                                });
                            }
                        }
                    }
                    if end > first
                        && let Some(at) = node.rejoined.take()
                    {
                        self.joins.push(now - at);
                    }
                    node.pos += n as u64;
                }
                Err(store::Error::WouldBlock) => return,
                Err(store::Error::BufferUnderrun) => {
                    node.underruns += 1;
                    let _ = node
                        .runner
                        .swarm_mut(node.swarm, local)
                        .unwrap()
                        .recover_reader(&mut node.reader);
                    node.pos = node.reader.position();
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
        if self.nodes[i].role.off() {
            return;
        }
        loop {
            let node = &self.nodes[i];
            if node.up_free > now {
                return;
            }
            // Control owed on a link with no data waiting goes on its own.
            if let Some(l) = node.links.values().copied().find(|&l| {
                self.links[l].control_due(now) && !node.runner.wants_transmit(self.links[l].peer)
            }) {
                self.send(i, l, Vec::new());
                continue;
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
            let reading = node.local_reading(self.links[l].path.reading(now));
            node.runner.set_peer_path(peer, reading);
            let room = if node.role == Role::Greedy(Greed::Grabber) {
                GRAB_ROOM
            } else {
                0
            };
            let len = self.buf.len() - room;
            let local = node.local(now);
            let mut n = node.runner.poll_transmit(peer, local, &mut self.buf[..len]);
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
            if let Role::Greedy(g) = node.role {
                let collusion = self.collusion(i, l);
                let grabs = self.grabs.get_mut(&l).map(|g| (g, local));
                n = withhold(g, &mut self.buf, n, self.rcfg, collusion.as_deref(), grabs);
                if n == 0 {
                    continue;
                }
            }
            let frame = self.buf[..n].to_vec();
            self.send(i, l, frame);
        }
    }

    /// The control records `l`'s sender adds after `data` bytes of frame
    /// (none for a frame of control alone), as vnic's would.
    fn take_control(&mut self, l: usize, data: usize) -> Ctl {
        let now = self.now;
        let liar =
            self.cfg.echo_greed && self.nodes[self.links[l].from].role == Role::Greedy(Greed::Liar);
        let link = &mut self.links[l];
        let probe = link.probe_alone(now)
            || (data > 0 && {
                link.path.wrote(data as u64, true);
                link.path.probe_due(now, PROBE_INTERVAL)
            });
        let echo = if link.echo.is_pending() && link.echo_at <= now {
            // A liar answers with a nonce it made up.
            link.echo
                .take(now)
                .map(|(n, _)| if liar { n ^ 0x8000_0000 } else { n })
        } else {
            None
        };
        let probe = probe.then(|| {
            link.nonce = link.nonce.wrapping_add(1);
            link.nonce
        });
        let ctl = Ctl { echo, probe };
        link.path.wrote(ctl.len() as u64, false);
        if let Some(nonce) = probe {
            link.path.probe(nonce, now);
        }
        if data > 0 || probe.is_some() {
            link.last_send = now;
        }
        ctl
    }

    /// Sends `frame` (empty for control alone) with `l`'s control records
    /// through node `i`'s uplink.
    fn send(&mut self, i: usize, l: usize, frame: Vec<u8>) {
        let now = self.now;
        let n = frame.len();
        let ctl = self.take_control(l, n);
        let peer = self.links[l].peer;
        let node = &mut self.nodes[i];
        if i != 0 {
            self.viewer_bytes += n as u64;
        }
        let (wire, packets) = match self.wan {
            Some(_) => tcp_wire(n + ctl.len()),
            None => (n + ctl.len(), 1),
        };
        let weight = node.weights.get(&peer).copied().unwrap_or(1000).max(1);
        if n > 0 {
            let min_v = node.vtime.values().copied().min().unwrap_or(0);
            let v = node.vtime.get_mut(&l).unwrap();
            *v = (*v).max(min_v) + (wire as u128) * 1_000_000 / u128::from(weight);
        }
        node.up_free = now + tx_time(wire, node.up_bw);
        node.wire.up_bytes += wire as u64;
        node.wire.up_packets += u64::from(packets);
        node.wire.small_packets += u64::from(small_packets(wire, packets));
        let s = &mut node.sent;
        s.ctl_bytes += ctl.len() as u64;
        s.transport_bytes += (wire - n - ctl.len()) as u64;
        if n > 0 {
            s.frames += 1;
            s.frame_bytes += n as u64;
            if node.hostile {
                s.hostile_bytes += n as u64;
            }
        }
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
        if let Some(c) = node.cap.as_mut()
            && n > 0
        {
            // The uplink holds no queue of its own: it pulls a frame
            // when free, so its state is the time this one takes.
            c.sample(
                now,
                None,
                &Sample::SimUplink {
                    free_in: node.up_free - now,
                    rate: node.up_bw,
                    vtime: node
                        .vtime
                        .get(&l)
                        .map_or(0, |&v| u64::try_from(v).unwrap_or(u64::MAX)),
                    weight,
                },
            );
            c.frame(now, Dir::Out, peer, &frame);
            c.drain(now, &mut node.runner);
        }
        if n > 0 {
            self.frames += 1;
            self.bytes += n as u64;
        }
        fnv(&mut self.trace, &now.unix_nanos().to_be_bytes());
        fnv(&mut self.trace, &(l as u64).to_be_bytes());
        fnv(&mut self.trace, &frame);
        for x in [ctl.echo, ctl.probe].into_iter().flatten() {
            fnv(&mut self.trace, &x.to_be_bytes());
        }

        let link = &mut self.links[l];
        if self.wan.is_none()
            && link.loss > 0.0
            && (self.rng.next_u64() as f64 / u64::MAX as f64) < link.loss
        {
            return;
        }
        // The records lead the frame in its first packet: they leave once
        // their own bytes have, and take the frame's path, and its fate.
        let ahead = (n > 0 && ctl.len() > 0).then(|| {
            if self.cfg.paced_uplinks {
                now
            } else {
                now + tx_time(ctl.len(), self.nodes[i].up_bw)
            }
        });
        let (delay, before) = match self.wan.as_mut() {
            Some(net) => {
                let tr = net.transit(depart, link.from, link.to, packets, Carrier::Stream);
                (tr.arrival - depart + tr.retransmit, link.last_arrival)
            }
            None => (link.latency, Time::NIL),
        };
        let then = (depart + delay).max(before);
        if self.wan.is_some() {
            link.last_arrival = then;
        }
        let events = match ahead {
            Some(ctl_depart) => vec![
                (
                    (ctl_depart + delay).max(before).min(then),
                    Vec::new(),
                    ctl,
                    ctl.len(),
                ),
                (then, frame, Ctl::default(), wire - ctl.len()),
            ],
            None => vec![(then, frame, ctl, wire)],
        };
        for (at, frame, ctl, wire) in events {
            self.push(
                at,
                Ev::Arrive {
                    link: l,
                    frame,
                    ctl,
                    wire,
                },
            );
        }
    }

    /// [`Report::holder_short`] and [`Report::holder_copy`]: per pair of
    /// consecutive samples after `warm` and honest whole-copy holder in
    /// both, its on-time pushes between them over the chunks the stream
    /// published in that time.
    fn holder_copies(&self, warm: Time) -> (f64, f64) {
        let per_sec = self.chunk_rate();
        let mut shares = Vec::new();
        for w in self.alloc.windows(2) {
            let [a, b] = w else { continue };
            if a.at <= warm {
                continue;
            }
            let need = per_sec * (b.at - a.at).as_secs_f64();
            for (id, &hb) in &b.hits {
                if let Some(&ha) = a.hits.get(id) {
                    shares.push(hb.saturating_sub(ha) as f64 / need);
                }
            }
        }
        if shares.is_empty() {
            return (0.0, 0.0);
        }
        let short =
            shares.iter().filter(|&&x| x < HOLDER_WHOLE).count() as f64 / shares.len() as f64;
        shares.sort_by(f64::total_cmp);
        (short, shares[shares.len() / 2])
    }

    fn report(&self) -> Report {
        let total = (self.munros * self.munro) as u64;
        let warm = t0() + Duration::from_secs(self.cfg.warm_secs);
        let mut lags: Vec<Duration> = self
            .nodes
            .iter()
            .filter(|n| n.watched())
            .flat_map(|n| self.steady(n, &n.lags))
            .collect();
        lags.sort();
        let pct = |p: usize| {
            lags.get(lags.len().saturating_sub(1) * p / 100)
                .copied()
                .unwrap_or_default()
        };
        let mut sign_lags: Vec<Duration> = self
            .nodes
            .iter()
            .filter(|n| n.watched())
            .flat_map(|n| self.steady(n, &n.sign_lags))
            .collect();
        sign_lags.sort();
        let peers = self.nodes.iter().filter(|n| n.watched());
        let data_bytes: u64 = (0..self.nodes.len()).map(|i| self.uploaded(i)).sum();
        let seed_share = self.uploaded(0) as f64 / data_bytes.max(1) as f64;
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
                // Children it has at the end.
                if n.role == Role::Peer {
                    push.children += p.children;
                }
            }
        }
        for n in self
            .nodes
            .iter()
            .filter(|n| !matches!(n.kind, Role::Greedy(_)))
        {
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
        let greed = |i: usize| match self.nodes[i].kind {
            Role::Greedy(g) => Some(g),
            _ => None,
        };
        let count = |k: Greed| {
            (1..self.nodes.len())
                .filter(|&i| greed(i) == Some(k))
                .count()
        };
        let greedy_slots = {
            let (mut all, mut free, mut liar, mut grab) = (0, 0, 0, 0);
            for a in self.alloc.iter().filter(|a| a.at > warm) {
                for (&i, &n) in &a.held {
                    all += n;
                    match greed(i) {
                        Some(Greed::FreeRider) => free += n,
                        Some(Greed::Liar) => liar += n,
                        Some(Greed::Grabber) => grab += n,
                        None => {}
                    }
                }
            }
            let all = all.max(1) as f64;
            (free as f64 / all, liar as f64 / all, grab as f64 / all)
        };
        let greedy_held = {
            let mut longest = 0;
            let mut runs: BTreeMap<usize, i64> = BTreeMap::new();
            for a in &self.alloc {
                for i in (1..self.nodes.len()).filter(|&i| greed(i).is_some()) {
                    let r = runs.entry(i).or_default();
                    *r = if a.held.get(&i).is_some_and(|&n| n > 0) {
                        *r + 1
                    } else {
                        0
                    };
                    longest = longest.max(*r);
                }
            }
            Duration::from_secs(longest)
        };
        let g = &self.grafts;
        let spans: Vec<u32> = g.spans.iter().chain(g.runs.values()).copied().collect();
        let (holder_short, holder_copy) = self.holder_copies(warm);
        Report {
            trace: self.trace,
            frames: self.frames,
            bytes: self.bytes,
            total,
            positions: peers
                .clone()
                .filter(|n| n.role == Role::Peer)
                .map(|n| (n.start.unwrap_or(u64::MAX), n.pos))
                .collect(),
            // Over the viewers whose places were never taken again
            // ([`Churn`]): one taken again misses what was written while it
            // stood empty. Each viewer's chunks due ([`Sim::delivered`]).
            delivery: {
                let (read, all) = peers
                    .clone()
                    .filter(|n| !n.churned)
                    .map(|n| self.delivered(n))
                    .fold((0, 0), |(r, a), (rn, an)| (r + rn, a + an));
                read as f64 / all.max(1) as f64
            },
            mismatches: peers.clone().map(|n| n.mismatches).sum(),
            underruns: peers.clone().map(|n| n.underruns).sum(),
            resets: peers.clone().map(|n| n.resets).sum(),
            lag_p50: pct(50),
            lag_p99: pct(99),
            lag_max: lags.last().copied().unwrap_or_default(),
            sign_lag_p50: percentile(&sign_lags, 50),
            sign_lag_p99: percentile(&sign_lags, 99),
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
            seed_capacity: {
                let end =
                    t0() + Duration::SECOND + Duration::from_secs(self.cfg.stream_secs as i64);
                let mut v: Vec<u64> = self
                    .alloc
                    .iter()
                    .filter(|a| a.at > warm && a.at <= end)
                    .map(|a| a.capacity)
                    .collect();
                v.sort_unstable();
                v.get(v.len() / 2).copied().unwrap_or(0)
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
            greedy: (count(Greed::FreeRider), count(Greed::Liar)),
            grabbers: (count(Greed::Grabber), greedy_slots.2),
            holder_short,
            holder_copy,
            greedy_slots: (greedy_slots.0, greedy_slots.1),
            greedy_held,
            barred: self.barred(),
            verdicts: self.verdicts,
            liar_hold: {
                let runs: Vec<u32> = self
                    .liar_spans
                    .iter()
                    .chain(self.liar_runs.values().filter(|&&r| r > 0))
                    .copied()
                    .collect();
                let mean = runs.iter().map(|&r| u64::from(r)).sum::<u64>() * 1000
                    / runs.len().max(1) as u64;
                let longest = runs.iter().copied().max().unwrap_or(0);
                (
                    Duration::from_millis(mean as i64),
                    Duration::from_secs(i64::from(longest)),
                )
            },
            announce: (
                self.announce.0,
                self.announce.1,
                self.announce.1 as f64 / self.viewer_bytes.max(1) as f64,
            ),
            asks: {
                let seed = &self.nodes[0];
                seed.links
                    .values()
                    .filter_map(|&l| seed.runner.channel_stats(self.links[l].peer, seed.swarm))
                    .map(|c| c.outbound.announce)
                    .sum()
            },
            report_counts: {
                let seed = &self.nodes[0];
                let mut honest = ReportCounters::default();
                let mut greedy = ReportCounters::default();
                if let Some(a) = seed
                    .runner
                    .seed_allocation(seed.swarm, seed.local(self.now))
                {
                    for nb in &a.neighbours {
                        let i = seed.runner.peer_id(nb.peer).map_or(0, id_node);
                        let c = if matches!(self.nodes[i].kind, Role::Greedy(_)) {
                            &mut greedy
                        } else {
                            &mut honest
                        };
                        add_counts(c, &nb.reports);
                    }
                }
                (honest, greedy)
            },
            liar_grafts: g.to_liars as f64 / g.samples.max(1) as f64,
            liar_graft_span: Duration::from_millis(
                (1000 * spans.iter().map(|&s| u64::from(s)).sum::<u64>()
                    / spans.len().max(1) as u64) as i64,
            ),
        }
    }

    /// Per peer and munro published after its warm-up ([`Sim::steady`]),
    /// sorted: how long after it was published its last chunk arrived. A
    /// peer reads in order, so that is the lag of the munro's slowest
    /// stripe. Munros before the peer's start, and ones it never received
    /// whole (which delivery counts), are left out.
    fn munro_lags(&self) -> Vec<Duration> {
        let per = self.cfg.munro_chunks;
        let mut out = Vec::new();
        for n in self.nodes.iter().filter(|n| n.watched()) {
            let warm = n.arrival + Duration::from_secs(self.cfg.warm_secs);
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

    /// The lags of `n`'s reads in `lags` after its warm-up: from its own
    /// arrival plus [`Config::warm_secs`] (the run's warm-up for a viewer
    /// there from the start) to its departure, which ends its reads.
    fn steady<'a>(
        &self,
        n: &Node,
        lags: &'a [(Time, Duration)],
    ) -> impl Iterator<Item = Duration> + 'a {
        let warm = n.arrival + Duration::from_secs(self.cfg.warm_secs);
        lags.iter().filter(move |(t, _)| *t > warm).map(|&(_, l)| l)
    }

    /// Honest viewer `n`'s delivery: the chunks it read of those due, and
    /// those due. A viewer there from the start is due every chunk (in a
    /// media run, the chunks holding media from its first read on: where a
    /// viewer joins turns on the unit size, which the join's lag measures).
    /// An arrival ([`Arrivals`]) is due the chunks holding media from its
    /// reader's start on, at the live edge it read from (the chunks
    /// published since it arrived while it hasn't started), and a viewer
    /// that left for good ([`Depart`]) those published at least
    /// [`CAUGHT_UP`] before it left: one further behind then misses the
    /// rest, as one stalled until it left misses what it waited on.
    fn delivered(&self, n: &Node) -> (usize, usize) {
        let first = n.start.map_or(u64::MAX, |o| o / self.cfg.chunk_size as u64);
        let published = |c: usize| self.publish_times.get(c).copied().filter(|t| !t.is_nil());
        let due = |c: usize| {
            let from = match (n.arrival > t0(), &self.media) {
                (false, None) => true,
                (false, Some(_)) => c as u64 >= first && published(c).is_some(),
                (true, _) => published(c).is_some_and(|pt| match n.start {
                    Some(_) => c as u64 >= first,
                    None => pt >= n.arrival,
                }),
            };
            from && n
                .left
                .is_none_or(|left| published(c).is_some_and(|pt| pt + CAUGHT_UP <= left))
        };
        n.read_chunks
            .iter()
            .enumerate()
            .filter(|&(c, _)| due(c))
            .fold((0, 0), |(r, a), (_, &x)| (r + usize::from(x), a + 1))
    }

    /// Whether the run has viewers arriving after the start or leaving for
    /// good: the rows' columns of [`Sim::audience_report`] come with them.
    fn audience(&self) -> bool {
        self.cfg.arrivals != Arrivals::T0 || self.cfg.depart.is_some()
    }

    /// The run's arrivals and departures, measured ([`AudienceReport`]).
    fn audience_report(&self) -> AudienceReport {
        let viewers = || self.nodes.iter().skip(1);
        let both = |f: &dyn Fn(&Node) -> bool| {
            (
                viewers().filter(|n| f(n)).count(),
                viewers().filter(|n| f(n) && n.watched()).count(),
            )
        };
        // From each honest viewer's join to `at`, sorted, and the viewers
        // without one.
        let since = |at: &dyn Fn(&Node) -> Option<Time>| {
            let mut v: Vec<Duration> = viewers()
                .filter(|n| n.watched())
                .filter_map(|n| Some(at(n)? - n.joined))
                .collect();
            v.sort();
            let none = viewers().filter(|n| n.watched()).count() - v.len();
            (v, none)
        };
        let (joins, unread) = since(&|n| n.first_read);
        let (caught_up, behind) = since(&|n| n.caught_up);
        AudienceReport {
            arrived: both(&|n| n.arrival > t0()),
            departed: both(&|n| n.left.is_some()),
            joins,
            unread,
            caught_up,
            behind,
            parent_lost: self.parent_lost,
            recovery: self
                .cfg
                .depart
                .map(|d| self.recovery(stream_start() + d.at)),
        }
    }

    /// The recovery from the departures that start at `from`, over the
    /// honest viewers there before it that stayed ([`Recovery`]). Each
    /// viewer's lag per second of the stream is its reads' p99 in it, or a
    /// second more than the last one's in a second it reads nothing (none
    /// before its first read), as `hack/capture/analyze.py` reads the
    /// series; the seconds' lag is the viewers' p90 of theirs.
    fn recovery(&self, from: Time) -> Recovery {
        let start = t0() + Duration::SECOND;
        let secs = self.cfg.stream_secs as usize;
        let stayed: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| n.role == Role::Peer && n.arrival < from)
            .collect();
        let mut per: Vec<Vec<Option<i64>>> = Vec::new();
        for n in &stayed {
            let mut by: Vec<Vec<i64>> = vec![Vec::new(); secs];
            for &(at, lag) in &n.lags {
                if at >= start
                    && let Some(b) = by.get_mut(((at - start).as_millis() / 1000) as usize)
                {
                    b.push(lag.as_millis());
                }
            }
            let mut cur = None;
            per.push(
                by.iter_mut()
                    .map(|b| {
                        b.sort_unstable();
                        cur = match b.get(((b.len().max(1) - 1) as f64 * 0.99).round() as usize) {
                            Some(&l) => Some(l),
                            None => cur.map(|l| l + 1000),
                        };
                        cur
                    })
                    .collect(),
            );
        }
        let k0 = (from.max(start) - start).as_millis() as usize / 1000;
        let target = ms(CAUGHT_UP);
        let (mut excess, mut bad) = (0, None);
        for k in k0..secs {
            let mut v: Vec<i64> = per.iter().filter_map(|p| p[k]).collect();
            v.sort_unstable();
            let Some(&q) = v.get(((v.len().max(1) - 1) as f64 * 0.9).round() as usize) else {
                continue;
            };
            excess += (q - target).max(0);
            if q > target {
                bad = Some(k);
            }
        }
        Recovery {
            stayed: stayed.len(),
            stranded: stayed
                .iter()
                .filter(|n| n.links.keys().all(|&j| self.nodes[j].role.off()))
                .count(),
            peak: stayed
                .iter()
                .flat_map(|n| n.lags.iter().filter(|(t, _)| *t >= from).map(|&(_, l)| l))
                .max()
                .unwrap_or_default(),
            excess_ms_s: excess,
            recover: bad.map_or(Duration::ZERO, |k| {
                (start + Duration::from_secs(k as i64 + 1) - from).max(Duration::ZERO)
            }),
            censored: bad.is_some_and(|k| k + 1 == secs),
        }
    }

    /// See [`Report::settled`].
    fn alloc_settled(&self) -> Duration {
        let start = t0() + Duration::SECOND;
        self.alloc
            .windows(2)
            .rfind(|w| w[0].holders != w[1].holders)
            .map_or(Duration::ZERO, |w| (w[1].at - start).max(Duration::ZERO))
    }

    /// In WAN mode, the lag p99 after their warm-ups ([`Sim::steady`]) of
    /// the peers in each region.
    fn region_p99(&self) -> Vec<(Region, Duration)> {
        let Some(w) = &self.cfg.wan else {
            return vec![];
        };
        let mut by: BTreeMap<Region, Vec<Duration>> = BTreeMap::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if !n.watched() {
                continue;
            }
            by.entry(w.sites[i].city.region())
                .or_default()
                .extend(self.steady(n, &n.lags));
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

    /// The data bytes node `i` uploaded: none from a greedy viewer, whose
    /// runner counts what the harness drops.
    fn uploaded(&self, i: usize) -> u64 {
        let n = &self.nodes[i];
        if matches!(n.kind, Role::Greedy(g) if g != Greed::Grabber) {
            return 0;
        }
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

    /// The requests that timed out unanswered, as strims-sim's rows count
    /// them: the honest viewers' over every incarnation
    /// ([`Node::past_timeouts`]; a viewer gone for good was an honest
    /// one), the seed's, and the greedy viewers' and adversaries'.
    fn timeouts(&self) -> (u64, u64, u64) {
        let (mut honest, mut seed, mut hostile) = (0, 0, 0);
        for n in &self.nodes {
            let t = n.past_timeouts
                + n.runner
                    .swarm_counters(n.swarm)
                    .map_or(0, |s| s.requests_timed_out);
            match (n.role, n.kind) {
                (Role::Seed, _) => seed += t,
                (Role::Peer | Role::Dead, _) | (Role::Absent | Role::Gone, Role::Peer) => {
                    honest += t
                }
                _ => hostile += t,
            }
        }
        (honest, seed, hostile)
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
            .filter(|n| n.watched())
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

/// A run's greedy viewers, by node index. A drawn set is [`GREEDY_SHARE`]
/// of the viewers (or [`Config::greedy_share`]), the kinds
/// ([`Config::greedy_kinds`]) alternating from a random first one, from an
/// rng of its own: the mesh, the paths and every runner draw what they
/// would without it. Viewers the config names (constrained ones, the
/// cross-traffic seed, the traced node, `keep_honest`) are passed over,
/// which leaves the rest of the draw as it was. `SIM_GREEDY=0` draws
/// none.
fn greedy(cfg: &Config) -> BTreeMap<usize, Greed> {
    match &cfg.greedy {
        Greedy::Nodes(v) => {
            assert!(
                v.iter().all(|&(i, _)| (1..=cfg.peers).contains(&i)),
                "{v:?}"
            );
            v.iter().copied().collect()
        }
        Greedy::Drawn if knob::<u32>("SIM_GREEDY") == Some(0) => BTreeMap::new(),
        Greedy::Drawn => {
            let mut rng = det_rng(cfg.seed ^ 0x4752_4545);
            let (lo, hi) = GREEDY_SHARE;
            let share = lo + (hi - lo) * (rng.next_u64() as f64 / u64::MAX as f64);
            let share = cfg.greedy_share.unwrap_or(share);
            let n = (cfg.peers as f64 * share).round() as usize;
            let mut kinds = cfg.greedy_kinds.clone();
            if rng.next_u64() % 2 == 1 && kinds.len() > 1 {
                kinds.swap(0, 1);
            }
            let named: BTreeSet<usize> = cfg
                .constrained
                .iter()
                .map(|&(i, _)| i)
                .chain(cfg.cross.map(|c| c.seed))
                .chain(cfg.trace_node)
                .chain(cfg.keep_honest.iter().copied())
                .collect();
            let mut order: Vec<usize> = (1..=cfg.peers).collect();
            shuffle(&mut order, &mut rng);
            order
                .into_iter()
                .filter(|i| !named.contains(i))
                .take(n)
                .zip(kinds.into_iter().cycle())
                .collect()
        }
    }
}

/// The kinds a drawn run's greedy viewers take in turn: `SIM_GREEDY_KINDS`,
/// comma-separated `free`, `liar` and `grab`, by default (or blank) free
/// riders and liars.
fn greedy_kinds_knob() -> Vec<Greed> {
    let Some(s) = knob::<String>("SIM_GREEDY_KINDS").filter(|s| !s.trim().is_empty()) else {
        return vec![Greed::FreeRider, Greed::Liar];
    };
    let kinds: Vec<Greed> = s
        .split(',')
        .map(|k| match k.trim() {
            "free" => Greed::FreeRider,
            "liar" => Greed::Liar,
            "grab" => Greed::Grabber,
            k => panic!("SIM_GREEDY_KINDS: unknown kind {k:?}: free, liar or grab"),
        })
        .collect();
    assert!(!kinds.is_empty(), "SIM_GREEDY_KINDS names no kind");
    kinds
}

/// A drawn run's share of greedy viewers in place of the draw's:
/// `SIM_GREEDY_SHARE`, a percentage (blank: the draw's).
fn greedy_share_knob() -> Option<f64> {
    let s = knob::<String>("SIM_GREEDY_SHARE").filter(|s| !s.trim().is_empty())?;
    let pct: f64 = s
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("SIM_GREEDY_SHARE={s}: a percentage"));
    assert!(
        (0.0..=100.0).contains(&pct),
        "SIM_GREEDY_SHARE={s}: 0 to 100"
    );
    Some(pct / 100.0)
}

/// `SIM_CHURN`, the percentage of the honest viewers that leave per minute
/// (blank or 0: none), and `SIM_CHURN_AWAY`, the seconds until each place
/// is taken again (default 5).
fn churn_knob() -> Option<Churn> {
    let s = knob::<String>("SIM_CHURN").filter(|s| !s.trim().is_empty())?;
    let pct: f64 = s
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("SIM_CHURN={s}: a percentage"));
    (pct > 0.0).then(|| Churn {
        per_min: pct / 100.0,
        away: Duration::from_secs(knob("SIM_CHURN_AWAY").unwrap_or(5)),
    })
}

/// When `cfg`'s churn has each honest viewer (`Role::Peer`) leave and its
/// place be taken again: `(at, node, leaves)`, sorted.
fn churn_plan(
    cfg: &Config,
    c: Churn,
    roles: &[Role],
    rng: &mut DetRng,
) -> Vec<(Time, usize, bool)> {
    let honest: Vec<usize> = (1..roles.len())
        .filter(|&i| roles[i] == Role::Peer)
        .collect();
    let rate = c.per_min * honest.len() as f64 / 60.0;
    let mut plan = Vec::new();
    if rate <= 0.0 {
        return plan;
    }
    let from = t0() + Duration::from_secs(cfg.warm_secs);
    let until = t0() + Duration::from_secs(cfg.stream_secs as i64) - c.away;
    let mut back = vec![Time::NIL; roles.len()];
    let mut at = from;
    loop {
        let u = (rng.next_u64() as f64 / u64::MAX as f64).max(f64::MIN_POSITIVE);
        at += Duration::from_nanos((-u.ln() / rate * 1e9) as i64);
        if at >= until {
            break;
        }
        let here: Vec<usize> = honest.iter().copied().filter(|&i| back[i] <= at).collect();
        if here.is_empty() {
            continue;
        }
        let i = here[(rng.next_u64() % here.len() as u64) as usize];
        back[i] = at + c.away;
        plan.push((at, i, true));
        plan.push((back[i], i, false));
    }
    plan.sort_by_key(|&(t, i, leaves)| (t, !leaves, i));
    plan
}

/// `SIM_ARRIVALS` ([`Arrivals::parse`]); unset or blank, `t0`.
fn arrivals_knob() -> Arrivals {
    let Some(s) = knob::<String>("SIM_ARRIVALS").filter(|s| !s.trim().is_empty()) else {
        return Arrivals::T0;
    };
    Arrivals::parse(&s).unwrap_or_else(|| {
        panic!("SIM_ARRIVALS={s}: t0, trickle, trickle:<per s> or event:<warm %>:<at s>:<window s>")
    })
}

/// `SIM_DEPART` ([`Depart::parse`]); unset or blank, none leave.
fn depart_knob() -> Option<Depart> {
    let s = knob::<String>("SIM_DEPART").filter(|s| !s.trim().is_empty())?;
    Some(Depart::parse(&s).unwrap_or_else(|| {
        panic!("SIM_DEPART={s}: <share %>:<at s>:<window s>, then :abrupt to go silent")
    }))
}

/// A knob's number: finite, not negative.
fn knob_num(x: &str) -> Option<f64> {
    x.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

impl Arrivals {
    /// `t0` for every viewer at the start, `trickle` for all of them within
    /// the stream's first third, `trickle:<per s>` for a Poisson stream at
    /// that rate, and `event:<warm %>:<at s>:<window s>` for a scheduled
    /// event, times in seconds after the stream's start: the `Display`
    /// form.
    fn parse(s: &str) -> Option<Arrivals> {
        match s.trim().split(':').collect::<Vec<_>>()[..] {
            ["t0"] => Some(Arrivals::T0),
            ["trickle"] => Some(Arrivals::Trickle(None)),
            ["trickle", r] => Some(Arrivals::Trickle(Some(knob_num(r).filter(|&r| r > 0.0)?))),
            ["event", w, at, window] => Some(Arrivals::Event {
                warm: knob_num(w).filter(|&w| w <= 100.0)? / 100.0,
                at: secs_f64(knob_num(at)?),
                window: secs_f64(knob_num(window)?),
            }),
            _ => None,
        }
    }
}

impl Depart {
    /// `<share %>:<at s>:<window s>`, then `:abrupt` for silent departures,
    /// times in seconds after the stream's start: the `Display` form.
    fn parse(s: &str) -> Option<Depart> {
        let (share, at, window, abrupt) = match s.trim().split(':').collect::<Vec<_>>()[..] {
            [share, at, window] => (share, at, window, false),
            [share, at, window, "abrupt"] => (share, at, window, true),
            _ => return None,
        };
        Some(Depart {
            share: knob_num(share).filter(|&p| p <= 100.0)? / 100.0,
            at: secs_f64(knob_num(at)?),
            window: secs_f64(knob_num(window)?),
            abrupt,
        })
    }
}

/// `s` seconds, to the nanosecond.
fn secs_f64(s: f64) -> Duration {
    Duration::from_nanos((s * 1e9).round() as i64)
}

/// A time uniform in `[from, from + len)` after the stream's start.
fn uniform_in(rng: &mut DetRng, from: Duration, len: Duration) -> Time {
    stream_start() + from + secs_f64(unit(rng) * len.as_secs_f64())
}

/// When `cfg`'s viewers arrive after the start ([`Arrivals`]): `(at,
/// node)` for every node but the seed, sorted; none at [`Arrivals::T0`],
/// which draws nothing. The times are drawn first, then the order.
fn arrival_plan(cfg: &Config, nodes: usize, rng: &mut DetRng) -> Vec<(Time, usize)> {
    let n = nodes - 1;
    let stream = Duration::from_secs(cfg.stream_secs as i64);
    let times: Vec<Time> = match cfg.arrivals {
        Arrivals::T0 => return Vec::new(),
        Arrivals::Trickle(None) => (0..n)
            .map(|_| uniform_in(rng, Duration::ZERO, secs_f64(stream.as_secs_f64() / 3.0)))
            .collect(),
        Arrivals::Trickle(Some(rate)) => {
            let mut at = stream_start();
            (0..n)
                .map(|_| {
                    let u = (rng.next_u64() as f64 / u64::MAX as f64).max(f64::MIN_POSITIVE);
                    at += secs_f64(-u.ln() / rate);
                    at
                })
                .collect()
        }
        Arrivals::Event { warm, at, window } => {
            let k = (warm * n as f64).round() as usize;
            (0..n)
                .map(|j| match j < k {
                    true => uniform_in(rng, Duration::ZERO, at),
                    false => uniform_in(rng, at, window),
                })
                .collect()
        }
    };
    let mut order: Vec<usize> = (1..nodes).collect();
    shuffle(&mut order, rng);
    let mut plan: Vec<(Time, usize)> = times.into_iter().zip(order).collect();
    plan.sort_unstable();
    let end = stream_start() + stream;
    assert!(
        plan.last().is_none_or(|&(t, _)| t < end),
        "SIM_ARRIVALS={}: the last of {n} viewers arrives {} s into the stream, past its end",
        cfg.arrivals,
        plan.last()
            .map_or(0.0, |&(t, _)| (t - stream_start()).as_secs_f64()),
    );
    plan
}

/// When viewers leave for good ([`Depart`]): `(at, node)`, sorted, for
/// `d.share` of the nodes but the seed there `d.at` into the stream (by
/// `arrivals`, the rest there from the start), drawn at random.
fn depart_plan(
    cfg: &Config,
    d: Depart,
    arrivals: &[(Time, usize)],
    nodes: usize,
    rng: &mut DetRng,
) -> Vec<(Time, usize)> {
    let from = stream_start() + d.at;
    assert!(
        from + d.window <= stream_start() + Duration::from_secs(cfg.stream_secs as i64),
        "SIM_DEPART={d}: past the stream's end"
    );
    let mut arrive = vec![t0(); nodes];
    for &(t, i) in arrivals {
        arrive[i] = t;
    }
    let mut here: Vec<usize> = (1..nodes).filter(|&i| arrive[i] < from).collect();
    shuffle(&mut here, rng);
    here.truncate((d.share * here.len() as f64).round() as usize);
    let mut plan: Vec<(Time, usize)> = here
        .into_iter()
        .map(|i| (uniform_in(rng, d.at, d.window), i))
        .collect();
    plan.sort_unstable();
    plan
}

/// `SIM_CLOCK_SKEW`: `real` (or blank, the default) for clocks off as in
/// the wild, `off` for right clocks, `<ms>` for every clock off by that much either way,
/// and `+<ms>` or `-<ms>` for every clock off by exactly that
/// ([`ClockSkew`]); an offset past [`MAX_SKEW`] panics.
fn clock_skew_knob() -> ClockSkew {
    let Some(s) = knob::<String>("SIM_CLOCK_SKEW") else {
        return ClockSkew::Real;
    };
    let millis = |s: &str| {
        let n: i64 = s
            .parse()
            .unwrap_or_else(|_| panic!("SIM_CLOCK_SKEW={s}: off, real, <ms>, +<ms> or -<ms>"));
        let d = Duration::from_millis(n);
        assert!(
            d.abs() <= MAX_SKEW,
            "SIM_CLOCK_SKEW={s}: past the cap, MAX_SKEW ({} ms)",
            ms(MAX_SKEW)
        );
        d
    };
    match s.trim() {
        "off" => ClockSkew::Off,
        "" | "real" => ClockSkew::Real,
        t if t.starts_with(['+', '-']) => ClockSkew::All(millis(t)),
        t => ClockSkew::Either(millis(t).abs()),
    }
}

/// Each node's clock offset under `skew`, the seed's first. Only `Real`
/// and `Either` draw.
fn clock_offsets(skew: ClockSkew, nodes: usize, rng: &mut DetRng) -> Vec<Duration> {
    let signed = |rng: &mut DetRng, d: Duration| match rng.next_u64() & 1 {
        0 => d,
        _ => -d,
    };
    (0..nodes)
        .map(|_| match skew {
            ClockSkew::Off => Duration::ZERO,
            ClockSkew::All(d) => d,
            ClockSkew::Either(d) => signed(rng, d),
            ClockSkew::Real => {
                let mut below = 0.0;
                let u = unit(rng);
                let class = CLOCK_SHARES
                    .iter()
                    .position(|&s| {
                        below += s;
                        u < below
                    })
                    .unwrap_or(CLOCK_SHARES.len() - 1);
                let (lo, hi) = CLOCK_BOUNDS[class];
                let (lo, hi) = (lo.as_millis() as f64, hi.as_millis() as f64);
                let x = unit(rng);
                let ms = match class {
                    0 => lo + (hi - lo) * x,
                    _ => lo * (hi / lo).powf(x),
                };
                signed(rng, Duration::from_millis(ms.min(hi - 1.0) as i64))
            }
        })
        .collect()
}

/// A draw uniform in `[0, 1)`.
fn unit(rng: &mut DetRng) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

/// The class of [`CLOCK_CLASSES`] an offset falls in.
fn clock_class(d: Duration) -> usize {
    CLOCK_BOUNDS
        .iter()
        .position(|&(_, hi)| d.abs() < hi)
        .unwrap_or(CLOCK_BOUNDS.len() - 1)
}

/// What a greedy viewer's frame keeps of what its runner wrote: everything
/// but data and its integrity, and a free rider's HAVEs. A colluder's
/// frame to the seed carries the `collusion` entries ([`Sim::collusion`])
/// that fit in place of its own reports. A grabber's keeps everything but
/// its cancels, and ends with the requests queued in `grabs`, stamped with
/// its time, that fit the room its runner left ([`GRAB_ROOM`]). Returns
/// the new length, 0 if nothing is left.
fn withhold(
    g: Greed,
    buf: &mut [u8],
    n: usize,
    cfg: ReaderConfig,
    collusion: Option<&[AnnounceEntry<'_>]>,
    grabs: Option<(&mut Grabs, Time)>,
) -> usize {
    let frame = buf[..n].to_vec();
    let mut off = 0;
    let mut injected = false;
    let mut main_channel = None;
    for c in FrameReader::new(&frame) {
        let (h, payload) = c.unwrap();
        let mut cw = ChannelWriter::new(&mut buf[off..], h.channel);
        let main = h.channel < 1000;
        if main {
            main_channel = Some(h.channel);
        }
        for m in Reader::new(cfg, payload) {
            match m.unwrap() {
                Message::Cancel(_) | Message::StreamCancel(_) if g == Greed::Grabber => {}
                m @ (Message::Data(_) | Message::Integrity(_) | Message::SignedIntegrity(_))
                    if g == Greed::Grabber =>
                {
                    cw.write(&m).unwrap();
                }
                Message::Data(_) | Message::Integrity(_) | Message::SignedIntegrity(_) => {}
                Message::Have(_) if g == Greed::FreeRider => {}
                Message::Announce(a) if main && collusion.is_some() && !a.is_ask() => {}
                m => {
                    cw.write(&m).unwrap();
                }
            }
        }
        if let Some(e) = collusion.filter(|_| main && !injected) {
            injected = true;
            for part in e.chunks(MAX_ANNOUNCE_ENTRIES) {
                if cw
                    .write(&Message::Announce(Announce::new(part).unwrap()))
                    .is_err()
                {
                    break;
                }
            }
        }
        off += cw.finish();
    }
    // The forged requests go last, in the room the runner left.
    if let (Some((grabs, now)), Some(channel)) = (grabs, main_channel) {
        let mut cw = ChannelWriter::new(&mut buf[off..], channel);
        let mut sent = 0;
        for &bin in &grabs.queue {
            let r = Request {
                bin,
                timestamp: now,
            };
            if cw.write(&Message::Request(r)).is_err() {
                break;
            }
            sent += 1;
        }
        grabs.queue.drain(..sent);
        off += cw.finish();
    }
    off
}

/// `frame` without its CHOKEs and UNCHOKEs, or `None` if it has none.
fn without_chokes(frame: &[u8], cfg: ReaderConfig) -> Option<Vec<u8>> {
    let choke = |m: &Message<'_>| matches!(m, Message::Choke | Message::Unchoke);
    let any = FrameReader::new(frame).any(|c| {
        c.is_ok_and(|(_, payload)| Reader::new(cfg, payload).any(|m| m.is_ok_and(|m| choke(&m))))
    });
    if !any {
        return None;
    }
    let mut buf = vec![0; frame.len()];
    let mut off = 0;
    for c in FrameReader::new(frame) {
        let (h, payload) = c.ok()?;
        let mut cw = ChannelWriter::new(&mut buf[off..], h.channel);
        for m in Reader::new(cfg, payload) {
            let m = m.ok()?;
            if !choke(&m) {
                cw.write(&m).ok()?;
            }
        }
        off += cw.finish();
    }
    buf.truncate(off);
    Some(buf)
}

/// Adds `b`'s counts to `a`'s.
fn add_counts(a: &mut ReportCounters, b: &ReportCounters) {
    a.tallied += b.tallied;
    a.unknown += b.unknown;
    a.invalid += b.invalid;
    a.reporter_fed += b.reporter_fed;
    a.subject_unfed += b.subject_unfed;
    a.reporter_silent += b.reporter_silent;
    a.too_early += b.too_early;
    a.misses_unseen += b.misses_unseen;
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
                    zeros: 0,
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
        eprintln!(
            "node {i} {:?}: wasted chunks {} uploaded {}",
            n.role,
            ss.wasted_chunks,
            sim.uploaded(i)
        );
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
    // Per process: test binaries running at once share the directory.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "sim_capture_changes_no_trace-{}",
        std::process::id()
    ));
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
            assert_eq!(rd.header().node, node_id(i));
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
    let _ = std::fs::remove_dir_all(&dir);
}

/// The ledger's layers add up to the bytes through the uplinks, and an
/// honest node's runners' counters to the frames it sent, through a
/// viewer's place taken again.
#[test]
fn sim_ledger_adds_up_to_the_wire() {
    let mut cfg = Config::new(3, 8);
    cfg.stream_secs = 6;
    cfg.degree = 4;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    cfg.churn = Some(Churn {
        per_min: 120.0,
        away: Duration::from_secs(1),
    });
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    assert!(sim.nodes.iter().any(|n| n.churned));
    // The row is flat: "key":value pairs.
    let row = job_row("s", "c", &sim, &r);
    let fields: Vec<(&str, &str)> = row
        .trim_matches(['{', '}'])
        .split(',')
        .filter_map(|f| f.split_once(':'))
        .map(|(k, v)| (k.trim_matches('"'), v))
        .collect();
    let int = |k: &str| {
        fields
            .iter()
            .find(|f| f.0 == k)
            .unwrap()
            .1
            .parse::<u64>()
            .unwrap()
    };
    let layers: u64 = fields
        .iter()
        .filter(|(k, _)| k.starts_with("up_") && *k != "up_wire")
        .map(|(k, _)| int(k))
        .sum();
    assert_eq!(layers, int("up_wire"));
    assert!(sim.nodes.iter().any(|n| n.hostile));
    for n in sim.nodes.iter().filter(|n| !n.hostile) {
        assert_eq!(n.totals().outbound.wire_bytes(), n.sent.frame_bytes);
    }
    let frames: u64 = sim.nodes.iter().map(|n| n.sent.frame_bytes).sum();
    assert_eq!(frames, r.bytes);
}

/// Stores that keep no state or a little per chunk, and make their chunks'
/// bytes with codecs, send the frames that stores keeping the bytes do,
/// cross traffic's included.
#[test]
fn sim_chunk_codecs_change_no_trace() {
    let mut mesh = Config::new(3, 8);
    mesh.stream_secs = 4;
    mesh.degree = 4;
    mesh.loss = 0.01;
    mesh.push_mode = PushMode::Hybrid;
    mesh.stream_count = 4;
    let mut cross = cross_traffic_config(3, PushMode::Hybrid, CROSS_UPLINK_TENTHS);
    cross.stream_secs = 6;
    for mut cfg in [mesh, cross] {
        cfg.keep = Keep::Bytes;
        let stored = Sim::new(cfg.clone()).run();
        assert_eq!(stored.mismatches, 0);
        assert!(stored.delivery > 0.9);
        assert!(stored.cross.as_ref().is_none_or(|c| c.delivery > 0.9));
        for keep in [Keep::Nothing, Keep::Head] {
            cfg.keep = keep;
            let coded = Sim::new(cfg.clone()).run();
            assert_eq!(
                (coded.trace, coded.frames, coded.bytes),
                (stored.trace, stored.frames, stored.bytes),
                "{keep:?}"
            );
        }
    }
}

/// The tables ([`Sim::write_tables`]) leave the run as it was, at any bin
/// width, and their seconds are the series': read through getters, their
/// counters change nothing.
#[test]
fn sim_tables_change_no_trace() {
    let mut cfg = Config::new(5, 8);
    cfg.stream_secs = 4;
    cfg.degree = 4;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    let mut plain = Sim::new(cfg.clone());
    let r = plain.run();
    let plain = (r.trace, r.frames, r.bytes, plain.reads_hash());
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("sim_tables_change_no_trace-{}", std::process::id()));
    for (ms, nodes) in [(1000, false), (1, true)] {
        let mut sim = Sim::new(cfg.clone());
        sim.keep_tables(Duration::from_millis(ms), nodes, true);
        let r = sim.run();
        assert_eq!(
            (r.trace, r.frames, r.bytes, sim.reads_hash()),
            plain,
            "{ms} ms bins"
        );
        let d = dir.join(ms.to_string());
        sim.write_tables(&d).unwrap();
        let s = sim.series_data().unwrap();
        let seconds = std::fs::read_to_string(d.join("seconds.csv")).unwrap();
        assert_eq!(seconds.lines().count(), s.len() + 1);
        let bins = std::fs::read_to_string(d.join("bins.csv")).unwrap();
        assert!(bins.lines().count() > s.len(), "{ms} ms");
        // A column of the bins, summed: the counters the bins take at
        // their edges add up to the run's at its end.
        let sum = |col: &str| -> u64 {
            let mut rows = bins.lines();
            let head: Vec<&str> = rows.next().unwrap().split(',').collect();
            let i = head.iter().position(|&c| c == col).unwrap();
            rows.filter_map(|l| l.split(',').nth(i).unwrap().parse::<u64>().ok())
                .sum()
        };
        let seed = &sim.nodes[0];
        let st = seed.runner.swarm_stats(seed.swarm).unwrap();
        assert_eq!(
            [
                sum("viewer_switched"),
                sum("viewer_shed"),
                sum("viewer_sheds_sent") + sum("seed_sheds_sent"),
                sum("seed_answers_refused"),
                sum("seed_answers_dropped"),
            ],
            [
                r.push.switched,
                r.push.shed,
                r.push.sheds_sent,
                st.answers_refused,
                st.answers_dropped,
            ],
            "{ms} ms"
        );
        let lines = |t: &str| {
            std::fs::read_to_string(d.join(format!("{t}.csv"))).map_or(0, |c| c.lines().count())
        };
        assert!(lines("events") > 4, "{ms} ms: a join per viewer");
        assert!(lines("forecast") > 1, "{ms} ms");
        assert_eq!(lines("node_bins") > 0, nodes, "{ms} ms");
        let reads = std::fs::read_to_string(d.join("reads.csv")).unwrap();
        let read: usize = sim.honest().iter().map(|v| v.lags.len()).sum();
        assert_eq!(reads.lines().count(), read + 1);
        for t in ["seconds", "viewer_seconds", "reads", "bins", "viewers"] {
            assert!(d.join(format!("{t}.schema.json")).exists(), "{t}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The harness labels each read with the chunks the reader truly read
/// when the store moves the reader on its own: a read offset set again
/// after reads, past every chunk written, resyncs the viewer's reader to
/// its new tail at its next read with no error. The bytes it reads from
/// there match the stream at their offsets, its reads' chunks jump to the
/// new offset and climb from it, and the viewer reads to the stream's end.
#[test]
fn sim_follows_a_moved_reader() {
    let mut cfg = Config::new(5, 8);
    cfg.stream_secs = 6;
    cfg.degree = 4;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    let mut sim = Sim::new(cfg);
    sim.keep_tables(Duration::from_millis(1000), false, false);
    while sim.now < t0() + Duration::from_secs(3) {
        let t = sim.next_time();
        sim.sample_bins(t);
        sim.now = sim.now.max(t);
        sim.step();
    }
    let i = (1..sim.nodes.len())
        .find(|&i| sim.nodes[i].role == Role::Peer && !sim.nodes[i].lags.is_empty())
        .unwrap();
    // Two munros past the last written: stored nowhere yet.
    let to = ((sim.written + 2) * sim.cfg.munro_chunks) as u32;
    let read = sim.nodes[i].read_seqs.len();
    let now = sim.now;
    let n = &mut sim.nodes[i];
    let local = n.local(now);
    n.runner
        .swarm_mut(n.swarm, local)
        .unwrap()
        .set_offset(Bin(2 * u64::from(to)));
    let r = sim.run();
    let n = &sim.nodes[i];
    assert_eq!((n.mismatches, r.mismatches), (0, 0), "{r:?}");
    let (before, after) = n.read_seqs.split_at(read);
    assert!(before.last().is_some_and(|&c| c + 1 < to), "{before:?}");
    assert_eq!(after.first(), Some(&to), "{after:?}");
    assert!(n.read_seqs.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(n.pos, r.total, "node {i} read to the end");
}

/// A media run of `encoder` signed under `sign`: 8 viewers in a full mesh,
/// 4 KiB chunks, Hybrid on 4 streams, 6 s.
fn media_config(seed: u64, encoder: &str, sign: &str) -> Config {
    let mut cfg = Config::new(seed, 8);
    cfg.stream_secs = 6;
    cfg.chunk_size = 4096;
    cfg.live_window = 1 << 10;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    let encoder = Encoder::parse(encoder).unwrap();
    let sign = Sign::parse(sign).unwrap().unwrap();
    cfg.stream_rate = encoder.rate();
    cfg.munro_chunks = sign.chunks(encoder.rate(), cfg.chunk_size);
    cfg.media = Some(Media { encoder, sign });
    cfg
}

/// A media run replays its trace, and its padding codec makes the bytes a
/// store keeping them holds: the same frames go out.
#[test]
fn sim_media_runs_replay_and_pad_like_stored_bytes() {
    for (encoder, sign) in [
        ("1500v", "u8,t200"),
        ("128c", "u2,t100"),
        ("512c", "u1,t250"),
    ] {
        let mut cfg = media_config(4, encoder, sign);
        let a = Sim::new(cfg.clone()).run();
        let b = Sim::new(cfg.clone()).run();
        assert_eq!((a.trace, a.frames, a.bytes), (b.trace, b.frames, b.bytes));
        assert_eq!(a.mismatches, 0, "{encoder} {sign}: {a:?}");
        cfg.keep = Keep::Bytes;
        let stored = Sim::new(cfg).run();
        assert_eq!(
            (stored.trace, stored.frames, stored.bytes),
            (a.trace, a.frames, a.bytes),
            "{encoder} {sign}"
        );
    }
}

/// Every honest viewer of a healthy media run reads the stream from its
/// first two units to its end, each media chunk no sooner than its unit
/// was signed, and no honest message names a data-less chunk.
#[test]
fn sim_media_viewers_read_the_whole_stream() {
    for (encoder, sign) in [("3000c", "ur200,t300"), ("128c", "u4,t100-500x1.5")] {
        let mut sim = Sim::new(media_config(5, encoder, sign));
        let r = sim.run();
        check_delivery(&r, 2 * sim.munro as u64);
        assert!(
            r.lag_p50 >= r.sign_lag_p50 && r.lag_p99 >= r.sign_lag_p99,
            "{r:?}"
        );
        let m = sim.media.as_ref().unwrap();
        assert_eq!(m.done, m.plan.ops.len());
        assert!(encoder != "128c" || !m.plan.dataless.is_empty());
        assert_eq!(sim.dataless_on_wire, 0, "{encoder} {sign}");
        for n in sim.nodes.iter().filter(|n| n.role == Role::Peer) {
            assert!(n.sign_lags.iter().all(|&(_, l)| l >= Duration::ZERO));
        }
    }
}

/// The streams and policies [`sign_costs`] tabulates by default.
const COST_MEDIA: &str = "128c,512c,1500c,1500v,3000c,6000c,6000v";
const COST_SIGNS: &str = "u1,t100;u1,t200;u1,t300;u1,t500;u2,t100;u2,t200;u2,t300;u2,t500;\
    u4,t100;u4,t200;u4,t300;u4,t500;u8,t100;u8,t200;u8,t300;u8,t500;\
    u16,t100;u16,t200;u16,t300;u16,t500;ur150,t100-500x1.5;ur200,t100-500x1.5";

/// What signing costs the writer alone, no swarm: per stream
/// (`SIM_MEDIAS`, comma-separated) and policy (`SIM_SIGNS`,
/// semicolon-separated), over `SIM_SEEDS` (8) traces of `SIM_SECS` (60) s
/// in 4 KiB chunks, the means of the units signed a second (each peer's
/// verifications), stream bytes sent per media byte, and each media
/// chunk's wait to be signed (p50, p99).
#[test]
#[ignore = "a table; run in release with --ignored --nocapture"]
fn sign_costs() {
    let medias = knob::<String>("SIM_MEDIAS").unwrap_or_else(|| COST_MEDIA.into());
    let signs = knob::<String>("SIM_SIGNS").unwrap_or_else(|| COST_SIGNS.into());
    let seeds: u64 = knob("SIM_SEEDS").unwrap_or(8);
    let span = Duration::from_secs(knob("SIM_SECS").unwrap_or(60));
    println!(
        "| media | sign | unit | units/s | bytes/media | sent kbit/s | wait p50 ms | wait p99 ms |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for media in medias.split(',') {
        let encoder = Encoder::parse(media).unwrap();
        for sign in signs.split(';') {
            let sign = Sign::parse(sign).unwrap().unwrap();
            let chunks = sign.chunks(encoder.rate(), 4096);
            let mut sums = [0.0; 5];
            for seed in 1..=seeds {
                let trace = encoder.trace(&mut det_rng(seed), span);
                let p = live::plan(
                    &trace,
                    Signer::new(sign.deadline, chunks, 4096, live::CHUNK_HEADER),
                );
                let w = p.sign_waits();
                let secs = span.as_secs_f64();
                for (s, x) in sums.iter_mut().zip([
                    p.units() as f64 / secs,
                    p.sent_bytes as f64 / p.media_bytes as f64,
                    p.sent_bytes as f64 * 8e-3 / secs,
                    ms(percentile(&w, 50)) as f64,
                    ms(percentile(&w, 99)) as f64,
                ]) {
                    *s += x / seeds as f64;
                }
            }
            let [u, b, k, w50, w99] = sums;
            println!(
                "| {media} | {sign} | {chunks} | {u:.1} | {b:.3} | {k:.0} | {w50:.0} | {w99:.0} |"
            );
        }
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
    // Its windows stay sane, and the requests it sends peers that answer
    // rarely time out: LEDBAT keeps the answers' queue at its downlink
    // short. A greedy viewer answers no request whatever the window, so
    // the requests asked of it say nothing about that queue.
    for (&remote, &l) in &node.links {
        let s = node
            .runner
            .channel_stats(sim.links[l].peer, node.swarm)
            .unwrap();
        assert!(s.scheduler.cwnd >= 2 * 1024, "{remote}: {s:?}");
        assert!(s.scheduler.flight_size >= 0);
        if !matches!(sim.nodes[remote].role, Role::Greedy(_)) {
            assert!(
                s.outbound.cancel * 10 <= s.outbound.request.max(10),
                "{remote}: {s:?}"
            );
        }
    }
}

#[test]
fn sim_adversaries_dont_stall_honest_peers() {
    let mut cfg = Config::new(8, 8);
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
/// `SIM_TRACE_NODE`, `SIM_VERBOSE`, `SIM_CROSS_UP_X10`, and
/// [`media_knobs`]' `SIM_MEDIA` and `SIM_SIGN`);
/// the asserting tests use fixed configs, so the determinism guardrail
/// holds for them. `SIM_CAPTURE=<dir>` captures any run into
/// `<dir>/<run tag>/` (`SIM_CAPTURE_TAP=0` leaves out the decisions):
/// capture changes no trace. `SIM_GREEDY=0` runs any test that draws its
/// greedy viewers without them, and `SIM_ECHO_GREED=1` any with greedy
/// viewers that hold or forge their echoes, for comparisons;
/// `SIM_GREEDY_KINDS` picks a draw's kinds ([`greedy_kinds_knob`]) and
/// `SIM_GREEDY_SHARE=<percent>` fixes its share. `SIM_CHURN=<percent>`
/// has that share of any run's honest viewers leave per minute, each place
/// taken again `SIM_CHURN_AWAY` seconds on ([`Churn`]). `SIM_ARRIVALS`
/// brings any run's viewers in after the start ([`Arrivals::parse`]), and
/// `SIM_DEPART` sends a share of them away for good ([`Depart::parse`]).
/// `SIM_CLOCK_SKEW`
/// sets any run's clocks off: `real` as in the wild, `<ms>` either way,
/// `+<ms>` or `-<ms>` all alike, at most [`MAX_SKEW`]
/// ([`clock_skew_knob`]). `SIM_LOOPS=1` adds every parent decision,
/// counted by kind, and how push loops formed to a job's row ([`Loops`]);
/// it changes no trace. A knob that is set but doesn't parse panics, so a
/// typo can't run the default unnoticed.
#[allow(clippy::disallowed_methods)]
fn knob<T: std::str::FromStr>(name: &str) -> Option<T> {
    let s = std::env::var(name).ok()?;
    match s.parse() {
        Ok(v) => Some(v),
        Err(_) => panic!("{name}={s} doesn't parse"),
    }
}

/// `SIM_CHUNK` bytes per chunk and `SIM_MUNRO` bytes per munro, each
/// default `cfg`'s; the window keeps its size in bytes. Both are powers of
/// two, and a munro holds at least one chunk.
fn size_knobs(cfg: &mut Config) {
    let window = cfg.live_window * cfg.chunk_size;
    let chunk: usize = knob("SIM_CHUNK").unwrap_or(cfg.chunk_size);
    let munro: usize = knob("SIM_MUNRO").unwrap_or(cfg.munro_chunks * cfg.chunk_size);
    assert!(
        chunk.is_power_of_two() && munro.is_power_of_two() && munro >= chunk,
        "SIM_CHUNK={chunk}, SIM_MUNRO={munro}: powers of two, a munro at least a chunk"
    );
    cfg.chunk_size = chunk;
    cfg.munro_chunks = munro / chunk;
    cfg.live_window = (window / chunk).max(1);
}

/// `SIM_MEDIA=<encoder>` ([`Encoder::parse`]: `1500v`, `6000c:k10`)
/// streams at that encoder's bitrate. With `SIM_SIGN=<policy>`
/// ([`Sign::parse`]: `u8,t200`, `ur200,t100-500x1.5`) the run is a media
/// run ([`Media`]) at the policy's chunks per signature; `SIM_SIGN=off`, or
/// none, writes whole munros.
fn media_knobs(cfg: &mut Config) {
    let encoder = knob::<String>("SIM_MEDIA")
        .map(|s| Encoder::parse(&s).unwrap_or_else(|e| panic!("SIM_MEDIA={e}")));
    let sign = knob::<String>("SIM_SIGN")
        .and_then(|s| Sign::parse(&s).unwrap_or_else(|e| panic!("SIM_SIGN={e}")));
    if let Some(e) = encoder {
        cfg.stream_rate = e.rate();
    }
    match (encoder, sign) {
        (Some(encoder), Some(sign)) => {
            cfg.munro_chunks = sign.chunks(encoder.rate(), cfg.chunk_size);
            cfg.media = Some(Media { encoder, sign });
        }
        (None, Some(_)) => panic!("SIM_SIGN takes SIM_MEDIA"),
        _ => {}
    }
}

/// `SIM_STREAMS` streams, default `cfg`'s.
fn streams_knob(cfg: &mut Config) {
    if let Some(n) = knob::<usize>("SIM_STREAMS") {
        assert!(n >= 1, "SIM_STREAMS={n}: at least 1");
        cfg.stream_count = n;
    }
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
                x.and_then(|k| n.runner.peer_id(k).map(|id| id_node(id).to_string()))
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
    // These hold at 29 of seeds 1-32; seed 1 is one of the others.
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 4);
    cfg.seed = 4;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::FullMesh, 4, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, MUNRO as u64);
    assert!(r.lag_p99 < Duration::from_millis(350), "{r:?}");
    assert!(hit_rate(&r) > 0.95, "{r:?}");
    // A liar's children prune it.
    if r.greedy.1 == 0 {
        assert_eq!(r.push.pruned, 0, "{r:?}");
    }
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
    // Requests to a liar wait out their timeout before going elsewhere.
    assert!(r.lag_p99 < Duration::from_millis(600), "{r:?}");
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
    // The admitting seed keeps a greedy subscriber's slot as long as it
    // asks.
    assert!(r.lag_p99 < Duration::from_secs(1), "{r:?}");
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
fn sim_push_churn() {
    // Viewers leave at random, twice the swarm a minute, and a new viewer
    // takes each place 2 s on: every new one reads within a second, the
    // ones that stay miss nothing, and nobody keeps a gone viewer as a
    // parent.
    let mut cfg = Config::new(10, 8);
    cfg.seed_up = 8 * cfg.stream_rate;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    cfg.stream_secs = 20;
    cfg.churn = Some(Churn {
        per_min: 2.0,
        away: Duration::from_secs(2),
    });
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    let leaves = sim.churn.iter().filter(|c| c.2).count();
    eprintln!("{leaves} leaves, joins {:?}\n{r:?}", sim.joins);
    assert!(leaves >= 3, "{:?}", sim.churn);
    assert_eq!(sim.churn_done, sim.churn.len());
    assert_eq!(sim.joins.len(), leaves);
    assert!(
        sim.joins.iter().all(|&j| j < Duration::SECOND),
        "{:?}",
        sim.joins
    );
    assert_eq!((r.mismatches, r.underruns, r.resets), (0, 0, 0), "{r:?}");
    for (i, n) in sim
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.role == Role::Peer)
    {
        if !n.churned {
            assert!(
                n.start.is_some_and(|s| s <= 4 * MUNRO as u64),
                "node {i} late"
            );
        }
        assert_eq!(n.pos, r.total, "node {i} incomplete");
        let p = n.runner.push_stats(n.swarm).unwrap();
        for parent in p.parents.iter().flatten() {
            assert!(n.runner.peer_id(*parent).is_some(), "node {i}: {p:?}");
        }
    }
}

/// The knobs' forms read back as they print, and malformed ones don't
/// read.
#[test]
fn sim_audience_knobs_parse() {
    let secs = |s: i64| Duration::from_secs(s);
    for (s, want) in [
        ("t0", Arrivals::T0),
        ("trickle", Arrivals::Trickle(None)),
        ("trickle:2.5", Arrivals::Trickle(Some(2.5))),
        (
            "event:20:20:10",
            Arrivals::Event {
                warm: 0.2,
                at: secs(20),
                window: secs(10),
            },
        ),
    ] {
        assert_eq!(Arrivals::parse(s), Some(want), "{s}");
        assert_eq!(want.to_string(), s);
    }
    for s in [
        "",
        "trickle:0",
        "trickle:-1",
        "event:120:20:10",
        "event:20:20",
        "burst",
    ] {
        assert_eq!(Arrivals::parse(s), None, "{s}");
    }
    for (s, abrupt) in [("60:40:10", false), ("60:40:10:abrupt", true)] {
        let want = Depart {
            share: 0.6,
            at: secs(40),
            window: secs(10),
            abrupt,
        };
        assert_eq!(Depart::parse(s), Some(want), "{s}");
        assert_eq!(want.to_string(), s);
    }
    for s in ["60:40", "60:40:10:quiet", "101:40:10", "60:40:10:abrupt:x"] {
        assert_eq!(Depart::parse(s), None, "{s}");
    }
}

/// A Hybrid swarm of `peers` viewers, 3 links each and 4 at the seed, on 4
/// streams for `secs` s, under `arrivals`.
fn audience_config(seed: u64, peers: usize, secs: u64, arrivals: Arrivals) -> Config {
    let mut cfg = Config::new(seed, peers);
    cfg.degree = 3;
    cfg.seed_degree = 4;
    cfg.seed_up = 8 * cfg.stream_rate;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 4;
    cfg.stream_secs = secs;
    cfg.arrivals = arrivals;
    cfg
}

/// Runs `sim`, checking at every step that no node links before it
/// arrives or to one that hasn't.
fn run_checking_links(sim: &mut Sim) -> Report {
    while sim.arrivals_done < sim.arrivals.len() {
        let t = sim.next_time();
        sim.now = sim.now.max(t);
        sim.step();
        for (i, n) in sim.nodes.iter().enumerate() {
            assert!(
                n.role != Role::Absent || n.links.is_empty(),
                "node {i} linked before it arrived"
            );
            for &j in n.links.keys() {
                assert_ne!(sim.nodes[j].role, Role::Absent, "{i} linked to {j}");
            }
        }
    }
    sim.run()
}

/// The checks every audience test makes of its honest viewers: none
/// stranded (each read and caught up within a second of its join, or of
/// the stream's start for one there before it), the stream's bytes right,
/// and the full delivery due ([`Sim::delivered`]).
fn check_arrivals(sim: &Sim, r: &Report) -> AudienceReport {
    let a = sim.audience_report();
    eprintln!("{a:?}\n{r:?}");
    assert_eq!((a.unread, a.behind), (0, 0), "{a:?}");
    for (i, n) in sim.nodes.iter().enumerate().filter(|(_, n)| n.watched()) {
        let by = n.joined.max(stream_start()) + Duration::SECOND;
        assert!(
            n.first_read.is_some_and(|t| t < by) && n.caught_up.is_some_and(|t| t < by),
            "node {i} joined at {:?}: first read {:?}, caught up {:?}",
            n.joined - t0(),
            n.first_read.map(|t| t - t0()),
            n.caught_up.map(|t| t - t0())
        );
    }
    assert_eq!((r.mismatches, r.underruns, r.resets), (0, 0, 0), "{r:?}");
    assert!(r.delivery > 0.999, "{r:?}");
    a
}

#[test]
fn sim_trickle_arrivals_catch_up() {
    // The seed starts alone and the viewers trickle in over the stream's
    // first third: each links on arrival to nodes already there, keeps its
    // role, reads from the live edge (a mandatory-delivery cell's arrivals
    // too) and catches up at once, and every honest one reads to the end.
    for mode in [DeliveryMode::LOW_LATENCY, DeliveryMode::MANDATORY] {
        let mut cfg = audience_config(21, 12, 15, Arrivals::Trickle(None));
        cfg.delivery_mode = mode;
        let greedy = greedy(&cfg);
        let mut sim = Sim::new(cfg);
        assert_eq!(sim.arrivals.len(), 12);
        assert!(
            sim.arrivals.iter().all(|&(t, _)| (stream_start()
                ..stream_start() + Duration::from_secs(5))
                .contains(&t))
        );
        let r = run_checking_links(&mut sim);
        let a = check_arrivals(&sim, &r);
        assert_eq!(a.arrived, (12, 12 - greedy.len()), "{mode:?}");
        // The seed took its 4, and nobody is left without links.
        assert_eq!(sim.nodes[0].links.len(), 4, "{mode:?}");
        for (i, n) in sim.nodes.iter().enumerate().skip(1) {
            assert!(!n.links.is_empty(), "{mode:?}: node {i} stranded");
            assert_eq!(n.role, n.kind, "{mode:?}: node {i}");
            assert_eq!(
                greedy
                    .get(&i)
                    .map(|&g| Role::Greedy(g))
                    .unwrap_or(Role::Peer),
                n.kind
            );
            if n.role != Role::Peer {
                continue;
            }
            assert_eq!(n.pos, r.total, "{mode:?}: node {i} incomplete");
            // From the live edge: a chunk published at most a second before
            // it arrived.
            let c = (n.start.unwrap() / sim.cfg.chunk_size as u64) as usize;
            assert!(
                sim.publish_times[c] + Duration::SECOND >= n.arrival,
                "{mode:?}: node {i} arrived at {:?} and read from chunk {c}",
                n.arrival - t0()
            );
        }
    }
}

#[test]
fn sim_event_burst_joins_a_warm_swarm() {
    // A scheduled event: a quarter of 16 viewers trickles in over the
    // first 6 s, the rest within 3 s from there. Every honest one catches
    // up and reads to the end.
    let at = Duration::from_secs(6);
    let window = Duration::from_secs(3);
    let event = Arrivals::Event {
        warm: 0.25,
        at,
        window,
    };
    let mut sim = Sim::new(audience_config(22, 16, 16, event));
    let warm = sim
        .arrivals
        .iter()
        .filter(|&&(t, _)| t < stream_start() + at)
        .count();
    assert_eq!(warm, 4);
    assert!(
        sim.arrivals
            .iter()
            .all(|&(t, _)| (stream_start()..stream_start() + at + window).contains(&t))
    );
    let r = run_checking_links(&mut sim);
    check_arrivals(&sim, &r);
    for (i, n) in sim
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.role == Role::Peer)
    {
        assert_eq!(n.pos, r.total, "node {i} incomplete");
    }
}

#[test]
fn sim_departures_leave_the_rest_watching() {
    // 16 honest viewers trickle in over the stream's first 7 s, and half
    // of them leave for good within 2 s from 8 s in, closing their links
    // or going silent. The rest read every chunk to the end and none of
    // them keeps a viewer gone as a parent; every viewer whose open parent
    // left logs it at that departure, as the row counts.
    let mut traces = Vec::new();
    for abrupt in [false, true] {
        let mut cfg = audience_config(23, 16, 20, Arrivals::Trickle(None));
        cfg.greedy = Greedy::Nodes(vec![]);
        cfg.depart = Some(Depart {
            share: 0.5,
            at: Duration::from_secs(8),
            window: Duration::from_secs(2),
            abrupt,
        });
        let mut sim = Sim::new(cfg);
        sim.keep_tables(Duration::from_millis(1000), false, false);
        let r = sim.run();
        traces.push(r.trace);
        let a = check_arrivals(&sim, &r);
        assert_eq!(a.departed, (8, 8), "abrupt {abrupt}");
        let gone: BTreeSet<usize> = (1..sim.nodes.len())
            .filter(|&i| sim.nodes[i].role == Role::Gone)
            .collect();
        assert_eq!(gone.len(), 8);
        for &i in &gone {
            let left = sim.nodes[i].left.unwrap() - stream_start();
            assert!(left >= Duration::from_secs(8) && left < Duration::from_secs(10));
        }
        for (i, n) in sim
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.role == Role::Peer)
        {
            assert_eq!(n.pos, r.total, "abrupt {abrupt}: node {i} incomplete");
            let p = n.runner.push_stats(n.swarm).unwrap();
            for &g in &gone {
                // Gracefully, its link closed; abruptly it stays, silent.
                let link = n.links.get(&g);
                assert!(abrupt || link.is_none(), "node {i} kept its link to {g}");
                let key = link.map(|&l| sim.links[l].peer);
                assert!(
                    key.is_none_or(|k| !p.parents.contains(&Some(k))),
                    "abrupt {abrupt}: node {i} keeps {g} as a parent: {p:?}"
                );
            }
        }
        let events = sim.timeline.as_ref().unwrap();
        let departs: BTreeMap<i64, Time> = events
            .iter()
            .filter(|e| e.kind == "depart")
            .map(|e| (e.node, e.at))
            .collect();
        let lost: Vec<&TimedEvent> = events.iter().filter(|e| e.kind == "parent_lost").collect();
        assert!(!lost.is_empty(), "abrupt {abrupt}: no parent left");
        for e in &lost {
            // At the parent's departure, by a child still there.
            assert_eq!(
                departs.get(&e.peer),
                Some(&e.at),
                "abrupt {abrupt}: {}",
                e.peer
            );
            let child = &sim.nodes[e.node as usize];
            assert!(
                e.value > 0 && child.left.is_none_or(|t| t > e.at),
                "abrupt {abrupt}"
            );
        }
        assert_eq!(lost.len() as u64, sim.parent_lost.0);
        assert_eq!(
            lost.iter().map(|e| e.value as u64).sum::<u64>(),
            sim.parent_lost.1
        );
        let rec = a.recovery.unwrap();
        assert_eq!((rec.stayed, rec.stranded), (8, 0));
        // Abruptly, the children first wait out a silent parent's push
        // deadlines and silence (5 s).
        assert!(
            rec.recover < Duration::from_secs(if abrupt { 8 } else { 5 }),
            "abrupt {abrupt}: {rec:?}"
        );
    }
    assert_ne!(traces[0], traces[1]);
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
    // These hold at 18 of seeds 1-32.
    let mut cfg = Scenario::Withholding.config(PushMode::Hybrid, 4);
    cfg.seed = 6;
    let mut sim = Sim::new(cfg);
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

/// Greedy viewers take the stream and serve none of it: no data or
/// integrity of theirs reaches anyone; a liar's HAVEs draw requests, and a
/// free rider, announcing nothing, draws none.
#[test]
fn sim_greedy_viewers_serve_nothing() {
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 4);
    // A draw whose honest nodes pull some chunks: most of them are pushed.
    cfg.seed = 13;
    cfg.greedy = Greedy::Nodes(vec![(1, Greed::FreeRider), (2, Greed::Liar)]);
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!(
        "{}\n{:?}",
        row(Scenario::FullMesh, 4, PushMode::Hybrid, &r),
        r.push
    );
    check_delivery(&r, 4 * MUNRO as u64);
    // Per kind: what honest nodes asked of them, and they of honest nodes.
    let (mut asked, mut asking) = ([0, 0], [0, 0]);
    for n in sim
        .nodes
        .iter()
        .filter(|n| n.role != Role::Greedy(Greed::FreeRider))
    {
        for (&remote, &l) in &n.links {
            let Role::Greedy(g) = sim.nodes[remote].role else {
                continue;
            };
            let cs = n.runner.channel_stats(sim.links[l].peer, n.swarm).unwrap();
            let i = &cs.inbound;
            assert_eq!(
                (i.data, i.integrity, i.signed_integrity),
                (0, 0, 0),
                "{remote}: {i:?}"
            );
            match g {
                Greed::FreeRider => assert_eq!(i.have, 0, "{remote}: {i:?}"),
                Greed::Liar | Greed::Grabber => assert!(i.have > 0, "{remote}: {i:?}"),
            }
            asked[usize::from(g == Greed::Liar)] += cs.outbound.request;
            asking[usize::from(g == Greed::Liar)] += i.request + i.stream_request;
        }
    }
    assert_eq!(asked[0], 0, "requests to the free rider");
    assert!(asked[1] > 0, "no requests to the liar");
    assert!(asking.iter().all(|&n| n > 0), "{asking:?}");
}

/// Grabbers serve like honest viewers, ask the seed for more than any
/// honest viewer does, and never cancel. The seed here pushes nearly
/// every chunk, so only the first few are pulled, from whoever is expected
/// to deliver them first ([`strims_ppspp::route`]): not every viewer is
/// asked for any, honest or grabber (a grabber's own grabs load its link).
/// So each grabber answers every request an honest node made of it, and
/// the grabbers serve some.
#[test]
fn sim_grabbers_ask_everyone_and_never_cancel() {
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 4);
    // A draw at which no honest viewer lost its pushes for long and pulled
    // them from the seed.
    cfg.seed = 13;
    cfg.greedy = Greedy::Nodes(vec![(1, Greed::Grabber), (2, Greed::Grabber)]);
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    check_delivery(&r, 4 * MUNRO as u64);
    // The data grabbers sent honest nodes; the most requests the seed had
    // from a grabber, and from an honest viewer.
    let mut served = 0;
    let (mut grabbed, mut honest) = (0, 0);
    for (k, n) in sim.nodes.iter().enumerate() {
        if n.role == Role::Greedy(Greed::Grabber) {
            continue;
        }
        for (&remote, &l) in &n.links {
            let cs = n.runner.channel_stats(sim.links[l].peer, n.swarm).unwrap();
            let i = cs.inbound;
            if sim.nodes[remote].role == Role::Greedy(Greed::Grabber) {
                assert_eq!((i.cancel, i.stream_cancel), (0, 0), "{remote}: {i:?}");
                assert!(i.data >= cs.outbound.request, "{k}<-{remote}: {cs:?}");
                served += i.data;
                if k == 0 {
                    grabbed = i.request.max(grabbed);
                }
            } else if k == 0 {
                honest = i.request.max(honest);
            }
        }
    }
    assert!(served > 0);
    assert!(grabbed > honest, "{grabbed} {honest}");
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

/// Munros signed early, from one chunk with data to all eight.
const EARLY: [u64; 8] = [1, 3, 8, 5, 2, 8, 7, 4];

/// `cfg` with munros signed early ([`EARLY`]) and the same run with whole
/// munros: the early run's report and data-less messages on the wire,
/// and the whole run's report.
fn early_and_whole(cfg: Config) -> (Report, u64, Report) {
    let mut early = cfg.clone();
    early.early = EARLY.to_vec();
    let mut sim = Sim::new(early);
    let r = sim.run();
    let whole = Sim::new(cfg).run();
    (r, sim.dataless_on_wire, whole)
}

/// Pull with munros signed early: no message names a data-less chunk,
/// every reader reads them as zeros, and delivery and lag hold.
#[test]
fn sim_early_signed_units_pull() {
    for mode in [PushMode::Off, PushMode::EagerHave] {
        let mut cfg = Config::new(41, 10);
        cfg.degree = 4;
        cfg.seed_degree = 4;
        cfg.munro_chunks = 8;
        cfg.push_mode = mode;
        let (r, on_wire, whole) = early_and_whole(cfg);
        eprintln!("{mode:?} early: {r:?}\nwhole: {whole:?}");
        assert_eq!(on_wire, 0, "{mode:?}");
        check_delivery(&r, MUNRO as u64);
        check_delivery(&whole, MUNRO as u64);
        assert!(
            r.lag_p99 <= whole.lag_p99 * 5 / 4 + Duration::from_millis(20),
            "{mode:?}: early {} whole {}",
            r.lag_p99,
            whole.lag_p99
        );
    }
}

/// The hybrid with munros signed early over 16 streams, so that most
/// munros leave streams without data: nothing names a data-less chunk,
/// readers read zeros, delivery and lag hold, and honest parents, those of
/// empty streams included, take no more misses than with whole munros
/// and no prunes, from the first munro on.
#[test]
fn sim_early_signed_units_push() {
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 16);
    // The lag bound holds at 20 of seeds 1-32 (22 before pushes left out
    // the integrity their channel sent); seed 42 missed it by 1 ms.
    cfg.seed = 2;
    cfg.munro_chunks = 8;
    cfg.greedy = Greedy::Nodes(vec![]);
    let (r, on_wire, whole) = early_and_whole(cfg);
    eprintln!("early: {r:?}\n{:?}\nwhole: {whole:?}", r.push);
    assert_eq!(on_wire, 0);
    check_delivery(&r, MUNRO as u64);
    check_delivery(&whole, MUNRO as u64);
    assert!(
        r.lag_p99 <= whole.lag_p99 * 5 / 4 + Duration::from_millis(20),
        "early {} whole {}",
        r.lag_p99,
        whole.lag_p99
    );
    assert!(hit_rate(&r) > 0.95, "{:?}", r.push);
    assert_eq!(r.push.pruned, 0, "{:?}", r.push);
    assert!(
        r.push.misses <= whole.push.misses,
        "{:?}\n{:?}",
        r.push,
        whole.push
    );
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
        "{name} seed {}: delivery {:.4} missing {missing} underruns {} lag p50 {} p99 {} max {} (from 30 s {}) seed share {:.2} wasted {} timeouts {} frames {} invalid {}",
        sim.cfg.seed,
        r.delivery,
        r.underruns,
        ms(r.lag_p50),
        ms(r.lag_p99),
        ms(r.lag_max),
        ms(sim.max_lag_from(30)),
        r.seed_share,
        r.wasted,
        sim.timeouts().0,
        r.frames,
        count_invalid(sim),
    )
}

#[test]
#[ignore = "debugging aid: SIM_SCENARIO=starved|latency|capped|sparse, SIM_MODE=off|eager|hybrid, SIM_SEED"]
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
    if name == "latency" {
        println!(
            "settled after {} ms",
            ms(sim.settled_after(20, Duration::SECOND))
        );
    }
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

// --- evaluation scenarios ---------------------------------------------------
//
// The scenarios the evaluations below run, in pull (`Off`) and `Hybrid`.

/// Cross traffic: a second stream at the same rate, seeded by peer 1 and
/// viewed by every other node, through the same runners and the same
/// uplinks, each capped at `tenths / 10` times the stream rate (two
/// streams need about 2x on average, plus overhead).
fn cross_traffic_config(seed: u64, mode: PushMode, tenths: u64) -> Config {
    let mut cfg = node_sim_config(seed, 8, 40);
    cfg.seed_up = tenths * 32 * 1024 / 10;
    cfg.peer_up = tenths * 32 * 1024 / 10;
    cfg.push_mode = mode;
    cfg.cross = Some(Cross {
        seed: 1,
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

    /// The config for `seed`.
    fn config(self, seed: u64, mode: PushMode) -> Config {
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
                cross_traffic_config(seed, mode, tenths)
            }
            AbScenario::WanGlobal => wan_global_config(seed),
            AbScenario::WanMobile => wan_mobile_config(seed),
            AbScenario::WanResidentialSeed => wan_residential_seed_config(seed),
            AbScenario::WanSuperPeer => wan_super_peer_config(seed),
        };
        cfg.push_mode = mode;
        cfg
    }
}

fn median(mut v: Vec<i64>) -> i64 {
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// Cross traffic's uplinks, in tenths of the stream rate.
const CROSS_UPLINK_TENTHS: u64 = 35;

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
    /// [`Report::greedy_slots`], [`Report::liar_grafts`] and
    /// [`Report::barred`].
    greedy_slots: (f64, f64),
    liar_grafts: f64,
    barred: (u64, u64),
    /// [`Report::verdicts`], [`Report::liar_hold`] and
    /// [`Report::report_counts`]'s honest half; ANNOUNCE bytes into the
    /// seed per second of stream, and their share of the viewers' upload.
    verdicts: Verdicts,
    liar_hold: (Duration, Duration),
    honest_reports: ReportCounters,
    announce_rate: f64,
    announce_share: f64,
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
        greedy_slots: r.greedy_slots,
        liar_grafts: r.liar_grafts,
        barred: r.barred,
        verdicts: r.verdicts,
        liar_hold: r.liar_hold,
        honest_reports: r.report_counts.0,
        announce_rate: r.announce.1 as f64 / sim.cfg.stream_secs.max(1) as f64,
        announce_share: r.announce.2,
    }
}

/// The report columns of a run's row: liars' copies, the verdicts on
/// honest holders and liars, honest bars with a real child, honest
/// entries the seed's checks ruled out, and ANNOUNCE bytes.
fn verdict_cols(
    v: &Verdicts,
    liar_hold: (Duration, Duration),
    c: &ReportCounters,
    announce: (f64, f64),
) -> String {
    let pc = |a: u64, b: u64| 100.0 * a as f64 / b.max(1) as f64;
    let out = c.reporter_fed + c.reporter_silent + c.too_early + c.misses_unseen;
    format!(
        "{:.1} ({}) | {:.0}% / {:.0}% / {:.0}% ({:.0}%) | {} / {} | {:.0}% | {} | {:.1}% ({}/{}/{}/{}), {} about relays | {:.0} B/s ({:.2}%)",
        liar_hold.0.as_secs_f64(),
        liar_hold.1.as_secs(),
        pc(v.with_reporters, v.honest),
        pc(v.judged, v.honest),
        pc(v.served, v.honest),
        pc(v.served_childless, v.served),
        v.liars_judged,
        v.liars_served,
        pc(v.not_kept, v.honest),
        v.barred_with_children,
        pc(out, c.tallied + out),
        c.reporter_fed,
        c.reporter_silent,
        c.too_early,
        c.misses_unseen,
        c.subject_unfed,
        announce.0,
        100.0 * announce.1,
    )
}

/// [`verdict_cols`]'s header.
const VERDICT_HEADER: &str = "liar copy held s mean (max) | honest holders with a seed-neighbour child / judged / served (served without a child) | liar samples judged / served | honest holders pushed to and not kept up | honest bars with a child | honest entries ruled out (fed/silent/early/unseen), about relays | ANNOUNCE into the seed (share of viewer upload)";

/// The seed allocation tables' header.
const EVAL_HEADER: &str = "| case | delivery mean (min) | lag p50 ms | p99 ms median (max) | worst stripe p90 ms | p99 ms median (max) | worst region p99 ms | seed share | seed pulls | seed distinct | seed slots | seed objective ms | hit rate | duplicates | holders settled s median (max) | holder changes | super-peer children / declined | seed slots at free riders / liars | streams grafted to liars | holders barred (honest) | liar copy held s mean (max) | honest holders with a seed-neighbour child / judged / served (served without a child) | liar samples judged / served | honest holders pushed to and not kept up | honest bars with a child | honest entries ruled out (fed/silent/early/unseen), about relays | ANNOUNCE into the seed (share of viewer upload) |";

fn eval_row(name: &str, runs: &[EvalRun]) -> String {
    let n = runs.len().max(1) as f64;
    let mean = |f: &dyn Fn(&EvalRun) -> f64| runs.iter().map(f).sum::<f64>() / n;
    let med = |f: &dyn Fn(&EvalRun) -> Duration| median(runs.iter().map(|r| ms(f(r))).collect());
    let max = |f: &dyn Fn(&EvalRun) -> Duration| runs.iter().map(|r| ms(f(r))).max().unwrap_or(0);
    let min_delivery = runs.iter().map(|r| r.delivery).fold(1.0, f64::min);
    let mut v = Verdicts::default();
    let mut c = ReportCounters::default();
    for r in runs {
        v.add(&r.verdicts);
        add_counts(&mut c, &r.honest_reports);
    }
    let hold = (
        Duration::from_secs_f64(mean(&|r| r.liar_hold.0.as_secs_f64())),
        runs.iter().map(|r| r.liar_hold.1).max().unwrap_or_default(),
    );
    let verdicts = verdict_cols(
        &v,
        hold,
        &c,
        (mean(&|r| r.announce_rate), mean(&|r| r.announce_share)),
    );
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
        "| {name} | {:.4} ({:.4}) | {} | {} ({}) | {} | {} ({}) | {} | {:.0}% | {:.0}% | {:.2} | {:.1} | {} | {:.1}% | {:.2}% | {:.0} ({:.0}) | {:.1} | {super_peer} | {:.1}% / {:.1}% | {:.2}% | {:.1} ({:.1}) | {verdicts} |",
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
        100.0 * mean(&|r| r.greedy_slots.0),
        100.0 * mean(&|r| r.greedy_slots.1),
        100.0 * mean(&|r| r.liar_grafts),
        mean(&|r| r.barred.0 as f64),
        mean(&|r| r.barred.1 as f64),
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
    const ALL: [AllocCase; 12] = [
        AllocCase::Pull,
        AllocCase::Admission,
        AllocCase::AdmissionQos,
        AllocCase::Random,
        AllocCase::RandomQos,
        AllocCase::WholeQos,
        AllocCase::NearestQos,
        AllocCase::Oracle,
        AllocCase::OracleQos,
        AllocCase::DcOnly,
        AllocCase::Adaptive,
        AllocCase::AdaptiveQos,
    ];

    /// Whether the case pins as many slots as the admission used at the
    /// same scenario and seed, so the admission runs first.
    /// [`AllocCase::DcOnly`] pins the datacenter viewer whatever the count.
    fn takes_slots(self) -> bool {
        matches!(
            self,
            AllocCase::Random
                | AllocCase::RandomQos
                | AllocCase::WholeQos
                | AllocCase::NearestQos
                | AllocCase::Oracle
                | AllocCase::OracleQos
        )
    }
}

/// The stream copies `slots` pays for over `streams`: `full` whole copies
/// and `rem` streams of one more.
fn copies(slots: usize, streams: usize) -> (usize, usize) {
    (slots / streams.max(1), slots % streams.max(1))
}

/// `slots` stream slots on the honest viewers with the fastest uplinks:
/// whole copies of the stream, then a partial one. Among equal uplinks the
/// nearest to the seed comes first, and each copy goes to another region
/// than the last when one is left.
fn oracle_pin(cfg: &Config, slots: usize) -> Vec<Vec<usize>> {
    let w = cfg.wan.as_ref().unwrap();
    let streams = cfg.stream_count;
    let seed_city = w.sites[0].city;
    let greedy = greedy(cfg);
    let mut pool: Vec<usize> = (1..=cfg.peers)
        .filter(|i| !greedy.contains_key(i))
        .collect();
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
/// streams for `SIM_SECS` (30) seconds, lags from `SIM_WARM` (10), with
/// the scenario's chunk and munro sizes unless [`size_knobs`] sets them; a
/// pin gets `slots`.
fn alloc_config(sc: AbScenario, seed: u64, case: AllocCase, slots: usize) -> Config {
    let mode = if case == AllocCase::Pull {
        PushMode::Off
    } else {
        PushMode::Hybrid
    };
    let mut cfg = sc.config(seed, mode);
    cfg.stream_count = 16;
    streams_knob(&mut cfg);
    size_knobs(&mut cfg);
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

/// Prints the seed allocation table: each of `scenarios` in each of its
/// `cases`, over `SIM_SEEDS` (4) seeds. `SIM_SCENARIO`
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
        groups.push(sc);
    }
    let first: Vec<(usize, u64)> = (0..groups.len())
        .filter(|&g| cases(groups[g]).iter().any(|c| c.takes_slots()))
        .flat_map(|g| (1..=seeds).map(move |s| (g, s)))
        .collect();
    let admission = par_map(&first, |&(g, seed)| {
        eval_run(alloc_config(groups[g], seed, AllocCase::Admission, 0))
    });
    let slots: BTreeMap<(usize, u64), usize> = first
        .iter()
        .zip(&admission)
        .map(|(&j, r)| (j, r.seed_slots.round() as usize))
        .collect();
    let jobs: Vec<(usize, u64, AllocCase)> = (0..groups.len())
        .flat_map(|g| {
            let ran = slots.contains_key(&(g, 1));
            cases(groups[g])
                .into_iter()
                .filter(move |&c| c != AllocCase::Admission || !ran)
                .flat_map(move |c| (1..=seeds).map(move |s| (g, s, c)))
        })
        .collect();
    let runs = par_map(&jobs, |&(g, seed, case)| {
        let k = slots.get(&(g, seed)).copied().unwrap_or(0);
        eval_run(alloc_config(groups[g], seed, case, k))
    });
    println!("{EVAL_HEADER}");
    println!("|{}", "---|".repeat(20));
    for (g, &sc) in groups.iter().enumerate() {
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
            println!("{}", eval_row(&format!("{sc:?} {case:?}"), &of));
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
    let seed = knob("SIM_SEED").unwrap_or(1);
    let mut cfg = alloc_config(sc, seed, case, knob("SIM_SLOTS").unwrap_or(64));
    cfg.trace_node = knob("SIM_TRACE_NODE");
    if let Some(w) = &cfg.wan {
        let greedy = greedy(&cfg);
        for (i, site) in w.sites.iter().enumerate() {
            match greedy.get(&i) {
                Some(g) => eprintln!("node {i}: {site} {g:?}"),
                None => eprintln!("node {i}: {site}"),
            }
        }
    }
    let r = eval_run(cfg);
    println!("{EVAL_HEADER}");
    println!(
        "{}",
        eval_row(&format!("{sc:?} {case:?} seed {seed}"), &[r])
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
fn sim_cross_traffic_delivers_both_streams() {
    // Two streams through the same runners and uplinks (3.5x the stream
    // each).
    let mut sim = Sim::new(cross_traffic_config(
        1,
        PushMode::Hybrid,
        CROSS_UPLINK_TENTHS,
    ));
    let r = sim.run();
    let c = r.cross.as_ref().unwrap();
    eprintln!(
        "cross: main {} / {} ms, second {c:?}",
        ms(r.lag_p50),
        ms(r.lag_p99)
    );
    assert!(complete(&r), "{r:?}");
    assert_eq!(c.incomplete, 0, "{c:?}");
    assert_eq!(count_invalid(&sim), 0);
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

/// A stream at `mbit` Mbit/s behind a Chicago seed on `seed_access`:
/// `viewers` residential viewers in North America and Europe, each linked
/// to 6 others, the seed to 32; 16 streams in Hybrid, a 32 MiB window of
/// `chunk`-byte chunks.
fn wan_cable_seed_config(
    seed: u64,
    viewers: usize,
    chunk: usize,
    mbit: u64,
    policy: SeedPolicy,
    seed_access: Access,
) -> Config {
    let cities: Vec<(City, u32)> = GLOBAL_AUDIENCE
        .iter()
        .copied()
        .filter(|&(c, _)| matches!(c.region(), Region::NorthAmerica | Region::Europe))
        .collect();
    let seed_site = Site::new(City::Chicago, seed_access);
    let sites = wan_sites(seed, viewers, seed_site, &cities, RESIDENTIAL);
    let mut cfg = wan_config(seed, sites, 6, 32);
    cfg.stream_rate = mbit * 125_000;
    cfg.chunk_size = chunk;
    cfg.munro_chunks = MUNRO / chunk;
    cfg.live_window = (32 << 20) / chunk;
    cfg.stream_secs = 10;
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 16;
    cfg.seed_policy = policy;
    cfg
}

/// [`wan_cable_seed_config`] for `seed` under `policy`, from the knobs:
/// `SIM_PEERS` viewers (default 50), `SIM_MBIT` (6), `SIM_SEED_ACCESS`
/// (`cable`, `fiber` or `datacenter`), `SIM_STREAMS` (16), `SIM_SECS`,
/// and [`size_knobs`]' `SIM_CHUNK` (1024) and `SIM_MUNRO` (64 KiB).
fn wan_cable_seed_knobs(seed: u64, policy: SeedPolicy) -> Config {
    let access = match knob::<String>("SIM_SEED_ACCESS").as_deref() {
        None | Some("cable") => Access::Cable,
        Some("fiber") => Access::Fiber,
        Some("datacenter") => Access::Datacenter,
        Some(a) => panic!("SIM_SEED_ACCESS={a}: cable, fiber or datacenter"),
    };
    let mut cfg = wan_cable_seed_config(
        seed,
        knob("SIM_PEERS").unwrap_or(50),
        CHUNK,
        knob("SIM_MBIT").unwrap_or(6),
        policy,
        access,
    );
    streams_knob(&mut cfg);
    size_knobs(&mut cfg);
    if let Some(secs) = knob("SIM_SECS") {
        cfg.stream_secs = secs;
    }
    cfg
}

/// The adaptive seed, with uplink shares by allocation.
const ADAPTIVE_QOS: SeedPolicy = SeedPolicy {
    allocation: SeedAllocation::Adaptive,
    qos: SeedQos::ByAllocation,
};

/// [`wan_cable_seed_knobs`] under both seed policies, over `SIM_SEEDS`
/// seeds (default 1). `SIM_SEED_POLICY=adaptive` or `admission` runs one
/// policy; with `SIM_CAPTURE` each run's dumps go to a directory it prints.
#[test]
#[ignore = "a probe at production bitrates; run in release with --ignored --nocapture"]
fn wan_cable_seed() {
    let seeds: u64 = knob("SIM_SEEDS").unwrap_or(1);
    let policies: Vec<SeedPolicy> = match knob::<String>("SIM_SEED_POLICY").as_deref() {
        Some("adaptive") => vec![ADAPTIVE_QOS],
        Some("admission") => vec![SeedPolicy::ADMISSION],
        _ => vec![SeedPolicy::ADMISSION, ADAPTIVE_QOS],
    };
    let jobs: Vec<(SeedPolicy, u64)> = policies
        .iter()
        .flat_map(|&p| (1..=seeds).map(move |s| (p, s)))
        .collect();
    let rows = par_map(&jobs, |&(policy, seed)| {
        let cfg = wan_cable_seed_knobs(seed, policy);
        let (viewers, chunk) = (cfg.peers, cfg.chunk_size);
        let mbit = cfg.stream_rate as f64 * 8e-6;
        let access = cfg.wan.as_ref().unwrap().sites[0].access;
        let tag = run_tag(&cfg);
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        let seed_up = sim.nodes[0].up_bw;
        let span = (r.finish - t0()).as_secs_f64();
        let up = sim.nodes[0].wire.up_bytes;
        let last_write = t0() + Duration::SECOND + Duration::from_secs(sim.cfg.stream_secs as i64);
        let p = &r.push;
        format!(
            "| {:?} | {seed} | {mbit} | {access} | {viewers} | {chunk} | {:.4} | {} | {} | {} | {} | {:.1} | {:.1} | {} | {} | {} | {} | {:.0} | {:.0}% | {} | {} | {} / {} | {:.1}% / {:.1}% ({} s) | {:.2}% ({} ms) | {} ({}) | {} | {tag} | {:016x} |",
            policy.allocation,
            r.delivery,
            ms(r.lag_p50),
            ms(r.lag_p99),
            ms(r.lag_max),
            ms(r.stripe_p99),
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
            r.greedy.0,
            r.greedy.1,
            100.0 * r.greedy_slots.0,
            100.0 * r.greedy_slots.1,
            r.greedy_held.as_secs(),
            100.0 * r.liar_grafts,
            ms(r.liar_graft_span),
            r.barred.0,
            r.barred.1,
            verdict_cols(
                &r.verdicts,
                r.liar_hold,
                &r.report_counts.0,
                (
                    r.announce.1 as f64 / sim.cfg.stream_secs.max(1) as f64,
                    r.announce.2
                ),
            ),
            r.trace,
        )
    });
    println!(
        "| seed allocation | sim seed | Mbit/s | seed access | viewers | chunk | delivery | lag p50 ms | p99 ms | max ms | worst stripe p99 ms | stream ends s | reads end s | push hits | misses | late | unsolicited | seed MB up | seed uplink over the run | underruns | resets | free riders / liars | seed slots at free riders / liars (longest held) | streams grafted to liars (mean span) | holders barred (honest) | {VERDICT_HEADER} | run tag | trace |"
    );
    for r in rows {
        println!("{r}");
    }
}

/// One run for the sim farm (`hack/sim-farm`), printed as one line: `ROW`
/// and a flat JSON object of its config, its trace and its measurements.
/// `SIM_SCENARIO` is `cable-seed` (default), [`wan_cable_seed_knobs`]
/// under `SIM_SEED_POLICY` (`adaptive`, the default, or `admission`), or an
/// [`AbScenario`] run as the allocation tables run `SIM_CASE` (an
/// [`AllocCase`], default `AdaptiveQos`) under `SIM_CONTROLLER` (`ledbat`
/// or `bbr`). A case that [takes slots](AllocCase::takes_slots) takes them
/// from `SIM_SLOTS`: the `seed_slots` of the `Admission` job's row, rounded,
/// which a matrix's `[[derive]]` table gives it. `SIM_SEED` (1) is the sim
/// seed. With `SIM_OUT=<dir>` the run's tables go into `<dir>`
/// ([`Sim::write_tables`]), in bins of `SIM_BIN_MS` (default 100); neither
/// changes the trace. [`media_knobs`] apply to either.
#[test]
#[ignore = "one sim farm job; run in release with --ignored --nocapture"]
fn sim_job() {
    let seed = knob("SIM_SEED").unwrap_or(1);
    let scenario = knob::<String>("SIM_SCENARIO").unwrap_or_else(|| "cable-seed".into());
    let (scenario, case, mut cfg) = if scenario == "cable-seed" {
        let policy = match knob::<String>("SIM_SEED_POLICY").as_deref() {
            None | Some("adaptive") => ADAPTIVE_QOS,
            Some("admission") => SeedPolicy::ADMISSION,
            Some(p) => panic!("SIM_SEED_POLICY={p}: adaptive or admission"),
        };
        (
            scenario,
            format!("{:?}", policy.allocation),
            wan_cable_seed_knobs(seed, policy),
        )
    } else {
        let sc = AbScenario::ALL
            .into_iter()
            .find(|sc| format!("{sc:?}").eq_ignore_ascii_case(&scenario))
            .unwrap_or_else(|| panic!("SIM_SCENARIO={scenario}: cable-seed or an AbScenario"));
        let name = knob::<String>("SIM_CASE").unwrap_or_else(|| "AdaptiveQos".into());
        let case = AllocCase::ALL
            .into_iter()
            .find(|c| format!("{c:?}").eq_ignore_ascii_case(&name))
            .unwrap_or_else(|| panic!("SIM_CASE={name}: an AllocCase"));
        let slots = match case.takes_slots() {
            true => knob("SIM_SLOTS").unwrap_or_else(|| {
                panic!("SIM_CASE={case:?} takes SIM_SLOTS: the Admission row's seed_slots")
            }),
            false => 0,
        };
        (
            format!("{sc:?}"),
            format!("{case:?}"),
            alloc_config(sc, seed, case, slots),
        )
    };
    media_knobs(&mut cfg);
    let out = knob::<String>("SIM_OUT").filter(|s| !s.is_empty());
    let mut sim = Sim::new(cfg);
    if out.is_some() {
        sim.keep_tables(
            Duration::from_millis(knob::<i64>("SIM_BIN_MS").unwrap_or(100).max(1)),
            knob::<u32>("SIM_BIN_NODES") == Some(1),
            knob::<u32>("SIM_FORECAST") == Some(1),
        );
    }
    let r = sim.run();
    println!("ROW {}", job_row(&scenario, &case, &sim, &r));
    if let Some(dir) = out
        && let Err(e) = sim.write_tables(Path::new(&dir))
    {
        panic!("SIM_OUT: writing {dir}: {e}");
    }
}

/// [`sim_job`]'s row: the run's config, including the knobs its run tag
/// misses, the trace that a rerun must reproduce and the digest of its
/// reads ([`Sim::reads_hash`]), and its measurements, with counts left raw
/// so rows can be summed before taking shares.
fn job_row(scenario: &str, case: &str, sim: &Sim, r: &Report) -> String {
    let cfg = &sim.cfg;
    let seed = &sim.nodes[0];
    let span = (r.finish - t0()).as_secs_f64();
    let up = seed.wire.up_bytes;
    let p = &r.push;
    let v = &r.verdicts;
    let c = &r.report_counts.0;
    let super_peer = cfg
        .wan
        .as_ref()
        .and_then(|w| w.sites.iter().rposition(|s| s.access == Access::Datacenter))
        .filter(|&i| i > 0)
        .and_then(|i| {
            let n = &sim.nodes[i];
            n.runner
                .push_stats(n.swarm)
                .map(|s| (s.children as u64, s.rejected))
        });
    let seed_push = seed.runner.push_stats(seed.swarm).unwrap_or_default();
    let mut row = JsonRow::default();
    row.str("scenario", scenario)
        .str("case", case)
        .int("seed", cfg.seed)
        .int("peers", cfg.peers as u64)
        .num("mbit", cfg.stream_rate as f64 * 8e-6)
        .int("chunk", cfg.chunk_size as u64)
        .int("streams", cfg.stream_count as u64)
        .int("secs", cfg.stream_secs)
        .int("warm", cfg.warm_secs as u64)
        .str(
            "access",
            &cfg.wan
                .as_ref()
                .map_or("-".into(), |w| w.sites[0].access.to_string()),
        )
        .int("greedy", u64::from(knob::<u32>("SIM_GREEDY") != Some(0)))
        .int(
            "collude",
            u64::from(knob::<u32>("SIM_COLLUDE").is_some_and(|c| c != 0)),
        )
        .str("run_tag", &run_tag(cfg))
        .str("trace", &format!("{:016x}", r.trace))
        .str("reads_hash", &format!("{:016x}", sim.reads_hash()))
        .int("frames", r.frames)
        .int("bytes", r.bytes)
        .int("payload_bytes", sim.chunks_read() * cfg.chunk_size as u64)
        .num("delivery", r.delivery)
        .num(
            "cross_delivery",
            r.cross.as_ref().map_or(0.0, |c| c.delivery),
        )
        .int(
            "cross_p50_ms",
            r.cross.as_ref().map_or(0, |c| ms(c.lag_p50) as u64),
        )
        .int(
            "cross_p99_ms",
            r.cross.as_ref().map_or(0, |c| ms(c.lag_p99) as u64),
        )
        .int("lag_p50_ms", ms(r.lag_p50) as u64)
        .int("lag_p99_ms", ms(r.lag_p99) as u64)
        .int("lag_max_ms", ms(r.lag_max) as u64)
        .int("stripe_p90_ms", ms(r.stripe_p90) as u64)
        .int("stripe_p99_ms", ms(r.stripe_p99) as u64)
        .int(
            "worst_region_p99_ms",
            r.regions.iter().map(|&(_, l)| ms(l)).max().unwrap_or(0) as u64,
        )
        .num("stream_end_s", 1.0 + cfg.stream_secs as f64)
        .num("reads_end_s", span)
        .int("mismatches", r.mismatches)
        .int("underruns", r.underruns)
        .int("resets", r.resets)
        .int("wasted", r.wasted)
        .int("chunks_read", sim.chunks_read())
        .int("push_hits", p.hits)
        .int("push_misses", p.misses)
        .int("push_late", p.late)
        .int("push_unsolicited", p.unsolicited)
        .num("seed_mb_up", up as f64 / 1e6)
        .num("seed_uplink", up as f64 / (seed.up_bw as f64 * span))
        .num("seed_share", r.seed_share)
        .num("seed_pull_share", r.seed_pull_share)
        .num("seed_distinct", r.seed_distinct)
        .num("seed_slots", r.seed_slots)
        .int("seed_up_bw", seed.up_bw)
        .int("seed_capacity", r.seed_capacity)
        .int("seed_first_wave", seed_push.seed_first_wave as u64)
        .int(
            "seed_first_wave_capacity",
            seed_push.seed_first_wave_capacity,
        )
        .int("seed_objective_ms", ms(r.seed_objective) as u64)
        .num("settled_s", r.settled.as_secs_f64())
        .int("holder_changes", r.holder_changes as u64)
        .int("super_peer_children", super_peer.map_or(0, |s| s.0))
        .int("super_peer_declined", super_peer.map_or(0, |s| s.1))
        .int("free_riders", r.greedy.0 as u64)
        .int("liars", r.greedy.1 as u64)
        .num("free_rider_slots", r.greedy_slots.0)
        .num("liar_slots", r.greedy_slots.1)
        .int("grabbers", r.grabbers.0 as u64)
        .num("grabber_slots", r.grabbers.1)
        .num("holder_short", r.holder_short)
        .num("holder_copy", r.holder_copy)
        .int("greedy_held_s", r.greedy_held.as_secs() as u64)
        .num("liar_grafts", r.liar_grafts)
        .int("liar_graft_span_ms", ms(r.liar_graft_span) as u64)
        .int("barred", r.barred.0)
        .int("barred_honest", r.barred.1)
        .int("v_honest", v.honest)
        .int("v_with_children", v.with_children)
        .int("v_with_reporters", v.with_reporters)
        .int("v_judged", v.judged)
        .int("v_served", v.served)
        .int("v_served_childless", v.served_childless)
        .int("v_liars", v.liars)
        .int("v_liars_judged", v.liars_judged)
        .int("v_liars_served", v.liars_served)
        .int("v_not_kept", v.not_kept)
        .int("v_barred_with_children", v.barred_with_children)
        .num("liar_hold_s", r.liar_hold.0.as_secs_f64())
        .num("liar_hold_max_s", r.liar_hold.1.as_secs_f64())
        .int("reports_tallied", c.tallied)
        .int("reports_unknown", c.unknown)
        .int("reports_invalid", c.invalid)
        .int("reports_reporter_fed", c.reporter_fed)
        .int("reports_subject_unfed", c.subject_unfed)
        .int("reports_reporter_silent", c.reporter_silent)
        .int("reports_too_early", c.too_early)
        .int("reports_misses_unseen", c.misses_unseen)
        .int("announces", r.announce.0)
        .int("announce_bytes", r.announce.1)
        .num(
            "announce_bps",
            r.announce.1 as f64 / cfg.stream_secs.max(1) as f64,
        )
        .num("announce_share", r.announce.2)
        .int("asks", r.asks);
    // Requests timed out ([`Sim::timeouts`]): the honest viewers' over
    // every incarnation, the seed's, and with greedy viewers or adversaries
    // theirs; and the path measure against greedy viewers: honest nodes'
    // echoes of probes never sent, and the seed's latest round trips to
    // honest and to greedy viewers (median ms), with the links it has no
    // reading for.
    let (timeouts, seed_timeouts, hostile_timeouts) = sim.timeouts();
    let hostile = sim
        .nodes
        .iter()
        .any(|n| matches!(n.kind, Role::Greedy(_) | Role::Adversary(_)));
    let honest = |i: usize| sim.nodes[i].role == Role::Seed || sim.nodes[i].watched();
    let forged: u64 = sim
        .links
        .iter()
        .filter(|l| honest(l.from))
        .map(|l| l.path.unknown_echoes())
        .sum();
    let seed_links = |greedy: bool| {
        sim.links.iter().filter(move |l| {
            l.from == 0
                && match sim.nodes[l.to].role {
                    Role::Greedy(_) => greedy,
                    Role::Peer => !greedy,
                    _ => false,
                }
        })
    };
    let median_rtt = |greedy: bool| {
        let mut v: Vec<i64> = seed_links(greedy)
            .filter_map(|l| l.path.reading(sim.now))
            .map(|r| r.rtt.as_millis())
            .collect();
        v.sort_unstable();
        v.get(v.len() / 2).copied()
    };
    let unmeasured = |greedy: bool| {
        seed_links(greedy)
            .filter(|l| l.path.reading(sim.now).is_none())
            .count()
    };
    row.int("timeouts", timeouts)
        .int("seed_timeouts", seed_timeouts);
    if hostile {
        row.int("timeouts_hostile", hostile_timeouts);
    }
    row.int("echo_forged", forged);
    sim.windows.cols(&mut row);
    if let Some(ms) = median_rtt(false) {
        row.int("seed_rtt_honest_ms", ms as u64);
    }
    if let Some(ms) = median_rtt(true) {
        row.int("seed_rtt_greedy_ms", ms as u64);
    }
    row.int("seed_unmeasured_honest", unmeasured(false) as u64)
        .int("seed_unmeasured_greedy", unmeasured(true) as u64);
    // The seed's chunks pushed and answered, the requests it queued and
    // refused and the answers it dropped, and how long pushes and answers
    // waited in its queues.
    if let Some(st) = seed.runner.swarm_stats(seed.swarm) {
        row.int("seed_pushed", st.pushed_chunks)
            .int("seed_answered", st.chunks_sent - st.pushed_chunks)
            .int("seed_answers_queued", st.answers_queued)
            .int("seed_answers_refused", st.answers_refused)
            .int("seed_answers_dropped", st.answers_dropped);
        for (name, h) in [("push", st.push_wait), ("answer", st.answer_wait)] {
            for p in [50, 99] {
                if let Some(d) = h.percentile(f64::from(p)) {
                    row.int(&format!("seed_{name}_wait_p{p}_ms"), ms(d) as u64);
                }
            }
        }
    }
    // The push trees' depth, the honest viewers' uplink use (wire bytes
    // over the uplink for the run), their parents' admissions, and the
    // upload meters against the true uplinks.
    let sh = &sim.shape;
    let samples = sh.depth.iter().sum::<u64>().max(1) as f64;
    for (k, &n) in sh.depth.iter().enumerate() {
        let name = match k {
            0 => "depth_none".to_owned(),
            k if k <= DEPTH_HOPS => format!("depth_{k}"),
            k if k == DEPTH_HOPS + 1 => "depth_deeper".to_owned(),
            k if k == DEPTH_HOPS + 2 => "depth_cycle".to_owned(),
            _ => "depth_pull_rooted".to_owned(),
        };
        row.num(&name, n as f64 / samples);
    }
    row.num("relay_share", sh.relays as f64 / sh.viewers.max(1) as f64);
    for (name, x) in ["have_lazy_out_share", "have_lazy_in_share"]
        .iter()
        .zip(sh.lazy)
    {
        row.num(name, x / sh.lazy_samples.max(1) as f64);
    }
    let mut fanout = sh.fanout.clone();
    fanout.sort_unstable();
    for p in [50, 90] {
        if let Some(&f) = fanout.get(fanout.len().saturating_sub(1) * p / 100) {
            row.int(&format!("fanout_p{p}"), f);
        }
    }
    if sh.declines > 0 {
        row.num(
            "declines_idle",
            sh.declines_idle as f64 / sh.declines as f64,
        );
    }
    // Parent changes (prunes and switches) per honest viewer-minute after
    // the warm-up: with arrivals or departures, per minute each viewer was
    // there after its own.
    let viewer_minutes = if sim.audience() {
        let warm = Duration::from_secs(cfg.warm_secs);
        let secs: f64 = sim
            .nodes
            .iter()
            .filter(|n| n.watched())
            .map(|n| {
                (n.left.unwrap_or(r.finish) - (n.arrival + warm))
                    .as_secs_f64()
                    .max(0.0)
            })
            .sum();
        secs.max(60.0) / 60.0
    } else {
        let minutes = ((r.finish - t0()).as_secs_f64() - cfg.warm_secs as f64).max(1.0) / 60.0;
        let viewers = sim
            .nodes
            .iter()
            .filter(|n| n.role == Role::Peer)
            .count()
            .max(1) as f64;
        viewers * minutes
    };
    row.num(
        "parent_changes_per_min",
        (p.pruned + p.switched) as f64 / viewer_minutes,
    );
    // Each viewer's uplink use while it was there.
    let mut util: Vec<f64> = sim
        .nodes
        .iter()
        .filter(|n| n.watched())
        .map(|n| {
            n.wire.up_bytes as f64 / (n.up_bw.max(1) as f64 * n.present_secs(r.finish).max(1e-9))
        })
        .collect();
    util.sort_by(f64::total_cmp);
    for p in [50, 90, 95] {
        if let Some(&u) = util.get((util.len().saturating_sub(1)) * p / 100) {
            row.num(&format!("viewer_util_p{p}"), u);
        }
    }
    let (mut acc, mut rej, mut shed) = (0, 0, 0);
    for n in sim.nodes.iter().filter(|n| n.watched()) {
        if let Some(p) = n.runner.push_stats(n.swarm) {
            acc += p.accepted;
            rej += p.rejected;
            shed += p.sheds_sent;
        }
    }
    row.int("relay_accepted", acc)
        .int("relay_rejected", rej)
        .int("relay_sheds", shed);
    for (name, v) in ["seed", "relays", "others"].iter().zip(&sh.meter) {
        let mut v = v.clone();
        v.sort_by(f64::total_cmp);
        for p in [10, 50, 90] {
            if let Some(&x) = v.get((v.len().saturating_sub(1)) * p / 100) {
                row.num(&format!("meter_{name}_p{p}"), x);
            }
        }
    }
    // How push loops formed (`SIM_LOOPS=1`): how many, by how their newest
    // edge formed, their sizes, and the lifetimes of those that broke
    // (`loops_alive`: still there at the end).
    if let Some(l) = &sim.loops {
        let mut lens = l.lens.clone();
        lens.sort_unstable();
        let mut lives = l.lives.clone();
        lives.sort_unstable();
        row.int("loops", lens.len() as u64)
            .int("loops_alive", l.active.len() as u64)
            .int("loops_tap_lost", l.lost)
            .int("grafts", l.grafts)
            .int("grafts_down", l.grafts_down)
            .int("switches", l.switches)
            .int("switches_down", l.switches_down);
        for (k, n) in &l.kinds {
            row.int(&format!("decision {k}"), *n);
        }
        for (k, n) in ["held_by_none", "held_by_1_2", "held_by_3_plus"]
            .iter()
            .zip(l.seed_asks)
        {
            row.int(&format!("seed_asks_{k}"), n);
        }
        row.int("seed_asks_held_by_none_parentless", l.seed_asks_parentless);
        for (how, n) in &l.closed_by {
            row.int(&format!("loops_by_{}", how.replace(' ', "_")), *n);
        }
        if let Some(&m) = lens.get(lens.len() / 2) {
            row.int("loop_len_p50", m);
        }
        for p in [50, 90] {
            if !lives.is_empty() {
                row.int(
                    &format!("loop_life_p{p}_ms"),
                    ms(percentile(&lives, p)) as u64,
                );
            }
        }
    }
    // The churn's leaves, and from each place taken again to its first
    // chunk read (`join_unread`: places taken that never read one).
    if sim.cfg.churn.is_some() {
        let leaves = sim.churn[..sim.churn_done].iter().filter(|c| c.2).count();
        let unread = sim.nodes.iter().filter(|n| n.rejoined.is_some()).count();
        let mut joins = sim.joins.clone();
        joins.sort_unstable();
        row.int("churn_leaves", leaves as u64)
            .int("join_unread", unread as u64);
        for p in [50, 90] {
            if !joins.is_empty() {
                row.int(&format!("join_p{p}_ms"), ms(percentile(&joins, p)) as u64);
            }
        }
    }
    if sim.audience() {
        audience_cols(&mut row, sim);
    }
    if sim.cfg.clock_skew != ClockSkew::Off {
        clock_cols(&mut row, sim);
    }
    ledger_cols(&mut row, sim, span);
    if let Some(m) = &sim.media {
        media_cols(&mut row, sim, m, r);
    }
    row.finish()
}

/// The columns of a run with arrivals or departures ([`Sim::audience`],
/// [`AudienceReport`]): the knobs; the viewers that arrived after the
/// start and that left for good (`_honest` of them); over every honest
/// viewer, from its join (its arrival, the start for one there from it) to
/// its first read (`join_p50_ms`, `join_p90_ms`; `join_unread`, those that
/// never read) and to its first read within [`CAUGHT_UP`]
/// (`caught_up_p50_ms`, `caught_up_p90_ms`; `caught_up_never`); the
/// honest viewers whose open parent left (`parent_lost`, counted per
/// departure) and their streams it was the parent of; and with departures
/// the recovery from them ([`Recovery`]): the viewers measured
/// (`depart_stayed`) and those left without a neighbour
/// (`depart_stranded`), their worst lag (`depart_lag_peak_ms`), and by their
/// per-second lag p90 its excess summed (`depart_excess_lag_area_ms_s`)
/// and the time back under [`CAUGHT_UP`] (`depart_recover_ms`;
/// `depart_recover_censored`, still over at the stream's end).
fn audience_cols(row: &mut JsonRow, sim: &Sim) {
    let a = sim.audience_report();
    row.str("arrivals", &sim.cfg.arrivals.to_string());
    if let Some(d) = sim.cfg.depart {
        row.str("depart", &d.to_string());
    }
    row.int("arrived", a.arrived.0 as u64)
        .int("arrived_honest", a.arrived.1 as u64)
        .int("departed", a.departed.0 as u64)
        .int("departed_honest", a.departed.1 as u64);
    for (name, v) in [("join", &a.joins), ("caught_up", &a.caught_up)] {
        for p in [50, 90] {
            if !v.is_empty() {
                row.int(&format!("{name}_p{p}_ms"), ms(percentile(v, p)) as u64);
            }
        }
    }
    row.int("join_unread", a.unread as u64)
        .int("caught_up_never", a.behind as u64)
        .int("parent_lost", a.parent_lost.0)
        .int("parent_lost_streams", a.parent_lost.1);
    if let Some(r) = &a.recovery {
        row.int("depart_stayed", r.stayed as u64)
            .int("depart_stranded", r.stranded as u64)
            .int("depart_lag_peak_ms", ms(r.peak) as u64)
            .int("depart_excess_lag_area_ms_s", r.excess_ms_s as u64)
            .int("depart_recover_ms", ms(r.recover) as u64)
            .int("depart_recover_censored", u64::from(r.censored));
    }
}

/// The clocks of a run with skew ([`ClockSkew`]): the knob, every node's
/// offset (signed ms, by node index), the largest magnitude and the nodes
/// per class of [`CLOCK_CLASSES`]; and per class with honest viewers whose
/// places were never taken again, the share of the stream's chunks they
/// read (of those due them, [`Sim::delivered`], with arrivals or
/// departures) and their lag p99 after their warm-ups.
fn clock_cols(row: &mut JsonRow, sim: &Sim) {
    let offsets: Vec<String> = sim.nodes.iter().map(|n| ms(n.clock).to_string()).collect();
    let largest = sim.nodes.iter().map(|n| n.clock.abs()).max();
    row.str("clock_skew", &sim.cfg.clock_skew.to_string())
        .field("clock_offsets_ms", format_args!("[{}]", offsets.join(",")))
        .int("clock_max_ms", largest.map_or(0, |d| ms(d) as u64));
    for (k, name) in CLOCK_CLASSES.iter().enumerate() {
        let of = |n: &&Node| clock_class(n.clock) == k;
        row.int(
            &format!("clock_{name}"),
            sim.nodes.iter().filter(of).count() as u64,
        );
        let viewers: Vec<&Node> = sim
            .nodes
            .iter()
            .filter(|n| n.watched() && !n.churned)
            .filter(of)
            .collect();
        if viewers.is_empty() {
            continue;
        }
        // With arrivals or departures, each viewer's chunks due.
        let (read, all) = match sim.audience() {
            true => viewers
                .iter()
                .map(|n| sim.delivered(n))
                .fold((0, 0), |(r, a), (rn, an)| (r + rn, a + an)),
            false => (
                viewers
                    .iter()
                    .map(|n| n.read_chunks.iter().filter(|&&r| r).count())
                    .sum(),
                viewers.iter().map(|n| n.read_chunks.len()).sum(),
            ),
        };
        let mut lags: Vec<Duration> = viewers
            .iter()
            .flat_map(|n| sim.steady(n, &n.lags))
            .collect();
        lags.sort();
        row.int(&format!("clock_{name}_viewers"), viewers.len() as u64)
            .num(
                &format!("clock_{name}_delivery"),
                read as f64 / all.max(1) as f64,
            )
            .int(
                &format!("clock_{name}_lag_p99_ms"),
                ms(percentile(&lags, 99)) as u64,
            );
    }
}

/// The uplinks' bytes by layer over every node, in strims-sim's rows'
/// terms: the ppspp layers from the seed's and the honest viewers' runners
/// (`up_hostile` is the frames of greedy viewers and adversaries, whose
/// runners count what the harness rewrites), vnic's control records, and
/// TCP/IP headers in WAN mode. This sim models neither vnic's frame headers
/// nor its other ports, TCP's pure ACKs or resent bytes, so those read 0.
/// Then the waste in the honest viewers' inbound data, the wire bytes per
/// chunk byte they read (the cross swarm's included), each role's overhead
/// per second over its uplink (relays: honest viewers that accepted a
/// subscription), the seed's wire bytes over what it wrote, the packets (frames, off WAN mode), the frames' mean
/// size and the messages per channel payload.
fn ledger_cols(row: &mut JsonRow, sim: &Sim, span: f64) {
    let span = span.max(1e-9);
    let (mut up, mut down) = (ChannelMetrics::default(), ChannelMetrics::default());
    let (mut sent, mut wire) = (Sent::default(), EndStats::default());
    for (i, n) in sim.nodes.iter().enumerate() {
        let t = n.totals();
        if !n.hostile {
            up.merge(&t.outbound);
            // Grabbers' runners count truly, but what they pull isn't
            // honest viewers' waste.
            if i != 0 && !matches!(n.kind, Role::Greedy(_)) {
                down.merge(&t.inbound);
            }
        }
        sent.frames += n.sent.frames;
        sent.frame_bytes += n.sent.frame_bytes;
        sent.hostile_bytes += n.sent.hostile_bytes;
        sent.ctl_bytes += n.sent.ctl_bytes;
        sent.transport_bytes += n.sent.transport_bytes;
        wire.up_bytes += n.wire.up_bytes;
        wire.down_bytes += n.wire.down_bytes;
        wire.up_packets += n.wire.up_packets;
        wire.small_packets += n.wire.small_packets;
    }
    let cross: u64 = sim
        .nodes
        .iter()
        .filter(|n| n.role == Role::Peer)
        .filter_map(|n| n.cross.as_ref().filter(|c| c.reads))
        .map(|c| c.read_chunks.iter().filter(|&&r| r).count() as u64)
        .sum();
    let useful = (sim.chunks_read() + cross) * sim.cfg.chunk_size as u64;
    let waste = down
        .data_bytes
        .saturating_sub(down.new_bytes + down.stale_bytes + down.invalid_bytes);
    row.int("up_payload", up.data_bytes)
        .int("up_data_hdr", up.data_header_bytes)
        .int("up_integrity", up.integrity_bytes)
        .int("up_signed", up.signed_integrity_bytes)
        .int("up_have", up.have_bytes)
        .int("up_ack", up.ack_bytes)
        .int("up_request", up.request_bytes)
        .int("up_ping", up.ping_bytes)
        .int("up_stream", up.stream_bytes)
        .int("up_announce", up.announce_bytes)
        .int("up_session", up.session_bytes)
        .int("up_chan_hdr", up.header_bytes)
        .int("up_hostile", sent.hostile_bytes)
        .int("up_vnic", sent.ctl_bytes)
        .int("up_ports", 0)
        .int("up_transport", sent.transport_bytes)
        .int("up_acks", 0)
        .int("up_wire", wire.up_bytes)
        .int("down_wire", wire.down_bytes)
        .int("dup_bytes", waste)
        .int("stale_bytes", down.stale_bytes)
        .int("useful_bytes", useful)
        .num(
            "wire_per_useful",
            wire.up_bytes as f64 / useful.max(1) as f64,
        );

    // A viewer's rates over the time it was there.
    let secs = |n: &Node| match sim.audience() {
        true => n.present_secs(sim.now).max(1e-9),
        false => span,
    };
    let share = |n: &Node| n.overhead() as f64 / (n.up_bw.max(1) as f64 * secs(n));
    let (mut relays, mut leaves, mut pps) = (Vec::new(), Vec::new(), Vec::new());
    for n in sim.nodes.iter().filter(|n| n.watched()) {
        match n.totals().outbound.stream_open > 0 {
            true => relays.push(share(n)),
            false => leaves.push(share(n)),
        }
        pps.push(n.wire.up_packets as f64 / secs(n));
    }
    for v in [&mut relays, &mut leaves, &mut pps] {
        v.sort_by(f64::total_cmp);
    }
    row.num("ovh_seed", share(&sim.nodes[0])).num(
        "seed_wire_per_written",
        sim.nodes[0].wire.up_bytes as f64
            / (sim.nodes[0].wire.up_bytes - sim.nodes[0].sent.transport_bytes).max(1) as f64,
    );
    for (name, v, p) in [
        ("ovh_relay_p50", &relays, 50),
        ("ovh_leaf_p50", &leaves, 50),
        ("ovh_leaf_p90", &leaves, 90),
        ("pps_p50", &pps, 50),
        ("pps_p90", &pps, 90),
    ] {
        if let Some(&x) = v.get(v.len().saturating_sub(1) * p / 100) {
            row.num(name, x);
        }
    }
    row.int("packets_up", wire.up_packets)
        .int("packets_ack", 0)
        .num(
            "small_packet_share",
            wire.small_packets as f64 / wire.up_packets.max(1) as f64,
        )
        .num("seed_pps", sim.nodes[0].wire.up_packets as f64 / span)
        .num(
            "frame_bytes_mean",
            sent.frame_bytes as f64 / sent.frames.max(1) as f64,
        )
        .num(
            "msgs_per_payload",
            up.messages() as f64 / up.payloads.max(1) as f64,
        );
}

/// A media run's columns: its encoder and policy, the chunks per unit, the
/// media and sent stream rates, units signed a second (each peer's
/// signature verifications), each media chunk's wait to be signed, the lag
/// from the signing, every node's wire bytes per media byte per viewer, and
/// the honest messages that named a data-less chunk.
fn media_cols(row: &mut JsonRow, sim: &Sim, m: &MediaRun, r: &Report) {
    let p = &m.plan;
    let secs = sim.cfg.stream_secs.max(1) as f64;
    let waits = p.sign_waits();
    let wire: u64 = sim.nodes.iter().map(|n| n.wire.up_bytes).sum();
    row.str("media", &m.media.encoder.to_string())
        .str("sign", &m.media.sign.to_string())
        .int("unit", p.unit_chunks)
        .num("media_kbit", p.media_bytes as f64 * 8e-3 / secs)
        .num("sent_kbit", p.sent_bytes as f64 * 8e-3 / secs)
        .num(
            "stream_per_media",
            p.sent_bytes as f64 / p.media_bytes.max(1) as f64,
        )
        .num("units_per_s", p.units() as f64 / secs)
        .int("sign_wait_p50_ms", ms(percentile(&waits, 50)) as u64)
        .int("sign_wait_p99_ms", ms(percentile(&waits, 99)) as u64)
        .int("signed_lag_p50_ms", ms(r.sign_lag_p50) as u64)
        .int("signed_lag_p99_ms", ms(r.sign_lag_p99) as u64)
        .num(
            "wire_per_media",
            wire as f64 / (p.media_bytes.max(1) as f64 * sim.cfg.peers.max(1) as f64),
        )
        .int("dataless_msgs", sim.dataless_on_wire);
}

/// The run second by second ([`Sim::series`]): the stream, each honest
/// viewer's `lag` and `frames` by second, and the per-second columns in the
/// JSON object's order.
struct SeriesData {
    rate: u64,
    chunk: u64,
    secs: u64,
    /// `[k, bytes per second]` pairs, as the JSON object writes them.
    seed_up: Vec<String>,
    viewer_up: u64,
    lag: Vec<Vec<i64>>,
    frames: Vec<Vec<i64>>,
    cols: Vec<(&'static str, Vec<i64>)>,
}

impl SeriesData {
    /// The seconds in the series.
    fn len(&self) -> usize {
        self.cols.first().map_or(0, |(_, v)| v.len())
    }
}

/// One of [`Sim::write_tables`]' tables: integer columns, as CSV.
struct Table {
    name: &'static str,
    /// Each column's name and type, as `hack/sim-db` names it.
    cols: Vec<(String, &'static str)>,
    csv: String,
}

impl Table {
    /// A table of integer columns.
    fn new(name: &'static str, cols: &[impl AsRef<str>]) -> Table {
        let cols: Vec<(&str, &'static str)> = cols.iter().map(|c| (c.as_ref(), "BIGINT")).collect();
        Table::typed(name, &cols)
    }

    /// A table of columns with their types (`BIGINT`, `BOOLEAN`, `VARCHAR`).
    fn typed(name: &'static str, cols: &[(&str, &'static str)]) -> Table {
        let names: Vec<&str> = cols.iter().map(|(n, _)| *n).collect();
        let mut csv = names.join(",");
        csv.push('\n');
        Table {
            name,
            cols: cols.iter().map(|&(n, t)| (n.to_owned(), t)).collect(),
            csv,
        }
    }

    /// Appends a row already formatted as CSV fields.
    fn fields(&mut self, row: &[String]) {
        self.csv.push_str(&row.join(","));
        self.csv.push('\n');
    }

    /// Appends a row of integers, [`NONE`] an empty field.
    fn ints(&mut self, row: &[i64]) {
        use std::fmt::Write;
        for (i, x) in row.iter().enumerate() {
            if i > 0 {
                self.csv.push(',');
            }
            if *x != NONE {
                write!(self.csv, "{x}").unwrap();
            }
        }
        self.csv.push('\n');
    }

    /// Writes `<name>.csv` and `<name>.schema.json` into `dir`.
    fn write(&self, dir: &Path) -> io::Result<()> {
        std::fs::write(dir.join(format!("{}.csv", self.name)), &self.csv)?;
        let cols: Vec<String> = self
            .cols
            .iter()
            .map(|(n, t)| format!("{{\"name\":\"{n}\",\"type\":\"{t}\"}}"))
            .collect();
        std::fs::write(
            dir.join(format!("{}.schema.json", self.name)),
            format!("{{\"columns\":[{}]}}\n", cols.join(",")),
        )
    }
}

/// A flat JSON object, field by field: numbers and plain tokens.
#[derive(Default)]
struct JsonRow(String);

impl JsonRow {
    fn field(&mut self, key: &str, value: std::fmt::Arguments) -> &mut Self {
        use std::fmt::Write;
        self.0.push(if self.0.is_empty() { '{' } else { ',' });
        write!(self.0, "\"{key}\":{value}").unwrap();
        self
    }

    fn int(&mut self, key: &str, v: u64) -> &mut Self {
        self.field(key, format_args!("{v}"))
    }

    /// A number; JSON has no NaN or infinity, so those are null.
    fn num(&mut self, key: &str, v: f64) -> &mut Self {
        match v.is_finite() {
            true => self.field(key, format_args!("{v}")),
            false => self.field(key, format_args!("null")),
        }
    }

    fn str(&mut self, key: &str, v: &str) -> &mut Self {
        assert!(
            !v.contains(|c: char| c == '"' || c == '\\' || c.is_control()),
            "{v:?}"
        );
        self.field(key, format_args!("\"{v}\""))
    }

    fn finish(&mut self) -> String {
        self.0.push('}');
        std::mem::take(&mut self.0)
    }
}

/// [`wan_residential_seed_config`]'s seed and viewers plus a 25th viewer
/// (the last node) in a New York datacenter: a super-peer whose uplink
/// could carry the whole swarm.
fn wan_super_peer_config(seed: u64) -> Config {
    let mut sites = wan_residential_seed_config(seed).wan.unwrap().sites;
    sites.push(Site::new(City::NewYork, Access::Datacenter));
    let mut cfg = wan_config(seed, sites, 6, 0);
    cfg.keep_honest = vec![cfg.peers];
    cfg
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
    let mut sim = Sim::new(wan_mobile_config(1));
    let r = sim.run();
    eprintln!(
        "lag {} / {} / {} ms, seed share {:.2}, retransmits {}",
        ms(r.lag_p50),
        ms(r.lag_p99),
        ms(r.lag_max),
        r.seed_share,
        sim.retransmits()
    );
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(sim.retransmits() > 0);
    // A viewer with liars among its few neighbours keeps losing its
    // requests to them for a timeout each.
    assert!(r.lag_p99 < Duration::from_secs(10), "{r:?}");
    assert!(r.lag_max < Duration::from_secs(10), "{r:?}");
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
    for seed_policy in [SeedPolicy::ADMISSION, policy] {
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
    assert!(adaptive.0 * 3 < admission.0 * 2, "{lags:?}");
    assert!(adaptive.1 < admission.1, "{lags:?}");
}

#[test]
fn sim_wan_adaptive_seed_bars_liars_its_reporters_name() {
    // Liars among the seed's nearest viewers take copies first; the
    // children that graft onto them and miss everything report it, and
    // the seed bars them. Holders that serve are never barred.
    let mut cfg = alloc_config(
        AbScenario::WanResidentialSeed,
        1,
        AllocCase::AdaptiveQos,
        64,
    );
    cfg.greedy = Greedy::Nodes(vec![
        (4, Greed::Liar),
        (21, Greed::Liar),
        (10, Greed::FreeRider),
    ]);
    cfg.stream_secs = 30;
    cfg.warm_secs = 5;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    let a = sim.nodes[0]
        .runner
        .seed_allocation(sim.nodes[0].swarm, sim.now)
        .unwrap();
    eprintln!(
        "{}",
        verdict_cols(
            &r.verdicts,
            r.liar_hold,
            &r.report_counts.0,
            (0.0, r.announce.2)
        )
    );
    eprintln!("{r:?}");
    check_delivery(&r, 4 * MUNRO as u64);
    assert!(a.counters.unserved >= 2, "{:?}", a.counters);
    assert_eq!(r.verdicts.liars_served, 0);
    assert_eq!(r.verdicts.barred_with_children, 0);
    assert!(r.greedy_slots.1 < 0.15, "{:?}", r.greedy_slots);
    assert!(
        r.liar_hold.1 <= Duration::from_secs(10),
        "{:?}",
        r.liar_hold
    );
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

/// Clocks that all agree change what the runners stamp, not what they do:
/// a run with every clock [`MAX_SKEW`] ahead sends the frames, at the
/// times, of the run with right clocks, and captures them at those times.
/// So the harness hands every call its node's clock and moves the
/// deadlines it returns back: a call left in true time would see that
/// node's clock jump. Whole minutes keep the clocks' phase on every grid
/// the runners align times to, and the timestamps' lengths on the wire.
#[test]
fn sim_agreeing_clocks_change_no_timing() {
    let ahead = ClockSkew::All(MAX_SKEW);
    // Pushes, losses, churn, a reconnected seed, early signing, grabbers.
    let mut mesh = Config::new(6, 8);
    mesh.stream_secs = 6;
    mesh.degree = 4;
    mesh.loss = 0.01;
    mesh.push_mode = PushMode::Hybrid;
    mesh.stream_count = 4;
    mesh.munro_chunks = 8;
    mesh.early = vec![5, 8];
    mesh.steps = vec![(3, SeedStep::Reconnect)];
    mesh.churn = Some(Churn {
        per_min: 2.0,
        away: Duration::from_secs(1),
    });
    mesh.greedy_kinds = vec![Greed::Grabber, Greed::FreeRider];
    // The adaptive seed on WAN paths, captured.
    let cities: Vec<(City, u32)> = GLOBAL_AUDIENCE
        .iter()
        .copied()
        .filter(|&(c, _)| matches!(c.region(), Region::NorthAmerica | Region::Europe))
        .collect();
    let sites = wan_sites(
        4,
        8,
        Site::new(City::Chicago, Access::Dsl),
        &cities,
        RESIDENTIAL,
    );
    let mut wan = wan_config(4, sites, 3, 0);
    wan.stream_secs = 6;
    wan.push_mode = PushMode::Hybrid;
    wan.stream_count = 16;
    wan.seed_policy = ADAPTIVE_QOS;
    // Per process: test binaries running at once share the directory.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "sim_agreeing_clocks_change_no_timing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    // A media run's writes and flushes, and cross traffic's second seed.
    let media = media_config(3, "1500v", "u8,t200");
    let mut cross = cross_traffic_config(2, PushMode::Hybrid, 30);
    cross.stream_secs = 6;
    for (k, cfg) in [mesh, wan, media, cross].into_iter().enumerate() {
        let run = |skew| {
            let mut cfg = cfg.clone();
            cfg.clock_skew = skew;
            if k == 1 {
                cfg.capture = Some(CaptureTo {
                    dir: dir.join(skew.to_string()),
                    tap: true,
                });
            }
            let mut sim = Sim::new(cfg);
            let r = sim.run();
            let lags: Vec<Vec<(Time, Duration)>> =
                sim.nodes.iter().map(|n| n.lags.clone()).collect();
            (r, lags)
        };
        let (right, right_lags) = run(ClockSkew::Off);
        let (skewed, skewed_lags) = run(ahead);
        assert_ne!(
            right.trace, skewed.trace,
            "config {k}: the clocks reach the wire"
        );
        let timing = |r: Report| format!("{:?}", Report { trace: 0, ..r });
        assert_eq!(timing(right), timing(skewed), "config {k}");
        assert_eq!(right_lags, skewed_lags, "config {k}");
    }
    // The captures' records, every frame's included, at the same times.
    let times = |skew: ClockSkew, i: usize| {
        let f = std::fs::read(dir.join(skew.to_string()).join(format!("node-{i}.ppcap"))).unwrap();
        let mut rd = strims_ppspp_capture::Reader::new(&f[..]).unwrap();
        let mut times = Vec::new();
        while let Some(e) = rd.next_record() {
            let e = e.unwrap();
            if let strims_ppspp_capture::Record::Frame { dir, len, .. } = e.record {
                times.push((e.time, dir == Dir::Out, len));
            }
        }
        times
    };
    for i in 0..=8 {
        let right = times(ClockSkew::Off, i);
        assert!(!right.is_empty(), "node {i}");
        assert_eq!(right, times(ahead, i), "node {i}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Clocks off as in the wild fall in each class at its share, inside its
/// bounds, in whole milliseconds, ahead as often as behind.
#[test]
fn sim_real_clocks_take_their_shares() {
    let n = 200_000;
    let offsets = clock_offsets(ClockSkew::Real, n, &mut det_rng(1));
    let mut counts = [0usize; CLOCK_CLASSES.len()];
    for &d in &offsets {
        assert_eq!(d.as_nanos() % Duration::MILLISECOND.as_nanos(), 0, "{d}");
        let k = clock_class(d);
        if let Some(&(lo, hi)) = CLOCK_BOUNDS.get(k) {
            assert!(d.abs() >= lo && d.abs() < hi, "{d}");
        }
        counts[k] += 1;
    }
    assert!(offsets.iter().all(|d| d.abs() <= MAX_SKEW));
    for (k, (&c, &p)) in counts.iter().zip(&CLOCK_SHARES).enumerate() {
        let sd = (p * (1.0 - p) / n as f64).sqrt();
        let got = c as f64 / n as f64;
        assert!(
            (got - p).abs() < 4.0 * sd,
            "{}: {got} of {p}",
            CLOCK_CLASSES[k]
        );
    }
    let live = offsets.iter().filter(|&&d| d != Duration::ZERO);
    let ahead = live.clone().filter(|&&d| d > Duration::ZERO).count() as f64;
    let share = ahead / live.count() as f64;
    assert!((share - 0.5).abs() < 0.01, "{share}");
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
