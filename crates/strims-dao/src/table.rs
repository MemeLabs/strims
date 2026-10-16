// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Record types.
//!
//! A [`Table`] holds records by id under its namespace; a [`Singleton`]
//! holds one record. Each record type is a zero-sized type implementing
//! one of the traits, and its operations are the traits' provided
//! methods.
//!
//! # Writes
//!
//! Writing a table row runs, in order:
//!
//! 1. the unique checks of the table's [`Index`]es, in declaration order
//!    ([`Table::on_unique_conflict`] decides each conflict);
//! 2. [`Table::on_change`];
//! 3. index maintenance;
//! 4. for a replicated table in a replicating transaction, a version bump
//!    for the writing replica and a replication [`Event`];
//! 5. the sealed put, and the change event.
//!
//! A replicating transaction can't create a replicated record whose
//! deletion its replica has logged ([`Error::Deleted`]): every write to a
//! record precedes or is concurrent with its deletion, so replicas that
//! apply the deletion last agree.
//!
//! [`Table::update`], [`Table::upsert`] and [`Table::transform`] write
//! nothing when the record is unchanged. [`Table::insert`] writes without
//! reading the previous record. Deleting runs [`Table::on_delete`], removes
//! the index entries, logs a replicated delete, and removes the row.
//!
//! # Applying changes from other replicas
//!
//! Applied changes don't replicate further, except that a unique key
//! conflict between an applied record and a local one is settled by the
//! table's [`ApplyConflict`], whose deletions replicate.

use std::future::Future;

use futures_util::future::LocalBoxFuture;
use prost::Message;
use strims_base::time::Time;
use strims_kv::Kv;
use strims_proto::chat::v1 as chat;
use strims_proto::dao::v1::VersionVector;
use strims_proto::network::v1 as network;
use strims_proto::profile::v1::Device;
use strims_proto::replication::v1::Event;

use crate::error::Error;
use crate::events::{Record, StoreEvent};
use crate::key::Namespace;
use crate::schema::system::is_deleted;
use crate::store::{Read, WriteTx, get_record, index_ids, scan_records};
use crate::vv;

/// A record with a version vector.
pub trait Versioned {
    /// The record's version.
    fn version(&self) -> Option<&VersionVector>;
    /// The record's version, for updating.
    fn version_mut(&mut self) -> &mut Option<VersionVector>;
}

macro_rules! versioned {
    ($($t:ty),* $(,)?) => {
        $(impl Versioned for $t {
            fn version(&self) -> Option<&VersionVector> {
                self.version.as_ref()
            }
            fn version_mut(&mut self) -> &mut Option<VersionVector> {
                &mut self.version
            }
        })*
    };
}

versioned!(
    Device,
    network::Network,
    network::bootstrap::BootstrapClient,
    chat::UiConfig,
    chat::UiConfigHighlight,
    chat::UiConfigTag,
    chat::UiConfigIgnore,
    chat::WhisperThread,
    chat::WhisperRecord,
);

/// How a record type replicates.
pub struct Replication<R> {
    /// The record's version.
    pub version: fn(&R) -> Option<&VersionVector>,
    /// The record's version, for updating.
    pub version_mut: fn(&mut R) -> &mut Option<VersionVector>,
    /// The part of a record other replicas receive.
    pub extract: fn(R) -> R,
    /// The record to store when a received record (first argument) is
    /// newer than the local one (second).
    pub merge: fn(R, &R) -> R,
    /// The record to store when a received record (first argument) and
    /// the local one (second) were written concurrently. Replicas resolve
    /// a conflict from opposite sides, so the result must not depend on
    /// which record is local.
    pub resolve: fn(R, &R) -> R,
}

impl<R> Clone for Replication<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for Replication<R> {}

impl<R: Versioned + Message + Clone> Replication<R> {
    /// Replicates whole records: a newer record replaces the local one,
    /// and of two concurrent records the one [`written_after`] the other
    /// wins.
    pub const DEFAULT: Replication<R> = Replication {
        version: R::version,
        version_mut: R::version_mut,
        extract: identity,
        merge: merge_newer,
        resolve: resolve_last_written,
    };
}

fn identity<R>(r: R) -> R {
    r
}

/// Raises `r`'s version to cover `other`'s.
pub fn upgrade_version<R: Versioned>(r: &mut R, other: &R) {
    if let Some(o) = other.version() {
        vv::upgrade(
            r.version_mut().get_or_insert_with(VersionVector::default),
            o,
        );
    }
}

/// `next` with its version raised to cover `local`'s.
pub fn merge_newer<R: Versioned>(mut next: R, local: &R) -> R {
    upgrade_version(&mut next, local);
    next
}

/// Of `next` and `local`, the one [`written_after`] the other, with its
/// version raised to cover both.
pub fn resolve_last_written<R: Versioned + Message + Clone>(next: R, local: &R) -> R {
    let (mut winner, loser) = if written_after(&next, local) {
        (next, local.clone())
    } else {
        (local.clone(), next)
    };
    upgrade_version(&mut winner, &loser);
    winner
}

/// Whether `a` was written after `b`: by version time, and between
/// records written in the same millisecond, by their encodings without
/// versions. The order is total, so concurrent replicas pick the same
/// record.
pub fn written_after<R: Versioned + Message + Clone>(a: &R, b: &R) -> bool {
    let at = a.version().map_or(0, |v| v.updated_at);
    let bt = b.version().map_or(0, |v| v.updated_at);
    if at != bt {
        return at > bt;
    }
    encode_without_version(a) > encode_without_version(b)
}

fn encode_without_version<R: Versioned + Message + Clone>(r: &R) -> Vec<u8> {
    let mut r = r.clone();
    *r.version_mut() = None;
    r.encode_to_vec()
}

/// A secondary index on a table: entries map a key computed from each
/// record to the record's id.
pub struct Index<R> {
    /// The namespace of the index's entries.
    pub ns: Namespace,
    /// The key a record is indexed under.
    pub key: fn(&R) -> Vec<u8>,
    /// Whether a record is indexed.
    pub condition: fn(&R) -> bool,
    /// Whether at most one record may hold a key.
    pub unique: bool,
}

impl<R> Index<R> {
    /// An index of every record by `key`.
    pub const fn new(ns: Namespace, key: fn(&R) -> Vec<u8>) -> Index<R> {
        Index {
            ns,
            key,
            condition: always,
            unique: false,
        }
    }

    /// An index where at most one record holds each key.
    pub const fn unique(ns: Namespace, key: fn(&R) -> Vec<u8>) -> Index<R> {
        Index {
            ns,
            key,
            condition: always,
            unique: true,
        }
    }

    /// The index restricted to records `condition` accepts.
    pub const fn when(mut self, condition: fn(&R) -> bool) -> Index<R> {
        self.condition = condition;
        self
    }
}

fn always<R>(_: &R) -> bool {
    true
}

/// What happens when a locally written record's unique key belongs to
/// another record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniqueConflict {
    /// The write fails with [`Error::UniqueViolation`].
    Reject,
    /// The other record is removed without its delete hook: the written
    /// record takes its place.
    Replace,
}

/// What happens when a record applied from another replica takes a
/// unique key another record holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyConflict {
    /// The record with the lower id stays. The other is deleted, and the
    /// deletion replicates, so every replica keeps the same record.
    LowerIdWins,
    /// The local record stays and the applied one is dropped here.
    /// Replicas may keep different records.
    KeepLocal,
}

fn ready<'a>() -> LocalBoxFuture<'a, Result<(), Error>> {
    Box::pin(async { Ok(()) })
}

/// A table of records by id.
pub trait Table: Sized + 'static {
    /// The record type.
    type Record: Message + Default + Clone + PartialEq + 'static;
    /// The namespace of the table's rows.
    const NS: Namespace;
    /// The table's indexes.
    const INDEXES: &'static [Index<Self::Record>] = &[];
    /// How the table replicates, if it does.
    const REPLICATION: Option<Replication<Self::Record>> = None;

    /// A record's id.
    fn id(r: &Self::Record) -> u64;

    /// What subscribers observe when a record changes or is deleted.
    fn observe(_: &Self::Record) -> Option<Record> {
        None
    }

    /// Runs before a record `m` replacing `p` is indexed and stored, and
    /// may adjust it.
    fn on_change<'a, K: Kv>(
        _tx: &'a mut WriteTx<K>,
        _m: &'a mut Self::Record,
        _p: Option<&'a Self::Record>,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        ready()
    }

    /// Runs before the record `p` is deleted.
    fn on_delete<'a, K: Kv>(
        _tx: &'a mut WriteTx<K>,
        _p: &'a Self::Record,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        ready()
    }

    /// Decides a conflict on the unique index `ix` between a locally
    /// written record `m`, which it may adjust, and `existing`.
    fn on_unique_conflict(
        _ix: &Index<Self::Record>,
        _m: &mut Self::Record,
        _existing: &Self::Record,
    ) -> UniqueConflict {
        UniqueConflict::Reject
    }

    /// How applying another replica's record settles a conflict on a
    /// unique index.
    const ON_APPLY_CONFLICT: ApplyConflict = ApplyConflict::LowerIdWins;

    /// The record `id`.
    fn get<R: Read>(
        tx: &mut R,
        id: u64,
    ) -> impl Future<Output = Result<Option<Self::Record>, Error>> {
        async move { get_record(tx, &Self::NS.record_key(id)).await }
    }

    /// Every record, in key order.
    fn get_all<R: Read>(tx: &mut R) -> impl Future<Output = Result<Vec<Self::Record>, Error>> {
        async move { scan_records(tx, &Self::NS.prefix()).await }
    }

    /// Writes `r` without reading the record it replaces. Resolves to the
    /// stored record.
    fn insert<K: Kv>(
        tx: &mut WriteTx<K>,
        r: Self::Record,
    ) -> impl Future<Output = Result<Self::Record, Error>> {
        write_row::<Self, K>(tx, r, None)
    }

    /// Replaces an existing record. Fails with [`Error::NotFound`] if
    /// there is none.
    fn update<K: Kv>(
        tx: &mut WriteTx<K>,
        r: Self::Record,
    ) -> impl Future<Output = Result<Self::Record, Error>> {
        async move {
            let p = Self::get(tx, Self::id(&r)).await?.ok_or(Error::NotFound)?;
            if p == r {
                return Ok(r);
            }
            write_row::<Self, K>(tx, r, Some(p)).await
        }
    }

    /// Writes `r`, replacing any record with its id.
    fn upsert<K: Kv>(
        tx: &mut WriteTx<K>,
        r: Self::Record,
    ) -> impl Future<Output = Result<Self::Record, Error>> {
        async move {
            let p = Self::get(tx, Self::id(&r)).await?;
            if p.as_ref() == Some(&r) {
                return Ok(r);
            }
            write_row::<Self, K>(tx, r, p).await
        }
    }

    /// Writes the record `f` makes of record `id`, or of a default record
    /// if there is none.
    fn transform<K: Kv, F: FnOnce(&mut Self::Record) -> Result<(), Error>>(
        tx: &mut WriteTx<K>,
        id: u64,
        f: F,
    ) -> impl Future<Output = Result<Self::Record, Error>> {
        async move {
            let p = Self::get(tx, id).await?;
            let mut v = p.clone().unwrap_or_default();
            f(&mut v)?;
            if p.as_ref() == Some(&v) {
                return Ok(v);
            }
            write_row::<Self, K>(tx, v, p).await
        }
    }

    /// Deletes record `id`. Resolves to whether it existed.
    fn delete<K: Kv>(tx: &mut WriteTx<K>, id: u64) -> impl Future<Output = Result<bool, Error>> {
        async move {
            let Some(p) = Self::get(tx, id).await? else {
                return Ok(false);
            };
            Self::on_delete(tx, &p).await?;
            let replica = tx.replica_id();
            remove_row::<Self, K>(tx, p, replica).await?;
            Ok(true)
        }
    }

    /// The ids `ix` maps `key` to.
    fn index_ids<R: Read>(
        tx: &mut R,
        ix: &Index<Self::Record>,
        key: &[u8],
    ) -> impl Future<Output = Result<Vec<u64>, Error>> {
        index_ids(tx, ix.ns, key)
    }

    /// The records `ix` maps `key` to.
    fn index_get_all<R: Read>(
        tx: &mut R,
        ix: &Index<Self::Record>,
        key: &[u8],
    ) -> impl Future<Output = Result<Vec<Self::Record>, Error>> {
        async move {
            let ids = index_ids(tx, ix.ns, key).await?;
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                out.extend(Self::get(tx, id).await?);
            }
            Ok(out)
        }
    }

    /// The first record `ix` maps `key` to.
    fn index_get<R: Read>(
        tx: &mut R,
        ix: &Index<Self::Record>,
        key: &[u8],
    ) -> impl Future<Output = Result<Option<Self::Record>, Error>> {
        async move {
            for id in index_ids(tx, ix.ns, key).await? {
                if let Some(r) = Self::get(tx, id).await? {
                    return Ok(Some(r));
                }
            }
            Ok(None)
        }
    }
}

/// Writes table row `m` over `p`, the row it replaces.
pub(crate) async fn write_row<T: Table, K: Kv>(
    tx: &mut WriteTx<K>,
    mut m: T::Record,
    mut p: Option<T::Record>,
) -> Result<T::Record, Error> {
    let id = T::id(&m);
    if p.is_none() && T::REPLICATION.is_some() {
        if tx.applying() && tx.deleted_here(T::NS, id) {
            return Err(Error::Superseded);
        }
        if tx.replica_id().is_some()
            && (tx.deleted_here(T::NS, id) || is_deleted(tx, T::NS, id).await?)
        {
            return Err(Error::Deleted);
        }
    }
    for ix in T::INDEXES.iter().filter(|ix| ix.unique) {
        if !(ix.condition)(&m) {
            continue;
        }
        let key = (ix.key)(&m);
        if let Some(p) = &p
            && (ix.condition)(p)
            && (ix.key)(p) == key
        {
            continue;
        }
        for other in index_ids(tx, ix.ns, &key).await? {
            if other == id {
                continue;
            }
            let Some(existing) = T::get(tx, other).await? else {
                tx.delete_index(ix.ns, &key, other).await?;
                continue;
            };
            if let Some(replica) = tx.applying_as() {
                match T::ON_APPLY_CONFLICT {
                    ApplyConflict::KeepLocal => return Err(Error::Superseded),
                    ApplyConflict::LowerIdWins if other < id => {
                        if let Some(p) = p.take() {
                            remove_row::<T, K>(tx, p, None).await?;
                        }
                        log_delete::<T, K>(tx, &mut m, replica);
                        return Err(Error::Superseded);
                    }
                    ApplyConflict::LowerIdWins => {
                        remove_row::<T, K>(tx, existing, Some(replica)).await?
                    }
                }
                continue;
            }
            match T::on_unique_conflict(ix, &mut m, &existing) {
                UniqueConflict::Reject => return Err(Error::UniqueViolation(ix.ns)),
                UniqueConflict::Replace => {
                    let replica = tx.replica_id();
                    remove_row::<T, K>(tx, existing, replica).await?
                }
            }
        }
    }

    T::on_change(tx, &mut m, p.as_ref()).await?;

    for ix in T::INDEXES {
        let key = (ix.key)(&m);
        let cond = (ix.condition)(&m);
        if let Some(p) = &p {
            let pkey = (ix.key)(p);
            let pcond = (ix.condition)(p);
            if pkey == key && pcond == cond {
                continue;
            }
            if pcond {
                tx.delete_index(ix.ns, &pkey, id).await?;
            }
        }
        if cond {
            tx.put_index(ix.ns, &key, id).await?;
        }
    }

    if let (Some(replica), Some(rep)) = (tx.replica_id(), T::REPLICATION) {
        let version = next_version(&rep, &mut m, p.as_ref(), replica, tx.now());
        tx.replicate(Event {
            namespace: T::NS.0,
            id,
            version: Some(version),
            delete: false,
            record: (rep.extract)(m.clone()).encode_to_vec(),
        });
    }

    tx.put_record(&T::NS.record_key(id), &m).await?;
    if let Some(r) = T::observe(&m) {
        tx.emit(StoreEvent::Changed(r));
    }
    Ok(m)
}

/// Versions a local write of `m` over `p` by `replica`: the write follows
/// both records' versions. Resolves to the new version.
fn next_version<R>(
    rep: &Replication<R>,
    m: &mut R,
    p: Option<&R>,
    replica: u64,
    now: Time,
) -> VersionVector {
    let pv = p.and_then(rep.version).cloned();
    let v = (rep.version_mut)(m).get_or_insert_with(VersionVector::default);
    if let Some(pv) = &pv {
        vv::upgrade(v, pv);
    }
    vv::increment(v, replica, now);
    v.clone()
}

/// Logs the deletion of `p` by `replica`.
fn log_delete<T: Table, K: Kv>(tx: &mut WriteTx<K>, p: &mut T::Record, replica: u64) {
    if let Some(rep) = T::REPLICATION {
        let version = next_version(&rep, p, None, replica, tx.now());
        tx.replicate(Event {
            namespace: T::NS.0,
            id: T::id(p),
            version: Some(version),
            delete: true,
            record: Vec::new(),
        });
    }
}

/// Removes table row `p` and its index entries without running the
/// table's delete hook, and logs the deletion as `replica`'s if given.
pub(crate) async fn remove_row<T: Table, K: Kv>(
    tx: &mut WriteTx<K>,
    mut p: T::Record,
    replica: Option<u64>,
) -> Result<(), Error> {
    let id = T::id(&p);
    for ix in T::INDEXES {
        if (ix.condition)(&p) {
            tx.delete_index(ix.ns, &(ix.key)(&p), id).await?;
        }
    }
    if let Some(replica) = replica {
        log_delete::<T, K>(tx, &mut p, replica);
    }
    tx.delete_key(&T::NS.record_key(id)).await?;
    if let Some(r) = T::observe(&p) {
        tx.emit(StoreEvent::Deleted(r));
    }
    Ok(())
}

/// A record type with one record.
pub trait Singleton: Sized + 'static {
    /// The record type.
    type Record: Message + Default + Clone + PartialEq + 'static;
    /// The record's key.
    const NS: Namespace;
    /// How the record replicates, if it does.
    const REPLICATION: Option<Replication<Self::Record>> = None;

    /// The record's value before one is stored.
    fn default_value() -> Option<Self::Record> {
        None
    }

    /// What subscribers observe when the record changes.
    fn observe(_: &Self::Record) -> Option<Record> {
        None
    }

    /// The record, or its default value if none is stored.
    fn get<R: Read>(tx: &mut R) -> impl Future<Output = Result<Option<Self::Record>, Error>> {
        async move {
            let r = get_record(tx, &Self::NS.key()).await?;
            Ok(r.or_else(Self::default_value))
        }
    }

    /// Stores `r`, unless the record already equals it.
    fn set<K: Kv>(
        tx: &mut WriteTx<K>,
        r: Self::Record,
    ) -> impl Future<Output = Result<Self::Record, Error>> {
        async move {
            let p = Self::get(tx).await?;
            if p.as_ref() == Some(&r) {
                return Ok(r);
            }
            write_singleton::<Self, K>(tx, r, p.as_ref()).await
        }
    }

    /// Stores the record `f` makes of the current one.
    fn transform<K: Kv, F: FnOnce(&mut Self::Record) -> Result<(), Error>>(
        tx: &mut WriteTx<K>,
        f: F,
    ) -> impl Future<Output = Result<Self::Record, Error>> {
        async move {
            let p = Self::get(tx).await?;
            let mut v = p.clone().unwrap_or_default();
            f(&mut v)?;
            if p.as_ref() == Some(&v) {
                return Ok(v);
            }
            write_singleton::<Self, K>(tx, v, p.as_ref()).await
        }
    }
}

/// Stores singleton record `m` over `p`, the record it replaces.
pub(crate) async fn write_singleton<S: Singleton, K: Kv>(
    tx: &mut WriteTx<K>,
    mut m: S::Record,
    p: Option<&S::Record>,
) -> Result<S::Record, Error> {
    if let (Some(replica), Some(rep)) = (tx.replica_id(), S::REPLICATION) {
        let version = next_version(&rep, &mut m, p, replica, tx.now());
        tx.replicate(Event {
            namespace: S::NS.0,
            id: 0,
            version: Some(version),
            delete: false,
            record: (rep.extract)(m.clone()).encode_to_vec(),
        });
    }
    tx.put_record(&S::NS.key(), &m).await?;
    if let Some(r) = S::observe(&m) {
        tx.emit(StoreEvent::Changed(r));
    }
    Ok(m)
}
