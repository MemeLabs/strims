// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic swarm simulation: the runner's correctness harness.
//!
//! The tests check what must hold however a swarm is timed: every honest
//! viewer reads the stream it is owed ([`check_delivery`]), every byte it
//! reads is the stream's, runner state and memory stay bounded under
//! attack, the protocol's invariants hold on the wire, every attacker
//! does what it is built to, runs replay exactly, and what only observes
//! (captures, chunk codecs, clocks that agree) changes nothing. None of
//! their timing gates a change: the links here model no socket queue and
//! no acknowledgement, and no node layer runs, so lags, queues and shares
//! are the model's. `strims-sim`, real nodes over turmoil, is the sim that
//! measures.
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
//! seed published it). No store keeps the stream's bytes, only which chunks
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
//! Grabbers, a kind a run draws only when [`Config::greedy_kinds`] names
//! it, serve like honest viewers but request every chunk announced to them
//! from everyone who announced it, never cancel, and ignore chokes. With
//! [`Config::collude`] the greedy viewers' reports to the seed praise liars
//! and defame the other holders ([`Sim::collusion`]).
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
//! wants. [`Config::echo_greed`] makes them anyway, to check that: free
//! riders hold their echoes [`ECHO_HOLD`], and liars echo nonces they never
//! read.
//! The runners' upload meters and adaptive seeds read it.
//!
//! The `sim_push_*` tests run the push/pull hybrid's scenarios
//! ([`Scenario`]) in a [`PushMode`] and stream count each.
//!
//! With [`Config::early`] the seed signs munros early, so they end in
//! data-less chunks: readers expect zeros there, and the harness counts
//! every message on the wire that names one ([`Sim::dataless_messages`]).
//!
//! Cross traffic is a second swarm in the same runners, seeded by a peer,
//! viewed by every other node, sharing every uplink.
//!
//! Nodes' clocks are off ([`ClockSkew`]), as production nodes' wall clocks
//! are, unless [`Config::clock_skew`] turns them off: each runner reads its
//! own clock in every call, and stamps what it writes with it, while every
//! result is measured in true time.
//!
//! Viewers come and go as an audience does ([`Arrivals`], [`Depart`]): by
//! default every viewer links at the start, with the seed. With
//! [`Config::arrivals`] the seed starts alone and the viewers trickle in,
//! or a scheduled event brings a burst into a warm swarm; each arrival
//! links to viewers already there, as a tracker's answer would, and reads
//! from the live edge. With [`Config::depart`] a share of them leaves for
//! good, closing its links or going silent, and a viewer whose link to one
//! closes asks for another neighbour the same way. Results then measure
//! each viewer from its own arrival to its departure. Their times count
//! from the stream's start.

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
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::{
    Announce, AnnounceEntry, Cancel, ChannelWriter, Data, Delays, FrameReader, Have,
    MAX_ANNOUNCE_ENTRIES, Message, MessageWriter, Pong, Reader, ReaderConfig, Request, Stream,
    StreamClose, StreamOpen, StreamRequest,
};
use strims_ppspp::store::{self, BufferReader, ChunkBytes, ChunkCodec};
use strims_ppspp::{
    ChannelTotals, DATA_QUEUE_CAP, DeliveryMode, Event, MAX_BIN, PeerKey, PushMode, PushStats,
    Runner, SeedAllocation, SeedPolicy, SeedQos, SwarmId, SwarmKey, SwarmOptions, Writer,
    WriterOptions,
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
/// How long a free rider holds its echoes with [`Config::echo_greed`].
const ECHO_HOLD: Duration = Duration::from_millis(300);

/// A viewer that leaves for good is owed the chunks published at least
/// this long before it left ([`Sim::delivered`]): delivery's share of the
/// 2 s glass-to-glass target.
const CAUGHT_UP: Duration = Duration::from_millis(1250);

/// How long a node's link to a viewer that left abruptly ([`Depart`])
/// stays open: node-core's read timeout on TCP and WebSocket links
/// (`strims_vnic::TcpOptions::read_timeout`, 25 s; vnic's keepalives keep a
/// live link from reaching it), the transport noticing a peer gone silent
/// (WebRTC's consent checks lapse at about 30 s). A client asks for another
/// neighbour once that link closes ([`Sim::relink`]): it can't know sooner
/// which silent peer is gone, nor have its tracker know.
const READ_TIMEOUT: Duration = Duration::from_secs(25);

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
/// start. The audience's times ([`Arrivals`], [`Depart`]) count from it.
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

    /// Whether it is a dishonest peer's: a greedy viewer's or an
    /// adversary's.
    fn dishonest(self) -> bool {
        matches!(self, Role::Greedy(_) | Role::Adversary(_))
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
    /// A second swarm sharing the nodes.
    cross: Option<Cross>,
    /// How the seed spends its upload in Hybrid.
    seed_policy: SeedPolicy,
    /// Real-world paths. With them `latency_ms`, `loss`, `seed_up`,
    /// `peer_up`, `peer_down` and `constrained` go unused, and `loss` must
    /// stay zero.
    wan: Option<Wan>,
    /// Capture every node's frames (see [`CaptureTo`]).
    capture: Option<CaptureTo>,
    /// Greedy viewers hold or forge their echoes (see the module docs).
    echo_greed: bool,
    /// Greedy viewers collude in their reports to the seed
    /// ([`Sim::collusion`]).
    collude: bool,
    /// The kinds a drawn run's greedy viewers take in turn.
    greedy_kinds: Vec<Greed>,
    /// Honest viewers leaving mid-stream and new ones taking their places.
    churn: Option<Churn>,
    /// What the nodes' stores keep of each chunk: any choice gives the same
    /// run ([`sim_chunk_codecs_change_no_trace`]), and only [`Keep::Bytes`]
    /// holds the stream at every node.
    keep: Keep,
    /// A live encoder's stream signed early, in place of whole munros at
    /// `stream_rate`.
    media: Option<Media>,
    /// How far the nodes' clocks are off.
    clock_skew: ClockSkew,
    /// When the viewers arrive.
    arrivals: Arrivals,
    /// Viewers leaving for good.
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
/// stream under every profile. Arrivals take no churn or cross traffic,
/// and all come before the stream's last write. Times
/// count from the stream's start ([`stream_start`]), a second after the
/// seed's.
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

/// Viewers leaving for good: `share` of the viewers there `at` into the
/// stream ([`stream_start`]; honest, greedy and adversaries alike, drawn at
/// random) leave within `window` from it, as that many Poisson departures
/// in it, and the rest keep watching. A departure is graceful, its links
/// closing at once as a closed tab's transport does (its neighbours'
/// `remove_peer`), or with `abrupt` silent, as a lost connection is: its
/// frames stop, its neighbours' trees notice only by the silence, as
/// [`Incident::Dies`], and their links to it close at their read timeouts
/// ([`READ_TIMEOUT`]). A viewer whose link to one that left closes asks
/// for another neighbour, as a tracker would answer it ([`Sim::relink`]),
/// so the trees repair over the mesh that's left and the links it gains: a
/// viewer is left with no neighbour there only while its links to viewers
/// that went silent wait out their timeouts. A viewer that leaves counts in
/// every result up to its departure. Drawn after the arrivals, from the
/// run's rng; departures take what [`Arrivals`] do not.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Depart {
    share: f64,
    at: Duration,
    window: Duration,
    abrupt: bool,
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
    /// Every clock is off by exactly this (at most [`MAX_SKEW`]), so they
    /// all agree: what the runners do is then timed as without it, which
    /// checks that the harness hands every call its node's clock
    /// ([`sim_agreeing_clocks_change_no_timing`]).
    All(Duration),
}

/// A run's clocks as a capture directory's name.
impl std::fmt::Display for ClockSkew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ClockSkew::Off => f.write_str("off"),
            ClockSkew::Real => f.write_str("real"),
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
/// sign either way at even odds.
const CLOCK_SHARES: [f64; 4] = [0.90, 0.05, 0.04, 0.01];

/// Each class's bounds on an offset's magnitude, `[lo, hi)`: uniform in
/// the first class, log-uniform in the others.
const CLOCK_BOUNDS: [(Duration, Duration); 4] = [
    (Duration::ZERO, Duration::from_millis(100)),
    (Duration::from_millis(100), Duration::from_secs(2)),
    (Duration::from_secs(2), Duration::from_secs(15)),
    (Duration::from_secs(15), MAX_SKEW),
];

/// The names of [`CLOCK_BOUNDS`]' classes, the last taking offsets of
/// [`MAX_SKEW`] itself.
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
            cross: None,
            seed_policy: SeedPolicy::ADMISSION,
            wan: None,
            capture: None,
            echo_greed: false,
            collude: false,
            greedy_kinds: vec![Greed::FreeRider, Greed::Liar],
            churn: None,
            keep: Keep::Nothing,
            media: None,
            clock_skew: ClockSkew::Real,
            arrivals: Arrivals::T0,
            depart: None,
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
    /// The chunk of each of `lags`.
    read_seqs: Vec<u32>,
    /// In a media run, each media chunk's read time and its lag from its
    /// unit's signing.
    sign_lags: Vec<(Time, Duration)>,
    /// The chunks it read, by index.
    read_chunks: Vec<bool>,
    removed: BTreeSet<PeerKey>,
    /// The cross-traffic swarm, if any.
    cross: Option<Side>,
    /// Bytes and packets through its uplink and downlink (TCP framing
    /// included in WAN mode) and its downlink's longest queue. The path's
    /// counters are the network's ([`Sim::wan_stats`]).
    wire: EndStats,
    /// The bytes of the ppspp frames its uplink carried.
    frame_bytes: u64,
    /// Its runners' counters from before it last took a place.
    past: ChannelTotals,
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
    /// Its past incarnations' joins, in order.
    past_joins: Vec<PastJoin>,
    /// When it left for good ([`Depart`]).
    left: Option<Time>,
    /// When its incarnation read its first chunk.
    first_read: Option<Time>,
    /// How far its clock is off ([`ClockSkew`]): its runner reads the true
    /// time plus this.
    clock: Duration,
}

impl Node {
    /// Its runners' counters since the run started.
    fn totals(&self) -> ChannelTotals {
        let mut t = self.past;
        t.merge(&self.runner.channel_totals());
        t
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
    /// With [`Config::collude`], when each greedy viewer reports next.
    collude_at: Vec<Time>,
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
    /// The links to viewers that left abruptly, each closing at its
    /// neighbour's read timeout ([`READ_TIMEOUT`]): `(at, neighbour,
    /// viewer gone)`.
    closing: BTreeSet<(Time, usize, usize)>,
    /// The links viewers made asking again for neighbours that left
    /// ([`Sim::relink`]).
    relinks: u64,
    /// From each place taken again to its viewer's first chunk read.
    joins: Vec<Duration>,
    /// What a new viewer's swarm is made from.
    swarm_id: SwarmId,
    options: SwarmOptions,
    /// A media run's stream.
    media: Option<MediaRun>,
}

/// A media run's stream ([`Media`]): its plan, the ops written, when the
/// stream starts (the plan's times count from it), the main swarm's codec,
/// and per chunk when its unit was signed.
struct MediaRun {
    plan: Plan,
    done: usize,
    start: Time,
    padded: Arc<Padded>,
    signed: Vec<Time>,
}

// Every field is printed through `Debug`.
#[allow(dead_code)]
#[derive(Debug)]
struct Report {
    trace: u64,
    frames: u64,
    bytes: u64,
    /// The stream's bytes.
    total: u64,
    /// Per honest viewer there at the end: start offset, final position.
    positions: Vec<(u64, u64)>,
    /// Of the chunks due to the honest viewers whose places were never
    /// taken again ([`Sim::delivered`]), the share they read.
    delivery: f64,
    mismatches: u64,
    underruns: u64,
    resets: u64,
    /// Push counters: a child's summed over honest peers, a parent's over
    /// every node but the greedy viewers.
    push: PushStats,
    /// The cross-traffic swarm's readers.
    cross: Option<CrossReport>,
}

/// A node's past incarnation's join ([`Node::past_joins`]): when it read
/// its first chunk, and whether it was an honest viewer's.
#[derive(Clone, Copy, Debug)]
struct PastJoin {
    first_read: Option<Time>,
    honest: bool,
}

/// The run's arrivals and departures ([`Arrivals`], [`Depart`]), over its
/// honest viewers, those that left included.
#[derive(Debug, Default)]
struct AudienceReport {
    /// Viewers that arrived after the start, and that left for good: the
    /// honest ones ([`Node::watched`]), and the greedy viewers and
    /// adversaries.
    arrived: (usize, usize),
    departed: (usize, usize),
    /// Honest viewers whose first join never read a chunk.
    unread: usize,
    /// With departures, how the honest viewers there before they started
    /// came through them ([`Sim::recovery`]).
    recovery: Option<Recovery>,
}

/// The honest viewers there before the departures started that stayed,
/// and those of them left at the end with no link to a node still there
/// ([`Sim::relink`]).
#[derive(Debug)]
struct Recovery {
    stayed: usize,
    stranded: usize,
}

/// The cross-traffic swarm's readers.
#[allow(dead_code)]
#[derive(Debug)]
struct CrossReport {
    /// The share of its chunks they read.
    delivery: f64,
    /// Those that didn't read it whole, or read a wrong byte.
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
        let capture = cfg.capture.clone();
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
                cfg.churn.is_none() && cfg.cross.is_none(),
                "arrivals and departures take no churn or cross traffic"
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
                first_read: None,
                sign_lags: vec![],
                read_chunks: vec![],
                removed: BTreeSet::new(),
                cross,
                wire: EndStats::default(),
                frame_bytes: 0,
                past: ChannelTotals::default(),
                hostile: matches!(role, Role::Greedy(g) if g != Greed::Grabber)
                    || matches!(role, Role::Adversary(_)),
                cap: None,
                churned: false,
                rejoined: None,
                arrival: t0(),
                past_joins: Vec::new(),
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
        let munro = cfg.munro_chunks * cfg.chunk_size;
        let munros = match &media {
            Some(m) => m.plan.units() as usize,
            None => (cfg.stream_rate * cfg.stream_secs) as usize / munro,
        };
        for node in &mut nodes {
            node.read_chunks = vec![false; munros * cfg.munro_chunks];
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
            wakes: BTreeSet::new(),
            wake_of,
            stale: every.clone(),
            undrained: every,
            collude_at,
            churn,
            churn_done: 0,
            arrivals,
            arrivals_done: 0,
            departures,
            departures_done: 0,
            closing: BTreeSet::new(),
            relinks: 0,
            joins: Vec::new(),
            swarm_id: SwarmId::new(&key.public),
            options,
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
        if let Some(&(at, _, _)) = self.closing.first() {
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
        while !self.done() {
            let t = self.next_time();
            if t > self.end {
                break;
            }
            self.now = self.now.max(t);
            self.step();
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
                    self.nodes[victim].role = Role::Dead;
                }
                Incident::Slows(bw) => {
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
                    self.nodes[0].up_bw = bw;
                }
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
        while let Some(&(at, j, i)) = self.closing.first() {
            if now < at {
                break;
            }
            self.closing.pop_first();
            self.timed_out(j, i);
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
        let swarm = strims_ppspp::Swarm::new(self.swarm_id.clone(), self.options.clone()).unwrap();
        let reader = swarm.reader();
        let key = runner.add_swarm(swarm);
        let n = &mut self.nodes[i];
        n.past_joins.push(PastJoin {
            first_read: n.first_read,
            honest: n.kind == Role::Peer,
        });
        if !n.hostile {
            n.past.merge(&n.runner.channel_totals());
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
        n.first_read = None;
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
        let n = &mut self.nodes[i];
        n.role = n.kind;
        n.arrival = now;
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
        self.link_to(i, to);
    }

    /// Links node `i` to each of `to` (in node order), each link's latency
    /// drawn as the mesh's are (the WAN's paths, in WAN mode).
    fn link_to(&mut self, i: usize, mut to: Vec<usize>) {
        let now = self.now;
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
    /// see its links close and drop them, and each viewer among them asks
    /// for another at once ([`Sim::relink`]); abruptly, it goes silent, and
    /// its neighbours' links to it close at their read timeouts
    /// ([`READ_TIMEOUT`], [`Sim::timed_out`]).
    fn depart(&mut self, i: usize) {
        if self.nodes[i].role.off() {
            return;
        }
        let abrupt = self.cfg.depart.is_some_and(|d| d.abrupt);
        let links: Vec<(usize, usize)> =
            self.nodes[i].links.iter().map(|(&j, &l)| (j, l)).collect();
        let n = &mut self.nodes[i];
        n.role = Role::Gone;
        n.left = Some(self.now);
        if abrupt {
            let at = self.now + READ_TIMEOUT;
            for &(j, _) in &links {
                if !self.nodes[j].role.off() {
                    self.closing.insert((at, j, i));
                }
            }
        } else {
            for &(j, l) in &links {
                self.unlink(j, i, l ^ 1);
            }
            for &(j, _) in &links {
                self.relink(j);
            }
        }
        self.touch(i);
    }

    /// Node `j` drops its link `l` to node `i`, gone, as its transport
    /// closes: its runner forgets the peer.
    fn unlink(&mut self, j: usize, i: usize, l: usize) {
        let rpeer = self.links[l].peer;
        self.nodes[j].runner.remove_peer(rpeer);
        let n = &mut self.nodes[j];
        n.links.remove(&i);
        n.vtime.remove(&l);
        self.touch(j);
    }

    /// Node `j`'s link to viewer `i`, which left abruptly, reaches its read
    /// timeout ([`READ_TIMEOUT`]): it closes, and `j` asks for another
    /// neighbour ([`Sim::relink`]).
    fn timed_out(&mut self, j: usize, i: usize) {
        if self.nodes[j].role.off() {
            return;
        }
        let Some(&l) = self.nodes[j].links.get(&i) else {
            return;
        };
        self.unlink(j, i, l);
        self.relink(j);
    }

    /// Viewer `j` lost a neighbour to a departure ([`Depart`]) and, left
    /// with fewer than an arrival takes ([`Config::degree`] viewers there;
    /// with [`Config::dial`], that many nodes), asks for another, as a
    /// client keeping that many peers asks its tracker again once a link
    /// closes; every link it has counts, those others made to it too. By
    /// the rule an arrival's links follow ([`Sim::arrive`]), it links to one
    /// random viewer there it has no link to, and to the seed while the
    /// seed has under [`Config::seed_degree`] viewers there (with
    /// [`Config::dial`], to one random node there it has no link to, the
    /// seed among them); in a full mesh it has a link to every one already.
    /// So a viewer that kept enough neighbours asks for none, and the mesh
    /// that's left keeps the arrivals' degree rather than filling in. The
    /// seed asks nobody: viewers link to it. The draw is the run's rng's.
    fn relink(&mut self, j: usize) {
        if j == 0 || self.nodes[j].role.off() {
            return;
        }
        let here = |k: usize| !self.nodes[k].role.off();
        let free = |k: usize| k != j && here(k) && !self.nodes[j].links.contains_key(&k);
        let linked = |seed: bool| {
            self.nodes[j]
                .links
                .keys()
                .filter(|&&k| here(k) && (seed || k != 0))
                .count()
        };
        let mut viewers: Vec<usize> = (1..self.nodes.len()).filter(|&k| free(k)).collect();
        let mut to = Vec::new();
        if self.cfg.dial > 0 {
            if linked(true) >= self.cfg.dial {
                return;
            }
            if free(0) {
                viewers.push(0);
            }
        } else {
            if self.cfg.degree > 0 && linked(false) >= self.cfg.degree {
                return;
            }
            let seed_has = self.nodes[0].links.keys().filter(|&&k| here(k)).count();
            if free(0) && (self.cfg.seed_degree == 0 || seed_has < self.cfg.seed_degree) {
                to.push(0);
            }
        }
        if !viewers.is_empty() {
            let k = (self.rng.next_u64() % viewers.len() as u64) as usize;
            to.push(viewers[k]);
        }
        self.relinks += to.len() as u64;
        self.link_to(j, to);
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
            self.nodes[to].runner.remove_peer(rpeer);
            self.nodes[to].removed.insert(rpeer);
        }
    }

    /// With [`Config::collude`], greedy viewer `i`'s report due on link `l`
    /// to the seed, once a second: for every holder the seed has now (the
    /// neighbours an adaptive seed holds copies at, or those any seed
    /// pushes to), on the stream its index picks, a liar is praised and
    /// anyone else defamed, eight pushes each.
    fn collusion(&mut self, i: usize, l: usize) -> Option<Vec<AnnounceEntry<'static>>> {
        if self.links[l].to != 0 || self.now < self.collude_at[i] || !self.cfg.collude {
            return None;
        }
        self.collude_at[i] = self.now + Duration::SECOND;
        let streams = self.cfg.stream_count.max(1);
        let seed = &self.nodes[0];
        let (mut holders, mut pushed) = (BTreeSet::new(), BTreeSet::new());
        if let Some(a) = seed
            .runner
            .seed_allocation(seed.swarm, seed.local(self.now))
        {
            for nb in &a.neighbours {
                let Some(id) = seed.runner.peer_id(nb.peer).map(id_node) else {
                    continue;
                };
                if nb.holder {
                    holders.insert(id);
                }
                if !nb.streams.is_empty() {
                    pushed.insert(id);
                }
            }
        }
        if holders.is_empty() {
            holders = pushed;
        }
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
                    for c in first..end {
                        if let Some(&pt) = self.publish_times.get(c as usize)
                            && !pt.is_nil()
                        {
                            node.lags.push((now, now - pt));
                            node.read_seqs.push(c as u32);
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
                    if end > first && node.first_read.is_none() {
                        node.first_read = Some(now);
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
        node.frame_bytes += n as u64;
        // A paced uplink (vnic's rate limit over turmoil, which models
        // no bandwidth) sends the frame at once and makes the next one
        // wait; a serial one takes the frame's transmission time.
        let depart = if self.cfg.paced_uplinks {
            now
        } else {
            node.up_free
        };

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

    fn report(&self) -> Report {
        let total = (self.munros * self.munro) as u64;
        let peers = self.nodes.iter().filter(|n| n.watched());
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
            resets: peers.map(|n| n.resets).sum(),
            push,
            cross: self.cfg.cross.map(|_| self.cross_report()),
        }
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

    /// The run's arrivals and departures, measured ([`AudienceReport`]).
    fn audience_report(&self) -> AudienceReport {
        let viewers = || self.nodes.iter().skip(1);
        let both = |f: &dyn Fn(&Node) -> bool| {
            (
                viewers().filter(|n| f(n) && n.watched()).count(),
                viewers().filter(|n| f(n) && n.kind.dishonest()).count(),
            )
        };
        let unread = viewers()
            .filter(|n| {
                let first = n.past_joins.first().copied().unwrap_or(PastJoin {
                    first_read: n.first_read,
                    honest: n.watched(),
                });
                first.honest && first.first_read.is_none()
            })
            .count();
        AudienceReport {
            arrived: both(&|n| n.arrival > t0()),
            departed: both(&|n| n.left.is_some()),
            unread,
            recovery: self
                .cfg
                .depart
                .map(|d| self.recovery(stream_start() + d.at)),
        }
    }

    /// How the honest viewers there before the departures that start at
    /// `from`, and that stayed, came through them ([`Recovery`]).
    fn recovery(&self, from: Time) -> Recovery {
        let stayed: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| n.role == Role::Peer && n.arrival < from)
            .collect();
        Recovery {
            stayed: stayed.len(),
            stranded: stayed
                .iter()
                .filter(|n| n.links.keys().all(|&j| self.nodes[j].role.off()))
                .count(),
        }
    }

    fn cross_report(&self) -> CrossReport {
        let total = (self.munros * self.munro) as u64;
        let readers: Vec<&Side> = self
            .nodes
            .iter()
            .filter_map(|n| n.cross.as_ref().filter(|x| x.reads))
            .collect();
        let (read, all) = readers.iter().fold((0, 0), |(r, a), x| {
            (
                r + x.read_chunks.iter().filter(|&&b| b).count(),
                a + x.read_chunks.len(),
            )
        });
        CrossReport {
            delivery: read as f64 / all.max(1) as f64,
            incomplete: readers
                .iter()
                .filter(|x| x.pos < total || x.mismatches > 0 || x.resets > 0)
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
/// of the viewers, the kinds ([`Config::greedy_kinds`]) alternating from a
/// random first one, from an rng of its own: the mesh, the paths and every
/// runner draw what they would without it. Viewers the config names
/// (constrained ones, the cross-traffic seed) are passed over, which leaves
/// the rest of the draw as it was.
fn greedy(cfg: &Config) -> BTreeMap<usize, Greed> {
    match &cfg.greedy {
        Greedy::Nodes(v) => {
            assert!(
                v.iter().all(|&(i, _)| (1..=cfg.peers).contains(&i)),
                "{v:?}"
            );
            v.iter().copied().collect()
        }
        Greedy::Drawn => {
            let mut rng = det_rng(cfg.seed ^ 0x4752_4545);
            let (lo, hi) = GREEDY_SHARE;
            let share = lo + (hi - lo) * (rng.next_u64() as f64 / u64::MAX as f64);
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
        "{:?}: the last of {n} viewers arrives {} s into the stream, past its end",
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
        "{d:?}: past the stream's end"
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

/// Each node's clock offset under `skew`, the seed's first. Only `Real`
/// draws.
fn clock_offsets(skew: ClockSkew, nodes: usize, rng: &mut DetRng) -> Vec<Duration> {
    let signed = |rng: &mut DetRng, d: Duration| match rng.next_u64() & 1 {
        0 => d,
        _ => -d,
    };
    (0..nodes)
        .map(|_| match skew {
            ClockSkew::Off => Duration::ZERO,
            ClockSkew::All(d) => d,
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

fn shuffle<T>(v: &mut [T], rng: &mut DetRng) {
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
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

/// How far into the stream an honest viewer there from its start may
/// begin reading, in bytes: a second at [`Config::new`]'s rate. A
/// live-edge viewer starts at the first chunk it gets, and one behind
/// relays, or whose first requests go to a hostile neighbour, gets its
/// first a few munros on.
const START_ALLOWANCE: u64 = 256 * 1024;

/// Every honest viewer there at the end read the stream from within
/// [`START_ALLOWANCE`] of its start to its end, and every byte it read was
/// the stream's.
fn check_delivery(r: &Report) {
    assert_eq!(r.mismatches, 0, "{r:?}");
    assert_eq!(r.underruns, 0, "{r:?}");
    assert_eq!(r.resets, 0, "{r:?}");
    for &(start, pos) in &r.positions {
        assert!(start <= START_ALLOWANCE, "late start {start}: {r:?}");
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
    check_delivery(&r);
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
    check_delivery(&r);
}

#[test]
fn sim_mandatory_delivery_gets_everything() {
    let mut cfg = Config::new(3, 8);
    cfg.delivery_mode = DeliveryMode::MANDATORY;
    cfg.degree = 3;
    let r = Sim::new(cfg).run();
    eprintln!("mandatory: {r:?}");
    check_delivery(&r);
    // Mandatory delivery reads from byte 0.
    assert!(r.positions.iter().all(|&(start, _)| start == 0), "{r:?}");
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

/// An honest node's runners' counters add up to the frames it sent,
/// through a viewer's place taken again, and every node's frames to the
/// run's.
#[test]
fn sim_runner_totals_are_the_frames_it_sent() {
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
    assert!(sim.nodes.iter().any(|n| n.hostile));
    for n in sim.nodes.iter().filter(|n| !n.hostile) {
        assert_eq!(n.totals().outbound.wire_bytes(), n.frame_bytes);
    }
    let frames: u64 = sim.nodes.iter().map(|n| n.frame_bytes).sum();
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
    while sim.now < t0() + Duration::from_secs(3) {
        let t = sim.next_time();
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
        check_delivery(&r);
        let m = sim.media.as_ref().unwrap();
        assert_eq!(m.done, m.plan.ops.len());
        assert!(encoder != "128c" || !m.plan.dataless.is_empty());
        assert_eq!(sim.dataless_on_wire, 0, "{encoder} {sign}");
        for n in sim.nodes.iter().filter(|n| n.role == Role::Peer) {
            assert!(n.sign_lags.iter().all(|&(_, l)| l >= Duration::ZERO));
        }
    }
}

#[test]
fn sim_ledbat_constrained_downlink() {
    // One peer's downlink is 1.5x the stream rate; everyone else would
    // happily send it more than that.
    let mut cfg = Config::new(6, 8);
    cfg.constrained = vec![(3, 3 * 256 * 1024 / 2)];
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!("constrained: {r:?}");
    check_delivery(&r);
    // Its windows stay within their bounds.
    let node = &sim.nodes[3];
    for (&remote, &l) in &node.links {
        let s = node
            .runner
            .channel_stats(sim.links[l].peer, node.swarm)
            .unwrap();
        assert!(s.scheduler.cwnd >= 2 * 1024, "{remote}: {s:?}");
        assert!(s.scheduler.flight_size >= 0);
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
    check_delivery(&r);

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
    check_delivery(&r);
    assert!(count_invalid(&sim) > 0);
}

// --- push/pull scenarios ----------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    /// 1+8, full mesh, seed uplink 16x the stream.
    FullMesh,
    /// 1+30, degree 6, seed degree 6, 1% loss.
    Sparse,
    /// 1+8, full mesh, seed uplink 8x the stream, and the busiest relay
    /// disappears at 6 s.
    ParentDies,
    /// 1+8, full mesh, seed uplink 8x the stream, and the busiest relay's
    /// uplink drops to half the stream rate at 6 s.
    SlowParent,
    /// FullMesh plus two malicious parents that accept every subscription
    /// and withhold every push.
    Withholding,
    /// 1+24 worldwide on real-world paths ([`wan_global_config`]).
    WanGlobal,
    /// 1+24 in North America and Europe, the seed on cable
    /// ([`wan_residential_seed_config`]).
    WanResidentialSeed,
}

impl Scenario {
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
        };
        cfg.push_mode = mode;
        cfg.stream_count = streams;
        cfg
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

// --- push/pull assertions ---------------------------------------------------

/// A 1+8 full mesh on 4 streams delivers, and the seed never subscribes.
#[test]
fn sim_push_full_mesh() {
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 4);
    cfg.seed = 4;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!("{r:?}");
    check_delivery(&r);
    let seed = &sim.nodes[0];
    let p = seed.runner.push_stats(seed.swarm).unwrap();
    assert!(p.parents.iter().all(Option::is_none), "{p:?}");
}

#[test]
fn sim_eager_have_full_mesh() {
    // Pull, with HAVEs and requests flushed as soon as they are queued.
    let r = Sim::new(Scenario::FullMesh.config(PushMode::EagerHave, 4)).run();
    eprintln!("{r:?}");
    check_delivery(&r);
    assert_eq!(r.push.expected, 0);
}

#[test]
fn sim_push_parent_dies() {
    // The busiest relay vanishes at 6 s without a word: its children's
    // deadlines release its streams to pull and the silence prunes it.
    let mut sim = Sim::new(Scenario::ParentDies.config(PushMode::Hybrid, 4));
    let r = sim.run();
    eprintln!("{r:?}");
    check_delivery(&r);
    let (_, victim) = sim.incident.unwrap();
    assert_eq!(sim.nodes[victim].role, Role::Dead);
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
    // takes each place 2 s on: every new one reads, the ones that stay
    // miss nothing, and nobody keeps a gone viewer as a parent.
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
    assert_eq!((r.mismatches, r.underruns, r.resets), (0, 0, 0), "{r:?}");
    for (i, n) in sim
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.role == Role::Peer)
    {
        if !n.churned {
            assert!(
                n.start.is_some_and(|s| s <= START_ALLOWANCE),
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

/// The checks every audience test makes of its honest viewers: each read
/// after its join, the stream's bytes right, and the full delivery due
/// ([`Sim::delivered`]).
fn check_arrivals(sim: &Sim, r: &Report) -> AudienceReport {
    let a = sim.audience_report();
    eprintln!("{a:?}\n{r:?}");
    assert_eq!(a.unread, 0, "{a:?}");
    assert_eq!((r.mismatches, r.underruns, r.resets), (0, 0, 0), "{r:?}");
    assert!(r.delivery > 0.999, "{r:?}");
    a
}

#[test]
fn sim_trickle_arrivals_read_from_the_live_edge() {
    // The seed starts alone and the viewers trickle in over the stream's
    // first third: each links on arrival to nodes already there, keeps its
    // role, reads from the live edge (a mandatory-delivery cell's arrivals
    // too), and every honest one reads to the end.
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
        assert_eq!(a.arrived, (12 - greedy.len(), greedy.len()), "{mode:?}");
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
    // first 6 s, the rest within 3 s from there. Every honest one reads to
    // the end.
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
    // or going silent. The rest read every chunk to the end, none of them
    // is left without a link to a node still there, and none keeps a
    // viewer gone as a parent.
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
        let r = sim.run();
        traces.push(r.trace);
        let a = check_arrivals(&sim, &r);
        assert_eq!(a.departed, (8, 0), "abrupt {abrupt}");
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
                // Gracefully, its link closed; abruptly it stays, silent,
                // short of its read timeout.
                let link = n.links.get(&g);
                assert!(abrupt || link.is_none(), "node {i} kept its link to {g}");
                let key = link.map(|&l| sim.links[l].peer);
                assert!(
                    key.is_none_or(|k| !p.parents.contains(&Some(k))),
                    "abrupt {abrupt}: node {i} keeps {g} as a parent: {p:?}"
                );
            }
        }
        let rec = a.recovery.unwrap();
        assert_eq!((rec.stayed, rec.stranded), (8, 0));
    }
    assert_ne!(traces[0], traces[1]);
}

#[test]
fn sim_departures_relink_as_a_tracker_answers() {
    // 24 honest viewers on 3 links each trickle in over the stream's first
    // 15 s, and 60 % of them leave within 2 s from 16 s in. Each viewer
    // whose link to one that left closes asks for another neighbour: at
    // the departure gracefully, at its read timeout abruptly. Every stayer
    // ends linked only to nodes there. Gracefully, each reads every chunk.
    // Abruptly, the stream stops reaching stayers whose links toward the
    // seed went silent until those links close and the viewers asking
    // again link to the seed; each stayer reads again once they do, some
    // too far behind by then to read every chunk by the stream's end.
    for abrupt in [false, true] {
        let mut cfg = audience_config(24, 24, 45, Arrivals::Trickle(None));
        cfg.greedy = Greedy::Nodes(vec![]);
        cfg.depart = Some(Depart {
            share: 0.6,
            at: Duration::from_secs(16),
            window: Duration::from_secs(2),
            abrupt,
        });
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        let a = match abrupt {
            false => check_arrivals(&sim, &r),
            true => sim.audience_report(),
        };
        assert!(sim.relinks > 0, "abrupt {abrupt}");
        assert!(sim.closing.is_empty(), "abrupt {abrupt}");
        let delay = if abrupt { READ_TIMEOUT } else { Duration::ZERO };
        let closed = sim.nodes.iter().filter_map(|n| n.left).max().unwrap() + delay;
        for (i, n) in sim.nodes.iter().enumerate().skip(1) {
            if n.role != Role::Peer {
                continue;
            }
            assert!(
                !n.links.is_empty() && n.links.keys().all(|&j| !sim.nodes[j].role.off()),
                "abrupt {abrupt}: node {i} links {:?}",
                n.links.keys().collect::<Vec<_>>()
            );
            match abrupt {
                false => assert_eq!(n.pos, r.total, "node {i} incomplete"),
                // Reading again once the links closed.
                true => assert!(
                    n.lags.last().is_some_and(|&(t, _)| t > closed),
                    "node {i} read last at {:?}",
                    n.lags.last().map(|&(t, _)| t - stream_start())
                ),
            }
        }
        let rec = a.recovery.unwrap();
        assert_eq!(rec.stranded, 0, "abrupt {abrupt}");
    }
}

#[test]
fn sim_push_slow_parent() {
    // The busiest relay's uplink drops to half the stream rate at 6 s.
    let r = Sim::new(Scenario::SlowParent.config(PushMode::Hybrid, 1)).run();
    eprintln!("{r:?}");
    check_delivery(&r);
}

#[test]
fn sim_push_withholding_parent() {
    // Two parents accept every subscription and push nothing: the
    // deadlines hand their chunks to pull and the silence prunes them.
    let mut cfg = Scenario::Withholding.config(PushMode::Hybrid, 4);
    cfg.seed = 6;
    let mut sim = Sim::new(cfg);
    let r = sim.run();
    eprintln!(
        "{r:?}
withheld opens {}",
        sim.withheld_opens
    );
    check_delivery(&r);
    assert!(sim.withheld_opens > 0, "the adversaries were never chosen");
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
    eprintln!("{r:?}");
    check_delivery(&r);
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
    check_delivery(&r);
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
    eprintln!("{r:?}");
    check_delivery(&r);
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
/// and every reader reads them as zeros and the stream to its end, as with
/// whole munros.
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
        check_delivery(&r);
        check_delivery(&whole);
    }
}

/// The hybrid with munros signed early over 16 streams, so that most
/// munros leave streams without data: nothing names a data-less chunk,
/// and readers read zeros and the stream to its end, as with whole
/// munros.
#[test]
fn sim_early_signed_units_push() {
    let mut cfg = Scenario::FullMesh.config(PushMode::Hybrid, 16);
    cfg.seed = 2;
    cfg.munro_chunks = 8;
    cfg.greedy = Greedy::Nodes(vec![]);
    let (r, on_wire, whole) = early_and_whole(cfg);
    eprintln!("early: {r:?}\n{:?}\nwhole: {whole:?}", r.push);
    assert_eq!(on_wire, 0);
    check_delivery(&r);
    check_delivery(&whole);
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
            // From 10 s after the step.
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
        let (_, r, ok) = node_sim(NodeScenario::SeedStarved, seed, mode);
        eprintln!("starved {mode:?}: {r:?}");
        ok.unwrap();
    }
}

#[test]
fn sim_latency_step_recovers() {
    // The seed's links step from 2–20 ms to 150 ms one way at 20 s. LEDBAT
    // reads the step as queueing until its base delay ages out, so the
    // request window's floor has to carry the stream meanwhile.
    for seed in [1, 2] {
        let (_, r, ok) = node_sim(NodeScenario::LatencyStep, seed, PushMode::Off);
        eprintln!("latency step: {r:?}");
        ok.unwrap();
    }
}

#[test]
fn sim_capped_seed_delivers_through_relays() {
    // 16 peers, the seed at 3x the stream rate: the relays carry what the
    // seed can't, and every peer reads the whole stream.
    for (seed, mode) in [
        (1, PushMode::Off),
        (2, PushMode::Off),
        (1, PushMode::EagerHave),
    ] {
        let (_, r, ok) = node_sim(NodeScenario::CappedSeed, seed, mode);
        eprintln!("capped {mode:?}: {r:?}");
        ok.unwrap();
    }
}

// --- cross traffic ----------------------------------------------------------

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

/// Cross traffic's uplinks, in tenths of the stream rate.
const CROSS_UPLINK_TENTHS: u64 = 35;

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
    eprintln!("cross: {r:?}");
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

/// The adaptive seed, with uplink shares by allocation.
const ADAPTIVE_QOS: SeedPolicy = SeedPolicy {
    allocation: SeedAllocation::Adaptive,
    qos: SeedQos::ByAllocation,
};

#[test]
fn sim_wan_global_audience() {
    // Viewers on five continents, on residential links, of a datacenter
    // seed in Frankfurt.
    for mode in [PushMode::Off, PushMode::Hybrid] {
        let mut sim = Sim::new(Scenario::WanGlobal.config(mode, 4));
        let r = sim.run();
        eprintln!(
            "{r:?}
retransmits {}",
            sim.retransmits()
        );
        check_delivery(&r);
    }
}

#[test]
fn sim_wan_residential_seed_relays() {
    // The seed's cable uplink carries fewer than 10 of the 24 viewers'
    // streams, so viewers relay the rest.
    for mode in [PushMode::Off, PushMode::Hybrid] {
        let mut sim = Sim::new(Scenario::WanResidentialSeed.config(mode, 4));
        let r = sim.run();
        eprintln!(
            "{r:?}
retransmits {}",
            sim.retransmits()
        );
        check_delivery(&r);
    }
}

#[test]
fn sim_wan_lossy_mobile_viewers() {
    // Every other viewer on LTE: a frame that loses a packet holds back
    // every frame behind it on its link, and the stream still arrives whole.
    let mut sim = Sim::new(wan_mobile_config(1));
    let r = sim.run();
    eprintln!("{r:?}\nretransmits {}", sim.retransmits());
    check_delivery(&r);
    assert!(sim.retransmits() > 0);
}

#[test]
fn sim_wan_seed_policies_deliver_16_streams() {
    // The cable seed on 16 streams, under the default admission and with
    // whole copies at its nearest viewers and uplink shares by allocation.
    for seed_policy in [SeedPolicy::ADMISSION, ADAPTIVE_QOS] {
        let mut cfg = Scenario::WanResidentialSeed.config(PushMode::Hybrid, 16);
        cfg.seed_policy = seed_policy;
        let mut sim = Sim::new(cfg);
        let r = sim.run();
        eprintln!(
            "{seed_policy:?}: {r:?}
retransmits {}",
            sim.retransmits()
        );
        check_delivery(&r);
    }
}

#[test]
fn sim_wan_adaptive_seed_delivers_past_liars_and_colluders() {
    // Liars among the cable seed's nearest viewers take copies first, and
    // the greedy viewers collude in their reports to the seed and hold or
    // forge their echoes; every honest viewer still reads the whole stream.
    let mut cfg = wan_residential_seed_config(1);
    cfg.push_mode = PushMode::Hybrid;
    cfg.stream_count = 16;
    cfg.seed_policy = ADAPTIVE_QOS;
    cfg.greedy = Greedy::Nodes(vec![
        (4, Greed::Liar),
        (21, Greed::Liar),
        (10, Greed::FreeRider),
    ]);
    cfg.collude = true;
    cfg.echo_greed = true;
    cfg.stream_secs = 30;
    cfg.warm_secs = 5;
    let r = Sim::new(cfg).run();
    eprintln!("{r:?}");
    check_delivery(&r);
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
        Sim::new(cfg).run()
    };
    let (a, b) = (run(4), run(4));
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.frames, b.frames);
    assert_eq!(a.push, b.push);
    assert_ne!(a.trace, run(5).trace);
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
