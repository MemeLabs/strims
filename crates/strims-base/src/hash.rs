// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The hasher of maps keyed by peers.
//!
//! An [`IndexMap`] iterates in insertion order whatever its hasher, but its
//! hashes decide which of its removals leave a tombstone, which its table
//! counts as taken until it next grows or rehashes, and so at which insert
//! the table grows. A node keys such maps by its peers (strims-ppspp's
//! channels), so a deployed build hashes with [`RandomState`], whose keys
//! std draws for each thread, and no peer can know which keys collide
//! (hash flooding). On wasm32-unknown-unknown, which has no entropy source,
//! std takes them from a stack and a heap address.
//!
//! A simulation counts its run's heap (strims-sim's `memory`), and under
//! `RandomState` the same run read a different heap from one process to
//! the next. The `fixed-hash` feature, which only a simulation's build
//! enables (strims-node's `turmoil`), hashes with std's [`DefaultHasher`]
//! at fixed keys instead, so a run's allocations follow its inserts and
//! removals alone. That hasher's algorithm is std's and can change with the
//! toolchain, which moves a run's heap but not its order.
//!
//! Cargo unifies features across the packages of one build, so a binary
//! built in one invocation with strims-sim (`--workspace`) hashes with the
//! fixed keys too: a deployed binary is built on its own.
//!
//! [`RandomState`]: std::hash::RandomState
//! [`DefaultHasher`]: std::hash::DefaultHasher

/// The hasher of [`IndexMap`] and [`IndexSet`]: `RandomState` in a
/// deployed build.
#[cfg(not(feature = "fixed-hash"))]
pub type MapHasher = std::hash::RandomState;

/// The hasher of [`IndexMap`] and [`IndexSet`]: std's `DefaultHasher` at
/// fixed keys in a simulation's build.
#[cfg(feature = "fixed-hash")]
pub type MapHasher = std::hash::BuildHasherDefault<std::hash::DefaultHasher>;

/// An insertion-ordered map hashed by [`MapHasher`]. Build one with
/// `default`; `new` is `RandomState`'s alone.
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, MapHasher>;

/// An insertion-ordered set hashed by [`MapHasher`]. Build one with
/// `default`; `new` is `RandomState`'s alone.
pub type IndexSet<K> = indexmap::IndexSet<K, MapHasher>;
