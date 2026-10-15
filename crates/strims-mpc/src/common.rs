// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Bit packing, the bit-matrix transpose, matrix padding and the Ristretto
//! helpers.

use aes::Aes256;
use aes::cipher::{BlockCipherEncrypt, KeyInit};
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;

use crate::Error;
use crate::aes_rng::AesRng;
use crate::block::{Block, block_from_uint};

/// Unpacks bits, most significant first.
pub(crate) fn bools_from_bytes(b: &[u8]) -> Vec<bool> {
    b.iter()
        .flat_map(|&v| (0..8).rev().map(move |i| v >> i & 1 != 0))
        .collect()
}

/// The inverse of [`bools_from_bytes`], zero padded to a whole byte.
pub(crate) fn bytes_from_bools(b: &[bool]) -> Vec<u8> {
    b.chunks(8)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u8, |acc, (i, &v)| acc | (u8::from(v) << (7 - i)))
        })
        .collect()
}

const TRANSPOSE_MASKS: [u8; 3] = [0x55, 0x33, 0x0f];

/// Whether [`transpose_matrix`] handles an `nrows` x `ncols * 8` bit matrix:
/// the shorter side is a power of two of at least 8 bits and the longer side
/// is a multiple of it. Every shape the protocols build satisfies this.
fn transpose_supported(len: usize, nrows: usize, ncols: usize) -> bool {
    let Some(bits) = ncols.checked_mul(8) else {
        return false;
    };
    let short = nrows.min(bits);
    let long = nrows.max(bits);
    nrows.checked_mul(ncols) == Some(len)
        && short >= 8
        && short.is_power_of_two()
        && long % short == 0
}

/// Transposes a row-major bit matrix of `nrows` rows and
/// `ncols` bytes per row in place, with Eklundh's algorithm. The result has
/// `ncols * 8` rows of `nrows / 8` bytes. Bits are numbered most
/// significant first within each byte.
///
/// Shapes outside [`transpose_supported`] are [`Error::UnsupportedMatrix`].
//
// invariant: once `transpose_supported` holds, `nrows` and `ncols * 8` are
// multiples of every power of two up to the shorter side, which bounds
// every index below. `transpose_matches_naive` covers each shape family.
//
// TODO: byte-wise Eklundh. A 64-bit-word or 8x8-in-a-`u64` version would be
// several times faster for the 128xN and 512xN matrices.
#[allow(clippy::indexing_slicing)]
pub(crate) fn transpose_matrix(m: &mut [u8], nrows: usize, ncols: usize) -> Result<(), Error> {
    if m.is_empty() && (nrows == 0 || ncols == 0) {
        return Ok(());
    }
    if !transpose_supported(m.len(), nrows, ncols) {
        return Err(Error::UnsupportedMatrix { nrows, ncols });
    }

    // Transpose every 8x8 bit tile in place.
    let mut width = 1;
    for m0 in TRANSPOSE_MASKS {
        let m1 = !m0;
        for r in (0..nrows).step_by(width * 2) {
            for c in 0..ncols {
                for j in 0..width {
                    let mi0 = c + (r + j) * ncols;
                    let mi1 = c + (r + width + j) * ncols;
                    let t = m[mi0];
                    m[mi0] = (m[mi0] & m1) | ((m[mi1] & m1) >> width);
                    m[mi1] = (m[mi1] & m0) | ((t & m0) << width);
                }
            }
        }
        width *= 2;
    }

    // Swap byte blocks until every square tile is transposed.
    while width < nrows && width < ncols * 8 {
        for r in (0..nrows).step_by(width * 2) {
            for c in (width / 8..ncols).step_by(width / 4) {
                for j in 0..width {
                    for k in 0..width / 8 {
                        let mi0 = c + k + (r + j) * ncols;
                        let mi1 = c + k + (r + width + j) * ncols - width / 8;
                        m.swap(mi0, mi1);
                    }
                }
            }
        }
        width *= 2;
    }

    if nrows == ncols * 8 {
        return Ok(());
    }

    // Lay the transposed square tiles out as the rectangular result.
    let t = m.to_vec();
    m.fill(0);
    if nrows < ncols * 8 {
        for r in 0..nrows {
            for c in (0..ncols).step_by(nrows / 8) {
                let ti = c + r * ncols;
                let mi = r * nrows / 8 + c * nrows;
                m[mi..mi + nrows / 8].copy_from_slice(&t[ti..ti + nrows / 8]);
            }
        }
    } else {
        for i in 0..nrows / (ncols * 8) {
            for j in 0..ncols * 8 {
                let ti = i * ncols * ncols * 8 + j * ncols;
                let mi = i * ncols + j * nrows / 8;
                m[mi..mi + ncols].copy_from_slice(&t[ti..ti + ncols]);
            }
        }
    }
    Ok(())
}

/// The number of bit columns to allocate for `ncols` values in a matrix with
/// `nrows` rows, so that [`transpose_matrix`] supports it.
pub(crate) fn pad_matrix(ncols: usize, nrows: usize) -> usize {
    if ncols < 8 {
        return 8;
    }
    let c = ncols.checked_next_power_of_two().unwrap_or(usize::MAX);
    if c < nrows {
        return c;
    }
    match ncols % nrows {
        0 => ncols,
        rem => ncols + (nrows - rem),
    }
}

/// The single-input Ristretto Elligator map applied to 32 bytes of `rng`.
pub(crate) fn point_from_rng(rng: &mut AesRng) -> RistrettoPoint {
    let mut buf = [0u8; 32];
    rng.fill_bytes(&mut buf);
    point_from_elligator(&buf)
}

/// The single-input Ristretto Elligator map.
///
/// curve25519-dalek exposes only the two-input map, which adds the maps of
/// both halves. The map sends zero to the identity, so a zero second half
/// yields the single-input map.
pub(crate) fn point_from_elligator(r: &[u8; 32]) -> RistrettoPoint {
    let mut wide = [0u8; 64];
    let (lo, _) = wide.split_at_mut(32);
    lo.copy_from_slice(r);
    RistrettoPoint::from_uniform_bytes(&wide)
}

/// 64 bytes of `rng`, little-endian, reduced mod the group order.
pub(crate) fn scalar_from_rng(rng: &mut AesRng) -> Scalar {
    let mut buf = [0u8; 64];
    rng.fill_bytes(&mut buf);
    Scalar::from_bytes_mod_order_wide(&buf)
}

/// The canonical 32-byte encoding.
pub(crate) fn point_bytes(p: &RistrettoPoint) -> [u8; 32] {
    p.compress().to_bytes()
}

/// Decodes a canonical encoding; anything else is [`Error::InvalidPoint`].
pub(crate) fn decode_point(b: &[u8; 32]) -> Result<RistrettoPoint, Error> {
    CompressedRistretto(*b)
        .decompress()
        .ok_or(Error::InvalidPoint)
}

/// AES-256 keyed by the point's encoding, applied to
/// `block_from_uint(tweak)`.
pub(crate) fn hash_point(tweak: u64, pt: &RistrettoPoint) -> Block {
    let cipher = Aes256::new(&point_bytes(pt).into());
    let mut b = aes::Block::from(block_from_uint(tweak));
    cipher.encrypt_block(&mut b);
    b.into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn transpose_known_answer() {
        #[rustfmt::skip]
        let mut input: Vec<u8> = vec![
            0b01010110, 0b10000100, 0b01111010, 0b11111110,
            0b10010111, 0b10011001, 0b01110011, 0b10011011,
            0b00111101, 0b10010110, 0b01110111, 0b10010111,
            0b00101010, 0b00111011, 0b01011000, 0b11101111,
            0b01100000, 0b10011011, 0b10100111, 0b10000011,
            0b00110010, 0b01000010, 0b10001111, 0b10000101,
            0b11101101, 0b00100101, 0b00110100, 0b11010000,
            0b10110100, 0b10010110, 0b00010000, 0b10001010,

            0b01001101, 0b10010010, 0b10101110, 0b00001011,
            0b01001000, 0b11010011, 0b00101101, 0b10011100,
            0b01100111, 0b11011100, 0b10011010, 0b00110000,
            0b00101011, 0b11111100, 0b01110110, 0b01011110,
            0b11010110, 0b01111011, 0b11011110, 0b00111101,
            0b01010100, 0b10000001, 0b11001101, 0b11111010,
            0b01010011, 0b00010010, 0b01100111, 0b10101010,
            0b01100101, 0b01011010, 0b10100101, 0b10101011,
        ];
        #[rustfmt::skip]
        let output: Vec<u8> = vec![
            0b01000011, 0b00001000,
            0b10001010, 0b11101111,
            0b00111111, 0b00110001,
            0b11100101, 0b00001110,
            0b00110010, 0b11010000,
            0b11100011, 0b10101101,
            0b11010100, 0b00111010,
            0b01100010, 0b10110011,

            0b11101001, 0b11110100,
            0b00000100, 0b01111001,
            0b00010010, 0b00011000,
            0b01111001, 0b11111011,
            0b01011000, 0b00111001,
            0b10100011, 0b00110000,
            0b00111101, 0b11001011,
            0b01011010, 0b01001100,

            0b00001100, 0b10101101,
            0b11110000, 0b00011110,
            0b11101010, 0b11010011,
            0b11110011, 0b00111000,
            0b10010100, 0b11101100,
            0b00101110, 0b11011111,
            0b11101100, 0b10111010,
            0b01101100, 0b01000111,

            0b11111111, 0b01000111,
            0b10010010, 0b00010100,
            0b10010000, 0b00101111,
            0b11100010, 0b01111100,
            0b11010001, 0b11011111,
            0b10110100, 0b01011000,
            0b11111001, 0b10010111,
            0b01111100, 0b10001001,
        ];
        transpose_matrix(&mut input, 16, 4).unwrap();
        assert_eq!(input, output);
    }

    fn bit(m: &[u8], row: usize, col: usize, row_bytes: usize) -> bool {
        m[row * row_bytes + col / 8] >> (7 - col % 8) & 1 != 0
    }

    /// Eklundh's algorithm agrees with the definition for every shape family
    /// the protocols use.
    #[test]
    fn transpose_matches_naive() {
        let shapes = [
            (8, 1),
            (8, 4),
            (16, 1),
            (128, 1),
            (128, 2),
            (128, 8),
            (128, 16),
            (128, 96),
            (512, 2),
            (512, 32),
            (512, 64),
            (16, 64),
            (64, 64),
            (1024, 64),
            (1536, 64),
        ];
        let mut rng = AesRng::new(&[3; 16]);
        for (nrows, ncols) in shapes {
            let mut m = vec![0u8; nrows * ncols];
            rng.fill_bytes(&mut m);
            let orig = m.clone();
            transpose_matrix(&mut m, nrows, ncols).unwrap();
            for r in 0..nrows {
                for c in 0..ncols * 8 {
                    assert_eq!(
                        bit(&orig, r, c, ncols),
                        bit(&m, c, r, nrows / 8),
                        "{nrows}x{ncols} at ({r}, {c})"
                    );
                }
            }
        }
    }

    #[test]
    fn transpose_rejects_unsupported_shapes() {
        assert!(transpose_matrix(&mut [0; 12], 12, 1).is_err());
        assert!(transpose_matrix(&mut [0; 48], 16, 3).is_err());
        assert!(transpose_matrix(&mut [0; 24], 8, 3).is_ok());
        assert!(transpose_matrix(&mut [0; 7], 8, 1).is_err());
        assert!(transpose_matrix(&mut [], 0, 0).is_ok());
    }

    #[test]
    fn bools_round_trip() {
        let b = [0b1000_0001, 0b0101_0000];
        let bools = bools_from_bytes(&b);
        assert_eq!(bools.len(), 16);
        assert!(bools[0] && bools[7] && bools[9] && bools[11]);
        assert_eq!(bytes_from_bools(&bools), b);
        assert_eq!(bytes_from_bools(&bools[..9]), [0b1000_0001, 0]);
    }
}
