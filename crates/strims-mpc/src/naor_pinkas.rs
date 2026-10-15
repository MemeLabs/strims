// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The Naor-Pinkas oblivious transfer protocol (cf. <https://dl.acm.org/citation.cfm?id=365502>).

use curve25519_dalek::ristretto::RistrettoPoint;

use crate::Error;
use crate::aes_rng::AesRng;
use crate::block::{Block, xor_block};
use crate::channel::Channel;
use crate::common::{hash_point, point_bytes, point_from_rng, scalar_from_rng};
use crate::ot::{OtReceiver, OtSender};

/// The Naor-Pinkas OT sender.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NaorPinkasSender;

impl OtSender for NaorPinkasSender {
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        rng: &mut AesRng,
    ) -> Result<(), Error> {
        let mut cs = Vec::with_capacity(inputs.len());
        for _ in inputs {
            let c = point_from_rng(rng);
            conn.write(&point_bytes(&c));
            cs.push(c);
        }
        conn.flush();

        let mut pks: Vec<[RistrettoPoint; 2]> = Vec::with_capacity(inputs.len());
        for c in &cs {
            let pk0 = conn.read_point().await?;
            pks.push([pk0, c - pk0]);
        }

        for (i, (input, pk)) in inputs.iter().zip(&pks).enumerate() {
            let i = i as u64;
            let r0 = scalar_from_rng(rng);
            let r1 = scalar_from_rng(rng);
            let e01 = xor_block(&input[0], &hash_point(i, &(pk[0] * r0)));
            let e11 = xor_block(&input[1], &hash_point(i, &(pk[1] * r1)));
            let e00 = RistrettoPoint::mul_base(&r0);
            let e10 = RistrettoPoint::mul_base(&r1);
            conn.write(&point_bytes(&e00));
            conn.write(&e01);
            conn.write(&point_bytes(&e10));
            conn.write(&e11);
        }
        conn.flush();
        Ok(())
    }
}

/// The Naor-Pinkas OT receiver.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NaorPinkasReceiver;

impl OtReceiver for NaorPinkasReceiver {
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error> {
        let mut cs = Vec::with_capacity(inputs.len());
        for _ in inputs {
            cs.push(conn.read_point().await?);
        }

        let mut ks = Vec::with_capacity(inputs.len());
        for (&b, c) in inputs.iter().zip(&cs) {
            let k = scalar_from_rng(rng);
            let pk = RistrettoPoint::mul_base(&k);
            if b {
                conn.write(&point_bytes(&(c - pk)));
            } else {
                conn.write(&point_bytes(&pk));
            }
            ks.push(k);
        }
        conn.flush();

        let mut out = Vec::with_capacity(inputs.len());
        for (i, (&b, k)) in inputs.iter().zip(&ks).enumerate() {
            let e00 = conn.read_point().await?;
            let e01 = conn.read_block().await?;
            let e10 = conn.read_point().await?;
            let e11 = conn.read_block().await?;
            let (e0, e1) = if b { (e10, e11) } else { (e00, e01) };
            out.push(xor_block(&hash_point(i as u64, &(e0 * k)), &e1));
        }
        Ok(out)
    }
}
