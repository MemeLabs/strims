// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the directory control needs from the node it runs on.

use std::future::Future;

/// A network the node runs.
pub type Net<H> = <H as strims_network::Host>::Network;

type VpnReader<H> = <H as strims_network::Host>::VpnReader;
type VpnWriter<H> = <H as strims_network::Host>::VpnWriter;
type NetError<H> = <H as strims_network::Host>::Error;

/// The node operations behind the directory: those of network membership
/// and of swarm streams, on the same networks, and a vpn port of a given
/// number.
pub trait Host:
    strims_network::Host + strims_transfer::SwarmHost<Network = <Self as strims_network::Host>::Network>
{
    /// Opens vpn port `port` on the network: its messages and sends, as
    /// [`open_vpn_port`](strims_network::Host::open_vpn_port) gives them.
    /// Fails while the port is open.
    fn open_vpn_port_at(
        &self,
        net: Net<Self>,
        port: u16,
    ) -> impl Future<Output = Result<(VpnReader<Self>, VpnWriter<Self>), NetError<Self>>>;
}
