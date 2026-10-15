// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic simulation: endpoints on an in-memory UDP network with
//! seeded latency, jitter (so reordering), loss and bandwidth, and a virtual
//! clock that jumps to the next delivery or `poll_timeout`.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::task::Poll;

use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};
use strims_webrtc::{
    CloseReason, ConnId, DATA_CHANNEL_MTU, EndpointConfig, Event, RemoteDescriptionAction, SdpType,
    SessionDescription, WebRtcEndpoint, stun,
};

fn rand_u64(rng: &mut DetRng) -> u64 {
    rand_core::Rng::next_u64(rng)
}

/// Link conditions, per direction.
#[derive(Clone, Copy, Debug)]
struct Conditions {
    /// One-way latency, uniform in this range (ms); per-packet draws give
    /// reordering.
    latency_ms: (i64, i64),
    /// Per mille of datagrams dropped.
    loss_permille: u64,
    /// Bytes per second each sender's uplink serializes; 0 for unlimited.
    bandwidth: i64,
}

const LAN: Conditions = Conditions {
    latency_ms: (5, 10),
    loss_permille: 0,
    bandwidth: 50_000_000,
};

const LOSSY: Conditions = Conditions {
    latency_ms: (20, 80),
    loss_permille: 30,
    bandwidth: 5_000_000,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Trace {
    Datagram {
        at: Time,
        from: SocketAddr,
        to: SocketAddr,
        len: usize,
        /// The first byte: STUN (0..=3) or the DTLS content type.
        kind: u8,
        dropped: bool,
    },
    Event {
        at: Time,
        node: usize,
        what: String,
    },
}

struct Node {
    ep: WebRtcEndpoint,
    /// The node's sockets.
    addrs: Vec<SocketAddr>,
    /// Per connection: bytes to send, bytes sent, bytes received.
    links: BTreeMap<ConnId, Link>,
    closed: Vec<(ConnId, CloseReason)>,
}

#[derive(Default)]
struct Link {
    open: bool,
    mtu: usize,
    out: Vec<u8>,
    sent: usize,
    blocked: bool,
    writable_events: usize,
    short_writes: usize,
    received: Vec<u8>,
    largest_message: usize,
}

/// A fake STUN server: answers binding requests with a mapped address.
struct StunServer {
    addr: SocketAddr,
    /// Mapped address per client socket (a NAT), or silence when `None`.
    mapping: Option<SocketAddr>,
}

struct Sim {
    rng: DetRng,
    now: Time,
    cond: Conditions,
    nodes: Vec<Node>,
    stun: Vec<StunServer>,
    /// Arrival time and sequence → datagram.
    queue: BTreeMap<(Time, u64), (SocketAddr, SocketAddr, Vec<u8>)>,
    seq: u64,
    /// When each sender's uplink is free again.
    uplink_free: BTreeMap<SocketAddr, Time>,
    trace: Vec<Trace>,
    dropped: u64,
    delivered: u64,
    unrouted: u64,
}

impl Sim {
    fn new(seed: u64, cond: Conditions) -> Sim {
        strims_webrtc::seed_str0m_thread_rng(seed);
        Sim {
            rng: det_rng(seed),
            now: Time::from_unix(1_700_000_000, 0),
            cond,
            nodes: Vec::new(),
            stun: Vec::new(),
            queue: BTreeMap::new(),
            seq: 0,
            uplink_free: BTreeMap::new(),
            trace: Vec::new(),
            dropped: 0,
            delivered: 0,
            unrouted: 0,
        }
    }

    fn add_node(&mut self, addrs: &[&str], config: EndpointConfig) -> usize {
        let mut ep = WebRtcEndpoint::new(self.now, config, fork(&mut self.rng));
        let addrs: Vec<SocketAddr> = addrs.iter().map(|a| a.parse().unwrap()).collect();
        for a in &addrs {
            ep.add_local_addr(*a);
        }
        self.nodes.push(Node {
            ep,
            addrs,
            links: BTreeMap::new(),
            closed: Vec::new(),
        });
        self.nodes.len() - 1
    }

    fn send(&mut self, from: SocketAddr, to: SocketAddr, data: Vec<u8>) {
        let dropped = rand_u64(&mut self.rng) % 1000 < self.cond.loss_permille;
        self.trace.push(Trace::Datagram {
            at: self.now,
            from,
            to,
            len: data.len(),
            kind: data.first().copied().unwrap_or(0),
            dropped,
        });
        // The uplink serializes every datagram, lost or not.
        let free = self.uplink_free.entry(from).or_insert(self.now);
        let start = (*free).max(self.now);
        let tx = match self.cond.bandwidth {
            0 => Duration::ZERO,
            bw => Duration::from_nanos((data.len() as i64 + 28) * 1_000_000_000 / bw),
        };
        *free = start + tx;
        if dropped {
            self.dropped += 1;
            return;
        }
        let (lo, hi) = self.cond.latency_ms;
        let latency = lo * 1000 + (rand_u64(&mut self.rng) % ((hi - lo) as u64 * 1000 + 1)) as i64;
        let at = start + tx + Duration::from_micros(latency);
        self.seq += 1;
        self.queue.insert((at, self.seq), (from, to, data));
    }

    /// Moves every node's datagrams to the network, applies app writes and
    /// collects events, until nothing changes.
    fn settle(&mut self) {
        loop {
            let mut progress = false;
            for i in 0..self.nodes.len() {
                while let Some(t) = self.nodes[i].ep.poll_udp_transmit() {
                    progress = true;
                    assert!(
                        self.nodes[i].addrs.contains(&t.source),
                        "transmit from a socket the node doesn't own: {}",
                        t.source
                    );
                    self.send(t.source, t.destination, t.contents);
                }
                while let Some(e) = self.nodes[i].ep.poll_event() {
                    progress = true;
                    self.on_event(i, e);
                }
                progress |= self.write(i);
            }
            if !progress {
                return;
            }
        }
    }

    fn on_event(&mut self, i: usize, e: Event) {
        let now = self.now;
        let what = match &e {
            Event::Data { conn, data } => format!("data {conn:?} {}", data.len()),
            other => format!("{other:?}"),
        };
        self.trace.push(Trace::Event {
            at: now,
            node: i,
            what,
        });
        let node = &mut self.nodes[i];
        match e {
            Event::LocalDescription(_) => {}
            Event::Open { conn, mtu } => {
                let l = node.links.entry(conn).or_default();
                l.open = true;
                l.mtu = mtu;
            }
            Event::Data { conn, data } => {
                let l = node.links.entry(conn).or_default();
                l.largest_message = l.largest_message.max(data.len());
                l.received.extend_from_slice(&data);
            }
            Event::Writable(conn) => {
                let l = node.links.entry(conn).or_default();
                l.blocked = false;
                l.writable_events += 1;
            }
            Event::Closed { conn, reason } => {
                if let Some(l) = node.links.get_mut(&conn) {
                    l.open = false;
                }
                node.closed.push((conn, reason));
            }
        }
    }

    /// Writes pending app bytes; returns whether anything was written.
    fn write(&mut self, i: usize) -> bool {
        let node = &mut self.nodes[i];
        let mut any = false;
        for (&conn, l) in node.links.iter_mut() {
            while l.open && !l.blocked && l.sent < l.out.len() {
                // Odd-sized writes, so messages don't line up with anything.
                let end = (l.sent + 100_003).min(l.out.len());
                let want = end - l.sent;
                let n = node.ep.link_write(conn, &l.out[l.sent..end]).unwrap();
                assert!(n <= want);
                l.sent += n;
                any |= n > 0;
                if n < want {
                    l.blocked = true;
                    l.short_writes += 1;
                }
            }
        }
        any
    }

    fn next_time(&self) -> Option<Time> {
        let net = self.queue.keys().next().map(|(t, _)| *t);
        self.nodes
            .iter()
            .filter_map(|n| n.ep.poll_timeout())
            .chain(net)
            .min()
    }

    /// Advances to the next deadline or delivery. Returns false when
    /// nothing is scheduled.
    fn step(&mut self) -> bool {
        self.settle();
        let Some(next) = self.next_time() else {
            return false;
        };
        self.now = self.now.max(next);
        while let Some(entry) = self.queue.first_entry() {
            if entry.key().0 > self.now {
                break;
            }
            let (from, to, data) = entry.remove();
            self.delivered += 1;
            if let Some(s) = self.stun.iter().find(|s| s.addr == to) {
                if let Some(mapped) = s.mapping {
                    let txn: [u8; 12] = data[8..20].try_into().unwrap();
                    let reply = stun_success(&txn, mapped);
                    let server = s.addr;
                    self.send(server, from, reply);
                }
                continue;
            }
            let now = self.now;
            match self.nodes.iter_mut().find(|n| n.addrs.contains(&to)) {
                Some(n) => {
                    if !n.ep.handle_udp_input(now, from, to, &data) {
                        self.unrouted += 1;
                    }
                }
                None => self.unrouted += 1,
            }
            self.settle();
        }
        let now = self.now;
        for n in &mut self.nodes {
            if n.ep.poll_timeout().is_some_and(|t| t <= now) {
                n.ep.handle_timeout(now);
            }
        }
        true
    }

    fn run_until(&mut self, limit: Duration, mut done: impl FnMut(&Sim) -> bool) {
        let end = self.now + limit;
        self.settle();
        while !done(self) {
            assert!(self.now < end, "simulation timed out at {}", self.now);
            assert!(self.step(), "simulation stalled at {}", self.now);
            self.settle();
        }
    }

    fn local_description(&mut self, i: usize, conn: ConnId) -> SessionDescription {
        let deadline = self.now + Duration::from_secs(30);
        loop {
            let now = self.now;
            match self.nodes[i].ep.poll_local_description(now, conn) {
                Poll::Ready(r) => return r.unwrap(),
                Poll::Pending => {
                    assert!(self.now < deadline, "gathering never finished");
                    assert!(self.step(), "stalled while gathering");
                }
            }
        }
    }

    /// Signals a connection from node `a` (offerer) to node `b`, out of
    /// band as peer exchange would, through the JSON link description.
    fn connect(&mut self, a: usize, b: usize) -> (ConnId, ConnId) {
        let now = self.now;
        let ca = self.nodes[a].ep.create_connection(now).unwrap();
        let cb = self.nodes[b].ep.create_connection(now).unwrap();
        let offer = self.local_description(a, ca);
        assert_eq!(offer.sdp_type, SdpType::Offer);
        let offer =
            SessionDescription::from_link_description(&offer.to_link_description()).unwrap();
        let now = self.now;
        assert_eq!(
            self.nodes[b]
                .ep
                .set_remote_description(now, cb, &offer)
                .unwrap(),
            RemoteDescriptionAction::AwaitInBackground
        );
        let answer = self.local_description(b, cb);
        assert_eq!(answer.sdp_type, SdpType::Answer);
        let answer =
            SessionDescription::from_link_description(&answer.to_link_description()).unwrap();
        let now = self.now;
        assert_eq!(
            self.nodes[a]
                .ep
                .set_remote_description(now, ca, &answer)
                .unwrap(),
            RemoteDescriptionAction::AwaitLink
        );
        self.nodes[a].links.entry(ca).or_default();
        self.nodes[b].links.entry(cb).or_default();
        (ca, cb)
    }

    fn link(&self, i: usize, c: ConnId) -> &Link {
        &self.nodes[i].links[&c]
    }

    fn is_open(&self, i: usize, c: ConnId) -> bool {
        self.nodes[i].links.get(&c).is_some_and(|l| l.open)
    }
}

/// A STUN success response with XOR-MAPPED-ADDRESS (IPv4).
fn stun_success(txn: &[u8; 12], addr: SocketAddr) -> Vec<u8> {
    let SocketAddr::V4(a) = addr else {
        panic!("ipv4 only")
    };
    let cookie = stun::MAGIC_COOKIE.to_be_bytes();
    let mut m = vec![0x01, 0x01, 0, 12];
    m.extend_from_slice(&cookie);
    m.extend_from_slice(txn);
    m.extend_from_slice(&[0, 0x20, 0, 8, 0, 1]);
    m.extend_from_slice(&(a.port() ^ (stun::MAGIC_COOKIE >> 16) as u16).to_be_bytes());
    m.extend(a.ip().octets().iter().zip(cookie).map(|(b, k)| b ^ k));
    m
}

fn no_stun() -> EndpointConfig {
    EndpointConfig {
        ice_servers: Some(vec![]),
        ..EndpointConfig::default()
    }
}

fn payload(seed: u64, n: usize) -> Vec<u8> {
    let mut rng = det_rng(seed);
    let mut v = vec![0u8; n];
    rand_core::Rng::fill_bytes(&mut rng, &mut v);
    v
}

struct Transfer {
    sim: Sim,
    a: usize,
    b: usize,
    ca: ConnId,
    cb: ConnId,
}

/// Two nodes, a link, and `size` bytes each way, checked byte for byte.
fn transfer(seed: u64, cond: Conditions, size: usize) -> Transfer {
    let mut sim = Sim::new(seed, cond);
    let a = sim.add_node(&["10.0.0.1:5000"], no_stun());
    let b = sim.add_node(&["10.0.0.2:6000"], no_stun());
    let (ca, cb) = sim.connect(a, b);
    sim.run_until(Duration::from_secs(30), |s| {
        s.is_open(a, ca) && s.is_open(b, cb)
    });

    let ab = payload(seed ^ 1, size);
    let ba = payload(seed ^ 2, size);
    sim.nodes[a].links.get_mut(&ca).unwrap().out = ab.clone();
    sim.nodes[b].links.get_mut(&cb).unwrap().out = ba.clone();
    sim.run_until(Duration::from_secs(600), |s| {
        s.link(b, cb).received.len() >= size && s.link(a, ca).received.len() >= size
    });
    assert!(sim.link(b, cb).received == ab, "a→b bytes differ");
    assert!(sim.link(a, ca).received == ba, "b→a bytes differ");
    for (i, c) in [(a, ca), (b, cb)] {
        let l = sim.link(i, c);
        assert_eq!(l.mtu, DATA_CHANNEL_MTU);
        assert!(
            l.largest_message <= DATA_CHANNEL_MTU,
            "{}",
            l.largest_message
        );
    }
    assert_eq!(sim.unrouted, 0);
    Transfer { sim, a, b, ca, cb }
}

#[test]
fn handshake_transfer_and_close() {
    let size = 4 << 20;
    let Transfer {
        mut sim,
        a,
        b,
        ca,
        cb,
    } = transfer(1, LAN, size);
    // 4 MiB at most 1 MiB in flight: the sender hit backpressure and was
    // released by Writable.
    for (i, c) in [(a, ca), (b, cb)] {
        let l = sim.link(i, c);
        assert!(l.short_writes > 0 && l.writable_events > 0);
        assert_eq!(l.largest_message, DATA_CHANNEL_MTU);
    }

    let now = sim.now;
    sim.nodes[a].ep.close(now, ca);
    sim.run_until(Duration::from_secs(5), |s| {
        !s.nodes[a].closed.is_empty() && !s.nodes[b].closed.is_empty()
    });
    assert_eq!(sim.nodes[a].closed, vec![(ca, CloseReason::Local)]);
    assert_eq!(sim.nodes[b].closed, vec![(cb, CloseReason::Remote)]);
    assert_eq!(sim.nodes[a].ep.connections().count(), 0);
    assert_eq!(sim.nodes[b].ep.connections().count(), 0);
    assert!(sim.nodes[a].ep.link_write(ca, b"x").is_err());
    // Nothing left to do.
    sim.settle();
    assert_eq!(sim.nodes[a].ep.poll_timeout(), None);
    assert_eq!(sim.nodes[b].ep.poll_timeout(), None);
}

#[test]
fn loss_and_reordering_are_recovered() {
    let size = 3 << 20;
    let Transfer {
        mut sim,
        a,
        b,
        ca,
        cb,
    } = transfer(2, LOSSY, size);
    assert!(sim.dropped > 100, "dropped only {}", sim.dropped);

    // Closing still ends both sides: the close_notify, or (if it was lost)
    // ICE consent failing on the other side.
    let now = sim.now;
    sim.nodes[a].ep.close(now, ca);
    sim.run_until(Duration::from_secs(90), |s| {
        !s.nodes[a].closed.is_empty() && !s.nodes[b].closed.is_empty()
    });
    assert_eq!(sim.nodes[a].closed, vec![(ca, CloseReason::Local)]);
    let (c, reason) = &sim.nodes[b].closed[0];
    assert_eq!(*c, cb);
    assert!(
        matches!(reason, CloseReason::Remote | CloseReason::IceFailed),
        "{reason:?}"
    );
}

/// The same seed gives the same trace: every datagram at the same time
/// between the same sockets with the same fate, and the same events. The
/// only differences allowed are a byte or two in DTLS handshake datagrams,
/// whose ECDHE keys are unseedable (see the crate's Determinism docs). The
/// network here doesn't depend on datagram size; neither does turmoil's.
#[test]
fn same_seed_same_trace() {
    let cond = Conditions {
        bandwidth: 0,
        ..LOSSY
    };
    let run = |seed| {
        let t = transfer(seed, cond, 512 << 10);
        assert!(t.sim.dropped > 0);
        (t.sim.trace, t.sim.now)
    };
    let (t1, end1) = run(7);
    let (t2, end2) = run(7);
    assert_eq!(end1, end2);
    assert_eq!(t1.len(), t2.len());
    let mut size_diffs = 0;
    for (x, y) in t1.iter().zip(&t2) {
        match (x, y) {
            (
                Trace::Datagram {
                    at,
                    from,
                    to,
                    len,
                    kind,
                    dropped,
                },
                Trace::Datagram {
                    at: at2,
                    from: from2,
                    to: to2,
                    len: len2,
                    kind: kind2,
                    dropped: dropped2,
                },
            ) => {
                assert_eq!(
                    (at, from, to, kind, dropped),
                    (at2, from2, to2, kind2, dropped2)
                );
                if len != len2 {
                    // A DTLS handshake record (content type 22).
                    assert_eq!(*kind, 22);
                    assert!(len.abs_diff(*len2) <= 2);
                    size_diffs += 1;
                }
            }
            _ => assert_eq!(x, y),
        }
    }
    assert!(size_diffs <= 4, "{size_diffs} datagrams differ in size");
    let (t3, _) = run(8);
    assert!(t1 != t3);
}

#[test]
fn one_socket_serves_many_connections() {
    // A server with one shared socket linked to three clients; one client
    // has two sockets (two host candidates).
    let mut sim = Sim::new(3, LAN);
    let s = sim.add_node(&["10.0.0.1:443"], no_stun());
    let c1 = sim.add_node(&["10.0.1.1:5000"], no_stun());
    let c2 = sim.add_node(&["10.0.1.2:5000", "10.0.1.2:5001"], no_stun());
    let c3 = sim.add_node(&["10.0.1.3:5000"], no_stun());
    // The server answers c1 and c3 and offers to c2, so both roles share
    // its socket.
    let pairs = [
        (c1, s, sim.connect(c1, s)),
        (s, c2, sim.connect(s, c2)),
        (c3, s, sim.connect(c3, s)),
    ];
    sim.run_until(Duration::from_secs(30), |sim| {
        pairs
            .iter()
            .all(|(x, y, (cx, cy))| sim.is_open(*x, *cx) && sim.is_open(*y, *cy))
    });
    for (k, (x, y, (cx, cy))) in pairs.iter().enumerate() {
        sim.nodes[*x].links.get_mut(cx).unwrap().out = payload(k as u64, 200_000);
        sim.nodes[*y].links.get_mut(cy).unwrap().out = payload(100 + k as u64, 300_000);
    }
    sim.run_until(Duration::from_secs(60), |sim| {
        pairs.iter().all(|(x, y, (cx, cy))| {
            sim.link(*y, *cy).received.len() >= 200_000
                && sim.link(*x, *cx).received.len() >= 300_000
        })
    });
    for (k, (x, y, (cx, cy))) in pairs.iter().enumerate() {
        assert!(sim.link(*y, *cy).received == payload(k as u64, 200_000));
        assert!(sim.link(*x, *cx).received == payload(100 + k as u64, 300_000));
    }
    assert_eq!(sim.nodes[s].ep.connections().count(), 3);
    assert_eq!(sim.unrouted, 0);
}

#[test]
fn server_reflexive_candidates_from_stun() {
    let mut sim = Sim::new(4, LAN);
    let stun_addr: SocketAddr = "198.51.100.1:3478".parse().unwrap();
    let silent: SocketAddr = "198.51.100.2:3478".parse().unwrap();
    sim.stun.push(StunServer {
        addr: stun_addr,
        mapping: Some("203.0.113.7:61000".parse().unwrap()),
    });
    sim.stun.push(StunServer {
        addr: silent,
        mapping: None,
    });
    let config = EndpointConfig {
        ice_servers: Some(vec!["stun:198.51.100.1".into(), "stun:198.51.100.2".into()]),
        ..EndpointConfig::default()
    };
    assert_eq!(config.stun_servers().len(), 2);
    let a = sim.add_node(&["10.0.0.1:5000"], config.clone());
    sim.nodes[a].ep.set_stun_servers(vec![stun_addr, silent]);
    let b = sim.add_node(&["10.0.0.2:6000"], no_stun());

    let start = sim.now;
    let ca = sim.nodes[a].ep.create_connection(start).unwrap();
    let offer = sim.local_description(a, ca);
    // The silent server holds gathering for the whole 5 s
    // `stun_gather_timeout`.
    assert_eq!(sim.now - start, Duration::from_secs(5));
    assert!(
        // str0m redacts the related address.
        offer
            .sdp
            .contains("203.0.113.7 61000 typ srflx raddr 0.0.0.0 rport 0"),
        "{}",
        offer.sdp
    );
    assert!(offer.sdp.contains("10.0.0.1 5000 typ host"));
    // Binding requests were retransmitted to the silent server (0, 0.5,
    // 1.5 and 3.5 s), once to the answering one.
    let to = |addr| {
        sim.trace
            .iter()
            .filter(|t| matches!(t, Trace::Datagram { to, .. } if *to == addr))
            .count()
    };
    assert_eq!((to(stun_addr), to(silent)), (1, 4));

    // The link still works (host candidates in the sim).
    let cb = sim.nodes[b].ep.create_connection(sim.now).unwrap();
    let now = sim.now;
    sim.nodes[b]
        .ep
        .set_remote_description(now, cb, &offer)
        .unwrap();
    let answer = sim.local_description(b, cb);
    let now = sim.now;
    sim.nodes[a]
        .ep
        .set_remote_description(now, ca, &answer)
        .unwrap();
    sim.nodes[a].links.entry(ca).or_default();
    sim.nodes[b].links.entry(cb).or_default();
    sim.run_until(Duration::from_secs(30), |s| {
        s.is_open(a, ca) && s.is_open(b, cb)
    });
}

#[test]
fn unanswered_connections_time_out() {
    let mut sim = Sim::new(5, LAN);
    let a = sim.add_node(&["10.0.0.1:5000"], no_stun());
    let now = sim.now;
    let ca = sim.nodes[a].ep.create_connection(now).unwrap();
    let _ = sim.local_description(a, ca);
    sim.run_until(Duration::from_secs(25), |s| !s.nodes[a].closed.is_empty());
    assert_eq!(sim.nodes[a].closed, vec![(ca, CloseReason::ConnectTimeout)]);
    assert_eq!(sim.now - now, Duration::from_secs(20));
}

#[test]
fn a_vanished_peer_fails_the_connection() {
    let mut sim = Sim::new(6, LAN);
    let a = sim.add_node(&["10.0.0.1:5000"], no_stun());
    let b = sim.add_node(&["10.0.0.2:6000"], no_stun());
    let (ca, cb) = sim.connect(a, b);
    sim.run_until(Duration::from_secs(30), |s| {
        s.is_open(a, ca) && s.is_open(b, cb)
    });
    // b disappears without a word: datagrams to it go nowhere.
    assert_eq!(b, sim.nodes.len() - 1);
    let _gone = sim.nodes.remove(b);
    let start = sim.now;
    sim.run_until(Duration::from_secs(120), |s| !s.nodes[a].closed.is_empty());
    assert_eq!(sim.nodes[a].closed, vec![(ca, CloseReason::IceFailed)]);
    let took = sim.now - start;
    assert!(
        took > Duration::from_secs(25) && took < Duration::from_secs(60),
        "{took}"
    );
}

#[test]
fn signalling_misuse_is_rejected() {
    let mut sim = Sim::new(7, LAN);
    let a = sim.add_node(&["10.0.0.1:5000"], no_stun());
    let b = sim.add_node(&["10.0.0.2:5000"], no_stun());
    let now = sim.now;
    let ca = sim.nodes[a].ep.create_connection(now).unwrap();
    let offer = sim.local_description(a, ca);
    // An offer to a side that already offered (glare).
    let cb = sim.nodes[b].ep.create_connection(now).unwrap();
    let other = sim.local_description(b, cb);
    assert_eq!(
        sim.nodes[a].ep.set_remote_description(now, ca, &other),
        Err(strims_webrtc::Error::UnexpectedOffer)
    );
    // An answer without an offer.
    let cc = sim.nodes[b].ep.create_connection(now).unwrap();
    let fake_answer = SessionDescription {
        sdp_type: SdpType::Answer,
        sdp: offer.sdp.clone(),
    };
    assert_eq!(
        sim.nodes[b]
            .ep
            .set_remote_description(now, cc, &fake_answer),
        Err(strims_webrtc::Error::NoPendingOffer)
    );
    // Garbage.
    let garbage = SessionDescription {
        sdp_type: SdpType::Offer,
        sdp: "not sdp".into(),
    };
    assert!(matches!(
        sim.nodes[b].ep.set_remote_description(now, cc, &garbage),
        Err(strims_webrtc::Error::Sdp(_))
    ));
    let pr = SessionDescription {
        sdp_type: SdpType::Pranswer,
        sdp: offer.sdp.clone(),
    };
    assert!(
        sim.nodes[b]
            .ep
            .set_remote_description(now, cc, &pr)
            .is_err()
    );
    // No sockets, no connection.
    let mut lonely = WebRtcEndpoint::new(now, no_stun(), det_rng(1));
    assert_eq!(
        lonely.create_connection(now),
        Err(strims_webrtc::Error::NoHostCandidates)
    );
    // Datagrams for nobody are refused.
    assert!(!sim.nodes[a].ep.handle_udp_input(
        now,
        "10.0.0.9:9".parse().unwrap(),
        "10.0.0.1:5000".parse().unwrap(),
        &[0x16, 0xfe, 0xfd, 0, 0]
    ));
}
