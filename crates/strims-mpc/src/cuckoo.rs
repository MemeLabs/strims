// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The cuckoo hash table PSZ uses to give each receiver element its own OPRF
//! slot.
//!
//! Both peers must compute the same bin for every element, so the table
//! size, the hash functions and the hash-index byte are wire protocol.

use aes::Aes128;
use aes::cipher::{BlockCipherEncrypt, KeyInit};

use crate::Error;
use crate::block::{Block, and_block, block_from_uint, xor_block};

/// Clears the low byte, which holds the hash index.
pub(crate) const CUCKOO_MASK: Block = [
    0x0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

/// Evictions before giving up.
const CUCKOO_NITERS: usize = 1000;

/// Largest set [`compute_mask_size`] accepts.
pub(crate) const MAX_CUCKOO_SET_SIZE: usize = 1 << 28;

/// Table size for `n` items and `nhashes` hash functions.
/// Numbers taken from <https://thomaschneider.de/papers/PSZ18.pdf>, §3.2.2.
pub(crate) fn compute_nbins(n: usize, nhashes: usize) -> Result<usize, Error> {
    let factor = match nhashes {
        3 if n < 1 << 27 => 1.37,
        3 => 1.62,
        4 => 1.09,
        5 => 1.05,
        _ => return Err(Error::InvalidCuckooParameters),
    };
    // Both peers must get the same bin count: keep this exact IEEE double
    // arithmetic.
    let r = (factor * n as f64).ceil() as usize;
    Ok(r.max(1))
}

/// Bytes of each OPRF output PSZ sends. Numbers taken
/// from <https://eprint.iacr.org/2016/799>, Table 2 (the `v` column).
pub(crate) fn compute_mask_size(n: usize) -> Result<usize, Error> {
    Ok(match n {
        n if n <= 1 << 8 => 7,
        n if n <= 1 << 12 => 8,
        n if n <= 1 << 16 => 9,
        n if n <= 1 << 20 => 10,
        n if n <= 1 << 24 => 11,
        n if n <= MAX_CUCKOO_SET_SIZE => 12,
        _ => return Err(Error::InvalidCuckooSetSize),
    })
}

/// The first three hash functions read 32 bit words of the (already random)
/// entry; later ones encrypt the index under the entry.
///
/// Hash 0 reads bytes 0..4, and byte 0 is the hash index, always 0 for it,
/// so it reaches only `nbins / gcd(256, nbins)` bins.
pub(crate) fn cuckoo_hash_bin(hash: &Block, hidx: usize, nbins: usize) -> usize {
    debug_assert!(nbins > 0);
    let nbins = nbins.max(1);
    if hidx < 3 {
        let mut b = [0u8; 4];
        if let Some(src) = hash.get(hidx * 4..hidx * 4 + 4) {
            b.copy_from_slice(src);
        }
        // nbins fits in u32 for every table compute_nbins builds.
        return (u32::from_le_bytes(b) % nbins as u32) as usize;
    }
    let cipher = Aes128::new(&(*hash).into());
    let mut b = aes::Block::from(block_from_uint(hidx as u64));
    cipher.encrypt_block(&mut b);
    let b: Block = b.into();
    (u128::from_le_bytes(b) % nbins as u128) as usize
}

/// An element placed in a [`CuckooHashMap`] bin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CuckooItem {
    /// The input with its hash index in the low byte.
    pub entry: Block,
    /// Position of the input in the caller's list.
    pub input_index: usize,
    /// Which hash function placed it.
    pub hash_index: usize,
}

/// A cuckoo hash table with one slot per bin.
#[derive(Clone, Debug)]
pub struct CuckooHashMap {
    /// One slot per bin; length `nbins`, fixed at construction.
    items: Vec<Option<CuckooItem>>,
    nhashes: usize,
}

impl CuckooHashMap {
    /// Builds the table for `inputs`.
    ///
    /// Fails with [`Error::CuckooHashMapFull`] if an element can't be placed.
    /// With three hash functions this happens for a few percent of small
    /// sets, because hash 0 reaches only some bins. No placement exists in
    /// those cases, so only different digests (a new coin toss) can help.
    pub fn new(inputs: &[Block], nhashes: usize) -> Result<CuckooHashMap, Error> {
        let nbins = compute_nbins(inputs.len(), nhashes)?;
        let mut c = CuckooHashMap {
            items: vec![None; nbins],
            nhashes,
        };
        for (i, input) in inputs.iter().enumerate() {
            c.hash(input, i)?;
        }
        Ok(c)
    }

    /// The number of bins.
    pub fn cap(&self) -> usize {
        self.items.len()
    }

    /// The bins in order.
    pub fn items(&self) -> &[Option<CuckooItem>] {
        &self.items
    }

    /// Inserts `input` as element `idx`, evicting as needed.
    pub fn hash(&mut self, input: &Block, idx: usize) -> Result<(), Error> {
        let nbins = self.items.len();
        let mut item = CuckooItem {
            entry: *input,
            input_index: idx,
            hash_index: 0,
        };
        for _ in 0..CUCKOO_NITERS {
            item.entry = xor_block(
                &and_block(&item.entry, &CUCKOO_MASK),
                &block_from_uint(item.hash_index as u64),
            );
            let bin = cuckoo_hash_bin(&item.entry, item.hash_index, nbins);
            let Some(slot) = self.items.get_mut(bin) else {
                return Err(Error::Internal);
            };
            match slot.replace(item) {
                None => return Ok(()),
                Some(evicted) => {
                    item = evicted;
                    item.hash_index = (item.hash_index + 1) % self.nhashes;
                }
            }
        }
        Err(Error::CuckooHashMapFull)
    }

    /// The entries in bin order.
    pub fn values(&self) -> Vec<Block> {
        self.items.iter().flatten().map(|i| i.entry).collect()
    }
}
