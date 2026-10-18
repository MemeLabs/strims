// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The video swarms on this node, by transfer id.
//!
//! The video control makes every video swarm the node runs: a capture
//! creates and writes one, and egress adds one to read. Each is the
//! transfer of its swarm id under the empty salt, so a transfer id names
//! at most one of them, and a stream of a swarm the node already runs
//! reads that swarm.
//!
//! An entry is a slot: it lasts from its insert to its removal. A capture
//! removes its slot when it closes, whatever streams read it; an egress
//! slot goes with its last stream. A slot's swarm can change while the
//! slot lasts (`swarm` is `None` while one is being added), and each swarm
//! it holds has its own generation. Slots and generations take their ids
//! from one counter, starting at 1, so they never repeat.
//!
//! A stream holds a [`Claim`]: its slot, and the generation of the swarm
//! it reads. When that swarm closes, the stream reads the slot's newer
//! swarm, or a newer slot's, once there is one
//! ([`Registry::poll_swarm`]).

use std::collections::{BTreeMap, BTreeSet};
use std::task::{Context, Poll, Waker};

use strims_ppspp::{SwarmId, SwarmOptions, Uri};
use strims_transfer::TransferId;

/// What made a video swarm on this node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SwarmKind {
    /// A browser capture: it writes the swarm, and removes it when it
    /// closes.
    Capture,
    /// Streams: added to read, and removed with the last of them.
    Egress,
}

/// A video swarm on this node, as
/// [`VideoControl::video_swarms`](crate::VideoControl::video_swarms) lists
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoSwarm {
    /// Its transfer: its swarm id under the empty salt.
    pub transfer_id: TransferId,
    pub swarm_uri: String,
    pub kind: SwarmKind,
    /// The node has the swarm; it is being added otherwise.
    pub added: bool,
    /// Streams reading it.
    pub streams: usize,
    /// The keys of the networks its transfer is published on, each once
    /// joined.
    pub network_keys: Vec<Vec<u8>>,
}

/// A stream's hold on a slot: the slot, and the generation of the swarm
/// the stream reads there, 0 before it reads one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Claim {
    pub(crate) slot: u64,
    pub(crate) generation: u64,
}

/// A video swarm on this node.
pub(crate) struct LocalSwarm<S> {
    /// The node's handle, once the swarm is added. Its transfer exists
    /// while it is set.
    pub(crate) swarm: Option<S>,
    pub(crate) id: SwarmId,
    /// Its options, with the defaults filled in.
    pub(crate) options: SwarmOptions,
    pub(crate) kind: SwarmKind,
    /// Streams holding the slot.
    pub(crate) refs: usize,
    /// The keys of the networks the transfer is published on, each once
    /// the node joins it. Bound: a capture's network keys; for an egress
    /// slot, the networks the node runs.
    pub(crate) networks: BTreeSet<Vec<u8>>,
    pub(crate) slot: u64,
    pub(crate) generation: u64,
    /// Tasks waiting for the slot's swarm to change. Bound: one per task,
    /// and the tasks are streams.
    waiters: Vec<Waker>,
}

impl<S> LocalSwarm<S> {
    fn wait(&mut self, cx: &Context<'_>) {
        if !self.waiters.iter().any(|w| w.will_wake(cx.waker())) {
            self.waiters.push(cx.waker().clone());
        }
    }
}

/// What became of an added swarm ([`Registry::set_swarm`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Added {
    /// The slot holds it: add its transfer.
    Kept,
    /// Its slot is gone, or no stream holds it any more and it went now:
    /// remove the swarm.
    Unwanted,
}

/// The node's video swarms; see the [module docs](self).
pub(crate) struct Registry<S> {
    /// Bound: the egress slots
    /// ([`EgressSettings::max_swarms`](crate::EgressSettings::max_swarms))
    /// and the captures
    /// ([`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures)).
    swarms: BTreeMap<TransferId, LocalSwarm<S>>,
    /// Streams open, of any swarm.
    pub(crate) streams: usize,
    /// Captures being created, which their bound counts.
    pub(crate) opening_captures: usize,
    next_id: u64,
    /// Waiters of slots that changed, for the owner to wake once it lets
    /// go of the registry.
    woken: Vec<Waker>,
}

impl<S> Default for Registry<S> {
    fn default() -> Self {
        Registry {
            swarms: BTreeMap::new(),
            streams: 0,
            opening_captures: 0,
            next_id: 1,
            woken: Vec::new(),
        }
    }
}

impl<S: Copy> Registry<S> {
    /// The waiters of the slots that changed since the last call.
    pub(crate) fn take_woken(&mut self) -> Vec<Waker> {
        std::mem::take(&mut self.woken)
    }

    fn wake(woken: &mut Vec<Waker>, e: &mut LocalSwarm<S>) {
        woken.append(&mut e.waiters);
    }

    pub(crate) fn get(&self, t: &TransferId) -> Option<&LocalSwarm<S>> {
        self.swarms.get(t)
    }

    pub(crate) fn get_mut(&mut self, t: &TransferId) -> Option<&mut LocalSwarm<S>> {
        self.swarms.get_mut(t)
    }

    /// The transfer ids, in order.
    pub(crate) fn ids(&self) -> Vec<TransferId> {
        self.swarms.keys().copied().collect()
    }

    /// Every swarm, in transfer id order.
    pub(crate) fn list(&self) -> Vec<VideoSwarm> {
        self.swarms
            .iter()
            .map(|(t, e)| VideoSwarm {
                transfer_id: *t,
                swarm_uri: Uri::new(e.id.clone(), e.options.uri_options()).to_string(),
                kind: e.kind,
                added: e.swarm.is_some(),
                streams: e.refs,
                network_keys: e.networks.iter().cloned().collect(),
            })
            .collect()
    }

    /// The slots of `kind`.
    pub(crate) fn count(&self, kind: SwarmKind) -> usize {
        self.swarms.values().filter(|s| s.kind == kind).count()
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Makes a slot for `t`, with `refs` streams holding it, in place of
    /// none: the claim of a stream holding it, before the stream reads.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn insert(
        &mut self,
        t: TransferId,
        swarm: Option<S>,
        id: SwarmId,
        options: SwarmOptions,
        kind: SwarmKind,
        networks: BTreeSet<Vec<u8>>,
        refs: usize,
    ) -> Claim {
        let slot = self.next_id();
        let generation = self.next_id();
        let prev = self.swarms.insert(
            t,
            LocalSwarm {
                swarm,
                id,
                options,
                kind,
                refs,
                networks,
                slot,
                generation,
                waiters: Vec::new(),
            },
        );
        if let Some(mut prev) = prev {
            Self::wake(&mut self.woken, &mut prev);
        }
        Claim {
            slot,
            generation: 0,
        }
    }

    /// Has one more stream hold the slot of `t`: its claim, before it
    /// reads.
    pub(crate) fn claim(&mut self, t: &TransferId) -> Option<Claim> {
        let e = self.swarms.get_mut(t)?;
        e.refs += 1;
        Some(Claim {
            slot: e.slot,
            generation: 0,
        })
    }

    /// Releases a stream's claim. Returns the egress slot the last of its
    /// streams released, which is gone from the registry. A slot whose
    /// swarm is being added stays until the add ends
    /// ([`set_swarm`](Self::set_swarm)), so adds in flight are bounded by
    /// the slots.
    pub(crate) fn release(&mut self, t: &TransferId, claim: Claim) -> Option<LocalSwarm<S>> {
        let e = self.swarms.get_mut(t)?;
        if e.slot != claim.slot {
            return None;
        }
        e.refs = e.refs.saturating_sub(1);
        if e.kind == SwarmKind::Egress && e.refs == 0 && e.swarm.is_some() {
            return self.remove(t);
        }
        None
    }

    /// Removes the slot of `t`.
    pub(crate) fn remove(&mut self, t: &TransferId) -> Option<LocalSwarm<S>> {
        let mut e = self.swarms.remove(t)?;
        Self::wake(&mut self.woken, &mut e);
        Some(e)
    }

    /// Removes the slot of `t` if it is `slot`.
    pub(crate) fn remove_slot(&mut self, t: &TransferId, slot: u64) -> Option<LocalSwarm<S>> {
        if self.swarms.get(t)?.slot != slot {
            return None;
        }
        self.remove(t)
    }

    /// Sets the swarm of slot `slot` of `t`, which has none, as a new
    /// generation; an egress slot no stream holds goes instead.
    pub(crate) fn set_swarm(&mut self, t: &TransferId, slot: u64, swarm: S) -> Added {
        let generation = self.next_id();
        let Some(e) = self.swarms.get_mut(t) else {
            return Added::Unwanted;
        };
        if e.slot != slot || e.swarm.is_some() {
            return Added::Unwanted;
        }
        if e.kind == SwarmKind::Egress && e.refs == 0 {
            self.remove(t);
            return Added::Unwanted;
        }
        e.swarm = Some(swarm);
        e.generation = generation;
        Self::wake(&mut self.woken, e);
        Added::Kept
    }

    /// The swarm of `t` for a stream with `claim` to read: one of another
    /// generation than the claim's. Waits while the slot has none. A
    /// stream whose slot went takes a claim on the slot that replaced it.
    /// `None` when there is no slot, or the swarm is the one the claim
    /// read: that one closed for good.
    pub(crate) fn poll_swarm(
        &mut self,
        t: &TransferId,
        claim: &mut Claim,
        cx: &Context<'_>,
    ) -> Poll<Option<S>> {
        let Some(e) = self.swarms.get_mut(t) else {
            return Poll::Ready(None);
        };
        if e.slot != claim.slot {
            e.refs += 1;
            *claim = Claim {
                slot: e.slot,
                generation: 0,
            };
        }
        if e.generation == claim.generation {
            return Poll::Ready(None);
        }
        match e.swarm {
            Some(s) => {
                claim.generation = e.generation;
                Poll::Ready(Some(s))
            }
            None => {
                e.wait(cx);
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::task::Waker;

    use super::*;

    fn tid(n: u8) -> TransferId {
        TransferId::new(&SwarmId::new(&[n; 32]), b"")
    }

    fn insert(r: &mut Registry<u32>, n: u8, swarm: Option<u32>, kind: SwarmKind) -> Claim {
        r.insert(
            tid(n),
            swarm,
            SwarmId::new(&[n; 32]),
            SwarmOptions::default(),
            kind,
            BTreeSet::new(),
            1,
        )
    }

    fn poll(r: &mut Registry<u32>, n: u8, c: &mut Claim) -> Poll<Option<u32>> {
        r.poll_swarm(&tid(n), c, &Context::from_waker(Waker::noop()))
    }

    #[test]
    fn the_last_stream_removes_an_egress_slot_only() {
        let mut r = Registry::default();
        let a = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        let b = r.claim(&tid(1)).unwrap();
        assert!(r.release(&tid(1), a).is_none());
        assert_eq!(r.release(&tid(1), b).unwrap().swarm, Some(7));
        assert!(r.get(&tid(1)).is_none());

        let c = insert(&mut r, 2, Some(8), SwarmKind::Capture);
        assert!(r.release(&tid(2), c).is_none());
        assert_eq!(r.get(&tid(2)).unwrap().refs, 0);
    }

    #[test]
    fn a_claim_on_a_removed_slot_releases_nothing() {
        let mut r = Registry::default();
        let old = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        r.remove(&tid(1));
        let new = insert(&mut r, 1, Some(9), SwarmKind::Egress);
        assert!(r.release(&tid(1), old).is_none());
        assert_eq!(r.get(&tid(1)).unwrap().refs, 1);
        assert!(r.release(&tid(1), new).is_some());
    }

    #[test]
    fn a_stream_waits_for_its_slots_swarm_then_reads_only_newer_ones() {
        let mut r = Registry::default();
        let mut c = insert(&mut r, 1, None, SwarmKind::Egress);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Pending);
        assert_eq!(r.set_swarm(&tid(1), c.slot, 7), Added::Kept);
        assert_eq!(r.take_woken().len(), 1);
        assert_eq!(r.set_swarm(&tid(1), c.slot, 8), Added::Unwanted);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(Some(7)));
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(None));
    }

    #[test]
    fn a_slot_released_while_its_swarm_is_added_goes_when_the_add_ends() {
        let mut r = Registry::default();
        let c = insert(&mut r, 1, None, SwarmKind::Egress);
        assert!(r.release(&tid(1), c).is_none());
        assert_eq!(r.count(SwarmKind::Egress), 1);
        let again = r.claim(&tid(1)).unwrap();
        assert!(r.release(&tid(1), again).is_none());
        assert_eq!(r.set_swarm(&tid(1), c.slot, 7), Added::Unwanted);
        assert!(r.get(&tid(1)).is_none());
    }

    #[test]
    fn a_stream_moves_its_claim_to_the_slot_that_replaced_its_own() {
        let mut r = Registry::default();
        let mut c = insert(&mut r, 1, Some(7), SwarmKind::Capture);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(Some(7)));
        r.remove(&tid(1));
        insert(&mut r, 1, Some(9), SwarmKind::Egress);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(Some(9)));
        assert_eq!(r.get(&tid(1)).unwrap().refs, 2);
        assert!(r.release(&tid(1), c).is_none());
        assert_eq!(r.get(&tid(1)).unwrap().refs, 1);
    }

    #[test]
    fn a_failed_add_removes_only_its_own_slot() {
        let mut r = Registry::default();
        let c = insert(&mut r, 1, None, SwarmKind::Egress);
        assert!(r.remove_slot(&tid(1), c.slot + 100).is_none());
        assert!(r.remove_slot(&tid(1), c.slot).is_some());
        assert_eq!(r.count(SwarmKind::Egress), 0);
    }
}
