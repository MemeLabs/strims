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
//! A walk hands the uplinks the step's new messages first, at the step's
//! start, and then runs the events due by its time in time order, each at
//! its own time, so every uplink is handed what it carries in time order
//! and serves it in that order, as a qdisc serves what its host hands it:
//! every event comes after the step's start, and no host hands its uplink
//! a packet before the walk reaches the time it hands it over. A receiver
//! takes what its downlink delivers as the delivery comes, and a browser's
//! network thread hands over each packet as it sends it ([`sctp`]). What
//! the step's messages cause within the step (on a path that delivers at
//! once, the pure ACK of a segment through its receiver's downlink) runs
//! in the walk that sent them, before the bin edge at its time is read.
//!
//! The scheduler relies on these properties of turmoil's links:
//!
//! - a link's in-flight queue changes only by appends (new sends, and
//!   messages a host refused and bounced back), by the removal of messages
//!   released in the previous walk, and by partitions (`partition` clears
//!   the queue, `partition_oneway` drops one direction). So a queue is
//!   always the records the last walk kept, followed by new messages. A
//!   walk visits only the links with records or messages in flight
//!   (`Sim::busy_links`), and passes over one whose queue is as long as
//!   its records and none of them due: it checks the queue of every other
//!   link it visits, and that every link with records has messages in
//!   flight, and fails the run when either doesn't hold.
//! - `hold` turns every message on a link back to held, so it never runs on
//!   a link with a message released in the same walk.
//! - `repair` makes a link healthy, so a repaired link is held anew.
//! - A TCP receiver orders segments by sequence number, so a segment that
//!   arrives late (a retransmission) holds back the rest of its
//!   connection, and only that connection.
//! - A connect completes, and its listener accepts, when the SYN is
//!   delivered.
//!
//! A connect's SYN crosses to its acceptor as any message does, and once
//! through the acceptor's downlink is answered at once by a SYN-ACK on the
//! acceptor's uplink, behind what the uplink holds, as Linux answers from
//! the SYN's softirq: a busy seed's SYN-ACK waits out its bursts. The SYN
//! is delivered when the SYN-ACK reaches the connector, which with the
//! window model acknowledges it with a pure ACK ahead of its first data.
//! The SYN and the SYN-ACK charge [`TCP_SYN`]'s bytes, and a full queue
//! takes them as it takes pure ACKs, where a tbf would drop one and Linux
//! resend it a second later. A modelled WebRTC link's ICE check is such a
//! connect ([`Scheduler::ice_check`]): its SYN-ACK is the STUN binding
//! response a browser sends, queued as one, of [`STUN_RESPONSE`]'s bytes,
//! and its connector sends no ACK.
//!
//! The controller's links carry nothing and stay with turmoil.
//!
//! With send buffers ([`Scenario::send_buffer`]), each data segment is
//! admitted to its sender's uplink queue once the queue holds less than
//! the model's `uplink_queue` of transmission time, and its bytes then
//! stop counting against its socket's mark (see [`crate::net`]). Admitted
//! segments are credited, and their writers woken, in admission time and
//! record order at the end of the walk that reaches that time. A sender's
//! new segments reach its uplink in the order its sockets took them, so
//! one socket's burst goes ahead of another's written after it, as each
//! `sendmsg` hands its packets to the qdisc.
//!
//! With the window model ([`SendBuffer::window`]) each TCP connection
//! sends its data segments within a congestion window instead, and is
//! credited as it sends them. It follows Linux's TCP (the Docker
//! testbed's bare TCP bench, `hack/testbed/bench.sh`, checks it):
//!
//! - Writes are cut to [`WINDOW_SEGMENT`](crate::spec::WINDOW_SEGMENT)s.
//!   A segment waits in its connection's queue while the packets in flight
//!   (sent, neither acknowledged nor known lost, each an MSS whatever its
//!   bytes) and its own would pass the window; a connection with nothing
//!   in flight sends one anyway. A connection writing sends what its
//!   window allows at once.
//! - The access queues hold `uplink_queue` of transmission time, up and
//!   down. An uplink queue takes the leading frames of a segment that
//!   fit in what it has left and loses the rest, as a byte-limited FIFO
//!   (Linux's tbf and bfifo) drops a packet
//!   ([`Network::up_admit`](strims_wan::Network::up_admit), the rule
//!   `strims_wan::replay` replays a testbed node's offers by), and frees
//!   each frame's room once its last bit has left
//!   ([`Network::up_backlog`](strims_wan::Network::up_backlog)); a
//!   downlink queue loses a segment that doesn't fit whole. A
//!   datagram that doesn't fit is lost the same way, and forgotten. An
//!   uplink with a shaper bucket (a site's
//!   [`Bucket`](strims_wan::Bucket), `STRIMS_SIM_UPLINK_BURST`) sends what
//!   its tokens cover at once and the rest at its rate, segments,
//!   datagrams, acknowledgements and competing uploads alike, and its
//!   queue holds bytes.
//! - A segment that loses a packet on its path (the sites' loss, drawn
//!   for each packet) is lost to its sender the same way from that packet
//!   on: the packets ahead of it arrive and the rest goes again alone, as
//!   Linux resends the packets a SACK leaves out; one that loses its first
//!   goes again whole.
//! - Its sender learns of the loss from the acknowledgement of a segment
//!   sent after it (RACK, `tcp_rack_mark_lost`, retransmissions' included),
//!   a quarter of the least round trip later, or at once in a recovery or
//!   once three packets past the gap are acknowledged; failing that from
//!   its tail-loss probe's, a round trip on. While anything is in flight
//!   a transmit timer is armed, as Linux's: the tail-loss probe two
//!   smoothed round trips after the last send (2 ms more, or `RTO_MIN`
//!   more with one packet in flight), or in a recovery or the loss state
//!   the retransmission timeout, which resending the oldest segment
//!   re-arms; the probe or the retransmission it sends reaches the
//!   receiver, which acknowledges at once what it owes. A spurious
//!   timeout (nothing lost) cuts nothing, as F-RTO or an undo would take
//!   it back. A retransmission no later segment shows lost waits for the
//!   retransmission timeout, which backs off and restarts the window from
//!   what is still in flight and a packet (Linux's loss state,
//!   `tcp_enter_loss`). The lost segment goes to the front of its
//!   connection's queue. Left out of the loss state: CUBIC's reset at it
//!   (`cubictcp_state`'s `bictcp_reset` and `bictcp_hystart_reset`: the
//!   model keeps the window lost last, HyStart's state and the
//!   TCP-friendly estimate), a lost retransmission in it moving to a
//!   recovery and cutting again (`FLAG_LOST_RETRANS`), and F-RTO.
//! - Recovery is Linux's (`tcp_fastretrans_alert`, [`Recovery`]): a loss
//!   a sender learns of outside one starts one, which lasts until
//!   everything it had sent then is acknowledged and cuts the window once,
//!   its threshold to [`BETA`] of the window; through it the window is
//!   what is in flight and what proportional rate reduction lets it send
//!   (RFC 6937, `tcp_cwnd_reduction`), retransmissions first, and at its
//!   end the threshold. A loss of a segment sent in it starts the next
//!   once it ends.
//! - A segment through the receiver's downlink is acknowledged once the
//!   acknowledgement crosses back, after the receiver's uplink queue. The
//!   receiver acknowledges as Linux's does ([`ack`]): at once past a
//!   segment's worth of bytes (every two full packets), as quickacks, as
//!   its reader drains a short packet outside pingpong mode, else on its
//!   delayed acknowledgement's timer, unless data it sends back carries the
//!   acknowledgement first; pure ones take their turn on its uplink, and
//!   one the path loses (the sites' loss) leaves what it acknowledged to
//!   the receiver's next, pure or on data. The window grows by the
//!   packets acknowledged in slow start, which HyStart's
//!   delay test ends, and as CUBIC (RFC 8312) past it, only while it
//!   limits the connection and outside a recovery; a connection idle for
//!   longer than its retransmission timeout restarts from a smaller one.
//!
//! The connection's order on the wire is its send order, and a resent
//! segment holds back the rest of its connection at the receiver.
//!
//! A connection a data channel opened ([`Scheduler::open_channel`])
//! carries an SCTP association instead ([`sctp`]).
//!
//! A host's competing uploads
//! ([`HostSpec::competing`](crate::spec::HostSpec::competing)) are TCP
//! connections that always have data and reach no host: full segments
//! enter the host's uplink queue within their window, are lost when it is
//! full or to the host's site loss (from the first frame lost, a frame
//! past a segment's first leaving the rest to go again alone), and are
//! acknowledged the upload's round trip after they leave the uplink,
//! unless the site loses the acknowledgement, which the next then carries;
//! their lost segments resent first.
//! Their windows follow the connections' CUBIC, HyStart and recoveries
//! too. They need the window model's bounded queues.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
#[allow(clippy::disallowed_types)]
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_wan::network::{Carried, Departures, RTO_MIN, overflows};
use strims_wan::wire::{TCP_SYN, tcp_wire, tx_time, udp_wire};
use strims_wan::{Carrier, Network, Options, Site};
use turmoil::{Protocol, Segment, Sim};

use crate::UDP_PORT;
use crate::fast_map::FastMap;
use crate::net::{ConnKey, SendBuffers};
use crate::report::{
    ACK_WHYS, AckWaits, PacketKind, PacketLoss, PacketProto, PacketRecord, SctpAssocs,
    WanHostReport, WanReport,
};
use crate::spec::{HostIx, Scenario};
use crate::world::epoch;
use strims_testbed::sctp::SctpEvent;

mod ack;
mod sctp;

use ack::{AckWhy, Owed, RcvClock};
pub(crate) use sctp::Chunking;
pub use sctp::SctpProfile;

/// The wire size charged for a STUN message or a DTLS record other than
/// application data. Their sizes depend on DTLS key material the seed
/// doesn't reach, so a fixed charge keeps a seed's timing reproducible,
/// and so does never losing one: which handshake messages a datagram
/// carries depends on those sizes, so the loss of one datagram would make
/// the handshake recover differently from run to run.
const NOMINAL_DATAGRAM: usize = 1228;

/// The IP bytes of a STUN binding success response over IPv4, its
/// XOR-MAPPED-ADDRESS, MESSAGE-INTEGRITY and FINGERPRINT, as the testbed's
/// Chromium and Firefox answer each other's ICE checks: what a modelled
/// check's reply charges ([`Scheduler::ice_check`]).
const STUN_RESPONSE: usize = 92;

/// An acknowledgement alone that its link charged `wire` bytes, as its
/// uplink carries it.
fn alone(wire: usize) -> Carried {
    Carried {
        bytes: wire as u64,
        packets: 1,
        small_packets: 0,
    }
}

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

/// Whether `p` is charged [`NOMINAL_DATAGRAM`]: STUN (RFC 5389 messages
/// start with 0b00) and DTLS change_cipher_spec, alert and handshake
/// records.
fn nominal(p: &Protocol) -> bool {
    matches!(p, Protocol::Udp(d) if matches!(d.0.first(), Some(0..=3 | 20..=22)))
}

/// The wire bytes and packets a message is charged.
fn wire(p: &Protocol) -> (usize, u32) {
    match p {
        _ if nominal(p) => (NOMINAL_DATAGRAM, 1),
        Protocol::Tcp(Segment::Data(_, b)) => tcp_wire(b.len()),
        Protocol::Tcp(Segment::Syn(_)) => (TCP_SYN, 1),
        Protocol::Tcp(_) => tcp_wire(0),
        Protocol::Udp(d) => udp_wire(d.0.len()),
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
    /// Charged [`NOMINAL_DATAGRAM`], and never lost.
    nominal: bool,
    state: State,
    /// The link it is in flight on ([`Scheduler::links`]).
    link: usize,
}

/// The records in flight by id, never iterated, and how many of them a
/// path dropped.
#[derive(Debug, Default)]
struct Records {
    by_id: FastMap<u64, Record>,
    dropped: usize,
}

impl Records {
    fn get(&self, id: &u64) -> Option<&Record> {
        self.by_id.get(id)
    }

    fn get_mut(&mut self, id: &u64) -> Option<&mut Record> {
        self.by_id.get_mut(id)
    }

    fn insert(&mut self, id: u64, r: Record) {
        self.by_id.insert(id, r);
    }

    fn remove(&mut self, id: &u64) -> Option<Record> {
        let r = self.by_id.swap_remove(id)?;
        if r.state == State::Dropped {
            self.dropped -= 1;
        }
        Some(r)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.by_id.len()
    }
}

#[derive(Clone, Debug)]
enum Event {
    /// The message reaches the receiver's downlink.
    Arrive,
    /// The message is through the receiver's downlink.
    Deliver,
    /// The SYN is through its acceptor's downlink, whose kernel answers it
    /// at once with a SYN-ACK on its uplink.
    SynAck,
    /// The SYN-ACK reaches the connector: the connect completes.
    SynAcked,
    /// The window model: `conn`'s sender learns its segment `seq` was
    /// lost, from later segments' acknowledgements or a tail-loss probe's,
    /// unless a later acknowledgement marked it first.
    Lost {
        conn: (SocketAddr, SocketAddr),
        seq: u64,
    },
    /// The window model: a retransmission timeout tells `conn`'s sender its
    /// retransmitted segment `seq` was lost again.
    Timeout {
        conn: (SocketAddr, SocketAddr),
        seq: u64,
    },
    /// The window model: `conn`'s transmit timer may be due.
    Probe { conn: (SocketAddr, SocketAddr) },
    /// The window model: what `conn`'s transmit timer sent reaches its
    /// receiver, which acknowledges at once what it owes.
    Probed { conn: (SocketAddr, SocketAddr) },
    /// The window model: the bytes `from..from + len` of segment `seq` of
    /// `conn`, in `packets` frames (`wire` on the wire), reach host `to`'s
    /// downlink, or are `through` it; `whole` when they end the segment, as
    /// all of it or the tail resent, and not when they are the leading
    /// frames the sender's uplink queue took, its tail dropped.
    Segment {
        conn: (SocketAddr, SocketAddr),
        to: HostIx,
        seq: u64,
        from: usize,
        len: usize,
        packets: u32,
        wire: usize,
        whole: bool,
        through: bool,
    },
    /// The window model: the receiver of `conn`'s data sends the
    /// acknowledgement it delayed, unless data it sent back carried it.
    DelAck { conn: (SocketAddr, SocketAddr) },
    /// The window model: an acknowledgement reaches `conn`'s sender, of
    /// the bytes at each segment of `pieces` (its sequence number and how
    /// many).
    Ack {
        conn: (SocketAddr, SocketAddr),
        pieces: Vec<(u64, usize)>,
    },
    /// Competing upload `i`'s segment `seq` of `frames` frames is
    /// acknowledged, a round trip of `rtt` after it went out.
    CrossAck {
        i: usize,
        seq: u64,
        frames: u32,
        rtt: Duration,
    },
    /// Competing upload `i` learns its segment `seq` of `frames` frames was
    /// lost.
    CrossLost { i: usize, seq: u64, frames: u32 },
    /// Competing upload `i`'s tail-loss probe is due: what lost
    /// acknowledgements acknowledged and no later one carried, its
    /// acknowledgement carries a round trip on.
    CrossProbe { i: usize },
    /// Competing upload `i` starts, or its setup's next round trip begins,
    /// or once the setup is done its data starts.
    CrossTrip { i: usize },
    /// A SACK of data channel `conn` reaches its sender: the association's
    /// SACK of the event's key.
    SctpSack { conn: (SocketAddr, SocketAddr) },
    /// Data channel `conn`'s retransmission timer is due.
    SctpTimeout { conn: (SocketAddr, SocketAddr) },
    /// Data channel `conn`'s receiver sends the SACK it held back.
    SctpDelSack { conn: (SocketAddr, SocketAddr) },
    /// A copy of chunk `seq` of data channel `conn` (`wire` bytes) reaches
    /// its receiver's downlink, its packet's last bit having left its
    /// sender's uplink at `left`.
    SctpCopy {
        conn: (SocketAddr, SocketAddr),
        seq: u64,
        wire: usize,
        last: bool,
        immediate: bool,
        left: Time,
    },
    /// Chunk `seq` of data channel `conn` (the `last` of its packet, with
    /// the I bit if `immediate`) is through its receiver's downlink, whose
    /// stack takes it.
    SctpThrough {
        conn: (SocketAddr, SocketAddr),
        seq: u64,
        last: bool,
        immediate: bool,
    },
    /// A host's network thread hands a data channel's packet to its uplink.
    SctpOut(Box<sctp::Handover>),
}

/// The window model's segment size, for the window's growth.
pub(crate) const MSS: f64 = 1448.0;

/// What `len` bytes of a segment take of a window: Linux counts the
/// window, what is in flight and what an acknowledgement delivers in
/// packets (`tcp_skb_pcount`), each an MSS whatever its bytes, so a
/// segment's short last packet takes a whole one.
fn window_bytes(len: usize) -> usize {
    len.div_ceil(MSS as usize) * MSS as usize
}

/// One event of the window model's TCP, traced for the bench
/// ([`Scheduler::trace_tcp`]): at `t`, on the connection from host `from`
/// to host `to`, `what` ("tx", "rtx", "drop", "ack", "lost", "rec",
/// "end", "rto", or "acklost" for the receiver's acknowledgement of it
/// lost on the path) of segment `seq` of `len` bytes, and the sender's
/// window, threshold and what the window counts in flight (its packets an
/// MSS each) after it.
#[derive(Clone, Debug)]
pub(crate) struct TcpEvent {
    pub t: Time,
    pub from: HostIx,
    pub to: HostIx,
    pub what: &'static str,
    pub seq: u64,
    pub len: usize,
    pub cwnd: f64,
    pub ssthresh: f64,
    pub in_flight: usize,
}

/// A competing upload's segment, in frames: two, as Linux's buffers are at
/// the testbed's rates and the window model cuts its connections'
/// ([`WINDOW_SEGMENT`](crate::spec::WINDOW_SEGMENT)); a frame's IP bytes;
/// and a whole segment's. Its sequence numbers count frames, so a tail the
/// queue dropped goes again as a segment of its own.
const CROSS_FRAMES: u32 = 2;
const CROSS_FRAME: usize = 1500;
pub(crate) const CROSS_WIRE: usize = CROSS_FRAMES as usize * CROSS_FRAME;
/// CUBIC's scaling constant, in segments per second cubed.
const CUBIC_C: f64 = 0.4;
/// Its TCP-friendly estimate's scale: Reno's window grows a packet for
/// each `cwnd * BETA_SCALE / 8` packets acknowledged (Linux's
/// `beta_scale`, `8 * (1024 + 717) / 3 / (1024 - 717)` in integers).
const BETA_SCALE: f64 = 15.0;
/// Its initial window: 10 segments.
const INIT_WINDOW: f64 = 10.0 * MSS;
/// The window after a loss, as a share of the window (CUBIC's beta).
pub(crate) const BETA: f64 = 0.7;
/// HyStart's samples a round, and the window it starts testing at: 16
/// segments, as Linux's CUBIC.
const HYSTART_SAMPLES: u32 = 8;
const HYSTART_LOW_WINDOW: f64 = 16.0 * MSS;

/// A host's uplink counters in WAN mode (`Scheduler::up_counters`), as the
/// tables name them: the bytes its link charged, the packets and the small
/// packets its uplink carried, its resends of stream data its path lost,
/// packets lost, the window model's window falls and timeouts, and the
/// acknowledgements alone its uplink carried, packets and the bytes its
/// link charged: TCP's pure ACKs, then the data channels' SACKs. What the
/// uplink carried counts each packet once its last bit has left
/// ([`Network::up_carried`]), as a capture past the queue sees it; a frame
/// its queue dropped never left. The acknowledgements aren't in the first
/// three. [`WanHostReport::acks`] and [`WanHostReport::ack_bytes`] count
/// both kinds as their uplink took them, so at the run's end they hold
/// these and those still queued, and those a crash's queue forgot, which
/// never left.
///
/// A resend counts once it has left: the window model's next transmission
/// of a segment its path lost a packet of (the tail from that packet, or
/// the whole segment from its first), the bytes its uplink carried before,
/// as a testbed capture sees a resend; a resend of what its own queue
/// dropped is no retransmit (that frame counts in the packets lost).
/// Without the window model, a stream message that lost a packet arrives
/// late by its [`retransmission`](strims_wan::network::retransmission)
/// delay, and its resend counts that delay after the message left.
/// [`EndStats::retransmits`](strims_wan::network::EndStats::retransmits)
/// counts the losses themselves, as their messages were handed over.
///
/// The packets lost are the host's datagrams the path lost (its sites'
/// loss), and with the window model the packets a full access queue lost
/// that the host sent, a frame each: its connections' segments' frames
/// and its data channels' packets at its uplink queue or at their
/// receiver's downlink queue, its datagrams at either, its SACKs at its
/// uplink queue, and its competing uploads' frames at its uplink queue,
/// each counted from when it was lost: at a queue as it came to it, a
/// datagram on its path as it was handed over. A testbed run's column is
/// its tbf's drops, the frames its uplink queue lost, pure TCP ACKs among
/// them, which the model's queues always take.
pub const UP_COUNTERS: [&str; 11] = [
    "up_bytes",
    "up_packets",
    "small_packets",
    "retransmits",
    "drops",
    "window_cuts",
    "rtos",
    "up_ack_packets",
    "up_ack_bytes",
    "up_sack_packets",
    "up_sack_bytes",
];

/// One TCP connection in the window model, or a competing upload.
#[derive(Debug)]
struct Flow {
    cwnd: f64,
    ssthresh: f64,
    /// What the window counts in flight ([`window_bytes`]: a packet an
    /// MSS), and the bytes that are, sent and neither acknowledged nor
    /// known lost (the link's samples report them).
    in_flight: usize,
    out_bytes: usize,
    /// Data records not yet sent whole, in write order, and how many of
    /// their bytes are unsent: Linux's write queue, whose unsent tail a
    /// write adds to, so writes the window holds back go out together.
    /// Bound: the connection's records in flight.
    unsent: VecDeque<(u64, usize)>,
    /// The segments sent and not yet acknowledged whole, by sequence
    /// number ([`Wire`]), and those lost and owed again, lowest first.
    /// Bound: the segments in flight.
    segments: BTreeMap<u64, Wire>,
    resend: BTreeSet<u64>,
    /// CUBIC's window before its last cut, in segments; when its epoch
    /// began (`NIL` until the first acknowledgement past slow start or a
    /// cut), the window it grows back to and how long that takes; and
    /// its TCP-friendly estimate (`tcp_cwnd`, segments) and the segments
    /// acknowledged toward its next one (`ack_cnt`).
    w_max: f64,
    epoch: Time,
    origin: f64,
    k: f64,
    tcp_cwnd: f64,
    ack_cnt: f64,
    /// The sequence number past the furthest data segment sent (turmoil
    /// numbers a connection's segments).
    sent_end: u64,
    /// The most bytes in flight this round trip and the last, and when
    /// this one began, and whether the window last held back data the
    /// connection had: the window grows only while it limits the flow.
    flight_now: usize,
    flight_last: usize,
    round: Time,
    held: bool,
    /// When each data segment in flight went out, by sequence number, and
    /// the smoothed round trip of their acknowledgements, its variation
    /// and its least. Bound: the segments in flight.
    sent_at: BTreeMap<u64, Time>,
    srtt: Duration,
    rttvar: Duration,
    min_rtt: Duration,
    /// CUBIC's least round trip (`delay_min`), HyStart's: its samples
    /// since the congestion control's state began, after the handshake's.
    delay_min: Duration,
    /// HyStart's round (it ends with the acknowledgement of the segment
    /// this numbers), its least round trip and samples so far, and whether
    /// it ended slow start.
    hs_end: u64,
    hs_rtt: Duration,
    hs_samples: u32,
    hs_found: bool,
    /// When the flow last sent.
    last_send: Time,
    /// Segments the access queues dropped that the sender doesn't know of
    /// yet, by sequence number (when they went out, and when it learns of
    /// the loss if a timer is to tell it: a lost retransmission's timeout,
    /// RACK's reordering window, a tail-loss probe's round trip), and those
    /// it retransmitted and hasn't seen acknowledged; and how often the
    /// retransmission timeout backed off. A segment stays here until its
    /// loss is learned, so a later segment's acknowledgement can mark it
    /// lost before its timer ([`Event::Lost`]). Bound: the segments in
    /// flight.
    lost: BTreeMap<u64, (Time, Option<Time>)>,
    retx: BTreeSet<u64>,
    backoff: u32,
    /// When the transmit timer fires as last armed ([`Flow::arm`],
    /// [`Flow::rearm_rto`]), and whether it is the tail-loss probe or the
    /// retransmission timeout; `NIL` with nothing in flight. While
    /// anything is in flight a timer is armed, as Linux's
    /// (`icsk_pending`), whatever was lost: data, a retransmission or an
    /// acknowledgement. The earliest [`Event::Probe`] queued for it, `NIL`
    /// without ([`Scheduler::watch`]).
    timer_at: Time,
    tlp: bool,
    probe_at: Time,
    /// When the retransmission timeout last fired.
    timed_out: Time,
    /// Its recovery while one runs.
    recovery: Option<Recovery>,
    /// After a retransmission timeout, the sequence number past the
    /// furthest segment sent then: until the cumulative ack passes it
    /// (Linux's loss state) a loss is resent and starts no recovery.
    loss_high: Option<u64>,
    /// Segments acknowledged past the cumulative ack point (SACKed), by
    /// sequence number. Bound: the segments in flight.
    sacked: BTreeMap<u64, usize>,
    /// Segments whose leading frames the uplink queue took and whose tail
    /// it dropped, by sequence number ([`Tail`]). Bound: the segments in
    /// flight.
    tails: BTreeMap<u64, Tail>,
    /// Segments the path lost a packet of, by sequence number, until the
    /// uplink sends them again: their next transmission resends bytes it
    /// carried. Bound: the segments in flight.
    path_lost: BTreeSet<u64>,
    /// The bytes of the connection's stream its new segments have carried:
    /// the next one's first byte, as an offset from the stream's first.
    off_end: u64,
}

/// A segment the window model sent: its bytes, the records they are of
/// (each record's id and its bytes in the segment, in order), the bytes
/// acknowledged so far, and its first byte's offset in its connection's
/// stream (the packet log's).
#[derive(Clone, Debug)]
struct Wire {
    len: usize,
    parts: Vec<(u64, usize)>,
    acked: usize,
    off: u64,
}

/// What is out of a segment whose leading frames the uplink queue took and
/// whose tail it dropped: the head's bytes not yet acknowledged, the
/// tail's, and whether the tail went again. The tail is lost like a
/// segment and resent alone, as Linux resends the frame a SACK left out.
#[derive(Clone, Copy, Debug)]
struct Tail {
    head: usize,
    tail: usize,
    resent: bool,
}

/// A connection's loss recovery, as Linux's `TCP_CA_Recovery` runs it: from
/// the first loss its sender learns of (`tcp_enter_recovery`) until the
/// cumulative ack passes `high`, the sequence number past the furthest
/// segment sent then. Its threshold is [`BETA`] of the window then,
/// `prior` (`prior_cwnd`, CUBIC's `cubictcp_recalc_ssthresh`); through it
/// the window is what is in flight and what proportional rate reduction
/// (RFC 6937, `tcp_cwnd_reduction`) lets the sender send on each
/// acknowledgement: while more than the threshold is in flight, the
/// threshold's share of what was delivered since it began, less what it
/// sent; under the threshold, as much as was delivered and a packet more
/// when the cumulative ack advanced without a new loss, up to the
/// threshold. Losses it learns of in it cut nothing more; at its end the
/// window is the threshold (`tcp_end_cwnd_reduction`: Linux undoes only
/// spurious retransmissions, which the model has none of), and if a
/// segment sent in it is still lost the next recovery begins.
#[derive(Clone, Copy, Debug)]
struct Recovery {
    high: u64,
    prior: f64,
    /// Bytes delivered and sent since it began (`prr_delivered`,
    /// `prr_out`), and the packets the slow start reduction bound added
    /// (one an acknowledgement that advanced snd_una and marked no loss),
    /// in bytes. In a recovery the receiver's out-of-order queue isn't
    /// empty, so it acknowledges each packet at once (`__tcp_ack_snd_check`):
    /// a segment acknowledged at the cumulative point stands for an ack a
    /// packet, a bonus each. They add up, since a segment takes more than
    /// one.
    delivered: usize,
    out: usize,
    bonus: usize,
}

impl Default for Flow {
    fn default() -> Flow {
        Flow {
            cwnd: INIT_WINDOW,
            ssthresh: f64::INFINITY,
            in_flight: 0,
            out_bytes: 0,
            unsent: VecDeque::new(),
            segments: BTreeMap::new(),
            resend: BTreeSet::new(),
            w_max: 0.0,
            epoch: Time::NIL,
            origin: 0.0,
            k: 0.0,
            tcp_cwnd: 0.0,
            ack_cnt: 0.0,
            sent_end: 0,
            flight_now: 0,
            flight_last: 0,
            round: Time::NIL,
            held: false,
            sent_at: BTreeMap::new(),
            srtt: Duration::ZERO,
            rttvar: Duration::ZERO,
            min_rtt: Duration::ZERO,
            delay_min: Duration::ZERO,
            hs_end: 0,
            hs_rtt: Duration::ZERO,
            hs_samples: 0,
            hs_found: false,
            last_send: Time::NIL,
            lost: BTreeMap::new(),
            retx: BTreeSet::new(),
            backoff: 0,
            timer_at: Time::NIL,
            tlp: false,
            probe_at: Time::NIL,
            timed_out: Time::NIL,
            recovery: None,
            loss_high: None,
            sacked: BTreeMap::new(),
            tails: BTreeMap::new(),
            path_lost: BTreeSet::new(),
            off_end: 0,
        }
    }
}

impl Flow {
    /// The retransmission timeout, as Linux sets it: the smoothed round
    /// trip and four times its variation, that at least [`RTO_MIN`].
    fn rto(&self) -> Duration {
        self.srtt + (self.rttvar * 4).max(RTO_MIN)
    }

    /// The retransmission timeout backed off as often as it fired since
    /// new data was last acknowledged (`icsk_rto << icsk_backoff`).
    fn rto_backed(&self) -> Duration {
        self.rto() * (1i64 << self.backoff.min(6))
    }

    /// Arms the retransmission timeout at `now`, as `tcp_rearm_rto` does:
    /// none with nothing in flight; what is left of the oldest outstanding
    /// segment's (the retransmission queue's head, `tcp_rto_delta_us`)
    /// where a tail-loss probe was pending, else a full one on.
    fn rearm_rto(&mut self, now: Time) {
        if self.in_flight == 0 {
            self.timer_at = Time::NIL;
            self.tlp = false;
            return;
        }
        let rto = self.rto_backed();
        let head = self.sent_at.first_key_value().map(|(_, &sent)| sent);
        self.timer_at = match head {
            Some(sent) if self.tlp => (sent + rto).max(now + Duration::from_micros(1)),
            _ => now + rto,
        };
        self.tlp = false;
    }

    /// Arms the transmit timer at `now`, on a send of new data or
    /// (`acked`) an acknowledgement of data, as Linux does
    /// (`tcp_event_new_data_sent`, `tcp_schedule_loss_probe`,
    /// `tcp_set_xmit_timer`): with nothing in flight none; in a recovery or
    /// the loss state the retransmission timeout ([`Flow::rearm_rto`]),
    /// which an acknowledgement re-arms and a send arms where none or a
    /// tail-loss probe is pending; otherwise the tail-loss probe, due two
    /// smoothed round trips on (the path's `base` before one is sampled),
    /// [`RTO_MIN`] more with one packet in flight (a delayed
    /// acknowledgement's) or 2 ms more with more, and no later than the
    /// retransmission timeout: a full one from an acknowledgement, what
    /// remains of the oldest outstanding segment's from a send.
    fn arm(&mut self, now: Time, acked: bool, base: Duration) {
        if self.in_flight == 0 {
            self.timer_at = Time::NIL;
            self.tlp = false;
            return;
        }
        if self.recovery.is_some() || self.loss_high.is_some() {
            if acked || self.timer_at.is_nil() || self.tlp {
                self.rearm_rto(now);
            }
            return;
        }
        let rto = self.rto_backed();
        let srtt = if self.srtt > Duration::ZERO {
            self.srtt
        } else {
            base
        };
        let extra = if self.in_flight as f64 <= MSS {
            RTO_MIN
        } else {
            Duration::from_millis(2)
        };
        let mut timeout = srtt * 2 + extra;
        let left = if acked {
            rto
        } else {
            let oldest = self
                .sent_at
                .first_key_value()
                .map_or(now, |(_, &sent)| sent);
            rto - (now - oldest)
        };
        if left > Duration::ZERO {
            timeout = timeout.min(left);
        }
        self.timer_at = now + timeout;
        self.tlp = true;
    }

    /// A round-trip `sample` of the segment at `seq`: the smoothed round
    /// trip and its variation (RFC 6298), and HyStart's delay test, which
    /// ends slow start once a round's least round trip of its first
    /// [`HYSTART_SAMPLES`] passes CUBIC's least (`delay_min`) by an eighth
    /// of it, clamped to 4-16 ms, as Linux's CUBIC does. A round begins
    /// with the acknowledgement of the first segment sent after the last
    /// one began, whatever the window, as `hystart_update` starts one
    /// before it tests the window against [`HYSTART_LOW_WINDOW`]: its
    /// samples are of the segments it sent alone.
    fn sampled(&mut self, seq: u64, sample: Duration) {
        self.estimate(sample);
        self.delay_min = if self.delay_min == Duration::ZERO {
            sample
        } else {
            self.delay_min.min(sample)
        };
        if self.hs_found || self.cwnd >= self.ssthresh {
            return;
        }
        if seq >= self.hs_end {
            self.hs_end = self.sent_end;
            self.hs_rtt = Duration::MAX;
            self.hs_samples = 0;
        }
        if self.cwnd < HYSTART_LOW_WINDOW {
            return;
        }
        self.hs_rtt = self.hs_rtt.min(sample);
        if self.hs_samples < HYSTART_SAMPLES {
            self.hs_samples += 1;
        } else {
            let thresh =
                (self.delay_min / 8).clamp(Duration::from_millis(4), Duration::from_millis(16));
            if self.hs_rtt > self.delay_min + thresh {
                self.hs_found = true;
                self.ssthresh = self.cwnd;
            }
        }
    }

    /// A round-trip `sample` of the smoothed round trip, its variation
    /// (RFC 6298) and the least round trip (`tcp_rtt_estimator`,
    /// `rtt_min`) alone: the handshake's, which the congestion control's
    /// state, begun after it, doesn't take.
    fn estimate(&mut self, sample: Duration) {
        if self.srtt == Duration::ZERO {
            self.srtt = sample;
            self.rttvar = sample / 2;
            self.min_rtt = sample;
        } else {
            let dev = if sample > self.srtt {
                sample - self.srtt
            } else {
                self.srtt - sample
            };
            self.rttvar = (self.rttvar * 3 + dev) / 4;
            self.srtt = (self.srtt * 7 + sample) / 8;
            self.min_rtt = self.min_rtt.min(sample);
        }
    }

    /// `len` bytes go out at `t`, the path's round trip `rtt`. A flow that
    /// sat idle with nothing in flight for more than its retransmission
    /// timeout first halves its window for each timeout it sat, to the
    /// initial window at least, as Linux's `tcp_slow_start_after_idle`.
    fn sent(&mut self, t: Time, len: usize, rtt: Duration) {
        if self.in_flight == 0 && !self.last_send.is_nil() && self.srtt > Duration::ZERO {
            let rto = self.rto();
            let mut idle = t - self.last_send;
            let restart = self.cwnd.min(INIT_WINDOW);
            while idle > rto && self.cwnd > restart {
                self.cwnd /= 2.0;
                idle -= rto;
            }
            self.cwnd = self.cwnd.max(restart);
        }
        if self.in_flight == 0 && !self.epoch.is_nil() && !self.last_send.is_nil() {
            // CUBIC's epoch skips the time the flow sat idle.
            self.epoch = (self.epoch + (t - self.last_send)).min(t);
        }
        self.last_send = t;
        self.in_flight += window_bytes(len);
        self.out_bytes += len;
        if self.round.is_nil() || t - self.round >= rtt {
            self.flight_last = self.flight_now;
            self.flight_now = 0;
            self.round = t;
        }
        self.flight_now = self.flight_now.max(self.in_flight);
    }

    /// Whether the window limited the flow lately, as Linux's
    /// `tcp_is_cwnd_limited` judges: in slow start, while the window is
    /// under twice what the flow put in flight; past it, while it holds
    /// back data the connection has, or the flow filled it.
    fn limited(&self) -> bool {
        let used = self.flight_now.max(self.flight_last) as f64;
        if self.cwnd < self.ssthresh {
            self.cwnd < 2.0 * used
        } else {
            self.held || used + MSS >= self.cwnd
        }
    }

    /// `len` bytes were acknowledged at `t`, the path's base round trip
    /// `base`: slow start below the threshold, CUBIC (RFC 8312, as Linux's
    /// `bictcp_update`) above it. Its epoch starts at the first
    /// acknowledgement past slow start or a cut, from the window lost last
    /// or the window now if that is larger; the window then grows toward
    /// the cubic curve a base round trip ahead, or toward Reno's window if
    /// that grows faster, by at most half a segment for each segment
    /// acknowledged (1.5 times a round trip). Reno's window is CUBIC's
    /// TCP-friendly estimate (`tcp_friendliness`): from the window at the
    /// epoch's start, a segment for each [`BETA_SCALE`] / 8 windows of
    /// segments acknowledged while the window limits the flow (the only
    /// acknowledgements this sees), so it grows with what the flow sends,
    /// not with time.
    fn grow(&mut self, t: Time, len: usize, base: Duration) {
        let len = window_bytes(len);
        if self.cwnd < self.ssthresh {
            self.cwnd += len as f64;
            return;
        }
        let cwnd = self.cwnd / MSS;
        let segs = len as f64 / MSS;
        if self.epoch.is_nil() {
            self.epoch = t;
            self.ack_cnt = segs;
            self.tcp_cwnd = cwnd;
            if self.w_max <= cwnd {
                self.k = 0.0;
                self.origin = cwnd;
            } else {
                self.k = ((self.w_max - cwnd) / CUBIC_C).cbrt();
                self.origin = self.w_max;
            }
        } else {
            self.ack_cnt += segs;
        }
        let base = base.as_secs_f64().max(1e-3);
        let since = (t - self.epoch).as_secs_f64();
        let cubic = self.origin + CUBIC_C * (since + base - self.k).powi(3);
        let mut step = if cubic > cwnd {
            (cubic - cwnd) / cwnd
        } else {
            0.01 / cwnd
        };
        if self.w_max == 0.0 {
            step = step.max(0.05);
        }
        let delta = (cwnd.floor() * BETA_SCALE / 8.0).floor().max(1.0);
        while self.ack_cnt > delta {
            self.ack_cnt -= delta;
            self.tcp_cwnd += 1.0;
        }
        if self.tcp_cwnd > cwnd {
            step = step.max((self.tcp_cwnd - cwnd) / cwnd);
        }
        self.cwnd = (cwnd + step.min(0.5) * segs) * MSS;
    }

    /// The threshold falls to [`BETA`] of the window, CUBIC's target
    /// lowered when the window falls short of the last one (fast
    /// convergence), as `cubictcp_recalc_ssthresh` sets them.
    fn reduce(&mut self) {
        let w = self.cwnd / MSS;
        self.w_max = if w < self.w_max {
            w * (1.0 + BETA) / 2.0
        } else {
            w
        };
        self.epoch = Time::NIL;
        self.ssthresh = (self.cwnd * BETA).max(2.0 * MSS);
    }

    /// A recovery begins ([`Recovery`]).
    fn enter_recovery(&mut self) {
        let prior = self.cwnd;
        self.reduce();
        self.recovery = Some(Recovery {
            high: self.sent_end,
            prior,
            delivered: 0,
            out: 0,
            bonus: 0,
        });
    }

    /// What a write would send at once, as Linux's write pushes what the
    /// window allows before the next (`tcp_push`): the window's room in
    /// packets, with nothing unsent or to resend waiting, outside a
    /// recovery and the loss state. Left out: the window's restart after
    /// an idle spell and TSO deferral, which [`Scheduler::pump`] applies as
    /// it sends.
    fn write_room(&self) -> u64 {
        if !self.unsent.is_empty()
            || !self.resend.is_empty()
            || self.recovery.is_some()
            || self.loss_high.is_some()
        {
            return 0;
        }
        let room = (self.cwnd - self.in_flight as f64).max(0.0) as u64;
        room / MSS as u64
    }

    /// The bytes of segment `seq` still out: its tail's, if the queue took
    /// its head.
    fn outstanding(&self, seq: u64) -> usize {
        match self.tails.get(&seq) {
            Some(t) => t.tail,
            None => self.segments.get(&seq).map_or(0, |g| g.len),
        }
    }

    /// The most bytes a new segment takes (`tcp_xmit_size_goal`, TSO
    /// autosizing's `tcp_tso_autosize`): a millisecond of the pacing rate
    /// (`sk_pacing_rate >> 10`; `tcp_update_pacing_rate`, twice the window
    /// a smoothed round trip in slow start's first half and 1.2 times it
    /// past), and 64 KiB more than doubled for each 512 us the least round
    /// trip takes (`tcp_tso_rtt_log`), in whole packets, at least two
    /// (`tcp_min_tso_segs`) and at most 64 KiB's; `base` stands for the
    /// round trip before one is sampled.
    fn size_goal(&self, base: Duration) -> usize {
        const GSO_MAX: f64 = 65536.0;
        let srtt = if self.srtt > Duration::ZERO {
            self.srtt
        } else {
            base
        };
        let ratio = if self.cwnd < self.ssthresh / 2.0 {
            2.0
        } else {
            1.2
        };
        let mut bytes = ratio * self.cwnd / srtt.as_secs_f64().max(1e-6) / 1024.0;
        let r = self.min_rtt.as_micros() >> 9;
        if self.min_rtt > Duration::ZERO && r < 64 {
            bytes += GSO_MAX / 2f64.powi(r as i32);
        }
        let segs = (bytes.min(GSO_MAX) / MSS).floor().max(2.0);
        (segs * MSS) as usize
    }

    /// The sequence number of the first segment not acknowledged: the
    /// cumulative ack point.
    fn snd_una(&self) -> u64 {
        [
            self.sent_at.keys().next(),
            self.lost.keys().next(),
            self.retx.iter().next(),
        ]
        .into_iter()
        .flatten()
        .copied()
        .min()
        .unwrap_or(self.sent_end)
    }

    /// The recovery's window on an acknowledgement of `acked` bytes, which
    /// `advanced` the cumulative ack point or not and marked segments lost
    /// or not: [`Recovery`]'s rate reduction, as `tcp_cwnd_reduction`
    /// computes it, in packets (the threshold and the window before the
    /// cut in whole ones, `snd_ssthresh` and `prior_cwnd`): above the
    /// threshold the ceiling of its share of what was delivered, less what
    /// went; at least one packet before anything went (the forced fast
    /// retransmit) and never under what is in flight after.
    fn reduction(&mut self, acked: usize, advanced: bool, newly_lost: bool) {
        let Some(r) = self.recovery.as_mut() else {
            return;
        };
        if acked == 0 {
            return;
        }
        let acked = window_bytes(acked);
        r.delivered += acked;
        let packets = |b: usize| i64::try_from(b / MSS as usize).unwrap_or(i64::MAX);
        let ssthresh = (self.ssthresh / MSS + 1e-9).floor() as i64;
        let pipe = packets(self.in_flight);
        let delta = ssthresh - pipe;
        let (delivered, out) = (packets(r.delivered), packets(r.out));
        let sndcnt = if delta < 0 {
            let prior = ((r.prior / MSS + 1e-9).floor() as i64).max(1);
            (ssthresh * delivered + prior - 1) / prior - out
        } else {
            if advanced && !newly_lost {
                r.bonus += acked;
            }
            (delivered + packets(r.bonus) - out)
                .max(packets(acked))
                .min(delta)
        };
        let sndcnt = sndcnt.max(i64::from(out == 0));
        self.cwnd = ((pipe + sndcnt) as f64 * MSS).max(MSS);
    }

    /// Whether the window holds back a segment of `len` bytes: past the
    /// window, unless nothing is in flight or it is a recovery's first.
    fn holds(&self, len: usize) -> bool {
        self.in_flight > 0
            && (self.in_flight + window_bytes(len)) as f64 > self.cwnd
            && self.recovery.is_none_or(|r| r.out > 0)
    }

    /// A loss a timer showed, RACK's or the tail-loss probe's, starts a
    /// recovery at `now`, whose first step counts a packet delivered, and
    /// arms the retransmission timeout where the probe was pending
    /// (`tcp_rack_reo_timeout`'s `tcp_rearm_rto`).
    fn recover_at(&mut self, now: Time) {
        self.enter_recovery();
        self.reduction(MSS as usize, false, true);
        if self.tlp || self.timer_at.is_nil() {
            self.rearm_rto(now);
        }
    }

    /// The piece from byte `from` of segment `seq`, the leading frames the
    /// uplink queue took, was lost too: the segment is lost whole, or the
    /// resent tail it was cut from is lost whole from where that starts.
    fn head_lost(&mut self, seq: u64, from: usize) {
        if from == 0 {
            self.tails.remove(&seq);
        } else if let Some(tl) = self.tails.get_mut(&seq) {
            tl.tail += tl.head;
            tl.head = 0;
            tl.resent = false;
        }
    }

    /// A segment of `len` bytes shown lost in a recovery apart from an
    /// acknowledgement (a competing upload's, a timer's): Linux marks a loss
    /// on the acknowledgement whose rate reduction then sets the window
    /// from what remains in flight, so the loss frees no room by itself.
    fn lost_in_recovery(&mut self, len: usize) {
        if self.recovery.is_some() {
            self.cwnd = (self.cwnd - window_bytes(len) as f64).max(MSS);
        }
    }

    /// `len` bytes went out in a recovery.
    fn recovery_sent(&mut self, len: usize) {
        if let Some(r) = self.recovery.as_mut() {
            r.out += window_bytes(len);
        }
    }

    /// The end of a recovery or of the loss state once the cumulative ack
    /// passes what was sent when it began: the window is the threshold.
    fn recovered(&mut self) {
        let una = self.snd_una();
        if let Some(r) = self.recovery
            && una >= r.high
        {
            self.recovery = None;
            if self.ssthresh.is_finite() {
                self.cwnd = self.ssthresh;
            }
        }
        if self.loss_high.is_some_and(|h| una >= h) {
            self.loss_high = None;
        }
        while self.sacked.first_key_value().is_some_and(|(&s, _)| s < una) {
            self.sacked.pop_first();
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
    /// Whether one of them may be due: a walk passes over a link with
    /// neither a due record nor a new message.
    due: bool,
}

/// A delivered record's place in its link's queue until the queue
/// compacts; no record takes the id.
const DELIVERED: u64 = u64::MAX;

/// An address as an integer that orders as [`IpAddr`] does: an
/// `Ipv4Addr`'s octets compare through `memcmp`.
type IpKey = (bool, u128);

/// A new message's queue: its sender, receiver and connection addresses.
type QueueKey = (HostIx, HostIx, IpKey, IpKey);

fn ip_key(ip: IpAddr) -> IpKey {
    match ip {
        IpAddr::V4(a) => (false, u128::from(a.to_bits())),
        IpAddr::V6(a) => (true, a.to_bits()),
    }
}

/// One competing upload (see the module docs).
struct Cross {
    host: HostIx,
    /// When its client starts, and when it sends data from and to: from
    /// the setup's round trips after the start, each the path's round trip
    /// and the uplink's queue as the trip begins, for as long as it was
    /// asked (`NIL` until the setup ends).
    start: Time,
    stop: Time,
    data_at: Time,
    rtt: Duration,
    flow: Flow,
    started: bool,
    /// The setup's round trips taken.
    trips: u32,
    /// Lost segments not yet resent, by sequence number, and their frames.
    /// Bound: the segments in flight.
    resend: BTreeMap<u64, u32>,
    /// What acknowledgements the host's access link lost acknowledged (each
    /// segment's sequence number, frames and send time), which the next to
    /// arrive acknowledges too, as Linux's cumulative and selective ones
    /// cover an earlier one's, and when its tail-loss probe is queued
    /// ([`Event::CrossProbe`], `NIL` without). Bound: the segments in
    /// flight.
    acks_lost: Vec<(u64, u32, Time)>,
    probe_at: Time,
}

/// A host pair as a set key.
fn pair_key(a: HostIx, b: HostIx) -> (HostIx, HostIx) {
    (a.min(b), a.max(b))
}

/// The run's WAN schedule, shared with the hosts' sockets: a capture's
/// link probes read the model's view of their connections. The run locks
/// it between steps, the probes during a host's turn, all on turmoil's
/// thread, so it is never contended.
#[derive(Clone, Default)]
#[allow(clippy::disallowed_types)]
pub(crate) struct SharedWan(Arc<Mutex<Option<Scheduler>>>);

impl fmt::Debug for SharedWan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SharedWan")
    }
}

impl SharedWan {
    pub(crate) fn lock(&self) -> MutexGuard<'_, Option<Scheduler>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The model's view of one TCP connection and its sender's uplink: the
/// window model's congestion window, slow start threshold and bytes in
/// flight and waiting for the window (`None` without the model or the
/// connection), and the uplink's queue as the time to drain it and its
/// rate.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ConnState {
    pub cwnd: Option<u64>,
    pub ssthresh: Option<u64>,
    pub in_flight: Option<u64>,
    pub pending: Option<u64>,
    pub uplink_queue: Option<Duration>,
    pub uplink_rate: Option<u64>,
}

/// The WAN schedule of one run.
pub(crate) struct Scheduler {
    net: Network,
    names: Vec<String>,
    ip_host: FastMap<IpAddr, HostIx>,
    /// Every turmoil link, in turmoil's order (which never changes once
    /// the hosts are registered).
    links: Vec<LinkState>,
    /// The link between each pair of hosts.
    link_of: BTreeMap<(HostIx, HostIx), usize>,
    /// The links with records: a walk visits them and the links turmoil
    /// has messages in flight on (`Sim::busy_links`), no other.
    recorded: BTreeSet<usize>,
    /// Scratch for [`Scheduler::match_queues`]: the links that released a
    /// message in the walk, ascending.
    released: Vec<usize>,
    records: Records,
    /// Pending events by time, then record. A record has at most one, and
    /// an event of a forgotten record is skipped.
    events: BTreeMap<(Time, u64), Event>,
    next: u64,
    frozen: BTreeSet<(HostIx, HostIx)>,
    partitioned: BTreeSet<(HostIx, HostIx)>,
    tick: Duration,
    /// The time of the walk running, or of the last: a bin edge is read no
    /// earlier ([`Scheduler::up_counters`]).
    walked: Time,
    buffers: Option<SendBuffers>,
    /// Data segments waiting for room in their sender's uplink queue, by
    /// admission time and record: their connection and length. Bound: the
    /// records in flight.
    admits: BTreeMap<(Time, u64), ((SocketAddr, SocketAddr), usize)>,
    /// The connections whose window a walk moved, whose room for writes
    /// at once ([`Flow::write_room`]) the send buffers then learn. Bound:
    /// the connections pumped or acknowledged in one walk.
    room_moved: Vec<(SocketAddr, SocketAddr)>,
    /// The window model's connections. Bound: one per TCP connection that
    /// sent data.
    flows: FastMap<(SocketAddr, SocketAddr), Flow>,
    /// Scratch for [`Scheduler::pump`].
    sending: Vec<((SocketAddr, SocketAddr), u64)>,
    /// Per host, its uplink queue's wait every `queue_every`: (time since
    /// the start, wait); and its bytes then, the wait at the uplink's rate.
    up_queue: Vec<Vec<(Duration, Duration)>>,
    up_backlog: Vec<Vec<(Duration, u64)>>,
    queue_every: Duration,
    /// The window model's data records not yet through to their receiver,
    /// and how many of their bytes the segments holding them have still
    /// to bring. Bound: the records in flight.
    arriving: FastMap<u64, usize>,
    /// The window model's TCP events, when traced ([`Scheduler::trace_tcp`]).
    /// Bound: the run's events; only the bench traces, for one run.
    tcp_trace: Option<Vec<TcpEvent>>,
    next_queue_sample: Time,
    next_cross_sample: Time,
    /// The competing uploads, in host order.
    cross: Vec<Cross>,
    /// Per host, the wire bytes its competing uploads have sent.
    cross_bytes: Vec<u64>,
    /// Per host with competing uploads, [`Scheduler::cross_bytes`] every
    /// [`QUEUE_SAMPLE_EVERY`]: (time since the start, bytes).
    cross_sent: Vec<Vec<(Duration, u64)>>,
    /// Per host with competing uploads, [`Scheduler::cross_bytes`] after
    /// each walk that moved it, stamped with the walk's time: what they had
    /// sent before a time is the last entry stamped before it, the same
    /// during the run as after it (a host's task at a time runs before
    /// that time's walk). Bound: one per walk while they run.
    cross_log: Vec<Vec<(Duration, u64)>>,
    /// Per host with competing uploads, their windows summed, sampled as
    /// [`Scheduler::cross_sent`] is.
    cross_cwnd: Vec<Vec<(Duration, u64)>>,
    /// Per host, the window model's segments its connections lost to a
    /// full access queue and their packets, the acknowledgements its uplink
    /// carried (pure ACKs and SACKs alone) with their wire bytes, and the
    /// SACKs among them with theirs.
    drops: Vec<u64>,
    drop_packets: Vec<u64>,
    acks: Vec<u64>,
    ack_bytes: Vec<u64>,
    sacks: Vec<u64>,
    sack_bytes: Vec<u64>,
    /// Per host, what its uplink carried of its pure ACKs and of its SACKs
    /// alone, each from when it left ([`Scheduler::up_counters`]).
    ack_out: Vec<Departures>,
    sack_out: Vec<Departures>,
    /// Per host, its resends of stream data its path lost, each from when
    /// it left (packets: one a resend; [`Scheduler::up_counters`]).
    retx_out: Vec<Departures>,
    /// Per host, the packets it lost, each from when it was lost
    /// ([`Scheduler::count_lost`]).
    drop_out: Vec<Departures>,
    /// Per host, the bytes it handed its transports that went out, resends
    /// left out: its wire bytes less these are the transports' framing and
    /// resends ([`Scheduler::sent`]).
    app_bytes: Vec<u64>,
    /// Per host, what its link charged the window model's resends of its
    /// own segments and chunks (a resent chunk's share of its packet).
    resent_bytes: Vec<u64>,
    /// Per host, its competing uploads' segments lost to its full uplink.
    cross_drops: Vec<u64>,
    /// Per host, the frames of those segments the queue dropped.
    cross_drop_packets: Vec<u64>,
    /// Per host, the window model's falls of its connections' windows on a
    /// loss (one per recovery), and the timeouts among them that restarted
    /// a window from a segment.
    cuts: Vec<u64>,
    rtos: Vec<u64>,
    /// The acknowledgements receivers owe, by the connection whose data
    /// they acknowledge. Bound: one per connection with data received.
    owed: FastMap<(SocketAddr, SocketAddr), Owed>,
    /// Per host, the acknowledgements its receivers sent by why
    /// ([`AckWhy`]), and its delayed ones' waits.
    ack_whys: Vec<[u64; ACK_WHYS.len()]>,
    ack_waits: Vec<AckWaits>,
    /// The data channels' senders and receivers, by the connection that
    /// carries the channel's data. Bound: one per channel opened.
    assocs: FastMap<(SocketAddr, SocketAddr), sctp::Assoc>,
    rcvs: FastMap<(SocketAddr, SocketAddr), sctp::Rcv>,
    /// Per host, when its network thread is done with what its
    /// associations sent so far ([`sctp`]'s module docs).
    threads: Vec<Time>,
    /// Per host, whether it is down, from its crash
    /// ([`Scheduler::crashed`]) to its restart ([`Scheduler::restarted`]).
    down: Vec<bool>,
    /// The answers of the stacks whose page counts a channel's amount
    /// ([`sctp::SctpProfile::amount_at_sends`]), by time and order: the
    /// channel's connection. Bound: the channels' send opportunities in
    /// flight on their threads.
    answers: BTreeMap<(Time, u64), (SocketAddr, SocketAddr)>,
    /// Per host, whether its packet log is on ([`Scenario::packet_log`]),
    /// and the packets it offered its uplink so far
    /// ([`WanHostReport::packets`]). Bound: the run's packets of the hosts
    /// the log names.
    packet_log: Vec<bool>,
    packets: Vec<Vec<PacketRecord>>,
    /// The logged hosts' links, the ordinal of each among its host's to
    /// its peer in the order the host first sent on them, by (host, peer,
    /// its addresses low first), and how many each host has to each peer.
    /// Bound: the logged hosts' connections.
    link_ords: BTreeMap<(HostIx, HostIx, (SocketAddr, SocketAddr)), u32>,
    link_counts: BTreeMap<(HostIx, HostIx), u32>,
    /// Per host, whether its associations' log is on
    /// ([`Scenario::sctp_log`]), and their events since the run last took
    /// them ([`Scheduler::take_sctp`], at each bin edge), by peer and link
    /// ([`WanHostReport::sctp`]). Bound: a bin's packets of the logged
    /// hosts' associations.
    sctp_log: Vec<bool>,
    sctp_events: Vec<SctpAssocs>,
    /// With [`Scenario::node_bins`], every host's offers of data to a
    /// swarm's host ([`WanHostReport::offers`]), as a testbed run counts
    /// its nodes': data to the signalling relay left out. Bound: the run's
    /// data messages.
    offers: Option<Vec<Vec<(Duration, u64, u32)>>>,
    /// Per host, whether it is a swarm's (its seed or a viewer).
    swarm_hosts: Vec<bool>,
    /// Per host, whether its WebRTC connections are the window model's
    /// data channels ([`HostSpec::rtc_stack`](crate::spec::HostSpec::rtc_stack)),
    /// whose listener on [`UDP_PORT`] takes the ICE checks' dials.
    rtc_hosts: Vec<bool>,
}

/// A message handed to a host's uplink, as its packet log and offers take
/// it ([`Scheduler::offered`]), right after the uplink took what it did.
struct Offered {
    at: Time,
    from: HostIx,
    to: Option<HostIx>,
    conn: Option<(SocketAddr, SocketAddr)>,
    proto: PacketProto,
    kind: PacketKind,
    /// Its IP bytes and packets ([`strims_wan::framing::frames`]), and the
    /// leading packets the uplink's queue took: the rest it dropped.
    wire: usize,
    packets: u32,
    took: u32,
    /// The offset of its first byte in its connection's stream, and
    /// whether it resent bytes its connection had sent; `None` for a
    /// datagram.
    off: Option<u64>,
    resent: Option<bool>,
    /// What the uplink held as it came ([`Network::up_held`]).
    held: usize,
    /// When what it took left the uplink and reached its receiver
    /// ([`Transit`](strims_wan::Transit)'s `depart` and `arrival`), the path's delay after each
    /// packet; `None` without a path the model times.
    transit: Option<(Time, Time)>,
    /// The first of its packets the path lost, the rest with it.
    path_lost: Option<u32>,
}

/// How often the scheduler samples each host's uplink queue, by default,
/// and its competing uploads.
pub(crate) const QUEUE_SAMPLE_EVERY: Duration = Duration::from_millis(100);

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
        let mut net = Network::new(&sites, options, &mut det_rng(seed ^ 0x5741_4e00));
        for (h, hs) in spec.hosts.iter().enumerate() {
            if !hs.up_steps.is_empty() {
                let steps = hs.up_steps.iter().map(|&(at, rate)| (sim_time(at), rate));
                net.set_up_steps(h, steps.collect());
            }
            if let Some(cap) = &hs.up_capacity {
                if !hs.up_steps.is_empty() {
                    return Err(format!(
                        "wan: host {} has a capacity schedule, which takes no rate steps",
                        hs.name
                    ));
                }
                net.set_up_capacity(h, cap.clone());
            }
        }
        let cross: Vec<Cross> = spec
            .hosts
            .iter()
            .enumerate()
            .flat_map(|(h, hs)| {
                hs.competing.iter().map(move |c| Cross {
                    host: h,
                    start: sim_time(c.start),
                    stop: sim_time(c.stop),
                    data_at: Time::NIL,
                    rtt: Duration::from(c.rtt),
                    flow: Flow::default(),
                    started: false,
                    trips: 0,
                    resend: BTreeMap::new(),
                    acks_lost: Vec::new(),
                    probe_at: Time::NIL,
                })
            })
            .collect();
        let windowed = buffers.as_ref().is_some_and(|b| b.model().window);
        if !cross.is_empty() && !windowed {
            return Err("wan: competing uploads need the window model".to_owned());
        }
        // Each competing upload's client starts at its start.
        let events: BTreeMap<(Time, u64), Event> = cross
            .iter()
            .enumerate()
            .map(|(i, c)| ((c.start, i as u64), Event::CrossTrip { i }))
            .collect();
        let names: Vec<String> = spec.hosts.iter().map(|h| h.name.clone()).collect();
        let ip_host: FastMap<IpAddr, HostIx> = names
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
                    due: false,
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
            recorded: BTreeSet::new(),
            released: Vec::new(),
            records: Records::default(),
            next: events.len() as u64,
            events,
            frozen: BTreeSet::new(),
            partitioned: BTreeSet::new(),
            tick: Duration::from(spec.tick),
            walked: Time::NIL,
            buffers,
            admits: BTreeMap::new(),
            room_moved: Vec::new(),
            flows: FastMap::default(),
            arriving: FastMap::default(),
            sending: Vec::new(),
            up_queue: vec![Vec::new(); spec.hosts.len()],
            up_backlog: vec![Vec::new(); spec.hosts.len()],
            queue_every: QUEUE_SAMPLE_EVERY,
            tcp_trace: None,
            next_queue_sample: epoch(),
            next_cross_sample: epoch(),
            cross,
            cross_bytes: vec![0; spec.hosts.len()],
            cross_sent: vec![Vec::new(); spec.hosts.len()],
            cross_log: vec![Vec::new(); spec.hosts.len()],
            cross_cwnd: vec![Vec::new(); spec.hosts.len()],
            drops: vec![0; spec.hosts.len()],
            drop_packets: vec![0; spec.hosts.len()],
            acks: vec![0; spec.hosts.len()],
            ack_bytes: vec![0; spec.hosts.len()],
            sacks: vec![0; spec.hosts.len()],
            sack_bytes: vec![0; spec.hosts.len()],
            ack_out: vec![Departures::default(); spec.hosts.len()],
            sack_out: vec![Departures::default(); spec.hosts.len()],
            retx_out: vec![Departures::default(); spec.hosts.len()],
            drop_out: vec![Departures::default(); spec.hosts.len()],
            app_bytes: vec![0; spec.hosts.len()],
            resent_bytes: vec![0; spec.hosts.len()],
            cross_drops: vec![0; spec.hosts.len()],
            cross_drop_packets: vec![0; spec.hosts.len()],
            cuts: vec![0; spec.hosts.len()],
            rtos: vec![0; spec.hosts.len()],
            owed: FastMap::default(),
            ack_whys: vec![[0; ACK_WHYS.len()]; spec.hosts.len()],
            ack_waits: vec![AckWaits::default(); spec.hosts.len()],
            assocs: FastMap::default(),
            rcvs: FastMap::default(),
            threads: vec![Time::NIL; spec.hosts.len()],
            down: vec![false; spec.hosts.len()],
            answers: BTreeMap::new(),
            packet_log: (0..spec.hosts.len())
                .map(|h| spec.packet_log.contains(&h))
                .collect(),
            packets: vec![Vec::new(); spec.hosts.len()],
            link_ords: BTreeMap::new(),
            link_counts: BTreeMap::new(),
            sctp_log: (0..spec.hosts.len())
                .map(|h| spec.sctp_log.as_ref().is_some_and(|s| s.contains(spec, h)))
                .collect(),
            sctp_events: vec![BTreeMap::new(); spec.hosts.len()],
            offers: spec.node_bins.then(|| vec![Vec::new(); spec.hosts.len()]),
            swarm_hosts: (0..spec.hosts.len())
                .map(|h| {
                    spec.swarms
                        .iter()
                        .any(|w| w.seed == h || w.viewers.contains(&h))
                })
                .collect(),
            rtc_hosts: spec
                .hosts
                .iter()
                .map(|h| spec.window_model() && h.webrtc && h.rtc_stack.is_some())
                .collect(),
        })
    }

    /// Whether host `h`'s packet log is on.
    fn logs(&self, h: HostIx) -> bool {
        self.packet_log.get(h).copied().unwrap_or(false)
    }

    /// Whether host `h`'s associations' log is on.
    fn sctp_logs(&self, h: HostIx) -> bool {
        self.sctp_log.get(h).copied().unwrap_or(false)
    }

    /// The associations' events logged since the last take, by host, in
    /// the order logged (in-packets' at their arrival, so not in time
    /// order).
    pub(crate) fn take_sctp(&mut self) -> Vec<SctpAssocs> {
        self.sctp_events.iter_mut().map(std::mem::take).collect()
    }

    /// Host `h`'s association with `peer` over `conn` (either way) logs
    /// event `e`.
    fn sctp_event(&mut self, h: HostIx, peer: HostIx, conn: ConnKey, e: SctpEvent) {
        let link = self.link_ord(h, peer, conn);
        if let Some(m) = self.sctp_events.get_mut(h) {
            m.entry((peer, link)).or_default().push(e);
        }
    }

    /// The ordinal of host `h`'s link to `peer` over `conn` (either way),
    /// among its links to `peer` in the order its logs first met each.
    fn link_ord(&mut self, h: HostIx, peer: HostIx, (a, b): ConnKey) -> u32 {
        let key = (h, peer, (a.min(b), a.max(b)));
        let next = self.link_counts.get(&(h, peer)).copied().unwrap_or(0);
        let ord = *self.link_ords.entry(key).or_insert(next);
        if ord == next {
            self.link_counts.insert((h, peer), next + 1);
        }
        ord
    }

    /// What host `h`'s uplink holds at `at` where its packet log is on,
    /// for an [`Offered`]; 0 otherwise, which nothing reads.
    fn held(&self, h: HostIx, at: Time) -> usize {
        if self.logs(h) {
            self.net.up_held(h, at)
        } else {
            0
        }
    }

    /// Message `o` handed to its host's uplink, called right after the
    /// uplink took what it did (its packets' departures are the uplink's
    /// last, [`Network::up_last_departures`]): its data to a swarm's host
    /// among its host's offers, and with its host's packet log on, each of
    /// its packets a row. Reads the model and moves nothing.
    fn offered(&mut self, o: &Offered) {
        let data = matches!(o.kind, PacketKind::Data | PacketKind::DataSack);
        let to_swarm =
            o.to.is_some_and(|to| self.swarm_hosts.get(to).copied().unwrap_or(false));
        if data
            && to_swarm
            && let Some(v) = self.offers.as_mut().and_then(|x| x.get_mut(o.from))
        {
            v.push((o.at - epoch(), o.wire as u64, o.packets.max(1)));
        }
        if !self.logs(o.from) {
            return;
        }
        let link = match (o.to, o.conn) {
            (Some(to), Some(conn)) => Some(self.link_ord(o.from, to, conn)),
            _ => None,
        };
        let framing = self.net.up_framing(o.from);
        let departs = self.net.up_last_departures(o.from);
        let Some(rows) = self.packets.get_mut(o.from) else {
            return;
        };
        let (mut held, mut off) = (o.held, o.off);
        let at = o.at - epoch();
        for (k, ip) in strims_wan::framing::frames(o.wire, o.packets).enumerate() {
            let k = u32::try_from(k).unwrap_or(u32::MAX);
            let charge = framing.packet(ip);
            let sent = if k < o.took {
                departs.get(k as usize).copied()
            } else {
                None
            };
            let lost = if k >= o.took {
                Some(PacketLoss::Queue)
            } else if o.path_lost.is_some_and(|f| k >= f) {
                Some(PacketLoss::Path)
            } else {
                None
            };
            let arrived = match (sent, lost, o.transit) {
                (Some(s), None, Some((depart, arrival))) => Some(s + (arrival - depart) - epoch()),
                _ => None,
            };
            let payload = match o.proto {
                PacketProto::Tcp => ip.saturating_sub(strims_wan::wire::TCP_HEADER),
                _ => 0,
            } as u64;
            rows.push(PacketRecord {
                at,
                sent: sent.map(|s| s - epoch()),
                arrived,
                lost,
                peer: o.to,
                link,
                proto: o.proto,
                ip_bytes: u32::try_from(ip).unwrap_or(u32::MAX),
                charged_bytes: Some(u32::try_from(charge).unwrap_or(u32::MAX)),
                kind: o.kind,
                range: off.filter(|_| payload > 0).map(|lo| (lo, lo + payload)),
                resent: o.resent,
                estimated: false,
                backlog_bytes: Some(held as u64),
            });
            if sent.is_some_and(|s| s > o.at) {
                held += charge;
            }
            off = off.map(|x| x + payload);
        }
    }

    fn name(&self, h: HostIx) -> &str {
        self.names.get(h).map_or("?", String::as_str)
    }

    /// The model's view at `now` of the connection whose sender's address
    /// pair is `conn`.
    pub(crate) fn conn_state(&self, conn: (SocketAddr, SocketAddr), now: Time) -> ConnState {
        let host = self.ip_host.get(&conn.0.ip()).copied();
        if let Some(a) = self.assocs.get(&conn) {
            let pending = a
                .pending()
                .filter_map(|id| match self.records.get(&id)?.what {
                    What::Data { len, .. } => Some(len as u64),
                    _ => None,
                })
                .sum();
            return ConnState {
                cwnd: Some(a.cwnd() as u64),
                ssthresh: Some(a.ssthresh() as u64),
                in_flight: Some(a.in_flight() as u64),
                pending: Some(pending),
                uplink_queue: host.map(|h| (self.net.up_free(h) - now).max(Duration::ZERO)),
                uplink_rate: host.map(|h| self.net.up_rate(h, now)),
            };
        }
        let flow = self.flows.get(&conn);
        let pending = flow.map(|f| {
            let unsent: usize = f.unsent.iter().map(|&(_, n)| n).sum();
            let resend: usize = f.resend.iter().map(|&q| f.outstanding(q)).sum();
            (unsent + resend) as u64
        });
        let host = self.ip_host.get(&conn.0.ip()).copied();
        ConnState {
            cwnd: flow.map(|f| f.cwnd as u64),
            ssthresh: flow.and_then(|f| f.ssthresh.is_finite().then_some(f.ssthresh as u64)),
            in_flight: flow.map(|f| f.out_bytes as u64),
            pending,
            uplink_queue: host.map(|h| (self.net.up_free(h) - now).max(Duration::ZERO)),
            uplink_rate: host.map(|h| self.net.up_rate(h, now)),
        }
    }

    /// One walk, between two steps (see the module docs).
    pub fn walk(&mut self, sim: &Sim<'_>) -> Result<(), String> {
        let now = sim_time(sim.elapsed());
        self.walked = now;
        // No edge is read before the walk's time ([`Scheduler::settled`]).
        self.net.settle(now);
        // The step's writes, at its start, go first: every event of the walk
        // comes after it (the last walk ran those before), so each uplink
        // takes them before what the events hand it, and what they cause
        // within the step runs in the walk.
        let fresh = self.match_queues(sim, false)?;
        self.send(now - self.tick, fresh);
        self.run_events(now);
        self.match_queues(sim, true)?;
        self.purge(sim);
        self.admit(now);
        self.tell_rooms();
        if now >= self.next_queue_sample {
            self.next_queue_sample = now + self.queue_every;
            for (h, (samples, bytes)) in self
                .up_queue
                .iter_mut()
                .zip(self.up_backlog.iter_mut())
                .enumerate()
            {
                let wait = (self.net.up_free(h) - now).max(Duration::ZERO);
                samples.push((now - epoch(), wait));
                let rate = i128::from(self.net.up_rate(h, now));
                let b = i128::from(wait.as_nanos()) * rate / 1_000_000_000;
                bytes.push((now - epoch(), u64::try_from(b).unwrap_or(u64::MAX)));
            }
        }
        if now >= self.next_cross_sample {
            self.next_cross_sample = now + QUEUE_SAMPLE_EVERY;
            for c in &self.cross {
                let (Some(samples), Some(&bytes)) = (
                    self.cross_sent.get_mut(c.host),
                    self.cross_bytes.get(c.host),
                ) else {
                    continue;
                };
                if samples.last().is_none_or(|&(t, _)| t < now - epoch()) {
                    samples.push((now - epoch(), bytes));
                    let window = self
                        .cross
                        .iter()
                        .filter(|x| x.host == c.host && x.started && now < x.stop)
                        .map(|x| x.flow.cwnd as u64)
                        .sum();
                    if let Some(w) = self.cross_cwnd.get_mut(c.host) {
                        w.push((now - epoch(), window));
                    }
                }
            }
        }
        for c in &self.cross {
            let (Some(log), Some(&bytes)) =
                (self.cross_log.get_mut(c.host), self.cross_bytes.get(c.host))
            else {
                continue;
            };
            if log.last().map_or(0, |&(_, b)| b) != bytes {
                log.push((now - epoch(), bytes));
            }
        }
        Ok(())
    }

    /// Host `h`'s competing uploads' log ([`Scheduler::cross_log`]).
    pub(crate) fn competing_log(&self, h: HostIx) -> &[(Duration, u64)] {
        self.cross_log.get(h).map_or(&[][..], Vec::as_slice)
    }

    /// What host `h`'s transports took of its messages that went to its
    /// uplink so far ([`Scheduler::app_bytes`]).
    pub(crate) fn own_app_bytes(&self, h: HostIx) -> u64 {
        self.app_bytes.get(h).copied().unwrap_or(0)
    }

    /// What host `h`'s link has charged its resends so far
    /// ([`Scheduler::resent_bytes`]).
    pub(crate) fn own_resent_bytes(&self, h: HostIx) -> u64 {
        self.resent_bytes.get(h).copied().unwrap_or(0)
    }

    /// What host `h`'s uplink has charged its own messages so far, its
    /// pure acknowledgements and competing uploads apart
    /// ([`EndStats::up_bytes`](strims_wan::network::EndStats::up_bytes)).
    pub(crate) fn own_up_bytes(&self, h: HostIx) -> u64 {
        self.net.stats(h).up_bytes
    }

    /// Tells the send buffers what the connections whose window the walk
    /// moved let out of their next writes at once.
    fn tell_rooms(&mut self) {
        let Some(b) = &self.buffers else {
            return;
        };
        if self.room_moved.is_empty() {
            return;
        }
        let mut moved = std::mem::take(&mut self.room_moved);
        moved.sort_unstable();
        moved.dedup();
        let rooms: Vec<_> = moved
            .iter()
            .filter_map(|c| Some((*c, self.flows.get(c)?.write_room())))
            .collect();
        b.set_rooms(&rooms);
        moved.clear();
        self.room_moved = moved;
    }

    /// Credits the segments admitted by `now` to their sockets, and hands
    /// the channels' pages the answers their stacks gave by then, in time
    /// order (an answer after the segments admitted at its time), and wakes
    /// the writers they free.
    fn admit(&mut self, now: Time) {
        let Some(b) = &self.buffers else {
            return;
        };
        loop {
            let admit = self.admits.first_key_value().map(|(&(t, _), _)| t);
            let answer = self.answers.first_key_value().map(|(&(t, _), _)| t);
            let woke = match (admit, answer) {
                (Some(t), a) if t <= now && a.is_none_or(|a| t <= a) => {
                    let Some(((at, _), (conn, len))) = self.admits.pop_first() else {
                        break;
                    };
                    b.admit(conn, len, at)
                }
                (_, Some(t)) if t <= now => {
                    let Some(((at, _), conn)) = self.answers.pop_first() else {
                        break;
                    };
                    b.answered(conn, at)
                }
                _ => break,
            };
            if let Some(w) = woke {
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
            match event {
                Event::Ack { conn, pieces } => {
                    // Linux takes an acknowledgement whole (`tcp_ack`), then
                    // sends what the window has room for.
                    for (seq, len) in pieces {
                        self.acked(t, conn, seq, len);
                    }
                    self.pump(t, &[conn]);
                    continue;
                }
                Event::Segment {
                    conn,
                    to,
                    seq,
                    from,
                    len,
                    packets,
                    wire,
                    whole,
                    through,
                } => {
                    if through {
                        self.segment_through(conn, seq, from, len);
                        if whole {
                            self.owe_ack(t, conn, (id, seq, len, packets));
                        } else {
                            self.owe_ack_head(t, conn, (id, seq, len, packets));
                        }
                        continue;
                    }
                    let rate = self.net.site(to).map_or(u64::MAX, |s| s.profile.down);
                    if self.window().is_some_and(|cap| {
                        overflows(self.net.down_free(to) - t, tx_time(wire, rate), cap)
                    }) {
                        let sender = self.ip_host.get(&conn.0.ip()).copied();
                        if let Some(d) = sender.and_then(|h| self.drops.get_mut(h)) {
                            *d += 1;
                        }
                        if let Some(h) = sender {
                            self.queue_lost(h, t, u64::from(packets));
                        }
                        if whole {
                            self.dropped(t, conn, seq);
                        } else if let Some(f) = self.flows.get_mut(&conn) {
                            // A head lost too, its tail already: the segment
                            // is lost whole, or a resent tail cut again is
                            // lost whole from where it starts.
                            f.head_lost(seq, from);
                        }
                        continue;
                    }
                    let done = self.net.receive(t, to, wire);
                    let key = self.next;
                    self.next += 1;
                    self.events.insert(
                        (done.max(t), key),
                        Event::Segment {
                            conn,
                            to,
                            seq,
                            from,
                            len,
                            packets,
                            wire,
                            whole,
                            through: true,
                        },
                    );
                    continue;
                }
                Event::Lost { conn, seq } | Event::Timeout { conn, seq } => {
                    let timeout = matches!(event, Event::Timeout { .. });
                    let from = self.ip_host.get(&conn.0.ip()).copied();
                    let Some(f) = self.flows.get_mut(&conn) else {
                        continue;
                    };
                    // Unless a later segment's acknowledgement told first.
                    if f.lost.get(&seq).is_none_or(|&(_, due)| due != Some(t)) {
                        continue;
                    }
                    f.lost.remove(&seq);
                    let len = f.outstanding(seq);
                    f.in_flight = f.in_flight.saturating_sub(window_bytes(len));
                    f.out_bytes = f.out_bytes.saturating_sub(len);
                    f.sent_at.remove(&seq);
                    f.retx.insert(seq);
                    f.resend.insert(seq);
                    if timeout {
                        if f.timed_out != t {
                            // Linux's loss state (`tcp_enter_loss`): the
                            // threshold falls unless a recovery or loss
                            // state already cut it for what is out, and the
                            // timeout backs off, once for the segments it
                            // finds lost together. CUBIC keeps its state
                            // (Linux's `bictcp_reset` would clear it), and a
                            // lost retransmission in the loss state stays in
                            // it (`FLAG_LOST_RETRANS` would start a
                            // recovery); no F-RTO.
                            if f.recovery.is_none() && f.loss_high.is_none() {
                                f.reduce();
                                if let Some(n) = from.and_then(|h| self.cuts.get_mut(h)) {
                                    *n += 1;
                                }
                            }
                            f.recovery = None;
                            f.loss_high = Some(f.sent_end);
                            f.backoff = f.backoff.saturating_add(1);
                            f.timed_out = t;
                            if let Some(n) = from.and_then(|h| self.rtos.get_mut(h)) {
                                *n += 1;
                            }
                        }
                        // Its window is what is still in flight and a
                        // packet (`tcp_enter_loss`, the segments the
                        // timeout found lost out of the flight).
                        f.cwnd = f.in_flight as f64 + MSS;
                    } else if f.recovery.is_none() && f.loss_high.is_none() {
                        // A loss shown by RACK's timer or a tail-loss probe
                        // starts a recovery, whose first step counts a
                        // packet delivered (`tcp_rack_reo_timeout`).
                        f.recover_at(t);
                        if let Some(n) = from.and_then(|h| self.cuts.get_mut(h)) {
                            *n += 1;
                        }
                        self.traced(t, conn, "rec", seq, 0);
                    } else {
                        f.lost_in_recovery(len);
                    }
                    if self.tcp_trace.is_some() {
                        self.traced(t, conn, if timeout { "rto" } else { "lost" }, seq, len);
                    }
                    self.pump(t, &[conn]);
                    continue;
                }
                Event::Probe { conn } => {
                    self.probe(t, conn);
                    continue;
                }
                Event::Probed { conn } => {
                    if let Some(a) = self.owed.get_mut(&conn).and_then(|o| o.probed(t)) {
                        self.send_ack(t, conn, a);
                    }
                    continue;
                }
                Event::DelAck { conn } => {
                    let rto = self.rcv_clock(conn).rto;
                    if let Some(a) = self.owed.get_mut(&conn).and_then(|o| o.timer(t, rto)) {
                        self.send_ack(t, conn, a);
                    }
                    continue;
                }
                Event::CrossAck {
                    i,
                    seq,
                    frames,
                    rtt,
                } => {
                    self.cross_ack_arrives(t, i, seq, frames, rtt);
                    continue;
                }
                Event::CrossLost { i, seq, frames } => {
                    self.cross_lost(t, i, seq, frames);
                    continue;
                }
                Event::CrossProbe { i } => {
                    self.cross_probe(t, i);
                    continue;
                }
                Event::CrossTrip { i } => {
                    self.cross_trip(t, i);
                    continue;
                }
                Event::SctpSack { conn } => {
                    self.sctp_sacked(t, conn, id);
                    continue;
                }
                Event::SctpTimeout { conn } => {
                    self.sctp_timeout(t, conn);
                    continue;
                }
                Event::SctpDelSack { conn } => {
                    self.sctp_delayed_sack(t, conn);
                    continue;
                }
                Event::SctpCopy {
                    conn,
                    seq,
                    wire,
                    last,
                    immediate,
                    ..
                } => {
                    self.sctp_copy_arrived(t, conn, seq, wire, (last, immediate));
                    continue;
                }
                Event::SctpThrough {
                    conn,
                    seq,
                    last,
                    immediate,
                } => {
                    self.sctp_received(t, conn, seq, (last, immediate));
                    continue;
                }
                Event::SctpOut(hand) => {
                    self.handed(t, *hand);
                    continue;
                }
                Event::SynAck => {
                    self.syn_ack(t, id);
                    continue;
                }
                Event::SynAcked => {
                    self.syn_acked(t, id);
                    continue;
                }
                _ => {}
            }
            let window = self.window();
            let Some(r) = self.records.get_mut(&id) else {
                continue;
            };
            match event {
                Event::Arrive => {
                    let data = match r.what {
                        What::Data { seq, len } => Some((seq, len)),
                        _ => None,
                    };
                    let datagram = matches!(r.what, What::Datagram(_));
                    let conn = (r.src, r.dst);
                    if let (Some((seq, _)), true) = (data, self.assocs.contains_key(&conn)) {
                        let (from, to, wire) = (r.from, r.to, r.wire);
                        let rate = self.net.site(to).map_or(u64::MAX, |s| s.profile.down);
                        if window.is_some_and(|cap| {
                            overflows(self.net.down_free(to) - t, tx_time(wire, rate), cap)
                        }) {
                            if let Some(d) = self.drops.get_mut(from) {
                                *d += 1;
                            }
                            let packets = u64::from(r.packets);
                            self.queue_lost(from, t, packets);
                            self.sctp_lost(conn, seq);
                            continue;
                        }
                        let done = self.net.receive(t, to, wire);
                        if let Some(r) = self.records.get_mut(&id) {
                            if done <= now {
                                r.state = State::Due;
                                if let Some(ls) = self.links.get_mut(r.link) {
                                    ls.due = true;
                                }
                            } else {
                                self.events.insert((done, id), Event::Deliver);
                            }
                        }
                        self.sctp_arrived(done.max(t), conn, seq);
                        continue;
                    }
                    if let (Some(cap), true) = (window, datagram && !r.nominal)
                        && overflows(
                            self.net.down_free(r.to) - t,
                            tx_time(
                                r.wire,
                                self.net.site(r.to).map_or(u64::MAX, |s| s.profile.down),
                            ),
                            cap,
                        )
                    {
                        if let Some(d) = self.drops.get_mut(r.from) {
                            *d += 1;
                        }
                        let (from, packets) = (r.from, u64::from(r.packets));
                        r.state = State::Dropped;
                        self.records.dropped += 1;
                        self.queue_lost(from, t, packets);
                        continue;
                    }
                    let done = self.net.receive(t, r.to, r.wire);
                    if r.what == What::Syn {
                        self.events.insert((done.max(t), id), Event::SynAck);
                    } else if done <= now {
                        r.state = State::Due;
                        if let Some(ls) = self.links.get_mut(r.link) {
                            ls.due = true;
                        }
                    } else {
                        self.events.insert((done.max(t), id), Event::Deliver);
                    }
                }
                Event::Deliver => {
                    r.state = State::Due;
                    if let Some(ls) = self.links.get_mut(r.link) {
                        ls.due = true;
                    }
                }
                Event::SynAck
                | Event::SynAcked
                | Event::Ack { .. }
                | Event::Segment { .. }
                | Event::Lost { .. }
                | Event::Timeout { .. }
                | Event::Probe { .. }
                | Event::Probed { .. }
                | Event::DelAck { .. }
                | Event::CrossAck { .. }
                | Event::CrossLost { .. }
                | Event::CrossProbe { .. }
                | Event::CrossTrip { .. }
                | Event::SctpSack { .. }
                | Event::SctpTimeout { .. }
                | Event::SctpDelSack { .. }
                | Event::SctpCopy { .. }
                | Event::SctpThrough { .. }
                | Event::SctpOut(_) => {}
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

    /// The `len` bytes at `seq` of `conn` were acknowledged at `t`, as
    /// Linux's `tcp_ack` takes an acknowledgement: what it delivered, the
    /// end of a recovery it completes, the losses RACK marks, a recovery
    /// they start, then the window: a recovery's rate reduction, or
    /// outside one its growth.
    fn acked(&mut self, t: Time, conn: (SocketAddr, SocketAddr), seq: u64, len: usize) {
        self.room_moved.push(conn);
        let base = self.conn_rtt(conn);
        let Some(f) = self.flows.get_mut(&conn) else {
            return;
        };
        let una = f.snd_una();
        let was_recovering = f.recovery.is_some();
        let sent = f.sent_at.remove(&seq);
        if let Some(sent) = sent {
            f.sampled(seq, t - sent);
        }
        f.in_flight = f.in_flight.saturating_sub(window_bytes(len));
        f.out_bytes = f.out_bytes.saturating_sub(len);
        if let Some(g) = f.segments.get_mut(&seq) {
            g.acked += len;
            if g.acked >= g.len {
                f.segments.remove(&seq);
                f.path_lost.remove(&seq);
            }
        }
        // A head's acknowledgement leaves its tail out; the tail's ends it.
        let head = match f.tails.get_mut(&seq) {
            Some(tl) if tl.head > 0 => {
                tl.head = tl.head.saturating_sub(len);
                true
            }
            Some(tl) => {
                tl.tail = tl.tail.saturating_sub(len);
                if tl.tail == 0 {
                    f.tails.remove(&seq);
                }
                false
            }
            None => false,
        };
        if !head && !f.retx.remove(&seq) {
            f.backoff = 0;
        }
        let advanced = f.snd_una() > una;
        if seq > una {
            *f.sacked.entry(seq).or_insert(0) += window_bytes(len) / MSS as usize;
        }
        f.recovered();
        // RACK: the acknowledgement of a segment sent later shows the
        // earlier ones lost, at once in a recovery or once three packets
        // past the gap are acknowledged (`tcp_rack_reo_wnd`), else a
        // quarter of the least round trip later.
        let mut marked = Vec::new();
        if let Some(sent) = sent {
            let earlier: Vec<u64> = f
                .lost
                .iter()
                .filter(|(_, v)| v.0 < sent)
                .map(|(&s, _)| s)
                .collect();
            let now = f.recovery.is_some() || f.sacked.values().sum::<usize>() >= 3;
            let reo = f.min_rtt / 4;
            for s in earlier {
                if now {
                    if f.lost.remove(&s).is_some() {
                        marked.push(s);
                    }
                } else if let Some(v) = f.lost.get_mut(&s)
                    && v.1.is_none_or(|due| due > t + reo)
                {
                    // It stays pending until then; a later acknowledgement
                    // may mark it first.
                    v.1 = Some(t + reo);
                    let key = self.next;
                    self.next += 1;
                    self.events
                        .insert((t + reo, key), Event::Lost { conn, seq: s });
                }
            }
        }
        let newly_lost = !marked.is_empty();
        let traced_lost = marked.clone();
        for s in marked {
            let l = f.outstanding(s);
            f.in_flight = f.in_flight.saturating_sub(window_bytes(l));
            f.out_bytes = f.out_bytes.saturating_sub(l);
            f.sent_at.remove(&s);
            f.retx.insert(s);
            f.resend.insert(s);
        }
        // A loss shown, or one a recovery left (a segment it sent lost),
        // starts the next recovery, outside the loss state.
        if f.recovery.is_none() && f.loss_high.is_none() && (newly_lost || !f.retx.is_empty()) {
            f.enter_recovery();
            if let Some(n) = self
                .ip_host
                .get(&conn.0.ip())
                .and_then(|&h| self.cuts.get_mut(h))
            {
                *n += 1;
            }
        }
        let in_recovery = f.recovery.is_some();
        if in_recovery {
            f.reduction(len, advanced, newly_lost);
        } else if (seq <= una || advanced) && f.limited() {
            // `tcp_may_raise_cwnd`: data the cumulative ack covers, not a
            // SACK's alone.
            f.grow(t, len, base);
        }
        // Data acknowledged at the cumulative point re-arms the timer.
        if seq <= una || advanced {
            f.arm(t, true, base);
        }
        if self.tcp_trace.is_some() {
            if !was_recovering && in_recovery {
                self.traced(t, conn, "rec", seq, 0);
            }
            if was_recovering && !in_recovery {
                self.traced(t, conn, "end", seq, 0);
            }
            for &s in &traced_lost {
                self.traced(t, conn, "lost", s, 0);
            }
            self.traced(t, conn, "ack", seq, len);
        }
        self.watch(t, conn);
    }

    /// The access queues dropped segment `seq` of `conn` at `t`. Its
    /// sender learns so from the acknowledgement of a segment sent after it
    /// ([`Scheduler::acked`]), or failing that from its tail-loss probe's
    /// ([`Scheduler::probe`]); a lost retransmission fails over to the
    /// retransmission timeout, doubled for each time it backed off.
    fn dropped(&mut self, t: Time, conn: (SocketAddr, SocketAddr), seq: u64) {
        let f = self.flows.entry(conn).or_default();
        let sent = f.sent_at.get(&seq).copied().unwrap_or(t);
        if f.retx.contains(&seq) {
            let due = (sent + f.rto() * (1i64 << f.backoff.min(6))).max(t);
            f.lost.insert(seq, (sent, Some(due)));
            let key = self.next;
            self.next += 1;
            self.events.insert((due, key), Event::Timeout { conn, seq });
            return;
        }
        f.lost.insert(seq, (sent, None));
        self.watch(t, conn);
    }

    /// Queues `conn`'s [`Event::Probe`] for its transmit timer, unless one
    /// is queued at or before it: with anything in flight the timer fires,
    /// so no loss, of data or of an acknowledgement, leaves the sender
    /// waiting for good.
    fn watch(&mut self, now: Time, conn: (SocketAddr, SocketAddr)) {
        let Some(f) = self.flows.get_mut(&conn) else {
            return;
        };
        if f.timer_at.is_nil() && f.in_flight > 0 {
            f.rearm_rto(now);
        }
        let at = f.timer_at;
        if at.is_nil() || (!f.probe_at.is_nil() && f.probe_at <= at) {
            return;
        }
        f.probe_at = at;
        let key = self.next;
        self.next += 1;
        self.events.insert((at, key), Event::Probe { conn });
    }

    /// `conn`'s transmit timer may be due at `t` ([`Scheduler::watch`]):
    /// unless it moved later, which queues it again, it fires. The
    /// tail-loss probe shows the data the access queues dropped and the
    /// sender doesn't know of, its acknowledgement a round trip later; the
    /// retransmission timeout, in a recovery or the loss state or once the
    /// probe has gone, sends them to the loss state, and backs off where it
    /// finds none (a spurious one, which Linux's F-RTO or an undo would
    /// take back: the model cuts nothing for it). Either reaches the
    /// receiver as a segment it has, which it acknowledges at once with
    /// everything it owes ([`Event::Probed`]), so a lost acknowledgement
    /// doesn't leave the sender waiting; then the retransmission timeout is
    /// armed a full one on.
    fn probe(&mut self, t: Time, conn: (SocketAddr, SocketAddr)) {
        let base = self.conn_rtt(conn);
        let reach = match (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) {
            (Some(&a), Some(&b)) => self.net.up_backlog(a, t) + self.net.one_way(a, b),
            _ => base / 2,
        };
        let Some(f) = self.flows.get_mut(&conn) else {
            return;
        };
        if f.probe_at == t {
            f.probe_at = Time::NIL;
        }
        if f.in_flight == 0 {
            f.timer_at = Time::NIL;
            f.tlp = false;
            return;
        }
        let at = f.timer_at;
        if at.is_nil() || at > t {
            self.watch(t, conn);
            return;
        }
        let unknown: Vec<u64> = f
            .lost
            .iter()
            .filter(|(_, v)| v.1.is_none())
            .map(|(&s, _)| s)
            .collect();
        if f.recovery.is_some() || f.loss_high.is_some() || !f.tlp {
            // The retransmission timeout: the segments lost and not yet
            // known go to the loss state.
            if unknown.is_empty() {
                f.backoff = f.backoff.saturating_add(1);
            }
            for s in unknown {
                if let Some(v) = f.lost.get_mut(&s) {
                    v.1 = Some(t);
                    let key = self.next;
                    self.next += 1;
                    self.events
                        .insert((t, key), Event::Timeout { conn, seq: s });
                }
            }
        } else {
            let back = if f.srtt > Duration::ZERO {
                f.srtt
            } else {
                base
            };
            // The probe shows the original segments lost a round trip on, as
            // its acknowledgement's SACK would; retransmissions lost again
            // wait for their timeout. The segments stay pending, as the
            // probe leaves RACK alone in Linux: a later segment's
            // acknowledgement before then marks them lost at once
            // (`tcp_rack_detect_loss`).
            for s in unknown {
                if let Some(v) = f.lost.get_mut(&s) {
                    v.1 = Some(t + back);
                    let key = self.next;
                    self.next += 1;
                    self.events
                        .insert((t + back, key), Event::Lost { conn, seq: s });
                }
            }
        }
        f.tlp = false;
        f.rearm_rto(t);
        let key = self.next;
        self.next += 1;
        self.events.insert((t + reach, key), Event::Probed { conn });
        self.watch(t, conn);
    }

    /// Segment `seg` (its record, sequence number, length and packets) of
    /// `conn` is through its receiver's downlink at `t`, which owes its
    /// acknowledgement and sends it as Linux does ([`ack`]): the pure ones
    /// it sends now take their turn on its uplink. The acknowledgement
    /// clocks the sender's window.
    fn owe_ack(&mut self, t: Time, conn: (SocketAddr, SocketAddr), seg: (u64, u64, usize, u32)) {
        self.owe_ack_in(t, conn, seg, true);
    }

    /// [`Scheduler::owe_ack`] for a segment's head, its tail still to come
    /// ([`ack::Owed::arrive_head`]).
    fn owe_ack_head(
        &mut self,
        t: Time,
        conn: (SocketAddr, SocketAddr),
        seg: (u64, u64, usize, u32),
    ) {
        self.owe_ack_in(t, conn, seg, false);
    }

    fn owe_ack_in(
        &mut self,
        t: Time,
        conn: (SocketAddr, SocketAddr),
        seg: (u64, u64, usize, u32),
        whole: bool,
    ) {
        let clock = self.rcv_clock(conn);
        let mut fresh = self.next;
        let o = self.owed.entry(conn).or_default();
        let before = o.due();
        let acks = if whole {
            o.arrive(t, seg.0, (seg.1, seg.2, seg.3), clock, &mut fresh)
        } else {
            o.arrive_head(t, seg.0, (seg.1, seg.2, seg.3), clock, &mut fresh)
        };
        let due = o.due();
        self.next = fresh;
        for a in acks {
            self.send_ack(t, conn, a);
        }
        if let Some(at) = due.filter(|_| due != before) {
            let key = self.next;
            self.next += 1;
            self.events.insert((at, key), Event::DelAck { conn });
        }
    }

    /// The receiver of `conn` sends pure acknowledgement `a` at `t`: it
    /// takes its turn on the receiver's uplink and reaches the sender a
    /// one-way trip after, unless the path loses it (the sites' loss, as
    /// a segment's packets), when the receiver's next acknowledgement
    /// carries what it acknowledged ([`ack::Owed::lost`]). A sender whose
    /// acknowledgements then stop has its transmit timer
    /// ([`Scheduler::probe`]), whose probe the receiver answers at once
    /// with what it owes.
    fn send_ack(&mut self, t: Time, conn: (SocketAddr, SocketAddr), a: ack::Ack) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        if a.pieces.is_empty() {
            return;
        }
        self.ack_counted(to, a.why, a.wait);
        let ack = tcp_wire(0).0;
        let held = self.held(to, t);
        let depart = self.net.occupy_up(t, to, ack, 1);
        self.ack_sent(to, self.net.up_charge(to, ack, 1), t, depart);
        let lost = self.net.lost(depart, to, from, 1);
        self.offered(&Offered {
            at: t,
            from: to,
            to: Some(from),
            conn: Some(conn),
            proto: PacketProto::Tcp,
            kind: PacketKind::Ack,
            wire: ack,
            packets: 1,
            took: 1,
            off: None,
            resent: None,
            held,
            transit: Some((depart, depart + self.net.one_way(to, from))),
            path_lost: lost.then_some(0),
        });
        if lost {
            if let Some(&(_, seq, len)) = a.pieces.first() {
                self.traced(depart, conn, "acklost", seq, len);
            }
            if let Some(o) = self.owed.get_mut(&conn) {
                o.lost(a.pieces);
            }
            return;
        }
        self.acked_at(depart + self.net.one_way(to, from), conn, a.pieces);
    }

    /// The receiver of `conn` as a sender: its smoothed round trip and
    /// retransmission timeout on the connection, or failing a sample, the
    /// path's base round trip ([`Scheduler::conn_rtt`]), where Linux's
    /// first estimate is its handshake's round trip, the SYN-ACK's queue
    /// wait in it.
    fn rcv_clock(&self, conn: (SocketAddr, SocketAddr)) -> RcvClock {
        match self.flows.get(&(conn.1, conn.0)) {
            Some(f) if f.srtt > Duration::ZERO => RcvClock {
                srtt: f.srtt,
                rto: f.rto(),
            },
            _ => {
                let rtt = self.conn_rtt(conn);
                RcvClock {
                    srtt: rtt,
                    rto: rtt + (rtt * 2).max(RTO_MIN),
                }
            }
        }
    }

    /// The acknowledgement of `pieces` (of segments of `conn`) reaches the
    /// sender at `at`.
    fn acked_at(&mut self, at: Time, conn: (SocketAddr, SocketAddr), pieces: Vec<ack::Piece>) {
        let Some(&(key, _, _)) = pieces.first() else {
            return;
        };
        let pieces = pieces.into_iter().map(|(_, seq, len)| (seq, len)).collect();
        self.events.insert((at, key), Event::Ack { conn, pieces });
    }

    /// Data of `conn` leaves its sender at `at` and reaches the peer at
    /// `arrival`: it carries what its sender owes on the reverse
    /// connection.
    fn piggyback(&mut self, conn: (SocketAddr, SocketAddr), at: Time, arrival: Time) {
        let back = (conn.1, conn.0);
        let Some(a) = self.owed.get_mut(&back).and_then(|o| o.data_sent(at)) else {
            return;
        };
        if let Some(&h) = self.ip_host.get(&conn.0.ip()) {
            self.ack_counted(h, AckWhy::Data, a.wait);
        }
        self.acked_at(arrival, back, a.pieces);
    }

    /// Host `h`'s receiver sent an acknowledgement for `why`, the first
    /// byte it acknowledges having waited `wait`.
    fn ack_counted(&mut self, h: HostIx, why: AckWhy, wait: Duration) {
        if let Some(n) = self
            .ack_whys
            .get_mut(h)
            .and_then(|w| w.get_mut(why as usize))
        {
            *n += 1;
        }
        if why == AckWhy::Timer
            && let Some(w) = self.ack_waits.get_mut(h)
        {
            w.add(wait);
        }
    }

    /// The base round trip of `conn`'s path.
    fn conn_rtt(&self, conn: (SocketAddr, SocketAddr)) -> Duration {
        match (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) {
            (Some(&a), Some(&b)) => self.net.rtt(a, b),
            _ => Duration::ZERO,
        }
    }

    /// Competing upload `i`'s acknowledgement of segment `seq` reaches its
    /// host at `t`, unless the host's access link loses it (the site's
    /// loss), when the next to arrive acknowledges it too.
    fn cross_ack_arrives(&mut self, t: Time, i: usize, seq: u64, frames: u32, rtt: Duration) {
        let Some(host) = self.cross.get(i).map(|c| c.host) else {
            return;
        };
        if self.net.lost_off_net(t, host, 1) {
            self.traced_cross(t, i, "acklost", seq, frames as usize * MSS as usize);
            if let Some(c) = self.cross.get_mut(i) {
                c.acks_lost.push((seq, frames, t - rtt));
                if c.probe_at.is_nil() {
                    // Its tail-loss probe, two smoothed round trips and 2
                    // ms past its last send, unless an acknowledgement
                    // comes first.
                    let srtt = c.flow.srtt.max(c.rtt);
                    let at = (c.flow.last_send + srtt * 2 + Duration::from_millis(2)).max(t);
                    c.probe_at = at;
                    let key = self.next;
                    self.next += 1;
                    self.events.insert((at, key), Event::CrossProbe { i });
                }
            }
            return;
        }
        let carried = self
            .cross
            .get_mut(i)
            .map(|c| std::mem::take(&mut c.acks_lost))
            .unwrap_or_default();
        for (s, n, sent) in carried {
            self.cross_acked(t, i, s, n, t - sent);
        }
        self.cross_acked(t, i, seq, frames, rtt);
    }

    /// Competing upload `i` at `t`, its start or the end of a round trip of
    /// its setup ([`Event::CrossTrip`]): each of the setup's round trips
    /// waits for the uplink's queue as it begins. The second is the data
    /// connection's handshake, its round trip's first sample (the smoothed
    /// round trip and the least); the last carries iperf3's cookie on it,
    /// whose acknowledgement is the first sample of CUBIC's state, begun at
    /// the handshake's end (`tcp_init_transfer`): HyStart's least round
    /// trip. Once the setup is done, its data starts.
    fn cross_trip(&mut self, t: Time, i: usize) {
        let Some(c) = self.cross.get_mut(i) else {
            return;
        };
        if !c.data_at.is_nil() {
            if c.flow.last_send.is_nil() {
                self.cross_pump(t, i);
            }
            return;
        }
        c.started = true;
        let (host, rtt) = (c.host, c.rtt);
        let trip = rtt + self.net.up_backlog(host, t);
        let Some(c) = self.cross.get_mut(i) else {
            return;
        };
        c.trips += 1;
        let at = t + trip;
        if c.trips == 2 {
            c.flow.estimate(trip);
        }
        if c.trips >= crate::spec::COMPETE_SETUP_TRIPS {
            c.data_at = at;
            c.stop += c.data_at - c.start;
            c.flow.sampled(0, trip);
        }
        let key = self.next;
        self.next += 1;
        self.events.insert((at, key), Event::CrossTrip { i });
    }

    /// Competing upload `i`'s tail-loss probe is due at `t`: if what lost
    /// acknowledgements acknowledged is still unheard, the probe's
    /// acknowledgement, a round trip on, carries it (and may be lost in
    /// turn, which queues the next probe).
    fn cross_probe(&mut self, t: Time, i: usize) {
        let Some(c) = self.cross.get_mut(i) else {
            return;
        };
        c.probe_at = Time::NIL;
        if c.acks_lost.is_empty() {
            return;
        }
        let (seq, frames, sent) = c.acks_lost.remove(0);
        let at = t + c.rtt + self.net.up_backlog(c.host, t);
        let key = self.next;
        self.next += 1;
        self.events.insert(
            (at, key),
            Event::CrossAck {
                i,
                seq,
                frames,
                rtt: at - sent,
            },
        );
    }

    /// Competing upload `i`'s segment `seq` was acknowledged at `t`, a
    /// round trip of `rtt` after it went out, as a connection's are
    /// ([`Scheduler::acked`]): the end of a recovery it completes, a
    /// recovery a loss left, then the recovery's window or its growth.
    fn cross_acked(&mut self, t: Time, i: usize, seq: u64, frames: u32, rtt: Duration) {
        let was = self.cross.get(i).is_some_and(|c| c.flow.recovery.is_some());
        let len = frames as usize * MSS as usize;
        if let Some(c) = self.cross.get_mut(i) {
            let f = &mut c.flow;
            let una = f.snd_una();
            if f.sent_at.remove(&seq).is_none() {
                return;
            }
            f.in_flight = f.in_flight.saturating_sub(len);
            f.retx.remove(&seq);
            f.sampled(seq, rtt);
            let advanced = f.snd_una() > una;
            f.recovered();
            if f.recovery.is_none() && f.loss_high.is_none() && !f.retx.is_empty() {
                f.enter_recovery();
            }
            // An upload always has data: its window always limits it.
            if f.recovery.is_some() {
                f.reduction(len, advanced, false);
            } else {
                f.grow(t, len, c.rtt);
            }
        }
        if self.tcp_trace.is_some() {
            let now = self.cross.get(i).is_some_and(|c| c.flow.recovery.is_some());
            if was && !now {
                self.traced_cross(t, i, "end", seq, len);
            }
            if !was && now {
                self.traced_cross(t, i, "rec", seq, len);
            }
            self.traced_cross(t, i, "ack", seq, len);
        }
        self.cross_pump(t, i);
    }

    /// Competing upload `i` learned at `t` that its segment `seq` was
    /// lost: it owes a resend, and outside a recovery one begins.
    fn cross_lost(&mut self, t: Time, i: usize, seq: u64, frames: u32) {
        let was = self.cross.get(i).is_some_and(|c| c.flow.recovery.is_some());
        let len = frames as usize * MSS as usize;
        if let Some(c) = self.cross.get_mut(i) {
            let f = &mut c.flow;
            if f.sent_at.remove(&seq).is_none() {
                return;
            }
            f.in_flight = f.in_flight.saturating_sub(len);
            f.retx.insert(seq);
            c.resend.insert(seq, frames);
            if f.recovery.is_none() && f.loss_high.is_none() {
                f.enter_recovery();
                f.reduction(MSS as usize, false, true);
            } else {
                f.lost_in_recovery(len);
            }
        }
        if self.tcp_trace.is_some() {
            if !was {
                self.traced_cross(t, i, "rec", seq, len);
            }
            self.traced_cross(t, i, "lost", seq, len);
        }
        self.cross_pump(t, i);
    }

    /// Sends what competing upload `i`'s window allows at `at`, while it
    /// runs.
    fn cross_pump(&mut self, at: Time, i: usize) {
        let Some(cap) = self.window() else {
            return;
        };
        loop {
            let Some(c) = self.cross.get_mut(i) else {
                return;
            };
            let f = &mut c.flow;
            // A segment's sequence number is its first frame's: a resent
            // tail goes alone.
            let (seq, frames) = match c.resend.first_key_value() {
                Some((&s, &n)) => (s, n),
                None => (f.sent_end, CROSS_FRAMES),
            };
            let len = frames as usize * MSS as usize;
            if !c.started || at < c.data_at || at >= c.stop || f.holds(len) {
                return;
            }
            if c.resend.remove(&seq).is_none() {
                f.sent_end += u64::from(frames);
            }
            let (host, rtt) = (c.host, c.rtt);
            let resent = f.retx.contains(&seq);
            f.recovery_sent(len);
            f.sent(at, len, f.srtt.max(rtt));
            f.sent_at.insert(seq, at);
            self.traced_cross(at, i, if resent { "rtx" } else { "tx" }, seq, len);
            let id = self.next;
            self.next += 1;
            // The queue takes the segment whole where it fits, else its
            // leading frames that fit, a frame at a time, and drops the
            // rest. A segment longer than the whole queue (a queue under
            // two frames' time) goes whole into a queue that isn't full,
            // even one without a frame's room, as a connection's segment
            // does ([`overflows`]).
            let admitted = self
                .net
                .up_admit(host, at, cap, frames as usize * CROSS_FRAME, frames);
            let (wait, mut took) = (admitted.wait, admitted.packets);
            let mut offer = Offered {
                at,
                from: host,
                to: None,
                conn: None,
                proto: PacketProto::Tcp,
                kind: PacketKind::Competing,
                wire: frames as usize * CROSS_FRAME,
                packets: frames,
                took,
                off: Some(seq * MSS as u64),
                resent: Some(resent),
                held: self.held(host, at),
                transit: None,
                path_lost: None,
            };
            if took < frames {
                if took == 0 {
                    self.offered(&offer);
                }
                if let Some(d) = self.cross_drops.get_mut(host) {
                    *d += 1;
                }
                if let Some(d) = self.cross_drop_packets.get_mut(host) {
                    *d += u64::from(frames - took);
                }
                self.count_lost(host, at, u64::from(frames - took));
                let tail = seq + u64::from(took);
                if let Some(c) = self.cross.get_mut(i) {
                    // The tail is a segment of its own from now on.
                    c.flow.sent_at.insert(tail, at);
                }
                self.traced_cross(at, i, "drop", tail, (frames - took) as usize * MSS as usize);
                self.events.insert(
                    (at + rtt + wait, id),
                    Event::CrossLost {
                        i,
                        seq: tail,
                        frames: frames - took,
                    },
                );
                if took == 0 {
                    continue;
                }
            }
            let wire = took as usize * CROSS_FRAME;
            let depart = self.net.occupy_up(at, host, wire, took);
            if let Some(b) = self.cross_bytes.get_mut(host) {
                *b += wire as u64;
            }
            let id = self.next;
            self.next += 1;
            let lost = self.net.first_lost_off_net(depart, host, took);
            offer.path_lost = lost;
            self.offered(&offer);
            match lost {
                Some(0) => {
                    self.events.insert(
                        (depart + rtt, id),
                        Event::CrossLost {
                            i,
                            seq,
                            frames: took,
                        },
                    );
                    continue;
                }
                Some(k) => {
                    // A frame past the first: the leading ones arrive and
                    // what follows is a segment of its own, resent alone.
                    let tail = seq + u64::from(k);
                    if let Some(c) = self.cross.get_mut(i) {
                        c.flow.sent_at.insert(tail, at);
                    }
                    let id = self.next;
                    self.next += 1;
                    self.events.insert(
                        (depart + rtt, id),
                        Event::CrossLost {
                            i,
                            seq: tail,
                            frames: took - k,
                        },
                    );
                    took = k;
                }
                None => {}
            }
            // Two packets are more than the receiver's rcv_mss, which
            // acknowledges them at once (`ack::Owed`); a lone one goes with
            // the next to come, or fills a hole.
            let ack = depart + rtt;
            self.events.insert(
                (ack, id),
                Event::CrossAck {
                    i,
                    seq,
                    frames: took,
                    rtt: ack - at,
                },
            );
        }
    }

    /// Sends what `conns`' windows allow at `at`, a segment from each in
    /// turn: a lost one again first, then one of the bytes unsent, as
    /// many as the size goal and the window's room in whole packets take
    /// (`tcp_write_xmit`'s `tcp_cwnd_test`, a segment past it cut to it as
    /// `tso_fragment` does), from as many records as hold them.
    fn pump(&mut self, at: Time, conns: &[(SocketAddr, SocketAddr)]) {
        self.room_moved.extend_from_slice(conns);
        let mut open: Vec<((SocketAddr, SocketAddr), Duration)> =
            conns.iter().map(|&c| (c, self.conn_rtt(c))).collect();
        while !open.is_empty() {
            open.retain(|&(conn, rtt)| {
                let Some(f) = self.flows.get_mut(&conn) else {
                    return false;
                };
                if let Some(&seq) = f.resend.first() {
                    let len = f.outstanding(seq);
                    if f.holds(len) {
                        f.held = true;
                        return false;
                    }
                    f.resend.pop_first();
                    if let Some(t) = f.tails.get_mut(&seq) {
                        t.resent = true;
                    }
                    let head = f.snd_una() >= seq;
                    f.recovery_sent(len);
                    f.sent(at, len, f.srtt.max(rtt));
                    f.sent_at.insert(seq, at);
                    // Resending the head re-arms the retransmission
                    // timeout a full one on (`tcp_xmit_retransmit_queue`).
                    if head || f.timer_at.is_nil() {
                        f.tlp = false;
                        f.timer_at = at + f.rto_backed();
                    }
                    self.sending.push((conn, seq));
                    return true;
                }
                let goal = f.size_goal(rtt);
                let mut have = 0;
                for &(id, n) in &f.unsent {
                    if self.records.get(&id).is_some() {
                        have += n;
                    }
                    if have >= goal {
                        break;
                    }
                }
                if have == 0 {
                    f.unsent.clear();
                    f.held = false;
                    return false;
                }
                let mut len = have.min(goal);
                let room =
                    (f.cwnd - f.in_flight as f64).max(0.0) as usize / MSS as usize * MSS as usize;
                if room > 0 && room < len {
                    // TSO deferral (`tcp_tso_should_defer`): outside a
                    // recovery and the loss state, a segment the window
                    // would cut waits for
                    // the room the next acknowledgement brings, if the flow
                    // sent within the last millisecond, the room is under a
                    // third of the window (`tcp_tso_win_divisor`) and the
                    // oldest segment's acknowledgement is due within a
                    // millisecond or half a round trip.
                    let srtt = if f.srtt > Duration::ZERO { f.srtt } else { rtt };
                    let soon = Duration::from_millis(1).min(srtt / 2);
                    let ack_due = f.sent_at.first_key_value().map(|(_, &sent)| sent + srtt);
                    if f.recovery.is_none()
                        && f.loss_high.is_none()
                        && !f.last_send.is_nil()
                        && at - f.last_send < Duration::from_millis(1)
                        && (room as f64) < f.cwnd / 3.0
                        && ack_due.is_some_and(|due| due - at <= soon)
                    {
                        f.held = true;
                        return false;
                    }
                    len = room;
                }
                if f.holds(len) {
                    f.held = true;
                    return false;
                }
                let mut parts = Vec::new();
                let mut left = len;
                while left > 0 {
                    let Some(front) = f.unsent.front_mut() else {
                        break;
                    };
                    let Some(r) = self.records.get(&front.0) else {
                        f.unsent.pop_front();
                        continue;
                    };
                    let take = front.1.min(left);
                    parts.push((front.0, take));
                    front.1 -= take;
                    left -= take;
                    if front.1 == 0 {
                        // Its bytes all out: they leave its socket's
                        // unsent count.
                        if let (true, What::Data { len, .. }) = (self.buffers.is_some(), r.what) {
                            self.admits.insert((at, front.0), (conn, len));
                        }
                        f.unsent.pop_front();
                    }
                }
                let len = len - left;
                let seq = f.sent_end;
                f.sent_end += 1;
                f.segments.insert(
                    seq,
                    Wire {
                        len,
                        parts,
                        acked: 0,
                        off: f.off_end,
                    },
                );
                f.off_end += len as u64;
                f.recovery_sent(len);
                f.sent(at, len, f.srtt.max(rtt));
                f.arm(at, false, rtt);
                f.sent_at.insert(seq, at);
                self.sending.push((conn, seq));
                true
            });
            for (conn, seq) in std::mem::take(&mut self.sending) {
                self.transmit_window(at, conn, seq);
            }
        }
        for &conn in conns {
            self.watch(at, conn);
        }
    }

    /// Data record `id` joins `conn`'s write queue.
    fn queue_window(&mut self, conn: (SocketAddr, SocketAddr), id: u64) {
        let len = match self.records.get(&id).map(|r| r.what) {
            Some(What::Data { len, .. }) => len,
            _ => return,
        };
        self.arriving.insert(id, len);
        self.flows
            .entry(conn)
            .or_default()
            .unsent
            .push_back((id, len));
    }

    /// The bytes `from..from + len` of segment `seq` of `conn` are through
    /// to its receiver: a record whose bytes have all come is due.
    fn segment_through(
        &mut self,
        conn: (SocketAddr, SocketAddr),
        seq: u64,
        from: usize,
        len: usize,
    ) {
        let Some(g) = self.flows.get(&conn).and_then(|f| f.segments.get(&seq)) else {
            return;
        };
        let (mut at, end) = (0, from + len);
        let mut done = Vec::new();
        for &(id, n) in &g.parts {
            let (a, b) = (at.max(from), (at + n).min(end));
            at += n;
            if b <= a {
                continue;
            }
            if let Some(left) = self.arriving.get_mut(&id) {
                *left = left.saturating_sub(b - a);
                if *left == 0 {
                    done.push(id);
                }
            }
        }
        for id in done {
            self.arriving.swap_remove(&id);
            if let Some(r) = self.records.get_mut(&id) {
                r.state = State::Due;
                if let Some(ls) = self.links.get_mut(r.link) {
                    ls.due = true;
                }
            }
        }
    }

    /// The window model's send of `conn`'s segment `seq` at `at`, lost if
    /// the uplink queue would overflow: its leading frames that fit go, the
    /// rest is lost, as the testbed's tbf takes and drops frames.
    fn transmit_window(&mut self, at: Time, conn: (SocketAddr, SocketAddr), seq: u64) {
        let Some(cap) = self.window() else {
            return;
        };
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let Some(f) = self.flows.get(&conn) else {
            return;
        };
        let Some(g) = f.segments.get(&seq) else {
            return;
        };
        let resent = f.retx.contains(&seq);
        // A tail the queue dropped goes again alone.
        let (start, len) = match f.tails.get(&seq) {
            Some(t) => (g.len - t.tail, t.tail),
            None => (0, g.len),
        };
        let off = g.off + start as u64;
        let (wire, packets) = tcp_wire(len);
        self.traced(at, conn, if resent { "rtx" } else { "tx" }, seq, len);
        // The queue takes the leading frames that fit, a frame at a time,
        // and drops the rest.
        let admitted = self.net.up_admit(from, at, cap, wire, packets);
        let (mut took, mut head_wire) = (admitted.packets, admitted.wire);
        let mut offer = Offered {
            at,
            from,
            to: Some(to),
            conn: Some(conn),
            proto: PacketProto::Tcp,
            kind: PacketKind::Data,
            wire,
            packets,
            took,
            off: Some(off),
            resent: Some(resent),
            held: self.held(from, at),
            transit: None,
            path_lost: None,
        };
        if took < packets {
            let head = (took as usize * MSS as usize).min(len);
            self.traced(at, conn, "drop", seq, len - head);
            if let Some(d) = self.drops.get_mut(from) {
                *d += 1;
            }
            self.queue_lost(from, at, u64::from(packets - took));
            if took == 0 {
                self.offered(&offer);
                self.dropped(at, conn, seq);
                return;
            }
            let f = self.flows.entry(conn).or_default();
            f.tails.insert(
                seq,
                Tail {
                    head,
                    tail: len - head,
                    resent: false,
                },
            );
        }
        let t = self
            .net
            .send(at, from, to, head_wire, took, Carrier::Lossless);
        if self
            .flows
            .get_mut(&conn)
            .is_some_and(|f| f.path_lost.remove(&seq))
        {
            self.retransmitted(from, at, t.depart);
        }
        // What went out of the segment: a tail the queue dropped goes again
        // as a resend.
        let went = (took as usize * MSS as usize).min(len);
        self.sent(from, if resent { 0 } else { went });
        if resent {
            let charge = self.net.up_charge(from, head_wire, took);
            self.resent(from, charge);
        }
        let lost = self.net.first_lost(t.depart, from, to, took);
        if lost.is_some()
            && let Some(f) = self.flows.get_mut(&conn)
        {
            f.path_lost.insert(seq);
        }
        offer.transit = Some((t.depart, t.arrival));
        offer.path_lost = lost;
        self.offered(&offer);
        match lost {
            Some(0) => {
                // Its first packet was lost on the path: its sender learns
                // so as of a full queue's drop, and resends it whole, or a
                // tail it resent the tail again.
                if let Some(f) = self.flows.get_mut(&conn) {
                    if start == 0 {
                        f.tails.remove(&seq);
                    } else {
                        f.tails.insert(
                            seq,
                            Tail {
                                head: 0,
                                tail: len,
                                resent: false,
                            },
                        );
                    }
                }
                self.dropped(at, conn, seq);
                return;
            }
            Some(k) => {
                // A later one: the leading packets arrive, and what follows
                // goes again alone, as a queue's dropped tail does.
                head_wire = strims_wan::framing::frames(head_wire, took)
                    .take(k as usize)
                    .sum();
                took = k;
                let head = (took as usize * MSS as usize).min(len);
                let f = self.flows.entry(conn).or_default();
                f.tails.insert(
                    seq,
                    Tail {
                        head,
                        tail: len - head,
                        resent: false,
                    },
                );
            }
            None => {}
        }
        let whole = took == packets;
        if !whole {
            self.dropped(at, conn, seq);
        }
        self.piggyback(conn, at, t.arrival);
        let key = self.next;
        self.next += 1;
        let sent = if whole {
            len
        } else {
            (took as usize * MSS as usize).min(len)
        };
        self.events.insert(
            (t.arrival, key),
            Event::Segment {
                conn,
                to,
                seq,
                from: start,
                len: sent,
                packets: took,
                wire: head_wire,
                whole,
                through: false,
            },
        );
    }

    /// Lines queues up against their records, on the links with records or
    /// messages in flight (a link with neither has nothing to line up): to
    /// `release`, the links with due records, whose due ones it releases
    /// ([`Scheduler::released`] holds the links that released a message);
    /// otherwise the links with new messages, which it records, releasing
    /// nothing. Returns the new records in queue order.
    fn match_queues(&mut self, sim: &Sim<'_>, release: bool) -> Result<Vec<u64>, String> {
        let mut fresh = Vec::new();
        let released = &mut self.released;
        if release {
            released.clear();
        }
        let mut err = None;
        let links = &mut self.links;
        let recorded = &mut self.recorded;
        let records = &mut self.records;
        let ip_host = &self.ip_host;
        let assocs = &self.assocs;
        let frozen = &self.frozen;
        let next = &mut self.next;
        // A link with records whose queue turmoil emptied on its own.
        let emptied = |recorded: &BTreeSet<usize>, after: Option<usize>, before: Option<usize>| {
            let from = after.map_or(0, |a| a + 1);
            let mut range = match before {
                Some(b) if b >= from => recorded.range(from..b),
                Some(_) => return None,
                None => recorded.range(from..),
            };
            range
                .next()
                .map(|li| format!("wan: link {li} dropped in-flight messages the scheduler didn't"))
        };
        let mut last = None;
        // Data channels whose reader takes messages through a chunk.
        let mut reads: Vec<(ConnKey, u64)> = Vec::new();
        sim.busy_links(|iter| {
            'links: for link in iter {
                let li = link.index();
                if let Some(e) = emptied(recorded, last, Some(li)) {
                    err = Some(e);
                    break;
                }
                last = Some(li);
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
                let new = link.len() != ls.records.len();
                if release && new {
                    err = Some(format!("wan: link {li} took messages during the walk"));
                    break;
                }
                // Nothing to release, or nothing new.
                if if release { !ls.due } else { !new } {
                    continue;
                }
                let frozen = frozen.contains(&pair_key(a, b));
                // A data channel hands its reader whole messages: a chunk
                // goes once every chunk of its connection through the end
                // of its message is due.
                let mut whole: Vec<(ConnKey, u64)> = Vec::new();
                if release && !assocs.is_empty() && !frozen {
                    let mut cut: Vec<ConnKey> = Vec::new();
                    for &id in &ls.records {
                        let Some(r) = records.get(&id) else {
                            continue;
                        };
                        let conn = (r.src, r.dst);
                        let (What::Data { seq, .. }, Some(assoc)) = (r.what, assocs.get(&conn))
                        else {
                            continue;
                        };
                        if cut.contains(&conn) {
                            continue;
                        }
                        if r.state != State::Due {
                            cut.push(conn);
                        } else if assoc.ends_message(seq) {
                            match whole.iter_mut().find(|(c, _)| *c == conn) {
                                Some(w) => w.1 = seq,
                                None => whole.push((conn, seq)),
                            }
                        }
                    }
                }
                let held = |r: &Record| match r.what {
                    What::Data { seq, .. } if assocs.contains_key(&(r.src, r.dst)) => !whole
                        .iter()
                        .any(|&(c, w)| c == (r.src, r.dst) && seq <= w),
                    _ => false,
                };
                // The link's known records come first in its queue, in
                // order; those it delivers leave a mark until it compacts.
                let known = ls.records.len();
                let (mut seen, mut gone) = (0, 0);
                if release {
                    ls.due = false;
                }
                for sent in link {
                    let (src, dst) = sent.pair();
                    let what = What::of(sent.protocol());
                    if seen < known {
                        let at = seen;
                        seen += 1;
                        let Some(id) = ls.records.get(at).copied() else {
                            break;
                        };
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
                        if !release {
                            continue;
                        }
                        if r.state == State::Due && (frozen || held(r)) {
                            ls.due = true;
                        } else if r.state == State::Due {
                            sent.deliver();
                            records.remove(&id);
                            if let Some(x) = ls.records.get_mut(at) {
                                *x = DELIVERED;
                            }
                            gone += 1;
                            if released.last() != Some(&li) {
                                released.push(li);
                            }
                        }
                        continue;
                    }
                    let (Some(&from), Some(&to)) = (ip_host.get(&src.ip()), ip_host.get(&dst.ip()))
                    else {
                        err = Some(format!("wan: link {li} carries {src}->{dst}"));
                        break 'links;
                    };
                    let (wire, packets) = match (what, assocs.get(&(src, dst))) {
                        (What::Data { len, .. }, Some(a)) => (a.profile().data_wire(len), 1),
                        _ => wire(sent.protocol()),
                    };
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
                            nominal: nominal(sent.protocol()),
                            state: State::InFlight,
                            link: li,
                        },
                    );
                    ls.records.push_back(id);
                    fresh.push(id);
                }
                if seen < known {
                    err = Some(format!(
                        "wan: link {li} dropped in-flight messages the scheduler didn't"
                    ));
                    break;
                }
                reads.extend(whole);
                while gone > 0 && ls.records.front() == Some(&DELIVERED) {
                    ls.records.pop_front();
                    gone -= 1;
                }
                if gone > 0 {
                    ls.records.retain(|&id| id != DELIVERED);
                }
                if ls.records.is_empty() {
                    recorded.remove(&li);
                } else {
                    recorded.insert(li);
                }
            }
            if err.is_none() {
                err = emptied(recorded, last, None);
            }
        });
        for (conn, seq) in reads {
            if let Some(a) = self.assocs.get_mut(&conn) {
                a.read_through(seq);
            }
        }
        match err {
            Some(e) => Err(e),
            None => Ok(fresh),
        }
    }

    /// Hands the new records to their senders' uplinks, sent at `at`. With
    /// send buffers a sender's TCP segments go in the order its sockets
    /// took them, each other message behind its link's last segment, and
    /// with the window model a connection sends what its window allows as
    /// it writes, as Linux's `sendmsg` does: one socket's burst reaches
    /// the uplink before another's written after it. Without, each sender
    /// serves its links in turn, one message at a time, in queue order on
    /// each link.
    fn send(&mut self, at: Time, fresh: Vec<u64>) {
        if let Some(b) = self.buffers.clone() {
            self.send_written(at, fresh, &b);
            return;
        }
        let by_sender = self.queues(fresh);
        let window = self.window().is_some();
        for mut queues in by_sender {
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
                            self.queue_window(conn, id);
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

    /// `fresh`'s records by sender, in host order, and each sender's by
    /// receiver and connection addresses, in that order, each queue in
    /// `fresh`'s order.
    fn queues(&self, fresh: Vec<u64>) -> Vec<Vec<VecDeque<u64>>> {
        // (sender, receiver, source and destination addresses), position
        let mut keyed: Vec<(QueueKey, usize, u64)> = fresh
            .into_iter()
            .enumerate()
            .filter_map(|(i, id)| {
                let r = self.records.get(&id)?;
                Some((
                    (r.from, r.to, ip_key(r.src.ip()), ip_key(r.dst.ip())),
                    i,
                    id,
                ))
            })
            .collect();
        keyed.sort_unstable();
        let mut senders: Vec<Vec<VecDeque<u64>>> = Vec::new();
        let mut last: Option<QueueKey> = None;
        for (key, _, id) in keyed {
            match last {
                Some(k) if k == key => {}
                Some((from, ..)) if from == key.0 => {
                    if let Some(s) = senders.last_mut() {
                        s.push(VecDeque::new());
                    }
                }
                _ => senders.push(vec![VecDeque::new()]),
            }
            last = Some(key);
            if let Some(q) = senders.last_mut().and_then(|s| s.last_mut()) {
                q.push_back(id);
            }
        }
        senders
    }

    /// [`Scheduler::send`] with send buffers.
    fn send_written(&mut self, at: Time, fresh: Vec<u64>, b: &SendBuffers) {
        let by_sender = self.queues(fresh);
        let window = self.window().is_some();
        for queues in by_sender {
            // (write order, queue position, record)
            // (write order, queue position, record, whether it ends a message)
            let mut keyed: Vec<(u64, usize, u64, bool)> = Vec::new();
            for q in queues {
                let mut last = 0;
                for id in q {
                    let write = self.records.get(&id).and_then(|r| match r.what {
                        What::Data { .. } => b.next_write((r.src, r.dst)),
                        _ => None,
                    });
                    last = write.map_or(last, |(stamp, _)| stamp);
                    let end = write.is_none_or(|(_, end)| end);
                    keyed.push((last, keyed.len(), id, end));
                }
            }
            keyed.sort_unstable();
            for (_, _, id, end) in keyed {
                let data = self.records.get(&id).and_then(|r| match r.what {
                    What::Data { .. } if window => Some((r.src, r.dst)),
                    _ => None,
                });
                match data {
                    Some(conn) if self.assocs.contains_key(&conn) => {
                        self.sctp_write(at, conn, id, end);
                    }
                    Some(conn) => {
                        self.queue_window(conn, id);
                        self.pump(at, &[conn]);
                    }
                    None => self.transmit(at, id),
                }
            }
        }
    }

    fn transmit(&mut self, at: Time, id: u64) {
        let window = self.window();
        let Some(r) = self.records.get_mut(&id) else {
            return;
        };
        let carrier = match r.what {
            What::Datagram(_) if r.nominal => Carrier::Lossless,
            What::Datagram(_) => Carrier::Datagram,
            _ => Carrier::Stream,
        };
        let mut offer = Offered {
            at,
            from: r.from,
            to: Some(r.to),
            conn: Some((r.src, r.dst)),
            proto: match r.what {
                What::Datagram(_) => PacketProto::Udp,
                _ => PacketProto::Tcp,
            },
            kind: match r.what {
                What::Datagram(_) if r.nominal => PacketKind::Control,
                What::Datagram(_) | What::Data { .. } => PacketKind::Data,
                What::Syn | What::Fin(_) | What::Rst => PacketKind::Control,
            },
            wire: r.wire,
            packets: r.packets,
            took: r.packets,
            off: None,
            resent: None,
            held: 0,
            transit: None,
            path_lost: None,
        };
        offer.held = self.held(offer.from, at);
        if let (Some(cap), Carrier::Datagram) = (window, carrier) {
            // A full queue drops a datagram as it does a segment; the
            // datagram's sender learns of it only from what it carries.
            if self
                .net
                .up_admit(offer.from, at, cap, offer.wire, offer.packets)
                .packets
                < offer.packets
            {
                offer.took = 0;
                self.offered(&offer);
                if let Some(d) = self.drops.get_mut(offer.from) {
                    *d += 1;
                }
                self.queue_lost(offer.from, at, u64::from(offer.packets));
                if let Some(r) = self.records.get_mut(&id) {
                    r.state = State::Dropped;
                }
                self.records.dropped += 1;
                return;
            }
        }
        let Some(r) = self.records.get_mut(&id) else {
            return;
        };
        if let (Some(b), What::Data { len, .. }) = (&self.buffers, r.what) {
            let q = b.model().uplink_queue;
            let admit = at.max(self.net.up_free(r.from) - Duration::from(q));
            self.admits.insert((admit, id), ((r.src, r.dst), len));
        }
        let app = match r.what {
            What::Data { len, .. } | What::Datagram(len) => len,
            What::Syn | What::Fin(_) | What::Rst => 0,
        };
        let from = r.from;
        let t = self.net.send(at, r.from, r.to, r.wire, r.packets, carrier);
        self.sent(from, app);
        if t.dropped {
            self.count_lost(from, at, 1);
        }
        if t.retransmit > Duration::ZERO {
            // Its resend arrives the delay late, so leaves the delay late.
            self.retransmitted(from, at, t.depart + t.retransmit);
        }
        offer.transit = Some((t.depart, t.arrival + t.retransmit));
        offer.path_lost = t.dropped.then_some(0);
        self.offered(&offer);
        let Some(r) = self.records.get_mut(&id) else {
            return;
        };
        if t.dropped {
            r.state = State::Dropped;
            self.records.dropped += 1;
            return;
        }
        self.events
            .insert((t.arrival + t.retransmit, id), Event::Arrive);
    }

    /// Whether `r`, a SYN, is a modelled WebRTC link's ICE check: a dial of
    /// a channel listener (`strims_node`'s `model_rtc`), whose connect is
    /// the check's round trip and its SYN-ACK the STUN binding response.
    fn ice_check(&self, r: &Record) -> bool {
        r.dst.port() == UDP_PORT && self.rtc_hosts.get(r.to).copied().unwrap_or(false)
    }

    /// SYN `id`'s acceptor answers it at `t`, as it comes through its
    /// downlink: a SYN-ACK handed to its uplink, behind what the uplink
    /// holds (Linux replies from the SYN's softirq), whose arrival at the
    /// connector completes the connect. An ICE check's answer is the STUN
    /// binding response a browser sends at once, as it queues behind its
    /// uplink too.
    fn syn_ack(&mut self, t: Time, id: u64) {
        let Some(r) = self.records.get(&id) else {
            return;
        };
        let (from, to, conn) = (r.to, r.from, (r.dst, r.src));
        let (wire, proto) = if self.ice_check(r) {
            (STUN_RESPONSE, PacketProto::Udp)
        } else {
            (TCP_SYN, PacketProto::Tcp)
        };
        let held = self.held(from, t);
        let back = self.net.send(t, from, to, wire, 1, Carrier::Stream);
        if back.retransmit > Duration::ZERO {
            self.retransmitted(from, t, back.depart + back.retransmit);
        }
        self.offered(&Offered {
            at: t,
            from,
            to: Some(to),
            conn: Some(conn),
            proto,
            kind: PacketKind::Control,
            wire,
            packets: 1,
            took: 1,
            off: None,
            resent: None,
            held,
            transit: Some((back.depart, back.arrival + back.retransmit)),
            path_lost: None,
        });
        self.events
            .insert((back.arrival + back.retransmit, id), Event::SynAcked);
    }

    /// SYN `id`'s SYN-ACK reaches the connector at `t`. Turmoil's connect
    /// and accept complete as it delivers the SYN, at the next step. With
    /// the window model, the connector acknowledges the SYN-ACK at once
    /// with a pure ACK on its uplink, ahead of its first data; an ICE
    /// check's connector, a browser's ICE agent, sends nothing. That ACK's
    /// loss would change nothing, the first data acknowledging the SYN-ACK
    /// in its place, so none is drawn: a counter of pure ACKs, a packet
    /// log's row and the uplink's time are all it moves.
    fn syn_acked(&mut self, t: Time, id: u64) {
        let check = self.records.get(&id).is_some_and(|r| self.ice_check(r));
        let Some(r) = self.records.get_mut(&id) else {
            return;
        };
        r.state = State::Due;
        if let Some(ls) = self.links.get_mut(r.link) {
            ls.due = true;
        }
        let (from, to, conn) = (r.from, r.to, (r.src, r.dst));
        if self.window().is_none() || check {
            return;
        }
        let ack = tcp_wire(0).0;
        let held = self.held(from, t);
        let depart = self.net.occupy_up(t, from, ack, 1);
        self.ack_sent(from, self.net.up_charge(from, ack, 1), t, depart);
        self.offered(&Offered {
            at: t,
            from,
            to: Some(to),
            conn: Some(conn),
            proto: PacketProto::Tcp,
            kind: PacketKind::Ack,
            wire: ack,
            packets: 1,
            took: 1,
            off: None,
            resent: None,
            held,
            transit: Some((depart, depart + self.net.one_way(from, to))),
            path_lost: None,
        });
    }

    /// Clears the lost datagrams of every direction that holds nothing
    /// else, on links that released nothing in this walk and are neither
    /// partitioned nor frozen.
    fn purge(&mut self, sim: &Sim<'_>) {
        if self.records.dropped == 0 {
            return;
        }
        let mut purges = Vec::new();
        for &li in &self.recorded {
            let Some(ls) = self.links.get(li) else {
                continue;
            };
            let Some((a, b)) = ls.hosts else {
                continue;
            };
            let key = pair_key(a, b);
            if ls.records.is_empty()
                || self.released.binary_search(&li).is_ok()
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
            if ls.records.is_empty() {
                self.recorded.remove(&li);
            }
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
            ls.due = false;
            if let Some(&li) = self.link_of.get(&pair_key(a, b)) {
                self.recorded.remove(&li);
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

    /// The hosts' uplink queues are sampled every `d` (by default
    /// [`QUEUE_SAMPLE_EVERY`]). A period that divides a schedule's sees
    /// the queue at the same few phases of it.
    pub fn sample_queues_every(&mut self, d: Duration) {
        self.queue_every = d;
    }

    /// The base one-way latency between `a` and `b` becomes `d`.
    pub fn set_latency(&mut self, a: HostIx, b: HostIx, d: Duration) {
        if a != b {
            self.net.set_one_way(a, b, Some(d));
        }
    }

    /// The window model's TCP events are traced from now on.
    pub(crate) fn trace_tcp(&mut self) {
        self.tcp_trace = Some(Vec::new());
    }

    /// The TCP events traced so far.
    pub(crate) fn take_tcp_trace(&mut self) -> Vec<TcpEvent> {
        self.tcp_trace.take().unwrap_or_default()
    }

    /// Traces `what` of segment `seq` (`len` bytes) of `conn` at `t`, with
    /// its flow's state after it.
    fn traced(
        &mut self,
        t: Time,
        conn: (SocketAddr, SocketAddr),
        what: &'static str,
        seq: u64,
        len: usize,
    ) {
        if self.tcp_trace.is_none() {
            return;
        }
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let (cwnd, ssthresh, in_flight) = self
            .flows
            .get(&conn)
            .map_or((0.0, 0.0, 0), |f| (f.cwnd, f.ssthresh, f.in_flight));
        if let Some(tr) = self.tcp_trace.as_mut() {
            tr.push(TcpEvent {
                t,
                from,
                to,
                what,
                seq,
                len,
                cwnd,
                ssthresh,
                in_flight,
            });
        }
    }

    /// Traces `what` of competing upload `i`'s segment `seq` at `t`, its
    /// sink as host `names.len() + i`.
    fn traced_cross(&mut self, t: Time, i: usize, what: &'static str, seq: u64, len: usize) {
        let Some(tr) = self.tcp_trace.as_mut() else {
            return;
        };
        let Some(c) = self.cross.get(i) else {
            return;
        };
        tr.push(TcpEvent {
            t,
            from: c.host,
            to: self.names.len() + i,
            what,
            seq,
            len,
            cwnd: c.flow.cwnd,
            ssthresh: c.flow.ssthresh,
            in_flight: c.flow.in_flight,
        });
    }

    /// The base latency from `a` to `b` alone becomes `d`.
    pub fn set_latency_from(&mut self, a: HostIx, b: HostIx, d: Duration) {
        if a != b {
            self.net.set_one_way_from(a, b, Some(d));
        }
    }

    /// `h` lost power at `now`, before the walk that reaches it. What its
    /// data channels' stack did by then goes first
    /// ([`Scheduler::sctp_until_crash`]), so its uplink holds it as the
    /// crash finds it; then its access links forget their queues, so what
    /// its uplink had yet to send never leaves (its packet log's rows of it
    /// take the crash as their fate, [`PacketLoss::Crash`]) and reaches no
    /// peer, and its data channels forget the rest
    /// ([`Scheduler::sctp_forget`]), its downlink taking no copy of a chunk
    /// until its restart.
    pub fn crashed(&mut self, h: HostIx, now: Time) {
        self.sctp_until_crash(h, now);
        self.net.reset(h, now);
        if let Some(rows) = self.packets.get_mut(h) {
            let at = now - epoch();
            for r in rows.iter_mut().filter(|r| r.sent.is_some_and(|s| s > at)) {
                r.sent = None;
                r.arrived = None;
                r.lost = Some(PacketLoss::Crash);
            }
        }
        self.sctp_forget(h, now);
        if let Some(d) = self.down.get_mut(h) {
            *d = true;
        }
        if let Some(free) = self.threads.get_mut(h) {
            *free = (*free).min(now);
        }
        for d in [
            self.ack_out.get_mut(h),
            self.sack_out.get_mut(h),
            self.retx_out.get_mut(h),
            self.drop_out.get_mut(h),
        ]
        .into_iter()
        .flatten()
        {
            d.forget(now);
        }
        let ip_host = &self.ip_host;
        self.flows.retain(|(src, dst), _| {
            ip_host.get(&src.ip()) != Some(&h) && ip_host.get(&dst.ip()) != Some(&h)
        });
        self.owed.retain(|(src, dst), _| {
            ip_host.get(&src.ip()) != Some(&h) && ip_host.get(&dst.ip()) != Some(&h)
        });
    }

    /// `h` restarted after its crash ([`Scheduler::crashed`]). Its peers'
    /// data channels with it from before the crash go on sending to it,
    /// and its new stack takes none of their packets.
    pub fn restarted(&mut self, h: HostIx) {
        if let Some(d) = self.down.get_mut(h) {
            *d = false;
        }
    }

    /// `h`'s link charged `charge` bytes for what it resent.
    fn resent(&mut self, h: usize, charge: usize) {
        if let Some(b) = self.resent_bytes.get_mut(h) {
            *b += charge as u64;
        }
    }

    /// `h` sent `app` bytes its transport took from it.
    fn sent(&mut self, h: usize, app: usize) {
        if let Some(b) = self.app_bytes.get_mut(h) {
            *b += app as u64;
        }
    }

    /// `h`'s uplink took a pure ACK that its link charged `wire` bytes
    /// ([`Network::up_charge`]: its IP bytes, and its framing at a site that
    /// charges it), handed to it at `at`, its last bit out at `depart`.
    fn ack_sent(&mut self, h: usize, wire: usize, at: Time, depart: Time) {
        self.count_ack(h, wire);
        let settled = self.settled(at);
        if let Some(d) = self.ack_out.get_mut(h) {
            d.add(settled, depart, alone(wire));
        }
    }

    /// `h`'s uplink took a SACK alone, as [`Scheduler::ack_sent`] takes a
    /// pure ACK: an acknowledgement the tables tell apart.
    fn sack_sent(&mut self, h: usize, wire: usize, at: Time, depart: Time) {
        self.count_ack(h, wire);
        if let (Some(n), Some(b)) = (self.sacks.get_mut(h), self.sack_bytes.get_mut(h)) {
            *n += 1;
            *b += wire as u64;
        }
        let settled = self.settled(at);
        if let Some(d) = self.sack_out.get_mut(h) {
            d.add(settled, depart, alone(wire));
        }
    }

    /// Counts an acknowledgement alone of `wire` charged bytes among `h`'s.
    fn count_ack(&mut self, h: usize, wire: usize) {
        if let (Some(n), Some(b)) = (self.acks.get_mut(h), self.ack_bytes.get_mut(h)) {
            *n += 1;
            *b += wire as u64;
        }
    }

    /// `h` resent, at `depart`, stream data its path had lost of a message
    /// handed to its uplink at `at`.
    fn retransmitted(&mut self, h: usize, at: Time, depart: Time) {
        let settled = self.settled(at);
        if let Some(d) = self.retx_out.get_mut(h) {
            let one = Carried {
                packets: 1,
                ..Carried::default()
            };
            d.add(settled, depart, one);
        }
    }

    /// `h` lost `packets` frames at `at` at a full queue, its uplink's or
    /// their receiver's downlink's, as they came to it.
    fn queue_lost(&mut self, h: usize, at: Time, packets: u64) {
        if let Some(d) = self.drop_packets.get_mut(h) {
            *d += packets;
        }
        self.count_lost(h, at, packets);
    }

    /// `h` lost `packets` packets at `at`, which count in its drops from
    /// then on ([`Scheduler::up_counters`]): at a full queue as they came to
    /// it, or a datagram on its path as it was handed over. A drop counts in
    /// the bin it came in, as the packets its uplink carried do, and one
    /// past a crash never counts ([`Scheduler::crashed`]), which the
    /// report's totals keep.
    fn count_lost(&mut self, h: usize, at: Time, packets: u64) {
        let settled = self.settled(at);
        if let Some(d) = self.drop_out.get_mut(h) {
            let lost = Carried {
                packets,
                ..Carried::default()
            };
            d.add(settled, at, lost);
        }
    }

    /// The hand-over time a ledger takes for one at `at`: no later than the
    /// walk's, at or after which every bin edge is read. A walk hands
    /// nothing over past its time, so this is a guard, which moves only a
    /// crash's hand-overs past the last walk ([`Scheduler::crashed`]).
    fn settled(&self, at: Time) -> Time {
        if self.walked.is_nil() {
            at
        } else {
            at.min(self.walked)
        }
    }

    /// Host `h`'s uplink counters at `at` ([`UP_COUNTERS`]), no earlier
    /// than the last walk's time: what its uplink carried before `at`, each
    /// packet from when it left, and the rest as they stand.
    pub fn up_counters(&self, h: HostIx, at: Time) -> [u64; UP_COUNTERS.len()] {
        let n = |v: &Vec<u64>| v.get(h).copied().unwrap_or(0);
        let out = |v: &Vec<Departures>| v.get(h).map_or_else(Carried::default, |d| d.before(at));
        let (up, acks, sacks) = (
            self.net.up_carried(h, at),
            out(&self.ack_out),
            out(&self.sack_out),
        );
        [
            up.bytes,
            up.packets,
            up.small_packets,
            out(&self.retx_out).packets,
            out(&self.drop_out).packets,
            n(&self.cuts),
            n(&self.rtos),
            acks.packets,
            acks.bytes,
            sacks.packets,
            sacks.bytes,
        ]
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
                        up_backlog: self.up_backlog.get(h).cloned().unwrap_or_default(),
                        up_queue_until: None,
                        competing: self.cross_sent.get(h).cloned().unwrap_or_default(),
                        competing_log: self.cross_log.get(h).cloned().unwrap_or_default(),
                        queue_drops: self.drops.get(h).copied().unwrap_or(0),
                        queue_drop_packets: self.drop_packets.get(h).copied().unwrap_or(0),
                        acks: self.acks.get(h).copied().unwrap_or(0),
                        ack_bytes: self.ack_bytes.get(h).copied().unwrap_or(0),
                        ack_whys: self.ack_whys.get(h).copied().unwrap_or_default(),
                        ack_waits: self.ack_waits.get(h).cloned().unwrap_or_default(),
                        app_bytes: self.app_bytes.get(h).copied().unwrap_or(0),
                        competing_drops: self.cross_drops.get(h).copied().unwrap_or(0),
                        competing_drop_packets: self
                            .cross_drop_packets
                            .get(h)
                            .copied()
                            .unwrap_or(0),
                        competing_cwnd: self.cross_cwnd.get(h).cloned().unwrap_or_default(),
                        packets: self.packets.get(h).cloned().unwrap_or_default(),
                        sctp: self
                            .sctp_events
                            .get(h)
                            .map(|m| {
                                m.iter()
                                    .map(|(&k, v)| {
                                        let mut v = v.clone();
                                        v.sort_by_key(|e| e.at);
                                        (k, v)
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        offers: self
                            .offers
                            .as_ref()
                            .map(|o| o.get(h).cloned().unwrap_or_default()),
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
    use crate::report::WanHostReport;
    use crate::spec::{Competing, HostSpec, SendBuffer};
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

    /// A flow that filled a window of `segs` segments of `seg` bytes, all
    /// in flight, and learnt the first was lost: its recovery begins.
    fn recovering(segs: u64, seg: usize) -> Flow {
        let mut f = Flow {
            cwnd: (segs as usize * seg) as f64,
            sent_end: segs,
            in_flight: segs as usize * seg,
            ..Flow::default()
        };
        for seq in 0..segs {
            f.sent_at.insert(seq, Time::EPOCH);
        }
        f.enter_recovery();
        f
    }

    /// Sends what `f`'s window lets out in segments of `seg` bytes,
    /// returning how many.
    fn send_allowed(f: &mut Flow, seg: usize) -> usize {
        let mut n = 0;
        while !f.holds(seg) {
            f.in_flight += seg;
            f.recovery_sent(seg);
            n += 1;
        }
        n
    }

    #[test]
    fn a_recovery_sends_the_thresholds_share_of_what_is_delivered() {
        // `tcp_cwnd_reduction` while more than ssthresh is in flight:
        // having delivered d bytes the sender has sent
        // ceil(ssthresh * d / prior_cwnd), CUBIC's ssthresh 717/1024 of it.
        let seg = MSS as usize;
        let mut f = recovering(100, seg);
        assert!((f.ssthresh - 70.0 * MSS).abs() < 1e-6, "{}", f.ssthresh);
        // The lost segment leaves the flight and goes again at once,
        // whatever the window: the forced fast retransmit.
        f.in_flight -= seg;
        f.cwnd = 0.0;
        assert_eq!(send_allowed(&mut f, seg), 1);
        let mut delivered = 0usize;
        let mut out = 1;
        while f.in_flight - seg > f.ssthresh as usize {
            f.in_flight -= seg;
            delivered += 1;
            f.reduction(seg, false, false);
            out += send_allowed(&mut f, seg);
            let linux = (70 * delivered).div_ceil(100).max(1);
            assert_eq!(out, linux, "sent after {delivered} packets delivered");
        }
        assert!(f.recovery.is_some());
    }

    #[test]
    fn a_later_acknowledgement_shows_a_probes_loss_at_once() {
        // A tail-loss probe finds segment 0 lost and learns so a round trip
        // on, as its acknowledgement's SACK would; the acknowledgements of
        // two segments sent after it, arriving before then, show it lost at
        // once (RACK, `tcp_rack_detect_loss`) and start the recovery there.
        let ms = |n: i64| Duration::from_millis(n);
        let mut sim = build();
        let sites = [
            ("a", Site::new(City::Frankfurt, Access::Fiber)),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        sim.host("a", || async { Ok(()) });
        sim.client("b", async { Ok(()) });
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        let conn: (SocketAddr, SocketAddr) = (
            "192.0.2.1:9000".parse().unwrap(),
            "192.0.2.2:40000".parse().unwrap(),
        );
        let seg = 2 * MSS as usize;
        let t0 = Time::EPOCH + ms(1000);
        let f = wan.flows.entry(conn).or_default();
        f.srtt = ms(30);
        f.rttvar = ms(5);
        f.min_rtt = ms(30);
        for (seq, at) in [(0, t0), (1, t0 + ms(1)), (2, t0 + ms(1))] {
            f.sent(at, seg, ms(30));
            f.sent_at.insert(seq, at);
        }
        f.arm(t0 + ms(1), false, ms(30));
        assert!(f.tlp);
        let probe = f.timer_at;
        wan.dropped(t0, conn, 0);
        wan.probe(probe, conn);
        let f = wan.flows.get(&conn).unwrap();
        assert!(f.recovery.is_none());
        wan.acked(probe + ms(5), conn, 1, seg);
        wan.acked(probe + ms(5), conn, 2, seg);
        let f = wan.flows.get(&conn).unwrap();
        assert!(
            f.recovery.is_some(),
            "no recovery at the later acknowledgements"
        );
        assert!(!f.lost.contains_key(&0) && f.resend.contains(&0));
    }

    #[test]
    fn a_recovery_sends_the_ceiling_of_its_share_from_the_first_packet() {
        // Without the forced retransmission first: after k packets
        // delivered past the threshold the sender has sent
        // max(ceil(ssthresh * k / prior_cwnd), 1).
        let seg = MSS as usize;
        let mut f = recovering(100, seg);
        f.in_flight -= seg;
        let mut out = 0;
        for k in 1..=20usize {
            f.in_flight -= seg;
            f.reduction(seg, false, false);
            out += send_allowed(&mut f, seg);
            assert_eq!(out, (70 * k).div_ceil(100).max(1), "after {k} delivered");
        }
    }

    #[test]
    fn a_recovery_never_sets_the_window_under_the_flight() {
        // `sndcnt` is at least 0 once anything went (`prr_out`): a
        // recovery whose forced retransmission is a two-packet segment,
        // then an acknowledgement of one packet past the threshold.
        let seg = MSS as usize;
        let mut f = recovering(100, seg);
        f.in_flight -= 2 * seg;
        f.reduction(seg, false, true);
        assert!(!f.holds(2 * seg), "the forced retransmission goes");
        f.in_flight += 2 * seg;
        f.recovery_sent(2 * seg);
        f.in_flight -= seg;
        f.reduction(seg, false, false);
        assert!(
            f.cwnd >= f.in_flight as f64,
            "cwnd {} under the flight {}",
            f.cwnd,
            f.in_flight
        );
    }

    #[test]
    fn a_loss_a_timer_shows_arms_the_retransmission_timeout() {
        // A burst arms the tail-loss probe 2 srtt + 2 ms on; a loss RACK's
        // timer shows just before it starts a recovery, which arms the
        // retransmission timeout from the oldest segment's send
        // (`tcp_rearm_rto`), not the probe's time.
        let ms = |n: i64| Duration::from_millis(n);
        let t0 = Time::EPOCH + ms(1000);
        let mut f = Flow {
            srtt: ms(30),
            rttvar: ms(5),
            ..Flow::default()
        };
        for seq in 0..10 {
            f.sent(t0, MSS as usize, ms(30));
            f.sent_at.insert(seq, t0);
        }
        f.arm(t0, false, ms(30));
        assert_eq!(f.timer_at, t0 + ms(62));
        assert!(f.tlp);
        f.recover_at(t0 + ms(61));
        assert!(!f.tlp);
        assert_eq!(f.timer_at, t0 + f.rto());
    }

    #[test]
    fn a_window_counts_packets_whatever_their_bytes() {
        // Linux's window of 10 packets lets 10 of a write's 1200-byte
        // tails out, not the 12 its bytes would.
        let mut f = Flow::default();
        let mut sent = 0;
        while !f.holds(1200) {
            f.sent(Time::EPOCH, 1200, StdDuration::from_millis(30).into());
            sent += 1;
        }
        assert_eq!(sent, 10);
        // A two-packet segment takes two.
        let mut f = Flow::default();
        f.sent(
            Time::EPOCH,
            1448 + 1200,
            StdDuration::from_millis(30).into(),
        );
        assert_eq!(f.in_flight, 2 * MSS as usize);
    }

    #[test]
    fn the_tcp_friendly_estimate_grows_with_the_packets_acknowledged() {
        // At the window lost last CUBIC's curve starts flat; its
        // TCP-friendly estimate grows a packet for each window times 15 / 8
        // packets acknowledged, however little time passes
        // (`tcp_friendliness`, `ack_cnt`).
        let ms = |n: i64| Duration::from_millis(n);
        let t = Time::EPOCH + ms(1000);
        let mut f = Flow {
            cwnd: 40.0 * MSS,
            ssthresh: 40.0 * MSS,
            w_max: 40.0,
            ..Flow::default()
        };
        let seg = 2 * MSS as usize;
        for _ in 0..37 {
            f.grow(t, seg, ms(30));
        }
        assert!(f.cwnd < 40.1 * MSS, "{} packets after 74", f.cwnd / MSS);
        for _ in 37..80 {
            f.grow(t, seg, ms(30));
        }
        assert!(f.cwnd >= 41.0 * MSS, "{} packets after 160", f.cwnd / MSS);
    }

    #[test]
    fn hystart_rounds_take_the_samples_of_their_own_segments() {
        // An upload that starts just before another's burst: its first
        // round comes back at the path's round trip and its second behind
        // the burst. HyStart's second round begins with the second's first
        // segment, though the window reached its low window during the
        // first, so slow start ends at the second's ninth sample, a window
        // of 36 packets; a round begun at the window's 16 packets would
        // hold the first round's last samples and run to 48.
        let ms = |n: i64| Duration::from_millis(n);
        let seg = 2 * MSS as usize;
        let mut f = Flow::default();
        let t = Time::EPOCH + ms(1000);
        let send = |f: &mut Flow| {
            while !f.holds(seg) {
                f.sent(t, seg, ms(30));
                f.sent_end += 2;
            }
        };
        send(&mut f);
        let mut seq = 0;
        while f.ssthresh.is_infinite() && seq < 400 {
            f.sampled(seq, if seq < 10 { ms(30) } else { ms(80) });
            f.in_flight -= seg;
            f.grow(t, seg, ms(30));
            send(&mut f);
            seq += 2;
        }
        assert_eq!(
            f.ssthresh / MSS,
            36.0,
            "slow start ended at the ack of {seq}"
        );
    }

    #[test]
    fn the_transmit_timer_rearms_on_each_send_and_acknowledgement() {
        // `tcp_schedule_loss_probe`: two smoothed round trips and 2 ms on
        // from a send with more than a packet out, no later than what is
        // left of the oldest segment's timeout; from an acknowledgement,
        // RTO_MIN more with one packet out, no later than a full timeout.
        let ms = |n: i64| Duration::from_millis(n);
        let t0 = Time::EPOCH + ms(1000);
        let mut f = Flow {
            srtt: ms(100),
            rttvar: ms(10),
            ..Flow::default()
        };
        let rto = f.rto();
        assert_eq!(rto, ms(300));
        for seq in 0..2 {
            f.sent(t0, MSS as usize, ms(30));
            f.sent_at.insert(seq, t0);
        }
        f.arm(t0, false, ms(30));
        assert_eq!(f.timer_at, t0 + ms(202));
        // A send 250 ms on leaves the oldest segment's timeout 50 ms.
        f.sent(t0 + ms(250), MSS as usize, ms(30));
        f.sent_at.insert(2, t0 + ms(250));
        f.arm(t0 + ms(250), false, ms(30));
        assert_eq!(f.timer_at, t0 + ms(300));
        // The first two acknowledged: one packet out, the timeout caps it.
        f.in_flight -= 2 * MSS as usize;
        f.sent_at.remove(&0);
        f.sent_at.remove(&1);
        f.arm(t0 + ms(260), true, ms(30));
        assert_eq!(f.timer_at, t0 + ms(260) + rto);
        // In a recovery an acknowledgement arms the timeout: what is left
        // of the oldest segment's where the probe was pending
        // (`tcp_rearm_rto`), a full one on after that; a send leaves it,
        // unless none is pending.
        f.enter_recovery();
        f.arm(t0 + ms(270), true, ms(30));
        assert_eq!(f.timer_at, t0 + ms(250) + rto);
        f.arm(t0 + ms(275), true, ms(30));
        assert_eq!(f.timer_at, t0 + ms(275) + rto);
        f.arm(t0 + ms(280), false, ms(30));
        assert_eq!(f.timer_at, t0 + ms(275) + rto);
        f.timer_at = Time::NIL;
        f.arm(t0 + ms(285), false, ms(30));
        assert_eq!(f.timer_at, t0 + ms(285) + rto);
        // With nothing in flight, none.
        f.in_flight = 0;
        f.arm(t0 + ms(290), true, ms(30));
        assert!(f.timer_at.is_nil());
    }

    #[test]
    fn under_the_threshold_a_recovery_grows_a_packet_an_advancing_ack() {
        // Under ssthresh, `tcp_cwnd_reduction` sends what was delivered and
        // a packet more for each acknowledgement that advanced snd_una and
        // marked no loss, up to ssthresh: the receiver acknowledges each
        // packet of a two-packet segment, so the flight grows a segment for
        // each such segment acknowledged.
        let seg = crate::spec::WINDOW_SEGMENT;
        let mut f = recovering(100, seg);
        let ssthresh_segs = (f.ssthresh as usize).div_ceil(seg);
        f.in_flight = 20 * seg;
        for k in 1..=60 {
            f.in_flight -= seg;
            f.reduction(seg, true, false);
            send_allowed(&mut f, seg);
            let segs = f.in_flight / seg;
            assert!(
                f.in_flight as f64 <= f.ssthresh,
                "ack {k}: {segs} segments in flight"
            );
            assert!(
                segs + 1 >= (20 + k).min(ssthresh_segs),
                "ack {k}: {segs} segments in flight"
            );
        }
        // An acknowledgement that marks a loss sends only what it delivered.
        let mut f = recovering(100, seg);
        f.in_flight = 20 * seg;
        for _ in 0..10 {
            f.in_flight -= seg;
            f.reduction(seg, true, true);
            send_allowed(&mut f, seg);
        }
        assert_eq!(f.in_flight, 20 * seg);
    }

    #[test]
    fn a_recovery_ends_at_its_threshold() {
        // Once the cumulative ack passes what was sent when it began the
        // window is the threshold (`tcp_end_cwnd_reduction`), and a
        // recovery begun from there cuts it by CUBIC's beta.
        let seg = MSS as usize;
        let mut f = recovering(100, seg);
        let ssthresh = f.ssthresh;
        f.sent_end = 140;
        f.sent_at.clear();
        f.sent_at.insert(99, Time::EPOCH);
        f.sent_at.insert(120, Time::EPOCH);
        f.recovered();
        assert!(f.recovery.is_some(), "the ack point is short of high_seq");
        f.sent_at.remove(&99);
        f.recovered();
        assert!(f.recovery.is_none());
        assert_eq!(f.cwnd, ssthresh);
        f.enter_recovery();
        assert!((f.ssthresh - 0.7 * ssthresh).abs() < 1e-6, "{}", f.ssthresh);
        assert_eq!(f.recovery.map(|r| r.high), Some(140));
    }

    #[test]
    fn a_resent_tail_cut_again_and_lost_goes_again_from_where_it_starts() {
        // A segment's tail resent alone, the uplink queue taking only its
        // first packet, which the receiver's downlink then loses: what
        // goes again is the tail from where it started, not the head the
        // receiver had.
        let mut f = Flow::default();
        f.tails.insert(
            7,
            Tail {
                head: 1448,
                tail: 1200,
                resent: true,
            },
        );
        f.head_lost(7, 1448);
        let tl = f.tails.get(&7).copied().unwrap();
        assert_eq!((tl.head, tl.tail, tl.resent), (0, 2648, false));
        // A segment's own head lost after its tail: lost whole.
        f.head_lost(7, 0);
        assert!(!f.tails.contains_key(&7));
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
    fn a_syn_ack_waits_behind_its_acceptors_uplink() {
        let mut sim = build();
        let up = 125_000; // 1 Mbit/s
        let sites = [
            ("a", Site::new(City::Frankfurt, Access::Fiber).with_up(up)),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
            ("c", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        // a writes its first connection 125 000 bytes at once, a second
        // of its uplink; its second connects in the middle of it.
        sim.host("a", || async {
            let l = TcpListener::bind("0.0.0.0:9000").await?;
            let (mut s, _) = l.accept().await?;
            s.write_all(&[1u8; 125_000]).await?;
            let (_held, _) = l.accept().await?;
            std::future::pending::<()>().await;
            Ok(())
        });
        sim.client("c", async move {
            let mut s = TcpStream::connect(("a", 9000)).await?;
            let mut buf = vec![0u8; 125_000];
            s.read_exact(&mut buf).await?;
            Ok(())
        });
        let connect = Rc::new(Cell::new(0));
        let got = connect.clone();
        sim.client("b", async move {
            tokio::time::sleep(StdDuration::from_millis(200)).await;
            let t0 = turmoil::sim_elapsed().unwrap();
            let _s = TcpStream::connect(("a", 9000)).await?;
            got.set(ms(turmoil::sim_elapsed().unwrap() - t0));
            Ok(())
        });
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        let (rtt, ab) = (
            wan.net.rtt(0, 2).as_millis(),
            wan.net.one_way(0, 1).as_millis(),
        );
        drive(&mut sim, &mut wan);
        // c's 87 segments, 129 524 bytes on the wire, leave a's uplink by
        // 1036 ms after their write, a round trip from the start; b's
        // SYN-ACK leaves after them.
        let free = rtt + 1036;
        let want = free + ab - 200;
        let took = connect.get();
        assert!(
            (want..want + 5).contains(&took),
            "connect {took} ms, want {want}"
        );
    }

    /// `b` connects to `a` on `port` and writes 1000 bytes, which `a`
    /// reads, both hosts' packets logged; with `window`, under the window
    /// model, and with `rtc`, `a` a host whose WebRTC connections are the
    /// model's data channels. Returns each host's packet log.
    fn handshake(window: bool, rtc: bool, port: u16) -> [Vec<PacketRecord>; 2] {
        let mut sim = build();
        let sites = [
            ("a", Site::new(City::Frankfurt, Access::Fiber)),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        let buffers = window.then(|| {
            SendBuffers::new(SendBuffer {
                uplink_queue: StdDuration::from_millis(100),
                window: true,
            })
        });
        let net = sim_net(buffers.clone());
        let n = net.clone();
        sim.host("a", move || {
            let net = n.clone();
            async move {
                let l = net.tcp_bind(format!("0.0.0.0:{port}")).await?;
                let (mut s, _) = net.tcp_accept(&l).await?;
                let mut buf = [0u8; 1000];
                s.read_exact(&mut buf).await?;
                std::future::pending::<()>().await;
                Ok(())
            }
        });
        sim.client("b", async move {
            let mut s = net.tcp_connect(format!("a:{port}")).await?;
            s.write_all(&[7u8; 1000]).await?;
            tokio::time::sleep(StdDuration::from_millis(500)).await;
            Ok(())
        });
        let mut s = spec(&sites);
        s.packet_log = BTreeSet::from([0, 1]);
        if window {
            s.wan = Some(QUIET);
            s.send_buffer = Some(SendBuffer {
                uplink_queue: StdDuration::from_millis(100),
                window: true,
            });
        }
        if rtc {
            s.hosts[0].webrtc = true;
            s.hosts[0].rtc_stack = Some(SctpProfile::CHROMIUM);
        }
        let mut wan = Scheduler::new(&sim, &s, QUIET, 1, buffers).unwrap();
        drive(&mut sim, &mut wan);
        [wan.packets[0].clone(), wan.packets[1].clone()]
    }

    /// The kinds and IP bytes of a packet log's first `n` rows.
    fn first_rows(rows: &[PacketRecord], n: usize) -> Vec<(PacketKind, PacketProto, u32)> {
        rows.iter()
            .take(n)
            .map(|r| (r.kind, r.proto, r.ip_bytes))
            .collect()
    }

    #[test]
    fn a_connect_is_a_syn_a_syn_ack_and_with_windows_an_ack_ahead_of_its_data() {
        use PacketKind::{Ack, Control, Data};
        use PacketProto::Tcp;
        let syn = TCP_SYN as u32;
        let ack = tcp_wire(0).0 as u32;
        let data = tcp_wire(1000).0 as u32;
        for window in [true, false] {
            let [a, b] = handshake(window, false, 9000);
            // The acceptor's first packet is its SYN-ACK, handed to its
            // uplink as the SYN reached it and logged as the SYN is.
            let (b_syn, a_syn_ack) = (&b[0], &a[0]);
            assert_eq!(first_rows(&a, 1), [(Control, Tcp, syn)], "window {window}");
            assert_eq!((a_syn_ack.peer, a_syn_ack.link), (Some(1), Some(0)));
            let reached = b_syn.arrived.unwrap();
            assert!(
                a_syn_ack.at >= reached && a_syn_ack.at < reached + Duration::from_millis(1),
                "{a_syn_ack:?} {b_syn:?}"
            );
            assert!(a_syn_ack.sent.is_some() && a_syn_ack.arrived > a_syn_ack.sent);
            if window {
                // The connector's handshake ACK goes as the SYN-ACK comes,
                // ahead of its data, a pure ACK of its counters.
                assert_eq!(
                    first_rows(&b, 3),
                    [(Control, Tcp, syn), (Ack, Tcp, ack), (Data, Tcp, data)]
                );
                assert_eq!(Some(b[1].at), a_syn_ack.arrived);
                assert!(b[2].at > b[1].at);
                assert_eq!(b.iter().filter(|r| r.kind == Ack).count(), 1);
            } else {
                // Without the window model no pure ACK goes.
                assert_eq!(first_rows(&b, 2), [(Control, Tcp, syn), (Data, Tcp, data)]);
                assert!(b.iter().all(|r| r.kind != Ack));
            }
        }
    }

    #[test]
    fn an_ice_checks_reply_is_a_stun_response_and_takes_no_ack() {
        use PacketKind::{Ack, Control, Data};
        use PacketProto::{Tcp, Udp};
        let syn = TCP_SYN as u32;
        let data = tcp_wire(1000).0 as u32;
        let [a, b] = handshake(true, true, UDP_PORT);
        assert_eq!(first_rows(&a, 1), [(Control, Udp, STUN_RESPONSE as u32)]);
        assert_eq!(first_rows(&b, 2), [(Control, Tcp, syn), (Data, Tcp, data)]);
        assert!(b.iter().all(|r| r.kind != Ack));
        // A connection to another port of the host is no check, and one to
        // that port of a host without modelled channels neither.
        for (rtc, port) in [(true, 9000), (false, UDP_PORT)] {
            let [a, b] = handshake(true, rtc, port);
            assert_eq!(first_rows(&a, 1), [(Control, Tcp, syn)], "{rtc} {port}");
            assert_eq!(b.get(1).map(|r| r.kind), Some(Ack), "{rtc} {port}");
        }
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
        // on the wire, 1.2 s at 1 Mbit/s; and the acceptor's SYN-ACK and
        // FIN.
        let took = got.get();
        assert!((1190..1215).contains(&took), "{took} ms");
        assert_eq!(
            wan.report(&spec(&sites)).hosts[0].stats.up_bytes,
            150_000 + TCP_SYN as u64 + 52
        );
    }

    /// [`SimNet`] with `buffers`, its writes cut to the window model's
    /// segments when they have the model, as scenarios cut them.
    fn sim_net(buffers: Option<SendBuffers>) -> SimNet {
        let window = buffers.as_ref().is_some_and(|b| b.model().window);
        SimNet::new(buffers).with_segment(window.then_some(crate::spec::WINDOW_SEGMENT))
    }

    /// A 1 Mbit/s sender, its spec adjusted by `shape`, writes 144 800
    /// bytes through [`SimNet`] with `buffers`, and `b` reads them. Returns
    /// when the last write returned, when the read finished (ms from the
    /// connect) and the sender's uplink queue peak.
    fn buffered_writes(
        buffers: Option<SendBuffers>,
        shape: impl FnOnce(&mut HostSpec),
    ) -> (i64, i64, Duration) {
        buffered_report(buffers, shape).0
    }

    /// [`buffered_writes`] and the sender's access link report.
    fn buffered_report(
        buffers: Option<SendBuffers>,
        shape: impl FnOnce(&mut HostSpec),
    ) -> ((i64, i64, Duration), WanHostReport) {
        let mut sim = build();
        let sites = [
            (
                "a",
                Site::new(City::Frankfurt, Access::Fiber).with_up(125_000),
            ),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        let (wrote, got) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let net = sim_net(buffers.clone());
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
        let mut spec = spec(&sites);
        shape(&mut spec.hosts[0]);
        let mut wan = Scheduler::new(&sim, &spec, QUIET, 1, buffers).unwrap();
        drive(&mut sim, &mut wan);
        let a = wan.report(&spec).hosts.swap_remove(0);
        ((wrote.get(), got.get(), a.stats.up_queue_max), a)
    }

    #[test]
    fn send_buffers_hold_writes_until_the_uplink_queue_has_room() {
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: false,
        });
        let (wrote, got, peak) = buffered_writes(Some(buffers), |_| {});
        // The writes run ahead of the uplink by its 100 ms queue and what the
        // socket holds: under the 16 KiB mark a write starts a buffer that
        // later writes fill to a GSO segment, and a writer that waits resumes
        // under half the mark. The last write goes about 0.46 s before the
        // 1.2 s the bytes take on the wire end.
        assert!((700..800).contains(&wrote), "last write after {wrote} ms");
        assert!((1190..1215).contains(&got), "{got} ms");
        // A segment waits at the uplink for at most the queue, a GSO segment
        // (0.51 s at 1 Mbit/s) and the write that started it.
        assert!(peak <= Duration::from_millis(710), "{peak:?}");
    }

    #[test]
    fn windows_keep_the_uplink_queue_bounded() {
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        let (wrote, got, peak) = buffered_writes(Some(buffers), |_| {});
        // The window, not the queue, paces the writes: the bytes take their
        // 1.2 s on the wire, and a segment waits at most the queue, as the
        // frames that don't fit are lost. The last segment's second frame
        // meets the queue full, and goes again on the tail-loss probe, two
        // round trips and RTO_MIN after its send: the last bytes come about
        // 1.6 s in.
        assert!(wrote > 700, "last write after {wrote} ms");
        assert!((1190..1700).contains(&got), "{got} ms");
        assert!(peak <= Duration::from_millis(100), "{peak:?}");
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
        let net = sim_net(Some(buffers.clone()));
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
        // downlink, and `b` gets more of the uplink (about 1.9 s against
        // 2.2: past its threshold its window grows with the segments it
        // has acknowledged).
        assert!(b_fifo > 2000, "{b_fifo} ms");
        assert!(b < b_fifo * 9 / 10, "{b} ms against {b_fifo} ms");
        assert!(c >= 5800 && c_fifo >= 5800, "{c} and {c_fifo} ms");
    }

    #[test]
    fn a_stream_follows_its_uplinks_steps() {
        // The uplink halves 300 ms into the run, after about 36 of the
        // 150 kB on the wire (the connect takes the first few ms): the
        // rest take 1.8 s at 62.5 kB/s.
        let halved = |h: &mut HostSpec| h.up_steps = vec![(StdDuration::from_millis(300), 62_500)];
        let (_, got, _) = buffered_writes(None, halved);
        assert!((2100..2135).contains(&got), "{got} ms");
        for window in [false, true] {
            let buffers = SendBuffers::new(SendBuffer {
                uplink_queue: StdDuration::from_millis(100),
                window,
            });
            let (_, got, _) = buffered_writes(Some(buffers), halved);
            assert!((2100..2500).contains(&got), "window {window}: {got} ms");
        }
        // Back to the full rate at 600 ms, after 18.75 kB more at the half
        // rate: the other 95 kB take 0.76 s.
        let back = |h: &mut HostSpec| {
            h.up_steps = vec![
                (StdDuration::from_millis(300), 62_500),
                (StdDuration::from_millis(600), 125_000),
            ];
        };
        let (_, got, _) = buffered_writes(None, back);
        assert!((1345..1380).contains(&got), "{got} ms");
    }

    /// `a`, on a 10 Mbit/s uplink with 100 ms windowed queues, a competing
    /// upload over `competing` (s) and each packet lost with probability
    /// `loss`, writes `bytes` to `b` from 1 s on. Returns when `b` had them
    /// (ms from 1 s) and `a`'s access link report.
    fn beside_an_upload(bytes: usize, competing: (u64, u64), loss: f64) -> (i64, WanHostReport) {
        beside_an_upload_drawn(bytes, competing, loss, 1)
    }

    /// [`beside_an_upload`] with the WAN's draws from `seed`.
    fn beside_an_upload_drawn(
        bytes: usize,
        competing: (u64, u64),
        loss: f64,
        seed: u64,
    ) -> (i64, WanHostReport) {
        let (got, a, _, _) = beside_an_upload_traced(bytes, competing, loss, seed);
        (got, a)
    }

    /// [`beside_an_upload_drawn`], the window model's TCP trace, the
    /// upload's sink host 2, and `a`'s uplink counters at the end.
    fn beside_an_upload_traced(
        bytes: usize,
        competing: (u64, u64),
        loss: f64,
        seed: u64,
    ) -> (i64, WanHostReport, Vec<TcpEvent>, [u64; UP_COUNTERS.len()]) {
        let (mut sim, mut wan, spec, got) = upload(bytes, (0, competing), loss, None, seed);
        drive(&mut sim, &mut wan);
        let trace = wan.take_tcp_trace();
        let counters = wan.up_counters(0, Time::MAX);
        (
            got.get(),
            wan.report(&spec).hosts.swap_remove(0),
            trace,
            counters,
        )
    }

    /// `a` (host 0) writing `bytes` to `b` (host 1) from 1 s in the window
    /// model, behind 100 ms uplink queues: `a`'s uplink at 10 Mbit/s,
    /// losing `loss`, `b`'s at `b_up` bytes a second (else its site's), and
    /// a competing upload on host `compete.0` over the seconds
    /// `compete.1`, the WAN's draws from `seed` and its TCP trace on.
    /// Returns the simulation, its scheduler and scenario, and when `b` had
    /// read it all (ms from 1 s).
    fn upload(
        bytes: usize,
        compete: (HostIx, (u64, u64)),
        loss: f64,
        b_up: Option<u64>,
        seed: u64,
    ) -> (Sim<'static>, Scheduler, Scenario, Rc<Cell<i64>>) {
        let mut sim = build();
        let b = Site::new(City::Frankfurt, Access::Fiber).with_loss(0.0);
        let sites = [
            (
                "a",
                Site::new(City::Frankfurt, Access::Fiber)
                    .with_up(1_250_000)
                    .with_loss(loss),
            ),
            ("b", b_up.map_or(b, |up| b.with_up(up))),
        ];
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        let net = sim_net(Some(buffers.clone()));
        sim.host("a", move || {
            let net = net.clone();
            async move {
                let l = net.tcp_bind("0.0.0.0:9000".into()).await?;
                let (mut s, _) = net.tcp_accept(&l).await?;
                net.configure_tcp_send_queue(&s, &TcpSocketConfig::default())?;
                for _ in 0..bytes / 14_480 {
                    s.write_all(&[1u8; 14_480]).await?;
                }
                Ok(())
            }
        });
        let got = Rc::new(Cell::new(0));
        let g = got.clone();
        sim.client("b", async move {
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            let mut s = TcpStream::connect(("a", 9000)).await?;
            if bytes > 0 {
                let mut buf = vec![0u8; bytes / 14_480 * 14_480];
                s.read_exact(&mut buf).await?;
            }
            g.set(ms(turmoil::sim_elapsed().unwrap()) - 1000);
            tokio::time::sleep(StdDuration::from_secs(2)).await;
            Ok(())
        });
        let mut spec = spec(&sites);
        let (host, (start, stop)) = compete;
        spec.hosts[host].competing = vec![Competing {
            start: StdDuration::from_secs(start),
            stop: StdDuration::from_secs(stop),
            rtt: StdDuration::from_millis(30),
        }];
        let options = Options {
            jitter: false,
            loss: loss > 0.0,
        };
        let mut wan = Scheduler::new(&sim, &spec, options, seed, Some(buffers)).unwrap();
        wan.set_latency(0, 1, Duration::from_millis(15));
        wan.trace_tcp();
        (sim, wan, spec, got)
    }

    /// The index of an [`UP_COUNTERS`] counter.
    fn up_ix(name: &str) -> usize {
        UP_COUNTERS.iter().position(|&c| c == name).unwrap()
    }

    #[test]
    fn a_pure_ack_counts_once_it_has_left_and_never_after_a_crash_queued_it() {
        // `b` acknowledges `a`'s writes while its own competing upload keeps
        // its 10 Mbit/s uplink's queue full, so its pure ACKs wait there:
        // read after each step's walk, the uplink's count holds those that
        // left, never more than it took, and fewer while some wait. A crash
        // while some wait forgets them: they never count.
        let (mut sim, mut wan, _, _) = upload(1_448_000, (1, (0, 30)), 0.0, Some(1_250_000), 1);
        let ack = up_ix("up_ack_packets");
        let took = |wan: &Scheduler| wan.acks[1] - wan.sacks[1];
        let mut waiting = 0;
        let (left, before) = loop {
            wan.walk(&sim).unwrap();
            let now = sim_time(sim.elapsed());
            let out = wan.up_counters(1, now)[ack];
            assert!(out <= took(&wan), "{out} of {} out at {now:?}", took(&wan));
            if out < took(&wan) {
                waiting += 1;
                if waiting == 100 {
                    let left = wan.up_counters(1, now + Duration::from_nanos(1))[ack];
                    let before = took(&wan);
                    wan.crashed(1, now);
                    break (left, before);
                }
            }
            assert!(
                !sim.step().unwrap(),
                "{waiting} steps with a pure ACK queued"
            );
        };
        assert!(left < before, "{left} of {before}");
        assert_eq!(wan.up_counters(1, Time::MAX)[ack], left);
    }

    /// What a step's write causes within the step counts by the walk's
    /// time: `a` reaches `b` at once, as the signalling relay on the hosts'
    /// bridge reaches its hosts, over fiber either way, so a segment it
    /// writes comes through `b`'s downlink within the step it was written
    /// in, and `b`'s pure ACK of it leaves then. Read after each step's
    /// walk, `b`'s uplink had carried every pure ACK its packet log says
    /// left before the walk's time, the last walk's ACKs among them.
    #[test]
    fn what_a_write_causes_within_its_step_counts_by_the_walks_time() {
        let mut sim = build();
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        let net = sim_net(Some(buffers.clone()));
        sim.host("a", move || {
            let net = net.clone();
            async move {
                let l = net.tcp_bind("0.0.0.0:9000".into()).await?;
                let (mut s, _) = net.tcp_accept(&l).await?;
                net.configure_tcp_send_queue(&s, &TcpSocketConfig::default())?;
                for _ in 0..20 {
                    s.write_all(&[1u8; 14_480]).await?;
                }
                Ok(())
            }
        });
        sim.client("b", async move {
            let mut s = TcpStream::connect(("a", 9000)).await?;
            let mut buf = vec![0u8; 20 * 14_480];
            s.read_exact(&mut buf).await?;
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            Ok(())
        });
        let mut s = spec(&[("a", fiber), ("b", fiber)]);
        s.packet_log = BTreeSet::from([1]);
        let mut wan = Scheduler::new(&sim, &s, QUIET, 1, Some(buffers)).unwrap();
        wan.set_latency_from(0, 1, Duration::ZERO);
        let ack = up_ix("up_ack_packets");
        let mut reads = Vec::new();
        loop {
            wan.walk(&sim).unwrap();
            let now = sim_time(sim.elapsed());
            reads.push((now, wan.up_counters(1, now)[ack]));
            if sim.step().unwrap() {
                break;
            }
        }
        let acks: Vec<Time> = wan.packets[1]
            .iter()
            .filter(|r| r.kind == PacketKind::Ack)
            .filter_map(|r| r.sent.map(|s| epoch() + s))
            .collect();
        // Some left within the step their segment came through in.
        let step = |d: Duration| d.as_nanos() / 1_000_000;
        let within = wan.packets[1]
            .iter()
            .filter(|r| r.kind == PacketKind::Ack && r.sent.is_some_and(|s| step(s) == step(r.at)))
            .count();
        assert!(within > 0 && acks.len() > 20, "{} pure ACKs", acks.len());
        for (now, got) in reads {
            let want = acks.iter().filter(|&&s| s < now).count() as u64;
            assert_eq!(got, want, "pure ACKs out by {now:?}");
        }
    }

    #[test]
    fn a_queued_sack_or_resend_counts_once_it_has_left_and_never_after_a_crash() {
        // `b`'s 10 Mbit/s uplink holds 100 ms of frames: a SACK alone
        // handed to it behind them counts at an edge once it has left, and
        // one still queued when the host loses power never does; so does a
        // resend, which counts from its own departure.
        let mut sim = build();
        echo(&mut sim, "a");
        echo(&mut sim, "b");
        let sites = [
            ("a", Site::new(City::Frankfurt, Access::Fiber)),
            (
                "b",
                Site::new(City::Frankfurt, Access::Fiber).with_up(1_250_000),
            ),
        ];
        let mut wan = Scheduler::new(&sim, &spec(&sites), QUIET, 1, None).unwrap();
        let ms = Duration::from_millis;
        let ns = Duration::from_nanos(1);
        let got = |wan: &Scheduler, at| {
            let c = wan.up_counters(1, at);
            (
                c[up_ix("up_sack_packets")],
                c[up_ix("up_ack_packets")],
                c[up_ix("retransmits")],
            )
        };
        let sack = |wan: &mut Scheduler, at| {
            wan.net.occupy_up(at, 1, 125_000, 84);
            let d = wan.net.occupy_up(at, 1, 80, 1);
            wan.sack_sent(1, wan.net.up_charge(1, 80, 1), at, d);
            d
        };
        let t0 = sim_time(StdDuration::from_secs(1));
        let d = sack(&mut wan, t0);
        assert!(d >= t0 + ms(100), "{d:?}");
        assert_eq!(got(&wan, t0 + ms(50)), (0, 0, 0));
        assert_eq!(got(&wan, d), (0, 0, 0));
        assert_eq!(got(&wan, d + ns), (1, 0, 0));
        // A resend handed over at `d` that leaves 300 ms later.
        wan.retransmitted(1, d, d + ms(300));
        assert_eq!(got(&wan, d + ms(300)), (1, 0, 0));
        assert_eq!(got(&wan, d + ms(300) + ns), (1, 0, 1));
        let t1 = d + ms(400);
        let d1 = sack(&mut wan, t1);
        wan.retransmitted(1, t1, t1 + ms(300));
        wan.crashed(1, t1 + ms(50));
        assert!(d1 > t1 + ms(50), "{d1:?}");
        assert_eq!(got(&wan, Time::MAX), (1, 0, 1));
    }

    #[test]
    fn a_retransmit_counts_once_its_resend_has_left() {
        // Over 5 % site loss and a queue that never fills, a segment the
        // path lost counts as a retransmit once its resend has left, so read
        // after each step's walk no more have counted than the trace's
        // resends handed over before then; the stats count each loss as its
        // segment went, so at least as many by the end.
        let mut counted = 0;
        for seed in 1..=2 {
            let (mut sim, mut wan, _, _) = upload(289_600, (0, (30, 30)), 0.05, None, seed);
            let rtx = up_ix("retransmits");
            let (mut resends, mut seen) = (0, 0);
            loop {
                wan.walk(&sim).unwrap();
                let now = sim_time(sim.elapsed());
                let trace = wan.tcp_trace.as_deref().unwrap_or_default();
                for e in trace.get(seen..).unwrap_or_default() {
                    resends += u64::from(e.from == 0 && e.what == "rtx");
                }
                seen = trace.len();
                let got = wan.up_counters(0, now)[rtx];
                assert!(
                    got <= resends,
                    "seed {seed}: {got} counted, {resends} resent by {now:?}"
                );
                if sim.step().unwrap() {
                    break;
                }
            }
            let end = wan.up_counters(0, Time::MAX)[rtx];
            assert!(end <= wan.net.stats(0).retransmits, "seed {seed}");
            counted += end;
        }
        assert!(counted > 0, "no resend of a path's loss");
    }

    /// Bytes `a`'s competing uploads had sent `s` seconds in.
    fn competing_sent(a: &WanHostReport, s: i64) -> u64 {
        a.competing
            .iter()
            .rev()
            .find(|&&(t, _)| t <= Duration::from_secs(s))
            .map_or(0, |&(_, b)| b)
    }

    #[test]
    fn a_competing_upload_shares_the_uplink() {
        // Alone, the upload fills the uplink, through a queue it keeps from
        // emptying.
        let (_, a) = beside_an_upload(0, (0, 30), 0.0);
        let alone = competing_sent(&a, 3) - competing_sent(&a, 1);
        assert!(alone >= 2_250_000, "{alone} bytes in 2 s");
        let busy = a
            .up_queue
            .iter()
            .filter(|&&(t, w)| t > Duration::from_secs(1) && w > Duration::from_millis(5));
        assert!(busy.count() > 15, "{:?}", a.up_queue);
        // 1.45 MB take about 1.2 s alone and longer beside it, which sends
        // less while they share. A connection that joins a full queue loses
        // its first window and takes some seconds to its share; the bench's
        // join-compete cases check the share against the testbed.
        let (quiet, _) = beside_an_upload(1_448_000, (30, 30), 0.0);
        let (shared, a) = beside_an_upload(1_448_000, (0, 30), 0.0);
        assert!((1150..1500).contains(&quiet), "{quiet} ms");
        assert!(shared > quiet * 3 / 2, "{shared} ms against {quiet} ms");
        let secs = (shared / 1000).max(1);
        let during = competing_sent(&a, 1 + secs) - competing_sent(&a, 1);
        let full = alone / 2 * secs.unsigned_abs();
        assert!(
            during < full * 19 / 20,
            "{during} bytes against {full} alone"
        );
        // Without windows nothing bounds the upload's queue.
        let mut s = spec(&[("a", Site::new(City::Frankfurt, Access::Fiber))]);
        s.hosts[0].competing = vec![Competing {
            start: StdDuration::ZERO,
            stop: StdDuration::from_secs(1),
            rtt: StdDuration::from_millis(30),
        }];
        assert!(Scheduler::new(&build(), &s, QUIET, 1, None).is_err());
    }

    #[test]
    fn an_uplinks_drops_are_the_frames_its_full_queue_lost() {
        // A connection's writes beside a competing upload: both lose frames
        // to the full uplink queue, segments of several frames among them.
        // The uplink's drops (the tables' `drops` column) are the frames
        // lost, as the testbed's tbf counts them, not the segments that
        // lost them.
        let (_, a, _, up) = beside_an_upload_traced(1_448_000, (0, 30), 0.0, 1);
        let at = |name: &str| UP_COUNTERS.iter().position(|&c| c == name).unwrap();
        let frames = a.queue_drop_packets + a.competing_drop_packets;
        let segments = a.queue_drops + a.competing_drops;
        assert_eq!(up[at("drops")], a.stats.drops + frames);
        assert!(
            a.queue_drops > 0 && frames > segments,
            "{frames} frames in {segments} segments, {} of them the connection's",
            a.queue_drops
        );
    }

    #[test]
    fn a_segment_that_loses_a_later_packet_on_its_path_resends_the_rest_alone() {
        // Over 5 % site loss and a queue that never fills, a segment whose
        // path loses a packet past its first goes again from that packet,
        // as Linux resends what a SACK leaves out; one whose first it
        // loses goes again whole.
        let mut rest = 0;
        let mut whole = 0;
        for seed in 1..=4 {
            let (_, _, trace, _) = beside_an_upload_traced(289_600, (30, 30), 0.05, seed);
            let link: Vec<_> = trace.iter().filter(|e| e.from == 0 && e.to == 1).collect();
            assert!(link.iter().all(|e| e.what != "drop"), "a queue drop");
            let mut sent = BTreeMap::new();
            for e in &link {
                match e.what {
                    "tx" => {
                        sent.insert(e.seq, e.len);
                    }
                    "rtx" if e.len < sent[&e.seq] => rest += 1,
                    "rtx" => whole += 1,
                    _ => {}
                }
            }
        }
        assert!(
            rest > 0 && whole > 0,
            "{rest} segments resent from a later packet, {whole} whole"
        );
    }

    #[test]
    fn a_competing_segment_that_loses_a_later_frame_on_its_path_resends_it_alone() {
        // The same for an upload's two-frame segments: one whose second
        // frame the path loses loses that frame alone.
        let mut rest = 0;
        let mut whole = 0;
        for seed in 1..=4 {
            let (_, _, trace, _) = beside_an_upload_traced(0, (0, 10), 0.05, seed);
            let up: Vec<_> = trace.iter().filter(|e| e.to == 2).collect();
            assert!(up.iter().all(|e| e.what != "drop"), "a queue drop");
            let mut sent = BTreeMap::new();
            for e in &up {
                match e.what {
                    "tx" | "rtx" => {
                        sent.insert(e.seq, e.len);
                    }
                    "lost" => match sent.get(&e.seq) {
                        Some(&len) if len <= e.len => whole += 1,
                        _ => rest += 1,
                    },
                    _ => {}
                }
            }
        }
        assert!(
            rest > 0 && whole > 0,
            "{rest} frames lost after a segment's first, {whole} segments whole"
        );
    }

    #[test]
    fn a_competing_uploads_cookie_is_cubics_first_round_trip() {
        // The data connection's handshake gives the round trip estimate its
        // first sample and the cookie's acknowledgement CUBIC's least round
        // trip (`delay_min`, its state begun after the handshake,
        // `tcp_init_transfer`): on an idle uplink both are the path's 30 ms
        // when the upload's first data goes, three trips after its start.
        let mut sim = build();
        let sites = [
            (
                "a",
                Site::new(City::Frankfurt, Access::Fiber).with_up(1_250_000),
            ),
            ("b", Site::new(City::Frankfurt, Access::Fiber)),
        ];
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        sim.host("a", || async { Ok(()) });
        sim.client("b", async {
            tokio::time::sleep(StdDuration::from_secs(2)).await;
            Ok(())
        });
        let mut spec = spec(&sites);
        spec.hosts[0].competing = vec![Competing {
            start: StdDuration::from_secs(1),
            stop: StdDuration::from_secs(2),
            rtt: StdDuration::from_millis(30),
        }];
        let mut wan = Scheduler::new(&sim, &spec, QUIET, 1, Some(buffers)).unwrap();
        let mut seen = None;
        loop {
            wan.walk(&sim).unwrap();
            if let Some(c) = wan.cross.first()
                && seen.is_none()
                && !c.data_at.is_nil()
            {
                seen = Some((c.flow.srtt, c.flow.min_rtt, c.flow.delay_min, c.data_at));
            }
            if sim.step().unwrap() {
                break;
            }
        }
        let ms = Duration::from_millis;
        let (srtt, min_rtt, delay_min, data_at) = seen.unwrap();
        assert_eq!((srtt, min_rtt, delay_min), (ms(30), ms(30), ms(30)));
        assert_eq!(data_at, sim_time(StdDuration::from_millis(1090)));
    }

    #[test]
    fn site_loss_holds_windows_down() {
        // Over a 30 ms round trip, a connection that loses 1 % of its
        // packets keeps a window of a few segments, as Linux's does
        // (hack/testbed/bench/lossy-over.sched): 1.45 MB take about 1.3 s
        // on the bare uplink and over eight draws of the loss about 1.5
        // times that on average, 1.0-2.6 times as the losses fall.
        let (clean, _) = beside_an_upload(1_448_000, (30, 30), 0.0);
        let lossy: Vec<i64> = (1..=8)
            .map(|seed| beside_an_upload_drawn(1_448_000, (30, 30), 0.01, seed).0)
            .collect();
        let mean = lossy.iter().sum::<i64>() / 8;
        assert!(mean * 10 > clean * 14, "{lossy:?} ms against {clean} ms");
        // An upload from the site loses its packets too, and sends about
        // half as much.
        let (_, a) = beside_an_upload(0, (0, 30), 0.0);
        let (_, b) = beside_an_upload(0, (0, 30), 0.01);
        let rate = |r: &WanHostReport| competing_sent(r, 6) - competing_sent(r, 2);
        assert!(
            rate(&b) * 5 < rate(&a) * 3,
            "{} bytes in 4 s against {} without the loss",
            rate(&b),
            rate(&a)
        );
    }

    #[test]
    fn a_competing_uploads_acknowledgements_cross_the_site_loss() {
        // The site loses the upload's acknowledgements as it loses its
        // data, each with the loss's probability; what a lost one
        // acknowledged goes with the next, so its segments aren't sent
        // again.
        let (_, _, trace, _) = beside_an_upload_traced(0, (0, 10), 0.05, 1);
        let upload = |what: &str| {
            trace
                .iter()
                .filter(|e| e.to == 2 && e.what == what)
                .collect::<Vec<_>>()
        };
        let (acked, unheard) = (upload("ack").len(), upload("acklost"));
        let share = unheard.len() as f64 / (acked + unheard.len()) as f64;
        assert!((0.02..0.09).contains(&share), "{share} of {acked} lost");
        let resent: BTreeSet<u64> = upload("rtx").iter().map(|e| e.seq).collect();
        let lost: BTreeSet<u64> = upload("lost").iter().map(|e| e.seq).collect();
        let again: Vec<u64> = unheard
            .iter()
            .map(|e| e.seq)
            .filter(|s| resent.contains(s) && !lost.contains(s))
            .collect();
        assert!(
            again.is_empty(),
            "resent after a lost acknowledgement: {again:?}"
        );
    }

    #[test]
    fn without_send_buffers_writes_never_wait() {
        let (wrote, got, peak) = buffered_writes(None, |_| {});
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
    fn a_full_queue_drops_datagrams() {
        let mut sim = build();
        let sites = [
            ("a", Site::new(City::Sydney, Access::Fiber).with_loss(0.0)),
            (
                "b",
                Site::new(City::Sydney, Access::Fiber)
                    .with_up(125_000)
                    .with_loss(0.0),
            ),
        ];
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
        // 600-byte datagrams every 2 ms, two and a half times what b's
        // 1 Mbit/s uplink carries, into its 100 ms queue.
        sim.client("b", async {
            let s = UdpSocket::bind("0.0.0.0:9001").await?;
            for _ in 0..1000 {
                s.send_to(&[23u8; 600], ("a", 9000)).await?;
                tokio::time::sleep(StdDuration::from_millis(2)).await;
            }
            tokio::time::sleep(StdDuration::from_millis(500)).await;
            Ok(())
        });
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        let spec = spec(&sites);
        let mut wan = Scheduler::new(&sim, &spec, QUIET, 1, Some(buffers)).unwrap();
        drive(&mut sim, &mut wan);
        // The uplink carries its share over the 2 s and the queue it holds
        // at the end; the rest is lost at the queue, and forgotten.
        let got = got.get();
        let (wire, _) = udp_wire(600);
        let share = 125_000 * 21 / 10 / u32::try_from(wire).unwrap();
        assert!(
            got.abs_diff(share) < share / 10,
            "{got} of 1000, {share} expected"
        );
        let report = wan.report(&spec);
        assert_eq!(u64::from(1000 - got), report.hosts[1].queue_drops);
        assert_eq!(wan.net.stats(1).drops, 0);
        assert_eq!(wan.in_flight(), 0, "lost datagrams stay tracked");
    }

    /// A scheduler of hosts `a` and `b` with send buffers and the window
    /// model, `a`'s packets logged, and `n` channels of `p` from `a` to `b`.
    fn channels_of(p: SctpProfile, n: u16) -> (Sim<'static>, Scheduler, Vec<ConnKey>) {
        channels_on(p, n, Site::new(City::Frankfurt, Access::Fiber))
    }

    /// [`channels_of`], `a` at `site`.
    fn channels_on(p: SctpProfile, n: u16, site: Site) -> (Sim<'static>, Scheduler, Vec<ConnKey>) {
        channels_between(p, n, site, Site::new(City::Frankfurt, Access::Fiber))
    }

    /// [`channels_of`], `a` at `at_a` and `b` at `at_b`, both hosts'
    /// packets logged.
    fn channels_between(
        p: SctpProfile,
        n: u16,
        at_a: Site,
        at_b: Site,
    ) -> (Sim<'static>, Scheduler, Vec<ConnKey>) {
        let mut sim = build();
        echo(&mut sim, "a");
        echo(&mut sim, "b");
        let sites = [("a", at_a), ("b", at_b)];
        let mut s = spec(&sites);
        s.packet_log = BTreeSet::from([0, 1]);
        let buffers = SendBuffers::new(SendBuffer {
            uplink_queue: StdDuration::from_millis(100),
            window: true,
        });
        let mut wan = Scheduler::new(&sim, &s, QUIET, 1, Some(buffers)).unwrap();
        let ip = |h: HostIx| *wan.ip_host.iter().find(|&(_, &x)| x == h).unwrap().0;
        let conns: Vec<ConnKey> = (0..n)
            .map(|i| {
                (
                    SocketAddr::new(ip(0), 1000 + i),
                    SocketAddr::new(ip(1), 2000),
                )
            })
            .collect();
        for &c in &conns {
            wan.open_channel(c, p);
        }
        (sim, wan, conns)
    }

    /// Writes `n` messages of `chunks` 1000-byte chunks on channel `c` at
    /// `at`, a chunk a packet, numbering their records from `id` on.
    fn write_chunks(wan: &mut Scheduler, id: &mut u64, at: Time, c: ConnKey, n: u64, chunks: u64) {
        write_sized(wan, id, at, c, n, chunks, 1000);
    }

    /// [`write_chunks`] of `len`-byte chunks.
    fn write_sized(
        wan: &mut Scheduler,
        id: &mut u64,
        at: Time,
        c: ConnKey,
        n: u64,
        chunks: u64,
        len: usize,
    ) {
        let p = wan.assocs[&c].profile();
        let (from, to) = (wan.ip_host[&c.0.ip()], wan.ip_host[&c.1.ip()]);
        for k in 0..chunks * n {
            wan.records.insert(
                *id,
                Record {
                    src: c.0,
                    dst: c.1,
                    from,
                    to,
                    what: What::Data { seq: *id, len },
                    wire: p.data_wire(len),
                    packets: 1,
                    nominal: false,
                    state: State::InFlight,
                    link: 0,
                },
            );
            wan.sctp_write(at, c, *id, k % chunks == chunks - 1);
            *id += 1;
        }
    }

    /// A Chromium host's channels send through its one thread: each send
    /// opportunity once the thread is done with those before it, whatever
    /// its channel, its packets a cost apart and each its cost after the
    /// one before; and its page's answer as the opportunity that left the
    /// channel's queue empty sent its last. A Firefox host's take no
    /// thread: its packets go as their send opportunity comes.
    #[test]
    fn a_browsers_channels_send_through_its_one_thread() {
        for p in [SctpProfile::CHROMIUM, SctpProfile::FIREFOX] {
            let (_sim, mut wan, conns) = channels_of(p, 3);
            // Messages of four chunks.
            let mut id = 0;
            let mut write = |wan: &mut Scheduler, at: Time, c: ConnKey, n: u64| {
                write_chunks(wan, &mut id, at, c, n, 4);
            };
            let t0 = sim_time(StdDuration::from_secs(1));
            let us = |n: i64| Duration::from_micros(n);
            write(&mut wan, t0, conns[0], 2);
            write(&mut wan, t0, conns[1], 2);
            // A third channel's message comes as the thread sends the second's.
            write(&mut wan, t0 + us(30), conns[2], 1);
            // Once the thread is idle, a message goes at once ...
            write(&mut wan, t0 + us(1000), conns[2], 1);
            // ... and one the window holds back in part leaves the channel's
            // queue waiting, unanswered.
            write(&mut wan, t0 + us(2000), conns[0], 1);
            // The walks that reach the thread's hand-overs, before anything
            // reaches `b`.
            wan.run_events(t0 + us(2100));
            let times: Vec<Duration> = wan.packets[0]
                .iter()
                .map(|r| r.at - (t0 - epoch()))
                .collect();
            let links: Vec<u32> = wan.packets[0].iter().map(|r| r.link.unwrap()).collect();
            let answers: Vec<(Duration, ConnKey)> = wan
                .answers
                .iter()
                .map(|(&(t, _), &c)| (t - t0, c))
                .collect();
            let Some(cost) = p.packet_cost else {
                assert_eq!(times.len(), 27, "{p:?}");
                assert!(times[..20].iter().all(|&t| t <= us(30)), "{times:?}");
                assert!(answers.is_empty());
                continue;
            };
            let at = |i: i64| cost * i;
            let mut expected: Vec<Duration> = (1..=20).map(at).collect();
            expected.extend((1..=4).map(|i| us(1000) + at(i)));
            expected.extend((1..=3).map(|i| us(2000) + at(i)));
            assert_eq!(times, expected);
            assert_eq!(
                links,
                [vec![0; 8], vec![1; 8], vec![2; 8], vec![0; 3]].concat()
            );
            // A send opportunity's chunks go as of when the thread took it up.
            let sent = |c: ConnKey, seq: u64| wan.assocs[&c].sent_at(seq).unwrap() - t0;
            assert_eq!(
                (sent(conns[0], 4), sent(conns[1], 8), sent(conns[2], 16)),
                (at(4), at(8), at(16))
            );
            assert_eq!(
                answers,
                [
                    (at(4), conns[0]),
                    (at(8), conns[0]),
                    (at(12), conns[1]),
                    (at(16), conns[1]),
                    (at(20), conns[2]),
                    (us(1000) + at(4), conns[2])
                ]
            );
        }
    }

    /// A browser's SACK held back for its peer's data goes first in the
    /// first packet of its next send opportunity toward that peer, whose
    /// chunks take the room it leaves, so no packet passes the MTU: ahead
    /// of a message of full chunks it goes alone and the burst's four
    /// packets follow it, and a small chunk shares its packet. A send
    /// opportunity whose window takes nothing sends it alone. A run gives
    /// the same packets each time.
    #[test]
    fn a_held_sack_takes_its_room_within_the_mtu() {
        for p in [SctpProfile::CHROMIUM, SctpProfile::FIREFOX] {
            let run = || {
                let (_sim, mut wan, conns) = channels_of(p, 1);
                let (ab, ba) = (conns[0], (conns[0].1, conns[0].0));
                wan.open_channel(ba, p);
                // Each direction's chunks numbered on from a base of its own.
                let (mut a_id, mut b_id) = (0, 1 << 20);
                let t0 = sim_time(StdDuration::from_secs(1));
                let ms = |n: i64| t0 + Duration::from_millis(n);
                let mut a_writes = |wan: &mut Scheduler, at: i64| {
                    wan.run_events(ms(at));
                    write_sized(wan, &mut a_id, ms(at), ab, 1, 1, 1000);
                    wan.run_events(ms(at + 20));
                };
                let mut b_writes = |wan: &mut Scheduler, at: i64, chunks: u64, len: usize| {
                    write_sized(wan, &mut b_id, ms(at), ba, 1, chunks, len);
                };
                // b acknowledges a's first DATA at once and holds back the
                // next packet's SACK, which goes ahead of a message of full
                // chunks, ...
                a_writes(&mut wan, 0);
                a_writes(&mut wan, 50);
                b_writes(&mut wan, 70, 4, p.max_payload);
                // ... with a small chunk, ...
                a_writes(&mut wan, 500);
                b_writes(&mut wan, 520, 1, 500);
                // ... and alone, the window full; the chunk goes with the
                // next message.
                a_writes(&mut wan, 900);
                let cwnd = wan.assocs[&ba].cwnd();
                wan.assocs.get_mut(&ba).unwrap().set_cwnd(0);
                b_writes(&mut wan, 920, 1, 500);
                wan.assocs.get_mut(&ba).unwrap().set_cwnd(cwnd);
                wan.run_events(ms(940));
                b_writes(&mut wan, 940, 1, 500);
                wan.run_events(ms(1500));
                wan.packets.clone()
            };
            let packets = run();
            assert_eq!(packets, run(), "{p:?}");
            // A packet's bytes in the log: its SCTP bytes and the overhead
            // past them, framing and all, which this site's link charges
            // as it is. The most SCTP bytes are dcSCTP's MTU rounded down
            // to 4.
            let ip = |sctp: usize| (sctp + p.packet_overhead) as u32;
            let most = ip(p.mtu / 4 * 4);
            for (h, rows) in packets.iter().enumerate() {
                for r in rows {
                    assert!(r.ip_bytes <= most, "{p:?} host {h}: {r:?}");
                }
            }
            let b: Vec<(PacketKind, u32)> = packets[1]
                .iter()
                .filter(|r| r.proto == PacketProto::Udp)
                .map(|r| (r.kind, r.ip_bytes))
                .collect();
            let (sack, full) = ((PacketKind::Sack, ip(12 + 16)), (PacketKind::Data, most));
            assert_eq!(
                b,
                [
                    sack,
                    sack,
                    full,
                    full,
                    full,
                    full,
                    (PacketKind::DataSack, ip(12 + 16 + 16 + 500)),
                    sack,
                    (PacketKind::Data, ip(12 + 2 * (16 + 500))),
                ],
                "{p:?}"
            );
        }
    }

    /// What a Chromium's network thread hands its uplink past the walk that
    /// sent it counts from when it left or was lost: three channels'
    /// messages written at the walk's time go a packet every 12 µs from
    /// 12 µs past it, each handed over in the walk that reaches it; the
    /// 1 Mbit/s uplink's bucket of two frames sends the first two at their
    /// hand-over, before the third's, and its 100 ms queue drops the
    /// burst's last packets. Read at the walk's time, nothing had left or
    /// been lost, and at each packet's hand-over or departure, once a walk
    /// reached it, what had before it. The sender's SCTP log has each
    /// packet go out once it is handed over. A crash at the walk's time
    /// forgets what the thread had yet to hand over: none of it leaves, is
    /// lost or is in the log.
    #[test]
    fn what_a_chromiums_thread_hands_over_past_the_walk_counts_from_when_it_left_or_was_lost() {
        let site = Site::new(City::Frankfurt, Access::Fiber)
            .with_up(125_000)
            .with_bucket(Some(strims_wan::Bucket::of(3028)));
        let written = || {
            let (sim, mut wan, conns) = channels_on(SctpProfile::CHROMIUM, 3, site);
            wan.sctp_log[0] = true;
            wan.walk(&sim).unwrap();
            let now = sim_time(sim.elapsed());
            let mut id = 0;
            for &c in &conns {
                write_chunks(&mut wan, &mut id, now, c, 2, 4);
            }
            (wan, now)
        };
        // A walk at `t`, of the events alone: the thread's hand-overs end
        // within a millisecond, long before anything reaches `b`.
        let reach = |wan: &mut Scheduler, t: Time| {
            wan.walked = t;
            wan.net.settle(t);
            wan.run_events(t);
        };
        let us = Duration::from_micros;
        // The packets the sender's SCTP log has gone out.
        let outs = |wan: &Scheduler| {
            wan.sctp_events[0]
                .values()
                .flatten()
                .filter(|e| e.dir == Some(strims_testbed::sctp::Dir::Out))
                .count()
        };
        let (mut wan, now) = written();
        assert!(wan.packets[0].is_empty());
        assert_eq!(outs(&wan), 0);
        reach(&mut wan, now + us(1000));
        let log = wan.packets[0].clone();
        assert_eq!(outs(&wan), log.len());
        let at = |d: Duration| epoch() + d;
        let last = log.iter().map(|r| at(r.at)).max().unwrap();
        let first_left = log.iter().filter_map(|r| r.sent.map(at)).min().unwrap();
        let lost = |r: &PacketRecord| r.lost == Some(PacketLoss::Queue);
        assert!(log.iter().all(|r| at(r.at) > now), "{log:?}");
        assert!(last < now + us(1000), "{log:?}");
        assert!(first_left < last && log.iter().any(lost), "{log:?}");
        let (left, drops) = (up_ix("up_packets"), up_ix("drops"));
        let mut reads: Vec<Time> = log
            .iter()
            .flat_map(|r| [Some(at(r.at)), r.sent.map(at)])
            .flatten()
            .chain([now, Time::MAX])
            .collect();
        reads.sort_unstable();
        let (mut wan, _) = written();
        for t in reads {
            reach(&mut wan, t.min(now + us(1000)));
            let c = wan.up_counters(0, t);
            let want = (
                log.iter()
                    .filter(|r| r.sent.is_some_and(|s| at(s) < t))
                    .count(),
                log.iter().filter(|r| lost(r) && at(r.at) < t).count(),
            );
            assert_eq!((c[left] as usize, c[drops] as usize), want, "at {t:?}");
        }
        let (mut wan, _) = written();
        wan.crashed(0, now);
        reach(&mut wan, now + us(1000));
        let c = wan.up_counters(0, Time::MAX);
        assert_eq!((c[left], c[drops]), (0, 0));
        assert!(wan.packets[0].is_empty());
        assert_eq!(outs(&wan), 0);
    }

    /// A Chromium stack whose channel's queue never empties answers its
    /// page once it has sent more than 100 KiB of the channel since its
    /// last answer, as that send opportunity's last packet goes.
    #[test]
    fn a_chromium_stack_answers_its_page_past_100_kib_its_queue_waiting() {
        let p = SctpProfile::CHROMIUM;
        let (_sim, mut wan, conns) = channels_of(p, 1);
        if let Some(a) = wan.assocs.get_mut(&conns[0]) {
            a.set_cwnd(1 << 20);
        }
        let t0 = sim_time(StdDuration::from_secs(1));
        let mut id = 0;
        // A message of eight chunks leaves four waiting: its send
        // opportunity sends four packets. The 26th's take the stack's sends
        // to 104 000 bytes since the start.
        for i in 0..30 {
            let at = t0 + Duration::from_millis(i);
            write_chunks(&mut wan, &mut id, at, conns[0], 1, 8);
        }
        let answers: Vec<Duration> = wan.answers.keys().map(|&(t, _)| t - t0).collect();
        let cost = p.packet_cost.unwrap();
        assert_eq!(answers, [Duration::from_millis(25) + cost * 4]);
    }

    /// A receiver's stack takes a packet as its downlink delivers it, and
    /// its SACK goes to its uplink no sooner, behind what its host handed
    /// the uplink before: four packets reach `b`'s 1 Mbit/s downlink
    /// together and come through it 8.6 ms apart, and a message `b` writes
    /// back on their association as they reach it, before the first is
    /// through, leaves at once and carries no SACK. Every packet `b`'s
    /// uplink was handed came no sooner than the one before, a Chromium's
    /// through its network thread, a Firefox's at once.
    #[test]
    fn a_receivers_sacks_go_to_its_uplink_as_its_downlink_delivers() {
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        for p in [SctpProfile::FIREFOX, SctpProfile::CHROMIUM] {
            let (_sim, mut wan, conns) = channels_between(p, 1, fiber, fiber.with_down(125_000));
            let (ab, ba) = (conns[0], (conns[0].1, conns[0].0));
            wan.open_channel(ba, p);
            // Records numbered apart from the scheduler's own keys.
            let mut id = 1 << 32;
            let t0 = sim_time(StdDuration::from_secs(1));
            write_chunks(&mut wan, &mut id, t0, ab, 1, 4);
            let t1 = t0 + wan.net.one_way(0, 1) + Duration::from_millis(1);
            wan.run_events(t1);
            write_chunks(&mut wan, &mut id, t1, ba, 1, 1);
            wan.run_events(t1 + Duration::from_millis(200));
            let log = &wan.packets[1];
            let sacks: Vec<&PacketRecord> =
                log.iter().filter(|r| r.kind == PacketKind::Sack).collect();
            let data: Vec<&PacketRecord> =
                log.iter().filter(|r| r.kind != PacketKind::Sack).collect();
            assert!(!sacks.is_empty(), "{p:?}: {log:?}");
            assert_eq!(data.len(), 1, "{p:?}: {log:?}");
            assert!(
                log.iter().map(|r| r.at).is_sorted(),
                "{p:?}: handed over out of time order: {log:?}"
            );
            assert!(
                sacks.iter().all(|s| s.sent > data[0].sent),
                "{p:?}: a SACK left before the message: {log:?}"
            );
            assert_eq!(data[0].kind, PacketKind::Data, "{p:?}: {log:?}");
            let first = t1 - epoch() + Duration::from_millis(7);
            assert!(sacks.iter().all(|s| s.at > first), "{p:?}: {log:?}");
        }
    }

    /// A Chromium's network thread hands its uplink what it sent before a
    /// crash that comes before the walk reaching it, and the crash finds it
    /// there: three channels' messages written at a walk's time go a packet
    /// every 12 µs, and a crash a millisecond on finds the 1 Mbit/s
    /// uplink's bucket of two frames sent and its 100 ms queue full, the
    /// burst's last packets lost to it. The crash forgets what was queued,
    /// which never leaves: the packet log keeps its offers, the crash their
    /// fate. Read up to 2 s on (past the chunks' retransmission timeouts and
    /// the SACKs of those that went), the uplink carried what left before
    /// the crash, lost what it lost before it, and was handed nothing after.
    #[test]
    fn a_crash_finds_what_a_chromiums_thread_handed_over_before_it() {
        let site = Site::new(City::Frankfurt, Access::Fiber)
            .with_up(125_000)
            .with_bucket(Some(strims_wan::Bucket::of(3028)));
        let (sim, mut wan, conns) = channels_on(SctpProfile::CHROMIUM, 3, site);
        wan.walk(&sim).unwrap();
        let now = sim_time(sim.elapsed());
        let mut id = 0;
        for &c in &conns {
            write_chunks(&mut wan, &mut id, now, c, 2, 4);
        }
        let crash = now + Duration::from_millis(1);
        wan.crashed(0, crash);
        let end = crash + Duration::from_secs(2);
        wan.walked = end;
        wan.net.settle(end);
        wan.run_events(end);
        let log = &wan.packets[0];
        let at = |d: Duration| epoch() + d;
        let lost = |r: &PacketRecord| r.lost == Some(PacketLoss::Queue);
        let forgot = |r: &PacketRecord| r.lost == Some(PacketLoss::Crash);
        assert_eq!(log.len(), 24, "{log:?}");
        assert!(log.iter().all(|r| at(r.at) < crash), "{log:?}");
        assert!(log.iter().any(lost), "{log:?}");
        assert!(
            log.iter().any(forgot),
            "nothing queued at the crash: {log:?}"
        );
        assert!(
            log.iter()
                .filter(|r| forgot(r))
                .all(|r| r.sent.is_none() && r.arrived.is_none()),
            "{log:?}"
        );
        assert!(
            log.iter().all(|r| r.sent.is_none_or(|s| at(s) <= crash)),
            "left after the crash: {log:?}"
        );
        let c = wan.up_counters(0, Time::MAX);
        let want = (
            log.iter()
                .filter(|r| r.sent.is_some_and(|s| at(s) < crash))
                .count(),
            log.iter().filter(|r| lost(r)).count(),
        );
        assert!(want.0 > 0, "{log:?}");
        assert_eq!(
            (c[up_ix("up_packets")] as usize, c[up_ix("drops")] as usize),
            want
        );
    }

    /// A receiver that crashes with chunks in its downlink's queue takes
    /// none of them and SACKs nothing, nor the copies its sender's
    /// retransmission timeouts send it, as a Firefox and as a Chromium:
    /// four packets reach `b`'s 1 Mbit/s downlink together, `b` crashes a
    /// millisecond on, before the first is through, and up to 2 s on its
    /// uplink is handed nothing.
    #[test]
    fn a_receiver_that_crashes_takes_and_sacks_nothing_after() {
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        for p in [SctpProfile::FIREFOX, SctpProfile::CHROMIUM] {
            let (_sim, mut wan, conns) = channels_between(p, 1, fiber, fiber.with_down(125_000));
            let ab = conns[0];
            let mut id = 1 << 32;
            let t0 = sim_time(StdDuration::from_secs(1));
            write_chunks(&mut wan, &mut id, t0, ab, 1, 4);
            let t1 = t0 + wan.net.one_way(0, 1) + Duration::from_millis(1);
            wan.walked = t1;
            wan.net.settle(t1);
            wan.run_events(t1);
            assert_eq!(wan.packets[0].len(), 4, "{p:?}");
            wan.crashed(1, t1);
            let end = t1 + Duration::from_secs(2);
            wan.walked = end;
            wan.net.settle(end);
            wan.run_events(end);
            assert!(
                wan.packets[0].len() > 4,
                "{p:?}: `a` resent nothing: {:?}",
                wan.packets[0]
            );
            assert!(wan.packets[1].is_empty(), "{p:?}: {:?}", wan.packets[1]);
        }
    }

    /// A walk at `t` of the events alone, as a step's walk runs them.
    fn reach(wan: &mut Scheduler, t: Time) {
        wan.walked = t;
        wan.net.settle(t);
        wan.run_events(t);
    }

    /// A crash that comes after a chunk is through its host's downlink and
    /// before the walk that reaches it finds the chunk taken and its SACK
    /// handed over, as a Firefox and as a Chromium: four packets reach
    /// `b`'s 1 Mbit/s downlink together and come through it 8.7 ms apart; a
    /// walk a millisecond after they reach it leaves the first still in the
    /// downlink, and `b` crashes 10 ms on, after the first is through.
    #[test]
    fn a_crash_finds_the_sack_of_what_its_downlink_delivered_before_it() {
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        for p in [SctpProfile::FIREFOX, SctpProfile::CHROMIUM] {
            let (_sim, mut wan, conns) = channels_between(p, 1, fiber, fiber.with_down(125_000));
            let mut id = 1 << 32;
            let t0 = sim_time(StdDuration::from_secs(1));
            write_chunks(&mut wan, &mut id, t0, conns[0], 1, 4);
            let t1 = t0 + wan.net.one_way(0, 1) + Duration::from_millis(1);
            reach(&mut wan, t1);
            assert!(wan.packets[1].is_empty(), "{p:?}: {:?}", wan.packets[1]);
            let crash = t1 + Duration::from_millis(10);
            wan.crashed(1, crash);
            let log = &wan.packets[1];
            let at = |d: Duration| epoch() + d;
            assert!(
                log.iter()
                    .any(|r| r.kind == PacketKind::Sack && at(r.at) > t1 && at(r.at) <= crash),
                "{p:?}: no SACK handed over between the walk and the crash: {log:?}"
            );
            let handed = log.len();
            reach(&mut wan, crash + Duration::from_secs(2));
            assert_eq!(wan.packets[1].len(), handed, "{p:?}: {:?}", wan.packets[1]);
        }
    }

    /// A receiver that crashed takes nothing of what its peer's association
    /// with its stack from before the crash sends it once it is up again, as
    /// a Firefox and as a Chromium: four packets reach `b`'s 1 Mbit/s
    /// downlink together, `b` crashes a millisecond on, before the first is
    /// through, and restarts 5 s later. Up to two minutes on, `a`'s
    /// association, which never heard of the crash, sends copies on its
    /// retransmission timer, which come through `b`'s downlink, and `b`'s
    /// uplink is handed nothing.
    #[test]
    fn a_restarted_receiver_takes_nothing_its_crashed_stack_was_sent() {
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        for p in [SctpProfile::FIREFOX, SctpProfile::CHROMIUM] {
            let (_sim, mut wan, conns) = channels_between(p, 1, fiber, fiber.with_down(125_000));
            let mut id = 1 << 32;
            let t0 = sim_time(StdDuration::from_secs(1));
            write_chunks(&mut wan, &mut id, t0, conns[0], 1, 4);
            let t1 = t0 + wan.net.one_way(0, 1) + Duration::from_millis(1);
            reach(&mut wan, t1);
            wan.crashed(1, t1);
            let t2 = t1 + Duration::from_secs(5);
            reach(&mut wan, t2);
            assert!(
                wan.packets[1].is_empty(),
                "{p:?}: down: {:?}",
                wan.packets[1]
            );
            wan.restarted(1);
            let (sent, through) = (wan.packets[0].len(), wan.net.stats(1).down_bytes);
            reach(&mut wan, t2 + Duration::from_secs(120));
            assert!(wan.packets[0].len() > sent, "{p:?}: `a` resent nothing");
            assert!(
                wan.net.stats(1).down_bytes > through,
                "{p:?}: no copy reached `b`"
            );
            assert!(
                wan.packets[1].is_empty(),
                "{p:?}: restarted `b` handed over: {:?}",
                wan.packets[1]
            );
        }
    }

    /// A SACK its receiver's crash forgot reaches no sender: a Firefox `b`
    /// on a 2 kB/s uplink with no bucket SACKs `a`'s chunk as it comes
    /// through, and its uplink takes 46 ms to send it; `b` crashes before
    /// it has left. The packet log keeps the SACK's offer, the crash its
    /// fate, nothing left `b`, and `a`, which took no SACK, resends.
    #[test]
    fn a_sack_its_receivers_crash_forgot_reaches_no_sender() {
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        let slow = fiber.with_up(2_000).with_bucket(None);
        let (_sim, mut wan, conns) = channels_between(SctpProfile::FIREFOX, 1, fiber, slow);
        let mut id = 1 << 32;
        let t0 = sim_time(StdDuration::from_secs(1));
        write_chunks(&mut wan, &mut id, t0, conns[0], 1, 1);
        let t1 = t0 + wan.net.one_way(0, 1) + Duration::from_millis(1);
        reach(&mut wan, t1);
        let handed = wan.packets[1].clone();
        let at = |d: Duration| epoch() + d;
        assert!(!handed.is_empty(), "no SACK handed over");
        assert!(
            handed
                .iter()
                .all(|r| r.lost.is_none() && r.sent.is_some_and(|s| at(s) > t1)),
            "{handed:?}"
        );
        wan.crashed(1, t1);
        let log = &wan.packets[1];
        assert_eq!(log.len(), handed.len(), "{log:?}");
        for (r, h) in log.iter().zip(&handed) {
            assert_eq!((r.at, r.kind, r.ip_bytes), (h.at, h.kind, h.ip_bytes));
            assert_eq!(
                (r.sent, r.arrived, r.lost),
                (None, None, Some(PacketLoss::Crash))
            );
        }
        assert_eq!(wan.up_counters(1, Time::MAX)[up_ix("up_packets")], 0);
        reach(&mut wan, t1 + Duration::from_secs(2));
        assert!(
            wan.packets[0].len() > 1,
            "`a` took a SACK that never left `b` and resent nothing: {:?}",
            wan.packets[0]
        );
    }

    /// A copy its sender's crash forgot reaches no receiver, as a Firefox
    /// and as a Chromium: `a`'s 25 kB/s uplink with no bucket sends its
    /// chunk in 44 ms, `b`'s SACK of it takes 0.92 s through its 100 B/s
    /// uplink, and `a`'s retransmission timer resends the chunk at 0.5 s, a
    /// copy, which takes 44 ms to leave; `a` crashes 20 ms on, before it
    /// has. `b` takes the chunk and SACKs it, and nothing more: the copy
    /// never comes.
    #[test]
    fn a_copy_its_senders_crash_forgot_reaches_no_receiver() {
        let fiber = Site::new(City::Frankfurt, Access::Fiber);
        let (at_a, at_b) = (
            fiber.with_up(25_000).with_bucket(None),
            fiber.with_up(100).with_bucket(None),
        );
        for p in [SctpProfile::FIREFOX, SctpProfile::CHROMIUM] {
            let (_sim, mut wan, conns) = channels_between(p, 1, at_a, at_b);
            let ab = conns[0];
            wan.open_channel((ab.1, ab.0), p);
            let mut id = 1 << 32;
            let t0 = sim_time(StdDuration::from_secs(1));
            write_chunks(&mut wan, &mut id, t0, ab, 1, 1);
            let crash = t0 + Duration::from_millis(520);
            reach(&mut wan, crash);
            let at = |d: Duration| epoch() + d;
            let log = wan.packets[0].clone();
            assert_eq!(log.len(), 2, "{p:?}: {log:?}");
            assert!(log[0].sent.is_some_and(|s| at(s) < crash), "{p:?}: {log:?}");
            assert!(log[1].sent.is_some_and(|s| at(s) > crash), "{p:?}: {log:?}");
            let sacks = wan.packets[1].len();
            assert!(sacks > 0, "{p:?}: `b` SACKed nothing");
            wan.crashed(0, crash);
            assert_eq!(wan.packets[0][1].lost, Some(PacketLoss::Crash), "{p:?}");
            reach(&mut wan, crash + Duration::from_secs(2));
            assert_eq!(
                wan.packets[1].len(),
                sacks,
                "{p:?}: `b` took a copy that never left `a`: {:?}",
                wan.packets[1]
            );
        }
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
