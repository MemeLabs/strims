// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Signatures and checksums over a message's stable encoding.
//!
//! A signed message carries the signer's public key in its `key` field and
//! the ed25519 signature of its [`StableEncode`] encoding, with `signature`
//! cleared, in its `signature` field. [`SignableMessage`] covers every
//! message for which `strims-proto` generates [`Signable`]: those with plain
//! `bytes key` and `bytes signature` fields, such as
//! `network::v1::directory::ListingSnippet`.
//!
//! The stable encoding has no length prefixes, so a signature doesn't bind
//! field boundaries: bytes can move between adjacent string or bytes fields
//! under the same signature.

use ed25519_dalek::Signer;
use strims_proto::stable::{Signable, StableEncode};
use strims_proto::types::Key;

use crate::crc32::Crc32;
use crate::{ED25519_PUBLIC_KEY_SIZE, Error, key};

/// Signing and verifying a message in place.
pub trait SignableMessage {
    /// Sets the key field to `key.public` and the signature field to the
    /// ed25519 signature of the stable encoding.
    ///
    /// Fails without modifying the message if `key` isn't a 64-byte ed25519
    /// private key ([`Error::UnsupportedKeyType`],
    /// [`Error::InvalidKeyLength`], [`Error::KeyPairMismatch`]).
    fn sign_message(&mut self, key: &Key) -> Result<(), Error>;

    /// Checks the signature against the key field.
    fn verify_message(&self) -> Result<(), Error>;
}

impl<T: Signable> SignableMessage for T {
    fn sign_message(&mut self, k: &Key) -> Result<(), Error> {
        let signing = key::signing_key(k)?;
        self.set_key(&k.public);
        let b = self.stable_encode_unsigned_to_vec();
        self.set_signature(signing.sign(&b).to_bytes().to_vec());
        Ok(())
    }

    fn verify_message(&self) -> Result<(), Error> {
        if self.key().len() != ED25519_PUBLIC_KEY_SIZE {
            return Err(Error::InvalidKeyLength);
        }
        let b = self.stable_encode_unsigned_to_vec();
        if !key::verify(self.key(), &b, self.signature()) {
            return Err(Error::InvalidMessageSignature);
        }
        Ok(())
    }
}

/// CRC-32 (IEEE) of the stable encoding.
pub fn crc32_message<M: StableEncode + ?Sized>(m: &M) -> u32 {
    let mut c = Crc32::new();
    m.stable_encode_into(&mut c);
    c.sum()
}

#[cfg(test)]
mod tests {
    // Tests panic on failure by design.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use crate::key_from_seed;
    use strims_proto::network::v1::directory::ListingSnippet;

    #[test]
    fn sign_verify_round_trip() {
        let k = key_from_seed(&[5; 32]);
        let mut r = ListingSnippet {
            title: "t".into(),
            user_count: 7,
            start_time: 1234,
            ..Default::default()
        };
        r.sign_message(&k).unwrap();
        assert_eq!(r.key, k.public);
        assert_eq!(r.signature.len(), 64);
        r.verify_message().unwrap();

        // re-signing a signed message ignores the old signature
        let sig = r.signature.clone();
        r.sign_message(&k).unwrap();
        assert_eq!(r.signature, sig);

        r.user_count = 8;
        assert_eq!(r.verify_message(), Err(Error::InvalidMessageSignature));
        r.key.pop();
        assert_eq!(r.verify_message(), Err(Error::InvalidKeyLength));
    }

    #[test]
    fn sign_rejects_bad_keys() {
        let k = key_from_seed(&[5; 32]);
        let mut r = ListingSnippet::default();
        let short = Key {
            private: k.private[..60].to_vec(),
            ..k.clone()
        };
        assert_eq!(r.sign_message(&short), Err(Error::InvalidKeyLength));
        let x = Key { r#type: 2, ..k };
        assert_eq!(r.sign_message(&x), Err(Error::UnsupportedKeyType));
        assert!(r.key.is_empty() && r.signature.is_empty());
    }

    #[test]
    fn snippet_signature_covers_nested_fields() {
        let k = key_from_seed(&[6; 32]);
        let mut s = ListingSnippet {
            title: "a".into(),
            tags: vec!["x".into()],
            ..Default::default()
        };
        s.sign_message(&k).unwrap();
        s.verify_message().unwrap();
        let mut t = s.clone();
        t.tags.push("y".into());
        assert!(t.verify_message().is_err());
        // moving bytes between adjacent string fields is not detected: the
        // stable encoding has no length prefixes
        let mut u = s.clone();
        u.title = String::new();
        u.description = "a".into();
        assert!(u.verify_message().is_ok());
    }

    #[test]
    fn crc32_of_empty_message() {
        assert_eq!(
            crc32_message(&strims_proto::types::Cache::default()),
            crc32_of(&[0u8; 8])
        );
    }

    fn crc32_of(b: &[u8]) -> u32 {
        use strims_proto::stable::StableSink;
        let mut c = Crc32::new();
        c.put(b);
        c.sum()
    }
}
