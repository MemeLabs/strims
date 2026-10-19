// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Ed25519 live signatures.
//!
//! A live signature signs `timestamp (u64 BE ns) ‖ payload` in the payload's
//! [`Domain`]: a munro's root digest, a chunk, or nothing for the epoch.
//! Keys are parsed once, when the signer or verifier is created.

use strims_base::time::Time;
use strims_crypto::{Domain, Signer, Verifier};

use super::{Error, Signature};

/// The length of an ed25519 signature.
pub const ED25519_SIGNATURE_SIZE: usize = 64;

/// Signs live data with a swarm's private key.
#[derive(Clone, Debug)]
pub struct Ed25519Signer {
    signer: Signer,
}

impl Ed25519Signer {
    /// From a 64 byte private key (seed ‖ public key). A key whose public
    /// half doesn't match its seed is rejected, since its signatures would
    /// not verify against the swarm id.
    pub fn new(private_key: &[u8]) -> Result<Ed25519Signer, Error> {
        let signer = Signer::from_private(private_key).map_err(|_| Error::InvalidKey)?;
        Ok(Ed25519Signer { signer })
    }

    /// Signs `t ‖ p` in `domain`.
    pub fn sign(&self, domain: Domain, t: Time, p: &[u8]) -> Signature {
        let ts = timestamp(t);
        let sig = self.signer.sign(domain, &[&ts, p]);
        Signature::from_slice(&sig).unwrap_or(Signature::EMPTY)
    }
}

/// Verifies live data against a swarm's public key.
#[derive(Clone, Copy, Debug)]
pub struct Ed25519Verifier {
    verifier: Verifier,
}

impl Ed25519Verifier {
    /// From a 32 byte public key, the swarm id. Other lengths are
    /// rejected; a key that isn't a usable curve point verifies nothing.
    pub fn new(public_key: &[u8]) -> Result<Ed25519Verifier, Error> {
        let verifier = Verifier::new(public_key).map_err(|_| Error::InvalidKey)?;
        Ok(Ed25519Verifier { verifier })
    }

    /// Checks `sig` over `t ‖ p` in `domain`.
    pub fn verify(&self, domain: Domain, t: Time, p: &[u8], sig: &[u8]) -> bool {
        let ts = timestamp(t);
        self.verifier.verify(domain, &[&ts, p], sig)
    }
}

/// The signed timestamp: nanoseconds since the epoch, big-endian.
fn timestamp(t: Time) -> [u8; 8] {
    (t.unix_nanos() as u64).to_be_bytes()
}
