// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A host's peer pool in the organic mode ([`Scenario::organic`]), for the
//! network construction experiments ([`Scenario::exp`]): which of the hosts
//! transfer discovery finds the node links to, how many swarm links it
//! keeps, and which it drops. It decides from what a deployed node's app
//! layer has, the node's public API (its links and their counters, the
//! runner's data per peer, how its peer exchanges ended), and acts through
//! it (`connect_peer`, `close_peer`, `set_max_peers`).
//!
//! Without a pool flag the node keeps transfer discovery's own policy:
//! link every host a search finds, and leave a refused one for the
//! search's backoff. With one:
//!
//! - `pool-retry`: the hosts searches find are candidates. While the node
//!   has fewer swarm links than its budget (`capN`, else [`NO_CAP`]), and
//!   fewer exchanges open than [`MAX_DIALING`], it dials the candidate
//!   tried least, longest known first, every [`TICK`]; a refused or failed
//!   exchange waits [`RETRY`], doubling with each try up to
//!   [`RETRY_MAX`].
//! - `pool-evict`, with `pool-retry`'s dialing: the node takes one link
//!   past its budget, so a newcomer's offer always finds room, and closes
//!   the least useful swarm link once it has more than the budget: the one
//!   that carried the least stream data either way over the last
//!   [`USE_SPAN`], among those at least [`MIN_AGE`] old, past the
//!   [`PROTECT`] that brought it the most data and the [`PROTECT`] it sent
//!   the most to. A swarm link is one to a peer the runner runs a swarm
//!   with; the signalling relay's isn't.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use strims_node::{ExchangeOutcome, HostId, NetworkId, Node, NodeError, NodeEvent};

use crate::world::sim_now;

/// How often the pool takes its links' counters and decides.
pub const TICK: Duration = Duration::from_secs(1);
/// The budget without a `capN` flag.
pub const NO_CAP: u32 = 64;
/// The most exchanges the pool keeps open.
pub const MAX_DIALING: usize = 4;
/// The wait after a candidate's first failed exchange ...
pub const RETRY: Duration = Duration::from_secs(2);
/// ... doubling up to this.
pub const RETRY_MAX: Duration = Duration::from_secs(60);
/// How long a link's use counts toward its value.
pub const USE_SPAN: Duration = Duration::from_secs(10);
/// A link younger than this is never closed for its use.
pub const MIN_AGE: Duration = Duration::from_secs(20);
/// The links protected from closing for each of what they brought and
/// what they took.
pub const PROTECT: usize = 2;

/// The pool's policy, from the scenario's experiment flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoolConfig {
    pub budget: u32,
    pub evict: bool,
}

impl PoolConfig {
    /// The pool the flags ask for; `None` without a pool flag.
    pub fn from_flags(exp: &BTreeSet<&'static str>, cap: Option<u32>) -> Option<PoolConfig> {
        let evict = exp.contains("pool-evict");
        (evict || exp.contains("pool-retry")).then(|| PoolConfig {
            budget: cap.unwrap_or(NO_CAP),
            evict,
        })
    }
}

#[derive(Debug)]
struct Candidate {
    found: Duration,
    tries: u32,
    next_try: Duration,
}

#[derive(Debug, Default)]
struct LinkUse {
    since: Duration,
    /// Stream data in and out at the last tick.
    read: u64,
    written: u64,
    /// Per tick of the last [`USE_SPAN`]: data in and out.
    recent: Vec<(u64, u64)>,
    swarm: bool,
}

impl LinkUse {
    fn brought(&self) -> u64 {
        self.recent.iter().map(|&(r, _)| r).sum()
    }

    fn took(&self) -> u64 {
        self.recent.iter().map(|&(_, w)| w).sum()
    }
}

#[derive(Debug, Default)]
struct State {
    /// Bound: the hosts searches found, less those linked.
    candidates: BTreeMap<HostId, Candidate>,
    /// Bound: the node's links.
    links: BTreeMap<HostId, LinkUse>,
    /// Exchanges the pool opened. Bound: [`MAX_DIALING`].
    dialing: BTreeSet<HostId>,
}

/// See the [module docs](self).
pub(crate) struct Pool {
    node: Node,
    net: NetworkId,
    cfg: PoolConfig,
    st: RefCell<State>,
}

impl Pool {
    pub(crate) fn new(node: Node, net: NetworkId, cfg: PoolConfig) -> Pool {
        Pool {
            node,
            net,
            cfg,
            st: RefCell::default(),
        }
    }

    /// A search found `host`: it becomes a candidate, dialed at the next
    /// tick if there's room. Returns whether an exchange started (never
    /// here).
    pub(crate) fn offer(&self, host: HostId) -> Result<bool, NodeError> {
        let mut st = self.st.borrow_mut();
        if !st.links.contains_key(&host) {
            let now = sim_now();
            st.candidates.entry(host).or_insert(Candidate {
                found: now,
                tries: 0,
                next_try: now,
            });
        }
        Ok(false)
    }

    /// Takes the node's link and exchange events.
    pub(crate) fn handle_event(&self, e: &NodeEvent) {
        let now = sim_now();
        let mut st = self.st.borrow_mut();
        match e {
            NodeEvent::PeerAdded(p) => {
                st.candidates.remove(&p.host_id);
                st.links.insert(
                    p.host_id,
                    LinkUse {
                        since: now,
                        ..LinkUse::default()
                    },
                );
            }
            NodeEvent::PeerRemoved { host_id } => {
                if st.links.remove(host_id).is_some_and(|l| l.swarm) {
                    st.candidates.entry(*host_id).or_insert(Candidate {
                        found: now,
                        tries: 1,
                        next_try: now + RETRY_MAX,
                    });
                }
            }
            NodeEvent::ExchangeClosed {
                host_id, outcome, ..
            } => {
                st.dialing.remove(host_id);
                let linked = matches!(
                    outcome,
                    ExchangeOutcome::Connected | ExchangeOutcome::RemoteConnected
                );
                if let Some(c) = st.candidates.get_mut(host_id)
                    && !linked
                {
                    c.tries += 1;
                    let wait = RETRY.saturating_mul(1 << c.tries.min(8)) / 2;
                    c.next_try = now + wait.min(RETRY_MAX);
                }
            }
            _ => {}
        }
    }

    /// Ticks every [`TICK`] for as long as the node runs.
    pub(crate) async fn run(&self) {
        let headroom = u32::from(self.cfg.evict);
        loop {
            tokio::time::sleep(TICK).await;
            let Ok(m) = self.node.metrics().await else {
                return;
            };
            let infra = self.observe(&m);
            let max = self.cfg.budget + infra + headroom;
            if self.node.set_max_peers(max).await.is_err() {
                return;
            }
            for host in self.to_dial() {
                if let Ok(true) = self.node.connect_peer(self.net, host).await {
                    self.st.borrow_mut().dialing.insert(host);
                }
            }
            if self.cfg.evict
                && let Some(victim) = self.victim()
            {
                let _ = self.node.close_peer(victim).await;
            }
        }
    }

    /// Folds the tick's counters into the links' use. Returns the links
    /// that aren't swarm links.
    fn observe(&self, m: &strims_node::NodeMetrics) -> u32 {
        let mut st = self.st.borrow_mut();
        let span = (USE_SPAN.as_millis() / TICK.as_millis()).max(1) as usize;
        for p in &m.runner.peers {
            let Some(l) = st.links.get_mut(&p.host_id) else {
                continue;
            };
            let (r, w) = (p.metrics.read.count, p.metrics.write.count);
            l.recent
                .push((r.saturating_sub(l.read), w.saturating_sub(l.written)));
            if l.recent.len() > span {
                l.recent.remove(0);
            }
            (l.read, l.written) = (r, w);
            l.swarm = !p.channels.is_empty();
        }
        let swarm = st.links.values().filter(|l| l.swarm).count();
        u32::try_from(st.links.len() - swarm).unwrap_or(u32::MAX)
    }

    /// The candidates to dial this tick.
    fn to_dial(&self) -> Vec<HostId> {
        let now = sim_now();
        let st = self.st.borrow();
        let swarm = st.links.values().filter(|l| l.swarm).count();
        let room = (self.cfg.budget as usize).saturating_sub(swarm + st.dialing.len());
        let slots = room.min(MAX_DIALING.saturating_sub(st.dialing.len()));
        let mut due: Vec<(&HostId, &Candidate)> = st
            .candidates
            .iter()
            .filter(|(h, c)| c.next_try <= now && !st.dialing.contains(h))
            .collect();
        due.sort_by_key(|(_, c)| (c.tries, c.found));
        due.into_iter().take(slots).map(|(&h, _)| h).collect()
    }

    /// The swarm link to close, if the node has more than its budget.
    fn victim(&self) -> Option<HostId> {
        let now = sim_now();
        let st = self.st.borrow();
        let swarm: Vec<(&HostId, &LinkUse)> = st.links.iter().filter(|(_, l)| l.swarm).collect();
        if swarm.len() <= self.cfg.budget as usize {
            return None;
        }
        let mut protected: BTreeSet<HostId> = BTreeSet::new();
        for key in [LinkUse::brought, LinkUse::took] {
            let mut by: Vec<&(&HostId, &LinkUse)> = swarm.iter().collect();
            by.sort_by_key(|(h, l)| (std::cmp::Reverse(key(l)), **h));
            protected.extend(by.iter().take(PROTECT).map(|(h, _)| **h));
        }
        swarm
            .iter()
            .filter(|(h, l)| now.saturating_sub(l.since) >= MIN_AGE && !protected.contains(h))
            .min_by_key(|(h, l)| (l.brought() + l.took(), std::cmp::Reverse(l.since), **h))
            .map(|(h, _)| **h)
    }
}
