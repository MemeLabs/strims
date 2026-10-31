// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What a run measured.
//!
//! A [`Report`] keeps the raw records (the frames each seed wrote, every
//! frame each viewer read with its lag, each host's counters) and derives
//! the measures from them:
//!
//! - **delivery** ([`Report::delivery`]): over a swarm's viewers, the
//!   frames written that each viewer read, over the frames written. Frames
//!   written while a viewer was down count as missed.
//! - **lag** ([`Report::lag`]): per frame read, the read time minus the
//!   write time the frame carries, in simulated milliseconds.
//! - **seed share**: the seed's share of everything the swarm's hosts
//!   wrote to their links ([`UploadDistribution::seed_share`]), and of the
//!   ppspp data bytes alone ([`Report::data_share`]).
//! - **duplicates** ([`Report::duplicates`]): chunks received that the
//!   node already had, over chunks received.
//! - **upload fairness** ([`UploadDistribution::gini`]): how unevenly the
//!   viewers and relays share the upload.
//! - **push hit rate** ([`SwarmCounters::hit_rate`]).
//!
//! The runner counters behind the data share, duplicates and hit rate are
//! described in [`crate::stats`].
//!
//! A media swarm's frames are its chunks that hold media, their times their
//! first bytes' ([`crate::MediaSpec`]); [`Report::media`] keeps what its
//! seed wrote.
//!
//! In WAN mode, [`Report::wan`] holds each host's site and access link
//! counters, and [`Report::lag_by_region`] and [`Report::lag_by_access`]
//! split the lag by the viewers' regions and access links.
//!
//! [`crate::measures`] splits lag and the upload meter's readings over a
//! stream's length and by the hosts' roles, and writes a run's measures as
//! one JSON row.

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use strims_ppspp::scheduler_peer::push::PUSH_OVERHEAD;
use strims_ppspp::scheduler_peer::push::seed::ALLOC_UTILIZATION;
use strims_wan::{Access, EndStats, Region, Site};

use crate::spec::{HostIx, Scenario};
use crate::stats::{Peaks, RunnerReport, SwarmCounters, TransferSample};
use crate::stream::{SeqSet, ViewerStats};

/// Percentiles of a sample (milliseconds, or any `u32` measure): the
/// quantile `q` is the value at index `round((n - 1) * q)` of the sorted
/// sample, 0 for an empty one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Percentiles {
    pub n: usize,
    pub p50: u32,
    pub p99: u32,
    pub max: u32,
}

impl Percentiles {
    pub fn of(values: impl IntoIterator<Item = u32>) -> Percentiles {
        let mut v: Vec<u32> = values.into_iter().collect();
        v.sort_unstable();
        let at = |q: f64| -> u32 {
            if v.is_empty() {
                return 0;
            }
            let i = ((v.len() - 1) as f64 * q).round() as usize;
            v.get(i).copied().unwrap_or(0)
        };
        Percentiles {
            n: v.len(),
            p50: at(0.5),
            p99: at(0.99),
            max: v.last().copied().unwrap_or(0),
        }
    }
}

impl fmt::Display for Percentiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p50 {} p99 {} max {}", self.p50, self.p99, self.max)
    }
}

/// One viewer of one swarm.
#[derive(Clone, Debug)]
pub struct ViewerReport {
    pub host: HostIx,
    pub name: String,
    pub swarm: usize,
    pub stats: ViewerStats,
}

/// One host's totals over all its incarnations.
#[derive(Clone, Debug, Default)]
pub struct HostReport {
    pub name: String,
    /// Bytes written to sockets (upload) and read from them.
    pub uploaded: u64,
    pub downloaded: u64,
    pub transfer_errors: u64,
    pub vpn_frame_errors: u64,
    pub events_dropped: u64,
    pub accept_errors: u64,
    pub restarts: u32,
    /// Node loop wakeups (`DriverMetrics::loop_iterations`).
    pub loop_iterations: u64,
    /// Links at the last sample.
    pub links: usize,
    pub vpn_messages_relayed: u64,
    pub vpn_link_queue_full: u64,
    /// Peer index and hash table records vpn pushed to new links
    /// (`Metrics::records_pushed`).
    pub vpn_records_pushed: u64,
    /// vpn input dropped as malformed, mismatched, unverifiable or
    /// undecryptable.
    pub vpn_bad_input: u64,
    /// The runner, link and network stats (see [`crate::stats`]).
    pub runner: RunnerReport,
    /// With [`Scenario::transfer_samples`], the node's transfers at every
    /// stats sample: (time since the start, scenario swarm, transfers),
    /// peers indexed by host.
    pub transfers: Vec<(std::time::Duration, usize, TransferSample)>,
}

/// vpn probe results (see [`crate::spec::VpnProbes`]).
#[derive(Clone, Debug, Default)]
pub struct VpnReport {
    pub unicast_sent: u64,
    pub unicast_delivered: u64,
    /// Hops of delivered unicasts.
    pub hops: BTreeMap<usize, u64>,
    /// Unicasts between hosts at most `MAX_MESSAGE_HOPS` (5) links apart
    /// in the spec's topology, and how many of them arrived: the share any
    /// routing could deliver (links made by peer exchange aside).
    pub in_reach_sent: u64,
    pub in_reach_delivered: u64,
    pub gets: u64,
    pub gets_ok: u64,
    pub searches: u64,
    pub searches_ok: u64,
    pub exchanges: u64,
    /// Offerer-side outcomes (`ExchangeOutcome` names).
    pub exchange_outcomes: BTreeMap<String, u64>,
    /// (time window label, sent, delivered), for comparing phases.
    pub unicast_by_phase: Vec<(String, u64, u64)>,
    /// Hash table gets and peer index searches by 10 s window of their
    /// start time.
    pub lookups_by_phase: Vec<LookupPhase>,
}

/// Lookups started in one 10 s window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LookupPhase {
    pub label: String,
    pub gets: u64,
    pub gets_ok: u64,
    pub searches: u64,
    pub searches_ok: u64,
}

impl VpnReport {
    pub fn unicast_rate(&self) -> f64 {
        ratio(self.unicast_delivered, self.unicast_sent)
    }

    pub fn in_reach_rate(&self) -> f64 {
        ratio(self.in_reach_delivered, self.in_reach_sent)
    }

    pub fn get_rate(&self) -> f64 {
        ratio(self.gets_ok, self.gets)
    }

    pub fn search_rate(&self) -> f64 {
        ratio(self.searches_ok, self.searches)
    }

    pub fn exchange_linked(&self) -> u64 {
        ["Connected", "RemoteConnected"]
            .iter()
            .map(|k| self.exchange_outcomes.get(*k).copied().unwrap_or(0))
            .sum()
    }
}

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 { 1.0 } else { a as f64 / b as f64 }
}

/// Memory of one run (needs [`crate::memory::Allocator`] as the test
/// binary's global allocator).
#[derive(Clone, Debug, Default)]
pub struct MemoryReport {
    /// Live heap bytes of the run at its peak.
    pub peak: i64,
    /// `(sim ms, live bytes)` at every controller sample.
    pub samples: Vec<(u64, i64)>,
}

impl MemoryReport {
    /// The largest sample in `[from, to)` (ms).
    pub fn max_between(&self, from: u64, to: u64) -> i64 {
        self.samples
            .iter()
            .filter(|(t, _)| (from..to).contains(t))
            .map(|&(_, b)| b)
            .max()
            .unwrap_or(0)
    }
}

/// One host's access link in WAN mode.
#[derive(Clone, Debug)]
pub struct WanHostReport {
    pub name: String,
    pub site: Site,
    pub stats: EndStats,
    /// The uplink queue's wait every 100 ms: (time since the start, wait).
    pub up_queue: Vec<(strims_base::time::Duration, strims_base::time::Duration)>,
    /// With competing uploads
    /// ([`HostSpec::competing`](crate::spec::HostSpec::competing)), the
    /// wire bytes they have sent, every 100 ms: (time since the start,
    /// bytes).
    pub competing: Vec<(strims_base::time::Duration, u64)>,
}

/// The access links of a WAN run, in host order.
#[derive(Clone, Debug, Default)]
pub struct WanReport {
    pub hosts: Vec<WanHostReport>,
}

impl WanReport {
    /// The longest waits for an uplink and for a downlink, with the hosts
    /// that had them.
    pub fn queue_peaks(&self) -> ((u32, &str), (u32, &str)) {
        let peak = |f: fn(&EndStats) -> strims_base::time::Duration| {
            self.hosts
                .iter()
                .map(|h| (ms(f(&h.stats)), h.name.as_str()))
                .max_by_key(|&(d, _)| d)
                .unwrap_or((0, "-"))
        };
        (peak(|s| s.up_queue_max), peak(|s| s.down_queue_max))
    }

    /// Stream messages that lost a packet, and lost datagrams, over every
    /// host.
    pub fn losses(&self) -> (u64, u64) {
        self.hosts.iter().fold((0, 0), |(r, d), h| {
            (r + h.stats.retransmits, d + h.stats.drops)
        })
    }
}

fn ms(d: strims_base::time::Duration) -> u32 {
    u32::try_from(d.as_millis().max(0)).unwrap_or(u32::MAX)
}

/// Everything one run measured.
#[derive(Clone, Debug)]
pub struct Report {
    pub scenario: Scenario,
    pub seed: u64,
    /// SHA-256 of the canonical trace.
    pub trace_hash: String,
    /// The canonical record of the run, one line per event stamped with
    /// simulated milliseconds: node events, dial outcome changes, stream
    /// milestones (writes, first frames, progress every 10 s of stream,
    /// underruns, resets, bad frames), faults, invariant violations, vpn
    /// probes and lookups, and each node's counters and runner state at
    /// every [`Scenario::sample_every`] sample.
    pub trace: Vec<String>,
    pub sim_secs: f64,
    pub wall_secs: f64,
    pub viewers: Vec<ViewerReport>,
    pub hosts: Vec<HostReport>,
    /// Frames each swarm's seed wrote.
    pub written: Vec<SeqSet>,
    /// What each media swarm's seed wrote.
    pub media: Vec<Option<MediaLog>>,
    pub vpn: Option<VpnReport>,
    pub memory: Option<MemoryReport>,
    /// `(ms, what)` for every invariant violation (see
    /// [`crate::Invariants`]).
    pub violations: Vec<(u64, String)>,
    /// The access links, in WAN mode.
    pub wan: Option<WanReport>,
}

/// What a media swarm's seed wrote ([`crate::MediaSpec`]), over its
/// incarnations.
#[derive(Clone, Debug, Default)]
pub struct MediaLog {
    /// Per media chunk, by sequence number: its first byte's time, in ms of
    /// run time.
    pub times: Vec<u64>,
    /// Per media chunk signed: how long after its first byte (ms).
    pub sign_waits: Vec<u32>,
    /// Chunks of padding alone: per flush that made some, its time (ms) and
    /// their number.
    pub pad_chunks: Vec<(u64, u64)>,
    /// Access unit bytes, stream bytes (padding included), and units signed.
    pub media_bytes: u64,
    pub stream_bytes: u64,
    pub units: u64,
    /// Chunks per unit and bytes per chunk.
    pub unit_chunks: u64,
    pub chunk: u64,
    /// The stream offset of each GOP's start, its keyframe's record, as
    /// written.
    pub gops: Vec<u64>,
}

impl Report {
    /// Simulated seconds per wall second.
    pub fn speed(&self) -> f64 {
        if self.wall_secs > 0.0 {
            self.sim_secs / self.wall_secs
        } else {
            0.0
        }
    }

    pub fn viewer(&self, swarm: usize, host: HostIx) -> Option<&ViewerReport> {
        self.viewers
            .iter()
            .find(|v| v.swarm == swarm && v.host == host)
    }

    /// Frames of `swarm` written in `range` that `v` never read.
    pub fn missing(&self, v: &ViewerReport, range: std::ops::Range<u64>) -> u64 {
        self.written
            .get(v.swarm)
            .map_or(0, |w| w.missing_from(&v.stats.received, range))
    }

    /// The lag of every frame read by every viewer of `swarm` (see the
    /// module docs).
    pub fn lag(&self, swarm: usize) -> Percentiles {
        Percentiles::of(
            self.viewers
                .iter()
                .filter(|v| v.swarm == swarm)
                .flat_map(|v| v.stats.lags.iter().map(|&(_, l)| l)),
        )
    }

    /// The lag of every frame read by the viewers of `swarm` among `hosts`.
    pub fn lag_of(&self, swarm: usize, hosts: &[HostIx]) -> Percentiles {
        Percentiles::of(
            self.viewers
                .iter()
                .filter(|v| v.swarm == swarm && hosts.contains(&v.host))
                .flat_map(|v| v.stats.lags.iter().map(|&(_, l)| l)),
        )
    }

    /// The lag of `swarm`'s viewers by the region of their site, for the
    /// regions that have viewers (WAN mode).
    pub fn lag_by_region(&self, swarm: usize) -> Vec<(Region, Percentiles)> {
        let mut by: BTreeMap<Region, Vec<HostIx>> = BTreeMap::new();
        for v in self.viewers.iter().filter(|v| v.swarm == swarm) {
            if let Some(site) = self.scenario.hosts.get(v.host).and_then(|h| h.site) {
                by.entry(site.city.region()).or_default().push(v.host);
            }
        }
        by.into_iter()
            .map(|(region, hosts)| (region, self.lag_of(swarm, &hosts)))
            .collect()
    }

    /// The lag of `swarm`'s viewers by their access link, for the kinds of
    /// access that have viewers (WAN mode).
    pub fn lag_by_access(&self, swarm: usize) -> Vec<(Access, Percentiles)> {
        let mut by: BTreeMap<Access, Vec<HostIx>> = BTreeMap::new();
        for v in self.viewers.iter().filter(|v| v.swarm == swarm) {
            if let Some(site) = self.scenario.hosts.get(v.host).and_then(|h| h.site) {
                by.entry(site.access).or_default().push(v.host);
            }
        }
        by.into_iter()
            .map(|(access, hosts)| (access, self.lag_of(swarm, &hosts)))
            .collect()
    }

    /// Delivered over written frames, summed over `swarm`'s viewers: each
    /// viewer is owed every frame the seed wrote during the run, including
    /// those written while the viewer was down.
    /// A media swarm's viewer is owed the frames from its first one on:
    /// where it joins turns on the unit size, which the startup lag
    /// measures.
    pub fn delivery(&self, swarm: usize) -> f64 {
        let Some(w) = self.written.get(swarm) else {
            return 0.0;
        };
        let media = self.media.get(swarm).is_some_and(Option::is_some);
        let (mut got, mut want) = (0, 0);
        for v in self.viewers.iter().filter(|v| v.swarm == swarm) {
            let from = match media {
                true => v
                    .stats
                    .joins
                    .iter()
                    .find_map(|j| j.first_seq)
                    .unwrap_or(w.bound()),
                false => 0,
            };
            let owed = w.count_in(from..w.bound());
            want += owed;
            got += owed - w.missing_from(&v.stats.received, from..w.bound());
        }
        ratio(got, want)
    }

    /// Upload of the swarm's seed, and the viewers' min, median and max.
    pub fn uploads(&self, swarm: usize) -> UploadDistribution {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return UploadDistribution::default();
        };
        let seed = self.hosts.get(s.seed).map_or(0, |h| h.uploaded);
        let mut v: Vec<u64> = s
            .viewers
            .iter()
            .chain(&s.relays)
            .filter_map(|&h| self.hosts.get(h).map(|r| r.uploaded))
            .collect();
        v.sort_unstable();
        let total: u64 = seed + v.iter().sum::<u64>();
        UploadDistribution {
            seed,
            seed_share: ratio(seed, total),
            min: v.first().copied().unwrap_or(0),
            median: v.get(v.len() / 2).copied().unwrap_or(0),
            max: v.last().copied().unwrap_or(0),
            uploaders: v.iter().filter(|&&b| b >= 64 * 1024).count(),
            gini: gini(&v),
        }
    }

    /// Swarm `swarm`'s runner counters summed over every host.
    pub fn swarm_counters(&self, swarm: usize) -> SwarmCounters {
        let mut c = SwarmCounters::default();
        for h in &self.hosts {
            c.add(&h.runner.swarm(swarm));
        }
        c
    }

    /// The seed's share of the data bytes sent on the swarm's channels,
    /// from the runner stats: ppspp chunk data alone, with no vpn,
    /// handshake or protocol overhead.
    pub fn data_share(&self, swarm: usize) -> f64 {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return 0.0;
        };
        let seed = self
            .hosts
            .get(s.seed)
            .map_or(0, |h| h.runner.swarm(swarm).data_sent);
        ratio(seed, self.swarm_counters(swarm).data_sent)
    }

    /// The seed's distinct chunks over the chunks it sent, from the runner
    /// stats: 1 when it sent each chunk once, 1/k for k copies of each.
    pub fn seed_distinct_ratio(&self, swarm: usize) -> f64 {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return 0.0;
        };
        let c = self
            .hosts
            .get(s.seed)
            .map(|h| h.runner.swarm(swarm))
            .unwrap_or_default();
        ratio(c.distinct_chunks_sent, c.chunks_sent)
    }

    /// Viewers and relays of the swarm that sent at least `min` data bytes
    /// on its channels (runner stats).
    pub fn data_uploaders(&self, swarm: usize, min: u64) -> usize {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return 0;
        };
        s.viewers
            .iter()
            .chain(&s.relays)
            .filter(|&&h| {
                self.hosts
                    .get(h)
                    .is_some_and(|r| r.runner.swarm(swarm).data_sent >= min)
            })
            .count()
    }

    /// Chunks received that their node already had
    /// (`SwarmSchedulerStats::wasted_chunks`), over all chunks received
    /// (data bytes received over the chunk size), summed over the swarm's
    /// hosts.
    pub fn duplicates(&self, swarm: usize) -> f64 {
        let c = self.swarm_counters(swarm);
        let chunk = self
            .scenario
            .swarms
            .get(swarm)
            .map_or(1, |s| s.options.chunk_size.max(1) as u64);
        let received = c.data_received / chunk;
        if received == 0 {
            0.0
        } else {
            c.wasted_chunks as f64 / received as f64
        }
    }

    /// The runner, link and network gauges' extremes over every host.
    pub fn peaks(&self) -> Peaks {
        let mut p = Peaks::default();
        for h in &self.hosts {
            p.merge(&h.runner.peaks);
        }
        p
    }

    /// One line per seed for the sweep's log.
    /// Swarm `swarm`'s data rate in bytes per second, as its seed writes:
    /// a media swarm's padding included.
    pub(crate) fn swarm_rate(&self, swarm: usize) -> f64 {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return 0.0;
        };
        match self.media.get(swarm).and_then(Option::as_ref) {
            Some(m) => {
                m.stream_bytes as f64 / s.stop.saturating_sub(s.start).as_secs_f64().max(1e-9)
            }
            None => s.rate(),
        }
    }

    /// The sequence numbers of swarm `swarm`'s frames written in `[from,
    /// to)` of run time: a media swarm's chunks by their first bytes' time.
    pub fn frames_between(
        &self,
        swarm: usize,
        from: Duration,
        to: Duration,
    ) -> std::ops::Range<u64> {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return 0..0;
        };
        match self.media.get(swarm).and_then(Option::as_ref) {
            Some(m) => {
                let ms = |t: Duration| u64::try_from(t.as_millis()).unwrap_or(u64::MAX);
                let at = |t: Duration| m.times.partition_point(|&x| x < ms(t)) as u64;
                at(from)..at(to)
            }
            None => s.frames_between(from, to),
        }
    }

    /// When frame `seq` of swarm `swarm` was written, in run time.
    pub fn written_at(&self, swarm: usize, seq: u64) -> Duration {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return Duration::ZERO;
        };
        match self.media.get(swarm).and_then(Option::as_ref) {
            Some(m) => Duration::from_millis(
                usize::try_from(seq)
                    .ok()
                    .and_then(|i| m.times.get(i))
                    .copied()
                    .unwrap_or(0),
            ),
            None => {
                let fpw = s.frames_per_write.max(1) as u64;
                s.start + s.write_interval * u32::try_from(seq / fpw).unwrap_or(u32::MAX)
            }
        }
    }

    /// The most host `h` can upload, in bytes per second: the smaller of
    /// its rate limit and its site's uplink. `None` with neither.
    pub fn upload_cap(&self, h: HostIx) -> Option<u64> {
        let spec = self.scenario.hosts.get(h)?;
        let site = spec.site.map(|s| s.profile.up);
        match (spec.uplink, site) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
        .filter(|&c| c > 0)
    }

    /// The most host `h` can upload at `t` (time since the start), in bytes
    /// per second: the smaller of its rate limit and what its site's uplink
    /// has available then: its rate ([`HostSpec::up_steps`]) less what its
    /// competing uploads sent in the second before ([`HostSpec::competing`]).
    /// `None` with neither.
    ///
    /// [`HostSpec::up_steps`]: crate::spec::HostSpec::up_steps
    /// [`HostSpec::competing`]: crate::spec::HostSpec::competing
    pub fn upload_cap_at(&self, h: HostIx, t: std::time::Duration) -> Option<u64> {
        let spec = self.scenario.hosts.get(h)?;
        let site = spec.site.map(|s| {
            spec.up_steps
                .iter()
                .filter(|&&(at, _)| at <= t)
                .max_by_key(|&&(at, _)| at)
                .map_or(s.profile.up, |&(_, rate)| rate)
                .saturating_sub(self.competing_rate(h, t))
        });
        match (spec.uplink, site) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
        .filter(|&c| c > 0)
    }

    /// The wire rate of host `h`'s competing uploads over the second
    /// before `t`, in bytes per second.
    pub fn competing_rate(&self, h: HostIx, t: std::time::Duration) -> u64 {
        let Some(name) = self.scenario.hosts.get(h).map(|s| s.name.as_str()) else {
            return 0;
        };
        let Some(w) = self
            .wan
            .as_ref()
            .and_then(|w| w.hosts.iter().find(|w| w.name == name))
        else {
            return 0;
        };
        let sent = |t: strims_base::time::Duration| {
            let i = w.competing.partition_point(|&(at, _)| at <= t);
            i.checked_sub(1)
                .and_then(|i| w.competing.get(i))
                .map_or(0, |&(_, b)| b)
        };
        let t = strims_base::time::Duration::from(t);
        sent(t).saturating_sub(sent(t - strims_base::time::Duration::from_secs(1)))
    }

    /// The upload meter's readings at `h` while a swarm was live, as
    /// fractions of what the host could upload at each sample
    /// ([`upload_cap_at`](Report::upload_cap_at)):
    /// (capacity ÷ cap, the push rate its subscriptions commit ÷ cap) per
    /// sample. Empty without a cap.
    pub fn meter_readings(&self, h: HostIx) -> Vec<(f64, f64)> {
        self.meter_series(h)
            .into_iter()
            .map(|(_, c, k)| (c, k))
            .collect()
    }

    /// [`meter_readings`](Report::meter_readings) with each sample's time.
    pub fn meter_series(&self, h: HostIx) -> Vec<(std::time::Duration, f64, f64)> {
        let Some(host) = self.hosts.get(h) else {
            return Vec::new();
        };
        host.runner
            .meter
            .iter()
            .filter(|m| self.live(m.t) && m.capacity > 0)
            .filter_map(|m| {
                let cap = self.upload_cap_at(h, m.t)? as f64;
                let committed: f64 = m
                    .children
                    .iter()
                    .map(|(&s, &n)| {
                        let streams = self
                            .scenario
                            .swarms
                            .get(s)
                            .map_or(1, |x| x.options.stream_count.max(1));
                        n as f64 * self.swarm_rate(s) / streams as f64 * PUSH_OVERHEAD
                    })
                    .sum();
                Some((m.t, m.capacity as f64 / cap, committed / cap))
            })
            .collect()
    }

    /// Whether any swarm streams at `t`.
    pub(crate) fn live(&self, t: std::time::Duration) -> bool {
        self.scenario
            .swarms
            .iter()
            .any(|s| t >= s.start && t <= s.stop)
    }

    /// The meter summary: capacity ÷ upload cap at the 10th, 50th and 90th
    /// percentile of the seeds' samples and the viewers', the highest push
    /// commitment ÷ upload cap at any host, and the most samples in a row
    /// any host committed more than [`ALLOC_UTILIZATION`] of its cap.
    fn meter_line(&self) -> Option<String> {
        let seeds: Vec<HostIx> = self.scenario.swarms.iter().map(|s| s.seed).collect();
        let (mut at_seeds, mut at_viewers, mut peak) = (Vec::new(), Vec::new(), (0.0f64, 0));
        let mut over = 0;
        for h in 0..self.hosts.len() {
            let mut run = 0;
            for (c, k) in self.meter_readings(h) {
                run = if k > ALLOC_UTILIZATION { run + 1 } else { 0 };
                over = over.max(run);
                if seeds.contains(&h) {
                    at_seeds.push(c);
                } else {
                    at_viewers.push(c);
                }
                if k > peak.0 {
                    peak = (k, h);
                }
            }
        }
        if at_seeds.is_empty() && at_viewers.is_empty() {
            return None;
        }
        let pct = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            let at = |q: f64| {
                v.get(((v.len() as f64 - 1.0) * q).round() as usize)
                    .copied()
                    .unwrap_or(0.0)
            };
            format!("{:.2}/{:.2}/{:.2}", at(0.1), at(0.5), at(0.9))
        };
        Some(format!(
            "\n  meter: capacity/cap p10/p50/p90 seeds {} viewers {}, peak committed/cap {:.2} at {}, over {ALLOC_UTILIZATION} for {over} samples",
            pct(&mut at_seeds),
            pct(&mut at_viewers),
            peak.0,
            self.hosts.get(peak.1).map_or("-", |h| h.name.as_str()),
        ))
    }

    /// Swarm `swarm`'s adaptive seed: its first wave against what its
    /// upload cap pays for (the window's sizing rule with the true
    /// capacity), and its last budget, window and copies.
    fn first_wave_line(&self, swarm: usize) -> Option<String> {
        let spec = self.scenario.swarms.get(swarm)?;
        let seed = self.hosts.get(spec.seed)?.runner.seeds.get(&swarm)?;
        let cap = self.upload_cap(spec.seed)?;
        let oracle = self.paid_copies(swarm)?;
        Some(format!(
            "\n  swarm {swarm} seed: first wave {} copies from {} KiB/s (cap {} KiB/s pays for {oracle}), budget {} slots, window {}, copies {}",
            seed.first_wave,
            seed.first_wave_capacity / 1024,
            cap / 1024,
            seed.budget,
            seed.window,
            seed.copies,
        ))
    }

    /// The whole copies swarm `swarm`'s seed's starting upload cap pays
    /// for under the adaptive seed's sizing rule (its utilization target
    /// over the stream rate with push overhead), at most one a viewer:
    /// what its first wave should open.
    pub fn paid_copies(&self, swarm: usize) -> Option<usize> {
        let spec = self.scenario.swarms.get(swarm)?;
        let cap = self.upload_cap(spec.seed)?;
        let rate = self.swarm_rate(swarm) * PUSH_OVERHEAD;
        Some(if rate > 0.0 {
            ((cap as f64 * ALLOC_UTILIZATION / rate) as usize).min(spec.viewers.len())
        } else {
            0
        })
    }

    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} seed {}: {:.0} s sim in {:.2} s wall ({:.1}x), trace {} lines {}",
            self.scenario.name,
            self.seed,
            self.sim_secs,
            self.wall_secs,
            self.speed(),
            self.trace.len(),
            self.trace_hash.get(..16).unwrap_or_default(),
        );
        for (i, _) in self.scenario.swarms.iter().enumerate() {
            let u = self.uploads(i);
            s.push_str(&format!(
                "\n  swarm {i}: delivery {:.4}, lag ms {}, upload seed {} KiB ({:.0}%), viewers min/median/max {}/{}/{} KiB, gini {:.2}",
                self.delivery(i),
                self.lag(i),
                u.seed / 1024,
                u.seed_share * 100.0,
                u.min / 1024,
                u.median / 1024,
                u.max / 1024,
                u.gini,
            ));
        }
        for (i, spec) in self.scenario.swarms.iter().enumerate() {
            let c = self.swarm_counters(i);
            s.push_str(&format!(
                "\n  swarm {i} runner: seed data share {:.0}%, seed distinct ratio {:.3}, duplicates {:.2}%, requests {}, haves {}, invalid {}, rejected {}",
                self.data_share(i) * 100.0,
                self.seed_distinct_ratio(i),
                self.duplicates(i) * 100.0,
                c.requests,
                c.haves,
                c.invalid,
                c.rejected,
            ));
            if spec.options.push_mode.resolve() == strims_ppspp::PushMode::Hybrid {
                s.push_str(&format!(
                    "\n  swarm {i} push: hits {} misses {} ({:.1}%), late {}, unsolicited {}, switched {} pruned {} shed {}, accepted {}, sheds sent {}, stale {}, suppressed {}",
                    c.hits,
                    c.misses,
                    c.hit_rate().unwrap_or(0.0) * 100.0,
                    c.late,
                    c.unsolicited,
                    c.switched,
                    c.pruned,
                    c.shed,
                    c.accepted,
                    c.sheds_sent,
                    c.stale_drops,
                    c.suppressed,
                ));
            }
        }
        if let Some(line) = self.meter_line() {
            s.push_str(&line);
        }
        for i in 0..self.scenario.swarms.len() {
            if let Some(line) = self.first_wave_line(i) {
                s.push_str(&line);
            }
        }
        if let Some(line) = self.bucket_line() {
            s.push_str(&line);
        }
        if let Some(line) = self.tracking_line() {
            s.push_str(&line);
        }
        let p = self.peaks();
        if !self.scenario.swarms.is_empty() {
            let drops: u64 = self.hosts.iter().map(|h| h.runner.queue_drops).sum();
            s.push_str(&format!(
                "\n  runner peaks: data queue {}/{} (pushes/requests), drops {drops}, request deadlines {}, cwnd {}–{} KiB, flight {} KiB, cto {} ms, rtt {} ms",
                p.queued_data[0],
                p.queued_data[1],
                p.pending_requests,
                p.cwnd_min.unwrap_or(0) / 1024,
                p.cwnd_max / 1024,
                p.flight_max / 1024,
                p.cto_max_ms,
                p.rtt_max_ms,
            ));
        }
        s.push_str(&format!(
            "\n  link peaks: pending {} KiB, blocked link samples {}, vpn queued {}",
            p.link_pending / 1024,
            p.write_blocked,
            p.vpn_queued,
        ));
        let rejoins: Vec<String> = self
            .viewers
            .iter()
            .flat_map(|v| {
                v.stats.joins.iter().skip(1).map(move |j| {
                    let after = |t: Option<u64>| {
                        t.map_or("never".to_owned(), |t| {
                            format!("{:.1} s", t.saturating_sub(j.start_ms) as f64 / 1000.0)
                        })
                    };
                    format!(
                        "{} first frame {} caught up {}",
                        v.name,
                        after(j.first_frame_ms),
                        after(j.caught_up_ms)
                    )
                })
            })
            .collect();
        if !rejoins.is_empty() {
            s.push_str(&format!("\n  rejoins: {}", rejoins.join("; ")));
        }
        if let Some(v) = &self.vpn {
            s.push_str(&format!(
                "\n  vpn: unicast {}/{} ({:.1}%), in reach {}/{} ({:.1}%), hops {:?}, gets {}/{}, searches {}/{}, exchanges {} {:?}",
                v.unicast_delivered,
                v.unicast_sent,
                v.unicast_rate() * 100.0,
                v.in_reach_delivered,
                v.in_reach_sent,
                v.in_reach_rate() * 100.0,
                v.hops,
                v.gets_ok,
                v.gets,
                v.searches_ok,
                v.searches,
                v.exchanges,
                v.exchange_outcomes,
            ));
            let pushed: u64 = self.hosts.iter().map(|h| h.vpn_records_pushed).sum();
            let bad: u64 = self.hosts.iter().map(|h| h.vpn_bad_input).sum();
            s.push_str(&format!(
                "\n  vpn records pushed to new links {pushed}, bad input {bad}"
            ));
            for (phase, sent, got) in &v.unicast_by_phase {
                s.push_str(&format!(
                    "\n    {phase}: {got}/{sent} ({:.0}%)",
                    ratio(*got, *sent) * 100.0
                ));
            }
            let phases: Vec<String> = v
                .lookups_by_phase
                .iter()
                .map(|p| {
                    format!(
                        "{} {}/{} {}/{}",
                        p.label, p.gets_ok, p.gets, p.searches_ok, p.searches
                    )
                })
                .collect();
            s.push_str(&format!(
                "\n    lookups (gets, searches): {}",
                phases.join("; ")
            ));
        }
        if let Some(wan) = &self.wan {
            for (i, _) in self.scenario.swarms.iter().enumerate() {
                let regions: Vec<String> = self
                    .lag_by_region(i)
                    .iter()
                    .map(|(r, p)| format!("{r} {p}"))
                    .collect();
                let access: Vec<String> = self
                    .lag_by_access(i)
                    .iter()
                    .map(|(a, p)| format!("{a} {p}"))
                    .collect();
                s.push_str(&format!(
                    "\n  swarm {i} lag ms by region: {}\n  swarm {i} lag ms by access: {}",
                    regions.join("; "),
                    access.join("; ")
                ));
            }
            let ((up, up_host), (down, down_host)) = wan.queue_peaks();
            let (retransmits, drops) = wan.losses();
            s.push_str(&format!(
                "\n  wan: uplink queue peak {up} ms ({up_host}), downlink queue peak {down} ms ({down_host}), retransmissions {retransmits}, datagrams lost {drops}"
            ));
        }
        if let Some(m) = &self.memory {
            s.push_str(&format!(
                "\n  memory: peak {} KiB, last sample {} KiB",
                m.peak / 1024,
                m.samples.last().map_or(0, |&(_, b)| b / 1024),
            ));
        }
        if !self.violations.is_empty() {
            s.push_str(&format!(
                "\n  invariant violations: {}",
                self.violations.len()
            ));
        }
        let wake: u64 = self.hosts.iter().map(|h| h.loop_iterations).sum();
        s.push_str(&format!(
            "\n  node loop wakeups: {:.0} per host per sim second",
            wake as f64 / self.hosts.len().max(1) as f64 / self.sim_secs.max(1.0)
        ));
        s
    }
}

/// See [`Report::uploads`]. Bytes written to links over the whole run,
/// vpn, handshakes and protocol overhead included.
#[derive(Clone, Copy, Debug, Default)]
pub struct UploadDistribution {
    pub seed: u64,
    /// The seed's upload over the seed's, viewers' and relays' together.
    pub seed_share: f64,
    pub min: u64,
    pub median: u64,
    pub max: u64,
    /// Viewers and relays that uploaded at least 64 KiB.
    pub uploaders: usize,
    /// Gini coefficient of the viewers' and relays' uploads (0: even,
    /// towards 1: one does all).
    pub gini: f64,
}

fn gini(sorted: &[u64]) -> f64 {
    let n = sorted.len() as f64;
    let total: f64 = sorted.iter().map(|&x| x as f64).sum();
    if sorted.is_empty() || total == 0.0 {
        return 0.0;
    }
    let weighted: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, &x)| (i as f64 + 1.0) * x as f64)
        .sum();
    (2.0 * weighted) / (n * total) - (n + 1.0) / n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_and_gini() {
        let p = Percentiles::of(1..=100);
        assert_eq!((p.p50, p.p99, p.max, p.n), (51, 99, 100, 100));
        assert_eq!(Percentiles::of([]).max, 0);
        assert!(gini(&[5, 5, 5, 5]).abs() < 1e-9);
        assert!(gini(&[0, 0, 0, 100]) > 0.7);
    }
}
