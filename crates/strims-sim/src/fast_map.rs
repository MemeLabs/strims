// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Maps the WAN model looks up on every message and never iterates for
//! anything order could change: hashed rather than ordered (a `BTreeMap`
//! keyed by addresses compares their octets through `memcmp`), and
//! insertion-ordered, so a run that does iterate one still repeats.

use std::hash::{BuildHasherDefault, Hasher};

use indexmap::IndexMap;

/// An insertion-ordered map hashed by [`FastHasher`].
pub(crate) type FastMap<K, V> = IndexMap<K, V, BuildHasherDefault<FastHasher>>;

/// FxHash's mix (a rotate, an xor and a multiply per word): fast for the
/// small integer and address keys of the model, which no peer chooses.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FastHasher(u64);

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for &w in words {
            self.write_u64(u64::from_le_bytes(w));
        }
        for &b in rest {
            self.write_u64(u64::from(b));
        }
    }

    fn write_u8(&mut self, x: u8) {
        self.write_u64(u64::from(x));
    }

    fn write_u16(&mut self, x: u16) {
        self.write_u64(u64::from(x));
    }

    fn write_u32(&mut self, x: u32) {
        self.write_u64(u64::from(x));
    }

    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(SEED);
    }

    fn write_usize(&mut self, x: usize) {
        self.write_u64(x as u64);
    }
}
