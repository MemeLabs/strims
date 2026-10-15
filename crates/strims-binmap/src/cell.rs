// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The 8 byte cell all map nodes share.
//!
//! A [`Map`](crate::Map) keeps every cell in one `Vec<Cell>`, indexed by
//! [`Ref`]. A cell plays one of three roles:
//!
//! - **data cell**: `left` and `right` each hold either a [`Bitmap`] or the
//!   [`Ref`] of a child data cell.
//! - **map cell**: every index divisible by 32 holds the "is a ref" flags for
//!   the next 31 cells. Bit `r & 31` of `left` says whether data cell `r`'s
//!   left half is a ref, and likewise for `right`.
//! - **free cell**: `left` holds the next free [`Ref`].
//!
//! The role-specific accessors are methods on [`Map`](crate::Map) taking a
//! [`Ref`], so no borrow of the cell vector outlives a single access.

use crate::bitmap::Bitmap;

/// Index of a cell in [`Map`](crate::Map)'s cell vector.
pub(crate) type Ref = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) left: u32,
    pub(crate) right: u32,
}

impl Cell {
    /// Both halves hold the same value.
    #[inline]
    pub(crate) fn symmetric(self) -> bool {
        self.left == self.right
    }

    /// A data cell with `b` in both halves.
    #[inline]
    pub(crate) fn uniform(b: Bitmap) -> Cell {
        Cell { left: b, right: b }
    }
}

/// The map cell holding `r`'s ref flags.
#[inline]
pub(crate) fn map_ref(r: Ref) -> Ref {
    r & !0x1f
}

/// True for the indices reserved for map cells.
#[inline]
pub(crate) fn is_map_ref(r: Ref) -> bool {
    r & 0x1f == 0
}

/// `r`'s bit within its map cell.
#[inline]
pub(crate) fn map_mask(r: Ref) -> u32 {
    r & 0x1f
}
