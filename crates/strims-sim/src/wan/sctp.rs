// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The window model's data channels: an SCTP association's sender and
//! receiver over one TCP connection of the sim, which carries its bytes in
//! order, as an ordered channel delivers them, and hands its reader whole
//! messages: a chunk reaches the reader once every chunk through the end of
//! its message has arrived, as a channel's `onmessage` fires (the queues'
//! release, `Scheduler::match_queues`). A channel's writes reach
//! the connection whole while its buffered amount (bytes unsent or
//! unacknowledged) is under its cap (see [`crate::net`]), as records
//! (turmoil segments), each marked where its message ends ([`Chunking`]);
//! here the association cuts them into DATA chunks and sends them as its
//! node's stack ([`SctpProfile`]) does, and the receiver acknowledges them
//! in SACKs as its own node's stack does. Chunks that go together share
//! packets as their stack bundles them; a packet is lost whole, and its
//! receiver counts it once. Channels
//! carry the bare bench's WebRTC links, and the node's on hosts with a
//! stack ([`HostSpec::rtc_stack`](crate::HostSpec::rtc_stack)).
//!
//! [`Stack::SctpProto`] is sctp-proto 0.10 as str0m drives it for the node
//! ([`SctpProfile::STR0M`]):
//!
//! - A write is cut into chunks as it is written, at the most a chunk
//!   carries, each a record and its sequence number its TSN.
//! - New chunks go out while the payload in flight (sent, not
//!   acknowledged) and the chunk fit the window; with nothing outstanding,
//!   one goes anyway. A chunk starts a packet when its payload would take
//!   the one it would join past the MTU.
//! - The receiver acknowledges every second packet, or a lone one after
//!   [`SctpProfile::delayed_sack`], and at once a packet with the I bit,
//!   which a dcSCTP sender sets (association/mod.rs, `handle_data` and
//!   `handle_peer_last_tsn_and_acknowledgement`, :2500, :2905), and every
//!   packet while it holds chunks past a gap. A SACK leaves on the
//!   receiver's uplink as a packet of its own, never with the receiver's
//!   data.
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
//!   header and padding) and leaves out those marked lost. A message is a
//!   record, cut into chunks as they go: each new chunk is the message's
//!   next bytes, as many as the room the packet and the window leave less a
//!   chunk's header, rounded down to 4 (net/dcsctp/tx/retransmission_queue.cc,
//!   `GetChunksToSend`, :479-507; rr_send_queue.cc,
//!   `OutgoingStream::Produce`, :245-249), given the next TSN as it is cut
//!   (`OutstandingData::Insert`, :509), and a message's last chunk and the
//!   next one's first share a packet. A resent chunk keeps its TSN and
//!   bytes, and goes only where it fits whole
//!   (outstanding_data.cc, `ExtractChunksThatCanFit`, :413-415). A
//!   message reaches its reader once each of its chunks is through the
//!   receiver's downlink, in order (dcSCTP's reassembly).
//! - The receiver acknowledges every second packet, a lone one after
//!   [`SctpProfile::delayed_sack`] or half its own timeout, whichever is
//!   less, and at once its first DATA, a packet past a gap, a duplicate
//!   and a packet with the I bit, which a sender sets while its window is
//!   under ten packets. A SACK it holds goes at its host's next send
//!   opportunity on the reverse direction, first in its first packet, whose
//!   chunks take the room it leaves within the MTU, or alone if none fits;
//!   never in a fast retransmission's packet. (A window halved by a loss
//!   stays under ten packets while its association sends less than it
//!   allows, as a thin relay's do: on the testbed 85-87 % of
//!   their SACKs answer one packet, within a millisecond of it.)
//! - The window grows when the cumulative point advances and the flight
//!   before the SACK came within a packet of the window: by the bytes it
//!   acknowledged, at most a packet, in slow start, by a packet once a
//!   window's bytes are acknowledged past it. Both count a chunk as sent,
//!   its header and padding with its payload (outstanding_data.cc,
//!   `AckChunk`, :128-129).
//! - Each SACK marks a miss for the chunks between its gap blocks, up to
//!   the highest one it newly acknowledges (in fast recovery when the
//!   cumulative point advances, up to its last gap block), a resent chunk
//!   too, from any SACK (WebRTC's branch-heads/8078, the dcSCTP Chromium
//!   156 and Firefox 158 run: net/dcsctp/tx/outstanding_data.cc's
//!   `NackBetweenAckBlocks` nacks every chunk between the blocks, and
//!   `Item::MarkAsRetransmitted` only clears its count). So a SACK its
//!   receiver sent before a resend reached it reads the chunk missing
//!   again: behind a full queue a lost chunk goes again every third SACK
//!   until its first resend arrives (on the testbed's thin star the
//!   seed's resends are some five a queue drop, most three SACKs apart).
//!   A third miss marks a chunk lost and it leaves the flight. Outside
//!   fast recovery it starts one (to the highest chunk outstanding) and
//!   goes in a packet of its own past the window at the next send
//!   opportunity, the rest it doesn't fit marked for retransmission; in
//!   fast recovery it is marked for retransmission within the window
//!   (`NackItem`'s `do_fast_retransmit` is `!is_in_fast_recovery`).
//! - The timeout is RFC 6298's with the variation at least
//!   [`SctpProfile::min_rttvar`], from SACKs without gaps (on the testbed's
//!   thin relays it sits at its 400 ms floor); when the timer fires every
//!   outstanding chunk is marked for retransmission and the window
//!   restarts from a packet. The timer backs off to
//!   [`SctpProfile::max_backoff`].
//!
//! A browser host's associations whose stack has a
//! [`SctpProfile::packet_cost`] send through its one network thread, as
//! Chromium 156's do (Chrome for Testing 156.0.8078.47, its WebRTC
//! branch-heads/8078). Its renderer runs every peer connection's network
//! work on one thread (third_party/blink's
//! peer_connection_dependency_factory.cc, `PeerConnectionStaticDeps`,
//! :420-424, the `WebRTC_W_and_N` thread, the connections' worker thread
//! too, :947-949). A data channel's `send` posts its message to that
//! thread as a task of its own (blink's rtc_data_channel.cc:851;
//! WebRTC's pc/sctp_data_channel.cc, `SendAsync`, :563-583), which hands
//! it to dcSCTP (media/sctp/dcsctp_transport.cc:386;
//! net/dcsctp/socket/dcsctp_socket.cc, `Send`, :544-557). Its send
//! opportunity (transmission_control_block.cc, `SendBufferedPackets`,
//! :218-275) seals each packet as a DTLS record of its own
//! (p2p/dtls/dtls_transport.cc:663) and hands it through ICE to the
//! renderer's socket, which sends it to the network service as a message
//! of its own (blink's p2p/socket_client_impl.cc:124), whose `sendto` takes
//! it (services/network/p2p/socket_udp.cc:485), before the next. A SACK
//! comes in on the same thread (dcsctp_transport.cc:744-747), and so does
//! its send opportunity (dcsctp_socket.cc:1591-1614). So a send
//! opportunity starts once the thread is done with the ones before it, its
//! chunks go out as of then, and its packets leave one after another, each
//! a packet's cost after the one before; a receiver's SACK alone takes its
//! turn the same way. Each goes to the host's uplink as it leaves the
//! thread, in the walk that reaches that time, after what the host handed
//! the uplink before it (its kernel's pure ACKs, its other sockets'
//! writes) and before what after (`Scheduler::hand_over`). An inbound
//! packet takes nothing of the thread here. A step's writes go before the
//! walk's events, which come after the step's start, so the thread takes
//! them first.
//!
//! A receiver takes a packet as its downlink delivers it, in the walk that
//! reaches the delivery (`Scheduler::sctp_through`), and sends its SACK
//! then or holds it as its stack does.
//!
//! A SACK of its own takes its turn on the receiver's uplink and is lost at
//! a full queue as any packet is. A chunk resent while its last copy is
//! still on its way, or through, is a copy: it takes its turn on both
//! access links and counts toward its packet at the receiver, which has
//! it.
//!
//! With a host's log on ([`Scenario::sctp_log`](crate::Scenario::sctp_log)),
//! its associations keep each packet they send or receive and each timer
//! they fire ([`SctpEvent`], the tables' `sctp_events`, as a testbed
//! Chromium node's dcSCTP logs its own): a packet out as the association
//! hands it to its uplink, with its window and flight as it left them; a
//! SACK in as the sender takes it, with the window and flight as the SACK
//! left them; DATA in as it reaches the receiver's access link. The log
//! reads the model and moves nothing.
//!
//! On the testbed's 105 % link the str0m model's window falls every
//! 3.4-4.2 s from about 37 KB, sometimes twice in a recovery, as
//! sctp-proto's does (3.4-4.3 s, 37 KB), and it loses 76 packets to the
//! testbed's 59-77.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::ops::Bound;

use strims_base::time::{Duration, Time};
use strims_wan::Carrier;
use strims_wan::network::overflows;
use strims_wan::wire::tx_time;

use strims_testbed::sctp::{Dir, SctpEvent, What as SctpWhat};

use super::{Event, Offered, Scheduler, What};
use crate::fast_map::FastMap;
use crate::late::HostKind;
use crate::net::ConnKey;
use crate::report::{PacketKind, PacketProto};
use crate::spec::HostIx;
use crate::world::epoch;

/// The SCTP stack an association's rules follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stack {
    SctpProto,
    DcSctp,
}

/// An SCTP stack's sender and receiver as the window model runs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SctpProfile {
    /// The host whose stack this is: a native node (str0m's), or the
    /// browser whose stack it models. A host's link setup, its timers'
    /// lateness, its pacer and its `hosts` row take its kind from this
    /// alone ([`HostKind::of_host`](crate::late::HostKind::of_host)),
    /// whatever the other fields, so a Chromium's stack at another packet
    /// cost stays a Chromium's.
    pub host: HostKind,
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
    /// Whether a page reads a channel's buffered amount as its own count:
    /// each message it sends adds to it, and each of the stack's answers
    /// takes off the bytes the stack had sent of the channel by the answer
    /// since its last, never falling as the stack sends between. Chromium's
    /// page reads it so (branch 8078): Blink's `RTCDataChannel` adds each
    /// message to its `buffered_amount_` as the page sends it and takes off
    /// each `OnBufferedAmountChange`'s `sent_data_size` (blink's
    /// rtc_data_channel.cc:522, 759-767), its `bufferedamountlow` coming as
    /// that takes it to the threshold. That size is the bytes of the
    /// channel's messages the network thread took less dcSCTP's buffered
    /// amount then, past those the last answer took off ([`ANSWER_EVERY`]);
    /// dcSCTP's amount falls as it cuts a message's chunks for a packet
    /// (net/dcsctp/tx/rr_send_queue.cc:266), not at the SACK. So the page's
    /// messages still on their way to the network thread at an answer, and
    /// those it sends after, stay on its count. The stack answers at the
    /// end of a send opportunity that leaves the channel's queue empty, or
    /// that brings what it sent of the channel since its last answer past
    /// 100 KiB (the one a message took, or a SACK's or a timer's that takes
    /// the queue to the threshold it set). A node's writer waiting for its
    /// mark then wakes on an answer.
    pub amount_at_sends: bool,
    /// What each packet takes of its host's one network thread, which
    /// every send opportunity of the host's associations waits for, if its
    /// sends go through one (see the module docs).
    pub packet_cost: Option<Duration>,
}

/// A DATA chunk's header, and a SACK chunk's with no gaps.
const CHUNK_HEADER: usize = 16;
const COMMON_HEADER: usize = 12;
/// A datagram's link framing: Ethernet's 14 bytes, which the testbed's tbf
/// counts in each packet, and an access link's framing about matches. Most
/// of a data channel's packets are small, so it costs a thin relay about
/// 4 % of its uplink.
const FRAMING: usize = 14;
/// A datagram's bytes on its sender's uplink: past its IP bytes, the link
/// charges the framing its site gives it, in place of [`FRAMING`] (see
/// `strims_wan::network`).
fn link_wire(net: &strims_wan::Network, h: usize, wire: usize) -> usize {
    if net.up_framing(h) == strims_wan::framing::Framing::NONE {
        wire
    } else {
        wire.saturating_sub(FRAMING)
    }
}

/// IP and UDP's 28 bytes, the framing, and 37 of an AES-GCM DTLS 1.2
/// record (str0m's).
const DTLS12_OVERHEAD: usize = FRAMING + 28 + 37;
/// IP and UDP's 28 bytes, the framing, and 22 of an AES-GCM DTLS 1.3 record
/// (the browsers': a 5-byte header, the content type and the tag; the
/// testbed's pcaps show their SACKs in 92-byte frames).
const DTLS13_OVERHEAD: usize = FRAMING + 28 + 22;

impl SctpProfile {
    /// The share of a full datagram's charge on a link of `framing` that is
    /// the channel's bytes (a DATA chunk's payload, vnic's): its payload
    /// over its SCTP, DTLS, UDP and IP headers and the link's framing
    /// ([`FRAMING`] where the site charges none of its own).
    pub(crate) fn full_share(&self, framing: strims_wan::framing::Framing) -> f64 {
        let ip = (self.mtu + self.packet_overhead).saturating_sub(FRAMING);
        let link = if framing == strims_wan::framing::Framing::NONE {
            ip + FRAMING
        } else {
            framing.packet(ip)
        };
        self.max_payload as f64 / link.max(1) as f64
    }

    /// sctp-proto 0.10 as str0m 0.24 runs it: 1150-byte datagrams, less
    /// str0m's allowances for DTLS (48) and SCTP (40), carry 1062 bytes of
    /// a chunk. RFC 4960's initial window (min(4 MTU, max(2 MTU, 4380))),
    /// a 1 MiB receive window, the timeout 3 s at first and 1-60 s, and the
    /// delayed SACK 200 ms.
    pub const STR0M: SctpProfile = SctpProfile {
        host: HostKind::Native,
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
        amount_at_sends: false,
        packet_cost: None,
    };

    /// dcSCTP's options as Chromium runs it: 1191-byte packets (a chunk of
    /// 1160 bytes at most, its room rounded down to 4), a first
    /// window of ten, a 5 MiB receive window, the timeout 500 ms at first
    /// and 400 ms-60 s with a variation of 27.5 ms at least (its 220 ms
    /// option over 8) and its timers backing off to 3 s at most, the
    /// delayed SACK 200 ms at most, and four packets a send opportunity;
    /// its packets 12 µs of its network thread each, a host's cost that its
    /// CPU model sets (the farm's EPYC 9R14 hosts' Chromium seeds: 12.3 µs
    /// apart in their trains, whatever their size or peer; a testbed run's
    /// twin takes its run's, `STRIMS_SIM_CAL_PACKET_COST_US`:
    /// strims-sim's `testbed::thread_cost`).
    /// Its misses are Chromium 156's (WebRTC's branch-heads/8078, see the
    /// module docs); WebRTC's main takes a resent chunk's misses only from
    /// SACKs that newly acknowledge a chunk sent after the resend
    /// (`IsMissIndication`) and fast-retransmits a loss found in recovery,
    /// so a profile of a Chromium that ships it takes those rules, as
    /// [`SctpProfile::FIREFOX`] would of a Firefox past 158.
    pub const CHROMIUM: SctpProfile = SctpProfile {
        host: HostKind::Chromium,
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
        amount_at_sends: true,
        packet_cost: Some(Duration::from_micros(12)),
    };

    /// dcSCTP as Firefox 158 runs it: Chromium's options but a 16 MiB
    /// receive window and a delayed SACK of 50 ms, the amount its page
    /// reads taken as its stack's, and no thread's cost (no testbed run
    /// has captured a Firefox seed's packets). (Between two Firefoxes its
    /// chunks are I-DATA, 4 bytes longer.)
    pub const FIREFOX: SctpProfile = SctpProfile {
        host: HostKind::Firefox,
        rwnd: 16 << 20,
        delayed_sack: Duration::from_millis(50),
        amount_at_sends: false,
        packet_cost: None,
        ..SctpProfile::CHROMIUM
    };

    /// This stack with each packet taking `cost` of its host's network
    /// thread, where its sends go through one ([`SctpProfile::packet_cost`]);
    /// any other as it is.
    pub fn with_packet_cost(self, cost: Duration) -> SctpProfile {
        SctpProfile {
            packet_cost: self.packet_cost.map(|_| cost),
            ..self
        }
    }

    /// Whether this is a native node's stack (str0m's), whose channels the
    /// node writes in process, rather than a browser's.
    pub fn native(&self) -> bool {
        self.stack == Stack::SctpProto
    }

    /// The DATA chunk payload a datagram of `ip_len` IP bytes carries, as
    /// strims-sim's testbed module reads a capture's datagrams: its length
    /// less IP's, UDP's and its DTLS record's bytes, SCTP's common header
    /// and one DATA chunk's header; none for a SACK alone.
    pub(crate) fn datagram_payload(&self, ip_len: u64) -> u64 {
        let headers = self.packet_overhead.saturating_sub(FRAMING) + COMMON_HEADER + CHUNK_HEADER;
        ip_len.saturating_sub(headers as u64)
    }

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

/// How a channel cuts its writes into records: sctp-proto into DATA
/// chunks as they are written, at the most a chunk carries; dcSCTP not at
/// all, a message a record, which its association cuts into chunks as it
/// sends them ([`Scheduler::rfc_pump`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Chunking {
    max: usize,
}

impl Chunking {
    pub(crate) fn new(p: &SctpProfile) -> Chunking {
        Chunking {
            max: match p.stack {
                Stack::SctpProto => p.max_payload.max(1),
                Stack::DcSctp => usize::MAX,
            },
        }
    }

    /// The bytes of the next record of a write with `left` bytes still to
    /// cut.
    pub(crate) fn next(&self, left: usize) -> usize {
        left.min(self.max).max(1)
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
    /// Marked to go again within the window, or (dcSCTP) as a fast
    /// retransmission past it.
    retransmit: bool,
    fast: bool,
    /// Acknowledged past a gap.
    acked: bool,
    /// Its last copy was lost, so a resend sends the record.
    lost: bool,
    /// (dcSCTP) A copy of it passed the receiver's downlink.
    through: bool,
}

impl Chunk {
    fn new(id: u64, len: usize, sent: Time) -> Chunk {
        Chunk {
            id,
            len,
            sent,
            nsent: 1,
            misses: 0,
            retransmit: false,
            fast: false,
            acked: false,
            lost: false,
            through: false,
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

/// A packet a send opportunity sends: its chunks, whether it is a fast
/// retransmission, the association's flight once it went (for its log),
/// and whether it carries the SACK its host's receiver holds back for the
/// reverse direction, first, its chunks taking the room it leaves (a
/// dcSCTP send opportunity's first packet's, [`Scheduler::rfc_pump`]); a
/// packet of no chunks is that SACK alone.
#[derive(Debug)]
struct Out {
    chunks: Vec<Send>,
    fast: bool,
    flight: usize,
    sack: bool,
}

/// What a host's stack hands its uplink ([`Scheduler::hand_over`]): a
/// packet of a connection's chunks, or a receiver's SACK alone of
/// `conn`'s data, with its event for the receiver's log if that is on.
#[derive(Clone, Debug)]
pub(super) enum Handover {
    Data(DataOut),
    Sack {
        conn: ConnKey,
        sack: Sack,
        log: Option<SctpEvent>,
    },
}

impl Handover {
    /// The host that hands it over.
    pub(super) fn host(&self, ip_host: &FastMap<IpAddr, HostIx>) -> Option<HostIx> {
        match self {
            Handover::Data(d) => ip_host.get(&d.conn.0.ip()).copied(),
            Handover::Sack { conn, .. } => ip_host.get(&conn.1.ip()).copied(),
        }
    }
}

/// Whether `e` is host `h`'s data channels' event, as `h` crashes at
/// `now`: a packet its network thread hands over, a chunk or a copy
/// reaching its downlink or through it, its receivers' delayed SACKs, and
/// its associations' timers and the SACKs reaching them; or a chunk it
/// sent, a first send or a copy, whose packet hadn't left its uplink by
/// `now`, which the crash forgot with the uplink's queue.
fn sctp_of(e: &Event, h: HostIx, now: Time, ip_host: &FastMap<IpAddr, HostIx>) -> bool {
    let at = |a: &SocketAddr| ip_host.get(&a.ip()) == Some(&h);
    match e {
        Event::SctpOut(hand) => hand.host(ip_host) == Some(h),
        Event::SctpSack { conn } | Event::SctpTimeout { conn } => at(&conn.0),
        Event::SctpChunk { conn, left, .. } => at(&conn.1) || (at(&conn.0) && *left > now),
        Event::SctpDelSack { conn } | Event::SctpThrough { conn, .. } => at(&conn.1),
        _ => false,
    }
}

/// A packet of `conn`'s chunks as its send opportunity built it: each
/// chunk's share of its bytes (the first's with the packet's overhead and
/// the SACK it carries) and whether it is a copy, the SACK it carries, its
/// event for the sender's log and what it carried for the receiver's, each
/// if that log is on, and its I bit.
#[derive(Clone, Debug)]
pub(super) struct DataOut {
    conn: ConnKey,
    chunks: Vec<Send>,
    shares: Vec<usize>,
    copies: Vec<bool>,
    sack: Option<Sack>,
    log: Option<SctpEvent>,
    carried: Option<Carried>,
    immediate: bool,
}

/// What a packet carried, for its log's events at either end
/// ([`SctpEvent`]).
#[derive(Clone, Copy, Debug)]
pub(super) struct Carried {
    what: SctpWhat,
    tsn: Option<(i64, i64)>,
    chunks: u32,
    data_bytes: u64,
    packet_bytes: u64,
}

impl Carried {
    /// The event of a packet that carried this, at `at` in `dir`, with the
    /// SACK it carried (`cum_tsn`, `gaps`) and the association's state.
    fn event(
        &self,
        at: Time,
        dir: Dir,
        sack: Option<(i64, u32)>,
        state: Option<(usize, usize)>,
    ) -> SctpEvent {
        SctpEvent {
            at: at - epoch(),
            dir: Some(dir),
            what: self.what,
            tsn: self.tsn,
            chunks: self.chunks,
            data_bytes: self.data_bytes,
            packet_bytes: self.packet_bytes,
            cum_tsn: sack.map(|(c, _)| c),
            gaps: sack.map(|(_, g)| g),
            cwnd_bytes: state.map(|(c, _)| c as u64),
            flight_bytes: state.map(|(_, f)| f as u64),
            estimated: false,
        }
    }
}

/// A SACK alone of `gaps` gap blocks, as its log takes it.
fn sack_alone(gaps: usize) -> Carried {
    Carried {
        what: SctpWhat::Sack,
        tsn: None,
        chunks: 0,
        data_bytes: 0,
        packet_bytes: (COMMON_HEADER + SctpProfile::sack_chunk(gaps)) as u64,
    }
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
#[derive(Clone, Debug)]
pub(super) struct Sack {
    cum: u64,
    above: Vec<u64>,
    /// For the log, what the packet it rode carried, or none alone.
    rode: Option<Carried>,
}

/// One association's sender.
#[derive(Debug)]
pub(super) struct Assoc {
    profile: SctpProfile,
    cwnd: usize,
    ssthresh: usize,
    partial_acked: usize,
    /// New chunks waiting for the window, in write order, and whether each
    /// is of a message its page counted ([`Scheduler::sctp_write`]); on dcSCTP, records it cuts into chunks as it sends,
    /// the first's bytes it has sent so far, and each record's bytes not
    /// yet through the receiver's downlink, which hands it to the reader
    /// once none are. Bound: the channel's cap.
    pending: VecDeque<(u64, bool)>,
    cut: usize,
    unthrough: FastMap<u64, usize>,
    /// Chunks past the cumulative point, by sequence number, and the bytes
    /// they put in the flight. Bound: the window.
    out: BTreeMap<u64, Chunk>,
    in_flight: usize,
    /// The cumulative point, from the first chunk sent, and that first
    /// chunk's sequence number, which its log counts TSNs from; the
    /// sequence number past the furthest sent, and the least that samples
    /// a round trip.
    cum: Option<u64>,
    base: Option<u64>,
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
    /// SACKs on their way, by event key, each with when its packet's last
    /// bit left the receiver's uplink. Bound: the SACKs a round trip
    /// holds.
    sacks: BTreeMap<u64, (Time, Sack)>,
    /// Its peer's host crashed since it opened ([`Scheduler::sctp_forget`]):
    /// the association goes on sending, as its stack has no word of the
    /// crash, its timer resending at its backoff's cap (WebRTC gives dcSCTP
    /// no limit on retransmissions, media/sctp/dcsctp_transport.cc:809-811,
    /// and sctp-proto has none by default), and the host, up again, takes
    /// none of it, its new stack holding no state for the association (a
    /// restarted browser's new ICE and DTLS state answer nothing on the old
    /// 5-tuple).
    peer_crashed: bool,
    /// The chunks on their way that a later chunk of their packet follows,
    /// and the last chunks of packets with the I bit: the receiver takes
    /// the packet at its last. Bound: the window.
    bundled: BTreeSet<u64>,
    immediate: BTreeSet<u64>,
    /// The chunks that end a message and haven't reached the reader. Bound:
    /// the channel's cap.
    ends: BTreeSet<u64>,
    /// For a page that counts the channel's amount
    /// ([`SctpProfile::amount_at_sends`]): the payload of its own messages'
    /// DATA chunks sent first since the stack last answered it, never the
    /// setup's flights nor a resend, which its page never counted; the queue's payload at which the stack
    /// checks again ([`ANSWER_EVERY`] under the queue's at its last check,
    /// or none); and whether a message ended since the last send
    /// opportunity, after which the stack checks.
    unanswered: usize,
    low: usize,
    wrote: bool,
}

/// How much of a channel a stack whose page counts the amount
/// ([`SctpProfile::amount_at_sends`]) sends past its last answer before
/// it answers again with the queue still waiting. Chromium's
/// `SctpDataChannel::MaybeSendOnBufferedAmountChanged`
/// (pc/sctp_data_channel.cc:872-916, `kMinBufferedAmountDiffToTriggerCallback`
/// :889) checks after a message's send (:941) and as the channel's queue
/// falls to the threshold it set at its last check, this much under the
/// queue then, or empty (:910-914; `OnBufferedAmountLow`, :640), and
/// answers once the queue is empty or the stack has sent more than this
/// since the last answer, with what it sent since (:899-904). Blink's
/// observer takes the answer on the network thread (its
/// `IsOkToCallOnTheNetworkThread`, pc/sctp_data_channel.cc:340-341;
/// sctp_data_channel.cc:220-230 is the signalling thread's adapter for one
/// that doesn't) and posts it to the page's thread, a task
/// per answer (blink's rtc_data_channel.cc:261-267), so the page takes each
/// in turn.
const ANSWER_EVERY: usize = 100 * 1024;

impl Assoc {
    pub(super) fn new(profile: SctpProfile) -> Assoc {
        Assoc {
            profile,
            cwnd: profile.initial_window,
            ssthresh: profile.rwnd,
            partial_acked: 0,
            pending: VecDeque::new(),
            cut: 0,
            unthrough: FastMap::default(),
            out: BTreeMap::new(),
            in_flight: 0,
            cum: None,
            base: None,
            // dcSCTP's TSNs are its own, from 1; sctp-proto's chunks are
            // its records, their sequence numbers their TSNs.
            next_seq: u64::from(profile.stack == Stack::DcSctp),
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
            peer_crashed: false,
            ends: BTreeSet::new(),
            bundled: BTreeSet::new(),
            immediate: BTreeSet::new(),
            unanswered: 0,
            low: 0,
            wrote: false,
        }
    }

    /// Forgets the round trips sampled so far, its timeout back at its
    /// profile's first: chunks sent before now sample none.
    fn restart_round_trips(&mut self) {
        self.srtt_ms = 0;
        self.rttvar_ms = 0.0;
        self.srtt = Duration::ZERO;
        self.rttvar = Duration::ZERO;
        self.rto = self.profile.rto_initial;
        self.measure_from = self.next_seq;
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

    /// When chunk `seq` first went, while it is outstanding.
    #[cfg(test)]
    pub(super) fn sent_at(&self, seq: u64) -> Option<Time> {
        self.out.get(&seq).map(|c| c.sent)
    }

    /// Its window is `cwnd` from now on.
    #[cfg(test)]
    pub(super) fn set_cwnd(&mut self, cwnd: usize) {
        self.cwnd = cwnd;
    }

    /// Its chunks outstanding: each one's TSN, payload bytes and sends.
    #[cfg(test)]
    pub(super) fn chunks(&self) -> Vec<(u64, usize, u32)> {
        self.out.iter().map(|(&s, c)| (s, c.len, c.nsent)).collect()
    }

    /// Chunk `seq` goes again within the window, as a timeout marks it.
    #[cfg(test)]
    pub(super) fn mark_retransmit(&mut self, seq: u64) {
        self.update(seq, |c| c.retransmit = true);
    }

    fn stack(&self) -> Stack {
        self.profile.stack
    }

    /// Sequence number `seq` as its log counts TSNs: from its first chunk.
    fn rel(&self, seq: u64) -> i64 {
        let base = self.base.unwrap_or(seq);
        i64::try_from(seq).unwrap_or(i64::MAX) - i64::try_from(base).unwrap_or(i64::MAX)
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

    /// Record `id` is gone with its link ([`Scheduler::partition`]): its
    /// bytes not yet through wait for no chunk.
    pub(super) fn forget(&mut self, id: u64) {
        self.unthrough.swap_remove(&id);
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

    /// Chunk `seq` of `len` bytes (record `id`) goes out first at `at`, of a
    /// message its page counted if `counted`.
    fn launch(&mut self, at: Time, id: u64, seq: u64, len: usize, counted: bool) {
        if self.cum.is_none() {
            self.cum = Some(seq.saturating_sub(1));
            self.base = Some(seq);
            self.measure_from = seq;
        }
        self.next_seq = self.next_seq.max(seq + 1);
        let c = Chunk::new(id, len, at);
        self.in_flight += c.flight(self.stack());
        self.out.insert(seq, c);
        if counted {
            self.unanswered = self.unanswered.saturating_add(len);
        }
    }
}

/// One association's receiver: its cumulative point, the chunks past it
/// it holds, a lone packet's SACK it holds back until `due`, and the
/// packet it is taking (its key) and whether the chunks it has taken of it
/// held its first DATA or a duplicate, which dcSCTP's alone reads.
#[derive(Debug, Default)]
pub(super) struct Rcv {
    cum: Option<u64>,
    above: BTreeSet<u64>,
    due: Option<Time>,
    marked: Option<(u64, bool)>,
}

impl Scheduler {
    /// When host `h`'s network thread takes up a send opportunity of `p`'s
    /// that came at `at`: once it is done with those before it, if `p`'s
    /// sends go through one (see the module docs).
    fn thread_start(&self, h: HostIx, at: Time, p: &SctpProfile) -> Time {
        match (p.packet_cost, self.threads.get(h)) {
            (Some(_), Some(&free)) => at.max(free),
            _ => at,
        }
    }

    /// When the next packet host `h`'s network thread sends from `at` on
    /// leaves it: its cost after the thread is done with the one before;
    /// at `at` where `p`'s sends take no thread.
    fn thread_send(&mut self, h: HostIx, at: Time, p: &SctpProfile) -> Time {
        let (Some(cost), Some(free)) = (p.packet_cost, self.threads.get_mut(h)) else {
            return at;
        };
        *free = at.max(*free) + cost;
        *free
    }

    /// `conn`'s stack answers its page at `at` that it sent `sent` bytes of
    /// the channel since its last answer, which the page takes off its count
    /// ([`SctpProfile::amount_at_sends`]) once the walk reaches `at`
    /// ([`Scheduler::admit`]).
    fn answer(&mut self, at: Time, conn: ConnKey, sent: usize) {
        if self.buffers.is_some() {
            let key = self.next;
            self.next += 1;
            self.answers.insert((at, key), (conn, sent));
        }
    }

    /// `conn` carries a data channel from now on, sent as `profile` sends.
    pub(crate) fn open_channel(&mut self, conn: ConnKey, profile: SctpProfile) {
        self.assocs.insert(conn, Assoc::new(profile));
    }

    /// Channel `conn` is up: its round trips start from its next chunk,
    /// as an association's estimate takes none from its handshake, whose
    /// flights the model sends as data; and none of its answers from before
    /// reach its page, which counts the messages it writes from now on.
    /// Its answers count only those messages' chunks
    /// ([`Scheduler::sctp_write`]): the side that takes the COOKIE ECHO
    /// writes its COOKIE ACK just before, and the scheduler takes that
    /// write's record after.
    pub(crate) fn channel_up(&mut self, conn: ConnKey) {
        if let Some(a) = self.assocs.get_mut(&conn) {
            a.restart_round_trips();
        }
        self.answers.retain(|_, (c, _)| *c != conn);
    }

    /// Data record `id` of channel `conn` was written by `at`, the last of
    /// its message if `end`, one its page counted in its amount if
    /// `counted` ([`SctpProfile::amount_at_sends`]): a message of its own,
    /// not a flight of the setup's.
    pub(super) fn sctp_write(
        &mut self,
        at: Time,
        conn: ConnKey,
        id: u64,
        end: bool,
        counted: bool,
    ) {
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let record = Assoc::chunk_of(&self.records, id);
        if end && let Some((seq, _)) = record {
            a.ends.insert(seq);
        }
        if a.stack() == Stack::DcSctp
            && let Some((_, len)) = record
        {
            a.unthrough.insert(id, len);
        }
        a.pending.push_back((id, counted));
        a.wrote |= end;
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
        let mut packets: Vec<Out> = Vec::new();
        {
            let Some(a) = self.assocs.get_mut(&conn) else {
                return;
            };
            let mtu = a.profile.mtu;
            // sctp-proto's flight counts a chunk until it is acknowledged,
            // so a resend leaves it as it was.
            let resent = |chunks: Vec<Send>, flight: usize| Out {
                chunks,
                fast: false,
                flight,
                sack: false,
            };
            let (mut poll, mut polled) = (Vec::new(), 0);
            for (&seq, c) in &mut a.out {
                if !c.retransmit || c.acked {
                    continue;
                }
                if polled + c.len > a.cwnd && !poll.is_empty() {
                    packets.extend(
                        bundle(std::mem::take(&mut poll), mtu)
                            .into_iter()
                            .map(|p| resent(p, a.in_flight)),
                    );
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
            packets.extend(
                bundle(poll, mtu)
                    .into_iter()
                    .map(|p| resent(p, a.in_flight)),
            );
            let mut flight = a.in_flight;
            let mut fresh = Vec::new();
            while let Some(&(id, counted)) = a.pending.front() {
                let Some((seq, len)) = Assoc::chunk_of(&self.records, id) else {
                    a.pending.pop_front();
                    continue;
                };
                if a.in_flight + len > a.cwnd && !a.out.is_empty() {
                    break;
                }
                a.pending.pop_front();
                a.launch(at, id, seq, len, counted);
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
            packets.extend(bundle(fresh, mtu).into_iter().map(|chunks| {
                flight += chunks.iter().map(|c| c.len).sum::<usize>();
                Out {
                    chunks,
                    fast: false,
                    flight,
                    sack: false,
                }
            }));
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
                    packets.push(Out {
                        chunks: packet,
                        fast: true,
                        flight: a.in_flight,
                        sack: false,
                    });
                }
            }
        }
        for packet in packets {
            self.sctp_transmit(at, conn, &packet);
        }
    }

    /// A send opportunity of dcSCTP that came at `at`: a fast
    /// retransmission's packet past the window, then up to `max_burst`
    /// packets each of the chunks marked for retransmission and then new
    /// ones, of what the window has left, as of when its host's thread
    /// takes it up. As its last packet goes it answers a page that counts
    /// the amount, as [`ANSWER_EVERY`]'s docs say.
    ///
    /// The first of those packets carries first the SACK its host's
    /// receiver holds back for the reverse direction, if it holds one, and
    /// its chunks take the room the SACK leaves within the MTU; with none
    /// of them fitting, the SACK goes alone. dcSCTP's
    /// `TransmissionControlBlock::SendBufferedPackets`
    /// (net/dcsctp/socket/transmission_control_block.cc:218-275) adds the
    /// SACK (`DataTracker::ShouldSendAck`, also a delayed one) to its first
    /// packet's builder before `RetransmissionQueue::GetChunksToSend` fills
    /// the builder's `bytes_remaining`, and sends a builder that holds the
    /// SACK alone (socket/packet_sender.cc:28-35). There the chunk at the
    /// head of the queue is cut to that room as it goes; here chunks are
    /// cut as they are written ([`Chunking`]), so one that the SACK leaves
    /// no room for goes in the next packet, and a SACK that goes alone
    /// ahead of chunks takes none of the burst's packets. The fast
    /// retransmission's packet carries no SACK
    /// (`TransmissionControlBlock::MaybeSendFastRetransmit`, :191-216, a
    /// builder of its own).
    fn rfc_pump(&mut self, at: Time, conn: ConnKey) {
        let mut packets: Vec<Out> = Vec::new();
        let Some(profile) = self.assocs.get(&conn).map(|a| a.profile) else {
            return;
        };
        let back = (conn.1, conn.0);
        let mut sack = self.held_sack(back);
        let from = self.ip_host.get(&conn.0.ip()).copied();
        let at = from.map_or(at, |h| self.thread_start(h, at, &profile));
        let queued = self.queued(conn);
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
                    let (id, len) = (c.id, c.len);
                    a.update(seq, |c| {
                        c.fast = false;
                        c.nsent += 1;
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
                    packets.push(Out {
                        chunks: packet,
                        fast: true,
                        flight: a.in_flight,
                        sack: false,
                    });
                }
            }
            let mut burst = 0;
            while burst < a.profile.max_burst.unwrap_or(usize::MAX) {
                let rides = sack.take();
                let mut room = a
                    .cwnd
                    .saturating_sub(a.in_flight)
                    .min(room_in_packet.saturating_sub(rides.unwrap_or(0)))
                    / 4
                    * 4;
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
                    let (id, len) = (c.id, c.len);
                    a.update(seq, |c| {
                        c.retransmit = false;
                        c.nsent += 1;
                        c.misses = 0;
                    });
                    packet.push(Send {
                        id,
                        seq,
                        len,
                        resend: true,
                    });
                }
                // The front message's next chunk, cut to the room left.
                while room > CHUNK_HEADER {
                    let Some(&(id, counted)) = a.pending.front() else {
                        break;
                    };
                    let rest = Assoc::chunk_of(&self.records, id)
                        .map_or(0, |(_, len)| len.saturating_sub(a.cut));
                    let len = rest.min(room - CHUNK_HEADER);
                    if len == rest {
                        a.pending.pop_front();
                        a.cut = 0;
                    } else {
                        a.cut += len;
                    }
                    if len == 0 {
                        continue;
                    }
                    room -= chunk_size(len);
                    let seq = a.next_seq;
                    a.launch(at, id, seq, len, counted);
                    packet.push(Send {
                        id,
                        seq,
                        len,
                        resend: false,
                    });
                }
                if packet.is_empty() {
                    if rides.is_some() {
                        packets.push(Out {
                            chunks: packet,
                            fast: false,
                            flight: a.in_flight,
                            sack: true,
                        });
                        continue;
                    }
                    break;
                }
                packets.push(Out {
                    chunks: packet,
                    fast: false,
                    flight: a.in_flight,
                    sack: rides.is_some(),
                });
                burst += 1;
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
        let mut done = at;
        for packet in packets {
            done = if packet.chunks.is_empty() {
                self.sctp_send_sack(at, back)
            } else {
                self.sctp_transmit(at, conn, &packet)
            };
        }
        if profile.amount_at_sends {
            let left = self.queued(conn);
            let answer = self.assocs.get_mut(&conn).and_then(|a| {
                let checks = std::mem::take(&mut a.wrote) || (queued > a.low && left <= a.low);
                if !checks {
                    return None;
                }
                a.low = left.saturating_sub(ANSWER_EVERY);
                let answers = (left == 0 && a.unanswered > 0) || a.unanswered > ANSWER_EVERY;
                answers.then(|| std::mem::take(&mut a.unanswered))
            });
            if let Some(sent) = answer {
                self.answer(done, conn, sent);
            }
        }
    }

    /// The payload of `conn`'s chunks waiting for its window: on dcSCTP,
    /// its messages' bytes not yet cut into chunks.
    pub(super) fn queued(&self, conn: ConnKey) -> usize {
        self.assocs.get(&conn).map_or(0, |a| {
            a.pending
                .iter()
                .filter_map(|&(id, _)| Assoc::chunk_of(&self.records, id))
                .map(|(_, len)| len)
                .sum::<usize>()
                .saturating_sub(a.cut)
        })
    }

    /// A packet of `conn`'s chunks goes out from `at`, as its host's thread
    /// sends it ([`Scheduler::thread_send`]): each chunk credited to its
    /// socket as it first goes, and a resent chunk whose last copy wasn't
    /// lost a copy; then handed to the uplink as the thread hands it over
    /// ([`Scheduler::hand_over`], [`Scheduler::data_handed`]). A packet
    /// given the SACK its sender's host holds back for the reverse direction
    /// ([`Out::sack`]) carries it, and a dcSCTP packet the I bit while the
    /// window is under ten packets.
    /// The sender's log, if on, logs it as it is handed over, with the
    /// window and flight it left. Returns when it went.
    fn sctp_transmit(&mut self, at: Time, conn: ConnKey, out: &Out) -> Time {
        let packet = &out.chunks[..];
        let (Some(_), Some(_)) = (self.window(), packet.first()) else {
            return at;
        };
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return at;
        };
        let Some((profile, cwnd)) = self.assocs.get(&conn).map(|a| (a.profile, a.cwnd)) else {
            return at;
        };
        let at = self.thread_send(from, at, &profile);
        let back = (conn.1, conn.0);
        let sack = if out.sack { self.sack_of(back) } else { None };
        let sack_bytes = sack
            .as_ref()
            .map_or(0, |s| SctpProfile::sack_chunk(gap_blocks(&s.above)));
        let logs = (self.sctp_logs(from), self.sctp_logs(to));
        let carried = (logs.0 || logs.1)
            .then(|| self.carried(conn, packet, out.fast, sack.is_some(), sack_bytes));
        let log = carried.filter(|_| logs.0).map(|c| {
            let sack = sack.as_ref().map(|s| self.sack_tsns(back, s));
            c.event(at, Dir::Out, sack, Some((cwnd, out.flight)))
        });
        let shares = packet
            .iter()
            .enumerate()
            .map(|(i, c)| {
                usize::from(i == 0) * (profile.packet_overhead + COMMON_HEADER + sack_bytes)
                    + chunk_size(c.len)
            })
            .collect();
        let copies = packet
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
                self.admits.entry((at, c.id)).or_insert((conn, 0)).1 += c.len;
            }
        }
        let data = DataOut {
            conn,
            chunks: packet.to_vec(),
            shares,
            copies,
            sack,
            log,
            carried: carried.filter(|_| logs.1),
            immediate: profile.stack == Stack::DcSctp && cwnd < 10 * profile.mtu,
        };
        self.hand_over(at, &profile, Handover::Data(data));
        at
    }

    /// A host's stack of profile `p` hands `hand` to its uplink at `at`: at
    /// once, or, where its sends go through its network thread (`p`'s
    /// [`SctpProfile::packet_cost`]), which hands each packet over past the
    /// work that sent it, in the walk that reaches `at`
    /// ([`Event::SctpOut`]). So its uplink takes each packet after what
    /// the host handed it before `at`, its kernel's pure ACKs and its other
    /// sockets' writes among them, and before what after, as a qdisc takes
    /// packets as they are handed to it.
    fn hand_over(&mut self, at: Time, p: &SctpProfile, hand: Handover) {
        if p.packet_cost.is_none() {
            self.handed(at, hand);
            return;
        }
        let key = self.next;
        self.next += 1;
        self.events
            .insert((at, key), Event::SctpOut(Box::new(hand)));
    }

    /// `hand` goes to its host's uplink at `at` ([`Scheduler::hand_over`]).
    pub(super) fn handed(&mut self, at: Time, hand: Handover) {
        match hand {
            Handover::Data(d) => self.data_handed(at, d),
            Handover::Sack { conn, sack, log } => self.sack_handed(at, conn, sack, log),
        }
    }

    /// Host `h` crashes at `now`, before the walk that reaches it: what its
    /// stack did by then it did, in time order, as the walk would have run
    /// it. Its network thread handed its uplink each packet due by then
    /// ([`Scheduler::handed`]), and its stack took each chunk its downlink
    /// delivered by then ([`Scheduler::sctp_received`]) and sent the SACK
    /// it sends at once. So its uplink holds what they handed it as the
    /// crash finds it, as it holds what the host handed it before.
    pub(super) fn sctp_until_crash(&mut self, h: HostIx, now: Time) {
        let mut after = None;
        loop {
            let ip_host = &self.ip_host;
            let from = after.map_or(Bound::Unbounded, Bound::Excluded);
            let next = self
                .events
                .range((from, Bound::Included((now, u64::MAX))))
                .find(|(_, e)| match e {
                    Event::SctpOut(hand) => hand.host(ip_host) == Some(h),
                    Event::SctpThrough { conn, .. } => ip_host.get(&conn.1.ip()) == Some(&h),
                    _ => false,
                })
                .map(|(&k, _)| k);
            let Some(key) = next else {
                break;
            };
            // What they bring comes later in the map: at their time or
            // after, keyed by the scheduler's counter.
            after = Some(key);
            match self.events.remove(&key) {
                Some(Event::SctpOut(hand)) => self.handed(key.0, *hand),
                Some(Event::SctpThrough {
                    conn,
                    seq,
                    packet,
                    last,
                    immediate,
                }) => self.sctp_received(key.0, conn, seq, (packet, last, immediate)),
                _ => {}
            }
        }
    }

    /// Host `h`'s data channels die with it at its crash at `now`, after
    /// its links forgot what they held ([`strims_wan::Network::reset`]): it
    /// forgets its associations, its receivers' state, and every event that
    /// would move them or bring them a packet ([`sctp_of`]): what its
    /// network thread had yet to hand over, what its downlink had yet to
    /// deliver, its timers, and the SACKs on their way to it. What its
    /// uplink had yet to send reaches no peer: neither its chunks nor its
    /// SACKs, alone or riding its data, whose packets left after `now`.
    /// (The run cuts its links before the crash, [`Scheduler::partition`],
    /// which loses what was on its way on them too, its chunks' records
    /// and [`Scheduler::sctp_cut`]'s packets.) Its peers' associations with
    /// it live on ([`Assoc::peer_crashed`]).
    pub(super) fn sctp_forget(&mut self, h: HostIx, now: Time) {
        let ip_host = &self.ip_host;
        let at = |a: &SocketAddr| ip_host.get(&a.ip()) == Some(&h);
        self.events.retain(|_, e| !sctp_of(e, h, now, ip_host));
        self.assocs.retain(|conn, _| !at(&conn.0));
        for (_, a) in self.assocs.iter_mut().filter(|(conn, _)| at(&conn.1)) {
            a.peer_crashed = true;
            a.sacks.retain(|_, (left, _)| *left <= now);
        }
        self.rcvs.retain(|conn, _| !at(&conn.1));
    }

    /// The path between hosts `x` and `y` was cut
    /// ([`Scheduler::partition`]): the data channels' packets on their way
    /// between them are lost with the messages on the link, as turmoil
    /// loses those: every chunk, a first send (lost to its sender, as at a
    /// full queue) or a copy, those still in their sender's uplink queue
    /// among them, and every SACK. What a downlink took before the cut is
    /// its receiver's. Until the path is repaired it loses what is handed
    /// onto it ([`Scheduler::data_handed`], [`Scheduler::sack_handed`]), as
    /// turmoil does a message sent across it.
    pub(super) fn sctp_cut(&mut self, x: HostIx, y: HostIx) {
        let ip_host = &self.ip_host;
        let between = |c: &ConnKey| {
            let ends = (ip_host.get(&c.0.ip()), ip_host.get(&c.1.ip()));
            ends == (Some(&x), Some(&y)) || ends == (Some(&y), Some(&x))
        };
        let mut lost = Vec::new();
        self.events.retain(|_, e| match e {
            Event::SctpChunk {
                conn, seq, copy, ..
            } if between(conn) => {
                if !*copy {
                    lost.push((*conn, *seq));
                }
                false
            }
            _ => true,
        });
        for (_, a) in self.assocs.iter_mut().filter(|(c, _)| between(c)) {
            a.sacks.clear();
        }
        for (conn, seq) in lost {
            self.sctp_lost(conn, seq);
        }
    }

    /// Packet `d` of its connection's chunks goes to its sender's uplink at
    /// `at`: lost whole at a full uplink queue or on its path (always on a
    /// cut one, [`Scheduler::partition`]), its chunks
    /// passing the receiver's downlink one after another, the first charged
    /// the packet's overhead. The receiver's log, if on, logs it as it
    /// arrives (with a SACK, as the receiver takes the SACK), and the
    /// sender's as it is handed over.
    fn data_handed(&mut self, at: Time, d: DataOut) {
        let DataOut {
            conn,
            chunks: packet,
            shares,
            copies,
            sack,
            log,
            carried,
            immediate,
        } = d;
        let Some(cap) = self.window() else {
            return;
        };
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        if let Some(e) = log {
            self.sctp_event(from, to, conn, e);
        }
        let back = (conn.1, conn.0);
        let rode = sack.is_some();
        let wire = link_wire(&self.net, from, shares.iter().sum());
        let charged = self.net.up_charge(from, wire, 1);
        let mut offer = Offered {
            at,
            from,
            to: Some(to),
            conn: Some(conn),
            proto: PacketProto::Udp,
            kind: if rode {
                PacketKind::DataSack
            } else {
                PacketKind::Data
            },
            wire,
            packets: 1,
            took: 1,
            off: None,
            resent: None,
            held: self.held(from, at),
            transit: None,
            path_lost: None,
        };
        let t = if self.net.up_admit(from, at, cap, wire, 1).packets == 0 {
            offer.took = 0;
            self.offered(&offer);
            if let Some(d) = self.drops.get_mut(from) {
                *d += 1;
            }
            self.queue_lost(from, at, 1);
            None
        } else {
            let mut t = self.net.send(at, from, to, wire, 1, Carrier::Datagram);
            t.dropped |= self.partitioned.contains(&super::pair_key(from, to));
            if t.dropped {
                self.count_lost(from, at, 1);
            }
            offer.transit = Some((t.depart, t.arrival));
            offer.path_lost = t.dropped.then_some(0);
            self.offered(&offer);
            let app: usize = packet.iter().filter(|c| !c.resend).map(|c| c.len).sum();
            self.sent(from, app);
            // The resent chunks' share of what the link charged the packet.
            let size = |resend: bool| -> usize {
                packet
                    .iter()
                    .filter(|c| c.resend == resend)
                    .map(|c| chunk_size(c.len))
                    .sum()
            };
            let (old, new) = (size(true), size(false));
            if old > 0 {
                self.resent(from, charged * old / (old + new));
            }
            Some(t)
        };
        if let Some(mut sack) = sack {
            if carried.is_some() {
                sack.rode = carried;
            }
            let went = t.filter(|t| !t.dropped).map(|t| (t.depart, t.arrival));
            self.sack_held(back, sack, went);
        }
        let Some(t) = t.filter(|t| !t.dropped) else {
            for (c, copy) in packet.iter().zip(&copies) {
                if !copy {
                    self.sctp_lost(conn, c.seq);
                }
            }
            return;
        };
        if !rode && let Some(c) = carried {
            self.sctp_event(to, from, conn, c.event(t.arrival, Dir::In, None, None));
        }
        let last = packet.len().saturating_sub(1);
        let cut = self
            .assocs
            .get(&conn)
            .is_some_and(|a| a.stack() == Stack::DcSctp);
        // The packet's key, its first chunk event's on dcSCTP, whose every
        // chunk is one.
        let in_packet = self.next;
        for (i, ((c, &copy), &w)) in packet.iter().zip(&copies).zip(&shares).enumerate() {
            if !copy
                && cut
                && let Some(x) = self
                    .assocs
                    .get_mut(&conn)
                    .and_then(|a| a.out.get_mut(&c.seq))
            {
                x.lost = false;
            }
            if copy || cut {
                let key = self.next;
                self.next += 1;
                self.events.insert(
                    (t.arrival, key),
                    Event::SctpChunk {
                        conn,
                        seq: c.seq,
                        wire: w,
                        copy,
                        packet: in_packet,
                        last: i == last,
                        immediate,
                        left: t.depart,
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

    /// Chunk `seq` of `conn` (`wire` bytes; `flags` as
    /// [`Scheduler::sctp_received`] takes them), a copy or a dcSCTP chunk,
    /// which its record doesn't carry, reached its receiver's downlink at
    /// `t`: lost at a full queue, a chunk not a `copy` lost to its sender.
    /// One that reaches a host down from a crash is gone, as a message on
    /// its cut links is ([`Scheduler::partition`], which loses the chunks
    /// on their way too, [`Scheduler::sctp_cut`]); one that reaches it up
    /// again takes its downlink, and its stack none of it
    /// ([`Scheduler::sctp_received`]).
    pub(super) fn sctp_chunk_arrived(
        &mut self,
        t: Time,
        conn: ConnKey,
        (seq, wire, copy): (u64, usize, bool),
        flags: (u64, bool, bool),
    ) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        if self.down.get(to).copied().unwrap_or(false) {
            return;
        }
        if let Some(cap) = self.window() {
            let rate = self.net.site(to).map_or(u64::MAX, |s| s.profile.down);
            if overflows(self.net.down_free(to) - t, tx_time(wire, rate), cap) {
                if let Some(d) = self.drops.get_mut(from) {
                    *d += 1;
                }
                self.queue_lost(from, t, 1);
                if !copy {
                    self.sctp_lost(conn, seq);
                }
                return;
            }
        }
        let done = self.net.receive(t, to, wire);
        self.sctp_through(done.max(t), conn, seq, flags);
    }

    /// Chunk `seq` of `conn` passes its receiver's downlink at `t`: an
    /// sctp-proto chunk its record carries, of no packet's key (its
    /// receiver reads no mark, [`Rcv`]).
    pub(super) fn sctp_arrived(&mut self, t: Time, conn: ConnKey, seq: u64) {
        let flags = self.assocs.get_mut(&conn).map_or((0, true, false), |a| {
            (0, !a.bundled.remove(&seq), a.immediate.remove(&seq))
        });
        self.sctp_through(t, conn, seq, flags);
    }

    /// Chunk `seq` of `conn` (`flags` as [`Scheduler::sctp_received`]
    /// takes them) passes its receiver's downlink at `t`, and its
    /// receiver's stack takes it then, in the walk that reaches `t`
    /// ([`Event::SctpThrough`]), as dcSCTP takes a packet as its socket
    /// hands it over (on a Chromium's network thread,
    /// dcsctp_transport.cc:744-747) and sends its SACK as it takes it or on
    /// its delayed SACK's timer. So a SACK goes to its host's uplink no
    /// sooner than what the host handed it before the delivery, which the
    /// uplink serves first, as a qdisc serves what it was given in the
    /// order given; a receiver's state, which its own packets read (the
    /// SACK it holds riding its DATA), holds no chunk before its delivery.
    fn sctp_through(
        &mut self,
        t: Time,
        conn: ConnKey,
        seq: u64,
        (packet, last, immediate): (u64, bool, bool),
    ) {
        let key = self.next;
        self.next += 1;
        self.events.insert(
            (t, key),
            Event::SctpThrough {
                conn,
                seq,
                packet,
                last,
                immediate,
            },
        );
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
    /// downlink at `t` (`flags`: its packet's key, whether the chunk is its
    /// last, and the packet's I bit): the receiver takes the chunk, and at its packet's
    /// last, sends a SACK now or holds a lone packet's. A receiver that
    /// crashed since `conn`'s association opened takes nothing of it, up
    /// again or not ([`Assoc::peer_crashed`]): the copies its peer's timer
    /// sends come through its downlink and go no further.
    pub(super) fn sctp_received(
        &mut self,
        t: Time,
        conn: ConnKey,
        seq: u64,
        (packet, last, immediate): (u64, bool, bool),
    ) {
        self.chunk_through(conn, seq);
        if self.assocs.get(&conn).is_some_and(|a| a.peer_crashed) {
            return;
        }
        let Some(p) = self.receiver_profile(conn) else {
            return;
        };
        let own_rto = self.assocs.get(&(conn.1, conn.0)).map(|a| a.rto);
        let r = self.rcvs.entry(conn).or_default();
        let first = r.cum.is_none();
        let cum = *r.cum.get_or_insert(seq.saturating_sub(1));
        let duplicate = seq <= cum || r.above.contains(&seq);
        // A mark of a packet whose last chunk the downlink lost goes with it.
        let carried = r.marked.take().is_some_and(|m| m == (packet, true));
        let marked = carried || first || duplicate;
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
            r.marked = Some((packet, marked));
            return;
        }
        let at_once = !r.above.is_empty()
            || r.due.is_some()
            || immediate
            || (p.stack == Stack::DcSctp && marked);
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

    /// dcSCTP chunk `seq` of `conn` is through its receiver's downlink: its
    /// record's bytes count it the first time, and with none left the
    /// record is due to its reader ([`Scheduler::match_queues`]).
    fn chunk_through(&mut self, conn: ConnKey, seq: u64) {
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let Some(c) = a
            .out
            .get_mut(&seq)
            .filter(|_| a.profile.stack == Stack::DcSctp)
        else {
            return;
        };
        if std::mem::replace(&mut c.through, true) {
            return;
        }
        let (id, len) = (c.id, c.len);
        let Some(left) = a.unthrough.get_mut(&id) else {
            return;
        };
        *left = left.saturating_sub(len);
        if *left > 0 {
            return;
        }
        a.unthrough.swap_remove(&id);
        if let Some(r) = self.records.get_mut(&id) {
            r.state = super::State::Due;
            if let Some(ls) = self.links.get_mut(r.link) {
                ls.due = true;
            }
        }
    }

    /// `conn`'s receiver sends the SACK it holds back, if it still does at
    /// `t`.
    pub(super) fn sctp_delayed_sack(&mut self, t: Time, conn: ConnKey) {
        if self.rcvs.get(&conn).is_some_and(|r| r.due == Some(t)) {
            self.sctp_send_sack(t, conn);
        }
    }

    /// The bytes of the SACK chunk `conn`'s receiver holds back, if it
    /// holds one: its host's next dcSCTP send opportunity on the reverse
    /// direction carries it ([`Scheduler::rfc_pump`]).
    fn held_sack(&self, conn: ConnKey) -> Option<usize> {
        let r = self
            .rcvs
            .get(&conn)
            .filter(|r| r.due.is_some() && r.cum.is_some())?;
        let above: Vec<u64> = r.above.iter().copied().collect();
        Some(SctpProfile::sack_chunk(gap_blocks(&above)))
    }

    /// The SACK `conn`'s receiver would send now, no longer held back.
    fn sack_of(&mut self, conn: ConnKey) -> Option<Sack> {
        let r = self.rcvs.get_mut(&conn)?;
        r.due = None;
        let cum = r.cum?;
        Some(Sack {
            cum,
            above: r.above.iter().copied().collect(),
            rode: None,
        })
    }

    /// `sack` of `conn` rode a packet whose last bit left the receiver's
    /// uplink at `went`'s first and that reaches the sender at its second,
    /// if the packet wasn't lost.
    fn sack_held(&mut self, conn: ConnKey, sack: Sack, went: Option<(Time, Time)>) {
        let Some((left, at)) = went else {
            return;
        };
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let key = self.next;
        self.next += 1;
        a.sacks.insert(key, (left, sack));
        self.events.insert((at, key), Event::SctpSack { conn });
    }

    /// `conn`'s receiver sends a SACK from `t`, as its host's thread sends
    /// it ([`Scheduler::thread_send`]), and hands it to its uplink as the
    /// thread hands it over ([`Scheduler::hand_over`],
    /// [`Scheduler::sack_handed`]). Its log, if on, logs it as it is
    /// handed over, with the window and flight of its own association.
    /// Returns when it went.
    fn sctp_send_sack(&mut self, t: Time, conn: ConnKey) -> Time {
        let (Some(_), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return t;
        };
        let Some(sack) = self.sack_of(conn) else {
            return t;
        };
        let Some(p) = self.receiver_profile(conn) else {
            return t;
        };
        let t = self.thread_send(to, t, &p);
        let log = self.sctp_logs(to).then(|| {
            let back = (conn.1, conn.0);
            let state = self.assocs.get(&back).map(|a| (a.cwnd, a.in_flight));
            let tsns = self.sack_tsns(conn, &sack);
            sack_alone(gap_blocks(&sack.above)).event(t, Dir::Out, Some(tsns), state)
        });
        self.hand_over(t, &p, Handover::Sack { conn, sack, log });
        t
    }

    /// `conn`'s receiver's SACK alone goes to its uplink at `t`: a packet
    /// that reaches the sender a one-way trip after it leaves, or is lost
    /// to a full uplink queue or on a cut path ([`Scheduler::partition`]);
    /// `log` goes to its log.
    fn sack_handed(&mut self, t: Time, conn: ConnKey, sack: Sack, log: Option<SctpEvent>) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let Some(p) = self.receiver_profile(conn) else {
            return;
        };
        if let Some(e) = log {
            self.sctp_event(to, from, conn, e);
        }
        let wire = link_wire(
            &self.net,
            to,
            p.packet_overhead + COMMON_HEADER + SctpProfile::sack_chunk(gap_blocks(&sack.above)),
        );
        let mut offer = Offered {
            at: t,
            from: to,
            to: Some(from),
            conn: Some(conn),
            proto: PacketProto::Udp,
            kind: PacketKind::Sack,
            wire,
            packets: 1,
            took: 1,
            off: None,
            resent: None,
            held: self.held(to, t),
            transit: None,
            path_lost: None,
        };
        if let Some(cap) = self.window()
            && self.net.up_admit(to, t, cap, wire, 1).packets == 0
        {
            offer.took = 0;
            self.offered(&offer);
            if let Some(d) = self.drops.get_mut(to) {
                *d += 1;
            }
            self.queue_lost(to, t, 1);
            return;
        }
        let depart = self.net.occupy_up(t, to, wire, 1);
        self.sack_sent(to, self.net.up_charge(to, wire, 1), t, depart);
        let at = depart + self.net.one_way(to, from);
        offer.transit = Some((depart, at));
        if self.partitioned.contains(&super::pair_key(from, to)) {
            offer.path_lost = Some(0);
            self.offered(&offer);
            self.count_lost(to, t, 1);
            return;
        }
        self.offered(&offer);
        self.sack_held(conn, sack, Some((depart, at)));
    }

    /// SACK `key` of `conn` reaches the sender at `t`, which takes it as
    /// its stack does (see the module docs).
    pub(super) fn sctp_sacked(&mut self, t: Time, conn: ConnKey, key: u64) {
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
        };
        let Some((_, sack)) = a.sacks.remove(&key) else {
            return;
        };
        let Some(cum0) = a.cum else {
            return;
        };
        let acked = if sack.cum < cum0 {
            0
        } else {
            match a.stack() {
                Stack::SctpProto => sctp_proto_sacked(a, t, conn, &sack, cum0),
                Stack::DcSctp => rfc_sacked(a, t, &sack, cum0),
            }
        };
        self.sctp_took(t, conn, &sack);
        if sack.cum < cum0 {
            return;
        }
        let Some(a) = self.assocs.get_mut(&conn) else {
            return;
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
        let state = (a.cwnd, a.in_flight);
        let key = self.next;
        self.next += 1;
        self.events.insert((due, key), Event::SctpTimeout { conn });
        if let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) && self.sctp_logs(from)
        {
            let e = SctpEvent {
                at: t - epoch(),
                dir: None,
                what: SctpWhat::T3,
                tsn: None,
                chunks: 0,
                data_bytes: 0,
                packet_bytes: 0,
                cum_tsn: None,
                gaps: None,
                cwnd_bytes: Some(state.0 as u64),
                flight_bytes: Some(state.1 as u64),
                estimated: false,
            };
            self.sctp_event(from, to, conn, e);
        }
        self.sctp_pump(t, conn);
    }

    /// What packet `packet` of `conn`'s chunks carries, for its log: a
    /// fast retransmission's if `fast`, with a SACK of `sack_bytes` if
    /// `sack`.
    fn carried(
        &self,
        conn: ConnKey,
        packet: &[Send],
        fast: bool,
        sack: bool,
        sack_bytes: usize,
    ) -> Carried {
        let a = self.assocs.get(&conn);
        let tsn = packet
            .iter()
            .map(|c| a.map_or(0, |a| a.rel(c.seq)))
            .fold(None, |r: Option<(i64, i64)>, x| {
                Some(r.map_or((x, x), |(lo, hi)| (lo.min(x), hi.max(x))))
            });
        let what = if fast {
            SctpWhat::FastRtx
        } else if packet.iter().any(|c| c.resend) {
            SctpWhat::Rtx
        } else if sack {
            SctpWhat::DataSack
        } else {
            SctpWhat::Data
        };
        Carried {
            what,
            tsn,
            chunks: u32::try_from(packet.len()).unwrap_or(u32::MAX),
            data_bytes: packet.iter().map(|c| c.len as u64).sum(),
            packet_bytes: (COMMON_HEADER
                + sack_bytes
                + packet.iter().map(|c| chunk_size(c.len)).sum::<usize>())
                as u64,
        }
    }

    /// The cumulative TSN and gap blocks of `sack`, a SACK of `conn`'s
    /// data, as the log counts them.
    fn sack_tsns(&self, conn: ConnKey, sack: &Sack) -> (i64, u32) {
        let cum = self.assocs.get(&conn).map_or(-1, |a| a.rel(sack.cum));
        (
            cum,
            u32::try_from(gap_blocks(&sack.above)).unwrap_or(u32::MAX),
        )
    }

    /// `conn`'s sender took `sack` at `t`: the packet that brought it, in
    /// its log, with the window and flight it left.
    fn sctp_took(&mut self, t: Time, conn: ConnKey, sack: &Sack) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        if !self.sctp_logs(from) {
            return;
        }
        let state = self.assocs.get(&conn).map(|a| (a.cwnd, a.in_flight));
        let tsns = self.sack_tsns(conn, sack);
        let carried = sack
            .rode
            .unwrap_or_else(|| sack_alone(gap_blocks(&sack.above)));
        let e = carried.event(t, Dir::In, Some(tsns), state);
        self.sctp_event(from, to, conn, e);
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
/// before it: returns the payload it acknowledged. The window grows by the
/// chunks' serialized bytes, header and padding with the payload, as
/// `OutstandingData::AckChunk`'s `bytes_acked` counts them.
fn rfc_sacked(a: &mut Assoc, t: Time, sack: &Sack, cum0: u64) -> usize {
    let stack = a.stack();
    let flight_before = a.in_flight;
    if sack.above.is_empty()
        && sack.cum > cum0
        && sack.cum >= a.measure_from
        && let Some(c) = a.out.get(&sack.cum)
        && c.nsent == 1
    {
        let sent = c.sent;
        a.sampled(t - sent);
    }
    if a.in_recovery && sack.cum >= a.recovery_exit {
        a.in_recovery = false;
    }
    let (mut acked, mut acked_wire, mut highest) = (0, 0, cum0);
    while let Some(entry) = a.out.first_entry() {
        if *entry.key() > sack.cum {
            break;
        }
        let (seq, c) = entry.remove_entry();
        a.in_flight = a.in_flight.saturating_sub(c.flight(stack));
        if !c.acked {
            acked += c.len;
            acked_wire += chunk_size(c.len);
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
        acked_wire += chunk_size(c.len);
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
    let (mut loss, recovering) = (false, a.in_recovery);
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
        let lost = c.misses + 1 >= 3;
        a.update(seq, |c| {
            c.misses += 1;
            if lost && recovering {
                c.retransmit = true;
            } else if lost {
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
                a.cwnd += acked_wire.min(mtu);
            }
        } else {
            a.partial_acked += acked_wire;
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

    /// The records `chunking` cuts `writes` into.
    fn cuts(chunking: Chunking, writes: &[usize]) -> Vec<Vec<usize>> {
        writes
            .iter()
            .map(|&len| {
                let mut pieces = Vec::new();
                let mut left = len;
                while left > 0 {
                    let n = chunking.next(left);
                    pieces.push(n);
                    left -= n;
                }
                pieces
            })
            .collect()
    }

    /// An association that comes up forgets the round trips its setup's
    /// flights sampled: its timeout is its profile's first again, a chunk
    /// sent before samples none, and its link's first chunk samples.
    #[test]
    fn an_association_up_starts_its_round_trips() {
        let t = |ms: i64| Time::EPOCH + Duration::from_millis(ms);
        let p = SctpProfile::CHROMIUM;
        let mut a = Assoc::new(p);
        a.launch(t(0), 1, 10, 100, true);
        a.launch(t(0), 2, 11, 100, true);
        rfc_sacked(
            &mut a,
            t(20),
            &Sack {
                cum: 10,
                above: Vec::new(),
                rode: None,
            },
            9,
        );
        assert_eq!(a.srtt, Duration::from_millis(20), "the setup's sample");
        assert_ne!(a.rto, p.rto_initial);
        a.restart_round_trips();
        assert_eq!((a.srtt, a.rto), (Duration::ZERO, p.rto_initial));
        rfc_sacked(
            &mut a,
            t(30),
            &Sack {
                cum: 11,
                above: Vec::new(),
                rode: None,
            },
            10,
        );
        assert_eq!(a.srtt, Duration::ZERO, "a setup chunk acked after");
        a.launch(t(40), 3, 12, 100, true);
        rfc_sacked(
            &mut a,
            t(65),
            &Sack {
                cum: 12,
                above: Vec::new(),
                rode: None,
            },
            11,
        );
        assert_eq!(a.srtt, Duration::from_millis(25), "the link's first");
    }

    /// dcSCTP's channels hand their association each message whole, which
    /// it cuts as it sends ([`Scheduler::rfc_pump`]).
    #[test]
    fn dcsctp_writes_a_message_whole() {
        for p in [SctpProfile::CHROMIUM, SctpProfile::FIREFOX] {
            assert_eq!(
                cuts(Chunking::new(&p), &[8192, 1, 70_000]),
                [vec![8192], vec![1], vec![70_000]]
            );
        }
    }

    /// An association of `CHROMIUM`'s with chunks 1 to `n` of 1000 bytes
    /// out, and the time they went.
    fn sent(n: u64) -> (Assoc, Time) {
        let mut a = Assoc::new(SctpProfile::CHROMIUM);
        a.cwnd = 1 << 20;
        let t = Time::from_unix_millis(1_000);
        for seq in 1..=n {
            a.launch(t, seq, seq, 1000, true);
        }
        (a, t)
    }

    /// A SACK of cumulative point `cum` and the chunks `above` it held.
    fn take(a: &mut Assoc, t: Time, cum: u64, above: impl IntoIterator<Item = u64>) {
        let cum0 = a.cum.unwrap();
        let sack = Sack {
            cum,
            above: above.into_iter().collect(),
            rode: None,
        };
        rfc_sacked(a, t, &sack, cum0);
    }

    /// Resends chunk `seq`, as a send opportunity does.
    fn resend(a: &mut Assoc, seq: u64) {
        a.update(seq, |c| {
            c.fast = false;
            c.retransmit = false;
            c.nsent += 1;
            c.misses = 0;
        });
    }

    #[test]
    fn dcsctp_resends_a_chunk_every_third_sack_that_reads_it_missing() {
        let (mut a, t) = sent(20);
        // Chunk 2 lost: three SACKs past it make it a fast retransmission,
        // and start fast recovery.
        for top in 3..=5 {
            take(&mut a, t, 1, 3..=top);
        }
        assert!(a.out[&2].fast && a.in_recovery);
        resend(&mut a, 2);
        // The SACKs its receiver sent before the resend reached it read it
        // missing again: the third marks it lost again.
        for top in 6..=7 {
            take(&mut a, t, 1, 3..=top);
            assert!(!a.out[&2].retransmit && !a.out[&2].fast, "after {top}");
        }
        take(&mut a, t, 1, 3..=8);
        assert_eq!(a.out[&2].misses, 3);
        assert!(a.out[&2].retransmit);
    }

    #[test]
    fn dcsctp_resends_a_loss_found_in_fast_recovery_within_the_window() {
        let (mut a, t) = sent(20);
        for top in 3..=5 {
            take(&mut a, t, 1, 3..=top);
        }
        assert!(a.in_recovery);
        take(&mut a, t, 1, 3..=6);
        // Chunk 7 lost too, found as recovery runs.
        for top in 8..=10 {
            take(&mut a, t, 1, (3..=6).chain(8..=top));
        }
        assert!(a.in_recovery);
        let c = &a.out[&7];
        assert!(c.retransmit && !c.fast, "{c:?}");
        // It left the flight all the same.
        assert_eq!(c.flight(Stack::DcSctp), 0);
    }

    #[test]
    fn sctp_proto_cuts_at_the_most_a_chunk_carries() {
        let p = SctpProfile::STR0M;
        let w = cuts(Chunking::new(&p), &[3000, 3000]);
        let full = p.max_payload;
        assert_eq!(w[1], vec![full, full, 3000 - 2 * full]);
    }
}
