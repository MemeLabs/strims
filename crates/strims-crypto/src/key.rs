// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! ed25519 keys. Signing is [`crate::sign`]'s.
//!
//! An ed25519 `strims.type.Key` holds `type = KEY_TYPE_ED25519`, the 32-byte
//! public key, and the 64-byte expanded private key, seed || public key.

use ed25519_dalek::SigningKey;
use strims_base::rng::DetRng;
use strims_proto::types::{Key, KeyType};

/// ed25519 public key size.
pub const ED25519_PUBLIC_KEY_SIZE: usize = 32;
/// ed25519 expanded private key size: seed || public key.
pub const ED25519_PRIVATE_KEY_SIZE: usize = 64;
/// ed25519 seed size.
pub const ED25519_SEED_SIZE: usize = 32;
/// ed25519 signature size.
pub const ED25519_SIGNATURE_SIZE: usize = 64;

/// Generates an ed25519 key from 32 bytes of `rng`.
pub fn generate_key(rng: &mut DetRng) -> Key {
    let mut seed = [0u8; ED25519_SEED_SIZE];
    rand_core::Rng::fill_bytes(rng, &mut seed);
    key_from_seed(&seed)
}

/// The ed25519 key with the given RFC 8032 seed.
pub fn key_from_seed(seed: &[u8; ED25519_SEED_SIZE]) -> Key {
    let sk = SigningKey::from_bytes(seed);
    Key {
        r#type: KeyType::Ed25519 as i32,
        private: sk.to_keypair_bytes().to_vec(),
        public: sk.verifying_key().to_bytes().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    // Tests panic on failure by design.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_base::rng::det_rng;

    #[test]
    fn generated_key_layout() {
        let k = generate_key(&mut det_rng(9));
        assert_eq!(k.r#type, KeyType::Ed25519 as i32);
        assert_eq!(k.private.len(), 64);
        assert_eq!(k.public.len(), 32);
        assert_eq!(&k.private[32..], k.public.as_slice());
        // deterministic
        assert_eq!(generate_key(&mut det_rng(9)), k);
        assert_ne!(generate_key(&mut det_rng(10)), k);
    }
}
