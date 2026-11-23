// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Node events and metrics.

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_kademlia::ID;
use strims_ppspp::codec::Stream;
use strims_ppspp::{
    AllocCounters, ChannelMetrics, ChannelStats, ChannelTotals, HaveRecord, LagForecast,
    MetricsSnapshot, ParentWhy, PeerStats, PushStats, ReportCounters, RequestRecord, SeedDecision,
    SeedSend, SwarmCounters, SwarmId, SwarmKey, SwarmSchedulerStats, TapKind,
};
use strims_vnic::{HostMetrics, LinkKind, LinkMetrics, LinkPath};
use strims_vpn::{Delivery, ExchangeOutcome, NetworkId};

/// Notifications from the node task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeEvent {
    /// A link finished its vnic handshake. The peer's RPC channels can be
    /// taken with `Node::take_peer_rpc`.
    PeerAdded(PeerInfo),
    /// A peer went away (transport closed, replaced or closed locally).
    /// It has left every network, with no `NetworkPeerRemoved`.
    PeerRemoved { host_id: ID },
    /// A connected peer was bound to a network, by
    /// `Node::bind_peer_network` or `NetworkOptions::auto_bind_port`. It
    /// follows the peer's `PeerAdded`.
    NetworkPeerAdded { network: NetworkId, host_id: ID },
    /// A peer's network binding went, by `Node::unbind_peer_network` or
    /// `Node::remove_network`.
    NetworkPeerRemoved { network: NetworkId, host_id: ID },
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
    /// vnic's measure of the link's path, the same on every transport.
    pub path: Option<LinkPath>,
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
    /// Bytes of broker frames handed to the app and taken from it.
    pub broker_bytes_read: u64,
    pub broker_bytes_written: u64,
    /// Broker channels closed because their input overflowed.
    pub broker_overflows: u64,
    /// Messages for a vpn port dropped because its reader was behind.
    pub vpn_port_drops: u64,
    /// Messages from vpn ports that vpn refused to send (too large, no
    /// network).
    pub vpn_send_errors: u64,
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
    /// With [`crate::NodeConfig::stats_bins`], the counters at each bin
    /// edge passed since the last snapshot; empty otherwise.
    pub bins: Vec<NodeBin>,
    /// With [`crate::NodeConfig::forecast_log`], the ppspp runner's
    /// forecast log since the last snapshot (`Runner::take_forecasts`), and
    /// the samples its caps left out; empty otherwise.
    pub forecasts: Vec<ForecastInfo>,
    pub forecasts_lost: u64,
    /// With [`crate::NodeConfig::forecast_log`] or
    /// [`crate::NodeConfig::request_log`], the chunks the runner stored
    /// since the last snapshot (`Runner::take_held`), and those its cap
    /// left out.
    pub held: Vec<HeldInfo>,
    pub held_lost: u64,
    /// With [`crate::NodeConfig::request_log`], the chunks HAVEs announced
    /// (`Runner::take_haves`) and the requests that ended
    /// (`Runner::take_requests`) since the last snapshot, and those the
    /// caps left out; empty otherwise.
    pub haves: Vec<HaveInfo>,
    pub haves_lost: u64,
    pub requests: Vec<RequestInfo>,
    pub requests_lost: u64,
    /// With [`crate::NodeConfig::request_log`], the seed log of each swarm
    /// the node seeds since the last snapshot (`Runner::take_seed_log`):
    /// its decisions about its neighbours, what became of each chunk it
    /// pushed or answered, and the records its caps left out.
    pub seed_decisions: Vec<LogInfo<SeedDecision>>,
    pub seed_sends: Vec<LogInfo<SeedSend>>,
    pub seed_log_lost: u64,
    /// With [`crate::NodeConfig::request_log`], the probes its links' path
    /// meters let go since the last snapshot (`strims_vnic::Host::take_probes`).
    pub path_probes: Vec<PathProbeInfo>,
}

/// A probe of a link's path meter ([`NodeMetrics::path_probes`]): the
/// link's peer, by host, and the probe's record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathProbeInfo {
    pub peer: ID,
    pub record: strims_base::path::ProbeRecord,
}

/// A record of the ppspp runner's seed log ([`NodeMetrics::seed_decisions`],
/// [`NodeMetrics::seed_sends`]), its peer named by host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogInfo<T> {
    pub swarm: SwarmKey,
    pub peer: Option<ID>,
    pub record: T,
}

/// A chunk the node first held ([`NodeMetrics::held`]): its swarm, its
/// bin, and when, by the node's clock, with its timeline here
/// (`strims_ppspp::Held`) and its source peer, the stream's parent then
/// and the peer last asked for it, by host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldInfo {
    pub swarm: SwarmKey,
    pub bin: Bin,
    pub at: Time,
    pub held: strims_ppspp::Held,
    pub peer: Option<ID>,
    pub parent: Option<ID>,
    pub asked: Option<ID>,
}

/// A chunk a HAVE announced ([`NodeMetrics::haves`]), its peer by host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HaveInfo {
    pub swarm: SwarmKey,
    pub peer: Option<ID>,
    pub have: HaveRecord,
}

/// A request the request round made ([`NodeMetrics::requests`]), its
/// asked peer and its rivals by host, in the record's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestInfo {
    pub swarm: SwarmKey,
    pub peer: Option<ID>,
    pub rivals: Vec<Option<ID>>,
    pub record: RequestRecord,
}

/// The node's counters at one edge of its stats bins
/// ([`crate::NodeConfig::stats_bins`]): read with getters only, before the
/// first pass past the edge, so they stand as they did at the edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeBin {
    /// The edge, by the node's clock.
    pub at: Time,
    /// Each swarm with a scheduler, in key order.
    pub swarms: Vec<SwarmBin>,
    /// Every channel's outbound message counters so far
    /// ([`RunnerMetrics::totals`]).
    pub outbound: ChannelMetrics,
    /// Bytes the driver wrote to its links so far
    /// ([`DriverMetrics::bytes_written`]).
    pub bytes_written: u64,
}

/// A swarm's counters at a bin edge ([`NodeBin`]): ppspp's `PushStats`'
/// pushes before their deadline (`hits`), deadlines released to pull
/// (`misses`), parents pruned, replaced (`switched`) and lost to their
/// closing the subscription (`shed`), subscribers closed for a saturated
/// uplink (`sheds_sent`), streams with an open parent, the upload meter's
/// capacity (the relay busy-time reading, `PushStats::upload_capacity`)
/// and busy thousandths, a seed's copies and the capacity its budget was
/// measured from (`PushStats::seed_budget_capacity`, zero elsewhere), and
/// the scheduler's counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwarmBin {
    pub swarm: SwarmKey,
    pub hits: u64,
    pub misses: u64,
    pub pruned: u64,
    pub switched: u64,
    pub shed: u64,
    pub sheds_sent: u64,
    pub parents: u64,
    pub upload_capacity: u64,
    pub upload_busy_permille: u64,
    pub seed_copies: u64,
    pub seed_budget_capacity: u64,
    pub counters: SwarmCounters,
}

/// One sample of the ppspp runner's forecast log ([`NodeMetrics::forecasts`]),
/// its peer named by host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForecastInfo {
    pub swarm: SwarmKey,
    pub peer: Option<ID>,
    pub forecast: LagForecast,
}

/// A change of a hybrid stream's open parent
/// ([`crate::NodeConfig::parent_log`], ppspp's `ParentChange`): when, by
/// the node's clock, and the stream's parent from then on, by host
/// (`None` for none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentInfo {
    pub at: Time,
    pub swarm: SwarmKey,
    pub stream: Stream,
    pub parent: Option<ID>,
    pub why: ParentWhy,
}

/// What a node's parent log hands its sink
/// ([`crate::NodeConfig::parent_log`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentLogEntry {
    /// A change of a hybrid stream's open parent.
    Change(ParentInfo),
    /// Changes the runner's log caps (ppspp's `PARENT_LOG_CAP`) left out
    /// since the last entry: what the sink lacks of the log.
    Lost(u64),
}

/// Whose timer a [`TimerRecord`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimerSource {
    /// The node task's one timer ([`crate::NodeIo::reset_sleep`]), which
    /// wakes the pass that runs its state machines' deadlines.
    NodeTask,
    /// A harness's sampling timer: the testbed node's line a second.
    Record,
    /// A stream writer's: each write waits for the time it is due.
    Writer,
    /// A browser driver's host pacer, which reads its channels' load again
    /// while it holds a write.
    Pacer,
    /// A harness's scripted timer of a fixed delay.
    Probe,
}

impl TimerSource {
    /// Every source, in order.
    pub const ALL: [TimerSource; 5] = [
        TimerSource::NodeTask,
        TimerSource::Record,
        TimerSource::Writer,
        TimerSource::Pacer,
        TimerSource::Probe,
    ];

    /// Its name in the tables: `node_task`, `record`, `writer`, `pacer`,
    /// `probe`.
    pub fn name(self) -> &'static str {
        match self {
            TimerSource::NodeTask => "node_task",
            TimerSource::Record => "record",
            TimerSource::Writer => "writer",
            TimerSource::Pacer => "pacer",
            TimerSource::Probe => "probe",
        }
    }

    /// The source named `name` ([`TimerSource::name`]).
    pub fn parse(name: &str) -> Option<TimerSource> {
        TimerSource::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// A timer that fired ([`crate::NodeConfig::timer_log`]). Every time is by
/// the clock of the code that armed it at that clock's finest precision
/// ([`crate::NodeIo::precise_now`]), but `deadline`, which is the time the
/// timer was armed for, as that code reckons time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerRecord {
    pub source: TimerSource,
    /// When it was armed for `deadline`.
    pub armed: Time,
    pub deadline: Time,
    /// When the code it woke ran.
    pub fired: Time,
    /// The delay it was armed for: `deadline` less the arming code's own
    /// reading of the time when it armed it.
    pub requested: Duration,
}

/// What a link's I/O did ([`crate::NodeConfig::link_log`]), as the code
/// that did it saw it: the node task, the driver's reader and writer, or
/// the transport under them. The sink stamps each with the time it is
/// handed over, so a record's time is when its code ran, but one that
/// carries its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkRecord {
    /// The link: its key's ffi value ([`slotmap::KeyData::as_ffi`]), which
    /// no other link of the node has had.
    pub link: u64,
    /// When it happened, by the sink's clock, from code the sink can't
    /// time as it runs (a simulation's network, outside its hosts).
    pub at: Option<Time>,
    pub event: LinkEvent,
}

/// See [`LinkRecord`]. A link's writes go through its writer in the order
/// the node queued them, and each write's events (begun, held, checked,
/// done) come in turn; a read is taken in the order it was read. Bytes
/// count from the link's first byte in each direction: the first its vnic
/// wrote, and the first its vnic read (a transport's own handshake left
/// out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkEvent {
    /// The node attached the link, over this transport.
    Opened(LinkKind),
    /// vnic named the link's peer.
    Peer(ID),
    /// The node handed the link's writer a buffer of this many bytes.
    Queued(usize),
    /// The writer began writing the next buffer queued, of this many
    /// bytes.
    Began(usize),
    /// The transport held the write in progress for this.
    Held(WriteWait),
    /// The writer read the transport's buffered bytes (`source`'s) as it
    /// decided whether the write may go now: `buffered`, and the mark it
    /// then waits for if it holds the write for its own cap.
    Checked {
        source: SampleSource,
        buffered: u64,
        low: Option<u64>,
    },
    /// A held writer's transport woke it: its buffered bytes fell to the
    /// mark it waited for, read as it woke.
    Low { source: SampleSource, buffered: u64 },
    /// The transport took the write's last byte.
    Done,
    /// A message of this many bytes arrived at a transport that delivers
    /// messages (a browser's data channel), before the reader took it.
    Arrived(usize),
    /// The reader returned a read of this many bytes.
    Read(usize),
    /// The node took a read of this many bytes from the reader.
    Taken(usize),
    /// The node read the link's transport or its own queues for it.
    Sample {
        source: SampleSource,
        state: LinkState,
    },
    /// The node dropped the link.
    Closed,
}

/// What a transport held a write for ([`LinkEvent::Held`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WriteWait {
    /// A TCP socket's unsent bytes at its `TCP_NOTSENT_LOWAT`, or its full
    /// send buffer.
    Notsent,
    /// A data channel's buffered amount at its own pacer's cap.
    Channel,
    /// A data channel's host pacer: the host's channels' load at its cap.
    Host,
}

impl WriteWait {
    /// Its name in the tables: `notsent`, `channel`, `host`.
    pub fn name(self) -> &'static str {
        match self {
            WriteWait::Notsent => "notsent",
            WriteWait::Channel => "channel",
            WriteWait::Host => "host",
        }
    }

    /// The wait named `name` ([`WriteWait::name`]).
    pub fn parse(name: &str) -> Option<WriteWait> {
        [WriteWait::Notsent, WriteWait::Channel, WriteWait::Host]
            .into_iter()
            .find(|w| w.name() == name)
    }
}

/// What a [`LinkEvent::Sample`] read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SampleSource {
    /// The kernel's TCP socket under a TCP link.
    Tcp,
    /// A WebSocket's: the kernel's socket, or a browser's `bufferedAmount`.
    Ws,
    /// A data channel's buffered amount.
    WebRtc,
    /// A simulation's model of the link's connection.
    Model,
    /// The node's own queues for the link.
    Queues,
}

impl SampleSource {
    /// Every source, in order.
    pub const ALL: [SampleSource; 5] = [
        SampleSource::Tcp,
        SampleSource::Ws,
        SampleSource::WebRtc,
        SampleSource::Model,
        SampleSource::Queues,
    ];

    /// Its name in the tables: `tcp`, `ws`, `webrtc`, `model`, `queues`.
    pub fn name(self) -> &'static str {
        match self {
            SampleSource::Tcp => "tcp",
            SampleSource::Ws => "ws",
            SampleSource::WebRtc => "webrtc",
            SampleSource::Model => "model",
            SampleSource::Queues => "queues",
        }
    }

    /// The source named `name` ([`SampleSource::name`]).
    pub fn parse(name: &str) -> Option<SampleSource> {
        SampleSource::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// A link's state as one reading of it gives it ([`LinkEvent::Sample`]):
/// what its source can tell, `None` for the rest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkState {
    /// Bytes written and not yet sent.
    pub unsent: Option<u64>,
    /// Bytes sent and not yet acknowledged.
    pub unacked: Option<u64>,
    pub cwnd: Option<u64>,
    pub ssthresh: Option<u64>,
    pub srtt: Option<Duration>,
    pub rttvar: Option<Duration>,
    /// Segments retransmitted so far.
    pub retrans: Option<u64>,
    /// Bytes a second.
    pub delivery_rate: Option<u64>,
    /// A channel's or a browser WebSocket's buffered amount.
    pub buffered: Option<u64>,
    /// A socket's `TCP_NOTSENT_LOWAT`.
    pub lowat: Option<u64>,
    /// The host's uplink queue, as the time it takes to drain.
    pub uplink_queue: Option<Duration>,
    /// vnic's link buffer's pending bytes.
    pub link_pending: Option<u64>,
    /// The driver's write buffers the transport's writer holds.
    pub write_held: Option<u64>,
    /// The length of the next frame of the link's ppspp writer's QoS
    /// session.
    pub qos_backlog: Option<u64>,
}

impl LinkState {
    /// A capture sample's reading, and what it read; `None` for a sample
    /// that reads no link ([`crate::CaptureSample::Path`],
    /// [`crate::CaptureSample::SimUplink`]).
    pub fn of(sample: &crate::CaptureSample) -> Option<(SampleSource, LinkState)> {
        use crate::CaptureSample as S;
        let s = match *sample {
            S::Queues {
                qos_backlog,
                link_pending,
                write_held,
                ..
            } => (
                SampleSource::Queues,
                LinkState {
                    link_pending: Some(link_pending),
                    write_held: Some(write_held),
                    qos_backlog,
                    ..LinkState::default()
                },
            ),
            S::Tcp {
                ws,
                unsent,
                unacked,
                cwnd,
                srtt,
                rttvar,
                retrans,
                delivery_rate,
            } => (
                if ws {
                    SampleSource::Ws
                } else {
                    SampleSource::Tcp
                },
                LinkState {
                    unsent: Some(unsent),
                    unacked: Some(unacked),
                    cwnd: Some(cwnd),
                    srtt: Some(srtt),
                    rttvar: Some(rttvar),
                    retrans: Some(retrans),
                    delivery_rate: Some(delivery_rate),
                    ..LinkState::default()
                },
            ),
            S::WebRtc { buffered } => (
                SampleSource::WebRtc,
                LinkState {
                    buffered: Some(buffered),
                    ..LinkState::default()
                },
            ),
            S::WebSocket { buffered } => (
                SampleSource::Ws,
                LinkState {
                    buffered: Some(buffered),
                    ..LinkState::default()
                },
            ),
            S::SimConn {
                unsent,
                lowat,
                cwnd,
                ssthresh,
                in_flight,
                uplink_queue,
                ..
            } => (
                SampleSource::Model,
                LinkState {
                    unsent: Some(unsent),
                    unacked: in_flight,
                    cwnd,
                    ssthresh,
                    lowat,
                    uplink_queue,
                    ..LinkState::default()
                },
            ),
            S::SimUplink { .. } | S::Path { .. } => return None,
        };
        Some(s)
    }
}

/// An event of the ppspp runner's tap ([`crate::NodeConfig::decision_log`],
/// ppspp's `TapEvent`): when, by the node's clock (the pass that made it),
/// its swarm, the peer it is about by host (`None` for none, or one the
/// node can't name), and what happened. The peer keys inside `kind` (a
/// `ParentChoice`'s candidates and parent, a `Switched`'s `from`) are the
/// runner's own: `peer` is named by host, and so are a `ParentChoice`'s
/// runner-up and parent, for a log's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionInfo {
    pub at: Time,
    pub swarm: Option<SwarmKey>,
    pub peer: Option<ID>,
    /// A `ParentChoice`'s runner-up by host; `None` for none, another
    /// kind, or one the node can't name.
    pub runner_up: Option<ID>,
    /// A `ParentChoice`'s parent by host, as `runner_up`.
    pub parent: Option<ID>,
    pub kind: TapKind,
}

/// What a node's decision log hands its sink
/// ([`crate::NodeConfig::decision_log`]).
// An entry goes to the sink by value, one at a time: boxing the large
// variant would allocate for every event.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionLogEntry {
    /// An event of the tap.
    Decision(DecisionInfo),
    /// Events the tap's buffers (ppspp's `TAP_CAP`) left out since the
    /// last entry: what the sink lacks of the tap.
    Lost(u64),
}

/// The ppspp runner's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerMetrics {
    /// Every swarm the node has (created or added), in key order.
    pub swarms: Vec<SwarmInfo>,
    /// Every connected peer, in vnic link order.
    pub peers: Vec<RunnerPeerInfo>,
    /// Every channel's message counters since the runner started, closed
    /// channels' included.
    pub totals: ChannelTotals,
}

/// One swarm: its store, its scheduler and its local handles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmInfo {
    pub swarm: SwarmKey,
    pub id: SwarmId,
    /// The swarm's `SwarmOptions::label`.
    pub label: String,
    /// Peers running a channel for it.
    pub channels: usize,
    pub store: StoreInfo,
    /// The scheduler's state (ppspp `Runner::swarm_stats`); `None` while
    /// the swarm has no channel, as the scheduler exists only then.
    pub scheduler: Option<SwarmSchedulerStats>,
    /// Push/pull state and counters (ppspp `Runner::push_stats`); `None`
    /// while the swarm has no channel.
    pub push: Option<PushStats>,
    /// Per stream, the host of its open parent (`PushStats::parents`).
    pub parent_hosts: Vec<Option<ID>>,
    /// A hybrid seed's view of its neighbours and its allocation (ppspp
    /// `Runner::seed_allocation`); `None` elsewhere.
    pub seed_allocation: Option<SeedAllocationInfo>,
    /// Open [`crate::SwarmReader`]s, and those waiting for data.
    pub readers: usize,
    pub readers_waiting: usize,
    /// Open [`crate::SwarmWriter`]s (0 or 1).
    pub publishers: usize,
}

/// A hybrid seed's view of a swarm (ppspp `SeedAllocationStats`): its
/// neighbours, and an adaptive seed's latest check (zero otherwise): the
/// budget in stream slots (zero until measured), the window and the
/// copies in effect, and its counters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeedAllocationInfo {
    /// The neighbours the node can name by host, in the runner's order.
    pub neighbours: Vec<SeedNeighbourInfo>,
    pub budget: usize,
    pub window: usize,
    pub copies: usize,
    pub counters: AllocCounters,
}

/// A hybrid seed's neighbour, by host (ppspp `SeedNeighbour`): the streams
/// pushed to it; per stream, the mean one-way delay of its first copies
/// lately as its reports give them, by its clock (`None` without any); the
/// channel's round trip and its smallest link round trip lately; whether
/// it holds a copy, and whether the reports barred it from holding one; at
/// an adaptive seed's latest check, the reporters that voted on it,
/// whether most saw it serve (`None`: too few to tell), the median of
/// their hits' mean one-way delay (each by its reporter's clock) and
/// whether the seed's pushes to it kept up; and what became of its own
/// report entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedNeighbourInfo {
    pub host_id: ID,
    pub streams: Vec<Stream>,
    pub lags: Vec<Option<Duration>>,
    pub rtt: Duration,
    pub min_rtt: Option<Duration>,
    pub holder: bool,
    pub barred: bool,
    pub reporters: usize,
    pub served: Option<bool>,
    pub report_delay: Option<Duration>,
    pub kept_up: bool,
    pub reports: ReportCounters,
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
    /// Of the bytes read, those that verified and were new to us
    /// (ppspp `Runner::peer_useful`).
    pub useful: u64,
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
