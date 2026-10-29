// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Merkle tree integrity.
//!
//! The stream is cut into munros of `chunks_per_signature` chunks. The
//! seeder signs each munro's root hash with a timestamp, at the munro's bin
//! with its count of chunks with data. A peer sending a chunk sends the
//! signed root (unless the receiver already has part of the munro) and the
//! uncle hashes the receiver lacks; the receiver recomputes the path and
//! checks it against a root it verified before or against the signature.
//!
//! A munro signed with fewer chunks than it spans ends in data-less
//! chunks, all zero. Their subtrees' digests are the zero digests (the
//! digest of an all-zero chunk, and of two zero digests above it, layer by
//! layer), which every verifier computes once: senders leave the uncles
//! wholly inside the data-less chunks out, and receivers put the zero
//! digests there whatever a peer sent. A stored munro keeps its count and
//! has those subtrees verified.
//!
//! [`MerkleSwarmVerifier`] keeps the verified trees of the last
//! `live_window / chunks_per_signature` munros in a ring. Each slot keeps
//! its tree allocation when it is reused, and verified trees are swapped
//! (not copied) in from channels, so steady state allocates nothing. The
//! ring advances by at most one window per stored munro, however far its
//! index jumps.

use core::mem;

use strims_base::time::Time;
use strims_binmap::{Bin, Map};
use strims_crypto::Domain;
use strims_proto::types::{Cache, cache};

use super::{
    Error, MerkleHashTreeFunction, Position, Signature, SignatureSigner, SignatureVerifier,
    ring_size,
};
use crate::codec::{Integrity, Message, MessageWriter, SignedIntegrity};
use crate::merkle::{Hasher, Tree};

/// The settings of a [`MerkleSwarmVerifier`].
#[derive(Clone, Debug)]
pub struct MerkleOptions {
    /// The live window in chunks.
    pub live_discard_window: usize,
    pub chunk_size: usize,
    pub chunks_per_signature: usize,
    pub verifier: SignatureVerifier,
    pub hash: MerkleHashTreeFunction,
}

#[derive(Debug)]
pub(super) struct Segment {
    pub(super) occupied: bool,
    timestamp: Time,
    signature: Signature,
    /// The munro's chunks with data, as signed; the rest are data-less.
    pub(super) chunks: u64,
    /// Kept when the slot is freed, so a reused slot doesn't allocate.
    tree: Option<Tree>,
}

/// The ring of verified munro trees. Indexes are munro indexes.
#[derive(Debug)]
pub(super) struct SegmentRing {
    tree_height: u64,
    mask: u64,
    pub(super) size: u64,
    pub(super) head: u64,
    pub(super) tail: u64,
    chunk_size: usize,
    hash_size: usize,
    /// Bound: `size` slots, `live_window / chunks_per_signature`.
    pub(super) segments: Box<[Segment]>,
}

impl SegmentRing {
    /// The stored segment whose tree contains `b`.
    fn segment(&self, b: Bin) -> Option<&Segment> {
        let i = b.0 >> self.tree_height;
        if i >= self.head || i < self.tail {
            return None;
        }
        let s = self.segments.get((i & self.mask) as usize)?;
        let tree = s.tree.as_ref()?;
        (s.occupied && tree.root_bin().contains(b)).then_some(s)
    }

    /// Stores a verified tree, signed with `chunks` chunks with data: swaps
    /// it into an empty slot (handing the slot's old allocation back through
    /// `tree`) or merges it into the munro's existing tree.
    fn store(&mut self, ts: Time, tree: &mut Tree, sig: Signature, chunks: u64) {
        let i = tree.root_bin().0 >> self.tree_height;
        if i < self.tail {
            return;
        }

        let head = i.saturating_add(1);
        if head > self.head {
            if self.head != 0 {
                // a jump of more than a window frees each slot once, so a
                // far-future munro costs at most one pass over the ring
                for k in 0..(head - self.head).min(self.size) {
                    if let Some(s) = self
                        .segments
                        .get_mut(((self.head + k) & self.mask) as usize)
                    {
                        s.occupied = false;
                    }
                }
            }
            self.head = head;
            if self.head > self.size {
                self.tail = self.head - self.size;
            }
        }

        let Some(slot) = self.segments.get_mut((i & self.mask) as usize) else {
            return;
        };
        if slot.occupied {
            if let Some(t) = &mut slot.tree {
                t.merge(tree);
            }
            return;
        }
        slot.occupied = true;
        slot.timestamp = ts;
        slot.signature = sig;
        slot.chunks = chunks;
        match &mut slot.tree {
            Some(t) => mem::swap(t, tree),
            None => {
                let fresh = Tree::new(tree.root_bin(), self.chunk_size, self.hash_size);
                slot.tree = Some(mem::replace(tree, fresh));
            }
        }
    }

    fn reset(&mut self) {
        self.head = 0;
        self.tail = 0;
        for s in self.segments.iter_mut() {
            s.occupied = false;
        }
    }
}

/// The largest subtrees of munro `root` that hold none of its first
/// `chunks` chunks: the subtrees of its data-less chunks, at most one per
/// layer, left to right.
pub(crate) fn dataless_subtrees(root: Bin, chunks: u64) -> impl Iterator<Item = Bin> {
    let n = root.base_length();
    let base = root.base_offset();
    let mut x = chunks;
    core::iter::from_fn(move || {
        if x >= n {
            return None;
        }
        // `n` is a power of two above `x`, so the run of `x`'s lowest set
        // bit from `x` stays inside the munro.
        let size = if x == 0 { n } else { 1 << x.trailing_zeros() };
        let b = Bin::new(u64::from(size.trailing_zeros()), (base + x) / size);
        x += size;
        Some(b)
    })
}

/// Whether every chunk of `b`, inside munro `root`, is past its first
/// `chunks`.
fn dataless(root: Bin, chunks: u64, b: Bin) -> bool {
    b.base_offset().saturating_sub(root.base_offset()) >= chunks
}

/// A Merkle swarm's verified munros. See the module docs.
#[derive(Debug)]
pub struct MerkleSwarmVerifier {
    pub(super) ring: SegmentRing,
    hash: MerkleHashTreeFunction,
    hasher: Hasher,
    /// The zero digest of each layer of a munro, from the chunks up: the
    /// digests of data-less subtrees.
    zeros: Box<[u8]>,
    /// The tree the writer and cache import fill before storing it.
    spare: Tree,
    signature_verifier: SignatureVerifier,
    live_window: u64,
}

impl MerkleSwarmVerifier {
    /// The live window and chunks per signature must be powers of two
    /// with `chunks_per_signature <= live_discard_window`.
    pub fn new(o: &MerkleOptions) -> Result<MerkleSwarmVerifier, Error> {
        let window = ring_size(o.live_discard_window)?;
        let cps = ring_size(o.chunks_per_signature)?;
        if cps > window || o.chunk_size == 0 {
            return Err(Error::InvalidOptions);
        }
        let mut hasher = o.hash.hasher().ok_or(Error::UnsupportedHashTreeFunction)?;
        let hash_size = hasher.size();
        let size = window / cps;
        let tree_height = u64::from(cps.trailing_zeros()) + 1;
        let segments = (0..size)
            .map(|_| Segment {
                occupied: false,
                timestamp: Time::EPOCH,
                signature: Signature::EMPTY,
                chunks: 0,
                tree: None,
            })
            .collect();
        let zeros = zero_digests(&mut hasher, o.chunk_size, tree_height);
        Ok(MerkleSwarmVerifier {
            ring: SegmentRing {
                tree_height,
                mask: size - 1,
                size,
                head: 0,
                tail: 0,
                chunk_size: o.chunk_size,
                hash_size,
                segments,
            },
            hash: o.hash,
            hasher,
            zeros,
            spare: Tree::new(Bin::new(tree_height - 1, 0), o.chunk_size, hash_size),
            signature_verifier: o.verifier.clone(),
            live_window: window,
        })
    }

    fn munro_layer(&self) -> u64 {
        self.ring.tree_height - 1
    }

    /// The digest of an all-zero subtree at `layer`.
    fn zero(&self, layer: u64) -> &[u8] {
        let (hs, l) = (
            self.ring.hash_size,
            usize::try_from(layer).unwrap_or(usize::MAX),
        );
        self.zeros
            .get(l.saturating_mul(hs)..l.saturating_add(1).saturating_mul(hs))
            .unwrap_or_default()
    }

    /// The integrity messages for sending `b` to a peer whose have map is
    /// `m`: the signed root if the peer has none of the munro, then the
    /// uncle hashes from `b` up, stopping below the first ancestor the peer
    /// has data under, less those of data-less subtrees, which the peer
    /// derives. A `b` with data-less chunks has no integrity to send. The
    /// golden vectors pin this output.
    pub fn write_integrity(
        &self,
        b: Bin,
        m: &Map,
        w: &mut impl MessageWriter,
    ) -> Result<usize, Error> {
        let s = self.ring.segment(b).ok_or(Error::MissingHashSubtree)?;
        let tree = s.tree.as_ref().ok_or(Error::MissingHashSubtree)?;
        let root = tree.root_bin();
        if dataless(root, s.chunks, b.base_right()) {
            return Err(Error::DataLess);
        }

        let mut n = 0;
        if m.empty_at(root) {
            n += w.write(&Message::SignedIntegrity(SignedIntegrity {
                bin: root,
                timestamp: s.timestamp,
                dataless: Position::with_data(root, s.chunks).dataless,
                signature: s.signature.as_slice(),
            }))?;
        }

        let mut b = b;
        while b != root {
            let p = b.parent();
            if !m.empty_at(p) {
                return Ok(n);
            }
            b = b.sibling();
            if !dataless(root, s.chunks, b) {
                n += w.write(&Message::Integrity(Integrity {
                    bin: b,
                    hash: tree.get(b).unwrap_or_default(),
                }))?;
            }
            b = p;
        }

        Ok(n)
    }

    /// A verifier for one peer channel.
    pub fn channel_verifier(&self) -> MerkleChannelVerifier {
        MerkleChannelVerifier {
            munro_layer: self.munro_layer(),
            bin: Bin::NONE,
            tree: Tree::new(
                Bin::new(self.munro_layer(), 0),
                self.ring.chunk_size,
                self.ring.hash_size,
            ),
            hasher: self.hasher.clone(),
            timestamp: Time::EPOCH,
            signature: Some(Signature::EMPTY),
            dataless: None,
            chunk_size: self.ring.chunk_size,
        }
    }

    /// Hashes, signs and stores munro `index` from local data, whose first
    /// `chunks` chunks hold data: the rest of `data` must be zero.
    /// Publishing the chunks is the caller's job.
    pub(crate) fn write_munro(
        &mut self,
        signer: &SignatureSigner,
        ts: Time,
        index: u64,
        data: &[u8],
        chunks: u64,
    ) -> Result<(), Error> {
        let b = Bin::new(self.munro_layer(), index);
        if chunks == 0 || chunks > b.base_length() {
            return Err(Error::InvalidChunkCount);
        }
        self.spare.reset(b)?;
        self.spare.fill(&mut self.hasher, b, data)?;
        let at = Position::with_data(b, chunks);
        let sig = signer.sign_unit(Domain::PpsppMunroRoot, ts, at, self.spare.root_digest());
        self.ring.store(ts, &mut self.spare, sig, chunks);
        Ok(())
    }

    /// Loads and verifies a cache. The data must start at chunk 0 and the
    /// cache may hold at most a live window of munros, each with data in
    /// every chunk ([`export_cache`](Self::export_cache)). Returns how many
    /// bytes of the data the imported signatures cover.
    ///
    /// Each munro's signature is checked under the current signed layout,
    /// its bin and count included: a cache signed under another layout, or
    /// altered, is refused whole, so a stale cache is dropped and made
    /// again, never misread.
    pub fn import_cache(&mut self, c: &Cache) -> Result<usize, Error> {
        let ic = c
            .integrity
            .as_ref()
            .and_then(|i| i.merkle_integrity.as_ref())
            .ok_or(Error::NoIntegrityCache)?;

        let n = ic.timestamps.len();
        if n == 0 {
            return Ok(0);
        }
        if n as u64 > self.ring.size {
            return Err(Error::CacheTooLarge);
        }
        if ic.signatures.len() < n {
            return Err(Error::IntegrityCacheIncomplete);
        }
        let munro_bytes = self.ring.chunk_size << self.munro_layer();
        if c.data.len() < n * munro_bytes {
            return Err(Error::IntegrityCacheIncomplete);
        }

        let sig_size = self.signature_verifier.size();
        let chunks = 1u64 << self.munro_layer();
        for (i, (&t, sig)) in ic.timestamps.iter().zip(&ic.signatures).enumerate() {
            let ts = Time::from_unix_nanos(t);
            let b = Bin::new(self.munro_layer(), i as u64);
            self.spare.reset(b)?;
            let data = c
                .data
                .get(i * munro_bytes..(i + 1) * munro_bytes)
                .ok_or(Error::IntegrityCacheIncomplete)?;
            self.spare.fill(&mut self.hasher, b, data)?;

            if sig.len() != sig_size
                || !self.signature_verifier.verify_unit(
                    Domain::PpsppMunroRoot,
                    ts,
                    Position {
                        bin: b,
                        dataless: 0,
                    },
                    self.spare.root_digest(),
                    sig,
                )
            {
                return Err(Error::InvalidSignature);
            }
            let sig = Signature::from_slice(sig).ok_or(Error::InvalidSignature)?;
            self.ring.store(ts, &mut self.spare, sig, chunks);
        }
        Ok(n * munro_bytes)
    }

    /// The timestamps and signatures of the munros from slot 0 up to the
    /// first empty slot or the first munro with data-less chunks: a cache
    /// carries no counts, so every munro in it holds data throughout.
    pub fn export_cache(&self) -> cache::Integrity {
        let mut c = cache::MerkleIntegrity::default();
        let chunks = 1u64 << self.munro_layer();
        for s in self.ring.segments.iter() {
            if !s.occupied || s.chunks != chunks {
                break;
            }
            c.timestamps.push(s.timestamp.unix_nanos());
            c.signatures.push(s.signature.as_slice().to_vec());
        }
        cache::Integrity {
            merkle_integrity: Some(c),
            ..Default::default()
        }
    }

    /// Forgets every munro.
    pub fn reset(&mut self) {
        self.ring.reset();
    }

    /// The live window in chunks.
    pub fn live_window(&self) -> u64 {
        self.live_window
    }

    /// The first bin of the oldest munro the ring keeps.
    pub(crate) fn verifiable_from(&self) -> Bin {
        Bin(self
            .ring
            .tail
            .checked_shl(u32::try_from(self.ring.tree_height).unwrap_or(u32::MAX))
            .unwrap_or(u64::MAX))
    }

    /// The hash function.
    pub fn hash_function(&self) -> MerkleHashTreeFunction {
        self.hash
    }
}

/// The hashes and signature a peer sent for one munro, and the tree they
/// are checked in.
#[derive(Debug)]
pub struct MerkleChannelVerifier {
    munro_layer: u64,
    /// The munro being collected, or `NONE` after a verification.
    bin: Bin,
    tree: Tree,
    hasher: Hasher,
    timestamp: Time,
    /// `None` if the peer's signature was longer than any we support.
    signature: Option<Signature>,
    /// The munro's count of data-less chunks the peer sent with its
    /// signature, if it sent one.
    dataless: Option<u64>,
    chunk_size: usize,
}

impl MerkleChannelVerifier {
    /// Switches to the munro containing `b` unless it is the current one.
    /// A channel starts at `NONE`, so its first message always selects a
    /// munro, munro 0 with one chunk per signature included.
    pub(crate) fn select(&mut self, b: Bin) {
        let shifted = b.layer_shift(self.munro_layer);
        if self.bin == shifted {
            return;
        }
        let root = Bin::new(self.munro_layer, shifted.0 >> (self.munro_layer + 1));
        self.bin = match self.tree.reset(root) {
            Ok(()) => root,
            Err(_) => Bin::NONE,
        };
        self.signature = Some(Signature::EMPTY);
        self.dataless = None;
    }

    /// The munro's signed root and its count of data-less chunks.
    pub(crate) fn set_signed_integrity(&mut self, ts: Time, dataless: u64, sig: &[u8]) {
        self.timestamp = ts;
        self.dataless = Some(dataless);
        self.signature = Signature::from_slice(sig);
    }

    /// Records an uncle hash if it belongs to the current munro.
    pub(crate) fn set_integrity(&mut self, b: Bin, hash: &[u8]) {
        if self.bin.contains(b) {
            self.tree.set(b, hash);
        }
    }

    /// Checks `d` at `b`; on success the tree is stored in the swarm
    /// verifier, its data-less subtrees verified as zero.
    /// Either way the next message starts a new munro.
    pub(crate) fn verify(
        &mut self,
        sv: &mut MerkleSwarmVerifier,
        b: Bin,
        d: &[u8],
    ) -> Result<(), Error> {
        let r = self.verify_inner(sv, b, d);
        if let Ok(chunks) = r {
            let root = self.bin;
            for z in dataless_subtrees(root, chunks) {
                self.tree.set(z, sv.zero(z.layer()));
                self.tree.set_verified(z);
            }
            sv.ring.store(
                self.timestamp,
                &mut self.tree,
                self.signature.unwrap_or(Signature::EMPTY),
                chunks,
            );
        }
        self.bin = Bin::NONE;
        r.map(|_| ())
    }

    /// Checks `d` at `b` and returns the munro's count of chunks with data:
    /// the stored munro's, or else the one the peer signed.
    fn verify_inner(&mut self, sv: &MerkleSwarmVerifier, b: Bin, d: &[u8]) -> Result<u64, Error> {
        if !self.bin.contains(b) {
            return Err(Error::BinOutOfBounds);
        }
        let len = usize::try_from(b.base_length())
            .ok()
            .and_then(|n| n.checked_mul(self.chunk_size))
            .ok_or(Error::BinOutOfBounds)?;
        if d.len() != len {
            return Err(Error::DataLength);
        }

        let root = self.bin;
        let segment = sv.ring.segment(b);
        let chunks = match (segment, self.dataless) {
            (Some(s), _) => s.chunks,
            // Without a stored munro only a signature can verify it.
            (None, None) => return Err(Error::SignatureTooShort),
            (None, Some(z)) if z >= root.base_length() => {
                return Err(Error::InvalidChunkCount);
            }
            (None, Some(z)) => root.base_length() - z,
        };
        if dataless(root, chunks, b.base_right()) {
            return Err(Error::DataLess);
        }
        // Data-less subtrees are zero whatever the peer sent for them.
        for z in dataless_subtrees(root, chunks) {
            self.tree.set(z, sv.zero(z.layer()));
        }

        let reference = segment.and_then(|s| s.tree.as_ref());
        if self.tree.verify(&mut self.hasher, b, d, reference)? {
            return Ok(chunks);
        }

        let sig = self
            .signature
            .filter(|s| s.as_slice().len() == sv.signature_verifier.size())
            .ok_or(Error::SignatureTooShort)?;
        if !sv.signature_verifier.verify_unit(
            Domain::PpsppMunroRoot,
            self.timestamp,
            Position::with_data(root, chunks),
            self.tree.root_digest(),
            sig.as_slice(),
        ) {
            return Err(Error::InvalidSignature);
        }
        Ok(chunks)
    }
}

/// The zero digests of a munro of `layers` layers: the digest of an
/// all-zero chunk of `chunk_size` bytes, then each layer's digest of two
/// of the layer below's, laid out a digest per layer from the chunks up.
fn zero_digests(h: &mut Hasher, chunk_size: usize, layers: u64) -> Box<[u8]> {
    let hs = h.size();
    let layers = usize::try_from(layers).unwrap_or(0);
    let mut z = vec![0u8; layers.saturating_mul(hs)];
    h.update(&vec![0; chunk_size]);
    if let Some(d) = z.get_mut(..hs) {
        h.finish_into(d);
    }
    for k in 1..layers {
        let (below, here) = z.split_at_mut(k * hs);
        let prev = below.get((k - 1) * hs..).unwrap_or_default();
        h.update(prev);
        h.update(prev);
        if let Some(d) = here.get_mut(..hs) {
            h.finish_into(d);
        }
    }
    z.into_boxed_slice()
}
