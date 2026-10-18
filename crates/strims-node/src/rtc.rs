// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The native WebRTC interface: [`NativeRtc`] is the node's
//! [`RtcEndpoint`] on `strims_webrtc::WebRtcEndpoint` (str0m), over UDP mux
//! sockets.
//!
//! str0m is sans-IO, so the endpoint lives in the node task with every
//! peer connection, and their data channels are in-process:
//!
//! - **Datagrams.** The UDP reader tasks (`udp.rs`) hand every datagram to
//!   the node (`Input::Datagram`), which passes it to
//!   [`RtcEndpoint::handle_datagram`] with the socket's bound address as
//!   `dst`. [`RtcEndpoint::flush`] sends the endpoint's datagrams to the
//!   socket bound to their `source`, through the writer task's queue.
//! - **Connections.** `ConnId`s map to [`RtcConn`]s one to one, through
//!   slotmap's `KeyData::as_ffi`.
//!
//! [`bind_webrtc`] binds the sockets before the node starts; their tasks
//! start with the node's other tasks.

use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;

use slotmap::{Key, KeyData};
use strims_base::rng::DetRng;
use strims_base::time::Time;
use strims_node_core::rtc::{RtcCloseReason, RtcConn, RtcEndpoint, RtcEvent};
use strims_node_core::{DriverMetrics, NodeConfig, NodeError};
use strims_vnic::webrtc::{RemoteDescriptionAction, SessionDescription};
use strims_webrtc::{CloseReason, ConnId, EndpointConfig, Event, WebRtcEndpoint};
use tokio::sync::mpsc;

use crate::net::Net;

/// What the endpoint starts from when WebRTC is enabled.
#[derive(Debug)]
pub(crate) struct RtcSetup {
    /// With `host_ip` already defaulted to the node's public IP.
    pub(crate) config: EndpointConfig,
    pub(crate) sockets: Vec<UdpIo>,
}

/// The node task's half of one UDP socket.
#[derive(Debug)]
pub(crate) struct UdpIo {
    /// The socket's bound address: `dst` of what it reads, `source` of what
    /// it sends.
    pub(crate) addr: SocketAddr,
    /// To the writer task; at most `WebRtcConfig::udp_send_queue` waiting.
    pub(crate) send_tx: mpsc::Sender<(SocketAddr, Vec<u8>)>,
    /// Read buffers back to the reader task.
    pub(crate) read_free_tx: mpsc::Sender<Vec<u8>>,
}

/// One UDP socket's task ends, until the node starts them.
pub(crate) struct UdpTasks<N: Net> {
    pub(crate) socket: usize,
    pub(crate) sock: Arc<N::UdpSocket>,
    pub(crate) free_rx: mpsc::Receiver<Vec<u8>>,
    pub(crate) send_rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
}

/// Binds the WebRTC interface's UDP sockets, one per listen address.
pub(crate) async fn bind_webrtc<N: Net>(
    net: &N,
    config: &NodeConfig,
) -> Result<(RtcSetup, Vec<UdpTasks<N>>), NodeError> {
    let w = &config.webrtc;
    if w.listen.is_empty() {
        return Err(NodeError::Config(
            "webrtc is enabled without a listen address".to_owned(),
        ));
    }
    let mut endpoint = w.endpoint.clone();
    endpoint.host_ip = endpoint.host_ip.or(config.public_ip);
    let mut sockets = Vec::new();
    let mut tasks = Vec::new();
    for (socket, a) in w.listen.iter().enumerate() {
        let sock = net.udp_bind(a.clone()).await?;
        let addr = net.udp_local_addr(&sock)?;
        // str0m matches a datagram's destination against the host
        // candidates, which must be addresses peers can reach.
        if addr.ip().is_unspecified()
            && !endpoint
                .host_ip
                .is_some_and(|ip| ip.is_ipv4() == addr.is_ipv4())
        {
            return Err(NodeError::Config(format!(
                "webrtc listen address {addr} is unspecified: bind a concrete ip, or set \
                 webrtc.endpoint.host_ip or public_ip (same address family)"
            )));
        }
        let (send_tx, send_rx) = mpsc::channel(w.udp_send_queue.max(1));
        let n = w.udp_read_buffers.max(1);
        let (read_free_tx, free_rx) = mpsc::channel(n);
        for _ in 0..n {
            let _ = read_free_tx.try_send(vec![0; w.udp_read_buffer_size.max(1)]);
        }
        sockets.push(UdpIo {
            addr,
            send_tx,
            read_free_tx,
        });
        tasks.push(UdpTasks {
            socket,
            sock: Arc::new(sock),
            free_rx,
            send_rx,
        });
    }
    Ok((
        RtcSetup {
            config: endpoint,
            sockets,
        },
        tasks,
    ))
}

/// The str0m endpoint and its sockets, as the node's [`RtcEndpoint`].
#[derive(Debug)]
pub(crate) struct NativeRtc {
    ep: WebRtcEndpoint,
    sockets: Vec<UdpIo>,
}

impl NativeRtc {
    pub(crate) fn new(setup: RtcSetup, now: Time, rng: DetRng) -> NativeRtc {
        let mut ep = WebRtcEndpoint::new(now, setup.config, rng);
        for s in &setup.sockets {
            ep.add_local_addr(s.addr);
        }
        NativeRtc {
            ep,
            sockets: setup.sockets,
        }
    }
}

fn rtc_conn(id: ConnId) -> RtcConn {
    RtcConn(id.data().as_ffi())
}

fn conn_id(conn: RtcConn) -> ConnId {
    ConnId::from(KeyData::from_ffi(conn.0))
}

fn close_reason(r: CloseReason) -> RtcCloseReason {
    match r {
        CloseReason::Local => RtcCloseReason::Local,
        CloseReason::Remote => RtcCloseReason::Remote,
        CloseReason::ConnectTimeout => RtcCloseReason::ConnectTimeout,
        CloseReason::IceFailed => RtcCloseReason::IceFailed,
        CloseReason::Error(e) => RtcCloseReason::Error(e),
    }
}

impl RtcEndpoint for NativeRtc {
    fn create_connection(&mut self, now: Time) -> Result<RtcConn, String> {
        self.ep
            .create_connection(now)
            .map(rtc_conn)
            .map_err(|e| e.to_string())
    }

    fn poll_local_description(
        &mut self,
        now: Time,
        conn: RtcConn,
    ) -> Poll<Result<SessionDescription, String>> {
        self.ep
            .poll_local_description(now, conn_id(conn))
            .map_err(|e| e.to_string())
    }

    fn set_remote_description(
        &mut self,
        now: Time,
        conn: RtcConn,
        desc: &SessionDescription,
    ) -> Result<RemoteDescriptionAction, String> {
        self.ep
            .set_remote_description(now, conn_id(conn), desc)
            .map_err(|e| e.to_string())
    }

    fn close(&mut self, now: Time, conn: RtcConn) {
        self.ep.close(now, conn_id(conn));
    }

    fn close_all(&mut self, now: Time) {
        let conns: Vec<ConnId> = self.ep.connections().collect();
        for conn in conns {
            self.ep.close(now, conn);
        }
    }

    fn poll_event(&mut self) -> Option<RtcEvent> {
        Some(match self.ep.poll_event()? {
            Event::LocalDescription(c) => RtcEvent::LocalDescription(rtc_conn(c)),
            Event::Open { conn, mtu } => RtcEvent::Open {
                conn: rtc_conn(conn),
                mtu,
            },
            Event::Data { conn, data } => RtcEvent::Data {
                conn: rtc_conn(conn),
                data,
            },
            Event::Writable(c) => RtcEvent::Writable(rtc_conn(c)),
            Event::Closed { conn, reason } => RtcEvent::Closed {
                conn: rtc_conn(conn),
                reason: close_reason(reason),
            },
        })
    }

    fn poll_timeout(&self) -> Option<Time> {
        self.ep.poll_timeout()
    }

    fn handle_timeout(&mut self, now: Time) {
        self.ep.handle_timeout(now);
    }

    /// Hands the endpoint's datagrams to the socket tasks. A full queue
    /// drops the datagram (UDP; ICE, DTLS and SCTP retransmit).
    fn flush(&mut self, metrics: &mut DriverMetrics) -> bool {
        let mut any = false;
        while let Some(t) = self.ep.poll_udp_transmit() {
            any = true;
            let len = t.contents.len() as u64;
            let sent = self
                .sockets
                .iter()
                .find(|s| s.addr == t.source)
                .is_some_and(|s| s.send_tx.try_send((t.destination, t.contents)).is_ok());
            if sent {
                metrics.udp_datagrams_written += 1;
                metrics.udp_bytes_written += len;
            } else {
                metrics.udp_send_dropped += 1;
            }
        }
        any
    }

    fn write(&mut self, conn: RtcConn, data: &[u8]) -> Result<usize, String> {
        self.ep
            .link_write(conn_id(conn), data)
            .map_err(|e| e.to_string())
    }

    fn buffered_amount(&mut self, conn: RtcConn) -> usize {
        self.ep.buffered_amount(conn_id(conn))
    }

    fn handle_datagram(
        &mut self,
        now: Time,
        socket: usize,
        from: SocketAddr,
        data: &[u8],
        metrics: &mut DriverMetrics,
    ) -> bool {
        let Some(addr) = self.sockets.get(socket).map(|s| s.addr) else {
            return false;
        };
        metrics.udp_datagrams_read += 1;
        metrics.udp_bytes_read += data.len() as u64;
        if !self.ep.handle_udp_input(now, from, addr, data) {
            metrics.udp_unrouted += 1;
        }
        true
    }

    fn return_udp_buffer(&mut self, socket: usize, buf: Vec<u8>) {
        if let Some(s) = self.sockets.get(socket) {
            // At most `udp_read_buffers` exist, so the channel has room.
            let _ = s.read_free_tx.try_send(buf);
        }
    }

    fn set_stun_servers(&mut self, addrs: Vec<SocketAddr>) {
        self.ep.set_stun_servers(addrs);
    }
}

#[cfg(test)]
mod tests {
    use slotmap::SlotMap;

    use super::*;

    /// Connection ids survive the trip through `RtcConn`, versions
    /// included, so a reused slot is a different connection.
    #[test]
    fn connection_ids_map_one_to_one() {
        let mut ids: SlotMap<ConnId, ()> = SlotMap::with_key();
        let a = ids.insert(());
        ids.remove(a);
        let b = ids.insert(());
        assert_ne!(a, b);
        assert_ne!(rtc_conn(a), rtc_conn(b));
        for id in [a, b, ConnId::null()] {
            assert_eq!(conn_id(rtc_conn(id)), id);
        }
    }
}
