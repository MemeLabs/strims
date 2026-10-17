// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A whole-node smoke test under turmoil: three hosts on a simulated
//! network with seeded latency, each running a real node on `TurmoilNet` and
//! a `SimClock`.
//!
//! - n1 dials n0 over TCP; n2 dials n1 over WebSocket.
//! - n2 sends a vpn message to n0 through n1, and n0 echoes it.
//! - n2 reads the hash table record n0 published.
//! - n2 links to n0 through peer exchange.
//! - n0 publishes a 1 MiB ppspp stream under a 512 KiB/s upload cap; n1 and
//!   n2 download it (n2 from both), and n2 checks every byte.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace hash, and another seed a
//! different one.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_node::{
    Clock, DeliveryMode, HashTableGetOptions, HostId, NetworkId, NetworkOptions, Node, NodeConfig,
    NodeEvent, NodeEvents, SimClock, SwarmId, SwarmKey, SwarmOptions, TurmoilNet, WriterOptions,
};
use strims_proto::types::Key;
use tokio::sync::mpsc;

const TCP_PORT: u16 = 7000;
const WS_PORT: u16 = 7001;
const VPN_PORT: u16 = 10_000;
const APP_PORT: u16 = 20_000;
const UPLOAD_CAP: u64 = 512 * 1024;
const STREAM_LEN: usize = 1024 * 1024;
const STEP: StdDuration = StdDuration::from_secs(30);

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Ids = Rc<RefCell<BTreeMap<String, HostId>>>;
type Nodes = Rc<RefCell<BTreeMap<String, Node>>>;

/// Keys and seeds every host derives from the simulation seed.
#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Ids,
    nodes: Nodes,
}

impl World {
    fn profile_key(&self, i: u8) -> Key {
        let mut s = [i; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn swarm_key(&self) -> Key {
        let mut s = [0x55; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn stream(&self) -> Vec<u8> {
        let mut rng = det_rng(self.seed ^ 0x5eed);
        let mut b = vec![0u8; STREAM_LEN];
        rand_core::Rng::fill_bytes(&mut rng, &mut b);
        b
    }

    fn tcp_uri(&self, i: u8) -> String {
        let ip = turmoil::lookup(format!("n{i}"));
        format!(
            "tcp://{ip}:{TCP_PORT}/{}",
            hex::encode(&self.profile_key(i).public)
        )
    }

    fn ws_uri(&self, i: u8) -> String {
        let ip = turmoil::lookup(format!("n{i}"));
        format!(
            "ws://{ip}:{WS_PORT}/{}",
            hex::encode(&self.profile_key(i).public)
        )
    }

    async fn id(&self, i: u8) -> HostId {
        loop {
            if let Some(id) = self.ids.borrow().get(&format!("n{i}")) {
                return *id;
            }
            tokio::time::sleep(StdDuration::from_millis(1)).await;
        }
    }
}

fn swarm_options() -> SwarmOptions {
    SwarmOptions {
        live_window: 1 << 11,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    }
}

/// One host's node, its swarm, and the script's view of its events.
struct Host {
    name: String,
    node: Node,
    net: NetworkId,
    swarm: SwarmKey,
    clock: SimClock,
    trace: Trace,
    events: mpsc::UnboundedReceiver<NodeEvent>,
    writer: Option<strims_node::SwarmWriter>,
}

impl Host {
    async fn start(w: &World, i: u8) -> Host {
        let name = format!("n{i}");
        let ip: IpAddr = turmoil::lookup(name.as_str());
        let mut config = NodeConfig::new(w.profile_key(i), w.seed.wrapping_add(u64::from(i)));
        config.label = name.clone();
        config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
        config.ws_listen = Some(format!("0.0.0.0:{WS_PORT}"));
        config.public_ip = Some(ip);
        config.allow_insecure_ws = true;
        config.max_peers = 16;
        config.rate_limit = Some(UPLOAD_CAP);
        let clock = SimClock::turmoil(epoch());
        let (node, events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
        assert_eq!(node.tcp_uri(), Some(w.tcp_uri(i).as_str()));
        assert_eq!(node.ws_uri(), Some(w.ws_uri(i).as_str()));
        w.ids.borrow_mut().insert(name.clone(), node.host_id());
        w.nodes.borrow_mut().insert(name.clone(), node.clone());

        let net = node
            .add_network(
                &key_from_seed(&[0xee; 32]).public,
                NetworkOptions {
                    auto_bind_port: Some(VPN_PORT),
                },
            )
            .await
            .unwrap();
        node.set_vpn_handler(net, APP_PORT).await.unwrap();
        let (swarm, writer) = if i == 0 {
            let (s, id, writer) = node
                .create_swarm(WriterOptions {
                    swarm_options: swarm_options(),
                    key: w.swarm_key(),
                })
                .await
                .unwrap();
            assert_eq!(id, SwarmId::new(&w.swarm_key().public));
            (s, Some(writer))
        } else {
            let id = SwarmId::new(&w.swarm_key().public);
            (node.add_swarm(id, swarm_options()).await.unwrap(), None)
        };
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::task::spawn_local(auto(
            name.clone(),
            node.clone(),
            events,
            net,
            swarm,
            clock,
            w.trace.clone(),
            tx,
        ));
        Host {
            name,
            node,
            net,
            swarm,
            clock,
            trace: w.trace.clone(),
            events: rx,
            writer,
        }
    }

    fn log(&self, line: String) {
        let t = (self.clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {} {line}", self.name));
    }

    async fn wait(&mut self, what: &str, mut f: impl FnMut(&NodeEvent) -> bool) -> NodeEvent {
        let res = tokio::time::timeout(STEP, async {
            loop {
                let e = self.events.recv().await.expect("node stopped");
                if f(&e) {
                    return e;
                }
            }
        })
        .await;
        res.unwrap_or_else(|_| panic!("{}: timed out waiting for {what}", self.name))
    }
}

/// Every host's event policy: log the event, run the swarm with each new
/// peer (channel 1 both ways), echo app messages, and pass events on to
/// the script.
#[allow(clippy::too_many_arguments)]
async fn auto(
    name: String,
    node: Node,
    mut events: NodeEvents,
    net: NetworkId,
    swarm: SwarmKey,
    clock: SimClock,
    trace: Trace,
    script: mpsc::UnboundedSender<NodeEvent>,
) {
    while let Some(e) = events.recv().await {
        let t = (clock.now() - epoch()).as_millis();
        let line = match &e {
            NodeEvent::PeerAdded(p) => {
                node.run_swarm(p.host_id, swarm, 1, 1).await.unwrap();
                format!("peer+ {} {:?} dialed={}", p.host_id, p.kind, p.dialed)
            }
            NodeEvent::PeerRemoved { host_id } => format!("peer- {host_id}"),
            NodeEvent::Message(d) => {
                if !d.body.starts_with(b"echo") {
                    let mut reply = b"echo ".to_vec();
                    reply.extend_from_slice(&d.body);
                    node.send(net, d.src, APP_PORT, APP_PORT, reply)
                        .await
                        .unwrap();
                }
                format!(
                    "msg {} hops {} {}",
                    d.src,
                    d.hops,
                    String::from_utf8_lossy(&d.body)
                )
            }
            NodeEvent::ExchangeClosed {
                host_id, outcome, ..
            } => format!("exchange {host_id} {outcome:?}"),
            NodeEvent::TransferError { host_id, error } => format!("transfer {host_id} {error}"),
            NodeEvent::NetworkPeerAdded { network, host_id } => {
                format!("net+ {network:?} {host_id}")
            }
            NodeEvent::NetworkPeerRemoved { network, host_id } => {
                format!("net- {network:?} {host_id}")
            }
        };
        trace.borrow_mut().push(format!("{t} {name} {line}"));
        let _ = script.send(e);
    }
}

async fn seed_host(w: World) -> turmoil::Result {
    let mut h = Host::start(&w, 0).await;
    let n1 = w.id(1).await;
    h.wait(
        "n1",
        |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == n1),
    )
    .await;
    h.node
        .hash_table_set(
            h.net,
            key_from_seed(&[0xaa; 32]),
            b"salt".to_vec(),
            b"n0 value".to_vec(),
        )
        .await
        .unwrap();
    let data = w.stream();
    let mut writer = h.writer.take().unwrap();
    for chunk in data.chunks(32 * 1024) {
        writer.write(chunk).await.unwrap();
        tokio::time::sleep(StdDuration::from_millis(20)).await;
    }
    writer.flush().await.unwrap();
    h.log("published".to_owned());
    std::future::pending::<()>().await;
    Ok(())
}

async fn relay_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 1).await;
    let n0 = h.node.dial(&w.tcp_uri(0)).await.unwrap();
    assert_eq!(n0, w.id(0).await);
    let _keep = h;
    std::future::pending::<()>().await;
    Ok(())
}

async fn client_host(w: World) -> turmoil::Result {
    let mut h = Host::start(&w, 2).await;
    let mut reader = h.node.open_reader(h.swarm).await.unwrap();
    let n0 = w.id(0).await;
    let n1 = w.id(1).await;
    // Wait for n1's link to n0, then join through n1 over WebSocket.
    tokio::time::sleep(StdDuration::from_millis(200)).await;
    assert_eq!(h.node.dial(&w.ws_uri(1)).await.unwrap(), n1);

    // vpn round trip through n1.
    h.node
        .send(h.net, n0, APP_PORT, APP_PORT, b"ping n0".to_vec())
        .await
        .unwrap();
    h.wait(
        "echo",
        |e| matches!(e, NodeEvent::Message(d) if d.src == n0 && d.body == b"echo ping n0"),
    )
    .await;

    // n0's hash table record.
    let value = tokio::time::timeout(STEP, async {
        loop {
            let mut get = h
                .node
                .hash_table_get(
                    h.net,
                    key_from_seed(&[0xaa; 32]).public,
                    b"salt".to_vec(),
                    HashTableGetOptions {
                        disable_cache: true,
                        timeout: strims_base::time::Duration::from_secs(1),
                    },
                )
                .await
                .unwrap();
            if let Some(v) = get.next().await {
                return v;
            }
        }
    })
    .await
    .expect("hash table get");
    assert_eq!(value, b"n0 value");
    h.log(format!("ht {}", String::from_utf8_lossy(&value)));

    // Link to n0 through peer exchange; then the stream comes from both.
    assert!(h.node.connect_peer(h.net, n0).await.unwrap());
    h.wait(
        "exchange",
        |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == n0),
    )
    .await;
    assert!(
        h.node
            .peers()
            .await
            .unwrap()
            .iter()
            .any(|p| p.host_id == n0)
    );

    let mut out = vec![0u8; STREAM_LEN];
    let mut off = 0;
    let mut mark = 0;
    tokio::time::timeout(StdDuration::from_secs(60), async {
        while off < STREAM_LEN {
            off += reader.read(&mut out[off..]).await.unwrap();
            if off / (128 * 1024) > mark {
                mark = off / (128 * 1024);
                h.log(format!("read {off}"));
            }
        }
    })
    .await
    .expect("stream");
    assert!(out == w.stream(), "stream bytes differ");
    h.log(format!("stream {}", hex::encode(Sha256::digest(&out))));

    // Every host's counters (n0 and n1 answer from their own runtimes).
    let elapsed = (h.clock.now() - epoch()).as_secs_f64();
    let nodes: Vec<(String, Node)> = w.nodes.borrow().clone().into_iter().collect();
    for (name, node) in nodes {
        let m = node.metrics().await.unwrap();
        assert_eq!(m.links.len(), 2, "{name}");
        assert_eq!(m.driver.transfer_errors, 0, "{name}");
        // The upload cap holds (plus QoS's one-second burst).
        let sent = m.driver.bytes_written as f64;
        assert!(
            sent <= UPLOAD_CAP as f64 * (elapsed + 1.0) + 65_536.0,
            "{name}: {sent}"
        );
        h.log(format!(
            "{name} frames out {} in {} bytes read {} written {} vpn sent {} relayed {}",
            m.host.frame_write_count,
            m.host.frame_read_count,
            m.driver.bytes_read,
            m.driver.bytes_written,
            m.vpn.messages_sent,
            m.vpn.messages_relayed
        ));
        // The runner, network and link state are deterministic too: the
        // trace hash covers them.
        assert_eq!(m.runner.swarms.len(), 1, "{name}");
        assert_eq!(m.runner.peers.len(), 2, "{name}");
        assert_eq!(m.networks.len(), 1, "{name}");
        let state = format!("{:?} {:?} {:?}", m.runner, m.networks, m.links);
        h.log(format!(
            "{name} runner {}",
            hex::encode(Sha256::digest(state.as_bytes()))
        ));
    }
    Ok(())
}

/// Runs the scenario and returns the trace.
fn run(seed: u64) -> Vec<String> {
    let w = World {
        seed,
        trace: Rc::new(RefCell::new(Vec::new())),
        ids: Rc::new(RefCell::new(BTreeMap::new())),
        nodes: Rc::new(RefCell::new(BTreeMap::new())),
    };
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(120))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    let ws = w.clone();
    sim.host("n0", move || seed_host(ws.clone()));
    let ws = w.clone();
    sim.host("n1", move || relay_host(ws.clone()));
    let ws = w.clone();
    sim.client("n2", client_host(ws));
    sim.run().unwrap();
    w.nodes.borrow_mut().clear();
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
fn same_seed_same_trace() {
    let a = run(7);
    for l in &a {
        eprintln!("{l}");
    }
    let b = run(7);
    assert_eq!(digest(&a), digest(&b), "same seed, different traces");
    let c = run(8);
    assert_ne!(digest(&a), digest(&c), "different seeds, same trace");
    eprintln!("seed 7: {} lines, {}", a.len(), digest(&a));
    eprintln!("seed 8: {} lines, {}", c.len(), digest(&c));
}
