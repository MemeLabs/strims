// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Transfers on whole nodes, under turmoil.
//!
//! - b is a bootstrap node; s and v join the network through it, which
//!   binds every peer (`auto_bind_port`).
//! - s publishes a swarm on the network; v adds the same swarm and
//!   publishes it too. Neither dials the other: v finds s in the
//!   network's peer index and asks its node to link through peer
//!   exchange.
//! - Once linked, the two announce the transfer to each other and bind a
//!   channel; nobody calls `Node::run_swarm`. v reads the whole stream.
//! - s removes the transfer, and v's channel to s closes.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace, and another seed a
//! different one.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::key_from_seed;
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{
    Clock, DeliveryMode, HostId, NetworkId, NetworkOptions, Node, NodeConfig, NodeEvent, SimClock,
    SwarmId, SwarmKey, SwarmOptions, TurmoilNet, WriterOptions,
};
use strims_proto::types::Key;
use strims_transfer::{Config, TransferControl};
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
const VPN_PORT: u16 = 60;
const STREAM_LEN: usize = 256 * 1024;
const SALT: &[u8] = b"salt";

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Transfers = TransferControl<NodeHost, NodeTimer<SimClock>>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    /// v has read the whole stream.
    read: Rc<Cell<bool>>,
    /// v has seen its channel to s close.
    done: Rc<Cell<bool>>,
}

impl World {
    fn new(seed: u64) -> World {
        World {
            seed,
            trace: Rc::default(),
            ids: Rc::default(),
            read: Rc::default(),
            done: Rc::default(),
        }
    }

    fn log(&self, clock: &SimClock, name: &str, line: impl Into<String>) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {name} {}", line.into()));
    }

    fn key(&self, name: &str) -> Key {
        let mut s = [name.as_bytes()[0]; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn swarm_key(&self) -> Key {
        self.key("swarm")
    }

    fn stream(&self) -> Vec<u8> {
        let mut rng = det_rng(self.seed ^ 0x5eed);
        let mut b = vec![0u8; STREAM_LEN];
        rand_core::Rng::fill_bytes(&mut rng, &mut b);
        b
    }

    fn uri(&self, name: &str) -> String {
        let ip = turmoil::lookup(name);
        format!(
            "tcp://{ip}:{TCP_PORT}/{}",
            hex::encode(&self.key(name).public)
        )
    }

    fn name(&self, id: &HostId) -> String {
        self.ids
            .borrow()
            .iter()
            .find(|(_, i)| *i == id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default()
    }

    async fn id(&self, name: &str) -> HostId {
        loop {
            if let Some(id) = self.ids.borrow().get(name) {
                return *id;
            }
            tokio::time::sleep(StdDuration::from_millis(1)).await;
        }
    }
}

fn swarm_options() -> SwarmOptions {
    SwarmOptions {
        live_window: 1 << 10,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    }
}

/// A node on the shared network, serving transfers to its peers.
struct Started {
    node: Node,
    net: NetworkId,
    clock: SimClock,
    transfers: Transfers,
}

async fn start(w: &World, name: &str, index: u64) -> Started {
    let ip: IpAddr = turmoil::lookup(name);
    let mut config = NodeConfig::new(w.key(name), w.seed.wrapping_add(index));
    config.label = name.to_owned();
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.public_ip = Some(ip);
    config.max_peers = 16;
    let clock = SimClock::turmoil(epoch());
    let (node, mut events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(name.to_owned(), node.host_id());
    let net = node
        .add_network(
            &key_from_seed(&[0xee; 32]).public,
            NetworkOptions {
                auto_bind_port: Some(VPN_PORT),
            },
        )
        .await
        .unwrap();
    let transfers = TransferControl::new(
        NodeHost::new(node.clone()),
        NodeTimer(clock),
        Config {
            search_interval: Duration::from_secs(5),
            ..Config::default()
        },
    );
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    let services = PeerServices::new(node.clone()).with(Rc::new(transfers.clone()));
    let (w2, me) = (w.clone(), name.to_owned());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            let line = match &e {
                NodeEvent::PeerAdded(p) => Some(format!("peer+ {}", w2.name(&p.host_id))),
                NodeEvent::PeerRemoved { host_id } => Some(format!("peer- {}", w2.name(host_id))),
                NodeEvent::NetworkPeerAdded { host_id, .. } => {
                    Some(format!("net+ {}", w2.name(host_id)))
                }
                NodeEvent::NetworkPeerRemoved { host_id, .. } => {
                    Some(format!("net- {}", w2.name(host_id)))
                }
                NodeEvent::TransferError { host_id, error } => {
                    Some(format!("transfer error {} {error}", w2.name(host_id)))
                }
                _ => None,
            };
            if let Some(line) = line {
                w2.log(&clock, &me, line);
            }
            services.handle_event(&e).await;
        }
    });
    Started {
        node,
        net,
        clock,
        transfers,
    }
}

/// The channel the node runs `swarm` on with `peer`, if any.
async fn node_channel(node: &Node, peer: HostId, swarm: SwarmKey) -> Option<(u64, u64)> {
    let m = node.metrics().await.unwrap();
    m.runner
        .peers
        .iter()
        .filter(|p| p.host_id == peer)
        .flat_map(|p| p.channels.iter())
        .find(|c| c.swarm == swarm)
        .map(|c| (c.channel, c.peer_channel))
}

/// Waits until `cond` holds, checking every 100 ms of simulated time.
async fn until<F: AsyncFnMut() -> bool>(what: &str, limit: StdDuration, mut cond: F) {
    let wait = async {
        while !cond().await {
            tokio::time::sleep(StdDuration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(limit, wait)
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

async fn bootstrap(w: World) -> turmoil::Result {
    let _b = start(&w, "b", 0).await;
    std::future::pending::<()>().await;
    Ok(())
}

async fn seed(w: World) -> turmoil::Result {
    let h = start(&w, "s", 1).await;
    h.node.dial(&w.uri("b")).await.unwrap();
    let (swarm, id, mut writer) = h
        .node
        .create_swarm(WriterOptions {
            swarm_options: swarm_options(),
            key: w.swarm_key(),
        })
        .await
        .unwrap();
    let t = h.transfers.add(swarm, &id, SALT.to_vec());
    assert!(h.transfers.publish(t, h.net));
    w.log(&h.clock, "s", format!("published {t}"));

    let v = w.id("v").await;
    until("v to bind", StdDuration::from_secs(120), async || {
        h.transfers.channels(v, t).is_some()
    })
    .await;
    let channels = h.transfers.channels(v, t).unwrap();
    w.log(&h.clock, "s", format!("bound with v {channels:?}"));
    for chunk in w.stream().chunks(32 * 1024) {
        writer.write(chunk).await.unwrap();
        tokio::time::sleep(StdDuration::from_millis(20)).await;
    }
    writer.flush().await.unwrap();
    assert_eq!(node_channel(&h.node, v, swarm).await, Some(channels));

    until(
        "v to read the stream",
        StdDuration::from_secs(120),
        async || w.read.get(),
    )
    .await;
    assert!(h.transfers.remove(t));
    w.log(&h.clock, "s", "removed");
    until(
        "v's channel to close",
        StdDuration::from_secs(60),
        async || w.done.get(),
    )
    .await;
    assert_eq!(node_channel(&h.node, v, swarm).await, None);
    let m = h.transfers.metrics();
    w.log(
        &h.clock,
        "s",
        format!(
            "metrics announces_sent={} answered={} runs={} stops={} closes_sent={} index_publishes={}",
            m.announces_sent,
            m.announces_answered,
            m.runs,
            m.stops,
            m.closes_sent,
            m.index_publishes
        ),
    );
    assert_eq!((m.runs, m.stops), (1, 1));
    Ok(())
}

async fn viewer(w: World) -> turmoil::Result {
    let h = start(&w, "v", 2).await;
    h.node.dial(&w.uri("b")).await.unwrap();
    let id = SwarmId::new(&w.swarm_key().public);
    let swarm = h.node.add_swarm(id.clone(), swarm_options()).await.unwrap();
    let mut reader = h.node.open_reader(swarm).await.unwrap();
    let t = h.transfers.add(swarm, &id, SALT.to_vec());
    // s publishes first, so v's first search can find it
    tokio::time::sleep(StdDuration::from_secs(2)).await;
    assert!(h.transfers.publish(t, h.net));
    w.log(&h.clock, "v", format!("published {t}"));

    let mut out = vec![0u8; STREAM_LEN];
    tokio::time::timeout(StdDuration::from_secs(180), reader.read_exact(&mut out))
        .await
        .expect("the stream arrives")
        .unwrap();
    assert!(out == w.stream(), "the stream arrives intact");
    w.log(
        &h.clock,
        "v",
        format!("stream {}", hex::encode(Sha256::digest(&out))),
    );
    let s = w.id("s").await;
    let peers = h.node.peers().await.unwrap();
    assert!(
        peers.iter().any(|p| p.host_id == s),
        "v linked to s through peer exchange"
    );
    w.read.set(true);

    until("s to close", StdDuration::from_secs(60), async || {
        node_channel(&h.node, s, swarm).await.is_none()
    })
    .await;
    assert_eq!(h.transfers.channels(s, t), None);
    w.log(&h.clock, "v", "closed by s");
    let m = h.transfers.metrics();
    w.log(
        &h.clock,
        "v",
        format!(
            "metrics searches={} connects={} accepted={} closes_received={}",
            m.searches, m.connects, m.announces_accepted, m.closes_received
        ),
    );
    assert!(m.connects >= 1);
    assert_eq!(m.closes_received, 1);
    w.done.set(true);
    Ok(())
}

fn run(seed_value: u64) -> Vec<String> {
    let w = World::new(seed_value);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed_value)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(600))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    let wb = w.clone();
    sim.host("b", move || bootstrap(wb.clone()));
    sim.client("s", seed(w.clone()));
    sim.client("v", viewer(w.clone()));
    sim.run().unwrap();
    w.trace.borrow().clone()
}

fn digest(trace: &[String]) -> String {
    let mut h = Sha256::new();
    for l in trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

#[test]
fn published_swarms_bind_by_announce_after_discovery() {
    let a = run(7);
    for l in &a {
        eprintln!("{l}");
    }
    assert_eq!(digest(&a), digest(&run(7)), "same seed, different traces");
    assert_ne!(digest(&a), digest(&run(8)), "different seeds, same trace");
}
