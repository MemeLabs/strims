// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A swarm's epoch is the time its writer started (or last reset), signed
//! by the swarm key. Handshakes carry it; a peer that sees a newer valid
//! epoch resets its copy of the swarm.

use strims_base::time::Time;
use strims_proto::types::cache;

use crate::integrity::SignatureVerifier;

/// Epoch errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EpochError {
    /// The offered epoch is older than ours.
    #[error("epoch is out of date")]
    OutOfDate,
    /// The swarm key didn't sign the offered epoch.
    #[error("signature mismatch")]
    InvalidSignature,
}

/// The swarm's current epoch: a timestamp and its signature.
#[derive(Clone, Debug)]
pub struct Epoch {
    verifier: SignatureVerifier,
    timestamp: Time,
    signature: Vec<u8>,
}

impl Epoch {
    /// No epoch yet: a nil timestamp and a zeroed signature.
    pub(crate) fn new(verifier: SignatureVerifier) -> Epoch {
        let signature = vec![0; verifier.size()];
        Epoch {
            verifier,
            timestamp: Time::NIL,
            signature,
        }
    }

    /// Loads the epoch from a cache. The signature is not checked; a
    /// missing epoch imports as time 0.
    pub fn import_cache(&mut self, c: Option<&cache::Epoch>) {
        self.timestamp = Time::from_unix_nanos(c.map_or(0, |c| c.timestamp));
        let mut sig = vec![0; self.verifier.size()];
        if let Some(c) = c {
            let n = sig.len().min(c.signature.len());
            if let (Some(dst), Some(src)) = (sig.get_mut(..n), c.signature.get(..n)) {
                dst.copy_from_slice(src);
            }
        }
        self.signature = sig;
    }

    /// The epoch for a cache.
    pub fn export_cache(&self) -> cache::Epoch {
        cache::Epoch {
            timestamp: self.timestamp.unix_nanos(),
            signature: self.signature.clone(),
        }
    }

    /// The timestamp and signature.
    pub fn value(&self) -> (Time, &[u8]) {
        (self.timestamp, &self.signature)
    }

    /// Adopts `t` if it is newer than the current epoch and `sig` signs it.
    /// Returns true if the epoch changed.
    pub fn sync(&mut self, t: Time, sig: &[u8]) -> Result<bool, EpochError> {
        if t == self.timestamp {
            return Ok(false);
        }
        if t < self.timestamp {
            return Err(EpochError::OutOfDate);
        }
        if !self.verifier.verify(t, &[], sig, &mut Vec::new()) {
            return Err(EpochError::InvalidSignature);
        }
        self.timestamp = t;
        self.signature = sig.to_vec();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::integrity::LiveSignatureAlgorithm;

    #[test]
    fn sync() {
        let key = strims_crypto::key_from_seed(&[8; 32]);
        let alg = LiveSignatureAlgorithm::ED25519;
        let signer = alg.signer(&key.private).unwrap();
        let mut e = Epoch::new(alg.verifier(&key.public).unwrap());
        assert!(e.value().0.is_nil());
        assert_eq!(e.value().1, [0; 64]);

        let t1 = Time::from_unix_millis(1000);
        let s1 = signer.sign(t1, &[], &mut Vec::new());
        assert_eq!(e.sync(t1, s1.as_slice()), Ok(true));
        assert_eq!(e.sync(t1, &[]), Ok(false));
        let t0 = Time::from_unix_millis(999);
        assert_eq!(e.sync(t0, s1.as_slice()), Err(EpochError::OutOfDate));
        let t2 = Time::from_unix_millis(1001);
        assert_eq!(e.sync(t2, s1.as_slice()), Err(EpochError::InvalidSignature));
        assert_eq!(e.value(), (t1, s1.as_slice()));

        let mut f = Epoch::new(alg.verifier(&key.public).unwrap());
        f.import_cache(Some(&e.export_cache()));
        assert_eq!(f.value(), e.value());
        f.import_cache(None);
        assert_eq!(f.value(), (Time::EPOCH, &[0u8; 64][..]));
    }
}
