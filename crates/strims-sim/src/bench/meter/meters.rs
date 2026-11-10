// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Upload meters over what every transport offers ([`Obs`]), the
//! references among them ([`Main`], [`Oracle`]), and the budget their
//! readings drive ([`Budget`]).

use std::collections::VecDeque;

use strims_base::path::{LATEST_MAX_AGE, PathMeter, PathReading, aggregate, latest_excess};
use strims_base::stats::Sma;
use strims_base::time::{Duration, Time};

use super::plan::{Condition, Role};

/// What a write carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// A unit's frame pushed to a child.
    Push,
    /// A chunk's frame that answers a request.
    Answer,
    /// The protocol's control frames: STATEs, HAVEs, requests, NOT_YETs,
    /// pings.
    Control,
    /// The link measure's probes and echoes.
    Probe,
    /// A meter's padding.
    Pad,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Push => "push",
            Class::Answer => "answer",
            Class::Control => "control",
            Class::Probe => "probe",
            Class::Pad => "pad",
        }
    }

    /// Whether the protocol's runner writes it (its upload meter counts it).
    pub fn runner(self) -> bool {
        matches!(self, Class::Push | Class::Answer | Class::Control)
    }
}

/// A unit's arrival at a neighbour, as it reports it: its read times in
/// its own clock (µs), which the node can't relate to its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitReport {
    pub unit: u32,
    /// When the read that brought the unit's first byte returned, and the
    /// one that completed its last frame.
    pub first_us: i64,
    pub last_us: i64,
    pub bytes: u32,
    /// Each frame's completion and bytes, in order.
    pub frames: Vec<(i64, u32)>,
}

/// A neighbour's report, in its STATE.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Units it completed since its last report.
    pub units: Vec<UnitReport>,
    /// Bytes it read from the link since its last report, and when its
    /// clock read that report's and this one's.
    pub bytes: u64,
    pub from_us: i64,
    pub to_us: i64,
}

/// One observation, as the node's link layer sees it.
#[derive(Clone, Copy, Debug)]
pub enum Obs<'a> {
    /// The node queued bytes for a link, above its link buffer (the
    /// runner's queue).
    Queued {
        link: usize,
        bytes: u64,
        class: Class,
        unit: Option<u32>,
    },
    /// The link's writer took them into the link's stream.
    Taken {
        link: usize,
        bytes: u64,
        class: Class,
        unit: Option<u32>,
    },
    /// Queued bytes the node dropped (their neighbour left).
    Dropped {
        link: usize,
        bytes: u64,
    },
    /// The transport took a write of `bytes` begun at `began`.
    Wrote {
        link: usize,
        bytes: u64,
        began: Time,
    },
    /// A probe went into the link's stream, after the bytes taken so far.
    Probe {
        link: usize,
        nonce: u32,
    },
    /// Its echo came back.
    Echo {
        link: usize,
        nonce: u32,
    },
    Report {
        link: usize,
        report: &'a Report,
    },
    /// The transport's unsent bytes (a data channel's buffered amount).
    Unsent {
        link: usize,
        bytes: u64,
    },
    /// The link's neighbour joined or left.
    Present {
        link: usize,
        present: bool,
    },
}

/// What the node's policy tells its meters at each reading.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub role: Role,
    /// The budget was lowered at least once (main's window was cut).
    pub cut: bool,
    pub copies: usize,
    /// A copy's rate, bytes per second.
    pub copy_rate: f64,
    /// The run's elapsed time.
    pub elapsed: Duration,
}

/// A meter's reading: the uplink it reads, bytes per second, and what it
/// read it from.
#[derive(Clone, Debug, Default)]
pub struct Reading {
    pub capacity: Option<f64>,
    pub diag: Vec<(&'static str, f64)>,
}

/// An upload meter.
pub trait Meter {
    fn name(&self) -> &str;
    fn observe(&mut self, now: Time, obs: &Obs<'_>);
    fn reading(&mut self, now: Time, ctx: &Ctx) -> Reading;
    /// Padding the meter asks for now, per link (driving meter only).
    fn padding(&mut self, _now: Time, _ctx: &Ctx) -> Vec<(usize, u64)> {
        Vec::new()
    }
    /// Copies (children) past the budget's the meter asks for now, a probe
    /// with real media (driving meter only).
    fn extra_copies(&mut self, _now: Time, _ctx: &Ctx) -> usize {
        0
    }
}

/// The meters by name, for `cond` on `links` links.
pub fn build(
    name: &str,
    cond: &Condition,
    links: usize,
    start: Time,
) -> Result<Box<dyn Meter>, String> {
    Ok(match name {
        "main" => Box::new(Main::new(links, None)),
        // Main told its site's uplink, as STRIMS_SIM_KNOWN_UPLINK tells it.
        "main-known" => Box::new(Main::new(links, Some(Oracle::nominal(cond) as f64))),
        "oracle" => Box::new(Oracle::new(cond, start, false)),
        "oracle-static" => Box::new(Oracle::new(cond, start, true)),
        "gcc" => Box::new(super::candidates::Gcc::new()),
        _ => {
            if let Some(d) = super::candidates::Train::parse(name) {
                return Ok(Box::new(d));
            }
            return Err(format!("no meter {name}"));
        }
    })
}

/// The true uplink at each instant, or with `fixed` the site's nominal
/// one (`STRIMS_SIM_KNOWN_UPLINK`).
pub struct Oracle {
    name: &'static str,
    up: u64,
    steps: Vec<(Duration, u64)>,
    start: Time,
}

impl Oracle {
    /// The site's uplink: the largest the run's steps give it.
    pub fn nominal(cond: &Condition) -> u64 {
        cond.up_steps
            .iter()
            .map(|&(_, r)| r)
            .chain([cond.up])
            .max()
            .unwrap_or(cond.up)
    }

    pub fn new(cond: &Condition, start: Time, fixed: bool) -> Oracle {
        let nominal = Oracle::nominal(cond);
        Oracle {
            name: if fixed { "oracle-static" } else { "oracle" },
            up: if fixed { nominal } else { cond.up },
            steps: if fixed {
                Vec::new()
            } else {
                cond.up_steps.clone()
            },
            start,
        }
    }
}

impl Meter for Oracle {
    fn name(&self) -> &str {
        self.name
    }

    fn observe(&mut self, _now: Time, _obs: &Obs<'_>) {}

    fn reading(&mut self, now: Time, _ctx: &Ctx) -> Reading {
        let t = now - self.start;
        let up = self
            .steps
            .iter()
            .rev()
            .find(|&&(at, _)| at <= t)
            .map_or(self.up, |&(_, r)| r);
        Reading {
            capacity: Some(up as f64),
            diag: Vec::new(),
        }
    }
}

/// The node's budget from a meter's reading, as main's adaptive seed
/// takes it: every [`ALLOC_CHECK_INTERVAL`], once the stream was measured
/// for [`ALLOC_MEASURE`], the copies [`ALLOC_UTILIZATION`] of the reading
/// pays for at a copy's rate, one at least, taken when they rise or fall
/// past [`OVER_BUDGET`]. A relay takes as many children.
#[derive(Clone, Debug, Default)]
pub struct Budget {
    pub budget: usize,
    pub measured: bool,
    /// Lowered at least once.
    pub cut: bool,
    measuring_since: Option<Time>,
    next_check: Option<Time>,
}

pub const ALLOC_UTILIZATION: f64 = 0.7;
pub const ALLOC_CHECK_INTERVAL: Duration = Duration::SECOND;
pub const ALLOC_MEASURE: Duration = Duration::from_secs(2);
pub const OVER_BUDGET: f64 = 0.1;

impl Budget {
    /// Whether a check is due.
    pub fn due(&self, now: Time) -> bool {
        self.next_check.is_none_or(|t| now >= t)
    }

    /// Checks the budget on `capacity` at `rate` a copy. Returns the
    /// copies.
    pub fn check(&mut self, now: Time, capacity: Option<f64>, rate: f64) -> usize {
        self.next_check = Some(now + ALLOC_CHECK_INTERVAL);
        if rate > 0.0 && self.measuring_since.is_none() {
            self.measuring_since = Some(now);
        }
        let measuring = self
            .measuring_since
            .is_some_and(|s| now - s >= ALLOC_MEASURE);
        if let Some(cap) = capacity.filter(|_| measuring && rate > 0.0) {
            let target = ((cap * ALLOC_UTILIZATION).max(0.0) / rate) as usize;
            let target = target.max(1);
            if !self.measured || target > self.budget {
                self.budget = target;
            } else if (target as f64) < self.budget as f64 * (1.0 - OVER_BUDGET) {
                self.budget = target;
                self.cut = true;
            }
            self.measured = true;
        }
        if self.measured { self.budget.max(1) } else { 1 }
    }
}

// Main's upload meter (strims-ppspp's runner and push scheduler), ported.

const MIN_BUSY: f64 = 0.05;
const SATURATED_BUSY: f64 = 0.5;
const QUEUE_FORMING: Duration = Duration::from_millis(10);
const QUEUE_FORMED: Duration = Duration::from_millis(25);
const RELAY_FLOOR: f64 = 32_000.0;
const RELAY_GROWTH: f64 = 4.0;
const RELAY_GROWTH_FORMING: f64 = 1.5;
const SEED_GROWTH: f64 = 1.5;
const DRAIN_QUEUE: Duration = Duration::from_millis(50);
const DRAIN_PROBE: f64 = 0.25;
const DRAIN_PROBE_PERIOD: Duration = Duration::from_secs(10);
const APP_LIMITED_QUEUED: u64 = 16 * 1024;
const HELD_SHARE: f64 = 0.5;
const HELD_QUEUE: Duration = Duration::from_millis(50);
const HELD_SPAN: Duration = Duration::from_millis(500);
const LATEST_FLOOR_SPAN: Duration = Duration::SECOND;
const PEAK_SPAN: Duration = Duration::from_secs(10);

fn takes_drained(drained: Option<f64>, delivered: f64, waited: bool, own_queued: u64) -> bool {
    delivered > 0.0
        && (waited || own_queued >= APP_LIMITED_QUEUED || drained.is_some_and(|d| delivered >= d))
}

fn window(link: Option<PathReading>) -> f64 {
    link.map_or(0.0, |r| {
        r.rate.unwrap_or(0) as f64 * r.base.map_or(0.0, |b| b.as_secs_f64())
    })
}

fn holds_back(link: Option<PathReading>, queued_for: Duration, best: [f64; 2]) -> bool {
    let Some(r) = link else {
        return false;
    };
    let w = window(Some(r));
    let other = if w >= best[0] { best[1] } else { best[0] };
    queued_for >= HELD_SPAN
        && w < HELD_SHARE * other
        && r.base.is_some()
        && (r.latest_excess() >= HELD_QUEUE || r.excess().is_some_and(|e| e >= HELD_QUEUE))
}

#[derive(Debug)]
struct UploadMeter {
    busy_since: Option<Time>,
    busy: Sma,
    budget_busy: Sma,
    budget_since: Option<Time>,
    bytes: Sma,
    queue_delay: Option<Duration>,
    latest_delay: Option<Duration>,
    excesses: Vec<Duration>,
    latest_history: VecDeque<(Time, Duration)>,
    delivered: f64,
    drained: Option<(f64, Time)>,
    standing: bool,
    written_peak: Option<(f64, Time)>,
}

/// What [`UploadMeter::info`] reads, as main's `UploadInfo`.
#[derive(Clone, Copy, Debug, Default)]
struct UploadInfo {
    hint: Option<f64>,
    rate: u64,
    busy: f64,
    budget_busy: f64,
    latest_delay: Option<Duration>,
    latest_floor: Option<Duration>,
    drained: Option<f64>,
    draining: bool,
    written_peak: f64,
}

impl UploadInfo {
    fn bounded_capacity(&self, queue: Option<Duration>, most: f64) -> Option<f64> {
        let queue = queue.unwrap_or(Duration::ZERO);
        let growth = if queue < QUEUE_FORMING {
            most
        } else if queue < QUEUE_FORMED {
            RELAY_GROWTH_FORMING.min(most)
        } else {
            1.0
        };
        let wrote = self.written_peak.max(self.rate as f64).max(RELAY_FLOOR);
        self.capacity_within(growth * wrote)
    }

    fn capacity_within(&self, bound: f64) -> Option<f64> {
        let measured =
            (self.rate > 0).then(|| (self.rate as f64 / self.busy.max(MIN_BUSY)).min(bound));
        match (measured, self.hint) {
            (Some(m), Some(h)) if self.busy >= SATURATED_BUSY => Some(m.min(h)),
            (_, Some(h)) => Some(h),
            (_, None) if self.draining => self.drained,
            (Some(m), None) => Some(self.drained.map_or(m, |d| m.min(d))),
            (None, None) => None,
        }
    }
}

impl UploadMeter {
    fn new() -> UploadMeter {
        UploadMeter {
            busy_since: None,
            busy: Sma::new(20, Duration::from_millis(100), Time::EPOCH),
            budget_busy: Sma::new(20, Duration::from_millis(100), Time::EPOCH),
            budget_since: None,
            bytes: Sma::new(20, Duration::from_millis(100), Time::EPOCH),
            queue_delay: None,
            latest_delay: None,
            excesses: Vec::new(),
            latest_history: VecDeque::new(),
            delivered: 0.0,
            drained: None,
            standing: false,
            written_peak: None,
        }
    }

    fn update_queue_delay(&mut self, paths: &[Option<PathReading>], now: Time) {
        self.excesses.clear();
        self.excesses
            .extend(paths.iter().filter_map(|p| p.as_ref()?.excess()));
        self.queue_delay = aggregate(&mut self.excesses);
        self.latest_delay = latest_excess(paths.iter().filter_map(|p| *p), now, &mut self.excesses);
        if let Some(d) = self.latest_delay {
            self.latest_history.push_back((now, d));
        }
        while self
            .latest_history
            .front()
            .is_some_and(|&(t, _)| now - t > LATEST_FLOOR_SPAN)
        {
            self.latest_history.pop_front();
        }
        self.delivered = paths
            .iter()
            .filter_map(|p| *p)
            .filter(|r| r.age(now) <= LATEST_MAX_AGE)
            .filter_map(|r| r.rate)
            .sum::<u64>() as f64;
        self.standing = self.queue_delay.is_some_and(|d| d >= DRAIN_QUEUE);
        if self.standing {
            let delivered: u64 = paths.iter().filter_map(|p| p.as_ref()?.rate).sum();
            let own: u64 = paths.iter().filter_map(|p| p.as_ref()?.queued).sum();
            let waited = self.busy.rate(Duration::SECOND, now) > 0;
            let drained = self.drained.map(|(d, _)| d);
            if takes_drained(drained, delivered as f64, waited, own) {
                self.drained = Some((delivered as f64, now));
            }
        }
    }

    fn observe(&mut self, now: Time, backlog: bool, budget: bool) {
        if let Some(t) = self.busy_since {
            let d = (now - t).as_nanos();
            if d > 0 {
                self.busy.add(d as u64, now);
            }
        }
        if let Some(t) = self.budget_since {
            let d = (now - t).as_nanos();
            if d > 0 {
                self.budget_busy.add(d as u64, now);
            }
        }
        self.budget_since = budget.then_some(now);
        self.busy_since = backlog.then_some(now);
    }

    fn info(&mut self, now: Time) -> UploadInfo {
        let busy = self.busy.rate(Duration::SECOND, now) as f64 / 1e9;
        let budget_busy = self.budget_busy.rate(Duration::SECOND, now) as f64 / 1e9;
        let rate = self.bytes.rate(Duration::SECOND, now);
        match self.written_peak {
            Some((peak, at)) if now - at <= PEAK_SPAN && peak > rate as f64 => {}
            _ => self.written_peak = Some((rate as f64, now)),
        }
        UploadInfo {
            hint: None,
            written_peak: self.written_peak.map_or(0.0, |(peak, _)| peak),
            rate,
            busy: busy.clamp(0.0, 1.0),
            budget_busy: budget_busy.clamp(0.0, 1.0),
            latest_delay: self.latest_delay,
            latest_floor: self.latest_history.iter().map(|&(_, d)| d).min(),
            drained: self.drained.map(|(c, at)| {
                c * (1.0
                    + DRAIN_PROBE * (now - at).as_secs_f64() / DRAIN_PROBE_PERIOD.as_secs_f64())
            }),
            draining: self.standing && self.drained.is_some_and(|(_, at)| at == now),
        }
    }
}

#[derive(Debug, Default)]
struct MainLink {
    path: PathMeter,
    /// Runner bytes queued and not yet taken.
    pending: u64,
    queued_since: Option<Time>,
}

/// Main's meter: vnic's path measure of each link, and the runner's upload
/// meter over them, read as the seed's budget (`seed_capacity`) or a
/// relay's admission (`relay_capacity`) reads it.
pub struct Main {
    name: &'static str,
    links: Vec<MainLink>,
    up: UploadMeter,
    hint: Option<f64>,
}

impl Main {
    /// With `hint`, the capacity the driver knows.
    pub fn new(links: usize, hint: Option<f64>) -> Main {
        Main {
            name: if hint.is_some() { "main-known" } else { "main" },
            links: (0..links).map(|_| MainLink::default()).collect(),
            up: UploadMeter::new(),
            hint,
        }
    }

    fn paths(&self, now: Time) -> Vec<Option<PathReading>> {
        self.links.iter().map(|l| l.path.reading(now)).collect()
    }

    fn observe_upload(&mut self, now: Time) {
        let paths = self.paths(now);
        let mut best = [0.0f64; 2];
        for p in &paths {
            let w = window(*p);
            if w > best[0] {
                best = [w, best[0]];
            } else if w > best[1] {
                best[1] = w;
            }
        }
        let (mut backlog, mut budget) = (false, false);
        for (l, p) in self.links.iter().zip(&paths) {
            let Some(since) = l.queued_since.filter(|_| l.pending > 0) else {
                continue;
            };
            backlog = true;
            budget |= !holds_back(*p, now - since, best);
        }
        self.up.observe(now, backlog, budget);
    }
}

impl Meter for Main {
    fn name(&self) -> &str {
        self.name
    }

    fn observe(&mut self, now: Time, obs: &Obs<'_>) {
        match *obs {
            Obs::Queued {
                link, bytes, class, ..
            } if class.runner() => {
                if let Some(l) = self.links.get_mut(link) {
                    if l.pending == 0 {
                        l.queued_since = Some(now);
                    }
                    l.pending += bytes;
                }
                self.observe_upload(now);
            }
            Obs::Taken {
                link, bytes, class, ..
            } => {
                if let Some(l) = self.links.get_mut(link) {
                    l.path.wrote(bytes, class != Class::Probe);
                    if class.runner() {
                        l.pending = l.pending.saturating_sub(bytes);
                    }
                }
                if class.runner() {
                    self.up.bytes.add(bytes, now);
                }
                self.observe_upload(now);
            }
            Obs::Dropped { link, bytes } => {
                if let Some(l) = self.links.get_mut(link) {
                    l.pending = l.pending.saturating_sub(bytes);
                }
                self.observe_upload(now);
            }
            Obs::Probe { link, nonce } => {
                if let Some(l) = self.links.get_mut(link) {
                    l.path.probe(nonce, now);
                }
            }
            Obs::Echo { link, nonce } => {
                if let Some(l) = self.links.get_mut(link) {
                    let _ = l.path.echo(nonce, now);
                }
            }
            _ => {}
        }
    }

    fn reading(&mut self, now: Time, ctx: &Ctx) -> Reading {
        self.observe_upload(now);
        let paths = self.paths(now);
        self.up.update_queue_delay(&paths, now);
        let info = UploadInfo {
            hint: self.hint,
            ..self.up.info(now)
        };
        let capacity = match ctx.role {
            // An adaptive seed's budget reads the busy time without the
            // links that hold their data back.
            Role::Seed => {
                let most = if ctx.cut { SEED_GROWTH } else { RELAY_GROWTH };
                UploadInfo {
                    busy: info.budget_busy,
                    ..info
                }
                .bounded_capacity(info.latest_floor, most)
            }
            Role::Relay => info.bounded_capacity(info.latest_delay, RELAY_GROWTH),
        };
        let ms = |d: Option<Duration>| d.map_or(-1.0, |d| d.as_secs_f64() * 1000.0);
        Reading {
            capacity,
            diag: vec![
                ("rate", info.rate as f64),
                ("busy", info.busy),
                ("budget_busy", info.budget_busy),
                ("floor_ms", ms(info.latest_floor)),
                ("latest_ms", ms(info.latest_delay)),
                ("drained", info.drained.unwrap_or(-1.0)),
                ("peak", info.written_peak),
            ],
        }
    }
}

/// vnic's rule for when a link probes: with a frame the owners fill once
/// [`PROBE_INTERVAL`](strims_base::path::PROBE_INTERVAL) has passed since
/// the last probe, if they wrote
/// [`BUSY_BYTES`](strims_base::path::BUSY_BYTES) since, or
/// [`QUIET_PROBE_INTERVAL`](strims_base::path::QUIET_PROBE_INTERVAL) if
/// they wrote less; never with [`MAX_PROBES`](strims_base::path::MAX_PROBES)
/// unanswered.
#[derive(Debug, Default)]
pub struct Prober {
    links: Vec<PathMeter>,
    nonce: u32,
}

impl Prober {
    pub fn new(links: usize) -> Prober {
        Prober {
            links: (0..links).map(|_| PathMeter::new()).collect(),
            nonce: 0x1000,
        }
    }

    /// Counts bytes into `link`'s stream.
    pub fn wrote(&mut self, link: usize, bytes: u64, data: bool) {
        if let Some(m) = self.links.get_mut(link) {
            m.wrote(bytes, data);
        }
    }

    /// A probe's nonce if one is due on `link` with a frame written now.
    pub fn due(&mut self, link: usize, now: Time) -> Option<u32> {
        let m = self.links.get_mut(link)?;
        if !m.probe_due(now, strims_base::path::PROBE_INTERVAL) {
            return None;
        }
        self.nonce = self.nonce.wrapping_add(0x9e37_79b9);
        Some(self.nonce)
    }

    /// The probe went out.
    pub fn probed(&mut self, link: usize, nonce: u32, now: Time) {
        if let Some(m) = self.links.get_mut(link) {
            m.probe(nonce, now);
        }
    }

    pub fn echo(&mut self, link: usize, nonce: u32, now: Time) {
        if let Some(m) = self.links.get_mut(link) {
            let _ = m.echo(nonce, now);
        }
    }
}
