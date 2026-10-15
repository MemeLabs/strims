// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Numbering for aligned logarithmic bins.
//!
//! Each number stands for the interval
//! `[layer_offset * 2^layer, (layer_offset + 1) * 2^layer)` of base chunks.
//! Bins use the "tail 111" encoding: the meaningless low bits are `0111...1`
//! and the head is the offset, so
//! `bin = 2^(layer + 1) * layer_offset + 2^layer - 1`.
//!
//! ```text
//!               7
//!           /       \
//!       3              11
//!     /   \           /  \
//!   1       5       9     13
//!  / \     / \     / \    / \
//! 0   2   4   6   8  10  12 14
//! ```
//!
//! Arithmetic is modulo 2^64 and shifts by 64 or more yield 0, so every
//! operation is defined for every `u64`, the [`Bin::NONE`] and [`Bin::ALL`]
//! sentinels included: bin numbers come from peers.

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// The highest layer [`Bin::new`] accepts.
const MAX_LAYER: u64 = 63;

/// A bin number.
///
/// The field is public so bins convert freely to and from `u64`. `+` and `-`
/// with a `u64` wrap (for example `Bin::NONE + 2 == Bin(1)`), and ordering is
/// the numeric order of the bin numbers.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bin(pub u64);

impl Bin {
    /// No bin.
    pub const NONE: Bin = Bin(0xffff_ffff_ffff_ffff);
    /// The bin covering every base chunk.
    pub const ALL: Bin = Bin(0x7fff_ffff_ffff_ffff);

    /// The bin at `offset` in `layer`, or [`Bin::NONE`] if `layer > 63`.
    #[inline]
    pub const fn new(layer: u64, offset: u64) -> Bin {
        if layer <= MAX_LAYER {
            Bin((offset.wrapping_mul(2).wrapping_add(1) << layer).wrapping_sub(1))
        } else {
            Bin::NONE
        }
    }

    #[inline]
    pub const fn is_all(self) -> bool {
        self.0 == Bin::ALL.0
    }

    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == Bin::NONE.0
    }

    /// The tail bits of the encoding, `b ^ (b + 1)`.
    #[inline]
    pub const fn layer_bits(self) -> Bin {
        Bin(self.0 ^ self.0.wrapping_add(1))
    }

    #[inline]
    pub const fn parent(self) -> Bin {
        let lbs = self.layer_bits().0;
        let nlbs = !lbs.wrapping_add(1);
        Bin((self.0 | lbs) & nlbs)
    }

    /// Left descendant.
    #[inline]
    pub const fn left(self) -> Bin {
        let t = self.0.wrapping_add(1);
        Bin(self.0 ^ (t.isolate_lowest_one() >> 1))
    }

    /// Right descendant.
    #[inline]
    pub const fn right(self) -> Bin {
        let t = self.0.wrapping_add(1);
        Bin(self.0.wrapping_add(t.isolate_lowest_one() >> 1))
    }

    /// Leftmost base bin in `self`.
    #[inline]
    pub const fn base_left(self) -> Bin {
        if self.is_none() {
            return Bin::NONE;
        }
        Bin(self.0 & self.0.wrapping_add(1))
    }

    /// Rightmost base bin in `self`.
    #[inline]
    pub const fn base_right(self) -> Bin {
        if self.is_none() {
            return Bin::NONE;
        }
        Bin((self.0 | self.0.wrapping_add(1)).wrapping_sub(1))
    }

    /// `(base_left, base_right)`.
    #[inline]
    pub const fn base(self) -> (Bin, Bin) {
        (self.base_left(), self.base_right())
    }

    /// The bin at layer offset - 1, or [`Bin::NONE`].
    #[inline]
    pub const fn layer_left(self) -> Bin {
        if self.is_none() {
            return Bin::NONE;
        }
        let t = self.layer_bits().0.wrapping_add(1);
        if self.0 < t {
            return Bin::NONE;
        }
        Bin(self.0 - t)
    }

    /// The bin at layer offset + 1.
    #[inline]
    pub const fn layer_right(self) -> Bin {
        if self.is_none() {
            return Bin::NONE;
        }
        Bin(self.0.wrapping_add(self.layer_bits().0).wrapping_add(1))
    }

    /// True if `self` is in the base layer.
    #[inline]
    pub const fn is_base(self) -> bool {
        self.0 & 1 == 0
    }

    /// Index within the layer.
    #[inline]
    pub const fn layer_offset(self) -> u64 {
        shr(self.0, self.layer() + 1)
    }

    /// The leftmost bin at layer `z` above or below `self`.
    #[inline]
    pub const fn layer_shift(self, z: u64) -> Bin {
        let t = shl(1, z);
        Bin((self.0 & !(self.layer_bits().0 | t)) | t.wrapping_sub(1))
    }

    /// True if `o` is equal to or a descendant of `self`.
    #[inline]
    pub const fn contains(self, o: Bin) -> bool {
        if self.is_none() {
            return false;
        }
        let t = self.0.wrapping_add(1);
        (self.0 & t) <= o.0 && o.0 < (self.0 | t)
    }

    /// Tree height at `self`; 64 for [`Bin::NONE`].
    #[inline]
    pub const fn layer(self) -> u64 {
        self.0.wrapping_add(1).trailing_zeros() as u64
    }

    /// Index of the leftmost base bin.
    #[inline]
    pub const fn base_offset(self) -> u64 {
        (self.0 & self.0.wrapping_add(1)) >> 1
    }

    /// Number of base bins in `self`; 0 for [`Bin::NONE`].
    #[inline]
    pub const fn base_length(self) -> u64 {
        self.0.wrapping_add(1).isolate_lowest_one()
    }

    #[inline]
    pub const fn sibling(self) -> Bin {
        Bin(self.0 ^ self.layer_bits().0.wrapping_add(1))
    }

    #[inline]
    pub const fn is_left(self) -> bool {
        self.0 & self.layer_bits().0.wrapping_add(1) == 0
    }

    #[inline]
    pub const fn is_right(self) -> bool {
        !self.is_left()
    }
}

/// `x << n`, or 0 once `n` reaches the width.
#[inline]
const fn shl(x: u64, n: u64) -> u64 {
    if n < 64 { x << n } else { 0 }
}

/// `x >> n`, or 0 once `n` reaches the width.
#[inline]
const fn shr(x: u64, n: u64) -> u64 {
    if n < 64 { x >> n } else { 0 }
}

impl From<u64> for Bin {
    #[inline]
    fn from(b: u64) -> Bin {
        Bin(b)
    }
}

impl From<Bin> for u64 {
    #[inline]
    fn from(b: Bin) -> u64 {
        b.0
    }
}

impl Add<u64> for Bin {
    type Output = Bin;
    #[inline]
    fn add(self, n: u64) -> Bin {
        Bin(self.0.wrapping_add(n))
    }
}

impl AddAssign<u64> for Bin {
    #[inline]
    fn add_assign(&mut self, n: u64) {
        *self = *self + n;
    }
}

impl Sub<u64> for Bin {
    type Output = Bin;
    #[inline]
    fn sub(self, n: u64) -> Bin {
        Bin(self.0.wrapping_sub(n))
    }
}

impl SubAssign<u64> for Bin {
    #[inline]
    fn sub_assign(&mut self, n: u64) {
        *self = *self - n;
    }
}

/// `ALL`, `NONE` or the decimal bin number.
impl fmt::Display for Bin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_all() {
            f.write_str("ALL")
        } else if self.is_none() {
            f.write_str("NONE")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl fmt::Debug for Bin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bin({self})")
    }
}

#[cfg(test)]
#[path = "bin_test.rs"]
mod tests;
