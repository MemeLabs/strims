// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The link scheduler of [WAN mode](crate#wan-mode).
//!
//! Every link between two scenario hosts is held, so turmoil delivers
//! nothing on its own: a held message draws no turmoil latency and stays
//! in its link's in-flight queue, in send order. Between two steps,
//! [`Scheduler::walk`] times each new message on the run's
//! [`Network`] and releases each message whose time has come with
//! `SentRef::deliver`; turmoil hands it to the receiving host at the start
//! of the next step. Delivery times round up to the next step.
//!
//! The scheduler relies on these properties of turmoil's links:
//!
//! - a link's in-flight queue changes only by appends (new sends, and
//!   messages a host refused and bounced back), by the removal of messages
//!   released in the previous walk, and by partitions (`partition` clears
//!   the queue, `partition_oneway` drops one direction). So a queue is
//!   always the records the last walk kept, followed by new messages. A
//!   walk checks this and fails the run when it doesn't hold.
//! - `hold` turns every message on a link back to held, so it never runs on
//!   a link with a message released in the same walk.
//! - `repair` makes a link healthy, so a repaired link is held anew.
//! - A TCP receiver orders segments by sequence number, so a segment that
//!   arrives late (a retransmission) holds back the rest of its
//!   connection, and only that connection.
//!
//! The controller's links carry nothing and stay with turmoil.
//!
//! With send buffers ([`Scenario::send_buffer`]), each data segment is
//! admitted to its sender's uplink queue once the queue holds less than
//! the model's `uplink_queue` of transmission time, and its bytes then
//! stop counting against its socket's mark (see [`crate::net`]). Admitted
//! segments are credited, and their writers woken, in admission time and
//! record order at the end of the walk that reaches that time.
//!
//! With the window model ([`SendBuffer::window`]) each TCP connection
//! sends its data segments within a congestion window instead, and is
//! credited as it sends them:
//!
//! - A segment waits in its connection's queue while the bytes in flight
//!   (sent, neither acknowledged nor known lost) and it would pass the
//!   window; a connection with nothing in flight sends one anyway.
//! - The access queues hold `uplink_queue` of transmission time, up and
//!   down. A segment that finds one full is lost there (segments are whole
//!   writes, up to a socket's mark, so a queue can pass its bound by one),
//!   and its sender learns so a round trip later: the segment goes to the front of
//!   its connection's queue, and the window falls to [`BETA`] of itself,
//!   at most once per round trip.
//! - A segment through the receiver's downlink is acknowledged once the
//!   acknowledgement crosses back, after the receiver's uplink queue: the
//!   window grows by the bytes acknowledged below its threshold (slow
//!   start) and by one [`MSS`] a window above it.
//!
//! The connection's order on the wire is its send order, and a resent
//! segment holds back the rest of its connection at the receiver.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, SocketAddr};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_wan::wire::{tcp_wire, udp_wire};
use strims_wan::{Carrier, Network, Options, Site};
use turmoil::{Protocol, Segment, Sim};

use crate::net::SendBuffers;
use crate::report::{WanHostReport, WanReport};
use crate::spec::{HostIx, Scenario};
use crate::world::epoch;

/// The wire size charged for a STUN message or a DTLS record other than
/// application data. Their sizes depend on DTLS key material the seed
/// doesn't reach, so a fixed charge keeps a seed's timing reproducible.
const NOMINAL_DATAGRAM: usize = 1228;

/// The run's clock at `elapsed` since its start.
pub(crate) fn sim_time(elapsed: std::time::Duration) -> Time {
    epoch() + Duration::from(elapsed)
}

/// What a message is, as far as matching it against its queue entry goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum What {
    Syn,
    Data { seq: u64, len: usize },
    Fin(u64),
    Rst,
    Datagram(usize),
}

impl What {
    fn of(p: &Protocol) -> What {
        match p {
            Protocol::Tcp(Segment::Syn(_)) => What::Syn,
            Protocol::Tcp(Segment::Data(seq, b)) => What::Data {
                seq: *seq,
                len: b.len(),
            },
            Protocol::Tcp(Segment::Fin(seq)) => What::Fin(*seq),
            Protocol::Tcp(Segment::Rst) => What::Rst,
            Protocol::Udp(d) => What::Datagram(d.0.len()),
        }
    }
}

/// The wire bytes and packets a message is charged.
fn wire(p: &Protocol) -> (usize, u32) {
    match p {
        Protocol::Tcp(Segment::Data(_, b)) => tcp_wire(b.len()),
        Protocol::Tcp(_) => tcp_wire(0),
        Protocol::Udp(d) => match d.0.first() {
            // STUN (RFC 5389 messages start with 0b00) and DTLS
            // change_cipher_spec, alert and handshake records.
            Some(0..=3 | 20..=22) => (NOMINAL_DATAGRAM, 1),
            _ => udp_wire(d.0.len()),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// On its way.
    InFlight,
    /// Through the receiver's downlink: released at the next walk, unless
    /// its pair is frozen.
    Due,
    /// A lost datagram: never released.
    Dropped,
}

#[derive(Clone, Debug)]
struct Record {
    src: SocketAddr,
    dst: SocketAddr,
    from: HostIx,
    to: HostIx,
    what: What,
    wire: usize,
    packets: u32,
    state: State,
}

#[derive(Clone, Copy, Debug)]
enum Event {
    /// The message reaches the receiver's downlink.
    Arrive,
    /// The message is through the receiver's downlink.
    Deliver,
    /// The window model: the sender learns the segment was lost.
    Lost,
    /// The window model: an acknowledgement of `len` bytes of `conn`
    /// reaches the sender.
    Ack {
        conn: (SocketAddr, SocketAddr),
        len: usize,
    },
}

/// The window model's segment size, for the window's growth.
pub(crate) const MSS: f64 = 1448.0;
/// Its initial window: 10 segments.
const INIT_WINDOW: f64 = 10.0 * MSS;
/// The window after a loss, as a share of the window (CUBIC's beta).
pub(crate) const BETA: f64 = 0.7;

/// One TCP connection in the window model.
#[derive(Debug)]
struct Flow {
    cwnd: f64,
    ssthresh: f64,
    /// Bytes sent, neither acknowledged nor known lost.
    in_flight: usize,
    /// Data records waiting for the window, in send order. Bound: the
    /// connection's records in flight.
    pending: VecDeque<u64>,
    /// When the window last fell.
    cut: Time,
}

impl Default for Flow {
    fn default() -> Flow {
        Flow {
            cwnd: INIT_WINDOW,
            ssthresh: f64::INFINITY,
            in_flight: 0,
            pending: VecDeque::new(),
            cut: Time::NIL,
        }
    }
}

#[derive(Clone, Debug)]
struct LinkState {
    /// The link's address pair, as turmoil orders it.
    pair: (IpAddr, IpAddr),
    /// The scenario hosts it joins; `None` for the controller's links.
    hosts: Option<(HostIx, HostIx)>,
    /// Its in-flight messages, in queue order.
    records: VecDeque<u64>,
}

/// One sender's new messages, by receiver and connection addresses.
type Queues = BTreeMap<(HostIx, IpAddr, IpAddr), VecDeque<u64>>;

/// A host pair as a set key.
fn pair_key(a: HostIx, b: HostIx) -> (HostIx, HostIx) {
    (a.min(b), a.max(b))
}

/// The WAN schedule of one run.
pub(crate) struct Scheduler {
    net: Network,
    names: Vec<String>,
    ip_host: BTreeMap<IpAddr, HostIx>,
    /// Every turmoil link, in turmoil's order (which never changes once
    /// the hosts are registered).
    links: Vec<LinkState>,
    /// The link between each pair of hosts.
    link_of: BTreeMap<(HostIx, HostIx), usize>,
    records: BTreeMap<u64, Record>,
    /// Pending events by time, then record. A record has at most one, and
    /// an event of a forgotten record is skipped.
    events: BTreeMap<(Time, u64), Event>,
    next: u64,
    frozen: BTreeSet<(HostIx, HostIx)>,
    partitioned: BTreeSet<(HostIx, HostIx)>,
    tick: Duration,
    buffers: Option<SendBuffers>,
    /// Data segments waiting for room in their sender's uplink queue, by
    /// admission time and record: their connection and length. Bound: the
    /// records in flight.
    admits: BTreeMap<(Time, u64), ((SocketAddr, SocketAddr), usize)>,
    /// The window model's connections. Bound: one per TCP connection that
    /// sent data.
    flows: BTreeMap<(SocketAddr, SocketAddr), Flow>,
    /// Scratch for [`Scheduler::pump`].
    sending: Vec<u64>,
    /// Per host, its uplink queue's wait every [`QUEUE_SAMPLE_EVERY`]:
    /// (time since the start, wait).
    up_queue: Vec<Vec<(Duration, Duration)>>,
    next_queue_sample: Time,
}

/// How often the scheduler samples each host's uplink queue.
const QUEUE_SAMPLE_EVERY: Duration = Duration::from_millis(100);

impl Scheduler {
    /// Holds every link between `spec`'s hosts, which `sim` has registered
    /// under their names, with `buffers` when the scenario has send buffers.
    /// Fails when a host has no site.
    pub fn new(
        sim: &Sim<'_>,
        spec: &Scenario,
        options: Options,
        seed: u64,
        buffers: Option<SendBuffers>,
    ) -> Result<Scheduler, String> {
        let sites = spec
            .hosts
            .iter()
            .map(|h| {
                h.site
                    .ok_or_else(|| format!("wan: host {} has no site", h.name))
            })
            .collect::<Result<Vec<Site>, String>>()?;
        let net = Network::new(&sites, options, &mut det_rng(seed ^ 0x5741_4e00));
        let names: Vec<String> = spec.hosts.iter().map(|h| h.name.clone()).collect();
        let ip_host: BTreeMap<IpAddr, HostIx> = names
            .iter()
            .enumerate()
            .map(|(h, n)| (sim.lookup(n.as_str()), h))
            .collect();
        let mut links = Vec::new();
        sim.links(|iter| {
            for link in iter {
                let pair = link.pair();
                let hosts = ip_host
                    .get(&pair.0)
                    .copied()
                    .zip(ip_host.get(&pair.1).copied());
                links.push(LinkState {
                    pair,
                    hosts,
                    records: VecDeque::new(),
                });
            }
        });
        let link_of = links
            .iter()
            .enumerate()
            .filter_map(|(i, l)| l.hosts.map(|(a, b)| (pair_key(a, b), i)))
            .collect();
        for (i, a) in names.iter().enumerate() {
            for b in names.iter().skip(i + 1) {
                sim.hold(a.as_str(), b.as_str());
            }
        }
        Ok(Scheduler {
            net,
            names,
            ip_host,
            links,
            link_of,
            records: BTreeMap::new(),
            events: BTreeMap::new(),
            next: 0,
            frozen: BTreeSet::new(),
            partitioned: BTreeSet::new(),
            tick: Duration::from(spec.tick),
            buffers,
            admits: BTreeMap::new(),
            flows: BTreeMap::new(),
            sending: Vec::new(),
            up_queue: vec![Vec::new(); spec.hosts.len()],
            next_queue_sample: epoch(),
        })
    }

    fn name(&self, h: HostIx) -> &str {
        self.names.get(h).map_or("?", String::as_str)
    }

    /// One walk, between two steps (see the module docs).
    pub fn walk(&mut self, sim: &Sim<'_>) -> Result<(), String> {
        let now = sim_time(sim.elapsed());
        self.run_events(now);
        let (fresh, released) = self.match_queues(sim)?;
        self.send(now - self.tick, fresh);
        self.purge(sim, &released);
        self.admit(now);
        if now >= self.next_queue_sample {
            self.next_queue_sample = now + QUEUE_SAMPLE_EVERY;
            for (h, samples) in self.up_queue.iter_mut().enumerate() {
                let wait = (self.net.up_free(h) - now).max(Duration::ZERO);
                samples.push((now - epoch(), wait));
            }
        }
        Ok(())
    }

    /// Credits the segments admitted by `now` to their sockets and wakes
    /// the writers they free.
    fn admit(&mut self, now: Time) {
        let Some(b) = &self.buffers else {
            return;
        };
        while let Some(entry) = self.admits.first_entry() {
            if entry.key().0 > now {
                break;
            }
            let (conn, len) = entry.remove();
            if let Some(w) = b.admit(conn, len) {
                w.wake();
            }
        }
    }

    /// Processes the events due by `now`, in time order: arrivals enter the
    /// receiver's downlink, and messages through it become due.
    fn run_events(&mut self, now: Time) {
        while let Some(entry) = self.events.first_entry() {
            let (t, id) = *entry.key();
            if t > now {
                break;
            }
            let event = entry.remove();
            if let Event::Ack { conn, len } = event {
                self.acked(t, conn, len);
                continue;
            }
            let window = self.window();
            let Some(r) = self.records.get_mut(&id) else {
                continue;
            };
            match event {
                Event::Arrive => {
                    let data = match r.what {
                        What::Data { len, .. } => Some(len),
                        _ => None,
                    };
                    if let (Some(cap), Some(_)) = (window, data)
                        && self.net.down_free(r.to) - t >= cap
                    {
                        let back = self.net.one_way(r.to, r.from);
                        self.events.insert((t + back, id), Event::Lost);
                        continue;
                    }
                    let done = self.net.receive(t, r.to, r.wire);
                    if done <= now {
                        r.state = State::Due;
                    } else {
                        self.events.insert((done, id), Event::Deliver);
                    }
                    if let (Some(_), Some(len)) = (window, data) {
                        let back = self.net.one_way(r.to, r.from)
                            + (self.net.up_free(r.to) - done).max(Duration::ZERO);
                        let conn = (r.src, r.dst);
                        self.events
                            .insert((done + back, id), Event::Ack { conn, len });
                    }
                }
                Event::Deliver => r.state = State::Due,
                Event::Lost => {
                    let (conn, from, to) = ((r.src, r.dst), r.from, r.to);
                    let len = match r.what {
                        What::Data { len, .. } => len,
                        _ => 0,
                    };
                    let rtt = self.net.rtt(from, to);
                    let f = self.flows.entry(conn).or_default();
                    f.in_flight = f.in_flight.saturating_sub(len);
                    if f.cut.is_nil() || t - f.cut >= rtt {
                        f.ssthresh = (f.cwnd * BETA).max(2.0 * MSS);
                        f.cwnd = f.ssthresh;
                        f.cut = t;
                    }
                    f.pending.push_front(id);
                    self.pump(t, &[conn]);
                }
                Event::Ack { .. } => {}
            }
        }
    }

    /// The window model's access queue bound, if it runs.
    fn window(&self) -> Option<Duration> {
        self.buffers
            .as_ref()
            .map(|b| b.model())
            .filter(|m| m.window)
            .map(|m| Duration::from(m.uplink_queue))
    }

    /// `len` bytes of `conn` were acknowledged at `t`.
    fn acked(&mut self, t: Time, conn: (SocketAddr, SocketAddr), len: usize) {
        let Some(f) = self.flows.get_mut(&conn) else {
            return;
        };
        f.in_flight = f.in_flight.saturating_sub(len);
        if f.cwnd < f.ssthresh {
            f.cwnd += len as f64;
        } else {
            f.cwnd += MSS * len as f64 / f.cwnd.max(MSS);
        }
        self.pump(t, &[conn]);
    }

    /// Sends what `conns`' windows allow at `at`, a segment from each in
    /// turn.
    fn pump(&mut self, at: Time, conns: &[(SocketAddr, SocketAddr)]) {
        let mut open: Vec<(SocketAddr, SocketAddr)> = conns.to_vec();
        while !open.is_empty() {
            open.retain(|conn| {
                let Some(f) = self.flows.get_mut(conn) else {
                    return false;
                };
                loop {
                    let Some(&id) = f.pending.front() else {
                        return false;
                    };
                    let Some(len) = self.records.get(&id).and_then(|r| match r.what {
                        What::Data { len, .. } => Some(len),
                        _ => None,
                    }) else {
                        f.pending.pop_front();
                        continue;
                    };
                    if f.in_flight > 0 && (f.in_flight + len) as f64 > f.cwnd {
                        return false;
                    }
                    f.pending.pop_front();
                    f.in_flight += len;
                    self.sending.push(id);
                    return true;
                }
            });
            for id in std::mem::take(&mut self.sending) {
                self.transmit_window(at, id);
            }
        }
    }

    /// The window model's send of data record `id` at `at`: credited to
    /// its socket at once, and lost if the uplink queue would overflow.
    fn transmit_window(&mut self, at: Time, id: u64) {
        let Some(cap) = self.window() else {
            return;
        };
        let Some(r) = self.records.get(&id) else {
            return;
        };
        let (from, to, wire, packets, conn) = (r.from, r.to, r.wire, r.packets, (r.src, r.dst));
        let len = match r.what {
            What::Data { len, .. } => len,
            _ => 0,
        };
        if self.buffers.is_some() {
            self.admits.insert((at, id), (conn, len));
        }
        let wait = (self.net.up_free(from) - at).max(Duration::ZERO);
        if wait >= cap {
            let back = self.net.rtt(from, to) + wait;
            self.events.insert((at + back, id), Event::Lost);
            return;
        }
        let t = self.net.send(at, from, to, wire, packets, Carrier::Stream);
        self.events
            .insert((t.arrival + t.retransmit, id), Event::Arrive);
    }

    /// Lines every queue up against its records, releases the due ones and
    /// records new messages. Returns the new records in queue order, and
    /// which links released a message.
    fn match_queues(&mut self, sim: &Sim<'_>) -> Result<(Vec<u64>, Vec<bool>), String> {
        let mut fresh = Vec::new();
        let mut released = vec![false; self.links.len()];
        let mut err = None;
        let links = &mut self.links;
        let records = &mut self.records;
        let ip_host = &self.ip_host;
        let frozen = &self.frozen;
        let next = &mut self.next;
        sim.links(|iter| {
            'links: for (li, link) in iter.enumerate() {
                let Some(ls) = links.get_mut(li) else {
                    err = Some(format!("wan: turmoil has a link {li} it didn't have"));
                    break;
                };
                if link.pair() != ls.pair {
                    err = Some(format!(
                        "wan: link {li} joins {:?}, expected {:?}",
                        link.pair(),
                        ls.pair
                    ));
                    break;
                }
                let Some((a, b)) = ls.hosts else {
                    continue;
                };
                let frozen = frozen.contains(&pair_key(a, b));
                let mut known = std::mem::take(&mut ls.records).into_iter();
                let mut kept = VecDeque::new();
                for sent in link {
                    let (src, dst) = sent.pair();
                    let what = What::of(sent.protocol());
                    if let Some(id) = known.next() {
                        let Some(r) = records.get(&id) else {
                            err = Some(format!("wan: link {li} lost record {id}"));
                            break 'links;
                        };
                        if (r.src, r.dst, r.what) != (src, dst, what) {
                            err = Some(format!(
                                "wan: link {li} holds {src}->{dst} {what:?} where {}->{} {:?} was in flight",
                                r.src, r.dst, r.what
                            ));
                            break 'links;
                        }
                        if r.state == State::Due && !frozen {
                            sent.deliver();
                            records.remove(&id);
                            if let Some(x) = released.get_mut(li) {
                                *x = true;
                            }
                        } else {
                            kept.push_back(id);
                        }
                        continue;
                    }
                    let (Some(&from), Some(&to)) = (ip_host.get(&src.ip()), ip_host.get(&dst.ip()))
                    else {
                        err = Some(format!("wan: link {li} carries {src}->{dst}"));
                        break 'links;
                    };
                    let (wire, packets) = wire(sent.protocol());
                    let id = *next;
                    *next += 1;
                    records.insert(
                        id,
                        Record {
                            src,
                            dst,
                            from,
                            to,
                            what,
                            wire,
                            packets,
                            state: State::InFlight,
                        },
                    );
                    kept.push_back(id);
                    fresh.push(id);
                }
                if known.next().is_some() {
                    err = Some(format!(
                        "wan: link {li} dropped in-flight messages the scheduler didn't"
                    ));
                    break;
                }
                ls.records = kept;
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok((fresh, released)),
        }
    }

    /// Hands the new records to their senders' uplinks, sent at `at`: each
    /// sender serves its links in turn, one message at a time, in queue
    /// order on each link.
    fn send(&mut self, at: Time, fresh: Vec<u64>) {
        let mut by_sender: BTreeMap<HostIx, Queues> = BTreeMap::new();
        for id in fresh {
            if let Some(r) = self.records.get(&id) {
                by_sender
                    .entry(r.from)
                    .or_default()
                    .entry((r.to, r.src.ip(), r.dst.ip()))
                    .or_default()
                    .push_back(id);
            }
        }
        let window = self.window().is_some();
        for queues in by_sender.into_values() {
            let mut queues: Vec<VecDeque<u64>> = queues.into_values().collect();
            let mut flows = Vec::new();
            while queues.iter().any(|q| !q.is_empty()) {
                for q in &mut queues {
                    let Some(id) = q.pop_front() else {
                        continue;
                    };
                    let data = self.records.get(&id).and_then(|r| match r.what {
                        What::Data { .. } if window => Some((r.src, r.dst)),
                        _ => None,
                    });
                    match data {
                        Some(conn) => {
                            self.flows.entry(conn).or_default().pending.push_back(id);
                            if !flows.contains(&conn) {
                                flows.push(conn);
                            }
                        }
                        None => self.transmit(at, id),
                    }
                }
            }
            self.pump(at, &flows);
        }
    }

    fn transmit(&mut self, at: Time, id: u64) {
        let Some(r) = self.records.get_mut(&id) else {
            return;
        };
        let carrier = match r.what {
            What::Datagram(_) => Carrier::Datagram,
            _ => Carrier::Stream,
        };
        if let (Some(b), What::Data { len, .. }) = (&self.buffers, r.what) {
            let q = b.model().uplink_queue;
            let admit = at.max(self.net.up_free(r.from) - Duration::from(q));
            self.admits.insert((admit, id), ((r.src, r.dst), len));
        }
        let t = self.net.send(at, r.from, r.to, r.wire, r.packets, carrier);
        if t.dropped {
            r.state = State::Dropped;
            return;
        }
        let mut arrive = t.arrival + t.retransmit;
        if r.what == What::Syn {
            // The handshake's reply leg: a connect completes a round trip
            // after the SYN leaves.
            arrive += self.net.one_way(r.to, r.from);
        }
        self.events.insert((arrive, id), Event::Arrive);
    }

    /// Clears the lost datagrams of every direction that holds nothing
    /// else, on links that released nothing in this walk and are neither
    /// partitioned nor frozen.
    fn purge(&mut self, sim: &Sim<'_>, released: &[bool]) {
        let mut purges = Vec::new();
        for (li, ls) in self.links.iter().enumerate() {
            let Some((a, b)) = ls.hosts else {
                continue;
            };
            let key = pair_key(a, b);
            if ls.records.is_empty()
                || released.get(li).copied().unwrap_or(true)
                || self.frozen.contains(&key)
                || self.partitioned.contains(&key)
            {
                continue;
            }
            for (x, y) in [(a, b), (b, a)] {
                let mut mine = ls
                    .records
                    .iter()
                    .filter_map(|id| self.records.get(id))
                    .filter(|r| r.from == x)
                    .peekable();
                if mine.peek().is_some() && mine.all(|r| r.state == State::Dropped) {
                    purges.push((li, x, y));
                }
            }
        }
        for (li, x, y) in purges {
            let (nx, ny) = (self.name(x).to_owned(), self.name(y).to_owned());
            sim.partition_oneway(nx.as_str(), ny.as_str());
            sim.repair_oneway(nx.as_str(), ny.as_str());
            sim.hold(nx.as_str(), ny.as_str());
            let Some(ls) = self.links.get_mut(li) else {
                continue;
            };
            let records = &mut self.records;
            ls.records.retain(|id| {
                let mine = records.get(id).is_some_and(|r| r.from == x);
                if mine {
                    records.remove(id);
                }
                !mine
            });
        }
    }

    /// Turmoil partitioned `a` from `b`, which cleared their link: forget
    /// its messages.
    pub fn partition(&mut self, a: HostIx, b: HostIx) {
        if a == b {
            return;
        }
        if let Some(ls) = self
            .link_of
            .get(&pair_key(a, b))
            .and_then(|&li| self.links.get_mut(li))
        {
            for id in ls.records.drain(..) {
                self.records.remove(&id);
            }
        }
        self.net.cut(a, b);
        self.partitioned.insert(pair_key(a, b));
    }

    /// Turmoil repaired the link between `a` and `b` and the caller held it
    /// again.
    pub fn repair(&mut self, a: HostIx, b: HostIx) {
        self.partitioned.remove(&pair_key(a, b));
    }

    /// Messages between `a` and `b` wait at their receiver until
    /// [`Scheduler::thaw`].
    pub fn freeze(&mut self, a: HostIx, b: HostIx) {
        if a != b {
            self.frozen.insert(pair_key(a, b));
        }
    }

    pub fn thaw(&mut self, a: HostIx, b: HostIx) {
        self.frozen.remove(&pair_key(a, b));
    }

    /// The base one-way latency between `a` and `b` becomes `d`.
    pub fn set_latency(&mut self, a: HostIx, b: HostIx, d: Duration) {
        if a != b {
            self.net.set_one_way(a, b, Some(d));
        }
    }

    /// `h` lost power at `now`: its access links forget their queues.
    pub fn crashed(&mut self, h: HostIx, now: Time) {
        self.net.reset(h, now);
        let ip_host = &self.ip_host;
        self.flows.retain(|(src, dst), _| {
            ip_host.get(&src.ip()) != Some(&h) && ip_host.get(&dst.ip()) != Some(&h)
        });
    }

    /// Each host's site and counters.
    pub fn report(&self, spec: &Scenario) -> WanReport {
        WanReport {
            hosts: spec
                .hosts
                .iter()
                .enumerate()
                .filter_map(|(h, hs)| {
                    Some(WanHostReport {
                        name: hs.name.clone(),
                        site: *self.net.site(h)?,
                        stats: self.net.stats(h),
                        up_queue: self.up_queue.get(h).cloned().unwrap_or_default(),
                    })
                })
                .collect(),
        }
    }

    /// Messages the scheduler tracks.
    #[cfg(test)]
    fn in_flight(&self) -> usize {
        self.records.len()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration as StdDuration;
    use strims_node::{Net, TcpSocketConfig};
    use strims_wan::{Access, City, Profile};

    use crate::net::SimNet;
    use crate::spec::SendBuffer;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use turmoil::net::{TcpListener, TcpStream, UdpSocket};

    const QUIET: Options = Options {
        jitter: false,
        loss: false,
    };

    fn spec(sites: &[(&str, Site)]) -> Scenario {
        let mut s = Scenario::new("wan unit");
        for &(name, site) in sites {
            s.host_at(name, site);
        }
        s
    }

    fn build() -> Sim<'static> {
        turmoil::Builder::new()
            .rng_seed(1)
            .epoch(std::time::UNIX_EPOCH + StdDuration::from_millis(crate::EPOCH_MS.unsigned_abs()))
            .simulation_duration(StdDuration::from_secs(60))
            .tick_duration(StdDuration::from_millis(1))
            .build()
    }

    /// Steps `sim` under `wan` until the clients finish.
    fn drive(sim: &mut Sim<'_>, wan: &mut Scheduler) {
        loop {
            wan.walk(sim).unwrap();
            if sim.step().unwrap() {
                break;
            }
        }
    }

    fn ms(d: StdDuration) -> i64 {
        i64::try_from(d.as_millis()).unwrap()
    }

    /// An echo server on `a` (port 9000).
    fn echo(sim: &mut Sim<'_>, name: &str) {
        sim.host(name, || async {
            let l = TcpListener::bind("0.0.0.0:9000").await?;
            loop {
                let (mut s, _) = l.accept().await?;
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 1 << 16];
                    while let Ok(n) = s.read(&mut buf).await {
                        if n == 0 || s.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });
    }

    /// Connect and echo times (ms) of a client of `server`.
    fn echo_client(sim: &mut Sim<'_>, name: &str, server: &'static str, out: Rc<Cell<(i64, i64)>>) {
        sim.client(name, async move {
            let t0 = turmoil::sim_elapsed().unwrap();
            let mut s = TcpStream::connect((server, 9000)).await?;
            let t1 = turmoil::sim_elapsed().unwrap();
            s.write_all(&[7u8; 100]).await?;
            let mut buf = [0u8; 100];
            s.read_exact(&mut buf).await?;
            let t2 = turmoil::sim_elapsed().unwrap();
            out.set((ms(t1 - t0), ms(t2 - t1)));
            Ok(())
        });
    }

    #[test]
    fn round_trips_take_the_paths_rtt() {
        let mut sim = build();
        let sites = [
            ("a", Site::new(City::London, Access::Fiber)),
            ("b", Site::new(City::Tokyo, Access::Fiber)),
        ];
        echo(&mut sim, "a");
        let out = Rc::new(Cell::new((0, 0)));
        echo_client(&mut sim, "b", "a", out.clone());
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        let rtt = wan.net.rtt(0, 1).as_millis();
        drive(&mut sim, &mut wan);
        let (connect, echo) = out.get();
        assert!(
            (connect - rtt).abs() <= 2,
            "connect {connect} ms, rtt {rtt} ms"
        );
        assert!((echo - rtt).abs() <= 2, "echo {echo} ms, rtt {rtt} ms");
        assert_eq!(wan.in_flight(), 0);
    }

    #[test]
    fn uplinks_pace_streams() {
        let mut sim = build();
        let up = 125_000; // 1 Mbit/s
        let sites = [
            ("a", Site::new(City::Frankfurt, Access::Fiber).with_up(up)),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        let got = Rc::new(Cell::new(0));
        sim.host("a", || async {
            let l = TcpListener::bind("0.0.0.0:9000").await?;
            let (mut s, _) = l.accept().await?;
            for _ in 0..10 {
                s.write_all(&[1u8; 14_480]).await?;
            }
            Ok(())
        });
        let g = got.clone();
        sim.client("b", async move {
            let mut s = TcpStream::connect(("a", 9000)).await?;
            let t0 = turmoil::sim_elapsed().unwrap();
            let mut buf = vec![0u8; 144_800];
            s.read_exact(&mut buf).await?;
            g.set(ms(turmoil::sim_elapsed().unwrap() - t0));
            Ok(())
        });
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        drive(&mut sim, &mut wan);
        // 144 800 bytes in 100 packets of 52 header bytes: 150 000 bytes
        // on the wire, 1.2 s at 1 Mbit/s.
        let took = got.get();
        assert!((1190..1215).contains(&took), "{took} ms");
        assert_eq!(
            wan.report(&spec(&sites)).hosts[0].stats.up_bytes,
            150_000 + 52
        );
    }

    /// A 1 Mbit/s sender writes 144 800 bytes through [`SimNet`] with
    /// `buffers`, and `b` reads them. Returns when the last write returned,
    /// when the read finished (ms from the connect) and the sender's
    /// uplink queue peak.
    fn buffered_writes(buffers: Option<SendBuffers>) -> (i64, i64, Duration) {
        let mut sim = build();
        let sites = [
            (
                "a",
                Site::new(City::Frankfurt, Access::Fiber).with_up(125_000),
            ),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        let (wrote, got) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let net = SimNet::new(buffers.clone());
        let w = wrote.clone();
        sim.host("a", move || {
            let (net, w) = (net.clone(), w.clone());
            async move {
                let l = net.tcp_bind("0.0.0.0:9000".into()).await?;
                let (mut s, _) = net.tcp_accept(&l).await?;
                net.configure_tcp_send_queue(&s, &TcpSocketConfig::default())?;
                let t0 = turmoil::sim_elapsed().unwrap();
                for _ in 0..10 {
                    s.write_all(&[1u8; 14_480]).await?;
                }
                w.set(ms(turmoil::sim_elapsed().unwrap() - t0));
                Ok(())
            }
        });
        let g = got.clone();
        sim.client("b", async move {
            let mut s = TcpStream::connect(("a", 9000)).await?;
            let t0 = turmoil::sim_elapsed().unwrap();
            let mut buf = vec![0u8; 144_800];
            s.read_exact(&mut buf).await?;
            g.set(ms(turmoil::sim_elapsed().unwrap() - t0));
            Ok(())
        });
        let spec = spec(&sites);
        let mut wan = Scheduler::new(&sim, &spec, QUIET, 1, buffers).unwrap();
        drive(&mut sim, &mut wan);
        let peak = wan.report(&spec).hosts[0].stats.up_queue_max;
        (wrote.get(), got.get(), peak)
    }

    #[test]
    fn send_buffers_hold_writes_until_the_uplink_queue_has_room() {
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: false,
        });
        let (wrote, got, peak) = buffered_writes(Some(buffers));
        // The writes run ahead of the uplink by its 100 ms queue and the two
        // writes the socket holds (one starts under the 16 KiB mark and ends
        // past it), about 0.46 s of the 1.2 s the bytes take on the wire.
        assert!((700..800).contains(&wrote), "last write after {wrote} ms");
        assert!((1190..1215).contains(&got), "{got} ms");
        // A segment waits at the uplink for at most the queue and those two
        // writes; the FIN, sent with the last write, also for that write.
        assert!(peak <= Duration::from_millis(470), "{peak:?}");
    }

    #[test]
    fn windows_keep_the_uplink_queue_bounded() {
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        let (wrote, got, peak) = buffered_writes(Some(buffers));
        // The window, not the queue, paces the writes: the bytes still take
        // their 1.2 s on the wire, and a message waits at most the queue
        // and the segment let in last (116 ms at 1 Mbit/s).
        assert!(wrote > 700, "last write after {wrote} ms");
        assert!((1190..1600).contains(&got), "{got} ms");
        assert!(peak <= Duration::from_millis(220), "{peak:?}");
    }

    /// `a`, on a 1 Mbit/s uplink, writes 144 800 bytes to each of `b` and
    /// `c` at once; `c`'s downlink takes 200 kbit/s. Returns when `b` and
    /// when `c` had everything (ms from the start).
    fn two_receivers(buffers: SendBuffers) -> (i64, i64) {
        let mut sim = build();
        let sites = [
            (
                "a",
                Site::new(City::Frankfurt, Access::Fiber).with_up(125_000),
            ),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
            (
                "c",
                Site::new(City::Frankfurt, Access::Fiber).with_down(25_000),
            ),
        ];
        let net = SimNet::new(Some(buffers.clone()));
        sim.host("a", move || {
            let net = net.clone();
            async move {
                let l = net.tcp_bind("0.0.0.0:9000".into()).await?;
                let mut writers = Vec::new();
                for _ in 0..2 {
                    let (mut s, _) = net.tcp_accept(&l).await?;
                    net.configure_tcp_send_queue(&s, &TcpSocketConfig::default())?;
                    writers.push(tokio::task::spawn_local(async move {
                        for _ in 0..10 {
                            s.write_all(&[1u8; 14_480]).await?;
                        }
                        Ok::<_, std::io::Error>(s)
                    }));
                }
                for w in writers {
                    let _ = w.await;
                }
                Ok(())
            }
        });
        let (b, c) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        for (name, done) in [("b", b.clone()), ("c", c.clone())] {
            sim.client(name, async move {
                let mut s = TcpStream::connect(("a", 9000)).await?;
                let mut buf = vec![0u8; 144_800];
                s.read_exact(&mut buf).await?;
                done.set(ms(turmoil::sim_elapsed().unwrap()));
                Ok(())
            });
        }
        let spec = spec(&sites);
        let mut wan = Scheduler::new(&sim, &spec, QUIET, 1, Some(buffers)).unwrap();
        drive(&mut sim, &mut wan);
        (b.get(), c.get())
    }

    #[test]
    fn a_slow_receiver_holds_back_its_connection_alone() {
        let model = |window| {
            SendBuffers::new(SendBuffer {
                uplink_queue: StdDuration::from_millis(100),
                window,
            })
        };
        let (b_fifo, c_fifo) = two_receivers(model(false));
        let (b, c) = two_receivers(model(true));
        // Without windows the uplink splits evenly however slow `c` reads:
        // `b` waits for `c`'s half. With them `c`'s window follows its
        // downlink, and `b` gets the rest of the uplink.
        assert!(b_fifo > 2000, "{b_fifo} ms");
        assert!(b < b_fifo * 3 / 4, "{b} ms against {b_fifo} ms");
        assert!(c >= 5800 && c_fifo >= 5800, "{c} and {c_fifo} ms");
    }

    #[test]
    fn without_send_buffers_writes_never_wait() {
        let (wrote, got, peak) = buffered_writes(None);
        assert!(wrote < 10, "last write after {wrote} ms");
        assert!((1190..1215).contains(&got), "{got} ms");
        assert!(peak >= Duration::from_millis(1000), "{peak:?}");
    }

    #[test]
    fn repaired_links_stay_shaped() {
        let mut sim = build();
        let sites = [
            ("a", Site::new(City::NewYork, Access::Fiber)),
            ("b", Site::new(City::SaoPaulo, Access::Fiber)),
        ];
        echo(&mut sim, "a");
        let out = Rc::new(Cell::new((0, 0)));
        let o = out.clone();
        sim.client("b", async move {
            tokio::time::sleep(StdDuration::from_secs(2)).await;
            let t0 = turmoil::sim_elapsed().unwrap();
            let mut s = TcpStream::connect(("a", 9000)).await?;
            s.write_all(&[7u8; 100]).await?;
            let mut buf = [0u8; 100];
            s.read_exact(&mut buf).await?;
            o.set((ms(turmoil::sim_elapsed().unwrap() - t0), 0));
            Ok(())
        });
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        let rtt = wan.net.rtt(0, 1).as_millis();
        for step in 0.. {
            if step == 500 {
                sim.partition("a", "b");
                wan.partition(0, 1);
            }
            if step == 1000 {
                sim.repair("a", "b");
                sim.hold("a", "b");
                wan.repair(0, 1);
            }
            wan.walk(&sim).unwrap();
            if sim.step().unwrap() {
                break;
            }
        }
        // A connect and an echo: two round trips.
        let (took, _) = out.get();
        assert!((took - 2 * rtt).abs() <= 3, "{took} ms, rtt {rtt} ms");
    }

    #[test]
    fn datagrams_are_lost_and_forgotten() {
        let mut sim = build();
        let lossy = |c| {
            let mut s = Site::new(c, Access::Lte);
            s.profile = Profile {
                loss: 0.3,
                ..s.profile
            };
            s
        };
        let sites = [("a", lossy(City::Sydney)), ("b", lossy(City::Sydney))];
        let got = Rc::new(Cell::new(0u32));
        let g = got.clone();
        sim.host("a", move || {
            let g = g.clone();
            async move {
                let s = UdpSocket::bind("0.0.0.0:9000").await?;
                let mut buf = [0u8; 2048];
                loop {
                    s.recv_from(&mut buf).await?;
                    g.set(g.get() + 1);
                }
            }
        });
        sim.client("b", async {
            let s = UdpSocket::bind("0.0.0.0:9001").await?;
            for _ in 0..1000 {
                s.send_to(&[23u8; 600], ("a", 9000)).await?;
                tokio::time::sleep(StdDuration::from_millis(2)).await;
            }
            tokio::time::sleep(StdDuration::from_millis(500)).await;
            Ok(())
        });
        let options = Options {
            jitter: true,
            loss: true,
        };
        let mut wan = Scheduler::new(&sim, &spec(&sites), options, 1, None).unwrap();
        drive(&mut sim, &mut wan);
        // Each datagram crosses two links losing 30 %: 49 % arrive.
        let got = got.get();
        assert!((440..540).contains(&got), "{got} of 1000");
        assert_eq!(u64::from(1000 - got), wan.net.stats(1).drops);
        assert_eq!(wan.in_flight(), 0, "lost datagrams stay tracked");
    }

    #[test]
    fn unexpected_queue_changes_fail_the_walk() {
        let mut sim = build();
        let sites = [
            ("a", Site::new(City::Mumbai, Access::Fiber)),
            ("b", Site::new(City::Singapore, Access::Fiber)),
        ];
        echo(&mut sim, "a");
        let out = Rc::new(Cell::new((0, 0)));
        echo_client(&mut sim, "b", "a", out);
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        for _ in 0..5 {
            wan.walk(&sim).unwrap();
            sim.step().unwrap();
        }
        assert_eq!(wan.in_flight(), 1, "the SYN");
        sim.partition("a", "b");
        assert!(wan.walk(&sim).is_err());
    }
}
