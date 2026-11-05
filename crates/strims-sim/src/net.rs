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
//! polled meanwhile. Each write is stamped with its order on the host, so
//! the WAN scheduler hands a sender's segments to its uplink in the order
//! its sockets took them. A byte is
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
//! [`SctpProfile::buffered_flight`](crate::wan::SctpProfile::buffered_flight))
//! is under the channel's cap, and once that is reached, again only from
//! under half of it. A node's channel is
//! paced as the browser's `DcSocket` paces it: under its own pacer's cap,
//! and the host's channels together under a [`HostPacer`]'s, each waiting
//! for its mark once refused. (The browser's writer polls the host's load
//! while the host holds it; here it reads it at each acknowledgement.) A
//! write goes to turmoil as segments of a DATA chunk's payload at most,
//! which the window model's association sends as packets.

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
use strims_node::{
    CaptureSample, ChannelPacer, HostPacer, HostSum, LinkProbe, Net, TcpOptions, TcpSocketConfig,
    TurmoilNet,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::fast_map::FastMap;
use crate::spec::SendBuffer;
use crate::wan::{Chunking, SharedWan, sim_time};

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
}

impl Conn {
    /// A data channel's buffered amount.
    fn buffered(&self) -> u64 {
        self.unsent + if self.flight { self.unacked } else { 0 }
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
    /// The load the channels the host holds wait for.
    low: usize,
    /// Bound: the host's open paced channels, as `waiting`.
    channels: Vec<ConnKey>,
    waiting: Vec<ConnKey>,
}

impl HostChannels {
    /// The channels now.
    fn sum(&self, conns: &FastMap<ConnKey, Conn>, now: Time) -> HostSum {
        let mut sum = HostSum::default();
        for c in self.channels.iter().filter_map(|k| conns.get(k)) {
            if let Some(p) = &c.pacer {
                sum.add(p, now, c.buffered() as usize, c.waits);
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
        self.with(|b| b.conns.get(&conn).map(|c| (c.unsent, c.lowat)))
    }

    /// The order of `conn`'s next write the scheduler hasn't taken (writes
    /// on every connection count in the order the hosts made them), and
    /// whether it ends a data channel's message.
    pub fn next_write(&self, conn: ConnKey) -> Option<(u64, bool)> {
        self.with(|b| b.conns.get_mut(&conn)?.writes.pop_front())
    }

    /// `len` bytes of `conn` reached the uplink queue at `at`. Returns the
    /// writer to wake, if it waits: once its unsent bytes are under half the
    /// mark, when Linux reports a socket with the mark writable. A data
    /// channel's bytes stay buffered until they are acknowledged
    /// ([`SendBuffers::acked`]) where its stack's amount counts the flight,
    /// and leave its amount now where it doesn't.
    pub fn admit(&self, conn: ConnKey, len: usize, at: Time) -> Option<Waker> {
        self.with(|b| {
            let c = b.conns.get_mut(&conn)?;
            c.unsent = c.unsent.saturating_sub(len as u64);
            c.tail = c.tail.min(c.unsent);
            if c.channel.is_some() {
                c.unacked = c.unacked.saturating_add(len as u64);
                return if c.flight { None } else { release(b, conn, at) };
            }
            let lowat = c.lowat?;
            if 2 * c.unsent < lowat {
                c.waker.take()
            } else {
                None
            }
        })
    }
}

impl SendBuffers {
    /// `conn` carries a data channel of `cap` bytes buffered at most, cut
    /// into chunks of `chunk` bytes at most, its writer paced if `paced`,
    /// its buffered amount counting the flight if `flight`.
    fn open_channel(&self, conn: ConnKey, cap: u64, chunking: Chunking, paced: bool, flight: bool) {
        self.with(|b| {
            if let Some(c) = b.conns.get_mut(&conn) {
                c.channel = Some((cap, chunking));
                c.flight = flight;
                c.pacer = paced.then(ChannelPacer::default);
                if paced {
                    b.hosts.entry(conn.0.ip()).or_default().channels.push(conn);
                }
            }
        });
    }

    /// A SACK acknowledged `len` bytes of data channel `conn` at `at`.
    /// Returns the writer to wake once its buffered amount is under half
    /// its cap, or a paced writer's once it is at its mark; wakes the
    /// channels its host holds once their load is at the host's mark.
    pub fn acked(&self, conn: ConnKey, len: usize, at: Time) -> Option<Waker> {
        self.with(|b| {
            let c = b.conns.get_mut(&conn)?;
            c.unacked = c.unacked.saturating_sub(len as u64);
            if c.flight { release(b, conn, at) } else { None }
        })
    }

    /// Data channel `conn`'s buffered amount.
    pub fn buffered(&self, conn: ConnKey) -> Option<u64> {
        self.with(|b| {
            let c = b.conns.get(&conn)?;
            c.channel.map(|_| c.buffered())
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
    /// module docs and the WAN scheduler). Returns the connection, or
    /// `None` without send buffers and the WAN model.
    pub fn open_channel_as(
        &self,
        stream: &SimTcpStream,
        cap: u64,
        profile: crate::wan::SctpProfile,
        paced: bool,
    ) -> Option<ConnKey> {
        let ((key, b), wan) = (stream.conn.as_ref()?, self.wan.as_ref()?);
        let mut guard = wan.lock();
        let sched = guard.as_mut()?;
        b.open_channel(
            *key,
            cap,
            Chunking::new(&profile),
            paced,
            profile.buffered_flight,
        );
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
    fn sample(&mut self, now: Time) -> Option<CaptureSample> {
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
/// amount is under half its cap, or a paced writer's once it is at its
/// mark; wakes the channels its host holds once their load is at the
/// host's mark.
fn release(b: &mut Buffers, conn: ConnKey, at: Time) -> Option<Waker> {
    let Buffers { conns, hosts, .. } = b;
    let c = conns.get_mut(&conn)?;
    if c.pacer.is_some()
        && let Some(h) = hosts.get_mut(&conn.0.ip())
        && !h.waiting.is_empty()
    {
        let sum = h.sum(conns, at);
        if sum.load <= h.low {
            h.pacer.observe(at, &sum);
            for k in std::mem::take(&mut h.waiting) {
                if let Some(w) = conns.get_mut(&k).and_then(|o| o.waker.take()) {
                    w.wake();
                }
            }
        }
    }
    let c = conns.get_mut(&conn)?;
    let (cap, _) = c.channel?;
    let buffered = c.buffered();
    match &mut c.pacer {
        // A browser reads the amount as it falls to the mark it waits for.
        Some(p) if c.waker.is_some() && buffered <= c.low => {
            p.observe(at, buffered as usize);
            c.waker.take()
        }
        Some(_) => None,
        None if 2 * buffered < cap => c.waker.take(),
        None => None,
    }
}

/// A turmoil TCP stream whose writes wait while its unsent bytes reach
/// its `TCP_NOTSENT_LOWAT`.
#[derive(Debug)]
pub(crate) struct SimTcpStream {
    inner: turmoil::net::TcpStream,
    conn: Option<(ConnKey, SendBuffers)>,
    segment: Option<usize>,
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
                    h.waiting.retain(|k| k != key);
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

impl SimTcpStream {
    /// For a data channel's connection, whether a write may go now, or
    /// `false` with the writer left waiting.
    fn channel_write(&self, cx: &Context<'_>) -> Option<bool> {
        let (key, b) = self.conn.as_ref()?;
        b.with(|x| {
            let Buffers { conns, hosts, .. } = x;
            let c = conns.get_mut(key)?;
            let (cap, _) = c.channel?;
            let buffered = c.buffered();
            let now = sim_time(turmoil::sim_elapsed().unwrap_or_default());
            let may = match &mut c.pacer {
                Some(p) => {
                    p.observe(now, buffered as usize);
                    c.low = p.low_mark(now, cap as usize) as u64;
                    let may = p.may_write(now, buffered as usize, cap as usize);
                    c.waits = !may;
                    may
                }
                None => buffered < cap,
            };
            let waited = c.waker.is_some();
            let paced = c.pacer.is_some();
            let mut held = false;
            if paced
                && may
                && !waited
                && let Some(h) = hosts.get_mut(&key.0.ip())
            {
                let sum = h.sum(conns, now);
                h.pacer.observe(now, &sum);
                if !h.pacer.may_write(now, &sum) {
                    held = true;
                    h.low = h.pacer.low_mark(now, &sum);
                    if !h.waiting.contains(key) {
                        h.waiting.push(*key);
                    }
                }
            }
            let c = conns.get_mut(key)?;
            if waited || !may || held {
                c.waker = Some(cx.waker().clone());
                return Some(false);
            }
            Some(true)
        })
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
                        b.with(|x| {
                            let stamp = x.stamp;
                            x.stamp += 1;
                            if let Some(c) = x.conns.get_mut(key) {
                                if let Some((_, ch)) = &mut c.channel {
                                    ch.cut(n);
                                }
                                c.unsent = c.unsent.saturating_add(n as u64);
                                c.writes.push_back((stamp, done + n == buf.len()));
                                if let Some(p) = &mut c.pacer {
                                    let now = sim_time(turmoil::sim_elapsed().unwrap_or_default());
                                    p.wrote(now, n);
                                    if let Some(h) = x.hosts.get_mut(&key.0.ip()) {
                                        h.pacer.wrote(now, n);
                                    }
                                }
                            }
                        });
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
        if let Some(may) = this.channel_write(cx) {
            return if may {
                this.write_message(cx, buf)
            } else {
                Poll::Pending
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
                    Some(GSO_SEGMENT - c.tail)
                } else if c.unsent >= lowat {
                    c.waker = Some(cx.waker().clone());
                    None
                } else {
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
                    if c.lowat.is_some() {
                        c.tail = c.tail.saturating_add(*n as u64);
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

    /// As the host's stack sends ([`SimNet::with_rtc_stack`]).
    fn open_channel(&self, stream: &SimTcpStream, cap: usize) -> bool {
        self.rtc_stack
            .is_some_and(|p| self.open_channel_as(stream, cap as u64, p, true).is_some())
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
