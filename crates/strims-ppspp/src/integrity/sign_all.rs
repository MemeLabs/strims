// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-chunk signatures.
//!
//! Every chunk is signed on its own (timestamp ‖ chunk data). The swarm
//! verifier keeps a ring of a live window of timestamps and signatures,
//! indexed by chunk; peers send one `SignedIntegrity` per chunk ahead of
//! the data.
//!
//! A channel collects one consecutive run of pending signatures, at most a
//! live window of them. A signature that doesn't continue the run starts a
//! new one, and every verification clears the run, so a failed or skipped
//! chunk never misaligns later ones. Signatures for chunks before the
//! ring's tail are dropped, never stored over a live chunk's slot.

use strims_base::time::Time;
use strims_binmap::Bin;
use strims_crypto::Domain;
use strims_proto::types::{Cache, cache};

use super::{Error, SignatureSigner, SignatureVerifier, ring_size};
use crate::codec::{Message, MessageWriter, SignedIntegrity};

/// The settings of a [`SignAllSwarmVerifier`].
#[derive(Clone, Debug)]
pub struct SignAllOptions {
    /// The live window in chunks.
    pub live_discard_window: usize,
    pub chunk_size: usize,
    pub verifier: SignatureVerifier,
}

/// A sign-all swarm's chunk signatures. See the module docs.
#[derive(Debug)]
pub struct SignAllSwarmVerifier {
    mask: u64,
    /// The window in bins (twice the chunks).
    size: Bin,
    head: Bin,
    pub(super) tail: Bin,
    /// Bound: a live window of entries, allocated once.
    timestamps: Box<[Time]>,
    signatures: Box<[u8]>,
    sig_size: usize,
    chunk_size: usize,
    signature_verifier: SignatureVerifier,
}

impl SignAllSwarmVerifier {
    /// The window must be a power of two.
    pub fn new(o: &SignAllOptions) -> Result<SignAllSwarmVerifier, Error> {
        let window = ring_size(o.live_discard_window)?;
        if o.chunk_size == 0 {
            return Err(Error::InvalidOptions);
        }
        let sig_size = o.verifier.size();
        Ok(SignAllSwarmVerifier {
            mask: window - 1,
            size: Bin(window * 2),
            head: Bin(0),
            tail: Bin(0),
            timestamps: vec![Time::EPOCH; o.live_discard_window].into_boxed_slice(),
            signatures: vec![0; o.live_discard_window * sig_size].into_boxed_slice(),
            sig_size,
            chunk_size: o.chunk_size,
            signature_verifier: o.verifier.clone(),
        })
    }

    fn slot(&self, b: Bin) -> usize {
        ((b.0 >> 1) & self.mask) as usize
    }

    fn signature(&self, i: usize) -> &[u8] {
        let s = self.sig_size;
        self.signatures.get(i * s..(i + 1) * s).unwrap_or_default()
    }

    /// One `SignedIntegrity` per chunk of `b`.
    pub fn write_integrity(&self, b: Bin, w: &mut impl MessageWriter) -> Result<usize, Error> {
        if !crate::peer_bin_ok(b, u64::MAX) {
            return Err(Error::MissingChunkSignature);
        }
        let mut n = 0;
        let (mut l, r) = (b.base_left(), b.base_right());
        while l <= r {
            if l < self.tail || l >= self.head {
                return Err(Error::MissingChunkSignature);
            }
            let i = self.slot(l);
            n += w.write(&Message::SignedIntegrity(SignedIntegrity {
                bin: l,
                timestamp: self.timestamps.get(i).copied().unwrap_or(Time::EPOCH),
                signature: self.signature(i),
            }))?;
            l += 2;
        }
        Ok(n)
    }

    /// Stores the signature of chunk `b`, advancing the ring. Chunks before
    /// the tail are ignored: their slot belongs to a live chunk.
    pub(crate) fn store_signature(&mut self, b: Bin, ts: Time, sig: &[u8]) {
        if b < self.tail {
            return;
        }
        if b >= self.head {
            self.head = b + 2;
            if self.head > self.size {
                self.tail = self.head - self.size.0;
            }
        }
        let i = self.slot(b);
        let s = self.sig_size;
        if let Some(t) = self.timestamps.get_mut(i) {
            *t = ts;
        }
        if let Some(dst) = self.signatures.get_mut(i * s..(i + 1) * s)
            && dst.len() == sig.len()
        {
            dst.copy_from_slice(sig);
        }
    }

    /// Signs and stores the chunk at `b` from local data. Publishing the
    /// chunk is the caller's job.
    pub(crate) fn write_chunk(&mut self, signer: &SignatureSigner, ts: Time, b: Bin, chunk: &[u8]) {
        let sig = signer.sign(Domain::PpsppChunk, ts, chunk);
        self.store_signature(b, ts, sig.as_slice());
    }

    /// A verifier for one peer channel.
    pub fn channel_verifier(&self) -> SignAllChannelVerifier {
        SignAllChannelVerifier {
            bin: Bin::NONE,
            timestamps: Vec::new(),
            signatures: Vec::new(),
            max_pending: self.timestamps.len(),
            sig_size: self.sig_size,
            chunk_size: self.chunk_size,
        }
    }

    /// Loads and verifies a cache. The head moves past the imported chunks,
    /// so their signatures can be sent. Returns how many bytes of the data
    /// the imported signatures cover.
    pub fn import_cache(&mut self, c: &Cache) -> Result<usize, Error> {
        let ic = c
            .integrity
            .as_ref()
            .and_then(|i| i.sign_all_integrity.as_ref())
            .ok_or(Error::NoIntegrityCache)?;

        let n = ic.timestamps.len();
        if n > self.timestamps.len() {
            return Err(Error::CacheTooLarge);
        }
        if ic.signatures.len() < n * self.sig_size {
            return Err(Error::IntegrityCacheIncomplete);
        }

        let (cs, ss) = (self.chunk_size, self.sig_size);
        for (i, &t) in ic.timestamps.iter().enumerate() {
            let chunk = c
                .data
                .get(i * cs..(i + 1) * cs)
                .ok_or(Error::IntegrityCacheIncomplete)?;
            let sig = ic
                .signatures
                .get(i * ss..(i + 1) * ss)
                .ok_or(Error::IntegrityCacheIncomplete)?;
            if !self.signature_verifier.verify(
                Domain::PpsppChunk,
                Time::from_unix_nanos(t),
                chunk,
                sig,
            ) {
                return Err(Error::InvalidChunkSignature);
            }
        }

        for (dst, &t) in self.timestamps.iter_mut().zip(&ic.timestamps) {
            *dst = Time::from_unix_nanos(t);
        }
        if let (Some(dst), Some(src)) = (
            self.signatures.get_mut(..n * ss),
            ic.signatures.get(..n * ss),
        ) {
            dst.copy_from_slice(src);
        }
        self.head = Bin(2 * n as u64);
        self.tail = Bin(0);
        Ok(n * cs)
    }

    /// The timestamps and signatures from slot 0 up to the first zero
    /// timestamp or the head, so a reset ring exports nothing stale.
    pub fn export_cache(&self) -> cache::Integrity {
        let mut c = cache::SignAllIntegrity::default();
        let filled = usize::try_from(self.head.0 / 2).unwrap_or(usize::MAX);
        for t in self.timestamps.iter().take(filled) {
            if *t == Time::EPOCH {
                break;
            }
            c.timestamps.push(t.unix_nanos());
        }
        c.signatures = self
            .signatures
            .get(..c.timestamps.len() * self.sig_size)
            .unwrap_or_default()
            .to_vec();
        cache::Integrity {
            sign_all_integrity: Some(c),
            ..Default::default()
        }
    }

    /// Forgets every signature.
    pub fn reset(&mut self) {
        self.head = Bin(0);
        self.tail = Bin(0);
    }
}

/// The run of chunk signatures a peer sent ahead of its data.
#[derive(Debug)]
pub struct SignAllChannelVerifier {
    /// The first chunk of the pending run, or `NONE`.
    bin: Bin,
    /// Bound: `max_pending` entries (a live window).
    pub(super) timestamps: Vec<Time>,
    signatures: Vec<u8>,
    max_pending: usize,
    sig_size: usize,
    chunk_size: usize,
}

impl SignAllChannelVerifier {
    fn clear(&mut self) {
        self.bin = Bin::NONE;
        self.timestamps.clear();
        self.signatures.clear();
    }

    /// Appends a chunk signature to the pending run. A signature that
    /// doesn't continue the run, or would grow it past a window, starts a
    /// new one.
    pub(crate) fn set_signed_integrity(&mut self, b: Bin, ts: Time, sig: &[u8]) {
        if !b.is_base() || b.0 >= Bin::ALL.0 || sig.len() != self.sig_size {
            self.clear();
            return;
        }
        let n = self.timestamps.len() as u64;
        if self.bin.is_none() || b != self.bin + 2 * n || self.timestamps.len() >= self.max_pending
        {
            self.clear();
            self.bin = b;
        }
        self.timestamps.push(ts);
        self.signatures.extend_from_slice(sig);
    }

    /// Checks `d` at `b` against the pending run and stores its signatures
    /// in the swarm verifier. The run is cleared afterwards whether or not
    /// verification succeeds.
    pub(crate) fn verify(
        &mut self,
        sv: &mut SignAllSwarmVerifier,
        b: Bin,
        d: &[u8],
    ) -> Result<(), Error> {
        let r = self.verify_inner(sv, b, d);
        self.clear();
        r
    }

    fn verify_inner(
        &mut self,
        sv: &mut SignAllSwarmVerifier,
        b: Bin,
        d: &[u8],
    ) -> Result<(), Error> {
        if !crate::peer_bin_ok(b, self.max_pending as u64) {
            return Err(Error::BinOutOfBounds);
        }
        let bl = b.base_left();
        if self.bin.is_none() || bl < self.bin {
            return Err(Error::MissingChunkSignature);
        }
        let off =
            usize::try_from((bl.0 - self.bin.0) / 2).map_err(|_| Error::MissingChunkSignature)?;
        let l = usize::try_from(b.base_length()).map_err(|_| Error::MissingChunkSignature)?;
        if off.saturating_add(l) > self.timestamps.len() {
            return Err(Error::MissingChunkSignature);
        }
        let (cs, ss) = (self.chunk_size, self.sig_size);
        if d.len() != l.saturating_mul(cs) {
            return Err(Error::DataLength);
        }

        for i in 0..l {
            let ts = self.timestamps.get(off + i).copied().unwrap_or(Time::EPOCH);
            let chunk = d.get(i * cs..(i + 1) * cs).unwrap_or_default();
            let sig = self
                .signatures
                .get((off + i) * ss..(off + i + 1) * ss)
                .unwrap_or_default();
            if !sv
                .signature_verifier
                .verify(Domain::PpsppChunk, ts, chunk, sig)
            {
                return Err(Error::InvalidChunkSignature);
            }
            sv.store_signature(bl + 2 * i as u64, ts, sig);
        }
        Ok(())
    }
}
