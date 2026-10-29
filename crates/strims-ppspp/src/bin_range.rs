// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Runs of base bins `lo..hi` (`hi` exclusive) as the aligned bins that
//! cover them exactly: at most two per layer, so map operations over a run
//! cost O(log) bins, not one per chunk.

use strims_binmap::{Bin, Map};

/// The largest aligned bins covering the base bins from `lo` up to `hi`,
/// in stream order. Empty if `hi <= lo`.
pub(crate) fn aligned(lo: Bin, hi: Bin) -> impl Iterator<Item = Bin> {
    let mut o = lo.base_offset();
    let end = hi.base_left().base_offset();
    core::iter::from_fn(move || {
        if hi.is_none() || o >= end {
            return None;
        }
        let len = end - o;
        // The layer: as aligned as `o` is, as long as the run is left, and
        // below the top layer.
        let k = o
            .trailing_zeros()
            .min(63 - len.leading_zeros())
            .min(MAX_LAYER);
        let b = Bin::new(u64::from(k), o >> k);
        o = o.saturating_add(1 << k);
        Some(b)
    })
}

/// The highest layer a run's bin takes: bins stay below [`Bin::ALL`].
const MAX_LAYER: u32 = 61;

/// Empties the base bins from `lo` up to `hi` in `m`.
pub(crate) fn reset(m: &mut Map, lo: Bin, hi: Bin) {
    for b in aligned(lo, hi) {
        m.reset(b);
    }
}

/// Fills in `dst` the base bins from `lo` up to `hi` that `src` has filled.
/// O(filled runs of `src` in the range).
pub(crate) fn copy_filled(dst: &mut Map, src: &Map, lo: Bin, hi: Bin) {
    for r in aligned(lo, hi) {
        for f in src.iterate_filled_at(r) {
            dst.set(f);
        }
    }
}

/// Empties in `dst` the base bins inside `within` that `src` has filled.
/// O(filled runs of `src` in `within`).
pub(crate) fn subtract_filled(dst: &mut Map, src: &Map, within: Bin) {
    for f in src.iterate_filled_at(within) {
        dst.reset(f);
    }
}

/// True if every base bin from `lo` up to `hi` that `m` has empty is
/// filled in `cover`.
pub(crate) fn empty_covered(m: &Map, cover: &Map, lo: Bin, hi: Bin) -> bool {
    aligned(lo, hi).all(|r| m.iterate_empty_at(r).all(|e| cover.filled_at(e)))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn bases(it: impl Iterator<Item = Bin>) -> Vec<u64> {
        it.flat_map(|b| {
            let (l, r) = b.base();
            (l.base_offset()..=r.base_offset()).collect::<Vec<_>>()
        })
        .collect()
    }

    /// A run's bins are aligned, in order, and cover it exactly.
    #[test]
    fn aligned_bins_cover_a_run_exactly() {
        for lo in 0..40u64 {
            for hi in 0..80u64 {
                let bins: Vec<Bin> = aligned(Bin(lo * 2), Bin(hi * 2)).collect();
                let want: Vec<u64> = (lo..hi.max(lo)).collect();
                assert_eq!(bases(bins.iter().copied()), want, "{lo}..{hi}");
                assert!(bins.len() <= 14, "{lo}..{hi}: {}", bins.len());
            }
        }
        // a run near the top of the bin space stays below ALL
        let top = (1u64 << 61) - 8;
        let bins: Vec<Bin> = aligned(Bin(top * 2), Bin((1 << 62) - 2)).collect();
        assert!(bins.iter().all(|b| !b.is_all() && !b.is_none()));
    }

    /// Copying, subtracting and covering touch only the run.
    #[test]
    fn map_operations_stay_inside_the_run() {
        let mut src = Map::new();
        src.set(Bin::new(4, 0)); // chunks 0..16
        src.set(Bin(2 * 20));
        let mut dst = Map::new();
        copy_filled(&mut dst, &src, Bin(2 * 3), Bin(2 * 21));
        let filled: Vec<u64> = (0..32).filter(|&o| dst.filled_at(Bin(o * 2))).collect();
        assert_eq!(filled, (3..16).chain([20]).collect::<Vec<_>>());

        reset(&mut dst, Bin(2 * 5), Bin(2 * 8));
        let filled: Vec<u64> = (0..32).filter(|&o| dst.filled_at(Bin(o * 2))).collect();
        assert_eq!(filled, (3..5).chain(8..16).chain([20]).collect::<Vec<_>>());

        let mut cut = Map::new();
        cut.set(Bin(2 * 9));
        cut.set(Bin(2 * 30));
        subtract_filled(&mut dst, &cut, Bin::new(4, 0));
        assert!(dst.empty_at(Bin(2 * 9)) && dst.filled_at(Bin(2 * 10)));

        // dst lacks 0..3, 5..8, 9, 16..20 and 21..; cover them in part
        let mut cover = Map::new();
        cover.set(Bin::new(3, 0));
        cover.set(Bin(2 * 9));
        assert!(empty_covered(&dst, &cover, Bin(0), Bin(2 * 10)));
        assert!(!empty_covered(&dst, &cover, Bin(0), Bin(2 * 17)));
        // outside both maps' roots everything is empty
        assert!(!empty_covered(&dst, &cover, Bin(2 * 100), Bin(2 * 101)));
        assert!(empty_covered(&dst, &cover, Bin(2 * 100), Bin(2 * 100)));
    }
}
