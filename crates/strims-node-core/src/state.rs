// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The sans-IO composition of vnic, vpn and the ppspp runner, plus the
//! driver's bookkeeping: links, dials, peer exchanges, swarm readers and
//! publishers.
//!
//! Every vnic peer gets a runner peer and a writer on
//! [`ports::TRANSFER_PORT`] in the transfer QoS class, an RPC channel pair
//! on [`ports::PEER_RPC_CLIENT_PORT`] and [`ports::PEER_RPC_SERVER_PORT`]
//! in the peer RPC class (see `rpc`), and is bound to every network with
//! an `auto_bind_port`. vnic frames and send opportunities reach their
//! owner by [`Tag`].
//!
//! [`State`] owns no sockets and spawns nothing. The driver feeds it link
//! bytes, commands and the time; it fills the link transmit buffers the
//! driver hands to the link tasks, and queues the dials the driver starts.
//!
//! The egress pipeline ([`State::pump`]) follows the core crates' driver
//! contract: drain events, `vpn.flush`, `vnic.poll_egress` with
//! fill callbacks routed by owner [`Tag`], then move link transmit bytes to
//! the link tasks' recycled write buffers, or into in-process WebRTC data
//! channels.
//!
//! The WebRTC interface (`rtc`, when enabled) is a fourth state machine
//! here: the node's [`RtcEndpoint`](crate::rtc::RtcEndpoint) with every
//! peer connection, fed driver input and timers, its output flushed and
//! its events drained with the others.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::task::Poll;

use rand_core::SeedableRng;
use slotmap::{SecondaryMap, SlotMap};
use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::Time;
use strims_kademlia::ID;
use strims_ppspp::store::{self, BufferReader};
use strims_ppspp::{Runner, RunnerError, SwarmId, SwarmKey, SwarmOptions, WriterOptions};
use strims_proto::types::{Cache, Key};
use strims_qos::ClassId;
use strims_vnic::{
    CandidateError, CloseReason, DialTarget, Fill, HostConfig, InboundFrame, LinkCandidatePool,
    LinkDescription, LinkInterface, LinkKind, PeerInitError, PeerKey as LinkKey, WriterId, ports,
};
use strims_vpn::{
    Event as VpnEvent, ExchangeId, HashTableGetId, HashTableGetOptions, HashTablePublisherId,
    NetworkId, PeerIndexHost, PeerIndexPublisherId, PeerIndexSearchId,
};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};

use crate::config::{NetworkOptions, NodeConfig, PeerRpcConfig, VpnPortConfig};
use crate::driver::{IdleOk, Interfaces, TaskGuard};
use crate::error::{NodeError, ReadError};
use crate::event::{
    ChannelInfo, DriverMetrics, LinkInfo, NetworkInfo, NodeEvent, NodeMetrics, PeerInfo,
    RunnerMetrics, RunnerPeerInfo, StoreInfo, SwarmInfo, WriterInfo,
};
use crate::exchange::{Candidate, CandidateState};
use crate::rpc::Outgoing;
use crate::rtc::{RtcConn, RtcFactory};
use crate::vpn_port::VpnFrame;

mod rpc;
mod rtc;
mod vpn_port;

pub(crate) use rpc::{RpcChannelId, RpcSide};

type PpsppPeer = strims_ppspp::PeerKey;
pub(crate) type VnicHost = strims_vnic::Host<Tag>;
pub(crate) type VpnHost = strims_vpn::Host<u16>;

/// Rounds of the egress pipeline per [`State::pump`] before the driver
/// yields. Each round either moves bytes to a link task or handles events;
/// everything else waits on I/O or a timer.
const PUMP_ROUNDS: usize = 8;

/// The weight of the transfer (ppspp) QoS class under vnic's root.
const TRANSFER_QOS_WEIGHT: u64 = 1;

slotmap::new_key_type! {
    /// A dial in progress.
    pub struct DialId;
    /// A swarm reader handle.
    pub(crate) struct ReaderId;
    /// A swarm publisher (ppspp `Writer`) handle.
    pub(crate) struct PublisherId;
}

/// vnic's routing tag: which owner a frame or a send opportunity is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tag {
    /// vpn frames for a network (the port pair bound for the peer).
    Vpn(NetworkId),
    /// ppspp frames on `TRANSFER_PORT`, for the runner's peer.
    Ppspp(PpsppPeer),
    /// A channel of the peer's RPC pair.
    Rpc(LinkKey, RpcSide),
}

/// The driver's half of a link: the channels to its I/O tasks and its
/// recycled write buffers.
#[derive(Debug)]
pub(crate) struct LinkIo {
    /// Filled write buffers to the writer task (at most `free_write`'s
    /// initial count in flight).
    pub(crate) write_tx: mpsc::Sender<Vec<u8>>,
    /// Write buffers back from the writer task.
    pub(crate) free_write: Vec<Vec<u8>>,
    /// The most bytes per write: the link MTU, so a message transport
    /// sends at most one MTU per message.
    pub(crate) max_write: usize,
    /// Read buffers back to the reader task.
    pub(crate) read_free_tx: mpsc::Sender<Vec<u8>>,
    /// Aborted when the link goes; the writer ends when `write_tx` drops.
    pub(crate) reader: TaskGuard,
    /// Lets the reader outwait its read timeout while the peer is
    /// established and hasn't sent a keepalive (see [`IdleOk`]).
    pub(crate) idle_ok: Option<IdleOk>,
}

impl Drop for LinkIo {
    /// The reader is aborted before the writer's channel closes.
    fn drop(&mut self) {
        self.reader.abort();
    }
}

/// A link's transport, as the node task sees it.
#[derive(Debug)]
enum LinkTransport {
    /// TCP, WebSocket, or a data channel the driver owns: I/O tasks and
    /// recycled buffers.
    Stream(LinkIo),
    /// An in-process data channel of the endpoint in [`State`].
    DataChannel(rtc::DataChannel),
}

impl LinkTransport {
    fn stream(&self) -> Option<&LinkIo> {
        match self {
            LinkTransport::Stream(io) => Some(io),
            LinkTransport::DataChannel(_) => None,
        }
    }
}

#[derive(Debug)]
struct LinkSlot {
    io: LinkTransport,
    kind: LinkKind,
    /// The dial that made the link; `None` for accepted and WebRTC links.
    dial: Option<DialId>,
    /// This node made the link: it dialed, or offered the WebRTC session.
    dialed: bool,
    /// A WebRTC link's connection.
    rtc: Option<RtcConn>,
}

/// Why a dial failed. A failed vnic handshake stops a candidate pool
/// (`CandidateError::PeerInit`).
#[derive(Clone, Debug)]
enum DialError {
    Connect(String),
    PeerInit(String),
    /// The link reached this node.
    OwnHost,
}

impl DialError {
    fn message(&self) -> &str {
        match self {
            DialError::Connect(m) | DialError::PeerInit(m) => m,
            DialError::OwnHost => "the peer is this node",
        }
    }

    fn into_node_error(self) -> NodeError {
        match self {
            DialError::OwnHost => NodeError::OwnHost,
            e => NodeError::Dial(e.message().to_owned()),
        }
    }
}

#[derive(Debug)]
struct Dial {
    target: DialTarget,
    waiter: Option<oneshot::Sender<Result<ID, NodeError>>>,
    /// The exchange and candidate index this dial is for.
    exchange: Option<(ExchangeId, usize)>,
}

/// A swarm's channel ids with one peer: ours (on the peer's frames) and
/// the peer's (on ours).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChannelIds {
    pub(crate) channel: u64,
    pub(crate) peer_channel: u64,
}

#[derive(Debug)]
struct PeerState {
    host_id: ID,
    pp: PpsppPeer,
    /// The `TRANSFER_PORT` channel's writer.
    transfer: Option<WriterId>,
    /// Swarms with a channel to this peer (bound: one per swarm).
    swarms: BTreeMap<SwarmKey, ChannelIds>,
    /// The RPC channel pair; `None` when peer RPC is off, or once the pair
    /// has closed.
    rpc: Option<rpc::PeerRpcState>,
}

#[derive(Debug)]
struct Exchange {
    pool: LinkCandidatePool<Candidate>,
    /// vpn asked for our descriptions and hasn't had them yet (a WebRTC
    /// candidate may still be gathering).
    gathering: bool,
}

/// A finished read, back to a [`crate::SwarmReader`].
#[derive(Debug)]
pub(crate) struct ReadReply {
    pub(crate) buf: Vec<u8>,
    /// The stream offset of the first byte read, and the bytes read.
    pub(crate) result: Result<(u64, usize), ReadError>,
}

#[derive(Debug)]
struct ReaderSlot {
    swarm: SwarmKey,
    reader: BufferReader,
    /// A read waiting for data: the buffer and the most bytes to read.
    pending: Option<(Vec<u8>, usize)>,
    reply: mpsc::Sender<ReadReply>,
}

/// What a [`crate::SwarmWriter`] asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteOp {
    /// Publish the buffer's first `n` bytes.
    Write(usize),
    Flush,
    Reset,
}

#[derive(Debug)]
pub(crate) struct WriteReply {
    pub(crate) buf: Vec<u8>,
    pub(crate) result: Result<(), NodeError>,
}

#[derive(Debug)]
struct PublisherSlot {
    swarm: SwarmKey,
    writer: strims_ppspp::Writer,
    reply: mpsc::Sender<WriteReply>,
}

/// vnic, vpn and the ppspp runner, composed.
#[derive(Debug)]
pub(crate) struct State {
    pub(crate) vnic: VnicHost,
    pub(crate) vpn: VpnHost,
    pub(crate) runner: Runner,
    transfer_class: ClassId,
    rpc_class: ClassId,
    /// The QoS class of the peers' broker channels.
    broker_class: ClassId,
    peer_rpc: PeerRpcConfig,
    /// Cloned into every RPC writer and vpn port.
    rpc_tx: mpsc::UnboundedSender<Outgoing>,
    /// RPC pairs to close after vnic's dispatch, and whether they
    /// overflowed. At most one entry per peer.
    rpc_closed: Vec<(LinkKey, RpcSide, bool)>,
    /// Listener URIs advertised to peer exchange.
    local_tcp: Option<LinkDescription>,
    local_ws: Option<LinkDescription>,
    /// The link interfaces the driver has.
    interfaces: Interfaces,
    allow_insecure_ws: bool,
    /// Bounded by the transports the driver attaches.
    links: SecondaryMap<LinkKey, LinkSlot>,
    /// Established peers (one per link).
    peers: SecondaryMap<LinkKey, PeerState>,
    ppspp_peers: SecondaryMap<PpsppPeer, LinkKey>,
    networks: SecondaryMap<NetworkId, NetworkOptions>,
    /// Ports opened with `open_vpn_port` (bound: the app's calls).
    vpn_ports: BTreeMap<(NetworkId, u16), vpn_port::PortState>,
    vpn_ports_config: VpnPortConfig,
    next_port_serial: u64,
    /// Messages from vpn port writers for the next pump. Bounded by the
    /// writers' credit.
    vpn_sends: VecDeque<VpnFrame>,
    /// Open hash table gets and peer index searches, by the app's calls.
    gets: BTreeMap<HashTableGetId, mpsc::Sender<Vec<u8>>>,
    searches: BTreeMap<PeerIndexSearchId, mpsc::Sender<PeerIndexHost>>,
    /// One per vpn exchange in progress (bounded by vpn).
    exchanges: BTreeMap<ExchangeId, Exchange>,
    /// Dials in progress: the app's and the exchanges'.
    dials: SlotMap<DialId, Dial>,
    /// Dials for the driver to start, in order.
    dial_queue: VecDeque<DialId>,
    /// Every swarm in the runner (bound: the app's calls).
    swarms: BTreeSet<SwarmKey>,
    readers: SlotMap<ReaderId, ReaderSlot>,
    publishers: SlotMap<PublisherId, PublisherSlot>,
    events: mpsc::Sender<NodeEvent>,
    pub(crate) metrics: DriverMetrics,
    /// Reused by `drain_transmit`.
    scratch_links: Vec<LinkKey>,
    /// ppspp frame errors seen inside vnic's dispatch, handled after it.
    frame_errors: Vec<(PpsppPeer, RunnerError)>,
    /// The WebRTC interface, when enabled.
    rtc: Option<rtc::Rtc>,
    /// Set while the endpoint's events are drained, so handlers that drain
    /// events themselves (a data-channel read) don't recurse into it.
    rtc_draining: bool,
}

impl State {
    /// Builds the hosts. Rngs are forked from `config.rng_seed` in a fixed
    /// order (vnic, vpn, runner), so a seed fixes every key and nonce.
    /// The WebRTC endpoint's rng is forked last, so enabling WebRTC
    /// changes no other key.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: &NodeConfig,
        local_tcp: Option<LinkDescription>,
        local_ws: Option<LinkDescription>,
        interfaces: Interfaces,
        webrtc: Option<RtcFactory>,
        now: Time,
        events: mpsc::Sender<NodeEvent>,
        rpc_tx: mpsc::UnboundedSender<Outgoing>,
    ) -> Result<State, NodeError> {
        let mut rng = det_rng(config.rng_seed);
        let vnic_rng = fork(&mut rng);
        let vnic_rng = config.host_key_seed.map_or(vnic_rng, DetRng::from_seed);
        let mut vnic = VnicHost::new(
            config.profile_key.clone(),
            HostConfig {
                label: config.label.clone(),
                rate_limit: config.rate_limit,
                node_platform: config.node_platform.clone(),
                node_version: config.node_version.clone(),
            },
            vnic_rng,
        )?;
        vnic.set_max_peers(config.max_peers);
        let vpn = VpnHost::new(&mut vnic, config.vpn.clone(), fork(&mut rng))?;
        let transfer_class = vnic.add_qos_class(vnic.qos_root(), TRANSFER_QOS_WEIGHT)?;
        let rpc_class = vnic.add_qos_class(vnic.qos_root(), rpc::PEER_RPC_QOS_WEIGHT)?;
        let broker_class = vnic.add_qos_class(vnic.qos_root(), rpc::BROKER_QOS_WEIGHT)?;
        let runner = Runner::new(fork(&mut rng));
        let rtc = webrtc.map(|factory| {
            rtc::Rtc::new(
                factory(now, fork(&mut rng)),
                config.webrtc.max_buffered_amount.max(1),
            )
        });
        Ok(State {
            vnic,
            vpn,
            runner,
            transfer_class,
            rpc_class,
            broker_class,
            peer_rpc: config.peer_rpc,
            rpc_tx,
            rpc_closed: Vec::new(),
            local_tcp,
            local_ws,
            interfaces,
            allow_insecure_ws: config.allow_insecure_ws,
            links: SecondaryMap::new(),
            peers: SecondaryMap::new(),
            ppspp_peers: SecondaryMap::new(),
            networks: SecondaryMap::new(),
            vpn_ports: BTreeMap::new(),
            vpn_ports_config: config.vpn_ports,
            next_port_serial: 0,
            vpn_sends: VecDeque::new(),
            gets: BTreeMap::new(),
            searches: BTreeMap::new(),
            exchanges: BTreeMap::new(),
            dials: SlotMap::with_key(),
            dial_queue: VecDeque::new(),
            swarms: BTreeSet::new(),
            readers: SlotMap::with_key(),
            publishers: SlotMap::with_key(),
            events,
            metrics: DriverMetrics::default(),
            scratch_links: Vec::new(),
            frame_errors: Vec::new(),
            rtc,
            rtc_draining: false,
        })
    }

    fn emit(&mut self, e: NodeEvent) {
        if self.events.try_send(e).is_err() {
            self.metrics.events_dropped += 1;
        }
    }

    // --- timers -----------------------------------------------------------

    /// The earliest deadline of the state machines.
    pub(crate) fn poll_timeout(&self) -> Option<Time> {
        [
            self.vnic.poll_timeout(),
            self.vpn.poll_timeout(),
            self.runner.poll_timeout(),
            self.rtc.as_ref().and_then(|r| r.ep.poll_timeout()),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Runs the timers that are due. vnic's deadline (the QoS rate limit) is
    /// handled by the next [`State::pump`].
    pub(crate) fn handle_timeouts(&mut self, now: Time) {
        if self.vpn.poll_timeout().is_some_and(|t| t <= now) {
            self.vpn.handle_timeout(now);
        }
        if self.runner.poll_timeout().is_some_and(|t| t <= now) {
            self.runner.handle_timeout(now);
        }
        if let Some(rtc) = &mut self.rtc
            && rtc.ep.poll_timeout().is_some_and(|t| t <= now)
        {
            rtc.ep.handle_timeout(now);
        }
    }

    // --- links --------------------------------------------------------------

    /// Registers the I/O of a link the driver just added to vnic: one it
    /// dialed or accepted, or the data channel of WebRTC connection `rtc`.
    pub(crate) fn insert_link(
        &mut self,
        link: LinkKey,
        kind: LinkKind,
        io: LinkIo,
        dial: Option<DialId>,
        rtc: Option<RtcConn>,
    ) {
        let dialed = match rtc {
            Some(conn) => self.rtc_stream_linked(conn, link),
            None => {
                if dial.is_none() {
                    self.metrics.accepted += 1;
                }
                dial.is_some()
            }
        };
        self.links.insert(
            link,
            LinkSlot {
                io: LinkTransport::Stream(io),
                kind,
                dial,
                dialed,
                rtc,
            },
        );
    }

    /// Feeds bytes read from a link to vnic, which dispatches frames to vpn
    /// and the runner. Events are handled after every vnic call, so owners
    /// register their handlers on `PeerAdded` before the frames that follow
    /// the handshake in the same read.
    pub(crate) fn handle_read(&mut self, link: LinkKey, now: Time, data: &mut [u8]) {
        if !self.links.contains_key(link) {
            return;
        }
        self.metrics.bytes_read += data.len() as u64;
        let mut off = 0;
        while off < data.len() {
            let Some(rest) = data.get_mut(off..) else {
                break;
            };
            let State {
                vnic,
                vpn,
                runner,
                frame_errors,
                metrics,
                peers,
                rpc_closed,
                peer_rpc,
                ..
            } = self;
            let res =
                vnic.handle_link_input(link, now, rest, &mut |f: InboundFrame<'_, Tag>| match f
                    .handler
                {
                    Tag::Vpn(net) => {
                        if let Err(e) = vpn.handle_frame(now, net, f.peer, f.body) {
                            metrics.vpn_frame_errors += 1;
                            tracing::trace!(error = %e, "vpn frame");
                        }
                    }
                    Tag::Ppspp(pp) => {
                        if let Err(e) = runner.handle_frame(pp, now, f.body)
                            && !frame_errors.iter().any(|(p, _)| *p == pp)
                        {
                            frame_errors.push((pp, e));
                        }
                    }
                    Tag::Rpc(peer, side) => rpc::dispatch(
                        peers,
                        rpc_closed,
                        if side == RpcSide::Broker {
                            &mut metrics.broker_bytes_read
                        } else {
                            &mut metrics.rpc_bytes_read
                        },
                        peer_rpc.inbound_buffer,
                        peer,
                        side,
                        f.body,
                    ),
                });
            let n = match res {
                Ok(n) => n,
                Err(e) => {
                    // vnic closed the link and queued `LinkClosed`.
                    tracing::debug!(error = %e, "link handshake failed");
                    0
                }
            };
            self.handle_frame_errors();
            self.handle_rpc_closed();
            self.process_events(now);
            if n == 0 {
                break;
            }
            off += n;
        }
        // A peer that sends keepalives gets the read timeout back.
        if let Some(flag) = self
            .links
            .get(link)
            .and_then(|s| s.io.stream())
            .and_then(|io| io.idle_ok.as_ref())
            && flag.get()
            && self.vnic.peer(link).is_some_and(|p| p.sends_keepalives())
        {
            flag.set(false);
        }
    }

    pub(crate) fn return_read_buf(&mut self, link: LinkKey, buf: Vec<u8>) {
        if let Some(io) = self.links.get(link).and_then(|s| s.io.stream()) {
            // At most `read_buffers` exist, so the channel has room.
            let _ = io.read_free_tx.try_send(buf);
        }
    }

    pub(crate) fn return_write_buf(&mut self, link: LinkKey, buf: Vec<u8>) {
        if let Some(LinkSlot {
            io: LinkTransport::Stream(io),
            ..
        }) = self.links.get_mut(link)
        {
            io.free_write.push(buf);
        }
    }

    /// The transport failed or closed on its own.
    pub(crate) fn link_down(&mut self, link: LinkKey, error: String, now: Time) {
        if !self.links.contains_key(link) {
            return;
        }
        tracing::debug!(%error, "link down");
        self.metrics.transport_failures += 1;
        self.vnic.remove_link(link);
        if let Some(slot) = self.links.remove(link) {
            if let Some(conn) = slot.rtc {
                self.rtc_link_closed(conn, DialError::Connect(error), now);
            } else if let Some(d) = slot.dial {
                self.dial_done(d, Err(DialError::Connect(error)), now);
            }
        }
        self.process_events(now);
    }

    /// vnic closed a link: drop its transport (a WebRTC link closes its
    /// peer connection).
    fn link_closed(&mut self, link: LinkKey, reason: CloseReason, now: Time) {
        self.metrics.links_closed += 1;
        let Some(slot) = self.links.remove(link) else {
            return;
        };
        let e = match reason {
            CloseReason::Failed(strims_vnic::Error::PeerInit(PeerInitError::OwnHost)) => {
                DialError::OwnHost
            }
            CloseReason::Failed(e) => DialError::PeerInit(e.to_string()),
            CloseReason::Replaced => DialError::Connect("replaced".to_owned()),
            CloseReason::Closed => DialError::Connect("closed".to_owned()),
        };
        if let Some(conn) = slot.rtc {
            self.rtc_link_closed(conn, e, now);
        } else if let Some(d) = slot.dial {
            self.dial_done(d, Err(e), now);
        }
    }

    // --- egress -------------------------------------------------------------

    /// The egress pipeline: events, `vpn.flush`, `vnic.poll_egress` (fill
    /// routed by owner tag), and link transmit bytes to the link tasks,
    /// until nothing moves. Returns true if it stopped with work left
    /// (the driver runs it again without waiting).
    pub(crate) fn pump(&mut self, now: Time) -> bool {
        self.send_vpn_frames(now);
        for _ in 0..PUMP_ROUNDS {
            self.process_events(now);
            self.vpn.flush(&mut self.vnic, now);
            self.process_events(now);
            let State {
                vnic,
                vpn,
                runner,
                peers,
                metrics,
                ..
            } = self;
            vnic.poll_egress(now, &mut |f| match f.tag {
                Tag::Vpn(net) => vpn.fill_egress(net, f.peer, f.body),
                Tag::Ppspp(pp) => {
                    let len = runner.poll_transmit(pp, now, f.body);
                    Fill {
                        len,
                        more: runner.wants_transmit(pp),
                    }
                }
                Tag::Rpc(peer, side) => {
                    let written = if side == RpcSide::Broker {
                        &mut metrics.broker_bytes_written
                    } else {
                        &mut metrics.rpc_bytes_written
                    };
                    rpc::fill(peers, written, peer, side, f.body)
                }
            });
            let moved = self.drain_transmit();
            let evented = self.process_events(now);
            if !moved && !evented && !self.vpn.needs_flush() {
                return false;
            }
        }
        true
    }

    /// Copies pending link bytes into free write buffers and hands them to
    /// the writer tasks. A link with no free buffer keeps its bytes in
    /// vnic, whose full send buffer then blocks the link's QoS sessions:
    /// that is the backpressure from a slow socket to the owners.
    ///
    /// In-process WebRTC links write straight into their data channel
    /// until it is full; they wait for the endpoint's `Writable` (the same
    /// backpressure, from SCTP's send buffer).
    fn drain_transmit(&mut self) -> bool {
        let State {
            vnic,
            links,
            scratch_links,
            metrics,
            rtc,
            ..
        } = self;
        scratch_links.clear();
        scratch_links.extend(vnic.links_with_transmit());
        let mut moved = false;
        for &link in scratch_links.iter() {
            let Some(slot) = links.get_mut(link) else {
                continue;
            };
            let io = match &mut slot.io {
                LinkTransport::Stream(io) => io,
                LinkTransport::DataChannel(dc) => {
                    if let Some(rtc) = rtc
                        && let Some(conn) = slot.rtc
                    {
                        let n = rtc::write_channel(rtc, conn, dc, vnic, link);
                        metrics.bytes_written += n as u64;
                        moved |= n > 0;
                    }
                    continue;
                }
            };
            while let Some(mut buf) = io.free_write.pop() {
                let chunk = match vnic.poll_link_transmit(link) {
                    Ok(p) => p.get(..p.len().min(io.max_write)).unwrap_or_default(),
                    Err(_) => &[],
                };
                if chunk.is_empty() {
                    io.free_write.push(buf);
                    break;
                }
                let n = chunk.len();
                buf.clear();
                buf.extend_from_slice(chunk);
                match io.write_tx.try_send(buf) {
                    Ok(()) => {
                        let _ = vnic.advance_link_transmit(link, n);
                        metrics.bytes_written += n as u64;
                        moved = true;
                    }
                    Err(e) => {
                        // The writer is gone; its reader reports the link.
                        io.free_write.push(e.into_inner());
                        break;
                    }
                }
            }
        }
        moved
    }

    // --- events -------------------------------------------------------------

    /// Drains vnic, vpn, runner and WebRTC events (and the endpoint's
    /// datagrams) until all are quiet. Returns whether there were any.
    fn process_events(&mut self, now: Time) -> bool {
        let mut any = false;
        loop {
            let mut progress = false;
            while let Some(e) = self.vnic.poll_event() {
                progress = true;
                self.on_vnic_event(e, now);
            }
            while let Some(e) = self.vpn.poll_event() {
                progress = true;
                self.on_vpn_event(e, now);
            }
            while let Some(e) = self.runner.poll_event() {
                progress = true;
                self.on_runner_event(e, now);
            }
            progress |= self.drain_rtc(now);
            if !progress {
                return any;
            }
            any = true;
        }
    }

    fn on_vnic_event(&mut self, e: strims_vnic::Event, now: Time) {
        match e {
            strims_vnic::Event::PeerAdded { peer, host_id } => self.peer_added(peer, host_id, now),
            strims_vnic::Event::PeerRemoved { peer, host_id } => {
                self.vpn.handle_peer_removed(peer);
                if let Some(ps) = self.peers.remove(peer) {
                    self.runner.remove_peer(ps.pp);
                    self.ppspp_peers.remove(ps.pp);
                }
                self.emit(NodeEvent::PeerRemoved { host_id });
            }
            strims_vnic::Event::LinkClosed { link, reason } => self.link_closed(link, reason, now),
        }
    }

    /// A peer finished its handshake: add it to the runner with a
    /// transfer writer, open its RPC pair, bind it to the auto-bound
    /// networks, and complete its dial.
    fn peer_added(&mut self, peer: LinkKey, host_id: ID, now: Time) {
        let pp = self.runner.add_peer(&host_id.to_bytes(), now);
        let transfer = match self.vnic.channel(
            peer,
            ports::TRANSFER_PORT,
            self.transfer_class,
            Tag::Ppspp(pp),
        ) {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::warn!(error = %e, "opening the transfer channel");
                None
            }
        };
        let rpc = self.open_peer_rpc(peer, host_id);
        self.ppspp_peers.insert(pp, peer);
        self.peers.insert(
            peer,
            PeerState {
                host_id,
                pp,
                transfer,
                swarms: BTreeMap::new(),
                rpc,
            },
        );
        let State {
            vnic,
            vpn,
            networks,
            ..
        } = self;
        let mut bound = Vec::new();
        for (net, o) in networks.iter() {
            if let Some(port) = o.auto_bind_port {
                match vpn.add_peer(vnic, net, peer, port, port, Tag::Vpn(net)) {
                    Ok(()) => bound.push(net),
                    Err(e) => tracing::warn!(error = %e, "binding a peer to a network"),
                }
            }
        }
        // Until the peer sends a keepalive, its idle link may be silent:
        // the read timeout must not close it.
        if let Some(flag) = self
            .links
            .get(peer)
            .and_then(|l| l.io.stream())
            .and_then(|io| io.idle_ok.as_ref())
        {
            flag.set(!self.vnic.peer(peer).is_some_and(|p| p.sends_keepalives()));
        }
        if let Some(d) = self.links.get(peer).and_then(|l| l.dial) {
            self.dial_done(d, Ok(host_id), now);
        }
        if let Some(conn) = self.links.get(peer).and_then(|l| l.rtc) {
            self.rtc_peer_added(conn, now);
        }
        if let Some(info) = self.peer_info(peer) {
            self.emit(NodeEvent::PeerAdded(info));
        }
        for network in bound {
            self.emit(NodeEvent::NetworkPeerAdded { network, host_id });
        }
    }

    fn on_vpn_event(&mut self, e: VpnEvent<u16>, now: Time) {
        match e {
            VpnEvent::Message(d) => {
                if let Some(d) = self.deliver_to_port(d) {
                    self.emit(NodeEvent::Message(d));
                }
            }
            VpnEvent::HashTableValue { get, value, .. } => {
                let closed = match self.gets.get(&get).map(|tx| tx.try_send(value)) {
                    Some(Err(TrySendError::Full(_))) => {
                        self.metrics.results_dropped += 1;
                        false
                    }
                    Some(Err(TrySendError::Closed(_))) => true,
                    _ => false,
                };
                if closed {
                    self.gets.remove(&get);
                    self.vpn.hash_table_cancel_get(get);
                }
            }
            VpnEvent::HashTableGetClosed { get, .. } => {
                self.gets.remove(&get);
            }
            VpnEvent::PeerIndexHost { search, host, .. } => {
                let closed = match self.searches.get(&search).map(|tx| tx.try_send(host)) {
                    Some(Err(TrySendError::Full(_))) => {
                        self.metrics.results_dropped += 1;
                        false
                    }
                    Some(Err(TrySendError::Closed(_))) => true,
                    _ => false,
                };
                if closed {
                    self.searches.remove(&search);
                    self.vpn.peer_index_cancel_search(search);
                }
            }
            VpnEvent::PeerIndexSearchClosed { search, .. } => {
                self.searches.remove(&search);
            }
            VpnEvent::GatherLocalDescriptions { exchange, .. } => {
                // TCP and WebSocket candidates answer at once. A WebRTC
                // candidate is pending until ICE gathering completes; the
                // endpoint's `LocalDescription` event gathers again.
                self.exchange(exchange, now).gathering = true;
                self.gather(exchange, now);
            }
            VpnEvent::DialRemoteDescriptions {
                exchange,
                descriptions,
                ..
            } => {
                self.exchange(exchange, now)
                    .pool
                    .set_remote_descriptions(descriptions);
                self.poll_exchange(exchange, now);
            }
            VpnEvent::ExchangeClosed {
                network,
                exchange,
                host_id,
                outcome,
            } => {
                // A dial the exchange started goes on; its link outlives
                // the exchange. A WebRTC connection that never opened is
                // closed.
                if let Some(ex) = self.exchanges.remove(&exchange) {
                    self.rtc_exchange_closed(&ex.pool, now);
                }
                for (_, d) in self.dials.iter_mut() {
                    if d.exchange.is_some_and(|(x, _)| x == exchange) {
                        d.exchange = None;
                    }
                }
                self.emit(NodeEvent::ExchangeClosed {
                    network,
                    host_id,
                    outcome,
                });
            }
        }
    }

    fn on_runner_event(&mut self, e: strims_ppspp::Event, now: Time) {
        match e {
            strims_ppspp::Event::Transmit(pp) => {
                if let Some(w) = self.transfer_writer(pp) {
                    let _ = self.vnic.set_backlog(w);
                }
            }
            strims_ppspp::Event::QosWeight { peer, weight } => {
                if let Some(w) = self.transfer_writer(peer) {
                    let _ = self.vnic.set_writer_weight(w, weight);
                }
            }
            strims_ppspp::Event::Readable(swarm) => self.wake_readers(swarm, now),
        }
    }

    fn transfer_writer(&self, pp: PpsppPeer) -> Option<WriterId> {
        let peer = *self.ppspp_peers.get(pp)?;
        self.peers.get(peer)?.transfer
    }

    /// A peer's failed ppspp frame stops its swarm channels and keeps the
    /// vnic peer.
    fn handle_frame_errors(&mut self) {
        while let Some((pp, e)) = self.frame_errors.pop() {
            let Some(&peer) = self.ppspp_peers.get(pp) else {
                continue;
            };
            let State { peers, runner, .. } = self;
            let Some(ps) = peers.get_mut(peer) else {
                continue;
            };
            for &s in ps.swarms.keys() {
                runner.stop_swarm(pp, s);
            }
            ps.swarms.clear();
            let host_id = ps.host_id;
            self.metrics.transfer_errors += 1;
            tracing::debug!(error = %e, "ppspp frame");
            self.emit(NodeEvent::TransferError {
                host_id,
                error: e.to_string(),
            });
        }
    }

    // --- dials --------------------------------------------------------------

    /// Queues a dial of a `tcp://` or `ws[s]://` URI. `waiter` gets the
    /// peer's host id once the vnic handshake completes. A driver without
    /// TCP refuses `tcp://`.
    pub(crate) fn start_dial(
        &mut self,
        uri: &str,
        waiter: Option<oneshot::Sender<Result<ID, NodeError>>>,
    ) -> Result<(), NodeError> {
        let target = DialTarget::parse(uri, self.allow_insecure_ws)
            .map_err(|e| NodeError::InvalidUri(e.to_string()))?;
        if matches!(target, DialTarget::Tcp(_)) && !self.interfaces.tcp {
            return Err(NodeError::Unsupported("tcp links"));
        }
        let d = self.dials.insert(Dial {
            target,
            waiter,
            exchange: None,
        });
        self.dial_queue.push_back(d);
        Ok(())
    }

    /// The next dial for the driver to start.
    pub(crate) fn next_dial(&mut self) -> Option<(DialId, DialTarget)> {
        while let Some(d) = self.dial_queue.pop_front() {
            let Some(dial) = self.dials.get(d) else {
                continue;
            };
            match dial.target {
                DialTarget::Tcp(_) => self.metrics.tcp_dials += 1,
                DialTarget::Ws(_) => self.metrics.ws_dials += 1,
            }
            return Some((d, dial.target.clone()));
        }
        None
    }

    /// The driver couldn't connect (or upgrade) the dial's transport.
    pub(crate) fn dial_failed(&mut self, d: DialId, error: String, now: Time) {
        self.dial_done(d, Err(DialError::Connect(error)), now);
    }

    fn dial_done(&mut self, d: DialId, result: Result<ID, DialError>, now: Time) {
        let Some(dial) = self.dials.remove(d) else {
            return;
        };
        if result.is_err() {
            match dial.target {
                DialTarget::Tcp(_) => self.metrics.tcp_dial_errors += 1,
                DialTarget::Ws(_) => self.metrics.ws_dial_errors += 1,
            }
        }
        if let Some(w) = dial.waiter {
            let _ = w.send(result.clone().map_err(DialError::into_node_error));
        }
        if let Some((x, i)) = dial.exchange {
            if let Some(c) = self
                .exchanges
                .get_mut(&x)
                .and_then(|ex| ex.pool.candidates_mut().get_mut(i))
            {
                c.state = match result {
                    Ok(_) => CandidateState::Linked,
                    Err(DialError::PeerInit(m)) => {
                        CandidateState::Failed(CandidateError::PeerInit(m))
                    }
                    Err(e @ DialError::OwnHost) => {
                        CandidateState::Failed(CandidateError::PeerInit(e.message().to_owned()))
                    }
                    Err(DialError::Connect(m)) => CandidateState::Failed(CandidateError::Failed(m)),
                };
            }
            self.poll_exchange(x, now);
        }
    }

    // --- peer exchange ------------------------------------------------------

    /// The exchange's candidate pool, created on first use: one candidate
    /// per interface, in the order TCP (when the driver has it), WebSocket,
    /// WebRTC. The WebRTC candidate gets a new peer connection; if the
    /// endpoint can't make one, the pool goes without it.
    fn exchange(&mut self, x: ExchangeId, now: Time) -> &mut Exchange {
        let fresh = (!self.exchanges.contains_key(&x)).then(|| {
            let insecure = self.allow_insecure_ws;
            let mut candidates = Vec::with_capacity(3);
            if self.interfaces.tcp {
                candidates.push(Candidate::new(
                    LinkInterface::Tcp,
                    self.local_tcp.clone(),
                    insecure,
                ));
            }
            candidates.push(Candidate::new(
                LinkInterface::Ws,
                self.local_ws.clone(),
                insecure,
            ));
            if let Some(c) = self.rtc_candidate(x, candidates.len(), now) {
                candidates.push(c);
            }
            candidates
        });
        self.exchanges.entry(x).or_insert_with(move || Exchange {
            pool: LinkCandidatePool::new(fresh.unwrap_or_default()),
            gathering: false,
        })
    }

    /// Gives vpn the exchange's local descriptions once every candidate
    /// has one (WebRTC: ICE gathering finished).
    fn gather(&mut self, x: ExchangeId, now: Time) {
        let State {
            exchanges,
            rtc,
            vpn,
            ..
        } = self;
        let Some(ex) = exchanges.get_mut(&x) else {
            return;
        };
        if !ex.gathering {
            return;
        }
        if let Some(rtc) = rtc {
            rtc::poll_local_descriptions(rtc, ex.pool.candidates_mut(), now);
        }
        if let Poll::Ready(r) = ex.pool.poll_local_descriptions() {
            ex.gathering = false;
            let r = r.map_err(|e| e.to_string());
            if let Err(e) = vpn.set_local_descriptions(now, x, r) {
                tracing::debug!(error = %e, "peer exchange: local descriptions");
            }
        }
    }

    /// Advances the exchange's remote descriptions: reports the result to
    /// vpn, or starts the dials its candidates asked for. A WebRTC
    /// candidate's description is applied here at once, and the pool may
    /// then move on (an answerer reports `false`), so this polls again
    /// until the pool waits on I/O.
    fn poll_exchange(&mut self, x: ExchangeId, now: Time) {
        // Each round settles a candidate; a pool has at most three.
        for _ in 0..8 {
            let Some(ex) = self.exchanges.get_mut(&x) else {
                return;
            };
            match ex.pool.poll_remote_descriptions() {
                Poll::Ready(r) => {
                    let r = r.map_err(|e| e.to_string());
                    if let Err(e) = self.vpn.set_remote_descriptions_result(now, x, r) {
                        tracing::debug!(error = %e, "peer exchange: remote result");
                    }
                    return;
                }
                Poll::Pending => {
                    let State {
                        exchanges,
                        dials,
                        dial_queue,
                        rtc,
                        ..
                    } = self;
                    let Some(ex) = exchanges.get_mut(&x) else {
                        return;
                    };
                    let mut applied = false;
                    for (i, c) in ex.pool.candidates_mut().iter_mut().enumerate() {
                        match &c.state {
                            CandidateState::Requested(target) => {
                                let d = dials.insert(Dial {
                                    target: target.clone(),
                                    waiter: None,
                                    exchange: Some((x, i)),
                                });
                                dial_queue.push_back(d);
                                c.state = CandidateState::Dialing(d);
                            }
                            CandidateState::Describe(_) => {
                                rtc::apply_remote_description(rtc.as_mut(), c, now);
                                applied = true;
                            }
                            _ => {}
                        }
                    }
                    if !applied {
                        return;
                    }
                }
            }
        }
    }

    // --- peers --------------------------------------------------------------

    fn peer_info(&self, peer: LinkKey) -> Option<PeerInfo> {
        let p = self.vnic.peer(peer)?;
        Some(PeerInfo {
            host_id: p.host_id(),
            profile_key: p.certificate().key.clone(),
            kind: self.vnic.link_kind(peer).unwrap_or(LinkKind::Other),
            dialed: self.links.get(peer).is_some_and(|l| l.dialed),
            node_platform: p.node_platform().to_owned(),
            node_version: p.node_version().to_owned(),
        })
    }

    pub(crate) fn peers(&self) -> Vec<PeerInfo> {
        self.peers
            .keys()
            .filter_map(|k| self.peer_info(k))
            .collect()
    }

    fn peer_key(&self, host_id: &ID) -> Result<LinkKey, NodeError> {
        self.vnic.get_peer(host_id).ok_or(NodeError::UnknownPeer)
    }

    /// Closes the peer's link. Returns whether it was connected.
    pub(crate) fn close_peer(&mut self, host_id: &ID) -> bool {
        self.vnic
            .get_peer(host_id)
            .is_some_and(|p| self.vnic.close_peer(p))
    }

    /// Closes every link and WebRTC connection.
    pub(crate) fn close(&mut self, now: Time) {
        self.vnic.close();
        self.process_events(now);
        self.rtc_close_all(now);
        self.process_events(now);
        self.links.clear();
    }

    /// Every counter, in a fixed order (see [`NodeMetrics`]). `&mut` and
    /// `now` because the runner's rates advance their windows to `now`.
    pub(crate) fn metrics(&mut self, now: Time) -> NodeMetrics {
        NodeMetrics {
            host: self.vnic.metrics(),
            vpn: self.vpn.metrics(),
            driver: self.metrics,
            links: self.link_metrics(),
            runner: self.runner_metrics(now),
            networks: self.network_metrics(),
        }
    }

    fn link_metrics(&mut self) -> Vec<LinkInfo> {
        let State {
            vnic, links, rtc, ..
        } = self;
        links
            .iter()
            .filter_map(|(k, slot)| {
                let (write_blocked, buffered_amount) = match &slot.io {
                    LinkTransport::Stream(io) => (io.free_write.is_empty(), 0),
                    LinkTransport::DataChannel(dc) => (
                        dc.blocked(),
                        match (rtc.as_mut(), slot.rtc) {
                            (Some(r), Some(conn)) => r.ep.buffered_amount(conn),
                            _ => 0,
                        },
                    ),
                };
                Some(LinkInfo {
                    host_id: vnic.peer(k).map(|p| p.host_id()),
                    kind: slot.kind,
                    metrics: vnic.link_metrics(k)?,
                    dialed: slot.dialed,
                    mtu: vnic.link_mtu(k).unwrap_or(0),
                    transmit_pending: vnic.poll_link_transmit(k).map_or(0, <[u8]>::len),
                    input_buffered: vnic.link_input_buffered(k).unwrap_or(0),
                    write_blocked,
                    buffered_amount,
                })
            })
            .collect()
    }

    fn runner_metrics(&mut self, now: Time) -> RunnerMetrics {
        let swarms = self
            .swarms
            .iter()
            .filter_map(|&key| {
                let swarm = self.runner.swarm(key)?;
                let (id, label) = (swarm.id().clone(), swarm.options().label.clone());
                let store = swarm.store();
                let store = StoreInfo {
                    ready: store.is_ready(),
                    tail: store.tail(),
                    next: store.next(),
                    window: store.window(),
                    readable_seq: store.readable_seq(),
                };
                let readers = self.readers.values().filter(|r| r.swarm == key);
                Some(SwarmInfo {
                    swarm: key,
                    id,
                    label,
                    channels: self
                        .peers
                        .values()
                        .filter(|p| p.swarms.contains_key(&key))
                        .count(),
                    store,
                    scheduler: self.runner.swarm_stats(key),
                    push: self.runner.push_stats(key),
                    readers: readers.clone().count(),
                    readers_waiting: readers.filter(|r| r.pending.is_some()).count(),
                    publishers: self.publishers.values().filter(|p| p.swarm == key).count(),
                })
            })
            .collect();
        let State {
            vnic,
            runner,
            peers,
            ..
        } = self;
        let peers = peers
            .values()
            .map(|ps| {
                let snapshot = runner.peer_metrics(ps.pp, now).unwrap_or_default();
                let writer = ps.transfer.and_then(|w| {
                    let session = vnic.writer_session(w)?;
                    let qos = vnic.qos();
                    Some(WriterInfo {
                        weight: qos.weight_of(session).ok()?,
                        backlogged: qos.is_backlogged(session).ok()?,
                        blocked: qos.is_blocked(session).ok()?,
                    })
                });
                let channels = ps
                    .swarms
                    .iter()
                    .map(|(&swarm, ids)| ChannelInfo {
                        swarm,
                        channel: ids.channel,
                        peer_channel: ids.peer_channel,
                        metrics: snapshot
                            .swarms
                            .iter()
                            .find(|(k, _)| *k == swarm)
                            .map(|(_, m)| *m)
                            .unwrap_or_default(),
                        stats: runner.channel_stats(ps.pp, swarm),
                    })
                    .collect();
                RunnerPeerInfo {
                    host_id: ps.host_id,
                    qos_weight: runner.peer_qos_weight(ps.pp),
                    queues: runner.peer_stats(ps.pp).unwrap_or_default(),
                    metrics: snapshot.metrics,
                    writer,
                    channels,
                }
            })
            .collect();
        RunnerMetrics { swarms, peers }
    }

    fn network_metrics(&self) -> Vec<NetworkInfo> {
        self.networks
            .keys()
            .map(|net| {
                let (mut links, mut routed_links, mut queued) = (0, 0, 0);
                for (peer, _, routed) in self.vpn.links(net) {
                    links += 1;
                    routed_links += usize::from(routed);
                    queued += self.vpn.queued(net, peer);
                }
                NetworkInfo {
                    network: net,
                    links,
                    routed_links,
                    queued,
                    topology: self.vpn.topology_len(net),
                }
            })
            .collect()
    }

    // --- networks -----------------------------------------------------------

    /// Joins a network and binds the connected peers when it has an
    /// `auto_bind_port`.
    pub(crate) fn add_network(
        &mut self,
        key: &[u8],
        options: NetworkOptions,
    ) -> Result<NetworkId, NodeError> {
        let net = self.vpn.add_network(&mut self.vnic, key)?;
        if let Some(port) = options.auto_bind_port {
            let peers: Vec<LinkKey> = self.peers.keys().collect();
            for peer in peers {
                if let Err(e) = self.bind_network_peer(net, peer, port, port) {
                    tracing::warn!(error = %e, "binding a peer to a network");
                }
            }
        }
        self.networks.insert(net, options);
        Ok(net)
    }

    pub(crate) fn remove_network(&mut self, net: NetworkId) -> Result<(), NodeError> {
        let bound: Vec<ID> = self.vpn.links(net).map(|(_, host_id, _)| host_id).collect();
        self.vpn.remove_network(&mut self.vnic, net)?;
        self.networks.remove(net);
        self.close_network_ports(net);
        for host_id in bound {
            self.emit(NodeEvent::NetworkPeerRemoved {
                network: net,
                host_id,
            });
        }
        Ok(())
    }

    /// Binds a peer to a network: frames on `local_port` are the
    /// network's, and it sends to `peer_port`. A new binding is reported
    /// as `NetworkPeerAdded`.
    fn bind_network_peer(
        &mut self,
        net: NetworkId,
        peer: LinkKey,
        local_port: u16,
        peer_port: u16,
    ) -> Result<(), strims_vpn::Error> {
        let bound = self.vpn.links(net).any(|(k, _, _)| k == peer);
        self.vpn.add_peer(
            &mut self.vnic,
            net,
            peer,
            local_port,
            peer_port,
            Tag::Vpn(net),
        )?;
        if !bound && let Some(host_id) = self.peers.get(peer).map(|p| p.host_id) {
            self.emit(NodeEvent::NetworkPeerAdded {
                network: net,
                host_id,
            });
        }
        Ok(())
    }

    /// Frames on `local_port` go to the network, and the network sends to
    /// `peer_port`.
    pub(crate) fn bind_peer_network(
        &mut self,
        host_id: &ID,
        net: NetworkId,
        local_port: u16,
        peer_port: u16,
    ) -> Result<(), NodeError> {
        let peer = self.peer_key(host_id)?;
        self.bind_network_peer(net, peer, local_port, peer_port)?;
        Ok(())
    }

    /// Unbinds the peer from the network. Returns whether it was bound.
    pub(crate) fn unbind_peer_network(&mut self, host_id: &ID, net: NetworkId) -> bool {
        let unbound = self.vpn.remove_peer(&mut self.vnic, net, host_id);
        if unbound {
            self.emit(NodeEvent::NetworkPeerRemoved {
                network: net,
                host_id: *host_id,
            });
        }
        unbound
    }

    /// A vnic port on the peer for a network binding.
    pub(crate) fn reserve_peer_port(&mut self, host_id: &ID) -> Result<u16, NodeError> {
        let peer = self.peer_key(host_id)?;
        Ok(self.vnic.reserve_port(peer)?)
    }

    /// Releases a port from [`State::reserve_peer_port`].
    pub(crate) fn release_peer_port(&mut self, host_id: &ID, port: u16) -> Result<(), NodeError> {
        let peer = self.peer_key(host_id)?;
        self.vnic.release_port(peer, port);
        Ok(())
    }

    // --- vpn services -------------------------------------------------------

    pub(crate) fn hash_table_get(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &[u8],
        salt: &[u8],
        options: HashTableGetOptions,
        tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<(), NodeError> {
        let get = self.vpn.hash_table_get(now, net, key, salt, options)?;
        // Inserted before the events (a cached value, a close) are drained.
        self.gets.insert(get, tx);
        Ok(())
    }

    pub(crate) fn hash_table_set(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &Key,
        salt: &[u8],
        value: &[u8],
    ) -> Result<HashTablePublisherId, NodeError> {
        Ok(self.vpn.hash_table_set(now, net, key, salt, value)?)
    }

    pub(crate) fn peer_index_search(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &[u8],
        salt: &[u8],
        tx: mpsc::Sender<PeerIndexHost>,
    ) -> Result<(), NodeError> {
        let search = self.vpn.peer_index_search(now, net, key, salt)?;
        self.searches.insert(search, tx);
        Ok(())
    }

    pub(crate) fn peer_index_publish(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &[u8],
        salt: &[u8],
        port: u16,
    ) -> Result<PeerIndexPublisherId, NodeError> {
        Ok(self.vpn.peer_index_publish(now, net, key, salt, port)?)
    }

    /// Starts a peer exchange. False when `host_id` is this host, a peer
    /// already, or an exchange is in progress.
    pub(crate) fn connect_peer(
        &mut self,
        now: Time,
        net: NetworkId,
        host_id: ID,
    ) -> Result<bool, NodeError> {
        Ok(self.vpn.connect(&self.vnic, now, net, host_id)?.is_some())
    }

    // --- swarms -------------------------------------------------------------

    /// A swarm this node publishes, with its writer.
    pub(crate) fn create_swarm(
        &mut self,
        options: WriterOptions,
        now: Time,
        reply: mpsc::Sender<WriteReply>,
    ) -> Result<(SwarmKey, SwarmId, PublisherId), NodeError> {
        let (writer, swarm) = strims_ppspp::Writer::new(options, now)?;
        let id = swarm.id().clone();
        let key = self.runner.add_swarm(swarm);
        self.swarms.insert(key);
        let p = self.publishers.insert(PublisherSlot {
            swarm: key,
            writer,
            reply,
        });
        Ok((key, id, p))
    }

    /// A swarm this node downloads (and relays).
    pub(crate) fn add_swarm(
        &mut self,
        id: SwarmId,
        options: SwarmOptions,
    ) -> Result<SwarmKey, NodeError> {
        let swarm = strims_ppspp::Swarm::new(id, options)?;
        let key = self.runner.add_swarm(swarm);
        self.swarms.insert(key);
        Ok(key)
    }

    /// A swarm this node downloads (and relays), starting from `cache`, and
    /// whether it took the cache. A refused cache leaves the swarm empty.
    ///
    /// The cache goes in before the runner takes the swarm, since the
    /// runner's scheduler starts from the store's chunks. A failed import
    /// can leave part of the cache behind (the epoch, some signatures), so
    /// the swarm it failed on is dropped.
    pub(crate) fn add_cached_swarm(
        &mut self,
        id: SwarmId,
        options: SwarmOptions,
        cache: &Cache,
    ) -> Result<(SwarmKey, bool), NodeError> {
        let mut swarm = strims_ppspp::Swarm::new(id.clone(), options.clone())?;
        let imported = match swarm.import_cache(cache) {
            Ok(()) => true,
            Err(e) => {
                tracing::debug!(swarm = %id, error = %e, "refused a swarm cache");
                swarm = strims_ppspp::Swarm::new(id, options)?;
                false
            }
        };
        let key = self.runner.add_swarm(swarm);
        self.swarms.insert(key);
        Ok((key, imported))
    }

    /// The swarm's stream as a cache; `None` for an unknown swarm, one
    /// that holds nothing, or one whose window moved past offset 0.
    pub(crate) fn export_cache(&self, swarm: SwarmKey) -> Option<Cache> {
        self.runner.swarm(swarm)?.export_cache().ok()
    }

    /// Stops the swarm's channels, closes its publishers and readers.
    pub(crate) fn remove_swarm(&mut self, swarm: SwarmKey) -> Result<(), NodeError> {
        self.runner
            .remove_swarm(swarm)
            .ok_or(NodeError::UnknownSwarm)?;
        self.swarms.remove(&swarm);
        self.publishers.retain(|_, p| p.swarm != swarm);
        for (_, r) in self.readers.iter_mut() {
            if r.swarm == swarm
                && let Some((buf, _)) = r.pending.take()
            {
                let _ = r.reply.try_send(ReadReply {
                    buf,
                    result: Err(ReadError::Closed),
                });
            }
        }
        for (_, ps) in self.peers.iter_mut() {
            ps.swarms.remove(&swarm);
        }
        Ok(())
    }

    /// Runs the swarm with the peer on these channel ids.
    pub(crate) fn run_swarm(
        &mut self,
        host_id: &ID,
        swarm: SwarmKey,
        channel: u64,
        peer_channel: u64,
        now: Time,
    ) -> Result<(), NodeError> {
        let peer = self.peer_key(host_id)?;
        let ps = self.peers.get_mut(peer).ok_or(NodeError::UnknownPeer)?;
        self.runner
            .run_swarm(ps.pp, swarm, channel, peer_channel, now)?;
        ps.swarms.insert(
            swarm,
            ChannelIds {
                channel,
                peer_channel,
            },
        );
        Ok(())
    }

    pub(crate) fn stop_swarm(&mut self, host_id: &ID, swarm: SwarmKey) -> Result<(), NodeError> {
        let peer = self.peer_key(host_id)?;
        let ps = self.peers.get_mut(peer).ok_or(NodeError::UnknownPeer)?;
        self.runner.stop_swarm(ps.pp, swarm);
        ps.swarms.remove(&swarm);
        Ok(())
    }

    pub(crate) fn open_reader(
        &mut self,
        swarm: SwarmKey,
        reply: mpsc::Sender<ReadReply>,
    ) -> Result<ReaderId, NodeError> {
        let reader = self
            .runner
            .swarm(swarm)
            .ok_or(NodeError::UnknownSwarm)?
            .reader();
        Ok(self.readers.insert(ReaderSlot {
            swarm,
            reader,
            pending: None,
            reply,
        }))
    }

    pub(crate) fn close_reader(&mut self, id: ReaderId) {
        self.readers.remove(id);
    }

    /// Reads into `buf[..max]`, or parks the buffer until the swarm is
    /// readable.
    pub(crate) fn read(&mut self, id: ReaderId, mut buf: Vec<u8>, max: usize, now: Time) {
        let State {
            readers, runner, ..
        } = self;
        let Some(slot) = readers.get_mut(id) else {
            return;
        };
        match read_swarm(runner, slot, &mut buf, max, now) {
            None => slot.pending = Some((buf, max)),
            Some(result) => {
                let _ = slot.reply.try_send(ReadReply { buf, result });
            }
        }
    }

    fn wake_readers(&mut self, swarm: SwarmKey, now: Time) {
        let State {
            readers, runner, ..
        } = self;
        for (_, slot) in readers.iter_mut() {
            if slot.swarm != swarm {
                continue;
            }
            let Some((mut buf, max)) = slot.pending.take() else {
                continue;
            };
            match read_swarm(runner, slot, &mut buf, max, now) {
                None => slot.pending = Some((buf, max)),
                Some(result) => {
                    let _ = slot.reply.try_send(ReadReply { buf, result });
                }
            }
        }
    }

    pub(crate) fn write(&mut self, id: PublisherId, op: WriteOp, buf: Vec<u8>, now: Time) {
        let State {
            publishers, runner, ..
        } = self;
        let Some(p) = publishers.get_mut(id) else {
            return;
        };
        let result = match op {
            WriteOp::Write(len) => runner
                .write(
                    p.swarm,
                    &mut p.writer,
                    now,
                    buf.get(..len).unwrap_or_default(),
                )
                .map(|_| ()),
            WriteOp::Flush => runner.flush_writer(p.swarm, &mut p.writer, now),
            WriteOp::Reset => runner.reset_writer(p.swarm, &mut p.writer, now),
        }
        .map_err(NodeError::from);
        let _ = p.reply.try_send(WriteReply { buf, result });
    }

    pub(crate) fn close_publisher(&mut self, id: PublisherId) {
        self.publishers.remove(id);
    }
}

/// Peer exchange steps vpn drives, for tests without a network.
#[cfg(test)]
impl State {
    /// Exchange `x`'s pool, created as vpn's first event for it does.
    pub(crate) fn test_pool(&mut self, x: ExchangeId, now: Time) -> &mut [Candidate] {
        self.exchange(x, now).pool.candidates_mut()
    }

    /// Applies `desc` to exchange `x`'s WebRTC candidate, as trying the
    /// remote descriptions does.
    pub(crate) fn test_describe(
        &mut self,
        x: ExchangeId,
        desc: strims_vnic::webrtc::SessionDescription,
        now: Time,
    ) {
        let State { exchanges, rtc, .. } = self;
        let Some(c) = exchanges.get_mut(&x).and_then(|ex| {
            ex.pool
                .candidates_mut()
                .iter_mut()
                .find(|c| c.conn.is_some())
        }) else {
            return;
        };
        c.state = CandidateState::Describe(desc);
        rtc::apply_remote_description(rtc.as_mut(), c, now);
    }

    pub(crate) fn test_link_count(&self) -> usize {
        self.links.len()
    }
}

/// One non-blocking read. `None`: nothing to read yet.
fn read_swarm(
    runner: &mut Runner,
    slot: &mut ReaderSlot,
    buf: &mut [u8],
    max: usize,
    now: Time,
) -> Option<Result<(u64, usize), ReadError>> {
    let n = max.min(buf.len());
    let res = match runner.swarm(slot.swarm) {
        Some(s) => s.read(&mut slot.reader, buf.get_mut(..n).unwrap_or_default()),
        None => return Some(Err(ReadError::Closed)),
    };
    Some(match res {
        // The read may resync the reader first, so its start is known only
        // after it.
        Ok(n) => Ok((slot.reader.position().wrapping_sub(n as u64), n)),
        Err(store::Error::WouldBlock) => return None,
        Err(store::Error::BufferUnderrun) => match runner.swarm_mut(slot.swarm, now) {
            Some(mut s) => match s.recover_reader(&mut slot.reader) {
                Ok(skipped) => Err(ReadError::Underrun { skipped }),
                Err(e) => Err(ReadError::Store(e.to_string())),
            },
            None => Err(ReadError::Closed),
        },
        Err(store::Error::StreamReset) => match runner.swarm(slot.swarm) {
            Some(s) => {
                slot.reader = s.reader();
                Err(ReadError::Reset)
            }
            None => Err(ReadError::Closed),
        },
        Err(store::Error::Closed) => Err(ReadError::Closed),
        Err(e) => Err(ReadError::Store(e.to_string())),
    })
}
