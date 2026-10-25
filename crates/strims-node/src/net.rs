// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The socket provider: real sockets ([`TokioNet`]) or turmoil's simulated
//! network (`TurmoilNet`, behind the `turmoil` feature).
//!
//! The driver is generic over [`Net`], so the same node code runs on real
//! sockets and inside a deterministic simulation. TCP carries the TCP and
//! WebSocket transports; UDP carries the WebRTC transport (str0m through
//! `strims-webrtc`), one [`Net::UdpSocket`] per configured mux address.
//! Futures are `Send`, so a node also runs on a multi-thread runtime.

use std::future::Future;
use std::io;
use std::net::SocketAddr;

use strims_node_core::LinkProbe;
use strims_vnic::TcpOptions;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::config::TcpSocketConfig;

/// TCP and UDP sockets. Addresses are `host:port` strings, resolved by the
/// implementation (DNS for tokio, turmoil's host names in a simulation).
pub trait Net: Clone + Send + Sync + 'static {
    type TcpStream: AsyncRead + AsyncWrite + Unpin + Send + 'static;
    type TcpListener: Send + Sync + 'static;
    type UdpSocket: Send + Sync + 'static;

    fn tcp_connect(&self, addr: String)
    -> impl Future<Output = io::Result<Self::TcpStream>> + Send;

    fn tcp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::TcpListener>> + Send;

    fn tcp_accept<'a>(
        &'a self,
        listener: &'a Self::TcpListener,
    ) -> impl Future<Output = io::Result<(Self::TcpStream, SocketAddr)>> + Send + 'a;

    fn tcp_listener_addr(&self, listener: &Self::TcpListener) -> io::Result<SocketAddr>;

    /// Applies the socket options of [`TcpOptions`]: keepalive, buffer
    /// sizes, and `TCP_NODELAY`, so vnic's small writes (vpn messages,
    /// ppspp control frames) don't wait on Nagle's algorithm. The deadlines
    /// are applied by the link tasks.
    fn configure_tcp(&self, stream: &Self::TcpStream, options: &TcpOptions) -> io::Result<()>;

    /// Applies [`TcpSocketConfig`]: the bound on the kernel's unsent bytes
    /// (`TCP_NOTSENT_LOWAT`) and the congestion controller. Returns the
    /// bound in effect, which the driver also sizes the link's writes to;
    /// `None` when the socket has none. The default sets nothing and
    /// returns `Ok(None)`.
    fn configure_tcp_send_queue(
        &self,
        stream: &Self::TcpStream,
        config: &TcpSocketConfig,
    ) -> io::Result<Option<u32>> {
        let _ = (stream, config);
        Ok(None)
    }

    /// A probe of the socket under a link (a WebSocket's when `ws`), for
    /// the ppspp capture. The default reads nothing.
    fn link_probe(&self, stream: &Self::TcpStream, ws: bool) -> Option<Box<dyn LinkProbe>> {
        let _ = (stream, ws);
        None
    }

    fn udp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::UdpSocket>> + Send;

    fn udp_send_to<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a [u8],
        to: SocketAddr,
    ) -> impl Future<Output = io::Result<usize>> + Send + 'a;

    fn udp_recv_from<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<(usize, SocketAddr)>> + Send + 'a;

    fn udp_local_addr(&self, socket: &Self::UdpSocket) -> io::Result<SocketAddr>;

    /// Resolves a host name (or an IP literal) for a UDP destination: the
    /// WebRTC interface's STUN servers. The default asks the system
    /// resolver (`tokio::net::lookup_host`); `TurmoilNet` resolves
    /// simulated host names.
    fn resolve(
        &self,
        host: String,
        port: u16,
    ) -> impl Future<Output = io::Result<Vec<SocketAddr>>> + Send {
        async move {
            Ok(tokio::net::lookup_host((host.as_str(), port))
                .await?
                .collect())
        }
    }
}

/// Real sockets.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioNet;

impl Net for TokioNet {
    type TcpStream = tokio::net::TcpStream;
    type TcpListener = tokio::net::TcpListener;
    type UdpSocket = tokio::net::UdpSocket;

    fn tcp_connect(
        &self,
        addr: String,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send {
        tokio::net::TcpStream::connect(addr)
    }

    fn tcp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::TcpListener>> + Send {
        tokio::net::TcpListener::bind(addr)
    }

    fn tcp_accept<'a>(
        &'a self,
        listener: &'a Self::TcpListener,
    ) -> impl Future<Output = io::Result<(Self::TcpStream, SocketAddr)>> + Send + 'a {
        listener.accept()
    }

    fn tcp_listener_addr(&self, listener: &Self::TcpListener) -> io::Result<SocketAddr> {
        listener.local_addr()
    }

    fn configure_tcp(&self, stream: &Self::TcpStream, options: &TcpOptions) -> io::Result<()> {
        stream.set_nodelay(true)?;
        let sock = socket2::SockRef::from(stream);
        if let Some(period) = options.keep_alive_period.to_std()
            && !period.is_zero()
        {
            sock.set_tcp_keepalive(&socket2::TcpKeepalive::new().with_time(period))?;
        }
        if options.read_buffer_size > 0 {
            sock.set_recv_buffer_size(options.read_buffer_size)?;
        }
        if options.write_buffer_size > 0 {
            sock.set_send_buffer_size(options.write_buffer_size)?;
        }
        Ok(())
    }

    fn configure_tcp_send_queue(
        &self,
        stream: &Self::TcpStream,
        config: &TcpSocketConfig,
    ) -> io::Result<Option<u32>> {
        let sock = socket2::SockRef::from(stream);
        set_congestion(&sock, config.congestion.as_deref());
        set_notsent_lowat(&sock, config.notsent_lowat)
    }

    /// `TCP_INFO`, queried through `NETLINK_SOCK_DIAG` by the socket's
    /// addresses (Linux only).
    fn link_probe(&self, stream: &Self::TcpStream, ws: bool) -> Option<Box<dyn LinkProbe>> {
        #[cfg(target_os = "linux")]
        {
            let (local, peer) = (stream.local_addr().ok()?, stream.peer_addr().ok()?);
            match crate::sock_diag::TcpProbe::new(local, peer, ws) {
                Ok(p) => Some(Box::new(p)),
                Err(e) => {
                    tracing::debug!(error = %e, "no TCP_INFO for the capture");
                    None
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (stream, ws);
            None
        }
    }

    fn udp_bind(&self, addr: String) -> impl Future<Output = io::Result<Self::UdpSocket>> + Send {
        tokio::net::UdpSocket::bind(addr)
    }

    fn udp_send_to<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a [u8],
        to: SocketAddr,
    ) -> impl Future<Output = io::Result<usize>> + Send + 'a {
        socket.send_to(buf, to)
    }

    fn udp_recv_from<'a>(
        &'a self,
        socket: &'a Self::UdpSocket,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<(usize, SocketAddr)>> + Send + 'a {
        socket.recv_from(buf)
    }

    fn udp_local_addr(&self, socket: &Self::UdpSocket) -> io::Result<SocketAddr> {
        socket.local_addr()
    }
}

#[cfg(target_os = "linux")]
fn set_congestion(sock: &socket2::SockRef<'_>, name: Option<&str>) {
    if let Some(name) = name
        && let Err(e) = sock.set_tcp_congestion(name.as_bytes())
    {
        tracing::debug!(error = %e, congestion = name, "tcp congestion controller unavailable");
    }
}

#[cfg(not(target_os = "linux"))]
fn set_congestion(_: &socket2::SockRef<'_>, _: Option<&str>) {}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set_notsent_lowat(sock: &socket2::SockRef<'_>, lowat: Option<u32>) -> io::Result<Option<u32>> {
    let Some(lowat) = lowat.filter(|&l| l > 0) else {
        return Ok(None);
    };
    sock.set_tcp_notsent_lowat(lowat)?;
    Ok(Some(lowat))
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn set_notsent_lowat(_: &socket2::SockRef<'_>, _: Option<u32>) -> io::Result<Option<u32>> {
    Ok(None)
}

#[cfg(feature = "turmoil")]
pub use sim::TurmoilNet;

#[cfg(feature = "turmoil")]
mod sim {
    use super::*;

    /// turmoil's simulated sockets. Must be used from a turmoil host or
    /// client. turmoil has no socket options, so [`Net::configure_tcp`] and
    /// [`Net::configure_tcp_send_queue`] are no-ops; the link deadlines
    /// still apply (on simulated time).
    #[derive(Clone, Copy, Debug, Default)]
    pub struct TurmoilNet;

    impl Net for TurmoilNet {
        type TcpStream = turmoil::net::TcpStream;
        type TcpListener = turmoil::net::TcpListener;
        type UdpSocket = turmoil::net::UdpSocket;

        fn tcp_connect(
            &self,
            addr: String,
        ) -> impl Future<Output = io::Result<Self::TcpStream>> + Send {
            turmoil::net::TcpStream::connect(addr)
        }

        fn tcp_bind(
            &self,
            addr: String,
        ) -> impl Future<Output = io::Result<Self::TcpListener>> + Send {
            turmoil::net::TcpListener::bind(addr)
        }

        fn tcp_accept<'a>(
            &'a self,
            listener: &'a Self::TcpListener,
        ) -> impl Future<Output = io::Result<(Self::TcpStream, SocketAddr)>> + Send + 'a {
            listener.accept()
        }

        fn tcp_listener_addr(&self, listener: &Self::TcpListener) -> io::Result<SocketAddr> {
            listener.local_addr()
        }

        fn configure_tcp(&self, stream: &Self::TcpStream, _: &TcpOptions) -> io::Result<()> {
            stream.set_nodelay(true)
        }

        fn udp_bind(
            &self,
            addr: String,
        ) -> impl Future<Output = io::Result<Self::UdpSocket>> + Send {
            turmoil::net::UdpSocket::bind(addr)
        }

        fn udp_send_to<'a>(
            &'a self,
            socket: &'a Self::UdpSocket,
            buf: &'a [u8],
            to: SocketAddr,
        ) -> impl Future<Output = io::Result<usize>> + Send + 'a {
            socket.send_to(buf, to)
        }

        fn udp_recv_from<'a>(
            &'a self,
            socket: &'a Self::UdpSocket,
            buf: &'a mut [u8],
        ) -> impl Future<Output = io::Result<(usize, SocketAddr)>> + Send + 'a {
            socket.recv_from(buf)
        }

        fn udp_local_addr(&self, socket: &Self::UdpSocket) -> io::Result<SocketAddr> {
            socket.local_addr()
        }

        /// turmoil's DNS: a simulated host's address (a name turmoil
        /// doesn't know yet gets a fresh address, as `turmoil::lookup`
        /// does).
        fn resolve(
            &self,
            host: String,
            port: u16,
        ) -> impl Future<Output = io::Result<Vec<SocketAddr>>> + Send {
            let ip = turmoil::lookup(host.as_str());
            std::future::ready(Ok(vec![SocketAddr::new(ip, port)]))
        }
    }
}
