// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the network control needs from the node it runs on.

use std::fmt;
use std::future::Future;

use futures_core::Stream;
use futures_sink::Sink;
use strims_kademlia::ID;
use strims_proto::types::Key;
use strims_vpn::{Delivery, HostAddr};

/// The node operations behind network membership: networks, peer port
/// bindings, each peer's broker channel, and the vpn ports and hash table
/// records of the networks' services.
pub trait Host: 'static {
    /// A network the node runs.
    type Network: Copy + Ord + fmt::Debug + 'static;
    /// A peer's broker bytes, as they arrive. It ends when the channel
    /// closes.
    type BrokerReader: Stream<Item = Result<Vec<u8>, Self::Error>> + Unpin + 'static;
    /// Bytes for a peer's broker channel.
    type BrokerWriter: Sink<Vec<u8>, Error = Self::Error> + Unpin + 'static;
    /// A vpn port's messages.
    type VpnReader: Stream<Item = Result<Delivery<u16>, Self::Error>> + Unpin + 'static;
    /// A vpn port's sends.
    type VpnWriter: Sink<(HostAddr, Vec<u8>), Error = Self::Error> + Unpin + 'static;
    /// A published service address, until withdrawn.
    type Publication: 'static;
    /// A failed node call.
    type Error: fmt::Display;

    /// The node's host id.
    fn host_id(&self) -> ID;

    /// Joins the network whose key is `key`.
    fn add_network(&self, key: &[u8]) -> impl Future<Output = Result<Self::Network, Self::Error>>;

    /// Leaves a network, unbinding its peers.
    fn remove_network(&self, net: Self::Network) -> impl Future<Output = Result<(), Self::Error>>;

    /// The peer's broker channel, taken once per link.
    fn take_broker(
        &self,
        host: ID,
    ) -> impl Future<Output = Result<(Self::BrokerReader, Self::BrokerWriter), Self::Error>>;

    /// Reserves a free vnic port on the peer.
    fn reserve_peer_port(&self, host: ID) -> impl Future<Output = Result<u16, Self::Error>>;

    /// Releases a reserved port.
    fn release_peer_port(
        &self,
        host: ID,
        port: u16,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Binds the peer to the network: frames on `local_port` are the
    /// network's, and it sends to `peer_port`.
    fn bind_peer(
        &self,
        host: ID,
        net: Self::Network,
        local_port: u16,
        peer_port: u16,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Unbinds the peer from the network.
    fn unbind_peer(
        &self,
        host: ID,
        net: Self::Network,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Opens a free vpn port on the network: its number, messages and
    /// sends.
    fn open_vpn_port(
        &self,
        net: Self::Network,
    ) -> impl Future<Output = Result<(u16, Self::VpnReader, Self::VpnWriter), Self::Error>>;

    /// Publishes this host's `port` in the network's hash table as the
    /// address of the service under (`key`, `salt`).
    fn publish_address(
        &self,
        net: Self::Network,
        key: Key,
        salt: Vec<u8>,
        port: u16,
    ) -> impl Future<Output = Result<Self::Publication, Self::Error>>;

    /// Withdraws a published address.
    fn unpublish_address(
        &self,
        publication: Self::Publication,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// The address of the service under (`key`, `salt`), if the network's
    /// hash table has one.
    fn resolve_address(
        &self,
        net: Self::Network,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> impl Future<Output = Result<Option<HostAddr>, Self::Error>>;
}
