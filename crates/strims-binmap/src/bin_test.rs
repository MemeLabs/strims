// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn init_get() {
    assert_eq!(Bin::new(1, 0), Bin(0x1));
    assert_eq!(Bin::new(2, 1), Bin(0xB));
    assert_eq!(Bin::new(2, 1).layer(), 2);
    assert_eq!(Bin::new(34, 2345).layer(), 34);
    assert_eq!(Bin::new(34, 2345).layer_bits(), Bin(0x7_ffff_ffff));
    assert_eq!(Bin::new(2, 1).layer_offset(), 1);
    assert_eq!(Bin::new(34, 2345).layer_offset(), 2345);
    assert_eq!(Bin::new(0, 123).layer_bits(), Bin((1 << 1) - 1));
    assert_eq!(Bin::new(16, 123).layer_bits(), Bin((1 << 17) - 1));
}

#[test]
fn navigation() {
    let mid = Bin::new(4, 18);
    assert_eq!(mid.parent(), Bin::new(5, 9));
    assert_eq!(mid.left(), Bin::new(3, 36));
    assert_eq!(mid.right(), Bin::new(3, 37));
    assert_eq!(Bin::new(4, 19).parent(), Bin::new(5, 9));
    let up32 = Bin::new(30, 1);
    assert_eq!(up32.parent(), Bin::new(31, 0));
}

#[test]
fn overflows() {
    assert!(!Bin::NONE.contains(Bin::new(0, 1)));
    assert!(Bin::ALL.contains(Bin::new(0, 1)));
    assert_eq!(Bin::NONE.base_length(), 0);
}

#[test]
fn advanced() {
    assert_eq!(Bin::new(2, 3).base_length(), 4);
    assert!(!Bin::new(1, 1234).is_base());
    assert!(Bin::new(0, 12345).is_base());
    assert_eq!(Bin::new(1, 1).base_left(), Bin::new(0, 2));
    assert_eq!(Bin::new(1, 1).base_right(), Bin::new(0, 3));
}

#[test]
fn bin_to_string() {
    assert_eq!(Bin::ALL.to_string(), "ALL");
    assert_eq!(Bin::NONE.to_string(), "NONE");
    assert_eq!(Bin::new(0, 1).to_string(), "2");
}

#[test]
fn layer_left_right() {
    for l in 0..10 {
        for o in 0..10 {
            let left = Bin::new(l, o);
            let right = Bin::new(l, o + 1);
            assert_eq!(left.layer_right(), right);
            assert_eq!(right.layer_left(), left);
        }
        assert_eq!(Bin::new(l, 0).layer_left(), Bin::NONE);
    }
}

#[test]
fn layer_shift() {
    for i in 0..10 {
        for j in 4..8 {
            assert_eq!(Bin::new(j, i).layer_shift(j - 1), Bin::new(j - 1, i * 2));
            assert_eq!(Bin::new(j, i).layer_shift(j - 2), Bin::new(j - 2, i * 4));
            assert_eq!(Bin::new(j, i).layer_shift(j - 3), Bin::new(j - 3, i * 8));
            assert_eq!(Bin::new(j, i).layer_shift(j - 4), Bin::new(j - 4, i * 16));

            assert_eq!(Bin::new(j - 1, i * 2).layer_shift(j), Bin::new(j, i));
            assert_eq!(Bin::new(j - 2, i * 4).layer_shift(j), Bin::new(j, i));
            assert_eq!(Bin::new(j - 3, i * 8).layer_shift(j), Bin::new(j, i));
            assert_eq!(Bin::new(j - 4, i * 16).layer_shift(j), Bin::new(j, i));
        }
    }
}

#[test]
fn layer_shift2() {
    let cases = [(Bin(512024), 8), (Bin(111), 6)];
    for (bin, layer) in cases {
        let mut expected = bin;
        while expected.layer() > layer {
            expected = expected.left();
        }
        while expected.layer() < layer {
            expected = expected.parent();
        }
        assert_eq!(bin.layer_shift(layer), expected, "bin {bin} layer {layer}");
        assert_eq!(
            bin.layer_shift(layer).layer(),
            layer,
            "bin {bin} layer {layer}"
        );
    }
}

#[test]
fn sentinel_arithmetic() {
    assert_eq!(Bin::NONE.layer_offset(), 0);
    assert_eq!(Bin::NONE.layer(), 64);
    assert_eq!(Bin::NONE.parent(), Bin::NONE);
    assert_eq!(Bin::NONE.left(), Bin::NONE);
    assert_eq!(Bin::NONE.right(), Bin::NONE);
    assert_eq!(Bin::NONE.sibling(), Bin::NONE);
    assert_eq!(Bin::NONE.base_left(), Bin::NONE);
    assert_eq!(Bin::NONE.base_right(), Bin::NONE);
    assert_eq!(Bin::NONE.layer_left(), Bin::NONE);
    assert_eq!(Bin::NONE.layer_right(), Bin::NONE);

    assert_eq!(Bin::ALL.layer_right(), Bin::ALL);
    assert_eq!(Bin::ALL.parent(), Bin::NONE);
    assert_eq!(Bin::ALL.left(), Bin(4611686018427387903));
    assert_eq!(Bin::ALL.right(), Bin(13835058055282163711));
    assert_eq!(Bin::ALL.base_length(), 9223372036854775808);
    assert_eq!(Bin::ALL.base_offset(), 0);
    assert_eq!(Bin::ALL.sibling(), Bin::ALL);
    assert_eq!(Bin::ALL.base_right(), Bin(0xffff_ffff_ffff_fffe));

    assert_eq!(Bin(5).layer_shift(64), Bin::NONE);
    assert_eq!(Bin::new(64, 1), Bin::NONE);
    assert_eq!(Bin::new(63, 1), Bin::ALL);
    assert_eq!(Bin::new(63, 0), Bin::ALL);
    assert_eq!(Bin::new(1, 1 << 63), Bin(1));

    // ppspp relies on `find_last_filled() + 2` turning NONE into bin 1.
    assert_eq!(Bin::NONE + 2, Bin(1));
    assert_eq!(Bin(0) - 2, Bin(0xffff_ffff_ffff_fffe));
}
