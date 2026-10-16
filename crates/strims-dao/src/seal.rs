// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Encryption at rest.
//!
//! A [`StorageKey`] seals values with AES-256-GCM as
//! `nonce (12) ‖ ciphertext ‖ tag (16)`, with no associated data. The
//! nonce is random, so a key must only ever seal with a rng seeded from
//! OS entropy: a rng replayed from a fixed seed repeats nonces under the
//! same key, which breaks GCM.
//!
//! Password keys come from PBKDF2-HMAC-SHA256 with the parameters stored
//! beside the sealed value in a `profile.v1.StorageKey` record.

use std::fmt;

use aes_gcm::aead::{AeadInOut, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce, Tag};
use rand_core::Rng;
use sha2::Sha256;
use strims_base::rng::DetRng;
use strims_proto::profile::v1::{self as profile, KdfType, storage_key};

/// Size of a storage key.
pub const KEY_SIZE: usize = 32;
/// Size of the nonce that starts a sealed value.
pub const NONCE_SIZE: usize = 12;
/// Size of the tag that ends a sealed value.
pub const TAG_SIZE: usize = 16;
/// Bytes sealing adds to a value.
pub const SEAL_OVERHEAD: usize = NONCE_SIZE + TAG_SIZE;

/// PBKDF2 iterations for new password keys.
pub const PBKDF2_ITERATIONS: u32 = 100_000;
/// Salt size for new password keys.
pub const PBKDF2_SALT_SIZE: usize = 16;
/// The most PBKDF2 iterations a stored record may ask for. Records arrive
/// in pasted pairing tokens, so this bounds the work one sign-in costs.
pub const MAX_PBKDF2_ITERATIONS: u32 = 10_000_000;
/// The longest PBKDF2 salt a stored record may carry.
pub const MAX_PBKDF2_SALT_SIZE: usize = 1024;

/// A key's parameters can't produce a storage key.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    /// The record names a key derivation function this build lacks.
    #[error("unsupported key derivation function")]
    UnsupportedKdf,
    /// The record's parameters are out of range.
    #[error("invalid key derivation parameters")]
    InvalidParams,
}

/// A sealed value can't be opened, or a value can't be sealed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SealError {
    /// The value is shorter than a nonce and a tag.
    #[error("sealed value is truncated")]
    Truncated,
    /// The value wasn't sealed with this key, or was altered.
    #[error("sealed value failed authentication")]
    Authentication,
    /// The value is longer than AES-GCM can seal.
    #[error("value is too long to seal")]
    TooLong,
}

/// An AES-256-GCM key for values at rest.
#[derive(Clone)]
pub struct StorageKey {
    key: [u8; KEY_SIZE],
    aead: Aes256Gcm,
}

impl fmt::Debug for StorageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StorageKey(..)")
    }
}

impl StorageKey {
    /// The key `key`.
    pub fn from_bytes(key: [u8; KEY_SIZE]) -> StorageKey {
        StorageKey {
            aead: Aes256Gcm::new(&key.into()),
            key,
        }
    }

    /// The key `record` derives from `password`. Derivation takes as long
    /// as the record's iteration count asks, up to
    /// [`MAX_PBKDF2_ITERATIONS`], on the calling thread.
    pub fn from_password(
        password: &str,
        record: &profile::StorageKey,
    ) -> Result<StorageKey, KeyError> {
        if record.kdf_type != KdfType::Pbkdf2Sha256 as i32 {
            return Err(KeyError::UnsupportedKdf);
        }
        let Some(storage_key::KdfOptions::Pbkdf2Options(o)) = &record.kdf_options else {
            return Err(KeyError::UnsupportedKdf);
        };
        if o.key_size as usize != KEY_SIZE
            || o.iterations == 0
            || o.iterations > MAX_PBKDF2_ITERATIONS
            || o.salt.len() > MAX_PBKDF2_SALT_SIZE
        {
            return Err(KeyError::InvalidParams);
        }
        let mut key = [0u8; KEY_SIZE];
        pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), &o.salt, o.iterations, &mut key);
        Ok(StorageKey::from_bytes(key))
    }

    /// A new password key with a random salt, and the record that derives
    /// it again.
    pub fn new_password_key(password: &str, rng: &mut DetRng) -> (StorageKey, profile::StorageKey) {
        let mut salt = vec![0u8; PBKDF2_SALT_SIZE];
        rng.fill_bytes(&mut salt);
        let mut key = [0u8; KEY_SIZE];
        pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut key);
        let record = profile::StorageKey {
            kdf_type: KdfType::Pbkdf2Sha256 as i32,
            kdf_options: Some(storage_key::KdfOptions::Pbkdf2Options(
                storage_key::Pbkdf2Options {
                    iterations: PBKDF2_ITERATIONS,
                    key_size: KEY_SIZE as u32,
                    salt,
                },
            )),
        };
        (StorageKey::from_bytes(key), record)
    }

    /// The key's bytes.
    pub fn bytes(&self) -> &[u8; KEY_SIZE] {
        &self.key
    }

    /// Seals `plaintext` under a nonce drawn from `rng`.
    pub fn seal(&self, rng: &mut DetRng, plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
        let mut nonce = [0u8; NONCE_SIZE];
        rng.fill_bytes(&mut nonce);
        self.seal_with_nonce(nonce, plaintext)
    }

    fn seal_with_nonce(
        &self,
        nonce: [u8; NONCE_SIZE],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, SealError> {
        let mut out = Vec::with_capacity(SEAL_OVERHEAD + plaintext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(plaintext);
        let body = out.get_mut(NONCE_SIZE..).unwrap_or_default();
        let tag = self
            .aead
            .encrypt_inout_detached(&Nonce::from(nonce), &[], body.into())
            .map_err(|_| SealError::TooLong)?;
        out.extend_from_slice(&tag);
        Ok(out)
    }

    /// The plaintext of a value [`seal`](Self::seal) produced.
    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, SealError> {
        let (nonce, rest) = sealed
            .split_at_checked(NONCE_SIZE)
            .ok_or(SealError::Truncated)?;
        let split = rest
            .len()
            .checked_sub(TAG_SIZE)
            .ok_or(SealError::Truncated)?;
        let (body, tag) = rest.split_at_checked(split).ok_or(SealError::Truncated)?;
        let nonce: [u8; NONCE_SIZE] = nonce.try_into().map_err(|_| SealError::Truncated)?;
        let tag: [u8; TAG_SIZE] = tag.try_into().map_err(|_| SealError::Truncated)?;
        let mut out = body.to_vec();
        self.aead
            .decrypt_inout_detached(
                &Nonce::from(nonce),
                &[],
                out.as_mut_slice().into(),
                &Tag::from(tag),
            )
            .map_err(|_| SealError::Authentication)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_base::rng::det_rng;

    use super::*;

    fn record(iterations: u32, key_size: u32, salt: &[u8]) -> profile::StorageKey {
        profile::StorageKey {
            kdf_type: KdfType::Pbkdf2Sha256 as i32,
            kdf_options: Some(storage_key::KdfOptions::Pbkdf2Options(
                storage_key::Pbkdf2Options {
                    iterations,
                    key_size,
                    salt: salt.to_vec(),
                },
            )),
        }
    }

    #[test]
    fn pbkdf2_sha256_known_answer() {
        // RFC 7914 §11, truncated to 32 bytes.
        let k = StorageKey::from_password("passwd", &record(1, 32, b"salt")).unwrap();
        assert_eq!(
            hex::encode(k.bytes()),
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc"
        );
    }

    #[test]
    fn password_keys_derive_again_from_their_record() {
        let mut rng = det_rng(1);
        let (k, r) = StorageKey::new_password_key("hunter2", &mut rng);
        let again = StorageKey::from_password("hunter2", &r).unwrap();
        assert_eq!(k.bytes(), again.bytes());
        let other = StorageKey::from_password("hunter3", &r).unwrap();
        assert_ne!(k.bytes(), other.bytes());
    }

    #[test]
    fn records_out_of_range_are_rejected() {
        let bad = [
            record(0, 32, b"s"),
            record(MAX_PBKDF2_ITERATIONS + 1, 32, b"s"),
            record(1, 16, b"s"),
            record(1, 32, &[0; MAX_PBKDF2_SALT_SIZE + 1]),
        ];
        for r in &bad {
            assert_eq!(
                StorageKey::from_password("p", r).unwrap_err(),
                KeyError::InvalidParams
            );
        }
        let mut r = record(1, 32, b"s");
        r.kdf_type = 0;
        assert_eq!(
            StorageKey::from_password("p", &r).unwrap_err(),
            KeyError::UnsupportedKdf
        );
        let r = profile::StorageKey {
            kdf_type: KdfType::Pbkdf2Sha256 as i32,
            kdf_options: None,
        };
        assert_eq!(
            StorageKey::from_password("p", &r).unwrap_err(),
            KeyError::UnsupportedKdf
        );
    }

    #[test]
    fn seal_round_trips_and_authenticates() {
        let mut rng = det_rng(2);
        let k = StorageKey::from_bytes([7; KEY_SIZE]);
        for len in [0, 1, 15, 16, 17, 1000] {
            let pt: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let sealed = k.seal(&mut rng, &pt).unwrap();
            assert_eq!(sealed.len(), pt.len() + SEAL_OVERHEAD);
            assert_eq!(k.open(&sealed).unwrap(), pt);

            let mut tampered = sealed.clone();
            let last = tampered.len() - 1;
            tampered[last] ^= 1;
            assert_eq!(k.open(&tampered), Err(SealError::Authentication));
        }
        let other = StorageKey::from_bytes([8; KEY_SIZE]);
        let sealed = k.seal(&mut rng, b"x").unwrap();
        assert_eq!(other.open(&sealed), Err(SealError::Authentication));
    }

    #[test]
    fn short_values_fail_to_open() {
        let k = StorageKey::from_bytes([7; KEY_SIZE]);
        for len in 0..SEAL_OVERHEAD {
            assert_eq!(k.open(&vec![0; len]), Err(SealError::Truncated));
        }
    }

    #[test]
    fn nonces_are_drawn_per_seal() {
        let mut rng = det_rng(3);
        let k = StorageKey::from_bytes([7; KEY_SIZE]);
        let a = k.seal(&mut rng, b"same").unwrap();
        let b = k.seal(&mut rng, b"same").unwrap();
        assert_ne!(a[..NONCE_SIZE], b[..NONCE_SIZE]);
        assert_eq!(
            k.seal_with_nonce([1; 12], b"same"),
            k.seal_with_nonce([1; 12], b"same")
        );
    }
}
