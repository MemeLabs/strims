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
//! its unsent bytes reach the mark, as a kernel socket does. A byte is
//! unsent from its write until the WAN scheduler admits its segment to the
//! sender's uplink queue, which it does once that queue holds less than
//! [`SendBuffer::uplink_queue`] of transmission time (see the WAN
//! scheduler). The segment's timing on the WAN doesn't change; only
//! when the writer may write again does.
//!
//! The model leaves out pushback from one path: a slow receiver or a
//! congestion window over a long round trip holds back one connection
//! while the uplink idles. Admission depends on the sender's uplink only.

// `Net` sockets are `Send`, so the state they share with the scheduler
// sits behind a lock, which only this module names. Everything runs on
// turmoil's thread, so it is never contended.
#![allow(clippy::disallowed_types)]

use std::collections::BTreeMap;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use strims_node::{Net, TcpOptions, TcpSocketConfig, TurmoilNet};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::spec::SendBuffer;

/// A TCP connection by its local and peer addresses.
pub(crate) type ConnKey = (SocketAddr, SocketAddr);

#[derive(Debug, Default)]
struct Conn {
    /// The socket's `TCP_NOTSENT_LOWAT`, once the node set it.
    lowat: Option<u64>,
    /// Bytes written and not yet admitted to the uplink queue.
    unsent: u64,
    waker: Option<Waker>,
}

/// Every connection's unsent bytes, shared by the hosts' sockets and the
/// WAN scheduler.
#[derive(Clone, Debug)]
pub(crate) struct SendBuffers(Arc<Mutex<Buffers>>);

#[derive(Debug)]
struct Buffers {
    model: SendBuffer,
    /// Bound: one per open TCP connection.
    conns: BTreeMap<ConnKey, Conn>,
}

impl SendBuffers {
    pub fn new(model: SendBuffer) -> SendBuffers {
        SendBuffers(Arc::new(Mutex::new(Buffers {
            model,
            conns: BTreeMap::new(),
        })))
    }

    fn with<R>(&self, f: impl FnOnce(&mut Buffers) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn model(&self) -> SendBuffer {
        self.with(|b| b.model)
    }

    /// `len` bytes of `conn` reached the uplink queue. Returns the writer
    /// to wake, if it waits.
    pub fn admit(&self, conn: ConnKey, len: usize) -> Option<Waker> {
        self.with(|b| {
            let c = b.conns.get_mut(&conn)?;
            c.unsent = c.unsent.saturating_sub(len as u64);
            let lowat = c.lowat?;
            if c.unsent < lowat {
                c.waker.take()
            } else {
                None
            }
        })
    }
}

/// turmoil's sockets, with send buffers when `buffers` is set.
#[derive(Clone, Debug, Default)]
pub(crate) struct SimNet {
    buffers: Option<SendBuffers>,
}

impl SimNet {
    pub fn new(buffers: Option<SendBuffers>) -> SimNet {
        SimNet { buffers }
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
        }
    }
}

/// A turmoil TCP stream whose writes wait while its unsent bytes reach
/// its `TCP_NOTSENT_LOWAT`.
#[derive(Debug)]
pub(crate) struct SimTcpStream {
    inner: turmoil::net::TcpStream,
    conn: Option<(ConnKey, SendBuffers)>,
}

impl Drop for SimTcpStream {
    fn drop(&mut self) {
        if let Some((key, b)) = &self.conn {
            b.with(|x| x.conns.remove(key));
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

impl AsyncWrite for SimTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = &mut *self;
        if let Some((key, b)) = &this.conn {
            let full = b.with(|x| match x.conns.get_mut(key) {
                Some(c) if c.lowat.is_some_and(|l| c.unsent >= l) => {
                    c.waker = Some(cx.waker().clone());
                    true
                }
                _ => false,
            });
            if full {
                return Poll::Pending;
            }
        }
        let r = Pin::new(&mut this.inner).poll_write(cx, buf);
        if let (Poll::Ready(Ok(n)), Some((key, b))) = (&r, &this.conn) {
            b.with(|x| {
                if let Some(c) = x.conns.get_mut(key) {
                    c.unsent = c.unsent.saturating_add(*n as u64);
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
