// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A deterministic in-memory driver for tests: vnic and vpn hosts on
//! in-memory links, with a virtual clock, per-link latency and frame loss.
//!
//! It is the driver loop in miniature:
//!
//! - link bytes go through `vnic.handle_link_input`, whose dispatch hands
//!   vpn frames to `vpn.handle_frame` (borrowed, no copy);
//! - vnic `PeerAdded` adds the peer to the network, `PeerRemoved` tells vpn;
//! - after every input: `vpn.flush`, `vnic.poll_egress` with
//!   `vpn.fill_egress` as the fill, then the link bytes go on the wire;
//! - timers wake at `min(vnic.poll_timeout(), vpn.poll_timeout())`.
//!
//! Peer exchange can be answered automatically: local descriptions are the
//! node's index, and dialing one links the two nodes.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};
use strims_crypto::key_from_seed;
use strims_kademlia::ID;
use strims_vnic::{HostConfig, LinkConfig, LinkKind, PeerKey};
use strims_vpn::{Config, Event, ExchangeId, Host, LinkDescription, NetworkId, VnicHost};

/// The vnic routing tag: vpn frames carry their network.
pub type Tag = NetworkId;
/// App handler tags: the port.
pub type AppTag = u16;

/// The port pair vpn links use.
pub const VPN_PORT: u16 = 10000;
/// The app port tests send to.
pub const APP_PORT: u16 = 20000;
pub const MTU: usize = 16 * 1024;
pub const SIM_INTERFACE: &str = "sim";

pub fn start() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

pub fn rand_f64(rng: &mut DetRng) -> f64 {
    (rand_core::Rng::next_u64(rng) >> 11) as f64 / (1u64 << 53) as f64
}

pub fn rand_below(rng: &mut DetRng, n: usize) -> usize {
    (rand_core::Rng::next_u64(rng) % n as u64) as usize
}

pub struct SimNode {
    pub vnic: VnicHost<Tag>,
    pub vpn: Host<AppTag>,
    pub net: NetworkId,
    /// vpn events, with the time they came out.
    pub events: Vec<(Time, Event<AppTag>)>,
    /// Add vnic peers to the network when they connect.
    pub auto_join: bool,
    /// Answer peer exchange events.
    pub auto_exchange: bool,
    /// Results of `handle_frame`, for tests that expect errors.
    pub frame_errors: Vec<strims_vpn::Error>,
    timer: Option<Time>,
}

pub struct Wire {
    pub ends: [(usize, PeerKey); 2],
    pub latency: Duration,
    /// Last arrival per direction, so bytes stay in order.
    last: [Time; 2],
    pub open: bool,
}

enum Ev {
    Deliver {
        wire: usize,
        to: usize,
        data: Vec<u8>,
    },
    Timer {
        node: usize,
        at: Time,
    },
}

pub struct World {
    pub now: Time,
    pub nodes: Vec<SimNode>,
    pub wires: Vec<Wire>,
    queue: BTreeMap<(Time, u64), Ev>,
    seq: u64,
    pub rng: DetRng,
    /// Probability that a vpn frame is lost.
    pub loss: f64,
    pub frames_lost: u64,
    pub trace: Vec<String>,
    pub network_key: Vec<u8>,
    pub ids: BTreeMap<ID, usize>,
}

impl World {
    /// `n` nodes in one network. Node `i`'s profile key comes from the
    /// seed; routing uses its host id, from its vnic host key.
    pub fn new(seed: u64, n: usize, config: Config) -> World {
        World::new_with(seed, n, |_| config.clone())
    }

    /// [`World::new`] with node `i` configured by `config(i)`, for mixed
    /// networks.
    pub fn new_with(seed: u64, n: usize, config: impl Fn(usize) -> Config) -> World {
        let mut rng = det_rng(seed);
        let network_key = key_from_seed(&[0xee; 32]).public;
        let mut nodes = Vec::new();
        let mut ids = BTreeMap::new();
        for i in 0..n {
            let mut profile_seed = [0u8; 32];
            rand_core::Rng::fill_bytes(&mut rng, &mut profile_seed);
            let hc = HostConfig {
                label: format!("node-{i}"),
                ..HostConfig::default()
            };
            let mut vnic = VnicHost::new(key_from_seed(&profile_seed), hc, fork(&mut rng)).unwrap();
            vnic.set_max_peers(64);
            let mut vpn = Host::new(&mut vnic, config(i), fork(&mut rng)).unwrap();
            let net = vpn.add_network(&mut vnic, &network_key).unwrap();
            vpn.set_handler(net, APP_PORT, APP_PORT).unwrap();
            ids.insert(vnic.id(), i);
            nodes.push(SimNode {
                vnic,
                vpn,
                net,
                events: Vec::new(),
                auto_join: true,
                auto_exchange: true,
                frame_errors: Vec::new(),
                timer: None,
            });
        }
        World {
            now: start(),
            nodes,
            wires: Vec::new(),
            queue: BTreeMap::new(),
            seq: 0,
            rng,
            loss: 0.0,
            frames_lost: 0,
            trace: Vec::new(),
            network_key,
            ids,
        }
    }

    pub fn id(&self, i: usize) -> ID {
        self.nodes[i].vnic.id()
    }

    pub fn index(&self, id: &ID) -> Option<usize> {
        self.ids.get(id).copied()
    }

    /// Links nodes `a` and `b`; the handshake runs as the world runs.
    pub fn link(&mut self, a: usize, b: usize, latency: Duration) -> usize {
        let now = self.now;
        let la = self.nodes[a]
            .vnic
            .add_link(now, LinkConfig::new(LinkKind::Other, MTU))
            .unwrap();
        let lb = self.nodes[b]
            .vnic
            .add_link(now, LinkConfig::new(LinkKind::Other, MTU))
            .unwrap();
        self.wires.push(Wire {
            ends: [(a, la), (b, lb)],
            latency,
            last: [now; 2],
            open: true,
        });
        let w = self.wires.len() - 1;
        self.service(a);
        self.service(b);
        w
    }

    pub fn linked(&self, a: usize, b: usize) -> bool {
        self.wires.iter().any(|w| {
            w.open
                && ((w.ends[0].0 == a && w.ends[1].0 == b)
                    || (w.ends[0].0 == b && w.ends[1].0 == a))
        })
    }

    /// Closes a wire: both transports go away.
    pub fn unlink(&mut self, w: usize) {
        self.wires[w].open = false;
        for (n, link) in self.wires[w].ends {
            self.nodes[n].vnic.remove_link(link);
            self.drain_vnic(n);
            self.service(n);
        }
    }

    fn push(&mut self, at: Time, ev: Ev) {
        self.seq += 1;
        self.queue.insert((at, self.seq), ev);
    }

    /// One driver pass for node `i`: vnic events, vpn flush, egress, wire
    /// writes, vpn events and the timer.
    pub fn service(&mut self, i: usize) {
        let now = self.now;
        self.drain_vnic(i);
        {
            let node = &mut self.nodes[i];
            node.vpn.flush(&mut node.vnic, now);
            let SimNode { vnic, vpn, .. } = node;
            vnic.poll_egress(now, &mut |f| vpn.fill_egress(f.tag, f.peer, f.body));
        }
        // Wire writes.
        for w in 0..self.wires.len() {
            if !self.wires[w].open {
                continue;
            }
            for dir in 0..2 {
                let (n, link) = self.wires[w].ends[dir];
                if n != i {
                    continue;
                }
                let data = take_transmit(&mut self.nodes[i].vnic, link);
                if data.is_empty() {
                    continue;
                }
                let at = (now + self.wires[w].latency).max(self.wires[w].last[dir]);
                self.wires[w].last[dir] = at;
                let to = self.wires[w].ends[1 - dir].0;
                self.push(at, Ev::Deliver { wire: w, to, data });
            }
        }
        self.drain_vpn(i);
        // Timer.
        let node = &mut self.nodes[i];
        let next = [node.vnic.poll_timeout(), node.vpn.poll_timeout()]
            .into_iter()
            .flatten()
            .min();
        if let Some(at) = next
            && node.timer != Some(at)
        {
            node.timer = Some(at);
            let at = at.max(now);
            self.push(at, Ev::Timer { node: i, at });
        }
    }

    fn drain_vnic(&mut self, i: usize) {
        let now = self.now;
        let node = &mut self.nodes[i];
        while let Some(e) = node.vnic.poll_event() {
            match e {
                strims_vnic::Event::PeerAdded { peer, .. } => {
                    if node.auto_join {
                        node.vpn
                            .add_peer(&mut node.vnic, node.net, peer, VPN_PORT, VPN_PORT, node.net)
                            .unwrap();
                    }
                }
                strims_vnic::Event::PeerRemoved { peer, .. } => node.vpn.handle_peer_removed(peer),
                strims_vnic::Event::LinkClosed { .. } => {}
            }
        }
        let _ = now;
    }

    fn drain_vpn(&mut self, i: usize) {
        let mut answers: Vec<(ExchangeId, Answer)> = Vec::new();
        let mut dials: Vec<(ExchangeId, usize)> = Vec::new();
        while let Some(e) = self.nodes[i].vpn.poll_event() {
            let line = self.describe(i, &e);
            self.trace.push(line);
            if self.nodes[i].auto_exchange {
                match &e {
                    Event::GatherLocalDescriptions { exchange, .. } => answers.push((
                        *exchange,
                        Answer::Local(vec![LinkDescription {
                            interface: SIM_INTERFACE.to_owned(),
                            description: i.to_string(),
                        }]),
                    )),
                    Event::DialRemoteDescriptions {
                        exchange,
                        descriptions,
                        ..
                    } => {
                        let target = descriptions
                            .iter()
                            .find(|d| d.interface == SIM_INTERFACE)
                            .and_then(|d| d.description.parse::<usize>().ok());
                        match target {
                            Some(t) if t < self.nodes.len() => dials.push((*exchange, t)),
                            _ => answers.push((*exchange, Answer::Remote(Ok(false)))),
                        }
                    }
                    _ => {}
                }
            }
            self.nodes[i].events.push((self.now, e));
        }
        let now = self.now;
        for (x, t) in dials {
            if !self.linked(i, t) {
                self.link(i, t, Duration::from_millis(10));
            }
            answers.push((x, Answer::Remote(Ok(true))));
        }
        if answers.is_empty() {
            return;
        }
        for (x, a) in answers {
            let vpn = &mut self.nodes[i].vpn;
            let _ = match a {
                Answer::Local(d) => vpn.set_local_descriptions(now, x, Ok(d)),
                Answer::Remote(r) => vpn.set_remote_descriptions_result(now, x, r),
            };
        }
        self.service(i);
    }

    fn describe(&self, i: usize, e: &Event<AppTag>) -> String {
        let name = |id: &ID| {
            self.index(id)
                .map_or_else(|| format!("{id}"), |n| format!("n{n}"))
        };
        let t = (self.now - start()).as_millis();
        match e {
            Event::Message(d) => format!(
                "{t} n{i} msg from {} port {} hops {} {:?}",
                name(&d.src),
                d.dst_port,
                d.hops,
                String::from_utf8_lossy(&d.body)
            ),
            Event::HashTableValue { value, .. } => {
                format!("{t} n{i} ht value {:?}", String::from_utf8_lossy(value))
            }
            Event::HashTableGetClosed { .. } => format!("{t} n{i} ht closed"),
            Event::PeerIndexHost { host, .. } => {
                format!("{t} n{i} pi host {} {}", name(&host.host_id), host.port)
            }
            Event::PeerIndexSearchClosed { .. } => format!("{t} n{i} pi closed"),
            Event::GatherLocalDescriptions { host_id, .. } => {
                format!("{t} n{i} px gather {}", name(host_id))
            }
            Event::DialRemoteDescriptions {
                host_id,
                descriptions,
                ..
            } => format!("{t} n{i} px dial {} {}", name(host_id), descriptions.len()),
            Event::ExchangeClosed {
                host_id, outcome, ..
            } => format!("{t} n{i} px closed {} {outcome:?}", name(host_id)),
        }
    }

    fn input(&mut self, wire: usize, to: usize, mut data: Vec<u8>) {
        if !self.wires[wire].open {
            return;
        }
        let link = self.wires[wire]
            .ends
            .iter()
            .find(|(n, _)| *n == to)
            .map(|(_, l)| *l)
            .unwrap();
        let now = self.now;
        let mut off = 0;
        while off < data.len() {
            let World {
                nodes,
                rng,
                loss,
                frames_lost,
                ..
            } = self;
            let SimNode {
                vnic,
                vpn,
                frame_errors,
                ..
            } = &mut nodes[to];
            let res = vnic.handle_link_input(link, now, &mut data[off..], &mut |f| {
                if *loss > 0.0 && rand_f64(rng) < *loss {
                    *frames_lost += 1;
                    return;
                }
                if let Err(e) = vpn.handle_frame(now, f.handler, f.peer, f.body) {
                    frame_errors.push(e);
                }
            });
            match res {
                Ok(n) => off += n,
                Err(_) => break,
            }
            self.drain_vnic(to);
        }
        self.service(to);
    }

    /// Runs events up to and including `until`.
    pub fn run_until(&mut self, until: Time) {
        while let Some((&(at, seq), _)) = self.queue.first_key_value() {
            if at > until {
                break;
            }
            let ev = self.queue.remove(&(at, seq)).unwrap();
            self.now = self.now.max(at);
            match ev {
                Ev::Deliver { wire, to, data } => self.input(wire, to, data),
                Ev::Timer { node, at } => {
                    if self.nodes[node].timer == Some(at) {
                        self.nodes[node].timer = None;
                        let now = self.now;
                        self.nodes[node].vpn.handle_timeout(now);
                        self.service(node);
                    }
                }
            }
        }
        self.now = self.now.max(until);
    }

    pub fn run_for(&mut self, d: Duration) {
        let until = self.now + d;
        self.run_until(until);
    }

    /// Takes node `i`'s events.
    pub fn take_events(&mut self, i: usize) -> Vec<Event<AppTag>> {
        std::mem::take(&mut self.nodes[i].events)
            .into_iter()
            .map(|(_, e)| e)
            .collect()
    }

    /// Sends from `a` to `b`'s app port and services `a`.
    pub fn send(&mut self, a: usize, b: usize, body: &[u8]) {
        let dst = self.id(b);
        let now = self.now;
        let node = &mut self.nodes[a];
        node.vpn
            .send(now, node.net, dst, APP_PORT, APP_PORT, body)
            .unwrap();
        self.service(a);
    }

    pub fn broadcast(&mut self, a: usize, body: &[u8]) {
        let now = self.now;
        let node = &mut self.nodes[a];
        node.vpn
            .broadcast(now, node.net, APP_PORT, APP_PORT, body)
            .unwrap();
        self.service(a);
    }

    /// Links nodes by XOR distance: each node sorts the others by distance
    /// from its host id and links to a few of the nearest, then to sparser
    /// farther ones.
    pub fn link_kademlia(
        &mut self,
        peers_per_node: usize,
        latency: &mut dyn FnMut(&mut DetRng) -> Duration,
    ) {
        let n = self.nodes.len();
        for i in 0..n {
            let me = self.id(i);
            let mut sorted: Vec<usize> = (0..n).collect();
            sorted.sort_by_key(|&j| me ^ self.id(j));
            let mut added = 0;
            let mut off = 1;
            let mut k = peers_per_node / 2;
            while added < peers_per_node && k != 0 {
                for &j in sorted.iter().skip(off).take(k) {
                    if j == i || self.linked(i, j) {
                        continue;
                    }
                    let l = latency(&mut self.rng);
                    self.link(i, j, l);
                    added += 1;
                }
                off = (off + k) * 2;
                k /= 2;
            }
        }
    }
}

enum Answer {
    Local(Vec<LinkDescription>),
    Remote(Result<bool, String>),
}

pub fn take_transmit(vnic: &mut VnicHost<Tag>, link: PeerKey) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let Ok(data) = vnic.poll_link_transmit(link) else {
            return out;
        };
        if data.is_empty() {
            return out;
        }
        let n = data.len();
        out.extend_from_slice(data);
        vnic.advance_link_transmit(link, n).unwrap();
    }
}

/// The app messages (body text) node `i` received, with their sender.
pub fn messages(events: &[Event<AppTag>]) -> Vec<(ID, usize, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Message(d) => {
                Some((d.src, d.hops, String::from_utf8_lossy(&d.body).into_owned()))
            }
            _ => None,
        })
        .collect()
}
