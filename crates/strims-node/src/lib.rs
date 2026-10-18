// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Tokio driver composing vnic, vpn and ppspp over TCP, WebSocket and
//! WebRTC, natively or under turmoil.
//!
//! A driver crate: OS access (sockets, the wall clock, OS entropy) happens
//! here, and only behind [`Net`], [`Clock`] and [`random_seed`].
//!
//! # Architecture
//!
//! One node task owns the sans-IO state machines: `strims_vnic::Host`,
//! `strims_vpn::Host`, `strims_ppspp::Runner`, and
//! `strims_webrtc::WebRtcEndpoint` when WebRTC is enabled. Nothing else
//! touches them. [`Node`] sends closures that run on the state and reply
//! over a oneshot. Per-link reader and writer tasks, and per-UDP-socket
//! tasks, own the sockets and trade recycled buffers with the node task
//! over bounded channels.
//!
//! | piece | role |
//! |---|---|
//! | [`Node`] | the API handle: dials, networks, vpn services and ports, swarms, peer RPC |
//! | `state` | the composition: event routing, egress pipeline, bookkeeping (no I/O) |
//! | `driver` | the node task's loop around the state |
//! | `link`, `transport` | link I/O tasks; TCP and WebSocket dial and accept |
//! | `udp` | WebRTC UDP socket tasks and the STUN resolver |
//! | [`Net`], [`Clock`] | sockets and time: tokio or turmoil, wall clock or simulated |
//!
//! Each pass of the node task handles inputs and commands, runs the due
//! timers, then pumps egress: drain events, flush vpn, poll vnic's egress
//! (which has vpn or the runner fill frames, by owner tag), and copy link
//! bytes into the link's free write buffers or its data channel.
//!
//! **Backpressure.** A link whose write buffers are all with its writer
//! task (or whose data channel is full) leaves its bytes in vnic. vnic's
//! full link buffer blocks the link's QoS sessions, so owners stop being
//! asked to fill frames until a buffer comes back. The node task never
//! waits on a socket, and every queue between tasks is bounded
//! ([`NodeConfig`], [`BufferConfig`], [`WebRtcConfig`]).
//!
//! **Allocation.** Stream link bytes go socket → read buffer → vnic
//! (decrypted in place) → owner, and back through vnic's link buffer and a
//! write buffer, with no per-frame allocation. tungstenite allocates every
//! WebSocket message, str0m every data-channel message and sent datagram,
//! and a vpn delivery moves its body into [`NodeEvent::Message`].
//!
//! **Peer RPC.** Every peer gets a vnic channel pair for RPC, in a QoS
//! class of weight 1. The app takes it once ([`Node::take_peer_rpc`]) as
//! byte streams to run `strims-rpc` over. A channel whose unread input
//! passes its buffer closes the pair; the peer stays connected, and closing
//! it is the app's call. See [`rpc`].
//!
//! **vpn RPC.** [`Node::open_vpn_port`] gives the app a port's messages and
//! sends, for `strims_rpc::vpn`. Services publish their address with
//! [`Node::publish_vpn_address`], and clients find it with
//! [`Node::resolve_vpn_address`]. See [`vpn_port`].
//!
//! # App layer
//!
//! [`app`] runs the app crates on a node: [`app::NodeTimer`] is the node's
//! clock as their timer, [`app::NodeHost`] is the node as the host of the
//! services a profile runs (replication, transfers, network membership
//! and the directory), and [`app::PeerServices`] serves every peer's RPC
//! link with the app's [`app::PeerHandler`]s.
//! [`video_host::NetVideoHost`] gives the video control its RTMP
//! listeners on the node's [`Net`].
//!
//! # Bindings
//!
//! Peers join networks through [`Node::bind_peer_network`], reported as
//! [`NodeEvent::NetworkPeerAdded`]. `strims-network` negotiates the
//! bindings over peer rpc and each peer's broker channel, and calls it;
//! [`NetworkOptions::auto_bind_port`] binds every peer on a fixed port
//! pair for simulations and transport tests, and doesn't mix with it.
//!
//! Swarm channels are negotiated by `strims-transfer` over peer rpc, for
//! the swarms published on networks the peers share. [`Node::run_swarm`]
//! is the operation it uses, and binds a channel directly in tests.
//!
//! # Determinism
//!
//! Under turmoil the same seed replays the same run. That holds while:
//!
//! - the runtime is current-thread (turmoil hosts are; so is
//!   `#[tokio::test]` by default). Multi-thread runtimes work, as
//!   everything is `Send`, but aren't deterministic;
//! - every `tokio::select!` is `biased`, and tasks spawn in loop order:
//!   accept loops and UDP tasks at start, a link's reader then its writer
//!   when the link is attached, dials after each pass;
//! - no hash map is iterated;
//! - every rng forks from [`NodeConfig::rng_seed`]: vnic, vpn and the
//!   runner in that order, then the WebRTC endpoint, so enabling WebRTC
//!   changes no other key ([`NodeConfig::host_key_seed`] overrides vnic's);
//! - time comes from the [`Clock`].
//!
//! Read-only queries ([`Node::metrics`], [`Node::peers`]) are answered after
//! passes the node makes anyway, and an idle node answers without running
//! timers or egress (see [`NodeConfig::query_delay`]). Waking an idle node
//! is still a poll of its task, which can shift the order of later passes,
//! so a simulation's sampling cadence is one of its inputs.
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
//! - **Cost.** Debug builds spend most of a run in ed25519, BLAKE2b and
//!   AES; run large suites with `--release` or `opt-level = 1`.

pub mod app;
pub mod clock;
pub mod config;
mod driver;
pub mod error;
pub mod event;
mod exchange;
mod handle;
mod link;
pub mod net;
pub mod rpc;
mod state;
mod transport;
mod udp;
pub mod video_host;
pub mod vpn_port;

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
pub use handle::{
    HashTableGet, Node, NodeEvents, NodeInfo, PeerIndexSearch, SwarmReader, SwarmWriter,
};
#[cfg(feature = "turmoil")]
pub use net::TurmoilNet;
pub use net::{Net, TokioNet};
pub use rpc::{PeerRpc, RpcChannel, RpcReader, RpcWriter};
pub use vpn_port::{VpnPort, VpnReader, VpnWriter};

pub use strims_kademlia::ID as HostId;
pub use strims_ppspp::{DeliveryMode, SwarmId, SwarmKey, SwarmOptions, WriterOptions};
pub use strims_vnic::{LinkKind, TcpOptions};
pub use strims_vpn::{
    Delivery, ExchangeOutcome, HashTableGetOptions, HashTablePublisherId, HostAddr, NetworkId,
    PeerIndexHost, PeerIndexPublisherId,
};
pub use strims_webrtc::{EndpointConfig as WebRtcEndpointConfig, seed_str0m_thread_rng};
