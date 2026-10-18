// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The Tokio driver of the node: [`strims_node_core`] over TCP, WebSocket
//! and WebRTC, natively or under turmoil.
//!
//! A driver crate: OS access (sockets, the wall clock, OS entropy,
//! threads) happens here, and only behind [`Net`], [`Clock`],
//! [`random_seed`] and [`psi::ThreadPsi`]. The node
//! itself (its state machines, its API handle [`Node`] and its loop) is
//! `strims_node_core`, re-exported here; this crate implements the core's
//! `NodeIo` on tokio (`native`) and spawns the node task with
//! [`Node::spawn`].
//!
//! # Architecture
//!
//! | piece | role |
//! |---|---|
//! | `native` | the core's `NodeIo`: spawning, the clock, the timer; [`Node::spawn`] on a [`Net`] with a [`SystemClock`] or [`SimClock`] |
//! | `link`, `transport` | link I/O tasks; TCP and WebSocket dial and accept |
//! | `rtc`, `udp` | the WebRTC endpoint (str0m) in the node task; its UDP socket tasks and the STUN resolver |
//! | [`Net`], [`Clock`] | sockets and time: tokio or turmoil, wall clock or simulated |
//! | [`app`] | the native platform of the core's app layer |
//! | [`psi`] | PSI sessions on a thread of their own, off the node's |
//!
//! str0m is sans-IO, so the WebRTC endpoint runs inside the node task as
//! the core's `RtcEndpoint`, and its data channels move link bytes
//! in-process. Datagrams go through one reader and one writer task per UDP
//! socket.
//!
//! **Allocation.** tungstenite allocates every WebSocket message, str0m
//! every data-channel message and sent datagram.
//!
//! # App layer
//!
//! The core composes the app crates on a node (`strims_node_core::app`),
//! and [`app`] gives it the native platform: [`app::NodeTimer`] is the
//! node's clock as their timer, [`app::NativePlatform`] the core's
//! [`app::AppPlatform`] on a [`Net`] and a [`Clock`], and
//! [`app::NodeFactory`] the session factory that runs a signed-in profile
//! on it. [`video_host::NetVideoHost`] gives the video control its RTMP
//! listeners on the node's [`Net`], and [`fetch::NetFetch`] the
//! directory its HTTPS requests. [`app::AppConfig::psi`] picks where the
//! network control's PSI sessions compute: inline, the default, or on a
//! [`psi::ThreadPsi`].
//!
//! # Determinism
//!
//! Under turmoil the same seed replays the same run, on the core's terms
//! ([Determinism](strims_node_core#determinism)): the runtime is
//! current-thread (turmoil hosts are; so is `#[tokio::test]` by default),
//! and time comes from the [`Clock`]. Tasks spawn in the loop's order:
//! accept loops, then each UDP socket's reader and writer, then the STUN
//! resolver at start; a link's reader then its writer when the link is
//! attached; dials after each pass.
//!
//! Randomness the seed doesn't reach changes bytes, never the schedule:
//!
//! - tungstenite draws the WebSocket key and every frame mask from
//!   `rand::thread_rng`. That changes wire bytes but not their lengths, so
//!   a test that hashes raw wire bytes must use TCP.
//! - str0m's thread-local `fastrand` (SDP session ids, ICE tie-breakers,
//!   transaction ids) is seeded by [`seed_str0m_thread_rng`], once per
//!   simulation before it runs; every turmoil host runs on that thread.
//! - dimpl's ECDHE keys, sctp-proto's tags and TSNs, and the time field of
//!   DTLS randoms are unseeded. They change datagram contents, and the
//!   ECDSA signature's DER length changes a few DTLS handshake records by
//!   up to 2 bytes. A trace with WebRTC must leave out datagram contents
//!   and sizes, and with them `DriverMetrics::udp_bytes_read` and
//!   `udp_bytes_written`.
//!
//! # Simulating with turmoil
//!
//! Enable the `turmoil` feature. `tests/turmoil.rs` is the template, and
//! `tests/turmoil_webrtc.rs` the one with WebRTC.
//!
//! - **Per host:** `NodeConfig::new(profile_key, seed + i)`, with every key
//!   derived from the simulation seed; listeners on `0.0.0.0:<port>` with
//!   `public_ip = Some(turmoil::lookup(name))`; then
//!   `Node::spawn(config, TurmoilNet, SimClock::turmoil(EPOCH))`. URIs are
//!   `tcp://<ip>:<port>/<hex profile key>`, so scripts can dial without
//!   exchanging addresses.
//! - **WebRTC:** `WebRtcConfig::local(["0.0.0.0:<port>"])`. turmoil binds
//!   only the wildcard, so host candidates advertise `public_ip`.
//! - **Scripts** run inside the host closures. Hosts share one thread, so
//!   share traces through `Rc<RefCell<_>>` and handle events in a
//!   `spawn_local` task. Scenario code follows the rules above: `biased`
//!   selects, no `HashMap`, no `SystemTime`, `Instant::now` or `OsRng`,
//!   every value from the seed, a fixed spawn order.
//! - **Traces** stamp entries with the [`SimClock`] and hash them; assert
//!   that a seed reproduces its hash and another seed changes it.
//! - **Network model.** turmoil models no bandwidth: model uplinks with
//!   [`NodeConfig::rate_limit`], which also makes QoS weights matter. Its
//!   `fail_rate` drops TCP segments and stalls the stream until the read
//!   timeout, so apply loss to UDP only. Per-datagram latency jitter
//!   reorders datagrams, which SCTP treats as loss. A partition between
//!   two WebRTC hosts makes ICE fail and the exchange time out.
//! - **Churn.** `sim.bounce` re-runs a host; `SimClock::turmoil` keeps its
//!   time consistent, and the node gets new links. turmoil has no TCP
//!   keepalive, so a crashed host's links close when the peer's next write
//!   fails, or at the read timeout once the peer has sent a keepalive.
//! - **PSI.** A profile's PSI sessions compute inline under
//!   [`app::AppConfig`]'s default [`app::PsiMode::Inline`]. A
//!   [`psi::ThreadPsi`] answers in wall time, so a simulation never uses
//!   [`app::PsiMode::Thread`].
//! - **Cost.** Debug builds spend most of a run in ed25519, BLAKE2b and
//!   AES; run large suites with `--release` or `opt-level = 1`.

pub mod app;
pub mod clock;
pub mod fetch;
mod link;
mod native;
pub mod net;
pub mod psi;
mod rtc;
mod transport;
mod udp;
pub mod video_host;

pub use strims_node_core::{error, event, rpc, vpn_port};

/// Node configuration (`strims_node_core::config`), and OS entropy for its
/// seed.
pub mod config {
    pub use strims_node_core::config::*;

    /// A seed from OS entropy, for production nodes.
    pub fn random_seed() -> Result<u64, getrandom::Error> {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
}

pub use clock::{Clock, SimClock, SystemClock};
pub use config::{
    BufferConfig, DEFAULT_NOTSENT_LOWAT, NetworkOptions, NodeConfig, PeerRpcConfig,
    TcpSocketConfig, VpnPortConfig, WebRtcConfig, random_seed,
};
pub use error::{NodeError, ReadError};
pub use event::{
    ChannelInfo, DriverMetrics, LinkInfo, NetworkInfo, NodeEvent, NodeMetrics, PeerInfo,
    RunnerMetrics, RunnerPeerInfo, StoreInfo, SwarmInfo, WriterInfo,
};
#[cfg(feature = "turmoil")]
pub use net::TurmoilNet;
pub use net::{Net, TokioNet};
pub use rpc::{PeerRpc, RpcChannel, RpcReader, RpcWriter};
pub use strims_node_core::{
    HashTableGet, Interfaces, Node, NodeEvents, NodeInfo, PeerIndexSearch, SwarmReader, SwarmWriter,
};
pub use vpn_port::{VpnPort, VpnReader, VpnWriter};

pub use strims_kademlia::ID as HostId;
pub use strims_ppspp::{DeliveryMode, SwarmId, SwarmKey, SwarmOptions, WriterOptions};
pub use strims_vnic::{LinkKind, TcpOptions};
pub use strims_vpn::{
    Delivery, ExchangeOutcome, HashTableGetOptions, HashTablePublisherId, HostAddr, NetworkId,
    PeerIndexHost, PeerIndexPublisherId,
};
pub use strims_webrtc::{EndpointConfig as WebRtcEndpointConfig, seed_str0m_thread_rng};
