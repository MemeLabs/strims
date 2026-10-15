// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The KOS malicious-secure oblivious transfer extension: ALSZ plus a
//! consistency check over a coin-tossed challenge.

use crate::Error;
use crate::aes_rng::AesRng;
use crate::alsz::{AlszReceiver, AlszSender};
use crate::block::{Block, ZERO_BLOCK, block_at, block_from_uint, clmul_block, xor_block};
use crate::channel::Channel;
use crate::cointoss::{cointoss_receive, cointoss_send};
use crate::common::{bools_from_bytes, bytes_from_bools};
use crate::ot::{OtReceiver, OtSender};

/// Statistical security padding, in OTs.
pub const SPP: usize = 40;

/// Rounds `m` up to a whole byte of OTs.
fn round8(m: usize) -> usize {
    m.div_ceil(8) * 8
}

/// The KOS OT sender.
pub(crate) struct KosSender {
    ot: AlszSender,
}

impl KosSender {
    /// Runs the underlying ALSZ setup.
    pub(crate) async fn new<O: OtReceiver>(
        conn: &mut Channel,
        ot: &mut O,
        rng: &mut AesRng,
    ) -> Result<Self, Error> {
        Ok(KosSender {
            ot: AlszSender::new(conn, ot, rng).await?,
        })
    }

    async fn send_setup(
        &mut self,
        conn: &mut Channel,
        m: usize,
        rng: &mut AesRng,
    ) -> Result<Vec<u8>, Error> {
        let ncols = round8(m) + 128 + SPP;
        let qs = self.ot.send_setup(conn, ncols).await?;
        let seed = rng.block();
        let seeds = cointoss_send(conn, &[seed]).await?;
        let mut chis = AesRng::new(seeds.first().unwrap_or(&ZERO_BLOCK));

        let mut check = [ZERO_BLOCK; 2];
        for i in 0..ncols {
            let chi = chis.block();
            let tmp = clmul_block(&block_at(&qs, i), &chi);
            check = [xor_block(&check[0], &tmp[0]), xor_block(&check[1], &tmp[1])];
        }

        let x = conn.read_block().await?;
        let t = [conn.read_block().await?, conn.read_block().await?];
        let tmp = clmul_block(&x, &self.ot.sb);
        check = [xor_block(&check[0], &tmp[0]), xor_block(&check[1], &tmp[1])];
        if check != t {
            return Err(Error::ConsistencyCheckFailed);
        }
        Ok(qs)
    }
}

impl OtSender for KosSender {
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        rng: &mut AesRng,
    ) -> Result<(), Error> {
        let qs = self.send_setup(conn, inputs.len(), rng).await?;
        let hash = &self.ot.hash;
        for (i, input) in inputs.iter().enumerate() {
            let q = block_at(&qs, i);
            let tweak = block_from_uint(i as u64);
            let y0 = xor_block(&hash.tccr_hash(&tweak, &q), &input[0]);
            let q1 = xor_block(&q, &self.ot.sb);
            let y1 = xor_block(&hash.tccr_hash(&tweak, &q1), &input[1]);
            conn.write(&y0);
            conn.write(&y1);
        }
        conn.flush();
        Ok(())
    }
}

/// The KOS OT receiver.
pub(crate) struct KosReceiver {
    ot: AlszReceiver,
}

impl KosReceiver {
    /// Runs the underlying ALSZ setup.
    pub(crate) async fn new<O: OtSender>(
        conn: &mut Channel,
        ot: &mut O,
        rng: &mut AesRng,
    ) -> Result<Self, Error> {
        Ok(KosReceiver {
            ot: AlszReceiver::new(conn, ot, rng).await?,
        })
    }

    async fn receive_setup(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<u8>, Error> {
        let m = round8(inputs.len());
        let mp = m + 128 + SPP;
        let mut r = bytes_from_bools(inputs);
        let mut ext = vec![0u8; (mp - m) / 8];
        rng.fill_bytes(&mut ext);
        r.extend_from_slice(&ext);

        let ts = self.ot.receive_setup(conn, &r, mp).await?;
        let seed = rng.block();
        let seeds = cointoss_receive(conn, &[seed]).await?;
        let mut chis = AesRng::new(seeds.first().unwrap_or(&ZERO_BLOCK));

        let mut x = ZERO_BLOCK;
        let mut t = [ZERO_BLOCK; 2];
        for (i, xi) in bools_from_bytes(&r).into_iter().enumerate() {
            let chi = chis.block();
            if xi {
                x = xor_block(&x, &chi);
            }
            let tmp = clmul_block(&block_at(&ts, i), &chi);
            t = [xor_block(&t[0], &tmp[0]), xor_block(&t[1], &tmp[1])];
        }

        conn.write(&x);
        conn.write(&t[0]);
        conn.write(&t[1]);
        conn.flush();
        Ok(ts)
    }
}

impl OtReceiver for KosReceiver {
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error> {
        let ts = self.receive_setup(conn, inputs, rng).await?;
        let mut out = Vec::with_capacity(inputs.len());
        for (i, &b) in inputs.iter().enumerate() {
            let y0 = conn.read_block().await?;
            let y1 = conn.read_block().await?;
            let th = self
                .ot
                .hash
                .tccr_hash(&block_from_uint(i as u64), &block_at(&ts, i));
            out.push(xor_block(if b { &y1 } else { &y0 }, &th));
        }
        Ok(out)
    }
}
