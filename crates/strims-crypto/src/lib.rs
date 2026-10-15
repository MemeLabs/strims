// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Keys, certificates, signed messages and key agreement.
//!
//! - [`key`]: ed25519 keys, signing and verification.
//! - [`cert`]: certificate requests, issuing and chain verification.
//! - [`signable`]: signatures and checksums over a message's stable encoding.
//! - [`curve25519`]: ed25519 to X25519 conversion and the X25519 key
//!   agreements of vnic links and vpn message ciphers.
//!
//! Key formats, signed byte layouts and derived keys are wire formats: every
//! node must produce and accept the same bytes. `tests/golden.rs` pins them.
//!
//! Nothing here reads the clock or OS entropy: validity checks take
//! `now: Time` and key or serial generation takes `&mut DetRng`.

pub mod cert;
mod crc32;
pub mod curve25519;
mod field;
pub mod key;
pub mod signable;

pub use cert::{
    CERT_PREDATE_DURATION, CertificateErrors, DEFAULT_CERT_TTL, cert_is_expired,
    certificate_network_key, certificate_parent, certificate_root, new_certificate_request,
    new_self_signed_certificate, serialize_certificate, serialize_certificate_request,
    sign_certificate_request, verify_certificate, verify_certificate_request,
};
pub use curve25519::{
    X25519_SIZE, aes_link_key, key_to_curve25519, message_cipher_key, private_key_to_curve25519,
    public_key_to_curve25519, x25519,
};
pub use key::{
    ED25519_PRIVATE_KEY_SIZE, ED25519_PUBLIC_KEY_SIZE, ED25519_SEED_SIZE, ED25519_SIGNATURE_SIZE,
    generate_key, key_from_seed, sign, signing_key, signing_key_from_private, verify,
};
pub use signable::{SignableMessage, crc32_message};

/// Re-exported so callers can cache a [`ed25519_dalek::SigningKey`] from
/// [`signing_key`] without depending on a matching version.
pub use ed25519_dalek;

/// Errors from key, certificate and message operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The key's type isn't one the operation supports.
    #[error("unsupported key type")]
    UnsupportedKeyType,
    /// A key has the wrong length.
    #[error("invalid key length")]
    InvalidKeyLength,
    /// A request asks for a usage outside the allowed set, or a certificate's
    /// signer lacks `KEY_USAGE_SIGN`.
    #[error("unsupported key usage")]
    UnsupportedKeyUsage,
    /// A certificate request isn't signed by its own key.
    #[error("invalid certificate request signature")]
    InvalidCertificateRequestSignature,
    /// A certificate isn't signed by its parent, or by itself if it has none.
    #[error("invalid certificate signature")]
    InvalidSignature,
    /// `now` is before a certificate's validity period.
    #[error("not before limit exceeded")]
    NotBeforeRange,
    /// `now` is after a certificate's validity period.
    #[error("not after limit exceeded")]
    NotAfterRange,
    /// A signed message's signature doesn't verify against its key field.
    #[error("invalid signature")]
    InvalidMessageSignature,
    /// A 64-byte private key whose public half doesn't match its seed.
    #[error("private key does not match its public key")]
    KeyPairMismatch,
    /// X25519 produced the all-zero output: the peer's point has low order.
    #[error("bad input point: low order point")]
    LowOrderPoint,
}
