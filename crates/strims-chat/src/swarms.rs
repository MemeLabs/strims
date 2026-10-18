// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A chat server's swarms and names on its network.
//!
//! Both swarms have the server's public key as their id, and every reader
//! opens them with the same options, so these values are part of the wire
//! format. A record frame is one integrity unit (chunk size × chunks per
//! signature), so a writer signs whole frames.

use strims_ppspp::integrity::{ProtectionMethod, VerifierOptions};
use strims_ppspp::store::BufferLayout;
use strims_ppspp::{DeliveryMode, SwarmId, SwarmOptions};

/// The hash table salt of the server's address.
pub const ADDRESS_SALT: &[u8] = b"chat";
/// The transfer salt of the event swarm.
pub const EVENT_SALT: &[u8] = b"chat:events";
/// The transfer salt of the asset swarm.
pub const ASSET_SALT: &[u8] = b"chat:assets";
/// The hash table salt of a profile's whisper address.
pub const WHISPER_SALT: &[u8] = b"chat:whisper";

/// The event swarm's record frame.
pub const EVENT_FRAME: usize = 256;
/// The asset swarm's record frame.
pub const ASSET_FRAME: usize = 128 * 1024;
/// The asset swarm's window: the most bytes of bundles a reader can take
/// from one stream. A record writer for the stream needs a `max_record` of
/// at least this, since one record may take all of it.
pub const ASSET_WINDOW_BYTES: usize = 32 * 1024 * 1024;

/// The event swarm of the server with key `server_key`: 256 B chunks,
/// each signed, a 1024 chunk (256 KiB) window, best effort delivery, with
/// the defaults for the rest.
pub fn event_options(server_key: &[u8]) -> SwarmOptions {
    SwarmOptions {
        label: label(server_key, "events"),
        chunk_size: 256,
        chunks_per_signature: 1,
        live_window: 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::SIGN_ALL,
            ..VerifierOptions::default()
        },
        delivery_mode: DeliveryMode::BEST_EFFORT,
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// The asset swarm of the server with key `server_key`: 1 KiB chunks
/// under Merkle trees of 128, a 32 Ki chunk (32 MiB) window, mandatory
/// delivery and an elastic buffer, with the defaults for the rest.
pub fn asset_options(server_key: &[u8]) -> SwarmOptions {
    SwarmOptions {
        label: label(server_key, "assets"),
        chunk_size: 1024,
        chunks_per_signature: 128,
        live_window: 32 * 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::MERKLE_TREE,
            ..VerifierOptions::default()
        },
        delivery_mode: DeliveryMode::MANDATORY,
        buffer_layout: BufferLayout::ELASTIC,
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// `chat_<first 8 characters of the swarm id>_<kind>`.
fn label(server_key: &[u8], kind: &str) -> String {
    let id = SwarmId::new(server_key).to_string();
    let prefix: String = id.chars().take(8).collect();
    format!("chat_{prefix}_{kind}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use strims_ppspp::integrity::{LiveSignatureAlgorithm, MerkleHashTreeFunction};

    /// A key whose swarm id isn't uniform, so the label shows which
    /// characters it takes.
    fn key() -> Vec<u8> {
        (0..32u8).map(|i| i.wrapping_mul(7)).collect()
    }

    #[test]
    fn a_record_frame_is_one_integrity_unit() {
        let e = event_options(&key());
        assert_eq!(e.chunk_size * e.chunks_per_signature, EVENT_FRAME);
        let a = asset_options(&key());
        assert_eq!(a.chunk_size * a.chunks_per_signature, ASSET_FRAME);
        assert_eq!(a.chunk_size * a.live_window, ASSET_WINDOW_BYTES);
    }

    #[test]
    fn labels_take_the_first_eight_characters_of_the_swarm_id() {
        // Recorded labels.
        assert_eq!(event_options(&key()).label, "chat_AADQ4FI4_events");
        assert_eq!(asset_options(&key()).label, "chat_AADQ4FI4_assets");
        assert_eq!(event_options(&[0; 32]).label, "chat_AAAAAAAA_events");
    }

    #[test]
    fn the_event_swarm_signs_every_chunk_of_a_256_kib_window() {
        let o = event_options(&key());
        assert_eq!(o.chunk_size, 256);
        assert_eq!(o.chunks_per_signature, 1);
        assert_eq!(o.live_window, 1024);
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
    fn the_asset_swarm_keeps_its_bundles_from_the_start() {
        let o = asset_options(&key());
        assert_eq!(o.chunk_size, 1024);
        assert_eq!(o.chunks_per_signature, 128);
        assert_eq!(o.live_window, 32 * 1024);
        assert_eq!(o.integrity.protection_method, ProtectionMethod::MERKLE_TREE);
        assert_eq!(o.delivery_mode, DeliveryMode::MANDATORY);
        assert_eq!(o.buffer_layout, BufferLayout::ELASTIC);
    }
}
