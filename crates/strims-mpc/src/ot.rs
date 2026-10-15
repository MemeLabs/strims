// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The OT sender and receiver traits, the runtime choice of OT protocol
//! ([`OtKind`]), and sans-IO sessions that run an OT or the KKRT OPRF on its
//! own.

use crate::Error;
use crate::aes_rng::AesRng;
use crate::alsz::{AlszReceiver, AlszSender};
use crate::block::{Block, Block512};
use crate::channel::Channel;
use crate::chou_orlandi::{ChouOrlandiReceiver, ChouOrlandiSender};
use crate::kkrt::{KkrtEncoder, KkrtReceiver, KkrtSender};
use crate::kos::{KosReceiver, KosSender};
use crate::naor_pinkas::{NaorPinkasReceiver, NaorPinkasSender};
use crate::session::{Session, finish};

/// The sending side of an oblivious transfer.
pub(crate) trait OtSender {
    /// Transfers one block of each pair, chosen by the receiver.
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        rng: &mut AesRng,
    ) -> Result<(), Error>;
}

/// The receiving side of an oblivious transfer.
pub(crate) trait OtReceiver {
    /// Learns `inputs[i] ? m1 : m0` of each pair.
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error>;
}

/// A base oblivious transfer protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseOt {
    /// Naor-Pinkas: no setup, and fresh randomness per
    /// OT on both sides.
    NaorPinkas,
    /// Chou-Orlandi: the sender sends one point at
    /// setup. The broker uses this one.
    ChouOrlandi,
}

/// An oblivious transfer protocol: a base OT, or an extension over one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtKind {
    /// The base OT alone.
    Base(BaseOt),
    /// ALSZ semi-honest extension.
    Alsz(BaseOt),
    /// KOS malicious extension. The broker uses `Kos(ChouOrlandi)`.
    Kos(BaseOt),
}

enum AnyBaseSender {
    NaorPinkas(NaorPinkasSender),
    ChouOrlandi(ChouOrlandiSender),
}

impl AnyBaseSender {
    async fn new(kind: BaseOt, conn: &mut Channel, rng: &mut AesRng) -> Result<Self, Error> {
        Ok(match kind {
            BaseOt::NaorPinkas => AnyBaseSender::NaorPinkas(NaorPinkasSender),
            BaseOt::ChouOrlandi => {
                AnyBaseSender::ChouOrlandi(ChouOrlandiSender::new(conn, rng).await?)
            }
        })
    }
}

impl OtSender for AnyBaseSender {
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        rng: &mut AesRng,
    ) -> Result<(), Error> {
        match self {
            AnyBaseSender::NaorPinkas(ot) => ot.send(conn, inputs, rng).await,
            AnyBaseSender::ChouOrlandi(ot) => ot.send(conn, inputs, rng).await,
        }
    }
}

enum AnyBaseReceiver {
    NaorPinkas(NaorPinkasReceiver),
    ChouOrlandi(ChouOrlandiReceiver),
}

impl AnyBaseReceiver {
    async fn new(kind: BaseOt, conn: &mut Channel, rng: &mut AesRng) -> Result<Self, Error> {
        Ok(match kind {
            BaseOt::NaorPinkas => AnyBaseReceiver::NaorPinkas(NaorPinkasReceiver),
            BaseOt::ChouOrlandi => {
                AnyBaseReceiver::ChouOrlandi(ChouOrlandiReceiver::new(conn, rng).await?)
            }
        })
    }
}

impl OtReceiver for AnyBaseReceiver {
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error> {
        match self {
            AnyBaseReceiver::NaorPinkas(ot) => ot.receive(conn, inputs, rng).await,
            AnyBaseReceiver::ChouOrlandi(ot) => ot.receive(conn, inputs, rng).await,
        }
    }
}

/// The sending side of any [`OtKind`].
enum AnyOtSender {
    Base(AnyBaseSender),
    Alsz(AlszSender),
    Kos(KosSender),
}

impl AnyOtSender {
    /// Runs the setup of `kind`'s sender (for an extension, as the receiver
    /// of its base OTs).
    async fn new(kind: OtKind, conn: &mut Channel, rng: &mut AesRng) -> Result<Self, Error> {
        Ok(match kind {
            OtKind::Base(b) => AnyOtSender::Base(AnyBaseSender::new(b, conn, rng).await?),
            OtKind::Alsz(b) => {
                let mut base = AnyBaseReceiver::new(b, conn, rng).await?;
                AnyOtSender::Alsz(AlszSender::new(conn, &mut base, rng).await?)
            }
            OtKind::Kos(b) => {
                let mut base = AnyBaseReceiver::new(b, conn, rng).await?;
                AnyOtSender::Kos(KosSender::new(conn, &mut base, rng).await?)
            }
        })
    }
}

impl OtSender for AnyOtSender {
    async fn send(
        &mut self,
        conn: &mut Channel,
        inputs: &[[Block; 2]],
        rng: &mut AesRng,
    ) -> Result<(), Error> {
        match self {
            AnyOtSender::Base(ot) => ot.send(conn, inputs, rng).await,
            AnyOtSender::Alsz(ot) => ot.send(conn, inputs, rng).await,
            AnyOtSender::Kos(ot) => ot.send(conn, inputs, rng).await,
        }
    }
}

/// The receiving side of any [`OtKind`].
enum AnyOtReceiver {
    Base(AnyBaseReceiver),
    Alsz(AlszReceiver),
    Kos(KosReceiver),
}

impl AnyOtReceiver {
    /// Runs the setup of `kind`'s receiver (for an extension, as the sender
    /// of its base OTs).
    async fn new(kind: OtKind, conn: &mut Channel, rng: &mut AesRng) -> Result<Self, Error> {
        Ok(match kind {
            OtKind::Base(b) => AnyOtReceiver::Base(AnyBaseReceiver::new(b, conn, rng).await?),
            OtKind::Alsz(b) => {
                let mut base = AnyBaseSender::new(b, conn, rng).await?;
                AnyOtReceiver::Alsz(AlszReceiver::new(conn, &mut base, rng).await?)
            }
            OtKind::Kos(b) => {
                let mut base = AnyBaseSender::new(b, conn, rng).await?;
                AnyOtReceiver::Kos(KosReceiver::new(conn, &mut base, rng).await?)
            }
        })
    }
}

impl OtReceiver for AnyOtReceiver {
    async fn receive(
        &mut self,
        conn: &mut Channel,
        inputs: &[bool],
        rng: &mut AesRng,
    ) -> Result<Vec<Block>, Error> {
        match self {
            AnyOtReceiver::Base(ot) => ot.receive(conn, inputs, rng).await,
            AnyOtReceiver::Alsz(ot) => ot.receive(conn, inputs, rng).await,
            AnyOtReceiver::Kos(ot) => ot.receive(conn, inputs, rng).await,
        }
    }
}

/// An OT sender session: sets up `kind` and transfers `inputs`. `seed`
/// seeds the protocol's [`AesRng`].
pub fn ot_sender(kind: OtKind, inputs: Vec<[Block; 2]>, seed: Block) -> Session<()> {
    Session::start(move |conn| async move {
        let mut conn = conn;
        let mut rng = AesRng::new(&seed);
        let result = async {
            let mut ot = AnyOtSender::new(kind, &mut conn, &mut rng).await?;
            ot.send(&mut conn, &inputs, &mut rng).await
        }
        .await;
        finish(&mut conn, result)
    })
}

/// An OT receiver session: sets up `kind` and learns the chosen blocks.
pub fn ot_receiver(kind: OtKind, choices: Vec<bool>, seed: Block) -> Session<Vec<Block>> {
    Session::start(move |conn| async move {
        let mut conn = conn;
        let mut rng = AesRng::new(&seed);
        let result = async {
            let mut ot = AnyOtReceiver::new(kind, &mut conn, &mut rng).await?;
            ot.receive(&mut conn, &choices, &mut rng).await
        }
        .await;
        finish(&mut conn, result)
    })
}

/// What a KKRT sender session produces.
#[derive(Clone, Debug)]
pub struct KkrtSenderOutput {
    /// Evaluates the PRF.
    pub encoder: KkrtEncoder,
    /// One PRF key per receiver input.
    pub seeds: Vec<Block512>,
}

/// A KKRT OPRF sender session over `kind` (whose OT receiver it runs) for a
/// receiver with `m` inputs.
pub fn kkrt_sender(kind: OtKind, m: usize, seed: Block) -> Session<KkrtSenderOutput> {
    Session::start(move |conn| async move {
        let mut conn = conn;
        let mut rng = AesRng::new(&seed);
        let result = async {
            let mut ot = AnyOtReceiver::new(kind, &mut conn, &mut rng).await?;
            let mut oprf = KkrtSender::new(&mut conn, &mut ot, &mut rng).await?;
            let seeds = oprf.send(&mut conn, m).await?;
            Ok(KkrtSenderOutput {
                encoder: oprf.encoder,
                seeds,
            })
        }
        .await;
        finish(&mut conn, result)
    })
}

/// A KKRT OPRF receiver session over `kind` (whose OT sender it runs):
/// learns the PRF output of each input.
pub fn kkrt_receiver(kind: OtKind, inputs: Vec<Block>, seed: Block) -> Session<Vec<Block512>> {
    Session::start(move |conn| async move {
        let mut conn = conn;
        let mut rng = AesRng::new(&seed);
        let result = async {
            let mut ot = AnyOtSender::new(kind, &mut conn, &mut rng).await?;
            let mut oprf = KkrtReceiver::new(&mut conn, &mut ot, &mut rng).await?;
            oprf.receive(&mut conn, &inputs, &mut rng).await
        }
        .await;
        finish(&mut conn, result)
    })
}
