// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The binmap itself.
//!
//! The map is a binary tree of [`Cell`]s rooted at [`Map::root_bin`]. Each
//! data cell stands for a bin of layer 6 or above; each half is either a ref
//! to a child cell or a 32 bit bitmap. A bitmap in a half above layer 5 is
//! repeated across every layer 5 bin below it, which is how uniform (and
//! periodic) regions stay a single cell.
//!
//! The root always starts at base 0: it begins as bin 63 and only grows by
//! [`Bin::parent`]. Every base bin outside the root is empty.
//!
//! Queries never allocate. Mutations allocate only when the cell vector
//! grows; it never shrinks.

use crate::bin::Bin;
use crate::bitmap::{
    self, BITMAP_EMPTY, BITMAP_FILLED, BITMAP_LAYER_BITS, Bitmap, bin_bitmap, offset_bitmap_bin,
};
use crate::cell::{Cell, Ref, is_map_ref, map_mask, map_ref};
use crate::iterator::Iter;

/// The root data cell. Index 0 is a map cell.
const ROOT_REF: Ref = 1;

/// A set of filled base bins.
///
/// Callers bound the bins they pass in. Base-iterating [`Bin::ALL`] walks
/// 2^63 bins, and each far-flung bin set can add up to 58 cells, which stay
/// allocated. Check peer-supplied bins against the swarm's live window
/// before they reach a map, and trim the tail with [`Map::reset_before`].
#[derive(Debug)]
pub struct Map {
    /// Head of the free cell list.
    free_top: Ref,
    /// Data cells in use, the root included.
    alloc_count: usize,
    /// Data cells in `cells` (map cells excluded).
    cell_count: usize,
    root_bin: Bin,
    /// Every cell. Bound: the map needs roughly one cell per 64 base bins of
    /// alternating content plus one per layer between a run boundary and the
    /// root (at most 58), so the owner bounds it by bounding the bins it sets,
    /// e.g. ppspp's live window and [`Map::reset_before`] at the tail. It
    /// never shrinks.
    cells: Vec<Cell>,
}

/// The cells visited by a trace, root first.
///
/// Lives on the stack of each operation so queries can take `&self`. Traces descend from the root (layer 63 at most)
/// to a layer 6 cell, so 64 entries always suffice.
struct TraceHistory {
    refs: [Ref; 64],
    len: usize,
}

impl TraceHistory {
    #[inline]
    fn new() -> TraceHistory {
        TraceHistory {
            refs: [0; 64],
            len: 0,
        }
    }

    #[inline]
    fn push(&mut self, r: Ref) {
        if let Some(slot) = self.refs.get_mut(self.len) {
            *slot = r;
            self.len += 1;
        }
    }

    #[inline]
    fn get(&self, i: usize) -> Ref {
        // invariant: callers only read indices below `len`, and `push` keeps
        // `len <= 64`.
        #[allow(clippy::indexing_slicing)]
        self.refs[i]
    }
}

impl Default for Map {
    fn default() -> Map {
        Map::new()
    }
}

/// `clone_from` reuses the destination's allocation, which suits ppspp's
/// per-peer snapshots.
impl Clone for Map {
    fn clone(&self) -> Map {
        Map {
            free_top: self.free_top,
            alloc_count: self.alloc_count,
            cell_count: self.cell_count,
            root_bin: self.root_bin,
            cells: self.cells.clone(),
        }
    }

    fn clone_from(&mut self, source: &Map) {
        self.free_top = source.free_top;
        self.alloc_count = source.alloc_count;
        self.cell_count = source.cell_count;
        self.root_bin = source.root_bin;
        self.cells.clone_from(&source.cells);
    }
}

// Cell access and the map cell's ref flags.
impl Map {
    #[inline]
    fn cell(&self, r: Ref) -> Cell {
        // invariant: every `Ref` the map follows was handed out by
        // `alloc_cell` or is `ROOT_REF` or a map ref, all below `cells.len()`;
        // `debug_parse` validates refs before building a map.
        #[allow(clippy::indexing_slicing)]
        self.cells[r as usize]
    }

    #[inline]
    fn cell_mut(&mut self, r: Ref) -> &mut Cell {
        // invariant: see `cell`.
        #[allow(clippy::indexing_slicing)]
        &mut self.cells[r as usize]
    }

    /// Whether data cell `r`'s left half is a ref.
    #[inline]
    fn is_left_ref(&self, r: Ref) -> bool {
        bitmap::get(self.cell(map_ref(r)).left, map_mask(r))
    }

    /// Whether data cell `r`'s right half is a ref.
    #[inline]
    fn is_right_ref(&self, r: Ref) -> bool {
        bitmap::get(self.cell(map_ref(r)).right, map_mask(r))
    }

    /// Whether either half of data cell `r` is a ref.
    #[inline]
    fn has_ref(&self, r: Ref) -> bool {
        let mc = self.cell(map_ref(r));
        let m = map_mask(r);
        bitmap::get(mc.right, m) || bitmap::get(mc.left, m)
    }

    #[inline]
    fn set_left_is_ref(&mut self, r: Ref, v: bool) {
        let mc = self.cell_mut(map_ref(r));
        mc.left = bitmap::set(mc.left, map_mask(r), v);
    }

    #[inline]
    fn set_right_is_ref(&mut self, r: Ref, v: bool) {
        let mc = self.cell_mut(map_ref(r));
        mc.right = bitmap::set(mc.right, map_mask(r), v);
    }

    #[inline]
    fn reset_is_ref(&mut self, r: Ref) {
        self.set_left_is_ref(r, false);
        self.set_right_is_ref(r, false);
    }
}

impl Map {
    /// An empty map rooted at bin 63.
    pub fn new() -> Map {
        let mut m = Map {
            free_top: 0,
            alloc_count: 0,
            cell_count: 0,
            root_bin: Bin(63),
            cells: Vec::new(),
        };
        m.alloc_cell();
        m
    }

    /// The bin the root cell covers. Every base bin outside it is empty.
    #[inline]
    pub fn root_bin(&self) -> Bin {
        self.root_bin
    }

    fn alloc_cell(&mut self) -> Ref {
        self.reserve_cells(1);

        let r = self.free_top;
        self.free_top = self.cell(r).left;
        self.alloc_count += 1;

        *self.cell_mut(r) = Cell::default();
        self.reset_is_ref(r);
        r
    }

    fn free_cell(&mut self, r: Ref) {
        let c = self.cell(r);
        if self.is_left_ref(r) {
            self.free_cell(c.left);
        }
        if self.is_right_ref(r) {
            self.free_cell(c.right);
        }

        self.cell_mut(r).left = self.free_top;
        self.free_top = r;
        self.alloc_count -= 1;
    }

    /// Makes sure more than `n + 1` free cells exist, growing the cell vector
    /// by 1.5x, to at least `n` more cells plus map cell overhead, and to at
    /// least 16 cells. The golden trace pins this policy and the free list
    /// order through its hash of the debug dump.
    fn reserve_cells(&mut self, n: usize) {
        if self.cell_count.saturating_sub(self.alloc_count) > n + 1 {
            return;
        }

        let l = self.cells.len();
        let nl = (l * 3 / 2).max(l + (n + 1) * 32 / 31).max(16);

        self.cells.reserve_exact(nl - l);
        self.cells.resize(nl, Cell::default());

        for i in (l..nl).rev() {
            // Refs are 32 bits; 2^32 cells would be 32 GiB.
            let r = i as Ref;
            if !is_map_ref(r) {
                self.cell_mut(r).left = self.free_top;
                self.free_top = r;
                self.cell_count += 1;
            }
        }
    }

    fn extend_root(&mut self) {
        if !self.has_ref(ROOT_REF) && self.cell(ROOT_REF).symmetric() {
            self.cell_mut(ROOT_REF).right = BITMAP_EMPTY;
        } else {
            let r = self.alloc_cell();

            *self.cell_mut(r) = self.cell(ROOT_REF);
            let (left_is_ref, right_is_ref) =
                (self.is_left_ref(ROOT_REF), self.is_right_ref(ROOT_REF));
            self.set_left_is_ref(r, left_is_ref);
            self.set_right_is_ref(r, right_is_ref);

            *self.cell_mut(ROOT_REF) = Cell {
                left: r,
                right: BITMAP_EMPTY,
            };
            self.set_left_is_ref(ROOT_REF, true);
            self.set_right_is_ref(ROOT_REF, false);
        }

        self.root_bin = self.root_bin.parent();
    }

    /// Merges the last traced cell into its ancestors while it and they are
    /// uniform.
    fn pack_cells(&mut self, history: &TraceHistory) {
        let mut i = history.len - 1;
        let mut r = history.get(i);
        if r == ROOT_REF {
            return;
        }

        let c = self.cell(r);
        if self.has_ref(r) || !c.symmetric() {
            return;
        }

        let b = c.left;
        loop {
            // invariant: history[0] is the root and the loop stops there.
            i -= 1;
            r = history.get(i);
            let c = self.cell(r);

            if !self.is_left_ref(r) {
                if c.left != b {
                    break;
                }
            } else if !self.is_right_ref(r) {
                if c.right != b {
                    break;
                }
            } else {
                break;
            }

            if r == ROOT_REF {
                break;
            }
        }

        let nr = history.get(i + 1);
        if self.is_left_ref(r) && self.cell(r).left == nr {
            self.set_left_is_ref(r, false);
            self.cell_mut(r).left = b;
        } else {
            self.set_right_is_ref(r, false);
            self.cell_mut(r).right = b;
        }
        self.free_cell(nr);
    }

    /// The deepest cell on the path to `target`, and its bin.
    fn trace(&self, target: Bin) -> (Ref, Bin) {
        let mut r = ROOT_REF;
        let mut b = self.root_bin;

        while target != b {
            if target < b {
                if !self.is_left_ref(r) {
                    break;
                }
                r = self.cell(r).left;
                b = b.left();
            } else {
                if !self.is_right_ref(r) {
                    break;
                }
                r = self.cell(r).right;
                b = b.right();
            }
        }
        (r, b)
    }

    /// [`Map::trace`], recording the path in `history`.
    fn trace_history(&self, target: Bin, history: &mut TraceHistory) -> (Ref, Bin) {
        let mut r = ROOT_REF;
        let mut b = self.root_bin;

        history.len = 0;
        history.push(r);
        while target != b {
            let c = self.cell(r);
            if target < b {
                if !self.is_left_ref(r) {
                    break;
                }
                r = c.left;
                b = b.left();
            } else {
                if !self.is_right_ref(r) {
                    break;
                }
                r = c.right;
                b = b.right();
            }
            history.push(r);
        }
        (r, b)
    }

    /// Marks every base bin in `b` filled. A no-op for [`Bin::NONE`].
    pub fn set(&mut self, b: Bin) {
        if b.is_none() {
            return;
        }
        if b.layer_bits().0 > BITMAP_LAYER_BITS {
            self.set_high_layer_bitmap(b, BITMAP_FILLED);
        } else {
            self.set_low_layer_bitmap(b, BITMAP_FILLED);
        }
    }

    /// Marks every base bin in `b` empty.
    pub fn reset(&mut self, b: Bin) {
        if b.layer_bits().0 > BITMAP_LAYER_BITS {
            self.set_high_layer_bitmap(b, BITMAP_EMPTY);
        } else {
            self.set_low_layer_bitmap(b, BITMAP_EMPTY);
        }
    }

    /// Fills every base bin before `b`'s first base bin.
    pub fn fill_before(&mut self, b: Bin) {
        let mut b = b.layer_left();
        while !b.is_none() {
            if b.is_left() {
                self.set(b);
                b = b.parent().layer_left();
            } else {
                b = b.parent();
            }
        }
    }

    /// Empties every base bin before `b`'s first base bin.
    pub fn reset_before(&mut self, b: Bin) {
        let mut b = b.layer_left();
        while !b.is_none() {
            if b.is_left() {
                self.reset(b);
                b = b.parent().layer_left();
            } else {
                b = b.parent();
            }
        }
    }

    fn set_low_layer_bitmap(&mut self, target: Bin, v: Bitmap) {
        let bin_bitmap = bin_bitmap(target);
        let bitmap = v & bin_bitmap;

        if !self.root_bin.contains(target) {
            if bitmap == BITMAP_EMPTY {
                return;
            }
            loop {
                self.extend_root();
                if self.root_bin.contains(target) {
                    break;
                }
            }
        }

        let pre_bin = Bin((target.0 & !(BITMAP_LAYER_BITS + 1)) | BITMAP_LAYER_BITS);
        let mut history = TraceHistory::new();
        let (mut r, mut b) = self.trace_history(target, &mut history);

        let c = self.cell(r);
        let bm;
        if target < b {
            bm = c.left;
            if bm & bin_bitmap == bitmap {
                return;
            }
            if b == pre_bin {
                self.cell_mut(r).left = (c.left & !bin_bitmap) | bitmap;
                self.pack_cells(&history);
                return;
            }
        } else {
            bm = c.right;
            if bm & bin_bitmap == bitmap {
                return;
            }
            if b == pre_bin {
                self.cell_mut(r).right = (c.right & !bin_bitmap) | bitmap;
                self.pack_cells(&history);
                return;
            }
        }

        (r, b) = self.grow_to(r, b, pre_bin, bm);

        let c = self.cell_mut(r);
        if target < b {
            c.left = (c.left & !bin_bitmap) | bitmap;
        } else {
            c.right = (c.right & !bin_bitmap) | bitmap;
        }
    }

    /// Splits the half of cell `r` (bin `b`) on the path to `pre_bin`, which
    /// holds `bm`, into a chain of cells holding `bm` down to `pre_bin`.
    /// Returns the `pre_bin` cell.
    fn grow_to(&mut self, mut r: Ref, mut b: Bin, pre_bin: Bin, bm: Bitmap) -> (Ref, Bin) {
        self.reserve_cells((b.layer() - pre_bin.layer()) as usize);

        loop {
            let nr = self.alloc_cell();
            *self.cell_mut(nr) = Cell::uniform(bm);

            if pre_bin < b {
                self.cell_mut(r).left = nr;
                self.set_left_is_ref(r, true);
                b = b.left();
            } else {
                self.cell_mut(r).right = nr;
                self.set_right_is_ref(r, true);
                b = b.right();
            }

            r = nr;

            if b == pre_bin {
                return (r, b);
            }
        }
    }

    fn set_high_layer_bitmap(&mut self, target: Bin, v: Bitmap) {
        if target.contains(self.root_bin) {
            let c = self.cell(ROOT_REF);
            if self.is_left_ref(ROOT_REF) {
                self.free_cell(c.left);
            }
            if self.is_right_ref(ROOT_REF) {
                self.free_cell(c.right);
            }

            self.root_bin = target;
            self.reset_is_ref(ROOT_REF);
            *self.cell_mut(ROOT_REF) = Cell::uniform(v);
            return;
        }

        let pre_bin = target.parent();

        if !self.root_bin.contains(pre_bin) {
            if v == BITMAP_EMPTY {
                return;
            }
            loop {
                self.extend_root();
                if self.root_bin.contains(pre_bin) {
                    break;
                }
            }
        }

        let mut history = TraceHistory::new();
        let (mut r, mut b) = self.trace_history(pre_bin, &mut history);
        let c = self.cell(r);

        let mut bm = BITMAP_EMPTY;
        if target < b {
            if self.is_left_ref(r) {
                self.set_left_is_ref(r, false);
                self.free_cell(c.left);
            } else {
                bm = c.left;
                if bm == v {
                    return;
                }
            }
            if b == pre_bin {
                self.cell_mut(r).left = v;
                self.pack_cells(&history);
                return;
            }
        } else {
            if self.is_right_ref(r) {
                self.set_right_is_ref(r, false);
                self.free_cell(c.right);
            } else {
                bm = c.right;
                if bm == v {
                    return;
                }
            }
            if b == pre_bin {
                self.cell_mut(r).right = v;
                self.pack_cells(&history);
                return;
            }
        }

        (r, b) = self.grow_to(r, b, pre_bin, bm);

        let c = self.cell_mut(r);
        if target < b {
            c.left = v;
        } else {
            c.right = v;
        }
    }

    fn test_bin(&self, target: Bin, v: Bitmap) -> bool {
        let (r, b) = self.trace(target);
        let c = self.cell(r);

        if target.layer_bits().0 > BITMAP_LAYER_BITS {
            if target < b {
                return c.left == v;
            }
            if target > b {
                return c.right == v;
            }
            return !self.has_ref(r) && c.left == v && c.right == v;
        }

        let bm = if target < b { c.left } else { c.right };
        let mask = bin_bitmap(target);
        bm & mask == v & mask
    }

    /// True if no bin is filled.
    pub fn is_empty(&self) -> bool {
        let c = self.cell(ROOT_REF);
        !self.has_ref(ROOT_REF) && c.left == BITMAP_EMPTY && c.right == BITMAP_EMPTY
    }

    /// True if every bin in the root is filled.
    pub fn is_filled(&self) -> bool {
        let c = self.cell(ROOT_REF);
        !self.has_ref(ROOT_REF) && c.left == BITMAP_FILLED && c.right == BITMAP_FILLED
    }

    /// True if every base bin in `b` is empty.
    pub fn empty_at(&self, b: Bin) -> bool {
        if !self.root_bin.contains(b) {
            return !b.contains(self.root_bin) || self.is_empty();
        }
        self.test_bin(b, BITMAP_EMPTY)
    }

    /// True if every base bin in `b` is filled.
    pub fn filled_at(&self, b: Bin) -> bool {
        if !self.root_bin.contains(b) {
            return false;
        }
        self.test_bin(b, BITMAP_FILLED)
    }

    /// The largest uniformly filled or empty bin the map stores that
    /// contains `target`, or [`Bin::NONE`] if `target` is mixed.
    ///
    /// Edge cases (ppspp maps `ALL` to [`Map::root_bin`]):
    /// - [`Bin::ALL`] when the whole map is empty, or when the root is
    ///   entirely filled and `target` is below layer 6.
    /// - For a target of layer 6 or above inside a uniform half of the root,
    ///   the half, even if the root itself is uniform.
    /// - For a `target` disjoint from the root, the largest bin that
    ///   contains `target` and is disjoint from the root.
    pub fn cover(&self, target: Bin) -> Bin {
        if !self.root_bin.contains(target) {
            if !target.contains(self.root_bin) {
                return self.cover_outside_root(target);
            }
            if self.is_empty() {
                return Bin::ALL;
            }
            return Bin::NONE;
        }

        let (r, mut b) = self.trace(target);
        let c = self.cell(r);

        if target.layer_bits().0 > BITMAP_LAYER_BITS {
            if target < b {
                if c.left == BITMAP_EMPTY || c.left == BITMAP_FILLED {
                    return b.left();
                }
                return Bin::NONE;
            }
            if b < target {
                if c.right == BITMAP_EMPTY || c.right == BITMAP_FILLED {
                    return b.right();
                }
                return Bin::NONE;
            }
            if self.is_left_ref(r) || self.is_right_ref(r) {
                return Bin::NONE;
            }
            if !c.symmetric() {
                return Bin::NONE;
            }
            if c.left == BITMAP_EMPTY {
                return Bin::ALL;
            }
            if c.left == BITMAP_FILLED {
                return b;
            }
            return Bin::NONE;
        }

        let bm;
        if target < b {
            bm = c.left;
            b = b.left();
        } else {
            bm = c.right;
            b = b.right();
        }

        if bm == BITMAP_EMPTY {
            if self.is_empty() {
                return Bin::ALL;
            }
            return b;
        }
        if bm == BITMAP_FILLED {
            if self.is_filled() {
                return Bin::ALL;
            }
            return b;
        }

        let mut nb = target;
        let mut mask = bin_bitmap(nb);

        if bm & mask == BITMAP_EMPTY {
            loop {
                b = nb;
                nb = nb.parent();
                mask = bin_bitmap(nb);
                if bm & mask != BITMAP_EMPTY {
                    return b;
                }
            }
        } else if bm & mask == mask {
            loop {
                b = nb;
                nb = nb.parent();
                mask = bin_bitmap(nb);
                if bm & mask != mask {
                    return b;
                }
            }
        }

        Bin::NONE
    }

    /// Cover of a `target` disjoint from the root: everything outside the
    /// root is empty, so it's the highest ancestor of `target` that is still
    /// disjoint from the root, or [`Bin::ALL`] for an empty map.
    fn cover_outside_root(&self, target: Bin) -> Bin {
        if self.is_empty() {
            return Bin::ALL;
        }
        let mut b = target;
        loop {
            let p = b.parent();
            if p.is_none() || p.contains(self.root_bin) {
                return b;
            }
            b = p;
        }
    }

    /// The largest empty bin starting at the first empty base bin;
    /// [`Bin::ALL`] if the map is empty and [`Bin::NONE`] if it is full.
    pub fn find_empty(&self) -> Bin {
        let mut r;
        let mut b;

        let c = self.cell(ROOT_REF);
        if self.is_left_ref(ROOT_REF) {
            r = c.left;
            b = self.root_bin.left();
        } else if c.left != BITMAP_FILLED {
            if c.left == BITMAP_EMPTY {
                if !self.is_right_ref(ROOT_REF) && c.right == BITMAP_EMPTY {
                    return Bin::ALL;
                }
                return self.root_bin.left();
            }
            return offset_bitmap_bin(self.root_bin.left(), !c.left);
        } else if self.is_right_ref(ROOT_REF) {
            r = c.right;
            b = self.root_bin.right();
        } else {
            if c.right == BITMAP_FILLED {
                if self.root_bin == Bin::ALL {
                    return Bin::NONE;
                }
                return self.root_bin.sibling();
            }
            return offset_bitmap_bin(self.root_bin.right(), !c.right);
        }

        let bm;
        loop {
            let c = self.cell(r);
            if self.is_left_ref(r) {
                r = c.left;
                b = b.left();
            } else if c.left != BITMAP_FILLED {
                bm = c.left;
                b = b.left();
                break;
            } else if self.is_right_ref(r) {
                r = c.right;
                b = b.right();
            } else {
                bm = c.right;
                b = b.right();
                break;
            }
        }

        offset_bitmap_bin(b, !bm)
    }

    /// The first empty base bin at or after `target`'s first base bin, or
    /// [`Bin::NONE`] if the root is [`Bin::ALL`] and full after `target`.
    ///
    /// Returns `target` itself when it is disjoint from the root, and
    /// [`Map::find_empty`] (which need not be a base bin) when it
    /// contains the root.
    pub fn find_empty_after(&self, target: Bin) -> Bin {
        self.find_after::<false>(target)
    }

    /// The largest filled bin starting at the first filled base bin, or
    /// [`Bin::NONE`] if the map is empty.
    pub fn find_filled(&self) -> Bin {
        let mut r;
        let mut b;

        let c = self.cell(ROOT_REF);
        if self.is_left_ref(ROOT_REF) {
            r = c.left;
            b = self.root_bin.left();
        } else if c.left != BITMAP_EMPTY {
            if c.left == BITMAP_FILLED {
                if !self.is_right_ref(ROOT_REF) && c.right == BITMAP_FILLED {
                    return self.root_bin;
                }
                return self.root_bin.left();
            }
            return offset_bitmap_bin(self.root_bin.left(), c.left);
        } else if self.is_right_ref(ROOT_REF) {
            r = c.right;
            b = self.root_bin.right();
        } else {
            if c.right == BITMAP_EMPTY {
                return Bin::NONE;
            }
            return offset_bitmap_bin(self.root_bin.right(), c.right);
        }

        let bm;
        loop {
            let c = self.cell(r);
            if self.is_left_ref(r) {
                r = c.left;
                b = b.left();
            } else if c.left != BITMAP_EMPTY {
                bm = c.left;
                b = b.left();
                break;
            } else if self.is_right_ref(r) {
                r = c.right;
                b = b.right();
            } else {
                bm = c.right;
                b = b.right();
                break;
            }
        }

        offset_bitmap_bin(b, bm)
    }

    /// The first filled base bin at or after `target`'s first base bin, or
    /// [`Bin::NONE`].
    ///
    /// A `target` that strictly contains the root is answered as for its
    /// first base bin.
    pub fn find_filled_after(&self, target: Bin) -> Bin {
        self.find_after::<true>(target)
    }

    /// [`Map::find_empty_after`] (`FILLED == false`) and
    /// [`Map::find_filled_after`] (`FILLED == true`), which differ only in
    /// the bitmap tests and the result outside the root.
    fn find_after<const FILLED: bool>(&self, mut target: Bin) -> Bin {
        // Searching for filled bins looks for set bits; searching for empty
        // bins looks for set bits in the complement.
        let want = |bm: Bitmap| if FILLED { bm } else { !bm };

        if !self.root_bin.contains(target) {
            if FILLED {
                if !target.contains(self.root_bin) {
                    return Bin::NONE;
                }
                target = target.base_left();
            } else {
                if target.contains(self.root_bin) {
                    return self.find_empty();
                }
                return target;
            }
        }

        if !target.is_base() {
            target = target.base_left();
        }

        let mut history = TraceHistory::new();
        let (mut r, mut b) = self.trace_history(target, &mut history);
        let c = self.cell(r);

        let mask = bin_bitmap(target).wrapping_sub(1);
        let bm_base_left = Bin(target.0 & !BITMAP_LAYER_BITS);
        let next_chunk = bm_base_left + (BITMAP_LAYER_BITS + 1);

        'descend: {
            if target < b {
                if want(c.left) & !mask != 0 {
                    return offset_bitmap_bin(bm_base_left, want(c.left) & !mask).base_left();
                } else if Bin(b.left().base_right().0 & !BITMAP_LAYER_BITS) != bm_base_left
                    && want(c.left) != 0
                {
                    return offset_bitmap_bin(next_chunk, want(c.left)).base_left();
                } else if self.is_right_ref(r) {
                    b = b.right();
                    r = c.right;
                    break 'descend;
                } else if want(c.right) != 0 {
                    return offset_bitmap_bin(b.right(), want(c.right)).base_left();
                }
            } else if want(c.right) & !mask != 0 {
                return offset_bitmap_bin(bm_base_left, want(c.right) & !mask).base_left();
            } else if Bin(b.base_right().0 & !BITMAP_LAYER_BITS) != bm_base_left
                && want(c.right) != 0
            {
                return offset_bitmap_bin(next_chunk, want(c.right)).base_left();
            }

            // Climb until an ancestor we reached from the left has a right
            // half with something in it. `i` is the index of `b`'s cell in
            // `history`.
            let mut i = history.len - 1;
            loop {
                if b == self.root_bin {
                    // Everything after the root is empty. Nothing follows an
                    // ALL root: ALL's sibling is ALL, which starts at bin 0.
                    if FILLED || self.root_bin.is_all() {
                        return Bin::NONE;
                    }
                    return self.root_bin.sibling().base_left();
                }

                b = b.parent();
                // invariant: `b` was not the root, so its cell is not
                // history[0].
                i -= 1;
                let pr = history.get(i);
                let c = self.cell(pr);
                if !self.is_right_ref(pr) {
                    if want(c.right) != 0 {
                        return offset_bitmap_bin(b.right(), want(c.right)).base_left();
                    }
                } else if c.right != r && b > target {
                    b = b.right();
                    r = c.right;
                    break;
                }
            }
        }

        // Descend to the leftmost half with something in it.
        let bm;
        loop {
            let c = self.cell(r);
            if self.is_left_ref(r) {
                b = b.left();
                r = c.left;
            } else if want(c.left) != 0 {
                b = b.left();
                bm = c.left;
                break;
            } else if self.is_right_ref(r) {
                b = b.right();
                r = c.right;
            } else {
                b = b.right();
                bm = c.right;
                break;
            }
        }

        offset_bitmap_bin(b, want(bm)).base_left()
    }

    /// The last filled base bin, or [`Bin::NONE`].
    pub fn find_last_filled(&self) -> Bin {
        if self.is_empty() {
            return Bin::NONE;
        }

        let mut b = self.root_bin;
        while b.layer() != 0 {
            if self.empty_at(b.right()) {
                b = b.left();
            } else {
                b = b.right();
            }
        }
        b
    }

    /// Iterates the empty bins in the root.
    pub fn iterate_empty(&self) -> Iter<'_> {
        Iter::new_empty_at(self, self.root_bin)
    }

    /// Iterates the filled bins in the root.
    pub fn iterate_filled(&self) -> Iter<'_> {
        Iter::new_filled_at(self, self.root_bin)
    }

    /// Iterates the empty bins in `b`.
    pub fn iterate_empty_at(&self, b: Bin) -> Iter<'_> {
        Iter::new_empty_at(self, b)
    }

    /// Iterates the filled bins in `b`.
    pub fn iterate_filled_at(&self, b: Bin) -> Iter<'_> {
        Iter::new_filled_at(self, b)
    }
}

// Accessors for the debug format and in-crate tests.
impl Map {
    pub(crate) fn raw_parts(&self) -> (Ref, usize, usize, Bin, &[Cell]) {
        (
            self.free_top,
            self.alloc_count,
            self.cell_count,
            self.root_bin,
            &self.cells,
        )
    }

    pub(crate) fn from_raw_parts(
        free_top: Ref,
        alloc_count: usize,
        cell_count: usize,
        root_bin: Bin,
        cells: Vec<Cell>,
    ) -> Map {
        Map {
            free_top,
            alloc_count,
            cell_count,
            root_bin,
            cells,
        }
    }

    /// Whether data cell `r`'s halves are refs, for the debug printer.
    pub(crate) fn ref_flags(&self, r: Ref) -> (bool, bool) {
        (self.is_left_ref(r), self.is_right_ref(r))
    }
}

#[cfg(test)]
#[path = "map_test.rs"]
mod tests;
