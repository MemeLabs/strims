// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Peer RPC channels.
//!
//! Every vnic peer gets a channel pair on
//! [`PEER_RPC_CLIENT_PORT`](strims_vnic::ports::PEER_RPC_CLIENT_PORT) and
//! [`PEER_RPC_SERVER_PORT`](strims_vnic::ports::PEER_RPC_SERVER_PORT), in a
//! QoS class of its own: this node's calls go out on the client channel,
//! and the peer's arrive on the server channel. The app takes a peer's pair
//! once, with [`crate::Node::take_peer_rpc`], and runs an RPC connection
//! over it (`strims_rpc::peer_connection`).
//!
//! A channel is a byte stream each way:
//!
//! - an [`RpcReader`] yields received frame bodies. It ends when the peer
//!   goes. Bytes it hasn't yielded are bounded by
//!   [`PeerRpcConfig::inbound_buffer`](crate::PeerRpcConfig::inbound_buffer);
//!   past that the node closes the pair and the reader yields
//!   [`NodeError::RpcOverflow`]. Frames that arrive before the pair is
//!   taken are held under the same bound.
//! - an [`RpcWriter`] takes one whole Call per item and sends it in vnic
//!   frames cut at the link MTU, ending a frame after each Call. It holds
//!   at most [`PeerRpcConfig::outbound_buffer`](crate::PeerRpcConfig::outbound_buffer)
//!   bytes plus one Call: it isn't ready while the node holds that much.
//!
//! Dropping a reader closes the pair; the peer stays connected.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures_util::task::AtomicWaker;
use futures_util::{Sink, Stream};
use strims_kademlia::ID;
use tokio::sync::mpsc;

use crate::error::NodeError;
use crate::state::RpcChannelId;
use crate::vpn_port::{PortId, VpnFrame};

/// A peer's RPC channel pair; see the [module docs](self).
#[derive(Debug)]
pub struct PeerRpc {
    /// The peer.
    pub host_id: ID,
    /// This node's calls to the peer.
    pub client: RpcChannel,
    /// The peer's calls to this node.
    pub server: RpcChannel,
}

/// One channel of a [`PeerRpc`].
#[derive(Debug)]
pub struct RpcChannel {
    pub reader: RpcReader,
    pub writer: RpcWriter,
}

/// A channel's received bytes.
#[derive(Debug)]
pub struct RpcReader {
    pub(crate) rx: mpsc::UnboundedReceiver<Result<Vec<u8>, NodeError>>,
    /// Bytes sent to `rx` and not yet taken from it.
    pub(crate) queued: Arc<AtomicUsize>,
}

impl Stream for RpcReader {
    type Item = Result<Vec<u8>, NodeError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let r = self.rx.poll_recv(cx);
        if let Poll::Ready(Some(Ok(b))) = &r {
            self.queued.fetch_sub(b.len(), Ordering::AcqRel);
        }
        r
    }
}

/// What a writer may still queue, shared with the node task.
#[derive(Debug, Default)]
pub(crate) struct RpcCredit {
    /// Bytes the writer may queue. Goes negative by at most one Call: a
    /// writer is ready while it's positive.
    pub(crate) bytes: AtomicI64,
    pub(crate) waker: AtomicWaker,
    /// The channel is gone.
    pub(crate) closed: AtomicBool,
}

impl RpcCredit {
    pub(crate) fn new(bytes: usize) -> RpcCredit {
        RpcCredit {
            bytes: AtomicI64::new(i64::try_from(bytes).unwrap_or(i64::MAX)),
            ..RpcCredit::default()
        }
    }

    /// Returns `n` bytes sent and wakes the writer.
    pub(crate) fn release(&self, n: usize) {
        self.bytes
            .fetch_add(i64::try_from(n).unwrap_or(i64::MAX), Ordering::AcqRel);
        self.waker.wake();
    }

    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.waker.wake();
    }

    /// Ready while there is credit; [`NodeError::Closed`] once closed.
    pub(crate) fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), NodeError>> {
        let ready = || {
            if self.closed.load(Ordering::Acquire) {
                Some(Err(NodeError::Closed))
            } else if self.bytes.load(Ordering::Acquire) > 0 {
                Some(Ok(()))
            } else {
                None
            }
        };
        if let Some(r) = ready() {
            return Poll::Ready(r);
        }
        self.waker.register(cx.waker());
        // credit returned between the check and the registration
        match ready() {
            Some(r) => Poll::Ready(r),
            None => Poll::Pending,
        }
    }

    /// Takes `n` bytes of credit for an item being sent.
    pub(crate) fn take(&self, n: usize) -> Result<(), NodeError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(NodeError::Closed);
        }
        self.bytes
            .fetch_sub(i64::try_from(n).unwrap_or(i64::MAX), Ordering::AcqRel);
        Ok(())
    }
}

/// What app writers hand the node task.
#[derive(Debug)]
pub(crate) enum Outgoing {
    /// A Call for a peer RPC channel.
    Rpc(RpcFrame),
    /// A message from a vpn port.
    Vpn(VpnFrame),
    /// A vpn port's reader was dropped.
    ClosePort(PortId),
}

/// A Call for the node task to send on a channel.
#[derive(Debug)]
pub(crate) struct RpcFrame {
    pub(crate) channel: RpcChannelId,
    pub(crate) call: Vec<u8>,
}

/// A channel's sender of Calls.
#[derive(Debug)]
pub struct RpcWriter {
    pub(crate) channel: RpcChannelId,
    pub(crate) tx: mpsc::UnboundedSender<Outgoing>,
    pub(crate) credit: Arc<RpcCredit>,
}

impl Sink<Vec<u8>> for RpcWriter {
    type Error = NodeError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), NodeError>> {
        self.credit.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, call: Vec<u8>) -> Result<(), NodeError> {
        self.credit.take(call.len())?;
        self.tx
            .send(Outgoing::Rpc(RpcFrame {
                channel: self.channel,
                call,
            }))
            .map_err(|_| NodeError::Stopped)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), NodeError>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), NodeError>> {
        Poll::Ready(Ok(()))
    }
}
