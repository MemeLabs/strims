// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! With [`Scenario::loops`](crate::Scenario::loops) (`STRIMS_SIM_LOOPS=1`),
//! how push loops (cycles of open parents) form among the first swarm's
//! viewers and how long they last, and how often a viewer asks one of its
//! own descendants for a stream.
//!
//! Every node's parent log (`NodeConfig::parent_log`) keeps a live map of
//! each host's open parent per stream, and its decision log
//! (`NodeConfig::decision_log`) each host's latest parent decisions per
//! stream ([`LOOP_LOG`] of them): the grafts and switches it asked, the
//! prunes, sheds, declines and open timeouts. The run hands both over in
//! the order the nodes made them, so at each decision the map stands as
//! it did then.
//!
//! - **grafts and switches**: those the honest viewers asked, and those
//!   that asked one of the asking viewer's own descendants (the asked
//!   host's chain of open parents for the stream passes through the
//!   asker).
//! - **loops**: at each stats round from [`LOOP_WARM`] after the swarm's
//!   start, the cycles of open parents the honest viewers' chains run
//!   into, by stream and members; a cycle not there at the round before
//!   formed, classed by how its newest edge formed ([`Loops::edge`]), and
//!   one gone broke after its lifetime.
//!
//! Nothing here reaches back into the nodes: it only reads what their logs
//! handed over.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use strims_base::stats::sketch;
use strims_ppspp::{ParentAction, TapKind};

use crate::fields::{JsonRow, RowInt};
use crate::report::Report;
use crate::spec::HostIx;

/// The decisions kept per (host, stream).
pub const LOOP_LOG: usize = 16;

/// Loops count from this long after the first swarm's start, once its
/// first trees had their time to form.
pub const LOOP_WARM: Duration = Duration::from_secs(10);

/// How a loop's newest edge formed: the names of the row's
/// `loops_by_{how}`.
pub const HOWS: [&str; 7] = [
    "switch",
    "graft_after_silent_prune",
    "graft_after_lossy_prune",
    "graft_after_shed",
    "graft_after_decline",
    "graft_after_nothing_logged",
    "unknown",
];

/// A parent decision of a host's, naming the other host by index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// With no parent, asked this host.
    Graft(HostIx),
    /// Asked this host to replace the parent.
    Switch(HostIx),
    /// Pruned the parent: silent, or else lossy.
    Pruned {
        silent: bool,
    },
    ShedBy,
    Declined,
    OpenTimeout,
}

impl Decision {
    /// The decision a tap event of a host's is, with its stream, and for a
    /// graft or switch the host it asked (`asked`, the event's peer).
    pub fn of(kind: &TapKind, asked: Option<HostIx>) -> Option<(u16, Option<Decision>)> {
        Some(match *kind {
            TapKind::ParentChoice { stream, choice } => match choice.action {
                ParentAction::Graft => (stream, asked.map(Decision::Graft)),
                ParentAction::Switch => (stream, asked.map(Decision::Switch)),
                _ => return None,
            },
            TapKind::Pruned { stream, why } => {
                (stream, Some(Decision::Pruned { silent: why.silent }))
            }
            TapKind::ShedBy { stream } => (stream, Some(Decision::ShedBy)),
            TapKind::Declined { stream } => (stream, Some(Decision::Declined)),
            TapKind::OpenTimeout { stream, .. } => (stream, Some(Decision::OpenTimeout)),
            _ => return None,
        })
    }

    /// Whether a decision log hands a tap event of `kind` on: the events
    /// [`Decision::of`] reads, and the tap's losses.
    pub fn kept(kind: &TapKind) -> bool {
        matches!(
            kind,
            TapKind::ParentChoice { .. }
                | TapKind::Pruned { .. }
                | TapKind::ShedBy { .. }
                | TapKind::Declined { .. }
                | TapKind::OpenTimeout { .. }
                | TapKind::Lost { .. }
        )
    }
}

/// The run's loop analysis (see the module docs).
#[derive(Clone, Debug, Default)]
pub struct Loops {
    /// Per (host, stream), its open parent: the incarnation that named it
    /// and the parent (`None` for none).
    parents: BTreeMap<(HostIx, u16), (u32, Option<HostIx>)>,
    /// Per (host, stream), its latest decisions, at run time.
    log: BTreeMap<(HostIx, u16), VecDeque<(Duration, Decision)>>,
    /// The loops at the latest round, by stream and members (sorted), and
    /// when each was first seen.
    active: BTreeMap<(u16, Vec<HostIx>), Duration>,
    /// The loops that formed, by how their newest edge formed.
    closed_by: BTreeMap<&'static str, u64>,
    /// Each formed loop's members, and the lifetimes of those that broke.
    lens: Vec<u64>,
    lives: Vec<Duration>,
    /// Tap events the decision logs left out.
    lost: u64,
    /// Grafts and switches the honest viewers asked, and those that asked
    /// one of the asker's own descendants.
    grafts: u64,
    grafts_down: u64,
    switches: u64,
    switches_down: u64,
}

/// What [`Loops`] found over a run: the row's loop fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoopReport {
    pub loops: u64,
    pub alive: u64,
    /// Per [`HOWS`] name, the loops whose newest edge formed so.
    pub by: Vec<(&'static str, u64)>,
    pub len_p50: Option<u64>,
    /// The p50 and p90 of the broken loops' lifetimes, in ms.
    pub life_ms: Option<(u64, u64)>,
    pub lost: u64,
    pub grafts: u64,
    pub grafts_down: u64,
    pub switches: u64,
    pub switches_down: u64,
}

impl Loops {
    /// Host `h`'s (incarnation `inc`) open parent of `stream` is `parent`
    /// from now on.
    pub fn parent(&mut self, h: HostIx, inc: u32, stream: u16, parent: Option<HostIx>) {
        self.parents.insert((h, stream), (inc, parent));
    }

    /// Host `h` made decision `d` about `stream` at `t`; `honest`: it is
    /// one of the first swarm's honest viewers, whose grafts and switches
    /// count. `live` says whether a host's map entry of an incarnation is
    /// its current one.
    pub fn decision(
        &mut self,
        h: HostIx,
        stream: u16,
        d: Decision,
        t: Duration,
        honest: bool,
        live: &dyn Fn(HostIx, u32) -> bool,
    ) {
        if honest && let Decision::Graft(p) | Decision::Switch(p) = d {
            let down = self.descends(p, h, stream, live);
            let (n, n_down) = match d {
                Decision::Graft(_) => (&mut self.grafts, &mut self.grafts_down),
                _ => (&mut self.switches, &mut self.switches_down),
            };
            *n += 1;
            *n_down += u64::from(down);
        }
        let log = self.log.entry((h, stream)).or_default();
        if log.len() == LOOP_LOG {
            log.pop_front();
        }
        log.push_back((t, d));
    }

    /// A graft or switch whose asked host the run can't name: it counts,
    /// though not as one asking a descendant.
    pub fn unnamed(&mut self, honest: bool, graft: bool) {
        if honest {
            *match graft {
                true => &mut self.grafts,
                false => &mut self.switches,
            } += 1;
        }
    }

    /// Tap events a decision log left out.
    pub fn lost(&mut self, n: u64) {
        self.lost += n;
    }

    /// Host `h`'s open parent of `stream`, where its entry is of its
    /// current incarnation.
    fn parent_of(
        &self,
        h: HostIx,
        stream: u16,
        live: &dyn Fn(HostIx, u32) -> bool,
    ) -> Option<HostIx> {
        let &(inc, p) = self.parents.get(&(h, stream))?;
        p.filter(|_| live(h, inc))
    }

    /// Whether `x`'s chain of open parents for `stream` passes through
    /// `ancestor`.
    fn descends(
        &self,
        x: HostIx,
        ancestor: HostIx,
        stream: u16,
        live: &dyn Fn(HostIx, u32) -> bool,
    ) -> bool {
        let mut at = x;
        for _ in 0..=self.parents.len() {
            if at == ancestor {
                return true;
            }
            match self.parent_of(at, stream, live) {
                Some(p) => at = p,
                None => return false,
            }
        }
        false
    }

    /// How the edge from `child` to `parent` on `stream` formed, and when:
    /// the latest graft or switch in `child`'s log that asked `parent`,
    /// and for a graft the latest loss before it.
    pub fn edge(
        &self,
        child: HostIx,
        parent: HostIx,
        stream: u16,
    ) -> Option<(Duration, &'static str)> {
        let log = self.log.get(&(child, stream))?;
        let at = log.iter().rposition(
            |(_, d)| matches!(d, Decision::Graft(p) | Decision::Switch(p) if *p == parent),
        )?;
        let &(t, d) = log.get(at)?;
        let how = match d {
            Decision::Switch(_) => "switch",
            _ => log
                .iter()
                .take(at)
                .rev()
                .find_map(|(_, d)| match d {
                    Decision::Pruned { silent: true } => Some("graft_after_silent_prune"),
                    Decision::Pruned { silent: false } => Some("graft_after_lossy_prune"),
                    Decision::ShedBy => Some("graft_after_shed"),
                    Decision::Declined | Decision::OpenTimeout => Some("graft_after_decline"),
                    _ => None,
                })
                .unwrap_or("graft_after_nothing_logged"),
        };
        Some((t, how))
    }

    /// One round at `now`: the loops the chains of `viewers` run into on
    /// each of `streams`, those new since the last round classed, those
    /// gone with their lifetimes.
    pub fn round(
        &mut self,
        now: Duration,
        viewers: &[HostIx],
        streams: u16,
        live: &dyn Fn(HostIx, u32) -> bool,
    ) {
        let mut found: BTreeMap<(u16, Vec<HostIx>), Vec<HostIx>> = BTreeMap::new();
        for &i in viewers {
            for s in 0..streams {
                let mut seen = vec![i];
                let mut at = self.parent_of(i, s, live);
                while let Some(p) = at {
                    if let Some(k) = seen.iter().position(|&x| x == p) {
                        let ring = seen.split_off(k);
                        let mut members = ring.clone();
                        members.sort_unstable();
                        found.entry((s, members)).or_insert(ring);
                        break;
                    }
                    seen.push(p);
                    at = self.parent_of(p, s, live);
                }
            }
        }
        let gone: Vec<_> = self
            .active
            .keys()
            .filter(|k| !found.contains_key(*k))
            .cloned()
            .collect();
        for k in gone {
            if let Some(first) = self.active.remove(&k) {
                self.lives.push(now.saturating_sub(first));
            }
        }
        for (key, ring) in found {
            if self.active.contains_key(&key) {
                continue;
            }
            let stream = key.0;
            let n = ring.len();
            let newest = (0..n)
                .filter_map(|j| self.edge(*ring.get(j)?, *ring.get((j + 1) % n)?, stream))
                .max_by_key(|&(t, _)| t);
            let how = newest.map_or("unknown", |(_, how)| how);
            *self.closed_by.entry(how).or_default() += 1;
            self.lens.push(n as u64);
            self.active.insert(key, now);
        }
    }

    pub fn report(&self) -> LoopReport {
        let mut lens = self.lens.clone();
        lens.sort_unstable();
        let mut lives: Vec<u64> = self
            .lives
            .iter()
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .collect();
        lives.sort_unstable();
        LoopReport {
            loops: lens.len() as u64,
            alive: self.active.len() as u64,
            by: HOWS
                .iter()
                .map(|&h| (h, self.closed_by.get(h).copied().unwrap_or(0)))
                .collect(),
            len_p50: sketch::percentile(&lens, 50),
            life_ms: sketch::percentile(&lives, 50).zip(sketch::percentile(&lives, 90)),
            lost: self.lost,
            grafts: self.grafts,
            grafts_down: self.grafts_down,
            switches: self.switches,
            switches_down: self.switches_down,
        }
    }
}

impl Report {
    /// With [`Report::loops`], the row's loop fields (`loops`,
    /// `loops_alive`, `loops_by_{how}`, `loop_len_p50`,
    /// `loop_life_p{p}_ms`, `loops_tap_lost`, `grafts`, `grafts_down`,
    /// `switches`, `switches_down`); the percentiles left out without
    /// values.
    pub(crate) fn loop_cols(&self, j: &mut JsonRow) {
        let Some(l) = &self.loops else {
            return;
        };
        j.int(RowInt::Loops, l.loops)
            .int(RowInt::LoopsAlive, l.alive)
            .int(RowInt::LoopsTapLost, l.lost)
            .int(RowInt::Grafts, l.grafts)
            .int(RowInt::GraftsDown, l.grafts_down)
            .int(RowInt::Switches, l.switches)
            .int(RowInt::SwitchesDown, l.switches_down);
        for &(how, n) in &l.by {
            j.int(RowInt::loops_by_how(how), n);
        }
        if let Some(m) = l.len_p50 {
            j.int(RowInt::LoopLenP50, m);
        }
        if let Some((p50, p90)) = l.life_ms {
            for (p, x) in RowInt::P_OF_LOOP_LIFE_P_P_MS.into_iter().zip([p50, p90]) {
                j.int(RowInt::loop_life_p_p_ms(p), x);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn always(_: HostIx, _: u32) -> bool {
        true
    }

    fn s(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    /// Viewers 1-3 under seed 0: 2 grafts onto 1 after a silent prune, 3
    /// onto 2, and 1 switches to 3, closing a loop of three whose newest
    /// edge is the switch; 1's switch asked one of its descendants, 2's
    /// and 3's grafts didn't. The loop breaks when 1 grafts back onto the
    /// seed.
    #[test]
    fn a_loop_is_classed_by_its_newest_edge_and_timed() {
        let mut l = Loops::default();
        let live: &dyn Fn(HostIx, u32) -> bool = &always;
        for v in 1..=3 {
            l.parent(v, 1, 0, Some(0));
        }
        l.decision(2, 0, Decision::Pruned { silent: true }, s(1), true, live);
        l.decision(2, 0, Decision::Graft(1), s(2), true, live);
        l.parent(2, 1, 0, Some(1));
        l.decision(3, 0, Decision::Graft(2), s(3), true, live);
        l.parent(3, 1, 0, Some(2));
        l.decision(1, 0, Decision::Switch(3), s(4), true, live);
        l.parent(1, 1, 0, Some(3));
        l.round(s(5), &[1, 2, 3], 2, live);
        l.round(s(6), &[1, 2, 3], 2, live);
        let r = l.report();
        assert_eq!((r.loops, r.alive, r.len_p50), (1, 1, Some(3)));
        assert_eq!(
            (r.grafts, r.grafts_down, r.switches, r.switches_down),
            (2, 0, 1, 1)
        );
        assert!(r.by.contains(&("switch", 1)));
        assert_eq!(r.by.iter().map(|&(_, n)| n).sum::<u64>(), 1);
        assert_eq!(l.edge(2, 1, 0), Some((s(2), "graft_after_silent_prune")));
        l.decision(
            1,
            0,
            Decision::Graft(0),
            Duration::from_millis(6500),
            true,
            live,
        );
        l.parent(1, 1, 0, Some(0));
        l.round(s(7), &[1, 2, 3], 2, live);
        let r = l.report();
        assert_eq!((r.loops, r.alive, r.life_ms), (1, 0, Some((2000, 2000))));
    }

    /// A parent named by an incarnation no longer current ends a chain,
    /// and a loop through it isn't one; only honest viewers' grafts count.
    #[test]
    fn a_gone_incarnation_ends_its_chains() {
        let mut l = Loops::default();
        l.parent(1, 1, 0, Some(2));
        l.parent(2, 1, 0, Some(1));
        let gone = |h: HostIx, inc: u32| !(h == 2 && inc == 1);
        l.round(s(5), &[1, 2], 1, &gone);
        assert_eq!(l.report().loops, 0);
        l.round(s(6), &[1, 2], 1, &always);
        assert_eq!(l.report().loops, 1);
        assert_eq!(
            l.report().by.iter().find(|x| x.0 == "unknown"),
            Some(&("unknown", 1))
        );
        l.decision(3, 0, Decision::Graft(1), s(7), false, &always);
        l.unnamed(false, true);
        assert_eq!(l.report().grafts, 0);
        l.unnamed(true, false);
        assert_eq!(l.report().switches, 1);
    }

    /// A decline or open timeout before a graft classes it as after a
    /// decline; a graft with nothing before it, after nothing logged; the
    /// log keeps its latest LOOP_LOG decisions.
    #[test]
    fn a_grafts_cause_is_the_latest_loss_before_it() {
        let mut l = Loops::default();
        l.decision(1, 0, Decision::Graft(2), s(1), true, &always);
        assert_eq!(l.edge(1, 2, 0), Some((s(1), "graft_after_nothing_logged")));
        l.decision(1, 0, Decision::OpenTimeout, s(2), true, &always);
        l.decision(1, 0, Decision::Graft(3), s(3), true, &always);
        assert_eq!(l.edge(1, 3, 0), Some((s(3), "graft_after_decline")));
        l.decision(1, 0, Decision::ShedBy, s(4), true, &always);
        l.decision(1, 0, Decision::Graft(2), s(5), true, &always);
        assert_eq!(l.edge(1, 2, 0), Some((s(5), "graft_after_shed")));
        for t in 6..6 + LOOP_LOG as u64 {
            l.decision(1, 0, Decision::Declined, s(t), true, &always);
        }
        assert_eq!(l.edge(1, 2, 0), None, "pushed out of the log");
    }
}
