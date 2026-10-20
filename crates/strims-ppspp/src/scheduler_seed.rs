// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What a hybrid seed pushes to whom ([`SeedPolicy`]).
//!
//! A seed receives no data, so reciprocity weighs its peers alike, and
//! every neighbour's graft prefers it: it announces first. Its admission
//! alone therefore places its pushes, by accepting, declining and closing
//! STREAM_REQUESTs, with no message of its own.
//!
//! # Adaptive allocation
//!
//! The seed's measured upload pays for some whole copies of the stream. It
//! pushes them to that many *holders*, the neighbours with the shortest
//! round trip to it, and declines everyone else: a holder's requests are
//! accepted for every stream, so its copy is whole. Near holders have each
//! chunk soonest and relay it from about where the seed would, multiplying
//! its uplink; a whole copy at few peers keeps the trees shallow. The
//! round trip includes the last mile, so mobile and satellite links rank
//! behind wired ones nearby.
//!
//! - **Budget.** K stream slots (one subscriber of one stream each) are
//!   what [`ALLOC_UTILIZATION`] of the measured upload capacity carries at
//!   a subscriber's cost; they pay for K / streams copies, or K of the
//!   streams at one holder when K is under one copy (the rest go to pull).
//!   It falls only once it is [`OVER_BUDGET`] too high. Pull answers take
//!   what pushes leave of the uplink, so neither they nor the time the
//!   uplink is busy with them bound it: [`SeedQos::ByAllocation`] puts
//!   them behind the pushes.
//! - **Holders.** While there are fewer holders than copies, the next
//!   neighbour to ask becomes one; until the store rate has been measured
//!   for [`ALLOC_MEASURE`] there are [`UNMEASURED_COPIES`], so the first
//!   to ask (the nearest) hold the stream. Past the copies, the farthest
//!   holders go. A neighbour
//!   displaces the farthest holder only with a round trip [`NEAR_MARGIN`]
//!   (or a tenth) shorter, one per [`ALLOC_CHECK_INTERVAL`], so jitter
//!   doesn't move the allocation. A neighbour that stops being a holder
//!   loses all its subscriptions (STREAM_CLOSE).
//! - **Round trips** are each neighbour's smallest ping round trip over
//!   the last one or two [`RTT_WINDOW`]s, which leaves out queueing.
//! - With room for every neighbour's every stream, everything is accepted.
//!
//! # Pins and uplink shares
//!
//! - **A pin** (`Runner::pin_seed_allocation`) fixes the peers each stream
//!   is pushed to: their requests are accepted, everyone else's declined
//!   and every other subscriber closed, and nothing is shed.
//! - **Uplink shares.** With [`SeedQos::ByAllocation`] the runner weights
//!   each peer at least in proportion to the streams pushed to it, and the
//!   peers it pushes nothing to at the minimum: pushes leave at the rate
//!   each holder needs, and pull answers take what they leave.
//!
//! # The seed's view
//!
//! [`SeedAllocationStats`] gives, per neighbour, its HAVE lag per stream
//! after publishing. A seed samples each stream once per signature unit, at
//! the unit's last chunk of it: a unit is published at once, so that chunk
//! tells when the neighbour had the unit's share of the stream whole.
//!
//! # Bounds
//!
//! Per channel a fixed record. A check costs O(channels × log
//! channels) plus the subscribers; an admission O(channels).

use std::collections::BTreeMap;

use strims_base::time::{Duration, Time};

use super::PushChannel;
use crate::codec::Stream;
use crate::controller::RequestController;
use crate::options::{SeedAllocation, SeedQos};
use crate::runner::{MAX_PEER_QOS_WEIGHT, MIN_PEER_QOS_WEIGHT, PeerKey};
use crate::scheduler::PeerTaskQueue;
use crate::scheduler_peer::PeerSwarmScheduler;
use crate::tap::TapKind;

#[cfg(doc)]
use crate::SeedPolicy;

/// How often an adaptive seed re-evaluates its allocation.
pub const ALLOC_CHECK_INTERVAL: Duration = Duration::SECOND;
/// The share of the measured upload capacity the budget spends on pushes.
pub const ALLOC_UTILIZATION: f64 = 0.7;
/// How long the store rate is measured before the budget follows it.
pub const ALLOC_MEASURE: Duration = Duration::from_secs(2);
/// The copies before the budget is measured.
pub const UNMEASURED_COPIES: usize = 4;
/// How far the budget must fall before it is lowered.
pub const OVER_BUDGET: f64 = 0.1;
/// How much nearer (or a tenth, if more) a neighbour must be to displace
/// a holder.
pub const NEAR_MARGIN: Duration = Duration::from_millis(5);
/// A round trip is the smallest over the current window of this length
/// and the one before.
pub const RTT_WINDOW: Duration = Duration::from_secs(10);

/// A seed's record of one neighbour.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Holder {
    /// It holds a copy.
    holds: bool,
    /// The smallest ping round trip in the current [`RTT_WINDOW`] (its
    /// index) and in the one before.
    window: i64,
    rtt: Option<Duration>,
    rtt_before: Option<Duration>,
}

impl Holder {
    /// A ping round trip.
    fn ping(&mut self, rtt: Duration, now: Time) {
        let w = now.unix_nanos() / RTT_WINDOW.as_nanos();
        if w != self.window {
            self.rtt_before = if w == self.window.saturating_add(1) {
                self.rtt
            } else {
                None
            };
            self.rtt = None;
            self.window = w;
        }
        self.rtt = Some(self.rtt.map_or(rtt, |r| r.min(rtt)));
    }

    /// The smallest round trip over the current window and the one before.
    fn min_rtt(&self, now: Time) -> Option<Duration> {
        let w = now.unix_nanos() / RTT_WINDOW.as_nanos();
        if w == self.window {
            [self.rtt, self.rtt_before].into_iter().flatten().min()
        } else if w == self.window.saturating_add(1) {
            self.rtt
        } else {
            None
        }
    }
}

/// An adaptive seed's state; see the module docs.
#[derive(Debug)]
pub(crate) struct Allocator {
    next_check: Time,
    /// When the store rate was first seen (`NIL` before).
    measuring_since: Time,
    /// The budget in stream slots, once measured; the copies it pays for,
    /// and the streams a holder may take (fewer than all only under one
    /// copy).
    budget: usize,
    copies: usize,
    streams: usize,
    /// Not measured yet, or room for every neighbour's every stream.
    unconstrained: bool,
    stats: AllocCounters,
}

impl Allocator {
    fn new(now: Time) -> Allocator {
        Allocator {
            next_check: now,
            measuring_since: Time::NIL,
            budget: 0,
            copies: UNMEASURED_COPIES,
            streams: usize::MAX,
            unconstrained: false,
            stats: AllocCounters::default(),
        }
    }
}

/// An adaptive seed's cumulative counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocCounters {
    /// Requests accepted, and declined.
    pub accepted: u64,
    pub declined: u64,
    /// Neighbours that became holders, and stopped being holders.
    pub joined: u64,
    pub left: u64,
}

/// A seed's view of its neighbours, for diagnostics and tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeedAllocationStats {
    pub neighbours: Vec<SeedNeighbour>,
    /// An adaptive seed's latest check (zero otherwise): the budget in
    /// stream slots, the copies it pays for, and the counters.
    pub budget: usize,
    pub copies: usize,
    pub counters: AllocCounters,
}

/// One neighbour of a seed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedNeighbour {
    pub peer: PeerKey,
    /// The streams pushed to it.
    pub streams: Vec<Stream>,
    /// Per stream, its mean HAVE lag after publishing, if it announced the
    /// stream within [`CANDIDATE_FRESHNESS`](super::CANDIDATE_FRESHNESS).
    pub lags: Vec<Option<Duration>>,
    /// The channel's round trip, and its smallest ping round trip lately.
    pub rtt: Duration,
    pub min_rtt: Option<Duration>,
    /// An adaptive seed's holder.
    pub holder: bool,
}

/// A ping round trip to the channel's peer, at a seed.
pub(crate) fn ping(pc: &mut Option<PushChannel>, rtt: Duration, now: Time) {
    if let Some(pc) = pc.as_mut() {
        pc.holder.ping(rtt, now);
    }
}

impl PeerSwarmScheduler {
    /// A seed allocating by its own measurements: no pin.
    pub(crate) fn adaptive_seed(&self) -> bool {
        self.s.is_seed
            && self.s.seed.allocation != SeedAllocation::Admission
            && self.s.push.as_ref().is_some_and(|p| p.pin.is_none())
    }

    /// An adaptive seed's admission of `peer`'s new request for a stream.
    pub(crate) fn grant(&mut self, peer: PeerKey, now: Time) -> bool {
        let holders = self.holders();
        let held = self
            .s
            .streams
            .iter()
            .filter(|st| st.subscribers.iter().any(|x| x.channel == peer))
            .count();
        let Some(h) = self
            .channels
            .get_mut(&peer)
            .and_then(|c| c.push.as_mut())
            .map(|pc| &mut pc.holder)
        else {
            return false;
        };
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        let a = p.alloc.get_or_insert_with(|| Allocator::new(now));
        if !h.holds && (a.unconstrained || holders < a.copies) {
            h.holds = true;
            a.stats.joined = a.stats.joined.saturating_add(1);
            self.s
                .tap
                .emit(self.s.key, Some(peer), || TapKind::HolderJoined);
        }
        let ok = h.holds && held < a.streams;
        let n = if ok {
            &mut a.stats.accepted
        } else {
            &mut a.stats.declined
        };
        *n = n.saturating_add(1);
        ok
    }

    /// The neighbours holding a copy.
    fn holders(&self) -> usize {
        self.channels
            .values()
            .filter(|c| c.push.as_ref().is_some_and(|pc| pc.holder.holds))
            .count()
    }

    /// An adaptive seed's check: the budget, then the holders.
    pub(crate) fn check_allocation(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        if !self.adaptive_seed() {
            return;
        }
        let rate = self.s.stream_rate(now);
        let streams = usize::from(self.s.stream_count);
        let neighbours = self
            .channels
            .values()
            .filter(|c| c.handshake_received)
            .count();
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        let upload = p.upload;
        let a = p.alloc.get_or_insert_with(|| Allocator::new(now));
        if now < a.next_check {
            return;
        }
        a.next_check = now + ALLOC_CHECK_INTERVAL;

        if rate > 0.0 && a.measuring_since.is_nil() {
            a.measuring_since = now;
        }
        let measured = !a.measuring_since.is_nil() && now - a.measuring_since >= ALLOC_MEASURE;
        let Some(cap) = upload.capacity().filter(|_| measured && rate > 0.0) else {
            return;
        };
        let target = (cap * ALLOC_UTILIZATION / rate) as usize;
        if a.budget == 0
            || target > a.budget
            || (target as f64) < a.budget as f64 * (1.0 - OVER_BUDGET)
        {
            a.budget = target;
        }
        let before = (a.budget, a.copies, a.streams);
        a.copies = (a.budget / streams.max(1)).max(1);
        a.streams = a.budget.clamp(1, streams);
        a.unconstrained = a.budget >= neighbours.saturating_mul(streams);
        let (copies, room) = (a.copies, a.streams);
        if before != (a.budget, a.copies, a.streams) {
            let budget = a.budget as u64;
            self.s.tap.emit(self.s.key, None, || TapKind::SeedBudget {
                budget,
                copies: copies as u64,
                streams: room as u64,
            });
        }
        if a.unconstrained {
            return;
        }

        // Under one copy, a holder's streams past the budget go.
        let mut excess: Vec<(PeerKey, Stream)> = Vec::new();
        if room < streams {
            let mut seen: BTreeMap<PeerKey, usize> = BTreeMap::new();
            for (i, st) in self.s.streams.iter().enumerate() {
                for x in &st.subscribers {
                    let n = seen.entry(x.channel).or_default();
                    *n += 1;
                    if *n > room {
                        excess.push((x.channel, i as Stream));
                    }
                }
            }
        }
        for (peer, stream) in excess {
            self.close_subscriber(peer, stream, q);
            let key = self.s.key;
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::Shed { stream });
        }

        // Holders by round trip, unmeasured ones last; and the nearest
        // neighbour that isn't one.
        let mut holders: Vec<(bool, Option<Duration>, PeerKey)> = Vec::new();
        let mut nearest: Option<(Duration, PeerKey)> = None;
        for (&peer, c) in &self.channels {
            let Some(h) = c.push.as_ref().map(|pc| pc.holder) else {
                continue;
            };
            let rtt = h.min_rtt(now);
            if h.holds {
                holders.push((rtt.is_none(), rtt, peer));
            } else if c.handshake_received
                && let Some(r) = rtt
                && nearest.is_none_or(|(n, _)| r < n)
            {
                nearest = Some((r, peer));
            }
        }
        holders.sort();
        let mut leave: Vec<PeerKey> = holders.iter().skip(copies).map(|h| h.2).collect();
        let mut join = None;
        if let Some((r, peer)) = nearest {
            if holders.len() < copies {
                join = Some(peer);
            } else if holders.len() == copies
                && let Some(&(_, far, holder)) = holders.last()
                && far.is_none_or(|f| r + NEAR_MARGIN.max(f / 10) < f)
            {
                leave.push(holder);
                join = Some(peer);
            }
        }
        for &peer in &leave {
            self.leave(peer, q);
        }
        if let Some(peer) = join
            && let Some(h) = self
                .channels
                .get_mut(&peer)
                .and_then(|c| c.push.as_mut())
                .map(|pc| &mut pc.holder)
        {
            h.holds = true;
            let key = self.s.key;
            self.s.tap.emit(key, Some(peer), || TapKind::HolderJoined);
        }
        if let Some(a) = self.s.push.as_mut().and_then(|p| p.alloc.as_mut()) {
            a.stats.left = a.stats.left.saturating_add(leave.len() as u64);
            a.stats.joined = a.stats.joined.saturating_add(u64::from(join.is_some()));
        }
    }

    /// `peer` stops being a holder: every subscription it has goes.
    fn leave(&mut self, peer: PeerKey, q: &mut impl PeerTaskQueue) {
        if let Some(h) = self
            .channels
            .get_mut(&peer)
            .and_then(|c| c.push.as_mut())
            .map(|pc| &mut pc.holder)
        {
            h.holds = false;
        }
        let key = self.s.key;
        self.s.tap.emit(key, Some(peer), || TapKind::HolderLeft);
        let streams: Vec<Stream> = self
            .s
            .streams
            .iter()
            .enumerate()
            .filter(|(_, st)| st.subscribers.iter().any(|x| x.channel == peer))
            .map(|(i, _)| i as Stream)
            .collect();
        for s in streams {
            self.close_subscriber(peer, s, q);
        }
    }

    /// Fixes the peers a seed pushes each stream to, one list per stream
    /// (`None` lifts the pin): subscribers outside it are closed. False if
    /// this isn't a hybrid seed.
    pub(crate) fn pin_allocation(
        &mut self,
        pin: Option<Vec<Vec<PeerKey>>>,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        if !self.s.is_seed {
            return false;
        }
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        p.pin = pin;
        let Some(pin) = p.pin.as_ref() else {
            return true;
        };
        let mut outside = Vec::new();
        for (i, st) in self.s.streams.iter().enumerate() {
            for sub in &st.subscribers {
                if !pin.get(i).is_some_and(|v| v.contains(&sub.channel)) {
                    outside.push((sub.channel, i as Stream));
                }
            }
        }
        for (peer, stream) in outside {
            self.close_subscriber(peer, stream, q);
        }
        true
    }

    /// A seed under [`SeedQos::ByAllocation`] raises each channel's weight
    /// in `weights` to its share of the streams pushed to it (at least the
    /// minimum). False, touching nothing, for any other swarm.
    pub(crate) fn seed_weights(&self, weights: &mut BTreeMap<PeerKey, u64>) -> bool {
        let s = &self.s;
        if !s.is_seed || s.push.is_none() || s.seed.qos != SeedQos::ByAllocation {
            return false;
        }
        let mut held: BTreeMap<PeerKey, u64> = BTreeMap::new();
        for st in &s.streams {
            for sub in &st.subscribers {
                *held.entry(sub.channel).or_default() += 1;
            }
        }
        let n = u64::from(s.stream_count.max(1));
        for &peer in self.channels.keys() {
            let h = held.get(&peer).copied().unwrap_or(0);
            let w = (MAX_PEER_QOS_WEIGHT * h / n).max(MIN_PEER_QOS_WEIGHT);
            let e = weights.entry(peer).or_insert(0);
            *e = (*e).max(w);
        }
        true
    }

    /// A seed's view of its neighbours; `None` for any other swarm.
    pub(crate) fn seed_allocation(&self, now: Time) -> Option<SeedAllocationStats> {
        let s = &self.s;
        if !s.is_seed {
            return None;
        }
        let p = s.push.as_ref()?;
        let neighbours = self
            .channels
            .iter()
            .map(|(&peer, c)| SeedNeighbour {
                peer,
                streams: s
                    .streams
                    .iter()
                    .enumerate()
                    .filter(|(_, st)| st.subscribers.iter().any(|x| x.channel == peer))
                    .map(|(i, _)| i as Stream)
                    .collect(),
                lags: c.push.as_ref().map_or_else(Vec::new, |pc| {
                    pc.have_lag
                        .iter()
                        .map(|l| l.fresh(now).then(|| l.mean()))
                        .collect()
                }),
                rtt: c.controller.rtt(),
                min_rtt: c.push.as_ref().and_then(|pc| pc.holder.min_rtt(now)),
                holder: c.push.as_ref().is_some_and(|pc| pc.holder.holds),
            })
            .collect();
        let mut out = SeedAllocationStats {
            neighbours,
            ..Default::default()
        };
        if let Some(a) = p.alloc.as_ref() {
            out.budget = a.budget;
            out.copies = a.copies;
            out.counters = a.stats;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_is_the_windowed_minimum() {
        let t = |s: i64| Time::from_unix_millis(1_000_000_000) + Duration::from_secs(s);
        let ms = Duration::from_millis;
        let mut h = Holder::default();
        assert_eq!(h.min_rtt(t(0)), None);
        h.ping(ms(50), t(0));
        h.ping(ms(30), t(1));
        h.ping(ms(80), t(2));
        assert_eq!(h.min_rtt(t(2)), Some(ms(30)));
        // The window before still counts ...
        h.ping(ms(60), t(10));
        assert_eq!(h.min_rtt(t(11)), Some(ms(30)));
        // ... and the one before that no longer does.
        h.ping(ms(70), t(20));
        assert_eq!(h.min_rtt(t(21)), Some(ms(60)));
        assert_eq!(h.min_rtt(t(31)), Some(ms(70)));
        assert_eq!(h.min_rtt(t(45)), None);
    }
}
