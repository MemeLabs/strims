// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Transfer ids.

use std::fmt;

use sha2::{Digest, Sha256};
use strims_ppspp::SwarmId;

/// A transfer's id: SHA-256 of the swarm id followed by the salt. Peers
/// name transfers by it, so the derivation is part of the wire format.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransferId(pub [u8; 32]);

impl TransferId {
    /// The id of `swarm` under `salt`.
    pub fn new(swarm: &SwarmId, salt: &[u8]) -> TransferId {
        let digest = Sha256::new()
            .chain_update(swarm.as_bytes())
            .chain_update(salt)
            .finalize();
        TransferId(digest.into())
    }

    /// An id from its bytes, if there are exactly 32 of them.
    pub fn from_bytes(b: &[u8]) -> Option<TransferId> {
        b.try_into().ok().map(TransferId)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TransferId({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_hash_the_swarm_id_then_the_salt() {
        let id = TransferId::new(&SwarmId::new(b"ab"), b"c");
        assert_eq!(
            id.to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn ids_parse_only_from_32_bytes() {
        let id = TransferId::new(&SwarmId::new(b"swarm"), b"");
        assert_eq!(TransferId::from_bytes(id.as_bytes()), Some(id));
        assert_eq!(TransferId::from_bytes(&[0; 31]), None);
        assert_eq!(TransferId::from_bytes(&[0; 33]), None);
    }
}
