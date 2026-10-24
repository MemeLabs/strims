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
//!   in samples where the node's upload rate was under half its cap.
//! - **queueing delay accuracy** ([`Report::queue_delay_accuracy`]): the
//!   meter's queueing delay against the standing queue on the host's
//!   uplink, the least wait of the WAN model's queue over the last
//!   [`STANDING_WINDOW`], and whether the delay reads past a threshold
//!   exactly when the uplink stayed busy throughout that window (WAN
//!   mode).

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
        let Some(s) = self.scenario.swarms.get(swarm) else {
            return Percentiles::default();
        };
        let range = s.frames_between(from, to);
        Percentiles::of(
            self.viewers
                .iter()
                .filter(|v| v.swarm == swarm)
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
    /// its cap, and all declined, over every host.
    pub fn declined(&self) -> (u64, u64) {
        let (mut wrong, mut all) = (0, 0);
        for (h, host) in self.hosts.iter().enumerate() {
            let cap = self.upload_cap(h).unwrap_or(0) as f64;
            let mut last = 0;
            for m in &host.runner.meter {
                let d = if m.rejected >= last {
                    m.rejected - last
                } else {
                    m.rejected
                };
                last = m.rejected;
                all += u64::from(d);
                if cap > 0.0 && (m.rate as f64) < cap / 2.0 {
                    wrong += u64::from(d);
                }
            }
        }
        (wrong, all)
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
            .num("sim_secs", self.sim_secs)
            .num("wall_secs", self.wall_secs);
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
