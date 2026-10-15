// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Correlation-robust hashes from a fixed-key AES permutation.

use aes::Aes128;
use aes::cipher::{BlockCipherEncrypt, KeyInit};

use crate::block::{Block, xor_block};

/// Key of the fixed hash ALSZ and KOS use. Both peers must use the same key.
pub const FIXED_KEY: Block = [
    0x52, 0xf2, 0x71, 0x98, 0xf2, 0xea, 0xf3, 0x4f, 0x3c, 0x93, 0xbe, 0xc1, 0x17, 0x21, 0x91, 0x3b,
];

/// AES-128 under a fixed key, used as a random permutation `π`.
#[derive(Clone)]
pub struct AesHash {
    cipher: Aes128,
}

impl AesHash {
    /// The hash keyed by `key`.
    pub fn new(key: &Block) -> AesHash {
        AesHash {
            cipher: Aes128::new(&(*key).into()),
        }
    }

    /// The hash keyed by [`FIXED_KEY`].
    pub fn fixed() -> AesHash {
        AesHash::new(&FIXED_KEY)
    }

    fn encrypt(&self, x: &Block) -> Block {
        let mut b = aes::Block::from(*x);
        self.cipher.encrypt_block(&mut b);
        b.into()
    }

    /// Correlation-robust hash `π(x) ⊕ x`.
    pub fn cr_hash(&self, x: &Block) -> Block {
        xor_block(&self.encrypt(x), x)
    }

    /// Tweakable circular correlation-robust hash
    /// `π(π(x) ⊕ i) ⊕ π(x)`.
    pub fn tccr_hash(&self, i: &Block, x: &Block) -> Block {
        let y = self.encrypt(x);
        let z = self.encrypt(&xor_block(&y, i));
        xor_block(&y, &z)
    }
}

impl core::fmt::Debug for AesHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AesHash")
    }
}
