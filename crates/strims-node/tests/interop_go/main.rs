// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Node-level Go↔Rust interop over real localhost sockets.
//!
//! Go nodes are the `golden/interop` helper: the stack cmd/svc's `newVPN`
//! builds (vnic with a TCP mux and a WebSocket listener, vpn with one
//! network and fixed port bindings, a ppspp runner with explicit
//! `RunSwarm`), one subprocess each, driven over stdin/stdout. Rust nodes
//! are [`strims_node::Node`]s on `TokioNet`, in `VerifyMode::Audit`.
//!
//! | test | covers |
//! |---|---|
//! | `dial_matrix` | Rust→Go and Go→Rust dials over TCP and WS: handshake, peer events, PeerInit fields, traffic, close |
//! | `vpn_chain_go_rust_go`, `vpn_chain_rust_go_rust` | unicast, broadcast, relaying through the other implementation, audit, hash table, peer index |
//! | `vpn_mixed_mesh_greedy`, `vpn_mixed_mesh_learned` | a 6-node mixed mesh: delivery rates with Rust's greedy and learned routing |
//! | `peer_exchange_both_ways` | Go asks Rust, Rust asks Go (refused until Go sets max peers) |
//! | `webrtc_exchange_rust_offers`, `webrtc_exchange_go_offers` | a pion node and a Rust node without listeners get a direct WebRTC link through peer exchange; vpn and a ppspp stream over it |
//! | `ppspp_chain_go_rust_go`, `ppspp_chain_rust_go_rust` | seed in one implementation, relay in the other, byte-exact |
//! | `ppspp_capped_rust_seed_go_viewers` | an upload-capped Rust seed with three Go viewers |
//! | `soak_mixed_streaming` (ignored) | 3 minutes: 6 mixed nodes, two streams, vpn traffic |
//!
//! Every test skips when `go` isn't on PATH and is bounded by a timeout;
//! the helpers exit when their stdin closes and after 15 minutes at most.
//! Run with `--nocapture` to see the measured rates.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    // Real subprocesses and sockets.
    clippy::disallowed_methods,
    clippy::disallowed_types,
    // Topologies index nodes pairwise; a handful of test nodes.
    clippy::needless_range_loop,
    clippy::large_enum_variant
)]

mod harness;

use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration as StdDuration;

use harness::{
    GoEvent, GoNode, GoOptions, Impl, Msg, RustNode, RustOptions, TestNode, Transport, helper,
    link, settle,
};
use strims_crypto::key_from_seed;
use strims_node::{ExchangeOutcome, HostId, LinkKind, NodeEvent};
use strims_vpn::{M_BROADCAST, M_ENCRYPT, Routing};
use tokio::time::Instant;

/// Fails the test if `f` takes longer than `secs`.
async fn bounded<T>(secs: u64, f: impl Future<Output = T>) -> T {
    tokio::time::timeout(StdDuration::from_secs(secs), f)
        .await
        .unwrap_or_else(|_| panic!("test timed out after {secs}s"))
}

async fn spawn(bin: &std::path::Path, imp: Impl, name: &str, seed: u8) -> TestNode {
    match imp {
        Impl::Go => {
            let mut o = GoOptions::new(seed);
            o.max_peers = 16;
            TestNode::Go(GoNode::spawn(&bin.to_path_buf(), name, o).await)
        }
        Impl::Rust => TestNode::Rust(RustNode::spawn(name, RustOptions::new(seed)).await),
    }
}

async fn spawn_all(bin: &std::path::Path, imps: &[Impl], seed0: u8) -> Vec<TestNode> {
    let mut nodes = Vec::new();
    for (i, &imp) in imps.iter().enumerate() {
        let name = format!("{}{i}", if imp == Impl::Go { "g" } else { "r" });
        nodes.push(spawn(bin, imp, &name, seed0 + i as u8).await);
    }
    nodes
}

// --- 1. dials ------------------------------------------------------------------

/// The dial cases: (Rust dials, transport).
const DIALS: [(bool, Transport); 4] = [
    (true, Transport::Tcp),
    (true, Transport::Ws),
    (false, Transport::Tcp),
    (false, Transport::Ws),
];

/// Links `r` with one Go node per [`DIALS`] case. Each link completes the
/// handshake on both sides, carries the peers' PeerInit fields, and passes
/// vpn traffic both ways.
async fn dial_cases(bin: &std::path::Path, r: RustNode, seed0: u8) -> (RustNode, Vec<GoNode>) {
    let mut r = TestNode::Rust(r);
    let r_id = r.host_id();
    let mut gos = Vec::new();
    for (i, &(rust_dials, t)) in DIALS.iter().enumerate() {
        let g = GoNode::spawn(
            &bin.to_path_buf(),
            &format!("g{i}"),
            GoOptions::new(seed0 + i as u8),
        )
        .await;
        let (g_id, g_profile) = (g.host_id, g.profile_key.clone());
        let mut g = TestNode::Go(g);
        if rust_dials {
            assert_eq!(r.dial(&g.uri(t)).await, g_id);
        } else {
            assert_eq!(g.dial(&r.uri(t)).await, r_id);
        }
        // Rust's peer event: kind, direction, and Go's PeerInit.
        let NodeEvent::PeerAdded(p) = r
            .rust()
            .wait(
                "peer",
                |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == g_id),
            )
            .await
        else {
            unreachable!()
        };
        assert_eq!(p.kind, t.kind());
        assert_eq!(p.dialed, rust_dials);
        assert_eq!(p.profile_key, g_profile);
        // The helper's PeerInit fields.
        assert_eq!(
            (p.node_platform.as_str(), p.node_version.as_str()),
            ("", "dev")
        );
        // Go's peer event, with Rust's PeerInit.
        let GoEvent::Peer {
            platform, version, ..
        } = g
            .go()
            .wait(
                "peer",
                |e| matches!(e, GoEvent::Peer { host_id, .. } if *host_id == r_id),
            )
            .await
        else {
            unreachable!()
        };
        assert_eq!((platform.as_str(), version.as_str()), ("", "dev"));

        // vpn over the link, both ways: one hop, encrypted, verified.
        let body = format!("r>g{i}");
        r.send(g_id, body.as_bytes()).await;
        let m = g.wait_msg(body.as_bytes()).await;
        assert_eq!((m.src, m.hops, m.flags), (r_id, 1, M_ENCRYPT));
        let body = format!("g{i}>r");
        g.send(r_id, body.as_bytes()).await;
        let m = r.wait_msg(body.as_bytes()).await;
        assert_eq!(
            (m.src, m.hops, m.flags, m.verified),
            (g_id, 1, M_ENCRYPT, Some(true))
        );
        let TestNode::Go(g) = g else { unreachable!() };
        gos.push(g);
    }
    let TestNode::Rust(r) = r else { unreachable!() };
    (r, gos)
}

/// One Rust node and four Go nodes: Rust dials two of them (TCP, WS), two
/// dial Rust (TCP, WS); then each side closes a link.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dial_matrix() {
    let Some(bin) = helper() else { return };
    bounded(180, async {
        let r = RustNode::spawn("r", RustOptions::new(1)).await;
        let r_id = r.node.host_id();
        let (mut r, mut gos) = dial_cases(&bin, r, 0x41).await;

        let peers = r.node.peers().await.unwrap();
        assert_eq!(peers.len(), 4);
        let m = r.node.metrics().await.unwrap();
        assert_eq!(m.links.len(), 4);
        assert_eq!((m.driver.tcp_dials, m.driver.ws_dials), (1, 1));
        assert_eq!((m.driver.tcp_dial_errors, m.driver.ws_dial_errors), (0, 0));
        assert_eq!(m.driver.accepted, 2);
        assert_eq!(m.driver.accept_errors, 0);
        assert_eq!(m.vpn.verify_failures, 0, "{:?}", m.vpn);

        // Closes: Rust closes the link it dialed over TCP; a Go node that
        // dialed over TCP closes its link.
        let (g0, g2) = (gos[0].host_id, gos[2].host_id);
        assert!(r.node.close_peer(g0).await.unwrap());
        gos[0]
            .wait(
                "peer removed",
                |e| matches!(e, GoEvent::PeerRemoved(h) if *h == r_id),
            )
            .await;
        r.wait(
            "peer removed",
            |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == g0),
        )
        .await;
        gos[2]
            .cmd(&format!("close-peer {}", hex::encode(r_id.to_bytes())))
            .await
            .unwrap();
        r.wait(
            "peer removed",
            |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == g2),
        )
        .await;
        let left: Vec<HostId> = r
            .node
            .peers()
            .await
            .unwrap()
            .iter()
            .map(|p| p.host_id)
            .collect();
        assert_eq!(left.len(), 2);
        assert!(left.contains(&gos[1].host_id) && left.contains(&gos[3].host_id));
        r.node.shutdown().await.unwrap();
    })
    .await;
}

/// The same four links, plus a Rust↔Rust TCP link, left idle for 32 s,
/// past every 25 s read deadline. Every link survives and still carries
/// traffic both ways: the Rust node sends keepalives, a TCP link to a peer
/// that has never sent one outwaits the read timeout, and WebSocket pings
/// are answered both ways.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn idle_links_keepalive() {
    let Some(bin) = helper() else { return };
    bounded(120, async {
        let r = RustNode::spawn("r", RustOptions::new(2)).await;
        let r_id = r.node.host_id();
        let (r, gos) = dial_cases(&bin, r, 0x45).await;
        let mut nodes: Vec<TestNode> = std::iter::once(TestNode::Rust(r))
            .chain(gos.into_iter().map(TestNode::Go))
            .collect();
        nodes.push(TestNode::Rust(
            RustNode::spawn("r2", RustOptions::new(3)).await,
        ));
        link(&mut nodes, 5, 0, Transport::Tcp).await;
        let started = Instant::now();
        settle(&mut nodes, StdDuration::from_secs(32), |_| false).await;
        let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
        for (i, n) in nodes.iter_mut().enumerate() {
            let removed: Vec<HostId> = match n {
                TestNode::Go(g) => g
                    .events()
                    .iter()
                    .filter_map(|e| match e {
                        GoEvent::PeerRemoved(h) => Some(*h),
                        _ => None,
                    })
                    .collect(),
                TestNode::Rust(r) => r
                    .events()
                    .iter()
                    .filter_map(|e| match e {
                        NodeEvent::PeerRemoved { host_id } => Some(*host_id),
                        _ => None,
                    })
                    .collect(),
            };
            assert!(
                removed.is_empty(),
                "n{i} lost {removed:?} after {:.1}s idle",
                started.elapsed().as_secs_f64()
            );
        }
        let m = nodes[0].rust().node.metrics().await.unwrap();
        eprintln!(
            "idle: rust sent {} keepalives, read {}; links {}",
            m.host.keepalive_write_count,
            m.host.keepalive_read_count,
            m.links.len()
        );
        assert_eq!(m.links.len(), 5);
        assert_eq!(m.driver.transport_failures, 0);
        // At least one per idle link per 10 s; only r2 sends them back.
        assert!(m.host.keepalive_write_count >= 10, "{:?}", m.host);
        assert!(m.host.keepalive_read_count >= 2, "{:?}", m.host);
        // Every link still carries traffic, both ways.
        for i in 1..nodes.len() {
            let body = format!("still here {i}");
            nodes[0].send(ids[i], body.as_bytes()).await;
            nodes[i].wait_msg(body.as_bytes()).await;
            let body = format!("back {i}");
            nodes[i].send(r_id, body.as_bytes()).await;
            assert_eq!(nodes[0].wait_msg(body.as_bytes()).await.hops, 1);
        }
    })
    .await;
}

// --- 2/3. vpn chains -----------------------------------------------------------

#[derive(Debug, Default)]
struct AuditCount {
    /// Messages from Go sources delivered to Rust handlers, and how many
    /// failed verification.
    go_delivered: usize,
    go_unverified: usize,
    /// `Metrics::verify_failures` summed over the Rust nodes (every frame
    /// handled, relayed ones included).
    verify_failures: u64,
    frames_received: u64,
}

fn audit(nodes: &[TestNode], msgs: &[(usize, Msg)], acc: &mut AuditCount) {
    let go_ids: Vec<HostId> = nodes
        .iter()
        .filter(|n| n.imp() == Impl::Go)
        .map(|n| n.host_id())
        .collect();
    for (to, m) in msgs {
        if nodes[*to].imp() == Impl::Rust && go_ids.contains(&m.src) {
            acc.go_delivered += 1;
            if m.verified != Some(true) {
                acc.go_unverified += 1;
            }
        }
    }
}

async fn rust_metrics(nodes: &mut [TestNode], acc: &mut AuditCount) {
    for n in nodes.iter_mut() {
        if let TestNode::Rust(r) = n {
            let m = r.node.metrics().await.unwrap();
            acc.verify_failures += m.vpn.verify_failures;
            acc.frames_received += m.vpn.frames_received;
        }
    }
}

/// A 3-node chain n0 – n1 – n2 (n0 dials n1 over `t[0]`, n1 dials n2 over
/// `t[1]`): unicasts between every pair (the ends through the middle),
/// broadcasts from every node, a hash table record and a peer index entry
/// from every node, found by both others.
async fn vpn_chain(imps: [Impl; 3], t: [Transport; 2], seed0: u8) {
    let Some(bin) = helper() else { return };
    let mut nodes = spawn_all(&bin, &imps, seed0).await;
    link(&mut nodes, 0, 1, t[0]).await;
    link(&mut nodes, 1, 2, t[1]).await;
    let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
    let label = format!("{:?}", imps);
    let mut received: Vec<(usize, Msg)> = Vec::new();

    // Unicast, every ordered pair.
    for a in 0..3 {
        for b in 0..3 {
            if a == b {
                continue;
            }
            let body = format!("u {a}>{b}");
            nodes[a].send(ids[b], body.as_bytes()).await;
            let m = nodes[b].wait_msg(body.as_bytes()).await;
            assert_eq!(m.src, ids[a]);
            assert_eq!(m.flags, M_ENCRYPT);
            assert_eq!(m.hops, a.abs_diff(b), "{label}: {body}: {m:?}");
            received.push((b, m));
        }
    }

    // Broadcasts from every node reach the other two.
    for a in 0..3 {
        let body = format!("b {a}");
        nodes[a].broadcast(body.as_bytes()).await;
        for b in 0..3 {
            if a == b {
                continue;
            }
            let m = nodes[b].wait_msg(body.as_bytes()).await;
            assert_eq!(m.src, ids[a]);
            assert_eq!(m.flags, M_BROADCAST);
            assert_eq!(m.hops, a.abs_diff(b), "{label}: {body}: {m:?}");
            received.push((b, m));
        }
    }

    let mut acc = AuditCount::default();
    audit(&nodes, &received, &mut acc);
    rust_metrics(&mut nodes, &mut acc).await;
    eprintln!("{label}: audit {acc:?}");
    assert_eq!(acc.go_unverified, 0, "{label}: {acc:?}");
    assert_eq!(acc.verify_failures, 0, "{label}: {acc:?}");

    // Hash table: every node publishes a record; both others get it.
    for a in 0..3u8 {
        nodes[a as usize]
            .ht_set(0x70 + a, b"chain", format!("value {a}").as_bytes())
            .await;
    }
    for a in 0..3u8 {
        let key = key_from_seed(&[0x70 + a; 32]).public;
        for b in 0..3usize {
            if b == a as usize {
                continue;
            }
            let mut got = None;
            for _ in 0..4 {
                got = nodes[b]
                    .ht_get(&key, b"chain", StdDuration::from_secs(3))
                    .await;
                if got.is_some() {
                    break;
                }
            }
            assert_eq!(
                got.as_deref(),
                Some(format!("value {a}").as_bytes()),
                "{label}: hash table record of n{a} at n{b}"
            );
        }
    }

    // Peer index: every node publishes; both others find it.
    for a in 0..3u16 {
        nodes[a as usize]
            .pi_publish(b"swarm", b"chain", 4000 + a)
            .await;
    }
    tokio::time::sleep(StdDuration::from_millis(500)).await;
    for a in 0..3usize {
        for b in 0..3usize {
            if a == b {
                continue;
            }
            let mut found = Vec::new();
            for _ in 0..4 {
                found = nodes[b]
                    .pi_search(b"swarm", b"chain", ids[a], StdDuration::from_secs(3))
                    .await;
                if found.iter().any(|(h, _)| *h == ids[a]) {
                    break;
                }
            }
            assert!(
                found.contains(&(ids[a], 4000 + a as u16)),
                "{label}: peer index entry of n{a} at n{b}: {found:?}"
            );
        }
    }

    for n in nodes.iter_mut() {
        assert_eq!(n.transfer_errors(), 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn vpn_chain_go_rust_go() {
    bounded(
        240,
        vpn_chain(
            [Impl::Go, Impl::Rust, Impl::Go],
            [Transport::Tcp, Transport::Ws],
            0x10,
        ),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn vpn_chain_rust_go_rust() {
    bounded(
        240,
        vpn_chain(
            [Impl::Rust, Impl::Go, Impl::Rust],
            [Transport::Ws, Transport::Tcp],
            0x20,
        ),
    )
    .await;
}

// --- 2. the mixed mesh ---------------------------------------------------------

/// The 6-node mesh: Go on even indices, Rust on odd ones; a ring plus two
/// chords, links alternating TCP and WS.
const MESH: [Impl; 6] = [
    Impl::Go,
    Impl::Rust,
    Impl::Go,
    Impl::Rust,
    Impl::Go,
    Impl::Rust,
];
const MESH_LINKS: [(usize, usize, Transport); 8] = [
    (0, 1, Transport::Tcp),
    (1, 2, Transport::Ws),
    (2, 3, Transport::Tcp),
    (3, 4, Transport::Ws),
    (4, 5, Transport::Tcp),
    (5, 0, Transport::Ws),
    (1, 4, Transport::Tcp),
    (3, 0, Transport::Ws),
];

#[derive(Debug, Default, Clone, Copy)]
struct Rate {
    sent: usize,
    delivered: usize,
    hops: usize,
    /// Delivered copies that took more hops than the shortest path.
    stretched: usize,
}

/// Shortest-path hop counts between every pair of mesh nodes (BFS).
fn mesh_distances() -> Vec<Vec<usize>> {
    let n = MESH.len();
    let mut d = vec![vec![usize::MAX; n]; n];
    for (s, row) in d.iter_mut().enumerate() {
        row[s] = 0;
        let mut queue = std::collections::VecDeque::from([s]);
        while let Some(u) = queue.pop_front() {
            for &(a, b, _) in &MESH_LINKS {
                let v = if a == u {
                    b
                } else if b == u {
                    a
                } else {
                    continue;
                };
                if row[v] == usize::MAX {
                    row[v] = row[u] + 1;
                    queue.push_back(v);
                }
            }
        }
    }
    d
}

impl Rate {
    fn pct(&self) -> f64 {
        100.0 * self.delivered as f64 / self.sent.max(1) as f64
    }
}

async fn build_mesh(bin: &std::path::Path, seed0: u8, routing: Routing) -> Vec<TestNode> {
    let mut nodes = Vec::new();
    for (i, &imp) in MESH.iter().enumerate() {
        let name = format!("{}{i}", if imp == Impl::Go { "g" } else { "r" });
        let seed = seed0 + i as u8;
        nodes.push(match imp {
            Impl::Go => {
                let mut o = GoOptions::new(seed);
                o.max_peers = 16;
                TestNode::Go(GoNode::spawn(&bin.to_path_buf(), &name, o).await)
            }
            Impl::Rust => {
                let mut o = RustOptions::new(seed);
                o.routing = routing;
                TestNode::Rust(RustNode::spawn(&name, o).await)
            }
        });
    }
    for &(a, b, t) in &MESH_LINKS {
        link(&mut nodes, a, b, t).await;
    }
    nodes
}

/// Every node unicasts to every other node `rounds` times (round 0 cold,
/// the rest after earlier traffic taught next hops), then broadcasts once.
/// Returns the unicast rates per round and per (source, destination)
/// implementation, and the broadcast reach.
async fn mesh_traffic(
    nodes: &mut [TestNode],
    rounds: usize,
    label: &str,
) -> (Vec<Rate>, BTreeMap<(Impl, Impl), Rate>, Rate, AuditCount) {
    let n = nodes.len();
    let ids: Vec<HostId> = nodes.iter().map(|x| x.host_id()).collect();
    let dist = mesh_distances();
    let mut per_round = Vec::new();
    let mut by_pair: BTreeMap<(Impl, Impl), Rate> = BTreeMap::new();
    let mut all: Vec<(usize, Msg)> = Vec::new();
    for round in 0..rounds {
        let mut expect = Vec::new();
        for a in 0..n {
            for b in 0..n {
                if a != b {
                    let body = format!("u{round} {a}>{b}");
                    nodes[a].send(ids[b], body.as_bytes()).await;
                    expect.push((a, b, body.into_bytes()));
                }
            }
        }
        let mut got: Vec<Vec<Msg>> = vec![Vec::new(); n];
        settle(nodes, StdDuration::from_secs(4), |nodes| {
            for (i, x) in nodes.iter_mut().enumerate() {
                got[i].extend(x.take_msgs());
            }
            expect
                .iter()
                .all(|(_, b, body)| got[*b].iter().any(|m| &m.body == body))
        })
        .await;
        let mut rate = Rate::default();
        for (a, b, body) in &expect {
            let pair = by_pair
                .entry((nodes[*a].imp(), nodes[*b].imp()))
                .or_default();
            rate.sent += 1;
            pair.sent += 1;
            if let Some(m) = got[*b].iter().find(|m| &m.body == body) {
                let stretched = usize::from(m.hops > dist[*a][*b]);
                if stretched != 0 {
                    eprintln!(
                        "{label}: round {round}: {a}>{b} took {} hops, shortest {}",
                        m.hops, dist[*a][*b]
                    );
                }
                rate.delivered += 1;
                rate.hops += m.hops;
                rate.stretched += stretched;
                pair.delivered += 1;
                pair.hops += m.hops;
                pair.stretched += stretched;
            } else {
                eprintln!(
                    "{label}: round {round}: lost {}",
                    String::from_utf8_lossy(body)
                );
            }
        }
        for (i, g) in got.into_iter().enumerate() {
            all.extend(g.into_iter().map(|m| (i, m)));
        }
        per_round.push(rate);
    }

    for a in 0..n {
        nodes[a].broadcast(format!("b {a}").as_bytes()).await;
    }
    let mut got: Vec<Vec<Msg>> = vec![Vec::new(); n];
    settle(nodes, StdDuration::from_secs(4), |nodes| {
        for (i, x) in nodes.iter_mut().enumerate() {
            got[i].extend(x.take_msgs());
        }
        (0..n).all(|a| {
            (0..n)
                .filter(|b| *b != a)
                .all(|b| got[b].iter().any(|m| m.body == format!("b {a}").as_bytes()))
        })
    })
    .await;
    let mut bcast = Rate::default();
    for a in 0..n {
        for b in (0..n).filter(|b| *b != a) {
            bcast.sent += 1;
            if let Some(m) = got[b]
                .iter()
                .find(|m| m.body == format!("b {a}").as_bytes())
            {
                bcast.delivered += 1;
                bcast.hops += m.hops;
            }
        }
    }
    for (i, g) in got.into_iter().enumerate() {
        all.extend(g.into_iter().map(|m| (i, m)));
    }

    let mut acc = AuditCount::default();
    audit(nodes, &all, &mut acc);
    rust_metrics(nodes, &mut acc).await;
    (per_round, by_pair, bcast, acc)
}

async fn vpn_mixed_mesh(routing: Routing, seed0: u8) {
    let Some(bin) = helper() else { return };
    let label = format!("mesh/{routing:?}");
    let mut nodes = build_mesh(&bin, seed0, routing).await;
    // Rust's link announcements go out a second after links change.
    tokio::time::sleep(StdDuration::from_secs(2)).await;
    let (rounds, by_pair, bcast, acc) = mesh_traffic(&mut nodes, 3, &label).await;
    for (i, r) in rounds.iter().enumerate() {
        eprintln!(
            "{label}: unicast round {i}: {}/{} ({:.1}%), mean hops {:.2}, {} longer than the shortest path",
            r.delivered,
            r.sent,
            r.pct(),
            r.hops as f64 / r.delivered.max(1) as f64,
            r.stretched
        );
    }
    for ((a, b), r) in &by_pair {
        eprintln!(
            "{label}: {a:?}->{b:?}: {}/{} ({:.1}%), {} stretched",
            r.delivered,
            r.sent,
            r.pct(),
            r.stretched
        );
    }
    eprintln!(
        "{label}: broadcast reach {}/{} ({:.1}%)",
        bcast.delivered,
        bcast.sent,
        bcast.pct()
    );
    eprintln!("{label}: audit {acc:?}");
    let mut dups = 0;
    let mut sent = 0;
    for n in nodes.iter_mut() {
        if let TestNode::Rust(r) = n {
            let m = r.node.metrics().await.unwrap();
            dups += m.vpn.duplicates;
            sent += m.vpn.frames_sent;
            eprintln!("{label}: {} vpn {:?}", r.name, m.vpn);
        }
    }
    eprintln!("{label}: rust duplicates {dups}, rust frames sent {sent}");

    // Every message from a Go node verified on the Rust side.
    assert_eq!(acc.go_unverified, 0, "{label}: {acc:?}");
    assert_eq!(acc.verify_failures, 0, "{label}: {acc:?}");
    // Rates are recorded, not required, except that neighbours always
    // reach each other and the warm rounds lose nothing on this mesh.
    let warm = rounds.last().unwrap();
    assert!(warm.pct() >= 90.0, "{label}: warm delivery {warm:?}");
    for n in nodes.iter_mut() {
        assert_eq!(n.transfer_errors(), 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn vpn_mixed_mesh_greedy() {
    bounded(240, vpn_mixed_mesh(Routing::Greedy, 0x30)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn vpn_mixed_mesh_learned() {
    bounded(240, vpn_mixed_mesh(Routing::Learned, 0x30)).await;
}

// --- 4. peer exchange ----------------------------------------------------------

/// A star around a Go node: Go leaves ga (max peers 16) and gb (max peers
/// 0), Rust leaves ra and rb. ga asks ra to link (Rust dials Go's offered
/// URI); rb asks gb, which refuses until it sets max peers and then dials
/// Rust's offered URI.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn peer_exchange_both_ways() {
    let Some(bin) = helper() else { return };
    bounded(180, async {
        let mut nodes = vec![
            TestNode::Go(GoNode::spawn(&bin, "x", GoOptions::new(0x50)).await),
            TestNode::Go(
                GoNode::spawn(
                    &bin,
                    "ga",
                    GoOptions {
                        max_peers: 16,
                        ..GoOptions::new(0x51)
                    },
                )
                .await,
            ),
            TestNode::Go(GoNode::spawn(&bin, "gb", GoOptions::new(0x52)).await),
            TestNode::Rust(RustNode::spawn("ra", RustOptions::new(0x53)).await),
            TestNode::Rust(RustNode::spawn("rb", RustOptions::new(0x54)).await),
        ];
        link(&mut nodes, 1, 0, Transport::Tcp).await;
        link(&mut nodes, 2, 0, Transport::Ws).await;
        link(&mut nodes, 3, 0, Transport::Tcp).await;
        link(&mut nodes, 4, 0, Transport::Ws).await;
        let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
        let (ga, gb, ra, rb) = (ids[1], ids[2], ids[3], ids[4]);

        // Go asks Rust: ra dials one of ga's listeners.
        nodes[1]
            .go()
            .cmd(&format!("px-connect {}", hex::encode(ra.to_bytes())))
            .await
            .unwrap();
        let NodeEvent::PeerAdded(p) = nodes[3]
            .rust()
            .wait(
                "ga by exchange",
                |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == ga),
            )
            .await
        else {
            unreachable!()
        };
        assert!(p.dialed, "{p:?}");
        assert_eq!(p.kind, LinkKind::Tcp, "Go offers TCP first: {p:?}");
        nodes[1].wait_peer(ra).await;
        nodes[1].send(ra, b"direct ga>ra").await;
        assert_eq!(nodes[3].wait_msg(b"direct ga>ra").await.hops, 1);

        // Rust asks Go: gb allows no new peers yet.
        let net = nodes[4].rust().net;
        assert!(nodes[4].rust().node.connect_peer(net, gb).await.unwrap());
        let NodeEvent::ExchangeClosed { outcome, .. } = nodes[4]
            .rust()
            .wait(
                "refusal",
                |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == gb),
            )
            .await
        else {
            unreachable!()
        };
        assert_eq!(
            outcome,
            ExchangeOutcome::Rejected("unable to add new peers".to_owned())
        );

        // With max peers set, gb dials one of rb's listeners.
        nodes[2].go().cmd("max-peers 16").await.unwrap();
        assert!(nodes[4].rust().node.connect_peer(net, gb).await.unwrap());
        let NodeEvent::ExchangeClosed { outcome, .. } = nodes[4]
            .rust()
            .wait(
                "exchange",
                |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == gb),
            )
            .await
        else {
            unreachable!()
        };
        assert_eq!(outcome, ExchangeOutcome::RemoteConnected);
        let NodeEvent::PeerAdded(p) = nodes[4]
            .rust()
            .wait(
                "gb by exchange",
                |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == gb),
            )
            .await
        else {
            unreachable!()
        };
        assert!(!p.dialed, "{p:?}");
        nodes[2].wait_peer(rb).await;
        nodes[4].send(gb, b"direct rb>gb").await;
        assert_eq!(nodes[2].wait_msg(b"direct rb>gb").await.hops, 1);
        for n in nodes.iter_mut() {
            assert_eq!(n.transfer_errors(), 0);
        }
    })
    .await;
}

// --- 4b. WebRTC through peer exchange ------------------------------------------

/// A Go node (pion, `NewWebRTCInterface` on a UDP mux) and a Rust node,
/// both with WebRTC and no listeners (as browser nodes), meet through a
/// Rust hub (Go dials it over TCP, Rust over WebSocket). One asks the
/// other for a link with peer exchange: the session descriptions go
/// through the hub, and a direct WebRTC link comes up. The hub then shuts
/// down; vpn messages go both ways in one hop, and the offerer streams
/// 4 MiB to the other over the data channel.
async fn webrtc_exchange(rust_offers: bool, seed0: u8) {
    let Some(bin) = helper() else { return };
    let label = if rust_offers {
        "webrtc, rust offers"
    } else {
        "webrtc, go offers"
    };
    let mut nodes = vec![
        TestNode::Rust(RustNode::spawn("hub", RustOptions::new(seed0)).await),
        TestNode::Go(
            GoNode::spawn(
                &bin,
                "g",
                GoOptions {
                    max_peers: 16,
                    listen: false,
                    webrtc: true,
                    ..GoOptions::new(seed0 + 1)
                },
            )
            .await,
        ),
        TestNode::Rust(
            RustNode::spawn(
                "r",
                RustOptions {
                    listen: false,
                    webrtc: true,
                    ..RustOptions::new(seed0 + 2)
                },
            )
            .await,
        ),
    ];
    assert_eq!(nodes[1].go().tcp_uri, "-");
    assert!(nodes[1].go().webrtc_addr.is_some());
    assert_eq!(nodes[2].rust().node.tcp_uri(), None);
    link(&mut nodes, 1, 0, Transport::Tcp).await;
    link(&mut nodes, 2, 0, Transport::Ws).await;
    let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
    let (hub, g, r) = (ids[0], ids[1], ids[2]);

    let started = Instant::now();
    if rust_offers {
        let net = nodes[2].rust().net;
        assert!(nodes[2].rust().node.connect_peer(net, g).await.unwrap());
    } else {
        nodes[1]
            .go()
            .cmd(&format!("px-connect {}", hex::encode(r.to_bytes())))
            .await
            .unwrap();
    }
    // Rust's side: a WebRTC link, "dialed" when it offered.
    let NodeEvent::PeerAdded(p) = nodes[2]
        .rust()
        .wait(
            "g over webrtc",
            |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == g),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!((p.kind, p.dialed), (LinkKind::WebRtc, rust_offers), "{p:?}");
    assert_eq!(p.profile_key, key_from_seed(&[seed0 + 1; 32]).public);
    // Go's side: a peer on a data channel link.
    let GoEvent::Peer { link, .. } = nodes[1]
        .go()
        .wait(
            "r over webrtc",
            |e| matches!(e, GoEvent::Peer { host_id, .. } if *host_id == r),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!(link, "*vnic.dcLink");
    if rust_offers {
        // A Rust offerer reports the link it awaited.
        let NodeEvent::ExchangeClosed { outcome, .. } = nodes[2]
            .rust()
            .wait(
                "exchange",
                |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == g),
            )
            .await
        else {
            unreachable!()
        };
        assert_eq!(outcome, ExchangeOutcome::Connected);
    }
    eprintln!("{label}: link up in {:?}", started.elapsed());

    // Only the WebRTC link from here.
    nodes[0].rust().node.shutdown().await.unwrap();
    nodes[1].wait_peer_removed(hub).await;
    nodes[2].wait_peer_removed(hub).await;
    nodes[1].send(r, b"g>r over webrtc").await;
    assert_eq!(nodes[2].wait_msg(b"g>r over webrtc").await.hops, 1);
    nodes[2].send(g, b"r>g over webrtc").await;
    assert_eq!(nodes[1].wait_msg(b"r>g over webrtc").await.hops, 1);

    // The offerer streams to the other side.
    let (seed, leech) = if rust_offers { (2, 1) } else { (1, 2) };
    let key_seed = seed0 ^ 0x5a;
    let id = nodes[seed].create_swarm("s", key_seed, LIVE_WINDOW).await;
    assert_eq!(nodes[leech].add_swarm("s", key_seed, LIVE_WINDOW).await, id);
    nodes[seed].run_swarm("s", ids[leech], 1, 1).await;
    nodes[leech].run_swarm("s", ids[seed], 1, 1).await;
    nodes[leech].start_read("s", WEBRTC_BYTES).await;
    let started = Instant::now();
    nodes[seed]
        .start_write("s", WEBRTC_BYTES, WEBRTC_RATE)
        .await;
    let deadline = started + StdDuration::from_secs(60);
    nodes[seed].written("s", deadline).await;
    let res = nodes[leech].read_result("s", deadline).await;
    eprintln!(
        "{label}: {:?} read {} bytes in {:.2}s",
        nodes[leech].imp(),
        res.len,
        started.elapsed().as_secs_f64()
    );
    res.assert_exact(label, WEBRTC_BYTES);

    let m = nodes[2].rust().node.metrics().await.unwrap();
    eprintln!(
        "{label}: rust wrote {} read {} link bytes; {} datagrams out, {} in, {} dropped",
        m.driver.bytes_written,
        m.driver.bytes_read,
        m.driver.udp_datagrams_written,
        m.driver.udp_datagrams_read,
        m.driver.udp_send_dropped
    );
    assert_eq!(m.links.len(), 1);
    assert_eq!(m.links[0].kind, LinkKind::WebRtc);
    assert_eq!(m.driver.webrtc_opened, 1);
    assert_eq!(m.driver.udp_send_dropped, 0);
    for n in nodes[1..].iter_mut() {
        assert_eq!(n.transfer_errors(), 0, "{label}: {}", n.name());
    }
}

const WEBRTC_BYTES: u64 = 4 << 20;
const WEBRTC_RATE: u64 = 2 << 20;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webrtc_exchange_rust_offers() {
    bounded(120, webrtc_exchange(true, 0x90)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webrtc_exchange_go_offers() {
    bounded(120, webrtc_exchange(false, 0x98)).await;
}

// --- 5. ppspp ------------------------------------------------------------------

/// 16 Ki chunks: a 16 MiB live window.
const LIVE_WINDOW: usize = 1 << 14;

/// A chain seed n0 – relay n1 – leech n2 (n1 dials n0 over `t[0]`, n2
/// dials n1 over `t[1]`): n0 publishes `total` bytes at `rate`; n1 and n2
/// read from offset 0 and must get every byte.
async fn ppspp_chain(imps: [Impl; 3], t: [Transport; 2], seed0: u8, total: u64, rate: u64) {
    let Some(bin) = helper() else { return };
    let label = format!("ppspp {imps:?}");
    let mut nodes = spawn_all(&bin, &imps, seed0).await;
    link(&mut nodes, 1, 0, t[0]).await;
    link(&mut nodes, 2, 1, t[1]).await;
    let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();

    let key_seed = seed0 ^ 0x5a;
    let id = nodes[0].create_swarm("s", key_seed, LIVE_WINDOW).await;
    assert_eq!(nodes[1].add_swarm("s", key_seed, LIVE_WINDOW).await, id);
    assert_eq!(nodes[2].add_swarm("s", key_seed, LIVE_WINDOW).await, id);
    nodes[0].run_swarm("s", ids[1], 1, 1).await;
    nodes[1].run_swarm("s", ids[0], 1, 1).await;
    nodes[1].run_swarm("s", ids[2], 1, 1).await;
    nodes[2].run_swarm("s", ids[1], 1, 1).await;
    nodes[1].start_read("s", total).await;
    nodes[2].start_read("s", total).await;

    let started = Instant::now();
    nodes[0].start_write("s", total, rate).await;
    let deadline = started + StdDuration::from_secs(90);
    nodes[0].written("s", deadline).await;
    let written = started.elapsed().as_secs_f64();
    for i in [1, 2] {
        let r = nodes[i].read_result("s", deadline).await;
        let took = started.elapsed().as_secs_f64();
        eprintln!(
            "{label}: n{i} ({:?}) read {} bytes in {took:.2}s (written in {written:.2}s)",
            nodes[i].imp(),
            r.len
        );
        r.assert_exact(&format!("{label} n{i}"), total);
    }
    for n in nodes.iter_mut() {
        assert_eq!(n.transfer_errors(), 0, "{label}: {}", n.name());
        if let TestNode::Rust(r) = n {
            let m = r.node.metrics().await.unwrap();
            eprintln!(
                "{label}: {} wrote {} read {} bytes",
                r.name, m.driver.bytes_written, m.driver.bytes_read
            );
            assert_eq!(m.driver.transfer_errors, 0);
        }
    }
}

const CHAIN_BYTES: u64 = 8 << 20;
const CHAIN_RATE: u64 = 2 << 20;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ppspp_chain_go_rust_go() {
    bounded(
        180,
        ppspp_chain(
            [Impl::Go, Impl::Rust, Impl::Go],
            [Transport::Tcp, Transport::Ws],
            0x60,
            CHAIN_BYTES,
            CHAIN_RATE,
        ),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ppspp_chain_rust_go_rust() {
    bounded(
        180,
        ppspp_chain(
            [Impl::Rust, Impl::Go, Impl::Rust],
            [Transport::Ws, Transport::Tcp],
            0x68,
            CHAIN_BYTES,
            CHAIN_RATE,
        ),
    )
    .await;
}

/// A Rust seed capped at 2 MiB/s and three Go viewers that only link to it
/// (so they upload nothing back and all sit at the minimum reciprocity
/// weight): 4 MiB at 512 KiB/s needs 1.5 MiB/s of the cap. Every viewer
/// gets every byte without falling far behind the writer.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ppspp_capped_rust_seed_go_viewers() {
    let Some(bin) = helper() else { return };
    bounded(180, async {
        const CAP: u64 = 2 << 20;
        const TOTAL: u64 = 4 << 20;
        const RATE: u64 = 512 << 10;
        let mut o = RustOptions::new(0x70);
        o.rate_limit = Some(CAP);
        let mut nodes = vec![TestNode::Rust(RustNode::spawn("seed", o).await)];
        for i in 0..3u8 {
            let mut o = GoOptions::new(0x71 + i);
            o.max_peers = 16;
            nodes.push(TestNode::Go(
                GoNode::spawn(&bin, &format!("v{i}"), o).await,
            ));
        }
        let ts = [Transport::Tcp, Transport::Ws, Transport::Tcp];
        for i in 1..4 {
            link(&mut nodes, i, 0, ts[i - 1]).await;
        }
        let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
        let id = nodes[0].create_swarm("s", 0x7a, LIVE_WINDOW).await;
        for i in 1..4 {
            assert_eq!(nodes[i].add_swarm("s", 0x7a, LIVE_WINDOW).await, id);
            nodes[i].run_swarm("s", ids[0], 1, 1).await;
            nodes[0].run_swarm("s", ids[i], 1, 1).await;
            nodes[i].start_read("s", TOTAL).await;
        }
        let started = Instant::now();
        nodes[0].start_write("s", TOTAL, RATE).await;
        let deadline = started + StdDuration::from_secs(90);
        nodes[0].written("s", deadline).await;
        let written = started.elapsed().as_secs_f64();
        let mut finished = Vec::new();
        for i in 1..4 {
            let r = nodes[i].read_result("s", deadline).await;
            let took = started.elapsed().as_secs_f64();
            r.assert_exact(&format!("viewer {i}"), TOTAL);
            finished.push(took);
        }
        let m = nodes[0].rust().node.metrics().await.unwrap();
        let elapsed = started.elapsed().as_secs_f64();
        let sent = m.driver.bytes_written as f64;
        eprintln!(
            "capped seed: written in {written:.2}s; viewers done at {finished:.2?}s; seed sent {sent} bytes ({:.0} B/s over {elapsed:.2}s)",
            sent / elapsed
        );
        assert!(
            sent <= CAP as f64 * (elapsed + 1.0) + 65_536.0,
            "seed exceeded its cap"
        );
        for (i, f) in finished.iter().enumerate() {
            assert!(
                *f - written < 15.0,
                "viewer {} finished {:.2}s after the writer",
                i + 1,
                f - written
            );
        }
        for n in nodes.iter_mut() {
            assert_eq!(n.transfer_errors(), 0);
        }
    })
    .await;
}

/// A seed and a viewer of the other implementation, linked over TCP. Two
/// seconds into a 6 MiB stream the seed closes the link; the viewer dials
/// back over WS, both run the swarm again, and the viewer's reader (which
/// stays open throughout) must still get every byte.
async fn ppspp_reconnect(seed_imp: Impl, seed0: u8) {
    let Some(bin) = helper() else { return };
    const TOTAL: u64 = 6 << 20;
    const RATE: u64 = 1 << 20;
    let viewer_imp = if seed_imp == Impl::Go {
        Impl::Rust
    } else {
        Impl::Go
    };
    let label = format!("reconnect {seed_imp:?} seed");
    let mut nodes = spawn_all(&bin, &[seed_imp, viewer_imp], seed0).await;
    link(&mut nodes, 1, 0, Transport::Tcp).await;
    let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
    let key_seed = seed0 ^ 0x5a;
    let id = nodes[0].create_swarm("s", key_seed, LIVE_WINDOW).await;
    assert_eq!(nodes[1].add_swarm("s", key_seed, LIVE_WINDOW).await, id);
    nodes[0].run_swarm("s", ids[1], 1, 1).await;
    nodes[1].run_swarm("s", ids[0], 1, 1).await;
    nodes[1].start_read("s", TOTAL).await;
    let started = Instant::now();
    nodes[0].start_write("s", TOTAL, RATE).await;

    tokio::time::sleep(StdDuration::from_secs(2)).await;
    nodes[0].close_peer(ids[1]).await;
    nodes[0].wait_peer_removed(ids[1]).await;
    nodes[1].wait_peer_removed(ids[0]).await;
    let down = started.elapsed().as_secs_f64();
    link(&mut nodes, 1, 0, Transport::Ws).await;
    nodes[0].run_swarm("s", ids[1], 1, 1).await;
    nodes[1].run_swarm("s", ids[0], 1, 1).await;
    let up = started.elapsed().as_secs_f64();

    let deadline = started + StdDuration::from_secs(60);
    nodes[0].written("s", deadline).await;
    let r = nodes[1].read_result("s", deadline).await;
    eprintln!(
        "{label}: link down at {down:.2}s, back at {up:.2}s; read done at {:.2}s",
        started.elapsed().as_secs_f64()
    );
    r.assert_exact(&label, TOTAL);
    for n in nodes.iter_mut() {
        assert_eq!(n.transfer_errors(), 0, "{label}: {}", n.name());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ppspp_reconnect_mid_stream() {
    bounded(180, async {
        ppspp_reconnect(Impl::Rust, 0x78).await;
        ppspp_reconnect(Impl::Go, 0x7c).await;
    })
    .await;
}

// --- 6. soak -------------------------------------------------------------------

/// Three minutes on the 6-node mixed mesh: a Rust seed and a Go seed each
/// stream at 256 KiB/s, every other node reads both streams byte-exact, and
/// vpn unicasts flow between rotating pairs the whole time.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "takes about 3.5 minutes"]
async fn soak_mixed_streaming() {
    let Some(bin) = helper() else { return };
    bounded(600, async {
        const SECS: u64 = 180;
        const RATE: u64 = 256 << 10;
        const TOTAL: u64 = SECS * RATE;
        let mut nodes = build_mesh(&bin, 0x90, Routing::default()).await;
        let ids: Vec<HostId> = nodes.iter().map(|n| n.host_id()).collect();
        let n = nodes.len();
        // Swarm "rs" seeded by r1 on channel 1, "gs" by g0 on channel 2.
        let streams = [("rs", 1usize, 0xa1u8, 1u64), ("gs", 0, 0xa2, 2)];
        for &(name, seed, key, _) in &streams {
            for i in 0..n {
                if i == seed {
                    nodes[i].create_swarm(name, key, LIVE_WINDOW).await;
                } else {
                    nodes[i].add_swarm(name, key, LIVE_WINDOW).await;
                }
            }
        }
        for &(a, b, _) in &MESH_LINKS {
            for &(name, _, _, ch) in &streams {
                nodes[a].run_swarm(name, ids[b], ch, ch).await;
                nodes[b].run_swarm(name, ids[a], ch, ch).await;
            }
        }
        for &(name, seed, _, _) in &streams {
            for i in (0..n).filter(|i| *i != seed) {
                nodes[i].start_read(name, TOTAL).await;
            }
        }
        let started = Instant::now();
        for &(name, seed, _, _) in &streams {
            nodes[seed].start_write(name, TOTAL, RATE).await;
        }

        // vpn traffic: a unicast every 250 ms between rotating pairs.
        let mut sent = Vec::new();
        let mut got: Vec<Vec<Msg>> = vec![Vec::new(); n];
        let mut k = 0usize;
        while started.elapsed() < StdDuration::from_secs(SECS) {
            let a = k % n;
            let b = (a + 1 + (k / n) % (n - 1)) % n;
            let body = format!("soak {k} {a}>{b}");
            nodes[a].send(ids[b], body.as_bytes()).await;
            sent.push((b, body.into_bytes()));
            k += 1;
            tokio::time::sleep(StdDuration::from_millis(250)).await;
            for (i, x) in nodes.iter_mut().enumerate() {
                got[i].extend(x.take_msgs());
            }
        }
        let deadline = Instant::now() + StdDuration::from_secs(120);
        for &(name, seed, _, _) in &streams {
            nodes[seed].written(name, deadline).await;
        }
        let mut results = Vec::new();
        for &(name, seed, _, _) in &streams {
            for i in (0..n).filter(|i| *i != seed) {
                let r = nodes[i].read_result(name, deadline).await;
                results.push((name, i, r, started.elapsed().as_secs_f64()));
            }
        }
        for (i, x) in nodes.iter_mut().enumerate() {
            got[i].extend(x.take_msgs());
        }
        let delivered = sent
            .iter()
            .filter(|(b, body)| got[*b].iter().any(|m| &m.body == body))
            .count();
        eprintln!(
            "soak: vpn unicasts delivered {delivered}/{} ({:.1}%)",
            sent.len(),
            100.0 * delivered as f64 / sent.len() as f64
        );
        for (name, i, r, t) in &results {
            eprintln!(
                "soak: {name} at {} done at {t:.1}s: {r:?}",
                nodes[*i].name()
            );
        }
        for (name, i, r, _) in &results {
            r.assert_exact(&format!("soak {name} at {}", nodes[*i].name()), TOTAL);
        }
        let mut acc = AuditCount::default();
        let all: Vec<(usize, Msg)> = got
            .iter()
            .enumerate()
            .flat_map(|(i, ms)| ms.iter().map(move |m| (i, m.clone())))
            .collect();
        audit(&nodes, &all, &mut acc);
        rust_metrics(&mut nodes, &mut acc).await;
        eprintln!("soak: audit {acc:?}");
        assert_eq!(acc.go_unverified, 0);
        assert_eq!(acc.verify_failures, 0);
        for x in nodes.iter_mut() {
            assert_eq!(x.transfer_errors(), 0, "{}", x.name());
            if let TestNode::Rust(r) = x {
                let m = r.node.metrics().await.unwrap();
                eprintln!("soak: {} driver {:?}", r.name, m.driver);
                assert_eq!(m.driver.transport_failures, 0, "{}", r.name);
            }
        }
        // Every link survived (TCP idle deadlines, WS pings both ways).
        for x in nodes.iter_mut() {
            let removed = match x {
                TestNode::Go(g) => g
                    .events()
                    .iter()
                    .filter(|e| matches!(e, GoEvent::PeerRemoved(_)))
                    .count(),
                TestNode::Rust(r) => r
                    .events()
                    .iter()
                    .filter(|e| matches!(e, NodeEvent::PeerRemoved { .. }))
                    .count(),
            };
            assert_eq!(removed, 0, "{} lost peers", x.name());
        }
        assert!(delivered * 100 >= sent.len() * 95);
    })
    .await;
}
