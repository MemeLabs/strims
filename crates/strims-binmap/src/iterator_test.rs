// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn map_with(filled: &[u64]) -> Map {
    let mut m = Map::new();
    for &b in filled {
        m.set(Bin(b));
    }
    m
}

fn bins(v: &[u64]) -> Vec<Bin> {
    v.iter().copied().map(Bin).collect()
}

#[test]
fn iterate_empty_at_base() {
    let cases: &[(&[u64], &str)] = &[
        (&[2, 5, 12, 22, 24], "basic"),
        (&[], "none filled"),
        (&[15], "all filled"),
        (&[7], "first half filled"),
        (&[23], "second half filled"),
    ];

    for &(filled, label) in cases {
        for b in [7, 15, 23].map(Bin) {
            let k = map_with(filled);

            let mut expected_filled = vec![];
            let mut expected_empty = vec![];
            let mut i = b.base_left();
            while i <= b.base_right() {
                if k.filled_at(i) {
                    expected_filled.push(i);
                } else {
                    expected_empty.push(i);
                }
                i += 2;
            }

            let actual_filled: Vec<_> = k.iterate_filled_at(b).bases().collect();
            let actual_empty: Vec<_> = k.iterate_empty_at(b).bases().collect();

            assert_eq!(actual_filled, expected_filled, "{label}, bin {b}: filled");
            assert_eq!(actual_empty, expected_empty, "{label}, bin {b}: empty");
        }
    }
}

#[test]
fn iterate() {
    let cases: &[(&[u64], &[u64], &str)] = &[
        (&[2, 5, 12, 22, 24], &[0, 9, 14, 17, 20, 26, 29], "basic"),
        (&[], &[15], "none filled"),
        (&[15], &[], "all filled"),
        (&[7], &[23], "first half filled"),
        (&[23], &[7], "second half filled"),
    ];

    for &(filled, empty, label) in cases {
        let k = map_with(filled);

        let actual_filled: Vec<_> = k.iterate_filled_at(Bin(15)).collect();
        let actual_empty: Vec<_> = k.iterate_empty_at(Bin(15)).collect();

        assert_eq!(actual_filled, bins(filled), "{label}: filled");
        assert_eq!(actual_empty, bins(empty), "{label}: empty");
    }
}

#[test]
fn iterate_after() {
    let start = Bin(12);
    let k = map_with(&[2, 5, 12, 22, 24]);

    let mut actual_filled = vec![];
    let mut it = k.iterate_filled_at(Bin(15));
    let mut next = it.next_after(start);
    while let Some(b) = next {
        actual_filled.push(b);
        next = it.next();
    }

    let mut actual_empty = vec![];
    let mut it = k.iterate_empty_at(Bin(15));
    let mut next = it.next_after(start);
    while let Some(b) = next {
        actual_empty.push(b);
        next = it.next();
    }

    assert_eq!(actual_filled, bins(&[12, 22, 24]), "filled");
    assert_eq!(actual_empty, bins(&[14, 17, 20, 26, 29]), "empty");
}

#[test]
fn iterate_intersection_base() {
    let cases: &[(&[u64], &[u64], &str)] = &[
        (&[5, 9, 19, 24], &[7, 17, 29], "basic"),
        (&[], &[], "none filled"),
        (&[15], &[15], "all filled"),
        (&[7], &[23], "no intersection"),
        (&[7, 21, 25], &[5, 9, 23], "middles intersect"),
        (&[0, 30], &[0, 30], "edges intersect"),
        (&[7], &[7], "first half filled"),
        (&[23], &[23], "second half filled"),
    ];

    for &(filled0, filled1, label) in cases {
        for b in [7, 15, 23].map(Bin) {
            let k0 = map_with(filled0);
            let k1 = map_with(filled1);

            let mut expected_filled = vec![];
            let mut expected_empty = vec![];
            let mut i = b.base_left();
            while i <= b.base_right() {
                if k0.filled_at(i) && k1.filled_at(i) {
                    expected_filled.push(i);
                } else if k0.empty_at(i) && k1.empty_at(i) {
                    expected_empty.push(i);
                }
                i += 2;
            }

            let actual_filled: Vec<_> =
                IntersectionIter::new(k0.iterate_filled_at(b), k1.iterate_filled_at(b))
                    .bases()
                    .collect();
            let actual_empty: Vec<_> =
                IntersectionIter::new(k0.iterate_empty_at(b), k1.iterate_empty_at(b))
                    .bases()
                    .collect();

            assert_eq!(actual_filled, expected_filled, "{label}, bin {b}: filled");
            assert_eq!(actual_empty, expected_empty, "{label}, bin {b}: empty");
        }
    }
}

#[test]
fn iterate_intersection() {
    struct Case {
        filled0: &'static [u64],
        filled1: &'static [u64],
        root: u64,
        filled: &'static [u64],
        difference0: &'static [u64],
        difference1: &'static [u64],
        label: &'static str,
    }
    let cases = [
        Case {
            filled0: &[5, 9, 19, 24],
            filled1: &[7, 17, 29],
            root: 15,
            filled: &[5, 9, 17],
            difference0: &[1, 13, 29],
            difference1: &[21, 24],
            label: "basic",
        },
        Case {
            filled0: &[],
            filled1: &[],
            root: 15,
            filled: &[],
            difference0: &[],
            difference1: &[],
            label: "none filled",
        },
        Case {
            filled0: &[15],
            filled1: &[15],
            root: 15,
            filled: &[15],
            difference0: &[],
            difference1: &[],
            label: "all filled",
        },
        Case {
            filled0: &[7],
            filled1: &[23],
            root: 15,
            filled: &[],
            difference0: &[23],
            difference1: &[7],
            label: "no intersection",
        },
        Case {
            filled0: &[7, 21, 25],
            filled1: &[5, 9, 23],
            root: 15,
            filled: &[5, 9, 21, 25],
            difference0: &[17, 29],
            difference1: &[1, 13],
            label: "middles intersect",
        },
        Case {
            filled0: &[0, 30],
            filled1: &[0, 30],
            root: 15,
            filled: &[0, 30],
            difference0: &[],
            difference1: &[],
            label: "edges intersect",
        },
        Case {
            filled0: &[7],
            filled1: &[7],
            root: 15,
            filled: &[7],
            difference0: &[],
            difference1: &[],
            label: "first half filled",
        },
        Case {
            filled0: &[23],
            filled1: &[23],
            root: 15,
            filled: &[23],
            difference0: &[],
            difference1: &[],
            label: "second half filled",
        },
        Case {
            filled0: &[255, 614],
            filled1: &[23],
            root: 511,
            filled: &[23],
            difference0: &[],
            difference1: &[7, 47, 95, 191, 383, 614],
            label: "wide",
        },
    ];

    for c in cases {
        let k0 = map_with(c.filled0);
        let k1 = map_with(c.filled1);
        let root = Bin(c.root);

        let actual_filled: Vec<_> =
            IntersectionIter::new(k0.iterate_filled_at(root), k1.iterate_filled_at(root)).collect();
        let actual_difference0: Vec<_> =
            IntersectionIter::new(k0.iterate_empty_at(root), k1.iterate_filled_at(root)).collect();
        let actual_difference1: Vec<_> =
            IntersectionIter::new(k0.iterate_filled_at(root), k1.iterate_empty_at(root)).collect();

        assert_eq!(actual_filled, bins(c.filled), "{}: filled", c.label);
        assert_eq!(
            actual_difference0,
            bins(c.difference0),
            "{}: difference 1-0",
            c.label
        );
        assert_eq!(
            actual_difference1,
            bins(c.difference1),
            "{}: difference 0-1",
            c.label
        );
    }
}

#[test]
fn iterate_intersection_after() {
    let root = Bin(15);
    let k0 = map_with(&[11, 28]);
    let k1 = map_with(&[15]);

    let mut actual = vec![];
    let mut it = IntersectionIter::new(k0.iterate_filled_at(root), k1.iterate_filled_at(root));
    let mut next = it.next_after(Bin(12));
    while let Some(b) = next {
        actual.push(b);
        next = it.next();
    }

    assert_eq!(actual, bins(&[13, 28]));
}

#[test]
fn iterate_base_after() {
    let k = map_with(&[11, 28]);

    let cases: &[(u64, &[u64])] = &[(0, &[8, 10, 12, 14, 28]), (12, &[12, 14, 28]), (30, &[])];

    for &(first, expected) in cases {
        let mut actual = vec![];
        let mut it = k.iterate_filled();
        let mut next = it.next_base_after(Bin(first));
        while let Some(b) = next {
            actual.push(b);
            next = it.next_base();
        }
        assert_eq!(actual, bins(expected), "first {first}");
    }
}

#[test]
fn iterator_to_slice() {
    let filled = [5, 11, 28, 129];
    let k = map_with(&filled);
    assert_eq!(k.iterate_filled().collect::<Vec<_>>(), bins(&filled));
}

/// Detached cursors see changes made to the map between steps (ppspp resets
/// bins of a map it is intersecting over).
#[test]
fn cursor_observes_mutation() {
    let mut have = Map::new();
    have.set(Bin(2));
    let mut request = map_with(&[7, 17]);
    let root = request.root_bin();

    let mut it = IntersectionCursor::new(Cursor::empty_at(root), Cursor::filled_at(root));
    let mut visited = vec![];
    while let Some(b) = it.next_base(&have, &request) {
        visited.push(b);
        request.reset(b);
    }

    assert_eq!(visited, bins(&[0, 4, 6, 8, 10, 12, 14, 16, 18]));
    assert!(request.filled_at(Bin(2)));
    assert_eq!(request.iterate_filled().collect::<Vec<_>>(), bins(&[2]));
}

#[test]
fn iter_resumes_from_cursor() {
    let k = map_with(&[5, 11, 28, 129]);
    let mut it = k.iterate_filled();
    assert_eq!(it.next(), Some(Bin(5)));
    let cursor = it.cursor();
    let rest: Vec<_> = Iter::from_cursor(&k, cursor).collect();
    assert_eq!(rest, bins(&[11, 28, 129]));
}

/// Walks over an ALL root visit each run once and terminate. A peer's HAVE
/// for ALL makes such a root.
#[test]
fn iterate_all_root() {
    let mut m = Map::new();
    m.set(Bin::ALL);
    m.reset(Bin(805306367)); // bases [2^28, 2^29)

    let mut expected = vec![Bin::new(28, 0)];
    expected.extend((29..63).map(|l| Bin::new(l, 1)));
    assert_eq!(m.iterate_filled_at(Bin::ALL).collect::<Vec<_>>(), expected);
    assert_eq!(
        IntersectionIter::new(m.iterate_filled_at(Bin::ALL), m.iterate_filled_at(Bin::ALL))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        m.iterate_empty_at(Bin::ALL).collect::<Vec<_>>(),
        vec![Bin(805306367)]
    );

    // Base iteration stops at the last base bin.
    let last = Bin::ALL.base_right();
    let mut it = m.iterate_filled_at(Bin::ALL);
    assert_eq!(it.next_base_after(last - 2), Some(last - 2));
    assert_eq!(it.next_base(), Some(last));
    assert_eq!(it.next_base(), None);
    assert_eq!(it.next_base(), None);
}

#[test]
fn iterate_none_is_empty() {
    let k = map_with(&[5, 11]);
    assert_eq!(k.iterate_empty_at(Bin::NONE).next(), None);
    assert_eq!(k.iterate_empty_at(Bin::NONE).bases().next(), None);
    assert_eq!(k.iterate_filled_at(Bin::NONE).next_after(Bin(0)), None);
    assert_eq!(k.iterate_filled_at(Bin::NONE).next_base_after(Bin(0)), None);
}

/// A walk whose gap is the last base bin ends there.
#[test]
fn iterate_at_end_of_bin_space() {
    let last = Bin::ALL.base_right();
    let mut k = Map::new();
    k.set(last);
    assert_eq!(k.root_bin(), Bin::ALL);

    assert_eq!(
        k.iterate_empty_at(last - 2).collect::<Vec<_>>(),
        vec![last - 2]
    );
    assert_eq!(k.iterate_filled_at(last - 2).collect::<Vec<_>>(), vec![]);
    assert_eq!(
        k.iterate_filled_at(Bin::ALL.right()).collect::<Vec<_>>(),
        vec![last]
    );
    assert_eq!(
        k.iterate_filled_at(Bin::ALL).bases().collect::<Vec<_>>(),
        vec![last]
    );

    // Finished walks stay finished.
    let mut it = k.iterate_empty_at(last - 2);
    assert_eq!(it.next(), Some(last - 2));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}
