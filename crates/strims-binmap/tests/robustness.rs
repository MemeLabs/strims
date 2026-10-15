// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! ppspp feeds peer-supplied bins (HAVE, REQUEST, ...) to [`Map`], so every
//! `u64` must be safe: no panics (debug builds check arithmetic overflow),
//! no runaway iteration, and a consistent cell structure afterwards. No
//! model here; `model.rs` checks the answers on a bounded universe.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_binmap::{Bin, IntersectionIter, Map};

fn arb_bin() -> impl Strategy<Value = Bin> {
    prop_oneof![
        3 => any::<u64>().prop_map(Bin),
        2 => (0..=63u64, 0..4u64).prop_map(|(l, o)| Bin::new(l, o)),
        2 => (0..=63u64, any::<u64>()).prop_map(|(l, o)| Bin::new(l, o)),
        2 => (0..=8u64, 0..256u64).prop_map(|(l, o)| Bin::new(l, o)),
        1 => prop::sample::select(vec![
            Bin::ALL,
            Bin::NONE,
            Bin::ALL.left(),
            Bin::ALL.right(),
            Bin::ALL.base_right(),
            Bin::ALL.base_right() - 2,
            Bin(0),
            Bin(63),
        ]),
    ]
}

/// Takes at most `n` items, so a never-ending iterator fails the length
/// assertion below.
fn take(it: impl Iterator<Item = Bin>, n: usize) -> Vec<Bin> {
    it.take(n).collect()
}

fn config() -> Config {
    Config {
        cases: 512,
        rng_seed: RngSeed::Fixed(0xb1a5_f00d),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_bins_are_safe(
        ops in prop::collection::vec((0..2usize, 0..4u8, arb_bin()), 0..24),
        queries in prop::collection::vec(arb_bin(), 1..8),
    ) {
        let mut maps = [Map::new(), Map::new()];
        for &(i, op, b) in &ops {
            match op {
                0 => maps[i].set(b),
                1 => maps[i].reset(b),
                2 => maps[i].fill_before(b),
                _ => maps[i].reset_before(b),
            }
            prop_assert!(Map::debug_parse(&maps[i].debug_string()).is_ok());
        }

        for m in &maps {
            m.find_empty();
            m.find_filled();
            m.find_last_filled();
            for &q in &queries {
                m.filled_at(q);
                m.empty_at(q);
                m.cover(q);
                m.find_empty_after(q);
                m.find_filled_after(q);

                for filled in [false, true] {
                    let it = || if filled { m.iterate_filled_at(q) } else { m.iterate_empty_at(q) };
                    // Bins come out in increasing order, so a walk that
                    // wraps around shows up as a repeat or a decrease.
                    let bins = take(it(), 64);
                    prop_assert!(bins.windows(2).all(|w| w[0].base_right() < w[1].base_left()), "{:?}", bins);
                    let bases = take(it().bases(), 64);
                    prop_assert!(bases.windows(2).all(|w| w[0] < w[1]), "{:?}", bases);

                    // Resuming anywhere, even outside `q`, is safe.
                    for &s in &queries {
                        let mut i = it();
                        let mut v = i.next_after(s);
                        for _ in 0..16 {
                            if v.is_none() {
                                break;
                            }
                            v = i.next();
                        }

                        let mut i = it();
                        let mut v = i.next_base_after(s);
                        for _ in 0..16 {
                            if v.is_none() {
                                break;
                            }
                            v = i.next_base();
                        }
                    }
                }
            }
        }

        fn it(m: &Map, filled: bool, q: Bin) -> strims_binmap::Iter<'_> {
            if filled { m.iterate_filled_at(q) } else { m.iterate_empty_at(q) }
        }
        for &q in &queries {
            for (k0, k1) in [(true, true), (true, false), (false, true), (false, false)] {
                let bins = take(IntersectionIter::new(it(&maps[0], k0, q), it(&maps[1], k1, q)), 64);
                prop_assert!(bins.windows(2).all(|w| w[0].base_right() < w[1].base_left()), "{:?}", bins);
                let bases = take(IntersectionIter::new(it(&maps[0], k0, q), it(&maps[1], k1, q)).bases(), 64);
                prop_assert!(bases.windows(2).all(|w| w[0] < w[1]), "{:?}", bases);
            }
        }
    }
}
