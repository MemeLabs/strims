// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! CRC-32 (IEEE) as a [`StableSink`], so [`crate::crc32_message`] needn't
//! buffer the encoding.

use strims_proto::stable::StableSink;

const POLY: u32 = 0xedb8_8320;

// invariant: `i` stays below 256.
#[allow(clippy::indexing_slicing)]
const TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 { POLY ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

pub(crate) struct Crc32(u32);

impl Crc32 {
    pub(crate) fn new() -> Self {
        Crc32(!0)
    }

    pub(crate) fn sum(&self) -> u32 {
        !self.0
    }
}

impl StableSink for Crc32 {
    fn put(&mut self, bytes: &[u8]) {
        for &b in bytes {
            // invariant: the index is masked to 0..=255.
            #[allow(clippy::indexing_slicing)]
            let t = TABLE[((self.0 ^ u32::from(b)) & 0xff) as usize];
            self.0 = t ^ (self.0 >> 8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_value() {
        let mut c = Crc32::new();
        c.put(b"1234");
        c.put(b"56789");
        assert_eq!(c.sum(), 0xcbf4_3926);
        assert_eq!(Crc32::new().sum(), 0);
    }
}
