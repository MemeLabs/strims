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
//!   A segment waits in its connection's queue while the bytes in flight
//!   (sent, neither acknowledged nor known lost) and it would pass the
//!   window; a connection with nothing in flight sends one anyway. A
//!   connection writing sends what its window allows at once.
//! - The access queues hold `uplink_queue` of transmission time, up and
//!   down. A segment that doesn't fit in what one has left is lost there,
//!   as a byte-limited FIFO (Linux's tbf and bfifo) drops it: a small
//!   write fits where a full segment doesn't.
//! - Its sender learns of the loss from a later segment's acknowledgement,
//!   a quarter of the least round trip later (RACK), or failing that from
//!   its tail-loss probe's, two smoothed round trips after its last send
//!   (and 200 ms more with one segment in flight) and a round trip on. A
//!   lost retransmission waits for the retransmission timeout, which backs
//!   off, and restarts the window from one segment. The lost segment goes
//!   to the front of its connection's queue, and the window falls to
//!   [`BETA`] of itself once a recovery: losses of segments sent before
//!   it fell don't cut it again.
//! - A segment through the receiver's downlink is acknowledged once the
//!   acknowledgement crosses back, after the receiver's uplink queue. Pure
//!   acknowledgements take their turn on the receiver's uplink as Linux
//!   sends them: one for every second segment received, or 40 ms after a
//!   lone one, unless the receiver's own data carries them first. The
//!   window grows by the bytes acknowledged in slow start, which HyStart's
//!   delay test ends, and as CUBIC (RFC 8312) past it, only while it
//!   limits the connection and outside a recovery; a connection idle for
//!   longer than its retransmission timeout restarts from a smaller one.
//!
//! The connection's order on the wire is its send order, and a resent
//! segment holds back the rest of its connection at the receiver.
//!
//! A host's competing uploads
//! ([`HostSpec::competing`](crate::spec::HostSpec::competing)) are TCP
//! connections that always have data and reach no host: full segments
//! enter the host's uplink queue within their window, are lost when it is
//! full, and are acknowledged the upload's round trip after they leave the
//! uplink. Their windows follow the connections' CUBIC, HyStart and
//! recoveries too. They need the window model's bounded queues.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
#[allow(clippy::disallowed_types)]
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_wan::network::RTO_MIN;
use strims_wan::wire::{tcp_wire, tx_time, udp_wire};
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
    /// The window model: the sender learns the segment was lost, from
    /// later segments' acknowledgements or a tail-loss probe's.
    Lost,
    /// The window model: a retransmission timeout tells the sender a
    /// retransmitted segment was lost again.
    Timeout,
    /// The window model: `conn`'s tail-loss probe is due.
    Probe { conn: (SocketAddr, SocketAddr) },
    /// The window model: the receiver of `conn`'s data sends the
    /// acknowledgement it delayed, unless data it sent back carried it.
    DelAck { conn: (SocketAddr, SocketAddr) },
    /// The window model: an acknowledgement of the `len` bytes at `seq`
    /// of `conn` reaches the sender.
    Ack {
        conn: (SocketAddr, SocketAddr),
        seq: u64,
        len: usize,
    },
    /// Competing upload `i`'s segment `seq` is acknowledged, a round trip
    /// of `rtt` after it went out.
    CrossAck { i: usize, seq: u64, rtt: Duration },
    /// Competing upload `i` learns its segment `seq` was lost.
    CrossLost { i: usize, seq: u64 },
    /// The receiver of competing upload `i` acknowledges its lone segment
    /// `seq`, unless a second one came first.
    CrossDelAck { i: usize, seq: u64 },
}

/// The window model's segment size, for the window's growth.
pub(crate) const MSS: f64 = 1448.0;
/// A competing upload's segment, its payload and its bytes on the wire:
/// two packets, as the window model cuts its connections'
/// ([`WINDOW_SEGMENT`](crate::spec::WINDOW_SEGMENT)) and as the upload's
/// receiver acknowledges them, so that they meet a full queue at one size.
const CROSS_SEGMENT: usize = crate::spec::WINDOW_SEGMENT;
const CROSS_WIRE: usize = 2 * 1500;
/// CUBIC's scaling constant, in segments per second cubed.
const CUBIC_C: f64 = 0.4;
/// Its initial window: 10 segments.
const INIT_WINDOW: f64 = 10.0 * MSS;
/// The window after a loss, as a share of the window (CUBIC's beta).
pub(crate) const BETA: f64 = 0.7;
/// HyStart's samples a round, and the window it starts testing at: 16
/// segments, as Linux's CUBIC.
const HYSTART_SAMPLES: u32 = 8;
const HYSTART_LOW_WINDOW: f64 = 16.0 * MSS;

/// One TCP connection in the window model, or a competing upload.
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
    /// CUBIC's window before its last cut, in segments; when its epoch
    /// began (`NIL` until the first acknowledgement past slow start or a
    /// cut), the window it grows back to and how long that takes, and the
    /// window Reno would have started the epoch with.
    w_max: f64,
    epoch: Time,
    origin: f64,
    k: f64,
    reno_start: f64,
    /// The sequence number past the furthest data segment sent (turmoil
    /// numbers a connection's segments), and that number when the window
    /// last fell: a loss of a segment sent before it belongs to the same
    /// recovery and doesn't cut the window again.
    sent_end: u64,
    recover: u64,
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
    /// yet, by sequence number (their record, when they went out and a lost
    /// retransmission's timeout), and those it retransmitted and hasn't
    /// seen acknowledged; how often their timeout backed off, and whether
    /// a tail-loss probe is pending. Bound: the segments in flight.
    lost: BTreeMap<u64, (u64, Time, Option<Time>)>,
    retx: BTreeSet<u64>,
    backoff: u32,
    probing: bool,
    /// When the retransmission timeout last fired.
    timed_out: Time,
}

impl Default for Flow {
    fn default() -> Flow {
        Flow {
            cwnd: INIT_WINDOW,
            ssthresh: f64::INFINITY,
            in_flight: 0,
            pending: VecDeque::new(),
            cut: Time::NIL,
            w_max: 0.0,
            epoch: Time::NIL,
            origin: 0.0,
            k: 0.0,
            reno_start: 0.0,
            sent_end: 0,
            recover: 0,
            flight_now: 0,
            flight_last: 0,
            round: Time::NIL,
            held: false,
            sent_at: BTreeMap::new(),
            srtt: Duration::ZERO,
            rttvar: Duration::ZERO,
            min_rtt: Duration::ZERO,
            hs_end: 0,
            hs_rtt: Duration::ZERO,
            hs_samples: 0,
            hs_found: false,
            last_send: Time::NIL,
            lost: BTreeMap::new(),
            retx: BTreeSet::new(),
            backoff: 0,
            probing: false,
            timed_out: Time::NIL,
        }
    }
}

impl Flow {
    /// The retransmission timeout, as Linux sets it: the smoothed round
    /// trip and four times its variation, that at least [`RTO_MIN`].
    fn rto(&self) -> Duration {
        self.srtt + (self.rttvar * 4).max(RTO_MIN)
    }

    /// When the tail-loss probe is due, the path's round trip `base` (for
    /// a flow with no round trip sampled yet): twice the smoothed round
    /// trip after the last send, and a delayed acknowledgement's
    /// [`RTO_MIN`] more with one segment in flight, as Linux's
    /// `tcp_schedule_loss_probe`.
    fn probe_at(&self, base: Duration) -> Time {
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
        self.last_send + (srtt * 2).max(Duration::from_millis(10)) + extra
    }

    /// A round-trip `sample` of the segment at `seq`: the smoothed round
    /// trip and its variation (RFC 6298), and HyStart's delay test, which
    /// ends slow start once a round's least round trip of its first
    /// [`HYSTART_SAMPLES`] passes the path's least by an eighth of it,
    /// clamped to 4-16 ms, as Linux's CUBIC does.
    fn sampled(&mut self, seq: u64, sample: Duration) {
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
        if self.hs_found || self.cwnd >= self.ssthresh || self.cwnd < HYSTART_LOW_WINDOW {
            return;
        }
        if seq >= self.hs_end {
            self.hs_end = self.sent_end;
            self.hs_rtt = Duration::MAX;
            self.hs_samples = 0;
        }
        self.hs_rtt = self.hs_rtt.min(sample);
        if self.hs_samples < HYSTART_SAMPLES {
            self.hs_samples += 1;
        } else {
            let thresh =
                (self.min_rtt / 8).clamp(Duration::from_millis(4), Duration::from_millis(16));
            if self.hs_rtt > self.min_rtt + thresh {
                self.hs_found = true;
                self.ssthresh = self.cwnd;
            }
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
        self.in_flight += len;
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
    /// `base` and its smoothed one `srtt`: slow start below the threshold,
    /// CUBIC (RFC 8312, as Linux's `bictcp_update`) above it. Its epoch
    /// starts at the first acknowledgement past slow start or a cut, from
    /// the window lost last or the window now if that is larger; the
    /// window then grows toward the cubic curve a base round trip ahead,
    /// by at most half a segment for each segment acknowledged (1.5 times
    /// a round trip), or follows the window Reno would have after as many
    /// smoothed round trips, whichever is larger.
    fn grow(&mut self, t: Time, len: usize, base: Duration, srtt: Duration) {
        if self.cwnd < self.ssthresh {
            self.cwnd += len as f64;
            return;
        }
        let cwnd = self.cwnd / MSS;
        if self.epoch.is_nil() {
            self.epoch = t;
            self.reno_start = cwnd;
            if self.w_max <= cwnd {
                self.k = 0.0;
                self.origin = cwnd;
            } else {
                self.k = ((self.w_max - cwnd) / CUBIC_C).cbrt();
                self.origin = self.w_max;
            }
        }
        let (base, srtt) = (base.as_secs_f64().max(1e-3), srtt.as_secs_f64().max(1e-3));
        let since = (t - self.epoch).as_secs_f64();
        let cubic = self.origin + CUBIC_C * (since + base - self.k).powi(3);
        let reno = self.reno_start + 3.0 * (1.0 - BETA) / (1.0 + BETA) * since / srtt;
        let segs = len as f64 / MSS;
        let mut step = if cubic > cwnd {
            ((cubic - cwnd) / cwnd).min(0.5)
        } else {
            0.01 / cwnd
        };
        if self.w_max == 0.0 {
            step = step.max(0.05);
        }
        let next = (cwnd + step * segs).max(reno.min(cwnd + 0.5 * segs));
        self.cwnd = next.max(cwnd) * MSS;
    }

    /// The window falls to [`BETA`] of itself at `t`, CUBIC's target
    /// lowered when it falls short of the last one (fast convergence).
    fn fall(&mut self, t: Time) {
        let w = self.cwnd / MSS;
        self.w_max = if w < self.w_max {
            w * (1.0 + BETA) / 2.0
        } else {
            w
        };
        self.epoch = Time::NIL;
        self.ssthresh = (self.cwnd * BETA).max(2.0 * MSS);
        self.cwnd = self.ssthresh;
        self.cut = t;
        self.recover = self.sent_end;
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

/// One competing upload (see the module docs).
struct Cross {
    host: HostIx,
    start: Time,
    stop: Time,
    rtt: Duration,
    flow: Flow,
    started: bool,
    /// A segment through the uplink whose acknowledgement its receiver
    /// delays for a second one: its sequence number, when it went out and
    /// when it left the uplink.
    odd: Option<(u64, Time, Time)>,
}

/// Whether a segment taking `tx` to send overflows an access queue that
/// holds `cap` of transmission time and has `wait` queued: when it doesn't
/// fit in what's left, as a byte-limited FIFO drops a packet. A segment
/// longer than the whole queue (a write the model keeps whole) is lost
/// only when the queue is full.
fn overflows(wait: Duration, tx: Duration, cap: Duration) -> bool {
    if tx > cap {
        wait >= cap
    } else {
        wait + tx > cap
    }
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
    /// The competing uploads, in host order.
    cross: Vec<Cross>,
    /// Per host, the wire bytes its competing uploads have sent.
    cross_bytes: Vec<u64>,
    /// Per host with competing uploads, [`Scheduler::cross_bytes`] every
    /// [`QUEUE_SAMPLE_EVERY`]: (time since the start, bytes).
    cross_sent: Vec<Vec<(Duration, u64)>>,
    /// Per host with competing uploads, their windows summed, sampled as
    /// [`Scheduler::cross_sent`] is.
    cross_cwnd: Vec<Vec<(Duration, u64)>>,
    /// Per host, the window model's segments its connections lost to a
    /// full access queue, and the acknowledgements its uplink carried.
    drops: Vec<u64>,
    acks: Vec<u64>,
    /// Per host, its competing uploads' segments lost to its full uplink.
    cross_drops: Vec<u64>,
    /// The acknowledgements receivers owe, by the connection whose data
    /// they acknowledge. Bound: one per connection with data received.
    owed: BTreeMap<(SocketAddr, SocketAddr), Owed>,
}

/// What a receiver owes its sender: the segments it hasn't acknowledged
/// (their records, sequence numbers and lengths), when its delayed
/// acknowledgement is due, when it last received, and how many segments it
/// still acknowledges at once (quickack).
#[derive(Clone, Debug, Default)]
struct Owed {
    waiting: Vec<(u64, u64, usize)>,
    due: Option<Time>,
    last: Time,
    quick: u32,
}

/// The segments a receiver acknowledges at once after a gap longer than
/// a retransmission timeout, as Linux's `TCP_MAX_QUICKACKS`.
const QUICKACKS: u32 = 16;

/// How long a receiver delays the acknowledgement of a lone segment, as
/// Linux's `TCP_DELACK_MIN`.
const DELACK: Duration = Duration::from_millis(40);

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
        let mut net = Network::new(&sites, options, &mut det_rng(seed ^ 0x5741_4e00));
        for (h, hs) in spec.hosts.iter().enumerate() {
            if !hs.up_steps.is_empty() {
                let steps = hs.up_steps.iter().map(|&(at, rate)| (sim_time(at), rate));
                net.set_up_steps(h, steps.collect());
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
                    rtt: Duration::from(c.rtt),
                    flow: Flow::default(),
                    started: false,
                    odd: None,
                })
            })
            .collect();
        let windowed = buffers.as_ref().is_some_and(|b| b.model().window);
        if !cross.is_empty() && !windowed {
            return Err("wan: competing uploads need the window model".to_owned());
        }
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
            cross,
            cross_bytes: vec![0; spec.hosts.len()],
            cross_sent: vec![Vec::new(); spec.hosts.len()],
            cross_cwnd: vec![Vec::new(); spec.hosts.len()],
            drops: vec![0; spec.hosts.len()],
            acks: vec![0; spec.hosts.len()],
            cross_drops: vec![0; spec.hosts.len()],
            owed: BTreeMap::new(),
        })
    }

    fn name(&self, h: HostIx) -> &str {
        self.names.get(h).map_or("?", String::as_str)
    }

    /// The model's view at `now` of the connection whose sender's address
    /// pair is `conn`.
    pub(crate) fn conn_state(&self, conn: (SocketAddr, SocketAddr), now: Time) -> ConnState {
        let flow = self.flows.get(&conn);
        let pending = flow.map(|f| {
            f.pending
                .iter()
                .filter_map(|id| match self.records.get(id)?.what {
                    What::Data { len, .. } => Some(len as u64),
                    _ => None,
                })
                .sum()
        });
        let host = self.ip_host.get(&conn.0.ip()).copied();
        ConnState {
            cwnd: flow.map(|f| f.cwnd as u64),
            ssthresh: flow.and_then(|f| f.ssthresh.is_finite().then_some(f.ssthresh as u64)),
            in_flight: flow.map(|f| f.in_flight as u64),
            pending,
            uplink_queue: host.map(|h| (self.net.up_free(h) - now).max(Duration::ZERO)),
            uplink_rate: host.map(|h| self.net.up_rate(h, now)),
        }
    }

    /// One walk, between two steps (see the module docs).
    pub fn walk(&mut self, sim: &Sim<'_>) -> Result<(), String> {
        let now = sim_time(sim.elapsed());
        self.run_events(now);
        for i in 0..self.cross.len() {
            if let Some(c) = self.cross.get_mut(i)
                && !c.started
                && now >= c.start
            {
                c.started = true;
                let start = c.start;
                self.cross_pump(start, i);
            }
        }
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
            match event {
                Event::Ack { conn, seq, len } => {
                    self.acked(t, conn, seq, len);
                    continue;
                }
                Event::Probe { conn } => {
                    self.probe(t, conn);
                    continue;
                }
                Event::DelAck { conn } => {
                    if self.owed.get(&conn).is_some_and(|o| o.due == Some(t)) {
                        self.ack_owed(t, conn);
                    }
                    continue;
                }
                Event::CrossAck { i, seq, rtt } => {
                    self.cross_acked(t, i, seq, rtt);
                    continue;
                }
                Event::CrossLost { i, seq } => {
                    self.cross_lost(t, i, seq);
                    continue;
                }
                Event::CrossDelAck { i, seq } => {
                    let odd = self.cross.get_mut(i).and_then(|c| match c.odd {
                        Some((s, sent, _)) if s == seq => {
                            c.odd = None;
                            Some(sent)
                        }
                        _ => None,
                    });
                    if let Some(sent) = odd {
                        self.cross_acked(t, i, seq, t - sent);
                    }
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
                    if let (Some(cap), Some(_)) = (window, data)
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
                        let (conn, seq) = ((r.src, r.dst), data.map_or(0, |(s, _)| s));
                        self.dropped(t, conn, seq, id);
                        continue;
                    }
                    let done = self.net.receive(t, r.to, r.wire);
                    if done <= now {
                        r.state = State::Due;
                    } else {
                        self.events.insert((done, id), Event::Deliver);
                    }
                    if let (Some(_), Some((seq, len))) = (window, data) {
                        let (conn, from, to) = ((r.src, r.dst), r.from, r.to);
                        self.owe_ack(done.max(t), conn, from, to, (id, seq, len));
                    }
                }
                Event::Deliver => r.state = State::Due,
                Event::Lost | Event::Timeout => {
                    let conn = (r.src, r.dst);
                    let (seq, len) = match r.what {
                        What::Data { seq, len } => (seq, len),
                        _ => (0, 0),
                    };
                    let f = self.flows.entry(conn).or_default();
                    if matches!(event, Event::Timeout) {
                        // Unless a later segment's acknowledgement told first.
                        if f.lost.get(&seq).is_none_or(|&(_, _, due)| due != Some(t)) {
                            continue;
                        }
                        f.lost.remove(&seq);
                    }
                    f.in_flight = f.in_flight.saturating_sub(len);
                    f.sent_at.remove(&seq);
                    if f.cut.is_nil() || seq >= f.recover {
                        f.fall(t);
                    }
                    if matches!(event, Event::Timeout) && f.timed_out != t {
                        // Linux's loss state: the window restarts from a
                        // segment, and the timeout backs off, once for the
                        // segments it finds lost together.
                        f.cwnd = MSS;
                        f.backoff = f.backoff.saturating_add(1);
                        f.timed_out = t;
                    }
                    f.retx.insert(seq);
                    f.pending.push_front(id);
                    self.pump(t, &[conn]);
                }
                Event::Ack { .. }
                | Event::Probe { .. }
                | Event::DelAck { .. }
                | Event::CrossAck { .. }
                | Event::CrossLost { .. }
                | Event::CrossDelAck { .. } => {}
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

    /// The `len` bytes at `seq` of `conn` were acknowledged at `t`. The
    /// window doesn't grow in a recovery (for data sent before it fell).
    fn acked(&mut self, t: Time, conn: (SocketAddr, SocketAddr), seq: u64, len: usize) {
        let base = self.conn_rtt(conn);
        let Some(f) = self.flows.get_mut(&conn) else {
            return;
        };
        let sent = f.sent_at.remove(&seq);
        if let Some(sent) = sent {
            f.sampled(seq, t - sent);
        }
        if f.limited() && (f.cut.is_nil() || seq >= f.recover) {
            f.grow(t, len, base, f.srtt.max(base));
        }
        f.in_flight = f.in_flight.saturating_sub(len);
        if !f.retx.remove(&seq) {
            f.backoff = 0;
        }
        // RACK: the acknowledgement of a segment sent later shows the
        // earlier ones lost once a quarter of the least round trip passes.
        if let Some(sent) = sent {
            let earlier: Vec<u64> = f
                .lost
                .iter()
                .filter(|(_, v)| v.1 < sent)
                .map(|(&s, _)| s)
                .collect();
            let reo = f.min_rtt / 4;
            for s in earlier {
                if let Some((id, _, _)) = f.lost.remove(&s) {
                    self.events.insert((t + reo, id), Event::Lost);
                }
            }
        }
        self.pump(t, &[conn]);
    }

    /// The access queues dropped segment `seq` (record `id`) of `conn` at
    /// `t`. Its sender learns so from the acknowledgement of a segment sent
    /// after it ([`Scheduler::acked`]), or failing that from its tail-loss
    /// probe's ([`Scheduler::probe`]); a lost retransmission fails over to
    /// the retransmission timeout, doubled for each time it backed off.
    fn dropped(&mut self, t: Time, conn: (SocketAddr, SocketAddr), seq: u64, id: u64) {
        let base = self.conn_rtt(conn);
        let f = self.flows.entry(conn).or_default();
        let sent = f.sent_at.get(&seq).copied().unwrap_or(t);
        if f.retx.contains(&seq) {
            let due = (sent + f.rto() * (1i64 << f.backoff.min(6))).max(t);
            f.lost.insert(seq, (id, sent, Some(due)));
            self.events.insert((due, id), Event::Timeout);
            return;
        }
        f.lost.insert(seq, (id, sent, None));
        if !f.probing {
            f.probing = true;
            let at = f.probe_at(base).max(t);
            let key = self.next;
            self.next += 1;
            self.events.insert((at, key), Event::Probe { conn });
        }
    }

    /// `conn`'s tail-loss probe is due at `t`: unless the flow sent since,
    /// which moves it, the probe leaves now and its acknowledgement, a
    /// round trip later, shows the segments lost.
    fn probe(&mut self, t: Time, conn: (SocketAddr, SocketAddr)) {
        let base = self.conn_rtt(conn);
        let Some(f) = self.flows.get_mut(&conn) else {
            return;
        };
        f.probing = false;
        if f.lost.values().all(|&(_, _, due)| due.is_some()) {
            return;
        }
        let at = f.probe_at(base);
        if at > t {
            f.probing = true;
            let key = self.next;
            self.next += 1;
            self.events.insert((at, key), Event::Probe { conn });
            return;
        }
        let back = if f.srtt > Duration::ZERO {
            f.srtt
        } else {
            base
        };
        // The probe shows the original segments lost; retransmissions
        // lost again wait for their timeout.
        let found: Vec<u64> = f
            .lost
            .iter()
            .filter(|(_, v)| v.2.is_none())
            .map(|(&s, _)| s)
            .collect();
        for s in found {
            if let Some((id, _, _)) = f.lost.remove(&s) {
                self.events.insert((t + back, id), Event::Lost);
            }
        }
    }

    /// Segment `seg` (its record, sequence number and length) of `conn`
    /// from `from` was through its receiver `to`'s downlink at `t`, which
    /// owes its acknowledgement, as Linux's `__tcp_ack_snd_check` and
    /// delayed acknowledgements send it: at once for every second segment,
    /// [`DELACK`] after a lone one, and at once for the first
    /// [`QUICKACKS`] segments of a connection or after a gap longer than a
    /// retransmission timeout; unless data the receiver sends back on the
    /// connection first carries it ([`Scheduler::piggyback`]). The
    /// acknowledgement clocks the sender's window, and a pure one takes
    /// its turn on the receiver's uplink: on the testbed, a seed
    /// acknowledged its viewers' 20-30-byte segments one for every two.
    fn owe_ack(
        &mut self,
        t: Time,
        conn: (SocketAddr, SocketAddr),
        from: HostIx,
        to: HostIx,
        seg: (u64, u64, usize),
    ) {
        let gap = RTO_MIN + self.net.rtt(from, to);
        let o = self.owed.entry(conn).or_default();
        if o.last.is_nil() || t - o.last > gap {
            o.quick = QUICKACKS;
        }
        o.last = t;
        o.waiting.push(seg);
        if o.quick > 0 || o.waiting.len() >= 2 {
            o.quick = o.quick.saturating_sub(1);
            self.ack_owed(t, conn);
        } else if o.due.is_none() {
            o.due = Some(t + DELACK);
            let key = self.next;
            self.next += 1;
            self.events
                .insert((t + DELACK, key), Event::DelAck { conn });
        }
    }

    /// The receiver of `conn` sends a pure acknowledgement at `t` of the
    /// segments it owes: it takes its turn on the receiver's uplink and
    /// reaches the sender a one-way trip after.
    fn ack_owed(&mut self, t: Time, conn: (SocketAddr, SocketAddr)) {
        let (Some(&from), Some(&to)) = (
            self.ip_host.get(&conn.0.ip()),
            self.ip_host.get(&conn.1.ip()),
        ) else {
            return;
        };
        let Some(o) = self.owed.get_mut(&conn) else {
            return;
        };
        o.due = None;
        let waiting = std::mem::take(&mut o.waiting);
        if waiting.is_empty() {
            return;
        }
        let depart = self.net.occupy_up(t, to, tcp_wire(0).0);
        if let Some(a) = self.acks.get_mut(to) {
            *a += 1;
        }
        self.acked_at(depart + self.net.one_way(to, from), conn, waiting);
    }

    /// The acknowledgement of `waiting` (segments of `conn`) reaches the
    /// sender at `at`.
    fn acked_at(
        &mut self,
        at: Time,
        conn: (SocketAddr, SocketAddr),
        waiting: Vec<(u64, u64, usize)>,
    ) {
        for (id, seq, len) in waiting {
            self.events.insert((at, id), Event::Ack { conn, seq, len });
        }
    }

    /// Data of `conn` leaves its sender and reaches the peer at `arrival`:
    /// it acknowledges what its sender owes on the reverse connection.
    fn piggyback(&mut self, conn: (SocketAddr, SocketAddr), arrival: Time) {
        let back = (conn.1, conn.0);
        let Some(o) = self.owed.get_mut(&back) else {
            return;
        };
        o.due = None;
        let waiting = std::mem::take(&mut o.waiting);
        self.acked_at(arrival, back, waiting);
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

    /// Competing upload `i`'s segment `seq` was acknowledged at `t`, a
    /// round trip of `rtt` after it went out: its window grows as the
    /// window model's connections' do.
    fn cross_acked(&mut self, t: Time, i: usize, seq: u64, rtt: Duration) {
        if let Some(c) = self.cross.get_mut(i) {
            let f = &mut c.flow;
            f.in_flight = f.in_flight.saturating_sub(CROSS_SEGMENT);
            f.sampled(seq, rtt);
            if f.cut.is_nil() || seq >= f.recover {
                f.grow(t, CROSS_SEGMENT, c.rtt, f.srtt.max(c.rtt));
            }
        }
        self.cross_pump(t, i);
    }

    /// Competing upload `i` learned at `t` that its segment `seq` was
    /// lost: its window falls once a recovery.
    fn cross_lost(&mut self, t: Time, i: usize, seq: u64) {
        if let Some(c) = self.cross.get_mut(i) {
            let f = &mut c.flow;
            f.in_flight = f.in_flight.saturating_sub(CROSS_SEGMENT);
            if f.cut.is_nil() || seq >= f.recover {
                f.fall(t);
            }
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
            if at < c.start
                || at >= c.stop
                || (f.in_flight > 0 && (f.in_flight + CROSS_SEGMENT) as f64 > f.cwnd)
            {
                return;
            }
            let (host, rtt) = (c.host, c.rtt);
            let seq = f.sent_end;
            f.sent_end += 1;
            f.sent(at, CROSS_SEGMENT, f.srtt.max(rtt));
            let id = self.next;
            self.next += 1;
            let wait = (self.net.up_free(host) - at).max(Duration::ZERO);
            if overflows(wait, tx_time(CROSS_WIRE, self.net.up_rate(host, at)), cap) {
                if let Some(d) = self.cross_drops.get_mut(host) {
                    *d += 1;
                }
                self.events
                    .insert((at + rtt + wait, id), Event::CrossLost { i, seq });
                continue;
            }
            let depart = self.net.occupy_up(at, host, CROSS_WIRE);
            if let Some(b) = self.cross_bytes.get_mut(host) {
                *b += CROSS_WIRE as u64;
            }
            // The receiver acknowledges every second segment, and a lone one
            // after DELACK.
            match c.odd.take() {
                Some((first, sent, _)) => {
                    let ack = depart + rtt;
                    let key = self.next;
                    self.next += 1;
                    self.events.insert(
                        (ack, key),
                        Event::CrossAck {
                            i,
                            seq: first,
                            rtt: ack - sent,
                        },
                    );
                    self.events.insert(
                        (ack, id),
                        Event::CrossAck {
                            i,
                            seq,
                            rtt: ack - at,
                        },
                    );
                }
                None => {
                    c.odd = Some((seq, at, depart));
                    self.events
                        .insert((depart + rtt + DELACK, id), Event::CrossDelAck { i, seq });
                }
            }
        }
    }

    /// Sends what `conns`' windows allow at `at`, a segment from each in
    /// turn.
    fn pump(&mut self, at: Time, conns: &[(SocketAddr, SocketAddr)]) {
        let mut open: Vec<(SocketAddr, SocketAddr)> = conns.to_vec();
        let rtts: Vec<Duration> = open.iter().map(|&c| self.conn_rtt(c)).collect();
        let rtt_of = |conn: &(SocketAddr, SocketAddr)| {
            conns
                .iter()
                .position(|c| c == conn)
                .and_then(|i| rtts.get(i).copied())
                .unwrap_or(Duration::ZERO)
        };
        while !open.is_empty() {
            open.retain(|conn| {
                let rtt = rtt_of(conn);
                let Some(f) = self.flows.get_mut(conn) else {
                    return false;
                };
                loop {
                    let Some(&id) = f.pending.front() else {
                        f.held = false;
                        return false;
                    };
                    let Some((seq, len)) = self.records.get(&id).and_then(|r| match r.what {
                        What::Data { seq, len } => Some((seq, len)),
                        _ => None,
                    }) else {
                        f.pending.pop_front();
                        continue;
                    };
                    if f.in_flight > 0 && (f.in_flight + len) as f64 > f.cwnd {
                        f.held = true;
                        return false;
                    }
                    f.pending.pop_front();
                    f.sent(at, len, f.srtt.max(rtt));
                    f.sent_end = f.sent_end.max(seq + 1);
                    f.sent_at.insert(seq, at);
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
        let (seq, len) = match r.what {
            What::Data { seq, len } => (seq, len),
            _ => (0, 0),
        };
        if self.buffers.is_some() {
            self.admits.insert((at, id), (conn, len));
        }
        let wait = (self.net.up_free(from) - at).max(Duration::ZERO);
        if overflows(wait, tx_time(wire, self.net.up_rate(from, at)), cap) {
            if let Some(d) = self.drops.get_mut(from) {
                *d += 1;
            }
            self.dropped(at, conn, seq, id);
            return;
        }
        let t = self.net.send(at, from, to, wire, packets, Carrier::Stream);
        self.piggyback(conn, t.arrival + t.retransmit);
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

    /// [`Scheduler::send`] with send buffers.
    fn send_written(&mut self, at: Time, fresh: Vec<u64>, b: &SendBuffers) {
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
            // (write order, queue position, record)
            let mut keyed: Vec<(u64, usize, u64)> = Vec::new();
            for q in queues.into_values() {
                let mut last = 0;
                for id in q {
                    let stamp = self.records.get(&id).and_then(|r| match r.what {
                        What::Data { .. } => b.next_write((r.src, r.dst)),
                        _ => None,
                    });
                    last = stamp.unwrap_or(last);
                    keyed.push((last, keyed.len(), id));
                }
            }
            keyed.sort_unstable();
            for (_, _, id) in keyed {
                let data = self.records.get(&id).and_then(|r| match r.what {
                    What::Data { .. } if window => Some((r.src, r.dst)),
                    _ => None,
                });
                match data {
                    Some(conn) => {
                        self.flows.entry(conn).or_default().pending.push_back(id);
                        self.pump(at, &[conn]);
                    }
                    None => self.transmit(at, id),
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
        self.owed.retain(|(src, dst), _| {
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
                        competing: self.cross_sent.get(h).cloned().unwrap_or_default(),
                        queue_drops: self.drops.get(h).copied().unwrap_or(0),
                        acks: self.acks.get(h).copied().unwrap_or(0),
                        competing_drops: self.cross_drops.get(h).copied().unwrap_or(0),
                        competing_cwnd: self.cross_cwnd.get(h).cloned().unwrap_or_default(),
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
        // The window, not the queue, paces the writes: the bytes still take
        // their 1.2 s on the wire, and a segment waits at most the queue, as
        // one that doesn't fit is lost.
        assert!(wrote > 700, "last write after {wrote} ms");
        assert!((1190..1300).contains(&got), "{got} ms");
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
        // downlink, and `b` gets the rest of the uplink.
        assert!(b_fifo > 2000, "{b_fifo} ms");
        assert!(b < b_fifo * 3 / 4, "{b} ms against {b_fifo} ms");
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

    /// `a`, on a 10 Mbit/s uplink with 100 ms windowed queues and a
    /// competing upload over `competing` (s), writes `bytes` to `b` from
    /// 1 s on. Returns when `b` had them (ms from 1 s) and `a`'s access
    /// link report.
    fn beside_an_upload(bytes: usize, competing: (u64, u64)) -> (i64, WanHostReport) {
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
        spec.hosts[0].competing = vec![Competing {
            start: StdDuration::from_secs(competing.0),
            stop: StdDuration::from_secs(competing.1),
            rtt: StdDuration::from_millis(30),
        }];
        let mut wan = Scheduler::new(&sim, &spec, QUIET, 1, Some(buffers)).unwrap();
        wan.set_latency(0, 1, Duration::from_millis(15));
        drive(&mut sim, &mut wan);
        (got.get(), wan.report(&spec).hosts.swap_remove(0))
    }

    #[test]
    fn a_competing_upload_shares_the_uplink() {
        let sent = |a: &WanHostReport, s: i64| {
            a.competing
                .iter()
                .rev()
                .find(|&&(t, _)| t <= Duration::from_secs(s))
                .map_or(0, |&(_, b)| b)
        };
        // Alone, the upload fills the uplink, through a queue it keeps from
        // emptying.
        let (_, a) = beside_an_upload(0, (0, 30));
        let alone = sent(&a, 3) - sent(&a, 1);
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
        let (quiet, _) = beside_an_upload(1_448_000, (30, 30));
        let (shared, a) = beside_an_upload(1_448_000, (0, 30));
        assert!((1150..1500).contains(&quiet), "{quiet} ms");
        assert!(shared > quiet * 3 / 2, "{shared} ms against {quiet} ms");
        let secs = (shared / 1000).max(1);
        let during = sent(&a, 1 + secs) - sent(&a, 1);
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
