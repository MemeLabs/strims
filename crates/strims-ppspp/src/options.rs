// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Swarm options.
//!
//! A zero field means "use the default": [`Swarm::new`]
//! fills zero fields from [`SwarmOptions::default_options`], so
//! `SwarmOptions { chunk_size: 2048, ..Default::default() }` is a swarm
//! with default options except for the chunk size.
//!
//! [`Swarm::new`]: crate::Swarm::new

use crate::codec::ProtocolOptionType;
use crate::controller::RequestControllerKind;
use crate::integrity::{SwarmVerifierOptions, VerifierOptions};
use crate::store::BufferLayout;
use crate::uri::UriOptions;

/// Whether the node publishes the swarm (`SEED`) or receives it (`PEER`).
/// Both run the peer scheduler. A seed's scheduler treats the first chunk
/// as set from the start, and in [`PushMode::Hybrid`] a seed never
/// subscribes and may forward every stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulingMethod(pub u8);

impl SchedulingMethod {
    pub const SEED: SchedulingMethod = SchedulingMethod(1);
    pub const PEER: SchedulingMethod = SchedulingMethod(2);
}

/// Where a receiver's read position starts once it has its first chunks:
/// `LOW_LATENCY` at the newest chunk it has, `BEST_EFFORT` half a window
/// behind that, `MANDATORY` at the start of the stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeliveryMode(pub u8);

impl DeliveryMode {
    pub const LOW_LATENCY: DeliveryMode = DeliveryMode(1);
    pub const BEST_EFFORT: DeliveryMode = DeliveryMode(2);
    pub const MANDATORY: DeliveryMode = DeliveryMode(3);
}

/// How the scheduler mixes push into pull; see
/// [`scheduler_peer::push`](crate::scheduler_peer::push). Local to this
/// node: every mode speaks the same v2 messages, so nodes in different
/// modes interoperate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PushMode {
    /// Pull only; HAVEs and the requests they trigger wait for the next
    /// tick.
    #[default]
    Off,
    /// Pull only, but HAVEs go out as soon as chunks are stored and
    /// requests as soon as HAVEs make bins requestable (coalesced per frame
    /// handled).
    EagerHave,
    /// `EagerHave` plus push subscriptions (STREAM_REQUEST/OPEN) with
    /// per-bin deadlines that release missed chunks to pull, parent
    /// selection, the Thicket one-interior-stream constraint and upload
    /// admission control.
    Hybrid,
}

/// Swarm options. Sizes are in bytes and counts in chunks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SwarmOptions {
    pub label: String,
    pub chunk_size: usize,
    pub chunks_per_signature: usize,
    pub stream_count: usize,
    /// The live window in chunks; a power of two.
    pub live_window: usize,
    pub integrity: VerifierOptions,
    pub scheduling_method: SchedulingMethod,
    pub delivery_mode: DeliveryMode,
    pub buffer_layout: BufferLayout,
    /// How push is mixed into pull; local, in neither handshakes nor
    /// URIs.
    pub push_mode: PushMode,
    /// The channels' request window model, local to this node.
    /// `Default` resolves to [`RequestControllerKind::DEFAULT`].
    pub request_controller: RequestControllerKind,
}

impl SwarmOptions {
    /// The defaults: 1 KiB chunks, 64 per signature, one
    /// stream, a 64 Ki chunk (64 MiB) window, Merkle/BLAKE2b-256/ed25519,
    /// the peer scheduler, low latency delivery and a circular buffer.
    pub fn default_options() -> SwarmOptions {
        SwarmOptions {
            label: String::new(),
            chunk_size: 1024,
            chunks_per_signature: 64,
            stream_count: 1,
            live_window: 1 << 16,
            integrity: VerifierOptions::default_options(),
            scheduling_method: SchedulingMethod::PEER,
            delivery_mode: DeliveryMode::LOW_LATENCY,
            buffer_layout: BufferLayout::CIRCULAR,
            push_mode: PushMode::Off,
            request_controller: RequestControllerKind::DEFAULT,
        }
    }

    /// Each zero field of `self` replaced by the field of `defaults`,
    /// recursing into `integrity`.
    pub fn assign_defaults(self, defaults: &SwarmOptions) -> SwarmOptions {
        fn or<T: Default + PartialEq>(v: T, d: T) -> T {
            if v == T::default() { d } else { v }
        }
        let d = defaults.clone();
        SwarmOptions {
            label: or(self.label, d.label),
            chunk_size: or(self.chunk_size, d.chunk_size),
            chunks_per_signature: or(self.chunks_per_signature, d.chunks_per_signature),
            stream_count: or(self.stream_count, d.stream_count),
            live_window: or(self.live_window, d.live_window),
            integrity: VerifierOptions {
                protection_method: or(
                    self.integrity.protection_method,
                    d.integrity.protection_method,
                ),
                merkle_hash_tree_function: or(
                    self.integrity.merkle_hash_tree_function,
                    d.integrity.merkle_hash_tree_function,
                ),
                live_signature_algorithm: or(
                    self.integrity.live_signature_algorithm,
                    d.integrity.live_signature_algorithm,
                ),
            },
            scheduling_method: or(self.scheduling_method, d.scheduling_method),
            delivery_mode: or(self.delivery_mode, d.delivery_mode),
            buffer_layout: or(self.buffer_layout, d.buffer_layout),
            push_mode: or(self.push_mode, d.push_mode),
            request_controller: or(self.request_controller, d.request_controller),
        }
    }

    /// The swarm verifier's options for these swarm options.
    pub fn integrity_verifier_options(&self) -> SwarmVerifierOptions {
        SwarmVerifierOptions {
            live_discard_window: self.live_window,
            chunk_size: self.chunk_size,
            chunks_per_signature: self.chunks_per_signature,
            verifier_options: self.integrity,
        }
    }

    /// The options a URI carries.
    pub fn uri_options(&self) -> UriOptions {
        let mut o = UriOptions::default();
        o.insert(ProtocolOptionType::ChunkSize, self.chunk_size as u64);
        o.insert(
            ProtocolOptionType::ChunksPerSignature,
            self.chunks_per_signature as u64,
        );
        o.insert(ProtocolOptionType::StreamCount, self.stream_count as u64);
        o.insert(
            ProtocolOptionType::ContentIntegrityProtectionMethod,
            u64::from(self.integrity.protection_method.0),
        );
        o.insert(
            ProtocolOptionType::MerkleHashTreeFunction,
            u64::from(self.integrity.merkle_hash_tree_function.0),
        );
        o.insert(
            ProtocolOptionType::LiveSignatureAlgorithm,
            u64::from(self.integrity.live_signature_algorithm.0),
        );
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod};

    /// Defaults fill zero fields only, the request controller included.
    #[test]
    fn swarm_options() {
        let defaults = SwarmOptions::default_options();
        assert_eq!(SwarmOptions::default().assign_defaults(&defaults), defaults);

        let custom = SwarmOptions {
            label: "test".into(),
            chunk_size: 2048,
            chunks_per_signature: 16,
            stream_count: 16,
            live_window: 1 << 12,
            scheduling_method: SchedulingMethod::PEER,
            integrity: VerifierOptions {
                protection_method: ProtectionMethod::SIGN_ALL,
                merkle_hash_tree_function: MerkleHashTreeFunction::SHA256,
                live_signature_algorithm: LiveSignatureAlgorithm::ED25519,
            },
            delivery_mode: DeliveryMode::MANDATORY,
            buffer_layout: BufferLayout::CIRCULAR,
            push_mode: PushMode::Hybrid,
            request_controller: RequestControllerKind::BbrLike,
        };
        assert_eq!(custom.clone().assign_defaults(&defaults), custom);
        // Both controllers can be chosen whichever is the default.
        for kind in [
            RequestControllerKind::Ledbat,
            RequestControllerKind::BbrLike,
        ] {
            let o = SwarmOptions {
                request_controller: kind,
                ..Default::default()
            }
            .assign_defaults(&defaults);
            assert_eq!(o.request_controller, kind);
        }
        assert_eq!(
            SwarmOptions::default()
                .assign_defaults(&defaults)
                .request_controller,
            RequestControllerKind::DEFAULT
        );
    }

    #[test]
    fn partial_options_take_defaults_per_field() {
        let o = SwarmOptions {
            chunk_size: 64,
            integrity: VerifierOptions {
                protection_method: ProtectionMethod::SIGN_ALL,
                ..Default::default()
            },
            ..Default::default()
        }
        .assign_defaults(&SwarmOptions::default_options());
        assert_eq!(o.chunk_size, 64);
        assert_eq!(o.live_window, 1 << 16);
        assert_eq!(o.integrity.protection_method, ProtectionMethod::SIGN_ALL);
        assert_eq!(
            o.integrity.merkle_hash_tree_function,
            MerkleHashTreeFunction::BLAKE2B256
        );
    }
}
