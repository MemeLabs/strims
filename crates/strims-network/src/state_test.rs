// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Runs between states in an event-queue simulation: real PSZ sessions,
//! real certificates, a fake node, and seeded latency on every channel.
//! Each channel (a peer's calls, its answers, its broker bytes, a side's
//! PSI engine messages each way) delivers in order, and channels race
//! each other.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use rand_core::Rng;
use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, generate_key, new_certificate_request, sign_certificate_request,
    sign_certificate_request_with_network,
};
use strims_kademlia::ID;
use strims_proto::network::v1::NetworkPeerBinding;
use strims_proto::types::certificate::ParentOneof;
use strims_proto::types::{Certificate, Key, KeyUsage};

use crate::Config;
use crate::psi::{PsiServer, PsiTransport, RemotePsi};
use crate::state::{Action, NetKey, Op, State};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);
const MTU: usize = 1400;
const PEER_SIGN: i32 = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;

/// A member's certificate on the network whose key is `network`.
fn membership(network: &Key, profile: &Key, rng: &mut DetRng) -> Certificate {
    let csr = new_certificate_request(profile, PEER_SIGN, "member").unwrap();
    sign_certificate_request_with_network(&csr, network, "net", DEFAULT_CERT_TTL, T0, rng).unwrap()
}

/// A certificate issued under an invitation from the member holding
/// `inviter` and `inviter_cert`.
fn invited(
    inviter: &Key,
    inviter_cert: &Certificate,
    invitee: &Key,
    rng: &mut DetRng,
) -> Certificate {
    let invite_key = generate_key(rng);
    let csr = new_certificate_request(&invite_key, KeyUsage::Sign as i32, "").unwrap();
    let mut invite = sign_certificate_request(&csr, Duration::HOUR * 24, inviter, T0, rng).unwrap();
    invite.parent_oneof = Some(ParentOneof::Parent(Box::new(inviter_cert.clone())));
    let csr = new_certificate_request(invitee, KeyUsage::Peer as i32, "invitee").unwrap();
    let mut cert = sign_certificate_request(&csr, DEFAULT_CERT_TTL, &invite_key, T0, rng).unwrap();
    cert.parent_oneof = Some(ParentOneof::Parent(Box::new(invite)));
    cert
}

#[derive(Clone, Debug)]
enum Msg {
    Negotiate {
        run: u64,
        count: u32,
    },
    Open {
        run: u64,
        bindings: Vec<NetworkPeerBinding>,
    },
    Close {
        key: NetKey,
    },
    Update {
        certificate: Certificate,
    },
    Answer {
        call: Box<Msg>,
        ok: bool,
    },
    Broker(Vec<u8>),
    /// A message to the side's PSI server, and one back.
    ToEngine(Vec<u8>),
    FromEngine(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Chan {
    Call,
    Answer,
    Broker,
    ToEngine,
    FromEngine,
}

/// Messages for a side's PSI server.
#[derive(Clone, Default)]
struct Outbox(Rc<RefCell<VecDeque<Vec<u8>>>>);

impl PsiTransport for Outbox {
    fn send(&self, msg: Vec<u8>) {
        self.0.borrow_mut().push_back(msg);
    }
}

/// A side's remote PSI engine and its server.
struct Engine {
    remote: RemotePsi,
    server: PsiServer,
    outbox: Outbox,
    /// The peers whose sessions notified.
    ready: Rc<RefCell<Vec<ID>>>,
    /// How long the server takes to answer each message, or `None` if it
    /// never does.
    delay: Option<Duration>,
    handled: usize,
}

struct Node {
    nets: BTreeMap<NetKey, u32>,
    next_net: u32,
    next_port: u16,
    /// (peer, net) -> (local port, peer port).
    bound: BTreeMap<(usize, u32), (u16, u16)>,
}

struct Side {
    st: State<u32>,
    host: ID,
    profile: Key,
    node: Node,
    /// Per peer: whether a call is out, and the calls waiting for it.
    calls: BTreeMap<usize, (bool, VecDeque<Msg>)>,
    counts: BTreeMap<NetKey, usize>,
    renew_via: Vec<(usize, NetKey)>,
    /// Peers (by index) this side is linked to, and their link ids here.
    links: BTreeMap<usize, u64>,
    /// The side's PSI engine, if remote.
    engine: Option<Engine>,
}

struct Sim {
    now: Time,
    rng: DetRng,
    sides: Vec<Side>,
    /// (deliver at, seq) -> (from, to, channel, message)
    queue: BTreeMap<(Time, u64), (usize, usize, Chan, Msg)>,
    seq: u64,
    /// When each channel last delivers, so it delivers in order.
    last: BTreeMap<(usize, usize, Chan), Time>,
    trace: Vec<String>,
}

impl Sim {
    fn new(seed: u64, n: usize, config: Config) -> Sim {
        let mut rng = det_rng(seed);
        let sides = (0..n)
            .map(|_| {
                let profile = generate_key(&mut rng);
                let mut id = [0u8; 32];
                rng.fill_bytes(&mut id);
                let host = ID::from_bytes(&id);
                Side {
                    st: State::new(host, &config, fork(&mut rng)),
                    host,
                    profile,
                    node: Node {
                        nets: BTreeMap::new(),
                        next_net: 1,
                        next_port: 1000,
                        bound: BTreeMap::new(),
                    },
                    calls: BTreeMap::new(),
                    counts: BTreeMap::new(),
                    renew_via: Vec::new(),
                    links: BTreeMap::new(),
                    engine: None,
                }
            })
            .collect();
        Sim {
            now: T0,
            rng,
            sides,
            queue: BTreeMap::new(),
            seq: 0,
            last: BTreeMap::new(),
            trace: Vec::new(),
        }
    }

    /// Runs side `i`'s sessions on a remote engine whose server answers
    /// each message after `delay` and a jitter up to as long again, or
    /// never.
    fn remote_psi(&mut self, i: usize, delay: Option<Duration>) {
        let outbox = Outbox::default();
        let remote = RemotePsi::new(outbox.clone());
        let ready: Rc<RefCell<Vec<ID>>> = Rc::default();
        let notified = ready.clone();
        let st = &mut self.sides[i].st;
        st.set_psi(Rc::new(remote.clone()));
        st.set_psi_wake(Rc::new(move |host| notified.borrow_mut().push(host)));
        self.sides[i].engine = Some(Engine {
            remote,
            server: PsiServer::new(),
            outbox,
            ready,
            delay,
            handled: 0,
        });
    }

    fn index_of(&self, host: ID) -> usize {
        self.sides.iter().position(|s| s.host == host).unwrap()
    }

    fn join(&mut self, i: usize, network: &Key, certificate: Certificate) {
        let now = self.now;
        let id = u64::from(network.public[0]);
        self.sides[i]
            .st
            .add_network(network.public.clone(), id, certificate, now);
        self.pump(i);
    }

    fn leave(&mut self, i: usize, network: &Key) {
        self.sides[i].st.remove_network(&network.public);
        self.pump(i);
    }

    fn connect(&mut self, i: usize, j: usize) {
        let now = self.now;
        let (hi, hj) = (self.sides[i].host, self.sides[j].host);
        let (pi, pj) = (
            self.sides[i].profile.public.clone(),
            self.sides[j].profile.public.clone(),
        );
        let li = self.sides[i].st.add_peer(hj, pj, now);
        self.sides[i].links.insert(j, li);
        let lj = self.sides[j].st.add_peer(hi, pi, now);
        self.sides[j].links.insert(i, lj);
        self.pump(i);
        self.pump(j);
    }

    fn send(&mut self, from: usize, to: usize, chan: Chan, msg: Msg) {
        let latency = Duration::from_millis(2 + (self.rng.next_u64() % 30) as i64);
        self.send_after(from, to, chan, msg, latency);
    }

    fn send_after(&mut self, from: usize, to: usize, chan: Chan, msg: Msg, latency: Duration) {
        let at = (self.now + latency).max(
            self.last
                .get(&(from, to, chan))
                .copied()
                .unwrap_or(Time::EPOCH),
        );
        self.last.insert((from, to, chan), at);
        self.seq += 1;
        self.queue.insert((at, self.seq), (from, to, chan, msg));
    }

    /// Carries out side `i`'s actions and sends its broker bytes. A remote
    /// engine's messages go to its server, and the sessions that notified
    /// move on.
    fn pump(&mut self, i: usize) {
        loop {
            let mut progressed = false;
            let (msgs, ready) = match &self.sides[i].engine {
                Some(e) => (
                    e.outbox.0.borrow_mut().drain(..).collect(),
                    e.ready.borrow_mut().drain(..).collect(),
                ),
                None => (Vec::new(), Vec::new()),
            };
            for m in msgs {
                progressed = true;
                self.send_after(i, i, Chan::ToEngine, Msg::ToEngine(m), Duration::ZERO);
            }
            for host in ready {
                progressed = true;
                let j = self.index_of(host);
                if let Some(&link) = self.sides[i].links.get(&j) {
                    let now = self.now;
                    self.sides[i].st.psi_ready(host, link, now);
                }
            }
            while let Some(a) = self.sides[i].st.poll_action() {
                progressed = true;
                self.act(i, a);
            }
            let peers: Vec<(usize, u64)> =
                self.sides[i].links.iter().map(|(j, l)| (*j, *l)).collect();
            for (j, link) in peers {
                let host = self.sides[j].host;
                let mut buf = [0u8; MTU];
                while let Some(n) = self.sides[i]
                    .st
                    .poll_broker_transmit(host, link, &mut buf, self.now)
                {
                    progressed = true;
                    self.send(i, j, Chan::Broker, Msg::Broker(buf[..n].to_vec()));
                }
            }
            if !progressed {
                return;
            }
        }
    }

    fn act(&mut self, i: usize, a: Action<u32>) {
        let now = self.now;
        match a {
            Action::Op(_, op) => {
                let s = &mut self.sides[i];
                match op {
                    Op::AddNetwork { key } => {
                        let net = s.node.next_net;
                        s.node.next_net += 1;
                        s.node.nets.insert(key.clone(), net);
                        s.st.network_added(&key, Some(net), now);
                    }
                    Op::RemoveNetwork { net } => {
                        s.node.nets.retain(|_, n| *n != net);
                        s.node.bound.retain(|(_, n), _| *n != net);
                    }
                    Op::ReservePorts {
                        host,
                        link,
                        run,
                        count,
                    } => {
                        let ports: Vec<u16> = (0..count)
                            .map(|_| {
                                s.node.next_port += 1;
                                s.node.next_port
                            })
                            .collect();
                        s.st.ports_reserved(host, link, run, Some(ports), now);
                    }
                    Op::ReleasePort { .. } => {}
                    Op::Bind {
                        host,
                        link,
                        net,
                        local_port,
                        peer_port,
                    } => {
                        assert!(s.st.is_current(host, link));
                        let j = self.index_of(host);
                        self.sides[i]
                            .node
                            .bound
                            .insert((j, net), (local_port, peer_port));
                    }
                    Op::Unbind { host, net, .. } => {
                        let j = self.index_of(host);
                        self.sides[i].node.bound.remove(&(j, net));
                    }
                }
            }
            Action::Negotiate {
                host, run, count, ..
            } => self.call(i, host, Msg::Negotiate { run, count }),
            Action::Open {
                host,
                run,
                bindings,
                ..
            } => self.call(i, host, Msg::Open { run, bindings }),
            Action::Close { host, key, .. } => self.call(i, host, Msg::Close { key }),
            Action::UpdateCertificate {
                host, certificate, ..
            } => self.call(i, host, Msg::Update { certificate }),
            Action::PeerCount { key, count } => {
                self.sides[i].counts.insert(key, count);
            }
            Action::RenewVia { host, key, .. } => {
                let j = self.index_of(host);
                self.sides[i].renew_via.push((j, key));
            }
            Action::Renew { .. } => {}
        }
    }

    /// Queues a call to `host`, one at a time per peer.
    fn call(&mut self, i: usize, host: ID, msg: Msg) {
        let j = self.index_of(host);
        let (busy, queued) = self.sides[i].calls.entry(j).or_default();
        if *busy {
            queued.push_back(msg);
        } else {
            *busy = true;
            self.send(i, j, Chan::Call, msg);
        }
    }

    fn deliver(&mut self, from: usize, to: usize, msg: Msg) {
        let msg = match msg {
            Msg::ToEngine(m) => {
                let e = self.sides[to].engine.as_mut().unwrap();
                let Some(delay) = e.delay else {
                    return;
                };
                e.handled += 1;
                let replies = e.server.handle(&m);
                for r in replies {
                    let jitter = Duration::from_nanos(
                        (self.rng.next_u64() % delay.as_nanos().max(1) as u64) as i64,
                    );
                    self.send_after(to, to, Chan::FromEngine, Msg::FromEngine(r), delay + jitter);
                }
                return;
            }
            Msg::FromEngine(m) => {
                self.sides[to].engine.as_ref().unwrap().remote.receive(&m);
                self.pump(to);
                return;
            }
            msg => msg,
        };
        let now = self.now;
        let host = self.sides[from].host;
        let Some(&link) = self.sides[to].links.get(&from) else {
            return;
        };
        self.trace.push(format!(
            "{} {from}->{to} {}",
            now.unix_millis(),
            describe(&msg)
        ));
        let st = &mut self.sides[to].st;
        match msg {
            Msg::Broker(b) => st.handle_broker_input(host, link, &b, now),
            Msg::Answer { call, ok } => {
                match *call {
                    Msg::Negotiate { run, .. } => st.negotiate_answered(host, link, run, ok, now),
                    Msg::Open { run, .. } => st.open_answered(host, link, run, ok, now),
                    Msg::Close { ref key } => st.close_answered(host, link, key, ok),
                    _ => {}
                }
                let next = {
                    let (busy, queued) = self.sides[to].calls.entry(from).or_default();
                    let next = queued.pop_front();
                    *busy = next.is_some();
                    next
                };
                if let Some(next) = next {
                    self.send(to, from, Chan::Call, next);
                }
            }
            call => {
                let ok = match &call {
                    Msg::Negotiate { count, .. } => st.handle_negotiate(host, link, *count, now),
                    Msg::Open { bindings, .. } => st.handle_open(host, link, bindings.clone(), now),
                    Msg::Close { key } => st.handle_close(host, link, key),
                    Msg::Update { certificate } => {
                        st.handle_update_certificate(host, link, Some(certificate.clone()), now)
                    }
                    _ => Ok(()),
                }
                .is_ok();
                self.send(
                    to,
                    from,
                    Chan::Answer,
                    Msg::Answer {
                        call: Box::new(call),
                        ok,
                    },
                );
            }
        }
        self.pump(to);
    }

    /// Runs until nothing is queued or due, or until `limit`.
    fn settle(&mut self, limit: Duration) {
        let end = self.now + limit;
        loop {
            let next_msg = self.queue.keys().next().map(|(t, _)| *t);
            let next_timer = self.sides.iter().filter_map(|s| s.st.poll_timeout()).min();
            let Some(next) = [next_msg, next_timer].into_iter().flatten().min() else {
                return;
            };
            if next.after(end) {
                return;
            }
            self.now = self.now.max(next);
            if next_msg == Some(next) {
                let key = *self.queue.keys().next().unwrap();
                let (from, to, _, msg) = self.queue.remove(&key).unwrap();
                self.deliver(from, to, msg);
            } else {
                for i in 0..self.sides.len() {
                    if self.sides[i]
                        .st
                        .poll_timeout()
                        .is_some_and(|t| !self.now.before(t))
                    {
                        let now = self.now;
                        self.sides[i].st.handle_timeout(now);
                        self.pump(i);
                    }
                }
            }
        }
    }

    /// Both sides hold the same open links, crosswise, for `key`, and
    /// the nodes bound them.
    fn assert_linked(&self, i: usize, j: usize, key: &[u8]) {
        let (a, b) = (&self.sides[i], &self.sides[j]);
        let la = a.st.link_ports(b.host, key);
        let lb = b.st.link_ports(a.host, key);
        let (Some((al, ap, true)), Some((bl, bp, true))) = (la, lb) else {
            panic!("{i}<->{j}: not both open: {la:?} {lb:?}");
        };
        assert_eq!((al, ap), (bp, bl), "{i}<->{j}: ports");
        let net_a = a.node.nets[key];
        let net_b = b.node.nets[key];
        assert_eq!(a.node.bound.get(&(j, net_a)), Some(&(al, ap)));
        assert_eq!(b.node.bound.get(&(i, net_b)), Some(&(bl, bp)));
    }

    fn assert_unlinked(&self, i: usize, j: usize, key: &[u8]) {
        let (a, b) = (&self.sides[i], &self.sides[j]);
        assert!(!a.st.has_link(b.host, key), "{i}->{j} has a link");
        assert!(!b.st.has_link(a.host, key), "{j}->{i} has a link");
    }

    fn assert_idle(&self) {
        for (i, s) in self.sides.iter().enumerate() {
            for j in s.links.keys() {
                assert!(s.st.is_idle(self.sides[*j].host), "{i} still runs with {j}");
            }
        }
    }
}

fn describe(m: &Msg) -> String {
    match m {
        Msg::Negotiate { count, .. } => format!("negotiate {count}"),
        Msg::Open { bindings, .. } => format!("open {}", bindings.len()),
        Msg::Close { .. } => "close".into(),
        Msg::Update { .. } => "update".into(),
        Msg::Answer { call, ok } => format!("answer {} {ok}", describe(call)),
        Msg::Broker(b) => format!("broker {}", b.len()),
        Msg::ToEngine(m) => format!("to engine {}", m.len()),
        Msg::FromEngine(m) => format!("from engine {}", m.len()),
    }
}

/// `n` networks' keys.
fn networks(rng: &mut DetRng, n: usize) -> Vec<Key> {
    (0..n).map(|_| generate_key(rng)).collect()
}

/// Two members with three networks each, two of them shared, bind
/// exactly the shared ones, whichever side holds more keys.
#[test]
fn shared_networks_bind_on_both_sides() {
    for seed in 0..6 {
        let mut sim = Sim::new(seed, 2, Config::default());
        let mut rng = det_rng(100 + seed);
        let nets = networks(&mut rng, 5);
        // side 0: n0 n1 n2 (+ n4 on odd seeds); side 1: n1 n2 n3
        let mut zero: Vec<usize> = vec![0, 1, 2];
        if seed % 2 == 1 {
            zero.push(4);
        }
        for k in zero {
            let c = membership(&nets[k], &sim.sides[0].profile.clone(), &mut rng);
            sim.join(0, &nets[k], c);
        }
        for k in [1, 2, 3] {
            let c = membership(&nets[k], &sim.sides[1].profile.clone(), &mut rng);
            sim.join(1, &nets[k], c);
        }
        sim.connect(0, 1);
        sim.settle(Duration::from_secs(60));
        sim.assert_idle();
        for k in [1, 2] {
            sim.assert_linked(0, 1, &nets[k].public);
        }
        for k in [0, 3, 4] {
            sim.assert_unlinked(0, 1, &nets[k].public);
        }
        assert_eq!(sim.sides[0].counts.get(&nets[1].public), Some(&1));
        let m = sim.sides[0].st.metrics();
        assert_eq!(m.runs_failed, 0, "seed {seed}");
    }
}

/// Small key sets hit the receiver's cuckoo failure on some seeds (about
/// one run in twenty with ten keys); those runs start again and bind all
/// the same.
#[test]
fn a_failed_cuckoo_table_runs_again() {
    let mut retried = 0;
    for seed in 0..120 {
        let mut sim = Sim::new(1000 + seed, 2, Config::default());
        let mut rng = det_rng(2000 + seed);
        let nets = networks(&mut rng, 11);
        for i in 0..2 {
            for net in &nets[i..i + 10] {
                let c = membership(net, &sim.sides[i].profile.clone(), &mut rng);
                sim.join(i, net, c);
            }
        }
        sim.connect(0, 1);
        sim.settle(Duration::from_secs(60));
        sim.assert_idle();
        for net in &nets[1..10] {
            sim.assert_linked(0, 1, &net.public);
        }
        retried +=
            sim.sides[0].st.metrics().cuckoo_retries + sim.sides[1].st.metrics().cuckoo_retries;
    }
    assert!(retried > 0, "no seed hit a cuckoo failure");
}

/// A member holding only an invitation's certificate pairs but stays
/// unbound until it renews; the renewed certificate opens the link on
/// both sides.
#[test]
fn provisional_links_open_once_renewed() {
    let mut sim = Sim::new(7, 2, Config::default());
    let mut rng = det_rng(70);
    let net = generate_key(&mut rng);
    let owner = sim.sides[0].profile.clone();
    let owner_cert = membership(&net, &owner, &mut rng);
    sim.join(0, &net, owner_cert.clone());
    let invitee = sim.sides[1].profile.clone();
    sim.join(1, &net, invited(&owner, &owner_cert, &invitee, &mut rng));
    sim.connect(0, 1);
    sim.settle(Duration::from_secs(30));
    sim.assert_idle();
    let (h0, h1) = (sim.sides[0].host, sim.sides[1].host);
    assert_eq!(
        sim.sides[0].st.link_ports(h1, &net.public).map(|l| l.2),
        Some(false)
    );
    assert_eq!(
        sim.sides[1].st.link_ports(h0, &net.public).map(|l| l.2),
        Some(false)
    );
    assert_eq!(sim.sides[1].renew_via, [(0, net.public.clone())]);
    assert!(sim.sides[0].renew_via.is_empty());

    let renewed = membership(&net, &invitee, &mut rng);
    let now = sim.now;
    sim.sides[1].st.set_certificate(&net.public, renewed, now);
    sim.pump(1);
    sim.settle(Duration::from_secs(30));
    sim.assert_linked(0, 1, &net.public);
}

/// Leaving a network closes its links on both sides; joining it again
/// binds it again.
#[test]
fn leaving_a_network_closes_its_links() {
    let mut sim = Sim::new(8, 2, Config::default());
    let mut rng = det_rng(80);
    let nets = networks(&mut rng, 2);
    let mut certs = Vec::new();
    for net in &nets {
        let c0 = membership(net, &sim.sides[0].profile.clone(), &mut rng);
        let c1 = membership(net, &sim.sides[1].profile.clone(), &mut rng);
        sim.join(0, net, c0.clone());
        sim.join(1, net, c1);
        certs.push(c0);
    }
    sim.connect(0, 1);
    sim.settle(Duration::from_secs(30));
    sim.assert_linked(0, 1, &nets[0].public);

    sim.leave(0, &nets[0]);
    sim.settle(Duration::from_secs(30));
    sim.assert_unlinked(0, 1, &nets[0].public);
    sim.assert_linked(0, 1, &nets[1].public);
    assert_eq!(sim.sides[1].counts.get(&nets[0].public), Some(&0));

    sim.join(0, &nets[0], certs[0].clone());
    sim.settle(Duration::from_secs(30));
    sim.assert_idle();
    sim.assert_linked(0, 1, &nets[0].public);
    sim.assert_linked(0, 1, &nets[1].public);
}

/// Three members of one network, each linked to the others, all bind.
#[test]
fn a_mesh_binds_every_pair() {
    let mut sim = Sim::new(9, 3, Config::default());
    let mut rng = det_rng(90);
    let net = generate_key(&mut rng);
    for i in 0..3 {
        let c = membership(&net, &sim.sides[i].profile.clone(), &mut rng);
        sim.join(i, &net, c);
    }
    sim.connect(0, 1);
    sim.connect(1, 2);
    sim.connect(0, 2);
    sim.settle(Duration::from_secs(60));
    sim.assert_idle();
    for (i, j) in [(0, 1), (1, 2), (0, 2)] {
        sim.assert_linked(i, j, &net.public);
    }
    assert_eq!(sim.sides[1].counts.get(&net.public), Some(&2));
}

/// The same seed gives the same run, message for message.
#[test]
fn same_seed_same_trace() {
    let run = |seed: u64| {
        let mut sim = Sim::new(seed, 2, Config::default());
        let mut rng = det_rng(seed);
        let nets = networks(&mut rng, 3);
        for i in 0..2 {
            for net in &nets[i..i + 2] {
                let c = membership(net, &sim.sides[i].profile.clone(), &mut rng);
                sim.join(i, net, c);
            }
        }
        sim.connect(0, 1);
        sim.settle(Duration::from_secs(60));
        sim.trace
    };
    assert_eq!(run(5), run(5));
    assert_ne!(run(5), run(6));
}

/// Garbage on the broker channel fails the session; the run backs off
/// and gives up after `max_attempts`.
#[test]
fn garbage_broker_bytes_fail_runs_until_they_give_up() {
    let config = Config::default();
    let mut rng = det_rng(11);
    let host = ID::from_bytes(&[1; 32]);
    let peer = ID::from_bytes(&[2; 32]);
    let mut st: State<u32> = State::new(host, &config, det_rng(12));
    let net = generate_key(&mut rng);
    let profile = generate_key(&mut rng);
    st.add_network(
        net.public.clone(),
        1,
        membership(&net, &profile, &mut rng),
        T0,
    );
    while let Some(a) = st.poll_action() {
        if let Action::Op(_, Op::AddNetwork { key }) = a {
            st.network_added(&key, Some(1), T0);
        }
    }
    let link = st.add_peer(peer, generate_key(&mut rng).public, T0);
    let mut now = T0;
    for _ in 0..200 {
        let Some(t) = st.poll_timeout() else {
            break;
        };
        now = now.max(t);
        st.handle_timeout(now);
        while let Some(a) = st.poll_action() {
            if let Action::Negotiate { run, .. } = a {
                st.negotiate_answered(peer, link, run, true, now);
                st.handle_negotiate(peer, link, 1, now).unwrap();
            }
        }
        let mut buf = [0u8; MTU];
        while st.poll_broker_transmit(peer, link, &mut buf, now).is_some() {}
        st.handle_broker_input(peer, link, &[0xff; 64], now);
    }
    assert!(st.is_idle(peer));
    let m = st.metrics();
    assert_eq!(m.runs_failed, u64::from(config.max_attempts));
    assert_eq!(m.runs_completed, 0);
}

/// A peer that keeps sending `Negotiate` starts runs no faster than
/// `min_peer_run_interval`.
#[test]
fn a_peer_negotiate_flood_is_paced() {
    let config = Config::default();
    let host = ID::from_bytes(&[1; 32]);
    let peer = ID::from_bytes(&[2; 32]);
    let mut st: State<u32> = State::new(host, &config, det_rng(1));
    let link = st.add_peer(peer, vec![9; 32], T0);
    let mut sent = 0;
    let mut now = T0;
    for _ in 0..100 {
        now += Duration::from_millis(10);
        // each Negotiate from the peer while starting is absorbed; one
        // that follows a finished run starts another
        st.handle_negotiate(peer, link, 1, now).unwrap();
        if let Some(t) = st.poll_timeout()
            && !now.before(t)
        {
            st.handle_timeout(now);
        }
        while let Some(a) = st.poll_action() {
            if let Action::Negotiate { run, .. } = a {
                sent += 1;
                // no keys here: the run ends once answered
                st.negotiate_answered(peer, link, run, true, now);
            }
        }
    }
    // one second of flooding
    assert!(sent <= 2, "{sent} runs in one second");
    assert!(st.metrics().runs_paced > 0);
}

/// A binding whose certificate isn't the peer's is refused: the sender
/// answers with none, and nothing pairs.
#[test]
fn a_binding_for_someone_else_is_refused() {
    let mut sim = Sim::new(13, 3, Config::default());
    let mut rng = det_rng(130);
    let net = generate_key(&mut rng);
    for i in 0..2 {
        let c = membership(&net, &sim.sides[i].profile.clone(), &mut rng);
        sim.join(i, &net, c);
    }
    // side 1 offers side 2's certificate as its own
    let stolen = membership(&net, &sim.sides[2].profile.clone(), &mut rng);
    let now = sim.now;
    sim.sides[1].st.set_certificate(&net.public, stolen, now);
    sim.connect(0, 1);
    sim.settle(Duration::from_secs(60));
    assert!(
        sim.sides[0]
            .st
            .link_ports(sim.sides[1].host, &net.public)
            .is_none()
    );
    let refused =
        sim.sides[0].st.metrics().bindings_refused + sim.sides[1].st.metrics().bindings_refused;
    assert!(refused > 0);
}

/// Sessions on remote engines whose answers come back late, behind the
/// peer's bytes and calls, still find and bind the shared networks: each
/// answer moves the run on through `psi_ready`. One side or both may be
/// remote.
#[test]
fn late_remote_psi_answers_still_bind_the_shared_networks() {
    for seed in 0..4 {
        let mut sim = Sim::new(300 + seed, 2, Config::default());
        let late = Some(Duration::from_millis(400));
        sim.remote_psi(0, late);
        if seed % 2 == 0 {
            sim.remote_psi(1, late);
        }
        let mut rng = det_rng(400 + seed);
        let nets = networks(&mut rng, 5);
        for k in [0, 1, 2, 4] {
            let c = membership(&nets[k], &sim.sides[0].profile.clone(), &mut rng);
            sim.join(0, &nets[k], c);
        }
        for k in [1, 2, 3] {
            let c = membership(&nets[k], &sim.sides[1].profile.clone(), &mut rng);
            sim.join(1, &nets[k], c);
        }
        sim.connect(0, 1);
        sim.settle(Duration::from_secs(120));
        sim.assert_idle();
        for k in [1, 2] {
            sim.assert_linked(0, 1, &nets[k].public);
        }
        for k in [0, 3, 4] {
            sim.assert_unlinked(0, 1, &nets[k].public);
        }
        for side in &sim.sides {
            assert_eq!(side.st.metrics().runs_failed, 0, "seed {seed}");
        }
        assert!(sim.sides[0].engine.as_ref().unwrap().handled > 0);
    }
}

/// A remote engine that never answers stalls its runs like a silent peer:
/// each fails at the stall timeout, and the peer gives up after
/// `max_attempts`.
#[test]
fn a_silent_psi_engine_stalls_runs_until_they_give_up() {
    let config = Config::default();
    let mut sim = Sim::new(17, 2, config);
    sim.remote_psi(0, None);
    let mut rng = det_rng(170);
    let net = generate_key(&mut rng);
    for i in 0..2 {
        let c = membership(&net, &sim.sides[i].profile.clone(), &mut rng);
        sim.join(i, &net, c);
    }
    sim.connect(0, 1);
    sim.settle(Duration::from_secs(600));
    assert!(sim.sides[0].st.is_idle(sim.sides[1].host));
    let m = sim.sides[0].st.metrics();
    assert_eq!(m.runs_failed, u64::from(config.max_attempts), "{m:?}");
    assert_eq!(m.runs_completed, 0);
    sim.assert_unlinked(0, 1, &net.public);
}
