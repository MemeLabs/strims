// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ids::{Exhausted, InvalidRanges};
use crate::key::Namespace;
use crate::seal::{KeyError, SealError};

/// A failed store operation.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The storage backend failed.
    #[error(transparent)]
    Kv(#[from] strims_kv::Error),
    /// A stored value didn't open with the store key.
    #[error(transparent)]
    Seal(#[from] SealError),
    /// A key couldn't be derived.
    #[error(transparent)]
    Key(#[from] KeyError),
    /// A stored or received record didn't decode.
    #[error("record decode: {0}")]
    Decode(#[from] prost::DecodeError),
    /// The profile has no free record ids.
    #[error(transparent)]
    IdsExhausted(#[from] Exhausted),
    /// Id ranges are malformed or exceed their bound.
    #[error(transparent)]
    InvalidRanges(#[from] InvalidRanges),
    /// The record doesn't exist.
    #[error("record not found")]
    NotFound,
    /// Another record holds the same unique index key.
    #[error("unique index {0} already holds the key")]
    UniqueViolation(Namespace),
    /// The store's format version isn't one this build reads.
    #[error("store version {have} is not supported")]
    UnsupportedStoreVersion {
        /// The stored version, 0 when none is stored.
        have: u32,
    },
    /// A store for the profile already exists.
    #[error("store already exists")]
    StoreExists,
    /// The profile has no device id to replicate as.
    #[error("profile has no device id")]
    NoReplicaId,
    /// The record was deleted, and replicated records aren't created
    /// again.
    #[error("record was deleted")]
    Deleted,
    /// An applied record lost a unique key conflict to a record with a
    /// lower id.
    #[error("superseded by a record with a lower id")]
    Superseded,
    /// The user name, password or session doesn't open a profile.
    #[error("invalid credentials")]
    Credentials,
    /// Another profile has the user name.
    #[error("user name is taken")]
    NameTaken,
    /// Another holder has the lease.
    #[error("lease is held")]
    LeaseBusy,
    /// A record or request is malformed.
    #[error("invalid {0}")]
    Invalid(&'static str),
}
