// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the transfer control needs from the node it runs on.

use std::fmt;
use std::future::Future;

use strims_kademlia::ID;

/// The node operations behind transfers: swarm channels, and the peer
/// index for discovery.
pub trait Host: 'static {
    /// A network the node belongs to.
    type Network: Copy + Ord + fmt::Debug + 'static;
    /// A swarm in the node.
    type Swarm: Copy + Eq + fmt::Debug + 'static;
    /// A peer index record the node publishes until it is withdrawn.
    type Publication: 'static;
    /// A failed node call.
    type Error: fmt::Display;

    /// The node's host id.
    fn host_id(&self) -> ID;

    /// Runs `swarm` with the peer `host`: the peer's frames for it carry
    /// `channel`, ours carry `peer_channel`.
    fn run_swarm(
        &self,
        host: ID,
        swarm: Self::Swarm,
        channel: u64,
        peer_channel: u64,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Stops `swarm` with the peer `host`. Stopping a swarm that isn't
    /// running is not an error.
    fn stop_swarm(
        &self,
        host: ID,
        swarm: Self::Swarm,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Advertises in `net`'s peer index that this host serves (`key`,
    /// `salt`).
    fn index_publish(
        &self,
        net: Self::Network,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> impl Future<Output = Result<Self::Publication, Self::Error>>;

    /// Withdraws a peer index record.
    fn index_unpublish(
        &self,
        publication: Self::Publication,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Up to `max` hosts that advertise (`key`, `salt`) in `net`'s peer
    /// index. The caller drops the future to end the search sooner.
    fn index_search(
        &self,
        net: Self::Network,
        key: Vec<u8>,
        salt: Vec<u8>,
        max: usize,
    ) -> impl Future<Output = Result<Vec<ID>, Self::Error>>;

    /// Asks the node to link to `host` through `net`. Resolves to whether
    /// it started a link.
    fn connect_peer(
        &self,
        net: Self::Network,
        host: ID,
    ) -> impl Future<Output = Result<bool, Self::Error>>;
}
