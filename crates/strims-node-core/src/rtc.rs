// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The WebRTC interface: one [`RtcEndpoint`] per node holds every peer
//! connection, behind a sans-IO interface the node task drives like vnic.
//!
//! The node never sees what implements a connection. It creates one per
//! peer exchange, trades its session descriptions over the network, and
//! closes it; the endpoint reports what happened as [`RtcEvent`]s. Both
//! signalling and the timers are the endpoint's, and nothing in it blocks.
//!
//! A connection's data channel becomes a vnic link in one of two ways:
//!
//! - **In-process.** The endpoint owns the channel: [`RtcEvent::Open`]
//!   adds the link, [`RtcEvent::Data`] carries its bytes in,
//!   [`RtcEndpoint::write`] takes them out until it accepts fewer than
//!   offered, and [`RtcEvent::Writable`] resumes writing. A native endpoint
//!   (str0m) works this way, with its UDP sockets fed through
//!   [`RtcEndpoint::handle_datagram`].
//! - **As a stream.** The driver owns the channel: it attaches it with
//!   `Input::Connected { rtc: Some(conn), .. }` once the channel is open,
//!   and it runs with I/O tasks and recycled buffers, like a TCP link. The
//!   endpoint then only signals (a browser's `RTCPeerConnection` behind a
//!   proxy, fed [`RtcInput`]s through [`RtcEndpoint::handle_signal`]).
//!
//! Either way the link is tied to its connection: the connection closing
//! removes the link, and the link closing closes the connection.
//!
//! **Determinism.** An endpoint draws every random value from the `DetRng`
//! its factory gets ([`RtcFactory`]), which the node forks after vnic, vpn
//! and the runner. Connection ids need only be unique among the live
//! connections of one endpoint.

use std::fmt::Debug;
use std::net::SocketAddr;
use std::task::Poll;

use strims_base::rng::DetRng;
use strims_base::time::Time;
use strims_vnic::webrtc::{RemoteDescriptionAction, SessionDescription};

use crate::event::DriverMetrics;

/// A peer connection of the node's [`RtcEndpoint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RtcConn(pub u64);

/// Why a connection ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RtcCloseReason {
    /// [`RtcEndpoint::close`] or [`RtcEndpoint::close_all`].
    Local,
    /// The peer closed the connection or the data channel.
    Remote,
    /// The data channel didn't open within the endpoint's connect timeout.
    ConnectTimeout,
    /// ICE stayed disconnected for the endpoint's failure timeout.
    IceFailed,
    /// The connection failed (handshake error, bad description, ...).
    Error(String),
}

/// What the endpoint tells the node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RtcEvent {
    /// The description asked for with
    /// [`RtcEndpoint::poll_local_description`] is ready: poll again.
    LocalDescription(RtcConn),
    /// An in-process data channel opened; `mtu` is the most one
    /// [`RtcEndpoint::write`] puts in a message.
    Open { conn: RtcConn, mtu: usize },
    /// The next bytes of an in-process channel, in order.
    Data { conn: RtcConn, data: Vec<u8> },
    /// An in-process channel has room again after a short
    /// [`RtcEndpoint::write`].
    Writable(RtcConn),
    /// The connection is gone and its id is no longer valid.
    Closed {
        conn: RtcConn,
        reason: RtcCloseReason,
    },
}

/// A signal from the driver about one connection, for an endpoint whose
/// connections live outside the node task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RtcInput {
    pub conn: RtcConn,
    pub signal: RtcSignal,
}

/// What happened to a connection outside the node task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RtcSignal {
    /// The local description asked for is ready, gathering complete.
    LocalDescription(SessionDescription),
    /// ICE and DTLS are up.
    Connected,
    /// The connection failed or closed, and why.
    Closed(String),
}

/// Every WebRTC connection of one node, as a sans-IO state machine.
///
/// After every call into it the node drains [`RtcEndpoint::flush`] and
/// [`RtcEndpoint::poll_event`] until both are idle, so neither queue needs
/// another bound. Errors are messages: the node reports them as candidate
/// failures and logs.
pub trait RtcEndpoint: Send + Debug {
    /// A new connection, gathering at once.
    fn create_connection(&mut self, now: Time) -> Result<RtcConn, String>;

    /// The local description: an offer if no remote description was set
    /// yet, else the answer. The first call decides which. Pending until
    /// gathering completes, which [`RtcEvent::LocalDescription`] reports.
    fn poll_local_description(
        &mut self,
        now: Time,
        conn: RtcConn,
    ) -> Poll<Result<SessionDescription, String>>;

    /// Applies the remote description: an answer to our offer awaits the
    /// link, an offer is answered and linked in the background.
    fn set_remote_description(
        &mut self,
        now: Time,
        conn: RtcConn,
        desc: &SessionDescription,
    ) -> Result<RemoteDescriptionAction, String>;

    /// Closes a connection; [`RtcEvent::Closed`] follows.
    fn close(&mut self, now: Time, conn: RtcConn);

    /// Closes every connection (shutdown).
    fn close_all(&mut self, now: Time);

    fn poll_event(&mut self) -> Option<RtcEvent>;

    fn poll_timeout(&self) -> Option<Time>;

    fn handle_timeout(&mut self, now: Time);

    /// Hands queued output (datagrams, commands) to the driver. Returns
    /// whether there was any.
    fn flush(&mut self, metrics: &mut DriverMetrics) -> bool;

    /// Writes an in-process channel's link bytes, returning the bytes
    /// accepted. A short count means its buffer is full, and
    /// [`RtcEvent::Writable`] follows. An error means the channel isn't
    /// open, and [`RtcEvent::Closed`] follows.
    fn write(&mut self, conn: RtcConn, data: &[u8]) -> Result<usize, String> {
        let _ = (conn, data);
        Err("no in-process data channels".to_owned())
    }

    /// Bytes written into an in-process channel and not yet acknowledged.
    fn buffered_amount(&mut self, conn: RtcConn) -> usize {
        let _ = conn;
        0
    }

    /// A datagram the driver read from its UDP socket `socket`. Returns
    /// whether the endpoint has that socket (and so may have output).
    fn handle_datagram(
        &mut self,
        now: Time,
        socket: usize,
        from: SocketAddr,
        data: &[u8],
        metrics: &mut DriverMetrics,
    ) -> bool {
        let _ = (now, socket, from, data, metrics);
        false
    }

    /// A read buffer of UDP socket `socket`, back from the node.
    fn return_udp_buffer(&mut self, socket: usize, buf: Vec<u8>) {
        let _ = (socket, buf);
    }

    /// The resolved STUN servers. Connections created afterwards gather
    /// server-reflexive candidates from them.
    fn set_stun_servers(&mut self, addrs: Vec<SocketAddr>) {
        let _ = addrs;
    }

    /// A signal from the driver ([`RtcInput`]).
    fn handle_signal(&mut self, now: Time, input: RtcInput) {
        let _ = (now, input);
    }
}

/// Builds a node's endpoint from the time and the endpoint's forked rng.
pub type RtcFactory = Box<dyn FnOnce(Time, DetRng) -> Box<dyn RtcEndpoint> + Send>;
