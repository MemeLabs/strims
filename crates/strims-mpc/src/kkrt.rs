// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The KKRT batched oblivious PRF, built on 512 OTs of a
//! [`PseudorandomCode`] whose keys are coin-tossed. The OT is used only
//! during setup.

use crate::Error;
use crate::aes_rng::AesRng;
use crate::block::{Block, Block512, and_in_place, xor_in_place};
use crate::channel::Channel;
use crate::cointoss::{cointoss_receive, cointoss_send};
use crate::common::{bools_from_bytes, pad_matrix, transpose_matrix};
use crate::ot::{OtReceiver, OtSender};
use crate::prc::PseudorandomCode;

/// Width of the code in bits: the number of base OTs.
const NCOLS: usize = 512;

const _: () = assert!(NCOLS / 8 == size_of::<Block512>());

/// Rows of the OPRF matrix for `m` inputs: `m` rounded up to 16, then padded
/// so [`transpose_matrix`] supports a `rows` x 512 matrix.
fn matrix_rows(m: usize) -> usize {
    pad_matrix(m.div_ceil(16) * 16, NCOLS)
}

/// The part of a KKRT sender that evaluates the PRF: the code and the
/// sender's secret choice bits.
#[derive(Clone, Debug)]
pub struct KkrtEncoder {
    code: PseudorandomCode,
    sb: Block512,
}

impl KkrtEncoder {
    /// `C(src) ∧ s`.
    pub fn encode(&self, src: &Block) -> Block512 {
        let mut dst = self.code.encode(src);
        and_in_place(&mut dst, &self.sb);
        dst
    }

    /// The PRF output for `src` under the key `seed` from
    /// [`KkrtSenderOutput::seeds`](crate::KkrtSenderOutput).
    pub fn compute(&self, seed: &Block512, src: &Block) -> Block512 {
        let mut dst = self.encode(src);
        xor_in_place(&mut dst, seed);
        dst
    }
}

/// The KKRT OPRF sender.
pub(crate) struct KkrtSender {
    pub(crate) encoder: KkrtEncoder,
    s: Vec<bool>,
    rngs: Vec<AesRng>,
}

impl KkrtSender {
    /// Coin-tosses the code keys, then runs 512 OTs as receiver.
    pub(crate) async fn new<O: OtReceiver>(
        conn: &mut Channel,
        ot: &mut O,
        rng: &mut AesRng,
    ) -> Result<Self, Error> {
        let mut sb = [0u8; 64];
        rng.fill_bytes(&mut sb);
        let s = bools_from_bytes(&sb);
        let seeds: [Block; 4] = core::array::from_fn(|_| rng.block());
        let keys = cointoss_send(conn, &seeds).await?;
        let keys: [Block; 4] = keys.try_into().map_err(|_| Error::Internal)?;
        let code = PseudorandomCode::new(&keys);

        let ks = ot.receive(conn, &s, rng).await?;
        Ok(KkrtSender {
            encoder: KkrtEncoder { code, sb },
            s,
            rngs: ks.iter().map(AesRng::new).collect(),
        })
    }

    /// Returns one PRF key per input slot.
    pub(crate) async fn send(
        &mut self,
        conn: &mut Channel,
        m: usize,
    ) -> Result<Vec<Block512>, Error> {
        let nrows = matrix_rows(m);
        let row = nrows / 8;
        let mut t0 = vec![0u8; row];
        let mut t1 = vec![0u8; row];
        let mut qs = vec![0u8; nrows * NCOLS / 8];
        for ((q, rng), &b) in qs.chunks_exact_mut(row).zip(&mut self.rngs).zip(&self.s) {
            rng.fill_bytes(q);
            conn.read_exact(&mut t0).await?;
            conn.read_exact(&mut t1).await?;
            xor_in_place(q, if b { &t1 } else { &t0 });
        }
        transpose_matrix(&mut qs, NCOLS, row)?;

        Ok(qs.as_chunks::<64>().0.iter().take(m).copied().collect())
    }
}

/// The KKRT OPRF receiver.
pub(crate) struct KkrtReceiver {
    code: PseudorandomCode,
    rngs: Vec<[AesRng; 2]>,
}

impl KkrtReceiver {
    /// Coin-tosses the code keys, then runs 512 OTs as sender.
    pub(crate) async fn new<O: OtSender>(
        conn: &mut Channel,
        ot: &mut O,
        rng: &mut AesRng,
    ) -> Result<Self, Error> {
        let seeds: [Block; 4] = core::array::from_fn(|_| rng.block());
        let keys = cointoss_receive(conn, &seeds).await?;
        let keys: [Block; 4] = keys.try_into().map_err(|_| Error::Internal)?;
        let code = PseudorandomCode::new(&keys);

        let mut ks = Vec::with_capacity(NCOLS);
        for _ in 0..NCOLS {
            let k0 = rng.block();
            let k1 = rng.block();
            ks.push([k0, k1]);
        }
        ot.send(conn, &ks, rng).await?;
        Ok(KkrtReceiver {
            code,
            rngs: ks
                .iter()
                .map(|k| [AesRng::new(&k[0]), AesRng::new(&k[1])])
                .collect(),
        })
    }

    /// The PRF output for each input.
    pub(crate) async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[Block],
        rng: &mut AesRng,
    ) -> Result<Vec<Block512>, Error> {
        let m = inputs.len();
        let nrows = matrix_rows(m);
        let row = nrows / 8;

        let mut t0s = vec![0u8; nrows * NCOLS / 8];
        rng.fill_bytes(&mut t0s);
        let out = t0s.as_chunks::<64>().0.iter().take(m).copied().collect();
        // TODO: transpose `t0s` in place and derive `t1` row by row; the clone
        // doubles peak memory.
        let mut t1s = t0s.clone();
        for (t1, input) in t1s.as_chunks_mut::<64>().0.iter_mut().zip(inputs) {
            xor_in_place(t1, &self.code.encode(input));
        }
        transpose_matrix(&mut t0s, nrows, NCOLS / 8)?;
        transpose_matrix(&mut t1s, nrows, NCOLS / 8)?;

        let mut t = vec![0u8; row];
        for ((t0, t1), rngs) in t0s
            .chunks_exact(row)
            .zip(t1s.chunks_exact(row))
            .zip(&mut self.rngs)
        {
            rngs[0].fill_bytes(&mut t);
            xor_in_place(&mut t, t0);
            conn.write(&t);
            rngs[1].fill_bytes(&mut t);
            xor_in_place(&mut t, t1);
            conn.write(&t);
        }
        conn.flush();
        Ok(out)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn matrix_rows_are_transposable() {
        assert_eq!(matrix_rows(0), 8);
        for m in [
            1, 15, 16, 17, 100, 255, 256, 257, 511, 512, 513, 1370, 90000,
        ] {
            let rows = matrix_rows(m);
            assert!(rows >= m && rows.is_multiple_of(16), "{m} -> {rows}");
            assert!(
                rows.is_power_of_two() || rows.is_multiple_of(NCOLS),
                "{m} -> {rows}"
            );
        }
    }
}
