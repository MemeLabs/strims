// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Unsigned LEB128 varints: 7 bits per byte, least significant group
//! first, the top bit set on every byte but the last.

/// The longest encoding of a `u64`.
pub const MAX_UVARINT_LEN: usize = 10;

/// The length of `v`'s minimal encoding.
pub fn uvarint_len(v: u64) -> usize {
    let bits = 64 - (v | 1).leading_zeros() as usize;
    bits.div_ceil(7)
}

/// Writes `v`'s minimal encoding to the start of `out`. Returns its length.
pub fn put_uvarint(out: &mut [u8; MAX_UVARINT_LEN], v: u64) -> usize {
    let mut v = v;
    let mut n = 0;
    for b in out.iter_mut() {
        n += 1;
        if v < 0x80 {
            *b = v as u8;
            break;
        }
        *b = v as u8 | 0x80;
        v >>= 7;
    }
    n
}

/// Reads a varint from the start of `b`. Returns its value and length, or
/// `None` when `b` ends inside it or it doesn't fit in a `u64`. Any
/// encoding of up to [`MAX_UVARINT_LEN`] bytes is accepted, non-minimal
/// ones included.
pub fn get_uvarint(b: &[u8]) -> Option<(u64, usize)> {
    let mut x = 0u64;
    let mut shift = 0;
    for (i, &byte) in b.iter().enumerate().take(MAX_UVARINT_LEN) {
        if byte < 0x80 {
            if i == MAX_UVARINT_LEN - 1 && byte > 1 {
                return None;
            }
            return Some((x | u64::from(byte) << shift, i + 1));
        }
        x |= u64::from(byte & 0x7f) << shift;
        shift += 7;
    }
    None
}

#[cfg(test)]
mod tests {
    // Test code: an out-of-range index is a test failure.
    #![allow(clippy::indexing_slicing)]

    use super::*;

    fn encode(v: u64) -> Vec<u8> {
        let mut b = [0; MAX_UVARINT_LEN];
        let n = put_uvarint(&mut b, v);
        b.get(..n).unwrap_or_default().to_vec()
    }

    #[test]
    fn values_encode_least_significant_group_first() {
        assert_eq!(encode(0), [0x00]);
        assert_eq!(encode(1), [0x01]);
        assert_eq!(encode(127), [0x7f]);
        assert_eq!(encode(128), [0x80, 0x01]);
        assert_eq!(encode(300), [0xac, 0x02]);
        assert_eq!(encode(16_384), [0x80, 0x80, 0x01]);
        assert_eq!(
            encode(u64::MAX),
            [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]
        );
    }

    #[test]
    fn lengths_match_the_minimal_encoding() {
        for shift in 0..64 {
            for v in [(1u64 << shift) - 1, 1 << shift, (1 << shift) + 1] {
                assert_eq!(uvarint_len(v), encode(v).len(), "{v}");
                assert_eq!(get_uvarint(&encode(v)), Some((v, uvarint_len(v))));
            }
        }
        assert_eq!(uvarint_len(u64::MAX), MAX_UVARINT_LEN);
    }

    #[test]
    fn reading_stops_at_the_first_byte_without_the_top_bit() {
        assert_eq!(get_uvarint(&[0xac, 0x02, 0xff]), Some((300, 2)));
    }

    #[test]
    fn a_non_minimal_encoding_is_accepted() {
        assert_eq!(get_uvarint(&[0x80, 0x00]), Some((0, 2)));
        assert_eq!(get_uvarint(&[0x81, 0x80, 0x00]), Some((1, 3)));
    }

    #[test]
    fn a_truncated_varint_is_rejected() {
        assert_eq!(get_uvarint(&[]), None);
        assert_eq!(get_uvarint(&[0x80]), None);
        assert_eq!(get_uvarint(&[0xff; 9]), None);
    }

    #[test]
    fn a_varint_beyond_64_bits_is_rejected() {
        let mut ten = [0xff; MAX_UVARINT_LEN];
        ten[9] = 0x02;
        assert_eq!(get_uvarint(&ten), None);
        assert_eq!(get_uvarint(&[0xff; 11]), None);
        let mut eleven = [0x80; 11];
        eleven[10] = 0x00;
        assert_eq!(get_uvarint(&eleven), None);
    }
}
