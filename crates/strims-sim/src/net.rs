// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The socket provider: turmoil's sockets, with TCP send buffers in WAN
//! mode when [`Scenario::send_buffer`](crate::Scenario::send_buffer) is set.
//!
//! turmoil accepts every write at once, so in WAN mode a sender's writes
//! queue on its access link however far behind it falls, and nothing
//! pushes back on vnic. With send buffers a TCP connection whose socket has
//! a `TCP_NOTSENT_LOWAT` mark (the node's
//! [`TcpSocketConfig`](strims_node::TcpSocketConfig)) holds its writes while
//! its unsent bytes reach the mark, as Linux does: Linux checks the mark
//! only as a write starts a new buffer, and fills that buffer to a GSO
//! segment ([`GSO_SEGMENT`]) first, so a burst of writes leaves up to the
//! mark plus a segment unsent, not the mark alone. A write it refuses
//! leaves the socket unwritable until its unsent bytes fall under half the
//! mark, as a real socket's readiness does, however often its writer is
//! polled meanwhile. With [`SendBuffer::window`] the bytes a write's
//! connection's window lets out at once count as sent from the write, as
//! Linux's write pushes what the window allows before the next
//! (`tcp_push`): a burst of writes leaves its first window's bytes out of
//! the mark and out of the buffer it fills, whose unsent remainder later
//! writes join, though the scheduler admits them only at its next walk
//! (sent at the step they were written in). Each write is stamped with
//! its order on the host, so the WAN scheduler hands a sender's segments
//! to its uplink in the order its sockets took them. A byte is
//! unsent from its write until the WAN scheduler admits its segment: to
//! the sender's uplink queue once that queue holds less than
//! [`SendBuffer::uplink_queue`] of transmission time, where the segment's
//! timing on the WAN doesn't change (only when the writer may write again
//! does); or, with [`SendBuffer::window`], when the connection's
//! congestion window lets it out, so a slow receiver or a long round trip
//! holds back that connection alone (see the WAN scheduler).
//!
//! A connection that carries a modelled data channel
//! ([`SimNet::open_channel_as`]) takes writes as a channel does instead: each
//! whole, as a message, while its buffered amount (bytes unsent, and for a
//! stack whose amount counts them, sent and not yet acknowledged by a SACK:
//! [`SctpProfile::buffered_flight`])
//! is under the channel's cap, and once that is reached, again only from
//! under half of it. A node's channel is
//! paced as the browser's `DcSocket` paces it: under its own pacer's cap,
//! and the host's channels together under a [`HostPacer`]'s, each waiting
//! for its mark once refused: one the host holds reads the host's load
//! again every [`HOST_POLL`] from its hold, as the browser's writer polls
//! it, against the mark the host held it for, and goes once the load is
//! at it; nothing else wakes it. On a stack whose page counts a channel's amount itself
//! ([`SctpProfile::amount_at_sends`]), a node's writer and its host's
//! pacer read the amount so: its last reading and each message written
//! since, the stack's as the stack answers ([`SendBuffers::answered`]: a
//! send opportunity of its association, on its host's thread, left the
//! channel's queue empty, or had sent more than 100 KiB of it since the last
//! answer); between, as the stack sends, it holds. A writer waiting for
//! its mark wakes on an answer. A
//! write goes to turmoil as segments of a DATA chunk's payload at most,
//! which the window model's association sends as packets.
//!
//! With the node's link log (`NodeConfig::link_log`, [`Net::log_link`]), a
//! connection hands it what the browser's writer would read: its buffered
//! amount (a TCP socket's unsent bytes) at each write it decides, with the
//! mark it then waits for, and what holds the write (its own cap, the
//! host's, or a socket's mark); and a paced channel's `bufferedamountlow`
//! as the browser fires it: each time its amount falls to its threshold
//! from above, its writer waiting or not. The threshold is the mark of the
//! writer's last refusal, and half the channel's cap before one, as the
//! browser sets it as it opens the channel.

// `Net` sockets are `Send`, so the state they share with the scheduler
// sits behind a lock, which only this module names. Everything runs on
// turmoil's thread, so it is never contended.
#![allow(clippy::disallowed_types)]

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use strims_base::time::Time;
use strims_node::config::LinkSink;
use strims_node::event::{LinkEvent, SampleSource, WriteWait};
use strims_node::{
    CaptureSample, ChannelPacer, HOST_POLL, HostPacer, HostSum, LinkProbe, LinkSetup, Net,
    TcpOptions, TcpSocketConfig, TurmoilNet,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::fast_map::FastMap;
use crate::spec::SendBuffer;
use crate::wan::{Chunking, SctpProfile, SharedWan, sim_time};

/// A TCP connection by its local and peer addresses.
pub(crate) type ConnKey = (SocketAddr, SocketAddr);

#[derive(Debug, Default)]
struct Conn {
    /// The socket's `TCP_NOTSENT_LOWAT`, once the node set it.
    lowat: Option<u64>,
    /// Bytes written and not yet admitted to the uplink queue ...
    unsent: u64,
    /// ... and of them, those in the last buffer a write started, which
    /// later writes fill to [`GSO_SEGMENT`] whatever the mark.
    tail: u64,
    /// What the connection's window lets out of its next writes at once,
    /// in packets, as the WAN scheduler last found it
    /// ([`SendBuffers::set_rooms`]) ...
    room: u64,
    /// ... and of `unsent`, the bytes written into it: sent from their
    /// write as far as the mark and the tail go, the scheduler admitting
    /// them at its next walk.
    early: u64,
    /// The writer a refused write left waiting. Until it is woken the
    /// socket stays unwritable, as a real socket's readiness does once a
    /// write would block: it isn't polled again before the wake.
    waker: Option<Waker>,
    /// The host-wide order of each write (one turmoil segment) the WAN
    /// scheduler hasn't taken yet, and whether it ends a data channel's
    /// message. Bound: the connection's segments written in one step.
    writes: VecDeque<(u64, bool)>,
    /// A data channel's cap and how it cuts its writes into chunks, and
    /// its bytes sent and not yet acknowledged.
    channel: Option<(u64, Chunking)>,
    unacked: u64,
    /// The channel's buffered amount counts `unacked`.
    flight: bool,
    /// A node's channel: its writer paces the buffered amount
    /// ([`ChannelPacer`]) under the cap, and while it waits, reads the
    /// amount only once it falls to the mark it waits for.
    pacer: Option<ChannelPacer>,
    low: u64,
    /// The pacer refused the last write: the channel waits on its own cap.
    waits: bool,
    /// The host's pacer held the writer's write: the host's load it waits
    /// for, until the write goes.
    held: Option<usize>,
    /// The channel's `bufferedAmountLowThreshold`: the mark its writer set
    /// at its last refusal, half the channel's cap before one; and whether
    /// its amount was above it at its last reading, so that a fall to it
    /// fires the event.
    threshold: u64,
    above: bool,
    /// On a stack whose page counts the amount itself
    /// ([`SctpProfile::amount_at_sends`]), a node's channel's amount as its
    /// writer reads it (see the module docs).
    shown: Option<u64>,
    /// The node's link log, and the link's name in it ([`Net::log_link`]).
    log: Option<(u64, LinkSink)>,
}

impl Conn {
    fn record(&self, event: LinkEvent) {
        if let Some((link, sink)) = &self.log {
            sink.record(*link, event);
        }
    }

    /// The bytes Linux would count unsent (`tcp_notsent_bytes`): those
    /// written and not yet admitted, less those the window let out at
    /// once.
    fn notsent(&self) -> u64 {
        self.unsent.saturating_sub(self.early)
    }

    /// A data channel's buffered amount.
    fn buffered(&self) -> u64 {
        self.unsent + if self.flight { self.unacked } else { 0 }
    }

    /// A data channel's buffered amount as its writer reads it.
    fn amount(&self) -> u64 {
        self.shown.unwrap_or_else(|| self.buffered())
    }
}

/// The most a socket's buffer takes before Linux checks
/// `TCP_NOTSENT_LOWAT` again: a GSO segment, 64 KiB less its headers in
/// whole 1448-byte segments. TCP sockets always segment in software, so
/// this holds with the device's TSO and GSO off, as on the testbed, whose
/// sockets leave at most 63712 bytes unsent at a 16 KiB mark.
pub(crate) const GSO_SEGMENT: u64 = 44 * 1448;

/// Every connection's unsent bytes, shared by the hosts' sockets and the
/// WAN scheduler.
#[derive(Clone, Debug)]
pub(crate) struct SendBuffers(Arc<Mutex<Buffers>>);

#[derive(Debug)]
struct Buffers {
    model: SendBuffer,
    /// The next write's order.
    stamp: u64,
    /// Bound: one per open TCP connection.
    conns: FastMap<ConnKey, Conn>,
    /// Each host's paced channels, by its address. Bound: one per host.
    hosts: FastMap<IpAddr, HostChannels>,
}

/// A host's paced channels together.
#[derive(Debug, Default)]
struct HostChannels {
    pacer: HostPacer,
    /// Bound: the host's open paced channels.
    channels: Vec<ConnKey>,
}

impl HostChannels {
    /// The channels now.
    fn sum(&self, conns: &FastMap<ConnKey, Conn>, now: Time) -> HostSum {
        let mut sum = HostSum::default();
        for c in self.channels.iter().filter_map(|k| conns.get(k)) {
            if let Some(p) = &c.pacer {
                sum.add(p, now, c.amount() as usize, c.waits);
            }
        }
        sum
    }
}

impl SendBuffers {
    pub fn new(model: SendBuffer) -> SendBuffers {
        SendBuffers(Arc::new(Mutex::new(Buffers {
            model,
            stamp: 0,
            conns: FastMap::default(),
            hosts: FastMap::default(),
        })))
    }

    fn with<R>(&self, f: impl FnOnce(&mut Buffers) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn model(&self) -> SendBuffer {
        self.with(|b| b.model)
    }

    /// `conn`'s unsent bytes and `TCP_NOTSENT_LOWAT`, while it is open.
    fn conn(&self, conn: ConnKey) -> Option<(u64, Option<u64>)> {
        self.with(|b| b.conns.get(&conn).map(|c| (c.notsent(), c.lowat)))
    }

    /// The order of `conn`'s next write the scheduler hasn't taken (writes
    /// on every connection count in the order the hosts made them), and
    /// whether it ends a data channel's message.
    pub fn next_write(&self, conn: ConnKey) -> Option<(u64, bool)> {
        self.with(|b| b.conns.get_mut(&conn)?.writes.pop_front())
    }

    /// Each of `rooms`' connections lets out that many packets of its next
    /// writes at once (see the module docs); a data channel's takes none.
    pub fn set_rooms(&self, rooms: &[(ConnKey, u64)]) {
        self.with(|b| {
            for &(conn, room) in rooms {
                if let Some(c) = b.conns.get_mut(&conn)
                    && c.channel.is_none()
                {
                    c.room = room;
                }
            }
        });
    }

    /// `len` bytes of `conn` reached the uplink queue at `at`. Returns the
    /// writer to wake, if it waits: once its unsent bytes are under half the
    /// mark, when Linux reports a socket with the mark writable. A data
    /// channel's bytes stay buffered until they are acknowledged
    /// ([`SendBuffers::acked`]) where its stack's amount counts the flight,
    /// and leave its amount now where it doesn't; a page that counts the
    /// amount reads it fall only as its stack answers
    /// ([`SendBuffers::answered`]).
    pub fn admit(&self, conn: ConnKey, len: usize, at: Time) -> Option<Waker> {
        self.with(|b| {
            let c = b.conns.get_mut(&conn)?;
            let early = c.early.min(len as u64);
            c.early -= early;
            c.unsent = c.unsent.saturating_sub(len as u64);
            c.tail = c.tail.min(c.notsent());
            if c.channel.is_some() {
                c.unacked = c.unacked.saturating_add(len as u64);
                return if c.flight || c.shown.is_some() {
                    None
                } else {
                    release(b, conn, at)
                };
            }
            let lowat = c.lowat?;
            if 2 * c.notsent() < lowat {
                c.waker.take()
            } else {
                None
            }
        })
    }
}

impl SendBuffers {
    /// `conn` carries a data channel of `cap` bytes buffered at most, sent
    /// as `profile` sends (its chunks, and whether its buffered amount
    /// counts the flight).
    fn open_channel(&self, conn: ConnKey, cap: u64, profile: &SctpProfile) {
        self.with(|b| {
            if let Some(c) = b.conns.get_mut(&conn) {
                c.channel = Some((cap, Chunking::new(profile)));
                // The browser's `DcSocket::new`: half the channel's cap.
                c.threshold = cap / 2;
                c.flight = profile.buffered_flight;
            }
        });
    }

    /// Data channel `conn`'s writer paces it from now on, as a node's page
    /// paces its link's messages, among its host's paced channels, and on a
    /// stack whose page counts the amount (`profile`'s), counts it from
    /// none.
    fn pace(&self, conn: ConnKey, profile: &SctpProfile) {
        self.with(|b| {
            if let Some(c) = b.conns.get_mut(&conn)
                && c.channel.is_some()
                && c.pacer.is_none()
            {
                c.pacer = Some(ChannelPacer::default());
                c.shown = profile.amount_at_sends.then_some(0);
                b.hosts.entry(conn.0.ip()).or_default().channels.push(conn);
            }
        });
    }

    /// A SACK acknowledged `len` bytes of data channel `conn` at `at`.
    /// Returns the writer to wake once its buffered amount is under half
    /// its cap, or a paced writer's waiting on its own cap once it is at
    /// its mark.
    pub fn acked(&self, conn: ConnKey, len: usize, at: Time) -> Option<Waker> {
        self.with(|b| {
            let c = b.conns.get_mut(&conn)?;
            c.unacked = c.unacked.saturating_sub(len as u64);
            if c.flight { release(b, conn, at) } else { None }
        })
    }

    /// Data channel `conn`'s stack answered its page at `at`: a page that
    /// counts the amount ([`SctpProfile::amount_at_sends`]) reads it as the
    /// stack's from now on. Returns the writer to wake, a paced writer's
    /// waiting on its own cap once the amount is at its mark.
    pub fn answered(&self, conn: ConnKey, at: Time) -> Option<Waker> {
        self.with(|b| {
            let c = b.conns.get_mut(&conn)?;
            if c.shown.is_some() {
                c.shown = Some(c.buffered());
            }
            release(b, conn, at)
        })
    }

    /// Data channel `conn`'s buffered amount, as its writer reads it.
    pub fn buffered(&self, conn: ConnKey) -> Option<u64> {
        self.with(|b| {
            let c = b.conns.get(&conn)?;
            c.channel.map(|_| c.amount())
        })
    }
}

/// turmoil's sockets, with send buffers when `buffers` is set.
#[derive(Clone, Debug, Default)]
pub(crate) struct SimNet {
    buffers: Option<SendBuffers>,
    /// The run's WAN model, for the capture's link probes.
    wan: Option<SharedWan>,
    /// [`Scenario::tcp_segment`](crate::Scenario::tcp_segment).
    segment: Option<usize>,
    /// [`HostSpec::rtc_stack`](crate::HostSpec::rtc_stack), under the
    /// window model.
    rtc_stack: Option<crate::wan::SctpProfile>,
}

impl SimNet {
    pub fn new(buffers: Option<SendBuffers>) -> SimNet {
        SimNet {
            buffers,
            wan: None,
            segment: None,
            rtc_stack: None,
        }
    }

    /// The node's WebRTC connections are data channels sent as `stack`
    /// sends ([`Net::models_channels`]); `None`: str0m over UDP.
    pub fn with_rtc_stack(self, stack: Option<crate::wan::SctpProfile>) -> SimNet {
        SimNet {
            rtc_stack: stack,
            ..self
        }
    }

    /// Splits TCP writes into pieces of at most `segment` bytes.
    pub fn with_segment(self, segment: Option<usize>) -> SimNet {
        SimNet { segment, ..self }
    }

    /// `stream`'s connection carries a modelled data channel from now on,
    /// buffering `cap` bytes at most and sent as `profile` sends (see the
    /// module docs and the WAN scheduler), unpaced. Returns the
    /// connection, or `None` without send buffers and the WAN model.
    pub fn open_channel_as(
        &self,
        stream: &SimTcpStream,
        cap: u64,
        profile: SctpProfile,
    ) -> Option<ConnKey> {
        let ((key, b), wan) = (stream.conn.as_ref()?, self.wan.as_ref()?);
        let mut guard = wan.lock();
        let sched = guard.as_mut()?;
        b.open_channel(*key, cap, &profile);
        sched.open_channel(*key, profile);
        Some(*key)
    }

    pub(crate) fn buffers(&self) -> Option<SendBuffers> {
        self.buffers.clone()
    }

    pub(crate) fn wan(&self) -> Option<SharedWan> {
        self.wan.clone()
    }

    /// Lets the capture's link probes read `wan`.
    pub fn with_wan(self, wan: SharedWan) -> SimNet {
        SimNet {
            wan: Some(wan),
            ..self
        }
    }

    fn wrap(&self, stream: turmoil::net::TcpStream) -> SimTcpStream {
        let conn = match (&self.buffers, stream.local_addr(), stream.peer_addr()) {
            (Some(b), Ok(local), Ok(peer)) => {
                b.with(|x| x.conns.insert((local, peer), Conn::default()));
                Some(((local, peer), b.clone()))
            }
            _ => None,
        };
        SimTcpStream {
            inner: stream,
            conn,
            segment: self.segment,
            host_poll: None,
        }
    }
}

/// A link's connection as the model holds it, for the capture.
#[derive(Debug)]
struct SimProbe {
    conn: ConnKey,
    buffers: Option<SendBuffers>,
    wan: Option<SharedWan>,
}

impl LinkProbe for SimProbe {
    /// Reads the model at true time: the node's `now` is off it by the
    /// node's clock offset ([`Scenario::clock_skew`](crate::Scenario::clock_skew)).
    fn sample(&mut self, _: Time) -> Option<CaptureSample> {
        let now = sim_time(turmoil::sim_elapsed().unwrap_or_default());
        let (unsent, lowat) = self
            .buffers
            .as_ref()
            .and_then(|b| b.conn(self.conn))
            .unwrap_or((0, None));
        let s = self
            .wan
            .as_ref()
            .and_then(|w| w.lock().as_ref().map(|s| s.conn_state(self.conn, now)))
            .unwrap_or_default();
        Some(CaptureSample::SimConn {
            unsent,
            lowat,
            cwnd: s.cwnd,
            ssthresh: s.ssthresh,
            in_flight: s.in_flight,
            pending: s.pending,
            uplink_queue: s.uplink_queue,
            uplink_rate: s.uplink_rate,
        })
    }
}

/// `conn`'s buffered amount fell at `at`: the writer to wake once the
/// amount is under half its cap, or a paced writer's waiting on its own cap
/// once it is at its mark. One its host holds, and the other channels its
/// host holds, wait for their poll.
fn release(b: &mut Buffers, conn: ConnKey, at: Time) -> Option<Waker> {
    let c = b.conns.get_mut(&conn)?;
    let (cap, _) = c.channel?;
    let buffered = c.amount();
    if c.pacer.is_some() {
        // The browser's `bufferedamountlow`: the amount fell to the
        // threshold from above.
        if c.above
            && buffered <= c.threshold
            && let Some((link, sink)) = &c.log
        {
            sink.record_at(
                *link,
                at,
                LinkEvent::Low {
                    source: SampleSource::Model,
                    buffered,
                },
            );
        }
        c.above = buffered > c.threshold;
    }
    match &mut c.pacer {
        // A browser waiting on its own cap reads the amount as it falls to
        // its mark; one its host holds sleeps until its poll.
        Some(p) if c.waker.is_some() && c.waits && buffered <= c.low => {
            p.observe(at, buffered as usize);
            c.waker.take()
        }
        Some(_) => None,
        None if 2 * buffered < cap => c.waker.take(),
        None => None,
    }
}

/// The check of data channel `key`'s writer at `now`, as a browser's
/// `DcSocket::send` makes it at each turn of its loop: its own pacer first,
/// which a refusal waits on; then its host's, which holds the write once,
/// for the mark it sets then, and whose load the writer then reads against
/// that mark at each poll, sending once it is there. `waker` wakes the
/// writer left waiting. The link log gets each check's reading and each
/// hold, as the browser's.
fn check(x: &mut Buffers, key: &ConnKey, now: Time, waker: &Waker) -> ChannelWrite {
    let Buffers { conns, hosts, .. } = x;
    let Some(c) = conns.get_mut(key) else {
        return ChannelWrite::Go;
    };
    let Some((cap, _)) = c.channel else {
        return ChannelWrite::Go;
    };
    c.waker = None;
    let buffered = c.amount();
    let may = match &mut c.pacer {
        Some(p) => {
            p.observe(now, buffered as usize);
            c.low = p.low_mark(now, cap as usize) as u64;
            let may = p.may_write(now, buffered as usize, cap as usize);
            c.waits = !may;
            if !may {
                c.threshold = c.low;
                c.above = buffered > c.threshold;
            }
            may
        }
        None => buffered < cap,
    };
    let paced = c.pacer.is_some();
    c.record(LinkEvent::Checked {
        source: SampleSource::Model,
        buffered,
        low: (paced && !may).then_some(c.low),
    });
    if !may {
        c.waker = Some(waker.clone());
        c.record(LinkEvent::Held(WriteWait::Channel));
        return ChannelWrite::Wait;
    }
    let held = c.held;
    let Some(h) = hosts.get_mut(&key.0.ip()).filter(|_| paced) else {
        return ChannelWrite::Go;
    };
    let sum = h.sum(conns, now);
    let at = match held {
        None => {
            h.pacer.observe(now, &sum);
            h.pacer.may_write(now, &sum)
        }
        Some(mark) => {
            let at = sum.load <= mark;
            if at {
                h.pacer.observe(now, &sum);
            }
            at
        }
    };
    let mark = (held.is_none() && !at).then(|| h.pacer.low_mark(now, &sum));
    let Some(c) = conns.get_mut(key) else {
        return ChannelWrite::Go;
    };
    if at {
        c.held = None;
        return ChannelWrite::Go;
    }
    c.waker = Some(waker.clone());
    if let Some(mark) = mark {
        c.held = Some(mark);
        c.record(LinkEvent::Held(WriteWait::Host));
    }
    ChannelWrite::Held
}

/// The simulation's time, as the scheduler's and the hosts' clocks read
/// it (outside a host, the run's start).
fn sim_now() -> Time {
    sim_time(turmoil::sim_elapsed().unwrap_or_default())
}

/// A turmoil TCP stream whose writes wait while its unsent bytes reach
/// its `TCP_NOTSENT_LOWAT`.
#[derive(Debug)]
pub(crate) struct SimTcpStream {
    inner: turmoil::net::TcpStream,
    conn: Option<(ConnKey, SendBuffers)>,
    segment: Option<usize>,
    /// While the host's pacer holds a channel's writer: when it reads the
    /// host's load again ([`HOST_POLL`]).
    host_poll: Option<Pin<Box<tokio::time::Sleep>>>,
}

/// What a channel's writer does now ([`SendBuffers::channel_write`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelWrite {
    Go,
    /// For its own amount to fall to its mark.
    Wait,
    /// For its next host poll ([`HOST_POLL`]), to read the host's load
    /// against the mark the host held it for.
    Held,
}

impl Drop for SimTcpStream {
    fn drop(&mut self) {
        if let Some((key, b)) = &self.conn {
            b.with(|x| {
                if let Some(c) = x.conns.swap_remove(key)
                    && c.pacer.is_some()
                    && let Some(h) = x.hosts.get_mut(&key.0.ip())
                {
                    h.channels.retain(|k| k != key);
                }
            });
        }
    }
}

impl AsyncRead for SimTcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl SendBuffers {
    /// For data channel `key`'s writer at `now`, whether a write may go, or
    /// what the writer, left waiting, waits for (`waker` wakes it). A
    /// writer that waits checks again only once woken: at its own mark (by
    /// [`release`]), or at its host poll ([`SendBuffers::host_poll`]).
    fn channel_write(&self, key: &ConnKey, now: Time, waker: &Waker) -> Option<ChannelWrite> {
        self.with(|x| {
            let c = x.conns.get_mut(key)?;
            c.channel?;
            if c.waker.is_some() {
                c.waker = Some(waker.clone());
                return Some(if c.held.is_some() && !c.waits {
                    ChannelWrite::Held
                } else {
                    ChannelWrite::Wait
                });
            }
            Some(check(x, key, now, waker))
        })
    }

    /// The host poll of `key`'s writer, which its host held, came at `now`:
    /// it checks again.
    fn host_poll(&self, key: &ConnKey, now: Time, waker: &Waker) -> Option<ChannelWrite> {
        self.with(|x| {
            x.conns.get_mut(key)?.channel?;
            Some(check(x, key, now, waker))
        })
    }

    /// Data channel `key`'s writer handed it `n` bytes of a message at
    /// `now`, the message's last if `end`: they are unsent, stamped with
    /// the host's order, and its pacers count them.
    fn wrote(&self, key: &ConnKey, n: usize, end: bool, now: Time) {
        self.with(|x| {
            let stamp = x.stamp;
            x.stamp += 1;
            if let Some(c) = x.conns.get_mut(key) {
                if let Some((_, ch)) = &mut c.channel {
                    ch.cut(n);
                }
                c.unsent = c.unsent.saturating_add(n as u64);
                // A page that counts the amount adds the message to its
                // last reading.
                if let Some(s) = &mut c.shown {
                    *s = s.saturating_add(n as u64);
                }
                c.above = c.amount() > c.threshold;
                c.writes.push_back((stamp, end));
                if let Some(p) = &mut c.pacer {
                    p.wrote(now, n);
                    if let Some(h) = x.hosts.get_mut(&key.0.ip()) {
                        h.pacer.wrote(now, n);
                    }
                }
            }
        });
    }
}

impl SimTcpStream {
    /// For a data channel's connection, whether a write may go now, or
    /// what the writer, left waiting, waits for: at its host poll, if
    /// `polled`, a check.
    fn channel_write(&self, cx: &Context<'_>, polled: bool) -> Option<ChannelWrite> {
        let (key, b) = self.conn.as_ref()?;
        if polled {
            b.host_poll(key, sim_now(), cx.waker())
        } else {
            b.channel_write(key, sim_now(), cx.waker())
        }
    }

    /// Writes `buf` as a channel's message: as turmoil segments, the
    /// chunks its [`Chunking`] cuts, each stamped and unsent until the WAN
    /// scheduler sends it.
    fn write_message(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut done = 0;
        while done < buf.len() {
            let left = buf.len() - done;
            let len = self.conn.as_ref().map_or(left, |(key, b)| {
                b.with(|x| {
                    x.conns.get_mut(key).map_or(left, |c| {
                        let waiting = c.unsent > 0;
                        c.channel
                            .as_mut()
                            .map_or(left, |(_, ch)| ch.next(left, waiting))
                    })
                })
            });
            let piece = buf.get(done..done + len).unwrap_or_default();
            match Pin::new(&mut self.inner).poll_write(cx, piece) {
                Poll::Ready(Ok(n)) => {
                    if let Some((key, b)) = &self.conn {
                        b.wrote(key, n, done + n == buf.len(), sim_now());
                    }
                    done += n;
                    if n < piece.len() {
                        break;
                    }
                }
                Poll::Ready(Err(e)) if done == 0 => return Poll::Ready(Err(e)),
                Poll::Ready(Err(_)) | Poll::Pending if done > 0 => break,
                other => return other,
            }
        }
        Poll::Ready(Ok(done))
    }
}

impl AsyncWrite for SimTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = &mut *self;
        let polled = this
            .host_poll
            .as_mut()
            .is_some_and(|poll| poll.as_mut().poll(cx).is_ready());
        if polled {
            this.host_poll = None;
        }
        if let Some(write) = this.channel_write(cx, polled) {
            return match write {
                ChannelWrite::Go => {
                    this.host_poll = None;
                    this.write_message(cx, buf)
                }
                // Its own cap's wait: no poll.
                ChannelWrite::Wait => {
                    this.host_poll = None;
                    Poll::Pending
                }
                ChannelWrite::Held => {
                    let poll = this.host_poll.get_or_insert_with(|| {
                        Box::pin(tokio::time::sleep(std::time::Duration::from_nanos(
                            HOST_POLL.as_nanos() as u64,
                        )))
                    });
                    let _ = poll.as_mut().poll(cx);
                    Poll::Pending
                }
            };
        }
        // The most this write may take: what the tail buffer has room for,
        // or below the mark a new buffer.
        let mut room = usize::MAX;
        if let Some((key, b)) = &this.conn {
            let take = b.with(|x| {
                let Some(c) = x.conns.get_mut(key) else {
                    return Some(u64::MAX);
                };
                let Some(lowat) = c.lowat else {
                    return Some(u64::MAX);
                };
                if c.waker.is_some() {
                    c.waker = Some(cx.waker().clone());
                    None
                } else if c.tail > 0 && c.tail < GSO_SEGMENT {
                    c.record(LinkEvent::Checked {
                        source: SampleSource::Model,
                        buffered: c.notsent(),
                        low: None,
                    });
                    Some(GSO_SEGMENT - c.tail)
                } else if c.notsent() >= lowat {
                    // Writable again under half the mark.
                    c.record(LinkEvent::Checked {
                        source: SampleSource::Model,
                        buffered: c.notsent(),
                        low: Some(lowat / 2),
                    });
                    c.waker = Some(cx.waker().clone());
                    None
                } else {
                    c.record(LinkEvent::Checked {
                        source: SampleSource::Model,
                        buffered: c.notsent(),
                        low: None,
                    });
                    c.tail = 0;
                    Some(GSO_SEGMENT)
                }
            });
            let Some(take) = take else {
                return Poll::Pending;
            };
            room = usize::try_from(take).unwrap_or(usize::MAX);
        }
        let n = this.segment.map_or(room, |n| n.max(1).min(room));
        let buf = buf.get(..buf.len().min(n)).unwrap_or(buf);
        let r = Pin::new(&mut this.inner).poll_write(cx, buf);
        if let (Poll::Ready(Ok(n)), Some((key, b))) = (&r, &this.conn) {
            b.with(|x| {
                let stamp = x.stamp;
                x.stamp += 1;
                if let Some(c) = x.conns.get_mut(key) {
                    c.unsent = c.unsent.saturating_add(*n as u64);
                    // A write takes whole packets of the room, a short
                    // one a packet (Linux's window counts packets).
                    let mss = crate::wan::MSS as u64;
                    let packets = (*n as u64).div_ceil(mss);
                    let early = if packets <= c.room {
                        *n as u64
                    } else {
                        c.room * mss
                    };
                    c.room -= packets.min(c.room);
                    c.early += early;
                    if c.lowat.is_some() {
                        c.tail = c.tail.saturating_add(*n as u64).min(c.notsent());
                    }
                    // A write is a whole message, if a channel later
                    // takes the connection.
                    c.writes.push_back((stamp, true));
                }
            });
        }
        r
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl Net for SimNet {
    type TcpStream = SimTcpStream;
    type TcpListener = turmoil::net::TcpListener;
    type UdpSocket = turmoil::net::UdpSocket;

    fn tcp_connect(&self, addr: String) -> impl Future<Output = io::Result<SimTcpStream>> + Send {
        let net = self.clone();
        async move {
            let s = TurmoilNet.tcp_connect(addr).await?;
            Ok(net.wrap(s))
        }
    }

    fn tcp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::TcpListener>> + Send {
        TurmoilNet.tcp_bind(addr)
    }

    async fn tcp_accept<'a>(
        &'a self,
        listener: &'a Self::TcpListener,
    ) -> io::Result<(SimTcpStream, SocketAddr)> {
        let (s, addr) = TurmoilNet.tcp_accept(listener).await?;
        Ok((self.wrap(s), addr))
    }

    fn tcp_listener_addr(&self, listener: &Self::TcpListener) -> io::Result<SocketAddr> {
        TurmoilNet.tcp_listener_addr(listener)
    }

    fn configure_tcp(&self, stream: &SimTcpStream, options: &TcpOptions) -> io::Result<()> {
        TurmoilNet.configure_tcp(&stream.inner, options)
    }

    /// With send buffers, the mark of `config` holds the stream's writes
    /// (see the module docs); without, nothing, as [`TurmoilNet`].
    fn configure_tcp_send_queue(
        &self,
        stream: &SimTcpStream,
        config: &TcpSocketConfig,
    ) -> io::Result<Option<u32>> {
        let (Some((key, b)), Some(lowat)) = (&stream.conn, config.notsent_lowat) else {
            return Ok(None);
        };
        b.with(|x| {
            if let Some(c) = x.conns.get_mut(key) {
                c.lowat = Some(u64::from(lowat));
            }
        });
        Ok(Some(lowat))
    }

    /// The connection hands the log its readings and holds (see the
    /// module docs).
    fn log_link(&self, stream: &SimTcpStream, link: u64, sink: LinkSink) {
        if let Some((key, b)) = &stream.conn {
            b.with(|x| {
                if let Some(c) = x.conns.get_mut(key) {
                    c.log = Some((link, sink));
                }
            });
        }
    }

    /// The model's view of the stream's connection: its send buffer, and
    /// with the WAN model, its window and its host's uplink.
    fn link_probe(&self, stream: &SimTcpStream, _ws: bool) -> Option<Box<dyn LinkProbe>> {
        let wan = self.wan.clone().filter(|w| w.lock().is_some());
        if self.buffers.is_none() && wan.is_none() {
            return None;
        }
        let conn = (
            stream.inner.local_addr().ok()?,
            stream.inner.peer_addr().ok()?,
        );
        Some(Box::new(SimProbe {
            conn,
            buffers: self.buffers.clone(),
            wan,
        }))
    }

    fn models_channels(&self) -> bool {
        self.rtc_stack.is_some()
    }

    /// As the host's stack sends ([`SimNet::with_rtc_stack`]), unpaced
    /// while the stack's own flights set it up.
    fn open_channel(&self, stream: &SimTcpStream, cap: usize) -> bool {
        self.rtc_stack
            .is_some_and(|p| self.open_channel_as(stream, cap as u64, p).is_some())
    }

    /// The page's writer paces the channel from now on, its pacer and its
    /// host's seeing none of the setup's flights, as a browser's stack
    /// sends its DTLS records and SCTP's control chunks past the page's
    /// `DcSocket`; and its association's round trips start here, as
    /// dcSCTP's estimate takes no sample from its handshake.
    fn channel_up(&self, stream: &SimTcpStream) {
        let (Some((key, b)), Some(profile)) = (stream.conn.as_ref(), self.rtc_stack) else {
            return;
        };
        b.pace(*key, &profile);
        if let Some(wan) = &self.wan
            && let Some(sched) = wan.lock().as_mut()
        {
            sched.channel_up(*key);
        }
    }

    /// str0m's channels stand for a native node's in-process ones. Its
    /// links hold a stream's burst below the runner (two 64 KiB MTUs each,
    /// past QoS), and the node writes each link's share into its channel in
    /// one go, link after link. On the testbed a native seed behind 20
    /// Mbit/s reads a busy share of 0-1 permille, its 250-ms writes leave
    /// it as one block of 280-310 kB within 5 ms, and its uplink's full
    /// queue drops the tail: the last links' channels lose whole bursts and
    /// wait out sctp-proto's timeout, a second, and their viewers prune the
    /// seed. A browser's buffers (one MTU and one message, two messages a
    /// pass) hold the burst in the runner and cut it finer.
    fn channels_in_process(&self) -> bool {
        self.rtc_stack.is_some_and(|p| p.native())
    }

    /// The host's stack's ([`SimNet::with_rtc_stack`]): Chromium's,
    /// Firefox's, else str0m's.
    fn link_setup(&self) -> LinkSetup {
        use crate::wan::SctpProfile;
        match self.rtc_stack {
            Some(p) if p == SctpProfile::CHROMIUM => LinkSetup::CHROMIUM,
            Some(p) if p == SctpProfile::FIREFOX => LinkSetup::FIREFOX,
            _ => LinkSetup::STR0M,
        }
    }

    fn udp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::UdpSocket>> + Send {
        TurmoilNet.udp_bind(addr)
    }

    fn udp_send_to<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a [u8],
        to: SocketAddr,
    ) -> impl Future<Output = io::Result<usize>> + Send + 'a {
        TurmoilNet.udp_send_to(socket, buf, to)
    }

    fn udp_recv_from<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<(usize, SocketAddr)>> + Send + 'a {
        TurmoilNet.udp_recv_from(socket, buf)
    }

    fn udp_local_addr(&self, socket: &Self::UdpSocket) -> io::Result<SocketAddr> {
        TurmoilNet.udp_local_addr(socket)
    }

    fn resolve(
        &self,
        host: String,
        port: u16,
    ) -> impl Future<Output = io::Result<Vec<SocketAddr>>> + Send {
        TurmoilNet.resolve(host, port)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// The pacers' cap before a round trip is known (`pace::FLOOR`), and the
    /// mark a writer refused at it waits for (less `pace::MIN_WRITE`).
    const FLOOR: u64 = 32 * 1024;
    const LOW: u64 = FLOOR - 8 * 1024;

    fn ms(t: u64) -> Time {
        sim_time(std::time::Duration::from_millis(t))
    }

    fn us(t: u64) -> Time {
        sim_time(std::time::Duration::from_micros(t))
    }

    /// A writer's waker, counting its wakes.
    #[derive(Default)]
    struct Wakes(AtomicUsize);

    impl std::task::Wake for Wakes {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Wakes {
        fn count(&self) -> usize {
            self.0.load(Ordering::Relaxed)
        }
    }

    fn waker() -> (Arc<Wakes>, Waker) {
        let w = Arc::new(Wakes::default());
        (w.clone(), Waker::from(w))
    }

    type Log = Arc<Mutex<Vec<(u64, LinkEvent)>>>;

    /// A host's `n` paced channels of `profile`, each to a peer of its own
    /// and logging as link `i`, their logs' events in the order they came.
    fn channels(profile: &SctpProfile, n: u8) -> (SendBuffers, Vec<ConnKey>, Log) {
        let b = SendBuffers::new(SendBuffer {
            uplink_queue: std::time::Duration::from_millis(50),
            window: true,
        });
        let log: Log = Arc::default();
        let sink = {
            let log = log.clone();
            LinkSink(Arc::new(move |r: strims_node::event::LinkRecord| {
                log.lock().unwrap().push((r.link, r.event));
            }))
        };
        let keys = (0..n)
            .map(|i| {
                let local: SocketAddr =
                    format!("10.0.0.1:{}", 1000 + u16::from(i)).parse().unwrap();
                let peer: SocketAddr = format!("10.0.1.{}:2000", i + 1).parse().unwrap();
                let key = (local, peer);
                b.with(|x| {
                    x.conns.entry(key).or_default().log = Some((u64::from(i), sink.clone()));
                });
                b.open_channel(key, 1 << 20, profile);
                b.pace(key, profile);
                key
            })
            .collect();
        (b, keys, log)
    }

    /// The link's lows in the log, by the amounts they read.
    fn lows(log: &Log, link: u64) -> Vec<u64> {
        log.lock()
            .unwrap()
            .iter()
            .filter_map(|(l, e)| match e {
                LinkEvent::Low { buffered, .. } if *l == link => Some(*buffered),
                _ => None,
            })
            .collect()
    }

    /// The link's holds in the log.
    fn holds(log: &Log, link: u64) -> Vec<WriteWait> {
        log.lock()
            .unwrap()
            .iter()
            .filter_map(|(l, e)| match e {
                LinkEvent::Held(w) if *l == link => Some(*w),
                _ => None,
            })
            .collect()
    }

    /// A channel's setup goes past its writer's pacers, as a browser's
    /// stack sends its handshake past the page's: its flights go at once,
    /// whatever their bytes, and neither pacer counts them; once it is up
    /// its writer paces it, among its host's channels.
    #[test]
    fn a_channel_is_paced_from_its_setups_end() {
        for p in [SctpProfile::CHROMIUM, SctpProfile::FIREFOX] {
            let b = SendBuffers::new(SendBuffer {
                uplink_queue: std::time::Duration::from_millis(50),
                window: true,
            });
            let key: ConnKey = (
                "10.0.0.1:1000".parse().unwrap(),
                "10.0.1.1:2000".parse().unwrap(),
            );
            b.with(|x| {
                x.conns.entry(key).or_default();
            });
            b.open_channel(key, 1 << 20, &p);
            let (_, w) = waker();
            for _ in 0..4 {
                assert!(b.channel_write(&key, ms(0), &w) == Some(ChannelWrite::Go));
                b.wrote(&key, FLOOR as usize, true, ms(0));
            }
            b.with(|x| {
                assert!(x.conns[&key].pacer.is_none());
                assert!(
                    x.hosts
                        .get(&key.0.ip())
                        .is_none_or(|h| h.channels.is_empty())
                );
            });
            let _ = b.admit(key, 4 * FLOOR as usize, ms(0));
            let _ = b.acked(key, 4 * FLOOR as usize, ms(0));
            b.pace(key, &p);
            b.with(|x| {
                assert!(x.conns[&key].pacer.is_some());
                assert_eq!(x.conns[&key].shown, p.amount_at_sends.then_some(0));
                assert_eq!(x.hosts[&key.0.ip()].channels, [key]);
            });
            assert!(b.channel_write(&key, ms(1), &w) == Some(ChannelWrite::Go));
            b.wrote(&key, FLOOR as usize, true, ms(1));
            assert!(b.channel_write(&key, ms(1), &w) == Some(ChannelWrite::Wait));
        }
    }

    /// A writer refused at its own cap waits for the amount to fall to its
    /// mark, and wakes as it does.
    #[test]
    fn a_channels_writer_waits_for_its_own_mark() {
        let (b, k, log) = channels(&SctpProfile::FIREFOX, 1);
        let (wakes, w) = waker();
        assert!(b.channel_write(&k[0], ms(0), &w) == Some(ChannelWrite::Go));
        b.wrote(&k[0], FLOOR as usize, true, ms(0));
        assert!(b.channel_write(&k[0], ms(0), &w) == Some(ChannelWrite::Wait));
        assert_eq!(holds(&log, 0), [WriteWait::Channel]);
        assert!(b.admit(k[0], (FLOOR - LOW - 1) as usize, ms(1)).is_none());
        let woke = b.admit(k[0], 1, ms(2));
        woke.expect("at its mark").wake();
        assert_eq!(wakes.count(), 1);
        assert!(b.channel_write(&k[0], ms(2), &w) == Some(ChannelWrite::Go));
    }

    /// The channel's `bufferedamountlow` comes each time its amount falls
    /// to the threshold its writer last set from above, whether its writer
    /// waits or not, and never while it stays below: before a refusal, at
    /// half the channel's cap, as the browser sets it as it opens it.
    #[test]
    fn a_channels_low_comes_at_each_fall_to_its_threshold() {
        let (b, k, log) = channels(&SctpProfile::FIREFOX, 1);
        let (_, w) = waker();
        // The channels' cap is 1 MiB: half of it before a refusal. An
        // emptying below it fires nothing; a fall from above it does.
        let half: u64 = 1 << 19;
        b.channel_write(&k[0], ms(0), &w);
        b.wrote(&k[0], 1000, true, ms(0));
        b.admit(k[0], 1000, ms(1));
        assert_eq!(lows(&log, 0), Vec::<u64>::new());
        b.wrote(&k[0], half as usize + 1000, true, ms(1));
        b.admit(k[0], 999, ms(2));
        assert_eq!(lows(&log, 0), Vec::<u64>::new());
        b.admit(k[0], 1, ms(2));
        assert_eq!(lows(&log, 0), [half]);
        b.admit(k[0], half as usize, ms(3));
        assert_eq!(lows(&log, 0), [half]);
        // A refusal sets the threshold to its mark; the fall to it wakes
        // the writer.
        b.wrote(&k[0], FLOOR as usize, true, ms(3));
        assert!(b.channel_write(&k[0], ms(3), &w) == Some(ChannelWrite::Wait));
        assert!(b.admit(k[0], (FLOOR - LOW) as usize, ms(4)).is_some());
        assert_eq!(lows(&log, 0), [half, LOW]);
        // Below it, none; above it again and back, one more, though nothing
        // waits.
        b.admit(k[0], 1000, ms(5));
        assert!(b.channel_write(&k[0], ms(5), &w) == Some(ChannelWrite::Go));
        b.wrote(&k[0], 2000, true, ms(5));
        assert!(b.admit(k[0], 2000, ms(6)).is_none());
        assert_eq!(lows(&log, 0), [half, LOW, LOW - 1000]);
    }

    /// The host holds a channel's writer while the host's channels' load is
    /// at its cap (the floor, before a round trip), and frees it only at
    /// the writer's poll once the load is at its mark: neither another
    /// channel's fall nor its own wakes it, as nothing wakes a browser's
    /// writer asleep for its poll.
    #[test]
    fn a_host_holds_a_writer_until_its_poll() {
        let (b, k, log) = channels(&SctpProfile::FIREFOX, 2);
        let (wakes, w) = waker();
        assert!(b.channel_write(&k[1], ms(0), &w) == Some(ChannelWrite::Go));
        b.wrote(&k[1], 30_000, true, ms(0));
        assert!(b.channel_write(&k[0], ms(0), &w) == Some(ChannelWrite::Go));
        b.wrote(&k[0], 10_000, true, ms(0));
        assert!(b.channel_write(&k[0], ms(1), &w) == Some(ChannelWrite::Held));
        assert_eq!(holds(&log, 0), [WriteWait::Host]);
        assert_eq!(
            b.host_poll(&k[0], ms(6), &w),
            Some(ChannelWrite::Held),
            "over the host's mark"
        );
        // Channel 1's fall, then its own, bring the load to 15 kB, under
        // the mark.
        assert!(b.admit(k[1], 20_000, ms(7)).is_none());
        assert!(b.admit(k[0], 5_000, ms(8)).is_none());
        assert_eq!(wakes.count(), 0);
        assert_eq!(b.channel_write(&k[0], ms(9), &w), Some(ChannelWrite::Held));
        assert_eq!(b.host_poll(&k[0], ms(11), &w), Some(ChannelWrite::Go));
        assert_eq!(holds(&log, 0), [WriteWait::Host]);
    }

    /// Each writer the host holds polls on its own time and reads the
    /// host's load against the mark the host held it for: one at its mark
    /// goes without the host's admission, and its write holds another whose
    /// poll comes later, which waits on with no hold more.
    #[test]
    fn a_held_writer_reads_its_own_mark_at_its_own_poll() {
        let (b, k, log) = channels(&SctpProfile::FIREFOX, 3);
        let ((wakes0, w0), (wakes1, w1)) = (waker(), waker());
        assert_eq!(b.channel_write(&k[2], ms(0), &w0), Some(ChannelWrite::Go));
        b.wrote(&k[2], 40_000, true, ms(0));
        assert_eq!(b.channel_write(&k[0], ms(1), &w0), Some(ChannelWrite::Held));
        assert_eq!(b.channel_write(&k[1], ms(3), &w1), Some(ChannelWrite::Held));
        b.admit(k[2], 20_000, ms(4));
        // Channel 0's poll: the load, 20 kB, is at its mark.
        assert_eq!(b.host_poll(&k[0], ms(6), &w0), Some(ChannelWrite::Go));
        b.wrote(&k[0], 10_000, true, ms(6));
        assert_eq!((wakes0.count(), wakes1.count()), (0, 0));
        // Channel 1's, at 30 kB: over it.
        assert_eq!(b.host_poll(&k[1], ms(8), &w1), Some(ChannelWrite::Held));
        assert_eq!(holds(&log, 0), [WriteWait::Host]);
        assert_eq!(holds(&log, 1), [WriteWait::Host]);
    }

    /// A Chromium page counts its channel's amount itself: each message on
    /// its last reading, the stack's at each of the stack's answers, and
    /// nothing as the stack sends between.
    #[test]
    fn a_chromium_page_counts_its_channels_amount() {
        let (b, k, _) = channels(&SctpProfile::CHROMIUM, 1);
        let read = || b.buffered(k[0]).unwrap();
        b.wrote(&k[0], 1000, true, us(0));
        b.wrote(&k[0], 500, true, us(100));
        assert_eq!(read(), 1500);
        // The stack sends the first message: the page reads it only at the
        // stack's answer.
        b.admit(k[0], 1000, us(150));
        assert_eq!(read(), 1500);
        b.answered(k[0], us(160));
        assert_eq!(read(), 500);
        // Past the answer the reading holds as the stack sends ...
        b.wrote(&k[0], 2000, true, ms(2));
        b.admit(k[0], 1000, ms(5));
        assert_eq!(read(), 2500);
        // ... and the next message adds to it, not to the stack's 1500.
        b.wrote(&k[0], 300, true, ms(6));
        assert_eq!(read(), 2800);
        // Empty, it reads 0 at the stack's answer.
        b.admit(k[0], 1800, ms(9));
        assert_eq!(read(), 2800);
        b.answered(k[0], ms(9));
        assert_eq!(read(), 0);
    }

    /// A Chromium writer refused at its cap waits however much its stack
    /// sends, and wakes on the stack's first answer that brings its reading
    /// to its mark; Firefox's reads its stack's amount and goes at its mark.
    #[test]
    fn a_chromium_writer_wakes_on_its_stacks_answer() {
        let (b, k, log) = channels(&SctpProfile::CHROMIUM, 1);
        let (_, w) = waker();
        b.wrote(&k[0], FLOOR as usize, true, ms(0));
        assert!(b.channel_write(&k[0], ms(0), &w) == Some(ChannelWrite::Wait));
        assert!(b.admit(k[0], 4000, us(100)).is_none());
        // An answer over the mark wakes nothing.
        assert!(b.answered(k[0], us(200)).is_none());
        assert_eq!(b.buffered(k[0]), Some(FLOOR - 4000));
        assert!(b.admit(k[0], (FLOOR - 4000 - 1) as usize, ms(10)).is_none());
        assert_eq!(b.buffered(k[0]), Some(FLOOR - 4000));
        assert!(b.answered(k[0], ms(10)).is_some(), "at its mark");
        assert_eq!(lows(&log, 0), [1]);

        let (b, k, _) = channels(&SctpProfile::FIREFOX, 1);
        b.wrote(&k[0], FLOOR as usize, true, ms(0));
        assert!(b.channel_write(&k[0], ms(0), &w) == Some(ChannelWrite::Wait));
        assert!(b.admit(k[0], 4000, us(500)).is_none());
        assert!(
            b.admit(k[0], (FLOOR - LOW - 4000) as usize, ms(10))
                .is_some()
        );
    }
}
