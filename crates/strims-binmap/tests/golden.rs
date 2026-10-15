// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Replays the differential trace that `golden/main.go` records from the Go
//! implementation, and checks every query result, iterator walk and cell
//! layout (as a hash of the debug dump) against it. Regenerate from the repo
//! root with
//! `go run -tags golden ./crates/strims-binmap/golden > crates/strims-binmap/golden/trace.txt`.
//!
//! The trace never sets ALL; the unit tests cover ALL roots. binmap has no
//! wire format, so the trace pins behaviour, layout and allocation only.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::indexing_slicing, clippy::panic)]

use strims_binmap::{Bin, IntersectionIter, Iter, Map};

const TRACE: &str = include_str!("../golden/trace.txt");
const ITER_CAP: usize = 20;

fn fnv1a64(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn num(s: &str) -> u64 {
    s.parse()
        .unwrap_or_else(|_| panic!("bad number {s:?} in trace"))
}

fn list(s: &str) -> Vec<u64> {
    if s == "-" {
        return vec![];
    }
    s.split(',').map(num).collect()
}

fn iter<'a>(m: &'a Map, kind: &str, b: Bin) -> Iter<'a> {
    match kind {
        "filled" => m.iterate_filled_at(b),
        "empty" => m.iterate_empty_at(b),
        _ => panic!("bad kind {kind}"),
    }
}

/// Up to `ITER_CAP` values from `first` and then `next`, as the generator
/// records them.
fn walk(mut first: impl FnMut() -> Option<Bin>, mut next: impl FnMut() -> Option<Bin>) -> Vec<u64> {
    let mut bins = vec![];
    let mut v = first();
    while let Some(b) = v {
        bins.push(b.0);
        if bins.len() == ITER_CAP {
            break;
        }
        v = next();
    }
    bins
}

/// Inputs whose answers are specified on [`Map::cover`] and
/// [`Map::find_filled_after`] and differ from the recorded ones.
fn not_compared(m: &Map, query: &str, q: Bin) -> bool {
    let root = m.root_bin();
    match query {
        "cover" => !root.contains(q) && !q.contains(root),
        "findfilledafter" => !root.contains(q) && q.contains(root),
        _ => false,
    }
}

#[test]
fn matches_go_trace() {
    let mut maps = [Map::new(), Map::new()];
    let mut compared = 0;
    let mut skipped = 0;

    for (n, line) in TRACE.lines().enumerate() {
        let n = n + 1;
        let (lhs, want) = match line.split_once(" = ") {
            Some((l, r)) => (l, Some(r)),
            None => (line, None),
        };
        let t: Vec<&str> = lhs.split(' ').collect();
        let map_id = |i: usize| -> usize { num(t[i]) as usize };

        match (t[0], want) {
            ("scenario", None) => maps = [Map::new(), Map::new()],
            (op @ ("set" | "reset" | "fillbefore" | "resetbefore"), None) => {
                let m = &mut maps[map_id(1)];
                let b = Bin(num(t[2]));
                match op {
                    "set" => m.set(b),
                    "reset" => m.reset(b),
                    "fillbefore" => m.fill_before(b),
                    _ => m.reset_before(b),
                }
            }
            (query, Some(want)) if t.len() == 3 && query != "iter" => {
                let m = &maps[map_id(1)];
                let q = Bin(num(t[2]));
                if not_compared(m, query, q) {
                    skipped += 1;
                    continue;
                }
                let got = match query {
                    "filledat" => u64::from(m.filled_at(q)),
                    "emptyat" => u64::from(m.empty_at(q)),
                    "cover" => m.cover(q).0,
                    "findemptyafter" => m.find_empty_after(q).0,
                    "findfilledafter" => m.find_filled_after(q).0,
                    _ => panic!("line {n}: unknown query {query}"),
                };
                assert_eq!(got, num(want), "line {n}: {line}");
                compared += 1;
            }
            (query, Some(want)) if t.len() == 2 => {
                let m = &maps[map_id(1)];
                let got = match query {
                    "findempty" => m.find_empty().0,
                    "findfilled" => m.find_filled().0,
                    "findlastfilled" => m.find_last_filled().0,
                    "root" => m.root_bin().0,
                    "empty" => u64::from(m.is_empty()),
                    "filled" => u64::from(m.is_filled()),
                    "dumphash" => fnv1a64(&m.debug_string()),
                    _ => panic!("line {n}: unknown query {query}"),
                };
                assert_eq!(got, num(want), "line {n}: {line}");
                compared += 1;
            }
            ("iter", Some(want)) => {
                let m = &maps[map_id(1)];
                let (kind, b, mode, start) = (t[2], Bin(num(t[3])), t[4], Bin(num(t[5])));
                let it = std::cell::RefCell::new(iter(m, kind, b));
                let got = match mode {
                    "next" => walk(|| it.borrow_mut().next(), || it.borrow_mut().next()),
                    "base" => walk(
                        || it.borrow_mut().next_base(),
                        || it.borrow_mut().next_base(),
                    ),
                    "after" => walk(
                        || it.borrow_mut().next_after(start),
                        || it.borrow_mut().next(),
                    ),
                    "baseafter" => walk(
                        || it.borrow_mut().next_base_after(start),
                        || it.borrow_mut().next_base(),
                    ),
                    _ => panic!("line {n}: unknown mode {mode}"),
                };
                assert_eq!(got, list(want), "line {n}: {line}");
                compared += 1;
            }
            ("inter", Some(want)) => {
                let (k0, k1, b, mode, start) = (t[1], t[2], Bin(num(t[3])), t[4], Bin(num(t[5])));
                let it = std::cell::RefCell::new(IntersectionIter::new(
                    iter(&maps[0], k0, b),
                    iter(&maps[1], k1, b),
                ));
                let got = match mode {
                    "next" => walk(|| it.borrow_mut().next(), || it.borrow_mut().next()),
                    "base" => walk(
                        || it.borrow_mut().next_base(),
                        || it.borrow_mut().next_base(),
                    ),
                    "after" => walk(
                        || it.borrow_mut().next_after(start),
                        || it.borrow_mut().next(),
                    ),
                    _ => panic!("line {n}: unknown mode {mode}"),
                };
                assert_eq!(got, list(want), "line {n}: {line}");
                compared += 1;
            }
            _ => panic!("line {n}: unparsed {line:?}"),
        }
    }

    assert!(compared > 2500, "compared only {compared} results");
    assert!(skipped * 20 < compared, "skipped {skipped} of {compared}");
}
