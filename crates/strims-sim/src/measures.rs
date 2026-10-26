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
//!   mode).
//! - **series** ([`Report::series`]): with transfer samples, a swarm's run
//!   second by second from its stream's start to the run's end, for
//!   `hack/capture/analyze.py`'s collapse verdicts: each viewer's lag and
//!   frames read, the viewers' pushes, prunes and parents, and the seed's
//!   meter, uplink and copies.

use std::fmt::Write;
use std::time::Duration;

use crate::report::{Percentiles, Report};
use crate::spec::HostIx;

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
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return Percentiles::default();
        };
        let ranges: Vec<_> = spans
            .iter()
            .map(|&(from, to)| s.frames_between(from, to))
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

    /// The meter's queueing delay at `hosts` against their standing uplink
    /// queues, over the samples that had a delay while a swarm streamed:
    /// the median absolute error in milliseconds; the share of samples
    /// whose uplink emptied in the window where the delay read at least
    /// `threshold_ms` (false positives); and the share whose uplink stayed
    /// busy where it read less (false negatives). `None` outside WAN mode
    /// or without samples.
    pub fn queue_delay_accuracy(&self, hosts: &[HostIx], threshold_ms: u32) -> Option<[f64; 3]> {
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
                let Some(delay) = m.queue_delay_ms else {
                    continue;
                };
                let from = m.t.saturating_sub(STANDING_WINDOW);
                let Some(standing) = queue
                    .iter()
                    .filter(|(t, _)| *t >= from && *t <= m.t)
                    .map(|&(_, d)| d)
                    .min()
                else {
                    continue;
                };
                errors.push(f64::from(delay.abs_diff(standing)));
                if standing < SATURATED_STANDING_MS {
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

    /// The run's measures as one JSON object, for scripts: the test, seed
    /// and knobs, whether the test's check passed (`passed`, 0 or 1, and
    /// its `failure`, empty when it passed), then per scenario swarm 0 its
    /// delivery, lag (overall, startup bucket, steady state, per bucket)
    /// and the meter's measures (see the module docs).
    pub fn row(&self, test: &str, failure: Option<&str>) -> String {
        let mut j = JsonRow::default();
        j.str("test", test)
            .str("scenario", &self.scenario.name)
            .int("seed", self.seed)
            .str("trace", self.trace_hash.get(..16).unwrap_or_default())
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
            );
        let swarm = 0;
        let lag = self.lag(swarm);
        let steady = self.steady_lag(swarm);
        let buckets = self.lag_buckets(swarm);
        let startup = buckets.first().copied().unwrap_or_default();
        j.num("delivery", self.delivery(swarm))
            .int("lag_p50_ms", u64::from(lag.p50))
            .int("lag_p99_ms", u64::from(lag.p99))
            .int("startup_p50_ms", u64::from(startup.p50))
            .int("startup_p99_ms", u64::from(startup.p99))
            .int("steady_p50_ms", u64::from(steady.p50))
            .int("steady_p99_ms", u64::from(steady.p99))
            .ints("bucket_p50_ms", buckets.iter().map(|p| u64::from(p.p50)))
            .ints("bucket_p99_ms", buckets.iter().map(|p| u64::from(p.p99)));
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
        for (name, hosts) in [("seeds", &roles.seeds), ("forwarders", &roles.forwarders)] {
            if let Some([err, fp, fneg]) =
                self.queue_delay_accuracy(hosts, QUEUE_DELAY_THRESHOLD_MS)
            {
                j.num(&format!("qd_err_{name}_ms"), err)
                    .num(&format!("qd_fp_{name}"), fp)
                    .num(&format!("qd_fn_{name}"), fneg);
            }
        }
        let (wrong, declined) = self.declined();
        j.int("declined_wrongly", wrong).int("declined", declined);
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
        let c = self.swarm_counters(swarm);
        j.num("seed_data_share", self.data_share(swarm))
            .num("seed_share", self.uploads(swarm).seed_share)
            .num("duplicates", self.duplicates(swarm))
            .int("push_hits", c.hits)
            .int("push_misses", c.misses)
            .int("pruned", c.pruned)
            .int("shed", c.shed)
            .int("switched", c.switched)
            .int("timeouts", c.timeouts)
            .num("sim_secs", self.sim_secs)
            .num("wall_secs", self.wall_secs);
        if let Some(series) = self.series(swarm) {
            j.field("series", format_args!("{series}"));
        }
        j.finish()
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
    /// | `lag`, `frames` | per viewer, the p99 lag in ms of the frames it read in each second (-1 without any) and how many it read |
    /// | `present` | viewers with a transfer sample in the second |
    /// | `hits`, `misses`, `pruned` | over the viewers, pushes before their deadline, deadlines released to pull, and parents pruned |
    /// | `parents` | the viewers' median count of streams with an open parent |
    /// | `capacity`, `busy`, `avail` | the seed's meter at its last reading in the second (capacity, busy in thousandths) and the upload it had then |
    /// | `copies` | the adaptive seed's copies |
    /// | `sent`, `pushed` | chunks the seed sent, and of them pushes |
    ///
    /// [`Scenario::transfer_samples`]: crate::Scenario::transfer_samples
    pub fn series(&self, swarm: usize) -> Option<String> {
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
        let ints = |v: &[i64]| {
            let v: Vec<String> = v.iter().map(i64::to_string).collect();
            format!("[{}]", v.join(","))
        };
        let fpw = spec.frames_per_write.max(1) as u64;
        let mut lags = Vec::new();
        let mut frames = Vec::new();
        for v in &viewers {
            let mut by: Vec<Vec<u32>> = vec![Vec::new(); n];
            for &(seq, lag) in &v.stats.lags {
                let written =
                    start + spec.write_interval * u32::try_from(seq / fpw).unwrap_or(u32::MAX);
                if let Some(k) = second(written + Duration::from_millis(u64::from(lag)))
                    && let Some(b) = by.get_mut(k)
                {
                    b.push(lag);
                }
            }
            lags.push(ints(
                &by.iter()
                    .map(|b| {
                        if b.is_empty() {
                            -1
                        } else {
                            i64::from(Percentiles::of(b.iter().copied()).p99)
                        }
                    })
                    .collect::<Vec<_>>(),
            ));
            frames.push(ints(&by.iter().map(|b| b.len() as i64).collect::<Vec<_>>()));
        }
        // Counters restart from 0 with a restarted node.
        let delta = |now: u64, before: u64| if now >= before { now - before } else { now };
        let (mut hits, mut misses, mut pruned, mut present) =
            (vec![0; n], vec![0; n], vec![0; n], vec![0; n]);
        let mut parents: Vec<Vec<i64>> = vec![Vec::new(); n];
        for v in &viewers {
            let mut last: Option<&crate::TransferSample> = None;
            for (t, x) in sampled(v.host) {
                if let Some(k) = sample_second(t) {
                    let (h, m, p) = last.map_or((0, 0, 0), |l| (l.hits, l.misses, l.pruned));
                    if let (Some(a), Some(b), Some(c), Some(d), Some(e)) = (
                        hits.get_mut(k),
                        misses.get_mut(k),
                        pruned.get_mut(k),
                        present.get_mut(k),
                        parents.get_mut(k),
                    ) {
                        *a += delta(x.hits, h) as i64;
                        *b += delta(x.misses, m) as i64;
                        *c += delta(x.pruned, p) as i64;
                        *d += 1;
                        e.push(x.parents as i64);
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
        let (mut sent, mut pushed) = (vec![0; n], vec![0; n]);
        let mut last: Option<&crate::TransferSample> = None;
        for (t, x) in sampled(seed) {
            if let Some(k) = sample_second(t)
                && let (Some(a), Some(b)) = (sent.get_mut(k), pushed.get_mut(k))
            {
                let (c, p) = last.map_or((0, 0), |l| (l.chunks_sent, l.pushed_chunks));
                *a += delta(x.chunks_sent, c) as i64;
                *b += delta(x.pushed_chunks, p) as i64;
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
        let mut j = JsonRow::default();
        j.int("rate", spec.rate() as u64)
            .int("chunk", spec.options.chunk_size as u64)
            .int("frame", crate::stream::FRAME_LEN as u64)
            .int("secs", spec.stop.saturating_sub(start).as_secs())
            .field("seed_up", format_args!("[{}]", seed_up.join(",")))
            .int("viewer_up", viewer_up)
            .field("lag", format_args!("[{}]", lags.join(",")))
            .field("frames", format_args!("[{}]", frames.join(",")))
            .field("present", format_args!("{}", ints(&present)))
            .field("hits", format_args!("{}", ints(&hits)))
            .field("misses", format_args!("{}", ints(&misses)))
            .field("pruned", format_args!("{}", ints(&pruned)))
            .field("parents", format_args!("{}", ints(&parents)))
            .field("capacity", format_args!("{}", ints(&capacity)))
            .field("busy", format_args!("{}", ints(&busy)))
            .field("avail", format_args!("{}", ints(&avail)))
            .field("copies", format_args!("{}", ints(&copies)))
            .field("sent", format_args!("{}", ints(&sent)))
            .field("pushed", format_args!("{}", ints(&pushed)));
        Some(j.finish())
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
    #![allow(clippy::indexing_slicing)]

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
            vpn: None,
            memory: None,
            violations: Vec::new(),
            wan: None,
        }
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
