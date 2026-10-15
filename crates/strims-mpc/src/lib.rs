// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Oblivious transfer and private set intersection.
//!
//! The network broker runs [`PszSender`]/[`PszReceiver`] between two peers to
//! find the network keys they share. The composition is Chou-Orlandi base
//! OTs, the KOS OT extension, the KKRT OPRF and PSZ, following GaloisInc's
//! swanky (`ocelot`). [`ot_sender`], [`kkrt_sender`] and their receivers run
//! the building blocks on their own.
//!
//! # Sessions
//!
//! Every protocol is a sans-IO [`Session`] over a reliable, ordered byte
//! stream with no framing of its own: feed every received byte to
//! `handle_input`, split anywhere, send what `poll_transmit` returns, and
//! read the outcome once `is_done`. All computation, curve arithmetic
//! included, runs inside `handle_input`; there are no timers. Sessions are
//! `Send` but not `Sync`.
//!
//! Each run takes the 16-byte seed of its [`AesRng`], which must be secret
//! and fresh per run; `from_rng` draws it from a `DetRng`. The same seeds
//! give the same transcript however the stream is split.
//!
//! # Driving the broker
//!
//! - Roles: the peer with more keys is the PSZ sender; on a tie, the peer
//!   with the greater host ID. Both peers must reach the same answer. The
//!   sender learns only the receiver's set size. The receiver learns which
//!   of its keys the sender also holds, in cuckoo-bin order.
//! - Transport: the stream runs over vnic frames on the network broker
//!   port. Feed each frame body to `handle_input`, and call `poll_transmit`
//!   with a buffer of the frame MTU.
//! - Deadline and retry: for a few percent of small key sets the receiver
//!   fails with [`Error::CuckooHashMapFull`] (see [`CuckooHashMap::new`]),
//!   and the sender then waits forever. Impose a deadline and retry with
//!   fresh seeds; each retry succeeds independently.
//! - Errors: drop the session on any error. The stream can't be
//!   resynchronised, so bytes left over from a failed run may still arrive
//!   and fail the next one.
//!
//! # Async inside
//!
//! The protocols are `async fn`s over an in-memory byte channel, driven by
//! the session with a no-op waker. This is a deliberate, local exception to
//! the core crates' "no async" rule: it keeps the protocol code
//! straight-line, needs no executor or runtime, and never escapes the crate.

mod aes_hash;
mod aes_rng;
mod alsz;
mod block;
mod channel;
mod chou_orlandi;
mod cointoss;
mod common;
mod cuckoo;
#[cfg(test)]
mod golden_tests;
mod kkrt;
mod kos;
pub mod mpctest;
mod naor_pinkas;
mod ot;
mod prc;
mod psz;
mod session;

pub use aes_hash::{AesHash, FIXED_KEY};
pub use aes_rng::AesRng;
pub use block::{Block, Block512};
pub use cuckoo::{CuckooHashMap, CuckooItem};
pub use kkrt::KkrtEncoder;
pub use kos::SPP;
pub use ot::{
    BaseOt, KkrtSenderOutput, OtKind, kkrt_receiver, kkrt_sender, ot_receiver, ot_sender,
};
pub use prc::PseudorandomCode;
pub use psz::{MAX_PEER_SET_SIZE, PszReceiver, PszSender};
pub use session::{Endpoint, Session};

/// Errors from the protocols. Any of them aborts the run.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// 32 bytes from the peer that aren't a canonical Ristretto encoding.
    #[error("bytes do not encode a valid point")]
    InvalidPoint,
    /// The OT extension receiver cheated or the transcripts diverged.
    #[error("consistency check failed")]
    ConsistencyCheckFailed,
    /// The coin-toss sender opened a seed that doesn't match its commitment.
    #[error("commitment check failed")]
    CommitmentCheckFailed,
    /// More than 2^28 elements.
    #[error("invalid cuckoo set size")]
    InvalidCuckooSetSize,
    /// Unsupported number of hash functions.
    #[error("invalid cuckoo hash parameters")]
    InvalidCuckooParameters,
    /// An element couldn't be placed in the cuckoo table.
    #[error("cuckoo hash full")]
    CuckooHashMapFull,
    /// The peer announced a set larger than this side accepts.
    #[error("peer set size {size} exceeds {max}")]
    PeerSetTooLarge { size: usize, max: usize },
    /// Input arrived that the protocol never reads: after it finished, or
    /// left over at the end. The peers disagree about the transcript.
    #[error("unexpected input")]
    UnexpectedInput,
    /// The input stream ended early.
    #[error("connection closed")]
    Closed,
    /// A bit matrix shape the transpose doesn't support.
    #[error("unsupported {nrows}x{ncols} byte matrix")]
    UnsupportedMatrix { nrows: usize, ncols: usize },
    /// An internal invariant didn't hold.
    #[error("internal error")]
    Internal,
}
