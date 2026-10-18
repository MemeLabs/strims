// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`IdbKv`]: the worker's [`Kv`], on IndexedDB.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use js_sys::Uint8Array;
use strims_kv::{Error, Kv, ReadTx, WriteTx};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    IdbCursorWithValue, IdbDatabase, IdbFactory, IdbKeyRange, IdbObjectStore, IdbRequest,
    IdbTransaction, IdbTransactionMode,
};

use crate::idb::{self, Completion, Request, STORE};
use crate::js;

/// A [`Kv`] on IndexedDB. Clones share their connections.
///
/// # Layout
///
/// Each table is a database named by the store's prefix and the table
/// name, at version 1, with one object store, `data`, of string keys and
/// `Uint8Array` values. Stores already in this layout open as they are.
/// Production stores have an empty prefix; tests give each store its own.
///
/// IndexedDB orders string keys by their UTF-16 code units, which is byte
/// order for the ASCII keys that scans use.
///
/// Only [`Kv::create_table`] creates a database. Beginning a transaction
/// on a missing table aborts the database's creation and fails with
/// [`Error::NoTable`], leaving no database behind.
///
/// # Transactions
///
/// IndexedDB commits a transaction on its own once its task returns to the
/// event loop with no request of the transaction pending (see
/// `strims_kv`'s transaction discipline):
///
/// - A transaction lives through any number of its own awaited
///   operations. Awaiting anything else commits the writes it has made so
///   far, and its later operations, [`WriteTx::commit`] included, fail with
///   [`Error::Inactive`].
/// - [`WriteTx::commit`] resolves once the transaction is durable
///   (`complete`). Dropping an uncommitted write transaction aborts it.
/// - A failed request aborts its transaction.
///
/// `IdbKv` never fails with [`Error::Busy`]. Transactions in different
/// tasks run concurrently, and IndexedDB orders those that write the same
/// table. A transaction or table operation begun while the same task has a
/// transaction open doesn't wait on it: the open transaction has no
/// request pending while the task awaits the new operation, so it commits,
/// and its later operations fail with [`Error::Inactive`].
///
/// # Connections
///
/// A table's connection is opened at its first use and cached. It closes
/// when another connection asks to delete or upgrade its database
/// (`versionchange`), and [`Kv::delete_table`] closes it before deleting
/// the database. The next use opens a new one.
#[derive(Clone)]
pub struct IdbKv(Rc<Inner>);

struct Inner {
    prefix: String,
    factory: Result<IdbFactory, String>,
    /// Open connections by table: at most one per table in use.
    conns: RefCell<BTreeMap<String, Rc<Conn>>>,
}

impl IdbKv {
    /// The store whose database names start with `prefix`. Where the
    /// global has no IndexedDB, every operation fails with
    /// [`Error::Backend`].
    pub fn new(prefix: &str) -> IdbKv {
        let factory = match js::scope().indexed_db() {
            Ok(Some(factory)) => Ok(factory),
            Ok(None) => Err("IndexedDB is unavailable".to_owned()),
            Err(e) => Err(js::describe(&e)),
        };
        IdbKv(Rc::new(Inner {
            prefix: prefix.to_owned(),
            factory,
            conns: RefCell::default(),
        }))
    }

    async fn begin(&self, table: &str, mode: IdbTransactionMode) -> Result<IdbTransaction, Error> {
        let inner = &self.0;
        let mut reopened = false;
        loop {
            let conn = match inner.cached(table) {
                Some(conn) => conn,
                None => {
                    let (db, _) = idb::open(inner.factory()?, &inner.name(table), false)?.await?;
                    reopened = true;
                    inner.insert(table, db)
                }
            };
            match conn.db.transaction_with_str_and_mode(STORE, mode) {
                Ok(tx) => return Ok(tx),
                // A connection closed by the browser refuses transactions
                // before its `close` event arrives; a new one doesn't.
                Err(e)
                    if !reopened && js::error_name(&e).as_deref() == Some("InvalidStateError") =>
                {
                    inner.evict_conn(table, &conn);
                    reopened = true;
                }
                Err(e) => {
                    return Err(match js::error_name(&e).as_deref() {
                        Some("NotFoundError") => Error::NoTable,
                        _ => idb::error(&e),
                    });
                }
            }
        }
    }
}

impl Inner {
    fn factory(&self) -> Result<&IdbFactory, Error> {
        self.factory.as_ref().map_err(|e| Error::Backend(e.clone()))
    }

    fn name(&self, table: &str) -> String {
        format!("{}{table}", self.prefix)
    }

    /// The table's open connection, dropping one that has closed.
    fn cached(&self, table: &str) -> Option<Rc<Conn>> {
        let mut conns = self.conns.borrow_mut();
        let conn = conns.get(table)?;
        if !conn.closed.get() {
            return Some(conn.clone());
        }
        let stale = conns.remove(table);
        drop(conns);
        drop(stale);
        None
    }

    /// Caches `db` as the table's connection. A connection cached
    /// meanwhile by another task's open is kept, and `db` closed.
    fn insert(&self, table: &str, db: IdbDatabase) -> Rc<Conn> {
        if let Some(conn) = self.cached(table) {
            db.close();
            return conn;
        }
        let conn = Conn::new(db);
        let old = self
            .conns
            .borrow_mut()
            .insert(table.to_owned(), conn.clone());
        drop(old);
        conn
    }

    /// Closes the table's cached connection.
    fn evict(&self, table: &str) {
        let conn = self.conns.borrow_mut().remove(table);
        drop(conn);
    }

    /// Closes `conn`, and drops it from the cache if it is still the
    /// table's cached connection.
    fn evict_conn(&self, table: &str, conn: &Rc<Conn>) {
        let current = self
            .conns
            .borrow()
            .get(table)
            .is_some_and(|c| Rc::ptr_eq(c, conn));
        if current {
            self.evict(table);
        }
        conn.close();
    }
}

/// A cached connection. Dropping it closes the connection once its
/// transactions finish.
struct Conn {
    db: IdbDatabase,
    closed: Rc<Cell<bool>>,
    _on_versionchange: Closure<dyn FnMut()>,
    _on_close: Closure<dyn FnMut()>,
}

impl Conn {
    fn new(db: IdbDatabase) -> Rc<Conn> {
        let closed = Rc::new(Cell::new(false));
        let on_versionchange = {
            let (db, closed) = (db.clone(), closed.clone());
            Closure::<dyn FnMut()>::new(move || {
                closed.set(true);
                db.close();
            })
        };
        // The browser closed the connection, e.g. because storage was
        // cleared.
        let on_close = {
            let closed = closed.clone();
            Closure::<dyn FnMut()>::new(move || closed.set(true))
        };
        db.set_onversionchange(Some(on_versionchange.as_ref().unchecked_ref()));
        db.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        Rc::new(Conn {
            db,
            closed,
            _on_versionchange: on_versionchange,
            _on_close: on_close,
        })
    }

    fn close(&self) {
        self.closed.set(true);
        self.db.close();
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        self.db.set_onversionchange(None);
        self.db.set_onclose(None);
        self.close();
    }
}

impl Kv for IdbKv {
    type Read = IdbRead;
    type Write = IdbWrite;

    async fn create_table(&self, table: &str) -> Result<bool, Error> {
        let inner = &self.0;
        if inner.cached(table).is_some() {
            return Ok(false);
        }
        let (db, created) = idb::open(inner.factory()?, &inner.name(table), true)?.await?;
        inner.insert(table, db);
        Ok(created)
    }

    async fn delete_table(&self, table: &str) -> Result<(), Error> {
        let inner = &self.0;
        inner.evict(table);
        // Blocked by our own connection only if another task opened one
        // since: close it, and the deletion goes ahead.
        let weak = Rc::downgrade(inner);
        let blocked_table = table.to_owned();
        let on_blocked = move || {
            if let Some(inner) = weak.upgrade() {
                inner.evict(&blocked_table);
            }
        };
        idb::delete(inner.factory()?, &inner.name(table), on_blocked)?.await
    }

    async fn begin_read(&self, table: &str) -> Result<IdbRead, Error> {
        let tx = self.begin(table, IdbTransactionMode::Readonly).await?;
        Ok(IdbRead {
            store: object_store(&tx)?,
        })
    }

    async fn begin_write(&self, table: &str) -> Result<IdbWrite, Error> {
        let tx = self.begin(table, IdbTransactionMode::Readwrite).await?;
        let store = object_store(&tx)?;
        Ok(IdbWrite {
            tx,
            store,
            finished: false,
        })
    }
}

/// A read-only [`IdbKv`] transaction.
pub struct IdbRead {
    store: IdbObjectStore,
}

impl ReadTx for IdbRead {
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        get(&self.store, key).await
    }

    async fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        scan(&self.store, prefix, after, limit).await
    }
}

/// A read-write [`IdbKv`] transaction. Dropping it uncommitted aborts it.
pub struct IdbWrite {
    tx: IdbTransaction,
    store: IdbObjectStore,
    /// `commit` has been called.
    finished: bool,
}

impl ReadTx for IdbWrite {
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        get(&self.store, key).await
    }

    async fn scan(
        &mut self,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        scan(&self.store, prefix, after, limit).await
    }
}

impl WriteTx for IdbWrite {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), Error> {
        let value = Uint8Array::from(value);
        let req = self
            .store
            .put_with_key(&value, &JsValue::from_str(key))
            .map_err(|e| idb::error(&e))?;
        idb::request(req).await.map(drop)
    }

    async fn delete(&mut self, key: &str) -> Result<(), Error> {
        let req = self
            .store
            .delete(&JsValue::from_str(key))
            .map_err(|e| idb::error(&e))?;
        idb::request(req).await.map(drop)
    }

    async fn commit(mut self) -> Result<(), Error> {
        self.finished = true;
        let complete = Completion::new(&self.tx);
        idb::commit(&self.tx)?;
        complete.await
    }
}

impl Drop for IdbWrite {
    fn drop(&mut self) {
        if !self.finished {
            // Fails only for a transaction that has already finished.
            let _ = self.tx.abort();
        }
    }
}

fn object_store(tx: &IdbTransaction) -> Result<IdbObjectStore, Error> {
    tx.object_store(STORE)
        .map_err(|e| match js::error_name(&e).as_deref() {
            Some("NotFoundError") => Error::NoTable,
            _ => idb::error(&e),
        })
}

async fn get(store: &IdbObjectStore, key: &str) -> Result<Option<Vec<u8>>, Error> {
    let req = store
        .get(&JsValue::from_str(key))
        .map_err(|e| idb::error(&e))?;
    let value = idb::request(req).await?;
    if value.is_undefined() {
        return Ok(None);
    }
    bytes(&value).map(Some)
}

/// Up to `limit` entries under `prefix` after `after`. The cursor starts
/// at the later of the two and stops at the first key outside the prefix;
/// the rows are collected inside its success callback, so the scan costs
/// one wake.
async fn scan(
    store: &IdbObjectStore,
    prefix: &str,
    after: Option<&str>,
    limit: usize,
) -> Result<Vec<(String, Vec<u8>)>, Error> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let range = match after {
        Some(after) if after >= prefix => {
            IdbKeyRange::lower_bound_with_open(&JsValue::from_str(after), true)
        }
        _ => IdbKeyRange::lower_bound(&JsValue::from_str(prefix)),
    }
    .map_err(|e| idb::error(&e))?;
    let req = store
        .open_cursor_with_range(&range)
        .map_err(|e| idb::error(&e))?;
    let prefix = prefix.to_owned();
    let mut found = Vec::new();
    Request::new(req, move |req| {
        match advance(req, &prefix, limit, &mut found) {
            Ok(true) => None,
            Ok(false) => Some(Ok(std::mem::take(&mut found))),
            Err(e) => Some(Err(e)),
        }
    })
    .await
}

/// Takes the cursor's row into `found`. Returns whether the cursor moved
/// on to another row.
fn advance(
    req: &IdbRequest,
    prefix: &str,
    limit: usize,
    found: &mut Vec<(String, Vec<u8>)>,
) -> Result<bool, Error> {
    let cursor = req.result().map_err(|e| idb::error(&e))?;
    if cursor.is_null() || cursor.is_undefined() {
        return Ok(false);
    }
    let cursor: IdbCursorWithValue = cursor.unchecked_into();
    let key = cursor
        .key()
        .map_err(|e| idb::error(&e))?
        .as_string()
        .ok_or_else(|| Error::Backend("stored key is not a string".to_owned()))?;
    if !key.starts_with(prefix) {
        return Ok(false);
    }
    let value = bytes(&cursor.value().map_err(|e| idb::error(&e))?)?;
    found.push((key, value));
    if found.len() >= limit {
        return Ok(false);
    }
    cursor.continue_().map_err(|e| idb::error(&e))?;
    Ok(true)
}

fn bytes(value: &JsValue) -> Result<Vec<u8>, Error> {
    value
        .dyn_ref::<Uint8Array>()
        .map(Uint8Array::to_vec)
        .ok_or_else(|| Error::Backend("stored value is not a Uint8Array".to_owned()))
}
