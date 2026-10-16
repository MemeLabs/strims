// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A [`strims_kv::Kv`] in one redb file.
//!
//! Each kv table is a redb table of the same name, with `&str` keys and
//! `&[u8]` values. One process opens a file at a time.
//!
//! Operations run synchronously inside futures that are ready on their
//! first poll, so the executor thread blocks for each operation's disk
//! I/O, including the sync on every commit. Beginning a transaction or a
//! table operation while a transaction is open fails with
//! [`Error::Busy`].

use std::cell::Cell;
use std::ops::Bound;
use std::path::Path;
use std::rc::Rc;

use redb::{
    Database, ReadOnlyTable, ReadTransaction, ReadableDatabase, ReadableTable, TableDefinition,
    TableError, TableHandle, WriteTransaction,
};
use strims_kv::{Error, Kv, ReadTx, WriteTx};

type Def<'a> = TableDefinition<'a, &'static str, &'static [u8]>;

fn def(name: &str) -> Def<'_> {
    TableDefinition::new(name)
}

fn backend(e: impl std::fmt::Display) -> Error {
    Error::Backend(e.to_string())
}

struct Shared {
    db: Database,
    open: Cell<usize>,
}

/// A redb-backed store. Clones share the open file.
#[derive(Clone)]
pub struct RedbKv(Rc<Shared>);

impl RedbKv {
    /// Opens the store at `path`, creating the file if it doesn't exist.
    pub fn open(path: impl AsRef<Path>) -> Result<RedbKv, Error> {
        Ok(RedbKv::from_database(
            Database::create(path).map_err(backend)?,
        ))
    }

    /// A store that lives in memory.
    pub fn in_memory() -> Result<RedbKv, Error> {
        let db = Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .map_err(backend)?;
        Ok(RedbKv::from_database(db))
    }

    fn from_database(db: Database) -> RedbKv {
        RedbKv(Rc::new(Shared {
            db,
            open: Cell::new(0),
        }))
    }

    fn guard(&self) -> Result<Guard, Error> {
        if self.0.open.get() != 0 {
            return Err(Error::Busy);
        }
        self.0.open.set(1);
        Ok(Guard(self.0.clone()))
    }

    fn idle(&self) -> Result<(), Error> {
        match self.0.open.get() {
            0 => Ok(()),
            _ => Err(Error::Busy),
        }
    }
}

fn has_table(txn: &WriteTransaction, name: &str) -> Result<bool, Error> {
    Ok(txn
        .list_tables()
        .map_err(backend)?
        .any(|t| t.name() == name))
}

impl Kv for RedbKv {
    type Read = RedbRead;
    type Write = RedbWrite;

    async fn create_table(&self, table: &str) -> Result<bool, Error> {
        self.idle()?;
        let txn = self.0.db.begin_write().map_err(backend)?;
        if has_table(&txn, table)? {
            return Ok(false);
        }
        txn.open_table(def(table)).map_err(backend)?;
        txn.commit().map_err(backend)?;
        Ok(true)
    }

    async fn delete_table(&self, table: &str) -> Result<(), Error> {
        self.idle()?;
        let txn = self.0.db.begin_write().map_err(backend)?;
        txn.delete_table(def(table)).map_err(backend)?;
        txn.commit().map_err(backend)
    }

    async fn begin_read(&self, table: &str) -> Result<RedbRead, Error> {
        let guard = self.guard()?;
        let txn = self.0.db.begin_read().map_err(backend)?;
        let table = match txn.open_table(def(table)) {
            Ok(t) => t,
            Err(TableError::TableDoesNotExist(_)) => return Err(Error::NoTable),
            Err(e) => return Err(backend(e)),
        };
        Ok(RedbRead {
            table,
            _txn: txn,
            _guard: guard,
        })
    }

    async fn begin_write(&self, table: &str) -> Result<RedbWrite, Error> {
        let guard = self.guard()?;
        let txn = self.0.db.begin_write().map_err(backend)?;
        if !has_table(&txn, table)? {
            return Err(Error::NoTable);
        }
        Ok(RedbWrite {
            txn,
            table: table.to_owned(),
            _guard: guard,
        })
    }
}

/// Holds a transaction's place in the open count.
struct Guard(Rc<Shared>);

impl Drop for Guard {
    fn drop(&mut self) {
        self.0.open.set(0);
    }
}

/// A read-only [`RedbKv`] transaction.
pub struct RedbRead {
    table: ReadOnlyTable<&'static str, &'static [u8]>,
    _txn: ReadTransaction,
    _guard: Guard,
}

impl ReadTx for RedbRead {
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        get(&self.table, key)
    }

    async fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        scan(&self.table, prefix, after, limit)
    }
}

/// A read-write [`RedbKv`] transaction. Dropping it aborts it.
pub struct RedbWrite {
    txn: WriteTransaction,
    table: String,
    _guard: Guard,
}

impl ReadTx for RedbWrite {
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let table = self.txn.open_table(def(&self.table)).map_err(backend)?;
        get(&table, key)
    }

    async fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        let table = self.txn.open_table(def(&self.table)).map_err(backend)?;
        scan(&table, prefix, after, limit)
    }
}

impl WriteTx for RedbWrite {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), Error> {
        let mut table = self.txn.open_table(def(&self.table)).map_err(backend)?;
        table.insert(key, value).map_err(backend)?;
        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), Error> {
        let mut table = self.txn.open_table(def(&self.table)).map_err(backend)?;
        table.remove(key).map_err(backend)?;
        Ok(())
    }

    async fn commit(self) -> Result<(), Error> {
        self.txn.commit().map_err(backend)
    }
}

fn get(
    table: &impl ReadableTable<&'static str, &'static [u8]>,
    key: &str,
) -> Result<Option<Vec<u8>>, Error> {
    Ok(table.get(key).map_err(backend)?.map(|v| v.value().to_vec()))
}

fn scan(
    table: &impl ReadableTable<&'static str, &'static [u8]>,
    prefix: &str,
    after: Option<&str>,
    limit: usize,
) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let start = match after {
        Some(a) if a >= prefix => Bound::Excluded(a),
        _ => Bound::Included(prefix),
    };
    let mut out = Vec::new();
    if limit == 0 {
        return Ok(out);
    }
    for entry in table
        .range::<&str>((start, Bound::Unbounded))
        .map_err(backend)?
    {
        let (k, v) = entry.map_err(backend)?;
        let k = k.value();
        if !k.starts_with(prefix) {
            break;
        }
        out.push((k.to_owned(), v.value().to_vec()));
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use futures_util::FutureExt;

    use super::*;

    #[test]
    fn conformance() {
        strims_kv::conformance::run_immediate(|| RedbKv::in_memory().unwrap())
            .now_or_never()
            .expect("redb operations are ready immediately");
    }

    #[test]
    fn reopened_file_keeps_committed_writes() {
        let dir = std::env::temp_dir().join(format!("strims-kv-redb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("reopen.redb");
        let _ = std::fs::remove_file(&path);

        async {
            let kv = RedbKv::open(&path).unwrap();
            kv.create_table("t").await.unwrap();
            let mut tx = kv.begin_write("t").await.unwrap();
            tx.put("kept", b"1").await.unwrap();
            tx.commit().await.unwrap();
            let mut tx = kv.begin_write("t").await.unwrap();
            tx.put("dropped", b"1").await.unwrap();
            drop(tx);
        }
        .now_or_never()
        .unwrap();

        async {
            let kv = RedbKv::open(&path).unwrap();
            assert!(!kv.create_table("t").await.unwrap());
            let mut tx = kv.begin_read("t").await.unwrap();
            assert_eq!(tx.get("kept").await.unwrap().as_deref(), Some(&b"1"[..]));
            assert_eq!(tx.get("dropped").await.unwrap(), None);
        }
        .now_or_never()
        .unwrap();

        std::fs::remove_file(&path).unwrap();
    }
}
