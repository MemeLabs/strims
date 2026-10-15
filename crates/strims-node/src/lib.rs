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
//! | [`Node`] | the API handle: dials, networks, vpn services, swarms |
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
//! # Bindings
//!
//! Peers join networks and swarms through explicit calls on both sides:
//! [`Node::bind_peer_network`] (or [`NetworkOptions::auto_bind_port`]) and
//! [`Node::run_swarm`]. These stand in until the app layer negotiates
//! network membership and swarm channels over peer rpc.
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

pub mod clock;
pub mod config;
mod driver;
pub mod error;
pub mod event;
mod exchange;
mod handle;
mod link;
pub mod net;
mod state;
mod transport;
mod udp;

pub use clock::{Clock, SimClock, SystemClock};
pub use config::{
    BufferConfig, DEFAULT_NOTSENT_LOWAT, NetworkOptions, NodeConfig, TcpSocketConfig, WebRtcConfig,
    random_seed,
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

pub use strims_kademlia::ID as HostId;
pub use strims_ppspp::{DeliveryMode, SwarmId, SwarmKey, SwarmOptions, WriterOptions};
pub use strims_vnic::{LinkKind, TcpOptions};
pub use strims_vpn::{
    Delivery, ExchangeOutcome, HashTableGetOptions, HashTablePublisherId, NetworkId, PeerIndexHost,
    PeerIndexPublisherId,
};
pub use strims_webrtc::{EndpointConfig as WebRtcEndpointConfig, seed_str0m_thread_rng};
