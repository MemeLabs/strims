// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A datagram channel to one remote port.
//!
//! Writes are [`Host::send`] calls; reads are the
//! [`crate::Event::Message`]s tagged with the channel's handler.

use strims_base::time::Time;
use strims_kademlia::ID;

use crate::{Error, Host, NetworkId};

/// A handler on a local port paired with one remote port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Channel {
    pub network: NetworkId,
    pub dst_id: ID,
    pub dst_port: u16,
    pub src_port: u16,
}

impl Channel {
    /// Sets `handler` on `src_port`.
    pub fn open<H: Copy>(
        host: &mut Host<H>,
        network: NetworkId,
        dst_id: ID,
        dst_port: u16,
        src_port: u16,
        handler: H,
    ) -> Result<Channel, Error> {
        host.set_handler(network, src_port, handler)?;
        Ok(Channel {
            network,
            dst_id,
            dst_port,
            src_port,
        })
    }

    /// Sends one message (encrypted).
    pub fn write<H: Copy>(&self, host: &mut Host<H>, now: Time, b: &[u8]) -> Result<usize, Error> {
        host.send(
            now,
            self.network,
            self.dst_id,
            self.dst_port,
            self.src_port,
            b,
        )?;
        Ok(b.len())
    }

    /// Removes the handler.
    pub fn close<H: Copy>(self, host: &mut Host<H>) {
        host.remove_handler(self.network, self.src_port);
    }
}
