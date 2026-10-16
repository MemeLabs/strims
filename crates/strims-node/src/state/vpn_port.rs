// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! vpn ports inside the node task (see [`crate::vpn_port`]).
//!
//! - **Inbound.** A delivery for an open port goes to its reader over an
//!   unbounded queue whose body bytes are bounded by `inbound_buffer`; one
//!   past the bound is dropped. Deliveries for other app ports stay
//!   [`NodeEvent::Message`](crate::NodeEvent::Message)s.
//! - **Outbound.** Messages from writers arrive as [`VpnFrame`](crate::vpn_port::VpnFrame)s and wait
//!   in `vpn_sends`, bounded by the writers' credit, until the next pump
//!   sends them with its `now`. vpn copies the body, so the credit comes
//!   back as each is sent.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use strims_base::time::Time;
use strims_vpn::{Delivery, NetworkId};
use tokio::sync::mpsc;

use super::State;
use crate::error::NodeError;
use crate::rpc::{Outgoing, RpcCredit};
use crate::vpn_port::{PortId, VpnPort, VpnReader, VpnWriter};

#[derive(Debug)]
pub(super) struct PortState {
    serial: u64,
    inbound: mpsc::UnboundedSender<Delivery<u16>>,
    /// Body bytes in `inbound` the reader hasn't taken.
    queued: Arc<AtomicUsize>,
    credit: Arc<RpcCredit>,
}

impl Drop for PortState {
    fn drop(&mut self) {
        self.credit.close();
    }
}

impl State {
    /// Takes `port` on the network, or a free port from 1000 up.
    pub(crate) fn open_vpn_port(
        &mut self,
        net: NetworkId,
        port: Option<u16>,
    ) -> Result<VpnPort, NodeError> {
        let port = match port {
            Some(p) => {
                self.vpn.set_handler(net, p, p)?;
                p
            }
            None => {
                let p = self.vpn.reserve_port(net)?;
                if let Err(e) = self.vpn.set_handler(net, p, p) {
                    self.vpn.release_port(net, p);
                    return Err(e.into());
                }
                p
            }
        };
        let id = PortId {
            network: net,
            port,
            serial: self.next_port_serial,
        };
        self.next_port_serial += 1;
        let (tx, rx) = mpsc::unbounded_channel();
        let queued = Arc::new(AtomicUsize::new(0));
        let credit = Arc::new(RpcCredit::new(self.vpn_ports_config.outbound_buffer));
        self.vpn_ports.insert(
            (net, port),
            PortState {
                serial: id.serial,
                inbound: tx,
                queued: queued.clone(),
                credit: credit.clone(),
            },
        );
        Ok(VpnPort {
            network: net,
            port,
            reader: VpnReader {
                port: id,
                rx,
                queued,
                tx: self.rpc_tx.clone(),
            },
            writer: VpnWriter {
                port: id,
                tx: self.rpc_tx.clone(),
                credit,
            },
        })
    }

    /// Hands a delivery to its port's reader. Gives it back if no port is
    /// open there.
    pub(super) fn deliver_to_port(&mut self, d: Delivery<u16>) -> Option<Delivery<u16>> {
        let Some(p) = self.vpn_ports.get(&(d.network, d.handler)) else {
            return Some(d);
        };
        let len = d.body.len();
        if p.queued.load(Ordering::Acquire) + len > self.vpn_ports_config.inbound_buffer {
            self.metrics.vpn_port_drops += 1;
            return None;
        }
        p.queued.fetch_add(len, Ordering::AcqRel);
        if let Err(mpsc::error::SendError(d)) = p.inbound.send(d) {
            // the reader is gone and its close is on the way
            p.queued.fetch_sub(d.body.len(), Ordering::AcqRel);
        }
        None
    }

    /// Takes what an app writer handed the node task.
    pub(crate) fn outgoing(&mut self, o: Outgoing) {
        match o {
            Outgoing::Rpc(f) => self.rpc_write(f),
            Outgoing::Vpn(f) => self.vpn_sends.push_back(f),
            Outgoing::ClosePort(id) => self.close_vpn_port(id),
        }
    }

    /// Sends the messages vpn ports' writers queued.
    pub(super) fn send_vpn_frames(&mut self, now: Time) {
        while let Some(f) = self.vpn_sends.pop_front() {
            let PortId { network, port, .. } = f.port;
            let Some(p) = self
                .vpn_ports
                .get(&(network, port))
                .filter(|p| p.serial == f.port.serial)
            else {
                continue;
            };
            p.credit.release(f.body.len());
            if let Err(e) = self
                .vpn
                .send(now, network, f.to.host, f.to.port, port, &f.body)
            {
                tracing::debug!(error = %e, "vpn port send");
                self.metrics.vpn_send_errors += 1;
            }
        }
    }

    fn close_vpn_port(&mut self, id: PortId) {
        let key = (id.network, id.port);
        if self
            .vpn_ports
            .get(&key)
            .is_some_and(|p| p.serial == id.serial)
        {
            self.vpn_ports.remove(&key);
            self.vpn.remove_handler(id.network, id.port);
        }
    }

    /// Removes an app handler set with `set_vpn_handler`. Ports opened with
    /// `open_vpn_port` close when their reader is dropped.
    pub(crate) fn remove_vpn_handler(&mut self, net: NetworkId, port: u16) -> bool {
        if self.vpn_ports.contains_key(&(net, port)) {
            return false;
        }
        self.vpn.remove_handler(net, port).is_some()
    }

    /// Ends the network's ports' readers and closes their writers.
    pub(super) fn close_network_ports(&mut self, net: NetworkId) {
        self.vpn_ports.retain(|(n, _), _| *n != net);
    }
}
