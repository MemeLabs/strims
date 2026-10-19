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

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, SocketAddr};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_wan::wire::{tcp_wire, udp_wire};
use strims_wan::{Carrier, Network, Options, Site};
use turmoil::{Protocol, Segment, Sim};

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
}

impl Scheduler {
    /// Holds every link between `spec`'s hosts, which `sim` has registered
    /// under their names. Fails when a host has no site.
    pub fn new(
        sim: &Sim<'_>,
        spec: &Scenario,
        options: Options,
        seed: u64,
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
        Ok(())
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
            let Some(r) = self.records.get_mut(&id) else {
                continue;
            };
            match event {
                Event::Arrive => {
                    let done = self.net.receive(t, r.to, r.wire);
                    if done <= now {
                        r.state = State::Due;
                    } else {
                        self.events.insert((done, id), Event::Deliver);
                    }
                }
                Event::Deliver => r.state = State::Due,
            }
        }
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
        for queues in by_sender.into_values() {
            let mut queues: Vec<VecDeque<u64>> = queues.into_values().collect();
            while queues.iter().any(|q| !q.is_empty()) {
                for q in &mut queues {
                    if let Some(id) = q.pop_front() {
                        self.transmit(at, id);
                    }
                }
            }
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
    use strims_wan::{Access, City, Profile};
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
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1).unwrap();
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
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1).unwrap();
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
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1).unwrap();
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
        let mut wan = Scheduler::new(&sim, &spec(&sites), options, 1).unwrap();
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
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1).unwrap();
        for _ in 0..5 {
            wan.walk(&sim).unwrap();
            sim.step().unwrap();
        }
        assert_eq!(wan.in_flight(), 1, "the SYN");
        sim.partition("a", "b");
        assert!(wan.walk(&sim).is_err());
    }
}
