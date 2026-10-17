// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! PSZ private set intersection over the KKRT OPRF, composed as the network
//! broker runs it: Chou-Orlandi base OTs, KOS extension, KKRT OPRF, PSZ.
//!
//! The sender learns only the receiver's set size; the receiver learns which
//! of its own elements the sender also holds.

use std::collections::BTreeSet;

use rand_core::Rng;
use sha2::{Digest, Sha256};
use strims_base::rng::DetRng;

use crate::Error;
use crate::aes_hash::AesHash;
use crate::aes_rng::AesRng;
use crate::block::{Block, ZERO_BLOCK, and_block, block_from_uint, xor_block, xor_in_place};
use crate::channel::Channel;
use crate::chou_orlandi::{ChouOrlandiReceiver, ChouOrlandiSender};
use crate::cointoss::{cointoss_receive, cointoss_send};
use crate::cuckoo::{
    CUCKOO_MASK, CuckooHashMap, MAX_CUCKOO_SET_SIZE, compute_mask_size, compute_nbins,
    cuckoo_hash_bin,
};
use crate::kkrt::{KkrtReceiver, KkrtSender};
use crate::kos::{KosReceiver, KosSender};
use crate::session::{Endpoint, Session, finish};

/// Hash functions of the receiver's cuckoo table.
const NHASHES: usize = 3;

/// The largest set size accepted from a peer ([`Error::PeerSetTooLarge`]).
///
/// The peer's announced size sizes the KKRT matrices, so this bounds what
/// one forged `u32` can make us allocate. At this bound the largest
/// allocation (the KKRT matrix) is about 6 MiB; real peers hold a handful of
/// network keys.
pub const MAX_PEER_SET_SIZE: usize = 1 << 16;

/// The largest mask PSZ sends, in bytes.
const MAX_MASK_SIZE: usize = 12;

/// Inputs shorter than a block are zero padded, longer ones replaced by a
/// truncated SHA-256, then hashed under the coin-tossed key with the
/// hash-index byte cleared.
///
/// Zero padding makes short inputs that differ only in trailing zero bytes
/// (`ab` and `ab\0`) collide. Broker inputs are 32-byte network keys, so
/// they always take the SHA-256 path.
pub(crate) fn compress_and_hash_inputs(inputs: &[Vec<u8>], key: &Block) -> Vec<Block> {
    let aes = AesHash::new(key);
    inputs
        .iter()
        .map(|input| {
            let mut tmp = ZERO_BLOCK;
            if input.len() < 16 {
                if let Some(dst) = tmp.get_mut(..input.len()) {
                    dst.copy_from_slice(input);
                }
            } else {
                let digest = Sha256::digest(input);
                if let Some(src) = digest.get(..16) {
                    tmp.copy_from_slice(src);
                }
            }
            and_block(&aes.cr_hash(&tmp), &CUCKOO_MASK)
        })
        .collect()
}

/// A uniform index below `n`, by rejection sampling.
fn uniform(rng: &mut AesRng, n: usize) -> usize {
    let n = n as u64;
    let limit = u64::MAX - u64::MAX % n;
    loop {
        let mut b = [0u8; 8];
        rng.fill_bytes(&mut b);
        let v = u64::from_le_bytes(b);
        if v < limit {
            return (v % n) as usize;
        }
    }
}

/// Fisher-Yates shuffle driven by the protocol rng. It hides which element
/// each mask belongs to; the receiver reads each hash function's masks as a
/// set, so any permutation is valid on the wire.
fn shuffle(ns: &mut [usize], rng: &mut AesRng) {
    for i in (1..ns.len()).rev() {
        let j = uniform(rng, i + 1);
        ns.swap(i, j);
    }
}

/// Reads the peer's set size, rejecting sizes above `max`.
async fn read_peer_size(conn: &mut Channel, max: usize) -> Result<usize, Error> {
    let pn = conn.read_int().await? as usize;
    if pn > max {
        return Err(Error::PeerSetTooLarge { size: pn, max });
    }
    Ok(pn)
}

fn own_size(inputs: &[Vec<u8>]) -> Result<u32, Error> {
    if inputs.len() > MAX_CUCKOO_SET_SIZE {
        return Err(Error::InvalidCuckooSetSize);
    }
    u32::try_from(inputs.len()).map_err(|_| Error::InvalidCuckooSetSize)
}

async fn psz_send(
    conn: &mut Channel,
    oprf: &mut KkrtSender,
    inputs: &[Vec<u8>],
    max_peer_set: usize,
    rng: &mut AesRng,
) -> Result<(), Error> {
    conn.write_int(own_size(inputs)?);
    conn.flush();
    let pn = read_peer_size(conn, max_peer_set).await?;
    let n = inputs.len().max(pn);

    let keys = cointoss_send(conn, &[rng.block()]).await?;
    let digests = compress_and_hash_inputs(inputs, keys.first().unwrap_or(&ZERO_BLOCK));

    let nbins = compute_nbins(pn, NHASHES)?;
    let mask_size = compute_mask_size(n)?;
    let seeds = oprf.send(conn, nbins).await?;

    let mut indices: Vec<usize> = (0..inputs.len()).collect();
    for i in 0..NHASHES {
        shuffle(&mut indices, rng);
        let hidx = block_from_uint(i as u64);
        for &j in &indices {
            let digest = digests.get(j).ok_or(Error::Internal)?;
            let bin = cuckoo_hash_bin(digest, i, nbins);
            let seed = seeds.get(bin).ok_or(Error::Internal)?;
            let mut encoded = oprf.encoder.encode(&xor_block(digest, &hidx));
            xor_in_place(&mut encoded, seed);
            conn.write(encoded.get(..mask_size).ok_or(Error::Internal)?);
        }
    }
    conn.flush();
    Ok(())
}

async fn psz_receive(
    conn: &mut Channel,
    oprf: &mut KkrtReceiver,
    inputs: &[Vec<u8>],
    max_peer_set: usize,
    rng: &mut AesRng,
) -> Result<Vec<Vec<u8>>, Error> {
    let own = own_size(inputs)?;
    let pn = read_peer_size(conn, max_peer_set).await?;
    conn.write_int(own);
    conn.flush();
    let n = inputs.len().max(pn);
    let mask_size = compute_mask_size(n)?;
    if mask_size > MAX_MASK_SIZE {
        return Err(Error::Internal);
    }

    let keys = cointoss_receive(conn, &[rng.block()]).await?;
    let digests = compress_and_hash_inputs(inputs, keys.first().unwrap_or(&ZERO_BLOCK));
    // A table that can't be built still runs the rest of the protocol with
    // every bin empty, so the sender finishes and neither side leaves bytes
    // unread on the stream.
    let table = match CuckooHashMap::new(&digests, NHASHES) {
        Ok(table) => Some(table),
        Err(Error::CuckooHashMapFull) => None,
        Err(e) => return Err(e),
    };
    let oprf_inputs: Vec<Block> = match &table {
        Some(table) => table
            .items()
            .iter()
            .map(|item| item.map_or(ZERO_BLOCK, |item| item.entry))
            .collect(),
        None => vec![ZERO_BLOCK; compute_nbins(inputs.len(), NHASHES)?],
    };
    let outputs = oprf.receive(conn, &oprf_inputs, rng).await?;

    // One set of the sender's masks per hash function; at most
    // MAX_PEER_SET_SIZE entries each.
    // TODO: a sorted `Vec` with binary search would be cheaper.
    let mut hs: [BTreeSet<[u8; MAX_MASK_SIZE]>; NHASHES] = Default::default();
    for h in &mut hs {
        for _ in 0..pn {
            let mut prefix = [0u8; MAX_MASK_SIZE];
            conn.read_exact(prefix.get_mut(..mask_size).ok_or(Error::Internal)?)
                .await?;
            h.insert(prefix);
        }
    }
    let Some(table) = table else {
        return Err(Error::CuckooHashMapFull);
    };

    let mut intersection = Vec::new();
    for (item, output) in table.items().iter().zip(&outputs) {
        let Some(item) = item else {
            continue;
        };
        let mut prefix = [0u8; MAX_MASK_SIZE];
        let (dst, src) = prefix
            .get_mut(..mask_size)
            .zip(output.get(..mask_size))
            .ok_or(Error::Internal)?;
        dst.copy_from_slice(src);
        let found = hs.get(item.hash_index).is_some_and(|h| h.contains(&prefix));
        if found {
            intersection.push(inputs.get(item.input_index).ok_or(Error::Internal)?.clone());
        }
    }
    Ok(intersection)
}

fn seed_from(rng: &mut DetRng) -> Block {
    let mut seed = ZERO_BLOCK;
    rng.fill_bytes(&mut seed);
    seed
}

/// The PSZ sender: reveals nothing and learns nothing but the receiver's set
/// size.
#[derive(Debug)]
pub struct PszSender(Session<()>);

impl PszSender {
    /// Starts the sender with its set and the seed of the protocol's
    /// [`AesRng`], which must be secret and fresh. A set of more than 2^28
    /// elements fails with [`Error::InvalidCuckooSetSize`] before anything
    /// is sent.
    pub fn new(inputs: Vec<Vec<u8>>, seed: Block) -> PszSender {
        PszSender::with_peer_limit(inputs, seed, MAX_PEER_SET_SIZE)
    }

    /// Like [`PszSender::new`], failing with [`Error::PeerSetTooLarge`] if
    /// the receiver announces more than `max_peer_set` elements (at most
    /// [`MAX_PEER_SET_SIZE`]).
    pub fn with_peer_limit(inputs: Vec<Vec<u8>>, seed: Block, max_peer_set: usize) -> PszSender {
        let max_peer_set = max_peer_set.min(MAX_PEER_SET_SIZE);
        PszSender(Session::start(move |conn| async move {
            let mut conn = conn;
            let mut rng = AesRng::new(&seed);
            let result = async {
                let mut ot = ChouOrlandiSender::new(&mut conn, &mut rng).await?;
                let mut ote = KosReceiver::new(&mut conn, &mut ot, &mut rng).await?;
                let mut oprf = KkrtSender::new(&mut conn, &mut ote, &mut rng).await?;
                psz_send(&mut conn, &mut oprf, &inputs, max_peer_set, &mut rng).await
            }
            .await;
            finish(&mut conn, result)
        }))
    }

    /// Like [`PszSender::new`], drawing the seed from `rng`.
    pub fn from_rng(inputs: Vec<Vec<u8>>, rng: &mut DetRng) -> PszSender {
        PszSender::new(inputs, seed_from(rng))
    }

    /// See [`Session::handle_input`].
    pub fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        self.0.handle_input(data)
    }

    /// See [`Session::poll_transmit`].
    pub fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.0.poll_transmit(buf)
    }

    /// See [`Session::is_done`].
    pub fn is_done(&self) -> bool {
        self.0.is_done()
    }

    /// `Ok(())` once the sender has finished, or why it failed.
    pub fn result(&self) -> Option<Result<(), Error>> {
        self.0.result().map(|r| r.copied().map_err(Clone::clone))
    }
}

impl Endpoint for PszSender {
    fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        PszSender::handle_input(self, data)
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        PszSender::poll_transmit(self, buf)
    }

    fn is_done(&self) -> bool {
        PszSender::is_done(self)
    }
}

/// The PSZ receiver: learns the intersection.
#[derive(Debug)]
pub struct PszReceiver(Session<Vec<Vec<u8>>>);

impl PszReceiver {
    /// Starts the receiver with its set and the seed of the protocol's
    /// [`AesRng`], which must be secret and fresh. A set of more than 2^28
    /// elements fails with [`Error::InvalidCuckooSetSize`] before anything
    /// is sent.
    ///
    /// If its cuckoo table can't be built, the receiver still completes the
    /// run, learning nothing, and then fails with
    /// [`Error::CuckooHashMapFull`]; the sender succeeds.
    pub fn new(inputs: Vec<Vec<u8>>, seed: Block) -> PszReceiver {
        PszReceiver::with_peer_limit(inputs, seed, MAX_PEER_SET_SIZE)
    }

    /// Like [`PszReceiver::new`], failing with [`Error::PeerSetTooLarge`]
    /// if the sender announces more than `max_peer_set` elements (at most
    /// [`MAX_PEER_SET_SIZE`]).
    pub fn with_peer_limit(inputs: Vec<Vec<u8>>, seed: Block, max_peer_set: usize) -> PszReceiver {
        let max_peer_set = max_peer_set.min(MAX_PEER_SET_SIZE);
        PszReceiver(Session::start(move |conn| async move {
            let mut conn = conn;
            let mut rng = AesRng::new(&seed);
            let result = async {
                let mut ot = ChouOrlandiReceiver::new(&mut conn, &mut rng).await?;
                let mut ote = KosSender::new(&mut conn, &mut ot, &mut rng).await?;
                let mut oprf = KkrtReceiver::new(&mut conn, &mut ote, &mut rng).await?;
                psz_receive(&mut conn, &mut oprf, &inputs, max_peer_set, &mut rng).await
            }
            .await;
            finish(&mut conn, result)
        }))
    }

    /// Like [`PszReceiver::new`], drawing the seed from `rng`.
    pub fn from_rng(inputs: Vec<Vec<u8>>, rng: &mut DetRng) -> PszReceiver {
        PszReceiver::new(inputs, seed_from(rng))
    }

    /// See [`Session::handle_input`].
    pub fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        self.0.handle_input(data)
    }

    /// See [`Session::poll_transmit`].
    pub fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.0.poll_transmit(buf)
    }

    /// See [`Session::is_done`].
    pub fn is_done(&self) -> bool {
        self.0.is_done()
    }

    /// The receiver's elements that the sender also holds, in cuckoo-table
    /// order, once finished.
    pub fn result(&self) -> Option<Result<&[Vec<u8>], &Error>> {
        self.0.result().map(|r| r.map(Vec::as_slice))
    }

    /// Takes the intersection out of a finished receiver.
    pub fn take_result(&mut self) -> Option<Result<Vec<Vec<u8>>, Error>> {
        self.0.take_result()
    }
}

impl Endpoint for PszReceiver {
    fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        PszReceiver::handle_input(self, data)
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        PszReceiver::poll_transmit(self, buf)
    }

    fn is_done(&self) -> bool {
        PszReceiver::is_done(self)
    }
}
