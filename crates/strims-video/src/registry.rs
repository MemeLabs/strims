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
//! seeds on any slot, and on slots of its own ([`Registry::seed`]): a
//! seeded egress slot is a seed slot, which no stream's release removes.
//! The end of its last seed ([`Registry::unseed`]) makes it an egress
//! slot again, which goes at once if no stream holds it. A capture's or
//! an ingest session's slot keeps its kind under seeds, which publish it
//! on their networks too. A slot's swarm can change while the slot lasts
//! (`swarm` is `None` while one is being added): an ingest session that
//! takes over a slot replaces its swarm ([`Registry::replace`]), and the
//! slot keeps its streams and seeds. An egress or seed slot whose swarm takes other options
//! ([`Registry::reopen`]) keeps its streams and seeds, and its next swarm
//! is one of those options: a swarm whose add began with others is not
//! taken ([`Registry::set_added`]). Each swarm a slot holds has its own
//! generation. Slots and generations take their ids from one counter,
//! starting at 1, so they never repeat.
//!
//! A stream holds a [`Claim`]: its slot, and the generation of the swarm
//! it reads. When that swarm closes, the stream reads the slot's newer
//! swarm, or a newer slot's, once there is one
//! ([`Registry::poll_swarm`]).
//!
//! A slot's transfer is published on its networks, each once the node
//! joins it: those of its writer, a capture's or an ingest session's, and
//! those its holders name. A stream of an egress or seed slot names the
//! networks it reads on, and a seed its rule's network. A holder's end
//! drops its names, and a network no one names any more, and that isn't
//! its writer's, is returned for the caller to withdraw the transfer from
//! ([`Released`]).

use std::collections::{BTreeMap, BTreeSet};
use std::task::{Context, Poll, Waker};

use strims_base::time::Time;
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

/// A stream's hold on a slot: the slot, the generation of the swarm the
/// stream reads there, 0 before it reads one, and whether the stream names
/// its networks on the slot, which it does on an egress or seed slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Claim {
    pub(crate) slot: u64,
    pub(crate) generation: u64,
    named: bool,
}

/// What a holder's end left: the slot, if it went with it, and otherwise
/// the networks the slot no longer publishes on.
pub(crate) struct Released<S> {
    /// The slot, gone from the registry, for the caller to take down.
    pub(crate) gone: Option<LocalSwarm<S>>,
    /// Networks no holder names any more, which aren't the writer's: the
    /// caller withdraws the transfer from those the node joined.
    pub(crate) unnamed: Vec<Vec<u8>>,
}

impl<S> Default for Released<S> {
    fn default() -> Self {
        Released {
            gone: None,
            unnamed: Vec::new(),
        }
    }
}

/// Counts one more holder naming each of `keys` in `names`.
fn name(names: &mut BTreeMap<Vec<u8>, usize>, keys: &[Vec<u8>]) {
    for k in keys {
        *names.entry(k.clone()).or_default() += 1;
    }
}

/// Counts one holder fewer naming each of `keys` in `names`: the keys no
/// holder names there any more.
fn unname(names: &mut BTreeMap<Vec<u8>, usize>, keys: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut dropped = Vec::new();
    for k in keys {
        if let Some(n) = names.get_mut(k) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                names.remove(k);
                dropped.push(k.clone());
            }
        }
    }
    dropped
}

/// Whether a stream names its networks on a slot of `kind`.
fn names_networks(kind: SwarmKind) -> bool {
    matches!(kind, SwarmKind::Egress | SwarmKind::Seed)
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
    /// The keys of the networks its writer publishes it on: a capture's
    /// network keys, an ingest session's one, and none for an egress or
    /// seed slot.
    pub(crate) own: BTreeSet<Vec<u8>>,
    /// The networks the streams holding the slot name, with how many name
    /// each. Bound:
    /// [`EgressSettings::max_network_keys`](crate::EgressSettings::max_network_keys)
    /// per stream.
    streamed: BTreeMap<Vec<u8>, usize>,
    /// The networks the seeds holding the slot name, with how many name
    /// each. Bound: one per seed.
    seeded: BTreeMap<Vec<u8>, usize>,
    pub(crate) slot: u64,
    pub(crate) generation: u64,
    /// When its swarm last took other options ([`Registry::reopen`]).
    pub(crate) reopened: Option<Time>,
    /// Tasks waiting for the slot's swarm to change. Bound: one per task,
    /// and the tasks are streams.
    waiters: Vec<Waker>,
}

/// Whether swarms of options `a` and `b` are one swarm: their URIs name
/// the same options. The live window takes no part.
pub(crate) fn same_options(a: &SwarmOptions, b: &SwarmOptions) -> bool {
    a.uri_options() == b.uri_options()
}

impl<S> LocalSwarm<S> {
    /// The keys of the networks the transfer is published on, each once
    /// the node joins it: its writer's and those its holders name.
    pub(crate) fn networks(&self) -> BTreeSet<Vec<u8>> {
        self.own
            .iter()
            .chain(self.streamed.keys())
            .chain(self.seeded.keys())
            .cloned()
            .collect()
    }

    /// Whether the transfer is published on the network `key`, once the
    /// node joins it.
    pub(crate) fn publishes_on(&self, key: &[u8]) -> bool {
        self.own.contains(key) || self.streamed.contains_key(key) || self.seeded.contains_key(key)
    }

    /// Of `keys`, those the slot no longer publishes on.
    fn unnamed(&self, keys: impl IntoIterator<Item = Vec<u8>>) -> Vec<Vec<u8>> {
        keys.into_iter().filter(|k| !self.publishes_on(k)).collect()
    }

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
pub(crate) enum Seeded<S> {
    /// The slot there was, slot `.0`, holds it.
    Held(u64),
    /// The slot there was, `slot`, holds it, and its swarm takes the
    /// seed's options ([`Registry::reopen`]): the swarm of other options
    /// it held, `old`, is the caller's to remove with its transfer, and
    /// the swarm of the seed's options its to add.
    Reopened { slot: u64, old: Option<S> },
    /// New seed slot `.0` holds it: add its swarm.
    Added(u64),
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
                network_keys: e.networks().into_iter().collect(),
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
    /// none, published on its writer's networks `networks`: the claim of a
    /// stream holding it, before the stream reads. Such a stream names no
    /// networks: [`claim`](Self::claim) has a stream name them.
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
                own: networks,
                streamed: BTreeMap::new(),
                seeded: BTreeMap::new(),
                slot,
                generation,
                reopened: None,
                waiters: Vec::new(),
            },
        );
        if let Some(mut prev) = prev {
            Self::wake(&mut self.woken, &mut prev);
        }
        Claim {
            slot,
            generation: 0,
            named: false,
        }
    }

    /// Has one more stream, reading on the networks `networks`, hold the
    /// slot of `t`: its claim, before it reads. On an egress or seed slot
    /// the stream names its networks.
    pub(crate) fn claim(&mut self, t: &TransferId, networks: &[Vec<u8>]) -> Option<Claim> {
        let e = self.swarms.get_mut(t)?;
        e.refs += 1;
        let named = names_networks(e.kind);
        if named {
            name(&mut e.streamed, networks);
        }
        Some(Claim {
            slot: e.slot,
            generation: 0,
            named,
        })
    }

    /// Releases the claim of a stream that read on the networks
    /// `networks`, as [`claim`](Self::claim) and
    /// [`poll_swarm`](Self::poll_swarm) were given them. The egress slot
    /// the last of its streams released is gone from the registry. A slot
    /// whose swarm is being added stays until the add ends
    /// ([`set_swarm`](Self::set_swarm)), so adds in flight are bounded by
    /// the slots. A seed slot stays.
    pub(crate) fn release(
        &mut self,
        t: &TransferId,
        claim: Claim,
        networks: &[Vec<u8>],
    ) -> Released<S> {
        let Some(e) = self.swarms.get_mut(t).filter(|e| e.slot == claim.slot) else {
            return Released::default();
        };
        e.refs = e.refs.saturating_sub(1);
        let dropped = if claim.named {
            unname(&mut e.streamed, networks)
        } else {
            Vec::new()
        };
        if e.unheld() && e.swarm.is_some() {
            return Released {
                gone: self.remove(t),
                unnamed: Vec::new(),
            };
        }
        Released {
            gone: None,
            unnamed: e.unnamed(dropped),
        }
    }

    /// Has one more seed, on network `network`, hold the slot of `t` at
    /// `now`, naming `network`: an egress or seed slot becomes or stays a
    /// seed slot, and its swarm takes the seed's `options` when it has
    /// others. A capture's or an ingest session's slot keeps its kind and
    /// its writer's swarm. With no slot, a new seed slot of the swarm `id`
    /// with `options` holds it.
    pub(crate) fn seed(
        &mut self,
        t: TransferId,
        network: Vec<u8>,
        id: SwarmId,
        options: SwarmOptions,
        now: Time,
    ) -> Seeded<S> {
        if let Some(e) = self.swarms.get_mut(&t) {
            e.seeds += 1;
            name(&mut e.seeded, std::slice::from_ref(&network));
            let slot = e.slot;
            if !names_networks(e.kind) {
                return Seeded::Held(slot);
            }
            e.kind = SwarmKind::Seed;
            if same_options(&e.options, &options) {
                return Seeded::Held(slot);
            }
            let old = self.reopen(&t, options, now).and_then(|(_, old)| old);
            return Seeded::Reopened { slot, old };
        }
        let claim = self.insert(t, None, id, options, SwarmKind::Seed, BTreeSet::new(), 0);
        if let Some(e) = self.swarms.get_mut(&t) {
            e.seeds = 1;
            name(&mut e.seeded, std::slice::from_ref(&network));
        }
        Seeded::Added(claim.slot)
    }

    /// Ends a seed on network `network` that slot `slot` of `t` holds. A
    /// seed slot's last seed makes it an egress slot again, which goes as
    /// the last stream's [`release`](Self::release) has it. A seed of a
    /// removed slot ended with it.
    pub(crate) fn unseed(&mut self, t: &TransferId, slot: u64, network: &[u8]) -> Released<S> {
        let Some(e) = self
            .swarms
            .get_mut(t)
            .filter(|e| e.slot == slot && e.seeds > 0)
        else {
            return Released::default();
        };
        e.seeds = e.seeds.saturating_sub(1);
        let dropped = unname(&mut e.seeded, &[network.to_vec()]);
        if e.seeds == 0 && e.kind == SwarmKind::Seed {
            e.kind = SwarmKind::Egress;
            if e.unheld() && e.swarm.is_some() {
                return Released {
                    gone: self.remove(t),
                    unnamed: Vec::new(),
                };
            }
        }
        Released {
            gone: None,
            unnamed: e.unnamed(dropped),
        }
    }

    /// Whether seeds hold slot `slot` of `t`.
    pub(crate) fn seeds_slot(&self, t: &TransferId, slot: u64) -> bool {
        self.swarms
            .get(t)
            .is_some_and(|e| e.slot == slot && e.seeds > 0)
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
    /// come, written by its new writer: the slot keeps its streams, its
    /// seeds and the networks they name, drops the networks its streams
    /// named, takes `id`, `options` and its writer's networks `networks`,
    /// and gets a new generation without a swarm until
    /// [`set_swarm`](Self::set_swarm). Its streams read the next swarm
    /// once theirs closes. Returns the slot and the swarm taken out, for
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
        e.own = networks;
        e.streamed.clear();
        e.generation = generation;
        Self::wake(&mut self.woken, e);
        Some((e.slot, old))
    }

    /// Takes the swarm out of the egress or seed slot of `t` at `now` for a
    /// swarm of `options` to come: the slot keeps its kind, streams, seeds
    /// and the networks they name, takes `options`, and gets a new
    /// generation without a swarm until
    /// [`set_added`](Self::set_added). Its streams read the next swarm
    /// once theirs closes. Returns the slot and the swarm taken out, for
    /// the caller to remove with its transfer before it adds the next;
    /// `None` without an egress or seed slot.
    pub(crate) fn reopen(
        &mut self,
        t: &TransferId,
        options: SwarmOptions,
        now: Time,
    ) -> Option<(u64, Option<S>)> {
        let generation = self.next_id();
        let e = self.swarms.get_mut(t).filter(|e| names_networks(e.kind))?;
        let old = e.swarm.take();
        e.options = options;
        e.generation = generation;
        e.reopened = Some(now);
        Self::wake(&mut self.woken, e);
        Some((e.slot, old))
    }

    /// [`set_swarm`](Self::set_swarm) of a swarm added with `options`: the
    /// slot doesn't take it once it has other options, which a
    /// [`reopen`](Self::reopen) during the add gave it.
    pub(crate) fn set_added(
        &mut self,
        t: &TransferId,
        slot: u64,
        options: &SwarmOptions,
        swarm: S,
    ) -> Added {
        if self
            .swarms
            .get(t)
            .is_some_and(|e| !same_options(&e.options, options))
        {
            return Added::Unwanted;
        }
        self.set_swarm(t, slot, swarm)
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

    /// The swarm of `t` for a stream with `claim`, reading on the networks
    /// `networks`, to read, with its chunk size: one of another generation
    /// than the claim's.
    /// Waits while the slot has none. A stream whose slot went takes a
    /// claim on the slot that replaced it, as [`claim`](Self::claim) has
    /// it. `None` when there is no slot, or the swarm is the one the claim
    /// read: that one closed for good.
    pub(crate) fn poll_swarm(
        &mut self,
        t: &TransferId,
        claim: &mut Claim,
        networks: &[Vec<u8>],
        cx: &Context<'_>,
    ) -> Poll<Option<(S, usize)>> {
        let Some(e) = self.swarms.get_mut(t) else {
            return Poll::Ready(None);
        };
        if e.slot != claim.slot {
            e.refs += 1;
            let named = names_networks(e.kind);
            if named {
                name(&mut e.streamed, networks);
            }
            *claim = Claim {
                slot: e.slot,
                generation: 0,
                named,
            };
        }
        if e.generation == claim.generation {
            return Poll::Ready(None);
        }
        match e.swarm {
            Some(s) => {
                claim.generation = e.generation;
                Poll::Ready(Some((s, e.options.chunk_size)))
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

    /// The swarm a stream with `c` reads next, without its chunk size.
    fn poll(r: &mut Registry<u32>, n: u8, c: &mut Claim) -> Poll<Option<u32>> {
        poll_on(r, n, c, &[])
    }

    fn poll_on(
        r: &mut Registry<u32>,
        n: u8,
        c: &mut Claim,
        networks: &[Vec<u8>],
    ) -> Poll<Option<u32>> {
        r.poll_swarm(&tid(n), c, networks, &Context::from_waker(Waker::noop()))
            .map(|s| s.map(|(swarm, _)| swarm))
    }

    #[test]
    fn the_last_stream_removes_an_egress_slot_only() {
        let mut r = Registry::default();
        let a = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        let b = r.claim(&tid(1), &[]).unwrap();
        assert!(r.release(&tid(1), a, &[]).gone.is_none());
        assert_eq!(r.release(&tid(1), b, &[]).gone.unwrap().swarm, Some(7));
        assert!(r.get(&tid(1)).is_none());

        let c = insert(&mut r, 2, Some(8), SwarmKind::Capture);
        assert!(r.release(&tid(2), c, &[]).gone.is_none());
        assert_eq!(r.get(&tid(2)).unwrap().refs, 0);
    }

    #[test]
    fn a_claim_on_a_removed_slot_releases_nothing() {
        let mut r = Registry::default();
        let old = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        r.remove(&tid(1));
        let new = insert(&mut r, 1, Some(9), SwarmKind::Egress);
        assert!(r.release(&tid(1), old, &[]).gone.is_none());
        assert_eq!(r.get(&tid(1)).unwrap().refs, 1);
        assert!(r.release(&tid(1), new, &[]).gone.is_some());
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
        assert!(r.release(&tid(1), c, &[]).gone.is_none());
        assert_eq!(r.count(SwarmKind::Egress), 1);
        let again = r.claim(&tid(1), &[]).unwrap();
        assert!(r.release(&tid(1), again, &[]).gone.is_none());
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
        assert!(r.release(&tid(1), c, &[]).gone.is_none());
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
        assert!(r.release(&tid(1), c, &[]).gone.is_none());
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

    fn seed(r: &mut Registry<u32>, n: u8, network: u8) -> Seeded<u32> {
        r.seed(
            tid(n),
            vec![network],
            SwarmId::new(&[n; 32]),
            SwarmOptions::default(),
            Time::EPOCH,
        )
    }

    #[test]
    fn a_seed_outlives_the_streams_on_its_slot() {
        let mut r = Registry::default();
        let c = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        assert_eq!(seed(&mut r, 1, 10), Seeded::Held(c.slot));
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds, e.refs), (SwarmKind::Seed, 1, 1));
        assert!(e.publishes_on(&[10]));
        assert!(r.release(&tid(1), c, &[]).gone.is_none());
        assert_eq!(r.get(&tid(1)).unwrap().refs, 0);
        assert!(r.seeds_slot(&tid(1), c.slot));

        // A second seed holds the slot past the first's end.
        assert_eq!(seed(&mut r, 1, 11), Seeded::Held(c.slot));
        assert!(r.unseed(&tid(1), c.slot, &[10]).gone.is_none());
        assert_eq!(r.get(&tid(1)).unwrap().seeds, 1);
        assert_eq!(
            r.unseed(&tid(1), c.slot, &[10]).gone.unwrap().swarm,
            Some(7)
        );
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
        let c = r.claim(&tid(1), &[]).unwrap();
        assert!(r.unseed(&tid(1), slot, &[10]).gone.is_none());
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds, e.refs), (SwarmKind::Egress, 0, 1));
        assert!(r.unseed(&tid(1), slot, &[10]).gone.is_none());
        assert_eq!(r.release(&tid(1), c, &[]).gone.unwrap().swarm, Some(7));
    }

    #[test]
    fn a_seed_ended_while_its_swarm_is_added_goes_when_the_add_ends() {
        let mut r = Registry::default();
        let Seeded::Added(slot) = seed(&mut r, 1, 10) else {
            panic!("no seed slot");
        };
        assert!(r.unseed(&tid(1), slot, &[10]).gone.is_none());
        assert_eq!(r.count(SwarmKind::Egress), 1);
        // Seeding it again takes the slot whose add is in flight.
        assert_eq!(seed(&mut r, 1, 10), Seeded::Held(slot));
        assert!(r.unseed(&tid(1), slot, &[10]).gone.is_none());
        assert_eq!(r.set_swarm(&tid(1), slot, 7), Added::Unwanted);
        assert!(r.get(&tid(1)).is_none());
    }

    #[test]
    fn a_writer_taking_over_a_seeded_slot_keeps_its_seeds() {
        let mut r = Registry::default();
        let Seeded::Added(slot) = seed(&mut r, 1, 10) else {
            panic!("no seed slot");
        };
        r.set_swarm(&tid(1), slot, 7);
        let c = r.claim(&tid(1), &[]).unwrap();
        let (taken, old) = r
            .replace(
                &tid(1),
                SwarmId::new(&[1; 32]),
                SwarmOptions::default(),
                SwarmKind::Ingest,
                BTreeSet::from([vec![11]]),
            )
            .unwrap();
        assert_eq!((taken, old), (slot, Some(7)));
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds, e.refs), (SwarmKind::Ingest, 1, 1));
        assert_eq!(e.networks(), BTreeSet::from([vec![10], vec![11]]));
        assert!(r.seeds_slot(&tid(1), slot));
        let released = r.unseed(&tid(1), slot, &[10]);
        assert!(released.gone.is_none());
        assert_eq!(released.unnamed, vec![vec![10]]);
        let e = r.get(&tid(1)).unwrap();
        assert_eq!((e.kind, e.seeds), (SwarmKind::Ingest, 0));
        assert!(!r.seeds_slot(&tid(1), slot));
        assert!(r.release(&tid(1), c, &[]).gone.is_none());
    }

    #[test]
    fn a_seed_holds_a_writers_slot_as_it_is() {
        let mut r = Registry::default();
        let capture = insert(&mut r, 1, Some(7), SwarmKind::Capture);
        let ingest = insert(&mut r, 2, Some(8), SwarmKind::Ingest);
        let other = SwarmOptions {
            chunks_per_signature: 1,
            ..SwarmOptions::default()
        };
        let seeded = r.seed(tid(1), vec![10], SwarmId::new(&[1; 32]), other, Time::EPOCH);
        assert_eq!(seeded, Seeded::Held(capture.slot));
        assert_eq!(seed(&mut r, 2, 10), Seeded::Held(ingest.slot));
        for (n, swarm) in [(1, 7), (2, 8)] {
            let e = r.get(&tid(n)).unwrap();
            assert_eq!((e.seeds, e.swarm), (1, Some(swarm)));
            assert!(e.publishes_on(&[10]));
        }
        assert_eq!(r.get(&tid(1)).unwrap().kind, SwarmKind::Capture);
        assert!(same_options(
            &r.get(&tid(1)).unwrap().options,
            &SwarmOptions::default()
        ));
        let released = r.unseed(&tid(1), capture.slot, &[10]);
        assert!(released.gone.is_none());
        assert_eq!(released.unnamed, vec![vec![10]]);
        assert_eq!(r.get(&tid(1)).unwrap().kind, SwarmKind::Capture);
    }

    fn keys(ks: &[u8]) -> Vec<Vec<u8>> {
        ks.iter().map(|&k| vec![k]).collect()
    }

    #[test]
    fn a_network_stays_published_while_a_holder_names_it() {
        let mut r = Registry::default();
        insert(&mut r, 1, Some(7), SwarmKind::Egress);
        let t = tid(1);
        // The insert's own claim names nothing; take two that do.
        let first = r.claim(&t, &keys(&[1, 2])).unwrap();
        let second = r.claim(&t, &keys(&[2])).unwrap();
        assert_eq!(seed(&mut r, 1, 3), Seeded::Held(first.slot));
        assert_eq!(
            r.get(&t).unwrap().networks(),
            BTreeSet::from([vec![1], vec![2], vec![3]])
        );

        let released = r.release(&t, first, &keys(&[1, 2]));
        assert!(released.gone.is_none());
        assert_eq!(released.unnamed, keys(&[1]));
        let released = r.release(&t, second, &keys(&[2]));
        assert_eq!(released.unnamed, keys(&[2]));
        assert!(r.get(&t).unwrap().publishes_on(&[3]));
        assert_eq!(r.list().remove(0).network_keys, keys(&[3]));

        // A seed and a stream naming one network: it stays until both end.
        let stream = r.claim(&t, &keys(&[3])).unwrap();
        assert!(r.unseed(&t, first.slot, &[3]).unnamed.is_empty());
        assert_eq!(r.get(&t).unwrap().kind, SwarmKind::Egress);
        assert!(r.get(&t).unwrap().publishes_on(&[3]));
        assert!(r.release(&t, stream, &keys(&[3])).gone.is_none());
    }

    #[test]
    fn streams_of_a_writers_slot_name_no_networks() {
        let mut r = Registry::default();
        let t = tid(1);
        r.insert(
            t,
            Some(7),
            SwarmId::new(&[1; 32]),
            SwarmOptions::default(),
            SwarmKind::Capture,
            BTreeSet::from([vec![1]]),
            0,
        );
        let c = r.claim(&t, &keys(&[2])).unwrap();
        assert_eq!(r.get(&t).unwrap().networks(), BTreeSet::from([vec![1]]));
        let released = r.release(&t, c, &keys(&[2]));
        assert!(released.gone.is_none() && released.unnamed.is_empty());
    }

    #[test]
    fn a_claim_moved_to_a_replacing_slot_names_its_networks_there() {
        let mut r = Registry::default();
        let t = tid(1);
        let mut c = insert(&mut r, 1, Some(7), SwarmKind::Capture);
        assert_eq!(
            poll_on(&mut r, 1, &mut c, &keys(&[2])),
            Poll::Ready(Some(7))
        );
        r.remove(&t);
        insert(&mut r, 1, Some(9), SwarmKind::Egress);
        assert_eq!(
            poll_on(&mut r, 1, &mut c, &keys(&[2])),
            Poll::Ready(Some(9))
        );
        assert!(r.get(&t).unwrap().publishes_on(&[2]));
        let released = r.release(&t, c, &keys(&[2]));
        assert!(released.gone.is_none());
        assert_eq!(released.unnamed, keys(&[2]));
    }

    #[test]
    fn a_writer_taking_over_a_slot_publishes_on_its_networks_alone() {
        let mut r = Registry::default();
        let t = tid(1);
        insert(&mut r, 1, Some(7), SwarmKind::Egress);
        let c = r.claim(&t, &keys(&[2])).unwrap();
        r.replace(
            &t,
            SwarmId::new(&[1; 32]),
            SwarmOptions::default(),
            SwarmKind::Ingest,
            BTreeSet::from([vec![1]]),
        );
        assert_eq!(r.get(&t).unwrap().networks(), BTreeSet::from([vec![1]]));
        let released = r.release(&t, c, &keys(&[2]));
        assert!(released.gone.is_none() && released.unnamed.is_empty());
    }

    fn options(chunks_per_signature: usize) -> SwarmOptions {
        SwarmOptions {
            chunks_per_signature,
            ..SwarmOptions::default()
        }
    }

    #[test]
    fn a_reopened_slot_keeps_its_holders_and_reads_a_swarm_of_its_new_options() {
        let mut r = Registry::default();
        let t = tid(1);
        let mut c = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Ready(Some(7)));
        assert_eq!(seed(&mut r, 1, 3), Seeded::Held(c.slot));
        let now = Time::EPOCH + strims_base::time::Duration::from_secs(5);
        let (slot, old) = r.reopen(&t, options(1), now).unwrap();
        assert_eq!((slot, old), (c.slot, Some(7)));
        let e = r.get(&t).unwrap();
        assert_eq!((e.kind, e.refs, e.seeds), (SwarmKind::Seed, 1, 1));
        assert_eq!(e.reopened, Some(now));
        assert!(e.publishes_on(&[3]));
        assert_eq!(poll(&mut r, 1, &mut c), Poll::Pending);
        // An add begun with the old options doesn't land.
        assert_eq!(r.set_added(&t, slot, &options(8), 8), Added::Unwanted);
        assert_eq!(r.set_added(&t, slot, &options(1), 9), Added::Kept);
        assert_eq!(
            r.poll_swarm(&t, &mut c, &[], &Context::from_waker(Waker::noop())),
            Poll::Ready(Some((9, SwarmOptions::default().chunk_size)))
        );
    }

    #[test]
    fn a_seed_of_other_options_reopens_the_slot() {
        let mut r = Registry::default();
        let t = tid(1);
        let c = insert(&mut r, 1, Some(7), SwarmKind::Egress);
        let seeded = r.seed(t, vec![3], SwarmId::new(&[1; 32]), options(1), Time::EPOCH);
        assert_eq!(
            seeded,
            Seeded::Reopened {
                slot: c.slot,
                old: Some(7)
            }
        );
        assert!(same_options(&r.get(&t).unwrap().options, &options(1)));
        // Writers' slots are read as they are.
        r.remove(&t);
        insert(&mut r, 1, Some(7), SwarmKind::Ingest);
        assert!(r.reopen(&t, options(1), Time::EPOCH).is_none());
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
