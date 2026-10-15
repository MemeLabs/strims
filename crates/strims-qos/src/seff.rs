// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The SEFF (Smallest Eligible virtual Finish time First) queue from Stoica
//! and Abdel-Wahab's EEVDF paper.
//!
//! An AVL tree ordered by virtual start time, where each node also records
//! the minimum virtual finish time in its subtree. [`select`] finds the
//! queued entry with the smallest finish time among those whose start time
//! is at most the virtual time, in O(log n).
//!
//! The tree is intrusive: its links live in the queued entries themselves
//! (one [`Links`] per scheduler node, reached through [`Arena`]), so queue
//! operations never allocate. Keys are `(time, seq)` pairs with a unique
//! per-insert `seq`. That makes every key distinct, so removal is an ordinary
//! keyed delete, and ties on finish time go to the earliest insert (FIFO).

/// A virtual time. 128 bits so that fixed-point weights (see `control.rs`)
/// never overflow: at 2^42 units per byte, a class runs for 2^86 bytes.
pub(crate) type VTime = u128;

/// A virtual time tag made unique by an insertion sequence number.
pub(crate) type Tag = (VTime, u64);

const NO_TAG: Tag = (VTime::MAX, u64::MAX);

/// Tree links and keys embedded in each queueable node.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Links<K> {
    pub left: Option<K>,
    pub right: Option<K>,
    pub height: u8,
    /// `(stime, seq)`: the tree order.
    pub start: Tag,
    /// `(ftime, seq)`.
    pub finish: Tag,
    /// Minimum `finish` in this subtree.
    pub min_finish: Tag,
}

impl<K> Default for Links<K> {
    fn default() -> Self {
        Links {
            left: None,
            right: None,
            height: 0,
            start: NO_TAG,
            finish: NO_TAG,
            min_finish: NO_TAG,
        }
    }
}

/// Storage for the tree links, keyed by node.
pub(crate) trait Arena<K: Copy> {
    fn links(&self, k: K) -> &Links<K>;
    fn links_mut(&mut self, k: K) -> &mut Links<K>;
}

fn height<K: Copy, A: Arena<K>>(a: &A, k: Option<K>) -> u8 {
    k.map_or(0, |k| a.links(k).height)
}

fn min_finish<K: Copy, A: Arena<K>>(a: &A, k: Option<K>) -> Tag {
    k.map_or(NO_TAG, |k| a.links(k).min_finish)
}

fn update<K: Copy, A: Arena<K>>(a: &mut A, k: K) {
    let l = *a.links(k);
    let h = height(a, l.left).max(height(a, l.right)) + 1;
    let m = l
        .finish
        .min(min_finish(a, l.left))
        .min(min_finish(a, l.right));
    let l = a.links_mut(k);
    l.height = h;
    l.min_finish = m;
}

fn balance_factor<K: Copy, A: Arena<K>>(a: &A, k: K) -> i16 {
    let l = a.links(k);
    i16::from(height(a, l.right)) - i16::from(height(a, l.left))
}

fn rotate_left<K: Copy, A: Arena<K>>(a: &mut A, t: K) -> K {
    let Some(m) = a.links(t).right else {
        return t;
    };
    a.links_mut(t).right = a.links(m).left;
    a.links_mut(m).left = Some(t);
    update(a, t);
    update(a, m);
    m
}

fn rotate_right<K: Copy, A: Arena<K>>(a: &mut A, t: K) -> K {
    let Some(m) = a.links(t).left else {
        return t;
    };
    a.links_mut(t).left = a.links(m).right;
    a.links_mut(m).right = Some(t);
    update(a, t);
    update(a, m);
    m
}

/// Restores the AVL invariant at `t` (whose subtrees are balanced) and
/// refreshes its stats. Returns the new subtree root.
fn rebalance<K: Copy, A: Arena<K>>(a: &mut A, t: K) -> K {
    update(a, t);
    let bf = balance_factor(a, t);
    if bf > 1 {
        if let Some(r) = a.links(t).right
            && balance_factor(a, r) < 0
        {
            let r = rotate_right(a, r);
            a.links_mut(t).right = Some(r);
        }
        rotate_left(a, t)
    } else if bf < -1 {
        if let Some(l) = a.links(t).left
            && balance_factor(a, l) > 0
        {
            let l = rotate_left(a, l);
            a.links_mut(t).left = Some(l);
        }
        rotate_right(a, t)
    } else {
        t
    }
}

/// Inserts `k` with the given tags into the tree rooted at `root` and
/// returns the new root. `k` must not already be in a tree.
pub(crate) fn insert<K: Copy, A: Arena<K>>(
    a: &mut A,
    root: Option<K>,
    k: K,
    start: Tag,
    finish: Tag,
) -> K {
    *a.links_mut(k) = Links {
        left: None,
        right: None,
        height: 1,
        start,
        finish,
        min_finish: finish,
    };
    insert_at(a, root, k)
}

fn insert_at<K: Copy, A: Arena<K>>(a: &mut A, t: Option<K>, k: K) -> K {
    let Some(t) = t else {
        return k;
    };
    if a.links(k).start < a.links(t).start {
        let l = insert_at(a, a.links(t).left, k);
        a.links_mut(t).left = Some(l);
    } else {
        let r = insert_at(a, a.links(t).right, k);
        a.links_mut(t).right = Some(r);
    }
    rebalance(a, t)
}

/// Removes `k` from the tree rooted at `root` and returns the new root.
/// `k` must be in the tree.
pub(crate) fn remove<K: Copy + PartialEq, A: Arena<K>>(
    a: &mut A,
    root: Option<K>,
    k: K,
) -> Option<K> {
    let t = root?;
    let key = a.links(k).start;
    let tl = *a.links(t);
    if key < tl.start {
        let l = remove(a, tl.left, k);
        a.links_mut(t).left = l;
    } else if key > tl.start {
        let r = remove(a, tl.right, k);
        a.links_mut(t).right = r;
    } else {
        // Keys are unique, so this is `k`. Its subtrees must be relinked,
        // or their entries become unselectable.
        debug_assert!(t == k);
        *a.links_mut(t) = Links::default();
        return match (tl.left, tl.right) {
            (None, r) => r,
            (l, None) => l,
            (Some(l), Some(r)) => {
                let (r, m) = remove_min(a, r);
                let ml = a.links_mut(m);
                ml.left = Some(l);
                ml.right = r;
                Some(rebalance(a, m))
            }
        };
    }
    Some(rebalance(a, t))
}

/// Detaches the leftmost node of `t`. Returns (new subtree root, detached).
fn remove_min<K: Copy, A: Arena<K>>(a: &mut A, t: K) -> (Option<K>, K) {
    match a.links(t).left {
        None => (a.links(t).right, t),
        Some(l) => {
            let (l, m) = remove_min(a, l);
            a.links_mut(t).left = l;
            (Some(rebalance(a, t)), m)
        }
    }
}

/// The eligible entry (start time `<= vtime`) with the smallest finish tag.
pub(crate) fn select<K: Copy, A: Arena<K>>(a: &A, root: Option<K>, vtime: VTime) -> Option<K> {
    enum Best<K> {
        Node(K),
        Subtree(K),
    }
    let mut best: Option<(Tag, Best<K>)> = None;
    let beats = |best: &Option<(Tag, Best<K>)>, f: Tag| best.as_ref().is_none_or(|b| f < b.0);

    // Walk the search path for `vtime`. At an eligible node, the node and
    // its whole left subtree are eligible; continue right for more.
    let mut t = root;
    while let Some(n) = t {
        let l = a.links(n);
        if l.start.0 <= vtime {
            if beats(&best, l.finish) {
                best = Some((l.finish, Best::Node(n)));
            }
            if let Some(left) = l.left {
                let m = a.links(left).min_finish;
                if beats(&best, m) {
                    best = Some((m, Best::Subtree(left)));
                }
            }
            t = l.right;
        } else {
            t = l.left;
        }
    }

    match best? {
        (_, Best::Node(n)) => Some(n),
        (f, Best::Subtree(s)) => {
            let mut t = Some(s);
            while let Some(n) = t {
                let l = a.links(n);
                if l.finish == f {
                    return Some(n);
                }
                t = if min_finish(a, l.left) == f {
                    l.left
                } else {
                    l.right
                };
            }
            None
        }
    }
}

/// Smallest queued start time, used to keep the virtual time from lagging
/// behind every queued entry.
pub(crate) fn min_start<K: Copy, A: Arena<K>>(a: &A, root: Option<K>) -> Option<VTime> {
    let mut t = root?;
    while let Some(l) = a.links(t).left {
        t = l;
    }
    Some(a.links(t).start.0)
}

/// Checks ordering, balance, heights and `min_finish`. Returns the number of
/// entries. Test-only.
#[cfg(test)]
pub(crate) fn check<K: Copy + core::fmt::Debug, A: Arena<K>>(a: &A, root: Option<K>) -> usize {
    fn walk<K: Copy + core::fmt::Debug, A: Arena<K>>(
        a: &A,
        t: Option<K>,
        lo: Option<Tag>,
        hi: Option<Tag>,
    ) -> (u8, Tag, usize) {
        let Some(n) = t else {
            return (0, NO_TAG, 0);
        };
        let l = a.links(n);
        assert!(lo.is_none_or(|lo| lo < l.start), "{n:?} out of order");
        assert!(hi.is_none_or(|hi| l.start < hi), "{n:?} out of order");
        let (lh, lm, lc) = walk(a, l.left, lo, Some(l.start));
        let (rh, rm, rc) = walk(a, l.right, Some(l.start), hi);
        assert!(lh.abs_diff(rh) <= 1, "{n:?} unbalanced");
        assert_eq!(l.height, lh.max(rh) + 1, "{n:?} height");
        let m = l.finish.min(lm).min(rm);
        assert_eq!(l.min_finish, m, "{n:?} min_finish");
        assert_eq!(l.finish.1, l.start.1, "{n:?} seq");
        (l.height, m, lc + rc + 1)
    }
    walk(a, root, None, None).2
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct VecArena(Vec<Links<usize>>);

    impl Arena<usize> for VecArena {
        fn links(&self, k: usize) -> &Links<usize> {
            &self.0[k]
        }
        fn links_mut(&mut self, k: usize) -> &mut Links<usize> {
            &mut self.0[k]
        }
    }

    #[derive(Debug, Clone)]
    enum Op {
        Insert(VTime, VTime),
        Remove(usize),
        Select(VTime),
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            3 => (0u128..50, 0u128..50).prop_map(|(s, d)| Op::Insert(s, d)),
            1 => any::<usize>().prop_map(Op::Remove),
            2 => (0u128..120).prop_map(Op::Select),
        ]
    }

    const SEED: u64 = 0x5eff;

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

        // The tree agrees with a linear scan and keeps its invariants.
        #[test]
        fn matches_model(ops in proptest::collection::vec(op(), 1..400)) {
            const N: usize = 64;
            let mut a = VecArena(vec![Links::default(); N]);
            let mut root = None;
            // (key, start, finish) of queued entries
            let mut model: Vec<(usize, Tag, Tag)> = Vec::new();
            let mut seq = 0;
            for op in ops {
                match op {
                    Op::Insert(s, d) => {
                        let Some(k) = (0..N).find(|k| model.iter().all(|m| m.0 != *k)) else { continue };
                        seq += 1;
                        let (st, ft) = ((s, seq), (s + d, seq));
                        root = Some(insert(&mut a, root, k, st, ft));
                        model.push((k, st, ft));
                    }
                    Op::Remove(i) => {
                        if model.is_empty() { continue }
                        let (k, _, _) = model.remove(i % model.len());
                        root = remove(&mut a, root, k);
                    }
                    Op::Select(v) => {
                        let want = model.iter().filter(|m| m.1.0 <= v).min_by_key(|m| m.2).map(|m| m.0);
                        prop_assert_eq!(select(&a, root, v), want);
                        let want_min = model.iter().map(|m| m.1.0).min();
                        prop_assert_eq!(min_start(&a, root), want_min);
                    }
                }
                prop_assert_eq!(check(&a, root), model.len());
            }
        }
    }

    #[test]
    fn fifo_on_equal_finish() {
        let mut a = VecArena(vec![Links::default(); 4]);
        let mut root = None;
        for (k, seq) in [(2usize, 1u64), (0, 2), (3, 3), (1, 4)] {
            root = Some(insert(&mut a, root, k, (0, seq), (10, seq)));
        }
        let mut order = Vec::new();
        while let Some(k) = select(&a, root, 0) {
            root = remove(&mut a, root, k);
            order.push(k);
        }
        assert_eq!(order, vec![2, 0, 3, 1]);
        assert!(root.is_none());
    }

    // Removing an inner entry keeps every other entry selectable.
    #[test]
    fn remove_keeps_siblings() {
        let mut a = VecArena(vec![Links::default(); 7]);
        let mut root = None;
        for k in 0..7 {
            let t = k as u64;
            root = Some(insert(&mut a, root, k, (t.into(), t), ((t + 1).into(), t)));
        }
        let top = root.unwrap();
        root = remove(&mut a, root, top);
        assert_eq!(check(&a, root), 6);
        let mut n = 0;
        while let Some(k) = select(&a, root, VTime::MAX) {
            root = remove(&mut a, root, k);
            n += 1;
        }
        assert_eq!(n, 6);
    }
}
