// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Keys, certificates, signed messages and key agreement.
//!
//! - [`key`]: ed25519 keys.
//! - [`sign`]: domain-separated ed25519 signatures.
//! - [`signed`]: signed objects that travel as an encoded body and its
//!   signature.
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
pub mod sign;
pub mod signable;
pub mod signed;

pub use cert::{
    CERT_PREDATE_DURATION, CertificateChain, CertificateErrors, DEFAULT_CERT_TTL, Issuer,
    certificate_body, certificate_parent, certificate_root, host_certificate, membership_trusted,
    network_certificate, new_certificate_request, new_self_signed_certificate,
    sign_certificate_request, sign_certificate_request_with_network, verify_certificate,
    verify_certificate_request,
};
pub use curve25519::{
    X25519_SIZE, aes_link_key, key_to_curve25519, message_cipher_key, private_key_to_curve25519,
    public_key_to_curve25519, x25519,
};
pub use key::{
    ED25519_PRIVATE_KEY_SIZE, ED25519_PUBLIC_KEY_SIZE, ED25519_SEED_SIZE, ED25519_SIGNATURE_SIZE,
    generate_key, key_from_seed, sign, verify,
};
pub use sign::{Domain, Signer, Verifier};
pub use signable::{SignableMessage, crc32_message};
pub use signed::{Envelope, SelfKeyed, SignedBody, Verified};

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
    /// A certificate's issuer isn't the signing key, or a parent isn't the
    /// certificate its child's body names as issuer.
    #[error("certificate issuer mismatch")]
    IssuerMismatch,
    /// A certificate names an issuer its chain doesn't carry.
    #[error("certificate issuer missing")]
    MissingIssuer,
    /// A certificate chain longer than [`cert::MAX_CHAIN_LENGTH`].
    #[error("certificate chain too long")]
    ChainTooLong,
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
    /// A signed object's body doesn't decode.
    #[error("malformed signed body")]
    MalformedBody,
    /// A 64-byte private key whose public half doesn't match its seed.
    #[error("private key does not match its public key")]
    KeyPairMismatch,
    /// X25519 produced the all-zero output: the peer's point has low order.
    #[error("bad input point: low order point")]
    LowOrderPoint,
}
