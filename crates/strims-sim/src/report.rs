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
//! In WAN mode, [`Report::wan`] holds each host's site and access link
//! counters, and [`Report::lag_by_region`] and [`Report::lag_by_access`]
//! split the lag by the viewers' regions and access links.

use std::collections::BTreeMap;
use std::fmt;

use strims_wan::{Access, EndStats, Region, Site};

use crate::spec::{HostIx, Scenario};
use crate::stats::{Peaks, RunnerReport, SwarmCounters};
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
    pub vpn: Option<VpnReport>,
    pub memory: Option<MemoryReport>,
    /// `(ms, what)` for every invariant violation (see
    /// [`crate::Invariants`]).
    pub violations: Vec<(u64, String)>,
    /// The access links, in WAN mode.
    pub wan: Option<WanReport>,
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
    pub fn delivery(&self, swarm: usize) -> f64 {
        let Some(w) = self.written.get(swarm) else {
            return 0.0;
        };
        let (mut got, mut want) = (0, 0);
        for v in self.viewers.iter().filter(|v| v.swarm == swarm) {
            want += w.len();
            got += w.len() - w.missing_from(&v.stats.received, 0..w.bound());
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
            if spec.options.push_mode == strims_ppspp::PushMode::Hybrid {
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
