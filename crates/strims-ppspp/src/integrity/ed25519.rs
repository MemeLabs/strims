// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Ed25519 live signatures.
//!
//! Both sides sign `uint64(t.UnixNano())` big-endian followed by the
//! payload (a Merkle root, a chunk, or nothing for the epoch). Keys are
//! parsed once, when the signer or verifier is created.

use ed25519_dalek::{Signer, Verifier, VerifyingKey};
use strims_base::time::Time;

use super::{Error, MAX_SIGNATURE_SIZE, Signature, signed_message};

/// The length of an ed25519 signature.
pub const ED25519_SIGNATURE_SIZE: usize = 64;

/// Signs live data with a swarm's private key.
#[derive(Clone)]
pub struct Ed25519Signer {
    key: ed25519_dalek::SigningKey,
}

impl core::fmt::Debug for Ed25519Signer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Ed25519Signer")
    }
}

impl Ed25519Signer {
    /// From a 64 byte private key (seed ‖ public key). A key whose public
    /// half doesn't match its seed is rejected, since its signatures would
    /// not verify against the swarm id.
    pub fn new(private_key: &[u8]) -> Result<Ed25519Signer, Error> {
        let key =
            strims_crypto::signing_key_from_private(private_key).map_err(|_| Error::InvalidKey)?;
        Ok(Ed25519Signer { key })
    }

    /// Signs `t ‖ p`.
    pub fn sign(&self, t: Time, p: &[u8], scratch: &mut Vec<u8>) -> Signature {
        let mut stack = [0u8; 8 + MAX_SIGNATURE_SIZE];
        let msg = signed_message(t, p, &mut stack, scratch);
        let sig = self.key.sign(msg).to_bytes();
        Signature::from_slice(&sig).unwrap_or(Signature::EMPTY)
    }
}

/// Verifies live data against a swarm's public key.
#[derive(Clone)]
pub struct Ed25519Verifier {
    /// `None` for 32 bytes that aren't a curve point: every signature then
    /// fails to verify.
    key: Option<VerifyingKey>,
}

impl core::fmt::Debug for Ed25519Verifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Ed25519Verifier")
    }
}

impl Ed25519Verifier {
    /// From a 32 byte public key, the swarm id. Other lengths are
    /// rejected.
    pub fn new(public_key: &[u8]) -> Result<Ed25519Verifier, Error> {
        let bytes: &[u8; 32] = public_key.try_into().map_err(|_| Error::InvalidKey)?;
        Ok(Ed25519Verifier {
            key: VerifyingKey::from_bytes(bytes).ok(),
        })
    }

    /// Checks `sig` over `t ‖ p` with cofactorless RFC 8032 verification.
    /// Every v2 peer must accept exactly the same signatures, so this must
    /// not change to cofactored or batch verification.
    pub fn verify(&self, t: Time, p: &[u8], sig: &[u8], scratch: &mut Vec<u8>) -> bool {
        let Some(key) = &self.key else {
            return false;
        };
        let Ok(sig) = ed25519_dalek::Signature::from_slice(sig) else {
            return false;
        };
        let mut stack = [0u8; 8 + MAX_SIGNATURE_SIZE];
        let msg = signed_message(t, p, &mut stack, scratch);
        key.verify(msg, &sig).is_ok()
    }
}
