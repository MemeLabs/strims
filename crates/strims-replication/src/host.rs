// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What replication needs from the node it runs on.

use std::fmt;
use std::future::Future;

use strims_kademlia::ID;
use strims_proto::types::Key;

/// The node services replication uses to find the profile's other
/// devices.
pub trait Host: 'static {
    /// A network the node belongs to.
    type Network: Copy + Ord + fmt::Debug + 'static;
    /// A record the node publishes until it is withdrawn.
    type Publication: 'static;
    /// A failed node call.
    type Error: fmt::Display;

    /// The node's host id.
    fn host_id(&self) -> ID;

    /// The networks the node belongs to.
    fn networks(&self) -> impl Future<Output = Vec<Self::Network>>;

    /// Publishes `value` in `net`'s hash table under (`key.public`, `salt`),
    /// signed with `key`.
    fn publish(
        &self,
        net: Self::Network,
        key: &Key,
        salt: Vec<u8>,
        value: Vec<u8>,
    ) -> impl Future<Output = Result<Self::Publication, Self::Error>>;

    /// Withdraws a publication.
    fn unpublish(
        &self,
        publication: Self::Publication,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Up to `max` values published in `net`'s hash table under (`key`,
    /// `salt`).
    fn lookup(
        &self,
        net: Self::Network,
        key: Vec<u8>,
        salt: Vec<u8>,
        max: usize,
    ) -> impl Future<Output = Result<Vec<Vec<u8>>, Self::Error>>;

    /// Asks the node to link to `host` through `net`. Resolves to whether
    /// it started a link.
    fn connect_peer(
        &self,
        net: Self::Network,
        host: ID,
    ) -> impl Future<Output = Result<bool, Self::Error>>;
}

/// The hash table salt under which device `device` publishes its host id.
pub fn discovery_salt(device: u64) -> Vec<u8> {
    let mut s = b"replication:".to_vec();
    s.extend_from_slice(strims_dao::key::base36(device).as_bytes());
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salts_name_devices_in_base36() {
        assert_eq!(discovery_salt(36), b"replication:10");
    }
}
