// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Measures of the upload meter and of lag over a stream's length, and the
//! run's one-line JSON row (`STRIMS_SIM_ROWS=1`, and a sim farm job's).
//!
//! - **buckets** ([`Report::buckets`]): a swarm's stream in [`BUCKET`]s of
//!   stream time from its start. The first is the startup; the steady
//!   state is everything after it ([`Report::steady_lag`]).
//! - **roles** ([`Report::roles`]): the swarms' seeds; forwarders, the other
//!   hosts that answered a STREAM_REQUEST (accepted or declined); and the
//!   rest.
//! - **standing queue** ([`Report::standing_queue`]): the share of a set of
//!   hosts' uplink queue samples, while a swarm streams, that wait longer
//!   than [`STANDING_QUEUE`] (WAN mode).
//! - **wrongly declined** ([`Report::declined`]): STREAM_REQUESTs declined
//!   in samples where the node's upload rate was under half of what it
//!   could upload then.
//! - **tracking** ([`Report::tracking`]): after each change of a seed's or
//!   forwarder's available upload while a swarm streams (a step of its
//!   uplink's rate, or a competing upload's start or stop), the share of
//!   the meter's samples until the host's next change, or the stream's
//!   end, that read more than [`TRACK_BAND`] off what was available. A
//!   meter that swings between true and far off reads off for as long as
//!   it swings, where the time until it first reads true would turn on
//!   chance.
//! - **lag after changes** ([`Report::lag_after_changes`]): the lag of the
//!   frames a swarm's seed wrote in the [`AFTER_CHANGE`] after each drop, or
//!   each rise, of its available upload.
//! - **queueing delay accuracy** ([`Report::queue_delay_accuracy`]): the
//!   meter's queueing delay against the standing queue on the host's
//!   uplink, the least wait of the WAN model's queue over the last
//!   [`STANDING_WINDOW`], and whether the delay reads past a threshold
//!   exactly when the uplink stayed busy throughout that window (WAN
//!   mode). vnic's queued bytes, which read the queue as it is rather than
//!   its floor, are also scored against the wait at the moment
//!   ([`QueueTruth`]).
//! - **network construction**, over the honest viewers (every swarm's
//!   viewers whose [`HostClass`] is honest) from [`NET_WARM`] on: their
//!   link counts at each stats sample ([`Report::link_counts`]); the links
//!   they made and lost per minute ([`Report::link_churn`]); and, in
//!   rounds of one second of stats samples, the shares of them with no
//!   honest neighbour (eclipsed: hostile and infrastructure hosts don't
//!   count) and with no path to their swarm's seed through honest hosts
//!   (cut off) ([`Report::eclipse`]).
//! - **series** ([`Report::series`]): with transfer samples, a swarm's run
//!   second by second from its stream's start to the run's end, which the
//!   tables carry ([`Report::write_tables`]) for
//!   `hack/capture/analyze.py`'s collapse verdicts: each viewer's lag and
//!   frames read, the viewers' pushes, prunes and parents, and the seed's
//!   meter, uplink and copies.
//! - **clock skew** ([`Report::clock_classes`]): with
//!   [`Scenario::clock_skew`](crate::Scenario::clock_skew), the hosts'
//!   clock offsets per [`SkewClass`], and the largest.
//! - **arrivals and departures** ([`crate::arrivals`]): with either, the
//!   measures follow each viewer's own stay ([`Report::presence`]): its
//!   startup and steady state ([`Report::viewer_spans`]), the frames it is
//!   owed ([`Report::owed`]), the rates per minute it was there for
//!   ([`Report::viewer_minutes`]), its link counts and depths from its own
//!   arrival; every join's time to its first frame and to catching up
//!   ([`Report::joins`]); and how the viewers that stayed came through the
//!   departures ([`Report::departure_recovery`]). Without them the row
//!   reads as it always has, and the new columns are left out.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::report::{HostReport, Percentiles, Report, ViewerReport, spearman};
use crate::skew::SkewClass;
use crate::spec::{HostClass, HostIx};
use crate::stats::{MeterSample, WindowSamples};
use crate::stream::CAUGHT_UP_MS;
use crate::tables::ms_since;
use strims_ppspp::ChannelMetrics;
use strims_wan::playout::{self, DELAYS_MS, Totals};

/// Tree depth counts from this long after a stream starts, or a viewer
/// arrives.
const DEPTH_WARM: Duration = Duration::from_secs(10);
/// A viewer that joins a running stream ([`crate::HostSpec::arrive_at`])
/// is in its startup for this long, its steady state after
/// ([`Report::viewer_spans`]): long enough to find its peers, join at the
/// live edge and catch up, as its join's `caught_up_*` measures.
pub const JOIN_WARM: Duration = Duration::from_secs(10);

/// A part of a viewer's stay ([`Report::viewer_spans`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Startup,
    Steady,
}

/// A swarm's joins ([`Report::joins`]): how many, and the ms each took to
/// its first frame and to catch up, ascending, of those that did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Joins {
    pub n: u64,
    pub first: Vec<u64>,
    pub caught_up: Vec<u64>,
}

/// How the viewers that stayed came through departures
/// ([`Report::departure_recovery`]), from the departures' window's opening
/// on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Recovery {
    /// The viewers measured, and those of them whose last stats sample had
    /// no link to a host of the swarm still there (a static topology links
    /// nothing anew).
    pub stayed: usize,
    pub stranded: usize,
    /// The worst lag of their reads.
    pub peak_ms: u32,
    /// Over the stream's seconds, the seconds' lag's excess over
    /// [`CAUGHT_UP_MS`], summed (ms × s), and the time to the end of the
    /// last second it was over (zero if none was); `censored` if that was
    /// the stream's last, so they never recovered in it.
    pub excess_ms_s: u64,
    pub recover_ms: u64,
    pub censored: bool,
}

impl Joins {
    /// The `p`-th percentile of `v`, which is sorted.
    pub fn at(v: &[u64], p: u32) -> Option<u64> {
        let i = ((v.len() as f64 - 1.0) * f64::from(p) / 100.0).round() as usize;
        v.get(i).copied()
    }
}
/// The network measures start this long into the run, once the first
/// links had their time to form.
pub const NET_WARM: Duration = Duration::from_secs(10);
/// The row's peer exchange columns, by `ExchangeOutcome` name.
const EXCHANGE_COLS: [(&str, &str); 7] = [
    ("exch_connected", "Connected"),
    ("exch_remote_connected", "RemoteConnected"),
    ("exch_answered", "Answered"),
    ("exch_rejected", "Rejected"),
    ("exch_not_connected", "NotConnected"),
    ("exch_failed", "Failed"),
    ("exch_timed_out", "TimedOut"),
];

/// The width of the stream-time buckets.
pub const BUCKET: Duration = Duration::from_secs(30);
/// Streams longer than this get a line of per-bucket measures in the
/// summary.
pub const LONG_STREAM: Duration = Duration::from_secs(120);
/// The standing queue at a moment is the least uplink wait over this long
/// before it, the meter's own window for a recent round trip.
pub const STANDING_WINDOW: Duration = Duration::from_secs(3);
/// The threshold [`Report::row`] judges the queueing delay's errors at.
pub const QUEUE_DELAY_THRESHOLD_MS: u32 = 80;
/// An uplink whose least wait over the window is at least this never
/// emptied in it: it was saturated.
pub const SATURATED_STANDING_MS: u32 = 5;
/// What [`Report::queue_delay_accuracy`] scores a reading against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueTruth {
    /// The standing queue, the least wait over [`STANDING_WINDOW`]: a
    /// positive is an uplink that never emptied in it.
    Standing,
    /// The wait at the moment: a positive is one of at least the
    /// threshold.
    Instant,
}

/// An uplink queue that waits longer than this stands.
pub const STANDING_QUEUE: strims_base::time::Duration =
    strims_base::time::Duration::from_millis(50);
/// A meter reading tracks the available upload within this share of it.
pub const TRACK_BAND: f64 = 0.2;
/// How long after a change of the seed's available upload its frames count
/// toward [`Report::lag_after_changes`].
pub const AFTER_CHANGE: Duration = Duration::from_secs(30);

/// One change of a host's available upload and how its meter followed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tracking {
    pub host: HostIx,
    /// When the change came, since the start of the run.
    pub at: Duration,
    /// Whether the rate fell.
    pub drop: bool,
    /// The share of the meter's samples until the next change that read
    /// more than [`TRACK_BAND`] off.
    pub off: f64,
}

/// The hosts of a run by their part in its swarms.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roles {
    pub seeds: Vec<HostIx>,
    pub forwarders: Vec<HostIx>,
    pub others: Vec<HostIx>,
}

/// p10, p50 and p90 of `v` (0 when empty), which is sorted.
/// Which hosts reach `from` through honest hosts over the round's links
/// `g` (by host: its neighbours), `from` and the end host aside.
fn honest_reach(g: &[Vec<HostIx>], honest: &[bool], from: HostIx) -> Vec<bool> {
    let mut seen = vec![false; g.len()];
    let mut queue = vec![from];
    if let Some(s) = seen.get_mut(from) {
        *s = true;
    }
    while let Some(h) = queue.pop() {
        if h != from && !honest.get(h).copied().unwrap_or(false) {
            continue;
        }
        for &x in g.get(h).into_iter().flatten() {
            if let Some(s) = seen.get_mut(x)
                && !*s
            {
                *s = true;
                queue.push(x);
            }
        }
    }
    seen
}

fn deciles(v: &mut [f64]) -> [f64; 3] {
    v.sort_by(f64::total_cmp);
    let at = |q: f64| {
        v.get(((v.len().max(1) - 1) as f64 * q).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    [at(0.1), at(0.5), at(0.9)]
}

impl Report {
    /// Swarm `swarm`'s stream in [`BUCKET`]s from its start, the last cut at
    /// its stop.
    pub fn buckets(&self, swarm: usize) -> Vec<(Duration, Duration)> {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut t = s.start;
        while t < s.stop {
            let end = (t + BUCKET).min(s.stop);
            out.push((t, end));
            t = end;
        }
        out
    }

    /// The lag of the frames of `swarm` written in `[from, to)` of run
    /// time, over every viewer.
    pub fn lag_between(&self, swarm: usize, from: Duration, to: Duration) -> Percentiles {
        self.lag_within(swarm, &[(from, to)])
    }

    /// The lag of the frames of `swarm` written in any of `spans` of run
    /// time, over every viewer.
    pub fn lag_within(&self, swarm: usize, spans: &[(Duration, Duration)]) -> Percentiles {
        if self.scenario.swarms.get(swarm).is_none() {
            return Percentiles::default();
        }
        let ranges: Vec<_> = spans
            .iter()
            .map(|&(from, to)| self.frames_between(swarm, from, to))
            .collect();
        Percentiles::of(
            self.viewers
                .iter()
                .filter(|v| v.swarm == swarm)
                .flat_map(|v| v.stats.lags.iter())
                .filter(|(seq, _)| ranges.iter().any(|r| r.contains(seq)))
                .map(|&(_, l)| l),
        )
    }

    /// The lag of the frames of `swarm` written in `[from, to)` of run
    /// time, over its viewers among `hosts`.
    pub fn lag_of_between(
        &self,
        swarm: usize,
        hosts: &[HostIx],
        from: Duration,
        to: Duration,
    ) -> Percentiles {
        if self.scenario.swarms.get(swarm).is_none() {
            return Percentiles::default();
        }
        let range = self.frames_between(swarm, from, to);
        Percentiles::of(
            self.viewers
                .iter()
                .filter(|v| v.swarm == swarm && hosts.contains(&v.host))
                .flat_map(|v| v.stats.lags.iter())
                .filter(|(seq, _)| range.contains(seq))
                .map(|&(_, l)| l),
        )
    }

    /// The lag per [`bucket`](Report::buckets).
    pub fn lag_buckets(&self, swarm: usize) -> Vec<Percentiles> {
        self.buckets(swarm)
            .into_iter()
            .map(|(a, b)| self.lag_between(swarm, a, b))
            .collect()
    }

    /// The lag after the first bucket.
    pub fn steady_lag(&self, swarm: usize) -> Percentiles {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return Percentiles::default();
        };
        self.lag_between(swarm, s.start + BUCKET, s.stop)
    }

    /// Viewer `v`'s startup and steady state in run time, while it was
    /// there ([`Report::presence`]) and the stream ran: a viewer there at
    /// the stream's start waits out the swarm's own startup, its first
    /// [`BUCKET`], and one that arrives later its own first [`JOIN_WARM`];
    /// its steady state runs from there to its departure.
    pub fn viewer_spans(&self, v: &ViewerReport) -> [(Duration, Duration); 2] {
        let Some(spec) = self.scenario.swarms.get(v.swarm) else {
            return [(Duration::ZERO, Duration::ZERO); 2];
        };
        let (arrive, leave) = self.presence(v.host);
        let from = arrive.max(spec.start);
        let to = leave.min(spec.stop).max(from);
        let warm = match arrive <= spec.start {
            true => spec.start + BUCKET,
            false => arrive + JOIN_WARM,
        };
        let warm = warm.clamp(from, to);
        [(from, warm), (warm, to)]
    }

    /// The lag of the frames `swarm`'s viewers (those among `hosts`, if
    /// given) read that were written in their own `phase`
    /// ([`Report::viewer_spans`]).
    pub fn lag_in_spans(
        &self,
        swarm: usize,
        hosts: Option<&[HostIx]>,
        phase: Phase,
    ) -> Percentiles {
        let mut lags = Vec::new();
        for v in self
            .viewers
            .iter()
            .filter(|v| v.swarm == swarm && hosts.is_none_or(|h| h.contains(&v.host)))
        {
            let [startup, steady] = self.viewer_spans(v);
            let (from, to) = match phase {
                Phase::Startup => startup,
                Phase::Steady => steady,
            };
            let range = self.frames_between(swarm, from, to);
            lags.extend(
                v.stats
                    .lags
                    .iter()
                    .filter(|(seq, _)| range.contains(seq))
                    .map(|&(_, l)| l),
            );
        }
        Percentiles::of(lags)
    }

    /// How long host `h` was there while the first swarm streamed
    /// ([`Report::presence`]).
    pub fn streamed(&self, h: HostIx) -> Duration {
        let Some(spec) = self.scenario.swarms.first() else {
            return Duration::ZERO;
        };
        let (arrive, leave) = self.presence(h);
        leave.min(spec.stop).saturating_sub(arrive.max(spec.start))
    }

    /// The minutes of run `hosts` were there for, summed
    /// ([`Report::presence`]): the run's length each without arrivals or
    /// departures.
    pub fn viewer_minutes(&self, hosts: &[HostIx]) -> f64 {
        if !self.scenario.profiled() {
            return hosts.len() as f64 * self.scenario.duration.as_secs_f64() / 60.0;
        }
        let ms: u128 = hosts
            .iter()
            .map(|&h| {
                let (a, b) = self.presence(h);
                b.saturating_sub(a).as_millis()
            })
            .sum();
        ms as f64 / 60_000.0
    }

    /// Whether host `h` was there at `t`, for `warm` at least
    /// ([`Report::presence`]).
    fn there(&self, h: HostIx, t: Duration, warm: Duration) -> bool {
        let arrive = self
            .scenario
            .hosts
            .get(h)
            .and_then(|s| s.arrive_at)
            .unwrap_or_default();
        t >= arrive + warm && self.scenario.departure(h).is_none_or(|(at, _)| t < at)
    }

    /// Every join of `swarm`'s honest viewers, the first included: the ms
    /// from the join to its first frame and to its first under
    /// [`CAUGHT_UP_MS`], a join's clock starting when it opened its reader
    /// or the stream started, whichever came last.
    pub fn joins(&self, swarm: usize) -> Joins {
        let start = self
            .scenario
            .swarms
            .get(swarm)
            .map_or(0, |s| s.start.as_millis() as u64);
        let mut out = Joins::default();
        for jn in self
            .viewers
            .iter()
            .filter(|v| v.swarm == swarm && self.class(v.host) == HostClass::Honest)
            .flat_map(|v| v.stats.joins.iter())
        {
            out.n += 1;
            let from = jn.start_ms.max(start);
            out.first
                .extend(jn.first_frame_ms.map(|t| t.saturating_sub(from)));
            out.caught_up
                .extend(jn.caught_up_ms.map(|t| t.saturating_sub(from)));
        }
        out.first.sort_unstable();
        out.caught_up.sort_unstable();
        out
    }

    /// The row's playback columns, as the ppspp sim's rows have them: per
    /// delay `D` of [`DELAYS_MS`], `swarm`'s viewers' playouts of what they
    /// read ([`playout::play`], the tables' `stalls`), each viewer from its
    /// playback's start to its last frame's play, or its departure if that
    /// came first ([`Totals`]): the stalls per minute played
    /// (`stalls_per_min_D`), the share of it stalled (`stalled_share_D`),
    /// and the mean ms from each one's join (its reader's start, or the
    /// stream's if later, as [`Report::joins`] times it) to its playback's
    /// start (`startup_ms_D`); with hostile hosts, the honest viewers'
    /// too, with `_honest`.
    fn playback_cols(&self, swarm: usize, j: &mut JsonRow) {
        let Some(spec) = self.scenario.swarms.get(swarm) else {
            return;
        };
        let start = spec.start;
        let viewers: Vec<&ViewerReport> =
            self.viewers.iter().filter(|v| v.swarm == swarm).collect();
        let hostile = (0..self.hosts.len()).any(|h| matches!(self.class(h), HostClass::Hostile(_)));
        let mut totals = [[Totals::default(); DELAYS_MS.len()]; 2];
        for v in viewers {
            let reads = self.read_rows(v);
            let join = v
                .stats
                .joins
                .first()
                .map_or(Duration::ZERO, |jn| Duration::from_millis(jn.start_ms));
            let join = ms_since(join.max(start), start);
            let left = self
                .scenario
                .departure(v.host)
                .map(|(at, _)| ms_since(at, start));
            let plays = DELAYS_MS.map(|d| playout::play(&reads, d));
            // Every viewer's, then the honest viewers'.
            let honest = self.class(v.host) == HostClass::Honest;
            for t in totals.iter_mut().take(1 + usize::from(honest)) {
                for (t, p) in t.iter_mut().zip(&plays) {
                    t.add(p, join, left);
                }
            }
        }
        for (who, t) in totals.iter().enumerate() {
            let suffix = match who {
                0 => "",
                _ if hostile => "_honest",
                _ => continue,
            };
            for (t, d) in t.iter().zip(DELAYS_MS) {
                if let Some(x) = t.stalls_per_min() {
                    j.num(&format!("stalls_per_min_{d}{suffix}"), x);
                }
                if let Some(x) = t.stalled_share() {
                    j.num(&format!("stalled_share_{d}{suffix}"), x);
                }
                if let Some(x) = t.startup_ms() {
                    j.int(&format!("startup_ms_{d}{suffix}"), x.max(0) as u64);
                }
            }
        }
    }

    /// The row's columns of a run with arrivals or departures, named as the
    /// ppspp sim's rows name them:
    /// - `arrived`, `arrived_honest`: the viewers that arrived after the
    ///   run's start ([`HostSpec::arrive_at`](crate::HostSpec::arrive_at)),
    ///   and of them the honest; `departed`, `departed_honest`, those that
    ///   left for good;
    /// - over every join ([`Report::joins`]), the ms to its first frame
    ///   (`join_p50_ms`, `join_p90_ms`; `join_unread`, the joins that read
    ///   nothing) and to catching up (`caught_up_p50_ms`,
    ///   `caught_up_p90_ms`; `caught_up_never`);
    /// - the honest viewers' push parents that departed
    ///   ([`Report::parent_losses`], counted per departure: `parent_lost`)
    ///   and the streams they parented (`parent_lost_streams`), and how
    ///   long the viewers took to notice (`parent_lost_noticed_p50_ms`,
    ///   `parent_lost_noticed_p90_ms`; `parent_lost_unnoticed`, never);
    /// - with departures, the stayers' recovery
    ///   ([`Report::departure_recovery`]): `depart_stayed`,
    ///   `depart_stranded`, `depart_lag_peak_ms`,
    ///   `depart_excess_lag_area_ms_s`, `depart_recover_ms` and
    ///   `depart_recover_censored`.
    fn audience_cols(&self, swarm: usize, j: &mut JsonRow) {
        let viewers = self
            .scenario
            .swarms
            .get(swarm)
            .map_or(&[][..], |s| &s.viewers[..]);
        let honest = |h: &&HostIx| self.class(**h) == HostClass::Honest;
        let arrived: Vec<HostIx> = viewers
            .iter()
            .copied()
            .filter(|&h| {
                self.scenario
                    .hosts
                    .get(h)
                    .is_some_and(|x| x.arrive_at.is_some())
            })
            .collect();
        let departed: Vec<HostIx> = viewers
            .iter()
            .copied()
            .filter(|&h| self.scenario.departure(h).is_some())
            .collect();
        j.int("arrived", arrived.len() as u64)
            .int(
                "arrived_honest",
                arrived.iter().filter(honest).count() as u64,
            )
            .int("departed", departed.len() as u64)
            .int(
                "departed_honest",
                departed.iter().filter(honest).count() as u64,
            );
        let s = self.joins(swarm);
        for (name, v) in [("join", &s.first), ("caught_up", &s.caught_up)] {
            for p in [50, 90] {
                if let Some(x) = Joins::at(v, p) {
                    j.int(&format!("{name}_p{p}_ms"), x);
                }
            }
        }
        j.int("join_unread", s.n - s.first.len() as u64)
            .int("caught_up_never", s.n - s.caught_up.len() as u64);
        let losses: Vec<&crate::ParentLoss> = self
            .parent_losses
            .iter()
            .filter(|l| self.class(l.viewer) == HostClass::Honest)
            .collect();
        let mut took: Vec<u64> = losses
            .iter()
            .filter_map(|l| Some(l.noticed_ms?.saturating_sub(l.departed_ms)))
            .collect();
        took.sort_unstable();
        j.int("parent_lost", losses.len() as u64)
            .int(
                "parent_lost_streams",
                losses.iter().map(|l| l.streams as u64).sum(),
            )
            .int("parent_lost_unnoticed", (losses.len() - took.len()) as u64);
        for p in [50, 90] {
            if let Some(x) = Joins::at(&took, p) {
                j.int(&format!("parent_lost_noticed_p{p}_ms"), x);
            }
        }
        if let Some(r) = self.departure_recovery(swarm) {
            j.int("depart_stayed", r.stayed as u64)
                .int("depart_stranded", r.stranded as u64)
                .int("depart_lag_peak_ms", u64::from(r.peak_ms))
                .int("depart_excess_lag_area_ms_s", r.excess_ms_s)
                .int("depart_recover_ms", r.recover_ms)
                .int("depart_recover_censored", u64::from(r.censored));
        }
    }

    /// With arrivals or departures, their line of the summary.
    pub(crate) fn profile_line(&self) -> Option<String> {
        if !self.scenario.profiled() {
            return None;
        }
        let s = self.joins(0);
        let q = |v: &[u64]| {
            let at = |p| Joins::at(v, p).map_or("-".to_owned(), |x| x.to_string());
            format!("{}/{}", at(50), at(90))
        };
        let mut line = format!(
            "\n  arrivals {}: {} joins, first frame p50/p90 {} ms, caught up {} ms, {} read nothing, {} never caught up",
            self.scenario.arrivals,
            s.n,
            q(&s.first),
            q(&s.caught_up),
            s.n - s.first.len() as u64,
            s.n - s.caught_up.len() as u64,
        );
        if let (Some(d), Some(r)) = (self.scenario.departures, self.departure_recovery(0)) {
            let mut took: Vec<u64> = self
                .parent_losses
                .iter()
                .filter_map(|l| Some(l.noticed_ms?.saturating_sub(l.departed_ms)))
                .collect();
            took.sort_unstable();
            line.push_str(&format!(
                "\n  departures {d}: {} stayed, {} stranded; lag peak {} ms, {} ms s past {CAUGHT_UP_MS} ms, recovered {} ms after the window opened{}; {} parents lost, noticed p50/p90 {} ms, {} never",
                r.stayed,
                r.stranded,
                r.peak_ms,
                r.excess_ms_s,
                r.recover_ms,
                if r.censored { " (not by the stream's end)" } else { "" },
                self.parent_losses.len(),
                q(&took),
                self.parent_losses.len() - took.len(),
            ));
        }
        Some(line)
    }

    /// Viewer `v`'s lag in each second of `swarm`'s stream, in ms: the p99
    /// of the frames it read in the second, or a second more than the last
    /// second's in one it read none in (`None` before its first read), as
    /// `hack/capture/analyze.py` reads the series and the ppspp sim's rows
    /// measure recovery.
    pub fn lag_by_second(&self, swarm: usize, v: &ViewerReport) -> Vec<Option<u32>> {
        let Some(spec) = self.scenario.swarms.get(swarm) else {
            return Vec::new();
        };
        let secs = spec.stop.saturating_sub(spec.start).as_secs() as usize;
        let mut by: Vec<Vec<u32>> = vec![Vec::new(); secs];
        for &(seq, lag) in &v.stats.lags {
            let read = self.written_at(swarm, seq) + Duration::from_millis(u64::from(lag));
            if let Some(k) = read.checked_sub(spec.start).map(|d| d.as_secs() as usize)
                && let Some(b) = by.get_mut(k)
            {
                b.push(lag);
            }
        }
        let mut cur: Option<u32> = None;
        by.into_iter()
            .map(|b| {
                cur = match b.is_empty() {
                    false => Some(Percentiles::of(b).p99),
                    true => cur.map(|l| l.saturating_add(1000)),
                };
                cur
            })
            .collect()
    }

    /// With departures ([`crate::Scenario::departures`]), how the viewers
    /// that stayed came through them ([`Recovery`]): `swarm`'s honest
    /// viewers there before the departures' window opened that didn't
    /// leave. Each second's lag is the p90 over them of each one's
    /// ([`Report::lag_by_second`]), from the second the window opens in to
    /// the stream's stop, as the ppspp sim's rows measure it.
    pub fn departure_recovery(&self, swarm: usize) -> Option<Recovery> {
        let d = self.scenario.departures?;
        let spec = self.scenario.swarms.get(swarm)?;
        let open = spec.start + d.at;
        let stayers: Vec<&ViewerReport> = self
            .viewers
            .iter()
            .filter(|v| {
                v.swarm == swarm
                    && self.class(v.host) == HostClass::Honest
                    && self.scenario.departure(v.host).is_none()
                    && self.presence(v.host).0 < open
            })
            .collect();
        let per: Vec<Vec<Option<u32>>> = stayers
            .iter()
            .map(|v| self.lag_by_second(swarm, v))
            .collect();
        let secs = spec.stop.saturating_sub(spec.start).as_secs() as usize;
        let k0 = open.saturating_sub(spec.start).as_secs() as usize;
        let target = u32::try_from(CAUGHT_UP_MS).unwrap_or(u32::MAX);
        let (mut excess, mut bad) = (0u64, None);
        for k in k0..secs {
            let mut v: Vec<u32> = per
                .iter()
                .filter_map(|p| p.get(k).copied().flatten())
                .collect();
            v.sort_unstable();
            let Some(&q) = v.get(((v.len().max(1) - 1) as f64 * 0.9).round() as usize) else {
                continue;
            };
            excess += u64::from(q.saturating_sub(target));
            if q > target {
                bad = Some(k);
            }
        }
        let peak_ms = stayers
            .iter()
            .flat_map(|v| v.stats.lags.iter())
            .filter(|&&(seq, lag)| {
                self.written_at(swarm, seq) + Duration::from_millis(u64::from(lag)) >= open
            })
            .map(|&(_, lag)| lag)
            .max()
            .unwrap_or(0);
        // Whether `h` still had a link to a member of the swarm that was
        // there at its last stats sample.
        let linked = |h: HostIx| {
            self.hosts
                .get(h)
                .and_then(|r| r.neighbours.last())
                .is_some_and(|(_, ns)| {
                    ns.iter()
                        .any(|&n| spec.member(n) && self.scenario.departure(n).is_none())
                })
        };
        Some(Recovery {
            stayed: stayers.len(),
            stranded: stayers.iter().filter(|v| !linked(v.host)).count(),
            peak_ms,
            excess_ms_s: excess,
            recover_ms: bad.map_or(0, |k| {
                let end = spec.start + Duration::from_secs(k as u64 + 1);
                end.saturating_sub(open).as_millis() as u64
            }),
            censored: bad.is_some_and(|k| k + 1 == secs),
        })
    }

    pub fn roles(&self) -> Roles {
        let mut r = Roles {
            seeds: self.scenario.swarms.iter().map(|s| s.seed).collect(),
            ..Roles::default()
        };
        r.seeds.sort_unstable();
        r.seeds.dedup();
        for (h, host) in self.hosts.iter().enumerate() {
            if r.seeds.contains(&h) {
                continue;
            }
            if host.runner.meter.iter().any(|m| m.requests_in > 0) {
                r.forwarders.push(h);
            } else {
                r.others.push(h);
            }
        }
        r
    }

    /// p10/p50/p90 of the meter's capacity ÷ cap over `hosts`' samples.
    pub fn meter_deciles(&self, hosts: &[HostIx]) -> [f64; 3] {
        let mut v: Vec<f64> = hosts
            .iter()
            .flat_map(|&h| self.meter_readings(h))
            .map(|(c, _)| c)
            .collect();
        deciles(&mut v)
    }

    /// The median capacity ÷ cap over `hosts`' samples in each of swarm
    /// `swarm`'s buckets.
    pub fn meter_buckets(&self, swarm: usize, hosts: &[HostIx]) -> Vec<f64> {
        let series: Vec<(Duration, f64, f64)> =
            hosts.iter().flat_map(|&h| self.meter_series(h)).collect();
        self.buckets(swarm)
            .into_iter()
            .map(|(a, b)| {
                let mut v: Vec<f64> = series
                    .iter()
                    .filter(|(t, _, _)| *t >= a && *t < b)
                    .map(|&(_, c, _)| c)
                    .collect();
                deciles(&mut v)[1]
            })
            .collect()
    }

    /// The highest push commitment ÷ cap at any host.
    pub fn peak_committed(&self) -> f64 {
        (0..self.hosts.len())
            .flat_map(|h| self.meter_readings(h))
            .map(|(_, k)| k)
            .fold(0.0, f64::max)
    }

    /// p10/p50/p90 of the meter's queueing delay at `hosts` while a swarm
    /// streams, in milliseconds, over the samples that had one, and the
    /// share of samples that had one.
    pub fn queue_delay_deciles(&self, hosts: &[HostIx]) -> ([f64; 3], f64) {
        let samples: Vec<_> = hosts
            .iter()
            .filter_map(|&h| self.hosts.get(h))
            .flat_map(|h| h.runner.meter.iter())
            .filter(|m| self.live(m.t))
            .collect();
        let mut v: Vec<f64> = samples
            .iter()
            .filter_map(|m| m.queue_delay_ms)
            .map(f64::from)
            .collect();
        let had = if samples.is_empty() {
            0.0
        } else {
            v.len() as f64 / samples.len() as f64
        };
        (deciles(&mut v), had)
    }

    /// p50 of the busy fraction at `hosts` while a swarm streams.
    pub fn busy_median(&self, hosts: &[HostIx]) -> f64 {
        let mut v: Vec<f64> = hosts
            .iter()
            .filter_map(|&h| self.hosts.get(h))
            .flat_map(|h| h.runner.meter.iter())
            .filter(|m| self.live(m.t))
            .map(|m| f64::from(m.busy_permille) / 1000.0)
            .collect();
        deciles(&mut v)[1]
    }

    /// The honest viewers' mean shares of their channels whose peer they
    /// ask for lazy HAVEs, and whose peer asked them of them, over their
    /// meter samples with channels while a swarm streams.
    pub fn lazy_shares(&self) -> [f64; 2] {
        let (mut sum, mut n) = ([0.0; 2], 0u64);
        for m in self
            .honest_viewers()
            .into_iter()
            .filter_map(|h| self.hosts.get(h))
            .flat_map(|h| h.runner.meter.iter())
            .filter(|m| self.live(m.t) && m.pace_channels > 0)
        {
            let c = f64::from(m.pace_channels);
            sum[0] += f64::from(m.lazy_out) / c;
            sum[1] += f64::from(m.lazy_in) / c;
            n += 1;
        }
        sum.map(|x| x / n.max(1) as f64)
    }

    /// The share of `hosts`' uplink queue samples while a swarm streams
    /// that wait longer than [`STANDING_QUEUE`]; 0 outside WAN mode.
    pub fn standing_queue(&self, hosts: &[HostIx]) -> f64 {
        let Some(wan) = &self.wan else {
            return 0.0;
        };
        let (mut over, mut n) = (0u64, 0u64);
        for &h in hosts {
            let Some(name) = self.scenario.hosts.get(h).map(|s| s.name.as_str()) else {
                continue;
            };
            let Some(w) = wan.hosts.iter().find(|w| w.name == name) else {
                continue;
            };
            for &(t, wait) in &w.up_queue {
                let Some(t) = t.to_std() else {
                    continue;
                };
                if self.live(t) {
                    n += 1;
                    over += u64::from(wait > STANDING_QUEUE);
                }
            }
        }
        if n == 0 { 0.0 } else { over as f64 / n as f64 }
    }

    /// A queueing delay reading at `hosts` (`read`, in milliseconds, from
    /// each meter sample) against their uplink queues as `truth` takes
    /// them, over the samples that had a delay while a swarm streamed: the
    /// median absolute error in milliseconds; the share of negatives (see
    /// [`QueueTruth`]) where the delay read at least `threshold_ms` (false
    /// positives); and the share of positives where it read less (false
    /// negatives). `None` outside WAN mode or without samples.
    pub fn queue_delay_accuracy(
        &self,
        hosts: &[HostIx],
        threshold_ms: u32,
        truth: QueueTruth,
        read: impl Fn(&MeterSample) -> Option<u32>,
    ) -> Option<[f64; 3]> {
        let wan = self.wan.as_ref()?;
        let (mut errors, mut neg, mut pos) = (Vec::new(), (0u64, 0u64), (0u64, 0u64));
        for &h in hosts {
            let name = self.scenario.hosts.get(h)?.name.as_str();
            let Some(w) = wan.hosts.iter().find(|w| w.name == name) else {
                continue;
            };
            let queue: Vec<(Duration, u32)> = w
                .up_queue
                .iter()
                .filter_map(|&(t, d)| {
                    Some((
                        t.to_std()?,
                        u32::try_from(d.as_millis().max(0)).unwrap_or(u32::MAX),
                    ))
                })
                .collect();
            for m in self
                .hosts
                .get(h)?
                .runner
                .meter
                .iter()
                .filter(|m| self.live(m.t))
            {
                let Some(delay) = read(m) else {
                    continue;
                };
                let wait = match truth {
                    QueueTruth::Standing => {
                        let from = m.t.saturating_sub(STANDING_WINDOW);
                        queue
                            .iter()
                            .filter(|(t, _)| *t >= from && *t <= m.t)
                            .map(|&(_, d)| d)
                            .min()
                    }
                    QueueTruth::Instant => queue
                        .get(..queue.partition_point(|(t, _)| *t <= m.t))
                        .and_then(<[_]>::last)
                        .map(|&(_, d)| d),
                };
                let Some(wait) = wait else {
                    continue;
                };
                errors.push(f64::from(delay.abs_diff(wait)));
                let negative = match truth {
                    QueueTruth::Standing => wait < SATURATED_STANDING_MS,
                    QueueTruth::Instant => wait < threshold_ms,
                };
                if negative {
                    neg.1 += 1;
                    neg.0 += u64::from(delay >= threshold_ms);
                } else {
                    pos.1 += 1;
                    pos.0 += u64::from(delay < threshold_ms);
                }
            }
        }
        if errors.is_empty() {
            return None;
        }
        let share = |(k, n): (u64, u64)| if n == 0 { 0.0 } else { k as f64 / n as f64 };
        Some([deciles(&mut errors)[1], share(neg), share(pos)])
    }

    /// Bytes per link per second that hosts wrote to measure their paths,
    /// over every host: vnic's frames on the keepalive port, and ppspp's
    /// PING and PONG messages (9 and 17 bytes, before the headers of the
    /// channel payloads they ride in). `None` without link time.
    pub fn path_overhead(&self) -> Option<(f64, f64)> {
        let mut o = crate::stats::PathOverhead::default();
        for h in &self.hosts {
            let p = h.runner.path_overhead;
            o.control_bytes += p.control_bytes;
            o.pings += p.pings;
            o.pongs += p.pongs;
            o.link_ms += p.link_ms;
        }
        if o.link_ms == 0 {
            return None;
        }
        let secs = o.link_ms as f64 / 1000.0;
        Some((
            o.control_bytes as f64 / secs,
            (9 * o.pings + 17 * o.pongs) as f64 / secs,
        ))
    }

    /// STREAM_REQUESTs declined while the node's upload rate was under half
    /// of what it could upload, and all declined, over every host.
    pub fn declined(&self) -> (u64, u64) {
        let (mut wrong, mut all) = (0, 0);
        for (h, host) in self.hosts.iter().enumerate() {
            let mut last = 0;
            for m in &host.runner.meter {
                let d = if m.rejected >= last {
                    m.rejected - last
                } else {
                    m.rejected
                };
                last = m.rejected;
                all += u64::from(d);
                let cap = self.upload_cap_at(h, m.t).unwrap_or(0) as f64;
                if cap > 0.0 && (m.rate as f64) < cap / 2.0 {
                    wrong += u64::from(d);
                }
            }
        }
        (wrong, all)
    }

    /// The changes of host `h`'s available upload while a swarm streams:
    /// when each came and whether the rate fell. A step of its uplink's
    /// rate is one; a competing upload falls at its start and rises at its
    /// stop.
    pub fn upload_changes(&self, h: HostIx) -> Vec<(Duration, bool)> {
        let Some(spec) = self.scenario.hosts.get(h) else {
            return Vec::new();
        };
        let rate = |t: Duration| {
            spec.up_steps
                .iter()
                .filter(|&&(at, _)| at <= t)
                .max_by_key(|&&(at, _)| at)
                .map(|&(_, r)| r)
        };
        let mut changes: Vec<(Duration, bool)> = spec
            .up_steps
            .iter()
            .filter_map(|&(at, after)| {
                let before = at
                    .checked_sub(Duration::from_nanos(1))
                    .and_then(rate)
                    .or(spec.site.map(|s| s.profile.up))?;
                (after != before).then_some((at, after < before))
            })
            .chain(
                spec.competing
                    .iter()
                    .flat_map(|c| [(c.start, true), (c.stop, false)]),
            )
            .filter(|&(at, _)| self.live(at))
            .collect();
        changes.sort_by_key(|&(at, _)| at);
        changes.dedup_by_key(|&mut (at, _)| at);
        changes
    }

    /// How the meters of the seeds and forwarders followed each change of
    /// their available upload, in host and time order.
    pub fn tracking(&self) -> Vec<Tracking> {
        let roles = self.roles();
        let end = self
            .scenario
            .swarms
            .iter()
            .map(|s| s.stop)
            .max()
            .unwrap_or_default();
        let mut out = Vec::new();
        for h in roles.seeds.iter().chain(&roles.forwarders).copied() {
            let changes = self.upload_changes(h);
            let Some(host) = self.hosts.get(h) else {
                continue;
            };
            for (i, &(at, drop)) in changes.iter().enumerate() {
                let until = changes.get(i + 1).map_or(end, |&(t, _)| t);
                let within: Vec<bool> = host
                    .runner
                    .meter
                    .iter()
                    .filter(|m| m.t >= at && m.t < until)
                    .map(|m| {
                        let cap = self.upload_cap_at(h, m.t).unwrap_or(0) as f64;
                        cap <= 0.0 || (m.capacity as f64 / cap - 1.0).abs() > TRACK_BAND
                    })
                    .collect();
                if within.is_empty() {
                    continue;
                }
                let off = within.iter().filter(|&&off| off).count();
                out.push(Tracking {
                    host: h,
                    at,
                    drop,
                    off: off as f64 / within.len() as f64,
                });
            }
        }
        out
    }

    /// The lag of the frames swarm `swarm`'s seed wrote in the
    /// [`AFTER_CHANGE`] after each drop of its available upload, and after
    /// each rise.
    pub fn lag_after_changes(&self, swarm: usize) -> (Percentiles, Percentiles) {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return (Percentiles::default(), Percentiles::default());
        };
        let spans = |drop: bool| -> Vec<(Duration, Duration)> {
            self.upload_changes(s.seed)
                .into_iter()
                .filter(|&(_, d)| d == drop)
                .map(|(at, _)| (at, at + AFTER_CHANGE))
                .collect()
        };
        (
            self.lag_within(swarm, &spans(true)),
            self.lag_within(swarm, &spans(false)),
        )
    }

    /// An adaptive seed's budget changes and copies changes per minute of
    /// stream.
    pub fn seed_changes(&self, swarm: usize) -> (f64, f64) {
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return (0.0, 0.0);
        };
        let Some(host) = self.hosts.get(s.seed) else {
            return (0.0, 0.0);
        };
        let samples: Vec<_> = host
            .runner
            .seed_changes
            .iter()
            .filter(|(t, x, _)| *x == swarm && *t >= s.start && *t <= s.stop)
            .map(|(_, _, seed)| seed)
            .collect();
        let changes = |f: fn(&crate::stats::SeedSample) -> usize| {
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .filter(|(a, b)| f(a) != f(b))
                .count() as f64
        };
        let minutes = (s.stop.saturating_sub(s.start)).as_secs_f64() / 60.0;
        if minutes <= 0.0 {
            return (0.0, 0.0);
        }
        (
            changes(|x| x.budget) / minutes,
            changes(|x| x.copies) / minutes,
        )
    }

    /// With changes of available upload, how the seeds' and forwarders'
    /// meters followed them ([`Report::tracking`]) and the lag after the
    /// seed's changes.
    pub(crate) fn tracking_line(&self) -> Option<String> {
        let tracking = self.tracking();
        if tracking.is_empty() {
            return None;
        }
        let side = |drop: bool| {
            let mut v: Vec<f64> = tracking
                .iter()
                .filter(|x| x.drop == drop)
                .map(|x| x.off)
                .collect();
            let max = v.iter().copied().fold(0.0, f64::max);
            format!("{} {:.2}/{max:.2}", v.len(), deciles(&mut v)[1])
        };
        let (drop, rise) = self.lag_after_changes(0);
        Some(format!(
            "\n  capacity changes: share of meter samples off median/max after drops {}, after rises {}; swarm 0 lag {}s after the seed's drops {drop}, rises {rise}",
            side(true),
            side(false),
            AFTER_CHANGE.as_secs(),
        ))
    }

    /// For streams longer than [`LONG_STREAM`], swarm 0's p99 lag and the
    /// seeds' and forwarders' median capacity ÷ cap per bucket.
    pub(crate) fn bucket_line(&self) -> Option<String> {
        let s = self.scenario.swarms.first()?;
        if s.stop.saturating_sub(s.start) <= LONG_STREAM {
            return None;
        }
        let roles = self.roles();
        let join = |v: Vec<String>| v.join(" ");
        Some(format!(
            "\n  swarm 0 per {} s: p99 ms {}\n    meter seeds {}\n    meter forwarders {}",
            BUCKET.as_secs(),
            join(
                self.lag_buckets(0)
                    .iter()
                    .map(|p| p.p99.to_string())
                    .collect()
            ),
            join(
                self.meter_buckets(0, &roles.seeds)
                    .iter()
                    .map(|c| format!("{c:.2}"))
                    .collect()
            ),
            join(
                self.meter_buckets(0, &roles.forwarders)
                    .iter()
                    .map(|c| format!("{c:.2}"))
                    .collect()
            ),
        ))
    }

    /// The hosts' clock offsets counted per [`SkewClass`], in
    /// [`SkewClass::ALL`]'s order, and the largest magnitude.
    pub fn clock_classes(&self) -> ([u64; SkewClass::ALL.len()], strims_base::time::Duration) {
        let mut per = [0; SkewClass::ALL.len()];
        let mut max = strims_base::time::Duration::ZERO;
        for h in &self.hosts {
            if let Some(n) = per.get_mut(SkewClass::of(h.clock_offset) as usize) {
                *n += 1;
            }
            max = max.max(h.clock_offset.abs());
        }
        (per, max)
    }

    /// The run's measures as one JSON object, for scripts: the test, seed,
    /// trace, reads' digest ([`Report::reads_hash`]) and knobs, whether the
    /// test's check passed (`passed`, 0 or 1, and its `failure`, empty when
    /// it passed), with clock skew its hosts'
    /// offsets (`clock_offsets_ms`, each host's in host order;
    /// `clock_max_ms`, the largest; `clock_<class>`, the hosts per
    /// [`SkewClass::name`]), as the ppspp sim's rows name them, then per
    /// scenario swarm 0 its delivery, lag (overall,
    /// startup bucket, steady state, per bucket), its viewers' playback at
    /// each delay `D` of [`DELAYS_MS`] (`stalls_per_min_D`,
    /// `stalled_share_D`, `startup_ms_D`: [`Totals`] of their playouts,
    /// and with hostile hosts the honest viewers' with `_honest`) and the
    /// meter's measures (see the module docs). With arrivals or departures, in the ppspp
    /// sim's rows' names: their profiles (`arrivals`, `depart`), the
    /// startup and steady state each viewer's own
    /// ([`Report::viewer_spans`]), the viewers that arrived and departed
    /// (`arrived`, `departed`, each with `_honest`), the joins
    /// ([`Report::joins`]: `join_p50_ms`, `join_p90_ms`, `join_unread`,
    /// `caught_up_p50_ms`, `caught_up_p90_ms`, `caught_up_never`), the
    /// push parents lost to departures (`parent_lost`,
    /// `parent_lost_streams`, and how soon they were noticed:
    /// `parent_lost_noticed_p50_ms`, `parent_lost_noticed_p90_ms`,
    /// `parent_lost_unnoticed`) and, with departures, the stayers' recovery
    /// ([`Report::departure_recovery`]: `depart_stayed`, `depart_stranded`,
    /// `depart_lag_peak_ms`, `depart_excess_lag_area_ms_s`,
    /// `depart_recover_ms`, `depart_recover_censored`).
    pub fn row(&self, test: &str, failure: Option<&str>) -> String {
        let reads = self.reads_hash();
        let mut j = JsonRow::default();
        j.str("test", test)
            .str("scenario", &self.scenario.name)
            .int("seed", self.seed)
            .str("trace", self.trace_hash.get(..16).unwrap_or_default())
            .str("reads_hash", reads.get(..16).unwrap_or_default())
            .int("passed", u64::from(failure.is_none()))
            .str("failure", failure.unwrap_or_default())
            .int(
                "send_buffer_ms",
                self.scenario
                    .send_buffer
                    .map_or(0, |b| b.uplink_queue.as_millis() as u64),
            )
            .int(
                "known_uplink",
                u64::from(
                    self.scenario
                        .hosts
                        .iter()
                        .any(|h| h.upload_capacity.is_some()),
                ),
            )
            .str("clock_skew", &self.scenario.clock_skew.to_string());
        if !self.scenario.clock_skew.is_off() {
            let (per, max) = self.clock_classes();
            let offsets = self.hosts.iter().map(|h| h.clock_offset.as_millis());
            j.signed_ints("clock_offsets_ms", offsets)
                .int("clock_max_ms", max.as_millis().unsigned_abs());
            for (c, n) in SkewClass::ALL.into_iter().zip(per) {
                j.int(&format!("clock_{}", c.name()), n);
            }
        }
        let swarm = 0;
        if !self.scenario.arrivals.is_t0() {
            j.str("arrivals", &self.scenario.arrivals.to_string());
        }
        if let Some(d) = self.scenario.departures {
            j.str("depart", &d.to_string());
        }
        let lag = self.lag(swarm);
        let buckets = self.lag_buckets(swarm);
        let (startup, steady) = match self.scenario.profiled() {
            true => (
                self.lag_in_spans(swarm, None, Phase::Startup),
                self.lag_in_spans(swarm, None, Phase::Steady),
            ),
            false => (
                buckets.first().copied().unwrap_or_default(),
                self.steady_lag(swarm),
            ),
        };
        j.num("delivery", self.delivery(swarm))
            .int("lag_p50_ms", u64::from(lag.p50))
            .int("lag_p99_ms", u64::from(lag.p99))
            .int("startup_p50_ms", u64::from(startup.p50))
            .int("startup_p99_ms", u64::from(startup.p99))
            .int("steady_p50_ms", u64::from(steady.p50))
            .int("steady_p99_ms", u64::from(steady.p99))
            .ints("bucket_p50_ms", buckets.iter().map(|p| u64::from(p.p50)))
            .ints("bucket_p99_ms", buckets.iter().map(|p| u64::from(p.p99)));
        self.playback_cols(swarm, &mut j);
        let roles = self.roles();
        for (name, hosts) in [
            ("seeds", &roles.seeds),
            ("forwarders", &roles.forwarders),
            ("others", &roles.others),
        ] {
            let d = self.meter_deciles(hosts);
            j.nums(&format!("meter_{name}"), d.into_iter())
                .int(&format!("n_{name}"), hosts.len() as u64);
        }
        j.nums(
            "meter_bucket_seeds",
            self.meter_buckets(swarm, &roles.seeds).into_iter(),
        )
        .nums(
            "meter_bucket_forwarders",
            self.meter_buckets(swarm, &roles.forwarders).into_iter(),
        );
        let (qd, had) = self.queue_delay_deciles(&roles.seeds);
        j.nums("queue_delay_seeds_ms", qd.into_iter())
            .num("queue_delay_seeds_had", had)
            .num("busy_seeds_p50", self.busy_median(&roles.seeds))
            .num("standing_seeds", self.standing_queue(&roles.seeds))
            .num(
                "standing_forwarders",
                self.standing_queue(&roles.forwarders),
            )
            .num("peak_committed", self.peak_committed());
        let [lazy_out, lazy_in] = self.lazy_shares();
        j.num("have_lazy_out_share", lazy_out)
            .num("have_lazy_in_share", lazy_in);
        type Reading = fn(&MeterSample) -> Option<u32>;
        let readings: [(&str, QueueTruth, Reading); 5] = [
            ("qd", QueueTruth::Standing, |m| m.queue_delay_ms),
            ("qdv", QueueTruth::Standing, |m| m.path_delay_ms),
            ("qdb", QueueTruth::Standing, |m| m.path_queued_ms),
            ("qdbi", QueueTruth::Instant, |m| m.path_queued_ms),
            ("qdl", QueueTruth::Instant, |m| m.path_latest_ms),
        ];
        for (reading, truth, read) in readings {
            for (name, hosts) in [("seeds", &roles.seeds), ("forwarders", &roles.forwarders)] {
                if let Some([err, fp, fneg]) =
                    self.queue_delay_accuracy(hosts, QUEUE_DELAY_THRESHOLD_MS, truth, read)
                {
                    j.num(&format!("{reading}_err_{name}_ms"), err)
                        .num(&format!("{reading}_fp_{name}"), fp)
                        .num(&format!("{reading}_fn_{name}"), fneg);
                }
            }
        }
        let (wrong, declined) = self.declined();
        j.int("declined_wrongly", wrong).int("declined", declined);
        if let Some((control, pings)) = self.path_overhead() {
            j.num("control_bps_link", control)
                .num("ping_bps_link", pings);
        }
        let tracking = self.tracking();
        if !tracking.is_empty() {
            for (name, drop) in [("drop", true), ("rise", false)] {
                let mut v: Vec<f64> = tracking
                    .iter()
                    .filter(|x| x.drop == drop)
                    .map(|x| x.off)
                    .collect();
                let n = v.len() as u64;
                let max = v.iter().copied().fold(0.0, f64::max);
                j.nums(
                    &format!("off_{name}"),
                    [deciles(&mut v)[1], max].into_iter(),
                )
                .int(&format!("off_{name}_n"), n);
            }
            let (drop, rise) = self.lag_after_changes(swarm);
            j.int("seed_drop_p99_ms", u64::from(drop.p99))
                .int("seed_rise_p99_ms", u64::from(rise.p99));
        }
        let (budget, copies) = self.seed_changes(swarm);
        j.num("seed_budget_changes_per_min", budget)
            .num("seed_copies_changes_per_min", copies);
        if let Some(seed) = self
            .scenario
            .swarms
            .get(swarm)
            .and_then(|s| self.hosts.get(s.seed))
            .and_then(|h| h.runner.seeds.get(&swarm))
        {
            j.int("seed_first_wave", seed.first_wave as u64)
                .int("seed_first_wave_capacity", seed.first_wave_capacity)
                .int("seed_budget", seed.budget as u64)
                .int("seed_copies", seed.copies as u64);
        }
        // The seed's answers queued, refused and dropped, and how long its
        // pushes and answers waited in its runner's queues.
        if let Some(r) = self
            .scenario
            .swarms
            .get(swarm)
            .and_then(|s| self.hosts.get(s.seed))
            .map(|h| &h.runner)
        {
            let c = r.swarm(swarm);
            j.int("seed_answers_queued", c.answers_queued)
                .int("seed_answers_refused", c.answers_refused)
                .int("seed_answers_dropped", c.answers_dropped);
            if let Some(w) = r.waits.get(&swarm) {
                for (name, h) in [("push", &w[0]), ("answer", &w[1])] {
                    for p in [50, 99] {
                        if let Some(d) = h.percentile(f64::from(p)) {
                            j.int(
                                &format!("seed_{name}_wait_p{p}_ms"),
                                d.as_millis().max(0) as u64,
                            );
                        }
                    }
                }
            }
        }
        // The viewers' uplink use: bytes written to sockets over the run
        // against their uplink for the stream's span (with arrivals or
        // departures, the span each was there for).
        if let Some(spec) = self.scenario.swarms.get(swarm) {
            let secs = spec.stop.saturating_sub(spec.start).as_secs_f64().max(1e-9);
            let profiled = self.scenario.profiled();
            let mut util: Vec<f64> = spec
                .viewers
                .iter()
                .chain(&spec.relays)
                .filter_map(|&h| {
                    let cap = self.upload_cap(h)? as f64;
                    let secs = match profiled {
                        true => self.streamed(h).as_secs_f64().max(1e-9),
                        false => secs,
                    };
                    Some(self.hosts.get(h)?.uploaded as f64 / (cap * secs))
                })
                .collect();
            util.sort_by(f64::total_cmp);
            for p in [50, 90, 95] {
                if let Some(&u) = util.get(util.len().saturating_sub(1) * p / 100) {
                    j.num(&format!("viewer_util_p{p}"), u);
                }
            }
            // Relays' fan-out: push subscribers per transfer sample of a
            // viewer with any.
            let mut fanout: Vec<usize> = spec
                .viewers
                .iter()
                .chain(&spec.relays)
                .filter_map(|&h| self.hosts.get(h))
                .flat_map(|r| {
                    r.transfers
                        .iter()
                        .filter(|(_, s, x)| *s == swarm && x.children > 0)
                        .map(|(_, _, x)| x.children)
                })
                .collect();
            fanout.sort_unstable();
            for p in [50, 90] {
                if let Some(&f) = fanout.get(fanout.len().saturating_sub(1) * p / 100) {
                    j.int(&format!("fanout_p{p}"), f as u64);
                }
            }
            if let Some(d) = self.depths(swarm) {
                for (name, v) in d {
                    j.num(name, v);
                }
            }
            // Parent changes (prunes and switches) per viewer-minute (with
            // arrivals or departures, per minute each viewer was there).
            let c = self.swarm_counters(swarm);
            let viewers = spec.viewers.len().max(1) as f64;
            let minutes = match profiled {
                true => {
                    let ms: u128 = spec
                        .viewers
                        .iter()
                        .map(|&h| self.streamed(h).as_millis())
                        .sum();
                    (ms as f64 / 60_000.0).max(1e-9)
                }
                false => viewers * secs / 60.0,
            };
            j.num(
                "parent_changes_per_min",
                (c.pruned + c.switched) as f64 / minutes,
            );
        }
        let c = self.swarm_counters(swarm);
        // Requests timed out ([`Report::timeouts`]): the honest viewers',
        // the seed's, and with hostile hosts theirs below.
        let (timeouts, seed_timeouts, hostile_timeouts) = self.timeouts(swarm);
        j.num("seed_data_share", self.data_share(swarm))
            .num("seed_share", self.uploads(swarm).seed_share)
            .num("duplicates", self.duplicates(swarm))
            .int("push_hits", c.hits)
            .int("push_misses", c.misses)
            .int("pruned", c.pruned)
            .int("shed", c.shed)
            .int("switched", c.switched)
            .int("timeouts", timeouts)
            .int("seed_timeouts", seed_timeouts)
            .num("sim_secs", self.sim_secs)
            .num("wall_secs", self.wall_secs);
        self.window_cols(swarm, &mut j);
        let [links_p50, links_p90] = self.link_counts();
        let [formed, dropped] = self.link_churn();
        let [eclipsed, cut] = self.eclipse();
        let hostile: Vec<HostIx> = (0..self.hosts.len())
            .filter(|&h| matches!(self.class(h), HostClass::Hostile(_)))
            .collect();
        if !hostile.is_empty() {
            let honest = self.honest_viewers();
            let s = self
                .scenario
                .swarms
                .first()
                .map_or(Duration::ZERO, |s| s.start);
            let e = self
                .scenario
                .swarms
                .first()
                .map_or(Duration::ZERO, |s| s.stop);
            for (name, hosts) in [("honest", &honest), ("hostile", &hostile)] {
                let lag = self.lag_of_between(swarm, hosts, s, e);
                let startup = match self.scenario.profiled() {
                    true => self.lag_in_spans(swarm, Some(hosts), Phase::Startup),
                    false => self.lag_of_between(swarm, hosts, s, (s + BUCKET).min(e)),
                };
                j.int(&format!("lag_p99_ms_{name}"), u64::from(lag.p99))
                    .int(&format!("lag_p50_ms_{name}"), u64::from(lag.p50))
                    .int(&format!("startup_p99_ms_{name}"), u64::from(startup.p99))
                    .num(&format!("delivery_{name}"), self.delivery_of(swarm, hosts));
            }
            j.int("timeouts_hostile", hostile_timeouts);
        }
        j.int("links_p50", links_p50)
            .int("links_p90", links_p90)
            .num("links_formed_per_host_min", formed)
            .num("links_dropped_per_host_min", dropped)
            .num("eclipsed_share", eclipsed)
            .num("cut_from_seed_share", cut);
        for (key, v) in self.capacity_position(swarm) {
            j.num(key, v);
        }
        let viewers = self.honest_viewers();
        let (trials, wins) = viewers
            .iter()
            .filter_map(|&h| self.hosts.get(h))
            .fold((0, 0), |(t, w), r| {
                (t + r.pool_trials, w + r.pool_trial_wins)
            });
        let minutes = self.viewer_minutes(&viewers);
        if trials > 0 && minutes > 0.0 {
            j.num("pool_trials_per_host_min", trials as f64 / minutes)
                .num("pool_trial_wins", wins as f64 / trials as f64);
        }
        // A seed's first wave of offers, and when its viewers found it
        // (`pool-find`).
        if let Some(r) = self
            .scenario
            .swarms
            .get(swarm)
            .and_then(|s| self.hosts.get(s.seed))
        {
            j.int("seed_offers_30s", r.offers_early);
        }
        let mut resolved: Vec<u64> = viewers
            .iter()
            .filter_map(|&h| {
                let at = self.hosts.get(h)?.source_resolved_ms?;
                Some(at.saturating_sub(self.presence(h).0.as_millis() as u64))
            })
            .collect();
        resolved.sort_unstable();
        if let Some(&p90) = resolved.get(((resolved.len() as f64 - 1.0) * 0.9).round() as usize) {
            j.int("source_resolved_p90_ms", p90);
        }
        let mut rejoins: Vec<u64> = self
            .viewers
            .iter()
            .flat_map(|v| v.stats.joins.iter().skip(1))
            .filter_map(|j| Some(j.caught_up_ms?.saturating_sub(j.start_ms)))
            .collect();
        rejoins.sort_unstable();
        if let Some(&p90) = rejoins.get(((rejoins.len() as f64 - 1.0) * 0.9).round() as usize) {
            j.int("rejoin_p90_ms", p90);
        }
        if self.scenario.profiled() {
            self.audience_cols(swarm, &mut j);
        }
        for (key, outcome) in EXCHANGE_COLS {
            j.int(key, self.exchanges_ended(outcome));
        }
        self.media_cols(swarm, &mut j);
        self.ledger_cols(&mut j);
        self.ack_cols(&mut j);
        j.finish()
    }

    /// With the window model, the acknowledgements the seed's TCP
    /// receivers sent, of what its viewers sent it, and the honest
    /// viewers' (summed), of what reached them, by why
    /// ([`crate::ACK_WHYS`]: `seed_ack_now`, `viewer_ack_timer`, ...),
    /// pure ones and those data carried alike; and the mean and p90 of the
    /// delayed ones' waits (`*_delack_mean_ms`, `*_delack_p90_ms`, to the
    /// millisecond below), from the first byte owed to the timer's send.
    fn ack_cols(&self, j: &mut JsonRow) {
        let Some(wan) = &self.wan else {
            return;
        };
        if !wan.hosts.iter().any(|w| w.ack_whys.iter().any(|&n| n > 0)) {
            return;
        }
        let seeds = self.roles().seeds;
        let (mut seed, mut viewer) = (
            ([0u64; crate::ACK_WHYS.len()], crate::AckWaits::default()),
            ([0u64; crate::ACK_WHYS.len()], crate::AckWaits::default()),
        );
        for (i, h) in self.hosts.iter().enumerate() {
            let Some(w) = wan.hosts.iter().find(|x| x.name == h.name) else {
                continue;
            };
            let into = if seeds.contains(&i) {
                &mut seed
            } else if self.class(i) == HostClass::Honest {
                &mut viewer
            } else {
                continue;
            };
            for (a, b) in into.0.iter_mut().zip(w.ack_whys) {
                *a += b;
            }
            into.1.merge(&w.ack_waits);
        }
        for (role, (whys, waits)) in [("seed", seed), ("viewer", viewer)] {
            for (why, n) in crate::ACK_WHYS.iter().zip(whys) {
                j.int(&format!("{role}_ack_{why}"), n);
            }
            if let (Some(mean), Some(p90)) = (waits.mean_ms(), waits.quantile_ms(0.9)) {
                j.num(&format!("{role}_delack_mean_ms"), mean)
                    .int(&format!("{role}_delack_p90_ms"), p90);
            }
        }
    }

    /// The uplinks' bytes by layer over every host, in the ppspp sim's
    /// rows' terms (`up_*`, adding up to `up_wire`): the ppspp layers from
    /// the honest hosts' runners; `up_hostile`, what hostile hosts wrote;
    /// `up_ports`, vnic's other ports (vpn, rpc); `up_vnic`, vnic's frame
    /// headers, control frames and link handshakes; and in WAN mode
    /// `up_transport`, the transports' framing and resends, and `up_acks`,
    /// pure ACKs and SACKs alone. Then the waste in the honest viewers'
    /// inbound data, the wire bytes per byte of the stream they read, each
    /// role's overhead per second over its uplink ([`Report::roles`]), the
    /// seed's wire bytes over what it wrote, the
    /// packets (WAN mode; acknowledgements count as small) and vnic's
    /// frames.
    fn ledger_cols(&self, j: &mut JsonRow) {
        let secs = self.sim_secs.max(1e-9);
        let wan = |h: &HostReport| {
            self.wan
                .as_ref()
                .and_then(|w| w.hosts.iter().find(|x| x.name == h.name))
        };
        let hostile = |h: HostIx| matches!(self.class(h), HostClass::Hostile(_));
        let (mut up, mut down) = (ChannelMetrics::default(), ChannelMetrics::default());
        let (mut ports, mut vnic, mut transport, mut acks, mut hostile_bytes) = (0, 0, 0, 0, 0);
        let (mut wire_down, mut packets, mut small, mut frames, mut frame_bytes) = (0, 0, 0, 0, 0);
        let mut ack_packets = 0;
        let seeds = self.roles().seeds;
        // A host's bytes through its uplink, framing included, and its
        // packets.
        let wire = |h: &HostReport| {
            let w = wan(h);
            let framing = w.map_or(0, |w| {
                w.stats.up_bytes.saturating_sub(w.app_bytes) + w.ack_bytes
            });
            (h.uploaded + framing, w.map(|w| w.stats.up_packets + w.acks))
        };
        for (i, h) in self.hosts.iter().enumerate() {
            if hostile(i) {
                hostile_bytes += h.uploaded;
            } else {
                let sent = h.ppspp.outbound.wire_bytes();
                up.merge(&h.ppspp.outbound);
                ports += h.frame_bytes.saturating_sub(sent);
                vnic += h.uploaded.saturating_sub(h.frame_bytes.max(sent));
                frames += h.frames_written;
                frame_bytes += h.frame_bytes;
                if !seeds.contains(&i) {
                    down.merge(&h.ppspp.inbound);
                }
            }
            match wan(h) {
                Some(w) => {
                    transport += w.stats.up_bytes.saturating_sub(w.app_bytes);
                    acks += w.ack_bytes;
                    wire_down += w.stats.down_bytes;
                    packets += w.stats.up_packets + w.acks;
                    small += w.stats.small_packets + w.acks;
                    ack_packets += w.acks;
                }
                None => wire_down += h.downloaded,
            }
        }
        let useful: u64 = self
            .viewers
            .iter()
            .filter(|v| !hostile(v.host))
            .map(|v| {
                let media = self.media.get(v.swarm).is_some_and(Option::is_some);
                let size = match media {
                    true => self
                        .scenario
                        .swarms
                        .get(v.swarm)
                        .map_or(0, |s| s.options.chunk_size),
                    false => crate::stream::FRAME_LEN,
                };
                v.stats.received.len() * size as u64
            })
            .sum();
        let up_wire = up.wire_bytes() + ports + vnic + hostile_bytes + transport + acks;
        let waste = down
            .data_bytes
            .saturating_sub(down.new_bytes + down.stale_bytes + down.invalid_bytes);
        j.int("up_payload", up.data_bytes)
            .int("up_data_hdr", up.data_header_bytes)
            .int("up_integrity", up.integrity_bytes)
            .int("up_signed", up.signed_integrity_bytes)
            .int("up_have", up.have_bytes)
            .int("up_ack", up.ack_bytes)
            .int("up_request", up.request_bytes)
            .int("up_ping", up.ping_bytes)
            .int("up_stream", up.stream_bytes)
            .int("up_announce", up.announce_bytes)
            .int("up_session", up.session_bytes)
            .int("up_chan_hdr", up.header_bytes)
            .int("up_hostile", hostile_bytes)
            .int("up_vnic", vnic)
            .int("up_ports", ports)
            .int("up_transport", transport)
            .int("up_acks", acks)
            .int("up_wire", up_wire)
            .int("down_wire", wire_down)
            .int("dup_bytes", waste)
            .int("stale_bytes", down.stale_bytes)
            .int("useful_bytes", useful)
            .num("wire_per_useful", up_wire as f64 / useful.max(1) as f64);

        let share = |i: HostIx| {
            let h = self.hosts.get(i)?;
            let over = wire(h).0.saturating_sub(h.ppspp.outbound.data_bytes);
            Some(over as f64 / (self.upload_cap(i)? as f64 * secs))
        };
        let roles = self.roles();
        let shares = |hosts: &[HostIx]| {
            let mut v: Vec<f64> = hosts
                .iter()
                .filter(|&&h| self.class(h) == HostClass::Honest)
                .filter_map(|&h| share(h))
                .collect();
            v.sort_by(f64::total_cmp);
            v
        };
        let at = |v: &[f64], p: usize| v.get(v.len().saturating_sub(1) * p / 100).copied();
        let (relays, leaves) = (shares(&roles.forwarders), shares(&roles.others));
        let seed = self.scenario.swarms.first().map(|s| s.seed);
        if let Some(x) = seed.and_then(share) {
            j.num("ovh_seed", x);
        }
        if let Some(h) = seed.and_then(|s| self.hosts.get(s)) {
            j.num(
                "seed_wire_per_written",
                wire(h).0 as f64 / h.uploaded.max(1) as f64,
            );
        }
        for (name, v, p) in [
            ("ovh_relay_p50", &relays, 50),
            ("ovh_leaf_p50", &leaves, 50),
            ("ovh_leaf_p90", &leaves, 90),
        ] {
            if let Some(x) = at(v, p) {
                j.num(name, x);
            }
        }
        if self.wan.is_some() {
            let mut pps: Vec<f64> = roles
                .forwarders
                .iter()
                .chain(&roles.others)
                .filter(|&&h| self.class(h) == HostClass::Honest)
                .filter_map(|&h| wire(self.hosts.get(h)?).1)
                .map(|p| p as f64 / secs)
                .collect();
            pps.sort_by(f64::total_cmp);
            j.int("packets_up", packets)
                .int("packets_ack", ack_packets)
                .num("small_packet_share", small as f64 / packets.max(1) as f64);
            for (name, p) in [("pps_p50", 50), ("pps_p90", 90)] {
                if let Some(x) = at(&pps, p) {
                    j.num(name, x);
                }
            }
            if let Some(p) = seed.and_then(|s| wire(self.hosts.get(s)?).1) {
                j.num("seed_pps", p as f64 / secs);
            }
        }
        j.num(
            "frame_bytes_mean",
            frame_bytes as f64 / frames.max(1) as f64,
        )
        .num(
            "msgs_per_payload",
            up.messages() as f64 / up.payloads.max(1) as f64,
        );
    }

    /// The request windows of the honest hosts' open channels of `swarm`
    /// at every stats sample while it streamed ([`WindowSamples`]), as the
    /// ppspp sim's rows name them: the window's p10, p50 and p90 and the
    /// flight's p50 and p90 in KiB, the requests out's p50, p90 and most,
    /// and the queuing delay's and the request timeout's p50 and p90 in
    /// ms. Most samples of a push channel have nothing out, so the flight
    /// and the requests out also read over the samples with any
    /// (`*_busy_*`), with the share of samples with requests out
    /// (`reqs_out_share`). Then in WAN mode the honest hosts' uplink
    /// queues' waits over their samples while a swarm streams, in ms
    /// (`upq_ms_*`: p50, p90 and the longest). Columns without samples are
    /// left out.
    fn window_cols(&self, swarm: usize, j: &mut JsonRow) {
        let honest = |i: HostIx| self.class(i) == HostClass::Honest;
        let mut w = WindowSamples::default();
        for (i, h) in self.hosts.iter().enumerate() {
            if honest(i)
                && let Some(x) = h.runner.windows.get(&swarm)
            {
                w.merge(x);
            }
        }
        let (flight, requests) = (w.flight.nonzero(), w.requests_out.nonzero());
        for (name, h, unit, ps) in [
            ("cwnd_kib", &w.cwnd, 1024.0, &[10, 50, 90][..]),
            ("flight_kib", &w.flight, 1024.0, &[50, 90]),
            ("flight_kib_busy", &flight, 1024.0, &[50, 90]),
            ("reqs_out", &w.requests_out, 1.0, &[50, 90]),
            ("reqs_out_busy", &requests, 1.0, &[50, 90]),
            ("ledbat_qd_ms", &w.queuing_delay, 1e3, &[50, 90]),
            ("cto_ms", &w.cto, 1e3, &[50, 90]),
        ] {
            for &p in ps {
                if let Some(x) = h.percentile(f64::from(p)) {
                    j.num(&format!("{name}_p{p}"), x as f64 / unit);
                }
            }
        }
        if let Some(n) = w.requests_out.max() {
            j.int("reqs_out_max", n).num(
                "reqs_out_share",
                requests.count() as f64 / w.requests_out.count() as f64,
            );
        }
        let Some(wan) = &self.wan else {
            return;
        };
        let mut waits: Vec<i64> = Vec::new();
        for (i, spec) in self.scenario.hosts.iter().enumerate() {
            if !honest(i) {
                continue;
            }
            let Some(up) = wan.hosts.iter().find(|x| x.name == spec.name) else {
                continue;
            };
            waits.extend(
                up.up_queue
                    .iter()
                    .filter(|(t, _)| t.to_std().is_some_and(|t| self.live(t)))
                    .map(|(_, d)| d.as_micros().max(0)),
            );
        }
        waits.sort_unstable();
        for p in [50, 90] {
            if let Some(&x) = waits.get(waits.len().saturating_sub(1) * p / 100) {
                j.num(&format!("upq_ms_p{p}"), x as f64 / 1e3);
            }
        }
        if let Some(&x) = waits.last() {
            j.num("upq_ms_max", x as f64 / 1e3);
        }
    }

    /// A digest (SHA-256, in hex) of the honest viewers' reads: each
    /// viewer's swarm, host and frames read, then each frame's seq and lag,
    /// which with its write time ([`Report::written_at`]) is when it was
    /// read. The trace records the run's events and few of its reads, so a
    /// change that moves only readers (where one starts, what it skips) can
    /// keep the trace and changes this.
    pub fn reads_hash(&self) -> String {
        let mut h = Sha256::new();
        let honest = self
            .viewers
            .iter()
            .filter(|v| self.class(v.host) == HostClass::Honest);
        for v in honest {
            h.update((v.swarm as u64).to_be_bytes());
            h.update((v.host as u64).to_be_bytes());
            h.update((v.stats.lags.len() as u64).to_be_bytes());
            for &(seq, lag) in &v.stats.lags {
                h.update(seq.to_be_bytes());
                h.update(lag.to_be_bytes());
            }
        }
        hex::encode(h.finalize())
    }

    /// Every swarm's viewers that are honest, each once.
    pub fn honest_viewers(&self) -> Vec<HostIx> {
        let mut v: Vec<HostIx> = self
            .scenario
            .swarms
            .iter()
            .flat_map(|s| s.viewers.iter().copied())
            .filter(|&h| self.class(h) == HostClass::Honest)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn class(&self, h: HostIx) -> HostClass {
        self.scenario
            .hosts
            .get(h)
            .map_or(HostClass::Infra, |s| s.class)
    }

    /// The requests of swarm `swarm` that timed out unanswered
    /// ([`crate::stats::SwarmCounters::timeouts`], every incarnation), as
    /// the ppspp sim's rows count them: the honest viewers' (every host but
    /// the seed and the hostile ones), the seed's, and the hostile hosts'.
    pub fn timeouts(&self, swarm: usize) -> (u64, u64, u64) {
        let seed = self.scenario.swarms.get(swarm).map(|s| s.seed);
        let (mut honest, mut seeds, mut hostile) = (0, 0, 0);
        for (h, r) in self.hosts.iter().enumerate() {
            let t = r.runner.swarm(swarm).timeouts;
            if Some(h) == seed {
                seeds += t;
            } else if matches!(self.class(h), HostClass::Hostile(_)) {
                hostile += t;
            } else {
                honest += t;
            }
        }
        (honest, seeds, hostile)
    }

    /// p50 and p90 of the honest viewers' link counts over their stats
    /// samples from [`NET_WARM`] after their arrival on, while they were
    /// there ([`Report::presence`]).
    pub fn link_counts(&self) -> [u64; 2] {
        let mut v: Vec<u64> = self
            .honest_viewers()
            .into_iter()
            .filter_map(|h| Some((h, self.hosts.get(h)?)))
            .flat_map(|(h, r)| r.neighbours.iter().map(move |x| (h, x)))
            .filter(|&(h, (t, _))| *t >= NET_WARM && self.there(h, *t, NET_WARM))
            .map(|(_, (_, ns))| ns.len() as u64)
            .collect();
        v.sort_unstable();
        let at = |q: f64| {
            let i = ((v.len() as f64 - 1.0) * q).round() as usize;
            v.get(i).copied().unwrap_or(0)
        };
        [at(0.5), at(0.9)]
    }

    /// Links the honest viewers made and lost, per minute of run they were
    /// there for ([`Report::viewer_minutes`]).
    pub fn link_churn(&self) -> [f64; 2] {
        let viewers = self.honest_viewers();
        let minutes = self.viewer_minutes(&viewers);
        let (mut added, mut removed) = (0, 0);
        for r in viewers.iter().filter_map(|&h| self.hosts.get(h)) {
            added += r.peers_added;
            removed += r.peers_removed;
        }
        if minutes <= 0.0 {
            return [0.0; 2];
        }
        [added as f64 / minutes, removed as f64 / minutes]
    }

    /// Over the honest viewers' stats samples from [`NET_WARM`] after their
    /// arrival on, while they were there ([`Report::presence`]), in rounds
    /// of a second: the share with no honest neighbour, and the share with
    /// no path to their swarm's seed through honest hosts. A link counts in
    /// a round when either end reported it.
    pub fn eclipse(&self) -> [f64; 2] {
        let n = self.hosts.len();
        let honest: Vec<bool> = (0..n).map(|h| self.class(h) == HostClass::Honest).collect();
        let rounds = self.link_rounds();
        let viewers: Vec<(HostIx, HostIx)> = self
            .scenario
            .swarms
            .iter()
            .flat_map(|s| s.viewers.iter().map(move |&v| (v, s.seed)))
            .filter(|&(v, _)| honest.get(v).copied().unwrap_or(false))
            .collect();
        let (mut samples, mut eclipsed, mut cut) = (0u64, 0u64, 0u64);
        for (&sec, g) in &rounds {
            let mut reach: BTreeMap<HostIx, Vec<bool>> = BTreeMap::new();
            for &(v, seed) in &viewers {
                let Some(ns) = g.get(v) else {
                    continue;
                };
                if ns.is_empty() || !self.there(v, Duration::from_secs(sec), NET_WARM) {
                    continue;
                }
                samples += 1;
                let honest_of = |x: &HostIx| honest.get(*x).copied().unwrap_or(false);
                if !ns.iter().any(honest_of) {
                    eclipsed += 1;
                }
                let r = reach
                    .entry(seed)
                    .or_insert_with(|| honest_reach(g, &honest, seed));
                if !r.get(v).copied().unwrap_or(false) {
                    cut += 1;
                }
            }
        }
        if samples == 0 {
            return [0.0; 2];
        }
        let s = samples as f64;
        [eclipsed as f64 / s, cut as f64 / s]
    }

    /// The link graph in rounds of a second from [`NET_WARM`] on: each
    /// host's neighbours, a link counting when either end reported it.
    fn link_rounds(&self) -> BTreeMap<u64, Vec<Vec<HostIx>>> {
        let n = self.hosts.len();
        let mut rounds: BTreeMap<u64, Vec<Vec<HostIx>>> = BTreeMap::new();
        for (h, r) in self.hosts.iter().enumerate() {
            for (t, ns) in &r.neighbours {
                if *t < NET_WARM {
                    continue;
                }
                let g = rounds
                    .entry(t.as_secs())
                    .or_insert_with(|| vec![Vec::new(); n]);
                for &x in ns {
                    if let Some(e) = g.get_mut(h) {
                        e.push(x);
                    }
                    if let Some(e) = g.get_mut(x) {
                        e.push(h);
                    }
                }
            }
        }
        rounds
    }

    /// Where the honest viewers of `swarm` sit by their uplinks (`Report::upload_cap`),
    /// as ROW columns; none while their uplinks are alike (three distinct
    /// at least):
    /// - `cap_depth_rho`: Spearman's rank correlation of the uplink with
    ///   the mean push-tree depth (a stream's hops to the seed), and
    ///   `cap_depth_rho_startup` over the first [`BUCKET`]: negative while
    ///   strong uplinks sit near the seed;
    /// - `cap_fanout_rho`: with the mean children;
    /// - `cap_hops_rho`: with the mean hops to the seed in the link graph
    ///   (through hosts other than infrastructure);
    /// - `seed_nbr_up_p50` and `depth1_up_p50`: the median uplink of the
    ///   seed's neighbours, and of the viewers it pushes to, each second's
    ///   median over the run;
    /// - `top_seed_link_s`: when the strongest viewer first neighbours the
    ///   seed, seconds after the stream's start or its arrival, whichever
    ///   came last (the stream's length if never);
    /// - `lag_p99_ms_weak`: the lag p99 of the weakest quarter;
    /// - `seed_offers_per_min`: offers to link the seed answered and
    ///   refused, per minute of run.
    pub fn capacity_position(&self, swarm: usize) -> Vec<(&'static str, f64)> {
        let mut out = Vec::new();
        let Some(spec) = self.scenario.swarms.get(swarm) else {
            return out;
        };
        let up: BTreeMap<HostIx, f64> = self
            .honest_viewers()
            .into_iter()
            .filter(|h| spec.viewers.contains(h))
            .filter_map(|h| Some((h, self.upload_cap(h)? as f64)))
            .collect();
        let mut distinct: Vec<f64> = up.values().copied().collect();
        distinct.sort_by(f64::total_cmp);
        distinct.dedup();
        if distinct.len() < 3 {
            return out;
        }
        let median = |mut v: Vec<f64>| -> Option<f64> {
            v.sort_by(f64::total_cmp);
            v.get(v.len() / 2).copied()
        };
        let rho = |means: &BTreeMap<HostIx, (f64, u64)>| {
            let pairs: Vec<(f64, f64)> = means
                .iter()
                .filter_map(|(h, &(sum, n))| Some((*up.get(h)?, sum / n.max(1) as f64)))
                .collect();
            spearman(&pairs)
        };
        if let Some(walk) = self.depth_walk(swarm) {
            let mean_depth = |to: Duration| {
                let mut m: BTreeMap<HostIx, (f64, u64)> = BTreeMap::new();
                for &(t, v, k) in &walk {
                    if t < to && (1..=7).contains(&k) {
                        let e = m.entry(v).or_default();
                        e.0 += k as f64;
                        e.1 += 1;
                    }
                }
                m
            };
            if let Some(r) = rho(&mean_depth(spec.stop)) {
                out.push(("cap_depth_rho", r));
            }
            if let Some(r) = rho(&mean_depth(spec.start + BUCKET)) {
                out.push(("cap_depth_rho_startup", r));
            }
            let mut by_second: BTreeMap<Duration, Vec<f64>> = BTreeMap::new();
            for &(t, v, k) in &walk {
                if k == 1
                    && let Some(&u) = up.get(&v)
                {
                    by_second.entry(t).or_default().push(u);
                }
            }
            for v in by_second.values_mut() {
                v.dedup();
            }
            if let Some(m) = median(by_second.into_values().filter_map(median).collect()) {
                out.push(("depth1_up_p50", m));
            }
        }
        let mut children: BTreeMap<HostIx, (f64, u64)> = BTreeMap::new();
        for &h in up.keys() {
            for (t, s, x) in self.hosts.get(h).map_or(&[][..], |r| &r.transfers[..]) {
                if *s == swarm && *t >= spec.start + DEPTH_WARM {
                    let e = children.entry(h).or_default();
                    e.0 += x.children as f64;
                    e.1 += 1;
                }
            }
        }
        if let Some(r) = rho(&children) {
            out.push(("cap_fanout_rho", r));
        }
        let rounds = self.link_rounds();
        let infra: Vec<bool> = (0..self.hosts.len())
            .map(|h| self.class(h) == HostClass::Infra)
            .collect();
        let mut hops: BTreeMap<HostIx, (f64, u64)> = BTreeMap::new();
        let mut seed_nbrs = Vec::new();
        let top = up
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(&h, _)| h);
        let mut top_linked: Option<u64> = None;
        for (&t, g) in &rounds {
            let ns = g.get(spec.seed).map_or(&[][..], Vec::as_slice);
            if top.is_some_and(|x| ns.contains(&x)) && top_linked.is_none() {
                top_linked = Some(t);
            }
            if let Some(m) = median(ns.iter().filter_map(|x| up.get(x).copied()).collect()) {
                seed_nbrs.push(m);
            }
            let mut dist = vec![u64::MAX; g.len()];
            let mut queue = std::collections::VecDeque::from([spec.seed]);
            if let Some(d) = dist.get_mut(spec.seed) {
                *d = 0;
            }
            while let Some(x) = queue.pop_front() {
                let dx = dist.get(x).copied().unwrap_or(u64::MAX);
                for &y in g.get(x).map_or(&[][..], Vec::as_slice) {
                    if infra.get(y).copied().unwrap_or(true) {
                        continue;
                    }
                    if let Some(d) = dist.get_mut(y)
                        && *d == u64::MAX
                    {
                        *d = dx + 1;
                        queue.push_back(y);
                    }
                }
            }
            for &h in up.keys() {
                if let Some(&d) = dist.get(h)
                    && d != u64::MAX
                {
                    let e = hops.entry(h).or_default();
                    e.0 += d as f64;
                    e.1 += 1;
                }
            }
        }
        if let Some(r) = rho(&hops) {
            out.push(("cap_hops_rho", r));
        }
        if let Some(m) = median(seed_nbrs) {
            out.push(("seed_nbr_up_p50", m));
        }
        let length = spec.stop.saturating_sub(spec.start).as_secs_f64();
        let came = top.map_or(spec.start, |h| self.presence(h).0.max(spec.start));
        let linked = top_linked.map_or(length, |t| {
            (t as f64 - came.as_secs_f64()).clamp(0.0, length)
        });
        out.push(("top_seed_link_s", linked));
        let mut by_up: Vec<(f64, HostIx)> = up.iter().map(|(&h, &u)| (u, h)).collect();
        by_up.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let weak: Vec<HostIx> = by_up
            .iter()
            .take(by_up.len().div_ceil(4))
            .map(|&(_, h)| h)
            .collect();
        let lag = self.lag_of_between(swarm, &weak, spec.start, spec.stop);
        out.push(("lag_p99_ms_weak", f64::from(lag.p99)));
        let minutes = self.scenario.duration.as_secs_f64() / 60.0;
        if let Some(r) = self.hosts.get(spec.seed)
            && minutes > 0.0
        {
            let answered = r.exchange_outcomes.get("Answered").copied().unwrap_or(0);
            out.push((
                "seed_offers_per_min",
                (answered + r.offers_refused) as f64 / minutes,
            ));
        }
        out
    }

    /// The honest viewers' peer exchanges that ended so.
    pub fn exchanges_ended(&self, outcome: &str) -> u64 {
        self.honest_viewers()
            .into_iter()
            .filter_map(|h| self.hosts.get(h))
            .filter_map(|r| r.exchange_outcomes.get(outcome))
            .sum()
    }

    /// Each viewer's push-tree depth per stream, once a second from
    /// [`DEPTH_WARM`] after the stream's start (and its arrival) while it
    /// was there, from the transfer samples (each host's latest by then): `(second,
    /// viewer, k)`, `k` the hops to the seed (1-6), 7 deeper, 0 no parent,
    /// 8 a cycle, 9 a chain that ends at a viewer with no parent (or one
    /// that departed). `None` without transfer samples.
    fn depth_walk(&self, swarm: usize) -> Option<Vec<(Duration, HostIx, usize)>> {
        let spec = self.scenario.swarms.get(swarm)?;
        let hosts: Vec<HostIx> = spec.viewers.iter().chain(&spec.relays).copied().collect();
        let mut samples: Vec<(Duration, HostIx, &[Option<usize>])> = hosts
            .iter()
            .filter_map(|&h| self.hosts.get(h).map(|r| (h, r)))
            .flat_map(|(h, r)| {
                r.transfers
                    .iter()
                    .filter(|(_, s, _)| *s == swarm)
                    .map(move |(t, _, x)| (*t, h, x.parent_peers.as_slice()))
            })
            .collect();
        if samples.is_empty() {
            return None;
        }
        samples.sort_by_key(|&(t, h, _)| (t, h));
        let streams = samples.iter().map(|s| s.2.len()).max().unwrap_or(0);
        let mut latest: BTreeMap<HostIx, &[Option<usize>]> = BTreeMap::new();
        let mut out = Vec::new();
        let from = spec.start + DEPTH_WARM;
        let mut next = from;
        let mut it = samples.iter().peekable();
        // A host that departed parents no one from then on.
        let left: Vec<Option<Duration>> = (0..self.hosts.len())
            .map(|h| self.scenario.departure(h).map(|(at, _)| at))
            .collect();
        let gone = |x: usize, t: Duration| left.get(x).copied().flatten().is_some_and(|at| at <= t);
        while let Some(&&(t, h, p)) = it.peek() {
            if t >= next {
                // A second's snapshot: every viewer's chain per stream,
                // from its own DEPTH_WARM after its arrival.
                for &v in &hosts {
                    let Some(pv) = latest.get(&v) else { continue };
                    if !self.there(v, next, DEPTH_WARM) {
                        continue;
                    }
                    for st in 0..streams {
                        let parent = |x: usize| {
                            latest
                                .get(&x)
                                .filter(|_| !gone(x, next))
                                .and_then(|p| p.get(st).copied().flatten())
                        };
                        let mut at = pv.get(st).copied().flatten();
                        let mut seen = vec![v];
                        let k = loop {
                            match at {
                                None if seen.len() == 1 => break 0,
                                None => break 9,
                                Some(x) if x == spec.seed && seen.len() <= 6 => break seen.len(),
                                Some(x) if x == spec.seed => break 7,
                                Some(x) if seen.contains(&x) => break 8,
                                Some(x) => {
                                    seen.push(x);
                                    at = parent(x);
                                }
                            }
                        };
                        out.push((next, v, k));
                    }
                }
                next = t.max(next) + Duration::from_secs(1);
                continue;
            }
            latest.insert(h, p);
            it.next();
        }
        Some(out)
    }

    /// The push trees' depth over the viewers' (viewer, stream) pairs,
    /// once a second from `DEPTH_WARM` after the stream's start, from the
    /// transfer samples (each host's latest by then): the shares with no
    /// open parent, `h` hops from the seed (1 to 6), deeper, in a cycle,
    /// and in a chain that ends at a viewer with no open parent. `None`
    /// without transfer samples.
    pub fn depths(&self, swarm: usize) -> Option<Vec<(&'static str, f64)>> {
        let walk = self.depth_walk(swarm)?;
        let mut counts = [0u64; 10];
        for &(_, _, k) in &walk {
            if let Some(c) = counts.get_mut(k) {
                *c += 1;
            }
        }
        let all = counts.iter().sum::<u64>();
        if all == 0 {
            return None;
        }
        let names = [
            "depth_none",
            "depth_1",
            "depth_2",
            "depth_3",
            "depth_4",
            "depth_5",
            "depth_6",
            "depth_deeper",
            "depth_cycle",
            "depth_pull_rooted",
        ];
        Some(
            names
                .into_iter()
                .zip(counts)
                .map(|(n, c)| (n, c as f64 / all as f64))
                .collect(),
        )
    }

    /// Swarm `swarm`'s run second by second, as one JSON object; `None`
    /// without transfer samples ([`Scenario::transfer_samples`]). Second
    /// `k` is `[k, k + 1)` s after the stream's start, through the run's
    /// end. Its fields, the per-second ones as arrays:
    ///
    /// | field | |
    /// |---|---|
    /// | `rate`, `chunk`, `frame` | the stream's bytes per second, chunk and frame sizes |
    /// | `secs` | the stream's length in seconds; the seconds after it are the run's tail |
    /// | `seed_up` | `[s, bytes per second]`: the seed's uplink from the start and at each step, `s` from the stream's start |
    /// | `viewer_up` | the viewers' uplinks, summed |
    /// | `lag`, `frames` | per viewer, the p99 lag in ms of the frames it read in each second (-1 without any) and how many it read; with arrivals or departures, `frames` is -1 in a second it wasn't there in ([`Report::presence`]) |
    /// | `present` | viewers with a transfer sample in the second |
    /// | `hits`, `misses`, `pruned` | over the viewers, pushes before their deadline, deadlines released to pull, and parents pruned |
    /// | `parents` | the viewers' median count of streams with an open parent |
    /// | `capacity`, `busy`, `avail` | the seed's meter at its last reading in the second (capacity, busy in thousandths) and the upload it had then |
    /// | `copies` | the adaptive seed's copies |
    /// | `sent`, `pushed` | chunks the seed sent, and of them pushes |
    /// | `seed_ovh`, `viewer_ovh` | bytes the seed and the honest viewers (summed) wrote to their links that weren't chunk data ([`crate::TransferSample::overhead`]; the row's `up_transport` and `up_acks` have the transports') |
    /// | `written`, `chunks` | a media swarm's chunks published: those holding media, then with those of padding alone |
    ///
    /// [`Scenario::transfer_samples`]: crate::Scenario::transfer_samples
    pub fn series(&self, swarm: usize) -> Option<String> {
        self.series_data(swarm).map(|s| s.json())
    }

    /// [`Report::series`]'s values, for its JSON object and for the
    /// tables ([`Report::write_tables`]).
    pub(crate) fn series_data(&self, swarm: usize) -> Option<SeriesData> {
        let spec = self.scenario.swarms.get(swarm)?;
        let viewers: Vec<_> = self.viewers.iter().filter(|v| v.swarm == swarm).collect();
        let sampled = |h: HostIx| {
            self.hosts.get(h).map_or(Vec::new(), |r| {
                r.transfers
                    .iter()
                    .filter(|(t, s, _)| *s == swarm && *t > spec.start)
                    .map(|(t, _, x)| (*t, x))
                    .collect::<Vec<_>>()
            })
        };
        if viewers.iter().all(|v| sampled(v.host).is_empty()) {
            return None;
        }
        let start = spec.start;
        let n = self.scenario.duration.saturating_sub(start).as_secs() as usize;
        let second = |t: Duration| -> Option<usize> {
            let k = (t.checked_sub(start)?.as_millis() / 1000) as usize;
            (k < n).then_some(k)
        };
        // A transfer sample counts what happened in the second before its
        // tick; a pass over the nodes takes a few ms, so a tick's samples
        // land just after it.
        let sample_second = |t: Duration| {
            let k = (t.checked_sub(start)?.as_millis() + 500) / 1000;
            (k >= 1 && (k as usize) <= n).then(|| k as usize - 1)
        };
        let mut lags = Vec::new();
        let mut frames = Vec::new();
        for v in &viewers {
            let mut by: Vec<Vec<u32>> = vec![Vec::new(); n];
            for &(seq, lag) in &v.stats.lags {
                let written = self.written_at(swarm, seq);
                if let Some(k) = second(written + Duration::from_millis(u64::from(lag)))
                    && let Some(b) = by.get_mut(k)
                {
                    b.push(lag);
                }
            }
            // The seconds the viewer was there for (with arrivals or
            // departures); `frames` is -1 in the others.
            let (arrive, leave) = self.presence(v.host);
            let there = |k: usize| {
                let a = start + Duration::from_secs(k as u64);
                !self.scenario.profiled() || (a + Duration::from_secs(1) > arrive && a < leave)
            };
            lags.push(
                by.iter()
                    .map(|b| {
                        if b.is_empty() {
                            -1
                        } else {
                            i64::from(Percentiles::of(b.iter().copied()).p99)
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            frames.push(
                by.iter()
                    .enumerate()
                    .map(|(k, b)| if there(k) { b.len() as i64 } else { -1 })
                    .collect::<Vec<_>>(),
            );
        }
        // Counters restart from 0 with a restarted node.
        let delta = |now: u64, before: u64| if now >= before { now - before } else { now };
        let (mut hits, mut misses, mut pruned, mut present) =
            (vec![0; n], vec![0; n], vec![0; n], vec![0; n]);
        let mut parents: Vec<Vec<i64>> = vec![Vec::new(); n];
        let mut viewer_ovh = vec![0; n];
        // The push counters are the honest viewers'.
        for v in viewers
            .iter()
            .filter(|v| !matches!(self.class(v.host), HostClass::Hostile(_)))
        {
            let mut last: Option<&crate::TransferSample> = None;
            for (t, x) in sampled(v.host) {
                if let Some(k) = sample_second(t) {
                    let (h, m, p, o) =
                        last.map_or((0, 0, 0, 0), |l| (l.hits, l.misses, l.pruned, l.overhead));
                    if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                        hits.get_mut(k),
                        misses.get_mut(k),
                        pruned.get_mut(k),
                        present.get_mut(k),
                        parents.get_mut(k),
                        viewer_ovh.get_mut(k),
                    ) {
                        *a += delta(x.hits, h) as i64;
                        *b += delta(x.misses, m) as i64;
                        *c += delta(x.pruned, p) as i64;
                        *d += 1;
                        e.push(x.parents as i64);
                        *f += delta(x.overhead, o) as i64;
                    }
                }
                last = Some(x);
            }
        }
        let parents: Vec<i64> = parents
            .iter_mut()
            .map(|p| {
                p.sort_unstable();
                p.get(p.len() / 2).copied().unwrap_or(-1)
            })
            .collect();
        let seed = spec.seed;
        let (mut sent, mut pushed, mut seed_ovh) = (vec![0; n], vec![0; n], vec![0; n]);
        let mut last: Option<&crate::TransferSample> = None;
        for (t, x) in sampled(seed) {
            if let Some(k) = sample_second(t)
                && let (Some(a), Some(b), Some(o)) =
                    (sent.get_mut(k), pushed.get_mut(k), seed_ovh.get_mut(k))
            {
                let (c, p, v) =
                    last.map_or((0, 0, 0), |l| (l.chunks_sent, l.pushed_chunks, l.overhead));
                *a += delta(x.chunks_sent, c) as i64;
                *b += delta(x.pushed_chunks, p) as i64;
                *o += delta(x.overhead, v) as i64;
            }
            last = Some(x);
        }
        let (mut capacity, mut busy) = (vec![-1; n], vec![-1; n]);
        if let Some(h) = self.hosts.get(seed) {
            for m in &h.runner.meter {
                if let Some(k) = second(m.t)
                    && let (Some(c), Some(b)) = (capacity.get_mut(k), busy.get_mut(k))
                {
                    *c = m.capacity as i64;
                    *b = i64::from(m.busy_permille);
                }
            }
        }
        let at = |k: usize| start + Duration::from_millis(k as u64 * 1000 + 500);
        let avail: Vec<i64> = (0..n)
            .map(|k| self.upload_cap_at(seed, at(k)).map_or(-1, |c| c as i64))
            .collect();
        let copies: Vec<i64> = (0..n)
            .map(|k| {
                self.hosts.get(seed).map_or(-1, |h| {
                    h.runner
                        .seed_changes
                        .iter()
                        .rev()
                        .find(|(t, s, _)| *s == swarm && *t <= at(k))
                        .map_or(0, |(_, _, x)| x.copies as i64)
                })
            })
            .collect();
        let host = self.scenario.hosts.get(seed);
        let mut seed_up = vec![format!(
            "[0,{}]",
            host.and_then(|h| h.site).map_or(0, |s| s.profile.up)
        )];
        for &(t, rate) in host.map_or(&[][..], |h| &h.up_steps[..]) {
            seed_up.push(format!(
                "[{:.1},{rate}]",
                t.saturating_sub(start).as_secs_f64()
            ));
        }
        let viewer_up: u64 = viewers.iter().filter_map(|v| self.upload_cap(v.host)).sum();
        let media = self.media.get(swarm).and_then(Option::as_ref);
        let frame = media.map_or(crate::stream::FRAME_LEN as u64, |_| {
            spec.options.chunk_size as u64
        });
        let hostile: Vec<i64> = viewers
            .iter()
            .enumerate()
            .filter(|(_, v)| matches!(self.class(v.host), HostClass::Hostile(_)))
            .map(|(i, _)| i as i64)
            .collect();
        let mut cols = vec![
            ("present", present),
            ("hits", hits),
            ("misses", misses),
            ("pruned", pruned),
            ("parents", parents),
            ("capacity", capacity),
            ("busy", busy),
            ("avail", avail),
            ("copies", copies),
            ("sent", sent),
            ("pushed", pushed),
            ("seed_ovh", seed_ovh),
            ("viewer_ovh", viewer_ovh),
        ];
        // A media swarm's chunks sent each second, those holding media, by
        // their first byte (`written`, what `frames` counts): its other
        // chunks are data-less.
        if let Some(m) = media {
            let mut written = vec![0i64; n];
            for &t in &m.times {
                if let Some(w) = second(Duration::from_millis(t)).and_then(|i| written.get_mut(i)) {
                    *w += 1;
                }
            }
            cols.push(("written", written));
        }
        Some(SeriesData {
            rate: self.swarm_rate(swarm) as u64,
            chunk: spec.options.chunk_size as u64,
            frame,
            secs: spec.stop.saturating_sub(start).as_secs(),
            seed_up,
            viewer_up,
            lag: lags,
            frames,
            cols,
            hostile,
        })
    }

    /// A media swarm's columns ([`crate::MediaSpec`]): its encoder and
    /// policy, the chunks per unit, the media and stream rates, units
    /// signed a second (each peer's signature verifications), each media
    /// chunk's wait to be signed, and every host's upload per media byte
    /// per viewer.
    fn media_cols(&self, swarm: usize, j: &mut JsonRow) {
        let (Some(spec), Some(Some(m))) = (self.scenario.swarms.get(swarm), self.media.get(swarm))
        else {
            return;
        };
        let Some(media) = spec.media else { return };
        let secs = spec.stop.saturating_sub(spec.start).as_secs_f64().max(1e-9);
        let waits = Percentiles::of(m.sign_waits.iter().copied());
        let uploaded: u64 = self.hosts.iter().map(|h| h.uploaded).sum();
        let viewers = spec.viewers.len().max(1) as f64;
        j.str("media", &media.encoder.to_string())
            .str("sign", &media.sign.to_string())
            .int("unit", m.unit_chunks)
            .num("media_kbit", m.media_bytes as f64 * 8e-3 / secs)
            .num("sent_kbit", m.sent_bytes as f64 * 8e-3 / secs)
            .num(
                "stream_per_media",
                m.sent_bytes as f64 / m.media_bytes.max(1) as f64,
            )
            .num("units_per_s", m.units as f64 / secs)
            .int("sign_wait_p50_ms", u64::from(waits.p50))
            .int("sign_wait_p99_ms", u64::from(waits.p99))
            .num(
                "wire_per_media",
                uploaded as f64 / (m.media_bytes.max(1) as f64 * viewers),
            );
    }
}

/// A swarm's run second by second ([`Report::series`]): its stream, each
/// viewer's `lag` and `frames` by second, and the per-second columns in
/// the JSON object's order.
pub(crate) struct SeriesData {
    pub rate: u64,
    pub chunk: u64,
    pub frame: u64,
    pub secs: u64,
    /// `[s, bytes per second]` pairs, as the JSON object writes them.
    pub seed_up: Vec<String>,
    pub viewer_up: u64,
    pub lag: Vec<Vec<i64>>,
    pub frames: Vec<Vec<i64>>,
    pub cols: Vec<(&'static str, Vec<i64>)>,
    /// The hostile viewers' places in `lag` and `frames`.
    pub hostile: Vec<i64>,
}

impl SeriesData {
    /// The seconds in the series.
    pub fn len(&self) -> usize {
        self.cols.first().map_or(0, |(_, v)| v.len())
    }

    fn json(&self) -> String {
        let ints = |v: &[i64]| {
            let v: Vec<String> = v.iter().map(i64::to_string).collect();
            format!("[{}]", v.join(","))
        };
        let nested = |v: &[Vec<i64>]| {
            let v: Vec<String> = v.iter().map(|x| ints(x)).collect();
            format!("[{}]", v.join(","))
        };
        let mut j = JsonRow::default();
        j.int("rate", self.rate)
            .int("chunk", self.chunk)
            .int("frame", self.frame)
            .int("secs", self.secs)
            .field("seed_up", format_args!("[{}]", self.seed_up.join(",")))
            .int("viewer_up", self.viewer_up)
            .field("lag", format_args!("{}", nested(&self.lag)))
            .field("frames", format_args!("{}", nested(&self.frames)));
        for (name, v) in &self.cols {
            j.field(name, format_args!("{}", ints(v)));
        }
        // The hostile viewers' places in `lag` and `frames`, if any: `hits`,
        // `misses`, `pruned`, `present` and `parents` leave them out.
        if !self.hostile.is_empty() {
            j.field("hostile", format_args!("{}", ints(&self.hostile)));
        }
        j.finish()
    }
}

/// A JSON object written field by field.
#[derive(Default)]
struct JsonRow(String);

impl JsonRow {
    fn field(&mut self, key: &str, value: std::fmt::Arguments) -> &mut Self {
        self.0.push(if self.0.is_empty() { '{' } else { ',' });
        let _ = write!(self.0, "\"{key}\":{value}");
        self
    }

    fn int(&mut self, key: &str, v: u64) -> &mut Self {
        self.field(key, format_args!("{v}"))
    }

    fn ints(&mut self, key: &str, v: impl Iterator<Item = u64>) -> &mut Self {
        let v: Vec<String> = v.map(|x| x.to_string()).collect();
        self.field(key, format_args!("[{}]", v.join(",")))
    }

    fn signed_ints(&mut self, key: &str, v: impl Iterator<Item = i64>) -> &mut Self {
        let v: Vec<String> = v.map(|x| x.to_string()).collect();
        self.field(key, format_args!("[{}]", v.join(",")))
    }

    /// JSON has no NaN or infinity, so those are null.
    fn num(&mut self, key: &str, v: f64) -> &mut Self {
        if v.is_finite() {
            self.field(key, format_args!("{v}"))
        } else {
            self.field(key, format_args!("null"))
        }
    }

    fn nums(&mut self, key: &str, v: impl Iterator<Item = f64>) -> &mut Self {
        let v: Vec<String> = v
            .map(|x| {
                if x.is_finite() {
                    format!("{x:.4}")
                } else {
                    "null".to_owned()
                }
            })
            .collect();
        self.field(key, format_args!("[{}]", v.join(",")))
    }

    /// A string without quotes, backslashes or control characters (names
    /// and hashes).
    fn str(&mut self, key: &str, v: &str) -> &mut Self {
        let v: String = v
            .chars()
            .filter(|c| *c != '"' && *c != '\\' && !c.is_control())
            .collect();
        self.field(key, format_args!("\"{v}\""))
    }

    fn finish(&mut self) -> String {
        if self.0.is_empty() {
            self.0.push('{');
        }
        self.0.push('}');
        std::mem::take(&mut self.0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing, clippy::unwrap_used)]

    use super::*;
    use crate::report::HostReport;
    use crate::spec::{Scenario, SwarmSpec};
    use crate::stats::MeterSample;
    use strims_wan::{Access, City, Site};

    /// A seed on a 1000 B/s uplink that halves from 20 s to 60 s, its
    /// meter reading `capacity(s)` each second of a 100 s stream.
    fn seed_reading(capacity: impl Fn(u64) -> u64) -> Report {
        let mut s = Scenario::new("tracking");
        let seed = s.host_at(
            "seed",
            Site::new(City::Chicago, Access::Cable).with_up(1000),
        );
        let viewer = s.host_at("v", Site::new(City::Chicago, Access::Cable));
        s.hosts[seed].uplink = None;
        s.hosts[seed].up_steps = vec![
            (Duration::from_secs(20), 500),
            (Duration::from_secs(60), 1000),
        ];
        s.swarm(SwarmSpec::live(
            seed,
            vec![viewer],
            1024,
            Duration::ZERO,
            Duration::from_secs(100),
        ));
        let mut host = HostReport::default();
        host.runner.meter = (0..100)
            .map(|t| MeterSample {
                t: Duration::from_secs(t),
                capacity: capacity(t),
                ..MeterSample::default()
            })
            .collect();
        Report {
            scenario: s,
            seed: 1,
            trace_hash: String::new(),
            trace: Vec::new(),
            sim_secs: 0.0,
            wall_secs: 0.0,
            viewers: Vec::new(),
            hosts: vec![host, HostReport::default()],
            written: Vec::new(),
            media: Vec::new(),
            vpn: None,
            memory: None,
            violations: Vec::new(),
            wan: None,
            wire_bins: Vec::new(),
            parent_losses: Vec::new(),
            parent_changes: Vec::new(),
        }
    }

    /// A seed (0), three honest viewers (1-3) and a hostile host (4), each
    /// reporting `links` (by host) at every second from 10 s to 19 s.
    fn linked(links: [&[HostIx]; 5]) -> Report {
        let mut s = Scenario::new("net");
        let hosts: Vec<HostIx> = (0..5).map(|i| s.host(&format!("h{i}"), None)).collect();
        s.hosts[4].class = HostClass::Hostile("test");
        s.swarm(SwarmSpec::live(
            hosts[0],
            vec![1, 2, 3, 4],
            1024,
            Duration::ZERO,
            Duration::from_secs(20),
        ));
        let mut r = seed_reading(|_| 0);
        r.scenario = s;
        r.hosts = links
            .iter()
            .map(|ns| HostReport {
                neighbours: (10..20)
                    .map(|t| (Duration::from_secs(t), ns.to_vec()))
                    .collect(),
                ..HostReport::default()
            })
            .collect();
        r
    }

    /// A seed (0) and a chain of four viewers, each the parent of the
    /// next, their uplinks falling down the chain and their links the
    /// chain's, every second from 10 s to 19 s.
    #[test]
    fn capacity_position_ranks_uplinks_against_depth_and_hops() {
        let mut s = Scenario::new("chain");
        let seed = s.host("seed", Some(9_000));
        let vs: Vec<HostIx> = [4_000, 3_000, 2_000, 1_000]
            .iter()
            .enumerate()
            .map(|(i, &up)| s.host(&format!("v{i}"), Some(up)))
            .collect();
        s.swarm(SwarmSpec::live(
            seed,
            vs.clone(),
            1024,
            Duration::ZERO,
            Duration::from_secs(20),
        ));
        let mut r = seed_reading(|_| 0);
        r.scenario = s;
        let parent = |h: HostIx| if h == 1 { 0 } else { h - 1 };
        r.hosts = (0..5)
            .map(|h: HostIx| HostReport {
                neighbours: (10..20)
                    .map(|t| {
                        (
                            Duration::from_secs(t),
                            if h == 4 { vec![3] } else { vec![h + 1] },
                        )
                    })
                    .collect(),
                transfers: (0..20)
                    .map(|t| {
                        let x = strims_testbed::transfer::TransferSample {
                            parent_peers: if h == 0 {
                                vec![None]
                            } else {
                                vec![Some(parent(h))]
                            },
                            children: usize::from(h < 4),
                            ..Default::default()
                        };
                        (Duration::from_secs(t), 0, x)
                    })
                    .collect(),
                ..HostReport::default()
            })
            .collect();
        let got: BTreeMap<&str, f64> = r.capacity_position(0).into_iter().collect();
        assert!((got["cap_depth_rho"] + 1.0).abs() < 1e-9, "{got:?}");
        assert!((got["cap_hops_rho"] + 1.0).abs() < 1e-9, "{got:?}");
        assert_eq!(got["seed_nbr_up_p50"], 4_000.0);
        assert_eq!(got["depth1_up_p50"], 4_000.0);
        assert_eq!(got["top_seed_link_s"], 10.0);
    }

    #[test]
    fn eclipse_counts_viewers_cut_off_by_hostile_hosts() {
        // 1 links the seed and 2; 3 has only the hostile host, through
        // which 2's path would go too: 3 is eclipsed, and cut off.
        let r = linked([&[1], &[0, 2], &[1, 4], &[4], &[2, 3]]);
        let [eclipsed, cut] = r.eclipse();
        assert!((eclipsed - 1.0 / 3.0).abs() < 1e-9, "{eclipsed}");
        assert!((cut - 1.0 / 3.0).abs() < 1e-9, "{cut}");
        assert_eq!(r.link_counts(), [2, 2]);
        // Links reported by one end count for both.
        let r = linked([&[], &[0], &[1], &[2], &[]]);
        assert_eq!(r.eclipse(), [0.0, 0.0]);
    }

    /// A viewer reads a frame every 250 ms 100 ms after its write, for
    /// 10 s, then nothing for 10 s, then again to the stream's end.
    #[test]
    fn a_viewer_that_reads_nothing_falls_behind_each_second() {
        let mut r = seed_reading(|_| 0);
        r.scenario.swarms[0] =
            SwarmSpec::live(0, vec![1], 4096, Duration::ZERO, Duration::from_secs(100));
        let mut stats = crate::stream::ViewerStats::default();
        stats.lags = (0..40).chain(80..400).map(|seq| (seq, 100)).collect();
        let v = crate::report::ViewerReport {
            host: 1,
            name: "v".into(),
            swarm: 0,
            stats,
        };
        let lag = r.lag_by_second(0, &v);
        assert_eq!(lag.len(), 100);
        assert_eq!(lag[9], Some(100));
        assert_eq!(lag[10], Some(1100));
        assert_eq!(lag[19], Some(10_100));
        assert_eq!(lag[20], Some(100));
        // It stays through departures from 8 s: over the target from 11 s
        // to 19 s.
        r.viewers = vec![v];
        r.scenario.departures = Some("50:8:2".parse().unwrap());
        let rec = r.departure_recovery(0).unwrap();
        let over: u64 = (2..=10).map(|j| j * 1000 + 100 - 1250).sum();
        assert_eq!(
            rec,
            Recovery {
                stayed: 1,
                stranded: 1,
                peak_ms: 100,
                excess_ms_s: over,
                recover_ms: 12_000,
                censored: false,
            }
        );
    }

    #[test]
    fn tracking_counts_the_samples_off_until_the_next_change() {
        let r = seed_reading(|t| match t {
            30..=64 => 500,
            66 => 600,
            _ => 1000,
        });
        assert_eq!(r.upload_cap_at(0, Duration::from_secs(19)), Some(1000));
        assert_eq!(r.upload_cap_at(0, Duration::from_secs(20)), Some(500));
        let t = r.tracking();
        assert_eq!(t.len(), 2);
        // 20-29 of 20-59 read twice the half rate.
        assert!(t[0].drop);
        assert!((t[0].off - 0.25).abs() < 1e-9, "{}", t[0].off);
        // 60-64 and 66 of 60-99 read off the full rate.
        assert!(!t[1].drop);
        assert!((t[1].off - 0.15).abs() < 1e-9, "{}", t[1].off);
    }

    #[test]
    fn a_meter_that_never_follows_reads_off_throughout() {
        let r = seed_reading(|_| 1000);
        let t = r.tracking();
        assert!((t[0].off - 1.0).abs() < 1e-9);
        assert!(t[1].off.abs() < 1e-9);
    }
}
