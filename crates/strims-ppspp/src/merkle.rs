// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Merkle hash trees over chunk data.
//!
//! A [`Tree`] covers the chunks under one root bin and stores a digest for
//! every bin under it, plus a bit per bin saying whether that digest is
//! verified. Leaves hash one chunk; a branch hashes its children's digests
//! concatenated. Peers exchange these digests, so the layout is part of the
//! v2 wire format; the golden vectors pin every hash function's digests.
//!
//! The [`Hasher`] is passed into the methods that hash, so the trees stored
//! in a swarm's segment ring carry no hasher state (a BLAKE3 hasher is about
//! 2 KiB).

use core::fmt;

use sha2::digest::{self, Digest};
use strims_binmap::Bin;

/// Merkle errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The data or a digest doesn't match a verified digest.
    #[error("hash mismatch")]
    HashMismatch,
    /// The bin is not under the tree's root.
    #[error("bin out of bounds")]
    BinOutOfBounds,
    /// Less data than the bin's chunks.
    #[error("data shorter than bin")]
    ShortData,
    /// `reset` to a root of another size.
    #[error("reset cannot change root bin size")]
    SizeMismatch,
}

/// A hash function for Merkle trees, holding its running state.
#[derive(Clone)]
pub enum Hasher {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha512(sha2::Sha512),
    Blake2b256(blake2::Blake2b256),
    Blake2b512(blake2::Blake2b512),
    /// Boxed: the BLAKE3 state is about 2 KiB.
    Blake3(Box<blake3::Hasher>),
    Sha3_224(sha3::Sha3_224),
    Sha3_256(sha3::Sha3_256),
    Sha3_384(sha3::Sha3_384),
    Sha3_512(sha3::Sha3_512),
}

fn finish<D: Digest + digest::FixedOutputReset>(h: &mut D, out: &mut [u8]) {
    let r = h.finalize_reset();
    if let Some(dst) = out.get_mut(..r.len()) {
        dst.copy_from_slice(&r);
    }
}

impl Hasher {
    pub fn sha1() -> Hasher {
        Hasher::Sha1(Default::default())
    }
    pub fn sha256() -> Hasher {
        Hasher::Sha256(Default::default())
    }
    pub fn sha512() -> Hasher {
        Hasher::Sha512(Default::default())
    }
    pub fn blake2b256() -> Hasher {
        Hasher::Blake2b256(Default::default())
    }
    pub fn blake2b512() -> Hasher {
        Hasher::Blake2b512(Default::default())
    }
    pub fn blake3() -> Hasher {
        Hasher::Blake3(Box::new(blake3::Hasher::new()))
    }
    pub fn sha3_224() -> Hasher {
        Hasher::Sha3_224(Default::default())
    }
    pub fn sha3_256() -> Hasher {
        Hasher::Sha3_256(Default::default())
    }
    pub fn sha3_384() -> Hasher {
        Hasher::Sha3_384(Default::default())
    }
    pub fn sha3_512() -> Hasher {
        Hasher::Sha3_512(Default::default())
    }

    /// Digest length in bytes.
    pub fn size(&self) -> usize {
        match self {
            Hasher::Sha1(_) => 20,
            Hasher::Sha256(_) | Hasher::Blake2b256(_) | Hasher::Blake3(_) | Hasher::Sha3_256(_) => {
                32
            }
            Hasher::Sha3_224(_) => 28,
            Hasher::Sha3_384(_) => 48,
            Hasher::Sha512(_) | Hasher::Blake2b512(_) | Hasher::Sha3_512(_) => 64,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        match self {
            Hasher::Sha1(h) => Digest::update(h, data),
            Hasher::Sha256(h) => Digest::update(h, data),
            Hasher::Sha512(h) => Digest::update(h, data),
            Hasher::Blake2b256(h) => Digest::update(h, data),
            Hasher::Blake2b512(h) => Digest::update(h, data),
            Hasher::Blake3(h) => {
                h.update(data);
            }
            Hasher::Sha3_224(h) => Digest::update(h, data),
            Hasher::Sha3_256(h) => Digest::update(h, data),
            Hasher::Sha3_384(h) => Digest::update(h, data),
            Hasher::Sha3_512(h) => Digest::update(h, data),
        }
    }

    /// Writes the digest into the first [`Hasher::size`] bytes of `out` and
    /// resets the state.
    pub fn finish_into(&mut self, out: &mut [u8]) {
        match self {
            Hasher::Sha1(h) => finish(h, out),
            Hasher::Sha256(h) => finish(h, out),
            Hasher::Sha512(h) => finish(h, out),
            Hasher::Blake2b256(h) => finish(h, out),
            Hasher::Blake2b512(h) => finish(h, out),
            Hasher::Blake3(h) => {
                let r = h.finalize();
                h.reset();
                if let Some(dst) = out.get_mut(..blake3::OUT_LEN) {
                    dst.copy_from_slice(r.as_bytes());
                }
            }
            Hasher::Sha3_224(h) => finish(h, out),
            Hasher::Sha3_256(h) => finish(h, out),
            Hasher::Sha3_384(h) => finish(h, out),
            Hasher::Sha3_512(h) => finish(h, out),
        }
    }
}

impl fmt::Debug for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Hasher::Sha1(_) => "Sha1",
            Hasher::Sha256(_) => "Sha256",
            Hasher::Sha512(_) => "Sha512",
            Hasher::Blake2b256(_) => "Blake2b256",
            Hasher::Blake2b512(_) => "Blake2b512",
            Hasher::Blake3(_) => "Blake3",
            Hasher::Sha3_224(_) => "Sha3_224",
            Hasher::Sha3_256(_) => "Sha3_256",
            Hasher::Sha3_384(_) => "Sha3_384",
            Hasher::Sha3_512(_) => "Sha3_512",
        })
    }
}

/// A tree of hashes.
///
/// Storage is `2 * root.base_length() - 1` digests and a verified bit per
/// bin, allocated once. Callers bound the root's layer (the swarm options
/// cap chunks per signature at the live window).
#[derive(Clone)]
pub struct Tree {
    hash_size: usize,
    chunk_size: usize,
    root_bin: Bin,
    base_left: Bin,
    verified: Box<[u64]>,
    digests: Box<[u8]>,
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tree")
            .field("root_bin", &self.root_bin)
            .field("hash_size", &self.hash_size)
            .field("chunk_size", &self.chunk_size)
            .finish_non_exhaustive()
    }
}

impl Tree {
    /// An empty tree.
    pub fn new(root_bin: Bin, chunk_size: usize, hash_size: usize) -> Tree {
        let bins = usize::try_from(root_bin.base_length())
            .unwrap_or(usize::MAX)
            .saturating_mul(2)
            .saturating_sub(1);
        Tree {
            hash_size,
            chunk_size,
            root_bin,
            base_left: root_bin.base_left(),
            verified: vec![0; bins.div_ceil(64)].into_boxed_slice(),
            digests: vec![0; bins.saturating_mul(hash_size)].into_boxed_slice(),
        }
    }

    /// Moves the tree to another root of the same size and clears the
    /// verified bits. Digests are left as they were.
    pub fn reset(&mut self, root_bin: Bin) -> Result<(), Error> {
        if self.root_bin.base_length() != root_bin.base_length() {
            return Err(Error::SizeMismatch);
        }
        self.root_bin = root_bin;
        self.base_left = root_bin.base_left();
        self.verified.fill(0);
        Ok(())
    }

    /// Copies the digests `o` has verified and `self` hasn't, and marks
    /// them verified. Both trees must be the same size.
    pub fn merge(&mut self, o: &Tree) {
        if self.hash_size != o.hash_size || self.digests.len() != o.digests.len() {
            return;
        }
        let s = self.hash_size;
        for (i, (tv, ov)) in self.verified.iter_mut().zip(o.verified.iter()).enumerate() {
            let mut added = *ov & !*tv;
            *tv |= *ov;

            let mut j = 0usize;
            while added != 0 {
                let n = added.trailing_zeros();
                added = added.checked_shr(n).unwrap_or(0);
                j += n as usize;

                // the run of set bits starting at j
                let n = added.wrapping_add(1).trailing_zeros();
                let from = (j + i * 64) * s;
                let to = (j + i * 64 + n as usize) * s;
                if let (Some(dst), Some(src)) =
                    (self.digests.get_mut(from..to), o.digests.get(from..to))
                {
                    dst.copy_from_slice(src);
                }
                added = added.checked_shr(n).unwrap_or(0);
                j += n as usize;
            }
        }
    }

    /// The bin the tree covers.
    pub fn root_bin(&self) -> Bin {
        self.root_bin
    }

    pub fn hash_size(&self) -> usize {
        self.hash_size
    }

    fn index(&self, b: Bin) -> Option<usize> {
        if !self.root_bin.contains(b) {
            return None;
        }
        usize::try_from(b.0 - self.base_left.0).ok()
    }

    fn digest(&self, i: usize) -> &[u8] {
        let s = self.hash_size;
        self.digests.get(i * s..(i + 1) * s).unwrap_or_default()
    }

    /// Sets the digest of `b` without verifying it. Returns false, changing
    /// nothing, if `b` is outside the tree or `d` isn't one digest long.
    pub fn set(&mut self, b: Bin, d: &[u8]) -> bool {
        let s = self.hash_size;
        if d.len() != s {
            return false;
        }
        let Some(i) = self.index(b) else {
            return false;
        };
        match self.digests.get_mut(i * s..(i + 1) * s) {
            Some(dst) => {
                dst.copy_from_slice(d);
                true
            }
            None => false,
        }
    }

    /// The digest of `b`, verified or not.
    pub fn get(&self, b: Bin) -> Option<&[u8]> {
        self.index(b).map(|i| self.digest(i))
    }

    /// The root digest.
    pub fn root_digest(&self) -> &[u8] {
        self.get(self.root_bin).unwrap_or_default()
    }

    pub(crate) fn is_verified(&self, b: Bin) -> bool {
        self.index(b).is_some_and(|j| {
            self.verified
                .get(j >> 6)
                .is_some_and(|w| w & (1 << (j & 0x3f)) != 0)
        })
    }

    pub(crate) fn set_verified(&mut self, b: Bin) {
        if let Some(j) = self.index(b)
            && let Some(w) = self.verified.get_mut(j >> 6)
        {
            *w |= 1 << (j & 0x3f);
        }
    }

    /// The digest of `b` from `p` if `p` has verified it, else from `self`.
    fn get_with<'s>(&'s self, b: Bin, p: Option<&'s Tree>) -> &'s [u8] {
        match p {
            Some(p) if p.is_verified(b) => p.get(b).unwrap_or_default(),
            _ => self.get(b).unwrap_or_default(),
        }
    }

    /// Stores the hasher's digest as the digest of `b`. If `p` has verified
    /// `b`, the digest is compared with `p`'s and `b` stays unmarked here.
    /// Returns `(ok, verified)`.
    fn set_or_verify(&mut self, h: &mut Hasher, b: Bin, p: Option<&Tree>) -> (bool, bool) {
        let Some(i) = self.index(b) else {
            return (false, false);
        };
        let s = self.hash_size;
        if let Some(d) = self.digests.get_mut(i * s..(i + 1) * s) {
            h.finish_into(d);
        }
        if let Some(p) = p
            && p.is_verified(b)
        {
            return (self.digest(i) == p.get(b).unwrap_or_default(), true);
        }
        self.set_verified(b);
        (true, false)
    }

    /// [`Tree::set_or_verify`] for branch `b`, hashing its children's
    /// digests.
    fn set_or_verify_branch(&mut self, h: &mut Hasher, b: Bin, p: Option<&Tree>) -> (bool, bool) {
        h.update(self.get_with(b.left(), p));
        h.update(self.get_with(b.right(), p));
        self.set_or_verify(h, b, p)
    }

    /// Hashes the chunks of `d` into the leaves under `b` and the branches
    /// up to `b`.
    pub fn fill(&mut self, h: &mut Hasher, b: Bin, d: &[u8]) -> Result<bool, Error> {
        self.fill_with(h, b, d, None)
    }

    /// Like [`Tree::fill`], checking against the digests `p` has verified.
    /// Returns true once `b` itself (or, for a leaf, the leaf) matches a
    /// verified digest.
    fn fill_with(
        &mut self,
        h: &mut Hasher,
        b: Bin,
        d: &[u8],
        p: Option<&Tree>,
    ) -> Result<bool, Error> {
        if !self.root_bin.contains(b) {
            return Err(Error::BinOutOfBounds);
        }
        let bl = usize::try_from(b.base_length()).map_err(|_| Error::BinOutOfBounds)?;
        let cs = self.chunk_size;
        if d.len() < bl.saturating_mul(cs) {
            return Err(Error::ShortData);
        }

        let mut l = b.base_left();
        let mut r = b.base_right();
        for i in 0..bl {
            h.update(d.get(i * cs..(i + 1) * cs).unwrap_or_default());
            match self.set_or_verify(h, l + 2 * i as u64, p) {
                (false, _) => return Err(Error::HashMismatch),
                (true, true) if b.is_base() => return Ok(true),
                _ => {}
            }
        }

        for i in 1..=b.layer() {
            l = l.parent();
            r = r.parent();
            let w = 1u64 << (i + 1);
            let mut j = l;
            while j <= r {
                match self.set_or_verify_branch(h, j, p) {
                    (false, _) => return Err(Error::HashMismatch),
                    (true, true) if b.layer() == i => return Ok(true),
                    _ => {}
                }
                j += w;
            }
        }

        Ok(false)
    }

    /// Checks the data `d` at `b` using the uncle digests set in `self` and
    /// the digests `p` has verified. Returns true if the path from `b`
    /// reaches a digest `p` verified. Otherwise the computed root digest is
    /// left for the caller to check against a signature.
    pub fn verify(
        &mut self,
        h: &mut Hasher,
        b: Bin,
        d: &[u8],
        p: Option<&Tree>,
    ) -> Result<bool, Error> {
        if self.fill_with(h, b, d, p)? {
            return Ok(true);
        }

        let mut b = b;
        while b != self.root_bin {
            self.set_verified(b.sibling());
            b = b.parent();
            match self.set_or_verify_branch(h, b, p) {
                (false, _) => return Err(Error::HashMismatch),
                (true, true) => return Ok(true),
                _ => {}
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    //! Tree tests: filling, verifying against reference trees, merging.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use rand_core::Rng;
    use strims_base::rng::det_rng;

    fn random_data(n: u64) -> Vec<u8> {
        let mut b = vec![0u8; n as usize];
        det_rng(0x0102_0304).fill_bytes(&mut b);
        b
    }

    fn init_root_hash(t: &mut Tree, digest: &[u8]) {
        let root = t.root_bin;
        assert!(t.set(root, digest));
        t.set_verified(root);
    }

    fn tree(root: Bin, cs: usize) -> Tree {
        Tree::new(root, cs, 32)
    }

    /// Data verifies against a tree that knows only the root, and merging
    /// marks every chunk verified.
    #[test]
    fn verify() {
        let h = &mut Hasher::sha256();
        for (cs, bin) in [(1024usize, Bin::new(5, 0)), (4096, Bin::new(6, 0))] {
            let data = random_data(bin.base_length() * cs as u64);

            let mut r = tree(bin, cs);
            r.fill(h, bin, &data).unwrap();

            let mut r0 = tree(bin, cs);
            init_root_hash(&mut r0, r.get(bin).unwrap());

            let mut r1 = tree(bin, cs);
            assert_eq!(r1.verify(h, bin, &data, Some(&r0)), Ok(true));

            r0.merge(&r1);
            for b in bin.base_left().0..=bin.base_right().0 {
                assert!(r0.is_verified(Bin(b)));
                assert_eq!(r0.get(Bin(b)), r.get(Bin(b)));
            }
        }
    }

    /// A wider bin verifies against digests a narrower verification stored.
    #[test]
    fn duplicate_verify() {
        let h = &mut Hasher::sha256();
        let bin = Bin::new(4, 0);
        let cs = 1024;
        let data = random_data(bin.base_length() * cs as u64);

        let mut r = tree(bin, cs);
        r.fill(h, bin, &data).unwrap();

        let mut r0 = tree(bin, cs);
        assert!(r0.set(bin, r.get(bin).unwrap()));
        r0.set_verified(bin);
        assert!(r0.set(Bin(11), r.get(Bin(11)).unwrap()));
        assert!(r0.set(Bin(23), r.get(Bin(23)).unwrap()));
        assert!(r0.verify(h, Bin(3), &data[..4 * cs], None).is_ok());

        let mut r1 = tree(bin, cs);
        assert_eq!(r1.verify(h, Bin(7), &data[..8 * cs], Some(&r0)), Ok(true));

        r0.merge(&r1);
        for i in 0..8 {
            assert!(r0.is_verified(Bin(i)));
        }
    }

    /// Merging a verified temporary tree gives the reference tree every
    /// digest a later verification needs.
    #[test]
    fn verify_merge() {
        let h = &mut Hasher::sha256();
        let cs = 1024usize;
        let bin = Bin::new(5, 0);
        let fill_bin = Bin::new(1, 4);
        let data = random_data(bin.base_length() * cs as u64);
        let fl = fill_bin.base_offset() as usize * cs;
        let fill_bytes = &data[fl..fl + fill_bin.base_length() as usize * cs];

        let mut r = tree(bin, cs);
        r.fill(h, bin, &data).unwrap();

        let copy_and_verify = |h: &mut Hasher, dst: &mut Tree, src: &Tree, parent: &Tree| {
            let mut b = fill_bin;
            while b != bin {
                assert!(dst.set(b.sibling(), src.get(b.sibling()).unwrap()));
                b = b.parent();
            }
            dst.verify(h, fill_bin, fill_bytes, Some(parent)).unwrap()
        };

        let mut vt = tree(bin, cs);
        // copy/verify root hash to verified tree (ex. received signed munro)
        init_root_hash(&mut vt, r.get(bin).unwrap());
        let mut tt0 = tree(bin, cs);
        // copy unverified sibling hashes to temp tree (ex. received sibling
        // hashes)
        let mut b = fill_bin;
        while b != bin {
            assert!(tt0.set(b.sibling(), r.get(b.sibling()).unwrap()));
            b = b.parent();
        }
        assert!(tt0.verify(h, fill_bin, fill_bytes, Some(&vt)).is_ok());
        assert!(copy_and_verify(h, &mut tt0, &r, &vt));

        // copy verified hashes from temp to verified tree
        vt.merge(&tt0);

        // confirm all the hashes that verify fill_bytes are in the verified
        // tree
        let mut tt1 = tree(bin, cs);
        assert_eq!(tt1.verify(h, fill_bin, fill_bytes, Some(&vt)), Ok(true));
    }

    /// A bin verifies against the root once its uncle digests are set.
    #[test]
    fn verify_forward() {
        let h = &mut Hasher::sha256();
        let cs = 1024usize;
        let root = Bin::new(5, 0);
        let fill_bin = Bin::new(1, 4);
        let data = random_data(root.base_length() * cs as u64);

        let mut r = tree(root, cs);
        r.fill(h, root, &data).unwrap();

        // create reference node with the root hash set
        let mut r0 = tree(root, cs);
        init_root_hash(&mut r0, r.get(root).unwrap());

        let mut r1 = tree(root, cs);
        // set hashes required to verify node on r1
        let mut b = fill_bin;
        while b != root {
            assert!(r1.set(b.sibling(), r.get(b.sibling()).unwrap()));
            b = b.parent();
        }

        assert_eq!(
            r1.verify(h, Bin(17), &data[8 * cs..10 * cs], Some(&r0)),
            Ok(true)
        );
    }

    /// Without verified digests, verification returns false and leaves the
    /// computed root for a signature check.
    #[test]
    fn no_verified_reference_node() {
        let h = &mut Hasher::sha256();
        let cs = 1024usize;
        let bin = Bin::new(5, 0);
        let fill_bin = Bin::new(1, 4);
        let data = random_data(bin.base_length() * cs as u64);

        let mut r = tree(bin, cs);
        r.fill(h, bin, &data).unwrap();

        // create reference node with no root hash
        let r0 = tree(bin, cs);

        let mut r1 = tree(bin, cs);
        let mut b = fill_bin;
        while b != bin {
            assert!(r1.set(b.sibling(), r.get(b.sibling()).unwrap()));
            b = b.parent();
        }

        // should return false since r0 has no hashes to verify against
        assert_eq!(
            r1.verify(h, Bin(17), &data[8 * cs..10 * cs], Some(&r0)),
            Ok(false)
        );
        // the computed root matches the real one, for a signature check
        assert_eq!(r1.root_digest(), r.root_digest());
    }

    #[test]
    fn tampered_data_fails() {
        let h = &mut Hasher::sha256();
        let cs = 64usize;
        let root = Bin::new(3, 0);
        let mut data = random_data(root.base_length() * cs as u64);
        let mut r = tree(root, cs);
        r.fill(h, root, &data).unwrap();
        let mut r0 = tree(root, cs);
        init_root_hash(&mut r0, r.root_digest());

        data[3 * cs + 5] ^= 1;
        let mut r1 = tree(root, cs);
        let mut b = Bin(6);
        while b != root {
            assert!(r1.set(b.sibling(), r.get(b.sibling()).unwrap()));
            b = b.parent();
        }
        assert_eq!(
            r1.verify(h, Bin(6), &data[3 * cs..4 * cs], Some(&r0)),
            Err(Error::HashMismatch)
        );
    }

    #[test]
    fn bounds_are_checked() {
        let h = &mut Hasher::sha256();
        let mut t = tree(Bin::new(2, 1), 16);
        assert_eq!(t.fill(h, Bin(0), &[0; 16]), Err(Error::BinOutOfBounds));
        assert_eq!(t.fill(h, Bin(8), &[0; 15]), Err(Error::ShortData));
        assert_eq!(t.fill(h, Bin::NONE, &[0; 16]), Err(Error::BinOutOfBounds));
        assert!(!t.set(Bin(7), &[0; 32]));
        assert!(!t.set(Bin(8), &[0; 31]));
        assert!(t.set(Bin(8), &[0; 32]));
        assert_eq!(t.get(Bin(16)), None);
        assert_eq!(t.reset(Bin::new(3, 0)), Err(Error::SizeMismatch));
        assert_eq!(t.reset(Bin::new(2, 9)), Ok(()));
        assert_eq!(t.root_bin(), Bin::new(2, 9));
    }

    /// Merge copies runs that span verified-bitmap words.
    #[test]
    fn merge_across_words() {
        let h = &mut Hasher::blake3();
        let cs = 8usize;
        let root = Bin::new(7, 0);
        let data = random_data(root.base_length() * cs as u64);
        let mut full = Tree::new(root, cs, 32);
        full.fill(h, root, &data).unwrap();
        let mut empty = Tree::new(root, cs, 32);
        empty.merge(&full);
        for b in root.base_left().0..=root.base_right().0 {
            assert!(empty.is_verified(Bin(b)), "{b}");
            assert_eq!(empty.get(Bin(b)), full.get(Bin(b)));
        }
    }
}
