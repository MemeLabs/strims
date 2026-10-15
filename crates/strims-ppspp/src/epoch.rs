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

    /// Loads the epoch from a cache. The swarm key must have signed it; a
    /// missing or nil epoch imports as no epoch. A refused epoch leaves the
    /// current one in place.
    pub fn import_cache(&mut self, c: Option<&cache::Epoch>) -> Result<(), EpochError> {
        let t = c.map_or(Time::NIL, |c| Time::from_unix_nanos(c.timestamp));
        if t.is_nil() {
            self.timestamp = Time::NIL;
            self.signature = vec![0; self.verifier.size()];
            return Ok(());
        }
        let sig = c.map_or(&[][..], |c| &c.signature);
        if sig.len() != self.verifier.size() || !self.verifier.verify(t, &[], sig, &mut Vec::new())
        {
            return Err(EpochError::InvalidSignature);
        }
        self.timestamp = t;
        self.signature = sig.to_vec();
        Ok(())
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
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

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
    }

    #[test]
    fn import_cache_requires_the_swarm_signature() {
        let key = strims_crypto::key_from_seed(&[9; 32]);
        let alg = LiveSignatureAlgorithm::ED25519;
        let signer = alg.signer(&key.private).unwrap();
        let mut e = Epoch::new(alg.verifier(&key.public).unwrap());
        let t1 = Time::from_unix_millis(1000);
        let s1 = signer.sign(t1, &[], &mut Vec::new());
        assert_eq!(e.sync(t1, s1.as_slice()), Ok(true));
        let good = e.export_cache();

        let mut f = Epoch::new(alg.verifier(&key.public).unwrap());
        assert_eq!(f.import_cache(Some(&good)), Ok(()));
        assert_eq!(f.value(), e.value());

        // forged or damaged epochs are refused and change nothing
        let later = cache::Epoch {
            timestamp: Time::from_unix_millis(1001).unix_nanos(),
            ..good.clone()
        };
        let mut flipped = good.clone();
        flipped.signature[0] ^= 1;
        let mut short = good.clone();
        short.signature.pop();
        for bad in [later, flipped, short] {
            assert_eq!(
                f.import_cache(Some(&bad)),
                Err(EpochError::InvalidSignature)
            );
            assert_eq!(f.value(), e.value());
        }

        // no epoch, or a nil one, imports as no epoch
        let nil = cache::Epoch {
            timestamp: Time::NIL.unix_nanos(),
            ..good
        };
        for c in [None, Some(&nil)] {
            f.import_cache(Some(&e.export_cache())).unwrap();
            assert_eq!(f.import_cache(c), Ok(()));
            assert_eq!(f.value(), (Time::NIL, &[0u8; 64][..]));
        }
    }
}
