// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Service addresses: a host and a port on one network.
//!
//! Servers publish theirs in the hash table as an encoded
//! `strims.vpn.v1.NetworkAddress`, under the service's key and a salt
//! naming the service.

use prost::Message;
use strims_kademlia::ID;
use strims_proto::vpn::v1::NetworkAddress;

/// A port on a host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostAddr {
    pub host: ID,
    pub port: u16,
}

impl HostAddr {
    pub fn new(host: ID, port: u16) -> HostAddr {
        HostAddr { host, port }
    }

    /// The address as published in the hash table.
    pub fn to_network_address(&self) -> NetworkAddress {
        NetworkAddress {
            host_id: self.host.to_bytes().to_vec(),
            port: u32::from(self.port),
        }
    }

    /// The address a record holds, or `None` if its host id isn't 32 bytes
    /// or its port is out of range.
    pub fn from_network_address(addr: &NetworkAddress) -> Option<HostAddr> {
        let host = ID::try_from(addr.host_id.as_slice()).ok()?;
        let port = u16::try_from(addr.port).ok()?;
        Some(HostAddr { host, port })
    }

    /// The encoded record.
    pub fn encode(&self) -> Vec<u8> {
        self.to_network_address().encode_to_vec()
    }

    /// Reads an encoded record; `None` if it doesn't decode or is invalid.
    pub fn decode(b: &[u8]) -> Option<HostAddr> {
        HostAddr::from_network_address(&NetworkAddress::decode(b).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_round_trip_and_reject_invalid_records() {
        let addr = HostAddr::new(ID([1, 2, 3, 4]), 4321);
        assert_eq!(HostAddr::decode(&addr.encode()), Some(addr));

        let mut bad = addr.to_network_address();
        bad.port = u32::from(u16::MAX) + 1;
        assert_eq!(HostAddr::from_network_address(&bad), None);
        let mut bad = addr.to_network_address();
        bad.host_id.pop();
        assert_eq!(HostAddr::from_network_address(&bad), None);
        assert_eq!(HostAddr::decode(&[0xff]), None);
    }
}
