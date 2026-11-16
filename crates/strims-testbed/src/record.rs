// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A node's records as strims-sim's tables take them, made from its
//! metrics: one definition for the simulated nodes and the testbed's.
//!
//! strims-sim folds every node's stats samples through [`meter_sample`]
//! and every stats bin through [`bin_counters`] and [`bin_waits`]; the
//! testbed's nodes run the same functions on their own samples and bins
//! and log the records ([`crate::node`]'s `rec`), which strims-sim's
//! `testbed` module reads back into a run's report. A testbed node keeps
//! its parent log in a [`ParentLog`] between its lines.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

use strims_base::path;
use strims_base::stats::WaitHistogram;
use strims_base::time::Time;
use strims_node_core::{NodeBin, NodeMetrics, ParentLogEntry, ParentSink, SwarmBin, SwarmKey};
use strims_ppspp::PushMode;

/// The width of the tables' bins, in ms, unless a sim run sets its own
/// (`STRIMS_SIM_BIN_MS`): the testbed's nodes keep their stats bins at it.
pub const BIN_MS: i64 = 100;

/// A node's upload meter at one sample (`PushStats::upload_rate`,
/// `upload_busy_permille`, `upload_capacity`, `upload_queue_delay` in
/// milliseconds), with the subscriptions it served per scenario swarm
/// (`PushStats::children`) and the STREAM_REQUESTs it answered (`accepted`
/// plus `rejected`, over its hybrid swarms, since the incarnation started).
/// `capacity` is the relay busy-time reading admission and shedding take
/// (`UploadInfo::relay_capacity`), a seed's too; an adaptive seed's budget
/// takes its own, per scenario swarm (`budget_capacity`,
/// `PushStats::seed_budget_capacity`: the reading its latest allocation
/// check measured the budget from, zero until measured).
///
/// Beside the meter's queueing delay, two from vnic's path measure over
/// the node's links with a recent round trip, each aggregated as the meter
/// does ([`strims_base::path::aggregate`]): how far each link's recent
/// round trip stands above its base (`path_delay_ms`), and how long each
/// link's queued bytes take at its delivery rate (`path_queued_ms`). And
/// how far the links' latest round trips stand above their bases
/// ([`strims_base::path::latest_excess`], `path_latest_ms`). And over its
/// hybrid swarms, its channels (`PushStats::pace_channels`) and those whose
/// peer it asks for lazy HAVEs and whose peer asked them of it
/// (`have_lazy_out`, `have_lazy_in`). And the bytes the node handed to
/// vnic since the incarnation started (`DriverMetrics::bytes_written`).
/// And what strims-sim's meter truth takes its share from (its
/// `report::own_share`), which the sim's counters give and a testbed
/// node's records leave out (0, and `asked` its `t`): the channel payload
/// bytes its runner has written since the run started, every incarnation
/// (the bytes `UploadInfo::rate` counts, `ChannelMetrics::wire_bytes` of
/// its outbound totals), what its uplink has charged its own messages
/// (`EndStats::up_bytes`; with no WAN, 0), and its transports' and vnic's
/// bytes beside them. With the live oracle, what it told the node ahead
/// of the sample (`told`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeterSample {
    /// When it was taken: simulated time since the start of the run in
    /// strims-sim, the wall clock since the Unix epoch on the testbed.
    pub t: Duration,
    /// When the controller asked for the sample, which the meter's truth
    /// is read at (strims-sim's `Report::upload_cap_at`): the live oracle
    /// tells the node that truth then, ahead of the sample.
    pub asked: Duration,
    pub rate: u64,
    pub busy_permille: u32,
    pub queue_delay_ms: Option<u32>,
    pub path_delay_ms: Option<u32>,
    pub path_queued_ms: Option<u32>,
    pub path_latest_ms: Option<u32>,
    pub capacity: u64,
    pub budget_capacity: BTreeMap<usize, u64>,
    pub children: BTreeMap<usize, usize>,
    pub requests_in: u32,
    pub rejected: u32,
    pub pace_channels: u32,
    pub lazy_out: u32,
    pub lazy_in: u32,
    pub bytes_written: u64,
    pub data_bytes: u64,
    pub link_bytes: u64,
    pub told: Option<u64>,
    /// What the host's transports took of its uplink's messages so far
    /// (vnic's bytes, counted as each message went to the link), and what
    /// it handed vnic, every incarnation's (`DriverMetrics::bytes_written`).
    pub app_bytes: u64,
    /// What its link charged its transports' resends so far (the window
    /// model's), which its share leaves out.
    pub resent_bytes: u64,
    pub vnic_bytes: u64,
    /// Its near-saturated windows so far ([`Gated`]).
    pub gated: Gated,
}

/// A host's near-saturated sample windows up to a meter sample: those
/// whose own link charge was at least strims-sim's
/// `report::NEAR_SATURATED` of what its link could carry for it, summed
/// (the transports' bytes, the link's charge of its first transmissions,
/// its resends left out, the runner's channel payload bytes and vnic's),
/// and whether the window ending at the sample was one. The meter's truth
/// takes its share from them (strims-sim's `report::own_share`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gated {
    pub app: u64,
    pub link: u64,
    pub data: u64,
    pub vnic: u64,
    pub window: bool,
}

/// An adaptive seed's `PushStats` seed fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeedSample {
    pub budget: usize,
    pub window: usize,
    pub copies: usize,
    pub first_wave: usize,
    pub first_wave_capacity: u64,
}

/// The meter sample of `m`, taken at `t` when the node's clock read
/// `clock` (its readings' times are by it), `None` without a hybrid swarm
/// (one meter per node: every hybrid swarm reads the same); and the
/// adaptive seed's state in each swarm it seeds, by scenario swarm.
/// `index` maps the node's swarm keys to scenario swarms.
pub fn meter_sample(
    m: &NodeMetrics,
    index: impl Fn(SwarmKey) -> Option<usize>,
    t: Duration,
    clock: Time,
) -> (Option<MeterSample>, Vec<(usize, SeedSample)>) {
    let mut meter: Option<MeterSample> = None;
    let mut seeds = Vec::new();
    for sw in &m.runner.swarms {
        let (Some(s), Some(_), Some(push)) = (index(sw.swarm), sw.scheduler, &sw.push) else {
            continue;
        };
        if push.mode.resolve() != PushMode::Hybrid {
            continue;
        }
        let ms = meter.get_or_insert_with(|| MeterSample {
            t,
            asked: t,
            bytes_written: m.driver.bytes_written,
            rate: push.upload_rate,
            busy_permille: push.upload_busy_permille,
            capacity: push.upload_capacity,
            queue_delay_ms: push
                .upload_queue_delay
                .map(|d| u32::try_from(d.as_millis().max(0)).unwrap_or(u32::MAX)),
            ..MeterSample::default()
        });
        ms.children.insert(s, push.children);
        let count = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
        ms.requests_in = ms
            .requests_in
            .saturating_add(count(push.accepted + push.rejected));
        ms.rejected = ms.rejected.saturating_add(count(push.rejected));
        let n = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        ms.pace_channels = ms.pace_channels.saturating_add(n(push.pace_channels));
        ms.lazy_out = ms.lazy_out.saturating_add(n(push.have_lazy_out));
        ms.lazy_in = ms.lazy_in.saturating_add(n(push.have_lazy_in));
        if push.seed_window > 0 {
            ms.budget_capacity.insert(s, push.seed_budget_capacity);
            seeds.push((
                s,
                SeedSample {
                    budget: push.seed_budget,
                    window: push.seed_window,
                    copies: push.seed_copies,
                    first_wave: push.seed_first_wave,
                    first_wave_capacity: push.seed_first_wave_capacity,
                },
            ));
        }
    }
    if let Some(ms) = meter.as_mut() {
        let ms_of = |d: strims_base::time::Duration| {
            u32::try_from(d.as_millis().max(0)).unwrap_or(u32::MAX)
        };
        let (excess, queued) = path::node_delays(m.links.iter().filter_map(|l| l.path?.reading));
        ms.path_delay_ms = excess.map(ms_of);
        ms.path_queued_ms = queued.map(ms_of);
        ms.path_latest_ms = path::latest_excess(
            m.links.iter().filter_map(|l| l.path?.reading),
            clock,
            &mut Vec::new(),
        )
        .map(ms_of);
    }
    (meter, seeds)
}

/// [`bin_counters`]: pushes before their deadline, deadlines released to
/// pull, parents pruned and streams with an open parent; the upload
/// meter's capacity (the relay busy-time reading,
/// `PushStats::upload_capacity`) and busy thousandths and a seed's copies;
/// chunks sent and pushed; requests timed out and chunks wasted; answers a
/// full queue refused and answers dropped at write time
/// ([`strims_ppspp::SwarmCounters`]); parents replaced, parents lost to
/// their closing the subscription and subscribers closed for a saturated
/// uplink (`PushStats`' `switched`, `shed`, `sheds_sent`); its outbound
/// messages' bytes by kind, its payloads and those of HAVEs alone
/// ([`strims_ppspp::ChannelMetrics`]); and the bytes it wrote that weren't
/// chunk data (the series' overhead); and the capacity an adaptive seed's
/// latest check measured its budget from
/// (`PushStats::seed_budget_capacity`, zero elsewhere).
pub const NODE_BIN_COUNTERS: [&str; 30] = [
    "hits",
    "misses",
    "pruned",
    "parents",
    "capacity",
    "busy",
    "copies",
    "sent",
    "pushed",
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
    "ovh",
    "budget_capacity",
];

/// [`NODE_BIN_COUNTERS`]' values at a bin edge `b`, of its swarm `sw`.
pub fn bin_counters(b: &NodeBin, sw: &SwarmBin) -> [u64; NODE_BIN_COUNTERS.len()] {
    let o = &b.outbound;
    let c = &sw.counters;
    [
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
    ]
}

/// Swarm `sw`'s push and answer waits so far, at a bin edge.
pub fn bin_waits(sw: &SwarmBin) -> [WaitHistogram; 2] {
    [sw.counters.push_wait, sw.counters.answer_wait]
}

/// The parent changes a testbed node's log hands over
/// ([`strims_node_core::NodeConfig::parent_log`]), kept until its next
/// line takes them: at most [`ParentLog::CAP`], those past it counted as
/// lost, as are those the runner's caps left out.
#[derive(Clone, Debug)]
pub struct ParentLog {
    tx: SyncSender<ParentLogEntry>,
    rx: Rc<Receiver<ParentLogEntry>>,
    lost: Arc<AtomicU64>,
}

impl Default for ParentLog {
    fn default() -> ParentLog {
        let (tx, rx) = sync_channel(Self::CAP);
        ParentLog {
            tx,
            rx: Rc::new(rx),
            lost: Arc::default(),
        }
    }
}

impl ParentLog {
    /// The entries a log keeps between two takes.
    pub const CAP: usize = 4096;

    /// The sink for the node's config.
    pub fn sink(&self) -> ParentSink {
        let (tx, lost) = (self.tx.clone(), self.lost.clone());
        ParentSink(Arc::new(move |e| {
            let n = match e {
                ParentLogEntry::Lost(n) => n,
                ParentLogEntry::Change(_) => u64::from(tx.try_send(e).is_err()),
            };
            lost.fetch_add(n, Ordering::Relaxed);
        }))
    }

    /// The changes since the last take, and the changes left out.
    pub fn take(&self) -> (Vec<ParentLogEntry>, u64) {
        (
            self.rx.try_iter().collect(),
            self.lost.swap(0, Ordering::Relaxed),
        )
    }
}

/// `v`, or `null`.
fn opt(v: Option<impl ToString>) -> String {
    v.map_or("null".to_owned(), |v| v.to_string())
}

/// `{"<swarm>":<value>,...}` of a map by scenario swarm.
fn by_swarm(m: &BTreeMap<usize, impl ToString>) -> String {
    let parts: Vec<String> = m
        .iter()
        .map(|(s, v)| format!("\"{s}\":{}", v.to_string()))
        .collect();
    format!("{{{}}}", parts.join(","))
}

impl MeterSample {
    /// One JSON object, fields named as here, `t` as `t_ms` in ms.
    pub fn json(&self) -> String {
        format!(
            "{{\"t_ms\":{},\"rate\":{},\"busy_permille\":{},\"queue_delay_ms\":{},\"path_delay_ms\":{},\"path_queued_ms\":{},\"path_latest_ms\":{},\"capacity\":{},\"budget_capacity\":{},\"children\":{},\"requests_in\":{},\"rejected\":{},\"pace_channels\":{},\"lazy_out\":{},\"lazy_in\":{},\"bytes_written\":{}}}",
            self.t.as_millis(),
            self.rate,
            self.busy_permille,
            opt(self.queue_delay_ms),
            opt(self.path_delay_ms),
            opt(self.path_queued_ms),
            opt(self.path_latest_ms),
            self.capacity,
            by_swarm(&self.budget_capacity),
            by_swarm(&self.children),
            self.requests_in,
            self.rejected,
            self.pace_channels,
            self.lazy_out,
            self.lazy_in,
            self.bytes_written,
        )
    }
}

impl SeedSample {
    /// `[budget, window, copies, first_wave, first_wave_capacity]`.
    pub fn json(&self) -> String {
        format!(
            "[{},{},{},{},{}]",
            self.budget, self.window, self.copies, self.first_wave, self.first_wave_capacity
        )
    }
}

/// A wait histogram's buckets that hold waits, as `[[bucket, count],...]`.
pub fn waits_json(h: &WaitHistogram) -> String {
    let mut s = String::from("[");
    for (i, &n) in h.buckets.iter().enumerate().filter(|(_, n)| **n > 0) {
        if s.len() > 1 {
            s.push(',');
        }
        let _ = write!(s, "[{i},{n}]");
    }
    s.push(']');
    s
}
