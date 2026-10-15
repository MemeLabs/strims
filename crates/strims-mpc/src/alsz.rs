// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The ALSZ semi-honest oblivious transfer extension, bootstrapped from 128
//! base OTs. The base OT is used only during setup.

use crate::Error;
use crate::aes_hash::AesHash;
use crate::aes_rng::AesRng;
use crate::block::{Block, block_at, xor_block, xor_in_place};
use crate::channel::Channel;
use crate::common::{bools_from_bytes, bytes_from_bools, pad_matrix, transpose_matrix};
use crate::ot::{OtReceiver, OtSender};

/// Rows of the extension matrix: the security parameter.
const NROWS: usize = 128;

/// The ALSZ OT sender, acting as receiver of the base OTs.
pub(crate) struct AlszSender {
    s: Vec<bool>,
    pub(crate) sb: Block,
    rngs: Vec<AesRng>,
    pub(crate) hash: AesHash,
}

impl AlszSender {
    /// Runs the base OTs with random choice bits.
    pub(crate) async fn new<O: OtReceiver>(
        conn: &mut Channel,
        ot: &mut O,
        rng: &mut AesRng,
    ) -> Result<Self, Error> {
        let sb = rng.block();
        let s = bools_from_bytes(&sb);
        let ks = ot.receive(conn, &s, rng).await?;
        Ok(AlszSender {
            s,
            sb,
            rngs: ks.iter().map(AesRng::new).collect(),
            hash: AesHash::fixed(),
        })
    }

    /// Returns the transposed `Q` matrix, one 16 byte row per
    /// OT (`pad_matrix(m, 128)` rows).
    pub(crate) async fn send_setup(
        &mut self,
        conn: &mut Channel,
        m: usize,
    ) -> Result<Vec<u8>, Error> {
        let ncols = pad_matrix(m, NROWS) / 8;
        let mut qs = vec![0u8; NROWS * ncols];
        let mut u = vec![0u8; ncols];
        for ((q, rng), &s) in qs.chunks_exact_mut(ncols).zip(&mut self.rngs).zip(&self.s) {
            conn.read_exact(&mut u).await?;
            rng.fill_bytes(q);
            if s {
                xor_in_place(q, &u);
            }
        }
        transpose_matrix(&mut qs, NROWS, ncols)?;
        Ok(qs)
    }
}

impl OtSender for AlszSender {
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        _rng: &mut AesRng,
    ) -> Result<(), Error> {
        let qs = self.send_setup(conn, inputs.len()).await?;
        for (i, input) in inputs.iter().enumerate() {
            let q = block_at(&qs, i);
            let y0 = xor_block(&self.hash.cr_hash(&q), &input[0]);
            let y1 = xor_block(&self.hash.cr_hash(&xor_block(&q, &self.sb)), &input[1]);
            conn.write(&y0);
            conn.write(&y1);
        }
        conn.flush();
        Ok(())
    }
}

/// The ALSZ OT receiver, acting as sender of the base OTs.
pub(crate) struct AlszReceiver {
    rngs: Vec<[AesRng; 2]>,
    pub(crate) hash: AesHash,
}

impl AlszReceiver {
    /// Runs the base OTs with random key pairs.
    pub(crate) async fn new<O: OtSender>(
        conn: &mut Channel,
        ot: &mut O,
        rng: &mut AesRng,
    ) -> Result<Self, Error> {
        let mut ks = Vec::with_capacity(NROWS);
        for _ in 0..NROWS {
            let k0 = rng.block();
            let k1 = rng.block();
            ks.push([k0, k1]);
        }
        ot.send(conn, &ks, rng).await?;
        Ok(AlszReceiver {
            rngs: ks
                .iter()
                .map(|k| [AesRng::new(&k[0]), AesRng::new(&k[1])])
                .collect(),
            hash: AesHash::fixed(),
        })
    }

    /// `r` holds the choice bits (most significant first) and
    /// is zero padded to the matrix width. Returns the transposed `T` matrix.
    pub(crate) async fn receive_setup(
        &mut self,
        conn: &mut Channel,
        r: &[u8],
        m: usize,
    ) -> Result<Vec<u8>, Error> {
        let ncols = pad_matrix(m, NROWS) / 8;
        let mut ts = vec![0u8; NROWS * ncols];
        let mut g = vec![0u8; ncols];
        let mut r = r.to_vec();
        r.resize(ncols, 0);
        for (t, rngs) in ts.chunks_exact_mut(ncols).zip(&mut self.rngs) {
            rngs[0].fill_bytes(t);
            rngs[1].fill_bytes(&mut g);
            xor_in_place(&mut g, t);
            xor_in_place(&mut g, &r);
            conn.write(&g);
        }
        conn.flush();
        transpose_matrix(&mut ts, NROWS, ncols)?;
        Ok(ts)
    }
}

impl OtReceiver for AlszReceiver {
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        _rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error> {
        let r = bytes_from_bools(inputs);
        let ts = self.receive_setup(conn, &r, inputs.len()).await?;
        let mut out = Vec::with_capacity(inputs.len());
        for (i, &b) in inputs.iter().enumerate() {
            let y0 = conn.read_block().await?;
            let y1 = conn.read_block().await?;
            let y = if b { y1 } else { y0 };
            out.push(xor_block(&y, &self.hash.cr_hash(&block_at(&ts, i))));
        }
        Ok(out)
    }
}
