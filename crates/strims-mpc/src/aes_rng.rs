// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic AES-128 counter-mode generator.
//!
//! The output is one contiguous keystream however the reads are sized:
//! block `k` is `AES(seed, le64(2k) || le64(2k + 1))`. Coin-toss commitments
//! are the first block of a seed's stream, so this layout is wire protocol.

use aes::Aes128;
use aes::cipher::{BlockCipherEncrypt, KeyInit};

use crate::block::Block;

/// Deterministic AES-128 counter-mode generator, seeded with a [`Block`].
#[derive(Clone)]
pub struct AesRng {
    cipher: Aes128,
    state: u64,
    buf: Block,
    /// Bytes of `buf` already handed out; `16` means empty.
    used: usize,
}

impl AesRng {
    /// The generator for `seed`.
    pub fn new(seed: &Block) -> AesRng {
        AesRng {
            cipher: Aes128::new(&(*seed).into()),
            state: 0,
            buf: [0; 16],
            used: 16,
        }
    }

    /// Encrypts the next counter block.
    fn next_block(&mut self) -> Block {
        let lo = u128::from(self.state);
        self.state = self.state.wrapping_add(1);
        let hi = u128::from(self.state);
        self.state = self.state.wrapping_add(1);
        let mut b = aes::Block::from((lo | (hi << 64)).to_le_bytes());
        self.cipher.encrypt_block(&mut b);
        b.into()
    }

    /// Fills `dst` with the next `dst.len()` bytes of the stream.
    //
    // TODO: encrypt runs of blocks with the `aes` crate's parallel block API;
    // the 128- and 512-column generators read many blocks at once.
    pub fn fill_bytes(&mut self, dst: &mut [u8]) {
        let mut dst = dst;
        if self.used < 16 {
            let buffered = self.buf.get(self.used..).unwrap_or_default();
            let n = buffered.len().min(dst.len());
            let (head, tail) = dst.split_at_mut(n);
            head.copy_from_slice(buffered.get(..n).unwrap_or_default());
            self.used += n;
            dst = tail;
        }
        let (blocks, rem) = dst.as_chunks_mut::<16>();
        for block in blocks {
            *block = self.next_block();
        }
        if !rem.is_empty() {
            self.buf = self.next_block();
            let n = rem.len();
            rem.copy_from_slice(self.buf.get(..n).unwrap_or_default());
            self.used = n;
        }
    }

    /// Reads the next block.
    pub fn block(&mut self) -> Block {
        let mut b = [0; 16];
        self.fill_bytes(&mut b);
        b
    }
}

impl core::fmt::Debug for AesRng {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AesRng")
            .field("state", &self.state)
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn first_block_known_answer() {
        let seed = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut rng = AesRng::new(&seed);
        let want = [
            181, 112, 57, 74, 222, 107, 231, 247, 116, 218, 250, 128, 240, 47, 242, 146,
        ];
        assert_eq!(rng.block(), want);
    }

    #[test]
    fn stream_is_independent_of_read_sizes() {
        let seed = [9; 16];
        let mut whole = vec![0u8; 300];
        AesRng::new(&seed).fill_bytes(&mut whole);
        let mut rng = AesRng::new(&seed);
        let mut parts = Vec::new();
        for n in [1, 15, 16, 17, 3, 0, 32, 64, 5, 100, 47] {
            let mut b = vec![0u8; n];
            rng.fill_bytes(&mut b);
            parts.extend_from_slice(&b);
        }
        assert_eq!(parts, whole);
    }
}
