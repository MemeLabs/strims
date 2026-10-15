// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! End-to-end message encryption.
//!
//! A message with [`crate::message::M_ENCRYPT`] carries
//! `nonce (12) | AES-256-GCM ciphertext | tag (16)` as its body. The key is
//! the raw X25519 of the sender's host key and the receiver's host id, both
//! converted from ed25519 (`strims_crypto::message_cipher_key`), so the two
//! ends derive the same key. There is no associated data.

use aes_gcm::aead::{AeadInOut, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce, Tag};
use strims_kademlia::ID;
use strims_proto::types::Key;

/// AES-GCM nonce size.
pub const NONCE_LEN: usize = 12;
/// AES-GCM tag size.
pub const TAG_LEN: usize = 16;
/// Bytes encryption adds to a body.
pub const CIPHER_OVERHEAD: usize = NONCE_LEN + TAG_LEN;

/// Cipher errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CipherError {
    /// The host key isn't a 64-byte ed25519 private key, or X25519 gave the
    /// all-zero output.
    #[error("message cipher key: {0}")]
    Key(#[from] strims_crypto::Error),
    /// Shorter than a nonce and a tag.
    #[error("encrypted body too short")]
    TooShort,
    /// Authentication failed.
    #[error("message authentication failed")]
    Open,
    #[error("message too long to encrypt")]
    Seal,
}

/// The AES-GCM cipher for one (host key, peer host id) pair.
#[derive(Clone)]
pub struct MessageCipher {
    aead: Aes256Gcm,
}

impl core::fmt::Debug for MessageCipher {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("MessageCipher")
    }
}

impl MessageCipher {
    /// The cipher between `host_key` and the host `peer`.
    pub fn new(host_key: &Key, peer: &ID) -> Result<MessageCipher, CipherError> {
        let key = strims_crypto::message_cipher_key(host_key, &peer.to_bytes())?;
        Ok(MessageCipher::from_key(&key))
    }

    /// A cipher for a raw 32-byte key.
    pub fn from_key(key: &[u8; 32]) -> MessageCipher {
        MessageCipher {
            aead: Aes256Gcm::new(key.into()),
        }
    }

    /// Appends `nonce | ciphertext | tag` for `plaintext` to `out`. A nonce
    /// must never repeat for the same pair of hosts.
    pub fn seal(
        &self,
        nonce: &[u8; NONCE_LEN],
        plaintext: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<(), CipherError> {
        let start = out.len();
        out.reserve(plaintext.len() + CIPHER_OVERHEAD);
        out.extend_from_slice(nonce);
        out.extend_from_slice(plaintext);
        let body = out.get_mut(start + NONCE_LEN..).ok_or(CipherError::Seal)?;
        match self
            .aead
            .encrypt_inout_detached(&Nonce::from(*nonce), &[], body.into())
        {
            Ok(tag) => {
                out.extend_from_slice(&tag);
                Ok(())
            }
            Err(_) => {
                out.truncate(start);
                Err(CipherError::Seal)
            }
        }
    }

    /// Appends the plaintext of a sealed body to `out`.
    pub fn open(&self, sealed: &[u8], out: &mut Vec<u8>) -> Result<(), CipherError> {
        if sealed.len() < CIPHER_OVERHEAD {
            return Err(CipherError::TooShort);
        }
        let (nonce, rest) = sealed.split_at(NONCE_LEN);
        let (ct, tag) = rest.split_at(rest.len() - TAG_LEN);
        let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| CipherError::TooShort)?;
        let tag: [u8; TAG_LEN] = tag.try_into().map_err(|_| CipherError::TooShort)?;
        let start = out.len();
        out.extend_from_slice(ct);
        let body = out.get_mut(start..).unwrap_or_default();
        match self.aead.decrypt_inout_detached(
            &Nonce::from(nonce),
            &[],
            body.into(),
            &Tag::from(tag),
        ) {
            Ok(()) => Ok(()),
            Err(_) => {
                out.truncate(start);
                Err(CipherError::Open)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_crypto::key_from_seed;

    fn id_of(k: &Key) -> ID {
        ID::unmarshal(&k.public).unwrap().0
    }

    /// B opens what A sealed for B; tampered and truncated bodies fail.
    #[test]
    fn message_cipher() {
        let a = key_from_seed(&[1; 32]);
        let b = key_from_seed(&[2; 32]);
        let c = MessageCipher::new(&a, &id_of(&b)).unwrap();
        let p = b"some test string";
        let mut s = Vec::new();
        c.seal(&[7; NONCE_LEN], p, &mut s).unwrap();
        assert_eq!(s.len(), p.len() + CIPHER_OVERHEAD);
        assert_ne!(&s[NONCE_LEN..NONCE_LEN + p.len()], p);
        let mut o = Vec::new();
        c.open(&s, &mut o).unwrap();
        assert_eq!(o, p);

        let mut s1 = Vec::new();
        c.seal(&[8; NONCE_LEN], p, &mut s1).unwrap();
        assert_ne!(s, s1);

        let rx = MessageCipher::new(&b, &id_of(&a)).unwrap();
        let mut o = Vec::new();
        rx.open(&s1, &mut o).unwrap();
        assert_eq!(o, p);

        // Tampering and truncation fail without panicking.
        let mut t = s1.clone();
        t[NONCE_LEN] ^= 1;
        let mut o = vec![9];
        assert_eq!(rx.open(&t, &mut o), Err(CipherError::Open));
        assert_eq!(o, vec![9]);
        assert_eq!(rx.open(&s1[..11], &mut o), Err(CipherError::TooShort));
        assert_eq!(
            rx.open(&s1[..CIPHER_OVERHEAD - 1], &mut o),
            Err(CipherError::TooShort)
        );
        // An empty plaintext is just a nonce and a tag.
        let mut e = Vec::new();
        c.seal(&[0; NONCE_LEN], b"", &mut e).unwrap();
        assert_eq!(e.len(), CIPHER_OVERHEAD);
        let mut o = Vec::new();
        rx.open(&e, &mut o).unwrap();
        assert!(o.is_empty());
    }
}
