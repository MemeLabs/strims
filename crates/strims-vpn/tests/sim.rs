// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic simulations of vpn networks.
//!
//! `simulation`: 32 hosts are linked by XOR distance
//! (`World::link_kademlia`): each sorts the others by XOR
//! distance from its vnic host id and links to its four nearest, then two
//! at ranks 10–11 and one at rank 24, so about 70 % of pairs are several
//! hops apart. (With half as many links this layout is disconnected.)
//! Links have seeded latencies (5–50 ms) and every
//! vpn frame is lost with probability 5 %. The clock is virtual and jumps
//! to the next delivery or deadline. It checks unicast delivery under loss,
//! the hop limit, deduplication, hash table gets after a publish, peer
//! index searches, peer exchange between unlinked hosts, and that the same
//! seed replays the same trace.
//!
//! The routing sweep runs unstructured link graphs (random regular,
//! small-world, hub-and-relay; ids unrelated to the links), 2 to 8 links
//! per host, 32 or 64 hosts, with and without frame loss, under
//! [`Routing::Greedy`], [`Routing::Learned`] and two mixes of the two. It
//! measures unicasts to hosts nobody has heard from, replies, unicasts
//! after some traffic, hash table gets and peer index searches, what each
//! costs in frames, how often a unicast took a longer path than the
//! shortest, and what link announcements cost and teach. The `sweep_*`
//! tests check the targets on a few scenarios; `routing_sweep` (ignored,
//! run it in release) prints the whole table, and `announcement_sweep`
//! (ignored) compares announcement reach 1 and 2 in mixed networks.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{World, rand_below, rand_f64};
use strims_base::time::Duration;
use strims_crypto::key_from_seed;
use strims_kademlia::ID;
use strims_vpn::{Config, Event, ExchangeOutcome, HashTableGetOptions, Routing};

const NODES: usize = 32;
const PEERS_PER_NODE: usize = 8;
const LOSS: f64 = 0.05;
const UNICASTS_PER_NODE: usize = 6;

#[derive(Debug, Default, PartialEq)]
struct Report {
    links: usize,
    multi_hop_pairs: usize,
    unicast_sent: usize,
    unicast_delivered: usize,
    max_hops: usize,
    duplicates_delivered: usize,
    duplicates_dropped: u64,
    broadcast_reach: Vec<usize>,
    ht_gets: usize,
    ht_found: usize,
    pi_searches: usize,
    pi_found: usize,
    exchanges: usize,
    exchanges_linked: usize,
    frames_lost: u64,
    verify_failures: u64,
}

fn run(seed: u64) -> (Report, Vec<String>) {
    // Verification is covered by `tests/host.rs`; auditing every hop here
    // would multiply the debug-build run time.
    let mut w = World::new(seed, NODES, Config::default());
    w.loss = LOSS;
    w.link_kademlia(PEERS_PER_NODE, &mut |rng| {
        Duration::from_millis(5 + rand_below(rng, 46) as i64)
    });
    w.run_for(Duration::from_secs(2));
    let mut r = Report {
        links: w.wires.len(),
        ..Report::default()
    };
    // Pairs without a direct link.
    for a in 0..NODES {
        for b in a + 1..NODES {
            if !w.linked(a, b) {
                r.multi_hop_pairs += 1;
            }
        }
    }

    // Unicast: each host to a few random others, encrypted.
    let mut sent = BTreeSet::new();
    for a in 0..NODES {
        for k in 0..UNICASTS_PER_NODE {
            let mut b = rand_below(&mut w.rng, NODES - 1);
            if b >= a {
                b += 1;
            }
            let body = format!("u {a} {b} {k}");
            w.send(a, b, body.as_bytes());
            sent.insert((b, body));
            w.run_for(Duration::from_millis(3));
        }
    }
    w.run_for(Duration::from_secs(3));
    r.unicast_sent = sent.len();
    let mut seen: BTreeMap<(usize, String), usize> = BTreeMap::new();
    for i in 0..NODES {
        for (_, e) in &w.nodes[i].events {
            if let Event::Message(d) = e {
                r.max_hops = r.max_hops.max(d.hops);
                *seen
                    .entry((i, String::from_utf8_lossy(&d.body).into_owned()))
                    .or_default() += 1;
            }
        }
        w.nodes[i].events.clear();
    }
    r.unicast_delivered = sent.iter().filter(|k| seen.contains_key(k)).count();
    r.duplicates_delivered = seen.values().filter(|&&n| n > 1).count();

    // Broadcasts: everyone they reach delivers them once.
    for k in 0..3 {
        let a = rand_below(&mut w.rng, NODES);
        let body = format!("b {k}");
        w.broadcast(a, body.as_bytes());
        w.run_for(Duration::from_secs(1));
        let mut reach = 0;
        for i in 0..NODES {
            let n = w.nodes[i]
                .events
                .iter()
                .filter(|(_, e)| matches!(e, Event::Message(d) if d.body == body.as_bytes()))
                .count();
            if n > 1 {
                r.duplicates_delivered += 1;
            }
            reach += n.min(1);
            w.nodes[i].events.clear();
        }
        r.broadcast_reach.push(reach);
    }

    // Hash table: one host publishes, others ask the network.
    let key = key_from_seed(&[0x61; 32]);
    let publisher = rand_below(&mut w.rng, NODES);
    let (net, now) = (w.nodes[publisher].net, w.now);
    w.nodes[publisher]
        .vpn
        .hash_table_set(now, net, &key, b"salt", b"value")
        .unwrap();
    w.service(publisher);
    w.run_for(Duration::from_secs(2));
    let opts = HashTableGetOptions {
        disable_cache: true,
        timeout: Duration::from_secs(5),
    };
    for _ in 0..10 {
        let g = rand_below(&mut w.rng, NODES);
        if g == publisher {
            continue;
        }
        let (net, now) = (w.nodes[g].net, w.now);
        w.nodes[g]
            .vpn
            .hash_table_get(now, net, &key.public, b"salt", opts)
            .unwrap();
        w.service(g);
        w.run_for(Duration::from_secs(6));
        r.ht_gets += 1;
        let found = w
            .take_events(g)
            .iter()
            .any(|e| matches!(e, Event::HashTableValue { value, .. } if value == b"value"));
        r.ht_found += usize::from(found);
    }

    // Peer index: one host publishes to its neighbours, others search.
    let holder = rand_below(&mut w.rng, NODES);
    let (net, now) = (w.nodes[holder].net, w.now);
    w.nodes[holder]
        .vpn
        .peer_index_publish(now, net, b"swarm", b"", 7000)
        .unwrap();
    w.service(holder);
    w.run_for(Duration::from_secs(2));
    let holder_id = w.id(holder);
    for _ in 0..10 {
        let s = rand_below(&mut w.rng, NODES);
        if s == holder {
            continue;
        }
        let (net, now) = (w.nodes[s].net, w.now);
        w.nodes[s]
            .vpn
            .peer_index_search(now, net, b"swarm", b"")
            .unwrap();
        w.service(s);
        w.run_for(Duration::from_secs(61));
        r.pi_searches += 1;
        let ev = w.take_events(s);
        let found = ev
            .iter()
            .any(|e| matches!(e, Event::PeerIndexHost { host, .. } if host.host_id == holder_id && host.port == 7000));
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::PeerIndexSearchClosed { .. }))
        );
        r.pi_found += usize::from(found);
    }

    // Peer exchange between unlinked hosts.
    for _ in 0..4 {
        let a = rand_below(&mut w.rng, NODES);
        let Some(b) =
            (0..NODES).find(|&b| b != a && !w.linked(a, b) && !w.nodes[a].vnic.has_peer(&w.id(b)))
        else {
            continue;
        };
        let (net, now, id) = (w.nodes[a].net, w.now, w.id(b));
        let node = &mut w.nodes[a];
        node.vpn.connect(&node.vnic, now, net, id).unwrap().unwrap();
        w.service(a);
        w.run_for(Duration::from_secs(25));
        r.exchanges += 1;
        let ev = w.take_events(a);
        assert!(ev.iter().any(
            |e| matches!(e, Event::GatherLocalDescriptions { host_id, .. } if *host_id == id)
        ));
        // Only this exchange: `a` may have been an earlier exchange's
        // target, with its events still queued.
        let closed: Vec<&ExchangeOutcome> = ev
            .iter()
            .filter_map(|e| match e {
                Event::ExchangeClosed {
                    outcome, host_id, ..
                } if *host_id == id => Some(outcome),
                _ => None,
            })
            .collect();
        assert_eq!(closed.len(), 1, "{ev:?}");
        if w.linked(a, b) && w.nodes[a].vnic.has_peer(&id) {
            assert!(matches!(closed[0], ExchangeOutcome::RemoteConnected));
            r.exchanges_linked += 1;
        }
    }

    r.frames_lost = w.frames_lost;
    for n in &w.nodes {
        let m = n.vpn.metrics();
        r.duplicates_dropped += m.duplicates;
        r.verify_failures += m.verify_failures;
    }
    (r, w.trace)
}

#[test]
fn simulation() {
    let (r, trace) = run(1);
    eprintln!("{r:#?}");
    assert!(r.multi_hop_pairs > NODES * (NODES - 1) / 2 - r.links - 1);
    assert!(r.links >= NODES, "{r:?}");

    // Unicast under 5 % frame loss.
    let rate = r.unicast_delivered as f64 / r.unicast_sent as f64;
    assert!(rate >= 0.95, "unicast delivery {rate:.3} ({r:?})");
    // Hop limit and dedup.
    assert!(r.max_hops <= strims_vpn::message::MAX_MESSAGE_HOPS);
    assert!(r.max_hops >= 2);
    assert_eq!(r.duplicates_delivered, 0);
    assert!(r.duplicates_dropped > 0);
    // A broadcast is a bounded fan-out towards its destination id: five
    // links, then four from each of those, and so on.
    assert!(
        r.broadcast_reach
            .iter()
            .all(|&n| n > strims_vpn::message::MAX_MESSAGE_REPLICAS),
        "{r:?}"
    );
    // Services.
    assert!(
        r.ht_found * 10 >= r.ht_gets * 9,
        "hash table {}/{}",
        r.ht_found,
        r.ht_gets
    );
    assert!(
        r.pi_found * 10 >= r.pi_searches * 8,
        "peer index {}/{}",
        r.pi_found,
        r.pi_searches
    );
    assert!(r.exchanges >= 3);
    assert_eq!(r.exchanges_linked, r.exchanges, "{r:?}");
    assert!(r.frames_lost > 0);

    // Same seed, same trace; another seed, another trace.
    let (r2, trace2) = run(1);
    assert_eq!(r, r2);
    assert_eq!(trace, trace2);
    let (_, trace3) = run(2);
    assert_ne!(trace, trace3);
}

// --- routing sweep -----------------------------------------------------------

/// Link graphs for the sweep. Node `i`'s id is random (from its key), so
/// the graph has no relation to XOR distance: links come from the app, not
/// from Kademlia.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Topo {
    /// Every host has exactly `d` links, chosen at random.
    RandomRegular,
    /// Watts–Strogatz: a ring lattice (each host linked to the next
    /// `d / 2` around the ring, plus the opposite host for odd `d`) whose
    /// non-ring links are rewired to random hosts with probability 0.25.
    SmallWorld,
    /// `n / 16` hubs (bootstrap servers) linked to each other; `n / 4`
    /// relays with one hub link each; every relay and leaf has `d` links
    /// in all, the rest chosen at random among relays and leaves.
    HubAndRelay,
    /// [`World::link_kademlia`]: each host links to its `d / 2` XOR-nearest
    /// hosts, then sparser farther ones. Built from the ids, so it is the
    /// one layout that suits greedy XOR routing; hosts end up with about
    /// `1.5 d` links.
    Kademlia,
    /// Random links between even and odd hosts only, `d` each. With
    /// [`Mode::Mixed`] every link joins a greedy host and a learned one, so
    /// no two learned hosts are linked.
    Bipartite,
}

impl Topo {
    const ALL: [Topo; 4] = [
        Topo::RandomRegular,
        Topo::SmallWorld,
        Topo::HubAndRelay,
        Topo::Kademlia,
    ];

    fn name(self) -> &'static str {
        match self {
            Topo::RandomRegular => "random-regular",
            Topo::SmallWorld => "small-world",
            Topo::HubAndRelay => "hub-and-relay",
            Topo::Kademlia => "kademlia",
            Topo::Bipartite => "bipartite",
        }
    }
}

#[derive(Clone, Debug)]
struct Graph {
    adj: Vec<BTreeSet<usize>>,
}

impl Graph {
    fn new(n: usize) -> Graph {
        Graph {
            adj: vec![BTreeSet::new(); n],
        }
    }

    fn add(&mut self, a: usize, b: usize) -> bool {
        if a == b || self.adj[a].contains(&b) {
            return false;
        }
        self.adj[a].insert(b);
        self.adj[b].insert(a);
        true
    }

    fn remove(&mut self, a: usize, b: usize) {
        self.adj[a].remove(&b);
        self.adj[b].remove(&a);
    }

    fn edges(&self) -> Vec<(usize, usize)> {
        let mut e = Vec::new();
        for (a, s) in self.adj.iter().enumerate() {
            for &b in s.range(a + 1..) {
                e.push((a, b));
            }
        }
        e
    }

    /// Hop distances from `s` (`usize::MAX`: unreachable).
    fn distances(&self, s: usize) -> Vec<usize> {
        let mut d = vec![usize::MAX; self.adj.len()];
        let mut q = std::collections::VecDeque::from([s]);
        d[s] = 0;
        while let Some(x) = q.pop_front() {
            for &y in &self.adj[x] {
                if d[y] == usize::MAX {
                    d[y] = d[x] + 1;
                    q.push_back(y);
                }
            }
        }
        d
    }

    /// Component label per node.
    fn components(&self) -> Vec<usize> {
        let mut c = vec![usize::MAX; self.adj.len()];
        let mut next = 0;
        for s in 0..self.adj.len() {
            if c[s] != usize::MAX {
                continue;
            }
            for (i, d) in self.distances(s).into_iter().enumerate() {
                if d != usize::MAX {
                    c[i] = next;
                }
            }
            next += 1;
        }
        c
    }
}

/// Adds random links so node `i` gains `extra[i]` links (no self links or
/// duplicates), then joins components by swapping two added links
/// `(a, b), (c, d)` for `(a, c), (b, d)`, which keeps every degree.
fn add_random_links(rng: &mut strims_base::rng::DetRng, g: &mut Graph, extra: &[usize]) {
    'attempt: for _ in 0..1000 {
        let mut h = g.clone();
        let mut added = BTreeSet::new();
        let mut stubs: Vec<usize> = extra
            .iter()
            .enumerate()
            .flat_map(|(i, &k)| std::iter::repeat_n(i, k))
            .collect();
        while !stubs.is_empty() {
            let a = stubs.swap_remove(rand_below(rng, stubs.len()));
            let valid: Vec<usize> = (0..stubs.len())
                .filter(|&j| stubs[j] != a && !h.adj[a].contains(&stubs[j]))
                .collect();
            if valid.is_empty() {
                continue 'attempt;
            }
            let b = stubs.swap_remove(valid[rand_below(rng, valid.len())]);
            h.add(a, b);
            added.insert((a.min(b), a.max(b)));
        }
        for _ in 0..1000 {
            let comp = h.components();
            let Some(other) = (0..comp.len()).find(|&i| comp[i] != comp[0]) else {
                *g = h;
                return;
            };
            let in_comp = |c: usize| -> Vec<(usize, usize)> {
                added
                    .iter()
                    .copied()
                    .filter(|&(a, _)| comp[a] == c)
                    .collect()
            };
            let (x, y) = (in_comp(comp[0]), in_comp(comp[other]));
            if x.is_empty() || y.is_empty() {
                continue 'attempt;
            }
            let (a, b) = x[rand_below(rng, x.len())];
            let (c, d) = y[rand_below(rng, y.len())];
            h.remove(a, b);
            h.remove(c, d);
            added.remove(&(a, b));
            added.remove(&(c, d));
            h.add(a, c);
            h.add(b, d);
            added.insert((a.min(c), a.max(c)));
            added.insert((b.min(d), b.max(d)));
        }
    }
    panic!("no connected graph for degrees {extra:?}");
}

/// A connected random graph whose links all join an even host and an odd
/// one, `d` links per host.
fn bipartite(rng: &mut strims_base::rng::DetRng, n: usize, d: usize) -> Graph {
    for _ in 0..1000 {
        let mut g = Graph::new(n);
        let mut odd: Vec<usize> = (0..n)
            .filter(|i| i % 2 == 1)
            .flat_map(|i| std::iter::repeat_n(i, d))
            .collect();
        let mut ok = true;
        for a in (0..n).filter(|i| i % 2 == 0) {
            for _ in 0..d {
                let valid: Vec<usize> = (0..odd.len())
                    .filter(|&j| !g.adj[a].contains(&odd[j]))
                    .collect();
                if valid.is_empty() {
                    ok = false;
                    break;
                }
                let b = odd.swap_remove(valid[rand_below(rng, valid.len())]);
                g.add(a, b);
            }
        }
        if ok && g.components().iter().all(|&c| c == 0) {
            return g;
        }
    }
    panic!("no connected bipartite graph for {n} hosts of degree {d}");
}

fn topology(rng: &mut strims_base::rng::DetRng, topo: Topo, ids: &[ID], d: usize) -> Graph {
    let n = ids.len();
    let mut g = Graph::new(n);
    match topo {
        Topo::Bipartite => g = bipartite(rng, n, d),
        Topo::Kademlia => {
            for i in 0..n {
                let mut sorted: Vec<usize> = (0..n).collect();
                sorted.sort_by_key(|&j| ids[i] ^ ids[j]);
                let (mut added, mut off, mut k) = (0, 1, d / 2);
                while added < d && k != 0 {
                    for &j in sorted.iter().skip(off).take(k) {
                        if g.add(i, j) {
                            added += 1;
                        }
                    }
                    off = (off + k) * 2;
                    k /= 2;
                }
            }
        }
        Topo::RandomRegular => add_random_links(rng, &mut g, &vec![d; n]),
        Topo::SmallWorld => {
            let mut lattice = Vec::new();
            for i in 0..n {
                g.add(i, (i + 1) % n);
                for j in 2..=d / 2 {
                    lattice.push((i, (i + j) % n));
                }
                if d % 2 == 1 && i < n / 2 {
                    lattice.push((i, i + n / 2));
                }
            }
            for (a, mut b) in lattice {
                if rand_f64(rng) < 0.25 {
                    for _ in 0..100 {
                        let c = rand_below(rng, n);
                        if c != a && !g.adj[a].contains(&c) {
                            b = c;
                            break;
                        }
                    }
                }
                g.add(a, b);
            }
        }
        Topo::HubAndRelay => {
            let hubs = (n / 16).max(2);
            let relays = n / 4;
            for a in 0..hubs {
                for b in a + 1..hubs {
                    g.add(a, b);
                }
            }
            let mut extra = vec![0; n];
            for (i, e) in extra.iter_mut().enumerate().skip(hubs) {
                if i < hubs + relays {
                    g.add(i, i % hubs);
                    *e = d - 1;
                } else {
                    *e = d;
                }
            }
            add_random_links(rng, &mut g, &extra);
        }
    }
    g
}

/// Which routing the hosts run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Every host runs [`Routing::Greedy`]: the baseline.
    Greedy,
    /// Every host runs [`Routing::Learned`].
    Learned,
    /// Even hosts run [`Routing::Greedy`], odd hosts [`Routing::Learned`].
    Mixed,
    /// One host in four runs [`Routing::Learned`], the rest
    /// [`Routing::Greedy`]: most learned hosts are separated by greedy
    /// ones.
    Minority,
}

impl Mode {
    fn routing(self, i: usize) -> Routing {
        match self {
            Mode::Greedy => Routing::Greedy,
            Mode::Learned => Routing::Learned,
            Mode::Mixed if i.is_multiple_of(2) => Routing::Greedy,
            Mode::Mixed => Routing::Learned,
            Mode::Minority if i % 4 == 1 => Routing::Learned,
            Mode::Minority => Routing::Greedy,
        }
    }
}

#[derive(Clone, Debug)]
struct Scenario {
    topo: Topo,
    nodes: usize,
    links: usize,
    loss: f64,
    mode: Mode,
    seed: u64,
    /// Unicasts per phase (about), keys published per service, and
    /// lookups per key.
    unicasts: usize,
    keys: u8,
    lookups: usize,
    /// Every host's configuration, routing aside.
    config: Config,
}

impl Scenario {
    fn new(topo: Topo, nodes: usize, links: usize, loss: f64, mode: Mode) -> Scenario {
        Scenario {
            topo,
            nodes,
            links,
            loss,
            mode,
            seed: 7,
            unicasts: 256,
            keys: 8,
            lookups: 4,
            config: Config::default(),
        }
    }

    /// A lighter workload for debug builds.
    fn light(self) -> Scenario {
        Scenario {
            unicasts: 128,
            keys: 4,
            lookups: 4,
            ..self
        }
    }
}

/// Delivered of sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Rate {
    ok: usize,
    of: usize,
}

impl Rate {
    fn add(&mut self, ok: bool) {
        self.of += 1;
        self.ok += usize::from(ok);
    }

    fn pct(self) -> f64 {
        if self.of == 0 {
            return 100.0;
        }
        self.ok as f64 * 100.0 / self.of as f64
    }

    /// [`Rate::pct`], 0 when nothing was counted.
    fn share(self) -> f64 {
        if self.of == 0 { 0.0 } else { self.pct() }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SweepReport {
    /// Cold unicasts whose destination is more than five hops away.
    beyond_hop_limit: usize,
    /// Unicasts sent at once by every host, before anything has been
    /// heard from anyone.
    cold: Rate,
    /// The same, for destinations within five hops.
    cold_reachable: Rate,
    /// Answers to delivered cold unicasts.
    reply: Rate,
    /// Unicasts to random hosts after that traffic, within five hops.
    warm_reachable: Rate,
    /// Hash table gets after a publish.
    ht: Rate,
    /// Peer index searches after a publish.
    pi: Rate,
    /// Frames sent per cold and per warm unicast (all copies, all hosts).
    cold_frames: u64,
    warm_frames: u64,
    /// Frames sent per hash table get and per peer index search
    /// (publishing and every answer included).
    ht_frames: u64,
    pi_frames: u64,
    /// Delivered warm unicasts whose first copy took a longer path than
    /// the shortest one.
    warm_longer: Rate,
    /// Delivered unicasts (cold and warm) to direct neighbours that took
    /// more than one hop.
    direct_longer: Rate,
    /// Frames sent per ten hosts while the links come up (link
    /// announcements; nothing else is sent then).
    announce_frames: u64,
    /// Hosts in the learned link graph, averaged over [`Routing::Learned`]
    /// hosts, before any traffic.
    known: usize,
}

fn frames_sent(w: &World) -> u64 {
    w.nodes.iter().map(|n| n.vpn.metrics().frames_sent).sum()
}

/// Bodies node `i` received on the app port, with the hops each took.
fn received(w: &mut World, i: usize) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for (_, e) in std::mem::take(&mut w.nodes[i].events) {
        if let Event::Message(d) = e {
            m.insert(String::from_utf8_lossy(&d.body).into_owned(), d.hops);
        }
    }
    m
}

fn sweep_run(s: Scenario) -> (SweepReport, Vec<String>) {
    let config = |i: usize| Config {
        routing: s.mode.routing(i),
        ..s.config.clone()
    };
    let n = s.nodes;
    let mut w = World::new_with(s.seed, n, config);
    let mut grng = strims_base::rng::fork(&mut w.rng);
    let ids: Vec<ID> = (0..n).map(|i| w.id(i)).collect();
    let g = topology(&mut grng, s.topo, &ids, s.links);
    for (a, b) in g.edges() {
        let l = Duration::from_millis(5 + rand_below(&mut w.rng, 46) as i64);
        w.link(a, b, l);
    }
    // Handshakes, then link announcements (and the lists they pass on).
    w.run_for(Duration::from_secs(5));
    w.loss = s.loss;
    let dist: Vec<Vec<usize>> = (0..n).map(|i| g.distances(i)).collect();
    let reachable = |a: usize, b: usize| dist[a][b] <= strims_vpn::message::MAX_MESSAGE_HOPS;
    let mut r = SweepReport {
        announce_frames: frames_sent(&w) * 10 / n as u64,
        ..SweepReport::default()
    };
    let learned: Vec<usize> = (0..n)
        .filter(|&i| s.mode.routing(i) == Routing::Learned)
        .collect();
    if !learned.is_empty() {
        let net = w.nodes[0].net;
        let total: usize = learned
            .iter()
            .map(|&i| w.nodes[i].vpn.topology_len(net))
            .sum();
        r.known = total / learned.len();
    }
    let per_node = (s.unicasts / n).max(1);
    let pick = |w: &mut World, a: usize| {
        let mut b = rand_below(&mut w.rng, n - 1);
        if b >= a {
            b += 1;
        }
        b
    };
    let send = |w: &mut World, a: usize, b: usize, body: &str| {
        let (net, now, dst) = (w.nodes[a].net, w.now, w.id(b));
        w.nodes[a]
            .vpn
            .send(
                now,
                net,
                dst,
                common::APP_PORT,
                common::APP_PORT,
                body.as_bytes(),
            )
            .unwrap();
    };

    // Cold unicasts: everyone sends at the same instant.
    let f0 = frames_sent(&w);
    let mut cold = Vec::new();
    for a in 0..n {
        for k in 0..per_node {
            let b = pick(&mut w, a);
            let body = format!("c {a} {b} {k}");
            send(&mut w, a, b, &body);
            cold.push((a, b, k, body));
        }
    }
    for a in 0..n {
        w.service(a);
    }
    w.run_for(Duration::from_secs(4));
    r.cold_frames = (frames_sent(&w) - f0) / cold.len() as u64;
    let got: Vec<BTreeMap<String, usize>> = (0..n).map(|i| received(&mut w, i)).collect();
    let mut replies = Vec::new();
    for (a, b, k, body) in &cold {
        let hops = got[*b].get(body);
        let ok = hops.is_some();
        if let (Some(&h), 1) = (hops, dist[*a][*b]) {
            r.direct_longer.add(h > 1);
        }
        r.cold.add(ok);
        if reachable(*a, *b) {
            r.cold_reachable.add(ok);
        } else {
            r.beyond_hop_limit += 1;
        }
        if ok {
            replies.push((*b, *a, format!("r {b} {a} {k}")));
        }
    }

    // Replies, all at once.
    for (b, a, body) in &replies {
        send(&mut w, *b, *a, body);
    }
    for b in 0..n {
        w.service(b);
    }
    w.run_for(Duration::from_secs(4));
    let got: Vec<BTreeMap<String, usize>> = (0..n).map(|i| received(&mut w, i)).collect();
    for (_, a, body) in &replies {
        r.reply.add(got[*a].contains_key(body));
    }

    // Warm unicasts, spaced out.
    let f0 = frames_sent(&w);
    let mut warm = Vec::new();
    for k in 0..per_node {
        for a in 0..n {
            let b = pick(&mut w, a);
            let body = format!("w {a} {b} {k}");
            w.send(a, b, body.as_bytes());
            warm.push((a, b, body));
            w.run_for(Duration::from_millis(2));
        }
    }
    w.run_for(Duration::from_secs(4));
    r.warm_frames = (frames_sent(&w) - f0) / warm.len() as u64;
    let got: Vec<BTreeMap<String, usize>> = (0..n).map(|i| received(&mut w, i)).collect();
    for (a, b, body) in &warm {
        let hops = got[*b].get(body);
        if let Some(&h) = hops {
            r.warm_longer.add(h > dist[*a][*b]);
            if dist[*a][*b] == 1 {
                r.direct_longer.add(h > 1);
            }
        }
        if reachable(*a, *b) {
            r.warm_reachable.add(hops.is_some());
        }
    }

    // Hash table gets after a publish, by a few other hosts at once.
    let f0 = frames_sent(&w);
    let opts = HashTableGetOptions {
        disable_cache: true,
        timeout: Duration::from_secs(4),
    };
    for p in 0..s.keys {
        let key = key_from_seed(&[0x61 + p; 32]);
        let publisher = rand_below(&mut w.rng, n);
        let (net, now) = (w.nodes[publisher].net, w.now);
        w.nodes[publisher]
            .vpn
            .hash_table_set(now, net, &key, b"salt", b"value")
            .unwrap();
        w.service(publisher);
        w.run_for(Duration::from_secs(2));
        let mut getters = Vec::new();
        while getters.len() < s.lookups {
            let g = pick(&mut w, publisher);
            if getters.contains(&g) {
                continue;
            }
            let (net, now) = (w.nodes[g].net, w.now);
            w.nodes[g]
                .vpn
                .hash_table_get(now, net, &key.public, b"salt", opts)
                .unwrap();
            w.service(g);
            getters.push(g);
        }
        w.run_for(Duration::from_secs(5));
        for g in getters {
            let found = w
                .take_events(g)
                .iter()
                .any(|e| matches!(e, Event::HashTableValue { value, .. } if value == b"value"));
            r.ht.add(found);
        }
    }
    r.ht_frames = (frames_sent(&w) - f0) / r.ht.of as u64;

    // Peer index searches after a publish, by a few other hosts at once.
    let f0 = frames_sent(&w);
    for p in 0..s.keys {
        let swarm = [b's', p];
        let holder = rand_below(&mut w.rng, n);
        let (net, now) = (w.nodes[holder].net, w.now);
        w.nodes[holder]
            .vpn
            .peer_index_publish(now, net, &swarm, b"", 7000)
            .unwrap();
        w.service(holder);
        w.run_for(Duration::from_secs(2));
        let holder_id = w.id(holder);
        let mut searchers = Vec::new();
        while searchers.len() < s.lookups {
            let s = pick(&mut w, holder);
            if searchers.contains(&s) {
                continue;
            }
            let (net, now) = (w.nodes[s].net, w.now);
            w.nodes[s]
                .vpn
                .peer_index_search(now, net, &swarm, b"")
                .unwrap();
            w.service(s);
            searchers.push(s);
        }
        w.run_for(Duration::from_secs(5));
        for s in searchers {
            let found = w.take_events(s).iter().any(
                |e| matches!(e, Event::PeerIndexHost { host, .. } if host.host_id == holder_id),
            );
            r.pi.add(found);
        }
    }
    r.pi_frames = (frames_sent(&w) - f0) / r.pi.of as u64;
    (r, w.trace)
}

fn sweep_table(rows: &[(Scenario, SweepReport)]) -> String {
    let mut out = String::from(
        "| routing | topology | hosts | links | loss | pairs >5 hops | cold | cold ≤5 hops | reply | warm ≤5 hops | ht get | pi search | frames/cold | frames/warm | frames/get | frames/search | warm longer % | direct >1 hop % | ann frames/host | graph |\n",
    );
    out.push_str(
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for (s, r) in rows {
        let reach = match s.config.announce_reach {
            r if r == Config::default().announce_reach => String::new(),
            r => format!(" reach {r}"),
        };
        out.push_str(&format!(
            "| {:?}{} | {} | {} | {} | {:.0}% | {:.0}% | {:.1} | {:.1} | {:.1} | {:.1} | {:.0} | {:.0} | {} | {} | {} | {} | {:.1} | {:.1} | {:.1} | {} |\n",
            s.mode,
            reach,
            s.topo.name(),
            s.nodes,
            s.links,
            s.loss * 100.0,
            r.beyond_hop_limit as f64 * 100.0 / r.cold.of as f64,
            r.cold.pct(),
            r.cold_reachable.pct(),
            r.reply.pct(),
            r.warm_reachable.pct(),
            r.ht.pct(),
            r.pi.pct(),
            r.cold_frames,
            r.warm_frames,
            r.ht_frames,
            r.pi_frames,
            r.warm_longer.share(),
            r.direct_longer.share(),
            r.announce_frames as f64 / 10.0,
            r.known,
        ));
    }
    out
}

/// The targets for [`Routing::Learned`] with no loss and three links or
/// more: every unicast to a host within the hop limit, and every reply.
fn assert_targets(s: &Scenario, r: &SweepReport) {
    if s.mode != Mode::Learned || s.loss > 0.0 || s.links < 3 || s.topo == Topo::Kademlia {
        return;
    }
    assert!(r.cold_reachable.pct() >= 99.0, "{s:?} {r:?}");
    assert!(r.warm_reachable.pct() >= 99.0, "{s:?} {r:?}");
    assert!(r.reply.pct() >= 99.0, "{s:?} {r:?}");
}

/// The sweep's targets on one scenario (with a light workload, for debug
/// builds).
fn sweep_smoke(s: Scenario) -> (SweepReport, Vec<String>) {
    let s = s.light();
    let (r, trace) = sweep_run(s.clone());
    eprintln!("{s:?} {r:?}");
    assert_targets(&s, &r);
    (r, trace)
}

/// Runs scenarios on all cores; each is its own deterministic world, so
/// the results don't depend on the scheduling. Returned in input order.
fn sweep_all(scenarios: Vec<Scenario>) -> Vec<(Scenario, SweepReport)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut out: Vec<(usize, SweepReport)> = std::thread::scope(|sc| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                sc.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(s) = scenarios.get(i) else {
                            return done;
                        };
                        done.push((i, sweep_run(s.clone()).0));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    });
    out.sort_by_key(|(i, _)| *i);
    scenarios
        .into_iter()
        .zip(out)
        .map(|(s, (_, r))| (s, r))
        .collect()
}

#[test]
fn sweep_random_regular() {
    sweep_smoke(Scenario::new(
        Topo::RandomRegular,
        32,
        3,
        0.0,
        Mode::Learned,
    ));
}

#[test]
fn sweep_small_world() {
    sweep_smoke(Scenario::new(Topo::SmallWorld, 32, 3, 0.0, Mode::Learned));
}

/// Also replay: the same seed gives the same trace.
#[test]
fn sweep_hub_and_relay_replays() {
    let s = Scenario::new(Topo::HubAndRelay, 32, 3, 0.0, Mode::Learned);
    let (r, trace) = sweep_smoke(s.clone());
    let (r2, trace2) = sweep_smoke(s);
    assert_eq!(r, r2);
    assert_eq!(trace, trace2);
}

/// Greedy hosts among learned ones.
#[test]
fn sweep_mixed() {
    let (r, _) = sweep_smoke(Scenario::new(Topo::RandomRegular, 32, 4, 0.0, Mode::Mixed));
    assert!(r.cold_reachable.pct() >= 99.0, "{r:?}");
    assert!(r.reply.pct() >= 99.0, "{r:?}");
}

/// The full sweep, with the greedy baseline and mixed networks; prints the
/// table. Slow in debug builds:
/// `cargo test -p strims-vpn --release --test sim -- --ignored --nocapture`.
#[test]
#[ignore]
fn routing_sweep() {
    let mut scenarios = Vec::new();
    for mode in [Mode::Greedy, Mode::Learned, Mode::Mixed, Mode::Minority] {
        for nodes in [32, 64] {
            for topo in Topo::ALL {
                for links in [2, 3, 4, 6, 8] {
                    // The XOR layout is disconnected below four links.
                    if topo == Topo::Kademlia && (links < 4 || nodes != 32) {
                        continue;
                    }
                    for loss in [0.0, 0.05] {
                        scenarios.push(Scenario::new(topo, nodes, links, loss, mode));
                    }
                }
            }
        }
    }
    let rows = sweep_all(scenarios);
    eprintln!("{}", sweep_table(&rows));
    for (s, r) in &rows {
        assert_targets(s, r);
    }
}

/// Link announcements one hop (`M_NORELAY`) against two (relayed once by
/// greedy hosts), in mixed networks; prints the table:
/// `cargo test -p strims-vpn --release --test sim announcement_sweep -- --ignored --nocapture`.
#[test]
#[ignore]
fn announcement_sweep() {
    let mut scenarios = Vec::new();
    for mode in [Mode::Mixed, Mode::Minority] {
        for nodes in [32, 64] {
            for topo in [
                Topo::RandomRegular,
                Topo::SmallWorld,
                Topo::HubAndRelay,
                Topo::Bipartite,
            ] {
                for links in [3, 4, 6, 8] {
                    for loss in [0.0, 0.05] {
                        for reach in [1, 2] {
                            let mut s = Scenario::new(topo, nodes, links, loss, mode);
                            s.config.announce_reach = reach;
                            scenarios.push(s);
                        }
                    }
                }
            }
        }
    }
    let rows = sweep_all(scenarios);
    eprintln!("{}", sweep_table(&rows));
}
