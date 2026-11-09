// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The window model's data channels: an SCTP association's sender and
//! receiver over one TCP connection of the sim, which carries its bytes in
//! order, as an ordered channel delivers them, and hands its reader whole
//! messages: a chunk reaches the reader once every chunk through the end of
//! its message has arrived, as a channel's `onmessage` fires (the queues'
//! release, `Scheduler::match_queues`). A channel's writes reach
//! the connection whole while its buffered amount (bytes unsent or
//! unacknowledged) is under its cap (see [`crate::net`]), cut into DATA
//! chunks (turmoil segments) and each marked where its message ends; here
//! the association sends them as its node's stack ([`SctpProfile`]) does,
//! and the receiver acknowledges them in SACKs as its own node's stack
//! does. Chunks that go together share packets as their stack bundles
//! them; a packet is lost whole, and its receiver counts it once. Channels
//! carry the bare bench's WebRTC links, and the node's on hosts with a
//! stack ([`HostSpec::rtc_stack`](crate::HostSpec::rtc_stack)).
//!
//! [`Stack::SctpProto`] is sctp-proto 0.10 as str0m drives it for the node
//! ([`SctpProfile::STR0M`]):
//!
//! - New chunks go out while the payload in flight (sent, not
//!   acknowledged) and the chunk fit the window; with nothing outstanding,
//!   one goes anyway. A chunk starts a packet when its payload would take
//!   the one it would join past the MTU.
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
//! [`Stack::DcSctp`] is dcSCTP, as Chromium and Firefox run it
//! ([`SctpProfile::CHROMIUM`]):
//!
//! - A message written, a SACK and a timeout are each a send opportunity
//!   of [`SctpProfile::max_burst`] packets at most. A packet takes the
//!   chunks marked for retransmission and then new ones that fit what the
//!   window has left past the flight, which counts chunks as sent (their
//!   header and padding) and leaves out those marked lost. (dcSCTP cuts a
//!   message's chunks to fit the room as it sends; here a write is cut as
//!   it is written ([`Chunking`]): its first chunk to the room the packet
//!   of the chunks waiting before it has left, the rest at the most a
//!   packet carries, and a chunk that doesn't fit the window waits.)
//! - The receiver acknowledges every second packet, a lone one after
//!   [`SctpProfile::delayed_sack`] or half its own timeout, whichever is
//!   less, and at once its first DATA, a packet past a gap, a duplicate
//!   and a packet with the I bit, which a sender sets while its window is
//!   under ten packets. A SACK it holds rides its next DATA. (A window
//!   halved by a loss stays under ten packets while its association sends
//!   less than it allows, as a thin relay's do: on the testbed 85-87 % of
//!   their SACKs answer one packet, within a millisecond of it.)
//! - The window grows when the cumulative point advances and the flight
//!   before the SACK came within a packet of the window: by at most a
//!   packet in slow start, by a packet once a window's bytes are
//!   acknowledged past it.
//! - Each SACK marks a miss for the chunks between its gap blocks, up to
//!   the highest one it newly acknowledges (in fast recovery when the
//!   cumulative point advances, up to its last gap block); a chunk resent
//!   takes misses only from chunks sent after it. A third miss marks a
//!   chunk lost: it leaves the flight, starts fast recovery if none runs
//!   (to the highest chunk outstanding), and goes in a packet of its own
//!   past the window at the next send opportunity, the rest it doesn't
//!   fit marked for retransmission.
//! - The timeout is RFC 6298's with the variation at least
//!   [`SctpProfile::min_rttvar`], from SACKs without gaps (on the testbed's
//!   thin relays it sits at its 400 ms floor); when the timer fires every
//!   outstanding chunk is marked for retransmission and the window
//!   restarts from a packet. The timer backs off to
//!   [`SctpProfile::max_backoff`].
//!
//! A SACK of its own takes its turn on the receiver's uplink and is lost at
//! a full queue as any packet is. A chunk resent while its last copy is
//! still on its way, or through, is a copy: it takes its turn on both
//! access links and counts toward its packet at the receiver, which has
//! it.
//!
//! On the testbed's 105 % link the str0m model's window falls every
//! 3.4-4.2 s from about 37 KB, sometimes twice in a recovery, as
//! sctp-proto's does (3.4-4.3 s, 37 KB), and it loses 76 packets to the
//! testbed's 59-77.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use strims_base::time::{Duration, Time};
use strims_wan::Carrier;
use strims_wan::wire::tx_time;

use super::{Event, Scheduler, What, overflows};
use crate::net::ConnKey;

/// The SCTP stack an association's rules follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stack {
    SctpProto,
    DcSctp,
}

/// An SCTP stack's sender and receiver as the window model runs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SctpProfile {
    pub stack: Stack,
    /// The most bytes of an SCTP packet (its common header and chunks),
    /// and of a DATA chunk's payload.
    pub mtu: usize,
    pub max_payload: usize,
    /// A packet's bytes on the wire besides SCTP's: IP, UDP and the DTLS
    /// record.
    pub packet_overhead: usize,
    /// The receive window it advertises, its peer's slow start threshold
    /// at first.
    pub rwnd: usize,
    pub initial_window: usize,
    /// The retransmission timeout before a round trip is sampled, its
    /// bounds, and the least round-trip variation it takes.
    pub rto_initial: Duration,
    pub rto_min: Duration,
    pub rto_max: Duration,
    pub min_rttvar: Duration,
    /// The most a timer backs off to after it fires.
    pub max_backoff: Duration,
    /// How long a receiver holds the SACK of a lone packet at most.
    pub delayed_sack: Duration,
    /// The packets a send opportunity sends at most, if it limits them.
    pub max_burst: Option<usize>,
    /// Whether the channel's buffered amount counts the bytes sent and not
    /// yet acknowledged: sctp-proto's does, until the SACK; Chromium's and
    /// Firefox's fall as their SCTP first sends the bytes, so a browser's
    /// pacers see none of the flight (on the testbed their hosts read a
    /// least round trip of 0 ms, from writes and those readings).
    pub buffered_flight: bool,
}

/// A DATA chunk's header, and a SACK chunk's with no gaps.
const CHUNK_HEADER: usize = 16;
const COMMON_HEADER: usize = 12;
/// A datagram's link framing: Ethernet's 14 bytes, which the testbed's tbf
/// counts in each packet, and an access link's framing about matches. Most
/// of a data channel's packets are small, so it costs a thin relay about
/// 4 % of its uplink.
const FRAMING: usize = 14;
/// IP and UDP's 28 bytes, the framing, and 37 of an AES-GCM DTLS 1.2
/// record (str0m's).
const DTLS12_OVERHEAD: usize = FRAMING + 28 + 37;
/// IP and UDP's 28 bytes, the framing, and 22 of an AES-GCM DTLS 1.3 record
/// (the browsers': a 5-byte header, the content type and the tag; the
/// testbed's pcaps show their SACKs in 92-byte frames).
const DTLS13_OVERHEAD: usize = FRAMING + 28 + 22;

impl SctpProfile {
    /// sctp-proto 0.10 as str0m 0.24 runs it: 1150-byte datagrams, less
    /// str0m's allowances for DTLS (48) and SCTP (40), carry 1062 bytes of
    /// a chunk. RFC 4960's initial window (min(4 MTU, max(2 MTU, 4380))),
    /// a 1 MiB receive window, the timeout 3 s at first and 1-60 s, and the
    /// delayed SACK 200 ms.
    pub const STR0M: SctpProfile = SctpProfile {
        stack: Stack::SctpProto,
        mtu: 1062 + CHUNK_HEADER + COMMON_HEADER,
        max_payload: 1062,
        packet_overhead: DTLS12_OVERHEAD,
        rwnd: 1 << 20,
        initial_window: 4360,
        rto_initial: Duration::from_millis(3000),
        rto_min: Duration::from_millis(1000),
        rto_max: Duration::from_millis(60_000),
        min_rttvar: Duration::ZERO,
        max_backoff: Duration::from_millis(60_000),
        delayed_sack: Duration::from_millis(200),
        max_burst: None,
        buffered_flight: true,
    };

    /// dcSCTP's options as Chromium runs it: 1191-byte packets (a chunk of
    /// 1160 bytes at most, its room rounded down to 4), a first
    /// window of ten, a 5 MiB receive window, the timeout 500 ms at first
    /// and 400 ms-60 s with a variation of 27.5 ms at least (its 220 ms
    /// option over 8) and its timers backing off to 3 s at most, the
    /// delayed SACK 200 ms at most, and four packets a send opportunity.
    pub const CHROMIUM: SctpProfile = SctpProfile {
        stack: Stack::DcSctp,
        mtu: 1191,
        max_payload: (1191 - COMMON_HEADER) / 4 * 4 - CHUNK_HEADER,
        packet_overhead: DTLS13_OVERHEAD,
        rwnd: 5 << 20,
        initial_window: 10 * 1191,
        rto_initial: Duration::from_millis(500),
        rto_min: Duration::from_millis(400),
        rto_max: Duration::from_millis(60_000),
        min_rttvar: Duration::from_micros(27_500),
        max_backoff: Duration::from_secs(3),
        delayed_sack: Duration::from_millis(200),
        max_burst: Some(4),
        buffered_flight: false,
    };

    /// dcSCTP as Firefox 158 runs it: Chromium's options but a 16 MiB
    /// receive window and a delayed SACK of 50 ms. (Between two Firefoxes
    /// its chunks are I-DATA, 4 bytes longer.)
    pub const FIREFOX: SctpProfile = SctpProfile {
        rwnd: 16 << 20,
        delayed_sack: Duration::from_millis(50),
        ..SctpProfile::CHROMIUM
    };

    /// A packet of one chunk of `len` bytes, on the wire.
    pub(crate) fn data_wire(&self, len: usize) -> usize {
        self.packet_overhead + COMMON_HEADER + chunk_size(len)
    }

    /// A SACK chunk with `gaps` gap blocks.
    fn sack_chunk(gaps: usize) -> usize {
        CHUNK_HEADER + 4 * gaps
    }
}

/// A DATA chunk of `len` payload bytes as sent: its header and padding.
fn chunk_size(len: usize) -> usize {
    CHUNK_HEADER + len.next_multiple_of(4)
}

/// How a channel cuts its writes into DATA chunks: at the most a chunk
/// carries, and on dcSCTP, which cuts each chunk to the room its packet
/// has left as it sends, a write's first chunk to the room the packet of
/// the chunks before it leaves while they wait to go, so a message's last
/// chunk and the next one's first share a packet as they do there.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Chunking {
    max: usize,
    /// dcSCTP's room for chunks in a packet, and the bytes of it the
    /// chunks cut since the channel last had none waiting take.
    fit: Option<usize>,
    open: usize,
}

impl Chunking {
    pub(crate) fn new(p: &SctpProfile) -> Chunking {
        Chunking {
            max: p.max_payload.max(1),
            fit: (p.stack == Stack::DcSctp).then(|| p.mtu - COMMON_HEADER),
            open: 0,
        }
    }

    /// The payload of the next chunk of a write with `left` bytes still to
    /// cut; `waiting` says chunks cut before it wait to go.
    pub(crate) fn next(&mut self, left: usize, waiting: bool) -> usize {
        if !waiting {
            self.open = 0;
        }
        let room = match self.fit {
            Some(fit) if self.open > 0 => fit.saturating_sub(self.open + CHUNK_HEADER) / 4 * 4,
            _ => 0,
        };
        let n = if room > 0 { room } else { self.max };
        left.min(n).max(1)
    }

    /// Records a chunk of `len` payload bytes cut.
    pub(crate) fn cut(&mut self, len: usize) {
        let Some(fit) = self.fit else {
            return;
        };
        let size = chunk_size(len);
        self.open = if self.open > 0 && self.open + size <= fit {
            self.open + size
        } else {
            size
        };
    }
}

/// A chunk sent and not yet below the cumulative point.
#[derive(Debug)]
struct Chunk {
    /// Its record and payload bytes.
    id: u64,
    len: usize,
    /// When it first went out, how often it went, and the sequence number
    /// past the furthest sent when it last went.
    sent: Time,
    nsent: u32,
    boundary: u64,
    misses: u32,
    /// Marked to go again within the window, or (dcSCTP) as a fast
    /// retransmission past it.
    retransmit: bool,
    fast: bool,
    /// Acknowledged past a gap.
    acked: bool,
    /// Its last copy was lost, so a resend sends the record.
    lost: bool,
}

impl Chunk {
    fn new(id: u64, len: usize, sent: Time, boundary: u64) -> Chunk {
        Chunk {
            id,
            len,
            sent,
            nsent: 1,
            boundary,
            misses: 0,
            retransmit: false,
            fast: false,
            acked: false,
            lost: false,
        }
    }

    /// The bytes it puts in the flight: sctp-proto counts payload until
    /// it is acknowledged, dcSCTP chunks as sent, not those marked lost.
    fn flight(&self, stack: Stack) -> usize {
        match stack {
            _ if self.acked => 0,
            Stack::SctpProto => self.len,
            Stack::DcSctp if self.retransmit || self.fast => 0,
            Stack::DcSctp => chunk_size(self.len),
        }
    }
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
    /// New chunks waiting for the window, in write order, and whether each
    /// ends its message. Bound: the channel's cap.
    pending: VecDeque<(u64, bool)>,
    /// Chunks past the cumulative point, by sequence number, and the bytes
    /// they put in the flight. Bound: the window.
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
    /// The smoothed round trip (none yet at zero) and its variation, and
    /// the timeout they give: sctp-proto's in whole milliseconds (its
    /// variation fractional), dcSCTP's in microseconds.
    srtt_ms: u64,
    rttvar_ms: f64,
    srtt: Duration,
    rttvar: Duration,
    rto: Duration,
    /// When the retransmission timer fires, and how often it fired since
    /// it last stopped.
    t3: Option<Time>,
    timeouts: u32,
    /// SACKs on their way, by event key. Bound: the SACKs a round trip
    /// holds.
    sacks: BTreeMap<u64, Sack>,
    /// The chunks on their way that a later chunk of their packet follows,
    /// and the last chunks of packets with the I bit: the receiver takes
    /// the packet at its last. Bound: the window.
    bundled: BTreeSet<u64>,
    immediate: BTreeSet<u64>,
    /// The chunks that end a message and haven't reached the reader. Bound:
    /// the channel's cap.
    ends: BTreeSet<u64>,
}

impl Assoc {
    pub(super) fn new(profile: SctpProfile) -> Assoc {
        Assoc {
            profile,
            cwnd: profile.initial_window,
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
            srtt: Duration::ZERO,
            rttvar: Duration::ZERO,
            rto: profile.rto_initial,
            t3: None,
            timeouts: 0,
            sacks: BTreeMap::new(),
            ends: BTreeSet::new(),
            bundled: BTreeSet::new(),
            immediate: BTreeSet::new(),
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

    pub(super) fn pending(&self) -> impl Iterator<Item = u64> + '_ {
        self.pending.iter().map(|&(id, _)| id)
    }

    fn stack(&self) -> Stack {
        self.profile.stack
    }

    /// Chunk `seq`'s state changes by `f`, the flight with it.
    fn update(&mut self, seq: u64, f: impl FnOnce(&mut Chunk)) {
        let stack = self.stack();
        if let Some(c) = self.out.get_mut(&seq) {
            let before = c.flight(stack);
            f(c);
            self.in_flight = self.in_flight.saturating_sub(before) + c.flight(stack);
        }
    }

    /// A round-trip sample: RFC 6298's estimate (alpha 1/8, beta 1/4),
    /// sctp-proto's from whole milliseconds, dcSCTP's with its variation
    /// kept at its least at least.
    fn sampled(&mut self, rtt: Duration) {
        match self.stack() {
            Stack::SctpProto => {
                let ms = u64::try_from(rtt.as_millis().max(0)).unwrap_or(0);
                if self.srtt_ms == 0 {
                    self.srtt_ms = ms;
                    self.rttvar_ms = ms as f64 / 2.0;
                } else {
                    self.rttvar_ms =
                        (6.0 * self.rttvar_ms + 2.0 * self.srtt_ms.abs_diff(ms) as f64) / 8.0;
                    self.srtt_ms = (7 * self.srtt_ms + ms) / 8;
                }
                let rto = self.srtt_ms + (4.0 * self.rttvar_ms) as u64;
                self.rto = Duration::from_millis(i64::try_from(rto).unwrap_or(i64::MAX))
                    .clamp(self.profile.rto_min, self.profile.rto_max);
            }
            Stack::DcSctp => {
                let rtt = rtt.max(Duration::ZERO);
                if self.srtt == Duration::ZERO {
                    self.srtt = rtt;
                    self.rttvar = rtt / 2;
                } else {
                    let dev = if rtt > self.srtt {
                        rtt - self.srtt
                    } else {
                        self.srtt - rtt
                    };
                    self.rttvar = (self.rttvar * 3 + dev) / 4;
                    self.srtt = (self.srtt * 7 + rtt) / 8;
                }
                self.rttvar = self.rttvar.max(self.profile.min_rttvar);
                self.rto =
                    (self.srtt + self.rttvar * 4).clamp(self.profile.rto_min, self.profile.rto_max);
            }
        }
    }

    /// The timer's period: the timeout, doubled for each time it fired.
    fn t3_period(&self) -> Duration {
        let factor = 1i64 << self.timeouts.min(30);
        Duration::from_micros(self.rto.as_micros().saturating_mul(factor))
            .min(self.profile.max_backoff)
    }

    /// The record and length of the waiting chunk `id`, if it is data.
    /// Whether chunk `seq` ends its message.
    pub(super) fn ends_message(&self, seq: u64) -> bool {
        self.ends.contains(&seq)
    }

    /// The reader has every message up to the one chunk `seq` ends.
    pub(super) fn read_through(&mut self, seq: u64) {
        self.ends = self.ends.split_off(&(seq + 1));
    }

    fn chunk_of(records: &super::Records, id: u64) -> Option<(u64, usize)> {
        records.get(&id).and_then(|r| match r.what {
            What::Data { seq, len } => Some((seq, len)),
            _ => None,
        })
    }

    /// Chunk `seq` of `len` bytes (record `id`) goes out first at `at`.
    fn launch(&mut self, at: Time, id: u64, seq: u64, len: usize) {
        if self.cum.is_none() {
            self.cum = Some(seq.saturating_sub(1));
            self.measure_from = seq;
        }
        self.next_seq = self.next_seq.max(seq + 1);
        let c = Chunk::new(id, len, at, self.next_seq);
        self.in_flight += c.flight(self.stack());
        self.out.insert(seq, c);
    }
}

/// One association's receiver: its cumulative point, the chunks past it
/// it holds, a lone packet's SACK it holds back until `due`, and whether
/// it has taken DATA yet.
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

    /// Data record `id` of channel `conn` was written by `at`, the last of
    /// its message if `end`.
    pub(super) fn sctp_write(&mut self, at: Time, conn: ConnKey, id: u64, end: bool) {
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        if end && let Some((seq, _)) = Assoc::chunk_of(&self.records, id) {
            a.ends.insert(seq);
        }
        a.pending.push_back((id, end));
        // dcSCTP sends as a message is written; sctp-proto as str0m polls.
        if end || a.stack() == Stack::SctpProto {
            self.sctp_pump(at, conn);
        }
    }

    /// A send opportunity of `conn`'s association at `at`.
    pub(super) fn sctp_pump(&mut self, at: Time, conn: ConnKey) {
        match self.assocs.get(&conn).map(Assoc::stack) {
            Some(Stack::SctpProto) => self.sctp_proto_pump(at, conn),
            Some(Stack::DcSctp) => self.rfc_pump(at, conn),
            None => {}
        }
    }

    /// One drain of sctp-proto: the chunks a timeout marked (a window's
    /// payload a poll), the new chunks the window takes, then a fast
    /// retransmission's packet, each group bundled into packets.
    fn sctp_proto_pump(&mut self, at: Time, conn: ConnKey) {
        let mut packets: Vec<Vec<Send>> = Vec::new();
        {
            let Some(a) = self.assocs.get_mut(&conn) else {
                return;
            };
            let mtu = a.profile.mtu;
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
            while let Some(&(id, _)) = a.pending.front() {
                let Some((seq, len)) = Assoc::chunk_of(&self.records, id) else {
                    a.pending.pop_front();
                    continue;
                };
                if a.in_flight + len > a.cwnd && !a.out.is_empty() {
                    break;
                }
                a.pending.pop_front();
                a.launch(at, id, seq, len);
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

    /// A send opportunity of dcSCTP: a fast retransmission's packet past
    /// the window, then up to `max_burst` packets each of the chunks marked
    /// for retransmission and then new ones, of what the window has left.
    fn rfc_pump(&mut self, at: Time, conn: ConnKey) {
        let mut packets: Vec<Vec<Send>> = Vec::new();
        {
            let Some(a) = self.assocs.get_mut(&conn) else {
                return;
            };
            let (mtu, stack) = (a.profile.mtu, a.stack());
            let room_in_packet = mtu - COMMON_HEADER;
            if a.out.values().any(|c| c.fast) {
                let first = a.cum.map_or(0, |c| c + 1);
                let (mut bytes, mut packet) = (0, Vec::new());
                let seqs: Vec<u64> = a
                    .out
                    .iter()
                    .filter(|(_, c)| c.fast)
                    .map(|(&s, _)| s)
                    .collect();
                for seq in seqs {
                    let Some(c) = a.out.get(&seq) else {
                        continue;
                    };
                    let size = chunk_size(c.len);
                    if bytes + size > room_in_packet {
                        // Ineligible for another fast retransmission: it goes
                        // within the window.
                        a.update(seq, |c| {
                            c.fast = false;
                            c.retransmit = true;
                        });
                        continue;
                    }
                    bytes += size;
                    let next = a.next_seq;
                    let (id, len) = (c.id, c.len);
                    a.update(seq, |c| {
                        c.fast = false;
                        c.nsent += 1;
                        c.boundary = next;
                        c.misses = 0;
                    });
                    packet.push(Send {
                        id,
                        seq,
                        len,
                        resend: true,
                    });
                }
                if packet.first().is_some_and(|c| c.seq == first) {
                    a.t3 = None;
                }
                if !packet.is_empty() {
                    packets.push(packet);
                }
            }
            for _ in 0..a.profile.max_burst.unwrap_or(usize::MAX) {
                let mut room = a.cwnd.saturating_sub(a.in_flight).min(room_in_packet) / 4 * 4;
                let mut packet = Vec::new();
                let marked: Vec<u64> = a
                    .out
                    .iter()
                    .filter(|(_, c)| c.retransmit && !c.acked)
                    .map(|(&s, _)| s)
                    .collect();
                for seq in marked {
                    let Some(c) = a.out.get(&seq) else {
                        continue;
                    };
                    let size = chunk_size(c.len);
                    if size > room {
                        continue;
                    }
                    room -= size;
                    let next = a.next_seq;
                    let (id, len) = (c.id, c.len);
                    a.update(seq, |c| {
                        c.retransmit = false;
                        c.nsent += 1;
                        c.boundary = next;
                        c.misses = 0;
                    });
                    packet.push(Send {
                        id,
                        seq,
                        len,
                        resend: true,
                    });
                }
                while room > CHUNK_HEADER {
                    let Some(&(id, _)) = a.pending.front() else {
                        break;
                    };
                    let Some((seq, len)) = Assoc::chunk_of(&self.records, id) else {
                        a.pending.pop_front();
                        continue;
                    };
                    if chunk_size(len) > room {
                        break;
                    }
                    a.pending.pop_front();
                    room -= chunk_size(len);
                    a.launch(at, id, seq, len);
                    packet.push(Send {
                        id,
                        seq,
                        len,
                        resend: false,
                    });
                }
                if packet.is_empty() {
                    break;
                }
                packets.push(packet);
            }
            let outstanding = a.out.values().any(|c| c.flight(stack) > 0 || c.retransmit);
            if outstanding && a.t3.is_none_or(|t| t < at) {
                let due = at + a.t3_period();
                a.t3 = Some(due);
                let key = self.next;
                self.next += 1;
                self.events.insert((due, key), Event::SctpTimeout { conn });
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
    /// the first charged the packet's overhead. A dcSCTP packet carries the
    /// SACK its sender holds back for the reverse direction, and the I bit
    /// while the window is under ten packets.
    fn sctp_transmit(&mut self, at: Time, conn: ConnKey, packet: &[Send]) {
        let (Some(cap), Some(_)) = (self.window(), packet.first()) else {
            return;
        };
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let Some((profile, cwnd)) = self.assocs.get(&conn).map(|a| (a.profile, a.cwnd)) else {
            return;
        };
        let back = (conn.1, conn.0);
        let ride = profile.stack != Stack::SctpProto
            && self.rcvs.get(&back).is_some_and(|r| r.due.is_some());
        let sack = if ride { self.sack_of(back) } else { None };
        let sack_bytes = sack
            .as_ref()
            .map_or(0, |s| SctpProfile::sack_chunk(gap_blocks(&s.above)));
        let share = |i: usize, len: usize| {
            usize::from(i == 0) * (profile.packet_overhead + COMMON_HEADER + sack_bytes)
                + chunk_size(len)
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
            let t = self.net.send(at, from, to, wire, 1, Carrier::Datagram);
            let app: usize = packet.iter().filter(|c| !c.resend).map(|c| c.len).sum();
            self.sent(from, app);
            Some(t)
        };
        if let Some(sack) = sack {
            self.sack_held(back, sack, t.filter(|t| !t.dropped).map(|t| t.arrival));
        }
        let Some(t) = t.filter(|t| !t.dropped) else {
            for (c, copy) in packet.iter().zip(&copies) {
                if !copy {
                    self.sctp_lost(conn, c.seq);
                }
            }
            return;
        };
        let last = packet.len() - 1;
        let immediate = profile.stack == Stack::DcSctp && cwnd < 10 * profile.mtu;
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
                        immediate,
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
                if i == last && immediate {
                    a.immediate.insert(c.seq);
                } else {
                    a.immediate.remove(&c.seq);
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
    /// packet, with the I bit if `immediate`) reached its receiver's
    /// downlink at `t`.
    pub(super) fn sctp_copy_arrived(
        &mut self,
        t: Time,
        conn: ConnKey,
        seq: u64,
        wire: usize,
        flags: (bool, bool),
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
        self.sctp_received(done.max(t), conn, seq, flags);
    }

    /// Chunk `seq` of `conn` passed its receiver's downlink at `t`.
    pub(super) fn sctp_arrived(&mut self, t: Time, conn: ConnKey, seq: u64) {
        let flags = self.assocs.get_mut(&conn).map_or((true, false), |a| {
            (!a.bundled.remove(&seq), a.immediate.remove(&seq))
        });
        self.sctp_received(t, conn, seq, flags);
    }

    /// The receiving end's stack for `conn`: its own association's, back
    /// to the sender.
    fn receiver_profile(&self, conn: ConnKey) -> Option<SctpProfile> {
        self.assocs
            .get(&(conn.1, conn.0))
            .or_else(|| self.assocs.get(&conn))
            .map(|a| a.profile)
    }

    /// A packet of chunk `seq` of `conn` was through its receiver's
    /// downlink at `t` (`flags`: the last chunk of its packet, and the
    /// packet's I bit): the receiver takes the chunk, and at its packet's
    /// last, sends a SACK now or holds a lone packet's.
    pub(super) fn sctp_received(
        &mut self,
        t: Time,
        conn: ConnKey,
        seq: u64,
        (last, immediate): (bool, bool),
    ) {
        let Some(p) = self.receiver_profile(conn) else {
            return;
        };
        let own_rto = self.assocs.get(&(conn.1, conn.0)).map(|a| a.rto);
        let r = self.rcvs.entry(conn).or_default();
        let first = r.cum.is_none();
        let cum = *r.cum.get_or_insert(seq.saturating_sub(1));
        let duplicate = seq <= cum || r.above.contains(&seq);
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
        let at_once = !r.above.is_empty()
            || r.due.is_some()
            || (p.stack == Stack::DcSctp && (first || duplicate || immediate));
        if at_once {
            self.sctp_send_sack(t, conn);
            return;
        }
        let delay = match (p.stack, own_rto) {
            (Stack::DcSctp, Some(rto)) => (rto / 2).min(p.delayed_sack),
            _ => p.delayed_sack,
        };
        r.due = Some(t + delay);
        let key = self.next;
        self.next += 1;
        self.events
            .insert((t + delay, key), Event::SctpDelSack { conn });
    }

    /// `conn`'s receiver sends the SACK it holds back, if it still does at
    /// `t`.
    pub(super) fn sctp_delayed_sack(&mut self, t: Time, conn: ConnKey) {
        if self.rcvs.get(&conn).is_some_and(|r| r.due == Some(t)) {
            self.sctp_send_sack(t, conn);
        }
    }

    /// The SACK `conn`'s receiver would send now, no longer held back.
    fn sack_of(&mut self, conn: ConnKey) -> Option<Sack> {
        let r = self.rcvs.get_mut(&conn)?;
        r.due = None;
        let cum = r.cum?;
        Some(Sack {
            cum,
            above: r.above.iter().copied().collect(),
        })
    }

    /// `sack` of `conn` rode a packet that reaches the sender at `at`, if
    /// the packet wasn't lost.
    fn sack_held(&mut self, conn: ConnKey, sack: Sack, at: Option<Time>) {
        let Some(at) = at else {
            return;
        };
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let key = self.next;
        self.next += 1;
        a.sacks.insert(key, sack);
        self.events.insert((at, key), Event::SctpSack { conn });
    }

    /// `conn`'s receiver sends a SACK at `t`: a packet on its uplink that
    /// reaches the sender a one-way trip after it leaves, or is lost to a
    /// full uplink queue.
    fn sctp_send_sack(&mut self, t: Time, conn: ConnKey) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let Some(sack) = self.sack_of(conn) else {
            return;
        };
        let Some(p) = self.receiver_profile(conn) else {
            return;
        };
        let wire =
            p.packet_overhead + COMMON_HEADER + SctpProfile::sack_chunk(gap_blocks(&sack.above));
        if let Some(cap) = self.window()
            && overflows(
                self.net.up_backlog(to, t),
                tx_time(wire, self.net.up_rate(to, t)),
                cap,
            )
        {
            if let Some(d) = self.drops.get_mut(to) {
                *d += 1;
            }
            return;
        }
        let depart = self.net.occupy_up(t, to, wire);
        self.sack_sent(to, wire);
        let at = depart + self.net.one_way(to, from);
        self.sack_held(conn, sack, Some(at));
    }

    /// SACK `key` of `conn` reaches the sender at `t`, which takes it as
    /// its stack does (see the module docs).
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
        let acked = match a.stack() {
            Stack::SctpProto => sctp_proto_sacked(a, t, conn, &sack, cum0),
            Stack::DcSctp => rfc_sacked(a, t, &sack, cum0),
        };
        if !a.out.is_empty() && a.t3.is_none_or(|d| d < t) {
            let due = t + a.t3_period();
            a.t3 = Some(due);
            let k = self.next;
            self.next += 1;
            self.events.insert((due, k), Event::SctpTimeout { conn });
        }
        if acked > 0
            && let Some(w) = self.buffers.as_ref().and_then(|b| b.acked(conn, acked, t))
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
        let mtu = a.profile.mtu;
        a.ssthresh = (a.cwnd / 2).max(4 * mtu);
        a.cwnd = mtu;
        if a.stack() != Stack::SctpProto {
            a.partial_acked = 0;
        }
        let seqs: Vec<u64> = a.out.keys().copied().collect();
        for seq in seqs {
            a.update(seq, |c| {
                if !c.acked {
                    c.retransmit = true;
                    c.fast = false;
                }
            });
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

/// The gap blocks of the chunks `above` a cumulative point.
fn gap_blocks(above: &[u64]) -> usize {
    above
        .iter()
        .zip(above.iter().skip(1))
        .filter(|&(a, b)| *b != a + 1)
        .count()
        + usize::from(!above.is_empty())
}

/// sctp-proto's `handle_sack` on `sack` at `t`, the cumulative point
/// `cum0` before it: returns the payload it acknowledged.
fn sctp_proto_sacked(a: &mut Assoc, t: Time, _conn: ConnKey, sack: &Sack, cum0: u64) -> usize {
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
        let (mtu, waiting) = (a.profile.mtu, !a.pending.is_empty());
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
        let (mtu, mut enter) = (a.profile.mtu, false);
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
    acked
}

/// dcSCTP's `HandleSack` on `sack` at `t`, the cumulative point `cum0`
/// before it: returns the payload it acknowledged.
fn rfc_sacked(a: &mut Assoc, t: Time, sack: &Sack, cum0: u64) -> usize {
    let stack = a.stack();
    let flight_before = a.in_flight;
    if sack.above.is_empty()
        && sack.cum > cum0
        && let Some(c) = a.out.get(&sack.cum)
        && c.nsent == 1
    {
        let sent = c.sent;
        a.sampled(t - sent);
    }
    if a.in_recovery && sack.cum >= a.recovery_exit {
        a.in_recovery = false;
    }
    let (mut acked, mut highest) = (0, cum0);
    while let Some(entry) = a.out.first_entry() {
        if *entry.key() > sack.cum {
            break;
        }
        let (seq, c) = entry.remove_entry();
        a.in_flight = a.in_flight.saturating_sub(c.flight(stack));
        if !c.acked {
            acked += c.len;
            highest = highest.max(seq);
        }
    }
    for &seq in &sack.above {
        let Some(c) = a.out.get(&seq) else {
            continue;
        };
        if c.acked {
            continue;
        }
        acked += c.len;
        highest = highest.max(seq);
        a.update(seq, |c| {
            c.acked = true;
            c.retransmit = false;
            c.fast = false;
        });
    }
    let advanced = sack.cum > cum0;
    if advanced {
        a.cum = Some(sack.cum);
    }
    // Misses between the gap blocks, up to the highest chunk newly acked
    // (in recovery as the cumulative point advances, up to the last gap
    // block).
    let top = match sack.above.last() {
        Some(&end) if a.in_recovery && advanced => end,
        _ => highest,
    };
    let mut loss = false;
    let holes: Vec<u64> = a
        .out
        .range(sack.cum + 1..top.max(sack.cum + 1))
        .filter(|(_, c)| !c.acked && !c.retransmit && !c.fast)
        .map(|(&s, _)| s)
        .collect();
    for seq in holes {
        let Some(c) = a.out.get(&seq) else {
            continue;
        };
        // A resent chunk takes misses only from chunks sent after it.
        if c.nsent > 1 && highest < c.boundary {
            continue;
        }
        let lost = c.misses + 1 >= 3;
        a.update(seq, |c| {
            c.misses += 1;
            if lost {
                c.fast = true;
            }
        });
        loss |= lost;
    }
    if advanced {
        a.t3 = None;
        a.timeouts = 0;
        let mtu = a.profile.mtu;
        // The flight came within a packet of the window; a packet at most a
        // SACK in slow start.
        let full = flight_before + mtu >= a.cwnd;
        if a.cwnd <= a.ssthresh {
            if full && !a.in_recovery {
                a.cwnd += acked.min(mtu);
            }
        } else {
            a.partial_acked += acked;
            if a.partial_acked >= a.cwnd && full {
                a.partial_acked -= a.cwnd;
                a.cwnd += mtu;
            }
        }
    }
    if loss && !a.in_recovery {
        let mtu = a.profile.mtu;
        a.ssthresh = (a.cwnd / 2).max(4 * mtu);
        a.cwnd = a.ssthresh;
        a.partial_acked = 0;
        a.in_recovery = true;
        a.recovery_exit = a.next_seq.saturating_sub(1);
    }
    acked
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// The payloads `chunking` cuts `writes` into, the earlier chunks
    /// still waiting at each write when `waiting` says so.
    fn cuts(mut chunking: Chunking, writes: &[usize], waiting: bool) -> Vec<Vec<usize>> {
        let mut first = true;
        writes
            .iter()
            .map(|&len| {
                let mut pieces = Vec::new();
                let mut left = len;
                while left > 0 {
                    let n = chunking.next(left, waiting && !first);
                    chunking.cut(n);
                    pieces.push(n);
                    left -= n;
                    first = false;
                }
                pieces
            })
            .collect()
    }

    #[test]
    fn dcsctp_fills_the_packet_a_message_ends_in() {
        let c = Chunking::new(&SctpProfile::CHROMIUM);
        let full = SctpProfile::CHROMIUM.max_payload;
        let tail = 8192 - 7 * full;
        // The next message's first chunk takes what the 72-byte tail's
        // packet has left: 1179 less the tail's chunk (88) and its own
        // header, in fours.
        let fill = (1179 - chunk_size(tail) - CHUNK_HEADER) / 4 * 4;
        let w = cuts(c, &[8192, 8192], true);
        assert_eq!(w[0], [vec![full; 7], vec![tail]].concat());
        assert_eq!(w[1].first(), Some(&fill));
        assert!(w[1].iter().skip(1).all(|&n| n <= full));
        assert_eq!(w[1].iter().sum::<usize>(), 8192);
        // A chunk and the next one's fill share a packet as bundled.
        assert!(chunk_size(tail) + chunk_size(fill) <= 1179);
        // Once the chunks before it went, a message starts a packet.
        assert_eq!(cuts(c, &[8192, 8192], false)[1].first(), Some(&full));
    }

    #[test]
    fn sctp_proto_cuts_at_the_most_a_chunk_carries() {
        let p = SctpProfile::STR0M;
        let w = cuts(Chunking::new(&p), &[3000, 3000], true);
        let full = p.max_payload;
        assert_eq!(w[1], vec![full, full, 3000 - 2 * full]);
    }
}
