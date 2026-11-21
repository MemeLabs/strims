// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A testbed run's packets and hosts, for the tables' `packets`, `hosts`
//! and `node_bins`' `offered_*`.
//!
//! - **Offers.** A node's offered capture (`pcap-<i>-offered.pcap`, a
//!   mirror of its eth0's egress ahead of the delay and the queue) holds
//!   each packet as the node handed it over, TCP's GSO buffers whole. The
//!   ifb ahead of the modem cuts a buffer into the link's packets (its
//!   offloads are off), so each is cut here as its device does: MSS-sized
//!   segments of the testbed's [`MTU`], their IP ids and sequence numbers
//!   counting on from the buffer's. Each packet enters the modem's queue
//!   the node's egress delay after its offer (its `links` line's netem),
//!   in offer order: a fixed delay keeps the order. Its frames that
//!   aren't IPv4 (ARP's, IPv6's neighbour discovery: [`others`]) are
//!   offers too, among its packets in time order, as its queue holds them
//!   and its tbf charges them as it does a packet: kind `control`,
//!   protocol `other`, their bytes past the Ethernet header.
//! - **Fates.** The modem's queue is a FIFO, so what the node's eth0
//!   carried (`pcap-<i>.pcap`, past the queue) is what it was offered, in
//!   order, less what the queue dropped: each offered packet is the first
//!   carried one of its destination, ports, IP id and length at or after
//!   its offer and within [`DROP_WAIT_US`] of it, else the queue's drop.
//!   Its departure is that capture's time; its backlog the charges of the
//!   packets that entered before it and hadn't left, the dropped left out;
//!   its arrival its receiver's own capture of it, inbound. A packet that
//!   entered the queue within twice its time ([`told`]: its `links`
//!   line's queue, the most a packet waits there, and as much again) of
//!   its eth0 capture's last record has no fate the captures tell (its
//!   container's teardown, a capture that stopped), so its node's rows end
//!   before it; its offers count it.
//! - **Hosts.** Each node's row is its `links` line's spec, rate, delay
//!   and queue, its kind (`KINDS`) and its links' stack, its clock the
//!   host's (0); the relay's, unshaped, its own.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use strims_base::time::Duration as SimDuration;
use strims_wan::framing::Framing;

use super::{DROP_WAIT_US, LEAD, Packet, Run, SINK, SUBNET, frames, links_of, packets, resends};
use crate::report::{
    HostReport, HostRow, PacketKind, PacketLoss, PacketProto, PacketRecord, WanReport,
};
use crate::spec::{HostIx, Scenario};
use crate::wan::{QUEUE_SAMPLE_EVERY, SctpProfile};

/// The testbed's MTU: its bridge's and its containers' eth0, whose ifbs cut
/// a GSO buffer into packets of at most this many IP bytes.
pub(super) const MTU: u64 = 1500;

/// A node's access link as its `links` line gives it: `node <i> <spec>
/// <up_mbit> <delay_ms> <queue_ms> [<modem_dev>]`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeLink {
    pub spec: String,
    pub up_mbit: f64,
    pub delay_ms: f64,
    pub queue_ms: f64,
}

/// Node `h`'s link from out dir `dir`'s `links` file; `None` without one,
/// or without its line.
pub fn node_link(dir: &Path, h: usize) -> Option<NodeLink> {
    let text = std::fs::read_to_string(dir.join("links")).ok()?;
    text.lines().find_map(|l| {
        let w: Vec<&str> = l.split_whitespace().collect();
        match w[..] {
            ["node", i, spec, up, delay, queue, ..] if i.parse() == Ok(h) => Some(NodeLink {
                spec: spec.to_owned(),
                up_mbit: up.parse().ok()?,
                delay_ms: delay.parse().ok()?,
                queue_ms: queue.parse().ok()?,
            }),
            _ => None,
        }
    })
}

/// A packet's flow: its destination, protocol and ports.
type Flow = (Option<u8>, u8, u16, u16);

/// One packet a node offered its link, as its device cut it.
struct Offered {
    /// When the node offered it (Unix µs).
    us: u64,
    dst: Option<u8>,
    proto: u8,
    ports: (u16, u16),
    ip_id: u16,
    ip_len: u64,
    /// TCP: its sequence number and payload; a pure acknowledgement.
    seq: Option<u32>,
    payload: u64,
    pure_ack: bool,
    /// Its buffer's resend of bytes its flow had offered (TCP).
    resent: bool,
    /// The packet as the capture read it, for the stack's reading of a
    /// datagram's length.
    datagram: bool,
    /// A frame that isn't IPv4 ([`others`]).
    frame: bool,
}

/// `p`'s IP protocol, from its parsed transport.
fn proto_of(p: &Packet) -> u8 {
    if p.tcp.is_some() {
        6
    } else if p.udp.is_some() {
        17
    } else {
        0
    }
}

/// `p`'s ports, TCP's or UDP's.
fn ports_of(p: &Packet) -> (u16, u16) {
    p.tcp.map(|(a, b, _)| (a, b)).or(p.udp).unwrap_or_default()
}

/// Node `me`'s offers `ps` (its offered capture, in order), each GSO buffer
/// cut into the link's packets ([`MTU`]).
fn cut(ps: &[&Packet], me: Option<u8>) -> Vec<Offered> {
    let mine: Vec<&Packet> = ps
        .iter()
        .copied()
        .filter(|p| p.src.is_some() && p.src == me)
        .collect();
    let again = resends(mine.iter().copied());
    let mut out = Vec::with_capacity(mine.len());
    for (p, resent) in mine.iter().zip(again) {
        let header = p.ip_len.saturating_sub(p.payload);
        let proto = proto_of(p);
        match p.tcp {
            Some((_, _, seq)) if p.ip_len > MTU && p.payload > 0 => {
                let mss = MTU.saturating_sub(header).max(1);
                let (mut left, mut k) = (p.payload, 0u64);
                while left > 0 {
                    let take = left.min(mss);
                    out.push(Offered {
                        us: p.us,
                        dst: p.dst,
                        proto,
                        ports: ports_of(p),
                        ip_id: p.ip_id.wrapping_add(k as u16),
                        ip_len: header + take,
                        seq: Some(seq.wrapping_add((k * mss) as u32)),
                        payload: take,
                        pure_ack: false,
                        resent,
                        datagram: false,
                        frame: false,
                    });
                    left -= take;
                    k += 1;
                }
            }
            _ => out.push(Offered {
                us: p.us,
                dst: p.dst,
                proto,
                ports: ports_of(p),
                ip_id: p.ip_id,
                ip_len: p.ip_len,
                seq: p.tcp.map(|(_, _, s)| s),
                payload: p.payload,
                pure_ack: p.pure_ack,
                resent,
                datagram: p.udp.is_some(),
                frame: false,
            }),
        }
    }
    out
}

/// Out dir `dir`'s capture `name`'s frames from node `me` (its address's
/// last octet) that aren't IPv4: ARP's and IPv6's neighbour discovery,
/// which its queue takes and its tbf charges as it does a packet. The
/// node's own come from the MAC its IPv4 packets do. Each its time (Unix
/// µs) and its bytes past the Ethernet header.
fn others(dir: &Path, name: &str, me: Option<u8>) -> Vec<(u64, u64)> {
    let Ok(bytes) = std::fs::read(dir.join(name)) else {
        return Vec::new();
    };
    let fs = frames(&bytes).unwrap_or_default();
    let ipv4 = |d: &[u8]| d.get(12..14) == Some(&[0x08, 0x00][..]);
    let mac = fs.iter().find_map(|&(_, _, d)| {
        let mine = ipv4(d)
            && d.get(26..29) == Some(&SUBNET[..])
            && d.get(29).copied().is_some_and(|o| Some(o) == me);
        mine.then(|| d.get(6..12)).flatten()
    });
    let Some(mac) = mac else {
        return Vec::new();
    };
    fs.iter()
        .filter(|&&(_, _, d)| !ipv4(d) && d.get(6..12) == Some(mac))
        .map(|&(us, wire, _)| (us, u64::from(wire).saturating_sub(14)))
        .collect()
}

/// `offers` (in order) with `frames` ([`others`]: time, bytes) among them
/// in time order, a frame after the packets of its µs.
fn with_frames(offers: Vec<Offered>, frames: &[(u64, u64)]) -> Vec<Offered> {
    let frame = |&(us, ip_len): &(u64, u64)| Offered {
        us,
        dst: None,
        proto: 0,
        ports: (0, 0),
        ip_id: 0,
        ip_len,
        seq: None,
        payload: 0,
        pure_ack: false,
        resent: false,
        datagram: false,
        frame: true,
    };
    let mut out = Vec::with_capacity(offers.len() + frames.len());
    let mut fs = frames.iter().peekable();
    for o in offers {
        while let Some(f) = fs.next_if(|f| f.0 < o.us) {
            out.push(frame(f));
        }
        out.push(o);
    }
    out.extend(fs.map(frame));
    out
}

/// A packet's identity at both ends of a link: its source, destination,
/// protocol, ports, IP id and length.
type Key = (Option<u8>, Option<u8>, u8, (u16, u16), u16, u64);

/// When each of `offers` left its link, matched in order against what the
/// link carried (`carried`, in capture order: FIFO, so a subsequence of
/// the offers): the first carried packet of its key at or after its offer
/// and within [`DROP_WAIT_US`], else `None`, the queue's drop.
fn fates(offers: &[Offered], carried: &[&Packet], me: Option<u8>) -> Vec<Option<u64>> {
    let mut at: BTreeMap<Key, VecDeque<u64>> = BTreeMap::new();
    for p in carried {
        at.entry((me, p.dst, proto_of(p), ports_of(p), p.ip_id, p.ip_len))
            .or_default()
            .push_back(p.us);
    }
    offers
        .iter()
        .map(|o| {
            let q = at.get_mut(&(me, o.dst, o.proto, o.ports, o.ip_id, o.ip_len))?;
            // One carried before it was offered is another's (a capture's
            // edge): never this one.
            while q.front().is_some_and(|&t| t < o.us) {
                q.pop_front();
            }
            match q.front() {
                Some(&t) if t <= o.us + DROP_WAIT_US => q.pop_front(),
                _ => None,
            }
        })
        .collect()
}

/// Of a node's `offers` (in order), those whose fates its eth0 capture
/// tells, its last record at Unix µs `end`: each that entered the queue
/// (`delay_us` after its offer) at least twice the queue's time (`link`'s;
/// [`DROP_WAIT_US`] without a link) before it, as a packet waits at most
/// the queue's time there.
fn told<'a>(
    offers: &'a [Offered],
    delay_us: u64,
    link: Option<&NodeLink>,
    end: u64,
) -> &'a [Offered] {
    let wait = link.map_or(DROP_WAIT_US, |l| (2.0 * l.queue_ms * 1000.0).round() as u64);
    let n = offers
        .iter()
        .take_while(|o| o.us + delay_us + wait <= end)
        .count();
    offers.get(..n).unwrap_or_default()
}

/// The node at address octet `o`, as `events` numbers hosts, of `nodes`.
fn node_of(o: Option<u8>, nodes: usize) -> Option<HostIx> {
    let i = usize::from(o?.checked_sub(10)?);
    (i < nodes).then_some(i)
}

/// A node's packets as its captures give them: the `packets` table's rows
/// (with both captures: its offered one and its eth0's), and its offers of
/// data to the swarm's nodes (with an offered capture: `None` without).
#[derive(Clone, Debug, Default)]
pub struct NodeCapture {
    pub packets: Vec<PacketRecord>,
    pub offers: Option<Vec<(SimDuration, u64, u32)>>,
}

/// Each of out dir `dir`'s `nodes` nodes' captures (see the module docs),
/// node `h` on `stacks[h]` (its data channels' stack, which reads a
/// datagram's length; `None` for TCP), times since Unix µs `run0_us`.
pub fn captures(
    dir: &Path,
    nodes: usize,
    stacks: &[Option<SctpProfile>],
    run0_us: u64,
) -> Vec<NodeCapture> {
    let since = |us: u64| {
        SimDuration::from_micros(i64::try_from(us.saturating_sub(run0_us)).unwrap_or(i64::MAX))
    };
    let mut out = vec![NodeCapture::default(); nodes];
    // Each sender's rows and the keys their receivers' captures know them
    // by: (sender, row, receiver, key, departure).
    let mut pending: Vec<(HostIx, usize, HostIx, Key, u64)> = Vec::new();
    for (h, cap) in out.iter_mut().enumerate() {
        let me = u8::try_from(10 + h).ok();
        let link = node_link(dir, h);
        let delay_us = link
            .as_ref()
            .map_or(0, |l| (l.delay_ms * 1000.0).round() as u64);
        let framing = link
            .as_ref()
            .and_then(|l| strims_wan::testbed::Spec::parse(&l.spec).ok())
            .map_or(Framing::ETHERNET, |x| x.framing());
        let stack = stacks.get(h).copied().flatten();
        let Some(offered) = read(dir, &format!("pcap-{h}-offered.pcap")) else {
            continue;
        };
        let refs: Vec<&Packet> = offered.iter().collect();
        let offers = with_frames(
            cut(&refs, me),
            &others(dir, &format!("pcap-{h}-offered.pcap"), me),
        );
        // Its data to the swarm, as it entered the queue.
        cap.offers = Some(
            offers
                .iter()
                .filter(|o| {
                    node_of(o.dst, nodes).is_some()
                        && kind_estimated(o, stack).0 == PacketKind::Data
                })
                .map(|o| (since(o.us + delay_us), o.ip_len, 1))
                .collect(),
        );
        let Some(carried) = read(dir, &format!("pcap-{h}.pcap")) else {
            continue;
        };
        let end = carried.iter().map(|p| p.us).max().unwrap_or(0);
        let offers = told(&offers, delay_us, link.as_ref(), end);
        // Its frames that aren't IPv4 as packets of their length, no
        // address and no id: in order, as the link carried them.
        let frames: Vec<Packet> = others(dir, &format!("pcap-{h}.pcap"), me)
            .into_iter()
            .map(|(us, ip_len)| Packet {
                us,
                src: me,
                dst: None,
                ip_len,
                pure_ack: false,
                payload: 0,
                tcp: None,
                ip_id: 0,
                udp: None,
            })
            .collect();
        let carried: Vec<&Packet> = carried
            .iter()
            .filter(|p| p.src.is_some() && p.src == me)
            .chain(&frames)
            .collect();
        let sent = fates(offers, &carried, me);
        let mut queue: VecDeque<(u64, u64)> = VecDeque::new();
        let mut held = 0u64;
        let mut first: BTreeMap<Flow, u32> = BTreeMap::new();
        let mut ords: BTreeMap<(HostIx, Flow), u32> = BTreeMap::new();
        let mut per_peer: BTreeMap<HostIx, u32> = BTreeMap::new();
        let mut rows = Vec::with_capacity(offers.len());
        for (o, sent) in offers.iter().zip(sent) {
            let entry = o.us + delay_us;
            while let Some(&(t, c)) = queue.front()
                && t <= entry
            {
                held -= c;
                queue.pop_front();
            }
            let charge = framing.packet(o.ip_len as usize) as u64;
            let peer = node_of(o.dst, nodes);
            let flow = (o.dst, o.proto, o.ports.0, o.ports.1);
            let link = peer.map(|p| {
                let next = per_peer.get(&p).copied().unwrap_or(0);
                let ord = *ords.entry((p, flow)).or_insert(next);
                if ord == next {
                    per_peer.insert(p, next + 1);
                }
                ord
            });
            let range = o.seq.filter(|_| o.payload > 0).map(|seq| {
                let base = *first.entry(flow).or_insert(seq);
                let lo = u64::from(seq.wrapping_sub(base));
                (lo, lo + o.payload)
            });
            let (k, estimated) = kind_estimated(o, stack);
            if let (Some(p), Some(t)) = (peer, sent) {
                pending.push((
                    h,
                    rows.len(),
                    p,
                    (me, o.dst, o.proto, o.ports, o.ip_id, o.ip_len),
                    t,
                ));
            }
            rows.push(PacketRecord {
                at: since(entry),
                sent: sent.map(since),
                arrived: None,
                lost: sent.is_none().then_some(PacketLoss::Queue),
                peer,
                link,
                proto: match o.proto {
                    6 => PacketProto::Tcp,
                    17 => PacketProto::Udp,
                    _ => PacketProto::Other,
                },
                ip_bytes: u32::try_from(o.ip_len).unwrap_or(u32::MAX),
                charged_bytes: Some(u32::try_from(charge).unwrap_or(u32::MAX)),
                kind: k,
                range,
                resent: (o.proto == 6 && o.payload > 0).then_some(o.resent),
                estimated,
                backlog_bytes: Some(held),
            });
            if let Some(t) = sent.filter(|&t| t > entry) {
                queue.push_back((t, charge));
                held += charge;
            }
        }
        cap.packets = rows;
    }
    // Each receiver's capture, inbound, tells when what it was sent
    // reached it.
    pending.sort_by_key(|&(_, _, p, _, _)| p);
    let mut i = 0;
    while i < pending.len() {
        let peer = pending.get(i).map_or(0, |x| x.2);
        let j = pending
            .get(i..)
            .map_or(0, |r| r.partition_point(|x| x.2 == peer))
            + i;
        if let Some(ps) = read(dir, &format!("pcap-{peer}.pcap")) {
            let me = u8::try_from(10 + peer).ok();
            let mut at: BTreeMap<Key, VecDeque<u64>> = BTreeMap::new();
            for p in ps.iter().filter(|p| p.dst.is_some() && p.dst == me) {
                at.entry((p.src, p.dst, proto_of(p), ports_of(p), p.ip_id, p.ip_len))
                    .or_default()
                    .push_back(p.us);
            }
            for &(h, row, _, key, sent) in pending.get(i..j).into_iter().flatten() {
                let Some(q) = at.get_mut(&key) else {
                    continue;
                };
                while q.front().is_some_and(|&t| t < sent) {
                    q.pop_front();
                }
                let got = match q.front() {
                    Some(&t) if t <= sent + DROP_WAIT_US => q.pop_front(),
                    _ => None,
                };
                if let Some(r) = out.get_mut(h).and_then(|c| c.packets.get_mut(row)) {
                    r.arrived = got.map(since);
                }
            }
        }
        i = j;
    }
    out
}

/// Fills the run's packets (`WanHostReport::packets`, each node with both
/// captures), its nodes' offers (`WanHostReport::offers`, each node with
/// an offered capture) and its hosts' rows (`HostReport::host_row`) (see
/// the module docs). `wan`'s hosts are `s`'s, in order.
pub(super) fn fill(run: &Run, s: &Scenario, wan: &mut WanReport, hosts: &mut [HostReport]) {
    let nodes = run.nodes.len();
    let run0_us = run.start_ms.saturating_sub(LEAD.as_millis() as u64) * 1000;
    let stacks: Vec<Option<SctpProfile>> = (0..nodes)
        .map(|h| s.hosts.get(h).and_then(|x| x.rtc_stack))
        .collect();
    for (h, cap) in captures(&run.dir, nodes, &stacks, run0_us)
        .into_iter()
        .enumerate()
    {
        let name = s.hosts.get(h).map(|x| x.name.as_str());
        if let Some(w) = wan.hosts.iter_mut().find(|w| Some(w.name.as_str()) == name) {
            w.packets = cap.packets;
            w.offers = cap.offers;
        }
    }
    for (h, host) in hosts.iter_mut().enumerate() {
        host.host_row = host_row(run, s, h);
    }
}

/// Each node's uplink queue (`WanHostReport::up_queue`, `up_backlog`)
/// where its captures tell it, sampled where the sim samples its uplinks:
/// every [`QUEUE_SAMPLE_EVERY`] from the run's start (the bins' edges) to
/// its last packet's entry, what its queue held then ([`queue_every`]),
/// and as a wait that over its tbf's rate as its tc samples show it
/// (`rates`, from `Network::read`): the latest sample's, the first's after
/// a step logged since that one, the first's before any, else the
/// scenario's. Its tc samples, which fall at their sampler's own phase, up
/// to an interval before an edge, stay past that alone, and the column
/// ends at the captures' last sample where they don't go on to the next
/// edge (a sample of theirs by it, their sampler not stopped before it):
/// that edge's would be the last sample's, an interval old. A node without
/// a tc log has no tbf to queue at, and its queue stays empty, as its
/// drops do ([`drops`]).
pub(super) fn queues(s: &Scenario, wan: &mut WanReport, rates: &[Vec<(SimDuration, u64)>]) {
    for (h, spec) in s.hosts.iter().enumerate() {
        let (Some(site), Some(w)) = (
            spec.site,
            wan.hosts.iter_mut().find(|w| w.name == spec.name),
        ) else {
            continue;
        };
        let tc = rates.get(h).map_or(&[][..], Vec::as_slice);
        if tc.is_empty() {
            continue;
        }
        let held = queue_every(&w.packets, QUEUE_SAMPLE_EVERY);
        let Some(&(last, _)) = held.last() else {
            continue;
        };
        let rate_at = |t: SimDuration| {
            let step = spec
                .up_steps
                .iter()
                .rfind(|&&(at, _)| SimDuration::from(at) <= t);
            let i = tc.partition_point(|&(x, _)| x <= t);
            let sample = match i.checked_sub(1).and_then(|i| tc.get(i)) {
                // A step logged since that sample (steps.log stamps one once
                // applied): the first sample after it shows its rate.
                Some(&(x, _)) if step.is_some_and(|&(at, _)| SimDuration::from(at) > x) => {
                    tc.get(i)
                }
                Some(s) => Some(s),
                None => tc.first(),
            };
            sample
                .map_or_else(|| step.map_or(site.profile.up, |&(_, r)| r), |&(_, r)| r)
                .max(1)
        };
        let after = w.up_queue.partition_point(|&(t, _)| t <= last);
        let next = last + QUEUE_SAMPLE_EVERY;
        let covered = w.up_queue.get(after).is_some_and(|&(t, _)| t <= next)
            && w.up_queue_until.is_none_or(|u| u >= next);
        if !covered {
            w.up_queue_until = Some(last);
        }
        let tail: Vec<_> = w.up_queue.drain(after..).collect();
        w.up_queue = held
            .iter()
            .map(|&(t, b)| {
                let wait = i64::try_from(b * 1_000_000 / rate_at(t)).unwrap_or(i64::MAX);
                (t, SimDuration::from_micros(wait))
            })
            .chain(tail)
            .collect();
        let after = w.up_backlog.partition_point(|&(t, _)| t <= last);
        let tail: Vec<_> = w.up_backlog.drain(after..).collect();
        w.up_backlog = held.into_iter().chain(tail).collect();
    }
}

/// A node's uplink queue every `every` from the run's start, as its
/// packets (`WanHostReport::packets`) tell it: at each time to its last
/// packet's entry, the charges of those that had entered its queue and not
/// left, the dropped left out, as each one's `backlog_bytes` counts those
/// ahead of it. None without packets.
fn queue_every(packets: &[PacketRecord], every: SimDuration) -> Vec<(SimDuration, u64)> {
    let Some(last) = packets.iter().map(|p| p.at).max() else {
        return Vec::new();
    };
    if every <= SimDuration::ZERO {
        return Vec::new();
    }
    let (mut joins, mut leaves) = (Vec::new(), Vec::new());
    for p in packets {
        if let Some(sent) = p.sent.filter(|&t| t > p.at) {
            let c = u64::from(p.charged_bytes.unwrap_or(p.ip_bytes));
            joins.push((p.at, c));
            leaves.push((sent, c));
        }
    }
    joins.sort_by_key(|&(t, _)| t);
    leaves.sort_by_key(|&(t, _)| t);
    let (mut joins, mut leaves) = (joins.into_iter().peekable(), leaves.into_iter().peekable());
    let (mut q, mut out, mut t) = (0u64, Vec::new(), SimDuration::ZERO);
    while t <= last {
        while let Some((_, c)) = joins.next_if(|&(x, _)| x <= t) {
            q += c;
        }
        // Each one left after it entered, so it is in `q` by now.
        while let Some((_, c)) = leaves.next_if(|&(x, _)| x <= t) {
            q = q.saturating_sub(c);
        }
        out.push((t, q));
        t += every;
    }
    out
}

/// A TCP bench's out dir's captures (`hack/testbed/bench.sh`'s: run.sh's
/// with `RELAY=0`, which has no node records), each node of its `links`
/// file's, times from its schedule's start (node 0's line `{"ev":
/// "start", "ms": ...}`): the schedule's start (Unix ms) and each node's.
pub fn bench_captures(dir: &Path) -> Result<(u64, Vec<NodeCapture>), String> {
    let nodes = std::fs::read_to_string(dir.join("links"))
        .map_err(|e| format!("links: {e}"))?
        .lines()
        .filter(|l| l.starts_with("node "))
        .count();
    let start = std::fs::read_to_string(dir.join("node-0.jsonl"))
        .map_err(|e| format!("node-0.jsonl: {e}"))?
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|v| v.get("ev").and_then(serde_json::Value::as_str) == Some("start"))
        .and_then(|v| v.get("ms")?.as_u64())
        .ok_or("node-0.jsonl: no schedule start")?;
    Ok((
        start,
        captures(dir, nodes, &vec![None; nodes], start * 1000),
    ))
}

/// What an offer was (a frame that isn't IPv4 a control packet), and
/// whether its length alone said so (a data channel's datagram, inside
/// DTLS).
fn kind_estimated(o: &Offered, stack: Option<SctpProfile>) -> (PacketKind, bool) {
    if o.frame {
        return (PacketKind::Control, false);
    }
    if o.dst == Some(SINK) {
        return (PacketKind::Competing, false);
    }
    match o.proto {
        6 if o.pure_ack => (PacketKind::Ack, false),
        6 if o.payload > 0 => (PacketKind::Data, false),
        6 => (PacketKind::Control, false),
        17 => {
            let sack = o.datagram && stack.is_some_and(|x| x.datagram_payload(o.ip_len) == 0);
            (
                if sack {
                    PacketKind::Sack
                } else {
                    PacketKind::Data
                },
                true,
            )
        }
        _ => (PacketKind::Data, false),
    }
}

/// Out dir `dir`'s file `name`'s IPv4 packets, if it is a capture.
fn read(dir: &Path, name: &str) -> Option<Vec<Packet>> {
    std::fs::read(dir.join(name)).ok().and_then(|b| packets(&b))
}

/// Host `h` of the run as the `hosts` table has it: a node's from its
/// `links` line and its kind, the relay's unshaped; `None` for a host the
/// testbed has none of.
fn host_row(run: &Run, s: &Scenario, h: HostIx) -> Option<HostRow> {
    let nodes = run.nodes.len();
    let kinds: Vec<&str> = run
        .settings
        .get("KINDS")
        .map_or_else(Vec::new, |k| k.split_whitespace().collect());
    let kind_of = |i: usize| match kinds.get(i).copied().unwrap_or("native") {
        "chromium" => "chromium",
        "firefox" => "firefox",
        _ => "native",
    };
    let role = if s.swarms.iter().any(|w| w.seed == h) {
        "seed"
    } else if s.swarms.iter().any(|w| w.viewers.contains(&h)) {
        "viewer"
    } else if h >= nodes
        && s.hosts
            .get(h)
            .is_some_and(|x| x.class == crate::spec::HostClass::Infra)
    {
        "relay"
    } else {
        return None;
    };
    if role == "relay" {
        return Some(HostRow {
            role,
            kind: "native",
            stack: "linux-tcp".to_owned(),
            ..HostRow::default()
        });
    }
    let kind = kind_of(h);
    // A native node's links are TCP, but those to browsers over WebRTC,
    // which are str0m's data channels.
    let webrtc = run.settings.get("BROWSER_LINKS").map(String::as_str) == Some("webrtc");
    let peers: Vec<usize> = run
        .file("topology")
        .map(|t| links_of(&t))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(a, b)| (a == h).then_some(b).or((b == h).then_some(a)))
        .collect();
    let stack = match kind {
        "native"
            if webrtc && !peers.is_empty() && peers.iter().all(|&p| kind_of(p) != "native") =>
        {
            "str0m"
        }
        "native" => "linux-tcp",
        _ => "dcsctp",
    };
    let link = node_link(&run.dir, h);
    let us = |ms: f64| (ms * 1000.0).round() as i64;
    Some(HostRow {
        role,
        kind,
        stack: stack.to_owned(),
        link_spec: link.as_ref().map(|l| l.spec.clone()),
        up_bps: link.as_ref().map(|l| (l.up_mbit * 125_000.0) as u64),
        delay_us: link.as_ref().map(|l| us(l.delay_ms)),
        queue_us: link.as_ref().map(|l| us(l.queue_ms)),
        clock_us: 0,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// A libpcap file of Ethernet `frames` (Unix µs, the frame).
    fn pcap(frames: &[(u64, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0xa1b2_c3d4u32.to_le_bytes());
        out.extend_from_slice(&[2, 0, 4, 0]);
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&65535u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        for (us, frame) in frames {
            let len = frame.len() as u64;
            for w in [us / 1_000_000, us % 1_000_000, len, len] {
                out.extend_from_slice(&(w as u32).to_le_bytes());
            }
            out.extend_from_slice(frame);
        }
        out
    }

    /// An Ethernet frame of ethertype `kind` from the host whose address's
    /// last octet is `src` (its MAC's last byte), carrying `body`.
    fn frame(src: u8, kind: [u8; 2], body: &[u8]) -> Vec<u8> {
        let mut f = vec![0u8; 6];
        f.extend_from_slice(&[2, 0, 0, 0, 0, src]);
        f.extend_from_slice(&kind);
        f.extend_from_slice(body);
        f
    }

    /// An ARP request from the host `src`: 28 bytes past its frame's
    /// header.
    fn arp(src: u8) -> Vec<u8> {
        frame(src, [0x08, 0x06], &[0; 28])
    }

    /// A TCP packet from `src` to `dst` (last octets), ports 4000 to 5000,
    /// of `payload` bytes at sequence number `seq`, IP id `id`, its flags
    /// `flags`, with the timestamp option (a 32-byte header), in its
    /// frame.
    fn tcp(src: u8, dst: u8, payload: usize, seq: u32, id: u16, flags: u8) -> Vec<u8> {
        let len = 20 + 32 + payload;
        let mut p = vec![0u8; len];
        p[0] = 0x45;
        p[2..4].copy_from_slice(&(len as u16).to_be_bytes());
        p[4..6].copy_from_slice(&id.to_be_bytes());
        p[9] = 6;
        p[12..16].copy_from_slice(&[10, 77, 0, src]);
        p[16..20].copy_from_slice(&[10, 77, 0, dst]);
        p[20..22].copy_from_slice(&4000u16.to_be_bytes());
        p[22..24].copy_from_slice(&5000u16.to_be_bytes());
        p[24..28].copy_from_slice(&seq.to_be_bytes());
        p[20 + 12] = 8 << 4;
        p[20 + 13] = flags;
        frame(src, [0x08, 0x00], &p)
    }

    #[test]
    fn a_nodes_offers_meet_what_its_link_carried_and_its_peer_received() {
        let dir = std::env::temp_dir().join(format!("strims-packets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("links"),
            "node 0 tbf 10 20 100 eth0\nnode 1 tbf 20 10 100 eth0\n",
        )
        .unwrap();
        let t0 = 1_000_000_000u64;
        // A GSO buffer of 3000 bytes (three packets: 1448, 1448 and 104
        // bytes, IP ids 10 to 12), a pure ACK, a competing upload's
        // packet and an ARP request.
        let offered = [
            (t0, tcp(10, 11, 3000, 1000, 10, 0x18)),
            (t0 + 100, tcp(10, 11, 0, 4000, 13, 0x10)),
            (t0 + 200, tcp(10, 250, 100, 7, 14, 0x18)),
            (t0 + 300, arp(10)),
            // Within twice the queue's 100 ms of the capture's end: no row.
            (t0 + 1_500_000, tcp(10, 11, 100, 4000, 15, 0x18)),
        ];
        // The delay is 20 ms; the queue dropped the second packet. The
        // capture holds a packet and an ARP request the node received too.
        let carried = [
            (t0 + 20_005, tcp(10, 11, 1448, 1000, 10, 0x18)),
            (t0 + 20_050, tcp(11, 10, 0, 1, 99, 0x10)),
            (t0 + 20_400, arp(11)),
            (t0 + 21_300, tcp(10, 11, 104, 3896, 12, 0x18)),
            (t0 + 21_500, tcp(10, 11, 0, 4000, 13, 0x10)),
            (t0 + 21_600, tcp(10, 250, 100, 7, 14, 0x18)),
            (t0 + 21_700, arp(10)),
            (t0 + 1_600_000, tcp(11, 10, 0, 1, 100, 0x10)),
        ];
        let received = [
            (t0 + 20_007, tcp(10, 11, 1448, 1000, 10, 0x18)),
            (t0 + 21_502, tcp(10, 11, 0, 4000, 13, 0x10)),
        ];
        std::fs::write(dir.join("pcap-0-offered.pcap"), pcap(&offered)).unwrap();
        std::fs::write(dir.join("pcap-0.pcap"), pcap(&carried)).unwrap();
        std::fs::write(dir.join("pcap-1.pcap"), pcap(&received)).unwrap();
        let caps = captures(&dir, 2, &[None, None], 0);
        let rows = &caps[0].packets;
        let us = |d: Option<SimDuration>| d.map(|d| d.as_micros() as u64 - t0);
        let got: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    r.at.as_micros() as u64 - t0,
                    us(r.sent),
                    us(r.arrived),
                    r.lost,
                    r.peer,
                    r.ip_bytes,
                    r.kind,
                    r.range,
                    r.backlog_bytes,
                )
            })
            .collect();
        let (q, data) = (Some(PacketLoss::Queue), PacketKind::Data);
        assert_eq!(
            got,
            [
                (
                    20_000,
                    Some(20_005),
                    Some(20_007),
                    None,
                    Some(1),
                    1500,
                    data,
                    Some((0, 1448)),
                    Some(0)
                ),
                (
                    20_000,
                    None,
                    None,
                    q,
                    Some(1),
                    1500,
                    data,
                    Some((1448, 2896)),
                    Some(1514)
                ),
                (
                    20_000,
                    Some(21_300),
                    None,
                    None,
                    Some(1),
                    156,
                    data,
                    Some((2896, 3000)),
                    Some(1514)
                ),
                (
                    20_100,
                    Some(21_500),
                    Some(21_502),
                    None,
                    Some(1),
                    52,
                    PacketKind::Ack,
                    None,
                    Some(170)
                ),
                (
                    20_200,
                    Some(21_600),
                    None,
                    None,
                    None,
                    152,
                    PacketKind::Competing,
                    Some((0, 100)),
                    Some(236)
                ),
                (
                    20_300,
                    Some(21_700),
                    None,
                    None,
                    None,
                    28,
                    PacketKind::Control,
                    None,
                    Some(402)
                ),
            ]
        );
        assert_eq!(rows[5].proto, PacketProto::Other);
        let links: Vec<Option<u32>> = rows.iter().map(|r| r.link).collect();
        assert_eq!(links, [Some(0), Some(0), Some(0), Some(0), None, None]);
        assert!(
            rows.iter()
                .all(|r| r.charged_bytes == Some(r.ip_bytes + 14))
        );
        // Its offers: its data to the swarm, the dropped packet's too.
        let offers: Vec<(u64, u64)> = caps[0]
            .offers
            .as_ref()
            .unwrap()
            .iter()
            .map(|&(t, b, _)| (t.as_micros() as u64 - t0, b))
            .collect();
        assert_eq!(
            offers,
            [
                (20_000, 1500),
                (20_000, 1500),
                (20_000, 156),
                (1_520_000, 152)
            ]
        );
        // Node 1 has no offered capture: no packets, its offers unmeasured.
        assert!(caps[1].packets.is_empty() && caps[1].offers.is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
