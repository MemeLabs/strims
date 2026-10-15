// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Publishes a local byte stream into a swarm.
//!
//! Input is buffered into integrity units (a munro of
//! `chunks_per_signature` chunks for Merkle, one chunk otherwise); each unit
//! is signed, split into chunks and published. The one staging buffer of one
//! unit is allocated when the writer is created.
//!
//! Times are truncated to milliseconds before signing: the wire carries
//! milliseconds and the signature covers nanoseconds, so a finer time would
//! not verify at the peer.
//!
//! The writer doesn't own its [`Swarm`], so the runner can: each call takes
//! the swarm it was created with.

use core::mem;

use strims_base::time::Time;
use strims_binmap::Bin;
use strims_proto::types::Key;

use crate::integrity::{self, ProtectionMethod, SignatureSigner, SwarmVerifier};
use crate::options::{SchedulingMethod, SwarmOptions};
use crate::store;
use crate::swarm::{Swarm, SwarmError};
use crate::swarm_id::SwarmId;

/// Options for a [`Writer`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WriterOptions {
    pub swarm_options: SwarmOptions,
    /// An ed25519 key: the 64 byte private key (seed ‖ public key) and the
    /// 32 byte public key.
    pub key: Key,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    /// The next munro's index.
    Merkle(u64),
    SignAll,
    None,
}

/// Publishes a byte stream into a swarm as signed chunks.
#[derive(Debug)]
pub struct Writer {
    id: SwarmId,
    signer: SignatureSigner,
    unit: Unit,
    chunk_size: usize,
    /// Bound: one integrity unit.
    staging: Vec<u8>,
    staged: usize,
    chunks: store::Writer,
    scratch: Vec<u8>,
}

impl Writer {
    /// Creates the swarm (with the seed scheduling method unless set), sets
    /// its read offset to 0 and signs the first epoch at `now`.
    pub fn new(o: WriterOptions, now: Time) -> Result<(Writer, Swarm), SwarmError> {
        let so = o.swarm_options.assign_defaults(&SwarmOptions {
            scheduling_method: SchedulingMethod::SEED,
            ..Default::default()
        });
        let id = SwarmId::new(&o.key.public);
        let mut swarm = Swarm::new(id.clone(), so)?;
        swarm.set_offset(Bin(0));

        let so = swarm.options();
        let signer = so
            .integrity
            .live_signature_algorithm
            .signer(&o.key.private)?;
        let (unit, unit_len) = match so.integrity.protection_method {
            ProtectionMethod::MERKLE_TREE => {
                (Unit::Merkle(0), so.chunk_size * so.chunks_per_signature)
            }
            ProtectionMethod::SIGN_ALL => (Unit::SignAll, so.chunk_size),
            _ => (Unit::None, so.chunk_size),
        };

        let mut w = Writer {
            id,
            signer,
            unit,
            chunk_size: so.chunk_size,
            staging: vec![0; unit_len],
            staged: 0,
            chunks: store::Writer::new(),
            scratch: Vec::with_capacity(8 + so.chunk_size),
        };
        w.init_epoch(&mut swarm, now)?;
        Ok((w, swarm))
    }

    /// The id of the swarm this writer publishes to.
    pub fn swarm_id(&self) -> &SwarmId {
        &self.id
    }

    /// Signs a new epoch at `now`. A clock that went back before the
    /// current epoch fails with
    /// [`EpochError::OutOfDate`](crate::EpochError::OutOfDate).
    fn init_epoch(&mut self, swarm: &mut Swarm, now: Time) -> Result<(), SwarmError> {
        let e = now.truncate(Time::PRECISION);
        let sig = self.signer.sign(e, &[], &mut self.scratch);
        swarm.sync_epoch(e, sig.as_slice())?;
        Ok(())
    }

    fn check(&self, swarm: &Swarm) -> Result<(), SwarmError> {
        if *swarm.id() != self.id {
            return Err(SwarmError::WrongSwarm);
        }
        Ok(())
    }

    /// Appends `p` to the stream. Every completed unit is signed with
    /// `now` and published.
    pub fn write(&mut self, swarm: &mut Swarm, now: Time, p: &[u8]) -> Result<usize, SwarmError> {
        self.check(swarm)?;
        let now = now.truncate(Time::PRECISION);
        let unit = self.staging.len();
        let mut p = p;
        let mut nn = 0;

        while self.staged + p.len() >= unit {
            if self.staged == 0 {
                let (u, rest) = p.split_at(unit);
                self.process(swarm, now, u)?;
                nn += unit;
                p = rest;
            } else {
                let n = unit - self.staged;
                let (head, rest) = p.split_at(n);
                if let Some(dst) = self.staging.get_mut(self.staged..) {
                    dst.copy_from_slice(head);
                }
                self.staged = 0;
                let staging = mem::take(&mut self.staging);
                let r = self.process(swarm, now, &staging);
                self.staging = staging;
                r?;
                nn += n;
                p = rest;
            }
        }

        if let Some(dst) = self.staging.get_mut(self.staged..self.staged + p.len()) {
            dst.copy_from_slice(p);
        }
        self.staged += p.len();
        Ok(nn + p.len())
    }

    /// Publishes a partial unit, padded with zeros to a whole unit, so the
    /// units written after it stay aligned with their signatures.
    pub fn flush(&mut self, swarm: &mut Swarm, now: Time) -> Result<(), SwarmError> {
        self.check(swarm)?;
        if self.staged == 0 {
            return Ok(());
        }
        if let Some(pad) = self.staging.get_mut(self.staged..) {
            pad.fill(0);
        }
        self.staged = 0;
        let staging = mem::take(&mut self.staging);
        let r = self.process(swarm, now.truncate(Time::PRECISION), &staging);
        self.staging = staging;
        r
    }

    /// Starts the stream over at chunk 0 under a new epoch signed at `now`:
    /// the swarm's store and verifier are reset and
    /// [`SwarmEvent::Reset`](crate::SwarmEvent::Reset) is queued. A reset
    /// within the millisecond of the current epoch keeps that epoch, so
    /// peers aren't told to reset.
    pub fn reset(&mut self, swarm: &mut Swarm, now: Time) -> Result<(), SwarmError> {
        self.check(swarm)?;
        self.init_epoch(swarm, now)?;
        self.staged = 0;
        if let Unit::Merkle(n) = &mut self.unit {
            *n = 0;
        }
        self.chunks.reset();
        swarm.reset();
        swarm.set_offset(Bin(0));
        Ok(())
    }

    /// Signs one unit and publishes its chunks.
    fn process(&mut self, swarm: &mut Swarm, now: Time, data: &[u8]) -> Result<(), SwarmError> {
        match &mut self.unit {
            Unit::Merkle(n) => {
                let SwarmVerifier::Merkle(v) = &mut swarm.verifier else {
                    return Err(integrity::Error::VerifierMismatch.into());
                };
                v.write_munro(&self.signer, now, *n, data)?;
                *n += 1;
                for chunk in data.chunks(self.chunk_size) {
                    swarm.publish(self.chunks.next_bin(), chunk)?;
                }
            }
            Unit::SignAll => {
                let SwarmVerifier::SignAll(v) = &mut swarm.verifier else {
                    return Err(integrity::Error::VerifierMismatch.into());
                };
                let b = self.chunks.next_bin();
                v.write_chunk(&self.signer, now, b, data);
                swarm.publish(b, data)?;
            }
            Unit::None => {
                swarm.publish(self.chunks.next_bin(), data)?;
            }
        }
        Ok(())
    }
}
