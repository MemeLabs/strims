// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A [`KBucket`] is a routing table of 256 buckets indexed by the length of
//! the common prefix between an entry's id and the table's own id. Each bucket
//! holds at most `k` entries; when it is full, inserting a closer entry
//! evicts the farthest one.

use crate::id::{ID, ID_BIT_LENGTH};

/// Something with a Kademlia id.
pub trait Interface {
    fn id(&self) -> ID;
}

impl Interface for ID {
    fn id(&self) -> ID {
        *self
    }
}

impl<T: Interface + ?Sized> Interface for &T {
    fn id(&self) -> ID {
        (**self).id()
    }
}

/// Outcome of [`KBucket::insert`]. The replaced entry, or the item that
/// wasn't inserted, is handed back.
#[derive(Debug, PartialEq, Eq)]
pub enum Insert<T> {
    /// Appended to a bucket with room.
    Added,
    /// The bucket was full: the farthest entry was replaced and is returned.
    Evicted(T),
    /// Not inserted because an entry with the same id exists.
    Duplicate(T),
    /// Not inserted because the bucket is full of closer entries.
    Rejected(T),
}

impl<T> Insert<T> {
    /// Whether the item is now in the table.
    pub fn inserted(&self) -> bool {
        matches!(self, Insert::Added | Insert::Evicted(_))
    }
}

/// A Kademlia routing table around the table's own id.
///
/// Memory: at most `k` entries per bucket and 256 buckets, so at most
/// `256 * k` entries. Buckets allocate on first use.
#[derive(Debug, Clone)]
pub struct KBucket<T> {
    k: usize,
    id: ID,
    /// Bounded: 256 buckets of at most `k` entries each.
    buckets: Vec<Vec<T>>,
    len: usize,
}

impl<T: Interface> KBucket<T> {
    /// An empty table around `id`, holding at most `k` entries per bucket.
    pub fn new(id: ID, k: usize) -> Self {
        KBucket {
            k,
            id,
            buckets: (0..ID_BIT_LENGTH).map(|_| Vec::new()).collect(),
            len: 0,
        }
    }

    /// The table's own id.
    pub fn id(&self) -> ID {
        self.id
    }

    /// Maximum entries per bucket.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True if the table has no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Removes every entry, keeping bucket allocations.
    pub fn reset(&mut self) {
        for b in &mut self.buckets {
            b.clear();
        }
        self.len = 0;
    }

    /// Removes every entry and returns them in [`KBucket::iter`] order. The
    /// table is empty even if the iterator is dropped early.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + use<T> {
        self.len = 0;
        let buckets = core::mem::replace(
            &mut self.buckets,
            (0..ID_BIT_LENGTH).map(|_| Vec::new()).collect(),
        );
        buckets.into_iter().flatten()
    }

    /// Iterates entries in bucket order (farthest prefix first), and in
    /// insertion order within a bucket.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buckets.iter().flatten()
    }

    /// Adds `n` to its bucket. A full bucket replaces, in place, its entry
    /// farthest from the table's own id if that is farther than `n`.
    pub fn insert(&mut self, n: T) -> Insert<T> {
        let nid = n.id();
        let self_id = self.id;
        let k = self.k;
        let Some(bucket) = self.buckets.get_mut(bucket_index(&self_id, &nid)) else {
            return Insert::Rejected(n);
        };

        if bucket.iter().any(|e| e.id() == nid) {
            return Insert::Duplicate(n);
        }

        if bucket.len() >= k {
            // Replace the entry farthest from our own id if it is farther
            // than the newcomer. Ties keep the earliest entry.
            let mut max_distance = self_id ^ nid;
            let mut max_index = None;
            for (j, e) in bucket.iter().enumerate() {
                let distance = self_id ^ e.id();
                if max_distance < distance {
                    max_distance = distance;
                    max_index = Some(j);
                }
            }
            return match max_index.and_then(|j| bucket.get_mut(j)) {
                Some(slot) => Insert::Evicted(core::mem::replace(slot, n)),
                None => Insert::Rejected(n),
            };
        }

        bucket.push(n);
        self.len += 1;
        Insert::Added
    }

    /// Removes and returns the entry with `id`.
    pub fn remove(&mut self, id: &ID) -> Option<T> {
        let bucket = self.buckets.get_mut(bucket_index(&self.id, id))?;
        let j = bucket.iter().position(|e| e.id() == *id)?;
        self.len -= 1;
        Some(bucket.remove(j))
    }

    /// The entry with `id`.
    pub fn get(&self, id: &ID) -> Option<&T> {
        self.buckets
            .get(bucket_index(&self.id, id))?
            .iter()
            .find(|e| e.id() == *id)
    }

    /// Mutable [`KBucket::get`].
    pub fn get_mut(&mut self, id: &ID) -> Option<&mut T> {
        self.buckets
            .get_mut(bucket_index(&self.id, id))?
            .iter_mut()
            .find(|e| e.id() == *id)
    }

    /// Whether an entry with `id` exists.
    pub fn contains(&self, id: &ID) -> bool {
        self.get(id).is_some()
    }

    /// Fills `out` with the `n` entries closest to `id` (fewer if the table
    /// is smaller), nearest first. `out` is cleared first and reused so a
    /// caller can avoid allocating per lookup.
    pub fn closest<'a>(&'a self, id: &ID, n: usize, out: &mut Vec<&'a T>) {
        out.clear();
        if n == 0 {
            return;
        }
        let i = bucket_index(&self.id, id);
        // Every entry in bucket i is closer to `id` than any entry in a
        // bucket above i, and those are closer than any entry in a bucket
        // below i, which get farther as the index falls.
        if let Some(b) = self.buckets.get(i) {
            out.extend(b.iter());
        }
        if out.len() < n {
            for b in self.buckets.iter().skip(i + 1) {
                out.extend(b.iter());
            }
        }
        for b in self.buckets.iter().take(i).rev() {
            if out.len() >= n {
                break;
            }
            out.extend(b.iter());
        }
        out.sort_unstable_by_key(|e| e.id() ^ *id);
        out.truncate(n);
    }
}

/// Bucket index of `id` in a table owned by `self_id`: the common prefix
/// length, or 255 when the ids are equal.
fn bucket_index(self_id: &ID, id: &ID) -> usize {
    let lz = (*self_id ^ *id).leading_zeros() as usize;
    lz.min(ID_BIT_LENGTH - 1)
}

/// Collects items and yields them nearest first by XOR distance to a target.
///
/// Unbounded: callers bound what they push.
#[derive(Debug, Clone)]
pub struct Filter<T> {
    id: ID,
    /// (distance, item), kept farthest first so `pop` takes the nearest.
    items: Vec<(ID, T)>,
    dirty: bool,
}

impl<T: Interface> Filter<T> {
    /// An empty filter around the target `id`.
    pub fn new(id: ID) -> Self {
        Filter {
            id,
            items: Vec::new(),
            dirty: false,
        }
    }

    /// Empties the filter and retargets it, keeping its allocation.
    pub fn reset(&mut self, id: ID) {
        self.id = id;
        self.items.clear();
        self.dirty = false;
    }

    pub fn push(&mut self, n: T) {
        let d = n.id() ^ self.id;
        self.items.push((d, n));
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Orders the items by distance. Reads sort on demand, so calling this
    /// is optional.
    pub fn sort(&mut self) {
        if self.dirty {
            // Distances to one target are distinct for distinct ids; equal
            // ids keep push order.
            self.items.sort_by_key(|item| core::cmp::Reverse(item.0));
            self.dirty = false;
        }
    }

    /// Removes and returns the nearest item.
    pub fn pop(&mut self) -> Option<T> {
        self.sort();
        self.items.pop().map(|(_, n)| n)
    }

    /// Iterates nearest first.
    pub fn iter(&mut self) -> impl Iterator<Item = &T> {
        self.sort();
        self.items.iter().rev().map(|(_, n)| n)
    }
}

impl<T: Interface> Extend<T> for Filter<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for n in iter {
            self.push(n);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rand_core::Rng;

    fn random_id(rng: &mut strims_base::rng::DetRng) -> ID {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        ID::from_bytes(&b)
    }

    #[test]
    fn kbucket_closest_sorted() {
        let mut rng = strims_base::rng::det_rng(1);
        let mut b = KBucket::new(ID::MAX, 20);
        for _ in 0..100 {
            let _ = b.insert(random_id(&mut rng));
        }
        let target = random_id(&mut rng);
        let mut out = Vec::new();
        b.closest(&target, 20, &mut out);
        assert_eq!(out.len(), 20);
        for w in out.windows(2) {
            if let [a, c] = w {
                assert!((a.id() ^ target).less(&(c.id() ^ target)));
            }
        }
    }

    #[test]
    fn insert_outcomes() {
        let me = ID::MIN;
        let mut b = KBucket::new(me, 2);
        // All three share bucket 0 (top bit set).
        let near = ID([0x8000_0000_0000_0000, 0, 0, 1]);
        let mid = ID([0x8000_0000_0000_0000, 1, 0, 0]);
        let far = ID([0xffff_0000_0000_0000, 0, 0, 0]);
        assert_eq!(b.insert(far), Insert::Added);
        assert_eq!(b.insert(mid), Insert::Added);
        assert_eq!(b.insert(mid), Insert::Duplicate(mid));
        assert_eq!(b.len(), 2);
        // Bucket full; `near` is closer than `far`, which is evicted in place.
        assert_eq!(b.insert(near), Insert::Evicted(far));
        assert_eq!(b.iter().copied().collect::<Vec<_>>(), vec![near, mid]);
        // A newcomer farther than everything present is rejected.
        assert_eq!(b.insert(far), Insert::Rejected(far));
        assert!(!Insert::Rejected(far).inserted());
        assert_eq!(b.len(), 2);
        assert_eq!(b.remove(&mid), Some(mid));
        assert_eq!(b.remove(&mid), None);
        assert_eq!(b.get(&near), Some(&near));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn zero_k_rejects() {
        let mut b = KBucket::new(ID::MIN, 0);
        assert_eq!(b.insert(ID::MAX), Insert::Rejected(ID::MAX));
        assert!(b.is_empty());
    }

    #[test]
    fn reset_empties() {
        let mut b = KBucket::new(ID([1, 2, 3, 4]), 4);
        let _ = b.insert(ID([5, 6, 7, 8]));
        assert!(!b.is_empty());
        b.reset();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert_eq!(b.iter().count(), 0);
        let _ = b.insert(ID([5, 6, 7, 8]));
        let _ = b.insert(ID([u64::MAX, 6, 7, 8]));
        assert_eq!(b.drain().next(), Some(ID([u64::MAX, 6, 7, 8])));
        assert!(b.is_empty());
        assert_eq!(b.iter().count(), 0);
    }

    #[test]
    fn bucket_index_is_common_prefix_length() {
        let me = ID::MIN;
        assert_eq!(bucket_index(&me, &me), 255);
        assert_eq!(bucket_index(&me, &ID([1 << 63, 0, 0, 0])), 0);
        assert_eq!(bucket_index(&me, &ID([0, 1, 0, 0])), 127);
        assert_eq!(bucket_index(&me, &ID([0, 0, 0, 1])), 255);
        assert_eq!(bucket_index(&me, &ID([0, 0, 0, 2])), 254);
    }

    #[test]
    fn filter_iterates_nearest_first() {
        let mut f = Filter::new(ID::MIN);
        f.extend([ID([1, 0, 0, 0]), ID([3, 0, 0, 0]), ID([2, 0, 0, 0])]);
        let got: Vec<u64> = f.iter().map(|i| i.0[0]).collect();
        assert_eq!(got, vec![1, 2, 3]);
        assert_eq!(f.pop(), Some(ID([1, 0, 0, 0])));
        f.push(ID([0, 1, 0, 0]));
        assert_eq!(f.pop(), Some(ID([0, 1, 0, 0])));
        assert_eq!(f.len(), 2);
        f.reset(ID::MAX);
        assert!(f.is_empty());
    }

    /// Ids clustered near `base` so that many buckets are populated and some
    /// fill up, which is where a closest search that stops early would
    /// return farther entries.
    fn clustered_id(rng: &mut strims_base::rng::DetRng, base: ID) -> ID {
        let mut id = random_id(rng);
        let shift = rng.next_u32() % 40;
        id.0[0] >>= shift;
        id ^ base
    }

    fn brute_closest(all: &[ID], target: &ID, n: usize) -> Vec<ID> {
        let mut v = all.to_vec();
        v.sort_by_key(|e| *e ^ *target);
        v.truncate(n);
        v
    }

    #[test]
    fn closest_is_exact() {
        let mut rng = strims_base::rng::det_rng(7);
        let me = random_id(&mut rng);
        let mut b = KBucket::new(me, 8);
        for _ in 0..3000 {
            let _ = b.insert(clustered_id(&mut rng, me));
        }
        let all: Vec<ID> = b.iter().copied().collect();
        let mut out = Vec::new();
        for i in 0..500 {
            let target = if i % 2 == 0 {
                clustered_id(&mut rng, me)
            } else {
                random_id(&mut rng)
            };
            for n in [1, 5, 10, 40] {
                b.closest(&target, n, &mut out);
                let got: Vec<ID> = out.iter().map(|e| **e).collect();
                assert_eq!(
                    got,
                    brute_closest(&all, &target, n),
                    "target {target} n {n}"
                );
            }
        }
    }

    #[derive(Debug, Clone)]
    enum Op {
        Insert(u8, u64),
        Remove(u8, u64),
    }

    fn op() -> impl Strategy<Value = Op> {
        (any::<bool>(), 0u8..64, any::<u64>()).prop_map(|(ins, shift, lo)| {
            if ins {
                Op::Insert(shift, lo % 32)
            } else {
                Op::Remove(shift, lo % 32)
            }
        })
    }

    fn op_id(shift: u8, lo: u64) -> ID {
        ID([u64::MAX >> shift, 0, 0, lo])
    }

    const SEED: u64 = 0x6b_75636b;

    fn config(cases: u32) -> ProptestConfig {
        ProptestConfig {
            cases,
            rng_seed: proptest::test_runner::RngSeed::Fixed(SEED),
            failure_persistence: None,
            ..ProptestConfig::default()
        }
    }

    proptest! {
        #![proptest_config(config(256))]

        // Model check: bucket bounds, counts, and the eviction rule.
        #[test]
        fn kbucket_model(ops in proptest::collection::vec(op(), 1..300), k in 1usize..4) {
            let me = ID::MIN;
            let mut b = KBucket::new(me, k);
            let mut model: Vec<Vec<ID>> = vec![Vec::new(); 256];
            for op in ops {
                match op {
                    Op::Insert(s, lo) => {
                        let id = op_id(s, lo);
                        let m = &mut model[bucket_index(&me, &id)];
                        let res = b.insert(id);
                        if m.contains(&id) {
                            prop_assert_eq!(res, Insert::Duplicate(id));
                        } else if m.len() < k {
                            m.push(id);
                            prop_assert_eq!(res, Insert::Added);
                        } else {
                            let (j, far) = m.iter().enumerate()
                                .max_by(|a, b| (*a.1 ^ me).cmp(&(*b.1 ^ me)).then(b.0.cmp(&a.0)))
                                .map(|(j, e)| (j, *e)).unwrap();
                            if (far ^ me) > (id ^ me) {
                                m[j] = id;
                                prop_assert_eq!(res, Insert::Evicted(far));
                            } else {
                                prop_assert_eq!(res, Insert::Rejected(id));
                            }
                        }
                    }
                    Op::Remove(s, lo) => {
                        let id = op_id(s, lo);
                        let m = &mut model[bucket_index(&me, &id)];
                        let want = m.iter().position(|e| *e == id).map(|j| m.remove(j));
                        prop_assert_eq!(b.remove(&id), want);
                    }
                }
                let flat: Vec<ID> = model.iter().flatten().copied().collect();
                prop_assert_eq!(b.iter().copied().collect::<Vec<_>>(), flat.clone());
                prop_assert_eq!(b.len(), flat.len());
            }
        }
    }
}
