// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Peer RPC channels and the network broker channel inside the node task
//! (see [`crate::rpc`]).
//!
//! A peer's channels live in its [`PeerState`]: vnic frames on their ports
//! reach them by [`Tag::Rpc`], and dropping the peer drops them, ending
//! their readers and closing their writers.
//!
//! - **Inbound.** Frame bodies go to the channel's reader over an
//!   unbounded queue whose bytes are bounded by `inbound_buffer`. A frame
//!   past the bound, or a dropped reader, closes the RPC pair (and the
//!   broker channel with it), or only the broker channel, after vnic's
//!   dispatch (`rpc_closed`).
//! - **Outbound.** Calls from writers arrive as [`RpcFrame`]s. Each is
//!   parked, then fed into the channel's [`FrameWriter`] as room allows and
//!   flushed when whole, so a Call's frames are cut at the link MTU as one
//!   write and flush would cut them. Bytes the egress pipeline takes go
//!   back to the writer as credit.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use strims_kademlia::ID;
use strims_vnic::{Fill, FrameWriter, PeerKey as LinkKey, WriterId, ports};
use tokio::sync::mpsc;

use super::{State, Tag};
use crate::error::NodeError;
use crate::rpc::{PeerRpc, RpcChannel, RpcCredit, RpcFrame, RpcReader, RpcWriter};

/// Which of a peer's channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RpcSide {
    /// This node's calls.
    Client,
    /// The peer's calls.
    Server,
    /// The network broker's byte stream.
    Broker,
}

/// A channel of a peer's RPC pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RpcChannelId {
    pub(crate) peer: LinkKey,
    pub(crate) side: RpcSide,
}

/// The weight of the peer RPC QoS class under vnic's root.
pub(super) const PEER_RPC_QOS_WEIGHT: u64 = 1;

#[derive(Debug)]
pub(super) struct RpcChannelState {
    writer: WriterId,
    /// Bytes of parked Calls cut into frames. Bounded by `outbound_buffer`.
    fw: FrameWriter,
    /// Calls waiting for room in `fw`, and how much of the first is in it.
    /// Bounded by the writer's credit.
    parked: VecDeque<Vec<u8>>,
    parked_off: usize,
    inbound: mpsc::UnboundedSender<Result<Vec<u8>, NodeError>>,
    /// Bytes in `inbound` the reader hasn't taken.
    queued: Arc<AtomicUsize>,
    credit: Arc<RpcCredit>,
}

impl Drop for RpcChannelState {
    fn drop(&mut self) {
        self.credit.close();
    }
}

impl RpcChannelState {
    /// Moves parked Calls into the frame writer while it has room.
    fn feed(&mut self) {
        while let Some(front) = self.parked.front() {
            let rest = front.get(self.parked_off..).unwrap_or_default();
            let n = self.fw.write(rest).unwrap_or(0);
            self.parked_off += n;
            if self.parked_off < front.len() {
                return;
            }
            let _ = self.fw.flush();
            self.parked.pop_front();
            self.parked_off = 0;
        }
    }

    /// Fills a frame body from the frame writer.
    fn fill(&mut self, body: &mut [u8]) -> Fill {
        let len = self.fw.fill(body);
        if len > 0 {
            self.credit.release(len);
        }
        self.feed();
        Fill {
            len,
            more: self.fw.has_frame(),
        }
    }

    /// Hands a frame body to the reader: [`NodeError::RpcOverflow`] past
    /// the limit, [`NodeError::Closed`] if the reader is gone.
    fn deliver(&self, body: &[u8], limit: usize) -> Result<(), NodeError> {
        let queued = self.queued.load(Ordering::Acquire);
        if queued + body.len() > limit {
            let _ = self.inbound.send(Err(NodeError::RpcOverflow));
            return Err(NodeError::RpcOverflow);
        }
        self.queued.fetch_add(body.len(), Ordering::AcqRel);
        self.inbound
            .send(Ok(body.to_vec()))
            .map_err(|_| NodeError::Closed)
    }
}

/// A peer's RPC pair and broker channel.
#[derive(Debug)]
pub(super) struct PeerRpcState {
    client: RpcChannelState,
    server: RpcChannelState,
    /// Held until the app takes them.
    handles: Option<PeerRpc>,
    /// `None` if it couldn't open, or once it has closed.
    broker: Option<RpcChannelState>,
    /// Held until the app takes it.
    broker_handle: Option<RpcChannel>,
}

impl PeerRpcState {
    fn side_mut(&mut self, side: RpcSide) -> Option<&mut RpcChannelState> {
        match side {
            RpcSide::Client => Some(&mut self.client),
            RpcSide::Server => Some(&mut self.server),
            RpcSide::Broker => self.broker.as_mut(),
        }
    }
}

/// The weight of the network broker's QoS class under vnic's root.
pub(super) const BROKER_QOS_WEIGHT: u64 = 1;

impl State {
    /// Opens the peer's RPC pair and broker channel, unless peer RPC is
    /// off.
    pub(super) fn open_peer_rpc(&mut self, peer: LinkKey, host_id: ID) -> Option<PeerRpcState> {
        if !self.peer_rpc.enabled {
            return None;
        }
        let (w0, w1) = match self.vnic.channel_pair(
            peer,
            ports::PEER_RPC_CLIENT_PORT,
            ports::PEER_RPC_SERVER_PORT,
            self.rpc_class,
            Tag::Rpc(peer, RpcSide::Client),
            Tag::Rpc(peer, RpcSide::Server),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!(error = %e, "opening the peer rpc channels");
                return None;
            }
        };
        let (client, client_handle) = self.rpc_channel(peer, RpcSide::Client, w0);
        let (server, server_handle) = self.rpc_channel(peer, RpcSide::Server, w1);
        let broker = match self.vnic.channel(
            peer,
            ports::NETWORK_BROKER_PORT,
            self.broker_class,
            Tag::Rpc(peer, RpcSide::Broker),
        ) {
            Ok(w) => Some(self.rpc_channel(peer, RpcSide::Broker, w)),
            Err(e) => {
                tracing::warn!(error = %e, "opening the broker channel");
                None
            }
        };
        let (broker, broker_handle) = broker.unzip();
        Some(PeerRpcState {
            client,
            server,
            handles: Some(PeerRpc {
                host_id,
                client: client_handle,
                server: server_handle,
            }),
            broker,
            broker_handle,
        })
    }

    fn rpc_channel(
        &self,
        peer: LinkKey,
        side: RpcSide,
        writer: WriterId,
    ) -> (RpcChannelState, RpcChannel) {
        let out = self.peer_rpc.outbound_buffer;
        let max_body = self.vnic.writer_max_body(writer).unwrap_or(1);
        let (tx, rx) = mpsc::unbounded_channel();
        let queued = Arc::new(AtomicUsize::new(0));
        let credit = Arc::new(RpcCredit::new(out));
        let state = RpcChannelState {
            writer,
            fw: FrameWriter::new(max_body, out),
            parked: VecDeque::new(),
            parked_off: 0,
            inbound: tx,
            queued: queued.clone(),
            credit: credit.clone(),
        };
        let handle = RpcChannel {
            reader: RpcReader { rx, queued },
            writer: RpcWriter {
                channel: RpcChannelId { peer, side },
                tx: self.rpc_tx.clone(),
                credit,
            },
        };
        (state, handle)
    }

    /// The peer's RPC pair, once.
    pub(crate) fn take_peer_rpc(&mut self, host_id: &ID) -> Result<PeerRpc, NodeError> {
        if !self.peer_rpc.enabled {
            return Err(NodeError::Unsupported("peer rpc is disabled"));
        }
        let peer = self.peer_key(host_id)?;
        let rpc = self
            .peers
            .get_mut(peer)
            .ok_or(NodeError::UnknownPeer)?
            .rpc
            .as_mut()
            .ok_or(NodeError::Closed)?;
        rpc.handles.take().ok_or(NodeError::RpcTaken)
    }

    /// The peer's broker channel, once.
    pub(crate) fn take_peer_broker(&mut self, host_id: &ID) -> Result<RpcChannel, NodeError> {
        if !self.peer_rpc.enabled {
            return Err(NodeError::Unsupported("peer rpc is disabled"));
        }
        let peer = self.peer_key(host_id)?;
        let rpc = self
            .peers
            .get_mut(peer)
            .ok_or(NodeError::UnknownPeer)?
            .rpc
            .as_mut()
            .ok_or(NodeError::Closed)?;
        if rpc.broker.is_none() {
            return Err(NodeError::Closed);
        }
        rpc.broker_handle.take().ok_or(NodeError::RpcTaken)
    }

    /// Queues a Call from a writer.
    pub(crate) fn rpc_write(&mut self, frame: RpcFrame) {
        let RpcChannelId { peer, side } = frame.channel;
        let Some(ch) = self
            .peers
            .get_mut(peer)
            .and_then(|p| p.rpc.as_mut())
            .and_then(|r| r.side_mut(side))
        else {
            // the channel closed; its writer has been told
            return;
        };
        ch.parked.push_back(frame.call);
        ch.feed();
        if ch.fw.has_frame()
            && let Err(e) = self.vnic.set_backlog(ch.writer)
        {
            tracing::debug!(error = %e, "peer rpc backlog");
        }
    }

    /// Closes the channels that overflowed or lost their reader during
    /// vnic's dispatch: the broker channel alone, or the RPC pair with the
    /// broker channel.
    pub(super) fn handle_rpc_closed(&mut self) {
        while let Some((peer, side, overflow)) = self.rpc_closed.pop() {
            let Some(rpc) = self.peers.get_mut(peer).and_then(|p| p.rpc.as_mut()) else {
                continue;
            };
            let writers = if side == RpcSide::Broker {
                let Some(broker) = rpc.broker.take() else {
                    continue;
                };
                if overflow {
                    self.metrics.broker_overflows += 1;
                }
                vec![broker.writer]
            } else {
                let Some(rpc) = self.peers.get_mut(peer).and_then(|p| p.rpc.take()) else {
                    continue;
                };
                if overflow {
                    self.metrics.rpc_overflows += 1;
                }
                let mut w = vec![rpc.client.writer, rpc.server.writer];
                w.extend(rpc.broker.map(|b| b.writer));
                w
            };
            for w in writers {
                let _ = self.vnic.close_channel(w);
            }
        }
    }
}

/// vnic's dispatch of a frame body to one of a peer's channels.
pub(super) fn dispatch(
    peers: &mut slotmap::SecondaryMap<LinkKey, super::PeerState>,
    rpc_closed: &mut Vec<(LinkKey, RpcSide, bool)>,
    bytes_read: &mut u64,
    limit: usize,
    peer: LinkKey,
    side: RpcSide,
    body: &[u8],
) {
    if body.is_empty() {
        return;
    }
    let Some(ch) = peers
        .get_mut(peer)
        .and_then(|p| p.rpc.as_mut())
        .and_then(|r| r.side_mut(side))
    else {
        return;
    };
    match ch.deliver(body, limit) {
        Ok(()) => *bytes_read += body.len() as u64,
        Err(e) => {
            if !rpc_closed.iter().any(|(p, s, _)| *p == peer && *s == side) {
                rpc_closed.push((peer, side, e == NodeError::RpcOverflow));
            }
        }
    }
}

/// vnic's egress fill for one of a peer's channels.
pub(super) fn fill(
    peers: &mut slotmap::SecondaryMap<LinkKey, super::PeerState>,
    bytes_written: &mut u64,
    peer: LinkKey,
    side: RpcSide,
    body: &mut [u8],
) -> Fill {
    match peers
        .get_mut(peer)
        .and_then(|p| p.rpc.as_mut())
        .and_then(|r| r.side_mut(side))
    {
        Some(ch) => {
            let f = ch.fill(body);
            *bytes_written += f.len as u64;
            f
        }
        None => Fill {
            len: 0,
            more: false,
        },
    }
}
