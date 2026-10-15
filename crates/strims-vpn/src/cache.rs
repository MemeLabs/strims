// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A small bounded map with first-in-first-out eviction, for caching
//! per-peer cryptographic state (message ciphers, verifying keys).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, VecDeque};

/// Bounded at `cap` entries; inserting into a full map evicts the oldest.
#[derive(Debug)]
pub(crate) struct BoundedMap<K, V> {
    map: BTreeMap<K, V>,
    /// Insertion order; the same keys as `map`.
    order: VecDeque<K>,
    cap: usize,
}

impl<K: Ord + Copy, V> BoundedMap<K, V> {
    pub(crate) fn new(cap: usize) -> BoundedMap<K, V> {
        BoundedMap {
            map: BTreeMap::new(),
            order: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    /// The value for `k`, creating it with `make` if absent.
    pub(crate) fn get_or_try_insert<E>(
        &mut self,
        k: K,
        make: impl FnOnce() -> Result<V, E>,
    ) -> Result<&V, E> {
        if !self.map.contains_key(&k) {
            while self.map.len() >= self.cap {
                let Some(old) = self.order.pop_front() else {
                    break;
                };
                self.map.remove(&old);
            }
        }
        match self.map.entry(k) {
            Entry::Occupied(e) => Ok(e.into_mut()),
            Entry::Vacant(e) => {
                let v = make()?;
                self.order.push_back(k);
                Ok(e.insert(v))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn evicts_oldest() {
        let mut m = BoundedMap::new(2);
        for k in 0..4u32 {
            m.get_or_try_insert::<()>(k, || Ok(k * 10)).unwrap();
        }
        assert_eq!(m.len(), 2);
        assert_eq!(*m.get_or_try_insert::<()>(3, || Ok(0)).unwrap(), 30);
        assert_eq!(*m.get_or_try_insert::<()>(0, || Ok(7)).unwrap(), 7);
        assert!(m.get_or_try_insert(9, || Err(())).is_err());
    }
}
