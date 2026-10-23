// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The directory's swarms and names on a network.
//!
//! Both swarms have the network key as their id, and every member opens
//! them with the same options, so these values are part of the wire
//! format. A record frame is one integrity unit (chunk size × chunks per
//! signature), so a writer signs whole frames.

use strims_ppspp::integrity::{ProtectionMethod, VerifierOptions};
use strims_ppspp::store::BufferLayout;
use strims_ppspp::{DeliveryMode, PushMode, SwarmId, SwarmOptions};

/// The hash table salt of the server's address.
pub const ADDRESS_SALT: &[u8] = b"directory";
/// The transfer salt of the event swarm.
pub const EVENT_SALT: &[u8] = b"directory:events";
/// The transfer salt of the asset swarm.
pub const ASSET_SALT: &[u8] = b"directory:assets";

/// The event swarm's record frame.
pub const EVENT_FRAME: usize = 1024;
/// The asset swarm's record frame.
pub const ASSET_FRAME: usize = 128 * 1024;

/// The event swarm of the network with key `network_key`: 1 KiB chunks,
/// each signed, a 2 MiB window, best effort delivery, pull only, with the
/// defaults for the rest.
pub fn event_options(network_key: &[u8]) -> SwarmOptions {
    SwarmOptions {
        label: label(network_key, "events"),
        chunk_size: 1024,
        chunks_per_signature: 1,
        live_window: 2 * 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::SIGN_ALL,
            ..VerifierOptions::default()
        },
        delivery_mode: DeliveryMode::BEST_EFFORT,
        push_mode: PushMode::Off,
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// The asset swarm of the network with key `network_key`: 1 KiB chunks
/// under Merkle trees of 128, a 2 MiB window, mandatory delivery and an
/// elastic buffer, pull only, with the defaults for the rest.
pub fn asset_options(network_key: &[u8]) -> SwarmOptions {
    SwarmOptions {
        label: label(network_key, "assets"),
        chunk_size: 1024,
        chunks_per_signature: 128,
        live_window: 2 * 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::MERKLE_TREE,
            ..VerifierOptions::default()
        },
        delivery_mode: DeliveryMode::MANDATORY,
        buffer_layout: BufferLayout::ELASTIC,
        push_mode: PushMode::Off,
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// `directory_<first 8 characters of the swarm id>_<kind>`.
fn label(network_key: &[u8], kind: &str) -> String {
    let id = SwarmId::new(network_key).to_string();
    let prefix: String = id.chars().take(8).collect();
    format!("directory_{prefix}_{kind}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use strims_ppspp::integrity::{LiveSignatureAlgorithm, MerkleHashTreeFunction};

    #[test]
    fn a_record_frame_is_one_integrity_unit() {
        let e = event_options(&[1; 32]);
        assert_eq!(e.chunk_size * e.chunks_per_signature, EVENT_FRAME);
        let a = asset_options(&[1; 32]);
        assert_eq!(a.chunk_size * a.chunks_per_signature, ASSET_FRAME);
    }

    #[test]
    fn the_event_swarm_signs_every_chunk_of_a_two_mib_window() {
        let o = event_options(&[0; 32]);
        assert_eq!(o.label, "directory_AAAAAAAA_events");
        assert_eq!(o.live_window * o.chunk_size, 2 * 1024 * 1024);
        assert_eq!(o.integrity.protection_method, ProtectionMethod::SIGN_ALL);
        assert_eq!(
            o.integrity.live_signature_algorithm,
            LiveSignatureAlgorithm::ED25519
        );
        assert_eq!(
            o.integrity.merkle_hash_tree_function,
            MerkleHashTreeFunction::BLAKE2B256
        );
        assert_eq!(o.stream_count, 1);
        assert_eq!(o.delivery_mode, DeliveryMode::BEST_EFFORT);
        assert_eq!(o.buffer_layout, BufferLayout::CIRCULAR);
    }

    #[test]
    fn the_asset_swarm_keeps_its_bundle_from_the_start() {
        let o = asset_options(&[0xff; 32]);
        assert_eq!(o.label, "directory_77777777_assets");
        assert_eq!(o.integrity.protection_method, ProtectionMethod::MERKLE_TREE);
        assert_eq!(o.delivery_mode, DeliveryMode::MANDATORY);
        assert_eq!(o.buffer_layout, BufferLayout::ELASTIC);
        assert_eq!(o.live_window, 2048);
    }
}
