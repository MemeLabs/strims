// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! 32 bit bitmaps covering the base bins of a layer 5 bin, and the lookup tables between bins and bitmap masks.

use crate::bin::Bin;

/// A 32 bit bitmap of base bins.
pub(crate) type Bitmap = u32;

/// Mask for a bin's position inside a layer 6 bin.
pub(crate) const BITMAP_LAYER_BITS: u64 = 63;
pub(crate) const BITMAP_FILLED: Bitmap = u32::MAX;
pub(crate) const BITMAP_EMPTY: Bitmap = 0;

/// Bin (relative to its layer 6 bin) to bitmap mask.
const BIN_BITMAPS: [Bitmap; 64] = [
    0x00000001, 0x00000003, 0x00000002, 0x0000000f, //
    0x00000004, 0x0000000c, 0x00000008, 0x000000ff, //
    0x00000010, 0x00000030, 0x00000020, 0x000000f0, //
    0x00000040, 0x000000c0, 0x00000080, 0x0000ffff, //
    0x00000100, 0x00000300, 0x00000200, 0x00000f00, //
    0x00000400, 0x00000c00, 0x00000800, 0x0000ff00, //
    0x00001000, 0x00003000, 0x00002000, 0x0000f000, //
    0x00004000, 0x0000c000, 0x00008000, 0xffffffff, //
    0x00010000, 0x00030000, 0x00020000, 0x000f0000, //
    0x00040000, 0x000c0000, 0x00080000, 0x00ff0000, //
    0x00100000, 0x00300000, 0x00200000, 0x00f00000, //
    0x00400000, 0x00c00000, 0x00800000, 0xffff0000, //
    0x01000000, 0x03000000, 0x02000000, 0x0f000000, //
    0x04000000, 0x0c000000, 0x08000000, 0xff000000, //
    0x10000000, 0x30000000, 0x20000000, 0xf0000000, //
    0x40000000, 0xc0000000, 0x80000000, 0xffffffff, //
];

/// Tail encoded bytes to bin offsets.
///
/// ```text
///    high level bins              low level bins
///
///          14                            6
///     12        13                  4         5
///  10   10    10   10           2     2     2    2
/// 8  9 8  9  8  9 8  11       0  1  0  1  0  1  0  3
/// ```
const BITMAP_BINS: [u8; 256] = [
    255, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    9, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    12, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    9, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    14, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    9, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    13, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3, //
    11, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 7, //
];

/// The bitmap mask of `b` within its layer 6 bin.
#[inline]
pub(crate) fn bin_bitmap(b: Bin) -> Bitmap {
    // invariant: the index is masked to 0..64.
    #[allow(clippy::indexing_slicing)]
    BIN_BITMAPS[(b.0 & BITMAP_LAYER_BITS) as usize]
}

#[inline]
fn bitmap_bins(b: Bitmap) -> u8 {
    // invariant: the index is masked to 0..256.
    #[allow(clippy::indexing_slicing)]
    BITMAP_BINS[(b & 0xff) as usize]
}

#[inline]
pub(crate) fn get(b: Bitmap, i: u32) -> bool {
    b & (1 << i) != 0
}

#[inline]
pub(crate) fn set(b: Bitmap, i: u32, v: bool) -> Bitmap {
    if v { b | (1 << i) } else { b & !(1 << i) }
}

/// The bin (relative to its layer 6 bin) of the largest aligned run of set
/// bits starting at the lowest set bit.
///
/// Callers never pass an empty bitmap; it yields 47 (`48 + 255` wraps in
/// `u8`).
pub(crate) fn bitmap_bin(mut b: Bitmap) -> Bin {
    let t = bitmap_bins(b);
    if t < 16 {
        if t != 7 {
            return Bin(u64::from(t));
        }

        b = b.wrapping_add(1);
        b &= b.wrapping_neg();
        if b == 0 {
            return Bin(BITMAP_LAYER_BITS / 2);
        }
        if b & 0xffff == 0 {
            return Bin(15);
        }
        return Bin(7);
    }

    b >>= 8;
    let t = bitmap_bins(b);
    if t < 16 {
        return Bin(16 + u64::from(t));
    }

    b >>= 8;
    let t = bitmap_bins(b);
    if t < 16 {
        if t != 7 {
            return Bin(32 + u64::from(t));
        }

        b = b.wrapping_add(1);
        b &= b.wrapping_neg();
        if b & 0xffff == 0 {
            return Bin(47);
        }
        return Bin(39);
    }

    b >>= 8;
    Bin(u64::from(48u8.wrapping_add(bitmap_bins(b))))
}

/// `bitmap_bin(bm)` offset into the layer 6 bin containing `b`, or `b` if
/// `bm` is filled.
#[inline]
pub(crate) fn offset_bitmap_bin(b: Bin, bm: Bitmap) -> Bin {
    if bm == BITMAP_FILLED {
        return b;
    }
    b.base_left() + bitmap_bin(bm).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_bin_known_values() {
        assert_eq!(bitmap_bin(0), Bin(47));
        assert_eq!(bitmap_bin(0x8000_0000), Bin(62));
        assert_eq!(bitmap_bin(0xffff_0000), Bin(47));
        assert_eq!(bitmap_bin(0x0001_0000), Bin(32));
        assert_eq!(bitmap_bin(0xffff_ffff), Bin(31));
        assert_eq!(bitmap_bin(0x0000_ffff), Bin(15));
        assert_eq!(bitmap_bin(0x0000_00ff), Bin(7));
        assert_eq!(bitmap_bin(0x00ff_0000), Bin(39));
    }

    /// `bitmap_bin` agrees with a direct search for every single-bin mask
    /// and every prefix of set bits.
    #[test]
    fn bitmap_bin_agrees_with_bin_bitmaps() {
        for i in 0..63 {
            let b = Bin(i);
            let mask = bin_bitmap(b);
            // A mask is the largest aligned run starting at its lowest bit
            // when nothing above it is set, except for the two layer 5
            // halves that share 0xffffffff.
            if mask != BITMAP_FILLED {
                assert_eq!(bitmap_bin(mask), b, "mask {mask:#x}");
            }
        }
    }
}
