// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Iterators over the filled or empty bins of a [`Map`], and over the
//! intersection of two such walks.
//!
//! - [`Cursor`] and [`IntersectionCursor`] hold only the iteration state and
//!   take the map(s) on every step, so each step sees the map as it is then.
//!   Use them when the map changes between steps (ppspp resets bins of a map
//!   it is intersecting over), or to keep a position in a struct without a
//!   lifetime.
//! - [`Iter`] and [`IntersectionIter`] borrow the map(s), implement
//!   [`Iterator`], and offer the stepping methods `next_base`,
//!   `next_base_after` and `next_after`.
//!
//! Every stepping method returns the new [`value`](Iter::value), or `None`
//! once the walk is done. The value is updated even when the step returns
//! `None`. A walk ends at the last base bin of the bin space and then keeps
//! returning `None`.

use crate::bin::Bin;
use crate::map::Map;

/// Which bins a [`Cursor`] visits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Empty,
    Filled,
}

/// The last base bin of the bin space, `ALL.base_right()`.
const LAST_BASE: Bin = Bin::ALL.base_right();

/// Iteration state over the empty or filled bins of a map within a bin.
/// The map is passed to every step.
///
/// Steps must move forward: `*_after(b)` with `b` before the current run
/// trusts the cached gap and returns `b` unchecked, so it is only meaningful
/// on a fresh cursor.
///
/// A cursor over [`Bin::NONE`] visits nothing.
#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    kind: Kind,
    /// Set by the first step. A fresh cursor sits at `base_left - 2`, which
    /// wraps when iterating from base 0; once started, a wrap means the walk
    /// ran off the end of the bin space.
    started: bool,
    i: Bin,
    end: Bin,
    gap_left: Bin,
    gap_right: Bin,
}

impl Cursor {
    /// Visits the empty bins in `b`.
    pub fn empty_at(b: Bin) -> Cursor {
        Cursor::new(Kind::Empty, b)
    }

    /// Visits the filled bins in `b`.
    pub fn filled_at(b: Bin) -> Cursor {
        Cursor::new(Kind::Filled, b)
    }

    fn new(kind: Kind, b: Bin) -> Cursor {
        let i = b.base_left() - 2;
        Cursor {
            kind,
            started: false,
            i,
            end: b.base_right(),
            gap_left: Bin(0),
            gap_right: i,
        }
    }

    /// True for a cursor over [`Bin::NONE`], the only bin whose
    /// `base_right` is NONE.
    #[inline]
    fn is_none_bin(&self) -> bool {
        self.end.is_none()
    }

    /// The first base bin at or after `i` in the other state.
    #[inline]
    fn find_neg(&self, m: &Map, i: Bin) -> Bin {
        match self.kind {
            Kind::Empty => m.find_filled_after(i),
            Kind::Filled => m.find_empty_after(i),
        }
    }

    /// The first base bin at or after `i` in the wanted state.
    #[inline]
    fn find_pos(&self, m: &Map, i: Bin) -> Bin {
        match self.kind {
            Kind::Empty => m.find_empty_after(i),
            Kind::Filled => m.find_filled_after(i),
        }
    }

    fn init_gap(&mut self, m: &Map, mut i: Bin) -> Bin {
        let mut gap = self.find_neg(m, i);
        if gap == i {
            i = self.find_pos(m, i);
            gap = self.find_neg(m, i);
        }

        self.gap_left = gap;
        self.gap_right = gap;
        i
    }

    /// Steps to the next base bin.
    pub fn next_base(&mut self, m: &Map) -> Option<Bin> {
        // NONE marks a finished walk.
        if self.is_none_bin() || self.started && self.i >= LAST_BASE {
            return None;
        }
        self.next_base_after(m, self.i + 2)
    }

    /// Steps to the first base bin at or after `i`.
    pub fn next_base_after(&mut self, m: &Map, mut i: Bin) -> Option<Bin> {
        if self.is_none_bin() {
            return None;
        }
        self.started = true;
        if i >= self.gap_right + 2 {
            i = self.init_gap(m, i);
        } else if i >= self.gap_left {
            i = self.init_gap(m, self.gap_right + 2);
        }

        self.i = i.base_left();
        (i <= self.end).then_some(self.i)
    }

    /// Steps to the next bin: the largest aligned bin that starts where the
    /// previous one ended and stays inside the run.
    pub fn next(&mut self, m: &Map) -> Option<Bin> {
        // NONE marks a finished walk.
        if self.is_none_bin() || self.started && self.i.base_right() >= LAST_BASE {
            return None;
        }
        let started = core::mem::replace(&mut self.started, true);
        let mut i = self.i.layer_right();
        let mut imin = i.base_left();
        if imin >= self.gap_left {
            // Nothing follows a gap at the last base (or a NONE gap, "no
            // other state after this run"). A fresh cursor's gap is its
            // `base_left - 2` start position, which may wrap.
            if started && self.gap_right >= LAST_BASE {
                self.i = Bin::NONE;
                return None;
            }
            i = self.init_gap(m, self.gap_right + 2);
            imin = i;
        }
        self.next_from(i, imin)
    }

    /// Steps to the first bin at or after `i`.
    pub fn next_after(&mut self, m: &Map, mut i: Bin) -> Option<Bin> {
        if self.is_none_bin() {
            return None;
        }
        self.started = true;
        let mut imin = i.base_left();
        if i >= self.gap_right + 2 {
            i = self.init_gap(m, i);
            imin = i;
        } else if i >= self.gap_left {
            i = self.init_gap(m, self.gap_right + 2);
            imin = i;
        }
        self.next_from(i, imin)
    }

    /// Climbs from `i` to the largest bin that starts no earlier than `imin`
    /// and stays inside the run and the walk's bound.
    fn next_from(&mut self, mut i: Bin, imin: Bin) -> Option<Bin> {
        if i.is_none() {
            self.i = i;
            return None;
        }

        loop {
            let t = i.parent();
            if imin > t.base_left() || t.base_right() >= self.gap_left || t.base_right() > self.end
            {
                break;
            }
            i = t;
        }

        while i.base_right() >= self.gap_left && !i.is_base() {
            i = i.left();
        }

        self.i = i;
        (i <= self.end).then_some(i)
    }

    /// The current bin.
    #[inline]
    pub fn value(&self) -> Bin {
        self.i
    }
}

/// [`Cursor`] bound to a map. [`Iterator::next`] yields the largest aligned
/// bins of each run; [`Iter::bases`] yields base bins.
///
/// To resume a walk from a bin:
///
/// ```
/// # use strims_binmap::{Bin, Iter, Map};
/// # let mut m = Map::new();
/// # m.set(Bin(4));
/// # m.set(Bin(6));
/// let mut it = Iter::new_filled_at(&m, m.root_bin());
/// let mut next = it.next_base_after(Bin(4));
/// while let Some(b) = next {
///     // ...
///     next = it.next_base();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Iter<'a> {
    map: &'a Map,
    cursor: Cursor,
}

impl<'a> Iter<'a> {
    /// Visits the empty bins of `m` in `b`.
    pub fn new_empty_at(m: &'a Map, b: Bin) -> Iter<'a> {
        Iter {
            map: m,
            cursor: Cursor::empty_at(b),
        }
    }

    /// Visits the filled bins of `m` in `b`.
    pub fn new_filled_at(m: &'a Map, b: Bin) -> Iter<'a> {
        Iter {
            map: m,
            cursor: Cursor::filled_at(b),
        }
    }

    /// Binds a detached cursor to `m`.
    pub fn from_cursor(m: &'a Map, cursor: Cursor) -> Iter<'a> {
        Iter { map: m, cursor }
    }

    /// The detached state, to resume later with [`Iter::from_cursor`].
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// See [`Cursor::next_base`].
    pub fn next_base(&mut self) -> Option<Bin> {
        self.cursor.next_base(self.map)
    }

    /// See [`Cursor::next_base_after`].
    pub fn next_base_after(&mut self, i: Bin) -> Option<Bin> {
        self.cursor.next_base_after(self.map, i)
    }

    /// See [`Cursor::next_after`].
    pub fn next_after(&mut self, i: Bin) -> Option<Bin> {
        self.cursor.next_after(self.map, i)
    }

    /// The current bin.
    #[inline]
    pub fn value(&self) -> Bin {
        self.cursor.value()
    }

    /// Yields base bins.
    pub fn bases(self) -> Bases<'a> {
        Bases(self)
    }
}

impl Iterator for Iter<'_> {
    type Item = Bin;

    fn next(&mut self) -> Option<Bin> {
        self.cursor.next(self.map)
    }
}

/// [`Iter`] stepping by [`Iter::next_base`].
#[derive(Clone, Debug)]
pub struct Bases<'a>(Iter<'a>);

impl Iterator for Bases<'_> {
    type Item = Bin;

    fn next(&mut self) -> Option<Bin> {
        self.0.next_base()
    }
}

/// Which side produced the last [`IntersectionCursor`] value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    None,
    A,
    B,
}

/// Iteration state over the bins two cursors both visit, e.g. bins empty in
/// one map and filled in another. Every step takes the maps for `a` and `b`
/// in that order.
#[derive(Clone, Copy, Debug)]
pub struct IntersectionCursor {
    a: Cursor,
    b: Cursor,
    p: Side,
    i: Bin,
}

impl IntersectionCursor {
    /// Intersects the walks of `a` and `b`.
    pub fn new(a: Cursor, b: Cursor) -> IntersectionCursor {
        IntersectionCursor {
            a,
            b,
            p: Side::None,
            i: Bin(0),
        }
    }

    /// Steps to the next base bin both walks visit.
    pub fn next_base(&mut self, ma: &Map, mb: &Map) -> Option<Bin> {
        self.a.next_base(ma)?;
        self.b.next_base(mb)?;

        while self.a.value() != self.b.value() {
            while self.a.value() < self.b.value() {
                self.a.next_base_after(ma, self.b.value())?;
            }
            while self.b.value() < self.a.value() {
                self.b.next_base_after(mb, self.a.value())?;
            }
        }

        self.i = self.a.value();
        Some(self.i)
    }

    /// Steps to the next bin both walks cover: the smaller of two
    /// overlapping values.
    pub fn next(&mut self, ma: &Map, mb: &Map) -> Option<Bin> {
        match self.p {
            Side::None => {
                self.a.next(ma)?;
                self.b.next_after(mb, self.a.value().base_left())?;
            }
            Side::A => {
                self.a.next(ma)?;
            }
            Side::B => {
                self.b.next(mb)?;
            }
        }
        self.next_overlap(ma, mb)
    }

    /// Steps to the first bin both walks cover at or after `i`.
    pub fn next_after(&mut self, ma: &Map, mb: &Map, i: Bin) -> Option<Bin> {
        self.a.next_after(ma, i)?;
        self.b.next_after(mb, self.a.value().base_left())?;
        self.next_overlap(ma, mb)
    }

    /// Advances whichever cursor is behind until one value contains the
    /// other, and yields the smaller.
    fn next_overlap(&mut self, ma: &Map, mb: &Map) -> Option<Bin> {
        loop {
            let (a, b) = (self.a.value(), self.b.value());
            if a.base_right() < b.base_right() && !b.contains(a) {
                self.a.next_after(ma, b.base_left())?;
            }

            let (a, b) = (self.a.value(), self.b.value());
            if b.contains(a) {
                self.i = a;
                self.p = Side::A;
                return Some(a);
            }

            if b.base_right() < a.base_right() && !a.contains(b) {
                self.b.next_after(mb, a.base_left())?;
            }

            let (a, b) = (self.a.value(), self.b.value());
            if a.contains(b) {
                self.i = b;
                self.p = Side::B;
                return Some(b);
            }
        }
    }

    /// The current bin.
    #[inline]
    pub fn value(&self) -> Bin {
        self.i
    }
}

/// [`IntersectionCursor`] bound to its maps. [`IntersectionIter::bases`]
/// yields base bins.
#[derive(Clone, Debug)]
pub struct IntersectionIter<'a> {
    ma: &'a Map,
    mb: &'a Map,
    cursor: IntersectionCursor,
}

impl<'a> IntersectionIter<'a> {
    /// Intersects two walks, each over its own map.
    pub fn new(a: Iter<'a>, b: Iter<'a>) -> IntersectionIter<'a> {
        IntersectionIter {
            ma: a.map,
            mb: b.map,
            cursor: IntersectionCursor::new(a.cursor, b.cursor),
        }
    }

    /// The detached state.
    pub fn cursor(&self) -> IntersectionCursor {
        self.cursor
    }

    /// See [`IntersectionCursor::next_base`].
    pub fn next_base(&mut self) -> Option<Bin> {
        self.cursor.next_base(self.ma, self.mb)
    }

    /// See [`IntersectionCursor::next_after`].
    pub fn next_after(&mut self, i: Bin) -> Option<Bin> {
        self.cursor.next_after(self.ma, self.mb, i)
    }

    /// The current bin.
    #[inline]
    pub fn value(&self) -> Bin {
        self.cursor.value()
    }

    /// Yields base bins.
    pub fn bases(self) -> IntersectionBases<'a> {
        IntersectionBases(self)
    }
}

impl Iterator for IntersectionIter<'_> {
    type Item = Bin;

    fn next(&mut self) -> Option<Bin> {
        self.cursor.next(self.ma, self.mb)
    }
}

/// [`IntersectionIter`] stepping by [`IntersectionIter::next_base`].
#[derive(Clone, Debug)]
pub struct IntersectionBases<'a>(IntersectionIter<'a>);

impl Iterator for IntersectionBases<'_> {
    type Item = Bin;

    fn next(&mut self) -> Option<Bin> {
        self.0.next_base()
    }
}

#[cfg(test)]
#[path = "iterator_test.rs"]
mod tests;
