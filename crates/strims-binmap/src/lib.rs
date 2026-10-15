// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Logarithmic bin numbering and compact bin bitmaps.
//!
//! Implements the binmap described in Grishchenko, Victor, and Johan
//! Pouwelse. "Binmaps: hybridizing bitmaps and binary trees." PDS Technical
//! Report PDS-2011-20001 (2011).
//!
//! - [`Bin`]: bin numbers and their arithmetic.
//! - [`Map`]: the set of filled bins, stored as a tree of 8 byte cells in a
//!   single vector with a free list.
//! - [`Iter`], [`IntersectionIter`], [`Cursor`], [`IntersectionCursor`]:
//!   walks over filled or empty bins.
//! - [`Map::debug_string`] and [`Map::debug_parse`]: a text dump of the
//!   cells, for tests.
//!
//! Bins from peers may be any `u64`. No input panics, and every walk
//! terminates, but the caller bounds the work: see [`Map`].

mod bin;
mod bitmap;
mod cell;
mod debug;
mod iterator;
mod map;

pub use bin::Bin;
pub use debug::DebugParseError;
pub use iterator::{Bases, Cursor, IntersectionBases, IntersectionCursor, IntersectionIter, Iter};
pub use map::Map;
