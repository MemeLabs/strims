// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Transactional key-value storage.
//!
//! A [`Kv`] holds named tables of string keys and byte values. All access
//! goes through transactions: [`Kv::begin_read`] and [`Kv::begin_write`]
//! return an owned transaction, [`WriteTx::commit`] applies a write
//! transaction atomically, and dropping a write transaction without
//! committing discards its writes.
//!
//! # Transaction discipline
//!
//! Between `begin_*` and the end of a transaction, a task awaits only that
//! transaction's operations, and it never begins a second transaction.
//! Browser storage commits a transaction on its own once control returns
//! to the event loop with no request pending, so awaiting anything else
//! inside a transaction lets it commit early; its later operations then
//! fail with [`Error::Inactive`]. A second transaction begun inside the
//! first can wait on it forever.
//!
//! Backends whose operations complete immediately ([`MemoryKv`] and the
//! native backend) enforce the rule: every operation's future is ready on
//! its first poll, so a conforming transaction runs to completion within
//! one poll of its task, and any `begin_*` while another transaction is
//! open fails with [`Error::Busy`].
//!
//! # Keys
//!
//! Scans order keys by their bytes. Keys made of ASCII characters order
//! the same way in every backend, so callers that scan use ASCII keys.

use std::future::Future;

mod memory;

#[cfg(any(test, feature = "conformance"))]
pub mod conformance;

pub use memory::{Dump, MemoryKv, MemoryRead, MemoryWrite};

/// A failed storage operation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The table doesn't exist.
    #[error("table not found")]
    NoTable,
    /// Another transaction is open on this store.
    #[error("another transaction is open")]
    Busy,
    /// The transaction ended before this operation, e.g. because its task
    /// awaited something other than the transaction.
    #[error("transaction is no longer active")]
    Inactive,
    /// The backend failed.
    #[error("storage backend: {0}")]
    Backend(String),
}

/// A store of named tables.
pub trait Kv: 'static {
    /// A read-only transaction on one table.
    type Read: ReadTx;
    /// A read-write transaction on one table.
    type Write: WriteTx;

    /// Creates `table` if it doesn't exist. Resolves to whether it was
    /// created.
    fn create_table(&self, table: &str) -> impl Future<Output = Result<bool, Error>>;

    /// Deletes `table` and everything in it. Deleting a missing table
    /// succeeds.
    fn delete_table(&self, table: &str) -> impl Future<Output = Result<(), Error>>;

    /// Begins a read-only transaction on `table`.
    fn begin_read(&self, table: &str) -> impl Future<Output = Result<Self::Read, Error>>;

    /// Begins a read-write transaction on `table`.
    fn begin_write(&self, table: &str) -> impl Future<Output = Result<Self::Write, Error>>;
}

/// Reads within a transaction.
pub trait ReadTx: 'static {
    /// The value at `key`.
    fn get(&mut self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>, Error>>;

    /// Up to `limit` entries whose keys start with `prefix` and sort
    /// strictly after `after`, in ascending key order.
    fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<(String, Vec<u8>)>, Error>>;
}

/// Writes within a transaction. A transaction reads its own writes.
pub trait WriteTx: ReadTx {
    /// Sets `key` to `value`.
    fn put(&mut self, key: &str, value: &[u8]) -> impl Future<Output = Result<(), Error>>;

    /// Removes `key`. Removing a missing key succeeds.
    fn delete(&mut self, key: &str) -> impl Future<Output = Result<(), Error>>;

    /// Applies the transaction's writes atomically.
    fn commit(self) -> impl Future<Output = Result<(), Error>>;
}
