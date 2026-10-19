// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! CRC-32 (IEEE), of bytes or of a message's encoding.
//!
//! prost encodes fields in tag order and maps as `BTreeMap`s, so a message
//! has one encoding in a given build, and its checksum is stable. A
//! checksum compared across builds would need a canonical encoding.

use prost::Message;

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

/// A running CRC-32 (IEEE).
#[derive(Clone, Copy, Debug)]
pub struct Crc32(u32);

impl Default for Crc32 {
    fn default() -> Self {
        Crc32::new()
    }
}

impl Crc32 {
    pub fn new() -> Self {
        Crc32(!0)
    }

    /// The checksum of the bytes so far.
    pub fn sum(&self) -> u32 {
        !self.0
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            // invariant: the index is masked to 0..=255.
            #[allow(clippy::indexing_slicing)]
            let t = TABLE[((self.0 ^ u32::from(b)) & 0xff) as usize];
            self.0 = t ^ (self.0 >> 8);
        }
    }
}

/// CRC-32 (IEEE) of the message's encoding.
pub fn crc32_message<M: Message>(m: &M) -> u32 {
    let mut c = Crc32::new();
    c.update(&m.encode_to_vec());
    c.sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_value() {
        let mut c = Crc32::new();
        c.update(b"1234");
        c.update(b"56789");
        assert_eq!(c.sum(), 0xcbf4_3926);
        assert_eq!(Crc32::new().sum(), 0);
    }

    #[test]
    fn messages_checksum_their_encoding() {
        assert_eq!(crc32_message(&strims_proto::types::Cache::default()), 0);
        let c = strims_proto::types::Cache {
            id: 7,
            ..Default::default()
        };
        let mut crc = Crc32::new();
        crc.update(&[0x08, 0x07]);
        assert_eq!(crc32_message(&c), crc.sum());
    }
}
