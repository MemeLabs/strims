// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Node events and metrics.

use strims_binmap::Bin;
use strims_kademlia::ID;
use strims_ppspp::{
    ChannelStats, MetricsSnapshot, PeerStats, PushStats, SwarmKey, SwarmSchedulerStats,
};
use strims_vnic::{HostMetrics, LinkKind, LinkMetrics};
use strims_vpn::{Delivery, ExchangeOutcome, NetworkId};

/// Notifications from the node task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeEvent {
    /// A link finished its vnic handshake. The peer's RPC channels can be
    /// taken with `Node::take_peer_rpc`.
    PeerAdded(PeerInfo),
    /// A peer went away (transport closed, replaced or closed locally).
    PeerRemoved { host_id: ID },
    /// A vpn message for a port registered with `Node::set_vpn_handler`.
    /// `handler` is the port.
    Message(Delivery<u16>),
    /// A peer exchange finished (see `Node::connect_peer`).
    ExchangeClosed {
        network: NetworkId,
        host_id: ID,
        outcome: ExchangeOutcome,
    },
    /// A peer's ppspp frame failed (decode, handshake or epoch): its swarm
    /// channels were stopped. The vnic peer stays connected.
    TransferError { host_id: ID, error: String },
}

/// A connected peer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerInfo {
    /// The peer's host id (its vnic host key).
    pub host_id: ID,
    /// The peer's profile public key (from its certificate's parent).
    pub profile_key: Vec<u8>,
    pub kind: LinkKind,
    /// This node made the link: it dialed, or offered the WebRTC session.
    pub dialed: bool,
    pub node_platform: String,
    pub node_version: String,
}

/// One link's counters and buffers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkInfo {
    /// `None` until the handshake completes.
    pub host_id: Option<ID>,
    pub kind: LinkKind,
    pub metrics: LinkMetrics,
    /// This node made the link: it dialed, or offered the WebRTC session.
    pub dialed: bool,
    /// The link's frame size limit.
    pub mtu: usize,
    /// Bytes vnic holds for the link that the transport hasn't taken yet
    /// (handshake bytes or frames).
    pub transmit_pending: usize,
    /// Bytes held of a frame (or handshake message) split across reads.
    pub input_buffered: usize,
    /// The transport takes no more bytes right now: every write buffer is
    /// with the writer task (TCP, WebSocket), or the data channel is at
    /// `WebRtcConfig::max_buffered_amount` or full (WebRTC). vnic then
    /// blocks the link's writers.
    pub write_blocked: bool,
    /// A WebRTC link's data-channel buffered amount: bytes written into the
    /// channel that the peer hasn't acknowledged, sent or not. At most
    /// `WebRtcConfig::max_buffered_amount`. 0 for TCP and WebSocket links.
    pub buffered_amount: usize,
}

/// The driver's own counters: dials, accepts, transports, queues, the
/// node loop and the WebRTC sockets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DriverMetrics {
    pub tcp_dials: u64,
    pub tcp_dial_errors: u64,
    pub ws_dials: u64,
    pub ws_dial_errors: u64,
    /// Connections accepted by the listeners and handed to vnic.
    pub accepted: u64,
    /// Accepted connections dropped before vnic (bad mux init, unknown
    /// profile key, failed upgrade, timeout).
    pub accept_errors: u64,
    /// Transports that failed or closed on their own.
    pub transport_failures: u64,
    /// Links vnic closed (failed handshake, replaced, closed locally).
    pub links_closed: u64,
    /// Bytes handed to vnic and taken from it.
    pub bytes_read: u64,
    pub bytes_written: u64,
    /// Events dropped because the event queue was full or unread.
    pub events_dropped: u64,
    /// Hash table values and peer index hosts dropped on a full stream.
    pub results_dropped: u64,
    /// vpn frames that failed `handle_frame` (malformed, undecryptable...).
    pub vpn_frame_errors: u64,
    pub transfer_errors: u64,
    /// Bytes of peer RPC frames handed to the app and taken from it.
    pub rpc_bytes_read: u64,
    pub rpc_bytes_written: u64,
    /// Peer RPC pairs closed because a channel's input overflowed.
    pub rpc_overflows: u64,
    /// API calls that can change the node (everything but queries).
    pub commands: u64,
    /// Read-only API calls (`Node::metrics`, `Node::peers`), answered after
    /// passes the node made anyway.
    pub queries: u64,
    /// Queries that found no pass due within `NodeConfig::query_delay` and
    /// woke the node. Only these make sampling visible to the run.
    pub query_wakes: u64,
    /// Passes of the node loop (timers, egress pump, dials).
    pub loop_iterations: u64,
    /// WebRTC peer connections created (one per peer exchange while the
    /// interface is enabled).
    pub webrtc_connections: u64,
    /// Data channels that opened and became vnic links.
    pub webrtc_opened: u64,
    /// Peer connections that ended (any reason, opened or not).
    pub webrtc_closed: u64,
    /// WebRTC UDP datagrams and their bytes, as read from and handed to the
    /// sockets. The byte counts include DTLS handshake records whose sizes
    /// vary by a byte or two between same-seed runs (see
    /// [Determinism](crate#determinism)).
    pub udp_datagrams_read: u64,
    pub udp_bytes_read: u64,
    pub udp_datagrams_written: u64,
    pub udp_bytes_written: u64,
    /// Datagrams dropped because a socket's send queue was full.
    pub udp_send_dropped: u64,
    /// Datagrams no connection took (stale, or for a closed connection).
    pub udp_unrouted: u64,
}

/// A snapshot of every counter in the node.
///
/// Collecting it walks every link, network, swarm and channel once and
/// allocates the vectors below; the binmap sizes in ppspp's stats also
/// grow with the maps. Nothing in it depends on the wall clock or on hash
/// order, so under turmoil the same seed gives the same snapshots. Taking
/// one changes nothing the node does: it is answered after a pass the node
/// makes anyway (see [`crate::NodeConfig::query_delay`]), and the rates it
/// reads advance time windows that the next sample would advance anyway.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeMetrics {
    pub host: HostMetrics,
    pub vpn: strims_vpn::Metrics,
    pub driver: DriverMetrics,
    /// In vnic's link order.
    pub links: Vec<LinkInfo>,
    /// The ppspp runner: swarms and per-peer transfer state.
    pub runner: RunnerMetrics,
    /// vpn routing state per joined network, in network order.
    pub networks: Vec<NetworkInfo>,
}

/// The ppspp runner's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerMetrics {
    /// Every swarm the node has (created or added), in key order.
    pub swarms: Vec<SwarmInfo>,
    /// Every connected peer, in vnic link order.
    pub peers: Vec<RunnerPeerInfo>,
}

/// One swarm: its store, its scheduler and its local handles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmInfo {
    pub swarm: SwarmKey,
    /// Peers running a channel for it.
    pub channels: usize,
    pub store: StoreInfo,
    /// The scheduler's state (ppspp `Runner::swarm_stats`); `None` while
    /// the swarm has no channel, as the scheduler exists only then.
    pub scheduler: Option<SwarmSchedulerStats>,
    /// Push/pull state and counters (ppspp `Runner::push_stats`); `None`
    /// while the swarm has no channel.
    pub push: Option<PushStats>,
    /// Open [`crate::SwarmReader`]s, and those waiting for data.
    pub readers: usize,
    pub readers_waiting: usize,
    /// Open [`crate::SwarmWriter`]s (0 or 1).
    pub publishers: usize,
}

/// A swarm's chunk store (ppspp `store::Buffer`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreInfo {
    /// A read offset is set.
    pub ready: bool,
    /// The first bin of the live window, or `Bin::NONE` before `ready`.
    pub tail: Bin,
    /// The first missing chunk after the read offset: where readers stop.
    pub next: Bin,
    /// The window in chunks.
    pub window: u64,
    /// Bumps on every change readers can observe (data, reset, recovery).
    pub readable_seq: u64,
}

/// One peer's transfer state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnerPeerInfo {
    pub host_id: ID,
    /// The upload weight the runner gives the peer.
    pub qos_weight: Option<u64>,
    /// The runner peer's write queues: whether a write cycle runs, channels
    /// queued to write, data queue lengths per priority (pushes, then
    /// requests) and refusals on a full queue.
    pub queues: PeerStats,
    /// Data bytes read from and written to the peer, and their rates over
    /// the last minute. The upload weights come from a separate meter.
    pub metrics: MetricsSnapshot,
    /// vnic's QoS session for the peer's transfer writer.
    pub writer: Option<WriterInfo>,
    /// The swarms run with the peer, in swarm key order.
    pub channels: Vec<ChannelInfo>,
}

/// A vnic writer's QoS session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriterInfo {
    pub weight: u64,
    /// It has a frame to send.
    pub backlogged: bool,
    /// Its link's send buffer is full.
    pub blocked: bool,
}

/// One swarm run with one peer.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelInfo {
    pub swarm: SwarmKey,
    /// Our channel id (the peer's frames carry it) and the peer's.
    pub channel: u64,
    pub peer_channel: u64,
    /// Data bytes and rates on this channel.
    pub metrics: MetricsSnapshot,
    /// The scheduler's view of the channel (handshake, choke, LEDBAT's
    /// cwnd, flight size, CTO and RTT, outstanding requests, binmap sizes)
    /// and the message counters both ways (ppspp `Runner::channel_stats`).
    /// `None` if the runner has no such channel.
    pub stats: Option<ChannelStats>,
}

// ppspp's `ChannelStats` derives only `PartialEq`, but every field in it is
// an integer, a bool, a `Duration` or a `Bin`, so equality is reflexive.
impl Eq for ChannelInfo {}

/// One joined vpn network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkInfo {
    pub network: NetworkId,
    /// Peers bound to the network, and those in its routing table (a link
    /// that doesn't fit its bucket is read but not routed through).
    pub links: usize,
    pub routed_links: usize,
    /// Message copies queued on its links, waiting for vnic.
    pub queued: usize,
    /// Hosts in the learned link graph (`Routing::Learned`).
    pub topology: usize,
}
