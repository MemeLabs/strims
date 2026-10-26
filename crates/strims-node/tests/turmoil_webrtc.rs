// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The WebRTC upgrade under turmoil: three hosts on a simulated network
//! with seeded latency, each a real node on `TurmoilNet` and a `SimClock`,
//! WebRTC over turmoil's UDP.
//!
//! - a and c have no listeners, only WebRTC; a dials b over TCP, c dials b
//!   over WebSocket.
//! - a links to c with `connect_peer`: the session descriptions go through
//!   b, and a direct WebRTC link comes up.
//! - b shuts down; a and c share only the new link.
//! - a vpn echo a → c → a in one hop each way.
//! - a publishes a 2 MiB ppspp stream that c downloads over the data
//!   channel and checks byte for byte.
//!
//! `webrtc_exchange_without_a_path` partitions a from c first: the
//! descriptions still go through b, but ICE can't connect, so the exchange
//! times out and both sides close the connection that never opened; after
//! the partition heals, a second exchange links them.
//!
//! **Determinism.** Every host appends what it sees, stamped with
//! simulated time, to one trace, and a wrapper around `TurmoilNet` records
//! every datagram (time, host, direction, addresses, size, first byte and
//! a hash of the contents). Two runs with one seed are compared field by
//! field (see the crate docs, "Determinism"):
//!
//! - reproducible, and hashed: the node trace (events, deliveries, read
//!   progress, link and vpn counters, datagram counts) and the datagram
//!   schedule (when, which host, which direction, which addresses);
//! - not reproducible, and excluded from the hash: datagram contents
//!   (DTLS ECDHE keys from `OsRng`, SCTP tags from `rand::rng()`, DTLS
//!   randoms with the wall clock) and the sizes of the few DTLS handshake
//!   datagrams that carry ECDSA signatures (DER lengths vary), hence also
//!   UDP byte counters.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::sync::mpsc as std_mpsc;
use std::time::Duration as StdDuration;

use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_node::{
    Clock, DeliveryMode, HostId, LinkKind, NetworkId, NetworkOptions, Node, NodeConfig, NodeEvent,
    NodeEvents, SimClock, SwarmId, SwarmKey, SwarmOptions, TcpOptions, TurmoilNet, WebRtcConfig,
    WriterOptions,
};
use strims_proto::types::Key;
use tokio::sync::mpsc;

const TCP_PORT: u16 = 7000;
const WS_PORT: u16 = 7001;
const UDP_PORT: u16 = 7002;
const VPN_PORT: u16 = 10_000;
const APP_PORT: u16 = 20_000;
const STREAM_LEN: usize = 2 * 1024 * 1024;
const STEP: StdDuration = StdDuration::from_secs(30);

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

// --- a TurmoilNet that records datagrams ------------------------------------

/// One datagram as a socket saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Datagram {
    /// Simulated time, nanoseconds.
    t: u128,
    host: &'static str,
    sent: bool,
    remote: SocketAddr,
    len: usize,
    /// The first byte: STUN (0..=3) or the DTLS content type (22 handshake,
    /// 23 application data, 21 alert, 20 change cipher spec).
    kind: u8,
    sha: [u8; 32],
}

/// `TurmoilNet` plus a record of every datagram, sent over a std channel
/// (`Net` must be `Send + Sync`).
#[derive(Clone)]
struct RecNet {
    host: &'static str,
    log: std_mpsc::Sender<Datagram>,
}

impl RecNet {
    fn record(&self, sent: bool, remote: SocketAddr, data: &[u8]) {
        let t = turmoil::sim_elapsed().unwrap_or_default().as_nanos();
        let _ = self.log.send(Datagram {
            t,
            host: self.host,
            sent,
            remote,
            len: data.len(),
            kind: data.first().copied().unwrap_or(0xff),
            sha: Sha256::digest(data).into(),
        });
    }
}

impl strims_node::Net for RecNet {
    type TcpStream = turmoil::net::TcpStream;
    type TcpListener = turmoil::net::TcpListener;
    type UdpSocket = turmoil::net::UdpSocket;

    fn tcp_connect(
        &self,
        addr: String,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send {
        TurmoilNet.tcp_connect(addr)
    }

    fn tcp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::TcpListener>> + Send {
        TurmoilNet.tcp_bind(addr)
    }

    fn tcp_accept<'a>(
        &'a self,
        listener: &'a Self::TcpListener,
    ) -> impl Future<Output = io::Result<(Self::TcpStream, SocketAddr)>> + Send + 'a {
        listener.accept()
    }

    fn tcp_listener_addr(&self, listener: &Self::TcpListener) -> io::Result<SocketAddr> {
        listener.local_addr()
    }

    fn configure_tcp(&self, stream: &Self::TcpStream, o: &TcpOptions) -> io::Result<()> {
        TurmoilNet.configure_tcp(stream, o)
    }

    fn udp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::UdpSocket>> + Send {
        TurmoilNet.udp_bind(addr)
    }

    fn udp_send_to<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a [u8],
        to: SocketAddr,
    ) -> impl Future<Output = io::Result<usize>> + Send + 'a {
        self.record(true, to, buf);
        socket.send_to(buf, to)
    }

    async fn udp_recv_from<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a mut [u8],
    ) -> io::Result<(usize, SocketAddr)> {
        let r = socket.recv_from(buf).await;
        if let Ok((n, from)) = &r {
            self.record(false, *from, &buf[..*n]);
        }
        r
    }

    fn udp_local_addr(&self, socket: &Self::UdpSocket) -> io::Result<SocketAddr> {
        socket.local_addr()
    }
}

// --- the scenario -------------------------------------------------------------

type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<&'static str, HostId>>>,
    nodes: Rc<RefCell<BTreeMap<&'static str, Node>>>,
    /// Set by a when the WebRTC link is up: b shuts down.
    stop_b: Rc<Cell<bool>>,
    log: std_mpsc::Sender<Datagram>,
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

    fn uri(&self, scheme: &str, port: u16) -> String {
        let ip = turmoil::lookup("b");
        format!(
            "{scheme}://{ip}:{port}/{}",
            hex::encode(&self.profile_key(1).public)
        )
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
        live_window: 1 << 12,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    }
}

struct Host {
    name: &'static str,
    node: Node,
    net: NetworkId,
    swarm: Option<SwarmKey>,
    clock: SimClock,
    trace: Trace,
    events: mpsc::UnboundedReceiver<NodeEvent>,
    writer: Option<strims_node::SwarmWriter>,
}

impl Host {
    async fn start(w: &World, name: &'static str, i: u8) -> Host {
        let ip: IpAddr = turmoil::lookup(name);
        let mut config = NodeConfig::new(w.profile_key(i), w.seed.wrapping_add(u64::from(i)));
        config.label = name.to_owned();
        config.public_ip = Some(ip);
        config.allow_insecure_ws = true;
        config.max_peers = 16;
        if name == "b" {
            config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
            config.ws_listen = Some(format!("0.0.0.0:{WS_PORT}"));
        } else {
            // turmoil binds only the wildcard: host candidates advertise
            // `public_ip`.
            config.webrtc = WebRtcConfig::local([format!("0.0.0.0:{UDP_PORT}")]);
        }
        let clock = SimClock::turmoil(epoch());
        let net = RecNet {
            host: name,
            log: w.log.clone(),
        };
        let (node, events) = Node::spawn(config, net, clock).await.unwrap();
        w.ids.borrow_mut().insert(name, node.host_id());
        w.nodes.borrow_mut().insert(name, node.clone());
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
        let id = SwarmId::new(&w.swarm_key().public);
        let (swarm, writer) = match name {
            "a" => {
                let (s, sid, writer) = node
                    .create_swarm(WriterOptions {
                        swarm_options: swarm_options(),
                        key: w.swarm_key(),
                    })
                    .await
                    .unwrap();
                assert_eq!(sid, id);
                (Some(s), Some(writer))
            }
            "c" => (
                Some(node.add_swarm(id, swarm_options()).await.unwrap()),
                None,
            ),
            _ => (None, None),
        };
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::task::spawn_local(auto(
            name,
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

/// Every host's event policy: log the event, run the swarm with a WebRTC
/// peer (channel 1 both ways), echo app messages, and pass events on.
#[allow(clippy::too_many_arguments)]
async fn auto(
    name: &'static str,
    node: Node,
    mut events: NodeEvents,
    net: NetworkId,
    swarm: Option<SwarmKey>,
    clock: SimClock,
    trace: Trace,
    script: mpsc::UnboundedSender<NodeEvent>,
) {
    while let Some(e) = events.recv().await {
        let t = (clock.now() - epoch()).as_millis();
        let line = match &e {
            NodeEvent::PeerAdded(p) => {
                if let Some(s) = swarm
                    && p.kind == LinkKind::WebRtc
                {
                    node.run_swarm(p.host_id, s, 1, 1).await.unwrap();
                }
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

async fn relay_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, "b", 1).await;
    while !w.stop_b.get() {
        tokio::time::sleep(StdDuration::from_millis(5)).await;
    }
    h.log("shutdown".to_owned());
    h.node.shutdown().await.unwrap();
    w.nodes.borrow_mut().remove("b");
    std::future::pending::<()>().await;
    Ok(())
}

async fn seed_host(w: World) -> turmoil::Result {
    let mut h = Host::start(&w, "a", 0).await;
    let b = h.node.dial(&w.uri("tcp", TCP_PORT)).await.unwrap();
    assert_eq!(b, w.id("b").await);
    let c = w.id("c").await;
    // Wait for c's link to b, then link to c through b.
    tokio::time::sleep(StdDuration::from_millis(200)).await;
    assert!(h.node.connect_peer(h.net, c).await.unwrap());
    let NodeEvent::PeerAdded(p) = h
        .wait(
            "c over webrtc",
            |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == c),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!((p.kind, p.dialed), (LinkKind::WebRtc, true));
    let NodeEvent::ExchangeClosed { outcome, .. } = h
        .wait(
            "exchange",
            |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == c),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!(outcome, strims_node::ExchangeOutcome::Connected);

    // Only the new link from here on.
    w.stop_b.set(true);
    h.wait(
        "b gone",
        |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == b),
    )
    .await;
    h.node
        .send(h.net, c, APP_PORT, APP_PORT, b"ping c".to_vec())
        .await
        .unwrap();
    let NodeEvent::Message(d) = h
        .wait(
            "echo",
            |e| matches!(e, NodeEvent::Message(d) if d.body == b"echo ping c"),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!((d.src, d.hops), (c, 1));

    let mut writer = h.writer.take().unwrap();
    for chunk in w.stream().chunks(64 * 1024) {
        writer.write(chunk).await.unwrap();
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    writer.flush().await.unwrap();
    h.log("published".to_owned());
    std::future::pending::<()>().await;
    Ok(())
}

async fn client_host(w: World) -> turmoil::Result {
    let mut h = Host::start(&w, "c", 2).await;
    let mut reader = h.node.open_reader(h.swarm.unwrap()).await.unwrap();
    let b = h.node.dial(&w.uri("ws", WS_PORT)).await.unwrap();
    let a = w.id("a").await;
    // b stops once a has the new link, which may be before c has it.
    let (mut webrtc, mut b_gone) = (None, false);
    while webrtc.is_none() || !b_gone {
        match h
            .wait("a over webrtc and b gone", |e| {
                matches!(e, NodeEvent::PeerAdded(p) if p.host_id == a)
                    || matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == b)
            })
            .await
        {
            NodeEvent::PeerAdded(p) => webrtc = Some(p),
            _ => b_gone = true,
        }
    }
    let p = webrtc.unwrap();
    assert_eq!((p.kind, p.dialed), (LinkKind::WebRtc, false));
    let NodeEvent::Message(d) = h
        .wait(
            "ping",
            |e| matches!(e, NodeEvent::Message(d) if d.body == b"ping c"),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!((d.src, d.hops), (a, 1));

    let mut out = vec![0u8; STREAM_LEN];
    let mut off = 0;
    let mut mark = 0;
    tokio::time::timeout(StdDuration::from_secs(60), async {
        while off < STREAM_LEN {
            off += reader.read(&mut out[off..]).await.unwrap();
            if off / (256 * 1024) > mark {
                mark = off / (256 * 1024);
                h.log(format!("read {off}"));
            }
        }
    })
    .await
    .expect("stream");
    assert!(out == w.stream(), "stream bytes differ");
    h.log(format!("stream {}", hex::encode(Sha256::digest(&out))));

    // a's and c's counters. UDP byte counts are left out: a few DTLS
    // handshake datagrams vary in size between runs.
    let nodes: Vec<(&str, Node)> = w.nodes.borrow().clone().into_iter().collect();
    for (name, node) in nodes {
        let m = node.metrics().await.unwrap();
        assert_eq!(m.links.len(), 1, "{name}");
        assert_eq!(m.links[0].kind, LinkKind::WebRtc, "{name}");
        assert_eq!(m.driver.transfer_errors, 0, "{name}");
        assert_eq!(m.driver.webrtc_opened, 1, "{name}");
        h.log(format!(
            "{name} frames out {} in {} bytes read {} written {} vpn sent {} relayed {} \
             webrtc conns {} opened {} closed {} datagrams out {} in {} dropped {} unrouted {}",
            m.host.frame_write_count,
            m.host.frame_read_count,
            m.driver.bytes_read,
            m.driver.bytes_written,
            m.vpn.messages_sent,
            m.vpn.messages_relayed,
            m.driver.webrtc_connections,
            m.driver.webrtc_opened,
            m.driver.webrtc_closed,
            m.driver.udp_datagrams_written,
            m.driver.udp_datagrams_read,
            m.driver.udp_send_dropped,
            m.driver.udp_unrouted,
        ));
    }
    Ok(())
}

struct Run {
    trace: Vec<String>,
    datagrams: Vec<Datagram>,
}

/// Runs the scenario.
fn run(seed: u64) -> Run {
    let (log, rx) = std_mpsc::channel();
    let w = World {
        seed,
        trace: Rc::new(RefCell::new(Vec::new())),
        ids: Rc::new(RefCell::new(BTreeMap::new())),
        nodes: Rc::new(RefCell::new(BTreeMap::new())),
        stop_b: Rc::new(Cell::new(false)),
        log,
    };
    // str0m's thread-local generator (SDP session ids, ICE tie-breakers,
    // connectivity-check transaction ids): every host runs on this thread.
    strims_node::seed_str0m_thread_rng(seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(120))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    let ws = w.clone();
    sim.host("b", move || relay_host(ws.clone()));
    let ws = w.clone();
    sim.host("a", move || seed_host(ws.clone()));
    sim.client("c", client_host(w.clone()));
    sim.run().unwrap();
    w.nodes.borrow_mut().clear();
    let trace = w.trace.borrow().clone();
    drop(w);
    Run {
        trace,
        datagrams: rx.try_iter().collect(),
    }
}

/// The datagram schedule: everything but size and contents.
fn schedule(d: &Datagram) -> String {
    format!(
        "{} {} {} {}",
        d.t,
        d.host,
        if d.sent { "send" } else { "recv" },
        d.remote
    )
}

/// The trace hash: the node trace and the datagram schedule.
fn digest(r: &Run) -> String {
    let mut h = Sha256::new();
    for l in &r.trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    for d in &r.datagrams {
        h.update(schedule(d).as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

#[test]
fn webrtc_upgrade_same_seed_same_trace() {
    let a = run(7);
    for l in &a.trace {
        eprintln!("{l}");
    }
    let b = run(7);

    // What reproduces: the node trace and the datagram schedule.
    assert_eq!(a.trace, b.trace, "same seed, different node traces");
    assert_eq!(a.datagrams.len(), b.datagrams.len());
    let sa: Vec<String> = a.datagrams.iter().map(schedule).collect();
    let sb: Vec<String> = b.datagrams.iter().map(schedule).collect();
    assert_eq!(sa, sb, "same seed, different datagram schedules");
    assert_eq!(digest(&a), digest(&b));

    // What doesn't: contents, and a few handshake datagrams' sizes.
    let mut size_diffs = Vec::new();
    let mut content_diffs = 0;
    let mut kinds: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
    for (x, y) in a.datagrams.iter().zip(&b.datagrams) {
        assert_eq!(x.kind, y.kind, "datagram kinds differ: {x:?} {y:?}");
        let k = kinds.entry(x.kind).or_default();
        k.0 += 1;
        if x.sha != y.sha {
            content_diffs += 1;
            k.1 += 1;
        }
        if x.len != y.len {
            size_diffs.push((x.kind, x.len.abs_diff(y.len), x.sent));
        }
    }
    let sent = a.datagrams.iter().filter(|d| d.sent).count();
    eprintln!(
        "datagrams: {} ({} sent); contents differ in {content_diffs}; sizes differ in {} \
         (kind, |delta|, sent): {size_diffs:?}",
        a.datagrams.len(),
        sent,
        size_diffs.len()
    );
    eprintln!("by first byte (count, contents differ): {kinds:?}");
    // Only DTLS handshake records (content type 22) change size, by the
    // ECDSA signatures' DER length (70 to 72 bytes).
    assert!(
        size_diffs
            .iter()
            .all(|&(kind, delta, _)| kind == 22 && delta <= 2),
        "{size_diffs:?}"
    );

    let c = run(8);
    assert_ne!(digest(&a), digest(&c), "different seeds, same trace");
    eprintln!(
        "seed 7: {} lines, {} datagrams, {}",
        a.trace.len(),
        a.datagrams.len(),
        digest(&a)
    );
    eprintln!(
        "seed 8: {} lines, {} datagrams, {}",
        c.trace.len(),
        c.datagrams.len(),
        digest(&c)
    );
}

// --- no path between a and c ---------------------------------------------------

async fn partitioned_seed(w: World) -> turmoil::Result {
    let mut h = Host::start(&w, "a", 0).await;
    let b = h.node.dial(&w.uri("tcp", TCP_PORT)).await.unwrap();
    let c = w.id("c").await;
    tokio::time::sleep(StdDuration::from_millis(200)).await;

    // The exchange runs through b, but no datagram gets from a to c.
    turmoil::partition("a", "c");
    assert!(h.node.connect_peer(h.net, c).await.unwrap());
    let NodeEvent::ExchangeClosed { outcome, .. } = h
        .wait(
            "exchange",
            |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == c),
        )
        .await
    else {
        unreachable!()
    };
    h.log(format!("first exchange {outcome:?}"));
    assert!(
        matches!(
            outcome,
            strims_node::ExchangeOutcome::TimedOut | strims_node::ExchangeOutcome::Failed(_)
        ),
        "{outcome:?}"
    );
    tokio::time::sleep(StdDuration::from_secs(1)).await;
    let m = h.node.metrics().await.unwrap();
    assert_eq!(
        (
            m.driver.webrtc_connections,
            m.driver.webrtc_opened,
            m.driver.webrtc_closed
        ),
        (1, 0, 1)
    );
    let peers = h.node.peers().await.unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].host_id, b);

    // Healed: a second exchange links.
    turmoil::repair("a", "c");
    assert!(h.node.connect_peer(h.net, c).await.unwrap());
    let NodeEvent::ExchangeClosed { outcome, .. } = h
        .wait(
            "second exchange",
            |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == c),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!(outcome, strims_node::ExchangeOutcome::Connected);
    let m = h.node.metrics().await.unwrap();
    assert_eq!(
        (m.driver.webrtc_connections, m.driver.webrtc_opened),
        (2, 1)
    );
    std::future::pending::<()>().await;
    Ok(())
}

async fn partitioned_client(w: World) -> turmoil::Result {
    let mut h = Host::start(&w, "c", 2).await;
    h.node.dial(&w.uri("ws", WS_PORT)).await.unwrap();
    let a = w.id("a").await;
    // The answer went out; the connection never opened and is closed when
    // the exchange ends.
    let NodeEvent::ExchangeClosed { outcome, .. } = h
        .wait(
            "exchange",
            |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == a),
        )
        .await
    else {
        unreachable!()
    };
    h.log(format!("first exchange {outcome:?}"));
    let NodeEvent::PeerAdded(p) = h
        .wait(
            "a over webrtc",
            |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == a),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!(p.kind, LinkKind::WebRtc);
    let m = h.node.metrics().await.unwrap();
    assert_eq!(
        (
            m.driver.webrtc_connections,
            m.driver.webrtc_opened,
            m.driver.webrtc_closed
        ),
        (2, 1, 1)
    );
    Ok(())
}

#[test]
fn webrtc_exchange_without_a_path() {
    let (log, _rx) = std_mpsc::channel();
    let w = World {
        seed: 11,
        trace: Rc::new(RefCell::new(Vec::new())),
        ids: Rc::new(RefCell::new(BTreeMap::new())),
        nodes: Rc::new(RefCell::new(BTreeMap::new())),
        stop_b: Rc::new(Cell::new(false)),
        log,
    };
    strims_node::seed_str0m_thread_rng(w.seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(w.seed)
        .simulation_duration(StdDuration::from_secs(120))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    let ws = w.clone();
    sim.host("b", move || relay_host(ws.clone()));
    let ws = w.clone();
    sim.host("a", move || partitioned_seed(ws.clone()));
    sim.client("c", partitioned_client(w.clone()));
    sim.run().unwrap();
    w.nodes.borrow_mut().clear();
    for l in w.trace.borrow().iter() {
        eprintln!("{l}");
    }
}
