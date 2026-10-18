// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A bit reader over a byte slice, most significant bit first.

/// Why a read failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BitsError {
    /// The input ended.
    End,
    /// An exp-Golomb code with more than 31 leading zeros.
    CodeTooLong,
}

pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    /// The next byte of `data` to load.
    pos: usize,
    /// The unread bits of the loaded byte, left-aligned.
    cur: u8,
    /// How many bits of `cur` are unread.
    left: u32,
    /// Strip emulation prevention bytes (H.264 7.4.1).
    rbsp: bool,
    /// Zero bytes just loaded, up to 2.
    zeros: u8,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            pos: 0,
            cur: 0,
            left: 0,
            rbsp: false,
            zeros: 0,
        }
    }

    /// A reader of a NAL unit's payload, which drops each 0x03 that
    /// follows two zero bytes.
    pub(crate) fn rbsp(data: &'a [u8]) -> Self {
        BitReader {
            rbsp: true,
            ..BitReader::new(data)
        }
    }

    fn load(&mut self) -> Result<u8, BitsError> {
        loop {
            let b = *self.data.get(self.pos).ok_or(BitsError::End)?;
            self.pos += 1;
            if !self.rbsp {
                return Ok(b);
            }
            if self.zeros == 2 && b == 0x03 {
                self.zeros = 0;
                continue;
            }
            self.zeros = if b == 0 { (self.zeros + 1).min(2) } else { 0 };
            return Ok(b);
        }
    }

    pub(crate) fn bit(&mut self) -> Result<bool, BitsError> {
        if self.left == 0 {
            self.cur = self.load()?;
            self.left = 8;
        }
        let bit = self.cur & 0x80 != 0;
        self.cur <<= 1;
        self.left -= 1;
        Ok(bit)
    }

    /// The next `n` bits as an unsigned number; `n` is at most 32.
    pub(crate) fn bits(&mut self, n: u32) -> Result<u32, BitsError> {
        let mut v = 0u32;
        for _ in 0..n.min(32) {
            v = (v << 1) | u32::from(self.bit()?);
        }
        Ok(v)
    }

    /// An unsigned exp-Golomb code, ue(v).
    pub(crate) fn ue(&mut self) -> Result<u32, BitsError> {
        let mut zeros = 0;
        while !self.bit()? {
            zeros += 1;
            if zeros > 31 {
                return Err(BitsError::CodeTooLong);
            }
        }
        let rest = u64::from(self.bits(zeros)?);
        // At most 2^31 - 1 + 2^31 - 1.
        u32::try_from((1u64 << zeros) - 1 + rest).map_err(|_| BitsError::CodeTooLong)
    }

    /// A signed exp-Golomb code, se(v).
    pub(crate) fn se(&mut self) -> Result<i64, BitsError> {
        let k = i64::from(self.ue()?);
        Ok(if k % 2 == 1 { (k + 1) / 2 } else { -(k / 2) })
    }
}

#[cfg(test)]
mod tests {
    // Test code: a panic is a test failure.
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::test_util::BitWriter;

    #[test]
    fn reads_bits_most_significant_first() {
        let mut r = BitReader::new(&[0b1010_0000, 0xff]);
        assert_eq!(r.bit(), Ok(true));
        assert_eq!(r.bits(3), Ok(0b010));
        assert_eq!(r.bits(8), Ok(0b0000_1111));
        assert_eq!(r.bits(4), Ok(0xf));
        assert_eq!(r.bit(), Err(BitsError::End));
    }

    #[test]
    fn reads_thirty_two_bits() {
        let mut r = BitReader::new(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(r.bits(32), Ok(0xdead_beef));
    }

    #[test]
    fn exp_golomb_round_trips() {
        let values = [0, 1, 2, 3, 7, 8, 255, 65_535, 1 << 31, u32::MAX - 1];
        let signed = [0, 1, -1, 2, -2, 127, -128, i32::MAX, -i32::MAX];
        let mut w = BitWriter::new();
        for v in values {
            w.ue(v);
        }
        for v in signed {
            w.se(v);
        }
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        for v in values {
            assert_eq!(r.ue(), Ok(v));
        }
        for v in signed {
            assert_eq!(r.se(), Ok(i64::from(v)));
        }
    }

    #[test]
    fn exp_golomb_longer_than_32_bits_is_refused() {
        let mut r = BitReader::new(&[0, 0, 0, 0, 0xff]);
        assert_eq!(r.ue(), Err(BitsError::CodeTooLong));
    }

    #[test]
    fn rbsp_drops_emulation_prevention_bytes() {
        let nal = [
            0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x03,
        ];
        let mut r = BitReader::rbsp(&nal);
        let mut out = Vec::new();
        while let Ok(b) = r.bits(8) {
            out.push(b);
        }
        assert_eq!(out, [0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03]);
    }

    #[test]
    fn plain_reader_keeps_emulation_prevention_bytes() {
        let mut r = BitReader::new(&[0x00, 0x00, 0x03]);
        assert_eq!(r.bits(24), Ok(0x03));
    }
}
