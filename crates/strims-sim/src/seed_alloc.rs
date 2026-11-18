// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A swarm's seed's allocation over a run, from its stats samples
//! ([`AllocSample`], `strims_node::event::SeedAllocationInfo`), and what
//! the reports made of its holders, against the real push trees:
//!
//! - **slots and objective**: the stream slots it pushes (one stream at
//!   one neighbour), and its objective, the p90 over its neighbours of each
//!   one's worst stream's delay as its reports give it
//!   ([`Report::seed_slots`], [`Report::seed_objective`]), over its samples
//!   in the seed meter's window ([`Report::seed_meter_window`]).
//! - **holders**: an adaptive seed's holders of a copy, or where it has
//!   none the neighbours it pushes anything to ([`holders`]); the samples
//!   at which they changed, and when they last did
//!   ([`Report::holder_changes`], [`Report::settled`]).
//! - **verdicts** ([`Report::verdicts`]): at each sample in the window, the
//!   holders' standing in the seed's tally (judged by a quorum of
//!   reporters, seen to serve, its pushes keeping up) against their real
//!   children, the hosts whose open parent of a stream they were at their
//!   transfer samples nearest the seed's.
//! - **reports** ([`Report::report_counts`]): what became of the honest
//!   viewers' report entries at the seed, every channel's.
//! - **announces** ([`Report::announces`]): the ANNOUNCEs the seed read
//!   and sent on the swarm's channels.
//! - **asks of the seed** ([`Report::seed_asks`]): with the request logs,
//!   the chunks the honest viewers asked the seed for, by how many of the
//!   asker's other neighbours held them for [`HELD_FOR`] then, and of
//!   those none held, the ones whose stream had no open parent at the
//!   asker.
//!
//! Everything here reads the records alone: nothing is drawn or decided.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use strims_base::stats::sketch;
use strims_node::event::SeedAllocationInfo;
use strims_ppspp::ReportCounters;

use crate::fields::{JsonRow, RowInt, RowNum};
use crate::report::Report;
use crate::spec::{HostClass, HostIx};

/// A delay a neighbour reported no copies of lately counts as this in the
/// seed's objective ([`Report::seed_objective`]).
pub const STALE: strims_base::time::Duration = strims_base::time::Duration::from_secs(2);

/// A hybrid seed's allocation at one stats sample: the node's view, and
/// each neighbour's host in its order (`None`: one the run can't name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocSample {
    pub info: SeedAllocationInfo,
    pub hosts: Vec<Option<HostIx>>,
}

/// A host's open parent of a stream from a time on: `None` for none.
type ParentAt = (Duration, Option<HostIx>);

/// How long a neighbour must have held a chunk for its HAVE to have
/// reached a viewer that asks the seed for it ([`Report::seed_asks`]).
pub const HELD_FOR: Duration = Duration::from_millis(100);

/// The hostile kinds that announce what they hold and serve none of it:
/// the liars of [`Verdicts`].
const LIARS: [&str; 3] = ["liar", "liar+echo", "colluder"];

/// The reports' verdicts against the real subtrees ([`Report::verdicts`]):
/// counts of holder samples (a neighbour holding a copy at a sample).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Verdicts {
    /// Honest holder samples, those with a real child, and those with one
    /// among the seed's neighbours (the ones that can report).
    pub honest: u64,
    pub with_children: u64,
    pub with_reporters: u64,
    /// Honest holder samples a quorum judged, those it saw serve, and of
    /// those the ones without a real child.
    pub judged: u64,
    pub served: u64,
    pub served_childless: u64,
    /// Liar holder samples, those judged, and those served.
    pub liars: u64,
    pub liars_judged: u64,
    pub liars_served: u64,
    /// Honest holder samples at which the seed pushed to the holder and
    /// its pushes didn't keep up.
    pub not_kept: u64,
    /// Honest neighbours barred at a sample and not at the one before that
    /// held a copy with a real child at the one before.
    pub barred_with_children: u64,
}

/// The ANNOUNCEs on a swarm's channels at its seed ([`Report::announces`]):
/// read (and their bytes) and sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Announces {
    pub read: u64,
    pub bytes: u64,
    pub sent: u64,
}

/// A sample's holders: an adaptive seed's holders of a copy, or where it
/// has none the neighbours it pushes anything to.
pub fn holders(a: &AllocSample) -> BTreeSet<HostIx> {
    let named = || a.info.neighbours.iter().zip(&a.hosts);
    let held: BTreeSet<HostIx> = named()
        .filter(|(n, _)| n.holder)
        .filter_map(|(_, h)| *h)
        .collect();
    if !held.is_empty() {
        return held;
    }
    named()
        .filter(|(n, _)| !n.streams.is_empty())
        .filter_map(|(_, h)| *h)
        .collect()
}

impl Report {
    /// `swarm`'s seed's allocation samples, in order.
    pub fn seed_samples(&self, swarm: usize) -> Vec<(Duration, &AllocSample)> {
        let Some(seed) = self.scenario.swarms.get(swarm).map(|s| s.seed) else {
            return Vec::new();
        };
        self.hosts.get(seed).map_or(Vec::new(), |h| {
            h.seed_allocs
                .iter()
                .filter(|(_, s, _)| *s == swarm)
                .map(|(t, _, a)| (*t, a))
                .collect()
        })
    }

    /// `swarm`'s seed's samples in the seed meter's window.
    fn window_samples(&self, swarm: usize) -> Vec<(Duration, &AllocSample)> {
        let Some((from, to)) = self.seed_meter_window(swarm) else {
            return Vec::new();
        };
        self.seed_samples(swarm)
            .into_iter()
            .filter(|&(t, _)| t >= from && t <= to)
            .collect()
    }

    /// The streams pushed to the seed's neighbours, summed, mean over its
    /// samples in the window: the row's `seed_slots`.
    pub fn seed_slots(&self, swarm: usize) -> Option<f64> {
        let w = self.window_samples(swarm);
        let slots: usize = w
            .iter()
            .map(|(_, a)| {
                a.info
                    .neighbours
                    .iter()
                    .map(|n| n.streams.len())
                    .sum::<usize>()
            })
            .sum();
        (!w.is_empty()).then(|| slots as f64 / w.len() as f64)
    }

    /// The seed's objective, in ms, median over its samples in the window:
    /// the p90 over its neighbours of each one's worst stream's delay, a
    /// stream without one counting as [`STALE`], each delay less the
    /// neighbour's clock's offset from the seed's (its reports time the
    /// seed's timestamps by its own clock). The row's `seed_objective_ms`.
    pub fn seed_objective(&self, swarm: usize) -> Option<u64> {
        let seed = self.scenario.swarms.get(swarm)?.seed;
        let offset = |h: Option<HostIx>| {
            h.and_then(|h| self.hosts.get(h))
                .map_or(strims_base::time::Duration::ZERO, |r| r.clock_offset)
        };
        let seed_offset = offset(Some(seed));
        let mut objectives: Vec<u64> = self
            .window_samples(swarm)
            .iter()
            .filter_map(|(_, a)| {
                let mut worst: Vec<u64> = a
                    .info
                    .neighbours
                    .iter()
                    .zip(&a.hosts)
                    .map(|(n, &h)| {
                        let off = offset(h) - seed_offset;
                        let w = n
                            .lags
                            .iter()
                            .map(|l| l.map_or(STALE, |l| (l - off).max(Default::default())))
                            .max()
                            .unwrap_or(STALE);
                        w.as_millis().max(0).unsigned_abs()
                    })
                    .collect();
                worst.sort_unstable();
                sketch::percentile(&worst, 90)
            })
            .collect();
        objectives.sort_unstable();
        sketch::percentile(&objectives, 50)
    }

    /// The seed's samples in the window at which its holders ([`holders`])
    /// differ from the sample before: the row's `holder_changes`.
    pub fn holder_changes(&self, swarm: usize) -> Option<u64> {
        let (from, to) = self.seed_meter_window(swarm)?;
        let s = self.seed_samples(swarm);
        (!s.is_empty()).then(|| {
            s.iter()
                .zip(s.iter().skip(1))
                .filter(|(a, b)| b.0 >= from && b.0 <= to && holders(a.1) != holders(b.1))
                .count() as u64
        })
    }

    /// When the seed's holders last changed, in seconds from the swarm's
    /// start (0 if they never did): the row's `settled_s`.
    pub fn settled(&self, swarm: usize) -> Option<f64> {
        let start = self.scenario.swarms.get(swarm)?.start;
        let s = self.seed_samples(swarm);
        (!s.is_empty()).then(|| {
            s.iter()
                .zip(s.iter().skip(1))
                .rfind(|(a, b)| holders(a.1) != holders(b.1))
                .map_or(0.0, |(_, b)| b.0.saturating_sub(start).as_secs_f64())
        })
    }

    /// Each host's children on `swarm` at `t`: the hosts whose open parent
    /// of a stream it is at their transfer samples nearest `t`, within half
    /// a stats period. `None` without transfer samples.
    fn children_at(&self, swarm: usize, t: Duration) -> Option<BTreeMap<HostIx, BTreeSet<HostIx>>> {
        if !self.scenario.transfer_samples {
            return None;
        }
        let half = self.scenario.stats_every.unwrap_or(Duration::from_secs(1)) / 2;
        let mut out: BTreeMap<HostIx, BTreeSet<HostIx>> = BTreeMap::new();
        for (h, r) in self.hosts.iter().enumerate() {
            let first = r.transfers.partition_point(|(at, _, _)| *at + half < t);
            let nearest = r
                .transfers
                .get(first..)
                .unwrap_or_default()
                .iter()
                .take_while(|(at, _, _)| *at <= t + half)
                .filter(|(_, s, _)| *s == swarm)
                .min_by_key(|(at, _, _)| at.abs_diff(t));
            for p in nearest
                .into_iter()
                .flat_map(|(_, _, x)| x.parent_peers.iter().flatten())
            {
                out.entry(*p).or_default().insert(h);
            }
        }
        Some(out)
    }

    /// The reports' verdicts on `swarm`'s seed's holders at its samples in
    /// the window ([`Verdicts`]), and whether the real children were known
    /// (transfer samples); `None` without a sample.
    pub fn verdicts(&self, swarm: usize) -> Option<(Verdicts, bool)> {
        let (from, to) = self.seed_meter_window(swarm)?;
        let samples = self.seed_samples(swarm);
        if samples.is_empty() {
            return None;
        }
        let kids = self.scenario.transfer_samples;
        let mut v = Verdicts::default();
        let mut before: Option<(Duration, &AllocSample)> = None;
        let mut children_before = None;
        for &(t, a) in &samples {
            let children = self.children_at(swarm, t).unwrap_or_default();
            if t < from || t > to {
                before = Some((t, a));
                children_before = Some(children);
                continue;
            }
            let neighbours: BTreeSet<HostIx> = a.hosts.iter().flatten().copied().collect();
            let barred_before: BTreeSet<HostIx> = before.map_or(BTreeSet::new(), |(_, b)| {
                b.info
                    .neighbours
                    .iter()
                    .zip(&b.hosts)
                    .filter(|(n, _)| n.barred)
                    .filter_map(|(_, h)| *h)
                    .collect()
            });
            for (n, &h) in a.info.neighbours.iter().zip(&a.hosts) {
                let Some(h) = h else {
                    continue;
                };
                let class = self.scenario.hosts.get(h).map(|x| x.class);
                if n.barred
                    && !barred_before.contains(&h)
                    && class == Some(HostClass::Honest)
                    && before.is_some_and(|(_, b)| holders(b).contains(&h))
                    && children_before
                        .as_ref()
                        .is_some_and(|c: &BTreeMap<HostIx, BTreeSet<HostIx>>| c.contains_key(&h))
                {
                    v.barred_with_children += 1;
                }
                if !n.holder {
                    continue;
                }
                let (judged, served) = (n.served.is_some(), n.served == Some(true));
                match class {
                    Some(HostClass::Hostile(k)) if LIARS.contains(&k) => {
                        v.liars += 1;
                        v.liars_judged += u64::from(judged);
                        v.liars_served += u64::from(served);
                    }
                    Some(HostClass::Honest) => {
                        let kids = children.get(&h);
                        let childless = kids.is_none_or(BTreeSet::is_empty);
                        v.honest += 1;
                        v.with_children += u64::from(!childless);
                        v.with_reporters += u64::from(
                            kids.is_some_and(|k| k.iter().any(|c| neighbours.contains(c))),
                        );
                        v.judged += u64::from(judged);
                        v.served += u64::from(served);
                        v.served_childless += u64::from(served && childless);
                        v.not_kept += u64::from(!n.kept_up && !n.streams.is_empty());
                    }
                    _ => {}
                }
            }
            before = Some((t, a));
            children_before = Some(children);
        }
        Some((v, kids))
    }

    /// What became of `swarm`'s honest viewers' report entries at its seed:
    /// each honest neighbour's own counters, the last of each run of
    /// samples over which none fell (a channel's life), summed over the
    /// runs and the neighbours. `None` without a sample.
    pub fn report_counts(&self, swarm: usize) -> Option<ReportCounters> {
        let samples = self.seed_samples(swarm);
        if samples.is_empty() {
            return None;
        }
        let arr = |c: &ReportCounters| {
            [
                c.tallied,
                c.unknown,
                c.invalid,
                c.reporter_fed,
                c.subject_unfed,
                c.reporter_silent,
                c.too_early,
                c.misses_unseen,
            ]
        };
        // Per honest neighbour: the folded runs, and the last reading.
        let mut by: BTreeMap<HostIx, ([u64; 8], [u64; 8])> = BTreeMap::new();
        for (_, a) in &samples {
            for (n, &h) in a.info.neighbours.iter().zip(&a.hosts) {
                let Some(h) = h.filter(|&h| {
                    self.scenario.hosts.get(h).map(|x| x.class) == Some(HostClass::Honest)
                }) else {
                    continue;
                };
                let now = arr(&n.reports);
                let (folded, last) = by.entry(h).or_default();
                if now.iter().zip(last.iter()).any(|(n, l)| n < l) {
                    for (f, l) in folded.iter_mut().zip(last.iter()) {
                        *f += l;
                    }
                }
                *last = now;
            }
        }
        let mut t = [0u64; 8];
        for (folded, last) in by.values() {
            for ((x, f), l) in t.iter_mut().zip(folded).zip(last) {
                *x += f + l;
            }
        }
        let [
            tallied,
            unknown,
            invalid,
            reporter_fed,
            subject_unfed,
            reporter_silent,
            too_early,
            misses_unseen,
        ] = t;
        Some(ReportCounters {
            tallied,
            unknown,
            invalid,
            reporter_fed,
            subject_unfed,
            reporter_silent,
            too_early,
            misses_unseen,
        })
    }

    /// The ANNOUNCEs `swarm`'s seed read on the swarm's channels, their
    /// bytes, and those it sent, every channel and incarnation: the row's
    /// `announces`, `announce_bytes` and `asks`.
    pub fn announces(&self, swarm: usize) -> Announces {
        let Some(seed) = self.scenario.swarms.get(swarm).map(|s| s.seed) else {
            return Announces::default();
        };
        self.hosts
            .get(seed)
            .into_iter()
            .flat_map(|r| r.messages.iter())
            .filter(|(_, s, _)| *s == swarm)
            .fold(Announces::default(), |a, (_, _, m)| Announces {
                read: a.read + m.announces_in,
                bytes: a.bytes + m.announce_bytes_in,
                sent: a.sent + m.announces_out,
            })
    }

    /// With the request logs ([`Scenario::request_logs`] and the tables),
    /// the chunks the first swarm's honest viewers asked its seed for
    /// (their request logs' records naming it), by how many of the asker's
    /// other neighbours (at its stats sample before the request) had held
    /// them for [`HELD_FOR`] (their arrival logs' first holds): none, one
    /// or two, three or more; and of those none held, the ones whose stream
    /// had no open parent at the asker then (its parent log). `None`
    /// without the logs.
    ///
    /// [`Scenario::request_logs`]: crate::Scenario::request_logs
    pub fn seed_asks(&self) -> Option<[u64; 4]> {
        if !self.scenario.request_logs || self.scenario.wire_bins.is_none() {
            return None;
        }
        let sw = self.scenario.swarms.first()?;
        let held: Vec<BTreeMap<u64, Duration>> = self
            .hosts
            .iter()
            .map(|r| {
                let mut m = BTreeMap::new();
                for a in &r.arrivals {
                    let at = m.entry(a.held.bin.base_offset()).or_insert(a.at);
                    *at = (*at).min(a.at);
                }
                m
            })
            .collect();
        let mut parents: BTreeMap<(HostIx, u16), Vec<ParentAt>> = BTreeMap::new();
        for p in &self.parent_changes {
            parents
                .entry((p.node, p.stream))
                .or_default()
                .push((p.at, p.parent));
        }
        let parentless = |v: HostIx, stream: u16, t: Duration| {
            parents.get(&(v, stream)).is_none_or(|c| {
                c.iter()
                    .take_while(|&&(at, _)| at <= t)
                    .last()
                    .is_none_or(|&(_, p)| p.is_none())
            })
        };
        let mut out = [0u64; 4];
        for &v in &sw.viewers {
            if self.scenario.hosts.get(v).map(|x| x.class) != Some(HostClass::Honest) {
                continue;
            }
            let Some(r) = self.hosts.get(v) else {
                continue;
            };
            for q in r.requests.iter().filter(|q| q.peer == Some(sw.seed)) {
                let t = (q.record.at - q.epoch).to_std().unwrap_or_default();
                let around = r
                    .neighbours
                    .iter()
                    .take_while(|(at, _)| *at <= t)
                    .last()
                    .map_or(&[][..], |(_, n)| n.as_slice());
                let (mut chunk, last) = q.record.bin.base();
                while chunk <= last {
                    let c = chunk.base_offset();
                    let n = around
                        .iter()
                        .filter(|&&x| x != sw.seed)
                        .filter(|&&x| {
                            held.get(x)
                                .and_then(|m| m.get(&c))
                                .is_some_and(|&at| at + HELD_FOR <= t)
                        })
                        .count();
                    if let Some(k) = out.get_mut(n.min(3).div_ceil(2)) {
                        *k += 1;
                    }
                    if n == 0
                        && parentless(v, q.record.stream, t)
                        && let Some(k) = out.get_mut(3)
                    {
                        *k += 1;
                    }
                    chunk += 2;
                }
            }
        }
        Some(out)
    }

    /// The row's seed allocation fields (see the module docs), each left
    /// out where it has no value.
    pub(crate) fn seed_alloc_cols(&self, swarm: usize, j: &mut JsonRow) {
        if let Some(x) = self.seed_slots(swarm) {
            j.num(RowNum::SeedSlots, x);
        }
        if let Some(x) = self.seed_objective(swarm) {
            j.int(RowInt::SeedObjectiveMs, x);
        }
        if let Some(x) = self.holder_changes(swarm) {
            j.int(RowInt::HolderChanges, x);
        }
        if let Some(x) = self.settled(swarm) {
            j.num(RowNum::SettledS, x);
        }
        if let Some(x) = self.holder_copy(swarm) {
            j.num(RowNum::HolderCopy, x);
        }
        if let Some((v, kids)) = self.verdicts(swarm) {
            j.int(RowInt::VHonest, v.honest)
                .int(RowInt::VJudged, v.judged)
                .int(RowInt::VServed, v.served)
                .int(RowInt::VLiars, v.liars)
                .int(RowInt::VLiarsJudged, v.liars_judged)
                .int(RowInt::VLiarsServed, v.liars_served)
                .int(RowInt::VNotKept, v.not_kept);
            if kids {
                j.int(RowInt::VWithChildren, v.with_children)
                    .int(RowInt::VWithReporters, v.with_reporters)
                    .int(RowInt::VServedChildless, v.served_childless)
                    .int(RowInt::VBarredWithChildren, v.barred_with_children);
            }
        }
        if let Some(c) = self.report_counts(swarm) {
            j.int(RowInt::ReportsTallied, c.tallied)
                .int(RowInt::ReportsUnknown, c.unknown)
                .int(RowInt::ReportsInvalid, c.invalid)
                .int(RowInt::ReportsReporterFed, c.reporter_fed)
                .int(RowInt::ReportsSubjectUnfed, c.subject_unfed)
                .int(RowInt::ReportsReporterSilent, c.reporter_silent)
                .int(RowInt::ReportsTooEarly, c.too_early)
                .int(RowInt::ReportsMissesUnseen, c.misses_unseen);
        }
        if let Some([none, few, many, parentless]) = self.seed_asks().filter(|_| swarm == 0) {
            for (held, n) in RowInt::HELD_OF_SEED_ASKS_HELD
                .into_iter()
                .zip([none, few, many])
            {
                j.int(RowInt::seed_asks_held(held), n);
            }
            j.int(RowInt::SeedAsksHeldByNoneParentless, parentless);
        }
        let Some(spec) = self.scenario.swarms.get(swarm) else {
            return;
        };
        let a = self.announces(swarm);
        let secs = spec.stop.saturating_sub(spec.start).as_secs_f64().max(1e-9);
        let sent: u64 = spec
            .viewers
            .iter()
            .filter_map(|&h| self.hosts.get(h))
            .map(|r| r.uploaded)
            .sum();
        j.int(RowInt::Announces, a.read)
            .int(RowInt::AnnounceBytes, a.bytes)
            .num(RowNum::AnnounceBps, a.bytes as f64 / secs)
            .num(RowNum::AnnounceShare, a.bytes as f64 / sent.max(1) as f64)
            .int(RowInt::Asks, a.sent);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_node::HostId;
    use strims_node::event::SeedNeighbourInfo;
    use strims_testbed::transfer::TransferSample;

    use super::*;
    use crate::report::HostReport;
    use crate::spec::{Scenario, SwarmSpec};
    use crate::stats::PeerMessages;

    fn ms(n: i64) -> strims_base::time::Duration {
        strims_base::time::Duration::from_millis(n)
    }

    /// Host `h` as a neighbour: whether it holds a copy, the streams pushed
    /// to it, its lags, whether a quorum saw it serve, its tallied entries.
    fn neighbour(
        h: u8,
        holder: bool,
        streams: u16,
        lag: Option<i64>,
        served: Option<bool>,
        tallied: u64,
    ) -> SeedNeighbourInfo {
        SeedNeighbourInfo {
            host_id: HostId::from_bytes(&[h; 32]),
            streams: (0..streams).collect(),
            lags: vec![lag.map(ms); 2],
            rtt: ms(20),
            min_rtt: None,
            holder,
            barred: false,
            reporters: 3,
            served,
            report_delay: None,
            kept_up: h != 2,
            reports: ReportCounters {
                tallied,
                ..ReportCounters::default()
            },
        }
    }

    /// A seed (0), honest viewers 1-3 and a liar (4) on a stream from 0 to
    /// 40 s, the seed sampled every second: viewers 1 and 2 hold a copy
    /// (8 streams each) to 30 s, then 1 and 3, the liar (4 streams)
    /// throughout; viewer 2's open parent is viewer 1, at transfer samples
    /// 30 ms after the seed's. Viewer 1's clock is 50 ms ahead of the
    /// seed's and its reports read 150 ms; viewer 2 reports nothing; 3 and
    /// the liar 80 ms. Viewer 1's tallied entries count 5 a second, its
    /// channel restarting at 33 s; viewer 2 is behind.
    fn run() -> Report {
        let mut s = Scenario::new("alloc");
        for i in 0..5 {
            s.host(&format!("h{i}"), None);
        }
        s.hosts[4].class = HostClass::Hostile("liar");
        s.swarm(SwarmSpec::live(
            0,
            vec![1, 2, 3, 4],
            16 * 1024,
            Duration::ZERO,
            Duration::from_secs(40),
        ));
        s.transfer_samples = true;
        let mut hosts = vec![HostReport::default(); 5];
        hosts[1].clock_offset = ms(50);
        for t in 0..=40u64 {
            let late = t > 30;
            let tallied = if t >= 33 { 5 * (t - 33) } else { 5 * t };
            let info = SeedAllocationInfo {
                neighbours: vec![
                    neighbour(1, true, 8, Some(150), Some(true), tallied),
                    neighbour(2, !late, if late { 0 } else { 8 }, None, Some(false), 0),
                    neighbour(3, late, if late { 8 } else { 0 }, Some(80), None, 0),
                    neighbour(4, true, 4, Some(80), Some(true), 7),
                ],
                ..SeedAllocationInfo::default()
            };
            let at = Duration::from_secs(t);
            hosts[0].seed_allocs.push((
                at,
                0,
                AllocSample {
                    info,
                    hosts: vec![Some(1), Some(2), Some(3), Some(4)],
                },
            ));
            for (h, parent) in [(1, Some(0)), (2, Some(1)), (3, Some(0))] {
                let x = TransferSample {
                    parent_peers: vec![parent; 2],
                    ..TransferSample::default()
                };
                hosts[h]
                    .transfers
                    .push((at + Duration::from_millis(30), 0, x));
            }
        }
        let announced = PeerMessages {
            announces_in: 3,
            announce_bytes_in: 300,
            announces_out: 4,
            ..PeerMessages::default()
        };
        let other_swarm = PeerMessages {
            announces_in: 9,
            ..PeerMessages::default()
        };
        hosts[0].messages = vec![(Some(1), 0, announced), (Some(2), 1, other_swarm)];
        Report {
            scenario: s,
            seed: 1,
            trace_hash: String::new(),
            trace: Vec::new(),
            sim_secs: 0.0,
            wall_secs: 0.0,
            viewers: Vec::new(),
            hosts,
            written: Vec::new(),
            media: Vec::new(),
            vpn: None,
            memory: None,
            violations: Vec::new(),
            wan: None,
            wire_bins: Vec::new(),
            wire_unmeasured: Vec::new(),
            sim: "strims-sim",
            wire_ends: std::collections::BTreeMap::new(),
            sockets: Vec::new(),
            parent_losses: Vec::new(),
            parent_changes: Vec::new(),
            parent_checks: Vec::new(),
            loops: None,
            skipped_faults: Vec::new(),
        }
    }

    /// The window runs from 25 s (`STARTUP_SECS`) to 40 s: 16 samples of
    /// 20 slots each; the holders change once in it, at 31 s; the
    /// objective's p90 over the neighbours is the 2 s a silent one counts,
    /// and with viewer 2 at 90 ms, viewer 1's 100 ms (150 less the 50 its
    /// clock runs ahead).
    #[test]
    fn slots_holders_and_the_objective_read_the_window() {
        let mut r = run();
        assert_eq!(r.seed_slots(0), Some(20.0));
        assert_eq!(r.holder_changes(0), Some(1));
        assert_eq!(r.settled(0), Some(31.0));
        assert_eq!(r.seed_objective(0), Some(2000));
        for (_, _, a) in &mut r.hosts[0].seed_allocs {
            a.info.neighbours[1].lags = vec![Some(ms(90))];
        }
        assert_eq!(r.seed_objective(0), Some(100));
    }

    /// Viewer 1 holds with a real child (viewer 2, a neighbour that can
    /// report) and is served; viewer 2 holds childless to 30 s, unserved
    /// and behind; viewer 3 holds childless and unjudged from 31 s; the
    /// liar holds, served, throughout.
    #[test]
    fn verdicts_hold_the_tally_against_the_real_children() {
        let (v, kids) = run().verdicts(0).unwrap();
        assert!(kids);
        let expect = Verdicts {
            honest: 16 + 6 + 10,
            with_children: 16,
            with_reporters: 16,
            judged: 16 + 6,
            served: 16,
            served_childless: 0,
            liars: 16,
            liars_judged: 16,
            liars_served: 16,
            not_kept: 6,
            barred_with_children: 0,
        };
        assert_eq!(v, expect);
        let mut r = run();
        r.scenario.transfer_samples = false;
        assert!(!r.verdicts(0).unwrap().1, "no children without transfers");
    }

    /// A barred honest holder that had a real child at the sample before
    /// counts once, at its bar.
    #[test]
    fn a_bar_counts_the_children_the_sample_before() {
        let mut r = run();
        for (t, _, a) in &mut r.hosts[0].seed_allocs {
            a.info.neighbours[0].barred = *t >= Duration::from_secs(28);
        }
        assert_eq!(r.verdicts(0).unwrap().0.barred_with_children, 1);
    }

    /// Viewer 1 asks the seed at 10 s for chunk 3, which neighbour 2 held
    /// from 9.8 s and 4 from 9 s (3 got it 50 ms before, too late to count),
    /// and for chunk 5, which no one held while stream 1 had no parent at
    /// viewer 1; at 20 s for chunk 5 again, once 2 grafted it onto itself
    /// and held it. The liar's request counts nothing, nor does a request
    /// of another neighbour.
    #[test]
    fn asks_of_the_seed_count_the_neighbours_that_held_them() {
        use strims_binmap::Bin;
        use strims_ppspp::{Held, RequestChannel, RequestEnd, RequestRecord};

        use crate::report::{ArrivalHostRecord, ParentRecord, RequestHostRecord};

        let mut r = run();
        r.scenario.request_logs = true;
        r.scenario.wire_bins = Some(Duration::from_millis(100));
        let t = |ms: u64| Duration::from_millis(ms);
        let epoch = strims_base::time::Time::from_unix_millis(1_000);
        let ask = |at: u64, chunk: u64, stream: u16, peer| RequestHostRecord {
            epoch,
            peer: Some(peer),
            rivals: Vec::new(),
            record: RequestRecord {
                at: epoch + strims_base::time::Duration::from_millis(at as i64),
                stream,
                bin: Bin(2 * chunk),
                timeout: ms(500),
                released: None,
                asked: RequestChannel {
                    peer: strims_ppspp::PeerKey::default(),
                    announced: None,
                    predicted: None,
                    rtt: ms(20),
                    channel_open: epoch,
                },
                rivals: Vec::new(),
                end: RequestEnd::Answered,
                ended: epoch,
            },
        };
        let held = |chunk: u64, at: u64| ArrivalHostRecord {
            at: t(at),
            expected: None,
            released: None,
            asked: None,
            last_ask: None,
            peer: None,
            parent: None,
            asked_peer: None,
            held: Held {
                bin: Bin(2 * chunk),
                ..Held::default()
            },
        };
        r.hosts[1].neighbours = vec![(t(5_000), vec![0, 2, 3, 4])];
        r.hosts[1].requests = vec![
            ask(10_000, 3, 0, 0),
            ask(10_000, 5, 1, 0),
            ask(20_000, 5, 1, 0),
            ask(10_000, 7, 0, 2),
        ];
        r.hosts[4].requests = vec![ask(10_000, 3, 0, 0)];
        r.hosts[2].arrivals = vec![held(3, 9_800), held(5, 15_000)];
        r.hosts[3].arrivals = vec![held(3, 9_950)];
        r.hosts[4].arrivals = vec![held(3, 9_000)];
        r.parent_changes = vec![ParentRecord {
            at: t(14_000),
            node: 1,
            stream: 1,
            parent: Some(2),
            why: strims_ppspp::ParentWhy::Open,
        }];
        assert_eq!(r.seed_asks(), Some([1, 2, 0, 1]));
        r.scenario.request_logs = false;
        assert_eq!(r.seed_asks(), None);
    }

    /// Each channel's last count, summed over its restarts, of the honest
    /// neighbours only; the announces of the swarm's channels alone.
    #[test]
    fn report_entries_and_announces_sum_every_channel() {
        let r = run();
        assert_eq!(r.report_counts(0).unwrap().tallied, 5 * 32 + 5 * 7);
        assert_eq!(
            r.announces(0),
            Announces {
                read: 3,
                bytes: 300,
                sent: 4
            }
        );
    }
}
