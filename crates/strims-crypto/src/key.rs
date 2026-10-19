// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! ed25519 keys, signing and verification.
//!
//! An ed25519 `strims.type.Key` holds `type = KEY_TYPE_ED25519`, the 32-byte
//! public key, and the 64-byte expanded private key, seed || public key.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use strims_base::rng::DetRng;
use strims_proto::types::{Key, KeyType};

use crate::Error;

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

/// Parses a 64-byte expanded private key. A public half that doesn't match
/// the seed is [`Error::KeyPairMismatch`].
pub(crate) fn signing_key_from_private(private: &[u8]) -> Result<SigningKey, Error> {
    let bytes: &[u8; ED25519_PRIVATE_KEY_SIZE] =
        private.try_into().map_err(|_| Error::InvalidKeyLength)?;
    SigningKey::from_keypair_bytes(bytes).map_err(|_| Error::KeyPairMismatch)
}

/// Signs `message` with a 64-byte expanded private key.
///
/// Derives the signing key on every call (two base-point multiplications).
pub fn sign(private: &[u8], message: &[u8]) -> Result<[u8; ED25519_SIGNATURE_SIZE], Error> {
    Ok(signing_key_from_private(private)?.sign(message).to_bytes())
}

/// Verifies an ed25519 signature. Returns `false` for a public key that
/// isn't 32 bytes or isn't a curve point.
///
/// Every node must accept exactly the same signatures: RFC 8032
/// cofactorless verification, non-canonical `S` rejected,
/// non-canonical public key `y` accepted, and the encoded `R` compared byte
/// for byte. These are ed25519-dalek's `verify` rules; `verify_strict`
/// rejects some signatures they accept.
///
/// Decompresses the public key on every call.
pub fn verify(public: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let Ok(public) = <&[u8; ED25519_PUBLIC_KEY_SIZE]>::try_from(public) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(signature) else {
        return false;
    };
    VerifyingKey::from_bytes(public).is_ok_and(|vk| vk.verify(message, &signature).is_ok())
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

    #[test]
    fn sign_and_verify() {
        let k = key_from_seed(&[3; 32]);
        let sig = sign(&k.private, b"msg").unwrap();
        assert!(verify(&k.public, b"msg", &sig));
        assert!(!verify(&k.public, b"msh", &sig));
        assert!(!verify(&k.public[..31], b"msg", &sig));
        assert!(!verify(&k.public, b"msg", &sig[..63]));
        assert_eq!(sign(&k.private[..63], b"msg"), Err(Error::InvalidKeyLength));

        let mut mismatched = k.private.clone();
        mismatched[40] ^= 1;
        assert_eq!(sign(&mismatched, b"msg"), Err(Error::KeyPairMismatch));
    }
}
