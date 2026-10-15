// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic simulation: four hosts on a full mesh of in-memory links
//! with a virtual clock.
//!
//! Each link direction is a reliable byte stream (like TCP or an SCTP data
//! channel) with a bandwidth, a latency drawn per link, and "loss": a
//! segment is lost with some probability and arrives a retransmission
//! timeout later, still in order. Arriving bytes are cut into random
//! chunks. The driver writes at most a socket window at a time and only when
//! the wire is free, so a slow link fills vnic's buffer and blocks its
//! writers, and QoS decides who sends.
//!
//! Traffic:
//! - 0→1 over a slow link: three always-backlogged writers under QoS
//!   classes of weight 1000, 500 and 250 (inverse weights 1, 2, 4), so the
//!   bytes must split 4:2:1;
//! - 2→3: two always-backlogged writers of equal weight under a host rate
//!   limit, released only through `poll_timeout`;
//! - finite streams of sequence-numbered frames between other pairs.
//!
//! Every frame carries its writer's sequence number, and receivers check
//! that each (peer, port) stream arrives complete and in order.

#![allow(
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
use strims_vnic::{Event, Fill, Host, HostConfig, LinkConfig, LinkKind, PeerKey, WriterId};

const MTU: usize = 1200;
const WINDOW: usize = 4096;
const SLOW_BPS: u64 = 1_000_000;
const FAST_BPS: u64 = 20_000_000;
const RATE_LIMIT: u64 = 300_000;
const LOSS: f64 = 0.02;
const RTO: Duration = Duration::from_millis(200);
const RUN: Duration = Duration::from_secs(3);
const FINITE_FRAMES: u64 = 150;

/// Tags are the port numbers.
type Tag = u32;

fn rand_u64(rng: &mut DetRng) -> u64 {
    rand_core::Rng::next_u64(rng)
}

fn rand_f64(rng: &mut DetRng) -> f64 {
    (rand_u64(rng) >> 11) as f64 / (1u64 << 53) as f64
}

struct WriterState {
    port: u16,
    seq: u64,
    /// Frames left to send; `None` is unbounded.
    remaining: Option<u64>,
    rng: DetRng,
}

struct SimHost {
    host: Host<Tag>,
    writers: BTreeMap<WriterId, WriterState>,
    /// Next expected sequence number per (sender, port).
    expect: BTreeMap<(ID, u16), u64>,
    /// Body bytes received per (sender, port).
    bytes: BTreeMap<(ID, u16), u64>,
    timeout_at: Option<Time>,
}

struct Wire {
    from: usize,
    from_link: PeerKey,
    to: usize,
    to_link: PeerKey,
    bps: u64,
    latency: Duration,
    busy_until: Time,
    last_arrival: Time,
}

enum Ev {
    Deliver { wire: usize, data: Vec<u8> },
    Writable { wire: usize },
    Timeout { host: usize },
}

struct Sim {
    now: Time,
    hosts: Vec<SimHost>,
    wires: Vec<Wire>,
    queue: BTreeMap<(Time, u64), Ev>,
    seq: u64,
    rng: DetRng,
    trace: Vec<String>,
}

impl Sim {
    fn new(seed: u64) -> Sim {
        let mut rng = det_rng(seed);
        let start = Time::from_unix_millis(1_700_000_000_000);
        let mut hosts: Vec<SimHost> = (0..4u8)
            .map(|i| {
                let config = HostConfig {
                    label: format!("sim-{i}"),
                    rate_limit: (i == 2).then_some(RATE_LIMIT),
                    ..HostConfig::default()
                };
                SimHost {
                    host: Host::new(key_from_seed(&[i + 1; 32]), config, fork(&mut rng)).unwrap(),
                    writers: BTreeMap::new(),
                    expect: BTreeMap::new(),
                    bytes: BTreeMap::new(),
                    timeout_at: None,
                }
            })
            .collect();
        let mut wires = Vec::new();
        for a in 0..4 {
            for b in a + 1..4 {
                let la = hosts[a]
                    .host
                    .add_link(start, LinkConfig::new(LinkKind::Other, MTU))
                    .unwrap();
                let lb = hosts[b]
                    .host
                    .add_link(start, LinkConfig::new(LinkKind::Other, MTU))
                    .unwrap();
                let latency = Duration::from_millis(5 + (rand_u64(&mut rng) % 45) as i64);
                for (from, fl, to, tl) in [(a, la, b, lb), (b, lb, a, la)] {
                    let bps = if (from, to) == (0, 1) {
                        SLOW_BPS
                    } else {
                        FAST_BPS
                    };
                    wires.push(Wire {
                        from,
                        from_link: fl,
                        to,
                        to_link: tl,
                        bps,
                        latency,
                        busy_until: start,
                        last_arrival: start,
                    });
                }
            }
        }
        let mut sim = Sim {
            now: start,
            hosts,
            wires,
            queue: BTreeMap::new(),
            seq: 0,
            rng,
            trace: Vec::new(),
        };
        for h in 0..4 {
            sim.service(h);
        }
        sim
    }

    fn push(&mut self, at: Time, ev: Ev) {
        self.seq += 1;
        self.queue.insert((at, self.seq), ev);
    }

    fn run(&mut self, until: Time) {
        while let Some((&(at, seq), _)) = self.queue.first_key_value() {
            if at > until {
                break;
            }
            let ev = self.queue.remove(&(at, seq)).unwrap();
            self.now = at;
            match ev {
                Ev::Deliver { wire, data } => self.deliver(wire, data),
                Ev::Writable { wire } => self.service(self.wires[wire].from),
                Ev::Timeout { host } => {
                    if self.hosts[host].timeout_at == Some(at) {
                        self.hosts[host].timeout_at = None;
                        self.service(host);
                    }
                }
            }
        }
    }

    fn deliver(&mut self, wire: usize, mut data: Vec<u8>) {
        let (to, link) = (self.wires[wire].to, self.wires[wire].to_link);
        let now = self.now;
        let mut off = 0;
        while off < data.len() {
            let SimHost {
                host,
                expect,
                bytes,
                ..
            } = &mut self.hosts[to];
            let trace = &mut self.trace;
            off += host
                .handle_link_input(link, now, &mut data[off..], &mut |f| {
                    let seq = u64::from_be_bytes(f.body[..8].try_into().unwrap());
                    let next = expect.entry((f.host_id, f.port)).or_insert(0);
                    assert_eq!(
                        seq, *next,
                        "host {to}: frame out of order on port {}",
                        f.port
                    );
                    *next += 1;
                    *bytes.entry((f.host_id, f.port)).or_insert(0) += f.body.len() as u64;
                    trace.push(format!(
                        "{now} {to} frame {} {} {}",
                        f.port,
                        seq,
                        f.body.len()
                    ));
                })
                .expect("link input");
            while let Some(e) = self.hosts[to].host.poll_event() {
                self.trace.push(format!("{now} {to} {e:?}"));
                if let Event::PeerAdded { peer, host_id } = e {
                    self.on_peer(to, peer, host_id);
                }
            }
        }
        self.service(to);
    }

    /// Opens the scenario's writers when a link comes up.
    fn on_peer(&mut self, h: usize, peer: PeerKey, peer_id: ID) {
        let to = (0..4)
            .find(|&i| self.hosts[i].host.id() == peer_id)
            .unwrap();
        for port in [7u16, 100, 101, 102, 200, 201] {
            self.hosts[h]
                .host
                .set_handler(peer, port, Tag::from(port))
                .unwrap();
        }
        let open = |sim: &mut Sim, port: u16, weight: Option<u64>, remaining: Option<u64>| {
            let host = &mut sim.hosts[h].host;
            let class = match weight {
                Some(w) => host.add_qos_class(host.qos_root(), w).unwrap(),
                None => host.qos_root(),
            };
            let w = host
                .open_writer(peer, port, class, Tag::from(port))
                .unwrap();
            host.set_backlog(w).unwrap();
            let rng = fork(&mut sim.rng);
            sim.hosts[h].writers.insert(
                w,
                WriterState {
                    port,
                    seq: 0,
                    remaining,
                    rng,
                },
            );
        };
        match (h, to) {
            (0, 1) => {
                open(self, 100, Some(1000), None);
                open(self, 101, Some(500), None);
                open(self, 102, Some(250), None);
            }
            (2, 3) => {
                open(self, 200, None, None);
                open(self, 201, None, None);
            }
            (1, 0) | (3, 0) | (1, 2) | (3, 1) | (1, 3) => open(self, 7, None, Some(FINITE_FRAMES)),
            _ => {}
        }
    }

    /// Fills frames and writes what the free wires take, until stuck.
    fn service(&mut self, h: usize) {
        let now = self.now;
        loop {
            let SimHost { host, writers, .. } = &mut self.hosts[h];
            host.poll_egress(now, &mut |f| {
                let w = writers.get_mut(&f.writer).unwrap();
                if w.remaining == Some(0) {
                    return Fill::NONE;
                }
                let len = match w.remaining {
                    // Finite streams use random sizes.
                    Some(_) => 8 + (rand_u64(&mut w.rng) as usize % (f.body.len() - 8)),
                    None => f.body.len(),
                };
                f.body[..8].copy_from_slice(&w.seq.to_be_bytes());
                f.body[8..len].fill(w.port as u8);
                w.seq += 1;
                if let Some(r) = &mut w.remaining {
                    *r -= 1;
                }
                Fill {
                    len,
                    more: w.remaining != Some(0),
                }
            });
            let mut progress = false;
            for i in 0..self.wires.len() {
                if self.wires[i].from != h || self.wires[i].busy_until > now {
                    continue;
                }
                let link = self.wires[i].from_link;
                let data = self.hosts[h].host.poll_link_transmit(link).unwrap();
                if data.is_empty() {
                    continue;
                }
                let n = data.len().min(WINDOW);
                let data = data[..n].to_vec();
                self.hosts[h].host.advance_link_transmit(link, n).unwrap();
                self.send(i, data);
                progress = true;
            }
            if !progress {
                break;
            }
        }
        let next = self.hosts[h]
            .host
            .poll_timeout()
            .map(|t| t.max(now + Duration::MILLISECOND));
        if let Some(at) = next
            && next != self.hosts[h].timeout_at
        {
            self.hosts[h].timeout_at = next;
            self.push(at, Ev::Timeout { host: h });
        }
    }

    fn send(&mut self, wire: usize, data: Vec<u8>) {
        let now = self.now;
        let lost = rand_f64(&mut self.rng) < LOSS;
        let w = &mut self.wires[wire];
        let tx = Duration::from_nanos((data.len() as u64 * 1_000_000_000 / w.bps) as i64);
        w.busy_until = now + tx;
        let mut arrival = w.busy_until + w.latency;
        if lost {
            arrival += RTO;
        }
        arrival = arrival.max(w.last_arrival);
        w.last_arrival = arrival;
        let busy = w.busy_until;
        // Arrive in up to three chunks.
        let mut rest = data;
        for _ in 0..2 {
            if rest.len() < 2 || rand_u64(&mut self.rng).is_multiple_of(2) {
                break;
            }
            let cut = 1 + (rand_u64(&mut self.rng) as usize % (rest.len() - 1));
            let tail = rest.split_off(cut);
            self.push(arrival, Ev::Deliver { wire, data: rest });
            rest = tail;
        }
        self.push(arrival, Ev::Deliver { wire, data: rest });
        self.push(busy, Ev::Writable { wire });
    }

    fn bytes(&self, to: usize, from: usize, port: u16) -> u64 {
        let id = self.hosts[from].host.id();
        self.hosts[to].bytes.get(&(id, port)).copied().unwrap_or(0)
    }
}

fn run_sim(seed: u64) -> Sim {
    let mut sim = Sim::new(seed);
    let end = sim.now + RUN;
    sim.run(end);
    sim
}

#[test]
fn handshakes_order_and_weighted_shares() {
    let sim = run_sim(1);

    // Every host linked to every other, once.
    for (i, h) in sim.hosts.iter().enumerate() {
        assert_eq!(h.host.peer_count(), 3, "host {i}");
        for (j, o) in sim.hosts.iter().enumerate() {
            if i != j {
                assert!(h.host.has_peer(&o.host.id()));
            }
        }
    }

    // Finite streams arrived complete (in order: checked on delivery).
    for (from, to) in [(1, 0), (3, 0), (1, 2), (3, 1), (1, 3)] {
        let id = sim.hosts[from].host.id();
        assert_eq!(
            sim.hosts[to].expect.get(&(id, 7)),
            Some(&FINITE_FRAMES),
            "{from}->{to}"
        );
    }

    // 0→1: the slow link is saturated and split 4:2:1.
    let b: Vec<u64> = [100, 101, 102]
        .iter()
        .map(|&p| sim.bytes(1, 0, p))
        .collect();
    let total: u64 = b.iter().sum();
    let secs = RUN.as_secs_f64();
    assert!(
        total as f64 > SLOW_BPS as f64 * secs * 0.8,
        "slow link underused: {total}"
    );
    for (got, want) in b.iter().zip([4.0 / 7.0, 2.0 / 7.0, 1.0 / 7.0]) {
        let share = *got as f64 / total as f64;
        assert!((share - want).abs() < 0.02, "shares {b:?}");
    }

    // 2→3: the rate limit holds (the bucket allows a one-second burst) and
    // equal weights split evenly.
    let r: Vec<u64> = [200, 201].iter().map(|&p| sim.bytes(3, 2, p)).collect();
    let total = r.iter().sum::<u64>() as f64;
    eprintln!("0->1 bytes by weight 1000/500/250: {b:?}; 2->3 rate-limited: {r:?}");
    let rate = RATE_LIMIT as f64;
    assert!(total <= rate * (secs + 1.0) + MTU as f64, "rate {total}");
    assert!(total >= rate * secs * 0.9, "rate {total}");
    let share = r[0] as f64 / total;
    assert!((share - 0.5).abs() < 0.02, "shares {r:?}");
}

#[test]
fn same_seed_same_trace() {
    let a = run_sim(7);
    let b = run_sim(7);
    assert!(a.trace.len() > 100);
    assert_eq!(a.trace, b.trace);
    let c = run_sim(8);
    assert_ne!(a.trace, c.trace);
}
