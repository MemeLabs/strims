// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node every driver runs: vnic, vpn and ppspp composed into one task,
//! its API handle, and the loop around them.
//!
//! The crate is runtime-neutral. It uses tokio's channels and `select!`
//! only (the `sync` and `macros` features, which need no runtime), owns no
//! sockets, reads no clock and spawns nothing. A driver
//! (`strims-node` natively and under turmoil, `strims-wasm` in a browser
//! worker) implements [`NodeIo`], builds a node with [`start`] and runs the
//! returned [`NodeTask`] on its executor.
//!
//! # Architecture
//!
//! One node task owns the sans-IO state machines: `strims_vnic::Host`,
//! `strims_vpn::Host`, `strims_ppspp::Runner`, and the WebRTC endpoint
//! ([`RtcEndpoint`]) when WebRTC is enabled. Nothing else touches them.
//! [`Node`] sends closures that run on the state and reply over a oneshot.
//! The driver's per-link reader and writer tasks own the transports and
//! trade recycled buffers with the node task over bounded channels
//! ([`LinkChannels`]).
//!
//! | piece | role |
//! |---|---|
//! | [`Node`] | the API handle: dials, networks, vpn services and ports, swarms, peer RPC |
//! | `state` | the composition: event routing, egress pipeline, bookkeeping (no I/O) |
//! | [`driver`] | the node task's loop around the state, and [`NodeIo`] |
//! | [`rtc`] | the WebRTC interface: [`RtcEndpoint`], and [`ProxyRtc`] for connections outside the node task |
//!
//! Each pass of the node task handles inputs and commands, runs the due
//! timers, then pumps egress: drain events, flush vpn, poll vnic's egress
//! (which has vpn or the runner fill frames, by owner tag), and copy link
//! bytes into the link's free write buffers or its in-process data
//! channel.
//!
//! **Backpressure.** A link whose write buffers are all with its writer
//! task (or whose data channel is full) leaves its bytes in vnic. vnic's
//! full link buffer blocks the link's QoS sessions, so owners stop being
//! asked to fill frames until a buffer comes back. The node task never
//! waits on a transport, and every queue between tasks is bounded
//! ([`NodeConfig`], [`BufferConfig`], [`WebRtcConfig`]).
//!
//! **Allocation.** Stream link bytes go transport → read buffer → vnic
//! (decrypted in place) → owner, and back through vnic's link buffer and a
//! write buffer, with no per-frame allocation in the node. A vpn delivery
//! moves its body into [`NodeEvent::Message`].
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
//! **Interfaces.** A driver says which link interfaces it has
//! ([`Interfaces`]). Without TCP, peer exchanges offer no TCP candidate
//! and `tcp://` dials fail with [`NodeError::Unsupported`].
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
//! On a deterministic driver (turmoil, a paused runtime) the same seed
//! replays the same run. That holds while:
//!
//! - the executor is single-threaded. Multi-thread runtimes work natively,
//!   as everything the node task holds is `Send`, but aren't
//!   deterministic;
//! - every `select!` is `biased`, and the driver spawns in the order the
//!   loop asks: its own tasks ([`NodeIo::start`]) before the first wait, a
//!   link's reader then its writer when the link is attached, dials after
//!   each pass;
//! - no hash map is iterated;
//! - every rng forks from [`NodeConfig::rng_seed`]: vnic, vpn and the
//!   runner in that order, then the WebRTC endpoint ([`RtcFactory`]), so
//!   enabling WebRTC changes no other key ([`NodeConfig::host_key_seed`]
//!   overrides vnic's);
//! - time comes from the driver ([`NodeIo::now`]).
//!
//! Read-only queries ([`Node::metrics`], [`Node::peers`]) are answered after
//! passes the node makes anyway, and an idle node answers without running
//! timers or egress (see [`NodeConfig::query_delay`]). Waking an idle node
//! is still a poll of its task, which can shift the order of later passes,
//! so a simulation's sampling cadence is one of its inputs.

pub mod config;
pub mod driver;
pub mod error;
pub mod event;
mod exchange;
mod handle;
pub mod rpc;
pub mod rtc;
mod state;
pub mod vpn_port;

pub use config::{
    BufferConfig, DEFAULT_NOTSENT_LOWAT, NetworkOptions, NodeConfig, PeerRpcConfig,
    TcpSocketConfig, VpnPortConfig, WebRtcConfig,
};
pub use driver::{
    IdleOk, Input, Inputs, Interfaces, LinkChannels, LinkTasks, NodeIo, NodeSetup, NodeTask,
    TaskGuard, start,
};
pub use error::{NodeError, ReadError};
pub use event::{
    ChannelInfo, DriverMetrics, LinkInfo, NetworkInfo, NodeEvent, NodeMetrics, PeerInfo,
    RunnerMetrics, RunnerPeerInfo, StoreInfo, SwarmInfo, WriterInfo,
};
pub use handle::{
    HashTableGet, Node, NodeEvents, NodeInfo, PeerIndexSearch, SpawnNode, SwarmReader, SwarmWriter,
};
pub use rpc::{PeerRpc, RpcChannel, RpcReader, RpcWriter};
pub use rtc::{
    ProxyRtc, RtcCloseReason, RtcCommand, RtcConn, RtcEndpoint, RtcEvent, RtcFactory, RtcInput,
    RtcSignal,
};
pub use state::DialId;
pub use vpn_port::{VpnPort, VpnReader, VpnWriter};

pub use strims_kademlia::ID as HostId;
pub use strims_ppspp::{DeliveryMode, SwarmId, SwarmKey, SwarmOptions, WriterOptions};
pub use strims_vnic::{LinkKind, PeerKey as LinkKey, TcpOptions};
pub use strims_vpn::{
    Delivery, ExchangeOutcome, HashTableGetOptions, HashTablePublisherId, HostAddr, NetworkId,
    PeerIndexHost, PeerIndexPublisherId,
};
pub use strims_webrtc::EndpointConfig as WebRtcEndpointConfig;
