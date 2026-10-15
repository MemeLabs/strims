// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A bounded link graph learned from message trailers, for routing on
//! unstructured link graphs.
//!
//! Nodes are other hosts. This host's own links are the network's links
//! and are not stored, so every path found avoids this host. Edges are
//! undirected and stamped with the last time a trailer (or another hint)
//! showed them; edges older than `ttl` are ignored and pruned.
//!
//! Bounds: at most `max_nodes` nodes (the least recently seen goes first)
//! and `max_degree` edges per node (the oldest goes first). Searching
//! reuses scratch buffers and doesn't allocate once they have grown.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use strims_base::time::{Duration, Time};
use strims_kademlia::ID;

/// A learned link to node `to` (an index into `Topology::nodes`).
#[derive(Clone, Copy, Debug)]
struct Edge {
    to: u32,
    seen: Time,
}

#[derive(Debug)]
struct Node {
    id: ID,
    seen: Time,
    /// Bounded by `Topology::max_degree`.
    edges: Vec<Edge>,
}

/// The learned graph of one network.
#[derive(Debug)]
pub(crate) struct Topology {
    me: ID,
    /// A slab; bounded by `max_nodes`.
    nodes: Vec<Option<Node>>,
    free: Vec<u32>,
    /// Same entries as `nodes`.
    by_id: BTreeMap<ID, u32>,
    /// Same entries as `nodes`, least recently seen first.
    by_seen: BTreeSet<(Time, u32)>,
    max_nodes: usize,
    max_degree: usize,
    ttl: Duration,
    // Search scratch, indexed like `nodes`.
    mark: Vec<u32>,
    dist: Vec<u8>,
    epoch: u32,
    queue: VecDeque<u32>,
}

impl Topology {
    pub(crate) fn new(me: ID, max_nodes: usize, max_degree: usize, ttl: Duration) -> Topology {
        Topology {
            me,
            nodes: Vec::new(),
            free: Vec::new(),
            by_id: BTreeMap::new(),
            by_seen: BTreeSet::new(),
            // Two, so that learning an edge never evicts its other end.
            max_nodes: max_nodes.clamp(2, u32::MAX as usize - 1),
            max_degree: max_degree.max(1),
            ttl,
            mark: Vec::new(),
            dist: Vec::new(),
            epoch: 0,
            queue: VecDeque::new(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.by_id.len()
    }

    fn node_mut(&mut self, i: u32) -> Option<&mut Node> {
        self.nodes.get_mut(i as usize).and_then(Option::as_mut)
    }

    fn touch(&mut self, i: u32, now: Time) {
        let Some(n) = self.nodes.get_mut(i as usize).and_then(Option::as_mut) else {
            return;
        };
        if n.seen != now {
            self.by_seen.remove(&(n.seen, i));
            n.seen = now;
            self.by_seen.insert((now, i));
        }
    }

    /// The node for `id`, inserted if needed; a full graph drops its least
    /// recently seen node other than `keep`.
    fn get_or_insert(&mut self, now: Time, id: ID, keep: u32) -> Option<u32> {
        if let Some(&i) = self.by_id.get(&id) {
            self.touch(i, now);
            return Some(i);
        }
        if self.by_id.len() >= self.max_nodes {
            let victim = self.by_seen.iter().map(|&(_, i)| i).find(|&i| i != keep)?;
            self.remove_node(victim);
        }
        let node = Node {
            id,
            seen: now,
            edges: Vec::new(),
        };
        let i = match self.free.pop() {
            Some(i) => {
                if let Some(slot) = self.nodes.get_mut(i as usize) {
                    *slot = Some(node);
                }
                i
            }
            None => {
                // invariant: fewer than max_nodes < u32::MAX nodes.
                let i = self.nodes.len() as u32;
                self.nodes.push(Some(node));
                i
            }
        };
        self.by_id.insert(id, i);
        self.by_seen.insert((now, i));
        Some(i)
    }

    fn remove_node(&mut self, i: u32) {
        let Some(n) = self.nodes.get_mut(i as usize).and_then(Option::take) else {
            return;
        };
        self.by_id.remove(&n.id);
        self.by_seen.remove(&(n.seen, i));
        for e in &n.edges {
            if let Some(m) = self.node_mut(e.to) {
                m.edges.retain(|x| x.to != i);
            }
        }
        self.free.push(i);
    }

    /// Adds or refreshes the half edge `i -> j`; a full node drops its
    /// oldest edge (both halves).
    fn half_edge(&mut self, i: u32, j: u32, now: Time) {
        let max = self.max_degree;
        let Some(n) = self.node_mut(i) else {
            return;
        };
        if let Some(e) = n.edges.iter_mut().find(|e| e.to == j) {
            e.seen = now;
            return;
        }
        let mut dropped = None;
        if n.edges.len() >= max
            && let Some((k, _)) = n.edges.iter().enumerate().min_by_key(|(_, e)| e.seen)
        {
            dropped = Some(n.edges.swap_remove(k).to);
        }
        n.edges.push(Edge { to: j, seen: now });
        if let Some(d) = dropped
            && let Some(m) = self.node_mut(d)
        {
            m.edges.retain(|x| x.to != i);
        }
    }

    /// Records that `a` and `b` are linked. Edges at this host are its
    /// own links and are ignored.
    pub(crate) fn learn_edge(&mut self, now: Time, a: ID, b: ID) {
        if a == b || a == self.me || b == self.me {
            return;
        }
        let Some(i) = self.get_or_insert(now, a, u32::MAX) else {
            return;
        };
        let Some(j) = self.get_or_insert(now, b, i) else {
            return;
        };
        self.half_edge(i, j, now);
        self.half_edge(j, i, now);
    }

    /// Records `a`'s own list of its links: refreshes those edges and,
    /// unless the list was cut short (`complete` false), drops the other
    /// edges learned at `a`.
    pub(crate) fn set_links(&mut self, now: Time, a: ID, links: &[ID], complete: bool) {
        if a == self.me {
            return;
        }
        for b in links {
            self.learn_edge(now, a, *b);
        }
        if !complete {
            return;
        }
        let Some(&i) = self.by_id.get(&a) else {
            return;
        };
        let stale: Vec<u32> = match self.nodes.get(i as usize).and_then(Option::as_ref) {
            Some(n) => n
                .edges
                .iter()
                .filter(|e| {
                    self.nodes
                        .get(e.to as usize)
                        .and_then(Option::as_ref)
                        .is_none_or(|m| !links.contains(&m.id))
                })
                .map(|e| e.to)
                .collect(),
            None => return,
        };
        for j in stale {
            if let Some(n) = self.node_mut(i) {
                n.edges.retain(|e| e.to != j);
            }
            if let Some(m) = self.node_mut(j) {
                m.edges.retain(|e| e.to != i);
            }
        }
    }

    /// Drops edges older than the ttl and nodes left without edges.
    pub(crate) fn prune(&mut self, now: Time) {
        let ttl = self.ttl;
        let mut empty = Vec::new();
        for (i, slot) in self.nodes.iter_mut().enumerate() {
            if let Some(n) = slot {
                n.edges.retain(|e| now - e.seen <= ttl);
                if n.edges.is_empty() {
                    // invariant: the slab has fewer than u32::MAX slots.
                    empty.push(i as u32);
                }
            }
        }
        for i in empty {
            self.remove_node(i);
        }
    }

    /// Hop distances to `dst` over edges younger than the ttl, up to
    /// `max_depth`, never through this host. Read them with
    /// [`Topology::distance`] until the next search. Returns false when
    /// `dst` isn't in the graph.
    pub(crate) fn search(&mut self, now: Time, dst: &ID, max_depth: u8) -> bool {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.mark.iter_mut().for_each(|m| *m = 0);
            self.epoch = 1;
        }
        let Some(&start) = self.by_id.get(dst) else {
            return false;
        };
        let len = self.nodes.len();
        if self.mark.len() < len {
            self.mark.resize(len, 0);
            self.dist.resize(len, 0);
        }
        let epoch = self.epoch;
        let ttl = self.ttl;
        let Topology {
            nodes,
            mark,
            dist,
            queue,
            ..
        } = self;
        queue.clear();
        if let (Some(m), Some(d)) = (mark.get_mut(start as usize), dist.get_mut(start as usize)) {
            *m = epoch;
            *d = 0;
        }
        queue.push_back(start);
        while let Some(x) = queue.pop_front() {
            let dx = dist.get(x as usize).copied().unwrap_or(u8::MAX);
            if dx >= max_depth {
                continue;
            }
            let Some(Some(n)) = nodes.get(x as usize) else {
                continue;
            };
            for e in &n.edges {
                if now - e.seen > ttl {
                    continue;
                }
                if let (Some(m), Some(d)) =
                    (mark.get_mut(e.to as usize), dist.get_mut(e.to as usize))
                    && *m != epoch
                {
                    *m = epoch;
                    *d = dx + 1;
                    queue.push_back(e.to);
                }
            }
        }
        true
    }

    /// `id`'s distance found by the last [`Topology::search`].
    pub(crate) fn distance(&self, id: &ID) -> Option<u8> {
        let &i = self.by_id.get(id)?;
        (self.mark.get(i as usize) == Some(&self.epoch))
            .then(|| self.dist.get(i as usize).copied())
            .flatten()
    }

    /// How many learned links `id` has (expired ones included until the
    /// next prune).
    pub(crate) fn degree(&self, id: &ID) -> usize {
        self.by_id
            .get(id)
            .and_then(|&i| self.nodes.get(i as usize)?.as_ref())
            .map_or(0, |n| n.edges.len())
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, id: &ID) -> bool {
        self.by_id
            .get(id)
            .and_then(|&i| self.nodes.get(i as usize)?.as_ref())
            .is_some_and(|n| n.id == *id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    fn id(i: u64) -> ID {
        ID([i, 0, 0, 0])
    }

    fn t(s: i64) -> Time {
        Time::from_unix(1_000_000 + s, 0)
    }

    #[test]
    fn shortest_distances_avoid_this_host() {
        let me = id(0);
        let mut g = Topology::new(me, 16, 4, Duration::MINUTE);
        // 1 - 2 - 3 - 4, and 1 - 5 - 4; edges at me are ignored.
        for (a, b) in [(1, 2), (2, 3), (3, 4), (1, 5), (5, 4), (0, 4), (0, 1)] {
            g.learn_edge(t(0), id(a), id(b));
        }
        assert!(!g.contains(&me));
        assert!(g.search(t(1), &id(4), 5));
        assert_eq!(g.distance(&id(4)), Some(0));
        assert_eq!(g.distance(&id(3)), Some(1));
        assert_eq!(g.distance(&id(5)), Some(1));
        assert_eq!(g.distance(&id(1)), Some(2));
        assert_eq!(g.distance(&id(2)), Some(2));
        // Depth-limited.
        assert!(g.search(t(1), &id(4), 1));
        assert_eq!(g.distance(&id(1)), None);
        assert_eq!(g.distance(&id(3)), Some(1));
        // Unknown destinations.
        assert!(!g.search(t(1), &id(9), 5));
        assert_eq!(g.distance(&id(3)), None);
    }

    #[test]
    fn announced_links_replace_learned_ones() {
        let mut g = Topology::new(id(0), 16, 8, Duration::MINUTE);
        g.learn_edge(t(0), id(1), id(2));
        g.learn_edge(t(0), id(1), id(3));
        g.set_links(t(1), id(1), &[id(3), id(4), id(0)], true);
        assert!(g.search(t(1), &id(1), 1));
        assert_eq!(g.distance(&id(2)), None);
        assert_eq!(g.distance(&id(3)), Some(1));
        assert_eq!(g.distance(&id(4)), Some(1));
        // A cut-short list only adds.
        g.set_links(t(2), id(1), &[id(5)], false);
        assert!(g.search(t(2), &id(1), 1));
        assert_eq!(g.distance(&id(3)), Some(1));
        assert_eq!(g.distance(&id(5)), Some(1));
    }

    #[test]
    fn edges_age_out() {
        let mut g = Topology::new(id(0), 16, 4, Duration::MINUTE);
        g.learn_edge(t(0), id(1), id(2));
        g.learn_edge(t(30), id(2), id(3));
        assert!(g.search(t(61), &id(3), 5));
        assert_eq!(g.distance(&id(2)), Some(1));
        assert_eq!(g.distance(&id(1)), None);
        g.prune(t(61));
        assert!(!g.contains(&id(1)));
        assert_eq!(g.len(), 2);
        g.prune(t(91));
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn nodes_and_degrees_are_bounded() {
        let mut g = Topology::new(id(0), 4, 2, Duration::MINUTE);
        // A star around 1: only its two newest edges stay.
        for (k, b) in [2, 3, 4].into_iter().enumerate() {
            g.learn_edge(t(k as i64), id(1), id(b));
        }
        assert_eq!(g.degree(&id(1)), 2);
        assert_eq!(g.degree(&id(2)), 0);
        assert!(g.search(t(5), &id(2), 5));
        assert_eq!(g.distance(&id(1)), None);
        // A fifth node evicts the least recently seen one (2).
        g.learn_edge(t(10), id(4), id(5));
        assert_eq!(g.len(), 4);
        assert!(!g.contains(&id(2)));
        assert!(g.contains(&id(5)));
        // Many more stay within the bound.
        for i in 10..100u64 {
            g.learn_edge(t(10 + i as i64), id(i), id(i + 1));
        }
        assert_eq!(g.len(), 4);
        assert!(g.nodes.len() <= 4);
    }
}
