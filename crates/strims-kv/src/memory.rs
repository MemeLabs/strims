// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! An in-memory [`Kv`] for tests and simulations.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::rc::Rc;

use crate::{Error, Kv, ReadTx, WriteTx};

type Table = BTreeMap<String, Vec<u8>>;

/// Every table's entries, by table name.
pub type Dump = BTreeMap<String, Table>;

#[derive(Default)]
struct Inner {
    tables: Dump,
    open: usize,
}

/// An in-memory store. Clones share the same tables.
///
/// Every operation completes immediately, and beginning a transaction
/// while another is open fails with [`Error::Busy`] (see the crate docs).
#[derive(Clone, Default)]
pub struct MemoryKv(Rc<RefCell<Inner>>);

impl MemoryKv {
    /// An empty store.
    pub fn new() -> MemoryKv {
        MemoryKv::default()
    }

    /// A copy of every table's entries.
    pub fn dump(&self) -> Dump {
        self.0.borrow().tables.clone()
    }

    /// Replaces the store's contents with `dump`.
    pub fn load(&self, dump: Dump) -> Result<(), Error> {
        let mut inner = self.0.borrow_mut();
        if inner.open != 0 {
            return Err(Error::Busy);
        }
        inner.tables = dump;
        Ok(())
    }

    /// The number of open transactions. A task that follows the
    /// transaction discipline leaves none open while it is suspended.
    pub fn open_transactions(&self) -> usize {
        self.0.borrow().open
    }

    fn begin(&self, table: &str) -> Result<Guard, Error> {
        let mut inner = self.0.borrow_mut();
        if inner.open != 0 {
            return Err(Error::Busy);
        }
        if !inner.tables.contains_key(table) {
            return Err(Error::NoTable);
        }
        inner.open += 1;
        Ok(Guard {
            kv: self.0.clone(),
            table: table.to_owned(),
        })
    }

    fn idle(&self) -> Result<std::cell::RefMut<'_, Inner>, Error> {
        let inner = self.0.borrow_mut();
        if inner.open != 0 {
            return Err(Error::Busy);
        }
        Ok(inner)
    }
}

impl Kv for MemoryKv {
    type Read = MemoryRead;
    type Write = MemoryWrite;

    async fn create_table(&self, table: &str) -> Result<bool, Error> {
        let mut inner = self.idle()?;
        if inner.tables.contains_key(table) {
            return Ok(false);
        }
        inner.tables.insert(table.to_owned(), Table::new());
        Ok(true)
    }

    async fn delete_table(&self, table: &str) -> Result<(), Error> {
        self.idle()?.tables.remove(table);
        Ok(())
    }

    async fn begin_read(&self, table: &str) -> Result<MemoryRead, Error> {
        Ok(MemoryRead(self.begin(table)?))
    }

    async fn begin_write(&self, table: &str) -> Result<MemoryWrite, Error> {
        Ok(MemoryWrite {
            guard: self.begin(table)?,
            overlay: BTreeMap::new(),
        })
    }
}

/// Holds a transaction's place in the open count. The table can't change
/// underneath it: no other transaction or table operation runs while it
/// lives.
struct Guard {
    kv: Rc<RefCell<Inner>>,
    table: String,
}

impl Guard {
    fn with<R>(&self, f: impl FnOnce(&Table) -> R) -> Result<R, Error> {
        let inner = self.kv.borrow();
        let table = inner.tables.get(&self.table).ok_or(Error::NoTable)?;
        Ok(f(table))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let mut inner = self.kv.borrow_mut();
        inner.open = inner.open.saturating_sub(1);
    }
}

/// A read-only [`MemoryKv`] transaction.
pub struct MemoryRead(Guard);

impl ReadTx for MemoryRead {
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.0.with(|t| t.get(key).cloned())
    }

    async fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        let empty = BTreeMap::new();
        self.0.with(|t| scan(t, &empty, prefix, after, limit))
    }
}

/// A read-write [`MemoryKv`] transaction. Writes collect in an overlay
/// that commit applies.
pub struct MemoryWrite {
    guard: Guard,
    overlay: BTreeMap<String, Option<Vec<u8>>>,
}

impl ReadTx for MemoryWrite {
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        if let Some(v) = self.overlay.get(key) {
            return Ok(v.clone());
        }
        self.guard.with(|t| t.get(key).cloned())
    }

    async fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        let overlay = &self.overlay;
        self.guard.with(|t| scan(t, overlay, prefix, after, limit))
    }
}

impl WriteTx for MemoryWrite {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.overlay.insert(key.to_owned(), Some(value.to_vec()));
        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), Error> {
        self.overlay.insert(key.to_owned(), None);
        Ok(())
    }

    async fn commit(mut self) -> Result<(), Error> {
        let overlay = std::mem::take(&mut self.overlay);
        let mut inner = self.guard.kv.borrow_mut();
        let table = inner
            .tables
            .get_mut(&self.guard.table)
            .ok_or(Error::NoTable)?;
        for (k, v) in overlay {
            match v {
                Some(v) => table.insert(k, v),
                None => table.remove(&k),
            };
        }
        Ok(())
    }
}

/// Merges the committed entries with a transaction's overlay, which wins
/// where both hold a key.
fn scan(
    table: &Table,
    overlay: &BTreeMap<String, Option<Vec<u8>>>,
    prefix: &str,
    after: Option<&str>,
    limit: usize,
) -> Vec<(String, Vec<u8>)> {
    let start = match after {
        Some(a) if a >= prefix => Bound::Excluded(a),
        _ => Bound::Included(prefix),
    };
    let range = (start, Bound::Unbounded);
    let mut base = table
        .range::<str, _>(range)
        .take_while(|(k, _)| k.starts_with(prefix))
        .peekable();
    let mut over = overlay
        .range::<str, _>(range)
        .take_while(|(k, _)| k.starts_with(prefix))
        .peekable();

    let mut out = Vec::new();
    while out.len() < limit {
        let order = match (base.peek(), over.peek()) {
            (None, None) => break,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some((b, _)), Some((o, _))) => b.as_str().cmp(o.as_str()),
        };
        if order == Ordering::Less {
            if let Some((k, v)) = base.next() {
                out.push((k.clone(), v.clone()));
            }
            continue;
        }
        if order == Ordering::Equal {
            base.next();
        }
        if let Some((k, Some(v))) = over.next() {
            out.push((k.clone(), v.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use futures_util::FutureExt;

    use super::*;

    #[test]
    fn conformance() {
        crate::conformance::run_immediate(MemoryKv::new)
            .now_or_never()
            .expect("memory operations are ready immediately");
    }

    #[test]
    fn dump_and_load_round_trip() {
        let a = MemoryKv::new();
        async {
            a.create_table("t").await.unwrap();
            let mut tx = a.begin_write("t").await.unwrap();
            tx.put("k", b"v").await.unwrap();
            tx.commit().await.unwrap();
        }
        .now_or_never()
        .unwrap();

        let b = MemoryKv::new();
        b.load(a.dump()).unwrap();
        assert_eq!(a.dump(), b.dump());
    }

    #[test]
    fn open_transactions_counts_until_drop() {
        let kv = MemoryKv::new();
        async {
            kv.create_table("t").await.unwrap();
            let tx = kv.begin_read("t").await.unwrap();
            assert_eq!(kv.open_transactions(), 1);
            assert_eq!(kv.load(Dump::new()), Err(Error::Busy));
            drop(tx);
            assert_eq!(kv.open_transactions(), 0);
        }
        .now_or_never()
        .unwrap();
    }
}
