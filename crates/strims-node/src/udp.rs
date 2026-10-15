// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The WebRTC interface's UDP sockets: one reader and one writer task per
//! socket, plus the STUN resolver.
//!
//! The tasks own nothing but the socket. Datagrams go to the node task
//! (`Input::Datagram`) in recycled buffers; the node task, which owns the
//! str0m endpoint, hands datagrams to send back over a bounded channel:
//!
//! ```text
//!   reader task --Datagram{buf}--> node --read_free_tx--> reader task
//!   node --send_tx (to, bytes)--> writer task --> socket
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use strims_webrtc::stun::StunServer;
use tokio::sync::mpsc;

use crate::link::{Input, with_timeout};
use crate::net::Net;

/// A pause after a failed receive that isn't a stale ICMP report, so a
/// broken socket doesn't spin.
const RECV_BACKOFF: StdDuration = StdDuration::from_millis(50);

/// How long one STUN server name may take to resolve.
const RESOLVE_TIMEOUT: strims_base::time::Duration = strims_base::time::Duration::from_secs(5);

/// Reads datagrams into recycled buffers until aborted.
pub(crate) async fn reader<N: Net>(
    net: N,
    socket: usize,
    sock: Arc<N::UdpSocket>,
    mut free: mpsc::Receiver<Vec<u8>>,
    input: mpsc::Sender<Input<N>>,
) {
    let mut spare: Option<Vec<u8>> = None;
    loop {
        let mut buf = match spare.take() {
            Some(b) => b,
            None => match free.recv().await {
                Some(b) => b,
                None => return,
            },
        };
        match net.udp_recv_from(&sock, &mut buf).await {
            Ok((len, from)) => {
                let msg = Input::Datagram {
                    socket,
                    buf,
                    len,
                    from,
                };
                if input.send(msg).await.is_err() {
                    return;
                }
            }
            Err(e) => {
                // An ICMP error for an earlier send (Windows reports them on
                // the next receive) says nothing about this socket; retry
                // at once. Anything else waits a little.
                let stale = matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionRefused
                );
                tracing::debug!(error = %e, socket, "udp receive");
                spare = Some(buf);
                if !stale {
                    tokio::time::sleep(RECV_BACKOFF).await;
                }
            }
        }
    }
}

/// Sends the node task's datagrams. Ends when the node drops the sender,
/// after the last queued datagram (a DTLS close_notify at shutdown).
pub(crate) async fn writer<N: Net>(
    net: N,
    sock: Arc<N::UdpSocket>,
    mut datagrams: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
) {
    while let Some((to, data)) = datagrams.recv().await {
        if let Err(e) = net.udp_send_to(&sock, &data, to).await {
            // UDP is lossy anyway: ICE, DTLS and SCTP retransmit.
            tracing::trace!(error = %e, %to, "udp send");
        }
    }
}

/// Resolves the STUN servers now and every `refresh`, and hands the
/// addresses to the node task.
pub(crate) async fn stun_resolver<N: Net>(
    net: N,
    servers: Vec<StunServer>,
    refresh: StdDuration,
    input: mpsc::Sender<Input<N>>,
) {
    loop {
        let mut addrs: Vec<SocketAddr> = Vec::new();
        for s in &servers {
            match with_timeout(RESOLVE_TIMEOUT, net.resolve(s.host.clone(), s.port)).await {
                Some(Ok(found)) => {
                    for a in found {
                        if !addrs.contains(&a) {
                            addrs.push(a);
                        }
                    }
                }
                Some(Err(e)) => tracing::debug!(error = %e, host = %s.host, "stun server"),
                None => tracing::debug!(host = %s.host, "stun server: resolve timeout"),
            }
        }
        if input.send(Input::StunServers(addrs)).await.is_err() {
            return;
        }
        if refresh.is_zero() {
            return;
        }
        tokio::time::sleep(refresh).await;
    }
}
