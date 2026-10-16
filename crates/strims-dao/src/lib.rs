// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A profile's encrypted store.
//!
//! Each profile keeps its records in one [`strims_kv`] table, sealed with
//! the profile's storage key ([`seal`]) under keys laid out by [`key`].

pub mod auth;
mod error;
pub mod events;
pub mod ids;
pub mod key;
pub mod lease;
pub mod replication;
pub mod schema;
pub mod seal;
pub mod store;
pub mod table;
pub mod vv;

pub use error::Error;
pub use events::{EventBus, Recv, StoreEvent, Subscription};
pub use store::{ProfileStore, Read, ReadTx, ReplicatedStore, WriteTx};
pub use table::{Index, Singleton, Table, UniqueConflict};

#[cfg(test)]
mod tests;
