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
//!   with; the signalling relay's isn't. A viewer that has taken in no
//!   stream data for [`STARVE_FOR`], [`JOIN_GRACE`] after its first swarm
//!   link, closes its oldest swarm link every [`STARVE_FOR`] whatever its
//!   budget, so one cut off from the stream links elsewhere.
//! - `pool-uplink`, with `pool-evict`'s: the budget follows the uplink
//!   the runner measured (its push admission's capacity) over the stream
//!   rate the node takes in: [`D_LO`] and [`LINKS_PER_COPY`] for each
//!   copy of the stream the capacity carries, at most [`D_HI`].
//! - `pool-busy`, with `pool-evict`'s: the budget grows by one each time
//!   the uplink has been calm for [`CALM_FOR`] (busy under [`CALM_BUSY`]
//!   and no queue past [`HOT_QUEUE`]), and shrinks by one each time it has
//!   been hot for [`HOT_FOR`] (busy past [`HOT_BUSY`] and queued past
//!   [`HOT_QUEUE`]), between [`D_LO`] and [`D_HI`]: links until the
//!   uplink fills, from [`D_LO`]. `capN` caps it.
//! - `pool-role`: `pool-copies` on the median capacity measured over
//!   [`COPIES_SPAN`], where a viewer that forwards less than
//!   [`LEAF_COPIES`] of the stream (a leaf, its uplink's measure no more
//!   than what it was asked for) takes the top: links cost a leaf little
//!   uplink. A leaf that stays [`BEHIND`] its peers takes [`COPIES_MIN`]:
//!   what it can take in is short. A seed takes [`SEED_LINKS`] and one for
//!   each copy of its stream (whose rate it knows) its uplink carries.
//! - `pool-unblock`, with `pool-evict`: of the swarm links that stay
//!   blocked ([`BLOCKED_SHARE`] of [`USE_SPAN`]), the one the node sent
//!   the most closes whatever the budget, so a peer that asks for more
//!   than it can take in (a downlink short of the stream) doesn't hold
//!   the node's uplink.
//! - `pool-recip`, with `pool-evict`: the link closed past the budget is
//!   the one whose peer gave the least stream data per second since it
//!   came up, past the [`PROTECT`] that gave the most: a peer that serves
//!   nothing back loses its links first wherever links are short.
//! - `pool-copies`, with `pool-evict`'s: the budget steps a link at a time
//!   (at most every [`COPIES_STEP`]) toward [`D_LO`] and
//!   [`LINKS_PER_COPY`] for each copy of the stream the uplink carries:
//!   the most capacity the runner measured over [`COPIES_SPAN`] over the
//!   stream rate the node takes in, between [`COPIES_MIN`] and [`D_HI`],
//!   from [`COPIES_START`]. A seed takes the top.
//! - `pool-shed`, with `pool-evict`'s: from the top of `pool-busy`'s
//!   range, the budget drops by one each [`HOT_FOR`] the uplink stays
//!   hot, its queue standing ([`HOT_SHARE`] of the ticks past
//!   [`HOT_QUEUE`]) while the links' overhead is high ([`HOT_OVERHEAD`] of
//!   what they write past the stream's data), and grows by one each
//!   [`CALM_FOR`] it stays calm ([`CALM_SHARE`]).
//! - `pool-trial`, with `pool-evict`: a link younger than [`MIN_AGE`] is on
//!   trial, unless it is silent in every swarm (ppspp's `SILENT`), which
//!   settles its value at once. While trials stand nothing is closed for
//!   them; once a trial ends, the link that goes is chosen among the old
//!   ones, the newcomer with them. So a newcomer no longer wins by coming,
//!   a full node takes one trial at a time and refuses other offers, and
//!   the host a link went to is shunned for [`RETRY_MAX`]: should it link
//!   again meanwhile, the link closes at once. A refused candidate waits
//!   its retry, whoever offers it.
//! - `pool-useful`, with `pool-trial`: links are kept for the verified new
//!   data their peers sent over [`USE_SPAN`], the node's own measurement,
//!   which a liar, a free rider or a sender of duplicates earns none of.
//!   The links the node pushes to are its leechers' allowance, which push
//!   admission sizes by the measured uplink, and are kept: a weak viewer
//!   that can't give back still takes its share, while a free rider, which
//!   the silence rule declines, gets none. Past them, the [`PROTECT`] that
//!   gave the most are kept, when they gave anything. A seed, which takes
//!   in nothing, closes first the neighbours its reports barred.
//! - `pool-probe`, with `pool-trial`: a node at its budget that is
//!   [`BEHIND`], or has an old link that is neither a child nor a top
//!   giver, dials a candidate drawn at random past its budget, its probe
//!   holding a link of its own so offers still find room. The probe waits
//!   [`MIN_AGE`] after the last, doubling to [`RETRY_MAX`] each time the
//!   probe's link is the one that goes once its trial ends, from
//!   [`MIN_AGE`] again once one stays or the node falls behind. A seed
//!   never probes: its neighbours come to it, one trial at a time.
//! - `pool-near`, with `pool-useful`: among links of equal value the
//!   farthest goes first, by its smallest round trip, within ppspp's
//!   `NEAR_MARGIN`.
//! - `pool-find`: a seed publishes its address under its swarm's key
//!   ([`SOURCE_SALT`]), which only its writer holds, and a viewer resolves
//!   it (vpn checks the record's signature) and dials the source first
//!   among its candidates, while it has room under its budget: once full
//!   it offers the seed nothing past it, so the seed sees each viewer's
//!   offers only while that viewer is still linking, its first wave, and
//!   refuses what its own budget has no room for.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

use strims_node::{
    ExchangeOutcome, HashTableGetOptions, HostId, NetworkId, Node, NodeError, NodeEvent,
};
use strims_ppspp::scheduler_peer::push::seed::NEAR_MARGIN;

use crate::spec::HostIx;
use crate::world::{World, sim_now};

/// How often the pool takes its links' counters and decides.
pub const TICK: Duration = Duration::from_secs(1);
/// The budget without a `capN` flag.
pub const NO_CAP: u32 = 64;
/// The least budget of `pool-uplink` and `pool-busy` ...
pub const D_LO: u32 = 4;
/// ... and the most.
pub const D_HI: u32 = 16;
/// `pool-uplink`: swarm links per copy of the stream the uplink carries.
pub const LINKS_PER_COPY: f64 = 2.0;
/// `pool-role`: a viewer that sends less than this many copies of the
/// stream it takes in is a leaf, and takes the top ...
pub const LEAF_COPIES: f64 = 0.5;
/// ... unless its first missing chunk has stayed this far behind the
/// newest its peers announce, in time at the rate they announce at, for
/// [`HOT_FOR`]: what it can take in, not its uplink, is short, and it
/// takes [`COPIES_MIN`].
pub const BEHIND: Duration = Duration::from_secs(3);
/// `pool-role`: a seed's links over the copies its uplink carries.
pub const SEED_LINKS: f64 = 2.0;
/// `pool-copies`: the budget before the uplink is measured ...
pub const COPIES_START: u32 = 9;
/// ... the least it takes from the measure ...
pub const COPIES_MIN: u32 = 6;
/// ... the span whose most capacity counts (a meter under a demand that
/// doesn't fill the uplink reads low) ...
pub const COPIES_SPAN: Duration = Duration::from_secs(30);
/// ... and the least time between two steps of one link.
pub const COPIES_STEP: Duration = Duration::from_secs(5);
/// `pool-busy`: the uplink is calm under this busy share ...
pub const CALM_BUSY: f64 = 0.8;
/// ... for this long, and hot past this share ...
pub const HOT_BUSY: f64 = 0.95;
/// ... with a queue past this ...
pub const HOT_QUEUE: Duration = Duration::from_millis(25);
/// ... for this long.
pub const HOT_FOR: Duration = Duration::from_secs(10);
pub const CALM_FOR: Duration = Duration::from_secs(30);
/// `pool-shed`: the uplink is hot once it has queued past [`HOT_QUEUE`]
/// at this share of the ticks of the last [`HOT_FOR`] ...
pub const HOT_SHARE: f64 = 0.7;
/// ... while what the links wrote past the stream's data is this share
/// of it at least, over the last [`USE_SPAN`] ...
pub const HOT_OVERHEAD: f64 = 0.15;
/// ... and calm once it has queued so at this share of the ticks of the
/// last [`CALM_FOR`] at most.
pub const CALM_SHARE: f64 = 0.1;
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
/// `pool-unblock`: a swarm link whose transport took no more bytes at
/// this share of the ticks of the last [`USE_SPAN`] closes, at most one
/// every [`STARVE_FOR`], and its host isn't dialed for [`RETRY_MAX`].
pub const BLOCKED_SHARE: f64 = 0.7;
/// A viewer starves after this long without stream data ...
pub const STARVE_FOR: Duration = Duration::from_secs(5);
/// ... once this long has passed since its first swarm link.
pub const JOIN_GRACE: Duration = Duration::from_secs(10);
/// `pool-find`: the salt a seed publishes its address under, with its
/// swarm's key.
pub const SOURCE_SALT: &[u8] = b"source";
/// `pool-find`: how long one lookup of a source waits; a viewer looks
/// again until it has one, for [`JOIN_GRACE`].
pub const RESOLVE_TIMEOUT: Duration = Duration::from_secs(2);

/// How the budget is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Fixed,
    Uplink,
    Busy,
    Shed,
    Copies,
    Role,
}

/// The pool's policy, from the scenario's experiment flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoolConfig {
    /// The budget, fixed or the most a control sets.
    pub budget: u32,
    pub evict: bool,
    /// `pool-recip`: links are kept for what their peers gave.
    pub recip: bool,
    /// `pool-unblock`: a link that can't take what the node sends goes.
    pub unblock: bool,
    /// The rate a seed publishes at, which its publisher knows.
    pub stream_rate: f64,
    pub control: Control,
    /// A hostile hog's pool ([`PoolConfig::greedy`]).
    pub greedy: bool,
    /// `pool-trial`, `pool-useful`, `pool-probe`, `pool-near`, `pool-find`.
    pub trial: bool,
    pub useful: bool,
    pub probe: bool,
    pub near: bool,
    pub find: bool,
}

impl PoolConfig {
    /// The pool the flags ask for; `None` without a pool flag.
    pub fn from_flags(exp: &BTreeSet<&'static str>, cap: Option<u32>) -> Option<PoolConfig> {
        let control = if exp.contains("pool-uplink") {
            Control::Uplink
        } else if exp.contains("pool-busy") {
            Control::Busy
        } else if exp.contains("pool-shed") {
            Control::Shed
        } else if exp.contains("pool-copies") {
            Control::Copies
        } else if exp.contains("pool-role") {
            Control::Role
        } else {
            Control::Fixed
        };
        let evict = exp.contains("pool-evict") || control != Control::Fixed;
        let budget = match control {
            Control::Fixed => cap.unwrap_or(NO_CAP),
            _ => cap.unwrap_or(D_HI),
        };
        (evict || exp.contains("pool-retry")).then_some(PoolConfig {
            budget,
            evict,
            recip: exp.contains("pool-recip"),
            unblock: exp.contains("pool-unblock"),
            stream_rate: 0.0,
            control,
            greedy: false,
            trial: exp.contains("pool-trial")
                || exp.contains("pool-useful")
                || exp.contains("pool-probe")
                || exp.contains("pool-near"),
            useful: exp.contains("pool-useful") || exp.contains("pool-near"),
            probe: exp.contains("pool-probe"),
            near: exp.contains("pool-near"),
            find: exp.contains("pool-find"),
        })
    }

    /// A hog's pool (`hostile.rs`): it links every host it finds, keeps
    /// every link, and dials a host again at once when it loses it or is
    /// refused.
    pub fn greedy(self) -> PoolConfig {
        PoolConfig {
            budget: NO_CAP,
            evict: false,
            recip: false,
            unblock: false,
            control: Control::Fixed,
            greedy: true,
            trial: false,
            useful: false,
            probe: false,
            near: false,
            ..self
        }
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
    /// Bytes the link wrote, at the last tick.
    wire: u64,
    /// Stream data in and out at the last tick.
    read: u64,
    written: u64,
    /// Per tick of the last [`USE_SPAN`]: data in and out.
    recent: Vec<(u64, u64)>,
    /// Stream data in at the link's first tick.
    first_read: Option<u64>,
    /// Per tick of the last [`USE_SPAN`]: whether the transport took no
    /// more bytes.
    blocked: Vec<bool>,
    swarm: bool,
    /// Verified new data in at the last tick, and per tick of the last
    /// [`USE_SPAN`] (`pool-useful`).
    useful: u64,
    valued: Vec<u64>,
    /// The peer subscribes to a stream from us; it is silent in every
    /// swarm; the smallest round trip of its channels; a seed's reports
    /// barred it; our probe made the link.
    child: bool,
    silent: bool,
    min_rtt: Option<Duration>,
    barred: bool,
    probe: bool,
}

impl LinkUse {
    /// Verified new data in over the last [`USE_SPAN`].
    fn value(&self) -> u64 {
        self.valued.iter().sum()
    }

    fn brought(&self) -> u64 {
        self.recent.iter().map(|&(r, _)| r).sum()
    }

    fn took(&self) -> u64 {
        self.recent.iter().map(|&(_, w)| w).sum()
    }

    /// Stream data in since the link came up.
    fn gave(&self) -> u64 {
        self.read
            .saturating_sub(self.first_read.unwrap_or(self.read))
    }
}

#[derive(Debug, Default)]
struct State {
    /// The budget now.
    budget: u32,
    /// Stream data the node took in per tick of the last [`USE_SPAN`].
    took_in: Vec<u64>,
    /// Since when the uplink has been calm, or hot (`pool-busy`).
    calm_since: Option<Duration>,
    hot_since: Option<Duration>,
    /// The node publishes a swarm.
    seed: bool,
    /// The runner's measured upload capacity per tick of the last
    /// [`COPIES_SPAN`], and when the budget last stepped (`pool-copies`).
    capacity: Vec<u64>,
    stepped_at: Option<Duration>,
    /// The newest chunk any peer announced, per tick of the last
    /// [`USE_SPAN`], and the first the node misses (`pool-role`).
    heads: Vec<(u64, u64)>,
    /// Since when the node has been [`BEHIND`] its peers.
    behind_since: Option<Duration>,
    /// When the node first had a swarm link, and last closed one starving.
    first_link: Option<Duration>,
    starved_at: Option<Duration>,
    /// When the node last closed a link that stayed blocked
    /// (`pool-unblock`), and the hosts it doesn't dial until then.
    /// Bound: one a [`STARVE_FOR`] for [`RETRY_MAX`].
    unblocked_at: Option<Duration>,
    shunned: BTreeMap<HostId, Duration>,
    /// Per tick of the last [`CALM_FOR`]: whether the uplink queued past
    /// [`HOT_QUEUE`], and the bytes the links wrote and the stream data
    /// among them (`pool-shed`).
    ticks: Vec<(bool, u64, u64)>,
    /// Bound: the hosts searches found, less those linked.
    candidates: BTreeMap<HostId, Candidate>,
    /// Bound: the node's links.
    links: BTreeMap<HostId, LinkUse>,
    /// Exchanges the pool opened. Bound: [`MAX_DIALING`].
    dialing: BTreeSet<HostId>,
    /// The probe out, dialing or on trial; when the next may go, and the
    /// wait after (`pool-probe`).
    probing: Option<HostId>,
    probe_at: Option<Duration>,
    probe_wait: Duration,
    /// The sources of the swarms the node views, as resolved
    /// (`pool-find`). Bound: one per swarm.
    sources: BTreeSet<HostId>,
}

/// See the [module docs](self).
pub(crate) struct Pool {
    node: Node,
    net: NetworkId,
    cfg: PoolConfig,
    st: RefCell<State>,
    /// The run's world (its rng, the trials' counters) and the host.
    w: Rc<World>,
    h: HostIx,
}

impl Pool {
    pub(crate) fn new(
        node: Node,
        net: NetworkId,
        cfg: PoolConfig,
        w: Rc<World>,
        h: HostIx,
    ) -> Pool {
        let budget = match cfg.control {
            Control::Fixed | Control::Shed => cfg.budget,
            Control::Copies | Control::Role => COPIES_START.min(cfg.budget),
            Control::Uplink | Control::Busy => D_LO.min(cfg.budget),
        };
        Pool {
            node,
            net,
            cfg,
            st: RefCell::new(State {
                budget,
                probe_wait: MIN_AGE,
                ..State::default()
            }),
            w,
            h,
        }
    }

    /// A search found `host`: it becomes a candidate, dialed at once if
    /// the node is under its budget. Returns whether an exchange started.
    pub(crate) async fn offer(&self, host: HostId) -> Result<bool, NodeError> {
        let room = {
            let mut st = self.st.borrow_mut();
            let now = sim_now();
            st.shunned.retain(|_, t| *t > now);
            if st.links.contains_key(&host)
                || st.dialing.contains(&host)
                || st.shunned.contains_key(&host)
            {
                return Ok(false);
            }
            let c = st.candidates.entry(host).or_insert(Candidate {
                found: now,
                tries: 0,
                next_try: now,
            });
            if self.cfg.trial && c.next_try > now {
                return Ok(false);
            }
            let swarm = st.links.values().filter(|l| l.swarm).count();
            swarm + st.dialing.len() < st.budget as usize
        };
        if !room {
            return Ok(false);
        }
        let started = self.node.connect_peer(self.net, host).await?;
        if started {
            self.st.borrow_mut().dialing.insert(host);
        }
        Ok(started)
    }

    /// Takes the node's link and exchange events.
    pub(crate) fn handle_event(&self, e: &NodeEvent) {
        let now = sim_now();
        let mut st = self.st.borrow_mut();
        match e {
            NodeEvent::PeerAdded(p) => {
                st.candidates.remove(&p.host_id);
                let probe = st.probing == Some(p.host_id);
                st.links.insert(
                    p.host_id,
                    LinkUse {
                        since: now,
                        probe,
                        ..LinkUse::default()
                    },
                );
            }
            NodeEvent::PeerRemoved { host_id } => {
                if st.probing == Some(*host_id) {
                    st.probing = None;
                    st.probe_at = Some(now + st.probe_wait);
                }
                if st.links.remove(host_id).is_some_and(|l| l.swarm) {
                    let wait = if self.cfg.greedy {
                        Duration::ZERO
                    } else {
                        RETRY_MAX
                    };
                    st.candidates.entry(*host_id).or_insert(Candidate {
                        found: now,
                        tries: 1,
                        next_try: now + wait,
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
                // A refused probe waits as the last did.
                if !linked && st.probing == Some(*host_id) {
                    st.probing = None;
                    st.probe_at = Some(now + st.probe_wait);
                }
                if let Some(c) = st.candidates.get_mut(host_id)
                    && !linked
                {
                    c.tries += 1;
                    let wait = RETRY.saturating_mul(1 << c.tries.min(8)) / 2;
                    c.next_try = now
                        + if self.cfg.greedy {
                            RETRY
                        } else {
                            wait.min(RETRY_MAX)
                        };
                }
            }
            _ => {}
        }
    }

    /// Ticks every [`TICK`] for as long as the node runs.
    /// `pool-find`: looks up the sources of the swarms the node views,
    /// each until it is found or [`JOIN_GRACE`] has passed, and makes each
    /// found one a candidate the node dials first.
    pub(crate) async fn resolve_sources(&self) {
        if !self.cfg.find {
            return;
        }
        let mut keys: Vec<Vec<u8>> = self
            .w
            .spec
            .swarms
            .iter()
            .zip(&self.w.swarm_keys)
            .filter(|(s, _)| s.viewers.contains(&self.h))
            .map(|(_, k)| k.public.clone())
            .collect();
        let start = sim_now();
        while !keys.is_empty() && sim_now().saturating_sub(start) < JOIN_GRACE {
            let mut left = Vec::new();
            for key in keys {
                let options = HashTableGetOptions {
                    disable_cache: false,
                    timeout: strims_base::time::Duration::from_secs(
                        RESOLVE_TIMEOUT.as_secs().cast_signed(),
                    ),
                };
                let found = self
                    .node
                    .resolve_vpn_address(self.net, key.clone(), SOURCE_SALT.to_vec(), options)
                    .await;
                match found {
                    Ok(Some(addr)) if addr.host != self.node.host_id() => {
                        let now = sim_now();
                        let mut st = self.st.borrow_mut();
                        st.sources.insert(addr.host);
                        if !st.links.contains_key(&addr.host) {
                            st.candidates.entry(addr.host).or_insert(Candidate {
                                found: now,
                                tries: 0,
                                next_try: now,
                            });
                        }
                        self.w.source_resolved(self.h);
                    }
                    Ok(Some(_)) => {}
                    _ => left.push(key),
                }
            }
            keys = left;
            if !keys.is_empty() {
                tokio::time::sleep(TICK).await;
            }
        }
    }

    pub(crate) async fn run(&self) {
        let headroom = u32::from(self.cfg.evict);
        loop {
            tokio::time::sleep(TICK).await;
            let Ok(m) = self.node.metrics().await else {
                return;
            };
            self.w.observe_bins(self.h, &m);
            let infra = self.observe(&m);
            self.control(&m);
            let max = {
                let st = self.st.borrow();
                st.budget + infra + headroom + u32::from(st.probing.is_some())
            };
            if self.node.set_max_peers(max).await.is_err() {
                return;
            }
            for host in self.to_dial() {
                let started = matches!(self.node.connect_peer(self.net, host).await, Ok(true));
                let mut st = self.st.borrow_mut();
                if started {
                    st.dialing.insert(host);
                } else if st.probing == Some(host) {
                    st.probing = None;
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
            l.first_read.get_or_insert(r);
            l.swarm = !p.channels.is_empty();
            l.valued.push(p.useful.saturating_sub(l.useful));
            if l.valued.len() > span {
                l.valued.remove(0);
            }
            l.useful = p.useful;
            let stats = || p.channels.iter().filter_map(|c| c.stats.as_ref());
            l.child = stats().any(|s| s.scheduler.subscribed > 0);
            l.silent = l.swarm
                && stats().count() == p.channels.len()
                && stats().all(|s| s.scheduler.silent);
            l.min_rtt = stats()
                .filter_map(|s| s.scheduler.min_rtt.to_std())
                .filter(|d| !d.is_zero())
                .min();
        }
        let barred: BTreeSet<HostId> = m
            .runner
            .swarms
            .iter()
            .flat_map(|s| s.seed_neighbours.iter())
            .filter(|n| n.barred)
            .map(|n| n.host_id)
            .collect();
        for (h, l) in st.links.iter_mut() {
            l.barred = barred.contains(h);
        }
        let mut wire = 0;
        for link in &m.links {
            let Some(l) = link.host_id.and_then(|h| st.links.get_mut(&h)) else {
                continue;
            };
            wire += link.metrics.write_bytes.saturating_sub(l.wire);
            l.wire = link.metrics.write_bytes;
            l.blocked.push(link.write_blocked);
            if l.blocked.len() > span {
                l.blocked.remove(0);
            }
        }
        let data: u64 = st
            .links
            .values()
            .filter_map(|l| l.recent.last())
            .map(|&(_, w)| w)
            .sum();
        let queued = m
            .runner
            .swarms
            .iter()
            .filter_map(|s| s.push.as_ref()?.upload_queue_delay?.to_std())
            .any(|d| d >= HOT_QUEUE);
        st.ticks.push((queued, wire, data));
        st.seed = m.runner.swarms.iter().any(|s| s.publishers > 0);
        if let Some(s) = m.runner.swarms.first()
            && let Some(sch) = &s.scheduler
        {
            st.heads.push((sch.peer_max_have_bin.0, s.store.next.0));
            if st.heads.len() > span {
                st.heads.remove(0);
            }
            let first = st.heads.first().copied().unwrap_or_default();
            let last = st.heads.last().copied().unwrap_or_default();
            let secs = (st.heads.len().saturating_sub(1) as f64 * TICK.as_secs_f64()).max(1.0);
            let per_sec = last.0.saturating_sub(first.0) as f64 / secs;
            let gap = last.0.saturating_sub(last.1) as f64;
            let behind = per_sec > 0.0 && gap / per_sec >= BEHIND.as_secs_f64();
            st.behind_since = if behind {
                st.behind_since.or(Some(sim_now()))
            } else {
                None
            };
        }
        if st.first_link.is_none() && st.links.values().any(|l| l.swarm) {
            st.first_link = Some(sim_now());
        }
        let keep = (CALM_FOR.as_millis() / TICK.as_millis()).max(1) as usize;
        if st.ticks.len() > keep {
            st.ticks.remove(0);
        }
        let total: u64 = st
            .links
            .values()
            .filter_map(|l| l.recent.last())
            .map(|&(r, _)| r)
            .sum();
        st.took_in.push(total);
        if st.took_in.len() > span {
            st.took_in.remove(0);
        }
        let swarm = st.links.values().filter(|l| l.swarm).count();
        u32::try_from(st.links.len() - swarm).unwrap_or(u32::MAX)
    }

    /// Sets the budget from the tick's measures (`pool-uplink`,
    /// `pool-busy`).
    fn control(&self, m: &strims_node::NodeMetrics) {
        let now = sim_now();
        let push = m.runner.swarms.iter().filter_map(|s| s.push.as_ref());
        let (mut capacity, mut busy, mut queue) = (0u64, 0.0f64, Duration::ZERO);
        for p in push {
            capacity = capacity.max(p.upload_capacity);
            busy = busy.max(f64::from(p.upload_busy_permille) / 1000.0);
            queue = queue.max(
                p.upload_queue_delay
                    .and_then(|d| d.to_std())
                    .unwrap_or_default(),
            );
        }
        let mut st = self.st.borrow_mut();
        let top = self.cfg.budget;
        match self.cfg.control {
            Control::Fixed => {}
            Control::Uplink => {
                let ticks = st.took_in.len().max(1) as f64;
                let rate = st.took_in.iter().sum::<u64>() as f64 / ticks / TICK.as_secs_f64();
                if capacity > 0 && rate > 0.0 {
                    let copies = capacity as f64 / rate;
                    let b = f64::from(D_LO) + LINKS_PER_COPY * copies;
                    st.budget = (b as u32).clamp(D_LO.min(top), top);
                }
            }
            Control::Copies | Control::Role => {
                st.capacity.push(capacity);
                let keep = (COPIES_SPAN.as_millis() / TICK.as_millis()).max(1) as usize;
                if st.capacity.len() > keep {
                    st.capacity.remove(0);
                }
                let most = if self.cfg.control == Control::Role {
                    let mut c = st.capacity.clone();
                    c.sort_unstable();
                    c.get(c.len() / 2).copied().unwrap_or(0)
                } else {
                    st.capacity.iter().copied().max().unwrap_or(0)
                };
                let ticks = st.took_in.len().max(1) as f64;
                let rate = st.took_in.iter().sum::<u64>() as f64 / ticks / TICK.as_secs_f64();
                let sent = st
                    .ticks
                    .iter()
                    .rev()
                    .take(st.took_in.len())
                    .map(|x| x.2)
                    .sum::<u64>() as f64
                    / ticks
                    / TICK.as_secs_f64();
                let leaf = self.cfg.control == Control::Role && sent < LEAF_COPIES * rate;
                let behind = st
                    .behind_since
                    .is_some_and(|t| now.saturating_sub(t) >= HOT_FOR);
                let target = if self.cfg.control == Control::Role && st.seed {
                    let copies = most as f64 / self.cfg.stream_rate.max(1.0);
                    ((SEED_LINKS + copies) as u32).clamp(COPIES_MIN.min(top), top)
                } else if st.seed {
                    top
                } else if leaf && behind {
                    COPIES_MIN.min(top)
                } else if leaf {
                    top
                } else if most > 0 && rate > 0.0 {
                    let b = f64::from(D_LO) + LINKS_PER_COPY * most as f64 / rate;
                    (b as u32).clamp(COPIES_MIN.min(top), top)
                } else {
                    st.budget
                };
                if target != st.budget
                    && st
                        .stepped_at
                        .is_none_or(|t| now.saturating_sub(t) >= COPIES_STEP)
                {
                    st.budget = if target > st.budget {
                        st.budget + 1
                    } else {
                        st.budget - 1
                    };
                    st.stepped_at = Some(now);
                }
            }
            Control::Shed => {
                let n = |d: Duration| (d.as_millis() / TICK.as_millis()).max(1) as usize;
                let recent = |d: Duration| {
                    st.ticks
                        .get(st.ticks.len().saturating_sub(n(d))..)
                        .unwrap_or_default()
                };
                let share = |t: &[(bool, u64, u64)]| {
                    t.iter().filter(|x| x.0).count() as f64 / t.len().max(1) as f64
                };
                let (wire, data) = recent(USE_SPAN)
                    .iter()
                    .fold((0, 0), |(w, d), x| (w + x.1, d + x.2));
                let overhead = if wire > 0 {
                    1.0 - data as f64 / wire as f64
                } else {
                    0.0
                };
                let hot = recent(HOT_FOR).len() >= n(HOT_FOR)
                    && share(recent(HOT_FOR)) >= HOT_SHARE
                    && overhead >= HOT_OVERHEAD;
                let calm =
                    recent(CALM_FOR).len() >= n(CALM_FOR) && share(recent(CALM_FOR)) <= CALM_SHARE;
                if hot
                    && st
                        .hot_since
                        .is_none_or(|t| now.saturating_sub(t) >= HOT_FOR)
                {
                    st.budget = st.budget.saturating_sub(1).max(D_LO.min(top));
                    st.hot_since = Some(now);
                } else if calm
                    && st
                        .calm_since
                        .is_none_or(|t| now.saturating_sub(t) >= CALM_FOR)
                {
                    st.budget = (st.budget + 1).min(top);
                    st.calm_since = Some(now);
                }
            }
            Control::Busy => {
                let calm = busy < CALM_BUSY && queue < HOT_QUEUE;
                let hot = busy > HOT_BUSY && queue >= HOT_QUEUE;
                st.calm_since = if calm {
                    st.calm_since.or(Some(now))
                } else {
                    None
                };
                st.hot_since = if hot {
                    st.hot_since.or(Some(now))
                } else {
                    None
                };
                if st
                    .calm_since
                    .is_some_and(|t| now.saturating_sub(t) >= CALM_FOR)
                {
                    st.budget = (st.budget + 1).min(top);
                    st.calm_since = Some(now);
                }
                if st
                    .hot_since
                    .is_some_and(|t| now.saturating_sub(t) >= HOT_FOR)
                {
                    st.budget = st.budget.saturating_sub(1).max(D_LO.min(top));
                    st.hot_since = Some(now);
                }
            }
        }
    }

    /// The candidates to dial this tick; with `pool-probe`, a probe past
    /// the budget when one is due.
    fn to_dial(&self) -> Vec<HostId> {
        let now = sim_now();
        let mut st = self.st.borrow_mut();
        let swarm = st.links.values().filter(|l| l.swarm).count();
        let room = (st.budget as usize).saturating_sub(swarm + st.dialing.len());
        let slots = room.min(MAX_DIALING.saturating_sub(st.dialing.len()));
        let mut due: Vec<(HostId, u32, Duration)> = st
            .candidates
            .iter()
            .filter(|(h, c)| {
                c.next_try <= now
                    && !st.dialing.contains(h)
                    && st.shunned.get(h).is_none_or(|&t| t <= now)
            })
            .map(|(&h, c)| (h, c.tries, c.found))
            .collect();
        // A swarm's source first (`pool-find`; none without).
        let sources = &st.sources;
        due.sort_by_key(|&(h, tries, found)| (!sources.contains(&h), tries, found));
        let mut out: Vec<HostId> = due.iter().take(slots).map(|d| d.0).collect();
        if self.cfg.probe && out.is_empty() && !due.is_empty() {
            let behind = st.behind_since.is_some();
            if behind {
                st.probe_wait = MIN_AGE;
            }
            let replaceable = {
                let protected = Self::protected(&self.cfg, &st);
                st.links.iter().any(|(h, l)| {
                    l.swarm && now.saturating_sub(l.since) >= MIN_AGE && !protected.contains(h)
                })
            };
            let ready = !st.seed
                && swarm == st.budget as usize
                && st.dialing.is_empty()
                && st.probing.is_none()
                && st
                    .first_link
                    .is_some_and(|t| now.saturating_sub(t) >= MIN_AGE)
                && st.probe_at.is_none_or(|t| now >= t)
                && (behind || replaceable);
            if ready {
                let n = due.len();
                let k = self
                    .w
                    .with_rng(|r| (strims_wan::sample::unit(r) * n as f64) as usize)
                    .min(n - 1);
                if let Some(&(h, ..)) = due.get(k) {
                    st.probing = Some(h);
                    out.push(h);
                }
            }
        }
        out
    }

    /// The swarm link that goes past the budget, if one does (see the
    /// [module docs](self)).
    fn choose(cfg: &PoolConfig, st: &State, now: Duration) -> Option<HostId> {
        let swarm: Vec<(&HostId, &LinkUse)> = st.links.iter().filter(|(_, l)| l.swarm).collect();
        if swarm.len() <= st.budget as usize {
            return None;
        }
        let young = |l: &LinkUse| now.saturating_sub(l.since) < MIN_AGE;
        if cfg.trial
            && swarm.len() - st.budget as usize
                <= swarm.iter().filter(|(_, l)| young(l) && !l.silent).count()
        {
            return None;
        }
        let protected = Self::protected(cfg, st);
        let unprotected = swarm
            .iter()
            .filter(|(h, l)| !protected.contains(h) && (!young(l) || (cfg.trial && l.silent)));
        if cfg.useful {
            let far = |l: &LinkUse| {
                if !cfg.near {
                    return 0;
                }
                let margin = NEAR_MARGIN.to_std().unwrap_or_default().as_nanos().max(1);
                l.min_rtt.map_or(u128::MAX, |d| d.as_nanos() / margin)
            };
            return unprotected
                .min_by_key(|(h, l)| {
                    (
                        !l.barred,
                        l.value(),
                        std::cmp::Reverse(far(l)),
                        std::cmp::Reverse(l.since),
                        **h,
                    )
                })
                .map(|(h, _)| **h);
        }
        if cfg.recip {
            // What the peer gave, per second linked.
            let rate = |l: &LinkUse| {
                let secs = now.saturating_sub(l.since).as_secs().max(1);
                l.gave() / secs
            };
            return unprotected
                .min_by_key(|(h, l)| (rate(l), std::cmp::Reverse(l.since), **h))
                .map(|(h, _)| **h);
        }
        unprotected
            .min_by_key(|(h, l)| (l.brought() + l.took(), std::cmp::Reverse(l.since), **h))
            .map(|(h, _)| **h)
    }

    /// The links eviction keeps whatever their use: with `pool-useful` the
    /// node's children and the [`PROTECT`] that gave it the most verified
    /// new data, when they gave any; otherwise the [`PROTECT`] that brought
    /// it the most data and the [`PROTECT`] it sent the most to (with
    /// `pool-recip`, that gave it the most since they came up).
    fn protected(cfg: &PoolConfig, st: &State) -> BTreeSet<HostId> {
        let swarm: Vec<(&HostId, &LinkUse)> = st.links.iter().filter(|(_, l)| l.swarm).collect();
        let mut protected: BTreeSet<HostId> = BTreeSet::new();
        if cfg.useful {
            protected.extend(swarm.iter().filter(|(_, l)| l.child).map(|(h, _)| **h));
            let mut by: Vec<&(&HostId, &LinkUse)> =
                swarm.iter().filter(|(_, l)| l.value() > 0).collect();
            by.sort_by_key(|(h, l)| (std::cmp::Reverse(l.value()), **h));
            protected.extend(by.iter().take(PROTECT).map(|(h, _)| **h));
            return protected;
        }
        let keys: &[fn(&LinkUse) -> u64] = if cfg.recip {
            &[LinkUse::gave]
        } else {
            &[LinkUse::brought, LinkUse::took]
        };
        for key in keys {
            let mut by: Vec<&(&HostId, &LinkUse)> = swarm.iter().collect();
            by.sort_by_key(|(h, l)| (std::cmp::Reverse(key(l)), **h));
            protected.extend(by.iter().take(PROTECT).map(|(h, _)| **h));
        }
        protected
    }

    /// The swarm link to close, if the node has more than its budget.
    fn victim(&self) -> Option<HostId> {
        let now = sim_now();
        let mut st = self.st.borrow_mut();
        // A host shunned after its link went that linked again goes at once.
        if self.cfg.trial
            && let Some(&h) = st.links.iter().find_map(|(h, l)| {
                (l.swarm && st.shunned.get(h).is_some_and(|&t| t > now)).then_some(h)
            })
        {
            return Some(h);
        }
        if self.cfg.unblock
            && st
                .unblocked_at
                .is_none_or(|t| now.saturating_sub(t) >= STARVE_FOR)
        {
            let span = (USE_SPAN.as_millis() / TICK.as_millis()).max(1) as usize;
            let stuck = st
                .links
                .iter()
                .filter(|(_, l)| {
                    l.swarm
                        && now.saturating_sub(l.since) >= MIN_AGE
                        && l.blocked.len() >= span
                        && l.blocked.iter().filter(|&&b| b).count() as f64
                            >= BLOCKED_SHARE * span as f64
                })
                .max_by_key(|(h, l)| (l.took(), **h));
            if let Some((&h, _)) = stuck {
                st.unblocked_at = Some(now);
                st.shunned.insert(h, now + RETRY_MAX);
                return Some(h);
            }
        }
        let swarm: Vec<(HostId, Duration, u64)> = st
            .links
            .iter()
            .filter(|(_, l)| l.swarm)
            .map(|(h, l)| (*h, l.since, l.brought() + l.took()))
            .collect();
        let starve = (STARVE_FOR.as_millis() / TICK.as_millis()).max(1) as usize;
        let starving = !st.seed
            && st
                .first_link
                .is_some_and(|t| now.saturating_sub(t) >= JOIN_GRACE)
            && st.took_in.len() >= starve
            && st.took_in.iter().rev().take(starve).all(|&b| b == 0)
            && st
                .starved_at
                .is_none_or(|t| now.saturating_sub(t) >= STARVE_FOR);
        if starving
            && let Some(&(oldest, ..)) = swarm.iter().min_by_key(|(h, since, _)| (*since, *h))
        {
            st.starved_at = Some(now);
            return Some(oldest);
        }
        let chosen = Self::choose(&self.cfg, &st, now);
        if let Some(v) = chosen
            && self.cfg.trial
        {
            st.shunned.insert(v, now + RETRY_MAX);
        }
        // The probe's trial ends: it stayed unless its link is the one
        // that goes.
        if let Some(pr) = st.probing
            && st
                .links
                .get(&pr)
                .is_some_and(|l| l.probe && now.saturating_sub(l.since) >= MIN_AGE)
        {
            let won = chosen != Some(pr);
            st.probe_wait = if won {
                MIN_AGE
            } else {
                (st.probe_wait * 2).min(RETRY_MAX)
            };
            st.probing = None;
            st.probe_at = Some(now + st.probe_wait);
            self.w.pool_trial(self.h, won);
        }
        chosen
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn host(i: u8) -> HostId {
        HostId::from_bytes(&[i; 32])
    }

    fn cfg(flags: &[&'static str]) -> PoolConfig {
        let exp: BTreeSet<&'static str> = flags.iter().copied().collect();
        PoolConfig::from_flags(&exp, Some(2)).unwrap()
    }

    /// A node with a budget of two and `links`: (host, linked at, verified
    /// new data over the span).
    fn state(links: &[(u8, u64, u64)]) -> State {
        let mut st = State {
            budget: 2,
            ..State::default()
        };
        for &(i, since, value) in links {
            st.links.insert(
                host(i),
                LinkUse {
                    since: Duration::from_secs(since),
                    swarm: true,
                    valued: vec![value],
                    ..LinkUse::default()
                },
            );
        }
        st
    }

    #[test]
    fn a_trial_closes_nothing_until_it_ends_and_the_incumbent_wins_ties() {
        let c = cfg(&["pool-role", "pool-useful"]);
        // 1 gave nothing, 2 gave, 3 came at 30 s and gave nothing.
        let st = state(&[(1, 0, 0), (2, 0, 100), (3, 30, 0)]);
        assert_eq!(Pool::choose(&c, &st, Duration::from_secs(40)), None);
        // Its trial over, the newcomer goes before the incumbent.
        assert_eq!(
            Pool::choose(&c, &st, Duration::from_secs(50)),
            Some(host(3))
        );
    }

    #[test]
    fn a_silent_trial_ends_at_once_and_children_stay() {
        let c = cfg(&["pool-role", "pool-useful"]);
        let mut st = state(&[(1, 0, 0), (2, 0, 100), (3, 30, 0)]);
        st.links.get_mut(&host(3)).unwrap().silent = true;
        assert_eq!(
            Pool::choose(&c, &st, Duration::from_secs(31)),
            Some(host(3))
        );
        // A child that gives nothing back stays; the old non-child goes.
        let mut st = state(&[(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
        st.links.get_mut(&host(1)).unwrap().child = true;
        st.links.get_mut(&host(2)).unwrap().child = true;
        assert_eq!(
            Pool::choose(&c, &st, Duration::from_secs(40)),
            Some(host(3))
        );
    }

    #[test]
    fn a_seed_closes_the_barred_first_and_near_keeps_the_nearest() {
        let c = cfg(&["pool-role", "pool-near"]);
        let mut st = state(&[(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
        st.links.get_mut(&host(1)).unwrap().barred = true;
        assert_eq!(
            Pool::choose(&c, &st, Duration::from_secs(40)),
            Some(host(1))
        );
        let mut st = state(&[(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
        let rtt = |ms| Some(Duration::from_millis(ms));
        st.links.get_mut(&host(1)).unwrap().min_rtt = rtt(20);
        st.links.get_mut(&host(2)).unwrap().min_rtt = rtt(90);
        st.links.get_mut(&host(3)).unwrap().min_rtt = rtt(22);
        assert_eq!(
            Pool::choose(&c, &st, Duration::from_secs(40)),
            Some(host(2))
        );
    }
}
