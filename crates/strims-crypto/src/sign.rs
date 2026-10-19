// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Domain-separated ed25519 signatures.
//!
//! Every signature names what it signs. The signed bytes are
//!
//! ```text
//! "strims\0" ‖ u8 len(context) ‖ context ‖ payload…
//! ```
//!
//! where the context is the [`Domain`]'s ASCII name and the payload is the
//! concatenation of the parts a caller passes. A signature made in one
//! domain never verifies in another, so one key may sign several kinds of
//! thing. Within a domain the payload's layout must be unambiguous: its
//! parts are fixed-size, or a single encoded message.
//!
//! Verification is strict: a public key or an `R` of small order verifies
//! nothing, a non-canonical `S` is rejected, and `R` is compared byte for
//! byte (RFC 8032 cofactorless verification). These are ed25519-dalek's
//! `verify_strict` rules.
//!
//! Signing and verifying frame the payload without copying it.

use core::fmt;

use curve25519_dalek::edwards::CompressedEdwardsY;
use ed25519_dalek::ed25519::signature::{MultipartSigner, MultipartVerifier};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use strims_proto::types::{Key, KeyType};

use crate::{ED25519_PRIVATE_KEY_SIZE, ED25519_PUBLIC_KEY_SIZE, ED25519_SIGNATURE_SIZE, Error};

/// What a signature signs. Each domain's payload is documented on its
/// variant; a new kind of signed object adds a domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    /// A certificate: its encoded `CertificateBody`.
    Certificate,
    /// A certificate request: its encoded `CertificateRequestBody`.
    CertificateRequest,
    /// A ppspp Merkle munro's root, live: `timestamp (u64 BE ns) ‖ root`.
    PpsppMunroRoot,
    /// A ppspp chunk, live: `timestamp (u64 BE ns) ‖ chunk`.
    PpsppChunk,
    /// A ppspp swarm epoch: `timestamp (u64 BE ns)`.
    PpsppEpoch,
    /// A video channel's stream key token: the 32-byte token.
    VideoStreamKey,
    /// A vpn message's trailer entry: header ‖ body ‖ earlier entries ‖
    /// time (i64 BE ns) ‖ host id, all of fixed or header-given length.
    VpnMessage,
    /// A vpn hash table record: its encoded body.
    VpnHashTableRecord,
    /// A vpn peer index record: its encoded body.
    VpnPeerIndexRecord,
}

impl Domain {
    /// Every domain, in declaration order.
    pub const ALL: [Domain; 9] = [
        Domain::Certificate,
        Domain::CertificateRequest,
        Domain::PpsppMunroRoot,
        Domain::PpsppChunk,
        Domain::PpsppEpoch,
        Domain::VideoStreamKey,
        Domain::VpnMessage,
        Domain::VpnHashTableRecord,
        Domain::VpnPeerIndexRecord,
    ];

    /// The domain's name in signed bytes. Contexts are lowercase ASCII and
    /// shorter than 256 bytes.
    pub const fn context(self) -> &'static str {
        match self {
            Domain::Certificate => "certificate",
            Domain::CertificateRequest => "certificate-request",
            Domain::PpsppMunroRoot => "ppspp-munro-root",
            Domain::PpsppChunk => "ppspp-chunk",
            Domain::PpsppEpoch => "ppspp-epoch",
            Domain::VideoStreamKey => "video-stream-key",
            Domain::VpnMessage => "vpn-message",
            Domain::VpnHashTableRecord => "vpn-hash-table-record",
            Domain::VpnPeerIndexRecord => "vpn-peer-index-record",
        }
    }
}

/// The prefix of every signed message.
const TAG: &[u8] = b"strims\0";

/// Payloads of up to this many parts are framed on the stack.
const INLINE_PARTS: usize = 6;

/// Calls `f` with the signed message as parts: the framing, then `payload`.
fn framed<R>(domain: Domain, payload: &[&[u8]], f: impl FnOnce(&[&[u8]]) -> R) -> R {
    let context = domain.context().as_bytes();
    // invariant: contexts are shorter than 256 bytes (see `Domain::context`).
    #[allow(clippy::cast_possible_truncation)]
    let len = [context.len() as u8];
    if payload.len() <= INLINE_PARTS {
        // Empty parts hash to nothing, so unused slots can stay empty.
        let mut parts: [&[u8]; 3 + INLINE_PARTS] = [&[]; 3 + INLINE_PARTS];
        for (slot, part) in parts
            .iter_mut()
            .zip([TAG, &len, context].iter().chain(payload))
        {
            *slot = part;
        }
        f(&parts)
    } else {
        let parts: Vec<&[u8]> = [TAG, &len[..], context]
            .into_iter()
            .chain(payload.iter().copied())
            .collect();
        f(&parts)
    }
}

/// Signs with an ed25519 key. Parse the key once and keep the signer.
#[derive(Clone)]
pub struct Signer {
    key: SigningKey,
}

impl fmt::Debug for Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signer")
            .field("public", &hex_prefix(&self.public()))
            .finish_non_exhaustive()
    }
}

fn hex_prefix(b: &[u8]) -> String {
    b.iter().take(4).map(|x| format!("{x:02x}")).collect()
}

impl Signer {
    /// The signer of an ed25519 [`Key`]. The type must be
    /// `KEY_TYPE_ED25519` ([`Error::UnsupportedKeyType`]); see
    /// [`Signer::from_private`] for the private key.
    pub fn new(key: &Key) -> Result<Signer, Error> {
        if key.r#type != KeyType::Ed25519 as i32 {
            return Err(Error::UnsupportedKeyType);
        }
        Signer::from_private(&key.private)
    }

    /// From a 64-byte expanded private key, seed ‖ public key
    /// ([`Error::InvalidKeyLength`]). A public half that doesn't match the
    /// seed is [`Error::KeyPairMismatch`], since its signatures wouldn't
    /// verify against it.
    pub fn from_private(private: &[u8]) -> Result<Signer, Error> {
        let bytes: &[u8; ED25519_PRIVATE_KEY_SIZE] =
            private.try_into().map_err(|_| Error::InvalidKeyLength)?;
        let key = SigningKey::from_keypair_bytes(bytes).map_err(|_| Error::KeyPairMismatch)?;
        Ok(Signer { key })
    }

    /// The public key.
    pub fn public(&self) -> [u8; ED25519_PUBLIC_KEY_SIZE] {
        self.key.verifying_key().to_bytes()
    }

    /// Signs the concatenation of `payload`'s parts in `domain`.
    pub fn sign(&self, domain: Domain, payload: &[&[u8]]) -> [u8; ED25519_SIGNATURE_SIZE] {
        framed(domain, payload, |parts| {
            MultipartSigner::<Signature>::multipart_sign(&self.key, parts).to_bytes()
        })
    }
}

/// Verifies signatures by one ed25519 public key. Parse the key once and
/// keep the verifier.
#[derive(Clone, Copy)]
pub struct Verifier {
    /// `None` for 32 bytes that aren't a curve point or have small order:
    /// every signature then fails to verify.
    key: Option<VerifyingKey>,
}

impl fmt::Debug for Verifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Verifier")
            .field("public", &self.key.map(|k| hex_prefix(k.as_bytes())))
            .finish()
    }
}

impl Verifier {
    /// From a 32-byte public key ([`Error::InvalidKeyLength`]). A key that
    /// isn't a curve point, or has small order, verifies nothing.
    pub fn new(public: &[u8]) -> Result<Verifier, Error> {
        let bytes: &[u8; ED25519_PUBLIC_KEY_SIZE] =
            public.try_into().map_err(|_| Error::InvalidKeyLength)?;
        Ok(Verifier {
            key: VerifyingKey::from_bytes(bytes)
                .ok()
                .filter(|k| !k.is_weak()),
        })
    }

    /// Whether `signature` signs the concatenation of `payload`'s parts in
    /// `domain`.
    pub fn verify(&self, domain: Domain, payload: &[&[u8]], signature: &[u8]) -> bool {
        let Some(key) = &self.key else {
            return false;
        };
        let Ok(signature) = Signature::from_slice(signature) else {
            return false;
        };
        let r = CompressedEdwardsY(*signature.r_bytes());
        if r.decompress().is_none_or(|r| r.is_small_order()) {
            return false;
        }
        framed(domain, payload, |parts| {
            key.multipart_verify(parts, &signature).is_ok()
        })
    }
}

/// [`Verifier::verify`] with a public key parsed for this one call. False
/// for a key that isn't 32 bytes.
pub fn verify(domain: Domain, public: &[u8], payload: &[&[u8]], signature: &[u8]) -> bool {
    Verifier::new(public).is_ok_and(|v| v.verify(domain, payload, signature))
}

#[cfg(test)]
mod tests {
    // Tests panic on failure by design.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use curve25519_dalek::constants::EIGHT_TORSION;

    use super::*;
    use crate::key_from_seed;

    fn signer() -> Signer {
        Signer::new(&key_from_seed(&[7; 32])).unwrap()
    }

    /// The signed message `framed` produces, as one buffer.
    fn message(domain: Domain, payload: &[&[u8]]) -> Vec<u8> {
        framed(domain, payload, |parts| parts.concat())
    }

    #[test]
    fn contexts_frame_in_one_length_byte() {
        let mut seen = Vec::new();
        for d in Domain::ALL {
            let c = d.context();
            assert!(c.len() < 256, "{d:?}");
            assert!(
                c.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
                "{d:?}"
            );
            assert!(!seen.contains(&c), "{d:?} shares its context");
            seen.push(c);
        }
        assert_eq!(
            message(Domain::PpsppEpoch, &[b"ab", b"c"]),
            b"strims\0\x0bppspp-epochabc"
        );
    }

    #[test]
    fn payload_parts_frame_as_their_concatenation() {
        let parts: Vec<&[u8]> = vec![b"a", b"bc", b"", b"d", b"e", b"f", b"g", b"h"];
        for n in 0..=parts.len() {
            let joined = parts[..n].concat();
            assert_eq!(
                message(Domain::PpsppChunk, &parts[..n]),
                message(Domain::PpsppChunk, &[&joined])
            );
        }
        let s = signer();
        let sig = s.sign(Domain::PpsppChunk, &parts);
        assert!(verify(
            Domain::PpsppChunk,
            &s.public(),
            &[&parts.concat()],
            &sig
        ));
    }

    #[test]
    fn signatures_verify_only_in_their_domain() {
        let s = signer();
        let sig = s.sign(Domain::PpsppMunroRoot, &[b"payload"]);
        let v = Verifier::new(&s.public()).unwrap();
        assert!(v.verify(Domain::PpsppMunroRoot, &[b"payload"], &sig));
        for d in Domain::ALL
            .into_iter()
            .filter(|&d| d != Domain::PpsppMunroRoot)
        {
            assert!(!v.verify(d, &[b"payload"], &sig), "{d:?}");
        }
        assert!(!v.verify(Domain::PpsppMunroRoot, &[b"payloae"], &sig));
        assert!(!v.verify(Domain::PpsppMunroRoot, &[b"payload"], &sig[..63]));
        // the framing isn't a prefix any payload can supply
        assert!(!v.verify(Domain::PpsppMunroRoot, &[], &sig));
    }

    #[test]
    fn keys_parse_strictly() {
        let k = key_from_seed(&[3; 32]);
        assert_eq!(
            Signer::from_private(&k.private[..63]).err(),
            Some(Error::InvalidKeyLength)
        );
        let mut mismatched = k.private.clone();
        mismatched[40] ^= 1;
        assert_eq!(
            Signer::from_private(&mismatched).err(),
            Some(Error::KeyPairMismatch)
        );
        let x = Key {
            r#type: KeyType::X25519 as i32,
            ..k.clone()
        };
        assert_eq!(Signer::new(&x).err(), Some(Error::UnsupportedKeyType));
        assert_eq!(
            Verifier::new(&k.public[..31]).err(),
            Some(Error::InvalidKeyLength)
        );
        assert_eq!(Signer::new(&k).unwrap().public().as_slice(), k.public);
    }

    /// Small-order `R` and public keys verify nothing, as with
    /// `verify_strict`.
    #[test]
    fn small_order_points_verify_nothing() {
        let s = signer();
        let payload: &[&[u8]] = &[b"payload"];
        let msg = message(Domain::PpsppChunk, payload);
        let sig = s.sign(Domain::PpsppChunk, payload);
        let strict = |public: &[u8; 32], sig: &[u8; 64]| {
            VerifyingKey::from_bytes(public)
                .is_ok_and(|k| k.verify_strict(&msg, &Signature::from_bytes(sig)).is_ok())
        };
        assert!(strict(&s.public(), &sig));
        assert!(verify(Domain::PpsppChunk, &s.public(), payload, &sig));

        for point in EIGHT_TORSION {
            let small = point.compress().to_bytes();
            // as the public key
            assert!(!strict(&small, &sig));
            assert!(!verify(Domain::PpsppChunk, &small, payload, &sig));
            // as R, with the real key and S
            let mut forged = sig;
            forged[..32].copy_from_slice(&small);
            assert!(!strict(&s.public(), &forged));
            assert!(!verify(Domain::PpsppChunk, &s.public(), payload, &forged));
        }
    }

    #[test]
    fn non_canonical_s_is_rejected() {
        let s = signer();
        let mut sig = s.sign(Domain::PpsppEpoch, &[]);
        // S + l is the same scalar, non-canonically encoded
        let l: [u8; 32] = [
            0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
            0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
        ];
        let mut carry = 0u16;
        for i in 0..32 {
            let v = u16::from(sig[32 + i]) + u16::from(l[i]) + carry;
            sig[32 + i] = v as u8;
            carry = v >> 8;
        }
        assert!(!verify(Domain::PpsppEpoch, &s.public(), &[], &sig));
    }
}
