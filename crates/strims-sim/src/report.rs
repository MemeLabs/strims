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

use strims_base::stats::WaitHistogram;
use strims_base::stats::sketch;
use strims_ppspp::scheduler_peer::push::PUSH_OVERHEAD;
use strims_ppspp::scheduler_peer::push::seed::ALLOC_UTILIZATION;
use strims_ppspp::{
    ChannelTotals, HaveRecord, Held, LagForecast, RequestRecord, SeedDecision, SeedSend,
};
use strims_wan::{Access, EndStats, Region, Site};

use crate::spec::{HostIx, Scenario};
use crate::stats::{Peaks, RunnerReport, SwarmCounters, TransferSample};
use crate::stream::{SeqSet, ViewerStats};

/// Percentiles of a sample (milliseconds, or any `u32` measure), read
/// through the sketch ([`sketch::percentile`]: the integer nearest rank,
/// the value's bucket's lower bound), 0 for an empty one; `max` exactly.
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
        let at = |p: u32| sketch::percentile(&v, p).unwrap_or(0);
        Percentiles {
            n: v.len(),
            p50: at(50),
            p99: at(99),
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
    /// At every stats sample (its answer's time since the start), the
    /// hosts the node had linked, every incarnation.
    pub neighbours: Vec<(std::time::Duration, Vec<HostIx>)>,
    /// Links the node made and lost (`NodeEvent::PeerAdded`/`PeerRemoved`).
    pub peers_added: u64,
    pub peers_removed: u64,
    /// How its peer exchanges ended, either side (`ExchangeOutcome` names).
    pub exchange_outcomes: BTreeMap<String, u64>,
    /// Offers to link it refused: its peers' exchanges with it that ended
    /// `Rejected`.
    pub offers_refused: u64,
    /// Offers to link it, answered or refused, in the run's first
    /// `EARLY_OFFERS`.
    pub offers_early: u64,
    /// Its pool's probes whose trials ended, and those whose link stayed
    /// (`pool-probe`).
    pub pool_trials: u64,
    pub pool_trial_wins: u64,
    /// When its pool first resolved a swarm's source (`pool-find`), in ms.
    pub source_resolved_ms: Option<u64>,
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
    /// Its ppspp runners' channel counters (`RunnerMetrics::totals`), and
    /// vnic's frames written, their bodies' bytes (every port's) and its
    /// control frames' bytes, headers included (`HostMetrics`).
    pub ppspp: ChannelTotals,
    pub frames_written: u64,
    pub frame_bytes: u64,
    pub control_bytes: u64,
    /// With [`Scenario::transfer_samples`], the node's transfers at every
    /// stats sample: (its answer's time since the start, scenario swarm,
    /// transfers), peers indexed by host.
    pub transfers: Vec<(std::time::Duration, usize, TransferSample)>,
    /// How far its clock read ahead of true time
    /// ([`Scenario::clock_skew`]).
    pub clock_offset: strims_base::time::Duration,
    /// With [`Scenario::wire_bins`], the node's counters at every edge of
    /// the tables' bins, every incarnation ([`NodeBinRecord`]).
    pub node_bins: Vec<NodeBinRecord>,
    /// With [`Scenario::wire_bins`], its ppspp runner's forecast log, every
    /// incarnation ([`ForecastRecord`]), and the samples its caps left out.
    pub forecasts: Vec<ForecastRecord>,
    pub forecasts_lost: u64,
    /// With the forecast log, every chunk of its first scenario swarm it
    /// first held, every incarnation: the chunk's index and when (true time
    /// from the run's start).
    pub held: Vec<(u64, Duration)>,
    /// With the forecast or the request log, the same chunks with their
    /// timelines ([`ArrivalHostRecord`]), and those the caps left out.
    pub arrivals: Vec<ArrivalHostRecord>,
    pub held_lost: u64,
    /// With [`Scenario::request_logs`], every chunk of its first scenario
    /// swarm a HAVE first announced on a channel ([`HaveHostRecord`]),
    /// every request it made ([`RequestHostRecord`]), and those the caps
    /// left out.
    pub haves: Vec<HaveHostRecord>,
    pub haves_lost: u64,
    pub requests: Vec<RequestHostRecord>,
    pub requests_lost: u64,
    /// With [`Scenario::request_logs`], the seed log of its first scenario
    /// swarm, where it seeds it, every incarnation: its decisions about its
    /// neighbours ([`SeedDecisionHostRecord`]) and what became of each
    /// chunk it pushed or answered ([`SeedSendHostRecord`]), and the
    /// records the runner's caps left out.
    pub seed_decisions: Vec<SeedDecisionHostRecord>,
    pub seed_sends: Vec<SeedSendHostRecord>,
    pub seed_log_lost: u64,
    /// With [`Scenario::wire_bins`], the changes of its parent log
    /// (`NodeConfig::parent_log`, [`Report::parent_changes`]) the runner's
    /// caps left out, every incarnation.
    pub parents_lost: u64,
    /// With [`Scenario::parent_checks`], the tap events its buffers and
    /// decision log left out, of every kind, every incarnation: at most
    /// that many of its [`Report::parent_checks`].
    pub parent_checks_lost: u64,
    /// Its messages with each peer on each scenario swarm, every
    /// incarnation, at its stats samples: (the peer's host, `None` for one
    /// it can't name, the swarm, the counts).
    pub messages: Vec<(Option<HostIx>, usize, crate::stats::PeerMessages)>,
    /// Its answers to its peers' path probes, and its reads of theirs.
    pub echoes: crate::stats::Echoes,
    /// Where it seeds a hybrid swarm, its allocation at every stats sample,
    /// every incarnation: (its answer's time since the start, scenario
    /// swarm, sample).
    pub seed_allocs: Vec<(std::time::Duration, usize, crate::seed_alloc::AllocSample)>,
    /// The host as a testbed run's settings and links give it (the
    /// `hosts` table's row, [`crate::testbed`]); `None` for a sim run's,
    /// which its scenario gives ([`Report::host_rows`]).
    pub host_row: Option<HostRow>,
    /// With [`Scenario::timer_log`], its timers that fired, every
    /// incarnation, in the order they fired ([`TimerHostRecord`]), where
    /// they ran (`None` without the log), and those its log's cap left
    /// out.
    pub timers: Vec<TimerHostRecord>,
    pub timer_context: Option<strims_testbed::record::TimerContext>,
    pub timers_lost: u64,
    /// With [`Scenario::link_log`], its links' writes, reads and readings,
    /// every incarnation, in the order its log made them
    /// ([`LinkHostRows`]), and the records its log's caps left out.
    pub link_rows: LinkHostRows,
    pub link_log_lost: u64,
}

/// A link of a host's in its link log's rows ([`HostReport::link_rows`]):
/// the host at its other end, the link's ordinal among the host's links to
/// it, and its transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkHostKey {
    pub peer: HostIx,
    pub link: u32,
    pub kind: strims_node::LinkKind,
}

/// A host's link log's rows ([`HostReport::link_rows`],
/// `strims_testbed::record::links`), each with its link, times in µs of
/// true time from the run's start ([`link_us`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkHostRows {
    pub writes: Vec<(LinkHostKey, strims_testbed::record::links::LinkWrite)>,
    pub reads: Vec<(LinkHostKey, strims_testbed::record::links::LinkRead)>,
    pub samples: Vec<(LinkHostKey, LinkSampleHostRecord)>,
    /// The epoch of the rows' times: true time at the run's start, by the
    /// clock the log read.
    pub epoch: Option<strims_base::time::Time>,
}

impl LinkHostRows {
    /// Adds a take's rows, each link's key from `key` (rows of a link it
    /// doesn't know are left out), their times from `epoch`.
    pub fn extend(
        &mut self,
        writes: &[strims_testbed::record::links::LinkWrite],
        reads: &[strims_testbed::record::links::LinkRead],
        samples: &[strims_testbed::record::links::LinkSample],
        key: impl Fn(u64) -> Option<LinkHostKey>,
        epoch: strims_base::time::Time,
    ) {
        self.epoch = Some(epoch);
        self.writes
            .extend(writes.iter().filter_map(|w| Some((key(w.link)?, *w))));
        self.reads
            .extend(reads.iter().filter_map(|r| Some((key(r.link)?, *r))));
        self.samples.extend(
            samples
                .iter()
                .filter_map(|s| Some((key(s.link)?, LinkSampleHostRecord::of(s)))),
        );
    }

    /// `t` in µs of true time from the run's start.
    pub fn us(&self, t: strims_base::time::Time) -> i64 {
        link_us(t, self.epoch.unwrap_or(t))
    }
}

/// A link sample as a host's report keeps it ([`LinkHostRows::samples`]),
/// its reading packed: each of [`LinkState`]'s readings in bytes, bytes a
/// second or µs, at most `u32::MAX - 1`, and `u32::MAX` for none, so a
/// run's millions of samples hold a quarter of what they would.
///
/// [`LinkState`]: strims_node::event::LinkState
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkSampleHostRecord {
    pub at: strims_base::time::Time,
    pub source: strims_node::event::SampleSource,
    pub cause: strims_testbed::record::links::SampleCause,
    state: [u32; 14],
}

impl LinkSampleHostRecord {
    pub fn of(s: &strims_testbed::record::links::LinkSample) -> Self {
        let n = |v: Option<u64>| v.map_or(u32::MAX, |v| u32::try_from(v).unwrap_or(u32::MAX - 1));
        let d = |v: Option<strims_base::time::Duration>| {
            n(v.map(|d| u64::try_from(d.as_nanos().div_euclid(1000)).unwrap_or(0)))
        };
        let x = &s.state;
        LinkSampleHostRecord {
            at: s.at,
            source: s.source,
            cause: s.cause,
            state: [
                n(x.unsent),
                n(x.unacked),
                n(x.cwnd),
                n(x.ssthresh),
                d(x.srtt),
                d(x.rttvar),
                n(x.retrans),
                n(x.delivery_rate),
                n(x.buffered),
                n(x.lowat),
                d(x.uplink_queue),
                n(x.link_pending),
                n(x.write_held),
                n(x.qos_backlog),
            ],
        }
    }

    /// The readings, in the `link_samples` table's order (unsent,
    /// unacked, cwnd, ssthresh, srtt_us, rttvar_us, retrans,
    /// delivery_rate, buffered, lowat, uplink_queue_us, link_pending,
    /// write_held, qos_backlog), `None` for none.
    pub fn readings(&self) -> [Option<u32>; 14] {
        self.state.map(|v| (v != u32::MAX).then_some(v))
    }
}

/// `t` in µs from `epoch`, floored.
pub fn link_us(t: strims_base::time::Time, epoch: strims_base::time::Time) -> i64 {
    (t - epoch).as_nanos().div_euclid(1000)
}

/// A timer of a host's that fired ([`HostReport::timers`],
/// `strims_node::event::TimerRecord`): its times in µs of true time from the
/// run's start, and the delay it was armed for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerHostRecord {
    pub source: strims_node::event::TimerSource,
    pub armed_us: i64,
    pub deadline_us: i64,
    pub fired_us: i64,
    pub requested_us: i64,
}

impl TimerHostRecord {
    /// Record `r` of a node whose clock read `epoch` at the run's start.
    pub fn of(r: strims_node::event::TimerRecord, epoch: strims_base::time::Time) -> Self {
        let us = |t: strims_base::time::Time| (t - epoch).as_nanos().div_euclid(1000);
        TimerHostRecord {
            source: r.source,
            armed_us: us(r.armed),
            deadline_us: us(r.deadline),
            fired_us: us(r.fired),
            requested_us: r.requested.as_nanos().div_euclid(1000),
        }
    }
}

/// A decision of a seed's ([`HostReport::seed_decisions`]): when (true
/// time from the run's start), the other host, and the record, its times by
/// the node's clock: [`Self::true_time`] reads them as true time.
#[derive(Clone, Copy, Debug)]
pub struct SeedDecisionHostRecord {
    pub at: Duration,
    pub peer: Option<HostIx>,
    /// The node clock's reading at the run's start.
    pub epoch: strims_base::time::Time,
    pub record: SeedDecision,
}

impl SeedDecisionHostRecord {
    /// Decision `record` of a node whose clock read `epoch` at the run's
    /// start, about host `peer`.
    pub fn of(record: SeedDecision, peer: Option<HostIx>, epoch: strims_base::time::Time) -> Self {
        SeedDecisionHostRecord {
            at: true_time(epoch, record.at),
            peer,
            epoch,
            record,
        }
    }

    /// The node's clock reading `t`, as true time from the run's start.
    pub fn true_time(&self, t: strims_base::time::Time) -> Duration {
        true_time(self.epoch, t)
    }
}

/// A node clock's reading `t`, where it read `epoch` at the run's start, as
/// true time from the run's start (none before it).
fn true_time(epoch: strims_base::time::Time, t: strims_base::time::Time) -> Duration {
    (t - epoch).to_std().unwrap_or_default()
}

/// A chunk a seed pushed or answered ([`HostReport::seed_sends`]): when it
/// was queued (an answer's request came) and when its send ended (true
/// time from the run's start), the peer's host, and the record, its times
/// by the node's clock: [`Self::true_time`] reads them as true time.
#[derive(Clone, Copy, Debug)]
pub struct SeedSendHostRecord {
    pub queued: Duration,
    pub ended: Duration,
    pub peer: Option<HostIx>,
    /// The node clock's reading at the run's start.
    pub epoch: strims_base::time::Time,
    pub record: SeedSend,
}

impl SeedSendHostRecord {
    /// Send `record` of a node whose clock read `epoch` at the run's start,
    /// to host `peer`.
    pub fn of(record: SeedSend, peer: Option<HostIx>, epoch: strims_base::time::Time) -> Self {
        SeedSendHostRecord {
            queued: true_time(epoch, record.queued),
            ended: true_time(epoch, record.ended),
            peer,
            epoch,
            record,
        }
    }

    /// The node's clock reading `t`, as true time from the run's start.
    pub fn true_time(&self, t: strims_base::time::Time) -> Duration {
        true_time(self.epoch, t)
    }
}

/// A chunk a host first held ([`HostReport::arrivals`]): its times as true
/// time, the hosts it names, and the record, its times by the host's
/// clock.
#[derive(Clone, Copy, Debug)]
pub struct ArrivalHostRecord {
    /// When the host first held the chunk, and when push expected it, it
    /// was released to pull, we first and last asked for it (true time
    /// from the run's start).
    pub at: Duration,
    pub expected: Option<Duration>,
    pub released: Option<Duration>,
    pub asked: Option<Duration>,
    pub last_ask: Option<Duration>,
    /// The source's host, the stream's parent's then, and the host last
    /// asked.
    pub peer: Option<HostIx>,
    pub parent: Option<HostIx>,
    pub asked_peer: Option<HostIx>,
    pub held: Held,
}

impl ArrivalHostRecord {
    /// Chunk `held` of a node whose clock read `epoch` at the run's start,
    /// its source's host, the stream's parent's then and the host last
    /// asked.
    pub fn of(
        held: Held,
        peer: Option<HostIx>,
        parent: Option<HostIx>,
        asked_peer: Option<HostIx>,
        epoch: strims_base::time::Time,
    ) -> Self {
        let t = |t| true_time(epoch, t);
        ArrivalHostRecord {
            at: t(held.at),
            expected: held.expected.map(t),
            released: held.released.map(t),
            asked: held.asked.map(t),
            last_ask: held.last_ask.map(|(_, x)| t(x)),
            peer,
            parent,
            asked_peer,
            held,
        }
    }
}

/// A chunk a HAVE announced to a host ([`HostReport::haves`]): when the
/// HAVE arrived and when its channel opened (true time from the run's
/// start), the announcing host, and the record, its times by the host's
/// clock.
#[derive(Clone, Copy, Debug)]
pub struct HaveHostRecord {
    pub at: Duration,
    pub opened: Duration,
    pub peer: Option<HostIx>,
    pub have: HaveRecord,
}

impl HaveHostRecord {
    /// HAVE `have` to a node whose clock read `epoch` at the run's start,
    /// from host `peer`.
    pub fn of(have: HaveRecord, peer: Option<HostIx>, epoch: strims_base::time::Time) -> Self {
        HaveHostRecord {
            at: true_time(epoch, have.at),
            opened: true_time(epoch, have.channel_open),
            peer,
            have,
        }
    }
}

/// A request a host made ([`HostReport::requests`]): the record, its times
/// by the host's clock, with that clock's epoch (true time from the run's
/// start is `t - epoch`), and the asked host and the rivals', in the
/// record's order.
#[derive(Clone, Debug)]
pub struct RequestHostRecord {
    pub epoch: strims_base::time::Time,
    pub peer: Option<HostIx>,
    pub rivals: Vec<Option<HostIx>>,
    pub record: RequestRecord,
}

/// A node's counters at an edge of the tables' bins
/// ([`HostReport::node_bins`], `strims_node::event::NodeBin`), for its first
/// scenario swarm.
#[derive(Clone, Debug)]
pub struct NodeBinRecord {
    /// The edge's number, from the swarm's start.
    pub bin: u64,
    /// Its incarnation: the counters restart with each.
    pub incarnation: u32,
    /// [`NODE_BIN_COUNTERS`]' values so far.
    pub counters: [u64; NODE_BIN_COUNTERS.len()],
    /// Its push and answer waits so far.
    pub waits: [WaitHistogram; 2],
}

/// [`NodeBinRecord::counters`]' names.
pub use strims_testbed::record::NODE_BIN_COUNTERS;

/// A forecast log sample of a node's ([`HostReport::forecasts`]): when it
/// was taken and when its channel opened (true time from the run's start),
/// the host it sampled, and the sample, its times by the node's clock.
#[derive(Clone, Copy, Debug)]
pub struct ForecastRecord {
    pub at: Duration,
    pub opened: Duration,
    pub peer: Option<HostIx>,
    pub forecast: LagForecast,
}

impl ForecastRecord {
    /// Sample `forecast` of a node whose clock read `epoch` at the run's
    /// start, of host `peer`.
    pub fn of(forecast: LagForecast, peer: Option<HostIx>, epoch: strims_base::time::Time) -> Self {
        ForecastRecord {
            at: true_time(epoch, forecast.at),
            opened: true_time(epoch, forecast.channel_open),
            peer,
            forecast,
        }
    }
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
/// binary's global allocator): its nodes' and network's, which leaves out
/// the harness's records of the run (trace, samples, viewer records).
#[derive(Clone, Debug, Default)]
pub struct MemoryReport {
    /// Live heap bytes of the run at its peak.
    pub peak: i64,
    /// Live heap bytes of the harness's records at their peak.
    pub records_peak: i64,
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

/// Why the window model's TCP receivers acknowledged data
/// ([`WanHostReport::ack_whys`]), each acknowledgement once: at once (more
/// than a segment unacknowledged, or data out of order), in quickack mode,
/// as the reader drained a short segment, on the delayed acknowledgement's
/// timer, or carried by data the receiver sent back.
pub const ACK_WHYS: [&str; 5] = ["now", "quick", "read", "timer", "data"];

/// The waits of a host's delayed acknowledgements, from the first byte
/// they acknowledge reaching the receiver to the acknowledgement leaving
/// it, in whole milliseconds to [`AckWaits::LONGEST_MS`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AckWaits {
    /// How many waited each whole millisecond, the last bin the longer.
    bins: Vec<u64>,
    sum_us: u64,
}

impl AckWaits {
    pub const LONGEST_MS: usize = 1000;

    pub fn add(&mut self, wait: strims_base::time::Duration) {
        let us = u64::try_from(wait.as_micros()).unwrap_or(0);
        let bin = usize::try_from(us / 1000)
            .unwrap_or(usize::MAX)
            .min(Self::LONGEST_MS);
        if self.bins.len() <= bin {
            self.bins.resize(bin + 1, 0);
        }
        if let Some(n) = self.bins.get_mut(bin) {
            *n += 1;
        }
        self.sum_us += us;
    }

    pub fn merge(&mut self, other: &AckWaits) {
        if self.bins.len() < other.bins.len() {
            self.bins.resize(other.bins.len(), 0);
        }
        for (a, b) in self.bins.iter_mut().zip(&other.bins) {
            *a += b;
        }
        self.sum_us += other.sum_us;
    }

    pub fn count(&self) -> u64 {
        self.bins.iter().sum()
    }

    /// The mean wait (ms), if any waited.
    pub fn mean_ms(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| self.sum_us as f64 / 1000.0 / n as f64)
    }

    /// The `p`th percentile of the waits, to the millisecond below, read
    /// through the sketch ([`sketch`]: the integer nearest rank, the whole
    /// milliseconds' bucket's lower bound).
    pub fn percentile_ms(&self, p: u32) -> Option<u64> {
        let ms = sketch::ranked(self.bins.iter().copied().enumerate(), self.count(), p)?;
        Some(sketch::floor_u64(ms as u64))
    }
}

/// A host's data channels' associations' events by peer and link
/// (`strims_testbed::sctp`, the `sctp_events` table).
pub type SctpAssocs = BTreeMap<(HostIx, u32), Vec<strims_testbed::sctp::SctpEvent>>;

/// One host's access link in WAN mode.
#[derive(Clone, Debug)]
pub struct WanHostReport {
    pub name: String,
    pub site: Site,
    pub stats: EndStats,
    /// The uplink queue's wait every 100 ms: (time since the start, wait).
    /// The sim's uplink model measures it; a testbed run derives it from its
    /// tbf's backlog over the tbf's nominal rate ([`crate::testbed`]).
    pub up_queue: Vec<(strims_base::time::Duration, strims_base::time::Duration)>,
    /// The uplink queue's bytes at the same samples: (time since the start,
    /// bytes). A testbed run measures them (its tbf's backlog); the sim
    /// derives them as the wait times the uplink's rate then.
    pub up_backlog: Vec<(strims_base::time::Duration, u64)>,
    /// Past this time the uplink queue is unmeasured (a testbed run's tbf
    /// sampler that stopped: its last sample and one interval more); `None`
    /// sampled to the end, as a sim's.
    pub up_queue_until: Option<strims_base::time::Duration>,
    /// With competing uploads
    /// ([`HostSpec::competing`](crate::spec::HostSpec::competing)), the
    /// wire bytes they have sent, every 100 ms: (time since the start,
    /// bytes).
    pub competing: Vec<(strims_base::time::Duration, u64)>,
    /// The same after each walk of the run that moved it, stamped with the
    /// walk's time (a testbed run's after each of their packets, stamped
    /// with its capture time): the truth's competing uploads
    /// (`competing_before`).
    pub competing_log: Vec<(strims_base::time::Duration, u64)>,
    /// With the window model, the segments its connections lost to a full
    /// access queue, up or down, and the pure acknowledgements (SACKs alone
    /// on a data channel) its uplink carried, with the bytes its link
    /// charged them (`Network::up_charge`): they take the uplink's time but
    /// aren't in `stats.up_bytes`, which counts what the link charged the
    /// rest. The tables'
    /// bins count the two kinds apart ([`crate::tables::ACK_COLS`]).
    pub queue_drops: u64,
    /// The packets of those segments, as a queue of frames (Linux's tbf
    /// behind a device that segments) counts its drops.
    pub queue_drop_packets: u64,
    pub acks: u64,
    pub ack_bytes: u64,
    /// With the window model, the acknowledgements its TCP receivers sent
    /// by why they sent them ([`ACK_WHYS`]), pure ones and those its data
    /// carried, and the delayed ones' waits.
    pub ack_whys: [u64; ACK_WHYS.len()],
    pub ack_waits: AckWaits,
    /// The bytes it handed its transports that went out, resends left
    /// out: `stats.up_bytes` less these are the transports' framing and
    /// resends. A str0m datagram's payload counts whole, its DTLS and SCTP
    /// framing included.
    pub app_bytes: u64,
    /// With competing uploads, their segments that lost a frame to the full
    /// uplink, and the frames they lost, and their windows summed every
    /// 100 ms while they run.
    pub competing_drops: u64,
    pub competing_drop_packets: u64,
    pub competing_cwnd: Vec<(strims_base::time::Duration, u64)>,
    /// With [`Scenario::packet_log`] naming it, every packet it offered
    /// its uplink, in the order the uplink's queue took them (the
    /// `packets` table): a testbed run's from its captures.
    pub packets: Vec<PacketRecord>,
    /// With [`Scenario::sctp_log`] naming it, its data channels'
    /// associations' events by peer and link, each association's in time
    /// order (the `sctp_events` table): a testbed run's from its dcSCTP's
    /// packet log.
    pub sctp: SctpAssocs,
    /// With [`Scenario::node_bins`], the data it offered its uplink's
    /// queue, before the queue dropped any (the `node_bins` table's
    /// `offered_*`): each offer's time (since the start), IP bytes and
    /// packets, in the order offered; `None` unmeasured (a testbed node
    /// without an offered capture, a run without node bins).
    pub offers: Option<Vec<(strims_base::time::Duration, u64, u32)>>,
}

/// What a packet a host offered its uplink was ([`PacketRecord`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketKind {
    /// A TCP segment with payload, or a datagram with data.
    Data,
    /// A pure TCP acknowledgement.
    Ack,
    /// An SCTP SACK alone.
    Sack,
    /// An SCTP packet of DATA with a SACK bundled.
    DataSack,
    /// A SYN, FIN or RST, a STUN or DTLS handshake datagram.
    Control,
    /// A competing upload's, to its sink.
    Competing,
}

impl PacketKind {
    /// Its name in the `packets` table.
    pub const fn name(self) -> &'static str {
        match self {
            PacketKind::Data => "data",
            PacketKind::Ack => "ack",
            PacketKind::Sack => "sack",
            PacketKind::DataSack => "data_sack",
            PacketKind::Control => "control",
            PacketKind::Competing => "competing",
        }
    }
}

/// A packet's transport ([`PacketRecord`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketProto {
    Tcp,
    Udp,
    Other,
}

impl PacketProto {
    /// Its name in the `packets` table.
    pub const fn name(self) -> &'static str {
        match self {
            PacketProto::Tcp => "tcp",
            PacketProto::Udp => "udp",
            PacketProto::Other => "other",
        }
    }
}

/// Where a packet was lost ([`PacketRecord`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketLoss {
    /// Its uplink's queue had no room for it.
    Queue,
    /// The path between the nodes lost it.
    Path,
}

impl PacketLoss {
    /// Its name in the `packets` table.
    pub const fn name(self) -> &'static str {
        match self {
            PacketLoss::Queue => "queue",
            PacketLoss::Path => "path",
        }
    }
}

/// A packet a host offered its uplink ([`WanHostReport::packets`]): a row
/// of the `packets` table, whose columns' docs are fields.toml's. Times
/// are true time from the run's start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketRecord {
    /// When it entered the uplink's queue, when its last bit left the
    /// uplink, and when it reached its receiver's access link.
    pub at: strims_base::time::Duration,
    pub sent: Option<strims_base::time::Duration>,
    pub arrived: Option<strims_base::time::Duration>,
    pub lost: Option<PacketLoss>,
    /// The host it went to, and the ordinal of the sender's link to it.
    pub peer: Option<HostIx>,
    pub link: Option<u32>,
    pub proto: PacketProto,
    pub ip_bytes: u32,
    /// What the shaper charged it.
    pub charged_bytes: Option<u32>,
    pub kind: PacketKind,
    /// The byte range of its connection's stream it carried, from the
    /// connection's first byte.
    pub range: Option<(u64, u64)>,
    pub resent: Option<bool>,
    /// Its kind read from its length, its payload hidden.
    pub estimated: bool,
    /// What the uplink held as it entered, in the bytes its link charges.
    pub backlog_bytes: Option<u64>,
}

/// A host as the `hosts` table has it, beside its name and number
/// ([`HostReport::host_row`], [`Report::host_rows`]); the columns' docs
/// are fields.toml's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostRow {
    pub role: &'static str,
    pub kind: &'static str,
    pub stack: String,
    pub link_spec: Option<String>,
    /// Its uplink's rate at the start, bytes per second.
    pub up_bps: Option<u64>,
    /// Its egress delay, its uplink queue's bound (in transmission time at
    /// its rate) and its clock's offset, µs.
    pub delay_us: Option<i64>,
    pub queue_us: Option<i64>,
    pub clock_us: i64,
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
    /// With [`Scenario::wire_bins`] in WAN mode, every host's uplink
    /// counters ([`crate::tables::UP_COUNTERS`]) at each bin edge from the
    /// first swarm's start.
    pub wire_bins: Vec<Vec<[u64; crate::wan::UP_COUNTERS.len()]>>,
    /// The indexes of the uplink counters in `wire_bins` its network didn't
    /// measure, which the tables leave empty: none of a sim's, those a
    /// testbed run's captures can't tell ([`crate::testbed`]).
    pub wire_unmeasured: Vec<usize>,
    /// The sim its tables say wrote them (`series_meta.json`'s `sim`):
    /// strims-sim's runs', a testbed run's [`crate::testbed::SIM`].
    pub sim: &'static str,
    /// Where a (host, uplink counter) of `wire_bins` stopped being measured
    /// before the run's end: its last edge (a testbed run's tbf sampler that
    /// stopped). Past it the tables leave its bins empty, not flat. None of
    /// a sim's.
    pub wire_ends: std::collections::BTreeMap<(HostIx, usize), usize>,
    /// A testbed run's TCP sockets, sample by sample ([`SocketSample`]):
    /// empty in a sim run, whose tables then hold no `sockets`.
    pub sockets: Vec<SocketSample>,
    /// With [`Scenario::departures`], every viewer's push parent that
    /// departed, in departure order.
    pub parent_losses: Vec<ParentLoss>,
    /// With [`Scenario::wire_bins`] (the tables), every change of a node's
    /// push parent of a stream of the first swarm, in time order: the
    /// nodes' parent logs (`NodeConfig::parent_log`), each change at the
    /// end of the node's pass that made it, a pass's in the order the node
    /// made them.
    pub parent_changes: Vec<ParentRecord>,
    /// With [`Scenario::parent_checks`] (and the tables), every parent
    /// check of a stream of the first swarm that did something, at every
    /// node, in time order (nodes' and streams' at one time in their
    /// order): the nodes' decision logs (`NodeConfig::decision_log`), each
    /// check at the end of the node's pass that made it.
    pub parent_checks: Vec<ParentCheckRecord>,
    /// With [`Scenario::loops`], how push loops formed and the grafts and
    /// switches asked ([`crate::loops`]).
    pub loops: Option<crate::loops::LoopReport>,
    /// The faults the run skipped, by their index in `scenario.faults`
    /// (in time order, as the run applies them; faults at one time in
    /// the order the scenario gave them): a crash of a host before it
    /// arrived, after it departed or while it is down from an earlier
    /// crash; a restart of a host that isn't down (one that isn't there,
    /// never crashed, or is up again); a host's second departure; a
    /// [`Fault::LatencyFrom`](crate::Fault::LatencyFrom) without the WAN
    /// model; and every fault at or past the run's end, which it never
    /// reached. The trace names each but the last; the tables read only
    /// the applied ones ([`Report::applied_faults`]).
    pub skipped_faults: Vec<usize>,
}

/// One TCP socket of a testbed node at one `ss` sample
/// ([`Report::sockets`], the `sockets` table): when (run time), the node and
/// the node at the other end (`None`: an address that isn't a node's), and
/// what `ss -tin` reports of it, each `None` where it leaves the field out:
/// round trips in µs, windows in segments, bytes and segments cumulative,
/// rates in bytes per second (ss's bits per second over 8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SocketSample {
    pub t: Duration,
    pub node: HostIx,
    pub peer: Option<HostIx>,
    pub rtt_us: Option<u64>,
    pub rttvar_us: Option<u64>,
    pub min_rtt_us: Option<u64>,
    pub cwnd_segments: Option<u64>,
    pub ssthresh_segments: Option<u64>,
    pub bytes_sent: Option<u64>,
    pub bytes_acked: Option<u64>,
    pub bytes_received: Option<u64>,
    pub notsent_bytes: Option<u64>,
    pub segs_out: Option<u64>,
    pub segs_in: Option<u64>,
    pub data_segs_out: Option<u64>,
    pub data_segs_in: Option<u64>,
    pub retrans_segments: Option<u64>,
    pub delivery_rate_bps: Option<u64>,
    pub pacing_rate_bps: Option<u64>,
}

/// A change of `node`'s push parent of `stream` ([`Report::parent_changes`]):
/// when, in run time, the parent from then on (`None` for none), and why.
/// A node's own stop without closing its links (a crash, an abrupt
/// departure) loses the parents it had ([`ParentWhy::Lost`]).
///
/// [`ParentWhy::Lost`]: strims_ppspp::ParentWhy::Lost
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentRecord {
    pub at: Duration,
    pub node: HostIx,
    pub stream: u16,
    pub parent: Option<HostIx>,
    pub why: strims_ppspp::ParentWhy,
}

/// A parent check of `node`'s ([`Report::parent_checks`], the tap's
/// `ParentChoice`): when, in run time, its stream and what it did; its
/// best candidate and runner-up and their mean announcement lags, and the
/// stream's open parent and its lags (hosts `None` for none, or one the
/// node couldn't name; lags `None` unmeasured); and the neighbours it
/// passed over and why, and its votes, as `ParentChoice` counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentCheckRecord {
    pub at: Duration,
    pub node: HostIx,
    pub stream: u16,
    pub action: strims_ppspp::ParentAction,
    pub best: Option<HostIx>,
    pub best_lag: Option<strims_base::time::Duration>,
    pub runner_up: Option<HostIx>,
    pub runner_up_lag: Option<strims_base::time::Duration>,
    pub parent: Option<HostIx>,
    pub parent_have: Option<strims_base::time::Duration>,
    pub parent_push: Option<strims_base::time::Duration>,
    pub unready: u64,
    pub subscriber: u64,
    pub backed_off: u64,
    pub stale: u64,
    pub behind: u64,
    pub votes: u64,
}

impl ParentCheckRecord {
    /// The check `choice` of `stream` at `node`, at `at`, its hosts as
    /// named.
    pub fn of(
        at: Duration,
        node: HostIx,
        stream: u16,
        c: &strims_ppspp::ParentChoice,
        [best, runner_up, parent]: [Option<HostIx>; 3],
    ) -> ParentCheckRecord {
        ParentCheckRecord {
            at,
            node,
            stream,
            action: c.action,
            best,
            best_lag: c.best.map(|b| b.1),
            runner_up,
            runner_up_lag: c.runner_up.map(|r| r.1),
            parent,
            parent_have: c.parent_have,
            parent_push: c.parent_push,
            unready: c.unready,
            subscriber: c.subscriber,
            backed_off: c.backed_off,
            stale: c.stale,
            behind: c.behind,
            votes: c.votes,
        }
    }
}

/// A host that departed ([`crate::Fault::Depart`]) while it was a push
/// parent of one of `viewer`'s streams of the first swarm, at the viewer's
/// last stats sample before: when it left, how many streams it parented,
/// and when the viewer stopped counting on it (its link to it closed, or a
/// stats sample named it parent of no stream), in ms of run time; `None`
/// if it never did (it left first, or the run ended).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentLoss {
    pub viewer: HostIx,
    pub parent: HostIx,
    pub departed_ms: u64,
    pub noticed_ms: Option<u64>,
    pub streams: usize,
}

/// What a media swarm's seed wrote ([`crate::MediaSpec`]), over its
/// incarnations.
#[derive(Clone, Debug, Default)]
pub struct MediaLog {
    /// Per media chunk, by sequence number: its first byte's time, in ms of
    /// run time.
    pub times: Vec<u64>,
    /// Per media chunk, by sequence number: when its unit was signed, in ms
    /// of run time; `None` until then, and for good for a chunk its writer
    /// reset before.
    pub signed: Vec<Option<u64>>,
    /// Per media chunk signed: how long after its first byte (ms).
    pub sign_waits: Vec<u32>,
    /// Access unit bytes, stream bytes sent (records and chunk headers:
    /// the zeros of a unit signed early go nowhere), and units signed.
    pub media_bytes: u64,
    pub sent_bytes: u64,
    pub units: u64,
    /// Chunks per unit and bytes per chunk.
    pub unit_chunks: u64,
    pub chunk: u64,
    /// The stream offset of each GOP's start, its keyframe's record, as
    /// written.
    pub gops: Vec<u64>,
}

impl Report {
    /// Each host's row of the `hosts` table, in host order: the one a
    /// testbed run's settings give it ([`HostReport::host_row`]), else its
    /// scenario's ([`Report::scenario_host_row`]).
    pub fn host_rows(&self) -> Vec<HostRow> {
        (0..self.scenario.hosts.len())
            .map(|h| {
                self.hosts
                    .get(h)
                    .and_then(|x| x.host_row.clone())
                    .unwrap_or_else(|| self.scenario_host_row(h))
            })
            .collect()
    }

    /// Host `h` as its scenario has it: its role (a swarm's seed or viewer,
    /// the signalling relay), the browser whose data channels it models
    /// (the window model's, under its stack's profile; str0m over UDP
    /// without them), its testbed link spec, its site's uplink, the
    /// egress delay its latency faults at time 0 agree on, the window
    /// model's queue, and its clock's offset.
    pub fn scenario_host_row(&self, h: HostIx) -> HostRow {
        let s = &self.scenario;
        let Some(hs) = s.hosts.get(h) else {
            return HostRow::default();
        };
        let role = if s.swarms.iter().any(|w| w.seed == h) {
            "seed"
        } else if s.swarms.iter().any(|w| w.viewers.contains(&h)) {
            "viewer"
        } else if hs.class == crate::spec::HostClass::Infra {
            "relay"
        } else {
            "other"
        };
        let window = s.send_buffer.is_some_and(|b| b.window);
        let kind = crate::late::HostKind::of_host(s, h).name();
        let stack = match hs.rtc_stack.filter(|_| window && hs.webrtc) {
            Some(p) if p == crate::wan::SctpProfile::CHROMIUM => "model-sctp-chromium",
            Some(p) if p == crate::wan::SctpProfile::FIREFOX => "model-sctp-firefox",
            Some(_) => "model-sctp-str0m",
            None if hs.webrtc => "str0m",
            None => "model-tcp",
        };
        let delay = s.egress_delay(h);
        let us = |d: Duration| i64::try_from(d.as_micros()).unwrap_or(i64::MAX);
        HostRow {
            role,
            kind,
            stack: stack.to_owned(),
            link_spec: hs.link_spec.clone(),
            up_bps: hs.site.map(|x| x.profile.up),
            delay_us: delay.map(us),
            queue_us: s
                .send_buffer
                .filter(|b| b.window && hs.site.is_some())
                .map(|b| us(b.uplink_queue)),
            clock_us: self.hosts.get(h).map_or(0, |x| x.clock_offset.as_micros()),
        }
    }

    /// When host `h` left for good and how: its applied
    /// [`Fault::Depart`](crate::Fault::Depart), if any.
    pub fn departure(&self, h: HostIx) -> Option<(Duration, crate::Departure)> {
        self.applied_faults().find_map(|f| match f.fault {
            crate::Fault::Depart(x, how) if x == h => Some((f.at, how)),
            _ => None,
        })
    }

    /// The faults the run applied, in time order: `scenario.faults` less
    /// [`Report::skipped_faults`].
    pub fn applied_faults(&self) -> impl Iterator<Item = &crate::spec::TimedFault> + '_ {
        self.scenario
            .faults
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.skipped_faults.contains(i))
            .map(|(_, f)| f)
    }

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
    /// viewer is owed every frame the seed wrote while it was there
    /// ([`Report::owed`]), including those written while it was down.
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
            let range = self.owed(v);
            let from = match media {
                true => v
                    .stats
                    .joins
                    .iter()
                    .find_map(|j| j.first_seq)
                    .unwrap_or(w.bound())
                    .max(range.start),
                false => range.start,
            };
            let owed = w.count_in(from..range.end);
            want += owed;
            got += owed - w.missing_from(&v.stats.received, from..range.end);
        }
        ratio(got, want)
    }

    /// [`Report::delivery`] over the viewers among `hosts`, each owed every
    /// frame written while it was there ([`Report::owed`]).
    pub fn delivery_of(&self, swarm: usize, hosts: &[HostIx]) -> f64 {
        let Some(w) = self.written.get(swarm) else {
            return 0.0;
        };
        let (mut got, mut want) = (0, 0);
        for v in self
            .viewers
            .iter()
            .filter(|v| v.swarm == swarm && hosts.contains(&v.host))
        {
            let range = self.owed(v);
            let owed = w.count_in(range.clone());
            want += owed;
            got += owed - w.missing_from(&v.stats.received, range);
        }
        ratio(got, want)
    }

    /// When host `h` was there: from its arrival ([`HostSpec::arrive_at`],
    /// the run's start without one) to its departure
    /// ([`crate::Fault::Depart`], the run's end without one).
    ///
    /// [`HostSpec::arrive_at`]: crate::HostSpec::arrive_at
    pub fn presence(&self, h: HostIx) -> (Duration, Duration) {
        let from = self
            .scenario
            .hosts
            .get(h)
            .and_then(|s| s.arrive_at)
            .unwrap_or_default();
        let to = self
            .departure(h)
            .map_or(self.scenario.duration, |(at, _)| at);
        (from, to)
    }

    /// The frames of its swarm viewer `v` is owed. A viewer that arrived
    /// ([`HostSpec::arrive_at`]) is owed them from its first frame on, or
    /// from its arrival if it read none: where a join at the live edge
    /// lands is its join's measure (the row's `join_*`). One that departed
    /// is owed those written [`CAUGHT_UP_MS`] before it left, which a
    /// viewer that kept up has read. The rest are owed every frame.
    ///
    /// [`HostSpec::arrive_at`]: crate::HostSpec::arrive_at
    /// [`CAUGHT_UP_MS`]: crate::stream::CAUGHT_UP_MS
    pub fn owed(&self, v: &ViewerReport) -> std::ops::Range<u64> {
        let bound = self.written.get(v.swarm).map_or(0, SeqSet::bound);
        let arrival = self.scenario.hosts.get(v.host).and_then(|s| s.arrive_at);
        let from = match arrival {
            Some(at) => v
                .stats
                .joins
                .first()
                .and_then(|j| j.first_seq)
                .unwrap_or_else(|| {
                    self.frames_between(v.swarm, at, self.scenario.duration)
                        .start
                }),
            None => 0,
        };
        let to = match self.departure(v.host) {
            Some((at, _)) => {
                let kept = at.saturating_sub(Duration::from_millis(crate::stream::CAUGHT_UP_MS));
                self.frames_between(v.swarm, Duration::ZERO, kept).end
            }
            None => bound,
        };
        from.min(to)..to
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
            median: sketch::percentile(&v, 50).unwrap_or(0),
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
    /// Swarm `swarm`'s data rate in bytes per second, as its seed sends a
    /// copy.
    pub(crate) fn swarm_rate(&self, swarm: usize) -> f64 {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return 0.0;
        };
        match self.media.get(swarm).and_then(Option::as_ref) {
            Some(m) => m.sent_bytes as f64 / s.stop.saturating_sub(s.start).as_secs_f64().max(1e-9),
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

    /// When frame `seq` of swarm `swarm` was due, in run time: its media
    /// unit's time, or its write's due time, which the frame carries as
    /// its stamp; a late writer ([`crate::Scenario::timer_lateness`])
    /// writes it up to 2 ms after.
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

    /// The most host `h` can upload, the capacity it was given: the smaller
    /// of its rate limit and what its site's uplink carries (its rate, or
    /// its capacity schedule's mean), in IP bytes at full-size packets (a
    /// 1500-byte packet's share of what its framing charges it), whatever
    /// the host sent. `None` with neither. The meter's truth, what it could
    /// upload over a second in its meter's units, is
    /// [`Report::upload_cap_at`].
    pub fn upload_cap(&self, h: HostIx) -> Option<u64> {
        let spec = self.scenario.hosts.get(h)?;
        let site = spec.site.map(|s| {
            let link = spec
                .up_capacity
                .as_ref()
                .map_or(s.profile.up, |c| c.mean_rate());
            let f = s.profile.framing;
            if f == strims_wan::framing::Framing::NONE {
                link
            } else {
                link * 1500 / f.packet(1500) as u64
            }
        });
        match (spec.uplink, site) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
        .filter(|&c| c > 0)
    }

    /// The meter's truth: what host `h` could upload over the second
    /// before `t` (time since the start), in its meter's units
    /// (`upload_truth`), from its meter samples and its competing uploads'
    /// log; `None` with neither a site nor a rate limit, and 0 while its
    /// link carries nothing for it (an outage), which the tables keep, the
    /// meter's ratios leave out ([`Report::meter_series`]) and the seed's
    /// outage readings take ([`Report::seed_outages`]).
    pub fn upload_cap_at(&self, h: HostIx, t: std::time::Duration) -> Option<u64> {
        let spec = self.scenario.hosts.get(h)?;
        let samples = self.hosts.get(h).map_or(&[][..], |r| &r.runner.meter[..]);
        // The samples before `t`: the oracle tells a sample's truth before
        // the sample is taken.
        let k = samples.partition_point(|m| m.asked < t);
        let last = k.checked_sub(1).and_then(|i| samples.get(i));
        let log = self
            .wan
            .as_ref()
            .and_then(|w| w.hosts.iter().find(|w| w.name == spec.name))
            .map_or(&[][..], |w| &w.competing_log[..]);
        upload_truth(spec, last, log, t)
    }

    /// The upload meter's readings at `h` while a swarm was live, as
    /// fractions of what the host could upload at each sample
    /// ([`upload_cap_at`](Report::upload_cap_at)):
    /// (capacity ÷ cap, the push rate its subscriptions commit ÷ cap) per
    /// sample, the capacity the relay busy-time reading
    /// ([`MeterSample::capacity`](crate::stats::MeterSample::capacity)), a
    /// seed's too. A sample whose truth is 0 (an outage) has no ratio and
    /// is left out, as every sample is without a cap: the seed's are
    /// [`seed_outages`](Report::seed_outages)'.
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
                let cap = self.upload_cap_at(h, m.asked).filter(|&c| c > 0)? as f64;
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

    /// Swarm `swarm`'s seed meter over its window
    /// ([`seed_meter_window`](Report::seed_meter_window)), the row's
    /// `meter_seed_p*` and `meter_budget_seed_p*`: at each of the seed's
    /// samples there, the meter's capacity ÷ what the seed could upload
    /// ([`meter_series`](Report::meter_series)); and once the seed's budget
    /// for the swarm is measured, the capacity its latest allocation check
    /// measured it from
    /// ([`MeterSample::budget_capacity`](crate::stats::MeterSample::budget_capacity))
    /// ÷ the same. Both empty without a window or a cap.
    pub fn seed_meter(&self, swarm: usize) -> (Vec<f64>, Vec<f64>) {
        let (Some(spec), Some((from, to))) = (
            self.scenario.swarms.get(swarm),
            self.seed_meter_window(swarm),
        ) else {
            return (Vec::new(), Vec::new());
        };
        let within = |t: Duration| t >= from && t <= to;
        let capacity = self.window_readings(swarm, &[spec.seed]);
        let budget = self.hosts.get(spec.seed).map_or(Vec::new(), |host| {
            host.runner
                .meter
                .iter()
                .filter(|m| within(m.t))
                .filter_map(|m| {
                    let c = *m.budget_capacity.get(&swarm).filter(|&&c| c > 0)?;
                    Some(
                        c as f64
                            / self.upload_cap_at(spec.seed, m.asked).filter(|&c| c > 0)? as f64,
                    )
                })
                .collect()
        });
        (capacity, budget)
    }

    /// Swarm `swarm`'s seed meter over its outages, the row's
    /// `meter_seed_outage_share` and `meter_seed_outage_over_bps_p*`: of
    /// the seed's samples in its window
    /// ([`seed_meter_window`](Report::seed_meter_window)) that have a
    /// reading (a capacity above 0, those
    /// [`seed_meter`](Report::seed_meter)'s ratios take) and a truth
    /// ([`upload_cap_at`](Report::upload_cap_at)), the share whose truth is
    /// 0, a link that carries nothing for it, where the ratios have none;
    /// and the meter's capacity at each of those, in bytes per second, all
    /// of it an over-read. `None` without a window or such a sample there.
    pub fn seed_outages(&self, swarm: usize) -> Option<(f64, Vec<u64>)> {
        let spec = self.scenario.swarms.get(swarm)?;
        let (from, to) = self.seed_meter_window(swarm)?;
        let (mut with, mut over) = (0usize, Vec::new());
        for m in self
            .hosts
            .get(spec.seed)?
            .runner
            .meter
            .iter()
            .filter(|m| m.t >= from && m.t <= to && m.capacity > 0)
        {
            match self.upload_cap_at(spec.seed, m.asked) {
                Some(0) => over.push(m.capacity),
                Some(_) => {}
                None => continue,
            }
            with += 1;
        }
        (with > 0).then(|| (over.len() as f64 / with as f64, over))
    }

    /// The meter's capacity ÷ what the host could upload at each of
    /// `hosts`' samples in swarm `swarm`'s seed meter window
    /// ([`seed_meter_window`](Report::seed_meter_window),
    /// [`meter_series`](Report::meter_series)): the seed's `meter_seed_p*`,
    /// a population's `meter_{pop}_p*`. Empty without a window.
    pub fn window_readings(&self, swarm: usize, hosts: &[HostIx]) -> Vec<f64> {
        let Some((from, to)) = self.seed_meter_window(swarm) else {
            return Vec::new();
        };
        hosts
            .iter()
            .flat_map(|&h| self.meter_series(h))
            .filter(|&(t, _, _)| t >= from && t <= to)
            .map(|(_, c, _)| c)
            .collect()
    }

    /// Swarm `swarm`'s seed meter's window: from the end of its startup
    /// phase, [`STARTUP_SECS`](strims_wan::vocab::STARTUP_SECS) after its
    /// stream's first write, to its last; None for a stream no longer than
    /// its startup. The steady window [`Report::relay_share`] and
    /// [`Report::holder_short`] read too.
    pub fn seed_meter_window(&self, swarm: usize) -> Option<(Duration, Duration)> {
        let s = self.scenario.swarms.get(swarm)?;
        let from = s.start + Duration::from_secs(strims_wan::vocab::STARTUP_SECS);
        (from <= s.stop).then_some((from, s.stop))
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
            let at = |p: u32| sketch::percentile(v, p).unwrap_or(0.0);
            format!("{:.2}/{:.2}/{:.2}", at(10), at(50), at(90))
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
        if !self.scenario.clock_skew.is_off() {
            let (per, max) = self.clock_classes();
            let per: Vec<String> = crate::skew::SkewClass::ALL
                .into_iter()
                .zip(per)
                .map(|(c, n)| format!("{} {n}", c.name()))
                .collect();
            s.push_str(&format!(
                "\n  clock skew {}: hosts {}, largest {} ms",
                self.scenario.clock_skew,
                per.join(", "),
                max.as_millis()
            ));
        }
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
        if let Some(line) = self.profile_line() {
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
                "\n  memory: peak {} KiB, last sample {} KiB, records peak {} KiB",
                m.peak / 1024,
                m.samples.last().map_or(0, |&(_, b)| b / 1024),
                m.records_peak / 1024,
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

/// Spearman's rank correlation of `pairs`, ties at their mean rank; `None`
/// for fewer than three pairs, or a side that doesn't vary.
pub fn spearman(pairs: &[(f64, f64)]) -> Option<f64> {
    fn ranks(v: &[f64]) -> Vec<f64> {
        let mut order: Vec<(f64, usize)> = v.iter().copied().zip(0..).collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut r = vec![0.0; v.len()];
        let mut i = 0;
        for run in order.chunk_by(|a, b| a.0 == b.0) {
            let mean = i as f64 + (run.len() - 1) as f64 / 2.0 + 1.0;
            for &(_, k) in run {
                if let Some(x) = r.get_mut(k) {
                    *x = mean;
                }
            }
            i += run.len();
        }
        r
    }
    if pairs.len() < 3 {
        return None;
    }
    let a = ranks(&pairs.iter().map(|p| p.0).collect::<Vec<_>>());
    let b = ranks(&pairs.iter().map(|p| p.1).collect::<Vec<_>>());
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(&b) {
        cov += (x - ma) * (y - mb);
        va += (x - ma) * (x - ma);
        vb += (y - mb) * (y - mb);
    }
    (va > 0.0 && vb > 0.0).then(|| cov / (va * vb).sqrt())
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

/// The truth's window: the second before the time it is read at.
pub const TRUTH_WINDOW: std::time::Duration = std::time::Duration::from_secs(1);

/// A sample window whose own link charge is at least this share of what
/// the host's link could carry for it (`available`) is near saturation:
/// its messages are full, and its share of the link is its transports'.
pub const NEAR_SATURATED: f64 = 0.8;

/// The meter's truth for host `spec` at `t` (time since the start): what
/// it could upload over `(t - 1 s, t]` ([`TRUTH_WINDOW`]), in its meter's
/// units, the channel bytes its runner writes (`UploadInfo::rate`'s),
/// within its rate limit; `None` with neither a site nor a limit, and 0
/// over a window its link carries nothing for it (an outage): what its
/// link could carry for it over the window ([`available`]) times its
/// share ([`own_share`]) as of `last`, its last meter sample asked for
/// before `t`, within its rate limit (QoS's, in vnic's bytes) in channel bytes
/// ([`channel_share`]).
///
/// The live oracle (`STRIMS_SIM_KNOWN_UPLINK`) tells the node this as the
/// controller asks for each of its stats samples, from the same inputs,
/// ahead of the sample, whose own truth is read at its answer
/// (`MeterSample::asked`).
pub(crate) fn upload_truth(
    spec: &crate::spec::HostSpec,
    last: Option<&crate::stats::MeterSample>,
    competing: &[(strims_base::time::Duration, u64)],
    t: std::time::Duration,
) -> Option<u64> {
    let to = t;
    let from = to.saturating_sub(TRUTH_WINDOW);
    let site = spec.site.map(|_| {
        let avail = available(spec, competing, from, to) as f64 * TRUTH_WINDOW.as_secs_f64()
            / (to - from).as_secs_f64().max(1e-9);
        (avail * own_share(spec, last)) as u64
    });
    // The rate limit is QoS's, in what vnic hands the transports.
    let limit = spec.uplink.map(|l| (l as f64 * channel_share(last)) as u64);
    match (limit, site) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// What host `spec`'s link could carry for it over `(from, to]` (times
/// since the start), in its link's bytes: what its uplink carries then (its
/// rate's mean, its steps' from their times, [`HostSpec::up_steps`]; with a
/// capacity schedule, [`HostSpec::up_capacity`], the bytes of its
/// opportunities in the window, `Capacity::bytes_between`; a bucket's
/// bursts leave it its rate), less what its link charged its competing
/// uploads in it ([`HostSpec::competing`]: their bytes sent in the window,
/// from the walk-stamped log `competing`, at what the link charges a
/// segment). 0 without a site.
///
/// [`HostSpec::up_steps`]: crate::spec::HostSpec::up_steps
/// [`HostSpec::up_capacity`]: crate::spec::HostSpec::up_capacity
/// [`HostSpec::competing`]: crate::spec::HostSpec::competing
pub(crate) fn available(
    spec: &crate::spec::HostSpec,
    competing: &[(strims_base::time::Duration, u64)],
    from: std::time::Duration,
    to: std::time::Duration,
) -> u64 {
    use strims_base::time::Duration as D;
    let Some(s) = spec.site else {
        return 0;
    };
    let (f, t) = (D::from(from), D::from(to));
    let carried = match &spec.up_capacity {
        Some(c) => c.bytes_between(crate::wan::sim_time(from), crate::wan::sim_time(to)),
        None => {
            let rate = mean_rate(s.profile.up, &spec.up_steps, f, t);
            (rate as f64 * (to.saturating_sub(from)).as_secs_f64()) as u64
        }
    };
    let framing = s.profile.framing;
    let w = crate::wan::CROSS_WIRE as u64;
    let cross = competing_before(competing, t) - competing_before(competing, f);
    let cross = cross * framing.message(crate::wan::CROSS_WIRE, 2) as u64 / w;
    carried.saturating_sub(cross)
}

/// Host `spec`'s outages that overlap `(from, to)` (times since the
/// start), in time order: the gaps of its capacity schedule
/// ([`HostSpec::up_capacity`], `Capacity::gaps`) longer than
/// [`TRUTH_WINDOW`], over which its truth falls to 0, each from the
/// opportunity before it to the one after (from the start, for one before
/// it). None without a schedule.
///
/// [`HostSpec::up_capacity`]: crate::spec::HostSpec::up_capacity
pub(crate) fn outages(
    spec: &crate::spec::HostSpec,
    from: std::time::Duration,
    to: std::time::Duration,
) -> Vec<(std::time::Duration, std::time::Duration)> {
    let Some(c) = &spec.up_capacity else {
        return Vec::new();
    };
    let since =
        |t: strims_base::time::Time| (t - crate::world::epoch()).to_std().unwrap_or_default();
    c.gaps(
        crate::wan::sim_time(from),
        crate::wan::sim_time(to),
        TRUTH_WINDOW.into(),
    )
    .into_iter()
    .map(|(a, b)| (since(a), since(b)))
    .collect()
}

/// What an uplink of `rate` bytes per second with `steps` carries over
/// `(from, to]`, one second of it: the window's mean rate.
fn mean_rate(
    rate: u64,
    steps: &[(std::time::Duration, u64)],
    from: strims_base::time::Duration,
    to: strims_base::time::Duration,
) -> u64 {
    use strims_base::time::Duration as D;
    let span = (to - from).as_nanos();
    if span <= 0 {
        return 0;
    }
    let mut steps: Vec<(D, u64)> = steps.iter().map(|&(at, r)| (D::from(at), r)).collect();
    steps.sort_by_key(|&(at, _)| at);
    let rate_at = |x: D| {
        steps
            .iter()
            .take_while(|&&(at, _)| at <= x)
            .last()
            .map_or(rate, |&(_, r)| r)
    };
    let mut bytes: i128 = 0;
    let mut a = from;
    for &(at, _) in steps.iter().filter(|&&(at, _)| at > from && at < to) {
        bytes += i128::from(rate_at(a)) * i128::from((at - a).as_nanos());
        a = at;
    }
    bytes += i128::from(rate_at(a)) * i128::from((to - a).as_nanos());
    u64::try_from(bytes / i128::from(span)).unwrap_or(0)
}

/// What a competing uploads' log (walk-stamped cumulative IP bytes, as
/// [`WanHostReport::competing_log`] holds it) says they had sent before
/// `t`: its last entry stamped before `t`, which the run had walked when a
/// host's task read it at `t`.
pub(crate) fn competing_before(
    log: &[(strims_base::time::Duration, u64)],
    t: strims_base::time::Duration,
) -> u64 {
    let i = log.partition_point(|&(at, _)| at < t);
    i.checked_sub(1)
        .and_then(|i| log.get(i))
        .map_or(0, |&(_, b)| b)
}

/// The share of what host `spec`'s link charges it that is channel bytes,
/// the meter's units, as of its meter sample `last` ([`channel_share`]
/// times [`transport_share`]): a property of its link, its transports and
/// vnic, taken over its windows near saturation ([`NEAR_SATURATED`]),
/// whatever the host was sending since.
pub fn own_share(spec: &crate::spec::HostSpec, last: Option<&crate::stats::MeterSample>) -> f64 {
    channel_share(last) * transport_share(spec, last)
}

/// The share of what a host handed vnic that was its runner's channel
/// bytes, over its near-saturated windows up to `last`
/// ([`Gated`](crate::stats::Gated): `data` over `vnic`, at most 1); 1 before
/// any, as vnic frames them (4 bytes a frame of up to 64 KiB).
pub fn channel_share(last: Option<&crate::stats::MeterSample>) -> f64 {
    last.map(|k| k.gated)
        .filter(|g| g.data > 0 && g.vnic > 0)
        .map_or(1.0, |g| (g.data as f64 / g.vnic as f64).min(1.0))
}

/// The share of what host `spec`'s link charged its messages' first
/// transmissions that its transports took (vnic's bytes), counted as each
/// message went to the link, over its near-saturated windows up to `last`
/// ([`Gated`](crate::stats::Gated): its resends left out, which a full
/// queue's congestion makes, not the link), else a full message's
/// ([`full_share`]). At most 1. It is cumulative: one window's share would
/// take that window's drops in.
pub fn transport_share(
    spec: &crate::spec::HostSpec,
    last: Option<&crate::stats::MeterSample>,
) -> f64 {
    last.map(|k| k.gated)
        .filter(|g| g.app > 0 && g.link > 0)
        .map_or_else(
            || full_share(spec),
            |g| (g.app as f64 / g.link as f64).min(1.0),
        )
}

/// What a full message of host `spec`'s transport carries of its link's
/// charge: a data channel's full datagram with the window model's data
/// channels ([`HostSpec::rtc_stack`]), else a TCP segment of two full
/// packets (`WINDOW_SEGMENT`), each a full MSS under its IP and TCP
/// headers, at its link's framing.
///
/// [`HostSpec::rtc_stack`]: crate::spec::HostSpec::rtc_stack
pub fn full_share(spec: &crate::spec::HostSpec) -> f64 {
    use strims_wan::wire::{MSS, TCP_HEADER};
    let framing = spec
        .site
        .map_or(strims_wan::framing::Framing::NONE, |s| s.profile.framing);
    match spec.rtc_stack {
        Some(p) => p.full_share(framing),
        None => (2 * MSS) as f64 / framing.message(2 * (MSS + TCP_HEADER), 2) as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_and_gini() {
        let p = Percentiles::of(1..=100);
        assert_eq!((p.p50, p.p99, p.max, p.n), (50, 99, 100, 100));
        let p = Percentiles::of((1..=1000).rev());
        assert_eq!((p.p50, p.p99, p.max, p.n), (500, 988, 1000, 1000));
        assert_eq!(Percentiles::of([]).max, 0);
        assert!(gini(&[5, 5, 5, 5]).abs() < 1e-9);
        assert!(gini(&[0, 0, 0, 100]) > 0.7);
    }

    #[test]
    fn spearman_ranks_ties_at_their_mean() {
        let up = [(1.0, 4.0), (2.0, 3.0), (3.0, 2.0), (4.0, 1.0)];
        assert!(spearman(&up).is_some_and(|r| (r + 1.0).abs() < 1e-9));
        // Ranks 1.5, 1.5, 3 against 1, 2, 3.
        let tied = [(1.0, 1.0), (1.0, 2.0), (2.0, 3.0)];
        assert!(spearman(&tied).is_some_and(|r| (r - 0.866_025_4).abs() < 1e-6));
        assert_eq!(spearman(&[(1.0, 1.0), (1.0, 2.0), (1.0, 3.0)]), None);
        assert_eq!(spearman(&[(1.0, 1.0), (2.0, 2.0)]), None);
    }
}
