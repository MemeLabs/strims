// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One run's shared state: keys derived from the seed, the host registry,
//! the trace and every statistic.
//!
//! Every host and the controller run on the turmoil thread, so the state is
//! an `Rc<World>` with a `RefCell` inside. Borrows never cross an `.await`.
//!
//! The state is the harness's record of the run, and it grows with the
//! run's length. Every change to it goes through [`World::record`], which
//! counts its allocations in a group of their own, so the run's memory
//! numbers stay those of its nodes and its network.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use strims_base::rng::{DetRng, det_rng};
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_node::config::{MetricsSink, ParentSink};
use strims_node::event::ParentInfo;
use strims_node::{
    ExchangeOutcome, HostId, NetworkId, Node, NodeConfig, NodeMetrics, PpsppCapture, SwarmKey,
    WebRtcConfig,
};
use strims_proto::types::Key;
use strims_wan::live::Au;

use crate::memory::{GroupCounters, MemoryGroup};
use crate::net::SendBuffers;
use crate::report::{
    ArrivalHostRecord, ForecastRecord, HaveHostRecord, HostReport, MediaLog, NodeBinRecord,
    ParentLoss, ParentRecord, RequestHostRecord, VpnReport,
};
use crate::skew::OffsetCapture;
use crate::spec::{HostIx, Scenario, Transport};
use crate::stats::{HostStats, TransferSample};
use crate::stream::{FrameParser, JoinRewind, MediaParser, Parsed, SeqSet, ViewerStats};
use crate::wan::SharedWan;
use crate::{EPOCH_MS, TCP_PORT, UDP_PORT, WS_PORT};

/// The peer index key every host publishes under (with its name as salt).
pub(crate) const PEER_INDEX_KEY: &[u8] = b"strims-sim peer index";
pub(crate) const HASH_TABLE_SALT: &[u8] = b"strims-sim";

pub(crate) fn epoch() -> Time {
    Time::from_unix_millis(EPOCH_MS)
}

/// The run's first stretch, over which a host's offers to link it count
/// apart ([`HostReport::offers_early`](crate::report::HostReport)): a
/// seed's first wave.
pub const EARLY_OFFERS: Duration = Duration::from_secs(30);

/// Simulated time since the start of the run, from inside a host.
pub(crate) fn sim_now() -> Duration {
    turmoil::sim_elapsed().unwrap_or_default()
}

pub(crate) fn sim_now_ms() -> u64 {
    u64::try_from(sim_now().as_millis()).unwrap_or(u64::MAX)
}

/// Key `i` of kind `tag` (`P`rofile, `S`warm, `N`etwork, `H`ash table):
/// a function of `(tag, seed, i)` alone, so no key depends on how many
/// others a scenario has.
fn derive_key(tag: u8, seed: u64, i: usize) -> Key {
    let mut s = [tag; 32];
    if let Some(d) = s.get_mut(..8) {
        d.copy_from_slice(&seed.to_le_bytes());
    }
    if let Some(d) = s.get_mut(8..16) {
        d.copy_from_slice(&(i as u64).to_le_bytes());
    }
    key_from_seed(&s)
}

/// A host's registry entry.
#[derive(Debug, Default)]
pub(crate) struct HostState {
    /// The running node, `None` while crashed.
    pub node: Option<Node>,
    pub host_id: Option<HostId>,
    pub incarnation: u32,
    pub nets: Vec<Option<NetworkId>>,
    /// Peers bound to each network of `nets`, by index.
    pub net_peers: BTreeMap<usize, u32>,
    /// Peer exchanges that ended, by the other host.
    pub exchanges: BTreeMap<HostIx, u32>,
    /// See [`HostReport::neighbours`].
    pub neighbours: Vec<(Duration, Vec<HostIx>)>,
    /// The node's key for each scenario swarm it takes part in.
    pub swarms: Vec<Option<SwarmKey>>,
    /// Each incarnation's key of the first scenario swarm, as it
    /// registered.
    pub first_swarm: Vec<(u32, Option<SwarmKey>)>,
    /// When each incarnation that stopped without closing its links (a
    /// crash, an abrupt departure) stopped.
    pub stopped: Vec<(u32, Duration)>,
    /// Links (indexes into `Scenario::links`) being dialed.
    pub dialing: BTreeSet<usize>,
    /// The last dial outcome per link, to log changes only.
    pub dial_ok: BTreeMap<usize, bool>,
    pub reading_paused: bool,
    /// When reading last resumed (ms).
    pub resumed_ms: u64,
    /// An upload cap for the host's control task to apply.
    pub uplink: Option<u64>,
    /// Peer exchanges for the host's control task to start: (to, network).
    pub connects: Vec<(HostIx, usize)>,
    /// Counters of finished incarnations.
    pub totals: HostReport,
    /// The current incarnation's last sample.
    pub last: Option<NodeMetrics>,
    /// Runner, link and network stats over every stats sample.
    pub stats: HostStats,
    /// Transfers per stats sample, every incarnation (see
    /// [`HostReport::transfers`]).
    pub transfers: Vec<(Duration, usize, TransferSample)>,
    /// Runner invariants this host already broke (each is logged once).
    pub broken: BTreeSet<&'static str>,
    /// See [`HostReport::node_bins`] and [`HostReport::forecasts`].
    pub node_bins: Vec<NodeBinRecord>,
    pub forecasts: Vec<ForecastRecord>,
    pub forecasts_lost: u64,
    pub held: Vec<(u64, Duration)>,
    /// See [`HostReport::arrivals`], [`HostReport::haves`] and
    /// [`HostReport::requests`].
    pub arrivals: Vec<ArrivalHostRecord>,
    pub held_lost: u64,
    pub haves: Vec<HaveHostRecord>,
    pub haves_lost: u64,
    pub requests: Vec<RequestHostRecord>,
    pub requests_lost: u64,
    /// When it left for good ([`crate::Fault::Depart`]), in ms.
    pub departed: Option<u64>,
    /// Crashed, and not restarted yet.
    pub down: bool,
    /// Its last crash found no host (see [`World::skips_crash`]).
    pub crash_skipped: bool,
    /// With [`Scenario::departures`], the hosts its first swarm's streams
    /// had as push parents at its last stats sample, one per stream.
    pub parents: Vec<HostIx>,
}

impl HostState {
    /// Totals plus the current incarnation's last sample.
    pub fn report(&self) -> HostReport {
        let mut r = self.totals.clone();
        if let Some(m) = &self.last {
            add_metrics(&mut r, m);
            r.links = m.links.len();
        }
        r.runner = self.stats.report();
        r.transfers = self.transfers.clone();
        r.neighbours = self.neighbours.clone();
        r.node_bins = self.node_bins.clone();
        r.forecasts = self.forecasts.clone();
        r.forecasts_lost = self.forecasts_lost;
        r.held = self.held.clone();
        r.arrivals = self.arrivals.clone();
        r.held_lost = self.held_lost;
        r.haves = self.haves.clone();
        r.haves_lost = self.haves_lost;
        r.requests = self.requests.clone();
        r.requests_lost = self.requests_lost;
        r
    }

    /// The incarnation ended: fold its last sample into the totals.
    pub fn retire(&mut self) {
        if let Some(m) = self.last.take() {
            add_metrics(&mut self.totals, &m);
        }
        self.stats.retire();
        self.node = None;
        self.host_id = None;
        self.nets.clear();
        self.swarms.clear();
        self.dialing.clear();
    }
}

fn add_metrics(r: &mut HostReport, m: &NodeMetrics) {
    r.uploaded += m.driver.bytes_written;
    r.downloaded += m.driver.bytes_read;
    r.ppspp.merge(&m.runner.totals);
    r.frames_written += m.host.frame_write_count;
    r.frame_bytes += m.host.frame_write_bytes;
    r.control_bytes += m.host.keepalive_write_bytes;
    r.transfer_errors += m.driver.transfer_errors;
    r.vpn_frame_errors += m.driver.vpn_frame_errors;
    r.events_dropped += m.driver.events_dropped;
    r.accept_errors += m.driver.accept_errors;
    r.loop_iterations += m.driver.loop_iterations;
    r.vpn_messages_relayed += m.vpn.messages_relayed;
    r.vpn_link_queue_full += m.vpn.link_queue_full;
    r.vpn_records_pushed += m.vpn.records_pushed;
    r.vpn_bad_input +=
        m.vpn.malformed + m.vpn.trailer_mismatch + m.vpn.verify_failures + m.vpn.decrypt_failures;
}

/// A unicast probe.
#[derive(Debug)]
pub(crate) struct Probe {
    pub sent_ms: u64,
    pub src: HostIx,
    pub dst: HostIx,
    pub delivered: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct WorldState {
    pub trace: Vec<String>,
    pub hosts: Vec<HostState>,
    /// Every host id any incarnation had.
    pub ids: BTreeMap<HostId, HostIx>,
    pub viewers: BTreeMap<(usize, HostIx), ViewerStats>,
    /// Frames each swarm's seed wrote.
    pub written: Vec<SeqSet>,
    /// What each media swarm's seed wrote.
    pub media: Vec<Option<MediaLog>>,
    /// Swarms whose seed should reset its writer.
    pub reset_writer: BTreeSet<usize>,
    pub vpn: VpnReport,
    pub probes: BTreeMap<u64, Probe>,
    /// Finished lookups: (start ms, a search (else a get), found).
    pub lookups: Vec<(u64, bool, bool)>,
    pub memory_samples: Vec<(u64, i64)>,
    /// `(ms, what)` for every invariant violation.
    pub violations: Vec<(u64, String)>,
    /// Frames per viewer at the previous sample, for the progress check.
    pub progress: BTreeMap<(usize, HostIx), u64>,
    /// The controller's choices (probe pairs).
    pub rng: DetRng,
    /// The uplink counters at each bin edge ([`crate::Report::wire_bins`]).
    pub wire_bins: Vec<Vec<[u64; crate::wan::UP_COUNTERS.len()]>>,
    /// See [`crate::Report::parent_losses`].
    pub parent_losses: Vec<ParentLoss>,
    /// With the tables, every node's parent changes as their sinks gave
    /// them ([`World::parent_changes`]): the host, its incarnation, the
    /// change.
    pub parents: Vec<(HostIx, u32, ParentInfo)>,
}

/// One run.
#[derive(Debug)]
pub(crate) struct World {
    pub seed: u64,
    pub spec: Scenario,
    pub profile_keys: Vec<Key>,
    pub swarm_keys: Vec<Key>,
    pub network_keys: Vec<Key>,
    /// The run's heap counters: its nodes and network, without the
    /// harness's records.
    pub memory: Option<GroupCounters>,
    /// The allocation group of the harness's records.
    records: RefCell<Option<MemoryGroup>>,
    /// WAN mode's TCP send buffers ([`Scenario::send_buffer`]).
    pub send_buffers: Option<SendBuffers>,
    /// The WAN schedule, once the run made it.
    pub wan: SharedWan,
    /// Each media swarm's access units, by arrival from its stream's
    /// start.
    pub media_traces: Vec<Option<Vec<Au>>>,
    /// How far each host's clock reads ahead of true time
    /// ([`Scenario::clock_skew`]); empty without skew.
    pub clock_offsets: Vec<strims_base::time::Duration>,
    /// With the tables, the nodes' parent sinks' channel
    /// ([`NodeConfig::parent_log`]): they send from inside the nodes'
    /// passes, and the records take what came at each stats sample, so it
    /// holds a sample's at most.
    parent_tx: mpsc::Sender<(HostIx, u32, ParentInfo)>,
    parent_rx: mpsc::Receiver<(HostIx, u32, ParentInfo)>,
    /// With the tables, the nodes' last snapshots' channel
    /// ([`NodeConfig::last_metrics`]): a node task sends its own as it
    /// goes, which [`World::observe_stopped`] takes.
    stopped_tx: mpsc::Sender<(HostIx, NodeMetrics)>,
    stopped_rx: mpsc::Receiver<(HostIx, NodeMetrics)>,
    pub st: RefCell<WorldState>,
}

impl World {
    pub fn new(
        mut spec: Scenario,
        seed: u64,
        memory: Option<GroupCounters>,
        records: Option<MemoryGroup>,
    ) -> World {
        let profile_keys = (0..spec.hosts.len())
            .map(|i| derive_key(b'P', seed, i))
            .collect();
        let swarm_keys = (0..spec.swarms.len())
            .map(|i| derive_key(b'S', seed, i))
            .collect();
        let network_keys = (0..spec.networks.len())
            .map(|i| derive_key(b'N', seed, i))
            .collect();
        // The run's rng draws the media traces first, before any choice.
        let mut rng = det_rng(seed ^ 0xc0_47_01);
        let media_traces: Vec<Option<Vec<Au>>> = spec
            .swarms
            .iter()
            .map(|s| {
                let span = s.stop.saturating_sub(s.start).as_millis();
                let span =
                    strims_base::time::Duration::from_millis(i64::try_from(span).unwrap_or(0));
                s.media.map(|m| m.encoder.trace(&mut rng, span))
            })
            .collect();
        let clock_offsets = crate::skew::offsets(spec.clock_skew, spec.hosts.len(), &mut rng);
        crate::arrivals::resolve(&mut spec, &mut rng);
        let st = WorldState {
            trace: Vec::new(),
            hosts: (0..spec.hosts.len())
                .map(|_| HostState::default())
                .collect(),
            ids: BTreeMap::new(),
            viewers: BTreeMap::new(),
            written: vec![SeqSet::default(); spec.swarms.len()],
            media: spec
                .swarms
                .iter()
                .map(|s| s.media.map(|_| MediaLog::default()))
                .collect(),
            reset_writer: BTreeSet::new(),
            vpn: VpnReport::default(),
            probes: BTreeMap::new(),
            lookups: Vec::new(),
            memory_samples: Vec::new(),
            violations: Vec::new(),
            progress: BTreeMap::new(),
            rng,
            wire_bins: Vec::new(),
            parent_losses: Vec::new(),
            parents: Vec::new(),
        };
        let send_buffers = spec.wan.and(spec.send_buffer).map(SendBuffers::new);
        let (parent_tx, parent_rx) = mpsc::channel();
        let (stopped_tx, stopped_rx) = mpsc::channel();
        World {
            seed,
            spec,
            profile_keys,
            swarm_keys,
            network_keys,
            memory,
            records: RefCell::new(records),
            send_buffers,
            wan: SharedWan::default(),
            media_traces,
            clock_offsets,
            parent_tx,
            parent_rx,
            stopped_tx,
            stopped_rx,
            st: RefCell::new(st),
        }
    }

    /// Runs `f` on the run's records with the records' allocation group
    /// entered. An allocation counts in the group current when it's made,
    /// a reallocation included, so a record grows only in here.
    fn record<R>(&self, f: impl FnOnce(&mut WorldState) -> R) -> R {
        let mut group = self.records.borrow_mut();
        let _counted = group.as_mut().map(MemoryGroup::enter);
        f(&mut self.st.borrow_mut())
    }

    /// Records the uplink counters at a bin edge
    /// ([`crate::Report::wire_bins`]), in the records' group.
    pub(crate) fn record_wire_bin(&self, counters: Vec<[u64; crate::wan::UP_COUNTERS.len()]>) {
        self.record(|st| st.wire_bins.push(counters));
    }

    pub fn name(&self, h: HostIx) -> &str {
        self.spec.name_of(h)
    }

    /// A peer's name, or `?` for an unknown host id.
    pub fn label(&self, h: Option<HostIx>) -> &str {
        h.map_or("?", |h| self.name(h))
    }

    // --- clocks -----------------------------------------------------------

    /// How far host `h`'s clock reads ahead of true time.
    pub fn clock_offset(&self, h: HostIx) -> strims_base::time::Duration {
        self.clock_offsets.get(h).copied().unwrap_or_default()
    }

    /// The time host `h`'s clock reads at the run's start: the run's epoch
    /// plus its offset.
    pub fn clock_epoch(&self, h: HostIx) -> Time {
        epoch() + self.clock_offset(h)
    }

    /// The time host `h`'s clock reads `t` into the run.
    pub fn clock_at(&self, h: HostIx, t: Duration) -> Time {
        self.clock_epoch(h) + strims_base::time::Duration::from(t)
    }

    // --- trace ----------------------------------------------------------

    /// Appends a trace line stamped with the current simulated time.
    pub fn log(&self, who: &str, line: impl AsRef<str>) {
        self.log_at(sim_now_ms(), who, line);
    }

    pub fn log_at(&self, ms: u64, who: &str, line: impl AsRef<str>) {
        self.record(|st| st.trace.push(format!("{ms} {who} {}", line.as_ref())));
    }

    // --- keys and addresses ----------------------------------------------

    pub fn profile_key(&self, h: HostIx) -> Option<&Key> {
        self.profile_keys.get(h)
    }

    /// The key a host publishes its hash table record under.
    pub fn hash_table_key(&self, h: HostIx) -> Key {
        derive_key(b'H', self.seed, h)
    }

    pub fn hash_table_value(&self, h: HostIx) -> Vec<u8> {
        format!("record of {}", self.name(h)).into_bytes()
    }

    pub fn peer_index_salt(&self, h: HostIx) -> Vec<u8> {
        self.name(h).as_bytes().to_vec()
    }

    pub fn uri(&self, h: HostIx, t: Transport) -> Option<String> {
        let ip = turmoil::lookup(self.name(h));
        let key = hex::encode(&self.profile_key(h)?.public);
        Some(match t {
            Transport::Tcp => format!("tcp://{ip}:{TCP_PORT}/{key}"),
            Transport::Ws => format!("ws://{ip}:{WS_PORT}/{key}"),
            Transport::WebRtc(_) => return None,
        })
    }

    /// The node configuration of host `h`'s `incarnation`. The profile key
    /// (and so the listener URIs) is the same in every incarnation; the rng
    /// seed, and with it the host key, is distinct per host and
    /// incarnation, so peers see a restarted host as a new host.
    pub fn node_config(&self, h: HostIx, incarnation: u32) -> Option<NodeConfig> {
        let spec = self.spec.hosts.get(h)?;
        let rng_seed = self
            .seed
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add((h as u64) << 20)
            .wrapping_add(u64::from(incarnation));
        let mut c = NodeConfig::new(self.profile_key(h)?.clone(), rng_seed);
        c.label = spec.name.clone();
        if spec.listen {
            c.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
            c.ws_listen = Some(format!("0.0.0.0:{WS_PORT}"));
        }
        if spec.webrtc {
            // turmoil binds only the wildcard; host candidates advertise
            // `public_ip`.
            c.webrtc = WebRtcConfig::local([format!("0.0.0.0:{UDP_PORT}")]);
        }
        c.public_ip = Some(turmoil::lookup(spec.name.as_str()));
        c.allow_insecure_ws = true;
        c.max_peers = spec.max_peers;
        c.rate_limit = spec.uplink;
        // A browser's channels hide their flight from its pacers.
        let browser = self.spec.is_browser(h);
        c.pace_egress = browser;
        if browser && let Some(cap) = self.spec.exp_browser_cap() {
            c.max_peers = c.max_peers.min(cap);
        }
        if let crate::HostClass::Hostile(kind) = spec.class
            && let Some(s) = self.spec.swarms.first()
        {
            c.hostile_egress = crate::hostile::egress(kind, &s.options);
        }
        if self.spec.exp.contains("index-sample") {
            c.vpn.peer_index_answers = strims_vpn::IndexAnswers::Sample;
        }
        c.upload_capacity = spec.upload_capacity_at(sim_now());
        c.vpn.routing = spec.routing.unwrap_or(self.spec.routing);
        if let (Some(width), Some(s)) = (self.spec.wire_bins, self.spec.swarms.first()) {
            c.stats_bins = Some(strims_node::config::StatsBins {
                origin: self.clock_at(h, s.start),
                width: width.max(Duration::from_millis(1)).into(),
            });
            c.forecast_log = self.spec.forecast_log;
            c.request_log = self.spec.request_logs;
            let tx = self.parent_tx.clone();
            c.parent_log = Some(ParentSink(Arc::new(move |p| {
                let _ = tx.send((h, incarnation, p));
            })));
            let tx = self.stopped_tx.clone();
            c.last_metrics = Some(MetricsSink(Arc::new(move |m| {
                let _ = tx.send((h, m));
            })));
        }
        if let Some(dir) = &self.spec.capture {
            let path = dir.join(format!("{}-{incarnation}.ppcap", spec.name));
            let offset = self.clock_offset(h);
            c.ppspp_capture = Some(PpsppCapture {
                open: Arc::new(move |_, _| {
                    std::fs::create_dir_all(path.parent().unwrap_or(&path))?;
                    let f = std::io::BufWriter::new(std::fs::File::create(&path)?);
                    if offset == strims_base::time::Duration::ZERO {
                        return Ok(Box::new(f));
                    }
                    Ok(Box::new(OffsetCapture::new(f, offset)))
                }),
                max_file_bytes: 0,
                tap: true,
            });
        }
        Some(c)
    }

    // --- registry ---------------------------------------------------------

    /// A new incarnation of `h` starts; returns its number (1 for the
    /// first).
    pub fn begin_incarnation(&self, h: HostIx) -> u32 {
        self.record(|st| {
            let Some(hs) = st.hosts.get_mut(h) else {
                return 0;
            };
            hs.incarnation += 1;
            hs.incarnation
        })
    }

    pub fn register(
        &self,
        h: HostIx,
        node: &Node,
        nets: Vec<Option<NetworkId>>,
        swarms: Vec<Option<SwarmKey>>,
    ) {
        let id = node.host_id();
        let inc = self.record(|st| {
            st.ids.insert(id, h);
            let hs = st.hosts.get_mut(h)?;
            hs.node = Some(node.clone());
            hs.host_id = Some(id);
            hs.nets = nets;
            hs.net_peers.clear();
            hs.first_swarm
                .push((hs.incarnation, swarms.first().copied().flatten()));
            hs.swarms = swarms;
            Some(hs.incarnation)
        });
        if let Some(inc) = inc {
            self.log(self.name(h), format!("up incarnation {inc}"));
        }
    }

    /// Registers a host id as soon as the node exists.
    pub fn register_id(&self, h: HostIx, id: HostId) {
        self.record(|st| st.ids.insert(id, h));
    }

    pub fn ix_of(&self, id: &HostId) -> Option<HostIx> {
        self.st.borrow().ids.get(id).copied()
    }

    pub fn node(&self, h: HostIx) -> Option<Node> {
        self.st.borrow().hosts.get(h).and_then(|s| s.node.clone())
    }

    pub fn host_id(&self, h: HostIx) -> Option<HostId> {
        self.st.borrow().hosts.get(h).and_then(|s| s.host_id)
    }

    /// A peer of `h` was bound to network `n`, or unbound.
    pub fn network_peer(&self, h: HostIx, n: usize, added: bool) {
        self.record(|st| {
            if let Some(hs) = st.hosts.get_mut(h) {
                let c = hs.net_peers.entry(n).or_default();
                *c = if added {
                    c.saturating_add(1)
                } else {
                    c.saturating_sub(1)
                };
            }
        });
    }

    /// Whether `h` has a peer bound to network `n`, and so a route to the
    /// rest of it.
    pub fn on_network(&self, h: HostIx, n: usize) -> bool {
        self.st
            .borrow()
            .hosts
            .get(h)
            .is_some_and(|s| s.net_peers.get(&n).is_some_and(|&c| c > 0))
    }

    pub fn net(&self, h: HostIx, n: usize) -> Option<NetworkId> {
        self.st
            .borrow()
            .hosts
            .get(h)
            .and_then(|s| s.nets.get(n).copied().flatten())
    }

    // --- dials ------------------------------------------------------------

    /// Marks link `l` of `h` as being dialed; false if it already is.
    pub fn start_dial(&self, h: HostIx, l: usize) -> bool {
        self.record(|st| st.hosts.get_mut(h).is_some_and(|s| s.dialing.insert(l)))
    }

    /// A dial finished; logs it when the link's outcome changed.
    pub fn end_dial(&self, h: HostIx, l: usize, result: Result<(), String>) {
        let ok = result.is_ok();
        let changed = self.record(|st| {
            st.hosts.get_mut(h).is_some_and(|s| {
                s.dialing.remove(&l);
                s.dial_ok.insert(l, ok) != Some(ok)
            })
        });
        if changed {
            let to = self.spec.links.get(l).map_or("?", |l| self.name(l.to));
            match result {
                Ok(()) => self.log(self.name(h), format!("dial {to} ok")),
                Err(e) => self.log(self.name(h), format!("dial {to} failed: {e}")),
            }
        }
    }

    // --- faults -------------------------------------------------------------

    /// `h` crashed at `ms`.
    pub fn crashed(&self, h: HostIx, ms: u64) {
        self.record(|st| {
            if let Some(s) = st.hosts.get_mut(h) {
                s.stopped.push((s.incarnation, Duration::from_millis(ms)));
                s.retire();
                s.totals.restarts += 1;
                s.down = true;
            }
        });
    }

    /// `h` restarted.
    pub fn restarted(&self, h: HostIx) {
        self.record(|st| {
            if let Some(s) = st.hosts.get_mut(h) {
                s.down = false;
            }
        });
    }

    /// Whether a crash of `h` at `now` finds no host: before its
    /// [`HostSpec::arrive_at`](crate::HostSpec::arrive_at), or after it
    /// departed. The restart that follows a skipped crash is skipped too
    /// ([`World::skips_restart`]).
    pub fn skips_crash(&self, h: HostIx, now: Duration) -> bool {
        let early = self
            .spec
            .hosts
            .get(h)
            .and_then(|s| s.arrive_at)
            .is_some_and(|at| now < at);
        let skip = early || self.departed(h);
        if skip
            || self
                .st
                .borrow()
                .hosts
                .get(h)
                .is_some_and(|s| s.crash_skipped)
        {
            self.record(|st| {
                if let Some(s) = st.hosts.get_mut(h) {
                    s.crash_skipped = skip;
                }
            });
        }
        skip
    }

    /// Whether a restart of `h` is skipped: it departed, or its crash was.
    pub fn skips_restart(&self, h: HostIx) -> bool {
        self.departed(h)
            || self
                .st
                .borrow()
                .hosts
                .get(h)
                .is_some_and(|s| s.crash_skipped)
    }

    /// Whether `h` is down from a crash it hasn't restarted from.
    pub fn is_down(&self, h: HostIx) -> bool {
        self.st.borrow().hosts.get(h).is_some_and(|s| s.down)
    }

    /// Whether `h` left for good ([`crate::Fault::Depart`]).
    pub fn departed(&self, h: HostIx) -> bool {
        self.st
            .borrow()
            .hosts
            .get(h)
            .is_some_and(|s| s.departed.is_some())
    }

    /// `h` leaves for good at `ms`; `stopped` when it stops with it (an
    /// abrupt departure), which retires its incarnation without counting
    /// a restart. Every viewer whose last stats sample had `h` as a push
    /// parent loses it ([`World::parent_gone`]).
    pub fn depart(&self, h: HostIx, ms: u64, stopped: bool) {
        self.record(|st| {
            match st.hosts.get_mut(h) {
                Some(s) if s.departed.is_none() => {
                    s.departed = Some(ms);
                    if stopped {
                        s.stopped.push((s.incarnation, Duration::from_millis(ms)));
                        s.retire();
                    }
                }
                _ => return,
            }
            let lost: Vec<ParentLoss> = st
                .hosts
                .iter()
                .enumerate()
                .filter(|&(v, x)| v != h && x.departed.is_none())
                .filter_map(|(v, x)| {
                    let streams = x.parents.iter().filter(|&&p| p == h).count();
                    (streams > 0).then_some(ParentLoss {
                        viewer: v,
                        parent: h,
                        departed_ms: ms,
                        noticed_ms: None,
                        streams,
                    })
                })
                .collect();
            st.parent_losses.extend(lost);
        });
    }

    /// `h` stopped counting on push parent `parent` at `now`: its link to
    /// it closed, or a stats sample named no stream of it. Notes the loss
    /// of a parent that departed, once, and logs it.
    fn parent_gone(&self, h: HostIx, parent: HostIx, now: u64) {
        let pending = self
            .st
            .borrow()
            .parent_losses
            .iter()
            .any(|l| l.viewer == h && l.parent == parent && l.noticed_ms.is_none());
        if !pending {
            return;
        }
        let after = self.record(|st| {
            let l = st
                .parent_losses
                .iter_mut()
                .find(|l| l.viewer == h && l.parent == parent && l.noticed_ms.is_none())?;
            l.noticed_ms = Some(now);
            Some(now.saturating_sub(l.departed_ms))
        });
        if let Some(after) = after {
            self.log(
                self.name(h),
                format!("parent lost {} after {after} ms", self.name(parent)),
            );
        }
    }

    /// `h`'s link to `peer` went ([`NodeEvent::PeerRemoved`]).
    ///
    /// [`NodeEvent::PeerRemoved`]: strims_node::NodeEvent::PeerRemoved
    pub fn peer_removed(&self, h: HostIx, peer: Option<HostIx>) {
        if let Some(p) = peer {
            self.parent_gone(h, p, sim_now_ms());
        }
    }

    pub fn set_reading_paused(&self, h: HostIx, paused: bool, now_ms: u64) {
        self.record(|st| {
            if let Some(s) = st.hosts.get_mut(h) {
                s.reading_paused = paused;
                if !paused {
                    s.resumed_ms = now_ms;
                }
            }
        });
    }

    pub fn reading_paused(&self, h: HostIx) -> bool {
        self.st
            .borrow()
            .hosts
            .get(h)
            .is_some_and(|s| s.reading_paused)
    }

    pub fn set_uplink(&self, h: HostIx, rate: u64) {
        self.record(|st| {
            if let Some(s) = st.hosts.get_mut(h) {
                s.uplink = Some(rate);
            }
        });
    }

    pub fn take_uplink(&self, h: HostIx) -> Option<u64> {
        self.record(|st| st.hosts.get_mut(h).and_then(|s| s.uplink.take()))
    }

    pub fn request_connect(&self, from: HostIx, to: HostIx, network: usize) {
        self.record(|st| {
            if let Some(s) = st.hosts.get_mut(from) {
                s.connects.push((to, network));
            }
        });
    }

    pub fn take_connects(&self, h: HostIx) -> Vec<(HostIx, usize)> {
        self.record(|st| {
            st.hosts
                .get_mut(h)
                .map(|s| std::mem::take(&mut s.connects))
                .unwrap_or_default()
        })
    }

    pub fn request_writer_reset(&self, swarm: usize) {
        self.record(|st| st.reset_writer.insert(swarm));
    }

    pub fn take_writer_reset(&self, swarm: usize) -> bool {
        self.record(|st| st.reset_writer.remove(&swarm))
    }

    // --- streams --------------------------------------------------------

    pub fn mark_written(&self, swarm: usize, seqs: std::ops::Range<u64>) {
        self.record(|st| {
            if let Some(w) = st.written.get_mut(swarm) {
                for s in seqs {
                    w.insert(s);
                }
            }
        });
    }

    /// A media chunk of `swarm` opened at `time_ms`: its sequence number,
    /// next after every incarnation's.
    pub fn media_chunk(&self, swarm: usize, time_ms: u64) -> u64 {
        self.record(|st| {
            let Some(Some(log)) = st.media.get_mut(swarm) else {
                return 0;
            };
            let seq = log.times.len() as u64;
            log.times.push(time_ms);
            if let Some(w) = st.written.get_mut(swarm) {
                w.insert(seq);
            }
            seq
        })
    }

    /// The start of media swarm `swarm`'s latest GOP at or before stream
    /// offset `offset`, as its writer wrote it.
    pub fn gop_start(&self, swarm: usize, offset: u64) -> Option<u64> {
        let st = self.st.borrow();
        let log = st.media.get(swarm)?.as_ref()?;
        log.gops.iter().rev().find(|&&g| g <= offset).copied()
    }

    /// Notes how viewer `h`'s join rewind of `swarm` went.
    pub fn viewer_rewound(&self, swarm: usize, h: HostIx, r: JoinRewind) {
        self.record(|st| st.viewers.entry((swarm, h)).or_default().rewound(r));
    }

    /// Updates media swarm `swarm`'s log.
    pub fn media_log(&self, swarm: usize, f: impl FnOnce(&mut MediaLog)) {
        self.record(|st| {
            if let Some(Some(log)) = st.media.get_mut(swarm) {
                f(log);
            }
        });
    }

    /// Feeds bytes read at `offset` to a media viewer's parser and stats,
    /// logging as [`World::viewer_data`] does.
    pub fn viewer_media(
        &self,
        swarm: usize,
        h: HostIx,
        parser: &mut MediaParser,
        offset: u64,
        data: &[u8],
    ) {
        let now = sim_now_ms();
        let Some(spec) = self.spec.swarms.get(swarm) else {
            return;
        };
        let chunk = spec.options.chunk_size;
        let every = (spec.rate() as u64 * 10 / chunk.max(1) as u64).max(1);
        let mut lines = Vec::new();
        self.record(|st| {
            let v = st.viewers.entry((swarm, h)).or_default();
            parser.push(self.seed, swarm as u32, chunk, offset, data, |p| {
                let first = v.record(p, now);
                match p {
                    Parsed::Frame(f) if first || f.seq % every == 0 => {
                        let lag = now.saturating_sub(f.time_ms);
                        let what = if first { "first chunk" } else { "chunk" };
                        lines.push(format!("swarm {swarm} {what} {} lag {lag}", f.seq));
                    }
                    Parsed::Bad => lines.push(format!("swarm {swarm} bad chunk")),
                    Parsed::Frame(_) => {}
                }
            });
        });
        for l in lines {
            self.log(self.name(h), l);
        }
    }

    pub fn viewer_start(&self, swarm: usize, h: HostIx) {
        self.record(|st| {
            st.viewers
                .entry((swarm, h))
                .or_default()
                .start(sim_now_ms())
        });
    }

    /// Feeds read bytes to the viewer's parser and stats; logs the first
    /// frame of an incarnation and a progress line every 10 s of stream.
    pub fn viewer_data(&self, swarm: usize, h: HostIx, parser: &mut FrameParser, data: &[u8]) {
        let now = sim_now_ms();
        let every = self.spec.swarms.get(swarm).map_or(1, |s| {
            let per_sec = 1000 / s.write_interval.as_millis().max(1) as u64;
            (per_sec * 10 * s.frames_per_write as u64).max(1)
        });
        let mut lines = Vec::new();
        self.record(|st| {
            let v = st.viewers.entry((swarm, h)).or_default();
            parser.push(self.seed, swarm as u32, data, |p| {
                let first = v.record(p, now);
                match p {
                    Parsed::Frame(f) if first || f.seq % every == 0 => {
                        let lag = now.saturating_sub(f.time_ms);
                        let what = if first { "first frame" } else { "frame" };
                        lines.push(format!("swarm {swarm} {what} {} lag {lag}", f.seq));
                    }
                    Parsed::Bad => lines.push(format!("swarm {swarm} bad frame")),
                    Parsed::Frame(_) => {}
                }
            });
        });
        for l in lines {
            self.log(self.name(h), l);
        }
    }

    pub fn viewer_underrun(&self, swarm: usize, h: HostIx, skipped: u64) {
        self.record(|st| {
            if let Some(v) = st.viewers.get_mut(&(swarm, h)) {
                v.underruns += 1;
                v.skipped_bytes += skipped;
            }
        });
        self.log(
            self.name(h),
            format!("swarm {swarm} underrun skipped {skipped}"),
        );
    }

    pub fn viewer_reset(&self, swarm: usize, h: HostIx) {
        self.record(|st| {
            if let Some(v) = st.viewers.get_mut(&(swarm, h)) {
                v.resets += 1;
            }
        });
        self.log(self.name(h), format!("swarm {swarm} reset"));
    }

    // --- vpn ----------------------------------------------------------------

    pub fn probe_sent(&self, id: u64, src: HostIx, dst: HostIx) {
        let p = Probe {
            sent_ms: sim_now_ms(),
            src,
            dst,
            delivered: None,
        };
        self.record(|st| st.probes.insert(id, p));
    }

    /// A probe arrived; records it once.
    pub fn probe_delivered(&self, id: u64, hops: usize) -> bool {
        self.record(|st| match st.probes.get_mut(&id) {
            Some(p) if p.delivered.is_none() => {
                p.delivered = Some(hops);
                true
            }
            _ => false,
        })
    }

    /// A peer exchange of `h` with `with` ended.
    pub fn exchange_closed(&self, h: HostIx, with: Option<HostIx>, outcome: &ExchangeOutcome) {
        let name = format!("{outcome:?}");
        let name = name.split('(').next().unwrap_or_default();
        self.record(|st| {
            *st.vpn.exchange_outcomes.entry(name.to_owned()).or_default() += 1;
            let Some(hs) = st.hosts.get_mut(h) else {
                return;
            };
            *hs.totals
                .exchange_outcomes
                .entry(name.to_owned())
                .or_default() += 1;
            if let Some(x) = with {
                let c = hs.exchanges.entry(x).or_default();
                *c = c.wrapping_add(1);
            }
            let early = u64::from(sim_now() < EARLY_OFFERS);
            if name == "Answered" {
                hs.totals.offers_early += early;
            }
            if name == "Rejected"
                && let Some(t) = with.and_then(|x| st.hosts.get_mut(x))
            {
                t.totals.offers_refused += 1;
                t.totals.offers_early += early;
            }
        });
    }

    /// `h`'s pool resolved a swarm's source (`pool-find`).
    pub fn source_resolved(&self, h: HostIx) {
        let now = sim_now_ms();
        self.record(|st| {
            if let Some(hs) = st.hosts.get_mut(h) {
                hs.totals.source_resolved_ms.get_or_insert(now);
            }
        });
    }

    /// A probe of `h`'s pool ended its trial, its link staying (`won`) or
    /// going.
    pub fn pool_trial(&self, h: HostIx, won: bool) {
        self.record(|st| {
            if let Some(hs) = st.hosts.get_mut(h) {
                hs.totals.pool_trials += 1;
                hs.totals.pool_trial_wins += u64::from(won);
            }
        });
    }

    /// `h` made a link (`added`) or lost one.
    pub fn peer_changed(&self, h: HostIx, added: bool) {
        self.record(|st| {
            if let Some(hs) = st.hosts.get_mut(h) {
                let c = if added {
                    &mut hs.totals.peers_added
                } else {
                    &mut hs.totals.peers_removed
                };
                *c += 1;
            }
        });
    }

    /// How many of `h`'s peer exchanges with `with` have ended.
    pub fn exchanges_with(&self, h: HostIx, with: HostIx) -> u32 {
        self.st
            .borrow()
            .hosts
            .get(h)
            .and_then(|s| s.exchanges.get(&with).copied())
            .unwrap_or(0)
    }

    /// A hash table get or peer index search started at `start_ms` ended.
    pub fn lookup_done(&self, start_ms: u64, search: bool, ok: bool) {
        self.record(|st| {
            if search {
                st.vpn.searches += 1;
                st.vpn.searches_ok += u64::from(ok);
            } else {
                st.vpn.gets += 1;
                st.vpn.gets_ok += u64::from(ok);
            }
            st.lookups.push((start_ms, search, ok));
        });
    }

    pub fn with_vpn(&self, f: impl FnOnce(&mut VpnReport)) {
        self.record(|st| f(&mut st.vpn));
    }

    pub fn with_rng<T>(&self, f: impl FnOnce(&mut DetRng) -> T) -> T {
        self.record(|st| f(&mut st.rng))
    }

    // --- samples --------------------------------------------------------

    /// Keeps a snapshot's stats bins and forecast log for the tables
    /// ([`HostReport::node_bins`], [`HostReport::forecasts`]): each snapshot
    /// hands them over once, so every caller of `Node::metrics` passes its
    /// snapshot here.
    pub fn observe_bins(&self, h: HostIx, m: &NodeMetrics) {
        let (Some(width), Some(swarm)) = (self.spec.wire_bins, self.spec.swarms.first()) else {
            return;
        };
        if m.bins.is_empty()
            && m.forecasts.is_empty()
            && m.held.is_empty()
            && m.forecasts_lost == 0
            && m.held_lost == 0
            && m.haves.is_empty()
            && m.haves_lost == 0
            && m.requests.is_empty()
            && m.requests_lost == 0
        {
            return;
        }
        let origin = self.clock_at(h, swarm.start);
        let width = width.max(Duration::from_millis(1)).as_millis() as i64;
        let epoch = self.clock_epoch(h);
        self.record(|st| {
            let ids = &st.ids;
            let Some(hs) = st.hosts.get_mut(h) else {
                return;
            };
            let key = hs.swarms.first().copied().flatten();
            for b in &m.bins {
                let Some(sw) = b.swarms.iter().find(|s| Some(s.swarm) == key) else {
                    continue;
                };
                let since = (b.at - origin).as_millis();
                if since < 0 {
                    continue;
                }
                let o = &b.outbound;
                let c = &sw.counters;
                hs.node_bins.push(NodeBinRecord {
                    bin: (since / width) as u64,
                    incarnation: hs.incarnation,
                    counters: [
                        sw.hits,
                        sw.misses,
                        sw.pruned,
                        sw.parents,
                        sw.upload_capacity,
                        sw.upload_busy_permille,
                        sw.seed_copies,
                        c.chunks_sent,
                        c.pushed_chunks,
                        c.requests_timed_out,
                        c.wasted_chunks,
                        c.answers_refused,
                        c.answers_dropped,
                        sw.switched,
                        sw.shed,
                        sw.sheds_sent,
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
                        b.bytes_written.saturating_sub(o.data_bytes),
                        sw.seed_budget_capacity,
                    ],
                    waits: [c.push_wait, c.answer_wait],
                });
            }
            for f in &m.forecasts {
                if Some(f.swarm) != key {
                    continue;
                }
                let at = (f.forecast.at - epoch).to_std().unwrap_or_default();
                let opened = (f.forecast.channel_open - epoch)
                    .to_std()
                    .unwrap_or_default();
                hs.forecasts.push(ForecastRecord {
                    at,
                    opened,
                    peer: f.peer.and_then(|id| ids.get(&id).copied()),
                    forecast: f.forecast,
                });
            }
            let t = |t: Time| (t - epoch).to_std().unwrap_or_default();
            let host = |id: Option<HostId>| id.and_then(|id| ids.get(&id).copied());
            for c in &m.held {
                if Some(c.swarm) == key {
                    let at = t(c.at);
                    hs.held.push((c.bin.base_offset(), at));
                    hs.arrivals.push(ArrivalHostRecord {
                        at,
                        expected: c.held.expected.map(t),
                        released: c.held.released.map(t),
                        asked: c.held.asked.map(t),
                        last_ask: c.held.last_ask.map(|(_, x)| t(x)),
                        peer: host(c.peer),
                        parent: host(c.parent),
                        asked_peer: host(c.asked),
                        held: c.held,
                    });
                }
            }
            for x in m.haves.iter().filter(|x| Some(x.swarm) == key) {
                hs.haves.push(HaveHostRecord {
                    at: t(x.have.at),
                    opened: t(x.have.channel_open),
                    peer: host(x.peer),
                    have: x.have,
                });
            }
            for x in m.requests.iter().filter(|x| Some(x.swarm) == key) {
                hs.requests.push(RequestHostRecord {
                    epoch,
                    peer: host(x.peer),
                    rivals: x.rivals.iter().map(|&w| host(w)).collect(),
                    record: x.record.clone(),
                });
            }
            hs.forecasts_lost += m.forecasts_lost;
            hs.held_lost += m.held_lost;
            hs.haves_lost += m.haves_lost;
            hs.requests_lost += m.requests_lost;
        });
    }

    /// Keeps the last snapshots of the nodes whose task went since the
    /// last call ([`NodeConfig::last_metrics`]) as a stats sample at `at`:
    /// the logs ([`World::observe_bins`]; not the bins: a departed host's
    /// end at its last sample), the counters ([`HostStats`]) and the
    /// metrics its incarnation's totals take. A crash's, taken as it drops
    /// the node, before the host's incarnation retires; and once the run
    /// `ended` at `at` (the simulation dropped every node), those of the
    /// hosts that departed, which no sample asked since they left. The
    /// others' ran to the final sample, and what came past it is left out.
    pub fn observe_stopped(&self, at: Duration, ended: bool) {
        let stopped: Vec<(HostIx, NodeMetrics)> = self.stopped_rx.try_iter().collect();
        for (h, mut m) in stopped {
            if ended && !self.departed(h) {
                continue;
            }
            m.bins.clear();
            self.observe_bins(h, &m);
            let live: Vec<bool> = self
                .spec
                .swarms
                .iter()
                .map(|s| at >= s.start && at <= s.stop)
                .collect();
            self.record(|st| {
                let Some(hs) = st.hosts.get_mut(h) else {
                    return;
                };
                let swarms = std::mem::take(&mut hs.swarms);
                hs.stats
                    .observe(&m, &swarms, &live, at, self.clock_at(h, at));
                hs.swarms = swarms;
            });
            self.store_metrics(h, m);
        }
    }

    pub fn store_metrics(&self, h: HostIx, m: NodeMetrics) {
        self.record(|st| {
            if let Some(s) = st.hosts.get_mut(h) {
                s.last = Some(m);
            }
        });
    }

    /// The node's key for each scenario swarm.
    pub fn swarm_keys(&self, h: HostIx) -> Vec<Option<SwarmKey>> {
        self.st
            .borrow()
            .hosts
            .get(h)
            .map(|s| s.swarms.clone())
            .unwrap_or_default()
    }

    /// Folds a stats sample of `h` into its [`HostStats`] and checks the
    /// runner invariants (see [`crate::Invariants`]). Each broken invariant
    /// is logged once per host.
    pub fn observe_stats(&self, h: HostIx, m: &NodeMetrics) {
        self.observe_bins(h, m);
        self.take_parents();
        if self.spec.departures.is_some() {
            self.observe_parents(h, m);
        }
        let inv = &self.spec.invariants;
        let mut found = Vec::new();
        self.record(|st| {
            let Some(hs) = st.hosts.get_mut(h) else {
                return;
            };
            let swarms = std::mem::take(&mut hs.swarms);
            let now = sim_now();
            let live: Vec<bool> = self
                .spec
                .swarms
                .iter()
                .map(|s| now >= s.start && now <= s.stop)
                .collect();
            let g = hs
                .stats
                .observe(m, &swarms, &live, now, self.clock_at(h, now));
            let linked = m.links.iter().filter_map(|l| l.host_id);
            let mut ns: Vec<HostIx> = linked.filter_map(|id| st.ids.get(&id).copied()).collect();
            ns.sort_unstable();
            ns.dedup();
            hs.neighbours.push((sim_now(), ns));
            if self.spec.transfer_samples {
                for (s, key) in swarms.iter().enumerate() {
                    let index = |id: &HostId| st.ids.get(id).copied();
                    if let Some(key) = key
                        && let Some(t) = TransferSample::of(m, *key, index)
                    {
                        hs.transfers.push((sim_now(), s, t));
                    }
                }
            }
            hs.swarms = swarms;
            let mut check = |kind: &'static str, broken: bool, what: String| {
                if broken && hs.broken.insert(kind) {
                    found.push(what);
                }
            };
            if let Some(max) = inv.max_data_queue {
                check(
                    "data queue",
                    g.queued_data > max,
                    format!("a data queue holds {} chunks, above {max}", g.queued_data),
                );
                check(
                    "queue drops",
                    g.queue_drops > 0,
                    format!(
                        "{} requests or pushes refused on a full queue",
                        g.queue_drops
                    ),
                );
            }
            if let Some(max) = inv.max_pending_requests {
                check(
                    "request deadlines",
                    g.pending_requests > max,
                    format!(
                        "a channel tracks {} request deadlines, above {max}",
                        g.pending_requests
                    ),
                );
            }
            if inv.clean_input {
                check(
                    "invalid data",
                    g.invalid > 0,
                    format!("{} data or chunks failed verification", g.invalid),
                );
                check(
                    "rejected input",
                    g.rejected > 0,
                    format!("{} ppspp messages rejected as invalid", g.rejected),
                );
                check(
                    "vpn bad input",
                    g.vpn_bad_input > 0,
                    format!("{} bad vpn frames", g.vpn_bad_input),
                );
            }
        });
        let now = sim_now_ms();
        for f in found {
            let f = format!("{} {f}", self.name(h));
            self.log("invariant", &f);
            self.record(|st| st.violations.push((now, f)));
        }
    }

    /// Moves what the nodes' parent sinks sent into the records.
    fn take_parents(&self) {
        self.record(|st| st.parents.extend(self.parent_rx.try_iter()));
    }

    /// Every node's parent changes of the first scenario swarm
    /// ([`crate::Report::parent_changes`]), in run time: those its parent
    /// log gave, and at each stop of an incarnation that didn't close its
    /// links, the loss of the parents it had then.
    pub fn parent_changes(&self) -> Vec<ParentRecord> {
        self.take_parents();
        let st = self.st.borrow();
        let mut by_host: BTreeMap<HostIx, Vec<(u32, ParentInfo)>> = BTreeMap::new();
        for &(h, inc, p) in &st.parents {
            by_host.entry(h).or_default().push((inc, p));
        }
        let mut out = Vec::new();
        for (h, hs) in st.hosts.iter().enumerate() {
            let epoch = self.clock_epoch(h);
            let mut open: BTreeMap<u16, HostIx> = BTreeMap::new();
            let (mut inc, mut last) = (0, Duration::ZERO);
            // An incarnation's parents are lost where it stopped (in the
            // fault's ms, not before its own last change).
            let stop = |inc: u32, last: Duration, open: &mut BTreeMap<u16, HostIx>| {
                let open = std::mem::take(open);
                let at = hs.stopped.iter().find(|&&(i, _)| i == inc);
                let Some(at) = at.map(|&(_, at)| at.max(last)) else {
                    return Vec::new();
                };
                open.into_keys()
                    .map(|stream| ParentRecord {
                        at,
                        node: h,
                        stream,
                        parent: None,
                        why: strims_ppspp::ParentWhy::Lost,
                    })
                    .collect()
            };
            for &(i, p) in by_host.get(&h).into_iter().flatten() {
                let key = hs.first_swarm.iter().find(|&&(j, _)| j == i);
                if key.and_then(|&(_, k)| k) != Some(p.swarm) {
                    continue;
                }
                if i != inc {
                    out.extend(stop(inc, last, &mut open));
                    inc = i;
                }
                let parent = p.parent.and_then(|id| st.ids.get(&id).copied());
                match parent {
                    Some(x) => open.insert(p.stream, x),
                    None => open.remove(&p.stream),
                };
                last = (p.at - epoch).to_std().unwrap_or_default();
                out.push(ParentRecord {
                    at: last,
                    node: h,
                    stream: p.stream,
                    parent,
                    why: p.why,
                });
            }
            out.extend(stop(inc, last, &mut open));
        }
        out.sort_by_key(|r| r.at);
        out
    }

    /// Keeps the push parents of `h`'s first scenario swarm from a stats
    /// sample ([`HostState::parents`]), and notes the departed ones it no
    /// longer has ([`World::parent_gone`]).
    fn observe_parents(&self, h: HostIx, m: &NodeMetrics) {
        let now = sim_now_ms();
        let gone = self.record(|st| {
            let ids = &st.ids;
            let hs = st.hosts.get_mut(h)?;
            let key = hs.swarms.first().copied().flatten()?;
            let parents: Vec<HostIx> = m
                .runner
                .swarms
                .iter()
                .find(|s| s.swarm == key)
                .map(|s| {
                    s.parent_hosts
                        .iter()
                        .filter_map(|p| p.as_ref().and_then(|id| ids.get(id).copied()))
                        .collect()
                })
                .unwrap_or_default();
            let gone: Vec<HostIx> = hs
                .parents
                .iter()
                .copied()
                .filter(|p| !parents.contains(p))
                .collect();
            hs.parents = parents;
            Some(gone)
        });
        for p in gone.unwrap_or_default() {
            self.parent_gone(h, p, now);
        }
    }

    /// Samples the heap and checks the invariants (see
    /// [`crate::Invariants`]).
    pub fn check_invariants(&self) {
        let now = sim_now_ms();
        let inv = &self.spec.invariants;
        let mut found = Vec::new();
        if let Some(c) = self.memory {
            let live = c.live();
            self.record(|st| st.memory_samples.push((now, live)));
            if let Some(max) = inv.max_heap
                && c.peak() > 0
                && live > max
            {
                found.push(format!("live heap {live} above {max}"));
            }
        }
        let in_window = inv.progress.iter().any(|&(a, b)| {
            let (a, b) = (a.as_millis() as u64, b.as_millis() as u64);
            (a..b).contains(&now)
        });
        self.record(|st| {
            for (&(swarm, h), v) in &st.viewers {
                let before = st.progress.insert((swarm, h), v.frames);
                let Some(hs) = st.hosts.get(h) else {
                    continue;
                };
                let settled = v
                    .joins
                    .last()
                    .is_some_and(|j| now.saturating_sub(j.start_ms) > 10_000)
                    && now.saturating_sub(hs.resumed_ms) > 10_000;
                if in_window
                    && hs.node.is_some()
                    && hs.departed.is_none()
                    && !hs.reading_paused
                    && settled
                    && before.is_some_and(|b| b >= v.frames)
                {
                    found.push(format!(
                        "{} read nothing new from swarm {swarm}",
                        self.name(h)
                    ));
                }
            }
        });
        for f in found {
            self.log("invariant", &f);
            self.record(|st| st.violations.push((now, f)));
        }
    }
}
