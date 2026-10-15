// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sans-IO virtual NIC: links, peers, frames and ports.
//!
//! A [`Host`] owns the host key, the links and the peers on them, the port
//! handlers and the QoS scheduler. The driver (`strims-node`, `strims-wasm`)
//! owns the sockets, the clock and the WebRTC stack: it feeds link bytes to
//! the host and drains the link buffers. Owners (vpn, the ppspp runner, rpc)
//! claim ports on peers and fill egress frames. `Host<T>`'s `T` is the
//! owners' routing tag: every handler and writer carries one, and frames and
//! send opportunities come back with it.
//!
//! # Driver contract
//!
//! 1. On connect, call [`Host::add_link`] with the transport's
//!    [`LinkConfig`]. Feed every read to [`Host::handle_link_input`], which
//!    stops early when the handshake completes (see its docs), and drain
//!    [`Host::poll_event`] after each call so owners can set handlers on
//!    [`Event::PeerAdded`] before any frame is dispatched.
//! 2. While [`Host::poll_link_transmit`] is non-empty, write it (or a
//!    prefix) and report what was written with
//!    [`Host::advance_link_transmit`]. [`Host::links_with_transmit`] lists
//!    the links with bytes waiting. Message-oriented transports (WebSocket,
//!    data channel) send at most [`Host::link_mtu`] bytes per message. A link
//!    is a byte stream: message boundaries need not match frames, and readers
//!    concatenate messages.
//! 3. Call [`Host::poll_egress`] after input, after owners change backlogs,
//!    after [`Host::advance_link_transmit`], and at [`Host::poll_timeout`];
//!    then drain events and send what the links have. The handshake deadline
//!    and keepalives need nothing more from the driver.
//! 4. On [`Event::LinkClosed`], close the transport. When the transport
//!    fails or closes on its own, call [`Host::remove_link`].
//! 5. Export [`HostMetrics`] and [`LinkMetrics`], and count dials and dial
//!    errors per scheme.
//!
//! Transport setup and options are in [`tcp`], [`ws`] and [`webrtc`]; peer
//! exchange drives link attempts through [`interface`].
//!
//! # Bounds
//!
//! Per link:
//!
//! - Before the handshake completes, the only inbound buffer is the reader
//!   for the current handshake message: at most `2 + 65535` bytes, held only
//!   while a message is split across reads. At most
//!   [`Host::max_pending_links`] links are in the handshake at once, each for
//!   at most [`Host::handshake_timeout`]; with the defaults that is about
//!   8.5 MB in all.
//! - One entry in the host's timer set.
//! - One frame reassembly buffer, which grows to the largest frame split
//!   across reads (at most 65535 bytes).
//! - Handshake bytes waiting to be sent (freed once sent), and one
//!   [`LinkConfig::send_buffer`] allocated when the peer is established.
//!   Frames are built in place, so egress allocates nothing.
//!
//! The peer table, handlers, reservations, writers and timers grow with
//! links and owner calls, never with peer input. The event queue holds the
//! link state changes since the last poll.

pub mod aes_link;
pub mod frame;
pub mod host;
pub mod interface;
pub mod json;
pub mod metrics;
mod out_buf;
pub mod stream;
pub mod tcp;
pub mod webrtc;
pub mod ws;

pub use aes_link::{AesLink, AesLinkError, AesLinkHandshake};
pub use frame::{
    FRAME_HEADER_LEN, Frame, FrameDecoder, FrameHeader, FrameWriter, MAX_FRAME_BODY_LEN,
};
pub use host::{
    AesLinkConfig, CloseReason, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_KEEPALIVE_INTERVAL,
    DEFAULT_MAX_PENDING_LINKS, EgressFrame, Event, Fill, Host, HostConfig, InboundFrame,
    LinkConfig, LinkKind, Peer, PeerInitError, PeerKey, WriterId, ports,
};
pub use interface::{
    CandidateError, CandidateErrors, DialTarget, LinkCandidate, LinkCandidatePool, LinkDescription,
    LinkInterface,
};
pub use metrics::{HostMetrics, LinkMetrics};
pub use tcp::{TcpMux, TcpMuxAcceptor, TcpOptions, TcpUri, format_tcp_uri};

/// The version vnic writes in `PeerInit`, `AESLinkInit` and `TCPMuxInit`.
pub const PROTOCOL_VERSION: u32 = 1;

/// Errors from [`Host`] calls.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The link was closed or never existed.
    #[error("unknown link")]
    UnknownLink,
    /// The link hasn't finished its handshake, so it has no peer yet.
    #[error("peer handshake not complete")]
    PeerNotEstablished,
    #[error("unknown frame writer")]
    UnknownWriter,
    /// The port already has a handler, or is [`ports::KEEPALIVE_PORT`],
    /// which vnic reserves for itself.
    #[error("port {0} already in use")]
    PortInUse(u16),
    /// Every port at or above the reserved range is taken.
    #[error("no free ports")]
    NoFreePorts,
    /// A link MTU must fit a header and a byte of body, and at most a
    /// `u16` length of body.
    #[error("invalid link mtu {0}")]
    InvalidMtu(usize),
    /// A link's send buffer must hold at least one MTU.
    #[error("send buffer of {0} bytes is smaller than the mtu")]
    SendBufferTooSmall(usize),
    /// `advance_link_transmit` past the pending bytes.
    #[error("advanced past pending transmit bytes")]
    AdvancePastEnd,
    /// The handshake failed and the link was closed.
    #[error("peer init: {0}")]
    PeerInit(#[from] PeerInitError),
    /// The link didn't finish its handshake within
    /// [`Host::handshake_timeout`].
    #[error("handshake timed out")]
    HandshakeTimeout,
    /// The link was still in the handshake when [`Host::max_pending_links`]
    /// newer links were, and was closed to make room.
    #[error("too many links in the handshake")]
    TooManyPendingLinks,
    #[error("aes link: {0}")]
    AesLink(#[from] AesLinkError),
    #[error("{0}")]
    Stream(#[from] stream::StreamError),
    #[error("{0}")]
    Crypto(#[from] strims_crypto::Error),
    #[error("qos: {0}")]
    Qos(#[from] strims_qos::Error),
}
