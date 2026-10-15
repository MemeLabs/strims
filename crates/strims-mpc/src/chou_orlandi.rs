// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The Chou-Orlandi "simplest" oblivious transfer.

use curve25519_dalek::ristretto::{RistrettoBasepointTable, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;

use crate::Error;
use crate::aes_rng::AesRng;
use crate::block::{Block, xor_block};
use crate::channel::Channel;
use crate::common::{hash_point, point_bytes, scalar_from_rng};
use crate::ot::{OtReceiver, OtSender};

/// The Chou-Orlandi OT sender.
pub(crate) struct ChouOrlandiSender {
    y: Scalar,
    s: RistrettoPoint,
}

impl ChouOrlandiSender {
    /// Sets up by sending `S = y·B`.
    pub(crate) async fn new(conn: &mut Channel, rng: &mut AesRng) -> Result<Self, Error> {
        let y = scalar_from_rng(rng);
        let s = RistrettoPoint::mul_base(&y);
        conn.write(&point_bytes(&s));
        conn.flush();
        Ok(ChouOrlandiSender { y, s })
    }
}

impl OtSender for ChouOrlandiSender {
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        _rng: &mut AesRng,
    ) -> Result<(), Error> {
        let ys = self.s * self.y;
        let mut ks = Vec::with_capacity(inputs.len());
        for i in 0..inputs.len() {
            let r = conn.read_point().await? * self.y;
            let i = i as u64;
            ks.push([hash_point(i, &r), hash_point(i, &(r - ys))]);
        }
        for (input, k) in inputs.iter().zip(&ks) {
            conn.write(&xor_block(&k[0], &input[0]));
            conn.write(&xor_block(&k[1], &input[1]));
        }
        conn.flush();
        Ok(())
    }
}

/// The Chou-Orlandi OT receiver.
pub(crate) struct ChouOrlandiReceiver {
    s: RistrettoPoint,
    /// Precomputed multiples of `S` (about 30 KiB).
    table: Box<RistrettoBasepointTable>,
}

impl ChouOrlandiReceiver {
    /// Sets up by reading the sender's `S`.
    pub(crate) async fn new(conn: &mut Channel, _rng: &mut AesRng) -> Result<Self, Error> {
        let s = conn.read_point().await?;
        Ok(ChouOrlandiReceiver {
            s,
            table: Box::new(RistrettoBasepointTable::create(&s)),
        })
    }
}

impl OtReceiver for ChouOrlandiReceiver {
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error> {
        let mut ks = Vec::with_capacity(inputs.len());
        for (i, &b) in inputs.iter().enumerate() {
            let x = scalar_from_rng(rng);
            let c = if b {
                self.s
            } else {
                RistrettoPoint::identity()
            };
            let r = c + RistrettoPoint::mul_base(&x);
            conn.write(&point_bytes(&r));
            ks.push(hash_point(i as u64, &(&*self.table * &x)));
        }
        conn.flush();

        let mut out = Vec::with_capacity(inputs.len());
        for (&b, k) in inputs.iter().zip(&ks) {
            let c0 = conn.read_block().await?;
            let c1 = conn.read_block().await?;
            out.push(xor_block(k, if b { &c1 } else { &c0 }));
        }
        Ok(out)
    }
}
