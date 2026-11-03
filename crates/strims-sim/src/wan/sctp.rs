// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The window model's data channels: an SCTP association's sender and
//! receiver over one TCP connection of the sim, which carries its bytes in
//! order, as an ordered channel delivers them. A channel's writes reach
//! the connection whole while its buffered amount (bytes unsent or
//! unacknowledged) is under its cap (see [`crate::net`]), cut into DATA
//! chunks (turmoil segments); here the association sends them as an
//! [`SctpProfile`]'s stack does, and its receiver acknowledges them in
//! SACKs. [`SctpProfile::STR0M`] is sctp-proto's, as str0m drives it for
//! the node (the bare bench's WebRTC cases, `hack/testbed/bench/rtc-*`,
//! check it):
//!
//! - New chunks go out while the bytes in flight (sent, not acknowledged)
//!   and the chunk fit the window; with nothing outstanding, one goes
//!   anyway. The chunks that go together share packets as sctp-proto
//!   bundles them: a chunk starts a packet when its payload would take the
//!   one it would join past the MTU. A packet is lost whole, and its
//!   receiver counts it once.
//! - The receiver acknowledges every second packet, or a lone one after
//!   [`SctpProfile::delayed_sack`], and every packet at once while it
//!   holds chunks past a gap. A SACK leaves on the receiver's uplink as a
//!   packet of its own, never with the receiver's data.
//! - A SACK acknowledges every chunk up to its cumulative point and those
//!   past it the receiver holds. The window grows when the cumulative
//!   point advances, only while chunks wait for it: by the bytes the SACK
//!   acknowledged (at most the window) in slow start, by a packet once a
//!   window's bytes are acknowledged past it.
//! - Each SACK marks a miss for every unacknowledged chunk below the
//!   highest one it newly acknowledges, or in fast recovery when the
//!   cumulative point advances, for every unacknowledged chunk. A third
//!   miss starts fast recovery (the window and its threshold halve, to
//!   four packets at least, once a recovery) and resends one packet of
//!   chunks with three misses sent once, past the window; so does each
//!   SACK in recovery that advances the cumulative point.
//! - The retransmission timer runs while chunks are outstanding, from the
//!   last time the cumulative point passed the earliest of them, for the
//!   timeout (RFC 4960's, from round trips of chunks sent once, in whole
//!   milliseconds), doubled for each time it fired since. When it fires
//!   the window restarts from a packet and every outstanding chunk goes
//!   again at once (str0m drains the association, which resends a
//!   window's worth each time it is polled).
//!
//! A chunk resent while its last copy is still on its way, or through, is
//! a copy: it takes its turn on both access links and counts toward its
//! packet at the receiver, which has it.
//!
//! On the testbed's 105 % link the model's window falls every 3.4-4.2 s
//! from about 37 KB, sometimes twice in a recovery, as sctp-proto's does
//! (3.4-4.3 s, 37 KB), and it loses 76 packets to the testbed's 59-77.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use strims_base::time::{Duration, Time};
use strims_wan::Carrier;
use strims_wan::wire::tx_time;

use super::{Event, Scheduler, What, overflows};
use crate::net::ConnKey;

/// An SCTP stack's sender and receiver as the window model runs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SctpProfile {
    /// A DATA chunk's most payload bytes.
    pub max_payload: usize,
    /// A packet's bytes on the wire besides its chunks: IP, UDP, the DTLS
    /// record and SCTP's common header.
    pub packet_overhead: usize,
    /// The peer's receive window, the slow start threshold's start.
    pub rwnd: usize,
    /// The retransmission timeout before a round trip is sampled, and its
    /// bounds.
    pub rto_initial: Duration,
    pub rto_min: Duration,
    pub rto_max: Duration,
    /// How long a receiver holds the SACK of a lone packet.
    pub delayed_sack: Duration,
}

/// A DATA chunk's header, and a SACK chunk's with no gaps.
const CHUNK_HEADER: usize = 16;
const COMMON_HEADER: usize = 12;

impl SctpProfile {
    /// sctp-proto 0.10 as str0m 0.24 runs it: 1150-byte datagrams, less
    /// str0m's allowances for DTLS (48) and SCTP (40), carry 1062 bytes of
    /// a chunk; a packet takes 28 bytes of IP and UDP, 37 of an AES-GCM
    /// DTLS 1.2 record and 12 of SCTP. The receive window is 1 MiB, the
    /// timeout 3 s at first and 1-60 s, and the delayed SACK 200 ms.
    pub const STR0M: SctpProfile = SctpProfile {
        max_payload: 1062,
        packet_overhead: 28 + 37 + COMMON_HEADER,
        rwnd: 1 << 20,
        rto_initial: Duration::from_millis(3000),
        rto_min: Duration::from_millis(1000),
        rto_max: Duration::from_millis(60_000),
        delayed_sack: Duration::from_millis(200),
    };

    /// The SCTP packet a full chunk fills, as sctp-proto counts it.
    fn mtu(&self) -> usize {
        self.max_payload + CHUNK_HEADER + COMMON_HEADER
    }

    /// RFC 4960's initial window: min(4 MTU, max(2 MTU, 4380 bytes)).
    fn initial_window(&self) -> usize {
        (4 * self.mtu()).min((2 * self.mtu()).max(4380))
    }

    /// A packet of one chunk of `len` bytes, on the wire.
    pub(crate) fn data_wire(&self, len: usize) -> usize {
        self.packet_overhead + CHUNK_HEADER + len.next_multiple_of(4)
    }

    /// A SACK with `gaps` gap blocks, on the wire.
    fn sack_wire(&self, gaps: usize) -> usize {
        self.packet_overhead + CHUNK_HEADER + 4 * gaps
    }
}

/// A chunk sent and not yet below the cumulative point.
#[derive(Debug)]
struct Chunk {
    /// Its record and payload bytes.
    id: u64,
    len: usize,
    /// When it first went out, and how often it went.
    sent: Time,
    nsent: u32,
    misses: u32,
    /// Marked by a timeout, to go again.
    retransmit: bool,
    /// Acknowledged past a gap.
    acked: bool,
    /// Its last copy was lost, so a resend sends the record.
    lost: bool,
}

/// A chunk to send: its record, sequence number and payload bytes, and
/// whether it went before.
#[derive(Clone, Copy, Debug)]
struct Send {
    id: u64,
    seq: u64,
    len: usize,
    resend: bool,
}

/// `chunks` bundled into packets in order, as sctp-proto bundles them: a
/// chunk starts a new packet when its payload would take the packet past
/// the MTU.
fn bundle(chunks: Vec<Send>, mtu: usize) -> Vec<Vec<Send>> {
    let mut packets: Vec<Vec<Send>> = Vec::new();
    let mut bytes = COMMON_HEADER;
    for c in chunks {
        match packets.last_mut() {
            Some(p) if bytes + c.len <= mtu => p.push(c),
            _ => {
                packets.push(vec![c]);
                bytes = COMMON_HEADER;
            }
        }
        bytes += CHUNK_HEADER + c.len;
    }
    packets
}

/// What a SACK says: the cumulative point, and the chunks past it the
/// receiver holds.
#[derive(Debug)]
pub(super) struct Sack {
    cum: u64,
    above: Vec<u64>,
}

/// One association's sender.
#[derive(Debug)]
pub(super) struct Assoc {
    profile: SctpProfile,
    cwnd: usize,
    ssthresh: usize,
    partial_acked: usize,
    /// New chunks waiting for the window, in write order. Bound: the
    /// channel's cap.
    pending: VecDeque<u64>,
    /// Chunks past the cumulative point, by sequence number, and the bytes
    /// of those not acknowledged. Bound: the window.
    out: BTreeMap<u64, Chunk>,
    in_flight: usize,
    /// The cumulative point, from the first chunk sent; the sequence
    /// number past the furthest sent, and the least that samples a round
    /// trip.
    cum: Option<u64>,
    next_seq: u64,
    measure_from: u64,
    in_recovery: bool,
    recovery_exit: u64,
    fast_resend: bool,
    /// The smoothed round trip (whole ms, none yet at 0) and its variation,
    /// and the timeout they give.
    srtt_ms: u64,
    rttvar_ms: f64,
    rto: Duration,
    /// When the retransmission timer fires, and how often it fired since
    /// it last stopped.
    t3: Option<Time>,
    timeouts: u32,
    /// SACKs on their way, by event key. Bound: the SACKs a round trip
    /// holds.
    sacks: BTreeMap<u64, Sack>,
    /// The chunks on their way that a later chunk of their packet follows:
    /// the receiver counts the packet at its last. Bound: the window.
    bundled: BTreeSet<u64>,
}

impl Assoc {
    pub(super) fn new(profile: SctpProfile) -> Assoc {
        Assoc {
            profile,
            cwnd: profile.initial_window(),
            ssthresh: profile.rwnd,
            partial_acked: 0,
            pending: VecDeque::new(),
            out: BTreeMap::new(),
            in_flight: 0,
            cum: None,
            next_seq: 0,
            measure_from: 0,
            in_recovery: false,
            recovery_exit: 0,
            fast_resend: false,
            srtt_ms: 0,
            rttvar_ms: 0.0,
            rto: profile.rto_initial,
            t3: None,
            timeouts: 0,
            sacks: BTreeMap::new(),
            bundled: BTreeSet::new(),
        }
    }

    pub(super) fn profile(&self) -> SctpProfile {
        self.profile
    }

    pub(super) fn cwnd(&self) -> usize {
        self.cwnd
    }

    pub(super) fn ssthresh(&self) -> usize {
        self.ssthresh
    }

    pub(super) fn in_flight(&self) -> usize {
        self.in_flight
    }

    pub(super) fn pending(&self) -> &VecDeque<u64> {
        &self.pending
    }

    /// RFC 4960's round-trip estimate (alpha 1/8, beta 1/4) from a sample
    /// in whole milliseconds, as sctp-proto's.
    fn sampled(&mut self, rtt: Duration) {
        let ms = u64::try_from(rtt.as_millis().max(0)).unwrap_or(0);
        if self.srtt_ms == 0 {
            self.srtt_ms = ms;
            self.rttvar_ms = ms as f64 / 2.0;
        } else {
            self.rttvar_ms = (6.0 * self.rttvar_ms + 2.0 * self.srtt_ms.abs_diff(ms) as f64) / 8.0;
            self.srtt_ms = (7 * self.srtt_ms + ms) / 8;
        }
        let rto = self.srtt_ms + (4.0 * self.rttvar_ms) as u64;
        self.rto = Duration::from_millis(i64::try_from(rto).unwrap_or(i64::MAX))
            .clamp(self.profile.rto_min, self.profile.rto_max);
    }

    /// The timer's period: the timeout, doubled for each time it fired.
    fn t3_period(&self) -> Duration {
        let factor = 1i64 << self.timeouts.min(30);
        Duration::from_micros(self.rto.as_micros().saturating_mul(factor)).min(self.profile.rto_max)
    }
}

/// One association's receiver: its cumulative point, the chunks past it
/// it holds, and a lone packet's SACK it holds back until `due`.
#[derive(Debug, Default)]
pub(super) struct Rcv {
    cum: Option<u64>,
    above: BTreeSet<u64>,
    due: Option<Time>,
}

impl Scheduler {
    /// `conn` carries a data channel from now on, sent as `profile` sends.
    pub(crate) fn open_channel(&mut self, conn: ConnKey, profile: SctpProfile) {
        self.assocs.insert(conn, Assoc::new(profile));
    }

    /// Data record `id` of channel `conn` was written by `at`.
    pub(super) fn sctp_write(&mut self, at: Time, conn: ConnKey, id: u64) {
        if let Some(a) = self.assocs.get_mut(&conn) {
            a.pending.push_back(id);
        }
        self.sctp_pump(at, conn);
    }

    /// Sends what `conn`'s association has to send at `at`, as one drain
    /// of sctp-proto: the chunks a timeout marked (a window's payload a
    /// poll), the new chunks the window takes, then a fast retransmission's
    /// packet, each group bundled into packets.
    pub(super) fn sctp_pump(&mut self, at: Time, conn: ConnKey) {
        let mut packets: Vec<Vec<Send>> = Vec::new();
        {
            let Some(a) = self.assocs.get_mut(&conn) else {
                return;
            };
            let mtu = a.profile.mtu();
            let (mut poll, mut polled) = (Vec::new(), 0);
            for (&seq, c) in &mut a.out {
                if !c.retransmit || c.acked {
                    continue;
                }
                if polled + c.len > a.cwnd && !poll.is_empty() {
                    packets.extend(bundle(std::mem::take(&mut poll), mtu));
                    polled = 0;
                }
                c.retransmit = false;
                c.nsent += 1;
                polled += c.len;
                poll.push(Send {
                    id: c.id,
                    seq,
                    len: c.len,
                    resend: true,
                });
            }
            packets.extend(bundle(poll, mtu));
            let mut fresh = Vec::new();
            while let Some(&id) = a.pending.front() {
                let Some((seq, len)) = self.records.get(&id).and_then(|r| match r.what {
                    What::Data { seq, len } => Some((seq, len)),
                    _ => None,
                }) else {
                    a.pending.pop_front();
                    continue;
                };
                if a.in_flight + len > a.cwnd && !a.out.is_empty() {
                    break;
                }
                a.pending.pop_front();
                if a.cum.is_none() {
                    a.cum = Some(seq.saturating_sub(1));
                    a.measure_from = seq;
                }
                a.out.insert(
                    seq,
                    Chunk {
                        id,
                        len,
                        sent: at,
                        nsent: 1,
                        misses: 0,
                        retransmit: false,
                        acked: false,
                        lost: false,
                    },
                );
                a.in_flight += len;
                a.next_seq = a.next_seq.max(seq + 1);
                fresh.push(Send {
                    id,
                    seq,
                    len,
                    resend: false,
                });
            }
            if !fresh.is_empty() && a.t3.is_none_or(|t| t < at) {
                let due = at + a.t3_period();
                a.t3 = Some(due);
                let key = self.next;
                self.next += 1;
                self.events.insert((due, key), Event::SctpTimeout { conn });
            }
            packets.extend(bundle(fresh, mtu));
            if std::mem::take(&mut a.fast_resend) {
                let mut bytes = COMMON_HEADER;
                let mut packet = Vec::new();
                for (&seq, c) in &mut a.out {
                    if c.acked || c.nsent > 1 || c.misses < 3 {
                        continue;
                    }
                    if bytes + CHUNK_HEADER + c.len > mtu {
                        break;
                    }
                    bytes += CHUNK_HEADER + c.len;
                    c.nsent += 1;
                    packet.push(Send {
                        id: c.id,
                        seq,
                        len: c.len,
                        resend: true,
                    });
                }
                if !packet.is_empty() {
                    packets.push(packet);
                }
            }
        }
        for packet in packets {
            self.sctp_transmit(at, conn, &packet);
        }
    }

    /// A packet of `conn`'s chunks goes out at `at`: each chunk credited to
    /// its socket as it first goes; lost whole at a full uplink queue or on
    /// its path; and a resent chunk whose last copy wasn't lost goes as a
    /// copy. Its chunks pass the receiver's downlink one after another,
    /// the first charged the packet's overhead.
    fn sctp_transmit(&mut self, at: Time, conn: ConnKey, packet: &[Send]) {
        let (Some(cap), Some(first)) = (self.window(), packet.first()) else {
            return;
        };
        let Some((from, to)) = self.records.get(&first.id).map(|r| (r.from, r.to)) else {
            return;
        };
        let Some(profile) = self.assocs.get(&conn).map(|a| a.profile) else {
            return;
        };
        let share = |i: usize, len: usize| {
            usize::from(i == 0) * profile.packet_overhead + CHUNK_HEADER + len.next_multiple_of(4)
        };
        let wire: usize = packet
            .iter()
            .enumerate()
            .map(|(i, c)| share(i, c.len))
            .sum();
        let copies: Vec<bool> = packet
            .iter()
            .map(|c| {
                c.resend
                    && self
                        .assocs
                        .get(&conn)
                        .and_then(|a| a.out.get(&c.seq))
                        .is_some_and(|x| !x.lost)
            })
            .collect();
        if self.buffers.is_some() {
            for c in packet.iter().filter(|c| !c.resend) {
                self.admits.insert((at, c.id), (conn, c.len));
            }
        }
        let wait = self.net.up_backlog(from, at);
        let t = if overflows(wait, tx_time(wire, self.net.up_rate(from, at)), cap) {
            if let Some(d) = self.drops.get_mut(from) {
                *d += 1;
            }
            None
        } else {
            Some(self.net.send(at, from, to, wire, 1, Carrier::Datagram))
        };
        let Some(t) = t.filter(|t| !t.dropped) else {
            for (c, copy) in packet.iter().zip(&copies) {
                if !copy {
                    self.sctp_lost(conn, c.seq);
                }
            }
            return;
        };
        let last = packet.len() - 1;
        for (i, (c, &copy)) in packet.iter().zip(&copies).enumerate() {
            let w = share(i, c.len);
            if copy {
                let key = self.next;
                self.next += 1;
                self.events.insert(
                    (t.arrival, key),
                    Event::SctpCopy {
                        conn,
                        seq: c.seq,
                        wire: w,
                        last: i == last,
                    },
                );
                continue;
            }
            if let Some(r) = self.records.get_mut(&c.id) {
                r.wire = w;
            }
            if let Some(a) = self.assocs.get_mut(&conn) {
                if let Some(x) = a.out.get_mut(&c.seq) {
                    x.lost = false;
                }
                if i == last {
                    a.bundled.remove(&c.seq);
                } else {
                    a.bundled.insert(c.seq);
                }
            }
            self.events.insert((t.arrival, c.id), Event::Arrive);
        }
    }

    /// Chunk `seq` of `conn` was lost: the sender learns so only from the
    /// SACKs and timeouts that follow.
    pub(super) fn sctp_lost(&mut self, conn: ConnKey, seq: u64) {
        if let Some(c) = self.assocs.get_mut(&conn).and_then(|a| a.out.get_mut(&seq)) {
            c.lost = true;
        }
    }

    /// A copy of chunk `seq` of `conn` (`wire` bytes, the `last` of its
    /// packet) reached its receiver's downlink at `t`.
    pub(super) fn sctp_copy_arrived(
        &mut self,
        t: Time,
        conn: ConnKey,
        seq: u64,
        wire: usize,
        last: bool,
    ) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        if let Some(cap) = self.window() {
            let rate = self.net.site(to).map_or(u64::MAX, |s| s.profile.down);
            if overflows(self.net.down_free(to) - t, tx_time(wire, rate), cap) {
                if let Some(d) = self.drops.get_mut(from) {
                    *d += 1;
                }
                return;
            }
        }
        let done = self.net.receive(t, to, wire);
        self.sctp_received(done.max(t), conn, seq, last);
    }

    /// Chunk `seq` of `conn` passed its receiver's downlink at `t`.
    pub(super) fn sctp_arrived(&mut self, t: Time, conn: ConnKey, seq: u64) {
        let last = self
            .assocs
            .get_mut(&conn)
            .is_none_or(|a| !a.bundled.remove(&seq));
        self.sctp_received(t, conn, seq, last);
    }

    /// A packet of chunk `seq` of `conn` was through its receiver's
    /// downlink at `t`: the receiver takes it and sends a SACK now, or
    /// holds a lone packet's.
    pub(super) fn sctp_received(&mut self, t: Time, conn: ConnKey, seq: u64, last: bool) {
        let Some(delay) = self.assocs.get(&conn).map(|a| a.profile.delayed_sack) else {
            return;
        };
        let r = self.rcvs.entry(conn).or_default();
        let cum = *r.cum.get_or_insert(seq.saturating_sub(1));
        if seq == cum + 1 {
            let mut c = seq;
            while r.above.remove(&(c + 1)) {
                c += 1;
            }
            r.cum = Some(c);
        } else if seq > cum + 1 {
            r.above.insert(seq);
        }
        if !last {
            return;
        }
        if !r.above.is_empty() || r.due.is_some() {
            self.sctp_send_sack(t, conn);
        } else {
            r.due = Some(t + delay);
            let key = self.next;
            self.next += 1;
            self.events
                .insert((t + delay, key), Event::SctpDelSack { conn });
        }
    }

    /// `conn`'s receiver sends the SACK it holds back, if it still does at
    /// `t`.
    pub(super) fn sctp_delayed_sack(&mut self, t: Time, conn: ConnKey) {
        if self.rcvs.get(&conn).is_some_and(|r| r.due == Some(t)) {
            self.sctp_send_sack(t, conn);
        }
    }

    /// `conn`'s receiver sends a SACK at `t`: a packet on its uplink that
    /// reaches the sender a one-way trip after it leaves.
    fn sctp_send_sack(&mut self, t: Time, conn: ConnKey) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let Some(r) = self.rcvs.get_mut(&conn) else {
            return;
        };
        r.due = None;
        let Some(cum) = r.cum else {
            return;
        };
        let above: Vec<u64> = r.above.iter().copied().collect();
        let gaps = above
            .iter()
            .zip(above.iter().skip(1))
            .filter(|&(a, b)| *b != a + 1)
            .count()
            + usize::from(!above.is_empty());
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let wire = a.profile.sack_wire(gaps);
        let key = self.next;
        self.next += 1;
        a.sacks.insert(key, Sack { cum, above });
        let depart = self.net.occupy_up(t, to, wire);
        if let Some(n) = self.acks.get_mut(to) {
            *n += 1;
        }
        let at = depart + self.net.one_way(to, from);
        self.events.insert((at, key), Event::SctpSack { conn });
    }

    /// SACK `key` of `conn` reaches the sender at `t`, which takes it as
    /// sctp-proto's `handle_sack` does (see the module docs).
    pub(super) fn sctp_sacked(&mut self, t: Time, conn: ConnKey, key: u64) {
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let Some(sack) = a.sacks.remove(&key) else {
            return;
        };
        let Some(cum0) = a.cum else {
            return;
        };
        if sack.cum < cum0 {
            return;
        }
        let mut acked = 0;
        let mut htna = sack.cum;
        let first = cum0 + 1;
        while let Some(entry) = a.out.first_entry() {
            let seq = *entry.key();
            if seq > sack.cum {
                break;
            }
            let c = entry.remove();
            if !c.acked {
                if seq == first {
                    a.t3 = None;
                    a.timeouts = 0;
                }
                acked += c.len;
                a.in_flight = a.in_flight.saturating_sub(c.len);
                if c.nsent == 1 && seq >= a.measure_from {
                    a.measure_from = a.next_seq;
                    a.sampled(t - c.sent);
                }
            }
            if a.in_recovery && seq == a.recovery_exit {
                a.in_recovery = false;
            }
        }
        for &seq in &sack.above {
            let Some(c) = a.out.get_mut(&seq) else {
                continue;
            };
            if c.acked {
                continue;
            }
            c.acked = true;
            c.retransmit = false;
            acked += c.len;
            a.in_flight = a.in_flight.saturating_sub(c.len);
            let (nsent, sent) = (c.nsent, c.sent);
            if nsent == 1 {
                a.measure_from = a.next_seq;
                a.sampled(t - sent);
            }
            htna = htna.max(seq);
        }
        let advanced = sack.cum > cum0;
        if advanced {
            a.cum = Some(sack.cum);
            if a.out.is_empty() {
                a.t3 = None;
                a.timeouts = 0;
            }
            let (mtu, waiting) = (a.profile.mtu(), !a.pending.is_empty());
            if a.cwnd <= a.ssthresh {
                if !a.in_recovery && waiting {
                    a.cwnd += acked.min(a.cwnd);
                }
            } else {
                a.partial_acked += acked;
                if a.partial_acked >= a.cwnd && waiting {
                    a.partial_acked -= a.cwnd;
                    a.cwnd += mtu;
                }
            }
        }
        // Miss indications (RFC 4960 7.2.4, as sctp-proto counts them).
        if !a.in_recovery || advanced {
            let top = if a.in_recovery { u64::MAX } else { htna };
            let (mtu, mut enter) = (a.profile.mtu(), false);
            for (&seq, c) in a.out.range_mut(sack.cum + 1..) {
                if seq >= top {
                    break;
                }
                if !c.acked && c.misses < 3 {
                    c.misses += 1;
                    if c.misses == 3 && !a.in_recovery && !enter {
                        enter = true;
                    }
                }
            }
            if enter {
                a.in_recovery = true;
                a.recovery_exit = htna;
                a.ssthresh = (a.cwnd / 2).max(4 * mtu);
                a.cwnd = a.ssthresh;
                a.partial_acked = 0;
                a.fast_resend = true;
            }
        }
        if a.in_recovery && advanced {
            a.fast_resend = true;
        }
        if !a.out.is_empty() && a.t3.is_none_or(|d| d < t) {
            let due = t + a.t3_period();
            a.t3 = Some(due);
            let k = self.next;
            self.next += 1;
            self.events.insert((due, k), Event::SctpTimeout { conn });
        }
        if acked > 0
            && let Some(w) = self.buffers.as_ref().and_then(|b| b.acked(conn, acked))
        {
            w.wake();
        }
        self.sctp_pump(t, conn);
    }

    /// `conn`'s retransmission timer is due at `t`, unless it moved: the
    /// window restarts from a packet, every outstanding chunk is marked to
    /// go again, and the timer backs off.
    pub(super) fn sctp_timeout(&mut self, t: Time, conn: ConnKey) {
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        if a.t3 != Some(t) {
            return;
        }
        let mtu = a.profile.mtu();
        a.ssthresh = (a.cwnd / 2).max(4 * mtu);
        a.cwnd = mtu;
        for c in a.out.values_mut() {
            if !c.acked {
                c.retransmit = true;
            }
        }
        a.timeouts = a.timeouts.saturating_add(1);
        let due = t + a.t3_period();
        a.t3 = Some(due);
        let key = self.next;
        self.next += 1;
        self.events.insert((due, key), Event::SctpTimeout { conn });
        self.sctp_pump(t, conn);
    }
}
