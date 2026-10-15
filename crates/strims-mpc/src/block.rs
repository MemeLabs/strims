// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! 128 and 512 bit blocks and carry-less multiplication.

/// A 128 bit block.
pub type Block = [u8; 16];

/// A 512 bit block.
pub type Block512 = [u8; 64];

/// The all-zero block.
pub(crate) const ZERO_BLOCK: Block = [0; 16];

/// `i` little-endian in the low 8 bytes.
pub(crate) fn block_from_uint(i: u64) -> Block {
    u128::from(i).to_le_bytes()
}

/// `a ^ b`.
pub(crate) fn xor_block(a: &Block, b: &Block) -> Block {
    (u128::from_le_bytes(*a) ^ u128::from_le_bytes(*b)).to_le_bytes()
}

/// `a & b`.
pub(crate) fn and_block(a: &Block, b: &Block) -> Block {
    (u128::from_le_bytes(*a) & u128::from_le_bytes(*b)).to_le_bytes()
}

/// Reads the block at `src[i*16..(i+1)*16]`. Callers index matrices whose
/// shape they computed, so a miss is an internal error: it asserts in debug
/// builds and yields zeros in release builds.
pub(crate) fn block_at(src: &[u8], i: usize) -> Block {
    let mut b = ZERO_BLOCK;
    match i
        .checked_mul(16)
        .and_then(|o| src.get(o..o.checked_add(16)?))
    {
        Some(s) => b.copy_from_slice(s),
        None => debug_assert!(false, "block {i} out of range"),
    }
    b
}

/// `dst ^= src` over the common prefix. Every caller passes equal lengths.
pub(crate) fn xor_in_place(dst: &mut [u8], src: &[u8]) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, s) in dst.iter_mut().zip(src) {
        *d ^= s;
    }
}

/// `dst &= src` over the common prefix. Every caller passes equal lengths.
pub(crate) fn and_in_place(dst: &mut [u8], src: &[u8]) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, s) in dst.iter_mut().zip(src) {
        *d &= s;
    }
}

/// Carry-less product of two 64 bit values. Always runs 64 rounds, so its
/// timing doesn't depend on `a`.
//
// TODO: Karatsuba over a 4-bit table, or the `polyval` crate's software
// multiply, would be much faster; this runs hundreds of times per KOS check.
fn clmul64(a: u64, b: u64) -> u128 {
    let b = u128::from(b);
    let mut r = 0u128;
    for i in 0..64 {
        let mask = 0u128.wrapping_sub(u128::from((a >> i) & 1));
        r ^= (b << i) & mask;
    }
    r
}

/// The 256 bit carry-less product of `a` and `b`, both read
/// as little-endian 128 bit integers, returned as (low, high) blocks.
pub(crate) fn clmul_block(a: &Block, b: &Block) -> [Block; 2] {
    let a = u128::from_le_bytes(*a);
    let b = u128::from_le_bytes(*b);
    let (a0, a1) = (a as u64, (a >> 64) as u64);
    let (b0, b1) = (b as u64, (b >> 64) as u64);

    let zero = clmul64(a0, b0);
    let one = clmul64(a0, b1);
    let two = clmul64(a1, b0);
    let three = clmul64(a1, b1);

    let tmp = one ^ two;
    let x = zero ^ (tmp << 64);
    let y = three ^ (tmp >> 64);
    [x.to_le_bytes(), y.to_le_bytes()]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn clmul_block_known_answers() {
        let cases: [(Block, Block, [Block; 2]); 2] = [
            (
                [
                    0x72, 0x79, 0xca, 0xcd, 0x58, 0xc8, 0x29, 0x1, 0x99, 0x5f, 0xd1, 0xa7, 0x94,
                    0x73, 0xe8, 0xcc,
                ],
                [
                    0xe4, 0xb, 0xd4, 0x1, 0x6, 0x1d, 0x55, 0xf6, 0xdc, 0xa6, 0xb8, 0x74, 0xf, 0x1d,
                    0x90, 0x6c,
                ],
                [
                    [
                        0x8, 0x28, 0xec, 0xf1, 0x6e, 0x94, 0x2, 0x2d, 0x18, 0x76, 0xb0, 0xb9, 0x75,
                        0x35, 0xe2, 0x77,
                    ],
                    [
                        0x4, 0x69, 0x89, 0xe0, 0x58, 0xb9, 0x74, 0xd3, 0x90, 0xc0, 0x5b, 0x10,
                        0x29, 0x43, 0x99, 0x2f,
                    ],
                ],
            ),
            (
                [
                    0x6c, 0x99, 0xc, 0x52, 0x68, 0xa6, 0xb7, 0x79, 0xb8, 0x55, 0x7e, 0x4c, 0xa5,
                    0xee, 0x45, 0x28,
                ],
                [
                    0xfc, 0x8d, 0x47, 0x89, 0x7e, 0x69, 0xa8, 0x55, 0x14, 0xfc, 0xdf, 0xa6, 0x9c,
                    0xbe, 0x2b, 0x28,
                ],
                [
                    [
                        0x90, 0x44, 0xe3, 0xb3, 0x70, 0xe4, 0xd1, 0xd2, 0x1d, 0x5d, 0x67, 0xe1,
                        0xa, 0x9f, 0xbd, 0xed,
                    ],
                    [
                        0x8b, 0xb6, 0x93, 0x16, 0xeb, 0xd0, 0xfc, 0x37, 0xcb, 0x3b, 0xa9, 0xe2,
                        0xf7, 0xb2, 0x4e, 0x4,
                    ],
                ],
            ),
        ];
        for (a, b, want) in cases {
            assert_eq!(clmul_block(&a, &b), want);
        }
    }

    #[test]
    fn block_helpers() {
        assert_eq!(block_from_uint(0x0102), {
            let mut b = ZERO_BLOCK;
            b[0] = 2;
            b[1] = 1;
            b
        });
        let a = block_from_uint(u64::MAX);
        assert_eq!(xor_block(&a, &a), ZERO_BLOCK);
        assert_eq!(and_block(&a, &ZERO_BLOCK), ZERO_BLOCK);
        let src: Vec<u8> = (0..40).collect();
        assert_eq!(block_at(&src, 1)[0], 16);
    }
}
