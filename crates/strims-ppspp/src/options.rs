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
use crate::integrity::{SwarmVerifierOptions, VerifierOptions};
use crate::store::{BufferLayout, ChunkBytes};
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
    /// The crate's default, resolved by `SwarmOptions::default_options`
    /// (zero means default, as for every other option).
    #[default]
    Default,
    /// Pull only; HAVEs and the requests they trigger wait for the next
    /// tick.
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

impl PushMode {
    /// The mode `Default` resolves to.
    pub const DEFAULT: PushMode = PushMode::Hybrid;

    /// `Default` resolved.
    pub fn resolve(self) -> PushMode {
        match self {
            PushMode::Default => PushMode::DEFAULT,
            m => m,
        }
    }
}

/// How a seed in [`PushMode::Hybrid`] spends its upload; see
/// [`scheduler_peer::push`](crate::scheduler_peer::push). Local to this
/// node, like `push_mode`; a non-seed ignores it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeedPolicy {
    pub allocation: SeedAllocation,
    pub qos: SeedQos,
}

impl SeedPolicy {
    /// The admitting seed, with equal uplink shares.
    pub const ADMISSION: SeedPolicy = SeedPolicy {
        allocation: SeedAllocation::Admission,
        qos: SeedQos::Equal,
    };
    /// The adaptive seed, with uplink shares by allocation.
    pub const ADAPTIVE: SeedPolicy = SeedPolicy {
        allocation: SeedAllocation::Adaptive,
        qos: SeedQos::ByAllocation,
    };

    /// Each `Default` field resolved.
    pub fn resolve(self) -> SeedPolicy {
        SeedPolicy {
            allocation: self.allocation.resolve(),
            qos: self.qos.resolve(),
        }
    }
}

/// Which push subscriptions a seed accepts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeedAllocation {
    /// The crate's default, resolved by `SwarmOptions::default_options`.
    #[default]
    Default,
    /// First come, while the projected upload stays under the admission
    /// target.
    Admission,
    /// The whole copies its measured upload pays for, to the neighbours
    /// nearest it; see
    /// [`scheduler_peer::push::seed`](crate::scheduler_peer::push::seed).
    Adaptive,
}

impl SeedAllocation {
    /// The allocation `Default` resolves to.
    pub const DEFAULT: SeedAllocation = SeedAllocation::Adaptive;

    /// `Default` resolved.
    pub fn resolve(self) -> SeedAllocation {
        match self {
            SeedAllocation::Default => SeedAllocation::DEFAULT,
            a => a,
        }
    }
}

/// How a seed weights its peers' shares of its uplink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeedQos {
    /// The crate's default, resolved by `SwarmOptions::default_options`.
    #[default]
    Default,
    /// By reciprocity, like any node: a seed receives no data, so every
    /// peer gets an equal share.
    Equal,
    /// Each peer at least in proportion to the streams pushed to it, and
    /// the minimum weight for peers it pushes nothing to: pushes leave at
    /// the rate each subscriber needs, and pull answers take what they
    /// leave.
    ByAllocation,
}

impl SeedQos {
    /// The shares `Default` resolves to.
    pub const DEFAULT: SeedQos = SeedQos::ByAllocation;

    /// `Default` resolved.
    pub fn resolve(self) -> SeedQos {
        match self {
            SeedQos::Default => SeedQos::DEFAULT,
            q => q,
        }
    }
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
    /// What the store keeps of each chunk, local to this node: its bytes
    /// (`Default`), or, for simulations, a codec's few bytes of state it
    /// makes the chunk from again ([`ChunkBytes::Codec`]).
    pub chunk_bytes: ChunkBytes,
    /// How push is mixed into pull; local, in neither handshakes nor
    /// URIs. `Default` resolves to [`PushMode::DEFAULT`].
    pub push_mode: PushMode,
    /// How a hybrid seed spends its upload, local to this node. Each
    /// `Default` field resolves to its type's `DEFAULT`.
    pub seed: SeedPolicy,
}

impl SwarmOptions {
    /// The defaults: 1 KiB chunks, 64 per signature, one
    /// stream, a 64 Ki chunk (64 MiB) window, Merkle/BLAKE2b-256/ed25519,
    /// the peer scheduler, low latency delivery, a circular buffer, and
    /// hybrid push with the adaptive seed.
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
            chunk_bytes: ChunkBytes::Stored,
            push_mode: PushMode::DEFAULT,
            seed: SeedPolicy {
                allocation: SeedAllocation::DEFAULT,
                qos: SeedQos::DEFAULT,
            },
        }
    }

    /// Each zero field of `self` replaced by the field of `defaults`,
    /// recursing into `integrity` and `seed`.
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
            chunk_bytes: or(self.chunk_bytes, d.chunk_bytes),
            push_mode: or(self.push_mode, d.push_mode),
            seed: SeedPolicy {
                allocation: or(self.seed.allocation, d.seed.allocation),
                qos: or(self.seed.qos, d.seed.qos),
            },
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

    /// Defaults fill zero fields only.
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
            chunk_bytes: ChunkBytes::Stored,
            push_mode: PushMode::Hybrid,
            seed: SeedPolicy {
                allocation: SeedAllocation::Adaptive,
                qos: SeedQos::ByAllocation,
            },
        };
        assert_eq!(custom.clone().assign_defaults(&defaults), custom);
    }

    /// Every push mode and seed policy can be chosen whichever is the
    /// default, a policy field by field.
    #[test]
    fn explicit_push_modes_and_seed_policies_survive_defaults() {
        let defaults = SwarmOptions::default_options();
        let o = SwarmOptions::default().assign_defaults(&defaults);
        assert_eq!(o.push_mode, PushMode::DEFAULT);
        assert_eq!(
            o.seed,
            SeedPolicy {
                allocation: SeedAllocation::DEFAULT,
                qos: SeedQos::DEFAULT,
            }
        );
        for mode in [PushMode::Off, PushMode::EagerHave, PushMode::Hybrid] {
            let o = SwarmOptions {
                push_mode: mode,
                ..Default::default()
            }
            .assign_defaults(&defaults);
            assert_eq!(o.push_mode, mode);
        }
        for seed in [SeedPolicy::ADMISSION, SeedPolicy::ADAPTIVE] {
            let o = SwarmOptions {
                seed,
                ..Default::default()
            }
            .assign_defaults(&defaults);
            assert_eq!(o.seed, seed);
        }
        let o = SwarmOptions {
            seed: SeedPolicy {
                allocation: SeedAllocation::Admission,
                qos: SeedQos::Default,
            },
            ..Default::default()
        }
        .assign_defaults(&defaults);
        assert_eq!(o.seed.allocation, SeedAllocation::Admission);
        assert_eq!(o.seed.qos, SeedQos::DEFAULT);
        assert_ne!(PushMode::DEFAULT, PushMode::Default);
        assert_ne!(SeedAllocation::DEFAULT, SeedAllocation::Default);
        assert_ne!(SeedQos::DEFAULT, SeedQos::Default);
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
