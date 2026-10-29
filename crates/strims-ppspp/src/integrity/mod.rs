// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Content integrity: Merkle trees, per-chunk signatures, or nothing.
//!
//! A [`SwarmVerifier`] holds what a swarm has verified (signed munro roots
//! and their trees for Merkle, chunk signatures for sign-all), sends it to
//! peers ([`SwarmVerifier::write_integrity`]) and checks their data against
//! it. Each peer channel collects the integrity messages that precede a
//! chunk in its own [`ChannelVerifier`].
//!
//! # Ownership
//!
//! The swarm owns its [`SwarmVerifier`], each channel owns a
//! [`ChannelVerifier`], and the calls that need both take the swarm
//! verifier as an argument ([`ChunkVerifier::verify`]).
//!
//! # Signatures
//!
//! A live signature covers the big-endian `u64` timestamp in nanoseconds
//! followed by the payload, in the payload's domain: a munro's root digest
//! ([`Domain::PpsppMunroRoot`], Merkle), a chunk's data
//! ([`Domain::PpsppChunk`], sign-all), or nothing ([`Domain::PpsppEpoch`]).
//! Neither payload covers the bin, so a valid munro or chunk can be replayed
//! at another position.
//!
//! # Peer input
//!
//! Bins from the wire are checked before they index anything: a Merkle
//! channel only accepts bins inside the munro it is verifying, sign-all
//! keeps at most a live window of pending signatures, and data must be
//! exactly `base_length * chunk_size` bytes. The segment and signature rings
//! advance by at most one window per call, however far a bin jumps.

mod ed25519;
mod merkle;
mod none;
mod sign_all;

#[cfg(test)]
mod merkle_test;
#[cfg(test)]
mod sign_all_test;

use strims_base::time::Time;
use strims_binmap::{Bin, Map};
use strims_crypto::Domain;
use strims_proto::types::{Cache, cache};

use crate::codec::{self, MessageWriter};

pub use ed25519::{ED25519_SIGNATURE_SIZE, Ed25519Signer, Ed25519Verifier};
pub use merkle::{MerkleChannelVerifier, MerkleOptions, MerkleSwarmVerifier};
pub use none::NONE_SIGNATURE_SIZE;
pub use sign_all::{SignAllChannelVerifier, SignAllOptions, SignAllSwarmVerifier};

/// Integrity errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No stored tree covers the bin, so its integrity can't be sent.
    #[error("missing hash subtree")]
    MissingHashSubtree,
    /// A signature of the wrong length.
    #[error("signature too short")]
    SignatureTooShort,
    /// A munro signature that doesn't verify.
    #[error("signature mismatch")]
    InvalidSignature,
    /// A bin outside the munro being verified or the live window.
    #[error("bin out of bounds")]
    BinOutOfBounds,
    /// No signature is stored or pending for a chunk.
    #[error("missing chunk signature")]
    MissingChunkSignature,
    /// A chunk signature that doesn't verify.
    #[error("invalid chunk signature")]
    InvalidChunkSignature,
    /// Data that isn't exactly the bin's chunks.
    #[error("data length does not match bin")]
    DataLength,
    #[error("unsupported protection method")]
    UnsupportedProtectionMethod,
    #[error("unsupported hash tree function")]
    UnsupportedHashTreeFunction,
    #[error("unsupported live signature algorithm")]
    UnsupportedSignatureAlgorithm,
    #[error("invalid integrity options")]
    InvalidOptions,
    #[error("invalid key")]
    InvalidKey,
    /// A channel verifier used with a swarm verifier of another method.
    #[error("verifier method mismatch")]
    VerifierMismatch,
    #[error("no supported integrity cache found")]
    NoIntegrityCache,
    #[error("integrity cache incomplete")]
    IntegrityCacheIncomplete,
    #[error("integrity cache larger than the live window")]
    CacheTooLarge,
    #[error(transparent)]
    Merkle(#[from] crate::merkle::Error),
    #[error(transparent)]
    Codec(#[from] codec::Error),
}

impl Error {
    /// True if a message writer ran out of space.
    pub fn is_not_enough_space(&self) -> bool {
        matches!(self, Error::Codec(codec::Error::NotEnoughSpace))
    }
}

macro_rules! wire_value {
    ($(#[$meta:meta])* $name:ident { $( $(#[$cmeta:meta])* $c:ident = $v:literal, )* }) => {
        $(#[$meta])*
        ///
        /// A `u8` as on the wire; 0 means unset (filled from defaults) and
        /// other undefined values are representable, so a peer's handshake
        /// value is compared exactly as sent.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u8);

        impl $name {
            $( $(#[$cmeta])* pub const $c: $name = $name($v); )*
        }
    };
}

wire_value! {
    /// How a swarm protects its content.
    ProtectionMethod {
        NONE = 1,
        MERKLE_TREE = 2,
        SIGN_ALL = 3,
    }
}

wire_value! {
    /// The hash function of a swarm's Merkle trees.
    MerkleHashTreeFunction {
        SHA1 = 1,
        SHA256 = 2,
        SHA512 = 3,
        BLAKE2B256 = 4,
        BLAKE2B512 = 5,
        /// Defined on the wire but unsupported.
        MD5 = 6,
        BLAKE3 = 7,
        SHA3_224 = 8,
        SHA3_256 = 9,
        SHA3_384 = 10,
        SHA3_512 = 11,
    }
}

wire_value! {
    /// How a swarm signs live data.
    LiveSignatureAlgorithm {
        NONE = 1,
        ED25519 = 2,
    }
}

impl MerkleHashTreeFunction {
    /// A fresh hasher, or `None` if unsupported.
    pub fn hasher(self) -> Option<crate::merkle::Hasher> {
        use crate::merkle::Hasher;
        Some(match self {
            Self::SHA1 => Hasher::sha1(),
            Self::SHA256 => Hasher::sha256(),
            Self::SHA512 => Hasher::sha512(),
            Self::BLAKE2B256 => Hasher::blake2b256(),
            Self::BLAKE2B512 => Hasher::blake2b512(),
            Self::BLAKE3 => Hasher::blake3(),
            Self::SHA3_224 => Hasher::sha3_224(),
            Self::SHA3_256 => Hasher::sha3_256(),
            Self::SHA3_384 => Hasher::sha3_384(),
            Self::SHA3_512 => Hasher::sha3_512(),
            _ => return None,
        })
    }

    /// The digest length, or `None` if unsupported.
    pub fn hash_size(self) -> Option<usize> {
        Some(match self {
            Self::SHA1 => 20,
            Self::SHA256 | Self::BLAKE2B256 | Self::BLAKE3 | Self::SHA3_256 => 32,
            Self::SHA3_224 => 28,
            Self::SHA3_384 => 48,
            Self::SHA512 | Self::BLAKE2B512 | Self::SHA3_512 => 64,
            _ => return None,
        })
    }
}

impl LiveSignatureAlgorithm {
    /// The signature length, or `None` if unsupported.
    pub fn signature_size(self) -> Option<usize> {
        match self {
            Self::NONE => Some(NONE_SIGNATURE_SIZE),
            Self::ED25519 => Some(ED25519_SIGNATURE_SIZE),
            _ => None,
        }
    }

    /// A signer for a 64 byte ed25519 private key (seed ‖ public key).
    pub fn signer(self, private_key: &[u8]) -> Result<SignatureSigner, Error> {
        match self {
            Self::NONE => Ok(SignatureSigner::None),
            Self::ED25519 => Ok(SignatureSigner::Ed25519(Ed25519Signer::new(private_key)?)),
            _ => Err(Error::UnsupportedSignatureAlgorithm),
        }
    }

    /// A verifier for a 32 byte ed25519 public key.
    pub fn verifier(self, public_key: &[u8]) -> Result<SignatureVerifier, Error> {
        match self {
            Self::NONE => Ok(SignatureVerifier::None),
            Self::ED25519 => Ok(SignatureVerifier::Ed25519(Ed25519Verifier::new(
                public_key,
            )?)),
            _ => Err(Error::UnsupportedSignatureAlgorithm),
        }
    }
}

/// The integrity settings of a swarm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VerifierOptions {
    pub protection_method: ProtectionMethod,
    pub merkle_hash_tree_function: MerkleHashTreeFunction,
    pub live_signature_algorithm: LiveSignatureAlgorithm,
}

impl VerifierOptions {
    /// Merkle trees of BLAKE3 with ed25519 signatures. In the browser
    /// worker, BLAKE3's wasm SIMD backend hashes a 4 KiB chunk's four 1 KiB
    /// blocks side by side: twice BLAKE2b-256's speed in V8 and four times
    /// in SpiderMonkey. Without SIMD it is half BLAKE2b's.
    pub fn default_options() -> VerifierOptions {
        VerifierOptions {
            protection_method: ProtectionMethod::MERKLE_TREE,
            merkle_hash_tree_function: MerkleHashTreeFunction::BLAKE3,
            live_signature_algorithm: LiveSignatureAlgorithm::ED25519,
        }
    }
}

/// The swarm settings a [`SwarmVerifier`] is built from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwarmVerifierOptions {
    /// The live window in chunks.
    pub live_discard_window: usize,
    pub chunk_size: usize,
    pub chunks_per_signature: usize,
    pub verifier_options: VerifierOptions,
}

/// Bytes for the largest supported signature.
pub const MAX_SIGNATURE_SIZE: usize = 64;

/// A signature stored inline: ed25519 or empty.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature {
    len: u8,
    bytes: [u8; MAX_SIGNATURE_SIZE],
}

impl Signature {
    pub const EMPTY: Signature = Signature {
        len: 0,
        bytes: [0; MAX_SIGNATURE_SIZE],
    };

    /// `None` if `b` is longer than [`MAX_SIGNATURE_SIZE`].
    pub fn from_slice(b: &[u8]) -> Option<Signature> {
        let mut s = Signature::EMPTY;
        s.bytes.get_mut(..b.len())?.copy_from_slice(b);
        s.len = b.len() as u8;
        Some(s)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.bytes.get(..usize::from(self.len)).unwrap_or_default()
    }
}

impl core::fmt::Debug for Signature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Signature({} bytes)", self.len)
    }
}

/// Signs live data.
// One per writer; boxing the key would only add an indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum SignatureSigner {
    None,
    Ed25519(Ed25519Signer),
}

impl SignatureSigner {
    /// Signs `t ‖ p` in `domain`.
    pub fn sign(&self, domain: Domain, t: Time, p: &[u8]) -> Signature {
        match self {
            SignatureSigner::None => Signature::EMPTY,
            SignatureSigner::Ed25519(s) => s.sign(domain, t, p),
        }
    }

    /// The signature length.
    pub fn size(&self) -> usize {
        match self {
            SignatureSigner::None => NONE_SIGNATURE_SIZE,
            SignatureSigner::Ed25519(_) => ED25519_SIGNATURE_SIZE,
        }
    }
}

/// Verifies live signatures. The ed25519 key is decompressed once, when
/// the verifier is created.
#[derive(Clone, Debug)]
pub enum SignatureVerifier {
    /// Accepts everything.
    None,
    Ed25519(Ed25519Verifier),
}

impl SignatureVerifier {
    /// Checks `sig` over `t ‖ p` in `domain`.
    pub fn verify(&self, domain: Domain, t: Time, p: &[u8], sig: &[u8]) -> bool {
        match self {
            SignatureVerifier::None => true,
            SignatureVerifier::Ed25519(v) => v.verify(domain, t, p, sig),
        }
    }

    /// The signature length.
    pub fn size(&self) -> usize {
        match self {
            SignatureVerifier::None => NONE_SIGNATURE_SIZE,
            SignatureVerifier::Ed25519(_) => ED25519_SIGNATURE_SIZE,
        }
    }
}

/// A swarm's integrity state: the signatures and hashes of the chunks it
/// holds, which it sends to peers and checks peer data against.
// One per swarm, created with it; boxing would only add an indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum SwarmVerifier {
    None,
    Merkle(MerkleSwarmVerifier),
    SignAll(SignAllSwarmVerifier),
}

impl SwarmVerifier {
    /// A verifier for the swarm's protection method.
    pub fn new(v: SignatureVerifier, o: &SwarmVerifierOptions) -> Result<SwarmVerifier, Error> {
        match o.verifier_options.protection_method {
            ProtectionMethod::NONE => Ok(SwarmVerifier::None),
            ProtectionMethod::MERKLE_TREE => Ok(SwarmVerifier::Merkle(MerkleSwarmVerifier::new(
                &MerkleOptions {
                    live_discard_window: o.live_discard_window,
                    chunk_size: o.chunk_size,
                    chunks_per_signature: o.chunks_per_signature,
                    verifier: v,
                    hash: o.verifier_options.merkle_hash_tree_function,
                },
            )?)),
            ProtectionMethod::SIGN_ALL => Ok(SwarmVerifier::SignAll(SignAllSwarmVerifier::new(
                &SignAllOptions {
                    live_discard_window: o.live_discard_window,
                    chunk_size: o.chunk_size,
                    verifier: v,
                },
            )?)),
            _ => Err(Error::UnsupportedProtectionMethod),
        }
    }

    /// The first bin this verifier can still check. A chunk's integrity
    /// carries its munro's signature only while the receiver has announced
    /// none of the munro's chunks, so a chunk whose munro has left the ring
    /// can't be verified any more. Sign-all chunks carry their own
    /// signatures.
    pub fn verifiable_from(&self) -> Bin {
        match self {
            SwarmVerifier::Merkle(v) => v.verifiable_from(),
            SwarmVerifier::None | SwarmVerifier::SignAll(_) => Bin(0),
        }
    }

    /// Writes the integrity messages a peer whose have map is `peer_have`
    /// needs to verify `b`: for Merkle, the signed munro root unless the
    /// peer has part of the munro, then uncle hashes up to the first
    /// ancestor the peer has data under; for sign-all, a signature per
    /// chunk.
    pub fn write_integrity(
        &self,
        b: Bin,
        peer_have: &Map,
        w: &mut impl MessageWriter,
    ) -> Result<usize, Error> {
        match self {
            SwarmVerifier::None => Ok(0),
            SwarmVerifier::Merkle(v) => v.write_integrity(b, peer_have, w),
            SwarmVerifier::SignAll(v) => v.write_integrity(b, w),
        }
    }

    /// A verifier for one peer channel.
    pub fn channel_verifier(&self) -> ChannelVerifier {
        match self {
            SwarmVerifier::None => ChannelVerifier::None,
            SwarmVerifier::Merkle(v) => ChannelVerifier::Merkle(v.channel_verifier()),
            SwarmVerifier::SignAll(v) => ChannelVerifier::SignAll(v.channel_verifier()),
        }
    }

    /// Loads and checks signatures from a cache. Returns how many bytes of
    /// the cache's data they cover: all of it when the swarm has no
    /// integrity protection.
    pub fn import_cache(&mut self, c: &Cache) -> Result<usize, Error> {
        match self {
            SwarmVerifier::None => Ok(c.data.len()),
            SwarmVerifier::Merkle(v) => v.import_cache(c),
            SwarmVerifier::SignAll(v) => v.import_cache(c),
        }
    }

    /// The signatures for a cache.
    pub fn export_cache(&self) -> cache::Integrity {
        match self {
            SwarmVerifier::None => cache::Integrity::default(),
            SwarmVerifier::Merkle(v) => v.export_cache(),
            SwarmVerifier::SignAll(v) => v.export_cache(),
        }
    }

    /// Forgets everything.
    pub fn reset(&mut self) {
        match self {
            SwarmVerifier::None => {}
            SwarmVerifier::Merkle(v) => v.reset(),
            SwarmVerifier::SignAll(v) => v.reset(),
        }
    }
}

/// Per-channel verification state: the integrity messages a peer sent for
/// the data that follows.
// One per peer channel, created with it.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ChannelVerifier {
    None,
    Merkle(MerkleChannelVerifier),
    SignAll(SignAllChannelVerifier),
}

impl ChannelVerifier {
    /// The verifier for messages about `b`. For Merkle this switches to
    /// `b`'s munro, discarding hashes collected for another one.
    pub fn chunk_verifier(&mut self, b: Bin) -> ChunkVerifier<'_> {
        if let ChannelVerifier::Merkle(v) = self {
            v.select(b);
        }
        ChunkVerifier(self)
    }
}

/// Collects a peer's integrity messages for a bin and checks data against
/// them.
#[derive(Debug)]
pub struct ChunkVerifier<'c>(&'c mut ChannelVerifier);

impl ChunkVerifier<'_> {
    /// A signed munro root (Merkle) or chunk signature (sign-all).
    pub fn set_signed_integrity(&mut self, b: Bin, t: Time, sig: &[u8]) {
        match self.0 {
            ChannelVerifier::None => {}
            ChannelVerifier::Merkle(v) => v.set_signed_integrity(t, sig),
            ChannelVerifier::SignAll(v) => v.set_signed_integrity(b, t, sig),
        }
    }

    /// An uncle hash (Merkle only).
    pub fn set_integrity(&mut self, b: Bin, hash: &[u8]) {
        if let ChannelVerifier::Merkle(v) = self.0 {
            v.set_integrity(b, hash);
        }
    }

    /// Checks `d`, the data of `b`, and on success records what it proved
    /// in `sv` so later chunks and peers can use it.
    pub fn verify(&mut self, sv: &mut SwarmVerifier, b: Bin, d: &[u8]) -> Result<(), Error> {
        match (&mut *self.0, sv) {
            (ChannelVerifier::None, SwarmVerifier::None) => Ok(()),
            (ChannelVerifier::Merkle(v), SwarmVerifier::Merkle(sv)) => v.verify(sv, b, d),
            (ChannelVerifier::SignAll(v), SwarmVerifier::SignAll(sv)) => v.verify(sv, b, d),
            _ => Err(Error::VerifierMismatch),
        }
    }
}

/// `n` as a ring size: a power of two of at least 1.
fn ring_size(n: usize) -> Result<u64, Error> {
    if n == 0 || !n.is_power_of_two() {
        return Err(Error::InvalidOptions);
    }
    u64::try_from(n).map_err(|_| Error::InvalidOptions)
}
