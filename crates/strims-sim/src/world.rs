// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One run's shared state: keys derived from the seed, the host registry,
//! the trace and every statistic.
//!
//! Every host and the controller run on the turmoil thread, so the state is
//! an `Rc<World>` with a `RefCell` inside. Borrows never cross an `.await`.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use strims_base::rng::{DetRng, det_rng};
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_node::{
    ExchangeOutcome, HostId, NetworkId, Node, NodeConfig, NodeMetrics, PpsppCapture, SwarmKey,
    WebRtcConfig,
};
use strims_proto::types::Key;

use crate::memory::GroupCounters;
use crate::report::{HostReport, VpnReport};
use crate::spec::{HostIx, Scenario, Transport};
use crate::stats::HostStats;
use crate::stream::{FrameParser, Parsed, SeqSet, ViewerStats};
use crate::{EPOCH_MS, TCP_PORT, UDP_PORT, WS_PORT};

/// The peer index key every host publishes under (with its name as salt).
pub(crate) const PEER_INDEX_KEY: &[u8] = b"strims-sim peer index";
pub(crate) const HASH_TABLE_SALT: &[u8] = b"strims-sim";

pub(crate) fn epoch() -> Time {
    Time::from_unix_millis(EPOCH_MS)
}

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
    /// The node's key for each scenario swarm it takes part in.
    pub swarms: Vec<Option<SwarmKey>>,
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
    /// Runner invariants this host already broke (each is logged once).
    pub broken: BTreeSet<&'static str>,
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
}

/// One run.
#[derive(Debug)]
pub(crate) struct World {
    pub seed: u64,
    pub spec: Scenario,
    pub profile_keys: Vec<Key>,
    pub swarm_keys: Vec<Key>,
    pub network_keys: Vec<Key>,
    pub memory: Option<GroupCounters>,
    pub st: RefCell<WorldState>,
}

impl World {
    pub fn new(spec: Scenario, seed: u64, memory: Option<GroupCounters>) -> World {
        let profile_keys = (0..spec.hosts.len())
            .map(|i| derive_key(b'P', seed, i))
            .collect();
        let swarm_keys = (0..spec.swarms.len())
            .map(|i| derive_key(b'S', seed, i))
            .collect();
        let network_keys = (0..spec.networks.len())
            .map(|i| derive_key(b'N', seed, i))
            .collect();
        let st = WorldState {
            trace: Vec::new(),
            hosts: (0..spec.hosts.len())
                .map(|_| HostState::default())
                .collect(),
            ids: BTreeMap::new(),
            viewers: BTreeMap::new(),
            written: vec![SeqSet::default(); spec.swarms.len()],
            reset_writer: BTreeSet::new(),
            vpn: VpnReport::default(),
            probes: BTreeMap::new(),
            lookups: Vec::new(),
            memory_samples: Vec::new(),
            violations: Vec::new(),
            progress: BTreeMap::new(),
            rng: det_rng(seed ^ 0xc0_47_01),
        };
        World {
            seed,
            spec,
            profile_keys,
            swarm_keys,
            network_keys,
            memory,
            st: RefCell::new(st),
        }
    }

    pub fn name(&self, h: HostIx) -> &str {
        self.spec.name_of(h)
    }

    /// A peer's name, or `?` for an unknown host id.
    pub fn label(&self, h: Option<HostIx>) -> &str {
        h.map_or("?", |h| self.name(h))
    }

    // --- trace ----------------------------------------------------------

    /// Appends a trace line stamped with the current simulated time.
    pub fn log(&self, who: &str, line: impl AsRef<str>) {
        self.log_at(sim_now_ms(), who, line);
    }

    pub fn log_at(&self, ms: u64, who: &str, line: impl AsRef<str>) {
        self.st
            .borrow_mut()
            .trace
            .push(format!("{ms} {who} {}", line.as_ref()));
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
        c.vpn.routing = spec.routing.unwrap_or(self.spec.routing);
        if let Some(dir) = &self.spec.capture {
            let path = dir.join(format!("{}-{incarnation}.ppcap", spec.name));
            c.ppspp_capture = Some(PpsppCapture {
                open: Arc::new(move |_, _| {
                    std::fs::create_dir_all(path.parent().unwrap_or(&path))?;
                    let f = std::fs::File::create(&path)?;
                    Ok(Box::new(std::io::BufWriter::new(f)))
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
        let mut st = self.st.borrow_mut();
        let Some(hs) = st.hosts.get_mut(h) else {
            return 0;
        };
        hs.incarnation += 1;
        hs.incarnation
    }

    pub fn register(
        &self,
        h: HostIx,
        node: &Node,
        nets: Vec<Option<NetworkId>>,
        swarms: Vec<Option<SwarmKey>>,
    ) {
        let id = node.host_id();
        let inc = {
            let mut st = self.st.borrow_mut();
            st.ids.insert(id, h);
            let Some(hs) = st.hosts.get_mut(h) else {
                return;
            };
            hs.node = Some(node.clone());
            hs.host_id = Some(id);
            hs.nets = nets;
            hs.swarms = swarms;
            hs.incarnation
        };
        self.log(self.name(h), format!("up incarnation {inc}"));
    }

    /// Registers a host id as soon as the node exists.
    pub fn register_id(&self, h: HostIx, id: HostId) {
        self.st.borrow_mut().ids.insert(id, h);
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
        self.st
            .borrow_mut()
            .hosts
            .get_mut(h)
            .is_some_and(|s| s.dialing.insert(l))
    }

    /// A dial finished; logs it when the link's outcome changed.
    pub fn end_dial(&self, h: HostIx, l: usize, result: Result<(), String>) {
        let changed = {
            let mut st = self.st.borrow_mut();
            let Some(s) = st.hosts.get_mut(h) else {
                return;
            };
            s.dialing.remove(&l);
            s.dial_ok.insert(l, result.is_ok()) != Some(result.is_ok())
        };
        if changed {
            let to = self.spec.links.get(l).map_or("?", |l| self.name(l.to));
            match result {
                Ok(()) => self.log(self.name(h), format!("dial {to} ok")),
                Err(e) => self.log(self.name(h), format!("dial {to} failed: {e}")),
            }
        }
    }

    // --- faults -------------------------------------------------------------

    pub fn crashed(&self, h: HostIx) {
        if let Some(s) = self.st.borrow_mut().hosts.get_mut(h) {
            s.retire();
            s.totals.restarts += 1;
        }
    }

    pub fn set_reading_paused(&self, h: HostIx, paused: bool, now_ms: u64) {
        if let Some(s) = self.st.borrow_mut().hosts.get_mut(h) {
            s.reading_paused = paused;
            if !paused {
                s.resumed_ms = now_ms;
            }
        }
    }

    pub fn reading_paused(&self, h: HostIx) -> bool {
        self.st
            .borrow()
            .hosts
            .get(h)
            .is_some_and(|s| s.reading_paused)
    }

    pub fn set_uplink(&self, h: HostIx, rate: u64) {
        if let Some(s) = self.st.borrow_mut().hosts.get_mut(h) {
            s.uplink = Some(rate);
        }
    }

    pub fn take_uplink(&self, h: HostIx) -> Option<u64> {
        self.st
            .borrow_mut()
            .hosts
            .get_mut(h)
            .and_then(|s| s.uplink.take())
    }

    pub fn request_connect(&self, from: HostIx, to: HostIx, network: usize) {
        if let Some(s) = self.st.borrow_mut().hosts.get_mut(from) {
            s.connects.push((to, network));
        }
    }

    pub fn take_connects(&self, h: HostIx) -> Vec<(HostIx, usize)> {
        self.st
            .borrow_mut()
            .hosts
            .get_mut(h)
            .map(|s| std::mem::take(&mut s.connects))
            .unwrap_or_default()
    }

    pub fn request_writer_reset(&self, swarm: usize) {
        self.st.borrow_mut().reset_writer.insert(swarm);
    }

    pub fn take_writer_reset(&self, swarm: usize) -> bool {
        self.st.borrow_mut().reset_writer.remove(&swarm)
    }

    // --- streams --------------------------------------------------------

    pub fn mark_written(&self, swarm: usize, seqs: std::ops::Range<u64>) {
        if let Some(w) = self.st.borrow_mut().written.get_mut(swarm) {
            for s in seqs {
                w.insert(s);
            }
        }
    }

    pub fn viewer_start(&self, swarm: usize, h: HostIx) {
        self.st
            .borrow_mut()
            .viewers
            .entry((swarm, h))
            .or_default()
            .start(sim_now_ms());
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
        {
            let mut st = self.st.borrow_mut();
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
        }
        for l in lines {
            self.log(self.name(h), l);
        }
    }

    pub fn viewer_underrun(&self, swarm: usize, h: HostIx, skipped: u64) {
        if let Some(v) = self.st.borrow_mut().viewers.get_mut(&(swarm, h)) {
            v.underruns += 1;
            v.skipped_bytes += skipped;
        }
        self.log(
            self.name(h),
            format!("swarm {swarm} underrun skipped {skipped}"),
        );
    }

    pub fn viewer_reset(&self, swarm: usize, h: HostIx) {
        if let Some(v) = self.st.borrow_mut().viewers.get_mut(&(swarm, h)) {
            v.resets += 1;
        }
        self.log(self.name(h), format!("swarm {swarm} reset"));
    }

    // --- vpn ----------------------------------------------------------------

    pub fn probe_sent(&self, id: u64, src: HostIx, dst: HostIx) {
        self.st.borrow_mut().probes.insert(
            id,
            Probe {
                sent_ms: sim_now_ms(),
                src,
                dst,
                delivered: None,
            },
        );
    }

    /// A probe arrived; records it once.
    pub fn probe_delivered(&self, id: u64, hops: usize) -> bool {
        let mut st = self.st.borrow_mut();
        match st.probes.get_mut(&id) {
            Some(p) if p.delivered.is_none() => {
                p.delivered = Some(hops);
                true
            }
            _ => false,
        }
    }

    pub fn exchange_closed(&self, outcome: &ExchangeOutcome) {
        let name = format!("{outcome:?}");
        let name = name.split('(').next().unwrap_or_default().to_owned();
        *self
            .st
            .borrow_mut()
            .vpn
            .exchange_outcomes
            .entry(name)
            .or_default() += 1;
    }

    /// A hash table get or peer index search started at `start_ms` ended.
    pub fn lookup_done(&self, start_ms: u64, search: bool, ok: bool) {
        let mut st = self.st.borrow_mut();
        if search {
            st.vpn.searches += 1;
            st.vpn.searches_ok += u64::from(ok);
        } else {
            st.vpn.gets += 1;
            st.vpn.gets_ok += u64::from(ok);
        }
        st.lookups.push((start_ms, search, ok));
    }

    pub fn with_vpn(&self, f: impl FnOnce(&mut VpnReport)) {
        f(&mut self.st.borrow_mut().vpn);
    }

    pub fn with_rng<T>(&self, f: impl FnOnce(&mut DetRng) -> T) -> T {
        f(&mut self.st.borrow_mut().rng)
    }

    // --- samples --------------------------------------------------------

    pub fn store_metrics(&self, h: HostIx, m: NodeMetrics) {
        if let Some(s) = self.st.borrow_mut().hosts.get_mut(h) {
            s.last = Some(m);
        }
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
        let inv = &self.spec.invariants;
        let mut found = Vec::new();
        {
            let mut st = self.st.borrow_mut();
            let Some(hs) = st.hosts.get_mut(h) else {
                return;
            };
            let swarms = std::mem::take(&mut hs.swarms);
            let g = hs.stats.observe(m, &swarms);
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
        }
        let now = sim_now_ms();
        for f in found {
            let f = format!("{} {f}", self.name(h));
            self.log("invariant", &f);
            self.st.borrow_mut().violations.push((now, f));
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
            self.st.borrow_mut().memory_samples.push((now, live));
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
        {
            let mut st = self.st.borrow_mut();
            let st = &mut *st;
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
        }
        for f in found {
            self.log("invariant", &f);
            self.st.borrow_mut().violations.push((now, f));
        }
    }
}
