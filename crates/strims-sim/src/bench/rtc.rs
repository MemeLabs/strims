// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The bench's links over WebRTC data channels: one endpoint per node
//! ([`WebRtcEndpoint`], str0m's stack as the node runs it) on a UDP socket
//! of [`Net`], its connections signalled over the bench's TCP links, and
//! each open channel read and written as a byte stream ([`DcStream`]).
//!
//! A channel takes writes while its buffered amount is under the node's
//! `max_buffered_amount` and, once full, again from half of it, as the
//! node's link writer and the browser's `DcSocket` do.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration as StdDuration;

use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_node::{Net, WebRtcConfig};
use strims_webrtc::{
    ConnId, EndpointConfig, Event, RemoteDescriptionAction, SessionDescription, WebRtcEndpoint,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::sync::Notify;

use super::BenchClock;

/// Where a node's endpoint listens and the address its host candidates
/// advertise.
#[derive(Clone, Debug)]
pub struct RtcPlan {
    pub udp: String,
    pub ip: IpAddr,
}

/// The endpoint's clock: the bench's microseconds from an epoch of its
/// own.
fn now<C: BenchClock>(clock: &C) -> Time {
    Time::from_unix_nanos(EPOCH_NS + clock.now_us() * 1000)
}

const EPOCH_NS: i64 = 1_700_000_000_000_000_000;

fn micros(t: Time) -> i64 {
    (t.unix_nanos() - EPOCH_NS) / 1000
}

/// One node's endpoint and what its channels hold.
pub(super) struct Rtc {
    ep: WebRtcEndpoint,
    /// Bytes read per connection and not yet taken by its stream.
    inbound: BTreeMap<ConnId, VecDeque<u8>>,
    open: BTreeSet<ConnId>,
    closed: BTreeSet<ConnId>,
    readers: BTreeMap<ConnId, Waker>,
    /// Writers waiting for their channel to drain to half the cap.
    writers: BTreeMap<ConnId, Waker>,
    cap: usize,
    /// Wakes the driver to flush what a write queued.
    kick: Rc<Notify>,
    /// Wakes signalling once the endpoint's state moved.
    moved: Rc<Notify>,
}

pub(super) type Shared = Rc<RefCell<Rtc>>;

/// Binds `plan`'s socket and starts node `me`'s endpoint; returns it and
/// the task that drives it.
pub(super) async fn start<N: Net, C: BenchClock>(
    net: &N,
    plan: &RtcPlan,
    me: usize,
    clock: &C,
) -> io::Result<(Shared, Pin<Box<dyn std::future::Future<Output = ()>>>)> {
    let defaults = WebRtcConfig::default();
    let sock = net.udp_bind(plan.udp.clone()).await?;
    let local = net.udp_local_addr(&sock)?;
    let config = EndpointConfig {
        ice_servers: Some(Vec::new()),
        host_ip: Some(plan.ip),
        ..defaults.endpoint
    };
    let mut ep = WebRtcEndpoint::new(now(clock), config, det_rng(me as u64 + 1));
    ep.add_local_addr(local);
    let shared = Rc::new(RefCell::new(Rtc {
        ep,
        inbound: BTreeMap::new(),
        open: BTreeSet::new(),
        closed: BTreeSet::new(),
        readers: BTreeMap::new(),
        writers: BTreeMap::new(),
        cap: defaults.max_buffered_amount,
        kick: Rc::new(Notify::new()),
        moved: Rc::new(Notify::new()),
    }));
    let driver = drive(net.clone(), sock, local, shared.clone(), clock.clone());
    Ok((shared, Box::pin(driver)))
}

/// Runs the endpoint on its socket: datagrams in and out, its timers, and
/// its events into the channels' streams.
async fn drive<N: Net, C: BenchClock>(
    net: N,
    sock: N::UdpSocket,
    local: SocketAddr,
    shared: Shared,
    clock: C,
) {
    let mut buf = vec![0u8; 1 << 16];
    let (kick, moved) = {
        let r = shared.borrow();
        (r.kick.clone(), r.moved.clone())
    };
    loop {
        loop {
            let t = shared.borrow_mut().ep.poll_udp_transmit();
            let Some(t) = t else {
                break;
            };
            let _ = net.udp_send_to(&sock, &t.contents, t.destination).await;
        }
        if settle(&shared) {
            moved.notify_waiters();
        }
        let deadline = shared.borrow().ep.poll_timeout();
        let wait = deadline.map_or(StdDuration::from_secs(1), |t| {
            StdDuration::from_micros((micros(t) - clock.now_us()).max(0).unsigned_abs())
        });
        tokio::select! {
            r = net.udp_recv_from(&sock, &mut buf) => {
                if let Ok((n, from)) = r {
                    let t = now(&clock);
                    shared
                        .borrow_mut()
                        .ep
                        .handle_udp_input(t, from, local, buf.get(..n).unwrap_or_default());
                }
            }
            () = tokio::time::sleep(wait) => {
                let t = now(&clock);
                shared.borrow_mut().ep.handle_timeout(t);
            }
            () = kick.notified() => {}
        }
    }
}

/// Takes the endpoint's events into the channels and wakes the streams
/// they free; returns whether a connection's state moved.
fn settle(shared: &Shared) -> bool {
    let mut r = shared.borrow_mut();
    let mut moved = false;
    while let Some(ev) = r.ep.poll_event() {
        match ev {
            Event::Data { conn, data } => {
                r.inbound.entry(conn).or_default().extend(data);
                if let Some(w) = r.readers.remove(&conn) {
                    w.wake();
                }
            }
            Event::Open { conn, .. } => {
                r.open.insert(conn);
                moved = true;
            }
            Event::Closed { conn, .. } => {
                r.closed.insert(conn);
                for w in [r.readers.remove(&conn), r.writers.remove(&conn)]
                    .into_iter()
                    .flatten()
                {
                    w.wake();
                }
                moved = true;
            }
            Event::LocalDescription(_) => moved = true,
            Event::Writable(_) => {}
        }
    }
    let cap = r.cap;
    let freed: Vec<ConnId> = r.writers.keys().copied().collect();
    for conn in freed {
        if r.ep.buffered_amount(conn) < cap / 2
            && let Some(w) = r.writers.remove(&conn)
        {
            w.wake();
        }
    }
    moved
}

/// Waits until `ready` holds of the endpoint.
async fn until(shared: &Shared, mut ready: impl FnMut(&mut Rtc) -> bool) {
    let moved = shared.borrow().moved.clone();
    loop {
        let notified = moved.notified();
        if ready(&mut shared.borrow_mut()) {
            return;
        }
        // Gathering completes on the endpoint's timers too.
        tokio::select! {
            () = notified => {}
            () = tokio::time::sleep(StdDuration::from_millis(5)) => {}
        }
    }
}

async fn local_description<C: BenchClock>(
    shared: &Shared,
    conn: ConnId,
    clock: &C,
) -> io::Result<SessionDescription> {
    let mut got = None;
    until(shared, |r| {
        match r.ep.poll_local_description(now(clock), conn) {
            Poll::Ready(d) => {
                got = Some(d);
                true
            }
            Poll::Pending => false,
        }
    })
    .await;
    got.ok_or_else(|| io::Error::other("no local description"))?
        .map_err(|e| io::Error::other(e.to_string()))
}

async fn send_description<S: AsyncWrite + Unpin>(
    s: &mut S,
    d: &SessionDescription,
) -> io::Result<()> {
    let text = d.to_json();
    s.write_all(&u32::try_from(text.len()).unwrap_or(u32::MAX).to_be_bytes())
        .await?;
    s.write_all(text.as_bytes()).await
}

async fn read_description<S: AsyncRead + Unpin>(s: &mut S) -> io::Result<SessionDescription> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len).await?;
    let mut text = vec![0u8; u32::from_be_bytes(len) as usize];
    s.read_exact(&mut text).await?;
    let text = String::from_utf8(text).map_err(io::Error::other)?;
    SessionDescription::from_json(&text).map_err(|e| io::Error::other(e.to_string()))
}

/// Opens a data channel over the signalling stream `s`: the dialer offers,
/// the other answers. Returns the channel's stream once it is open.
pub(super) async fn connect<S: AsyncRead + AsyncWrite + Unpin, C: BenchClock>(
    shared: &Shared,
    s: &mut S,
    dialer: bool,
    clock: &C,
) -> io::Result<DcStream> {
    let conn = shared
        .borrow_mut()
        .ep
        .create_connection(now(clock))
        .map_err(|e| io::Error::other(e.to_string()))?;
    if dialer {
        let offer = local_description(shared, conn, clock).await?;
        send_description(s, &offer).await?;
        let answer = read_description(s).await?;
        let action = shared
            .borrow_mut()
            .ep
            .set_remote_description(now(clock), conn, &answer)
            .map_err(|e| io::Error::other(e.to_string()))?;
        if action != RemoteDescriptionAction::AwaitLink {
            return Err(io::Error::other("the answer was taken as an offer"));
        }
    } else {
        let offer = read_description(s).await?;
        shared
            .borrow_mut()
            .ep
            .set_remote_description(now(clock), conn, &offer)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let answer = local_description(shared, conn, clock).await?;
        send_description(s, &answer).await?;
    }
    let kick = shared.borrow().kick.clone();
    kick.notify_one();
    until(shared, |r| {
        r.open.contains(&conn) || r.closed.contains(&conn)
    })
    .await;
    if shared.borrow().closed.contains(&conn) {
        return Err(io::Error::other("the data channel closed before it opened"));
    }
    Ok(DcStream {
        shared: shared.clone(),
        conn,
    })
}

/// An open data channel as an ordered byte stream (see the module docs).
pub(super) struct DcStream {
    shared: Shared,
    conn: ConnId,
}

impl DcStream {
    /// What the channel holds unacknowledged or unsent.
    pub(super) fn buffered(&self) -> usize {
        self.shared.borrow_mut().ep.buffered_amount(self.conn)
    }

    pub(super) fn handle(&self) -> DcStream {
        DcStream {
            shared: self.shared.clone(),
            conn: self.conn,
        }
    }
}

impl AsyncRead for DcStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut r = self.shared.borrow_mut();
        let conn = self.conn;
        let rtc = &mut *r;
        match rtc.inbound.get_mut(&conn).filter(|q| !q.is_empty()) {
            Some(q) => {
                let n = q.len().min(buf.remaining());
                let (a, b) = q.as_slices();
                let from_a = a.len().min(n);
                buf.put_slice(a.get(..from_a).unwrap_or_default());
                buf.put_slice(b.get(..n - from_a).unwrap_or_default());
                q.drain(..n);
                Poll::Ready(Ok(()))
            }
            None if rtc.closed.contains(&conn) => Poll::Ready(Ok(())),
            None => {
                rtc.readers.insert(conn, cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl AsyncWrite for DcStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut r = self.shared.borrow_mut();
        let conn = self.conn;
        if r.closed.contains(&conn) {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        let cap = r.cap;
        if r.ep.buffered_amount(conn) >= cap || r.writers.contains_key(&conn) {
            r.writers.insert(conn, cx.waker().clone());
            return Poll::Pending;
        }
        let mtu = r.ep.link_mtu(conn).unwrap_or(buf.len()).max(1);
        let n = buf.len().min(mtu);
        let res = r.ep.link_write(conn, buf.get(..n).unwrap_or_default());
        r.kick.notify_one();
        Poll::Ready(res.map_err(|e| io::Error::other(e.to_string())))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
