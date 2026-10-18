// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! vpn ports for the app's RPC.
//!
//! [`crate::Node::open_vpn_port`] takes a port on a network and returns its
//! messages as a stream and its sends as a sink, the transport that
//! `strims_rpc::vpn::connect` runs on:
//!
//! - a [`VpnReader`] yields the port's [`Delivery`]s, decrypted and stamped
//!   with the time they arrived. Bytes it hasn't yielded are bounded by
//!   [`VpnPortConfig::inbound_buffer`](crate::VpnPortConfig::inbound_buffer);
//!   messages past that are dropped and counted
//!   ([`DriverMetrics::vpn_port_drops`](crate::DriverMetrics::vpn_port_drops)).
//!   It ends when the port's network is removed.
//! - a [`VpnWriter`] takes `(destination, body)` and sends the body
//!   encrypted from the port. It holds at most
//!   [`VpnPortConfig::outbound_buffer`](crate::VpnPortConfig::outbound_buffer)
//!   bytes plus one message. Messages vpn refuses (too large, no route)
//!   are dropped and counted
//!   ([`DriverMetrics::vpn_send_errors`](crate::DriverMetrics::vpn_send_errors)).
//!
//! Dropping the reader closes the port and frees its number.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures_util::{Sink, Stream};
use strims_vpn::{Delivery, HostAddr, NetworkId};
use tokio::sync::mpsc;

use crate::error::NodeError;
use crate::rpc::{Outgoing, RpcCredit};

/// An open port: the network, the port number and a serial that tells it
/// from earlier ports with the same number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PortId {
    pub(crate) network: NetworkId,
    pub(crate) port: u16,
    pub(crate) serial: u64,
}

/// A message for the node task to send from a port.
#[derive(Debug)]
pub(crate) struct VpnFrame {
    pub(crate) port: PortId,
    pub(crate) to: HostAddr,
    pub(crate) body: Vec<u8>,
}

/// An open vpn port; see the [module docs](self).
#[derive(Debug)]
pub struct VpnPort {
    pub network: NetworkId,
    pub port: u16,
    pub reader: VpnReader,
    pub writer: VpnWriter,
}

/// A port's messages.
#[derive(Debug)]
pub struct VpnReader {
    pub(crate) port: PortId,
    pub(crate) rx: mpsc::UnboundedReceiver<Delivery<u16>>,
    /// Body bytes sent to `rx` and not yet taken from it.
    pub(crate) queued: Arc<AtomicUsize>,
    pub(crate) tx: mpsc::UnboundedSender<Outgoing>,
}

impl Stream for VpnReader {
    type Item = Result<Delivery<u16>, NodeError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let r = self.rx.poll_recv(cx);
        if let Poll::Ready(Some(d)) = &r {
            self.queued.fetch_sub(d.body.len(), Ordering::AcqRel);
        }
        r.map(|d| d.map(Ok))
    }
}

impl Drop for VpnReader {
    fn drop(&mut self) {
        let _ = self.tx.send(Outgoing::ClosePort(self.port));
    }
}

/// A port's sender of messages.
#[derive(Debug)]
pub struct VpnWriter {
    pub(crate) port: PortId,
    pub(crate) tx: mpsc::UnboundedSender<Outgoing>,
    pub(crate) credit: Arc<RpcCredit>,
}

impl Sink<(HostAddr, Vec<u8>)> for VpnWriter {
    type Error = NodeError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), NodeError>> {
        self.credit.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, (to, body): (HostAddr, Vec<u8>)) -> Result<(), NodeError> {
        self.credit.take(body.len())?;
        self.tx
            .send(Outgoing::Vpn(VpnFrame {
                port: self.port,
                to,
                body,
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
