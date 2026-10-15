// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Model check: drives two [`Map`]s with random `set`, `reset`,
//! `fill_before` and `reset_before` operations, mirrors them in a plain
//! `Vec<bool>` of base bins, and checks every query and iterator against it.
//!
//! The expectations include the edge cases documented on [`Map::cover`] and
//! [`Map::find_empty_after`]: `cover` may answer ALL for a uniform root,
//! `find_empty_after` returns the target itself outside the root, and
//! iterators decompose each run greedily into the largest aligned bins.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_binmap::{Bin, Cursor, IntersectionCursor, IntersectionIter, Iter, Map};

/// Operations stay below this many base bins, so the root stays at or
/// below layer 12. Queries and iterations reach twice as far.
const UNIVERSE: u64 = 4096;
const QUERY_LIMIT: u64 = 2 * UNIVERSE;

#[derive(Clone, Copy, Debug)]
enum Op {
    Set,
    Reset,
    FillBefore,
    ResetBefore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Filled,
    Empty,
    Mixed,
}

/// A map as a bitmap of base bins, with prefix sums for range queries and
/// next-filled/next-empty tables for the find queries.
struct Model {
    bits: Vec<bool>,
    prefix: Vec<u64>,
    next: [Vec<u64>; 2],
}

impl Model {
    fn new() -> Model {
        Model {
            bits: vec![false; UNIVERSE as usize],
            prefix: vec![],
            next: [vec![], vec![]],
        }
    }

    fn apply(&mut self, op: Op, b: Bin) {
        let (lo, hi) = (b.base_offset(), b.base_offset() + b.base_length());
        let (lo, hi, v) = match op {
            Op::Set => (lo, hi, true),
            Op::Reset => (lo, hi, false),
            Op::FillBefore => (0, lo, true),
            Op::ResetBefore => (0, lo, false),
        };
        for x in lo..hi.min(UNIVERSE) {
            self.bits[x as usize] = v;
        }
    }

    fn freeze(&mut self) {
        self.prefix = std::iter::once(0)
            .chain(self.bits.iter().scan(0, |n, &b| {
                *n += u64::from(b);
                Some(*n)
            }))
            .collect();
        for v in [false, true] {
            let mut next = vec![u64::MAX; QUERY_LIMIT as usize + 1];
            for x in (0..QUERY_LIMIT).rev() {
                next[x as usize] = if self.get(x) == v {
                    x
                } else {
                    next[x as usize + 1]
                };
            }
            self.next[usize::from(v)] = next;
        }
    }

    fn get(&self, x: u64) -> bool {
        x < UNIVERSE && self.bits[x as usize]
    }

    fn count(&self, lo: u64, hi: u64) -> u64 {
        let clamp = |x: u64| x.min(UNIVERSE) as usize;
        self.prefix[clamp(hi)] - self.prefix[clamp(lo)]
    }

    fn state(&self, b: Bin) -> State {
        let (lo, len) = (b.base_offset(), b.base_length());
        let n = self.count(lo, lo.saturating_add(len));
        if n == 0 {
            State::Empty
        } else if n == len {
            State::Filled
        } else {
            State::Mixed
        }
    }

    fn is_empty(&self) -> bool {
        self.count(0, UNIVERSE) == 0
    }

    /// The first base at or after `from` in state `v`, below QUERY_LIMIT.
    fn first(&self, from: u64, v: bool) -> Option<u64> {
        let next = &self.next[usize::from(v)];
        next.get(from as usize).copied().filter(|&x| x != u64::MAX)
    }

    fn last_filled(&self) -> Option<u64> {
        (0..UNIVERSE).rev().find(|&x| self.get(x))
    }
}

/// Splits the bases `[lo, hi)` into the largest aligned bins, left to right.
fn greedy(mut lo: u64, hi: u64, out: &mut Vec<Bin>) {
    while lo < hi {
        let mut l = lo.trailing_zeros().min(63) as u64;
        while lo + (1 << l) > hi {
            l -= 1;
        }
        out.push(Bin::new(l, lo >> l));
        lo += 1 << l;
    }
}

/// Greedy decomposition of the bases in `[lo, hi)` where `pred` holds.
fn runs(lo: u64, hi: u64, pred: impl Fn(u64) -> bool) -> Vec<Bin> {
    let mut out = vec![];
    let mut x = lo;
    while x < hi {
        if !pred(x) {
            x += 1;
            continue;
        }
        let start = x;
        while x < hi && pred(x) {
            x += 1;
        }
        greedy(start, x, &mut out);
    }
    out
}

fn bases(lo: u64, hi: u64, pred: impl Fn(u64) -> bool) -> Vec<Bin> {
    (lo..hi).filter(|&x| pred(x)).map(|x| Bin(2 * x)).collect()
}

fn base_range(b: Bin) -> (u64, u64) {
    (b.base_offset(), b.base_offset() + b.base_length())
}

/// Every bin of layer 0..=13 below QUERY_LIMIT.
fn query_bins() -> impl Iterator<Item = Bin> {
    (0..=13u64).flat_map(|l| (0..QUERY_LIMIT >> l).map(move |o| Bin::new(l, o)))
}

fn check_point_queries(m: &Map, model: &Model) -> Result<(), TestCaseError> {
    let root = m.root_bin();
    prop_assert_eq!(m.is_empty(), model.is_empty());
    prop_assert!(root.base_offset() == 0 && root.layer() >= 6 && root.layer() <= 12);

    for q in query_bins().chain([root, root.parent(), Bin::ALL]) {
        let s = model.state(q);
        prop_assert_eq!(m.filled_at(q), s == State::Filled, "filled_at({})", q);
        prop_assert_eq!(m.empty_at(q), s == State::Empty, "empty_at({})", q);

        let c = m.cover(q);
        match s {
            State::Mixed => prop_assert_eq!(c, Bin::NONE, "cover({}) of mixed", q),
            _ if c == Bin::ALL => prop_assert!(
                model.is_empty() || s == State::Filled && m.is_filled() && q.layer() < 6,
                "cover({}) = ALL",
                q
            ),
            _ => {
                prop_assert!(c.contains(q), "cover({}) = {}", q, c);
                prop_assert_eq!(model.state(c), s, "cover({}) = {}", q, c);
                // Maximal, except the high layer answer inside a uniform root.
                let p = c.parent();
                prop_assert!(
                    model.state(p) != s || p == root,
                    "cover({}) = {} not maximal",
                    q,
                    c
                );
            }
        }

        let first = q.base_offset();

        let f = m.find_filled_after(q);
        match model.first(first, true) {
            Some(x) => prop_assert_eq!(f, Bin(2 * x), "find_filled_after({})", q),
            None => prop_assert_eq!(f, Bin::NONE, "find_filled_after({})", q),
        }

        let e = m.find_empty_after(q);
        let x = model.first(first, false).unwrap_or(u64::MAX);
        prop_assert_eq!(e.base_offset(), x, "find_empty_after({}) = {}", q, e);
        prop_assert_eq!(
            model.state(e),
            State::Empty,
            "find_empty_after({}) = {}",
            q,
            e
        );
        if root.contains(q) {
            prop_assert!(e.is_base(), "find_empty_after({}) = {}", q, e);
        }
    }

    let e = m.find_empty();
    if model.is_empty() {
        prop_assert_eq!(e, Bin::ALL);
    } else {
        prop_assert_eq!(Some(e.base_offset()), model.first(0, false), "find_empty");
        prop_assert_eq!(model.state(e), State::Empty, "find_empty = {}", e);
        let p = e.parent();
        prop_assert!(
            p.base_offset() < e.base_offset() || model.state(p) != State::Empty,
            "find_empty = {} not maximal",
            e
        );
    }

    let f = m.find_filled();
    match model.first(0, true) {
        None => prop_assert_eq!(f, Bin::NONE),
        Some(x) => {
            prop_assert_eq!(f.base_offset(), x, "find_filled = {}", f);
            prop_assert_eq!(model.state(f), State::Filled, "find_filled = {}", f);
            let p = f.parent();
            prop_assert!(
                p.base_offset() < f.base_offset() || model.state(p) != State::Filled,
                "find_filled = {} not maximal",
                f
            );
        }
    }

    match model.last_filled() {
        Some(x) => prop_assert_eq!(m.find_last_filled(), Bin(2 * x)),
        None => prop_assert_eq!(m.find_last_filled(), Bin::NONE),
    }
    Ok(())
}

fn iter<'a>(m: &'a Map, filled: bool, b: Bin) -> Iter<'a> {
    if filled {
        m.iterate_filled_at(b)
    } else {
        m.iterate_empty_at(b)
    }
}

fn cursor(filled: bool, b: Bin) -> Cursor {
    if filled {
        Cursor::filled_at(b)
    } else {
        Cursor::empty_at(b)
    }
}

/// Drains `first` then `next`, with a bound so a looping iterator fails the
/// test.
fn drain(
    first: Option<Bin>,
    mut next: impl FnMut() -> Option<Bin>,
) -> Result<Vec<Bin>, TestCaseError> {
    let mut out = vec![];
    let mut v = first;
    while let Some(b) = v {
        out.push(b);
        prop_assert!(
            out.len() <= 2 * QUERY_LIMIT as usize,
            "iterator does not end"
        );
        v = next();
    }
    Ok(out)
}

/// The bins an iteration over `b` should visit.
fn iteration_bins(m: &Map, extra: Bin) -> Vec<Bin> {
    let root = m.root_bin();
    [root, root.parent(), extra, Bin::new(6, 1), Bin::new(9, 3)]
        .into_iter()
        .filter(|b| b.base_length() <= QUERY_LIMIT)
        .collect()
}

fn check_iterators(m: &Map, model: &Model, extra: Bin, start: u64) -> Result<(), TestCaseError> {
    for b in iteration_bins(m, extra) {
        let (lo, hi) = base_range(b);
        for filled in [true, false] {
            let pred = |x: u64| model.get(x) == filled;

            let got: Vec<_> = {
                let mut it = iter(m, filled, b);
                let first = it.next();
                drain(first, || it.next())?
            };
            prop_assert_eq!(&got, &runs(lo, hi, pred), "next {} filled {}", b, filled);

            let got: Vec<_> = {
                let mut it = iter(m, filled, b).bases();
                let first = it.next();
                drain(first, || it.next())?
            };
            prop_assert_eq!(&got, &bases(lo, hi, pred), "bases {} filled {}", b, filled);

            // Resuming at a base bin continues the same decomposition from
            // there.
            let s = Bin(2 * start);
            let from = lo.max(start);
            let got: Vec<_> = {
                let mut it = iter(m, filled, b);
                let first = it.next_after(s);
                drain(first, || it.next())?
            };
            prop_assert_eq!(
                &got,
                &runs(from, hi, pred),
                "after {} {} filled {}",
                s,
                b,
                filled
            );

            let got: Vec<_> = {
                let mut it = iter(m, filled, b);
                let first = it.next_base_after(s);
                drain(first, || it.next_base())?
            };
            prop_assert_eq!(
                &got,
                &bases(from, hi, pred),
                "base after {} {} filled {}",
                s,
                b,
                filled
            );
        }
    }
    Ok(())
}

fn check_intersections(
    maps: &[Map; 2],
    models: &[Model; 2],
    extra: Bin,
    start: u64,
) -> Result<(), TestCaseError> {
    let mut bins = iteration_bins(&maps[0], extra);
    bins.extend(iteration_bins(&maps[1], extra));
    for b in bins {
        let (lo, hi) = base_range(b);
        for (k0, k1) in [(true, true), (true, false), (false, true), (false, false)] {
            let pred = |x: u64| models[0].get(x) == k0 && models[1].get(x) == k1;
            let new = || IntersectionIter::new(iter(&maps[0], k0, b), iter(&maps[1], k1, b));

            let got: Vec<_> = {
                let mut it = new();
                let first = it.next();
                drain(first, || it.next())?
            };
            prop_assert_eq!(&got, &runs(lo, hi, pred), "inter {} {}/{}", b, k0, k1);

            let got: Vec<_> = {
                let mut it = new().bases();
                let first = it.next();
                drain(first, || it.next())?
            };
            prop_assert_eq!(
                &got,
                &bases(lo, hi, pred),
                "inter bases {} {}/{}",
                b,
                k0,
                k1
            );

            // The detached cursor walks the same bins.
            let mut c = IntersectionCursor::new(cursor(k0, b), cursor(k1, b));
            let first = c.next_base(&maps[0], &maps[1]);
            let got = drain(first, || c.next_base(&maps[0], &maps[1]))?;
            prop_assert_eq!(
                &got,
                &bases(lo, hi, pred),
                "cursor bases {} {}/{}",
                b,
                k0,
                k1
            );

            let s = Bin(2 * start);
            let got: Vec<_> = {
                let mut it = new();
                let first = it.next_after(s);
                drain(first, || it.next())?
            };
            let mut want_bases = vec![];
            for g in &got {
                let (glo, ghi) = base_range(*g);
                want_bases.extend((glo..ghi).map(|x| Bin(2 * x)));
            }
            prop_assert_eq!(
                &want_bases,
                &bases(lo.max(start), hi, pred),
                "inter after {} {} {}/{}",
                s,
                b,
                k0,
                k1
            );
        }
    }
    Ok(())
}

fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        5 => Just(Op::Set),
        3 => Just(Op::Reset),
        1 => Just(Op::FillBefore),
        1 => Just(Op::ResetBefore),
    ]
}

/// Bins inside the universe, mostly small and near the start so runs and
/// bitmaps interleave.
fn arb_op_bin() -> impl Strategy<Value = Bin> {
    prop_oneof![
        6 => (0..=4u64).prop_flat_map(|l| (Just(l), 0..(1024u64 >> l))),
        3 => (5..=9u64).prop_flat_map(|l| (Just(l), 0..(UNIVERSE >> l))),
        1 => (10..=11u64).prop_flat_map(|l| (Just(l), 0..(UNIVERSE >> l))),
    ]
    .prop_map(|(l, o)| Bin::new(l, o))
}

/// A bin and, half the time, a copy of it 32, 64, 128 or 256 bases to the
/// right. Mirrored operations make sibling cells identical, which the map
/// packs into a bitmap repeated across a high layer half; random operations
/// alone almost never reach those code paths.
fn arb_op_bins() -> impl Strategy<Value = Vec<Bin>> {
    (
        arb_op_bin(),
        prop_oneof![Just(0u64), Just(32), Just(64), Just(128), Just(256)],
    )
        .prop_map(|(b, shift)| {
            let mirror = Bin::new(b.layer(), b.layer_offset() + (shift >> b.layer()));
            let mut bins = vec![b];
            if shift != 0
                && b.base_length() <= shift
                && mirror.base_right().base_offset() < UNIVERSE
            {
                bins.push(mirror);
            }
            bins
        })
}

fn arb_iter_bin() -> impl Strategy<Value = Bin> {
    (0..=12u64)
        .prop_flat_map(|l| (Just(l), 0..(QUERY_LIMIT >> l)).prop_map(|(l, o)| Bin::new(l, o)))
}

fn config() -> Config {
    Config {
        cases: 96,
        rng_seed: RngSeed::Fixed(0x5eed_b1a5),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn map_matches_model(
        ops in prop::collection::vec((0..2usize, arb_op(), arb_op_bins()), 0..40),
        extra in arb_iter_bin(),
        start in 0..QUERY_LIMIT,
    ) {
        let mut maps = [Map::new(), Map::new()];
        let mut models = [Model::new(), Model::new()];
        for (i, op, bins) in &ops {
            let (i, op) = (*i, *op);
            for &b in bins {
                match op {
                    Op::Set => maps[i].set(b),
                    Op::Reset => maps[i].reset(b),
                    Op::FillBefore => maps[i].fill_before(b),
                    Op::ResetBefore => maps[i].reset_before(b),
                }
                models[i].apply(op, b);
            }
            let b = bins[0];

            // The cell structure stays a consistent tree with an exact free
            // list (validated by the debug parser), and clones agree.
            let dump = maps[i].debug_string();
            prop_assert!(Map::debug_parse(&dump).is_ok(), "inconsistent after {:?} {}", op, b);
            prop_assert_eq!(maps[i].clone().debug_string(), dump);
        }

        for (m, model) in maps.iter().zip(models.iter_mut()) {
            model.freeze();
            check_point_queries(m, model)?;
            check_iterators(m, model, extra, start)?;
        }
        check_intersections(&maps, &models, extra, start)?;
    }
}
