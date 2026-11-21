//! The hasher of turmoil's maps.
//!
//! A simulation's state lives on its thread, so a caller counting that
//! thread's allocations counts its maps'. A map hashed by `RandomState`
//! takes keys the OS draws for each thread, and they decide which of its
//! removals leave a tombstone, which a table counts as taken until it next
//! grows or rehashes: a stream's reorder buffer (`StreamSocket::buf`) grew
//! at a different insert from one process to the next, and the same
//! simulation held a different heap. Every map here hashes with SipHash's
//! fixed keys ([`DefaultHasher::new`]), so its allocations follow the
//! simulation's inserts and removals alone. No peer chooses a simulation's
//! keys.

use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;

/// The hasher of every map here.
pub(crate) type Fixed = BuildHasherDefault<DefaultHasher>;

/// An insertion-ordered map hashed by [`Fixed`].
pub(crate) type IndexMap<K, V> = indexmap::IndexMap<K, V, Fixed>;

/// An insertion-ordered set hashed by [`Fixed`].
pub(crate) type IndexSet<K> = indexmap::IndexSet<K, Fixed>;
