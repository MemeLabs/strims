// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The video swarms on this node, by transfer id.
//!
//! The video control makes every video swarm the node runs: a capture
//! or an ingest session creates and writes one, egress adds one to read,
//! and autoseed one to seed. Each is the transfer of its swarm id under
//! the empty salt, so a transfer id names at most one of them, and a
//! stream of a swarm the node already runs reads that swarm.
//!
//! An entry is a slot: it lasts from its insert to its removal. A capture
//! or an ingest session removes its slot when it ends, whatever streams
//! read it; an egress slot goes with its last stream. Autoseed holds
//! seeds on egress slots and slots of its own ([`Registry::seed`]): a
//! seeded slot is a seed slot, which no stream's release removes. The
//! end of its last seed ([`Registry::unseed`]) makes it an egress slot
//! again, which goes at once if no stream holds it. A slot's swarm can
//! change while the slot lasts (`swarm` is `None` while one is being
//! added): an ingest session that takes over a slot replaces its swarm
//! ([`Registry::replace`]), and the slot keeps its streams but not its
//! seeds. Each swarm a slot holds has its own generation. Slots and
//! generations take their ids from one counter, starting at 1, so they
//! never repeat.
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
    /// An RTMP ingest session: it writes the swarm of its channel's key,
    /// and removes it when it ends.
    Ingest,
    /// Autoseed rules: added to seed, or a stream's swarm they seed. It
    /// goes with the last of their seeds once no stream reads it.
    Seed,
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
    /// Autoseed rules seeding it.
    pub seeds: usize,
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
    /// Autoseed rules seeding the slot: some for a seed slot, none for
    /// any other. Bound: the rules seeding at once
    /// ([`AutoseedSettings::max_seeds`](crate::AutoseedSettings::max_seeds)).
    pub(crate) seeds: usize,
    /// The keys of the networks the transfer is published on, each once
    /// the node joins it. Bound: a capture's network keys; an ingest
    /// session's one; for an egress or seed slot, the networks the node
    /// runs.
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

    /// An egress slot that no stream or seed holds, which goes once its
    /// swarm is added.
    fn unheld(&self) -> bool {
        self.kind == SwarmKind::Egress && self.refs == 0 && self.seeds == 0
    }
}

/// What became of an added swarm ([`Registry::set_swarm`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Added {
    /// The slot holds it: add its transfer.
    Kept,
    /// Its slot is gone, or no stream or seed holds it any more and it
    /// went now: remove the swarm.
    Unwanted,
}

/// What became of a seed ([`Registry::seed`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seeded {
    /// The slot there was, slot `.0`, holds it.
    Held(u64),
    /// New seed slot `.0` holds it: add its swarm.
    Added(u64),
    /// The swarm is a capture's or an ingest session's: nothing holds the
    /// seed.
    Refused,
}

/// The node's video swarms; see the [module docs](self).
pub(crate) struct Registry<S> {
    /// Bound: the egress slots
    /// ([`EgressSettings::max_swarms`](crate::EgressSettings::max_swarms)),
    /// the seed slots and the egress slots their seeds left while their
    /// swarms were added (a swarm per rule,
    /// [`AutoseedSettings::max_rules`](crate::AutoseedSettings::max_rules)),
    /// the captures
    /// ([`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures))
    /// and the ingest sessions
    /// ([`IngressSettings::max_sessions`](crate::IngressSettings::max_sessions)).
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
                seeds: e.seeds,
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
                seeds: 0,
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
    /// the slots. A seed slot stays.
    pub(crate) fn release(&mut self, t: &TransferId, claim: Claim) -> Option<LocalSwarm<S>> {
        let e = self.swarms.get_mut(t)?;
        if e.slot != claim.slot {
            return None;
        }
        e.refs = e.refs.saturating_sub(1);
        if e.unheld() && e.swarm.is_some() {
            return self.remove(t);
        }
        None
    }

    /// Has one more seed, on network `network`, hold the slot of `t`: an
    /// egress or seed slot becomes or stays a seed slot, and its transfer
    /// goes on `network` too. With no slot, a new seed slot of the swarm
    /// `id` with `options` holds it. A capture's or an ingest session's
    /// slot takes no seed.
    pub(crate) fn seed(
        &mut self,
        t: TransferId,
        network: Vec<u8>,
        id: SwarmId,
        options: SwarmOptions,
    ) -> Seeded {
        if let Some(e) = self.swarms.get_mut(&t) {
            if !matches!(e.kind, SwarmKind::Egress | SwarmKind::Seed) {
                return Seeded::Refused;
            }
            e.kind = SwarmKind::Seed;
            e.seeds += 1;
            e.networks.insert(network);
            return Seeded::Held(e.slot);
        }
        let networks = BTreeSet::from([network]);
        let claim = self.insert(t, None, id, options, SwarmKind::Seed, networks, 0);
        if let Some(e) = self.swarms.get_mut(&t) {
            e.seeds = 1;
        }
        Seeded::Added(claim.slot)
    }

    /// Ends a seed slot `slot` of `t` holds. The slot's last seed makes
    /// it an egress slot again, which goes as the last stream's
    /// [`release`](Self::release) has it; it is returned if it went. A
    /// seed of a removed slot, or of a slot an ingest session took over,
    /// ended with it.
    pub(crate) fn unseed(&mut self, t: &TransferId, slot: u64) -> Option<LocalSwarm<S>> {
        let e = self.swarms.get_mut(t)?;
        if e.slot != slot || e.kind != SwarmKind::Seed {
            return None;
        }
        e.seeds = e.seeds.saturating_sub(1);
        if e.seeds > 0 {
            return None;
        }
        e.kind = SwarmKind::Egress;
        if e.unheld() && e.swarm.is_some() {
            return self.remove(t);
        }
        None
    }

    /// Whether slot `slot` of `t` is a seed slot.
    pub(crate) fn seeds_slot(&self, t: &TransferId, slot: u64) -> bool {
        self.swarms
            .get(t)
            .is_some_and(|e| e.slot == slot && e.kind == SwarmKind::Seed)
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

    /// Takes the swarm out of the slot of `t` for a swarm of `kind` to
    /// come: the slot keeps its streams, drops its seeds, takes `id`,
    /// `options` and `networks`, and gets a new generation without a
    /// swarm until [`set_swarm`](Self::set_swarm). Its streams read the
    /// next swarm once theirs closes. Returns the slot and the swarm taken out, for
    /// the caller to remove with its transfer; `None` without a slot.
    pub(crate) fn replace(
        &mut self,
        t: &TransferId,
        id: SwarmId,
        options: SwarmOptions,
        kind: SwarmKind,
        networks: BTreeSet<Vec<u8>>,
    ) -> Option<(u64, Option<S>)> {
        let generation = self.next_id();
        let e = self.swarms.get_mut(t)?;
        let old = e.swarm.take();
        e.id = id;
        e.options = options;
        e.kind = kind;
        e.seeds = 0;
        e.networks = networks;
        e.generation = generation;
        Self::wake(&mut self.woken, e);
        Some((e.slot, old))
    }

    /// Sets the swarm of slot `slot` of `t`, which has none, as a new
    /// generation; an egress slot no stream or seed holds goes instead.
    pub(crate) fn set_swarm(&mut self, t: &TransferId, slot: u64, swarm: S) -> Added {
        let generation = self.next_id();
        let Some(e) = self.swarms.get_mut(t) else {
            return Added::Unwanted;
        };
        if e.slot != slot || e.swarm.is_some() {
            return Added::Unwanted;
        }
        if e.unheld() {
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
    #![allow(clippy::unwrap_used, clippy::panic)]

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
    fn a_replaced_swarm_keeps_its_slot_and_streams_for_the_next() {
        let mut r = Registry::default();
        let mut c = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(Some(7)));
        let (slot, old) = r
            .replace(
                &tid(1),
                SwarmId::new(&[1; 32]),
                SwarmOptions::default(),
                SwarmKind::Ingest,
                BTreeSet::new(),
            )
            .unwrap();
        assert_eq!((slot, old), (c.slot, Some(7)));
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Pending);
        assert_eq!(r.set_swarm(&tid(1), slot, 8), Added::Kept);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(Some(8)));
        // The last stream leaves an ingest slot in place.
        assert!(r.release(&tid(1), c).is_none());
        assert_eq!(r.get(&tid(1)).unwrap().kind, SwarmKind::Ingest);
        assert!(
            r.replace(
                &tid(2),
                SwarmId::new(&[2; 32]),
                SwarmOptions::default(),
                SwarmKind::Ingest,
                BTreeSet::new()
            )
            .is_none()
        );
    }

    fn seed(r: &mut Registry<u32>, n: u8, network: u8) -> Seeded {
        r.seed(
            tid(n),
            vec![network],
            SwarmId::new(&[n; 32]),
            SwarmOptions::default(),
        )
    }

    #[test]
    fn a_seed_outlives_the_streams_on_its_slot() {
        let mut r = Registry::default();
        let c = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        assert_eq!(seed(&mut r, 1, 10), Seeded::Held(c.slot));
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds, e.refs), (SwarmKind::Seed, 1, 1));
        assert!(e.networks.contains(&vec![10]));
        assert!(r.release(&tid(1), c).is_none());
        assert_eq!(r.get(&tid(1)).unwrap().refs, 0);
        assert!(r.seeds_slot(&tid(1), c.slot));

        // A second seed holds the slot past the first's end.
        assert_eq!(seed(&mut r, 1, 11), Seeded::Held(c.slot));
        assert!(r.unseed(&tid(1), c.slot).is_none());
        assert_eq!(r.get(&tid(1)).unwrap().seeds, 1);
        assert_eq!(r.unseed(&tid(1), c.slot).unwrap().swarm, Some(7));
        assert!(r.get(&tid(1)).is_none());
    }

    #[test]
    fn unseeding_a_slot_with_streams_leaves_it_egress() {
        let mut r = Registry::default();
        let Seeded::Added(slot) = seed(&mut r, 1, 10) else {
            panic!("no seed slot");
        };
        assert_eq!(r.count(SwarmKind::Seed), 1);
        assert_eq!(r.set_swarm(&tid(1), slot, 7), Added::Kept);
        let c = r.claim(&tid(1)).unwrap();
        assert!(r.unseed(&tid(1), slot).is_none());
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds, e.refs), (SwarmKind::Egress, 0, 1));
        assert!(r.unseed(&tid(1), slot).is_none());
        assert_eq!(r.release(&tid(1), c).unwrap().swarm, Some(7));
    }

    #[test]
    fn a_seed_ended_while_its_swarm_is_added_goes_when_the_add_ends() {
        let mut r = Registry::default();
        let Seeded::Added(slot) = seed(&mut r, 1, 10) else {
            panic!("no seed slot");
        };
        assert!(r.unseed(&tid(1), slot).is_none());
        assert_eq!(r.count(SwarmKind::Egress), 1);
        // Seeding it again takes the slot whose add is in flight.
        assert_eq!(seed(&mut r, 1, 10), Seeded::Held(slot));
        assert!(r.unseed(&tid(1), slot).is_none());
        assert_eq!(r.set_swarm(&tid(1), slot, 7), Added::Unwanted);
        assert!(r.get(&tid(1)).is_none());
    }

    #[test]
    fn replace_clears_the_seeds() {
        let mut r = Registry::default();
        let Seeded::Added(slot) = seed(&mut r, 1, 10) else {
            panic!("no seed slot");
        };
        r.set_swarm(&tid(1), slot, 7);
        let c = r.claim(&tid(1)).unwrap();
        let (taken, old) = r
            .replace(
                &tid(1),
                SwarmId::new(&[1; 32]),
                SwarmOptions::default(),
                SwarmKind::Ingest,
                BTreeSet::new(),
            )
            .unwrap();
        assert_eq!((taken, old), (slot, Some(7)));
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds, e.refs), (SwarmKind::Ingest, 0, 1));
        assert!(!r.seeds_slot(&tid(1), slot));
        assert!(r.unseed(&tid(1), slot).is_none());
        assert_eq!(r.get(&tid(1)).unwrap().kind, SwarmKind::Ingest);
        assert!(r.release(&tid(1), c).is_none());
    }

    #[test]
    fn captures_and_ingest_sessions_take_no_seeds() {
        let mut r = Registry::default();
        insert(&mut r, 1, Some(7), SwarmKind::Capture);
        insert(&mut r, 2, Some(8), SwarmKind::Ingest);
        assert_eq!(seed(&mut r, 1, 10), Seeded::Refused);
        assert_eq!(seed(&mut r, 2, 10), Seeded::Refused);
        assert_eq!(r.get(&tid(1)).unwrap().seeds, 0);
        assert!(r.get(&tid(2)).unwrap().networks.is_empty());
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
