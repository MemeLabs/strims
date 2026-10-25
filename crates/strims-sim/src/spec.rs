// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Scenario descriptions.
//!
//! A [`Scenario`] is plain data: scenario functions build one from a seed
//! (topology and fault times may depend on it) and hand it to
//! [`crate::run`](fn@crate::run) or [`crate::sweep`](fn@crate::sweep).
//! Times are simulated time since the start of the run.

use std::time::Duration;

use strims_ppspp::{PushMode, RequestControllerKind, SeedPolicy, SwarmOptions};
use strims_testbed::shape::LiveShape;
use strims_vpn::Routing;
use strims_wan::Site;

use crate::stream::FRAME_LEN;

/// A host's index in [`Scenario::hosts`].
pub type HostIx = usize;

/// A link's transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transport {
    Tcp,
    Ws,
}

/// One host.
#[derive(Clone, Debug)]
pub struct HostSpec {
    pub name: String,
    /// The upload cap in bytes per second (`NodeConfig::rate_limit`).
    /// Turmoil models no bandwidth, so outside WAN mode this is the host's
    /// only rate limit.
    pub uplink: Option<u64>,
    /// The upload capacity the node knows (`NodeConfig::upload_capacity`);
    /// `None` leaves ppspp to `uplink`, if set, and its measurement.
    pub upload_capacity: Option<u64>,
    pub max_peers: u32,
    /// vpn routing for this host; `None` uses [`Scenario::routing`].
    pub routing: Option<Routing>,
    /// TCP and WebSocket listeners (on by default). A host without them
    /// can only dial, or link through WebRTC.
    pub listen: bool,
    /// A WebRTC UDP socket on [`crate::UDP_PORT`], offered in peer
    /// exchanges (`NodeConfig::webrtc`).
    pub webrtc: bool,
    /// Where the host sits on the WAN ([`Scenario::wan`]); every host has
    /// one in WAN mode.
    pub site: Option<Site>,
    /// WAN mode: the upload available on the site's uplink from each time
    /// on, in bytes per second, as when other traffic takes part of it for
    /// a while (`strims_wan::Network::set_up_steps`). Empty keeps the
    /// site's rate throughout.
    pub up_steps: Vec<(Duration, u64)>,
    /// WAN mode with [`SendBuffer::window`]: bulk uploads by other
    /// software on the host, which share its uplink with the node's
    /// traffic (see the WAN scheduler).
    pub competing: Vec<Competing>,
}

/// A bulk upload that isn't the node's: a TCP connection that always has
/// data, from `start` to `stop`, to a sink `rtt` away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Competing {
    pub start: Duration,
    pub stop: Duration,
    pub rtt: Duration,
}

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
/// viewers read and verify them, relays join without reading.
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
        }
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

    pub fn with_push_mode(mut self, mode: PushMode) -> SwarmSpec {
        self.options.push_mode = mode;
        self
    }

    /// The request controller every node runs for this swarm.
    pub fn with_controller(mut self, kind: RequestControllerKind) -> SwarmSpec {
        self.options.request_controller = kind;
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

    /// The stream rate in bytes per second.
    pub fn rate(&self) -> f64 {
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
    Crash(HostIx),
    /// Start the host's software again (`Sim::bounce`): a new node with the
    /// same profile key (so the same URIs) and a new host key. Repairs every
    /// partition between the host and the others first, including those of
    /// a [`Fault::Partition`].
    Restart(HostIx),
    /// Fixed one-way latency between the groups (`Sim::set_link_latency`).
    /// In WAN mode it replaces the paths' base latency, and jitter, loss
    /// and bandwidth still apply.
    Latency(Vec<HostIx>, Vec<HostIx>, Duration),
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
    /// Inside these windows, every running viewer that started reading
    /// more than 10 s ago, isn't paused and didn't resume in the last 10 s
    /// reads at least one frame between two samples.
    pub progress: Vec<(Duration, Duration)>,
    /// No peer's data queue holds more than this many chunks (per
    /// priority), and no queue refuses a request or push for lack of room
    /// (`PeerStats::dropped`). ppspp's hard cap is `DATA_QUEUE_CAP`
    /// (8192, eight [`LIVE_WINDOW`]s); a queue toward an honest peer holds
    /// at most what that peer may have requested, one window.
    pub max_data_queue: Option<usize>,
    /// No channel tracks more than this many request deadlines
    /// (`ChannelSchedulerStats::pending_requests`: requests sent within
    /// the last CTO, answered or not; ppspp stops requesting at one
    /// window, `max_pending_requests`).
    pub max_pending_requests: Option<usize>,
    /// Honest nodes never send bad input, and every node in a scenario is
    /// honest: no channel receives data or chunks that fail verification
    /// or messages ppspp rejects as invalid (`ChannelMetrics::rejected`
    /// stays 0), and no vpn frame is malformed, mismatched, unverifiable or
    /// undecryptable.
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

/// A fault and when it happens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedFault {
    pub at: Duration,
    pub fault: Fault,
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
    /// changes no trace.
    pub capture: Option<std::path::PathBuf>,
    /// WAN mode: TCP send buffers, so a sender's sockets push back on
    /// vnic once its uplink queue is full (see [`SendBuffer`]). `None`
    /// accepts every write at once.
    pub send_buffer: Option<SendBuffer>,
    /// Keep every node's transfers per swarm at every stats sample
    /// ([`crate::HostReport::transfers`]), as `calibration_star` does to
    /// compare with the testbed's nodes. Changes no trace.
    pub transfer_samples: bool,
}

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
        }
    }

    /// Adds a node and returns its index.
    pub fn host(&mut self, name: &str, uplink: Option<u64>) -> HostIx {
        self.hosts.push(HostSpec {
            name: name.to_owned(),
            uplink,
            upload_capacity: None,
            max_peers: 64,
            routing: None,
            listen: true,
            webrtc: false,
            site: None,
            up_steps: Vec::new(),
            competing: Vec::new(),
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
