// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Node configuration.

use std::net::IpAddr;

use strims_base::time::Duration;
use strims_proto::types::Key;
use strims_vnic::TcpOptions;
use strims_vnic::ws::WsOptions;
use strims_webrtc::EndpointConfig;

/// Everything a node needs to start.
#[derive(Clone, Debug)]
pub struct NodeConfig {
    /// The profile key: ed25519, with a 64-byte private key (seed then
    /// public key) and a 32-byte public key. TCP listeners route and
    /// encrypt by its public key, and the host certificate is signed with
    /// it.
    pub profile_key: Key,
    /// Seeds every rng in the node (vnic, vpn, the ppspp runner). Use
    /// [`random_seed`] in production and the simulation seed in tests.
    pub rng_seed: u64,
    /// Seeds vnic's rng, whose first draw is the host key. `None` derives it
    /// from `rng_seed`.
    pub host_key_seed: Option<[u8; 32]>,
    /// The host certificate's subject.
    pub label: String,
    /// TCP listen address (`host:port`; port 0 picks one).
    pub tcp_listen: Option<String>,
    /// WebSocket listen address. The link is served at `/<hex profile key>`.
    pub ws_listen: Option<String>,
    /// The address peers reach this node at, for the advertised TCP URI
    /// (and the WebSocket URI when `ws_public_hostname` is empty). Needed
    /// when listening on an unspecified address. Without it, such a
    /// listener advertises no URI.
    pub public_ip: Option<IpAddr>,
    /// The host in the advertised WebSocket URI; empty uses `public_ip`.
    pub ws_public_hostname: String,
    /// The port in the advertised WebSocket URI; 0 keeps the listen port.
    pub ws_public_port: u16,
    /// Allow dialing `ws://`.
    pub allow_insecure_ws: bool,
    /// The upload cap in bytes per second, applied to vnic's QoS
    /// (`strims_qos::Control::set_rate_limit`).
    ///
    /// Set it a little below the real uplink. QoS weights (vpn vs.
    /// transfer classes, and the ppspp runner's per-peer upload
    /// reciprocity) only shape traffic when QoS is the bottleneck; if the
    /// socket is, frames queue in the kernel and in the link buffers in
    /// arrival order. `None` keeps QoS's default (2^40 B/s, so
    /// effectively uncapped and unweighted). Change it at runtime with
    /// `Node::set_rate_limit`.
    pub rate_limit: Option<u64>,
    /// vnic's peer budget, which gates peer exchange. 0 refuses every
    /// offer.
    pub max_peers: u32,
    /// Deadlines for TCP links, and keepalive and buffer sizes for the
    /// sockets under TCP and WebSocket links. The read timeout (25 s) must
    /// exceed vnic's keepalive interval
    /// ([`strims_vnic::DEFAULT_KEEPALIVE_INTERVAL`]), so a peer that sends
    /// keepalives is never silent for a whole read timeout. TCP keepalive
    /// (20 s) finds a dead peer on a link exempt from the read timeout.
    pub tcp: TcpOptions,
    /// Send-queue options for the TCP sockets under TCP and WebSocket
    /// links.
    pub tcp_socket: TcpSocketConfig,
    /// Deadlines and pings for WebSocket links. `ping_interval` (20 s) must
    /// be shorter than `read_timeout` (25 s): pongs renew an idle link's
    /// read deadline.
    pub ws: WsOptions,
    /// The WebRTC interface (off by default).
    pub webrtc: WebRtcConfig,
    pub vpn: strims_vpn::Config,
    /// Connect plus transport handshake (WebSocket upgrade) for dials, and
    /// the `TCPMuxInit` read on accepted connections.
    pub dial_timeout: Duration,
    pub buffers: BufferConfig,
    /// The RPC channel pair every peer gets (see [`crate::rpc`]).
    pub peer_rpc: PeerRpcConfig,
    /// Node events waiting for the app; more are dropped and counted.
    pub event_queue: usize,
    /// Commands waiting for the node task.
    pub command_queue: usize,
    /// Transport inputs (reads, finished writes, new links) waiting for the
    /// node task. Link tasks wait when it is full.
    pub input_queue: usize,
    /// How long a query (`Node::metrics`, `Node::peers`) may wait for the
    /// node's next pass.
    ///
    /// Queries are answered right after a pass the node makes anyway, so
    /// sampling a node changes nothing it does: the same seed gives the
    /// same run with or without sampling. A wakeup made only to answer is
    /// not passive: an extra poll of the node task changes how the runtime
    /// orders the tasks after it, and under a backlogged upload cap that
    /// changes which frames go out when. A node whose next pass is further
    /// away than this (an idle node) is woken, and answers without running
    /// its timers or egress (counted in `DriverMetrics::query_wakes`). A
    /// node with peers passes at least every 100 ms (the ppspp tick). Zero
    /// wakes the node for every query.
    pub query_delay: Duration,
    /// Sent in `PeerInit`.
    pub node_platform: String,
    pub node_version: String,
}

/// The default `TCP_NOTSENT_LOWAT`: 16 KiB.
///
/// The kernel wakes a blocked writer once fewer than half the mark's bytes
/// wait to be sent, so about 8 KiB stay queued while the writer task
/// refills: 0.8 ms of data at 10 MB/s, much longer than a refill takes.
/// Above that, every byte the kernel holds unsent is a byte QoS can no
/// longer reorder; with this mark it holds at most about 32 KiB (the mark
/// plus one write), a few milliseconds at stream rates.
pub const DEFAULT_NOTSENT_LOWAT: u32 = 16 * 1024;

/// Kernel options for the TCP sockets under TCP and WebSocket links, set
/// on every dialed and accepted socket.
///
/// Data waiting in vnic's QoS is sent freshest first and by class weight;
/// data in the kernel's send buffer leaves in the order it was written.
/// These options keep the kernel's share small when a link is slower than
/// the node's upload. Socket options exist only on real sockets
/// ([`crate::TokioNet`]); simulated ones ignore them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpSocketConfig {
    /// `TCP_NOTSENT_LOWAT`: the most bytes the kernel queues that it hasn't
    /// sent yet. Bytes in flight are bounded separately, by the congestion
    /// window. A link whose socket has the mark also cuts its writes to
    /// that size (at least 4 KiB) and keeps at most one MTU plus one write
    /// in vnic, so the bytes between QoS and the wire stay a few writes.
    /// `None` leaves the kernel's default (unbounded up to the send
    /// buffer). Set on Linux and Android; a no-op elsewhere.
    pub notsent_lowat: Option<u32>,
    /// `TCP_CONGESTION`, the kernel's congestion controller for the
    /// socket (for example `"bbr"`). Best effort: a controller the kernel
    /// lacks or doesn't allow is logged at debug level and the socket
    /// keeps the system default. Linux only; a no-op elsewhere.
    pub congestion: Option<String>,
}

impl Default for TcpSocketConfig {
    fn default() -> TcpSocketConfig {
        TcpSocketConfig {
            notsent_lowat: Some(DEFAULT_NOTSENT_LOWAT),
            congestion: None,
        }
    }
}

/// Per-link transport buffers. A link holds `read_buffers` buffers of
/// `read_buffer_size` bytes and `write_buffers` buffers of up to one link
/// MTU, allocated when the link is added and reused for its lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferConfig {
    pub read_buffers: usize,
    pub read_buffer_size: usize,
    pub write_buffers: usize,
}

impl Default for BufferConfig {
    fn default() -> BufferConfig {
        BufferConfig {
            read_buffers: 2,
            read_buffer_size: 64 * 1024,
            write_buffers: 2,
        }
    }
}

impl NodeConfig {
    /// A node with no listeners and the defaults.
    pub fn new(profile_key: Key, rng_seed: u64) -> NodeConfig {
        NodeConfig {
            profile_key,
            rng_seed,
            host_key_seed: None,
            label: String::new(),
            tcp_listen: None,
            ws_listen: None,
            public_ip: None,
            ws_public_hostname: String::new(),
            ws_public_port: 0,
            allow_insecure_ws: false,
            rate_limit: None,
            max_peers: 0,
            tcp: TcpOptions::default(),
            tcp_socket: TcpSocketConfig::default(),
            ws: WsOptions::default(),
            webrtc: WebRtcConfig::default(),
            vpn: strims_vpn::Config::default(),
            dial_timeout: Duration::from_secs(20),
            buffers: BufferConfig::default(),
            peer_rpc: PeerRpcConfig::default(),
            event_queue: 1024,
            command_queue: 256,
            input_queue: 256,
            query_delay: Duration::from_millis(200),
            node_platform: String::new(),
            node_version: "dev".to_owned(),
        }
    }
}

/// The WebRTC interface: data-channel links made by vpn peer exchange
/// (`strims-webrtc`, str0m), over UDP mux sockets.
///
/// WebRTC links are never dialed: they come up when `Node::connect_peer`
/// (or a remote offer) trades session descriptions over the network, so a
/// node without listeners (a browser, a node behind NAT) can still link
/// directly to another node it reaches only through the network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebRtcConfig {
    /// Offer a WebRTC candidate in peer exchanges. Off by default; enabling
    /// it needs at least one `listen` address.
    pub enabled: bool,
    /// One UDP mux socket per address (`ip:port`; port 0 picks one). Every
    /// connection gets a host candidate on each socket. str0m matches a
    /// datagram's destination against the host candidates, so bind
    /// concrete IPs: an unspecified address (`0.0.0.0`) needs
    /// `endpoint.host_ip` or [`NodeConfig::public_ip`] of the same family,
    /// which host candidates then advertise.
    pub listen: Vec<String>,
    /// The str0m endpoint: `ice_servers` (`None` is
    /// [`strims_webrtc::DEFAULT_ICE_SERVERS`], `Some(vec![])` none; only
    /// `stun:` URLs are used), `host_ip` (NAT 1:1 host candidates; `None`
    /// uses [`NodeConfig::public_ip`]), and the connect, ICE failure, send
    /// buffer and connection limits.
    pub endpoint: EndpointConfig,
    /// How often STUN server names are resolved again. They are first
    /// resolved in the background at start; a connection created before
    /// that answer gathers host candidates only.
    pub stun_refresh: Duration,
    /// Receive buffers per UDP socket, recycled between the socket task and
    /// the node task.
    pub udp_read_buffers: usize,
    /// The largest datagram read (larger ones are truncated by the OS).
    /// WebRTC datagrams fit a path MTU of about 1200 bytes.
    pub udp_read_buffer_size: usize,
    /// Datagrams queued per socket for sending; more are dropped and
    /// counted (`DriverMetrics::udp_send_dropped`), and SCTP retransmits.
    pub udp_send_queue: usize,
    /// The most bytes a WebRTC link keeps in its data channel's buffered
    /// amount; once there, the link writes again when it has drained to
    /// half. Bytes held back wait in vnic's QoS, which serves fresh data
    /// and the class weights first; the channel sends in FIFO order.
    ///
    /// The buffered amount counts bytes sent and not yet acknowledged as
    /// well as bytes not yet sent, so this also bounds a link's throughput
    /// to between half this and this much per round trip: the default,
    /// 256 KiB, allows 1.3 to 2.6 MB/s at a 100 ms round trip, above
    /// stream rates, and holds at most that much in FIFO order. The
    /// endpoint's own `send_buffer` (larger) is the hard limit behind it.
    pub max_buffered_amount: usize,
}

impl Default for WebRtcConfig {
    fn default() -> WebRtcConfig {
        WebRtcConfig {
            enabled: false,
            listen: Vec::new(),
            endpoint: EndpointConfig::default(),
            stun_refresh: Duration::from_secs(300),
            udp_read_buffers: 32,
            udp_read_buffer_size: 4096,
            udp_send_queue: 1024,
            max_buffered_amount: 256 * 1024,
        }
    }
}

impl WebRtcConfig {
    /// Enabled, on these UDP addresses, with no ICE servers (host
    /// candidates only: LANs, tests, simulations).
    pub fn local(listen: impl IntoIterator<Item = String>) -> WebRtcConfig {
        WebRtcConfig {
            enabled: true,
            listen: listen.into_iter().collect(),
            endpoint: EndpointConfig {
                ice_servers: Some(Vec::new()),
                ..EndpointConfig::default()
            },
            ..WebRtcConfig::default()
        }
    }
}

/// Peer RPC channels (see [`crate::rpc`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerRpcConfig {
    /// Open an RPC channel pair for every peer. Off, frames on the pair's
    /// ports are dropped by vnic.
    pub enabled: bool,
    /// Received bytes a channel holds that the app hasn't read, including
    /// before it takes the pair. A frame past this closes the pair. At
    /// least the largest Call plus a link MTU, so one Call can arrive while
    /// the app is busy.
    pub inbound_buffer: usize,
    /// Bytes a channel's writer may have queued in the node: the writer
    /// isn't ready while the node holds this much (plus at most one Call).
    pub outbound_buffer: usize,
}

impl Default for PeerRpcConfig {
    fn default() -> PeerRpcConfig {
        PeerRpcConfig {
            enabled: true,
            inbound_buffer: 1024 * 1024,
            outbound_buffer: 1024 * 1024,
        }
    }
}

/// Options for a network joined with `Node::add_network`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkOptions {
    /// Bind every vnic peer to the network on this port pair as it
    /// connects (and the peers already connected), so nodes that agree on
    /// a port join a network without calling `Node::bind_peer_network` per
    /// peer.
    pub auto_bind_port: Option<u16>,
}

/// A seed from OS entropy, for production nodes.
pub fn random_seed() -> Result<u64, getrandom::Error> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b)?;
    Ok(u64::from_le_bytes(b))
}
