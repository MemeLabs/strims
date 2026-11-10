// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Candidate upload meters: the units' own bursts as trains ([`Train`]),
//! and libwebrtc's GCC over them ([`Gcc`]).

use std::collections::{BTreeMap, VecDeque};

use strims_base::time::{Duration, Time};

use super::meters::{ALLOC_UTILIZATION, Class, Ctx, Meter, Obs, Reading, Report};

/// Of `xs`, the largest a minority of liars can't raise: with `n` values
/// and fewer than a third lying, the `⌊n/3⌋`-th from the top is at most an
/// honest one. With fewer than three, the least.
fn robust_high(xs: &mut [f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| b.total_cmp(a));
    if xs.len() < 3 {
        return xs.last().copied();
    }
    xs.get(xs.len() / 3).copied()
}

fn quantile(xs: &mut [f64], p: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(f64::total_cmp);
    let i = ((xs.len() as f64 * p) as usize).min(xs.len() - 1);
    xs.get(i).copied()
}

/// A unit's frames on one link as the node wrote them.
#[derive(Clone, Debug, Default)]
struct Sent {
    /// When the node queued its first frame, and when the link took each,
    /// in order.
    queued: Option<Time>,
    taken: Vec<Time>,
    bytes: u64,
}

/// What the node sent each link, by unit, kept for the reports.
#[derive(Debug, Default)]
struct SentLog {
    units: BTreeMap<(usize, u32), Sent>,
}

impl SentLog {
    fn observe(&mut self, now: Time, obs: &Obs<'_>) {
        match *obs {
            Obs::Queued {
                link,
                bytes,
                class: Class::Push,
                unit: Some(u),
            } => {
                let s = self.units.entry((link, u)).or_default();
                s.queued.get_or_insert(now);
                s.bytes += bytes;
            }
            Obs::Taken {
                link,
                class: Class::Push,
                unit: Some(u),
                ..
            } => {
                if let Some(s) = self.units.get_mut(&(link, u)) {
                    s.taken.push(now);
                }
            }
            _ => {}
        }
        while self.units.len() > 4096 {
            self.units.pop_first();
        }
    }

    fn get(&self, link: usize, unit: u32) -> Option<&Sent> {
        self.units.get(&(link, unit))
    }
}

/// One unit's dispersion on one link, as its neighbour reported it.
#[derive(Clone, Copy, Debug)]
struct Sample {
    at: Time,
    /// Its frames after the first, over the time from the first frame's
    /// read to the last's.
    rate: f64,
    span_us: i64,
    /// The first frame's read in the neighbour's clock less the unit's
    /// queueing in the node's: the path's delay plus the clocks' offset,
    /// plus whatever the first frame waited behind.
    first_owd_us: i64,
}

/// The units' own bursts as trains: a child reads each pushed unit's frames
/// as its link delivers them, and reports when (in its own clock). Frames
/// that went out together leave the uplink at its rate while it holds them,
/// so a unit's frames after the first that waited behind the one before
/// (taken within the first half of the gap between their reads), over those
/// gaps, is a rate the uplink drained at: the uplink's, unless the link's
/// window, the child's downlink or a gap held them back, so a lower bound.
/// Jitter that delays the first frame more than the last shortens the span:
/// as CapProbe keeps the pair whose delays sum least, a sample counts only
/// while its first frame's delay (its read less its taking) came within
/// [`CLEAN_SHARE`] of the span of the link's [`CLEAN_BASE`] quantile over
/// the window ([`TRAIN_WINDOW`]).
/// Each link reads the largest of its clean samples, and the node the
/// largest a third of liars can't raise (`robust_high`).
pub struct Train {
    name: String,
    /// Keep samples whatever their first frame's delay.
    unfiltered: bool,
    /// The pooled clean samples' 60th percentile, not each link's largest.
    pool: bool,
    /// The clean filter's base: this quantile of the link's first-frame
    /// delays.
    base: f64,
    sent: SentLog,
    samples: BTreeMap<usize, VecDeque<Sample>>,
}

/// The span a train meter reads its samples over.
pub const TRAIN_WINDOW: Duration = Duration::from_secs(2);
/// The shortest span a sample may have: shorter ones read the receiver's
/// read granularity.
pub const MIN_SPAN_US: i64 = 1_000;
/// How far a clean sample's first frame may come behind the base, as a
/// share of its span ...
pub const CLEAN_SHARE: f64 = 0.1;
/// ... the base being this quantile of the link's first-frame delays over
/// the window: the least lets through only samples no jitter compressed,
/// but rejects most of them after a latency step, under a competing
/// upload's sawtooth and on lossy links; the median lets Wi-Fi's jitter
/// through.
pub const CLEAN_BASE: f64 = 0.25;

impl Train {
    /// `train`, `train-min` (the clean filter's base the link's least
    /// first-frame delay), `train-med` (its median), `train-raw` (no filter)
    /// or `train-pool`.
    pub fn parse(name: &str) -> Option<Train> {
        let (unfiltered, pool, base) = match name {
            "train" => (false, false, CLEAN_BASE),
            "train-min" => (false, false, 0.0),
            "train-med" => (false, false, 0.5),
            "train-raw" => (true, false, 0.0),
            "train-pool" => (false, true, CLEAN_BASE),
            _ => return None,
        };
        Some(Train {
            name: name.to_owned(),
            unfiltered,
            pool,
            base,
            sent: SentLog::default(),
            samples: BTreeMap::new(),
        })
    }

    fn report(&mut self, now: Time, link: usize, report: &Report) {
        for u in &report.units {
            let Some(sent) = self.sent.get(link, u.unit) else {
                continue;
            };
            if u.frames.len() < 2 || sent.taken.len() != u.frames.len() {
                continue;
            }
            let (Some(&(t0, _)), Some(&taken0)) = (u.frames.first(), sent.taken.first()) else {
                continue;
            };
            // The frames that waited behind the one before: taken within the
            // first half of the gap between their reads, so most of the gap
            // was the uplink draining them (a frame taken once its
            // predecessor had left shows only when it was taken: Salsify's
            // grace).
            let (mut bytes, mut span) = (0u64, 0i64);
            for (k, w) in u.frames.windows(2).enumerate() {
                let (Some(&(ta, _)), Some(&(tb, b))) = (w.first(), w.get(1)) else {
                    continue;
                };
                let (Some(&sa), Some(&sb)) = (sent.taken.get(k), sent.taken.get(k + 1)) else {
                    continue;
                };
                if 2 * (sb - sa).as_micros() <= tb - ta {
                    bytes += u64::from(b);
                    span += tb - ta;
                }
            }
            if span < MIN_SPAN_US {
                continue;
            }
            let s = self.samples.entry(link).or_default();
            s.push_back(Sample {
                at: now,
                rate: bytes as f64 / (span as f64 / 1e6),
                span_us: span,
                first_owd_us: t0 - (taken0 - Time::EPOCH).as_micros(),
            });
            while s.front().is_some_and(|x| now - x.at > TRAIN_WINDOW) {
                s.pop_front();
            }
        }
    }

    /// Each link's clean samples in the window.
    fn clean(&self, now: Time) -> BTreeMap<usize, Vec<f64>> {
        let mut out = BTreeMap::new();
        for (&link, s) in &self.samples {
            let fresh: Vec<&Sample> = s.iter().filter(|x| now - x.at <= TRAIN_WINDOW).collect();
            let mut owds: Vec<f64> = fresh.iter().map(|x| x.first_owd_us as f64).collect();
            let Some(base) = quantile(&mut owds, self.base) else {
                continue;
            };
            let base = base as i64;
            let rates: Vec<f64> = fresh
                .iter()
                .filter(|x| {
                    self.unfiltered
                        || (x.first_owd_us - base) as f64 <= CLEAN_SHARE * x.span_us as f64
                })
                .map(|x| x.rate)
                .collect();
            if !rates.is_empty() {
                out.insert(link, rates);
            }
        }
        out
    }
}

impl Meter for Train {
    fn name(&self) -> &str {
        &self.name
    }

    fn observe(&mut self, now: Time, obs: &Obs<'_>) {
        self.sent.observe(now, obs);
        if let Obs::Report { link, report } = *obs {
            self.report(now, link, report);
        }
    }

    fn reading(&mut self, now: Time, _ctx: &Ctx) -> Reading {
        let clean = self.clean(now);
        let links = clean.len();
        let capacity = if self.pool {
            let mut all: Vec<f64> = clean.into_values().flatten().collect();
            quantile(&mut all, 0.6)
        } else {
            let mut best: Vec<f64> = clean
                .into_values()
                .filter_map(|r| r.into_iter().reduce(f64::max))
                .collect();
            robust_high(&mut best)
        };
        Reading {
            capacity,
            diag: vec![("links", links as f64)],
        }
    }
}

/// libwebrtc's GCC over the units' bursts, its receiver timestamps traded
/// for aggregates a minority of liars can't move.
///
/// - Each link's detector takes each unit as a send group: the change of
///   its last frame's read (the neighbour's clock) less that of its taking
///   (the node's), accumulated, smoothed (0.9) and fit by least squares over
///   the last 20 groups; the slope, scaled by the groups and 4, against a
///   threshold that adapts (up 0.0087, down 0.039 per ms, within [6, 600],
///   frozen past 15 above it), as `TrendlineEstimator` does. The node
///   overuses while the median of its fresh links' margins does.
/// - The estimate falls to 0.85 of the delivered rate at an overuse (once a
///   round trip, 200 ms), and otherwise grows 8 % a second, to at most 1.5
///   times the delivered rate, as `AimdRateControl` does. The delivered rate
///   is each link's reported bytes, at most what the node sent it, summed:
///   a neighbour can lower its own share, never raise it.
/// - App-limited, the delivered rate is what the node needs, so it probes
///   with real media: every [`GCC_PROBE_EVERY`] without an overuse it pushes
///   one more copy for [`GCC_PROBE_SPAN`]; a probe whose delivered rate
///   falls under 0.9 of its sent rate found the uplink (0.95 of what it
///   delivered), as `ProbeBitrateEstimator` reads a cluster.
pub struct Gcc {
    sent: SentLog,
    links: BTreeMap<usize, Trend>,
    /// Bytes each link took, and its neighbour reported reading, by when.
    took: BTreeMap<usize, VecDeque<(Time, u64)>>,
    got: BTreeMap<usize, VecDeque<(Time, u64)>>,
    estimate: Option<f64>,
    last_cut: Option<Time>,
    last_update: Option<Time>,
    last_overuse: Option<Time>,
    probe_until: Option<Time>,
    next_probe: Option<Time>,
    probe_start: Option<Time>,
    /// The copies the probe in progress adds, and the most it sent.
    probe_extra: usize,
    probe_rate: f64,
}

pub const GCC_PROBE_EVERY: Duration = Duration::from_secs(5);
pub const GCC_PROBE_SPAN: Duration = Duration::SECOND;
const RATE_SPAN: Duration = Duration::SECOND;
const TREND_GROUPS: usize = 20;

#[derive(Debug)]
struct Trend {
    last: Option<(i64, Time)>,
    acc_ms: f64,
    smoothed: f64,
    points: VecDeque<(f64, f64)>,
    first_arrival: Option<i64>,
    threshold: f64,
    last_threshold_update: Option<i64>,
    modified: f64,
    at: Option<Time>,
}

impl Default for Trend {
    fn default() -> Trend {
        Trend {
            last: None,
            acc_ms: 0.0,
            smoothed: 0.0,
            points: VecDeque::new(),
            first_arrival: None,
            threshold: 12.5,
            last_threshold_update: None,
            modified: 0.0,
            at: None,
        }
    }
}

impl Trend {
    /// A group read at `recv_us` (the neighbour's clock), taken at `sent`.
    fn group(&mut self, now: Time, recv_us: i64, sent: Time) {
        if let Some((r0, s0)) = self.last {
            let d_ms = (recv_us - r0) as f64 / 1e3 - (sent - s0).as_secs_f64() * 1e3;
            self.acc_ms += d_ms;
            self.smoothed = 0.9 * self.smoothed + 0.1 * self.acc_ms;
            let first = *self.first_arrival.get_or_insert(recv_us);
            self.points
                .push_back(((recv_us - first) as f64 / 1e3, self.smoothed));
            while self.points.len() > TREND_GROUPS {
                self.points.pop_front();
            }
            if self.points.len() >= TREND_GROUPS {
                let n = self.points.len() as f64;
                let mx = self.points.iter().map(|p| p.0).sum::<f64>() / n;
                let my = self.points.iter().map(|p| p.1).sum::<f64>() / n;
                let num: f64 = self.points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
                let den: f64 = self.points.iter().map(|p| (p.0 - mx).powi(2)).sum();
                if den > 0.0 {
                    let slope = num / den;
                    self.modified = (self.points.len().min(60) as f64) * slope * 4.0;
                    let dt = self
                        .last_threshold_update
                        .map_or(0.0, |t| ((recv_us - t) as f64 / 1e3).min(100.0));
                    self.last_threshold_update = Some(recv_us);
                    let m = self.modified.abs();
                    if m <= self.threshold + 15.0 {
                        let k = if m < self.threshold { 0.039 } else { 0.0087 };
                        self.threshold =
                            (self.threshold + k * (m - self.threshold) * dt).clamp(6.0, 600.0);
                    }
                    self.at = Some(now);
                }
            }
        }
        self.last = Some((recv_us, sent));
    }

    fn margin(&self) -> f64 {
        self.modified - self.threshold
    }
}

fn rate(q: &VecDeque<(Time, u64)>, now: Time) -> f64 {
    q.iter()
        .filter(|&&(t, _)| now - t <= RATE_SPAN)
        .map(|&(_, b)| b)
        .sum::<u64>() as f64
        / RATE_SPAN.as_secs_f64()
}

impl Gcc {
    pub fn new() -> Gcc {
        Gcc {
            sent: SentLog::default(),
            links: BTreeMap::new(),
            took: BTreeMap::new(),
            got: BTreeMap::new(),
            estimate: None,
            last_cut: None,
            last_update: None,
            last_overuse: None,
            probe_until: None,
            next_probe: None,
            probe_start: None,
            probe_extra: 0,
            probe_rate: 0.0,
        }
    }

    /// Each link's delivered rate, at most what the node sent it, summed;
    /// and what it sent.
    fn delivered(&self, now: Time) -> (f64, f64) {
        let mut d = 0.0;
        let mut s = 0.0;
        for (link, q) in &self.took {
            let sent = rate(q, now);
            let got = self.got.get(link).map_or(0.0, |g| rate(g, now));
            d += got.min(sent);
            s += sent;
        }
        (d, s)
    }

    fn overusing(&self, now: Time) -> bool {
        let mut margins: Vec<f64> = self
            .links
            .values()
            .filter(|t| t.at.is_some_and(|a| now - a <= RATE_SPAN))
            .map(Trend::margin)
            .collect();
        quantile(&mut margins, 0.5).is_some_and(|m| m > 0.0)
    }
}

impl Default for Gcc {
    fn default() -> Gcc {
        Gcc::new()
    }
}

impl Meter for Gcc {
    fn name(&self) -> &str {
        "gcc"
    }

    fn observe(&mut self, now: Time, obs: &Obs<'_>) {
        self.sent.observe(now, obs);
        match *obs {
            Obs::Taken {
                link, bytes, class, ..
            } if class.runner() => {
                self.took.entry(link).or_default().push_back((now, bytes));
            }
            Obs::Report { link, report } => {
                self.got
                    .entry(link)
                    .or_default()
                    .push_back((now, report.bytes));
                for u in &report.units {
                    let Some(sent) = self.sent.get(link, u.unit) else {
                        continue;
                    };
                    let (Some(&taken), Some(&(last, _))) = (sent.taken.last(), u.frames.last())
                    else {
                        continue;
                    };
                    self.links.entry(link).or_default().group(now, last, taken);
                }
            }
            _ => {}
        }
        for q in self.took.values_mut().chain(self.got.values_mut()) {
            while q.front().is_some_and(|&(t, _)| now - t > RATE_SPAN * 2) {
                q.pop_front();
            }
        }
    }

    fn reading(&mut self, now: Time, ctx: &Ctx) -> Reading {
        let (delivered, sent) = self.delivered(now);
        let over = self.overusing(now);
        let dt = self.last_update.map_or(0.0, |t| (now - t).as_secs_f64());
        self.last_update = Some(now);
        if over {
            self.last_overuse = Some(now);
        }
        let probing = self.probe_until.is_some_and(|t| now < t);
        if over && self.probe_start.is_some() {
            // The probe queued the uplink: it found the capacity.
            self.probe_start = None;
            self.probe_rate = 0.0;
        }
        if let Some(until) = self.probe_until.filter(|_| self.probe_start.is_some()) {
            if probing {
                self.probe_rate = self.probe_rate.max(sent);
            } else if now - until >= GCC_PROBE_SPAN {
                // The probe and the round after it read no overuse: the
                // uplink carried what it sent.
                self.probe_start = None;
                self.estimate = Some(self.estimate.unwrap_or(0.0).max(self.probe_rate));
                self.probe_rate = 0.0;
            }
        }
        let est = match self.estimate {
            None if delivered > 0.0 => delivered,
            None => {
                return Reading::default();
            }
            Some(e) => e,
        };
        let est = if over
            && self
                .last_cut
                .is_none_or(|t| now - t >= Duration::from_millis(200))
        {
            self.last_cut = Some(now);
            if delivered > 0.0 {
                0.85 * delivered
            } else {
                est
            }
        } else if over {
            est
        } else {
            let grown = est * 1.08f64.powf(dt);
            grown.min((1.5 * sent).max(est))
        };
        self.estimate = Some(est);
        if self.next_probe.is_none() {
            self.next_probe = Some(now + GCC_PROBE_EVERY);
        }
        if !probing
            && self.next_probe.is_some_and(|t| now >= t)
            && self.last_overuse.is_none_or(|t| now - t >= GCC_PROBE_EVERY)
        {
            // As many copies as the budget's next one needs shown.
            let need = ((ctx.copies + 1) as f64 / ALLOC_UTILIZATION).ceil() as usize;
            self.probe_extra = need.saturating_sub(ctx.copies).max(1);
            self.probe_until = Some(now + GCC_PROBE_SPAN);
            self.probe_start = Some(now);
            self.next_probe = Some(now + GCC_PROBE_EVERY);
        }
        let mut margins: Vec<f64> = self.links.values().map(Trend::margin).collect();
        Reading {
            capacity: Some(est),
            diag: vec![
                ("delivered", delivered),
                ("sent", sent),
                ("over", f64::from(u8::from(over))),
                ("margin", quantile(&mut margins, 0.5).unwrap_or(0.0)),
                ("probing", f64::from(u8::from(probing))),
            ],
        }
    }

    fn extra_copies(&mut self, now: Time, _ctx: &Ctx) -> usize {
        if self.probe_until.is_some_and(|t| now < t) {
            self.probe_extra
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minority_of_liars_cant_raise_the_reading() {
        let mut honest = vec![10.0, 11.0, 12.0, 9.0, 10.5, 11.5];
        let clean = robust_high(&mut honest.clone());
        honest.extend([1e9, 1e9]);
        let lied = robust_high(&mut honest);
        assert!(lied <= Some(12.0), "{lied:?}");
        assert!(clean <= lied);
        assert_eq!(robust_high(&mut [5.0, 7.0]), Some(5.0));
        assert_eq!(robust_high(&mut []), None);
    }
}
