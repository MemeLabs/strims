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
//! its parent log in a [`ParentLog`] between its lines, and its parent
//! checks in a [`DecisionLog`], with its timer log on its timers in a
//! [`TimerLog`], with its link log on its links' I/O in a
//! [`links::LinkLog`], and with its scheduler's logs on hands each line
//! their records ([`logs`]).

pub mod links;
pub mod logs;

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
use strims_node_core::{
    DecisionInfo, DecisionLogEntry, DecisionSink, NodeBin, NodeMetrics, ParentLogEntry, ParentSink,
    SwarmBin, SwarmKey, TimerRecord, TimerSink, TimerSource,
};
use strims_ppspp::PushMode;
use strims_ppspp::TapKind;

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
/// check measured the budget from, zero until measured), and the latest
/// floor that check read its growth from (`latest_floor_ms`,
/// `PushStats::seed_latest_floor_ms`: none until measured, or where it
/// read no fresh round trips).
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
/// `report::own_share`): the channel payload bytes its runner has written
/// since the run started, every incarnation's (`channel_bytes`: the bytes
/// `UploadInfo::rate` counts, `ChannelMetrics::wire_bytes` of its outbound
/// totals; none where a node's line lacks them), and what it handed vnic
/// (`vnic_bytes`), which
/// [`meter_sample`] takes from the node's metrics, its incarnation's, and
/// strims-sim adds its finished incarnations' to; and what its uplink has
/// charged its own messages (`link_bytes`, `EndStats::up_bytes`; with no
/// WAN, 0) and its transports' bytes beside them, which the sim's counters
/// give and a testbed run's captures (strims-sim's `testbed` module), 0
/// in a node's records, as `asked` is its `t` there. With the live
/// oracle, what it told the node ahead of the sample (`told`).
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
    pub latest_floor_ms: BTreeMap<usize, u32>,
    pub children: BTreeMap<usize, usize>,
    pub requests_in: u32,
    pub rejected: u32,
    pub pace_channels: u32,
    pub lazy_out: u32,
    pub lazy_in: u32,
    pub bytes_written: u64,
    pub channel_bytes: Option<u64>,
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
            channel_bytes: Some(m.runner.totals.outbound.wire_bytes()),
            vnic_bytes: m.driver.bytes_written,
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
            if let Some(f) = push.seed_latest_floor_ms {
                ms.latest_floor_ms
                    .insert(s, u32::try_from(f).unwrap_or(u32::MAX));
            }
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

/// The parent checks a testbed node's decision log hands over
/// ([`strims_node_core::NodeConfig::decision_log`]: the tap's
/// `ParentChoice`s, its other events left out), kept until its next line
/// takes them: at most [`DecisionLog::CAP`], those past it counted as
/// lost, as are the events the tap's buffers left out, of any kind.
#[derive(Clone, Debug)]
pub struct DecisionLog {
    tx: SyncSender<DecisionInfo>,
    rx: Rc<Receiver<DecisionInfo>>,
    lost: Arc<AtomicU64>,
}

impl Default for DecisionLog {
    fn default() -> DecisionLog {
        let (tx, rx) = sync_channel(Self::CAP);
        DecisionLog {
            tx,
            rx: Rc::new(rx),
            lost: Arc::default(),
        }
    }
}

impl DecisionLog {
    /// The checks a log keeps between two takes.
    pub const CAP: usize = 4096;

    /// The sink for the node's config.
    pub fn sink(&self) -> DecisionSink {
        let (tx, lost) = (self.tx.clone(), self.lost.clone());
        DecisionSink(Arc::new(move |e| {
            let n = match e {
                DecisionLogEntry::Lost(n) => n,
                DecisionLogEntry::Decision(d) if matches!(d.kind, TapKind::ParentChoice { .. }) => {
                    u64::from(tx.try_send(d).is_err())
                }
                DecisionLogEntry::Decision(_) => 0,
            };
            lost.fetch_add(n, Ordering::Relaxed);
        }))
    }

    /// The checks since the last take, and the events left out.
    pub fn take(&self) -> (Vec<DecisionInfo>, u64) {
        (
            self.rx.try_iter().collect(),
            self.lost.swap(0, Ordering::Relaxed),
        )
    }
}

/// Where a node's timers run ([`TimerLog`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerContext {
    /// A native node's tokio task (a simulated node's too).
    Task,
    /// A browser node's worker.
    Worker,
}

impl TimerContext {
    /// Its name in the tables: `task`, `worker`.
    pub fn name(self) -> &'static str {
        match self {
            TimerContext::Task => "task",
            TimerContext::Worker => "worker",
        }
    }

    /// The context named `name` ([`TimerContext::name`]).
    pub fn parse(name: &str) -> Option<TimerContext> {
        [TimerContext::Task, TimerContext::Worker]
            .into_iter()
            .find(|c| c.name() == name)
    }
}

/// The timers of a node that fired, kept until the next take: those its
/// node task's log hands over ([`strims_node_core::NodeConfig::timer_log`],
/// the sink [`TimerLog::sink`]) and its harness's own ([`TimerLog::push`]),
/// all run in one `context`. It keeps at most [`TimerLog::CAP`] between
/// two takes, in a buffer it allocates once, and counts those past it as
/// lost. A testbed node's line takes them ([`crate::node`]), and
/// strims-sim takes a simulated node's at its stats samples.
#[derive(Clone, Debug)]
pub struct TimerLog {
    tx: SyncSender<TimerRecord>,
    rx: Rc<Receiver<TimerRecord>>,
    lost: Arc<AtomicU64>,
    context: TimerContext,
}

impl TimerLog {
    /// The timers a log keeps between two takes.
    pub const CAP: usize = 4096;

    /// An empty log of timers that run in `context`.
    pub fn new(context: TimerContext) -> TimerLog {
        let (tx, rx) = sync_channel(Self::CAP);
        TimerLog {
            tx,
            rx: Rc::new(rx),
            lost: Arc::default(),
            context,
        }
    }

    /// Where the timers run.
    pub fn context(&self) -> TimerContext {
        self.context
    }

    /// The sink for the node's config.
    pub fn sink(&self) -> TimerSink {
        let (tx, lost) = (self.tx.clone(), self.lost.clone());
        TimerSink(Arc::new(move |r| {
            if tx.try_send(r).is_err() {
                lost.fetch_add(1, Ordering::Relaxed);
            }
        }))
    }

    /// Keeps a timer of the harness's.
    pub fn push(&self, r: TimerRecord) {
        if self.tx.try_send(r).is_err() {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The timers since the last take, in the order they fired, and those
    /// left out.
    pub fn take(&self) -> (Vec<TimerRecord>, u64) {
        (
            self.rx.try_iter().collect(),
            self.lost.swap(0, Ordering::Relaxed),
        )
    }
}

/// µs from `origin` to `t`, floored.
fn us_since(t: Time, origin: Time) -> i64 {
    t.unix_nanos()
        .saturating_sub(origin.unix_nanos())
        .div_euclid(1000)
}

/// `rows` as a line's records hold them ([`crate::node`]'s `rec.timers`),
/// with their log's `context` and its `lost` count: `{"context": name,
/// "lost": n, "rows": [[source, armed_us, deadline_us, fired_us,
/// requested_us], ...]}`, times in µs from `origin`, floored.
pub fn timers_json(rows: &[TimerRecord], lost: u64, context: TimerContext, origin: Time) -> String {
    let mut s = format!(
        "{{\"context\":\"{}\",\"lost\":{lost},\"rows\":[",
        context.name()
    );
    for (i, r) in rows.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(
            s,
            "[\"{}\",{},{},{},{}]",
            r.source.name(),
            us_since(r.armed, origin),
            us_since(r.deadline, origin),
            us_since(r.fired, origin),
            r.requested.as_nanos().div_euclid(1000),
        );
    }
    s.push_str("]}");
    s
}

/// A line's `rec.timers` ([`timers_json`]) read back, times from
/// `origin`: its rows, its lost count and their context; `None` for one
/// that isn't that object. A row that doesn't parse is left out.
pub fn timers_of(
    v: &serde_json::Value,
    origin: Time,
) -> Option<(Vec<TimerRecord>, u64, TimerContext)> {
    let context = TimerContext::parse(v.get("context")?.as_str()?)?;
    let lost = v.get("lost")?.as_u64()?;
    let at = |us: i64| origin + strims_base::time::Duration::from_micros(us);
    let rows = v
        .get("rows")?
        .as_array()?
        .iter()
        .filter_map(|r| {
            let n = |k: usize| r.get(k).and_then(serde_json::Value::as_i64);
            Some(TimerRecord {
                source: TimerSource::parse(r.get(0)?.as_str()?)?,
                armed: at(n(1)?),
                deadline: at(n(2)?),
                fired: at(n(3)?),
                requested: strims_base::time::Duration::from_micros(n(4)?),
            })
        })
        .collect();
    Some((rows, lost, context))
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
            "{{\"t_ms\":{},\"rate\":{},\"busy_permille\":{},\"queue_delay_ms\":{},\"path_delay_ms\":{},\"path_queued_ms\":{},\"path_latest_ms\":{},\"capacity\":{},\"budget_capacity\":{},\"latest_floor_ms\":{},\"children\":{},\"requests_in\":{},\"rejected\":{},\"pace_channels\":{},\"lazy_out\":{},\"lazy_in\":{},\"bytes_written\":{},\"channel_bytes\":{}}}",
            self.t.as_millis(),
            self.rate,
            self.busy_permille,
            opt(self.queue_delay_ms),
            opt(self.path_delay_ms),
            opt(self.path_queued_ms),
            opt(self.path_latest_ms),
            self.capacity,
            by_swarm(&self.budget_capacity),
            by_swarm(&self.latest_floor_ms),
            by_swarm(&self.children),
            self.requests_in,
            self.rejected,
            self.pace_channels,
            self.lazy_out,
            self.lazy_in,
            self.bytes_written,
            opt(self.channel_bytes),
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_base::time::Duration as Span;
    use strims_ppspp::{ParentAction, ParentChoice};

    use super::*;

    fn row(source: TimerSource, ms: i64) -> TimerRecord {
        let t = Time::from_unix_millis(1_700_000_000_000 + ms);
        TimerRecord {
            source,
            armed: t - Span::from_micros(2_500),
            deadline: t - Span::from_micros(300),
            fired: t,
            requested: Span::from_millis(2),
        }
    }

    /// A log keeps what its sink and its harness hand it in order, up to
    /// its cap between takes, counting the rest as lost, and a take
    /// empties it.
    #[test]
    fn a_timer_log_keeps_its_cap_between_takes() {
        let log = TimerLog::new(TimerContext::Task);
        let sink = log.sink();
        (sink.0)(row(TimerSource::NodeTask, 0));
        log.push(row(TimerSource::Record, 1));
        let (rows, lost) = log.take();
        assert_eq!(
            rows,
            [row(TimerSource::NodeTask, 0), row(TimerSource::Record, 1)]
        );
        assert_eq!(lost, 0);
        for i in 0..TimerLog::CAP + 5 {
            (sink.0)(row(TimerSource::NodeTask, i as i64));
        }
        let (rows, lost) = log.take();
        assert_eq!((rows.len(), lost), (TimerLog::CAP, 5));
        assert_eq!(log.take(), (Vec::new(), 0));
        // The harness's own timers count past the cap too.
        for i in 0..TimerLog::CAP + 3 {
            log.push(row(TimerSource::Writer, i as i64));
        }
        let (rows, lost) = log.take();
        assert_eq!((rows.len(), lost), (TimerLog::CAP, 3));
    }

    /// The probe's timers: its delays in turn, one a period, each kept as
    /// it fires, from the period after a late start, on a clock whose
    /// timers fire on time.
    #[test]
    fn the_probe_arms_its_delays_in_turn() {
        use crate::node::{PROBE_DELAYS_MS, PROBE_PERIOD_MS, Runtime, probe};
        use std::cell::Cell;
        use std::future::Future;
        use std::pin::pin;
        use std::task::{Context, Poll, Waker};

        /// A clock that jumps to each deadline it is waited for.
        struct Jump(Cell<u64>);
        impl Runtime for Jump {
            fn now_ms(&self) -> u64 {
                self.0.get()
            }
            fn sleep_until_ms(&self, at_ms: u64) -> impl Future<Output = ()> {
                self.0.set(self.0.get().max(at_ms));
                std::future::ready(())
            }
            fn cpu_ms(&self) -> Option<u64> {
                None
            }
            fn emit(&self, _: String) {}
            fn error(&self, _: String) {}
        }
        let start = 1_700_000_000_000;
        let rt = Jump(Cell::new(start + 70));
        let log = TimerLog::new(TimerContext::Task);
        let fut = pin!(probe(&rt, &log, start, start + 10 * PROBE_PERIOD_MS));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(fut.poll(&mut cx), Poll::Ready(())));
        let (rows, lost) = log.take();
        assert_eq!(lost, 0);
        // It started 70 ms in: at the second period, the second delay.
        assert_eq!(rows.len(), 8);
        for (i, r) in rows.iter().enumerate() {
            let k = i as u64 + 2;
            let d = PROBE_DELAYS_MS[k as usize % PROBE_DELAYS_MS.len()];
            assert_eq!(r.source, TimerSource::Probe);
            assert_eq!(r.requested, Span::from_millis(d as i64));
            let armed = start + k * PROBE_PERIOD_MS;
            assert_eq!(r.armed, Time::from_unix_millis(armed as i64));
            assert_eq!(r.deadline, Time::from_unix_millis((armed + d) as i64));
            assert_eq!(r.fired, r.deadline);
        }
    }

    /// A line's timers read back as they were written, to the µs, times
    /// before the origin too.
    #[test]
    fn timers_read_back_as_written() {
        let origin = Time::from_unix_millis(1_700_000_000_010);
        let rows = [row(TimerSource::Probe, 0), row(TimerSource::Pacer, 25)];
        let json = timers_json(&rows, 7, TimerContext::Worker, origin);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["rows"][0][1], -12_500);
        let (back, lost, context) = timers_of(&v, origin).unwrap();
        assert_eq!(
            (back.as_slice(), lost, context),
            (&rows[..], 7, TimerContext::Worker)
        );
    }

    /// The log keeps the tap's parent checks alone, up to its cap, and
    /// counts as lost those past it and the events the tap left out.
    #[test]
    fn the_decision_log_keeps_parent_checks_and_counts_the_rest_lost() {
        let log = DecisionLog::default();
        let sink = log.sink();
        let choice = ParentChoice {
            action: ParentAction::Graft,
            parent: None,
            best: None,
            runner_up: None,
            unready: 0,
            subscriber: 0,
            backed_off: 0,
            stale: 0,
            behind: 0,
            votes: 0,
            parent_have: None,
            parent_push: None,
        };
        let event = |kind| {
            DecisionLogEntry::Decision(DecisionInfo {
                at: Time::from_unix_millis(1),
                swarm: None,
                peer: None,
                runner_up: None,
                parent: None,
                kind,
            })
        };
        for _ in 0..DecisionLog::CAP + 2 {
            (sink.0)(event(TapKind::ParentChoice { stream: 1, choice }));
        }
        (sink.0)(event(TapKind::HolderJoined));
        (sink.0)(DecisionLogEntry::Lost(3));
        let (checks, lost) = log.take();
        assert_eq!((checks.len(), lost), (DecisionLog::CAP, 5));
        assert!(
            checks
                .iter()
                .all(|d| matches!(d.kind, TapKind::ParentChoice { stream: 1, .. }))
        );
        assert_eq!(log.take().1, 0);
    }
}
