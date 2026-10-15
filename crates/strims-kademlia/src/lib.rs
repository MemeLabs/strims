// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Kademlia ids and k-buckets.
//!
//! - [`ID`]: 256 bit ids, their byte, hex and JSON forms, and XOR distance.
//! - [`KBucket`]: a routing table bounded at `k` entries per prefix length,
//!   with exact closest-entry lookups.
//! - [`Filter`]: an unbounded collector that yields items nearest first.

mod id;
mod kbucket;

pub use id::{ID, ID_BIT_LENGTH, ID_JSON_LENGTH, ID_LENGTH};
pub use kbucket::{Filter, Insert, Interface, KBucket};

/// Errors from id decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The buffer is shorter than 32 bytes.
    #[error("buffer too short")]
    BufferTooShort,
    /// The slice is not exactly 32 bytes.
    #[error("invalid id length")]
    InvalidLength,
    /// Not a quoted, padded standard base64 encoding of 32 bytes.
    #[error("invalid id json")]
    InvalidJson,
}
