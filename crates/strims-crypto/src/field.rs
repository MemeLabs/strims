// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Minimal arithmetic in GF(2^255 - 19) for [`crate::curve25519`].
//!
//! [`crate::curve25519::public_key_to_curve25519`] must convert any 32
//! bytes, including off-curve points and non-canonical encodings.
//! curve25519-dalek converts only decompressed points and keeps its field
//! type private, so the arithmetic lives here. Only public data passes
//! through, so nothing here needs to be constant time.
//!
//! Elements are five 51-bit limbs, little-endian.

const MASK: u64 = (1 << 51) - 1;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Fe([u64; 5]);

impl Fe {
    pub(crate) const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    /// Loads a little-endian encoding, ignoring bit 255 and accepting
    /// non-canonical values.
    pub(crate) fn from_bytes(b: &[u8; 32]) -> Fe {
        let mut w = [0u64; 4];
        for (wi, chunk) in w.iter_mut().zip(b.as_chunks::<8>().0) {
            *wi = u64::from_le_bytes(*chunk);
        }
        let [w0, w1, w2, w3] = w;
        Fe([
            w0 & MASK,
            ((w0 >> 51) | (w1 << 13)) & MASK,
            ((w1 >> 38) | (w2 << 26)) & MASK,
            ((w2 >> 25) | (w3 << 39)) & MASK,
            (w3 >> 12) & MASK,
        ])
    }

    /// Canonical little-endian encoding (fully reduced mod p).
    pub(crate) fn to_bytes(self) -> [u8; 32] {
        let [mut l0, mut l1, mut l2, mut l3, mut l4] = self.weak_reduce().weak_reduce().0;

        // q = 1 iff the value is >= p; then add 19q and drop bit 255, which
        // subtracts pq.
        let mut q = (l0 + 19) >> 51;
        q = (l1 + q) >> 51;
        q = (l2 + q) >> 51;
        q = (l3 + q) >> 51;
        q = (l4 + q) >> 51;

        l0 += 19 * q;
        l1 += l0 >> 51;
        l0 &= MASK;
        l2 += l1 >> 51;
        l1 &= MASK;
        l3 += l2 >> 51;
        l2 &= MASK;
        l4 += l3 >> 51;
        l3 &= MASK;
        l4 &= MASK;

        let words = [
            l0 | (l1 << 51),
            (l1 >> 13) | (l2 << 38),
            (l2 >> 26) | (l3 << 25),
            (l3 >> 39) | (l4 << 12),
        ];
        let mut out = [0u8; 32];
        for (chunk, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(words) {
            *chunk = word.to_le_bytes();
        }
        out
    }

    /// Carries so every limb is below 2^51, except limb 0, which can exceed
    /// it by a little.
    fn weak_reduce(self) -> Fe {
        let [mut l0, mut l1, mut l2, mut l3, mut l4] = self.0;
        l1 += l0 >> 51;
        l0 &= MASK;
        l2 += l1 >> 51;
        l1 &= MASK;
        l3 += l2 >> 51;
        l2 &= MASK;
        l4 += l3 >> 51;
        l3 &= MASK;
        l0 += (l4 >> 51) * 19;
        l4 &= MASK;
        Fe([l0, l1, l2, l3, l4])
    }

    pub(crate) fn add(self, o: Fe) -> Fe {
        let (a, b) = (self.weak_reduce().0, o.weak_reduce().0);
        let mut r = [0u64; 5];
        for ((ri, ai), bi) in r.iter_mut().zip(a).zip(b) {
            *ri = ai + bi;
        }
        Fe(r).weak_reduce()
    }

    pub(crate) fn sub(self, o: Fe) -> Fe {
        // Adds 16p before subtracting so limbs never underflow.
        const SIXTEEN_P: [u64; 5] = [16 * (MASK - 18), 16 * MASK, 16 * MASK, 16 * MASK, 16 * MASK];
        let (a, b) = (self.weak_reduce().0, o.weak_reduce().0);
        let mut r = [0u64; 5];
        for (((ri, ai), bi), pi) in r.iter_mut().zip(a).zip(b).zip(SIXTEEN_P) {
            *ri = ai + pi - bi;
        }
        Fe(r).weak_reduce()
    }

    pub(crate) fn mul(self, o: Fe) -> Fe {
        let [a0, a1, a2, a3, a4] = self.weak_reduce().0;
        let [b0, b1, b2, b3, b4] = o.weak_reduce().0;
        let m = |x: u64, y: u64| u128::from(x) * u128::from(y);
        let (b1_19, b2_19, b3_19, b4_19) = (b1 * 19, b2 * 19, b3 * 19, b4 * 19);

        let r0 = m(a0, b0) + m(a1, b4_19) + m(a2, b3_19) + m(a3, b2_19) + m(a4, b1_19);
        let r1 = m(a0, b1) + m(a1, b0) + m(a2, b4_19) + m(a3, b3_19) + m(a4, b2_19);
        let r2 = m(a0, b2) + m(a1, b1) + m(a2, b0) + m(a3, b4_19) + m(a4, b3_19);
        let r3 = m(a0, b3) + m(a1, b2) + m(a2, b1) + m(a3, b0) + m(a4, b4_19);
        let r4 = m(a0, b4) + m(a1, b3) + m(a2, b2) + m(a3, b1) + m(a4, b0);

        let mask = u128::from(MASK);
        let r1 = r1 + (r0 >> 51);
        let r2 = r2 + (r1 >> 51);
        let r3 = r3 + (r2 >> 51);
        let r4 = r4 + (r3 >> 51);
        let r0 = (r0 & mask) + (r4 >> 51) * 19;
        let r1 = (r1 & mask) + (r0 >> 51);
        // Every value is now below 2^64, so the narrowing casts are exact.
        Fe([
            (r0 & mask) as u64,
            r1 as u64,
            (r2 & mask) as u64,
            (r3 & mask) as u64,
            (r4 & mask) as u64,
        ])
        .weak_reduce()
    }

    fn square(self) -> Fe {
        self.mul(self)
    }

    /// `self^(p-2)`, so zero maps to zero.
    pub(crate) fn invert(self) -> Fe {
        // p - 2 = 2^255 - 21: bits 254..=5 are set, and the low five bits
        // are 0b01011.
        let mut r = Fe::ONE;
        for bit in (0..255u32).rev() {
            r = r.square();
            if bit >= 5 || (0b01011 >> bit) & 1 == 1 {
                r = r.mul(self);
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fe(n: u64) -> Fe {
        let mut b = [0u8; 32];
        b[..8].copy_from_slice(&n.to_le_bytes());
        Fe::from_bytes(&b)
    }

    const P: [u8; 32] = {
        let mut p = [0xffu8; 32];
        p[0] = 0xed;
        p[31] = 0x7f;
        p
    };

    #[test]
    fn small_arithmetic() {
        assert_eq!(fe(3).mul(fe(5)).to_bytes(), fe(15).to_bytes());
        assert_eq!(fe(3).add(fe(5)).to_bytes(), fe(8).to_bytes());
        assert_eq!(fe(3).sub(fe(5)).add(fe(2)).to_bytes(), [0u8; 32]);
        assert_eq!(fe(7).mul(fe(7).invert()).to_bytes(), Fe::ONE.to_bytes());
        assert_eq!(fe(0).invert().to_bytes(), [0u8; 32]);
    }

    #[test]
    fn canonical_encoding() {
        // p encodes as zero, p + 1 as one, and bit 255 is ignored.
        assert_eq!(Fe::from_bytes(&P).to_bytes(), [0u8; 32]);
        let mut p1 = P;
        p1[0] += 1;
        assert_eq!(Fe::from_bytes(&p1).to_bytes(), Fe::ONE.to_bytes());
        let mut high = [0u8; 32];
        high[0] = 9;
        high[31] = 0x80;
        assert_eq!(Fe::from_bytes(&high).to_bytes(), fe(9).to_bytes());
        // 2^255 - 1 = p + 18
        assert_eq!(Fe::from_bytes(&[0xff; 32]).to_bytes(), fe(18).to_bytes());
        // -1 = p - 1
        let mut minus_one = P;
        minus_one[0] -= 1;
        assert_eq!(fe(0).sub(Fe::ONE).to_bytes(), minus_one);
        assert_eq!(
            Fe::from_bytes(&minus_one)
                .mul(Fe::from_bytes(&minus_one))
                .to_bytes(),
            Fe::ONE.to_bytes()
        );
    }
}
