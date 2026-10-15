// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A network's links, routing table, next-hop hints and link selection.
//!
//! A `Network` is one overlay: its links (vnic peers added with
//! `Host::add_peer`), the routing table over them, its port handlers, the
//! next-hop hints learned from trailers, and a bounded outbound queue per
//! link. Message processing that needs the whole host (signing, dedup,
//! services) is in [`crate::host`].
//!
//! # Greedy selection
//!
//! [`Routing::Greedy`] for everything, and both modes for broadcasts: take
//! the `2 * MAX_MESSAGE_REPLICAS` routed links closest to the destination;
//! for a unicast, a direct link alone, or else the learned next hop first;
//! then up to `MAX_MESSAGE_REPLICAS - hops` of them, skipping hosts already
//! in the trailer. A broadcast so reaches a bounded set of hosts near its
//! destination id: five links, then four from each of those, and so on.
//!
//! # Learned routing
//!
//! [`Routing::Learned`] unicasts use a link graph learned from trailers
//! (consecutive entries are linked), link announcements on port 20 and
//! peer index search answers (a responder lists only holders it is linked
//! to). Announcements give every host the links three hops out before any
//! traffic, so a unicast to an unknown destination goes blind for at most
//! two hops.
//!
//! - A direct link to the destination gets the only copy unless it is
//!   backlogged (`Backlog`). A backup over a longer path costs frames
//!   and, when it wins the race, arrives over the longer path; it only
//!   pays off where copies are late or lost.
//! - Otherwise the links on the shortest learned paths the hop limit still
//!   allows, up to `Config::route_copies`.
//! - With no known path, the greedy fan-out of `MAX_MESSAGE_REPLICAS - hops`
//!   links, best connected (most learned links) first, then by XOR
//!   distance: those reach the most hosts that may know a path.
//! - A copy whose receiver could neither be the destination nor relay it
//!   isn't sent.
//! - The dedup table keeps the fewest hops seen per message, and a unicast
//!   copy with fewer hops than every earlier one is relayed again without a
//!   second delivery: an earlier copy that took a detour may have run out
//!   of hops. That happens at most four times per host and message.
//! - A broadcast flagged `M_NORELAY` goes to every routed link.
//!
//! Per message, the sender and each relay search the learned graph
//! breadth-first: depth at most four, at most
//! `topology_max_nodes * topology_max_degree` link visits, and no
//! allocation once the scratch has grown. A direct link that takes the
//! only copy skips the search.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use strims_base::time::{Duration, Time};
use strims_kademlia::{ID, Insert, Interface, KBucket};
use strims_qos::ClassId;
use strims_vnic::{PeerKey, WriterId};

use crate::buf::BufId;
use crate::host::Routing;
use crate::message::{MAX_MESSAGE_HOPS, MAX_MESSAGE_REPLICAS};
use crate::peer_index::PeerIndexStore;
use crate::topology::Topology;

/// Entries per routing-table bucket.
pub const LINK_BUCKET_SIZE: usize = 20;

/// How many routed links sending considers.
const CANDIDATES: usize = MAX_MESSAGE_REPLICAS * 2;

/// How long a link that dropped a copy on a full queue counts as lossy.
pub(crate) const DROP_MEMORY: Duration = Duration::from_secs(10);

/// When a link's copies are likely to be late or lost: its queue holds at
/// least `messages` messages or `bytes` bytes, or it dropped a copy on a
/// full queue within [`DROP_MEMORY`]. vnic links are reliable and ordered,
/// so a full vpn queue is the only place a copy can be lost while the link
/// is up. [`Routing::Learned`] adds a backup copy to a unicast over a
/// direct link only then.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Backlog {
    pub(crate) messages: usize,
    pub(crate) bytes: usize,
}

/// Who handles a port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler<H> {
    HashTable,
    PeerIndex,
    PeerExchange,
    /// Link announcements.
    LinkAnnounce,
    App(H),
}

/// A vnic peer in the network.
#[derive(Debug)]
pub(crate) struct Link {
    pub(crate) host_id: ID,
    /// The port whose vnic handler feeds this network.
    pub(crate) src_port: u16,
    pub(crate) writer: WriterId,
    /// The largest message the link's frames carry (MTU - 4).
    pub(crate) max_body: usize,
    /// Marshaled messages waiting for `fill_egress`. Bounded by
    /// `Config::link_queue_messages` and `Config::link_queue_bytes`.
    pub(crate) queue: VecDeque<BufId>,
    pub(crate) queued_bytes: usize,
    /// In the routing table. A link evicted from a full bucket stays
    /// attached (its frames are still read) but isn't routed through; it
    /// is re-inserted when a bucket frees up.
    pub(crate) routed: bool,
    /// When a copy for this link was last dropped on a full queue
    /// (`Time::NIL`: never).
    pub(crate) dropped_at: Time,
}

impl Link {
    /// Whether copies queued now are likely to be late or lost.
    pub(crate) fn backlogged(&self, now: Time, b: Backlog) -> bool {
        self.queue.len() >= b.messages
            || self.queued_bytes >= b.bytes
            || (!self.dropped_at.is_nil() && now - self.dropped_at < DROP_MEMORY)
    }
}

/// A routing-table entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Route {
    pub(crate) id: ID,
    pub(crate) peer: PeerKey,
}

impl Interface for Route {
    fn id(&self) -> ID {
        self.id
    }
}

/// Up to [`CANDIDATES`] links, or up to [`MAX_MESSAGE_REPLICAS`] targets.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Targets {
    items: [Option<Route>; CANDIDATES],
    len: usize,
    /// Chosen by [`Routing::Learned`] from a learned path or direct link.
    pub(crate) learned: bool,
}

impl Targets {
    fn new() -> Targets {
        Targets {
            items: [None; CANDIDATES],
            len: 0,
            learned: false,
        }
    }

    fn push(&mut self, r: Route) {
        if let Some(slot) = self.items.get_mut(self.len) {
            *slot = Some(r);
            self.len += 1;
        }
    }

    /// Appends routes until there are `limit`.
    fn extend<K>(&mut self, best: [Option<(K, Route)>; CANDIDATES], limit: usize) {
        for (_, r) in best.into_iter().flatten() {
            if self.len >= limit {
                break;
            }
            self.push(r);
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = Route> + '_ {
        self.items.iter().take(self.len).flatten().copied()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }
}

/// The [`CANDIDATES`] items with the smallest keys, in order (ties keep
/// the iteration order), without allocating.
fn top<K: Ord + Copy>(items: impl Iterator<Item = (K, Route)>) -> [Option<(K, Route)>; CANDIDATES] {
    let mut best: [Option<(K, Route)>; CANDIDATES] = [None; CANDIDATES];
    for item in items {
        let mut item = Some(item);
        for slot in best.iter_mut() {
            match (*slot, item) {
                (_, None) => break,
                (None, _) => {
                    *slot = item;
                    break;
                }
                (Some(cur), Some(new)) if new.0 < cur.0 => {
                    *slot = item;
                    item = Some(cur);
                }
                _ => {}
            }
        }
    }
    best
}

/// A learned route: messages from `target` arrived through `via`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NextHop {
    via: ID,
    distance: usize,
    updated: Time,
}

/// Next-hop hints learned from message trailers ([`Routing::Greedy`]).
///
/// A hint is replaced by a shorter route, refreshed when seen again,
/// expires after `ttl` and goes with its link, so a stale hint never blocks
/// later routes to its target. Bounded at `cap` hints.
#[derive(Debug)]
pub(crate) struct NextHopMap {
    /// Bounded by `cap`; keys come from peer input.
    routes: BTreeMap<ID, NextHop>,
    cap: usize,
    ttl: Duration,
}

impl NextHopMap {
    pub(crate) fn new(cap: usize, ttl: Duration) -> NextHopMap {
        NextHopMap {
            routes: BTreeMap::new(),
            cap,
            ttl,
        }
    }

    fn expired(&self, h: &NextHop, now: Time) -> bool {
        now - h.updated > self.ttl
    }

    /// Records that `target` is `distance` hops away through `via`.
    pub(crate) fn insert(&mut self, now: Time, target: ID, via: ID, distance: usize) {
        let ttl = self.ttl;
        if let Some(h) = self.routes.get_mut(&target) {
            let expired = now - h.updated > ttl;
            if expired || distance < h.distance {
                *h = NextHop {
                    via,
                    distance,
                    updated: now,
                };
            } else if h.via == via && distance == h.distance {
                h.updated = now;
            }
            return;
        }
        if self.routes.len() >= self.cap {
            self.prune(now);
            if self.routes.len() >= self.cap {
                return;
            }
        }
        self.routes.insert(
            target,
            NextHop {
                via,
                distance,
                updated: now,
            },
        );
    }

    /// The link towards `target`, unless the hint expired.
    pub(crate) fn get(&self, now: Time, target: &ID) -> Option<ID> {
        let h = self.routes.get(target)?;
        (!self.expired(h, now)).then_some(h.via)
    }

    /// Forgets hints through a removed link.
    pub(crate) fn remove_via(&mut self, via: &ID) {
        self.routes.retain(|_, h| h.via != *via);
    }

    pub(crate) fn prune(&mut self, now: Time) {
        let ttl = self.ttl;
        self.routes.retain(|_, h| now - h.updated <= ttl);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.routes.len()
    }
}

/// One overlay network.
#[derive(Debug)]
pub(crate) struct Network<H> {
    /// The network's CA public key.
    pub(crate) key: Vec<u8>,
    /// Keys this network's hash table records, and its message ids in the
    /// host-wide dedup table.
    pub(crate) store_id: u32,
    /// The QoS class of the network's writers.
    pub(crate) class: ClassId,
    /// The last message sequence number; the wire carries its low 16 bits.
    pub(crate) seq: u64,
    /// Bounded by the vnic peers.
    pub(crate) links: BTreeMap<PeerKey, Link>,
    pub(crate) by_host: BTreeMap<ID, PeerKey>,
    /// The routing table over the routed links.
    pub(crate) routes: KBucket<Route>,
    /// Bounded by the owner's calls.
    pub(crate) handlers: BTreeMap<u16, Handler<H>>,
    pub(crate) reservations: BTreeSet<u16>,
    pub(crate) next_hop: NextHopMap,
    /// The link graph learned from trailers ([`Routing::Learned`]).
    pub(crate) topology: Topology,
    pub(crate) peer_index: PeerIndexStore,
    /// Links whose queue changed since the last `flush`.
    pub(crate) dirty: BTreeSet<PeerKey>,
    /// The lists this host's links announced ([`crate::announce`]).
    pub(crate) neighbour_lists: crate::announce::NeighbourLists,
    /// The links (or a neighbour's list) changed: `flush` schedules a link
    /// announcement.
    pub(crate) links_changed: bool,
    /// When the next link announcement is due (`Time::NIL`: none).
    pub(crate) announce_at: Time,
    /// Links added since this host's records were last pushed
    /// (`Host::push_records`). Bounded by the links.
    pub(crate) new_links: BTreeSet<PeerKey>,
    /// When the next record push is due (`Time::NIL`: none).
    pub(crate) push_at: Time,
    /// When records were last pushed (`Time::NIL`: never).
    pub(crate) pushed_at: Time,
}

impl<H: Copy> Network<H> {
    pub(crate) fn new(
        key: Vec<u8>,
        store_id: u32,
        class: ClassId,
        host_id: ID,
        next_hop: NextHopMap,
        topology: Topology,
        peer_index: PeerIndexStore,
    ) -> Network<H> {
        let mut handlers = BTreeMap::new();
        handlers.insert(strims_vnic::ports::HASH_TABLE_PORT, Handler::HashTable);
        handlers.insert(strims_vnic::ports::PEER_INDEX_PORT, Handler::PeerIndex);
        handlers.insert(
            strims_vnic::ports::PEER_EXCHANGE_PORT,
            Handler::PeerExchange,
        );
        handlers.insert(crate::announce::LINK_ANNOUNCE_PORT, Handler::LinkAnnounce);
        let reservations = handlers.keys().copied().collect();
        Network {
            key,
            store_id,
            class,
            seq: 0,
            links: BTreeMap::new(),
            by_host: BTreeMap::new(),
            routes: KBucket::new(host_id, LINK_BUCKET_SIZE),
            handlers,
            reservations,
            next_hop,
            topology,
            peer_index,
            dirty: BTreeSet::new(),
            neighbour_lists: Default::default(),
            links_changed: false,
            announce_at: Time::NIL,
            new_links: BTreeSet::new(),
            push_at: Time::NIL,
            pushed_at: Time::NIL,
        }
    }

    /// Whether the host is a routed link.
    pub(crate) fn has_peer(&self, id: &ID) -> bool {
        self.routes.contains(id)
    }

    /// Adds a link to the table and the routing table. Returns the link
    /// evicted from the routing table, if any.
    pub(crate) fn insert_link(&mut self, peer: PeerKey, mut link: Link) {
        let id = link.host_id;
        match self.routes.insert(Route { id, peer }) {
            Insert::Added => link.routed = true,
            Insert::Evicted(old) => {
                link.routed = true;
                if let Some(l) = self.links.get_mut(&old.peer) {
                    l.routed = false;
                }
            }
            Insert::Duplicate(_) | Insert::Rejected(_) => link.routed = false,
        }
        self.by_host.insert(id, peer);
        self.links.insert(peer, link);
        self.links_changed = true;
        self.new_links.insert(peer);
    }

    /// Removes a link, returning it with its queue for the caller to
    /// release, and routes through links waiting outside full buckets.
    pub(crate) fn remove_link(&mut self, peer: PeerKey) -> Option<Link> {
        let link = self.links.remove(&peer)?;
        if self.by_host.get(&link.host_id) == Some(&peer) {
            self.by_host.remove(&link.host_id);
        }
        if link.routed {
            self.routes.remove(&link.host_id);
        }
        self.next_hop.remove_via(&link.host_id);
        self.neighbour_lists.remove(&link.host_id);
        self.dirty.remove(&peer);
        self.new_links.remove(&peer);
        self.links_changed = true;
        // Re-insert evicted links. Every one of them is farther than the
        // entries of its (full) bucket, so this never evicts.
        let standby: Vec<(PeerKey, ID)> = self
            .links
            .iter()
            .filter(|(_, l)| !l.routed)
            .map(|(k, l)| (*k, l.host_id))
            .collect();
        for (k, id) in standby {
            if let Insert::Added | Insert::Evicted(_) = self.routes.insert(Route { id, peer: k })
                && let Some(l) = self.links.get_mut(&k)
            {
                l.routed = true;
            }
        }
        Some(link)
    }

    /// The [`CANDIDATES`] routed links closest to `dst`, nearest first:
    /// `strims_kademlia::KBucket::closest` without allocating.
    fn closest(&self, dst: &ID) -> Targets {
        let mut out = Targets::new();
        let mut dist = [ID::MAX; CANDIDATES];
        for r in self.routes.iter() {
            let d = r.id ^ *dst;
            let n = out.len;
            if n == CANDIDATES && dist.last().is_some_and(|last| d >= *last) {
                continue;
            }
            // Insertion sort into the first n (or n - 1 when full) slots.
            let mut i = n.min(CANDIDATES - 1);
            while i > 0 && dist.get(i - 1).is_some_and(|p| *p > d) {
                if let (Some(&pd), Some(&pr)) = (dist.get(i - 1), out.items.get(i - 1)) {
                    if let Some(x) = dist.get_mut(i) {
                        *x = pd;
                    }
                    if let Some(x) = out.items.get_mut(i) {
                        *x = pr;
                    }
                }
                i -= 1;
            }
            if let Some(x) = dist.get_mut(i) {
                *x = d;
            }
            if let Some(x) = out.items.get_mut(i) {
                *x = Some(*r);
            }
            out.len = (n + 1).min(CANDIDATES);
        }
        out
    }

    /// The links a message to `dst` goes to. `trailer` holds the hosts
    /// the message has been through (empty at the sender).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn select_links(
        &mut self,
        now: Time,
        dst: &ID,
        unicast: bool,
        trailer: &[ID],
        routing: Routing,
        copies: usize,
        backlog: Backlog,
    ) -> Targets {
        let hops = trailer.len();
        let exclude = |id: &ID| trailer.contains(id);
        match routing {
            Routing::Learned if unicast => {
                self.select_learned(now, dst, hops, exclude, copies, backlog)
            }
            // Every host must store and look up hash table records at the
            // same hosts, whatever its routing mode.
            _ => self.select_greedy(now, dst, unicast, hops, exclude),
        }
    }

    /// [`Routing::Learned`] unicast: a direct link to `dst` alone unless
    /// it is backlogged ([`Backlog`]); otherwise the links with the
    /// shortest learned paths to `dst` that the hop limit still allows
    /// (after a backlogged direct link, only links that aren't), up to
    /// `copies` links in all and no more than
    /// `MAX_MESSAGE_REPLICAS - hops`. With no known path, the greedy fan-out
    /// of `MAX_MESSAGE_REPLICAS - hops` links, best connected first. A copy
    /// whose receiver can't relay goes only to a direct link.
    fn select_learned(
        &mut self,
        now: Time,
        dst: &ID,
        hops: usize,
        exclude: impl Fn(&ID) -> bool,
        copies: usize,
        backlog: Backlog,
    ) -> Targets {
        let fan_out = MAX_MESSAGE_REPLICAS.saturating_sub(hops);
        // Hops left after the next one.
        let remaining = MAX_MESSAGE_HOPS.saturating_sub(hops + 1);
        let mut out = Targets::new();
        let backlogged = |links: &BTreeMap<PeerKey, Link>, r: &Route| {
            links
                .get(&r.peer)
                .is_some_and(|l| l.backlogged(now, backlog))
        };
        let direct = self.routes.get(dst).copied();
        if let Some(r) = direct {
            out.push(r);
            // The direct link is the shortest path. A backup over a longer
            // one costs two or more frames and, when it wins the race,
            // delivers over the longer path; it only pays off when the
            // direct copy is late or lost.
            if !backlogged(&self.links, &r) {
                out.learned = true;
                return out;
            }
        }
        if remaining == 0 {
            out.learned = out.len > 0;
            return out;
        }
        // invariant: remaining < MAX_MESSAGE_HOPS.
        if self.topology.search(now, dst, remaining as u8) {
            let (topology, links) = (&self.topology, &self.links);
            let best = top(self.routes.iter().filter_map(|r| {
                if r.id == *dst || exclude(&r.id) || (direct.is_some() && backlogged(links, r)) {
                    return None;
                }
                let d = topology.distance(&r.id)?;
                Some(((d, r.id ^ *dst), *r))
            }));
            out.extend(best, fan_out.min(copies.max(1)));
        }
        if out.len > 0 {
            out.learned = true;
            return out;
        }
        // No known path: the best connected links first (they reach the
        // most hosts that may know one), then the closest by XOR distance.
        let topology = &self.topology;
        let best = top(self.routes.iter().filter(|r| !exclude(&r.id)).map(|r| {
            let degree = topology.degree(&r.id);
            ((usize::MAX - degree, r.id ^ *dst), *r)
        }));
        out.extend(best, fan_out);
        out
    }

    /// Greedy selection: take the [`CANDIDATES`] links closest to `dst`, put a direct link first for
    /// a unicast to a neighbour (and send only to it), or else the learned
    /// next hop, then take up to `MAX_MESSAGE_REPLICAS - hops` of them.
    pub(crate) fn select_greedy(
        &self,
        now: Time,
        dst: &ID,
        unicast: bool,
        hops: usize,
        exclude: impl Fn(&ID) -> bool,
    ) -> Targets {
        let mut c = self.closest(dst);
        if unicast {
            let direct = c
                .items
                .first()
                .copied()
                .flatten()
                .is_some_and(|r| r.id == *dst);
            if direct {
                c.len = 1;
            } else if let Some(r) = self
                .next_hop
                .get(now, dst)
                .and_then(|via| self.routes.get(&via))
            {
                let r = *r;
                let mut k = c.iter().position(|x| x == r).unwrap_or(c.len);
                if k == c.len && c.len < CANDIDATES {
                    c.len += 1;
                }
                k = k.min(CANDIDATES - 1);
                c.items.copy_within(0..k, 1);
                if let Some(x) = c.items.first_mut() {
                    *x = Some(r);
                }
            }
        }

        let limit = MAX_MESSAGE_REPLICAS.saturating_sub(hops);
        let mut out = Targets::new();
        let mut k = 0;
        for r in c.iter() {
            if exclude(&r.id) {
                continue;
            }
            out.push(r);
            if unicast && r.id == *dst {
                break;
            }
            k += 1;
            if k >= limit {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use slotmap::SlotMap;
    use strims_base::rng::det_rng;

    fn rand_id(rng: &mut strims_base::rng::DetRng) -> ID {
        let mut b = [0u8; 32];
        rand_core::Rng::fill_bytes(rng, &mut b);
        ID::from_bytes(&b)
    }

    fn network(self_id: ID) -> Network<()> {
        let class = strims_qos::Control::new().root();
        Network::new(
            vec![],
            1,
            class,
            self_id,
            NextHopMap::new(16, Duration::MINUTE),
            Topology::new(self_id, 16, 4, Duration::MINUTE),
            PeerIndexStore::new(16),
        )
    }

    fn link(host_id: ID, writer: WriterId) -> Link {
        Link {
            host_id,
            src_port: 1,
            writer,
            max_body: 1000,
            queue: VecDeque::new(),
            queued_bytes: 0,
            routed: false,
            dropped_at: Time::NIL,
        }
    }

    const BACKLOG: Backlog = Backlog {
        messages: 32,
        bytes: 128 * 1024,
    };

    #[test]
    fn closest_matches_kbucket() {
        let mut rng = det_rng(3);
        let mut keys: SlotMap<PeerKey, ()> = SlotMap::with_key();
        let mut writers: SlotMap<WriterId, ()> = SlotMap::with_key();
        let mut n = network(rand_id(&mut rng));
        for _ in 0..60 {
            n.insert_link(keys.insert(()), link(rand_id(&mut rng), writers.insert(())));
        }
        for _ in 0..50 {
            let dst = rand_id(&mut rng);
            let mut want = Vec::new();
            n.routes.closest(&dst, CANDIDATES, &mut want);
            let got: Vec<Route> = n.closest(&dst).iter().collect();
            let want: Vec<Route> = want.into_iter().copied().collect();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn greedy_selection_order_and_limits() {
        let mut rng = det_rng(4);
        let mut keys: SlotMap<PeerKey, ()> = SlotMap::with_key();
        let mut writers: SlotMap<WriterId, ()> = SlotMap::with_key();
        let me = rand_id(&mut rng);
        let mut n = network(me);
        let ids: Vec<ID> = (0..12).map(|_| rand_id(&mut rng)).collect();
        for id in &ids {
            n.insert_link(keys.insert(()), link(*id, writers.insert(())));
        }
        let now = Time::from_unix(100, 0);
        let none = |_: &ID| false;

        // Direct unicast: only the neighbour.
        let t = n.select_greedy(now, &ids[3], true, 0, none);
        assert_eq!(t.iter().map(|r| r.id).collect::<Vec<_>>(), vec![ids[3]]);

        // Broadcast: the 5 closest, fewer with more hops.
        let far = rand_id(&mut rng);
        let t = n.select_greedy(now, &far, false, 0, none);
        assert_eq!(t.len(), 5);
        let c: Vec<ID> = n.closest(&far).iter().map(|r| r.id).collect();
        assert_eq!(t.iter().map(|r| r.id).collect::<Vec<_>>(), c[..5]);
        assert_eq!(n.select_greedy(now, &far, false, 3, none).len(), 2);
        // Excluded hosts don't count toward the replicas.
        let t = n.select_greedy(now, &far, false, 3, |id| *id == c[0]);
        assert_eq!(t.iter().map(|r| r.id).collect::<Vec<_>>(), c[1..3]);

        // A learned next hop goes first; one already among the candidates
        // moves to the front.
        let via = c[7];
        n.next_hop.insert(now, far, via, 2);
        let t = n.select_greedy(now, &far, true, 0, none);
        let got: Vec<ID> = t.iter().map(|r| r.id).collect();
        assert_eq!(got, vec![c[7], c[0], c[1], c[2], c[3]]);
        // ...and expires.
        let later = now + Duration::from_secs(61);
        let t = n.select_greedy(later, &far, true, 0, none);
        assert_eq!(t.iter().map(|r| r.id).collect::<Vec<_>>(), c[..5]);
    }

    #[test]
    fn learned_unicasts_follow_shortest_paths() {
        let mut rng = det_rng(5);
        let mut keys: SlotMap<PeerKey, ()> = SlotMap::with_key();
        let mut writers: SlotMap<WriterId, ()> = SlotMap::with_key();
        let me = rand_id(&mut rng);
        let mut n = network(me);
        let [a, b, c, d, x] = [0; 5].map(|_| rand_id(&mut rng));
        for id in [a, b, c] {
            n.insert_link(keys.insert(()), link(id, writers.insert(())));
        }
        let now = Time::from_unix(100, 0);
        // a - x - d, b - d; c knows nobody.
        n.topology.learn_edge(now, a, x);
        n.topology.learn_edge(now, x, d);
        n.topology.learn_edge(now, b, d);
        let ids = |t: Targets| t.iter().map(|r| r.id).collect::<Vec<_>>();
        let mut sel = |trailer: &[ID], copies| {
            let t = n.select_links(now, &d, true, trailer, Routing::Learned, copies, BACKLOG);
            (t.learned, ids(t))
        };
        // Shortest first, up to `copies`.
        assert_eq!(sel(&[], 2), (true, vec![b, a]));
        assert_eq!(sel(&[], 1), (true, vec![b]));
        // Not back into the trailer.
        assert_eq!(sel(&[b], 2), (true, vec![a]));
        // Only paths the hop limit allows: after three hops, one is left.
        let t3 = [x, x, x];
        assert_eq!(sel(&t3, 2), (true, vec![b]));
        // After four, only a direct link would do.
        assert_eq!(sel(&[x, x, x, x], 2), (false, vec![]));

        // A direct link alone: a backup over a known path costs more frames
        // and can arrive first over a longer path.
        let kd = keys.insert(());
        n.insert_link(kd, link(d, writers.insert(())));
        let sel = |n: &mut Network<()>, at: Time| {
            let t = n.select_links(at, &d, true, &[], Routing::Learned, 2, BACKLOG);
            (t.learned, ids(t))
        };
        assert_eq!(sel(&mut n, now), (true, vec![d]));
        // A backlogged direct link gets a backup path, over a link that
        // isn't backlogged itself.
        n.links.get_mut(&kd).unwrap().queued_bytes = BACKLOG.bytes;
        assert_eq!(sel(&mut n, now), (true, vec![d, b]));
        let kb = n.by_host[&b];
        n.links.get_mut(&kb).unwrap().queued_bytes = BACKLOG.bytes;
        assert_eq!(sel(&mut n, now), (true, vec![d, a]));
        n.links.get_mut(&kb).unwrap().queued_bytes = 0;
        // So does one that recently dropped a copy on a full queue, until
        // the drop is DROP_MEMORY old.
        let l = n.links.get_mut(&kd).unwrap();
        l.queued_bytes = 0;
        l.dropped_at = now;
        assert_eq!(sel(&mut n, now), (true, vec![d, b]));
        assert_eq!(sel(&mut n, now + DROP_MEMORY), (true, vec![d]));
        n.links.get_mut(&kd).unwrap().dropped_at = Time::NIL;

        // Unknown destinations: the greedy fan-out, best connected first.
        let far = rand_id(&mut rng);
        let t = n.select_links(now, &far, true, &[], Routing::Learned, 2, BACKLOG);
        assert!(!t.learned);
        let got = ids(t);
        assert_eq!(got.len(), 4);
        // d has two learned links (x and b); c has none.
        assert_eq!(got[0], d);
        assert_eq!(got[3], c);
        // Greedy ignores the graph.
        let g = n.select_links(now, &d, true, &[], Routing::Greedy, 2, BACKLOG);
        assert_eq!(ids(g), vec![d]);
        let g = n.select_greedy(now, &far, true, 0, |_| false);
        assert_eq!(
            n.select_links(now, &far, true, &[], Routing::Greedy, 2, BACKLOG)
                .iter()
                .count(),
            g.iter().count()
        );
    }

    /// A hint through a removed link must not block later routes to its
    /// target.
    #[test]
    fn next_hop_hints_are_dropped_with_their_link() {
        let now = Time::from_unix(100, 0);
        let mut m = NextHopMap::new(2, Duration::MINUTE);
        let (a, b, x, y, z) = (ID([1; 4]), ID([2; 4]), ID([3; 4]), ID([4; 4]), ID([5; 4]));
        m.insert(now, x, a, 1);
        // Equal or longer routes don't replace it...
        m.insert(now, x, b, 1);
        assert_eq!(m.get(now, &x), Some(a));
        // ...until the link goes.
        m.remove_via(&a);
        m.insert(now, x, b, 3);
        assert_eq!(m.get(now, &x), Some(b));
        // Bounded: a full map refuses new targets until hints expire.
        m.insert(now, y, a, 1);
        m.insert(now, z, a, 1);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(now, &z), None);
        let later = now + Duration::from_secs(61);
        m.insert(later, z, a, 1);
        assert_eq!(m.get(later, &z), Some(a));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn evicted_links_wait_and_return() {
        let mut keys: SlotMap<PeerKey, ()> = SlotMap::with_key();
        let mut writers: SlotMap<WriterId, ()> = SlotMap::with_key();
        let me = ID::MIN;
        let mut n = network(me);
        // All in bucket 0 (top bit set); 21 entries overflow it.
        let mut peers = Vec::new();
        for i in 0..=LINK_BUCKET_SIZE as u64 {
            let id = ID([(1 << 63) | (u64::MAX - i * (1 << 40)), 0, 0, 0]);
            let k = keys.insert(());
            peers.push((k, id));
            n.insert_link(k, link(id, writers.insert(())));
        }
        assert_eq!(n.links.len(), LINK_BUCKET_SIZE + 1);
        assert_eq!(n.routes.len(), LINK_BUCKET_SIZE);
        // The first (farthest) was evicted by the last (closest).
        let (k0, id0) = peers[0];
        assert!(!n.links[&k0].routed);
        assert!(!n.has_peer(&id0));
        // Removing a routed link brings it back.
        let (k1, id1) = peers[1];
        n.remove_link(k1).unwrap();
        assert!(!n.has_peer(&id1));
        assert!(n.links[&k0].routed);
        assert!(n.has_peer(&id0));
        // Removing an unrouted link leaves the table alone.
        let k2 = keys.insert(());
        let id2 = ID([u64::MAX, 1, 0, 0]);
        n.insert_link(k2, link(id2, writers.insert(())));
        assert!(!n.links[&k2].routed);
        n.remove_link(k2).unwrap();
        assert_eq!(n.routes.len(), LINK_BUCKET_SIZE);
    }
}
