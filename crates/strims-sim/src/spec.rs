// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Scenario descriptions.
//!
//! A [`Scenario`] is plain data: scenario functions build one from a seed
//! (topology and fault times may depend on it) and hand it to
//! [`crate::run`](fn@crate::run) or [`crate::sweep`](fn@crate::sweep).
//! Times are simulated time since the start of the run.

use std::collections::BTreeSet;
use std::time::Duration;

use strims_ppspp::{DeliveryMode, PushMode, SeedPolicy, SwarmOptions};
use strims_testbed::shape::LiveShape;
use strims_vpn::Routing;
use strims_wan::live::{Encoder, Sign};
use strims_wan::{Access, City, Site};

pub use crate::arrivals::{Arrivals, Departure, Departures};
use crate::skew::ClockSkew;
use crate::stream::FRAME_LEN;
use crate::wan::SctpProfile;

/// The port of [`Scenario::over_webrtc`]'s signalling network.
const SIGNAL_PORT: u16 = 9900;

/// A host's index in [`Scenario::hosts`].
pub type HostIx = usize;

/// A link's transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transport {
    Tcp,
    Ws,
    /// WebRTC, linked by a peer exchange on the network of this index
    /// (`Node::connect_peer`): both hosts need [`HostSpec::webrtc`] and no
    /// listeners, and the exchange a route between them on the network.
    WebRtc(usize),
}

/// What a host is in a run, for the measures that split hosts by it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostClass {
    /// A node that follows the protocol.
    #[default]
    Honest,
    /// A node the scenario adds to carry others' signalling (the
    /// signalling relay): no swarm's.
    Infra,
    /// A node that departs from the protocol, by kind.
    Hostile(&'static str),
}

/// One host.
#[derive(Clone, Debug)]
pub struct HostSpec {
    pub name: String,
    pub class: HostClass,
    /// The upload cap in bytes per second (`NodeConfig::rate_limit`).
    /// Turmoil models no bandwidth, so outside WAN mode this is the host's
    /// only rate limit.
    pub uplink: Option<u64>,
    /// The upload capacity the node knows (`NodeConfig::upload_capacity`);
    /// `None` leaves ppspp to `uplink`, if set, and its measurement.
    pub upload_capacity: Option<u64>,
    /// The live oracle: the node is told the meter's truth
    /// ([`Report::upload_cap_at`](crate::Report::upload_cap_at)) as it
    /// boots and ahead of each of its stats samples
    /// (`Node::set_upload_capacity`, which it takes at once), from the same
    /// inputs the run's report takes it from, in place of
    /// `upload_capacity`. At a sample whose truth is above 0 its meter reads
    /// the truth, or less where the meter's link was busy for at least half
    /// its window (it takes the lower of what it knows and what it
    /// measures); a truth of 0 tells it no capacity, which leaves it none
    /// known, so its meter measures there.
    pub upload_capacity_live: bool,
    pub max_peers: u32,
    /// vpn routing for this host; `None` uses [`Scenario::routing`].
    pub routing: Option<Routing>,
    /// TCP and WebSocket listeners (on by default). A host without them
    /// can only dial, or link through WebRTC.
    pub listen: bool,
    /// A WebRTC UDP socket on [`crate::UDP_PORT`], offered in peer
    /// exchanges (`NodeConfig::webrtc`).
    pub webrtc: bool,
    /// With [`SendBuffer::window`], the host's WebRTC connections are the
    /// window model's data channels, sent as this stack sends, in place of
    /// str0m over UDP: TCP connections to [`crate::UDP_PORT`] that carry
    /// SCTP associations, as a browser's are, behind the node's endpoint
    /// proxy (`strims_node`'s `model_rtc`). `None`, or without the window
    /// model: str0m.
    pub rtc_stack: Option<crate::SctpProfile>,
    /// Where the host sits on the WAN ([`Scenario::wan`]); every host has
    /// one in WAN mode.
    pub site: Option<Site>,
    /// WAN mode: the upload available on the site's uplink from each time
    /// on, in bytes per second, as when other traffic takes part of it for
    /// a while (`strims_wan::Network::set_up_steps`). Empty keeps the
    /// site's rate throughout.
    pub up_steps: Vec<(Duration, u64)>,
    /// WAN mode: the site's uplink sends in this schedule's delivery
    /// opportunities in place of its rate and bucket
    /// (`strims_wan::Network::set_up_capacity`), as a cellular or satellite
    /// uplink grants them; with [`HostSpec::up_steps`] too, the run fails,
    /// as the testbed refuses a trace link's steps.
    pub up_capacity: Option<std::sync::Arc<strims_wan::capacity::Capacity>>,
    /// WAN mode with [`SendBuffer::window`]: bulk uploads by other
    /// software on the host, which share its uplink with the node's
    /// traffic (see the WAN scheduler).
    pub competing: Vec<Competing>,
    /// The testbed harness's join (`--join-at`): from this time in the run,
    /// or the host's arrival if later ([`HostSpec::arrive_at`]), the host
    /// dials its links to its swarms' hosts, all at once in link order, as
    /// the harness makes a node's links at its join (strims-testbed's
    /// topology module); its links to hosts no swarm has (a signalling
    /// relay) from its boot. Its node boots with the run (or at its
    /// restart), ahead of its arrival, as the harness starts every node
    /// before the stream, so a relay links it to its network before its
    /// join; it joins its swarms at its arrival. A link still down after
    /// its dial is redialed with the rest ([`Scenario::redial_every`]).
    /// `None`: the host boots at its arrival and dials every link then.
    pub links_at: Option<Duration>,
    /// When the host arrives: its node starts (unless
    /// [`HostSpec::links_at`] booted it already), joins its swarms and
    /// dials its links then ([`Scenario::arrivals`] sets it for the first
    /// swarm's viewers). A viewer that arrives joins at the live edge, as
    /// a player does: its swarms read from the newest chunk its peers have
    /// (`DeliveryMode::LOW_LATENCY`), and a media swarm's reader moves back
    /// to its GOP's start within [`ARRIVAL_REWIND`], as egress joins, where
    /// reading from the stream's start would put the backlog of a live
    /// window into its lag. A viewer that restarts ([`Fault::Restart`])
    /// rejoins the same way. `None`: with the run, reading as the swarm's
    /// options say until it restarts.
    pub arrive_at: Option<Duration>,
    /// The testbed access link spec its site is shaped as
    /// (`strims_wan::testbed::Spec`, a twin's link knobs), which the
    /// tables' `hosts` name; `None` for a site of the WAN model's own.
    pub link_spec: Option<String>,
}

/// How long the join of a media viewer that arrives or restarts waits for
/// its rewind to its GOP's start ([`HostSpec::arrive_at`]):
/// `strims-video`'s default `rewind_timeout`.
pub const ARRIVAL_REWIND: Duration = Duration::from_secs(10);

/// A bulk upload that isn't the node's: a TCP connection that always has
/// data, to a sink `rtt` away. Its client starts at `start` and sends
/// data after iperf3's setup ([`COMPETE_SETUP_TRIPS`] round trips through
/// the uplink), for `stop - start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Competing {
    pub start: Duration,
    pub stop: Duration,
    pub rtt: Duration,
}

/// The round trips an iperf3 client takes from its start to its first
/// data: its parameters and the server's CREATE_STREAMS on the control
/// stream, its data stream's handshake, and its cookie on that stream,
/// which the server answers with TEST_START and TEST_RUNNING. Each takes
/// the path's round trip and the uplink's queue as it begins.
pub const COMPETE_SETUP_TRIPS: u32 = 3;

/// A link the `from` host dials (and redials while it is down).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkSpec {
    pub from: HostIx,
    pub to: HostIx,
    pub transport: Transport,
}

/// How network members bind their peers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// `NetworkOptions::auto_bind_port`: every vnic peer, members or not.
    Auto,
    /// `Node::bind_peer_network` on `PeerAdded`, only between members.
    Explicit,
}

/// A vpn network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkSpec {
    pub members: Vec<HostIx>,
    pub binding: Binding,
    /// The vnic port pair both ends bind.
    pub port: u16,
}

/// A live stream: one seed writes self-checking frames at a fixed rate,
/// or a live encoder's access units signed early ([`MediaSpec`]); viewers
/// read and verify them, relays join without reading.
#[derive(Clone, Debug)]
pub struct SwarmSpec {
    pub seed: HostIx,
    pub viewers: Vec<HostIx>,
    pub relays: Vec<HostIx>,
    pub options: SwarmOptions,
    /// The seed writes `frames_per_write` frames every `write_interval`.
    pub write_interval: Duration,
    pub frames_per_write: usize,
    /// First and last write times.
    pub start: Duration,
    pub stop: Duration,
    /// In place of the frames, the seed writes this live stream.
    pub media: Option<MediaSpec>,
    /// A media swarm's viewers join live and at a GOP, as egress joins:
    /// a reader starts at the newest chunk it has, and after its first read
    /// moves back to the start of the GOP that read falls in, through the
    /// swarm's backlog, within this deadline; one that doesn't land in time
    /// is dropped and reading goes on where it was. `None`: readers read on
    /// from where they start ([`SwarmSpec::with_join_rewind`]), but for a
    /// viewer's join at the live edge, which rewinds within
    /// [`ARRIVAL_REWIND`] ([`HostSpec::arrive_at`]).
    pub join_rewind: Option<Duration>,
}

/// A live encoder's stream that its seed signs early
/// ([`strims_wan::live`]): each access unit's record as it arrives, a
/// unit signed when it fills or at `sign`'s deadline after its first
/// byte, padded with zeros to its end. Chunks are self-checking
/// ([`crate::stream::MediaCodec`]); a frame of the reports is a chunk that
/// holds media, its time its first byte's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaSpec {
    pub encoder: Encoder,
    pub sign: Sign,
}

impl SwarmSpec {
    /// A live stream of about `rate` bytes per second from `start` to
    /// `stop`: a write every 250 ms of a power-of-two number of 1 KiB
    /// frames (rounded down), 1 KiB chunks, one signature per write (so
    /// each write publishes at once), a 1 MiB window, mandatory delivery.
    pub fn live(
        seed: HostIx,
        viewers: Vec<HostIx>,
        rate: u64,
        start: Duration,
        stop: Duration,
    ) -> SwarmSpec {
        let shape = LiveShape::live(rate, LIVE_WINDOW);
        SwarmSpec {
            seed,
            viewers,
            relays: Vec::new(),
            options: shape.options,
            write_interval: shape.write_interval,
            frames_per_write: shape.frames_per_write,
            start,
            stop,
            media: None,
            join_rewind: None,
        }
    }

    /// The same swarm in `chunk`-byte chunks, its window holding as many
    /// bytes. Its frame writes may then not fill whole chunks: for a swarm
    /// that streams media ([`SwarmSpec::with_media`]).
    pub fn with_chunk_size(mut self, chunk: usize) -> SwarmSpec {
        let window = self.options.live_window * self.options.chunk_size;
        self.options.chunk_size = chunk;
        self.options.live_window = (window / chunk.max(1)).max(1);
        self
    }

    /// The same swarm streaming `encoder`'s access units signed under
    /// `sign`, its chunks per signature the policy's at the encoder's rate.
    pub fn with_media(mut self, encoder: Encoder, sign: Sign) -> SwarmSpec {
        self.options.chunks_per_signature = sign.chunks(encoder.rate(), self.options.chunk_size);
        self.media = Some(MediaSpec { encoder, sign });
        self
    }

    /// The same stream in `chunk`-byte chunks signed `munro` bytes at a
    /// time ([`LiveShape::with_chunks`]).
    pub fn with_chunks(mut self, chunk: usize, munro: usize) -> Result<SwarmSpec, String> {
        let shape = LiveShape {
            options: self.options,
            write_interval: self.write_interval,
            frames_per_write: self.frames_per_write,
        }
        .with_chunks(chunk, munro)?;
        self.options = shape.options;
        Ok(self)
    }

    /// The same media swarm with viewers that join live and rewind to their
    /// GOP's start within `deadline` ([`SwarmSpec::join_rewind`]), in low
    /// latency delivery.
    pub fn with_join_rewind(mut self, deadline: Duration) -> SwarmSpec {
        self.options.delivery_mode = DeliveryMode::LOW_LATENCY;
        self.join_rewind = Some(deadline);
        self
    }

    pub fn with_push_mode(mut self, mode: PushMode) -> SwarmSpec {
        self.options.push_mode = mode;
        self
    }

    /// `n` streams: chunk `i` is pushed in stream `i % n` (Hybrid).
    pub fn with_stream_count(mut self, n: usize) -> SwarmSpec {
        self.options.stream_count = n;
        self
    }

    /// How the seed spends its upload in Hybrid.
    pub fn with_seed_policy(mut self, policy: SeedPolicy) -> SwarmSpec {
        self.options.seed = policy;
        self
    }

    /// A live window of `chunks` chunks (see [`Invariants::for_window`]).
    pub fn with_live_window(mut self, chunks: usize) -> SwarmSpec {
        self.options.live_window = chunks;
        self
    }

    /// The stream rate in bytes per second: a media stream's encoder's.
    pub fn rate(&self) -> f64 {
        if let Some(m) = self.media {
            return m.encoder.rate() as f64;
        }
        (self.frames_per_write * FRAME_LEN) as f64 / self.write_interval.as_secs_f64()
    }

    /// Writes from `start` to `stop`.
    pub fn writes(&self) -> u64 {
        let span = self.stop.saturating_sub(self.start).as_millis();
        let step = self.write_interval.as_millis().max(1);
        u64::try_from(span.div_ceil(step)).unwrap_or(u64::MAX)
    }

    /// Frames written over the whole stream, if the seed never stops.
    pub fn total_frames(&self) -> u64 {
        self.writes() * self.frames_per_write as u64
    }

    /// The frame sequence numbers written in `[from, to)`.
    pub fn frames_between(&self, from: Duration, to: Duration) -> std::ops::Range<u64> {
        let k = |t: Duration| -> u64 {
            let t = t.clamp(self.start, self.stop).saturating_sub(self.start);
            let step = self.write_interval.as_millis().max(1);
            u64::try_from(t.as_millis().div_ceil(step)).unwrap_or(u64::MAX)
        };
        let fpw = self.frames_per_write as u64;
        k(from) * fpw..k(to) * fpw
    }

    /// Whether `h` takes part in the swarm.
    pub fn member(&self, h: HostIx) -> bool {
        self.seed == h || self.viewers.contains(&h) || self.relays.contains(&h)
    }
}

/// vpn traffic the controller generates on one network, between running
/// members chosen by the run's rng.
#[derive(Clone, Debug)]
pub struct VpnProbes {
    pub network: usize,
    pub start: Duration,
    pub stop: Duration,
    /// An encrypted unicast between a random pair.
    pub unicast_every: Option<Duration>,
    /// A hash table get (cache disabled) and a peer index search for a
    /// random host's records.
    pub lookup_every: Option<Duration>,
    /// A peer exchange between a random pair that isn't linked.
    pub exchange_every: Option<Duration>,
}

/// Something that happens to the network or a host at a given time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Drop everything between the two groups (`turmoil::partition`). TCP
    /// streams that lose a segment stall until their read timeout.
    Partition(Vec<HostIx>, Vec<HostIx>),
    Repair(Vec<HostIx>, Vec<HostIx>),
    /// Hold everything between the groups and deliver it on release. In
    /// WAN mode, messages still cross the path and the receiver's downlink
    /// meanwhile, and wait there.
    Hold(Vec<HostIx>, Vec<HostIx>),
    Release(Vec<HostIx>, Vec<HostIx>),
    /// Kill the host's software (`Sim::crash`), after partitioning it from
    /// every other host. The FINs of the sockets the crash drops are lost,
    /// as in a power loss; peers notice by timeouts, or by an RST once the
    /// host answers again.
    ///
    /// A crash before the host's [`HostSpec::arrive_at`], or after it
    /// departed, finds no host, and one while it is down from an earlier
    /// crash finds it stopped: either is skipped.
    Crash(HostIx),
    /// Start the host's software again (`Sim::bounce`): a new node with the
    /// same profile key (so the same URIs) and a new host key. Repairs every
    /// partition between the host and the others first, including those of
    /// a [`Fault::Partition`]. A viewer rejoins its swarms at the live edge,
    /// as one that arrives does ([`HostSpec::arrive_at`]); a host crashed
    /// before it ever ran starts its first incarnation here, and reads as
    /// one there from the run.
    ///
    /// Only a host that is down from a crash restarts: a restart of one
    /// that isn't there, never crashed, or is up again from an earlier
    /// restart is skipped, since bouncing a live host would drop its
    /// sockets without the cut-off a crash makes first, in an order that
    /// isn't reproducible.
    Restart(HostIx),
    /// The host leaves for good ([`Departure`]), and takes part in no
    /// fault after. An abrupt departure is a crash that leaves the host
    /// cut off and stopped, and counts no restart.
    Depart(HostIx, Departure),
    /// Fixed one-way latency between the groups (`Sim::set_link_latency`).
    /// In WAN mode it replaces the paths' base latency, and jitter, loss
    /// and bandwidth still apply.
    Latency(Vec<HostIx>, Vec<HostIx>, Duration),
    /// [`Fault::Latency`] in one direction, from the first group to the
    /// second, as a sender's egress delay (netem's) holds what it sends.
    /// WAN mode only: turmoil's links take one latency both ways, so
    /// without the WAN model it is skipped.
    LatencyFrom(Vec<HostIx>, Vec<HostIx>, Duration),
    /// The host's swarm readers stop reading, or resume.
    PauseReading(HostIx),
    ResumeReading(HostIx),
    /// The seed of swarm `n` starts a new epoch (`SwarmWriter::reset`).
    ResetWriter(usize),
    /// A new upload cap for the host (`Node::set_rate_limit`), applied
    /// within 100 ms.
    Uplink(HostIx, u64),
    /// `from` links to `to` through peer exchange on network `network`
    /// (`Node::connect_peer`), within 100 ms. With WebRTC on both sides and
    /// no listeners, the link is a direct WebRTC link.
    Connect {
        from: HostIx,
        to: HostIx,
        network: usize,
    },
}

/// What the controller checks during a run: the heap and progress at
/// every sample ([`Scenario::sample_every`]), the runner bounds and clean
/// input at every stats sample ([`Scenario::stats_every`]).
/// [`Scenario::new`] turns on the runner bounds and clean input for every
/// scenario. Violations are logged in the trace and listed in
/// [`crate::Report::violations`], each runner invariant once per host.
///
/// The runner checks read cumulative counters at the stats samples, so
/// what a channel counted after its last sample goes unseen (see
/// [`crate::stats`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invariants {
    /// The run's live heap stays at or below this many bytes (needs the
    /// tracking allocator; see [`crate::memory`]).
    pub max_heap: Option<i64>,
    /// Inside these windows, every running honest viewer that started
    /// reading more than 10 s ago, isn't paused and didn't resume in the
    /// last 10 s reads at least one frame between two samples (a hostile
    /// one may be starved: [`HostClass::Hostile`]).
    pub progress: Vec<(Duration, Duration)>,
    /// No peer's data queue holds more than this many chunks (per
    /// priority), and no queue refuses a request or push for lack of room
    /// (`PeerStats::dropped`). ppspp's hard cap is `DATA_QUEUE_CAP`
    /// (8192, eight [`LIVE_WINDOW`]s) a queue; a channel's answers, to any
    /// peer, cover distinct chunks of the window's span, one window at
    /// most, whatever the peer requests.
    pub max_data_queue: Option<usize>,
    /// No channel tracks more than this many request deadlines
    /// (`ChannelSchedulerStats::pending_requests`: requests sent within
    /// the last CTO, answered or not; ppspp stops requesting at one
    /// window, `max_pending_requests`).
    pub max_pending_requests: Option<usize>,
    /// Honest nodes never send bad input: no channel from an honest peer
    /// receives data or chunks that fail verification or messages ppspp
    /// rejects as invalid (`ChannelMetrics::rejected` stays 0), and no vpn
    /// frame is malformed, mismatched, unverifiable or undecryptable. A
    /// hostile peer's input ([`HostClass::Hostile`]) doesn't count: a kind
    /// may send what an honest runner rejects (a grabber's requests).
    pub clean_input: bool,
}

impl Invariants {
    /// The defaults of [`Scenario::new`] for swarms with a live window of
    /// `chunks` chunks: no data queue above one window, ppspp's cap on
    /// request deadlines for that window, clean input.
    pub fn for_window(chunks: usize) -> Invariants {
        Invariants {
            max_data_queue: Some(chunks),
            max_pending_requests: Some(strims_ppspp::scheduler_peer::max_pending_requests(
                chunks as u64,
            )),
            clean_input: true,
            ..Invariants::default()
        }
    }
}

/// The live window of [`SwarmSpec::live`], in chunks.
pub const LIVE_WINDOW: usize = 1 << 10;
/// The default [`Invariants::max_data_queue`]: one live window.
pub const MAX_DATA_QUEUE: usize = LIVE_WINDOW;
/// The default [`Invariants::max_pending_requests`]: ppspp's cap for the
/// live window.
pub fn max_pending_requests() -> usize {
    strims_ppspp::scheduler_peer::max_pending_requests(LIVE_WINDOW as u64)
}

/// Some of a scenario's hosts ([`Scenario::timer_log`],
/// [`Scenario::link_log`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostSet {
    All,
    /// The hosts that seed a swarm.
    Seeds,
    Only(BTreeSet<HostIx>),
}

impl HostSet {
    /// Whether host `h` of `s` is one of them.
    pub fn contains(&self, s: &Scenario, h: HostIx) -> bool {
        match self {
            HostSet::All => h < s.hosts.len(),
            HostSet::Seeds => s.swarms.iter().any(|w| w.seed == h),
            HostSet::Only(hs) => hs.contains(&h),
        }
    }

    /// A knob's set: `all` or `1`, `seed`, or `hosts:` and host indices
    /// split by commas (`hosts:0`, `hosts:0,3`), each written as a number
    /// is (`7`, never `07`); `None` for `off`, `0` or nothing. A bare
    /// number is the knob's on and off, never a host's.
    pub fn parse(v: &str) -> Result<Option<HostSet>, String> {
        let list = match v.trim() {
            "" | "off" | "0" => return Ok(None),
            "all" | "1" => return Ok(Some(HostSet::All)),
            "seed" => return Ok(Some(HostSet::Seeds)),
            x => x.strip_prefix("hosts:").ok_or_else(|| {
                format!("{x}: all or 1, seed, off or 0, or hosts: and host indices")
            })?,
        };
        list.split(',')
            .map(|x| {
                x.parse::<HostIx>()
                    .ok()
                    .filter(|h| h.to_string() == x)
                    .ok_or_else(|| format!("hosts:{list}: {x} is no host index"))
            })
            .collect::<Result<BTreeSet<_>, _>>()
            .map(|hs| Some(HostSet::Only(hs)))
    }

    /// The knob's form of the set ([`HostSet::parse`] reads it back).
    pub fn knob(&self) -> String {
        match self {
            HostSet::All => "all".to_owned(),
            HostSet::Seeds => "seed".to_owned(),
            HostSet::Only(hs) => {
                let list: Vec<String> = hs.iter().map(HostIx::to_string).collect();
                format!("hosts:{}", list.join(","))
            }
        }
    }
}

/// A fault and when it happens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedFault {
    pub at: Duration,
    pub fault: Fault,
}

/// Browser pages' times to make their offers, each in place of their
/// stack's (strims-node's `LinkSetup`) where set: those of a testbed run's
/// pages (strims-sim's `testbed::offer_timing`, from its harness's
/// `rtcTiming` log and its captures), which their host's CPU and load
/// move, a page apart from another: the bpace runs' run 1 read one Firefox's
/// gathering end 15 ms after its connect and the other's 125. Each holds
/// an entry a host of its kind, by its rank among them in host order (a
/// testbed run's nodes' order), or one entry for all of them; empty, the
/// stack's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageTiming {
    /// A Firefox page's gathering end for an offer after its node's
    /// connect (`LinkSetup::offer_ready`).
    pub firefox_gather: Vec<Duration>,
    /// A Firefox offer's leaving for the relay after that end
    /// (`LinkSetup::offer_post`).
    pub firefox_post: Vec<Duration>,
    /// A Chromium page's first setting of an offer after its node's
    /// connect, which follows every offer its page was asked for by then,
    /// at the median (`LinkSetup::set_ready`).
    pub chromium_set: Vec<Duration>,
}

impl PageTiming {
    /// `setup`, a stack's, with the times of its host of rank `rank` among
    /// those of its kind in place of its own (the world checks each list
    /// holds one entry or one a host of its kind: [`PageTiming::lists`]).
    pub fn apply(&self, setup: strims_node::LinkSetup, rank: usize) -> strims_node::LinkSetup {
        use strims_node::LinkSetup;
        let at = |xs: &[Duration], or: Duration| match xs {
            [x] => *x,
            xs => xs.get(rank).copied().unwrap_or(or),
        };
        if setup.name == LinkSetup::FIREFOX.name {
            LinkSetup {
                offer_ready: at(&self.firefox_gather, setup.offer_ready),
                offer_post: at(&self.firefox_post, setup.offer_post),
                ..setup
            }
        } else if setup.name == LinkSetup::CHROMIUM.name {
            LinkSetup {
                set_ready: at(&self.chromium_set, setup.set_ready),
                ..setup
            }
        } else {
            setup
        }
    }

    /// Host `h`'s rank among the hosts of its kind
    /// ([`HostKind::of_host`](crate::late::HostKind::of_host)) in `s`, which
    /// picks its entry ([`PageTiming::apply`]), as a testbed run's nodes'
    /// order picks theirs.
    pub fn rank(s: &Scenario, h: HostIx) -> usize {
        use crate::late::HostKind;
        let kind = HostKind::of_host(s, h);
        (0..h).filter(|&i| HostKind::of_host(s, i) == kind).count()
    }

    /// Each list, its name and the kind of host it holds an entry of.
    pub fn lists(&self) -> [(&'static str, &[Duration], crate::late::HostKind); 3] {
        use crate::late::HostKind;
        [
            ("firefox_gather", &self.firefox_gather, HostKind::Firefox),
            ("firefox_post", &self.firefox_post, HostKind::Firefox),
            ("chromium_set", &self.chromium_set, HostKind::Chromium),
        ]
    }
}

/// A whole scenario.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub hosts: Vec<HostSpec>,
    pub links: Vec<LinkSpec>,
    pub networks: Vec<NetworkSpec>,
    pub swarms: Vec<SwarmSpec>,
    pub faults: Vec<TimedFault>,
    pub probes: Option<VpnProbes>,
    pub invariants: Invariants,
    /// vpn routing of every host without its own (`HostSpec::routing`).
    pub routing: Routing,
    /// turmoil's per-message latency range, outside WAN mode.
    pub latency: (Duration, Duration),
    /// WAN mode: when set, every host has a [`HostSpec::site`], and
    /// messages between hosts cross [`strims_wan::Network`] paths (see
    /// [WAN mode](crate#wan-mode)).
    pub wan: Option<strims_wan::Options>,
    /// How long the run lasts (simulated).
    pub duration: Duration,
    /// How often the controller samples every node's metrics (and memory),
    /// logs them and checks the heap and progress invariants.
    pub sample_every: Duration,
    /// How often the controller folds every node's runner, link and network
    /// stats into the report and checks the runner invariants (`None`: at
    /// `sample_every` only). Stats samples log nothing, but they are
    /// queries that can wake an idle node, so the cadence is part of the
    /// scenario (see [Determinism](crate#determinism)).
    pub stats_every: Option<Duration>,
    /// How often hosts check their links and redial the missing ones.
    pub redial_every: Duration,
    /// turmoil's step.
    pub tick: Duration,
    /// Capture every node's ppspp traffic, decisions included, into
    /// `<host>-<incarnation>.ppcap` files in this directory. Capture
    /// changes no trace. Records' times are true time, whatever the node's
    /// clock offset (see [`skew`](crate::skew)).
    pub capture: Option<std::path::PathBuf>,
    /// WAN mode: TCP send buffers, so a sender's sockets push back on
    /// vnic once its uplink queue is full (see [`SendBuffer`]). `None`
    /// accepts every write at once.
    pub send_buffer: Option<SendBuffer>,
    /// Keep every node's transfers per swarm at every stats sample
    /// ([`crate::HostReport::transfers`]), as `calibration_star` does to
    /// compare with the testbed's nodes. Changes no trace.
    pub transfer_samples: bool,
    /// WAN mode: read the network's uplink counters at every edge of bins
    /// this wide from the first swarm's start ([`Report::wire_bins`]), for
    /// the tables ([`Report::write_tables`]). The run loop reads them
    /// between steps and adds nothing to the schedule, so it changes no
    /// trace.
    ///
    /// [`Report::wire_bins`]: crate::Report::wire_bins
    /// [`Report::write_tables`]: crate::Report::write_tables
    pub wire_bins: Option<Duration>,
    /// With [`Scenario::wire_bins`], every node keeps its ppspp runner's
    /// forecast log for the tables' `forecast` (a row per neighbour per
    /// chunk announced: tens of MB a run). Changes no trace.
    pub forecast_log: bool,
    /// With [`Scenario::wire_bins`], every node keeps its ppspp runner's
    /// request log (`Runner::set_request_log`, with the held log) for the
    /// tables' `arrivals`, `haves` and `asks` (`STRIMS_SIM_SPEC=1`: a row
    /// per neighbour per chunk announced, and per request and rival).
    /// Changes no trace.
    pub request_logs: bool,
    /// WAN mode: the hosts whose every packet offered to their uplink the
    /// WAN scheduler logs as its queue takes or drops it, for the tables'
    /// `packets` (`STRIMS_SIM_PACKET_LOG`, [`WanHostReport::packets`]): a
    /// row per packet, some 1.5 k a second at a busy 20 Mbit/s uplink,
    /// each held to the run's end (every host of star-31-5's 30 s, 1.17 M
    /// rows, took 180 MB more at the peak), so a long or wide cell logs
    /// its seed or a few hosts. None by default. Changes no trace.
    ///
    /// [`WanHostReport::packets`]: crate::report::WanHostReport::packets
    pub packet_log: BTreeSet<HostIx>,
    /// WAN mode: the tables write each node's bins
    /// (`STRIMS_SIM_BIN_NODES=1`), so the WAN scheduler keeps every host's
    /// offers to its uplink for their `offered_*` columns
    /// ([`WanHostReport::offers`]). Changes no trace.
    ///
    /// [`WanHostReport::offers`]: crate::report::WanHostReport::offers
    pub node_bins: bool,
    /// With [`Scenario::wire_bins`], every node's decision log
    /// (`NodeConfig::decision_log`) keeps its parent checks for the
    /// tables' `parent_checks` (`STRIMS_SIM_PARENT_CHECKS=1`). The log
    /// changes no decision, so no trace; a capture takes the tap it would,
    /// so the two don't mix.
    pub parent_checks: bool,
    /// TCP writes in pieces of at most this many bytes, as segments: the
    /// window model ([`SendBuffer::window`]) then fills a window segment by
    /// segment, as real TCP does, where a whole write of up to a GSO
    /// buffer otherwise waits until the window takes all of it. `None`:
    /// [`WINDOW_SEGMENT`] with the window model, and writes whole without
    /// it ([`Scenario::segment`]).
    pub tcp_segment: Option<usize>,
    /// Every node's stores keep their chunks' bytes, rather than each
    /// chunk's frames as [`FrameCodec`](crate::stream::FrameCodec) state:
    /// the same run, with every node holding its window of the stream.
    /// Changes no trace.
    pub stored_chunks: bool,
    /// The network (an index into `networks`) every host publishes its
    /// swarms on and finds their other hosts through ([`Scenario::organic`]):
    /// transfer discovery's peer index searches, then a peer exchange to
    /// each host found. `None`: the scenario's links are the topology.
    pub discovery: Option<usize>,
    /// The browsers' signalling relay ([`Scenario::over_webrtc`]) sits on
    /// the hosts' bridge, unshaped, as the Docker testbed's does
    /// (`hack/testbed/run.sh`'s tb-relay): each host reaches it after its
    /// own egress delay ([`Scenario::egress_delay`]), and it reaches each
    /// at once. Off, it is a host of the WAN.
    pub signal_on_bridge: bool,
    /// Experiment flags (`STRIMS_SIM_EXP`), each one of [`EXP_FLAGS`].
    pub exp: BTreeSet<&'static str>,
    /// How far each host's clock is off true time ([`ClockSkew`]), at most
    /// [`MAX_SKEW`](crate::skew::MAX_SKEW): as user machines' clocks are
    /// ([`ClockSkew::Real`]) by default.
    pub clock_skew: ClockSkew,
    /// Where the run's start falls in its clocks' second: every host's
    /// clock reads [`EPOCH_MS`](crate::EPOCH_MS) plus this (and its skew)
    /// at the run's start, while the frame writers and everything the
    /// harness measures keep the run's own time. A node's grids on Unix
    /// time (its upload meter's 100 ms intervals) then sit at this phase
    /// against the stream's writes, as a deployed stream starts at any ms
    /// of its second. `None` draws it from the run's rng, a whole ms in
    /// `[0, 1000)` uniformly (the crate docs' Determinism says when);
    /// `Some` sets it (a whole ms under a second, which `World::new`
    /// asserts) and draws nothing,
    /// `Some(ZERO)` the run as it was before the phase.
    pub epoch_phase: Option<Duration>,
    /// What each packet of a browser whose stack sends through its host's
    /// network thread takes of it ([`SctpProfile::packet_cost`]), in place
    /// of its stack's, as [`Scenario::over_webrtc`] gives each host its
    /// stack; `None` keeps each stack's. A testbed run's twin takes its
    /// run's (calibration_star's `STRIMS_SIM_CAL_PACKET_COST_US`): the
    /// cost is its host's CPU's.
    pub packet_cost: Option<Duration>,
    /// When a browser's page makes its offers, in place of its stack's
    /// ([`PageTiming`]): a testbed run's twin takes its run's
    /// (calibration_star's `STRIMS_SIM_CAL_FIREFOX_GATHER_MS`,
    /// `STRIMS_SIM_CAL_FIREFOX_POST_MS` and
    /// `STRIMS_SIM_CAL_CHROMIUM_SET_MS`), which its host sets, a page its
    /// node's; the default keeps each stack's.
    pub page_timing: PageTiming,
    /// Whether each browser ([`Scenario::is_browser`]) draws its links'
    /// setups as its stack's do (strims-node's `SetupDraws`): its
    /// metronome's phase, one shared by every host when the clocks are
    /// ([`ClockSkew::Off`], as the testbed's containers share their host's
    /// monotonic clock), else its own, and a stream of its own for its
    /// links' parts; on by default. Off, each part is its stack's fixed one,
    /// the ticks counted from each node's model's start, and nothing is
    /// drawn; a run without browsers draws nothing either way.
    pub link_setup_draws: bool,
    /// Whether each host's node task wakes and its frame writer writes as
    /// late as a real node's of its kind do ([`crate::late`] says which
    /// timers that covers, and how late): on by default; off, they run at
    /// their deadlines and nothing is drawn.
    pub timer_lateness: bool,
    /// Whether a browser host's held channel writers' polls of its pacer
    /// fire as late as its kind's host pacer polls are, with
    /// [`Scenario::timer_lateness`] ([`crate::late`]; `crate::net`): each
    /// writer asleep on a timer of its own, those armed for one deadline as
    /// late as one another: on by default. Off, each polls `HOST_POLL`
    /// from its hold on time, and nothing is drawn.
    pub pacer_lateness: bool,
    /// Whether a browser host's held channel writers whose polls come at
    /// one instant read the host's load in turn, each once the work of the
    /// one before it is done, as the browser's timer callbacks run
    /// (`crate::net`): on by default. Off, they read it as their tasks run,
    /// their writes interleaving.
    pub pacer_turns: bool,
    /// Whether the holds a browser seed's frame writer's pass meets start
    /// as late as the page's work makes them, with
    /// [`Scenario::timer_lateness`] ([`crate::late`]'s hold start;
    /// `crate::net`): such a hold, its write's begin and its first poll's
    /// arming, whole ms past the frame write's due ms, no earlier than the
    /// pass, a draw for each pass: on by default. Off, at the pass, and
    /// nothing is drawn.
    pub pacer_hold_start: bool,
    /// With [`Scenario::pacer_hold_start`], whether every write a browser
    /// seed's frame writer's pass hands a data channel starts late, not
    /// only the held ones ([`crate::late`]'s pass start; `crate::net`):
    /// each such write's begin and check, and a held one's first poll's
    /// arming, whole ms past the frame write's due ms, no earlier than the
    /// pass, a draw for each pass. Off by default: the write path bench
    /// (`write_fit_of`) doesn't fit it better yet (PLAN).
    pub pacer_pass_start: bool,
    /// With [`Scenario::wire_bins`], the hosts that keep their timers as
    /// they fire for the tables' `timers` (`NodeConfig::timer_log`, and
    /// their frame writers' and probes' waits): none by default. Changes
    /// no trace.
    pub timer_log: Option<HostSet>,
    /// Every host of [`Scenario::timer_log`] runs the timer probe through
    /// the first swarm's stream (`strims_testbed::node::probe`, as a
    /// testbed node does with `TIMER_PROBE=1`), its timers as late as
    /// [`Scenario::timer_lateness`] draws them from a stream of its own:
    /// off by default.
    pub timer_probe: bool,
    /// With [`Scenario::wire_bins`], the hosts that keep their links' I/O
    /// as it happens for the tables' `link_writes`, `link_reads` and
    /// `link_samples` (`NodeConfig::link_log`, and their modelled
    /// connections' readings and holds under it): none by default.
    /// Changes no trace.
    pub link_log: Option<HostSet>,
    /// With [`Scenario::wire_bins`], the hosts whose data channels' SCTP
    /// associations log each packet they send or receive and each timer
    /// they fire, for the tables' `sctp_events` (the window model's
    /// associations, [`crate::wan::sctp`]): none by default. Changes no
    /// trace.
    pub sctp_log: Option<HostSet>,
    /// When the first swarm's viewers arrive ([`crate::arrivals`]): with
    /// the run by default ([`Arrivals::T0`]). The run resolves it into
    /// [`HostSpec::arrive_at`] and lays the links out in arrival order.
    pub arrivals: Arrivals,
    /// A share of the first swarm's viewers leaving for good
    /// ([`crate::arrivals`]), resolved by the run into
    /// [`Fault::Depart`]s; `None`: no one leaves.
    pub departures: Option<Departures>,
    /// The dishonest mix (`STRIMS_SIM_HOSTILE=mix`): at its start the run
    /// draws a share of the first swarm's viewers, between the bounds of
    /// `hostile.rs`'s `MIX_SHARE`, from its honest ones but
    /// [`keep_honest`](Self::keep_honest), and makes them hostile hosts of
    /// `MIX_KINDS` in turn from a random first one, from the run's rng
    /// after the departures (see [Determinism](crate#determinism)).
    pub mix: bool,
    /// The viewers the scenario singles out (the slow viewer, the
    /// super-peer), which the dishonest mix never draws.
    pub keep_honest: BTreeSet<HostIx>,
    /// The first swarm's seed's allocation set from outside
    /// (`STRIMS_SIM_SEED_PIN`, [`crate::seed_pin`]): its policy, and a pin
    /// of the viewers it pushes each stream to, which the run picks after
    /// the dishonest mix's draw, from the run's rng where it draws (see
    /// [Determinism](crate#determinism)). `None`: the scenario's own.
    pub seed_pin: Option<crate::seed_pin::SeedPin>,
    /// Every node's parent and decision logs feed the loop analysis
    /// (`STRIMS_SIM_LOOPS=1`, [`crate::loops`], [`Report::loops`]). The
    /// logs change no decision, so no trace; a capture takes the tap the
    /// decision log would, so the two don't mix.
    ///
    /// [`Report::loops`]: crate::Report::loops
    pub loops: bool,
}

/// The experiment flags a scenario takes ([`Scenario::exp`]):
/// - `capN`: every browser ([`Scenario::is_browser`]) keeps N peers at
///   most, and N swarm links in a pool's budget.
/// - `pool-retry`, `pool-evict`, `pool-uplink`, `pool-busy`, `pool-shed`, `pool-copies`, `pool-recip`, `pool-role`, `pool-unblock`, `pool-trial`, `pool-useful`, `pool-probe`, `pool-near`, `pool-find`: an organic
///   host's peer pool (see `pool.rs`).
/// - `index-sample`: peer index searches are answered with a sample of the
///   holders, drawn for each search, not the nearest to the requester
///   (`strims_vpn::IndexAnswers`).
pub const EXP_FLAGS: &[&str] = &[
    "cap6",
    "cap8",
    "cap9",
    "cap10",
    "cap12",
    "cap16",
    "pool-retry",
    "pool-evict",
    "pool-uplink",
    "pool-busy",
    "pool-shed",
    "pool-copies",
    "pool-recip",
    "pool-role",
    "pool-unblock",
    "pool-trial",
    "pool-useful",
    "pool-probe",
    "pool-near",
    "pool-find",
    "index-sample",
];

impl Scenario {
    /// Makes each kind's share of the first swarm's viewers hostile hosts
    /// of that kind (see `hostile.rs`): `round(n share)` of the `n`
    /// viewers each, at most `n` in all, spread evenly over their order
    /// and the kinds interleaved over those places, each place taking the
    /// kind furthest behind its share of the places so far (the earlier
    /// in `kinds` on a tie).
    pub fn hostile(&mut self, kinds: &[(&'static str, f64)]) {
        let Some(viewers) = self.swarms.first().map(|s| s.viewers.clone()) else {
            return;
        };
        let n = viewers.len();
        let counts: Vec<usize> = kinds
            .iter()
            .map(|&(_, share)| ((n as f64 * share).round() as usize).min(n))
            .collect();
        let m = counts.iter().sum::<usize>().min(n);
        let mut taken = vec![0usize; kinds.len()];
        for j in 0..m {
            // Behind by `(j + 1) count / m - taken`, in units of 1 / m.
            let behind = |k: usize| {
                let (c, t) = (counts.get(k).copied(), taken.get(k).copied());
                ((j + 1) * c.unwrap_or(0)).cast_signed() - (t.unwrap_or(0) * m).cast_signed()
            };
            let Some(k) = (0..kinds.len())
                .filter(|&k| taken.get(k) < counts.get(k))
                .min_by_key(|&k| (std::cmp::Reverse(behind(k)), k))
            else {
                break;
            };
            let i = ((j as f64 + 0.5) * n as f64 / m as f64) as usize;
            if let (Some(&v), Some(&(kind, _)), Some(t)) =
                (viewers.get(i), kinds.get(k), taken.get_mut(k))
            {
                self.make_hostile(v, kind, *t);
                *t += 1;
            }
        }
    }

    /// Makes viewer `v` a hostile host of `kind`, the `j`-th of its kind
    /// (from 0): a whitewasher rejoins under a fresh host key from 20 s
    /// into the first swarm, staggered 3 s a whitewasher, every
    /// [`REJOIN_EVERY`](crate::hostile::REJOIN_EVERY).
    pub(crate) fn make_hostile(&mut self, v: HostIx, kind: &'static str, j: usize) {
        if let Some(h) = self.hosts.get_mut(v) {
            h.class = HostClass::Hostile(kind);
        }
        if kind == "whitewasher"
            && let Some((start, stop)) = self.swarms.first().map(|s| (s.start, s.stop))
        {
            let mut t = start + Duration::from_secs(20 + 3 * j as u64);
            while t + Duration::from_secs(1) < stop {
                self.at(t, Fault::Crash(v));
                self.at(t + Duration::from_secs(1), Fault::Restart(v));
                t += crate::hostile::REJOIN_EVERY;
            }
        }
    }

    /// Viewer churn: from 20 s into the first swarm, every `60 / per_min`
    /// seconds the next of its viewers in turn leaves and rejoins `away`
    /// later, at the live edge ([`Fault::Restart`]), until the swarm stops.
    /// It draws nothing from the run's rng.
    pub fn churn(&mut self, per_min: f64, away: Duration) {
        let Some((viewers, start, stop)) = self
            .swarms
            .first()
            .map(|s| (s.viewers.clone(), s.start, s.stop))
        else {
            return;
        };
        if viewers.is_empty() || per_min <= 0.0 {
            return;
        }
        let every = Duration::from_secs_f64(60.0 / per_min);
        let mut t = start + Duration::from_secs(20);
        let mut k = 0;
        while t + away < stop {
            if let Some(&v) = viewers.get(k % viewers.len()) {
                self.at(t, Fault::Crash(v));
                self.at(t + away, Fault::Restart(v));
            }
            t += every;
            k += 1;
        }
    }

    /// The peer budget the experiment flags give browsers.
    pub fn exp_browser_cap(&self) -> Option<u32> {
        self.exp
            .iter()
            .find_map(|f| f.strip_prefix("cap").and_then(|n| n.parse().ok()))
    }
}

/// The window model's segment: two packets, acknowledged as one, as a
/// receiver's delayed acknowledgements (one for every two full segments,
/// Linux's) clock its sender. Against the Docker testbed's bare TCP bench
/// (`hack/testbed/bench.sh`), a seed's bursts to two holders and its small
/// writes to eight idle links on a 5 Mbit/s uplink lost 0.7 times the
/// testbed's packets with these segments and 2.7 times with single
/// packets, and their delays matched at p50 and p90 within about 10 ms;
/// whole writes (up to a GSO buffer) lost a fifth and held the holders'
/// windows still.
pub const WINDOW_SEGMENT: usize = 2 * 1448;

/// The TCP send buffer model of WAN mode. A connection whose node set a
/// `TCP_NOTSENT_LOWAT` mark holds its writes while its bytes not yet in
/// the sender's uplink queue reach the mark.
///
/// Without `window`, a segment enters that queue once the queue holds
/// less than `uplink_queue` of transmission time, and nothing pushes back
/// from one path (a slow receiver, or a congestion window over a long
/// round trip). With `window`, each connection sends within a congestion
/// window, and the access queues hold at most `uplink_queue` of
/// transmission time both ways (see the WAN scheduler).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SendBuffer {
    /// The access network's buffer, in transmission time.
    pub uplink_queue: Duration,
    /// Congestion windows and bounded access queues.
    pub window: bool,
}

impl Scenario {
    /// How often the controller samples every node (`sample_every`, at
    /// least 100 ms) and folds their stats ([`Scenario::stats_every`],
    /// within 100 ms and that).
    pub fn cadences(&self) -> (Duration, Duration) {
        let every = self.sample_every.max(Duration::from_millis(100));
        let stats = self
            .stats_every
            .map_or(every, |d| d.clamp(Duration::from_millis(100), every));
        (every, stats)
    }

    /// The size TCP writes are cut to ([`Scenario::tcp_segment`]).
    pub fn segment(&self) -> Option<usize> {
        self.tcp_segment
            .or(self.window_model().then_some(WINDOW_SEGMENT))
    }

    /// Whether TCP runs under the window model: WAN mode with
    /// [`SendBuffer::window`].
    pub fn window_model(&self) -> bool {
        self.wan.is_some() && self.send_buffer.is_some_and(|b| b.window)
    }

    /// Whether host `h` is a browser: the window model's data channels
    /// on a stack that hides their flight from the host's pacers.
    pub fn is_browser(&self, h: HostIx) -> bool {
        self.window_model()
            && self
                .hosts
                .get(h)
                .and_then(|s| s.rtc_stack)
                .is_some_and(|p| !p.buffered_flight)
    }

    /// An empty scenario with 2–20 ms latency, 60 s long.
    pub fn new(name: &str) -> Scenario {
        Scenario {
            name: name.to_owned(),
            hosts: Vec::new(),
            links: Vec::new(),
            networks: Vec::new(),
            swarms: Vec::new(),
            faults: Vec::new(),
            probes: None,
            invariants: Invariants {
                max_data_queue: Some(MAX_DATA_QUEUE),
                max_pending_requests: Some(max_pending_requests()),
                clean_input: true,
                ..Invariants::default()
            },
            routing: Routing::default(),
            latency: (Duration::from_millis(2), Duration::from_millis(20)),
            wan: None,
            duration: Duration::from_secs(60),
            sample_every: Duration::from_secs(5),
            stats_every: Some(Duration::from_secs(1)),
            redial_every: Duration::from_secs(1),
            tick: Duration::from_millis(1),
            capture: None,
            send_buffer: None,
            transfer_samples: false,
            wire_bins: None,
            forecast_log: false,
            request_logs: false,
            packet_log: BTreeSet::new(),
            node_bins: false,
            parent_checks: false,
            tcp_segment: None,
            stored_chunks: false,
            discovery: None,
            signal_on_bridge: false,
            exp: BTreeSet::new(),
            clock_skew: ClockSkew::Real,
            epoch_phase: None,
            packet_cost: None,
            page_timing: PageTiming::default(),
            link_setup_draws: true,
            timer_lateness: true,
            pacer_lateness: true,
            pacer_turns: true,
            pacer_hold_start: true,
            pacer_pass_start: false,
            timer_log: None,
            timer_probe: false,
            link_log: None,
            sctp_log: None,
            arrivals: Arrivals::T0,
            departures: None,
            mix: false,
            keep_honest: BTreeSet::new(),
            seed_pin: None,
            loops: false,
        }
    }

    /// Whether viewers arrive after the run's start or leave it
    /// ([`Scenario::arrivals`], [`Scenario::departures`], or a host's own
    /// [`HostSpec::arrive_at`], as `calibration_star`'s join after the
    /// stream's start and a testbed run's `JOINS` set it): the row's
    /// measures then follow each viewer's own presence.
    pub fn profiled(&self) -> bool {
        !self.arrivals.is_t0()
            || self.departures.is_some()
            || self.hosts.iter().any(|h| h.arrive_at.is_some())
    }

    /// Whether `h` takes part in any of the swarms.
    pub fn in_swarms(&self, h: HostIx) -> bool {
        self.swarms.iter().any(|s| s.member(h))
    }

    /// Whether a run of the scenario reaches time `at`: a fault at its
    /// duration or past it is one the run never applies
    /// (Report::skipped_faults), on a testbed run's report as on a sim's.
    pub fn reaches(&self, at: Duration) -> bool {
        at < self.duration
    }

    /// When host `h` leaves for good ([`Fault::Depart`]), and how: its
    /// earliest departure, the first given of those at one time, as the
    /// run applies them (a later one is a second departure, which it
    /// skips).
    pub fn departure(&self, h: HostIx) -> Option<(Duration, Departure)> {
        self.faults
            .iter()
            .filter_map(|f| match f.fault {
                Fault::Depart(x, how) if x == h => Some((f.at, how)),
                _ => None,
            })
            .min_by_key(|&(at, _)| at)
    }

    /// Adds a node and returns its index.
    pub fn host(&mut self, name: &str, uplink: Option<u64>) -> HostIx {
        self.hosts.push(HostSpec {
            name: name.to_owned(),
            class: HostClass::Honest,
            uplink,
            upload_capacity: None,
            upload_capacity_live: false,
            max_peers: 64,
            routing: None,
            listen: true,
            webrtc: false,
            rtc_stack: None,
            site: None,
            up_steps: Vec::new(),
            up_capacity: None,
            competing: Vec::new(),
            links_at: None,
            arrive_at: None,
            link_spec: None,
        });
        self.hosts.len() - 1
    }

    /// Adds a node at `site` and returns its index; turns on WAN mode. The
    /// node caps its upload at 90 % of the site's uplink, so its own
    /// scheduling, not the access link's queue, sets the pace.
    pub fn host_at(&mut self, name: &str, site: Site) -> HostIx {
        let h = self.host(name, Some(site.profile.up / 10 * 9));
        if let Some(hs) = self.hosts.get_mut(h) {
            hs.site = Some(site);
        }
        self.wan.get_or_insert_with(strims_wan::Options::default);
        h
    }

    /// Adds a node named `<prefix><i>` at each of `sites`.
    pub fn hosts_at(&mut self, prefix: &str, sites: &[Site]) -> Vec<HostIx> {
        sites
            .iter()
            .enumerate()
            .map(|(i, &site)| self.host_at(&format!("{prefix}{i}"), site))
            .collect()
    }

    /// Adds `n` nodes named `<prefix><i>`.
    pub fn hosts(&mut self, prefix: &str, n: usize, uplink: Option<u64>) -> Vec<HostIx> {
        (0..n)
            .map(|i| self.host(&format!("{prefix}{i}"), uplink))
            .collect()
    }

    /// Links the first swarm's seed to each of its viewers that has no
    /// scenario link with it, the viewer dialing it over TCP: a seed every
    /// viewer links to, as the seed allocation pins' bounds take one
    /// (`STRIMS_SIM_SEED_LINKS=all`, [`crate::seed_pin`]).
    pub fn link_seed_to_viewers(&mut self) {
        let Some(sw) = self.swarms.first() else {
            return;
        };
        let (seed, viewers) = (sw.seed, sw.viewers.clone());
        for v in viewers {
            let linked = self
                .links
                .iter()
                .any(|l| (l.from, l.to) == (v, seed) || (l.from, l.to) == (seed, v));
            if !linked {
                self.link(v, seed, Transport::Tcp);
            }
        }
    }

    /// Every host as a browser links: each link over WebRTC
    /// ([`Transport::WebRtc`]), its peer exchange through a signalling
    /// relay, a host every other links to over TCP (in WAN mode, in a New
    /// York datacenter) and that no swarm has; every host without
    /// listeners, host `i` on `stacks[i % stacks.len()]`
    /// ([`HostSpec::rtc_stack`]; `None`, or none given, str0m over UDP).
    /// Returns the relay.
    pub fn over_webrtc(&mut self, stacks: &[Option<SctpProfile>]) -> HostIx {
        let (relay, net) = self.signalling();
        if self.signal_on_bridge {
            self.bridge(relay);
        }
        for l in &mut self.links {
            l.transport = Transport::WebRtc(net);
        }
        for h in 0..relay {
            if let Some(spec) = self.hosts.get_mut(h) {
                spec.listen = false;
                spec.webrtc = true;
                spec.rtc_stack = stacks
                    .get(h % stacks.len().max(1))
                    .copied()
                    .flatten()
                    .map(|p| self.packet_cost.map_or(p, |c| p.with_packet_cost(c.into())));
            }
            self.link(h, relay, Transport::Tcp);
        }
        relay
    }

    /// Puts `relay` on the hosts' bridge ([`Scenario::signal_on_bridge`]):
    /// each host before it with an egress delay reaches it after that
    /// delay, and it reaches every one at once.
    fn bridge(&mut self, relay: HostIx) {
        for h in 0..relay {
            if let Some(d) = self.egress_delay(h) {
                self.at(Duration::ZERO, Fault::LatencyFrom(vec![h], vec![relay], d));
            }
        }
        self.at(
            Duration::ZERO,
            Fault::LatencyFrom(vec![relay], (0..relay).collect(), Duration::ZERO),
        );
    }

    /// The delay host `h`'s egress holds what it sends: that of the latency
    /// faults at time 0 that time it, where they all agree.
    pub fn egress_delay(&self, h: HostIx) -> Option<Duration> {
        let mut delays = self
            .faults
            .iter()
            .filter(|f| f.at.is_zero())
            .filter_map(|f| match &f.fault {
                Fault::LatencyFrom(from, _, d) if from.contains(&h) => Some(*d),
                Fault::Latency(a, b, d) if a.contains(&h) || b.contains(&h) => Some(*d),
                _ => None,
            });
        let first = delays.next();
        first.filter(|&d| delays.all(|x| x == d))
    }

    /// Adds the signalling relay: a host every other may link to (in WAN
    /// mode, in a New York datacenter), with room for all of them, that no
    /// swarm has, and the network of every host it carries peer exchanges
    /// on. Returns the relay and the network.
    fn signalling(&mut self) -> (HostIx, usize) {
        let relay = if self.wan.is_some() {
            self.host_at("signal", Site::new(City::NewYork, Access::Datacenter))
        } else {
            self.host("signal", None)
        };
        let net = self.network(NetworkSpec {
            members: (0..self.hosts.len()).collect(),
            binding: Binding::Auto,
            port: SIGNAL_PORT,
        });
        if let Some(r) = self.hosts.get_mut(relay) {
            r.max_peers = u32::try_from(relay).unwrap_or(u32::MAX).max(r.max_peers);
            r.class = HostClass::Infra;
        }
        (relay, net)
    }

    /// The hosts make their swarms' links themselves, as deployed nodes
    /// do: each keeps only its link to the signalling relay (added, over
    /// TCP, where [`Scenario::over_webrtc`] didn't), publishes its swarms
    /// on the relay's network and links to the hosts it finds there
    /// ([`Scenario::discovery`]). The scenario's other links go. Peer
    /// exchanges carry what a host can be reached by: WebRTC sessions, or
    /// its listeners. A host boots at its arrival, as a deployed node
    /// starts when its user opens it: the testbed harness's join
    /// ([`HostSpec::links_at`]), which makes a fixed topology's links, goes
    /// with them.
    pub fn organic(&mut self) {
        let relay = self.discovering();
        self.links.retain(|l| l.to == relay || l.from == relay);
        for h in &mut self.hosts {
            h.links_at = None;
        }
    }

    /// [`Scenario::organic`]'s discovery beside the scenario's links, which
    /// stay: each host also links to the signalling relay, publishes its
    /// swarms on its network and links to the hosts it finds there. Returns
    /// the relay.
    pub fn discovering(&mut self) -> HostIx {
        let signal = self.networks.iter().position(|n| n.port == SIGNAL_PORT);
        let relay = self.hosts.iter().position(|h| h.class == HostClass::Infra);
        let (relay, net) = match (relay, signal) {
            (Some(r), Some(n)) => (r, n),
            _ => {
                let (r, n) = self.signalling();
                for h in 0..r {
                    self.link(h, r, Transport::Tcp);
                }
                (r, n)
            }
        };
        self.discovery = Some(net);
        relay
    }

    pub fn link(&mut self, from: HostIx, to: HostIx, transport: Transport) {
        self.links.push(LinkSpec {
            from,
            to,
            transport,
        });
    }

    pub fn network(&mut self, spec: NetworkSpec) -> usize {
        self.networks.push(spec);
        self.networks.len() - 1
    }

    pub fn swarm(&mut self, spec: SwarmSpec) -> usize {
        self.swarms.push(spec);
        self.swarms.len() - 1
    }

    pub fn at(&mut self, at: Duration, fault: Fault) {
        self.faults.push(TimedFault { at, fault });
    }

    pub fn name_of(&self, h: HostIx) -> &str {
        self.hosts.get(h).map_or("?", |s| s.name.as_str())
    }

    /// Links (in the spec's topology) from `h` to every host, `None` where
    /// there is no path.
    pub fn hop_distances(&self, h: HostIx) -> Vec<Option<usize>> {
        let mut d = vec![None; self.hosts.len()];
        let mut queue = std::collections::VecDeque::new();
        if let Some(x) = d.get_mut(h) {
            *x = Some(0);
            queue.push_back(h);
        }
        while let Some(x) = queue.pop_front() {
            let next = d.get(x).copied().flatten().unwrap_or(0) + 1;
            for n in self.neighbours(x) {
                if let Some(slot) = d.get_mut(n)
                    && slot.is_none()
                {
                    *slot = Some(next);
                    queue.push_back(n);
                }
            }
        }
        d
    }

    /// Hosts with a direct link to `h` in the spec.
    pub fn neighbours(&self, h: HostIx) -> Vec<HostIx> {
        let mut n: Vec<HostIx> = self
            .links
            .iter()
            .filter_map(|l| {
                if l.from == h {
                    Some(l.to)
                } else if l.to == h {
                    Some(l.from)
                } else {
                    None
                }
            })
            .collect();
        n.sort_unstable();
        n.dedup();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host set's knob: its words, `hosts:` and indices as numbers are
    /// written, and never a bare number for a host.
    #[test]
    fn a_host_sets_knob_names_its_hosts_unambiguously() {
        let only = |hs: &[HostIx]| Some(HostSet::Only(hs.iter().copied().collect()));
        assert_eq!(HostSet::parse("1"), Ok(Some(HostSet::All)));
        assert_eq!(HostSet::parse("all"), Ok(Some(HostSet::All)));
        assert_eq!(HostSet::parse("0"), Ok(None));
        assert_eq!(HostSet::parse(" off "), Ok(None));
        assert_eq!(HostSet::parse(""), Ok(None));
        assert_eq!(HostSet::parse("seed"), Ok(Some(HostSet::Seeds)));
        assert_eq!(HostSet::parse("hosts:0"), Ok(only(&[0])));
        assert_eq!(HostSet::parse("hosts:1"), Ok(only(&[1])));
        assert_eq!(HostSet::parse("hosts:3,0"), Ok(only(&[0, 3])));
        for bad in [
            "2", "0,2", "hosts:", "hosts:07", "hosts:1,", "hosts:x", "seeds",
        ] {
            assert!(HostSet::parse(bad).is_err(), "{bad}");
        }
        for set in [HostSet::All, HostSet::Seeds, HostSet::Only([0, 5].into())] {
            assert_eq!(HostSet::parse(&set.knob()), Ok(Some(set.clone())));
        }
    }
}
