// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

use rand_core::Rng;
use strims_base::rng::det_rng;

fn bin(layer: u64, offset: u64) -> Bin {
    Bin::new(layer, offset)
}

fn map_with(filled: &[u64]) -> Map {
    let mut m = Map::new();
    for &b in filled {
        m.set(Bin(b));
    }
    m
}

#[test]
fn set_get() {
    let mut bs = Map::new();
    let b3 = bin(1, 0);
    let b2 = bin(0, 1);
    bs.set(b3);

    assert!(bs.filled_at(b3));
    assert!(bs.filled_at(b2));
}

#[test]
fn chess() {
    let mut chess16 = Map::new();

    for i in 0..16 {
        if i & 1 == 1 {
            chess16.set(bin(0, i));
        } else {
            chess16.reset(bin(0, i));
        }
    }

    for i in 0..16 {
        if i & 1 == 1 {
            assert!(chess16.filled_at(bin(0, i)));
        } else {
            assert!(chess16.empty_at(bin(0, i)));
        }
    }

    assert!(!chess16.empty_at(bin(4, 0)));
    for i in (0..16).step_by(2) {
        chess16.set(bin(0, i));
    }

    assert!(chess16.filled_at(bin(4, 0)));
    assert!(chess16.filled_at(bin(2, 3)));

    chess16.set(bin(4, 1));
    assert!(chess16.filled_at(bin(5, 0)));
}

#[test]
fn staircase() {
    const TOPLAYR: u64 = 44;
    let mut staircase = Map::new();
    for i in 0..TOPLAYR {
        staircase.set(bin(i, 1));
    }

    assert!(!staircase.filled_at(bin(TOPLAYR, 0)));
    assert!(!staircase.empty_at(bin(TOPLAYR, 0)));

    staircase.set(bin(0, 0));
    assert!(staircase.filled_at(bin(TOPLAYR, 0)));
}

#[test]
fn hole() {
    let mut hole = Map::new();
    hole.set(bin(8, 0));
    hole.reset(bin(6, 1));
    hole.reset(bin(6, 2));

    assert!(hole.filled_at(bin(6, 0)));
    assert!(hole.filled_at(bin(6, 3)));
    assert!(!hole.filled_at(bin(8, 0)));
    assert!(!hole.empty_at(bin(8, 0)));
    assert!(hole.empty_at(bin(6, 1)));
}

#[test]
fn find() {
    let mut hole = Map::new();
    hole.set(bin(4, 0));
    hole.reset(bin(1, 1));
    hole.reset(bin(0, 7));
    assert_eq!(hole.find_empty().base_left(), bin(0, 2));
}

#[test]
fn alloc() {
    let mut b = Map::new();
    b.set(bin(1, 0));
    b.set(bin(1, 1));
    b.reset(bin(1, 0));
    b.reset(bin(1, 1));

    assert_eq!(b.alloc_count, 1);
}

#[test]
fn cover() {
    let mut b = Map::new();
    b.set(bin(2, 0));
    b.set(bin(4, 1));

    assert_eq!(b.cover(bin(0, 30)), bin(4, 1));
    assert_eq!(b.cover(bin(0, 3)), bin(2, 0));
    assert_eq!(b.cover(bin(2, 0)), bin(2, 0));
}

#[test]
fn seq_length() {
    let mut b = Map::new();
    b.set(bin(3, 0));
    b.set(bin(1, 4));
    b.set(bin(0, 10));
    b.set(bin(3, 2));
    assert_eq!(b.find_empty().base_offset(), 11);
}

#[test]
fn empty_filled() {
    let mut b = Map::new();

    assert!(b.empty_at(Bin::ALL));
    b.set(bin(1, 0));
    b.set(bin(0, 2));
    b.set(bin(0, 6));
    b.set(bin(1, 5));
    b.set(bin(0, 9));

    assert!(!b.empty_at(Bin::ALL));
    assert!(b.empty_at(bin(2, 3)));
    assert!(!b.filled_at(bin(2, 3)));
    assert!(b.filled_at(bin(1, 0)));
    assert!(b.filled_at(bin(1, 5)));
    assert!(!b.filled_at(bin(1, 3)));

    b.set(bin(0, 3));
    b.set(bin(0, 7));
    b.set(bin(0, 8));

    assert!(b.filled_at(bin(2, 0)));
    assert!(b.filled_at(bin(2, 2)));
    assert!(!b.filled_at(bin(2, 1)));

    b.set(bin(1, 2));
    assert!(b.filled_at(bin(2, 1)));
}

#[test]
fn find_empty_after() {
    let mut hole = Map::new();

    for s in 0..8 {
        for i in s..8 {
            hole.set(bin(3, 0));
            hole.reset(bin(0, i));
            assert_eq!(hole.find_empty_after(bin(0, s)), bin(0, i));
        }
    }
}

#[test]
fn find_empty_after2() {
    let cases: &[(&[u64], u64, u64)] = &[(&[2, 5], 2, 8), (&[255], 8191, 767)];
    for &(filled, b, expected) in cases {
        let m = map_with(filled);
        assert_eq!(m.find_empty_after(Bin(b)), Bin(expected), "{filled:?} {b}");
    }
}

#[test]
fn find_empty_after_sanity() {
    let mut m = Map::new();
    let mut rng = det_rng(4);
    let n = 100_000;

    for _ in 0..100_000 {
        let v = Bin(rng.next_u64() % n);
        if rng.next_u32() & 1 == 0 {
            m.set(v);
        } else {
            m.reset(v);
        }

        let b = Bin(rng.next_u64() % n);
        let next = m.find_empty_after(b);
        assert!(
            next.is_none() || m.empty_at(next),
            "next empty after {b} bin {next} should be empty"
        );
        assert!(
            next >= b.base_left(),
            "next empty after {b} bin {next} should be >= {b}"
        );
    }
}

#[test]
fn find_filled1() {
    let mut hole = Map::new();
    hole.set(bin(3, 0));
    hole.reset(bin(0, 0));
    assert_eq!(hole.find_filled(), bin(0, 1));
}

#[test]
fn find_filled2() {
    let mut hole = Map::new();
    hole.set(bin(3, 0));
    hole.reset(bin(0, 1));
    assert_eq!(hole.find_filled(), bin(0, 0));
}

#[test]
fn find_filled3() {
    let mut hole = Map::new();
    hole.set(bin(3, 0));
    hole.reset(bin(2, 0));
    assert_eq!(hole.find_filled().base_left(), bin(0, 4));
}

#[test]
fn find_filled_after() {
    let cases: &[(&[u64], u64, u64)] = &[
        (&[1, 4, 9], 6, 8),
        (&[2047], 4014, 4014),
        (&[2447, 2471, 2481, 2511, 2535, 2545], 2484, 2496),
        (&[62218, 62282], 62220, 62282),
        (&[83156], 41012, 83156),
        (&[27922, 33531], 33462, 33528),
    ];
    for &(filled, b, expected) in cases {
        let m = map_with(filled);
        assert_eq!(m.find_filled_after(Bin(b)), Bin(expected), "{filled:?} {b}");
    }
}

#[test]
fn find_filled_after_sanity() {
    let mut m = Map::new();
    let mut rng = det_rng(4);
    let n = 100_000;

    for _ in 0..100_000 {
        let v = Bin(rng.next_u64() % n);
        if rng.next_u32() & 1 == 0 {
            m.set(v);
        } else {
            m.reset(v);
        }

        let b = Bin(rng.next_u64() % n);
        let next = m.find_filled_after(b);
        assert!(
            next.is_none() || m.filled_at(next),
            "next filled after {b} bin {next} should be filled"
        );
        assert!(
            next >= b.base_left(),
            "next filled after {b} bin {next} should be >= {b}"
        );
    }
}

#[test]
fn fill_before() {
    for i in (2..32).step_by(2) {
        let mut m = Map::new();
        m.fill_before(Bin(i));
        for b in (0..i).step_by(2) {
            assert!(m.filled_at(Bin(b)));
        }
        assert!(!m.filled_at(Bin(i)));
    }
}

#[test]
fn reset_before() {
    for i in (2..32).step_by(2) {
        let mut m = Map::new();
        m.set(Bin(63));
        m.reset_before(Bin(i));
        for b in (0..i).step_by(2) {
            assert!(m.empty_at(Bin(b)));
        }
        assert!(m.filled_at(Bin(i)));
    }
}

#[test]
fn edge_cases() {
    let m = Map::new();
    assert_eq!(m.cover(Bin(0)), Bin::ALL);
    assert_eq!(m.cover(Bin(63)), Bin::ALL);
    assert_eq!(m.cover(Bin(31)), Bin::ALL);
    assert_eq!(m.find_empty_after(Bin(127)), Bin::ALL);
    assert_eq!(m.find_empty(), Bin::ALL);
    assert_eq!(m.find_filled(), Bin::NONE);
    assert_eq!(m.find_last_filled(), Bin::NONE);

    let m = map_with(&[63]);
    assert_eq!(m.cover(Bin(0)), Bin::ALL);
    assert_eq!(m.cover(Bin(31)), Bin::ALL);
    assert_eq!(m.cover(Bin(63)), Bin(63));
    assert_eq!(m.cover(Bin(127)), Bin::NONE);

    let m = map_with(&[63, 127]);
    assert_eq!(m.root_bin(), Bin(127));
    assert_eq!(m.cover(Bin(0)), Bin::ALL);
    assert_eq!(m.cover(Bin(63)), Bin(63));
    assert_eq!(m.cover(Bin(127)), Bin(127));

    let m = map_with(&[4]);
    assert_eq!(m.find_filled_after(Bin(63)), Bin(4));
    assert_eq!(m.find_empty_after(Bin(127)), Bin(1));
    assert_eq!(m.find_empty_after(Bin(1001)), Bin(1001));

    let mut m = Map::new();
    m.set(Bin::ALL);
    assert_eq!(m.root_bin(), Bin::ALL);
    assert!(m.is_filled());
    assert_eq!(m.find_empty(), Bin::NONE);
    assert_eq!(m.cover(Bin(5)), Bin::ALL);
    assert!(m.filled_at(Bin::ALL));
    assert_eq!(m.find_last_filled(), Bin(0xffff_ffff_ffff_fffe));

    let mut m = Map::new();
    m.set(Bin::NONE);
    m.reset(Bin::NONE);
    assert_eq!(m.root_bin(), Bin(63));
    assert!(m.is_empty());
}

/// The cover of a target outside the root contains the target and is
/// disjoint from the root.
#[test]
fn cover_outside_root_contains_target() {
    let m = map_with(&[0]);
    assert_eq!(m.root_bin(), Bin(63));
    for (target, expected) in [(300, 383), (1000, 767), (191, 191), (128, 191)] {
        let c = m.cover(Bin(target));
        assert_eq!(c, Bin(expected), "cover({target})");
        assert!(c.contains(Bin(target)));
        assert!(m.empty_at(c));
        assert!(c.parent().contains(m.root_bin()));
    }

    // Everything is empty in an empty map.
    assert_eq!(Map::new().cover(Bin(300)), Bin::ALL);
}

/// A target that strictly contains the root finds the root's filled bins.
#[test]
fn find_filled_after_target_containing_root() {
    let m = map_with(&[4]);
    assert_eq!(m.find_filled_after(Bin(127)), Bin(4));
    assert_eq!(m.find_filled_after(Bin::ALL), Bin(4));
    assert_eq!(Map::new().find_filled_after(Bin(127)), Bin::NONE);
}

#[test]
fn clone_from_reuses_cells() {
    let src = map_with(&[0, 200, 4000]);
    let mut dst = map_with(&[1, 2, 3, 5, 9, 70, 300, 9000, 20000, 100000]);
    let cap = dst.cells.capacity();
    assert!(cap >= src.cells.len());
    dst.clone_from(&src);
    assert_eq!(dst.cells.capacity(), cap);
    assert_eq!(dst.debug_string(), src.debug_string());
    assert_eq!(src.clone().debug_string(), src.debug_string());
}

#[test]
fn cells_grow_by_half_without_slack() {
    let mut m = Map::new();
    assert_eq!(m.cells.len(), 16);
    assert_eq!(m.cells.capacity(), 16);
    for i in 0..64 {
        m.set(Bin(i * 256));
    }
    // 16 -> 24 -> 36 -> ... with no slack beyond the requested length.
    assert_eq!(m.cells.capacity(), m.cells.len());
}

/// With an ALL root, the result never precedes the target, and a root full
/// after the target gives NONE.
#[test]
fn find_empty_after_all_root() {
    let mut m = Map::new();
    m.set(Bin::ALL);
    m.reset(Bin(805306367)); // bases [2^28, 2^29)
    assert_eq!(m.root_bin(), Bin::ALL);
    assert_eq!(m.find_empty_after(Bin(0)), Bin(1 << 29));
    assert_eq!(m.find_empty_after(Bin(0x3fff_fffe)), Bin(0x3fff_fffe));
    assert_eq!(m.find_empty_after(Bin(0x4000_0000)), Bin::NONE);
    assert_eq!(m.find_filled_after(Bin(0x4000_0000)), Bin(0x4000_0000));
}
