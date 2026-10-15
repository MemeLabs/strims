// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sans-IO WebRTC data-channel links for vnic, built on str0m.
//!
//! A [`WebRtcEndpoint`] holds every peer connection of a node. Each
//! connection carries one vnic link over the pre-negotiated data channel
//! ([`DATA_CHANNEL_INIT`]: id 1, label `"data"`). Signalling is non-trickle
//! offer/answer in [`SessionDescription`] JSON, produced once ICE gathering
//! completes.
//!
//! The crate is native-only. On `wasm32` it compiles to the re-exports from
//! [`strims_vnic::webrtc`] alone: browsers use `RTCPeerConnection` through
//! the JS bridge, with the same signalling.
//!
//! # Driver contract
//!
//! Build one endpoint per node with [`WebRtcEndpoint::new`], passing it a
//! forked `DetRng`.
//!
//! **Sockets.** Either bind one shared UDP socket per local IP and register
//! each with [`WebRtcEndpoint::add_local_addr`] (every
//! [`WebRtcEndpoint::create_connection`] gets a host candidate on each), or
//! bind sockets per connection and use
//! [`WebRtcEndpoint::create_connection_on`]. Either way:
//!
//! - `dst` in [`WebRtcEndpoint::handle_udp_input`] must be the address the
//!   socket is bound to. A socket bound to the wildcard address needs
//!   [`EndpointConfig::host_ip`]; without it, bind one socket per interface
//!   address.
//! - Send each [`Transmit`] from the socket bound to its `source`.
//!
//! **STUN.** Resolve [`EndpointConfig::stun_servers`] at startup and every
//! few minutes, and pass the addresses to
//! [`WebRtcEndpoint::set_stun_servers`]. Connections created afterwards
//! gather server-reflexive candidates from them for at most
//! [`EndpointConfig::stun_gather_timeout`]. `turn:` and `stuns:` URLs are
//! skipped.
//!
//! **The loop.** Feed every datagram to `handle_udp_input`, and call
//! [`WebRtcEndpoint::handle_timeout`] once [`WebRtcEndpoint::poll_timeout`]
//! passes. After every call into the endpoint, signalling, `link_write` and
//! `close` included, drain [`WebRtcEndpoint::poll_udp_transmit`] and
//! [`WebRtcEndpoint::poll_event`]: nothing else bounds those queues.
//!
//! **Events into vnic.**
//!
//! - [`Event::LocalDescription`]: re-poll the peer exchange that owns the
//!   connection.
//! - [`Event::Open`]: add the link with
//!   `host.add_link(now, LinkConfig::webrtc().with_mtu(mtu))` and map the
//!   connection to it. `PeerInit` is the first thing on the link.
//! - [`Event::Data`]: call `Host::handle_link_input` until it has consumed
//!   every byte, draining host events between calls.
//! - [`Event::Writable`]: resume the link's writes.
//! - [`Event::Closed`]: remove the link, or fail the pending candidate with
//!   `CandidateError::Failed`.
//!
//! **vnic into the channel.** After `Host::poll_egress`, for each WebRTC
//! link in `Host::links_with_transmit`, repeat `poll_link_transmit`,
//! [`WebRtcEndpoint::link_write`] and `advance_link_transmit(n)` until vnic
//! has nothing left or `link_write` accepts 0 bytes; [`Event::Writable`]
//! follows. On vnic's `LinkClosed`, call [`WebRtcEndpoint::close`].
//!
//! **`LinkCandidate`** (interface [`WEBRTC_INTERFACE`]), for vpn peer
//! exchange:
//!
//! - Creating one calls `create_connection`; an error fails the candidate.
//! - `poll_local_description` is [`WebRtcEndpoint::poll_local_description`]
//!   through `SessionDescription::to_link_description`.
//! - `set_remote_description` converts with
//!   `SessionDescription::from_link_description`, calls
//!   [`WebRtcEndpoint::set_remote_description`] and keeps the
//!   [`RemoteDescriptionAction`].
//! - `poll_remote_description`: after `AwaitInBackground` (answering),
//!   ready with `false` at once, and the link is added on `Open` later.
//!   After `AwaitLink` (offering), pending until `Open` and then vnic's
//!   `PeerAdded` for that link, then ready with `true`. `Closed` fails it,
//!   and a `PeerInit` failure on the link fails it with
//!   `CandidateError::PeerInit`.
//! - Re-poll the exchange after `LocalDescription`, `Open`, `PeerAdded` and
//!   `Closed` for its connection.
//! - When an exchange ends before its connection opened, `close` the
//!   connection. The endpoint closes it anyway after
//!   [`EndpointConfig::connect_timeout`], so a connection left to the
//!   background never leaks.
//!
//! # Bounds
//!
//! - At most [`EndpointConfig::max_connections`] connections.
//! - Per connection: an SCTP send buffer of at most
//!   [`EndpointConfig::send_buffer`], sctp-proto's 1 MiB receive window, a
//!   certificate of about 400 bytes, at most 8 learned routes, and one STUN
//!   transaction per (host socket, STUN server) until gathering ends.
//! - One call queues a bounded amount: the responses to one datagram, or at
//!   most `send_buffer` of data for one `link_write`.
//! - str0m allocates each received message ([`Event::Data`]) and each
//!   datagram ([`Transmit`]); the endpoint hands both over without copying.
//!
//! # Determinism
//!
//! The endpoint's `DetRng`, forked per connection, seeds the DTLS
//! certificate and key (and so the SDP fingerprint), the ICE credentials,
//! the STUN gathering transaction ids and dimpl's rng (DTLS randoms,
//! retransmission jitter, AES-GCM explicit nonces).
//!
//! str0m draws the SDP session id, `a=mid`, the ICE tie-breaker and the
//! connectivity-check transaction ids from `fastrand`'s thread-local
//! generator, which [`seed_str0m_thread_rng`] seeds.
//!
//! Not seedable:
//!
//! - dimpl's ECDHE keys come from `OsRng`, so DTLS key material and every
//!   ciphertext differ per run. The ECDSA signatures over the ephemeral keys
//!   vary in DER length, so a few handshake datagrams vary by a byte or two.
//! - sctp-proto's initiate tag, initial TSN and state cookie come from the
//!   OS rng, and dimpl's DTLS 1.2 randoms embed the system time. Only
//!   encrypted contents change.
//! - Each endpoint reads `Instant::now()` once, as an anchor; only
//!   differences from it reach str0m.
//!
//! The schedule of datagrams and events is therefore reproducible as long
//! as the network model doesn't depend on exact datagram sizes. Never
//! compare datagram contents across runs.

pub use strims_vnic::webrtc::{
    DATA_CHANNEL_INIT, DATA_CHANNEL_MTU, DEFAULT_ICE_SERVERS, RemoteDescriptionAction, SdpType,
    SessionDescription, WEBRTC_INTERFACE,
};

#[cfg(not(target_arch = "wasm32"))]
mod cert;
#[cfg(not(target_arch = "wasm32"))]
mod clock;
#[cfg(not(target_arch = "wasm32"))]
mod crypto;
#[cfg(not(target_arch = "wasm32"))]
mod endpoint;
#[cfg(not(target_arch = "wasm32"))]
pub mod stun;

#[cfg(not(target_arch = "wasm32"))]
pub use endpoint::{
    CloseReason, ConnId, ConnectionState, EndpointConfig, Error, Event, Transmit, WebRtcEndpoint,
};

/// Seeds the thread-local generator str0m draws non-cryptographic values
/// from (see [Determinism](crate#determinism)). It is process state outside
/// any endpoint, so only simulations should call it: once per simulated
/// thread, before creating connections.
#[cfg(not(target_arch = "wasm32"))]
pub fn seed_str0m_thread_rng(seed: u64) {
    fastrand::seed(seed);
}
