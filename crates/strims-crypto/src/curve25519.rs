// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! ed25519 to X25519 conversion, and the X25519 key agreements of vnic TCP
//! links and vpn message ciphers.
//!
//! The raw X25519 output is the AES-256 key, with no KDF; both ends of a
//! link must derive the same bytes. The ciphers live in the vnic and vpn
//! crates.

use sha2::{Digest, Sha512};
use strims_proto::types::{Key, KeyType};

use crate::field::Fe;
use crate::{ED25519_PRIVATE_KEY_SIZE, ED25519_PUBLIC_KEY_SIZE, Error};

/// X25519 scalar, point and shared-secret size.
pub const X25519_SIZE: usize = 32;

/// Converts an ed25519 private key (seed || public) to a clamped X25519
/// private key: the first half of SHA-512(seed), clamped. Only the seed is
/// read.
pub fn private_key_to_curve25519(
    ed25519_private: &[u8; ED25519_PRIVATE_KEY_SIZE],
) -> [u8; X25519_SIZE] {
    let digest = Sha512::digest(&ed25519_private[..32]);
    let mut out = [0u8; X25519_SIZE];
    out.iter_mut().zip(digest.iter()).for_each(|(o, d)| *o = *d);
    clamp(&mut out);
    out
}

fn clamp(k: &mut [u8; X25519_SIZE]) {
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;
}

/// Converts an ed25519 public key to the X25519 public key of the same
/// point, u = (1 + y) / (1 - y) (RFC 7748 section 4.1).
///
/// The input isn't validated: bit 255 (the sign of x) is ignored,
/// non-canonical y is reduced, the result for an off-curve y is whatever the
/// formula gives, and y = 1 maps to u = 0. Peers derive keys from each
/// other's host IDs, so every 32-byte input must map to the same u on every
/// node.
pub fn public_key_to_curve25519(
    ed25519_public: &[u8; ED25519_PUBLIC_KEY_SIZE],
) -> [u8; X25519_SIZE] {
    let y = Fe::from_bytes(ed25519_public);
    let num = Fe::ONE.add(y);
    let den = Fe::ONE.sub(y).invert();
    num.mul(den).to_bytes()
}

/// Converts an ed25519 [`Key`] to an X25519 [`Key`] (type
/// `KEY_TYPE_X25519`) with a 32-byte clamped private key. A private key
/// that isn't 64 bytes or a public key that isn't 32 is
/// [`Error::InvalidKeyLength`]. The key type isn't checked.
pub fn key_to_curve25519(k: &Key) -> Result<Key, Error> {
    let private: &[u8; ED25519_PRIVATE_KEY_SIZE] = k
        .private
        .as_slice()
        .try_into()
        .map_err(|_| Error::InvalidKeyLength)?;
    let public: &[u8; ED25519_PUBLIC_KEY_SIZE] = k
        .public
        .as_slice()
        .try_into()
        .map_err(|_| Error::InvalidKeyLength)?;
    Ok(Key {
        r#type: KeyType::X25519 as i32,
        private: private_key_to_curve25519(private).to_vec(),
        public: public_key_to_curve25519(public).to_vec(),
    })
}

/// X25519 scalar multiplication. Both inputs must be 32 bytes
/// ([`Error::InvalidKeyLength`]), and an all-zero result is
/// [`Error::LowOrderPoint`]. The scalar is clamped and bit 255 of the point
/// is ignored.
pub fn x25519(scalar: &[u8], point: &[u8]) -> Result<[u8; X25519_SIZE], Error> {
    let scalar: [u8; X25519_SIZE] = scalar.try_into().map_err(|_| Error::InvalidKeyLength)?;
    let point: [u8; X25519_SIZE] = point.try_into().map_err(|_| Error::InvalidKeyLength)?;
    let out = x25519_dalek::x25519(scalar, point);
    if out == [0u8; X25519_SIZE] {
        return Err(Error::LowOrderPoint);
    }
    Ok(out)
}

/// The AES-256-CFB key of a vnic TCP link: X25519 of our X25519 private key
/// (from [`key_to_curve25519`] of the profile key) and the peer's X25519
/// public key from its `AESLinkInit`.
///
/// A dialer must first check the peer's `AESLinkInit.key` against
/// [`public_key_to_curve25519`] of the ed25519 key in the dialed URI.
pub fn aes_link_key(
    local_x25519: &Key,
    peer_x25519_public: &[u8],
) -> Result<[u8; X25519_SIZE], Error> {
    x25519(&local_x25519.private, peer_x25519_public)
}

/// The AES-256-GCM key of a vpn message cipher: X25519 of our ed25519 host
/// key converted to X25519 and the peer host ID (an ed25519 public key)
/// converted to X25519.
///
/// The host key's private key must be 64 bytes ([`Error::InvalidKeyLength`]);
/// only its seed is read.
pub fn message_cipher_key(
    host_key: &Key,
    peer_host_id: &[u8; ED25519_PUBLIC_KEY_SIZE],
) -> Result<[u8; X25519_SIZE], Error> {
    let private: &[u8; ED25519_PRIVATE_KEY_SIZE] = host_key
        .private
        .as_slice()
        .try_into()
        .map_err(|_| Error::InvalidKeyLength)?;
    x25519(
        &private_key_to_curve25519(private),
        &public_key_to_curve25519(peer_host_id),
    )
}

#[cfg(test)]
mod tests {
    // Tests panic on failure by design.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use crate::key_from_seed;
    use strims_base::rng::det_rng;

    #[test]
    fn ed25519_to_curve25519_matches_base_mult() {
        let mut rng = det_rng(1);
        for _ in 0..32 {
            let k = crate::generate_key(&mut rng);
            let c = key_to_curve25519(&k).unwrap();
            let public: [u8; 32] = c.public.as_slice().try_into().unwrap();
            let private: [u8; 32] = c.private.as_slice().try_into().unwrap();
            assert_eq!(
                x25519_dalek::x25519(private, x25519_dalek::X25519_BASEPOINT_BYTES),
                public
            );
        }
    }

    #[test]
    fn public_conversion_matches_dalek_for_valid_points() {
        let mut rng = det_rng(2);
        for _ in 0..64 {
            let k = crate::generate_key(&mut rng);
            let public: [u8; 32] = k.public.as_slice().try_into().unwrap();
            let vk = ed25519_dalek::VerifyingKey::from_bytes(&public).unwrap();
            assert_eq!(
                public_key_to_curve25519(&public),
                vk.to_montgomery().to_bytes()
            );
        }
    }

    #[test]
    fn identity_maps_to_zero() {
        let mut y1 = [0u8; 32];
        y1[0] = 1;
        assert_eq!(public_key_to_curve25519(&y1), [0u8; 32]);
    }

    #[test]
    fn shared_secrets_agree() {
        let a = key_from_seed(&[1; 32]);
        let b = key_from_seed(&[2; 32]);
        let (ca, cb) = (
            key_to_curve25519(&a).unwrap(),
            key_to_curve25519(&b).unwrap(),
        );
        let ab = aes_link_key(&ca, &cb.public).unwrap();
        let ba = aes_link_key(&cb, &ca.public).unwrap();
        assert_eq!(ab, ba);

        let b_id: [u8; 32] = b.public.as_slice().try_into().unwrap();
        let a_id: [u8; 32] = a.public.as_slice().try_into().unwrap();
        let m_ab = message_cipher_key(&a, &b_id).unwrap();
        assert_eq!(m_ab, message_cipher_key(&b, &a_id).unwrap());
        // both derivations reach the same secret
        assert_eq!(m_ab, ab);
    }

    #[test]
    fn x25519_rejects_bad_input() {
        let k = [7u8; 32];
        assert_eq!(x25519(&k, &[0u8; 32]), Err(Error::LowOrderPoint));
        let mut one = [0u8; 32];
        one[0] = 1;
        assert_eq!(x25519(&k, &one), Err(Error::LowOrderPoint));
        assert_eq!(x25519(&k[..31], &one), Err(Error::InvalidKeyLength));
        assert_eq!(x25519(&k, &[9u8; 33]), Err(Error::InvalidKeyLength));
        assert!(
            key_to_curve25519(&Key {
                r#type: 1,
                private: vec![0; 63],
                public: vec![0; 32]
            })
            .is_err()
        );
    }
}
