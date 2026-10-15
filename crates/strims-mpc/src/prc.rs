// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The pseudorandom code KKRT uses.

use aes::Aes128;
use aes::cipher::{BlockCipherEncrypt, KeyInit};

use crate::block::{Block, Block512};

/// Four AES-128 keys, one per 128 bit lane.
#[derive(Clone)]
pub struct PseudorandomCode {
    ciphers: [Aes128; 4],
}

impl PseudorandomCode {
    /// The code with one key per lane.
    pub fn new(keys: &[Block; 4]) -> PseudorandomCode {
        PseudorandomCode {
            ciphers: keys.map(|k| Aes128::new(&k.into())),
        }
    }

    /// Lane `i` of the output is `AES(k_i, b)`.
    pub fn encode(&self, b: &Block) -> Block512 {
        let mut dst = [0u8; 64];
        for (lane, cipher) in dst.as_chunks_mut::<16>().0.iter_mut().zip(&self.ciphers) {
            let mut x = aes::Block::from(*b);
            cipher.encrypt_block(&mut x);
            *lane = x.into();
        }
        dst
    }
}

impl core::fmt::Debug for PseudorandomCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("PseudorandomCode")
    }
}
