// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Profile stores and their transactions.
//!
//! A [`ProfileStore`] is one profile's kv table. Its values are sealed with
//! the profile's [`StorageKey`]; its keys are laid out by [`crate::key`].
//! Reads and writes happen in a [`ReadTx`] or [`WriteTx`], through the
//! record types in [`crate::schema`]. A write transaction's change events
//! reach the store's [`EventBus`] once it commits.
//!
//! A [`ReplicatedStore`] writes as one of the profile's replicas: its
//! transactions version every replicated record they write and log the
//! changes as [`EventLog`]s for the profile's other devices.
//!
//! The kv transaction discipline applies: a task holding a transaction
//! awaits only that transaction's operations and begins no other.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use prost::Message;
use strims_base::rng::{DetRng, fork};
use strims_base::time::Time;
use strims_kv::{Kv, ReadTx as _, WriteTx as _};
use strims_proto::dao::v1::{SecondaryIndexKey, StoreVersion};
use strims_proto::replication::v1::{Event, EventLog};

use crate::error::Error;
use crate::events::{EventBus, StoreEvent};
use crate::ids;
use crate::key::{Namespace, index_entry_key, index_prefix, profile_table};
use crate::schema::profile::{Profile, ProfileIds};
use crate::schema::system::{Checkpoints, EventLogs, StoreVersions};
use crate::seal::StorageKey;
use crate::table::{Singleton, Table};

/// The store format this build reads and writes.
pub const STORE_VERSION: u32 = 8;

/// Ids a store reserves from its ranges at a time.
const ID_RESERVATION: u64 = 100;

/// The most encoded event bytes in one event log, so a log fits one
/// replication message with room to spare.
pub const MAX_LOG_BYTES: usize = 256 * 1024;

/// The most events in one event log.
pub const MAX_LOG_EVENTS: usize = 4096;

struct Inner<K> {
    kv: K,
    id: u64,
    table: String,
    key: StorageKey,
    rng: RefCell<DetRng>,
    bus: EventBus,
    ids: Cell<(u64, u64)>,
}

/// One profile's encrypted store. Clones share the store.
pub struct ProfileStore<K: Kv> {
    inner: Rc<Inner<K>>,
}

impl<K: Kv> Clone for ProfileStore<K> {
    fn clone(&self) -> Self {
        ProfileStore {
            inner: self.inner.clone(),
        }
    }
}

impl<K: Kv> ProfileStore<K> {
    /// Creates the store of profile `id` at the current format version.
    /// Fails with [`Error::StoreExists`] if the profile has a store.
    ///
    /// `rng` draws the nonces values are sealed under, so outside
    /// simulations it must be seeded from OS entropy.
    pub async fn create(
        kv: K,
        id: u64,
        key: StorageKey,
        rng: DetRng,
        now: Time,
    ) -> Result<Self, Error> {
        let store = ProfileStore::new(kv, id, key, rng);
        if !store.inner.kv.create_table(&store.inner.table).await? {
            return Err(Error::StoreExists);
        }
        let mut tx = store.write(now).await?;
        StoreVersions::set(
            &mut tx,
            StoreVersion {
                version: STORE_VERSION,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(store)
    }

    /// Opens the store of profile `id`, which must be at the current
    /// format version.
    ///
    /// `rng` draws the nonces values are sealed under, so outside
    /// simulations it must be seeded from OS entropy.
    pub async fn open(kv: K, id: u64, key: StorageKey, rng: DetRng) -> Result<Self, Error> {
        let store = ProfileStore::new(kv, id, key, rng);
        let mut tx = store.read().await?;
        let have = StoreVersions::get(&mut tx).await?.map_or(0, |v| v.version);
        drop(tx);
        if have != STORE_VERSION {
            return Err(Error::UnsupportedStoreVersion { have });
        }
        Ok(store)
    }

    fn new(kv: K, id: u64, key: StorageKey, rng: DetRng) -> Self {
        ProfileStore {
            inner: Rc::new(Inner {
                kv,
                id,
                table: profile_table(id),
                key,
                rng: RefCell::new(rng),
                bus: EventBus::new(),
                ids: Cell::new((0, 0)),
            }),
        }
    }

    /// The profile's id.
    pub fn profile_id(&self) -> u64 {
        self.inner.id
    }

    /// The store's key.
    pub fn key(&self) -> &StorageKey {
        &self.inner.key
    }

    /// The kv store holding the profile's table.
    pub fn kv(&self) -> &K {
        &self.inner.kv
    }

    /// The bus the store's change events go out on.
    pub fn events(&self) -> &EventBus {
        &self.inner.bus
    }

    /// Begins a read transaction.
    pub async fn read(&self) -> Result<ReadTx<K>, Error> {
        Ok(ReadTx {
            kv: self.inner.kv.begin_read(&self.inner.table).await?,
            store: self.inner.clone(),
        })
    }

    /// Begins a write transaction that doesn't replicate its changes.
    pub async fn write(&self, now: Time) -> Result<WriteTx<K>, Error> {
        self.begin_write(now, None).await
    }

    async fn begin_write(&self, now: Time, replica: Option<u64>) -> Result<WriteTx<K>, Error> {
        let kv = self.inner.kv.begin_write(&self.inner.table).await?;
        let rng = fork(&mut self.inner.rng.borrow_mut());
        Ok(WriteTx {
            kv,
            store: self.inner.clone(),
            rng,
            now,
            events: Vec::new(),
            replica,
            replication: Vec::new(),
            applying: None,
            reserved: None,
        })
    }

    /// Deletes the store and everything in it.
    pub async fn delete(self) -> Result<(), Error> {
        Ok(self.inner.kv.delete_table(&self.inner.table).await?)
    }
}

/// A profile store that writes as one of the profile's replicas.
pub struct ReplicatedStore<K: Kv> {
    store: ProfileStore<K>,
    replica: u64,
}

impl<K: Kv> Clone for ReplicatedStore<K> {
    fn clone(&self) -> Self {
        ReplicatedStore {
            store: self.store.clone(),
            replica: self.replica,
        }
    }
}

impl<K: Kv> ReplicatedStore<K> {
    /// Writes to `store` as the profile's device. Fails with
    /// [`Error::NoReplicaId`] if the profile has no device id yet.
    pub async fn new(store: ProfileStore<K>) -> Result<Self, Error> {
        let mut tx = store.read().await?;
        let replica = Profile::get(&mut tx).await?.map_or(0, |p| p.device_id);
        drop(tx);
        if replica == 0 {
            return Err(Error::NoReplicaId);
        }
        Ok(ReplicatedStore { store, replica })
    }

    /// The replica this store writes as: the profile's device id.
    pub fn replica_id(&self) -> u64 {
        self.replica
    }

    /// The store, for transactions that don't replicate.
    pub fn store(&self) -> &ProfileStore<K> {
        &self.store
    }

    /// Begins a read transaction.
    pub async fn read(&self) -> Result<ReadTx<K>, Error> {
        self.store.read().await
    }

    /// Begins a write transaction that replicates its changes.
    pub async fn write(&self, now: Time) -> Result<WriteTx<K>, Error> {
        self.store.begin_write(now, Some(self.replica)).await
    }

    /// The bus the store's change events go out on.
    pub fn events(&self) -> &EventBus {
        self.store.events()
    }
}

mod private {
    pub trait Sealed {}
}

/// A transaction records can be read in.
pub trait Read: private::Sealed {
    #[doc(hidden)]
    type Kv: strims_kv::ReadTx;
    #[doc(hidden)]
    fn parts(&mut self) -> (&mut Self::Kv, &StorageKey);
}

/// A read-only transaction.
pub struct ReadTx<K: Kv> {
    kv: K::Read,
    store: Rc<Inner<K>>,
}

impl<K: Kv> private::Sealed for ReadTx<K> {}

impl<K: Kv> Read for ReadTx<K> {
    type Kv = K::Read;

    fn parts(&mut self) -> (&mut K::Read, &StorageKey) {
        (&mut self.kv, &self.store.key)
    }
}

/// A read-write transaction. Dropping it without committing discards its
/// writes and events.
pub struct WriteTx<K: Kv> {
    kv: K::Write,
    store: Rc<Inner<K>>,
    rng: DetRng,
    now: Time,
    events: Vec<StoreEvent>,
    replica: Option<u64>,
    replication: Vec<Event>,
    applying: Option<u64>,
    reserved: Option<(u64, u64)>,
}

impl<K: Kv> private::Sealed for WriteTx<K> {}

impl<K: Kv> Read for WriteTx<K> {
    type Kv = K::Write;

    fn parts(&mut self) -> (&mut K::Write, &StorageKey) {
        (&mut self.kv, &self.store.key)
    }
}

impl<K: Kv> WriteTx<K> {
    /// The time the transaction stamps its writes with.
    pub fn now(&self) -> Time {
        self.now
    }

    /// The replica the transaction writes as, if it replicates.
    pub fn replica_id(&self) -> Option<u64> {
        self.replica
    }

    /// Whether the transaction applies changes from other replicas.
    pub fn applying(&self) -> bool {
        self.applying.is_some()
    }

    /// The replica applying changes from others, which logs the deletions
    /// that settle unique key conflicts.
    pub(crate) fn applying_as(&self) -> Option<u64> {
        self.applying
    }

    pub(crate) fn set_applying(&mut self, replica: u64) {
        self.applying = Some(replica);
    }

    /// The transaction's rng.
    pub fn rng(&mut self) -> &mut DetRng {
        &mut self.rng
    }

    /// A new record id from the profile's ranges.
    pub async fn generate_id(&mut self) -> Result<u64, Error> {
        if let Some((next, end)) = self.reserved
            && next < end
        {
            self.reserved = Some((next + 1, end));
            return Ok(next);
        }
        let (next, end) = self.store.ids.get();
        if next < end {
            self.store.ids.set((next + 1, end));
            return Ok(next);
        }
        let mut ranges = ProfileIds::get(self)
            .await?
            .unwrap_or_else(ids::initial_ranges);
        let (first, end) = ids::incr(&mut ranges, ID_RESERVATION)?;
        ProfileIds::set(self, ranges).await?;
        self.reserved = Some((first + 1, end));
        Ok(first)
    }

    pub(crate) fn emit(&mut self, e: StoreEvent) {
        self.events.push(e);
    }

    pub(crate) fn replicate(&mut self, e: Event) {
        self.replication.push(e);
    }

    pub(crate) fn deleted_here(&self, ns: Namespace, id: u64) -> bool {
        self.replication
            .iter()
            .any(|e| e.delete && e.namespace == ns.0 && e.id == id)
    }

    pub(crate) fn salt(&self) -> &[u8] {
        self.store.key.bytes()
    }

    pub(crate) async fn put_record(&mut self, key: &str, m: &impl Message) -> Result<(), Error> {
        let sealed = self.store.key.seal(&mut self.rng, &m.encode_to_vec())?;
        Ok(self.kv.put(key, &sealed).await?)
    }

    pub(crate) async fn delete_key(&mut self, key: &str) -> Result<(), Error> {
        Ok(self.kv.delete(key).await?)
    }

    pub(crate) async fn put_index(
        &mut self,
        ns: Namespace,
        key: &[u8],
        id: u64,
    ) -> Result<(), Error> {
        let k = index_entry_key(ns, self.salt(), key, id);
        let entry = SecondaryIndexKey {
            key: key.to_vec(),
            id,
        };
        self.put_record(&k, &entry).await
    }

    pub(crate) async fn delete_index(
        &mut self,
        ns: Namespace,
        key: &[u8],
        id: u64,
    ) -> Result<(), Error> {
        let k = index_entry_key(ns, self.salt(), key, id);
        self.delete_key(&k).await
    }

    /// Commits the transaction, then emits its events. A replicating
    /// transaction first logs its changes.
    pub async fn commit(mut self) -> Result<(), Error> {
        let events = std::mem::take(&mut self.replication);
        if let Some(replica) = self.replica.or(self.applying)
            && !events.is_empty()
        {
            self.commit_event_logs(replica, events).await?;
        }
        let WriteTx {
            kv,
            store,
            events,
            reserved,
            ..
        } = self;
        kv.commit().await?;
        if let Some(r) = reserved {
            store.ids.set(r);
        }
        store.bus.emit(events);
        Ok(())
    }

    async fn commit_event_logs(&mut self, replica: u64, events: Vec<Event>) -> Result<(), Error> {
        self.replica = None;
        self.applying = None;
        for events in split_events(events) {
            let mut ranges = ProfileIds::get(self)
                .await?
                .unwrap_or_else(ids::initial_ranges);
            let (id, _) = ids::incr(&mut ranges, 1)?;
            ProfileIds::set(self, ranges).await?;
            let checkpoint = Checkpoints::increment(self, replica).await?;
            let log = EventLog {
                id,
                checkpoint: Some(checkpoint),
                events,
            };
            EventLogs::insert(self, strip_records(log.clone())).await?;
            self.emit(StoreEvent::EventLog(log));
        }
        Ok(())
    }
}

/// Splits a transaction's events into logs of at most [`MAX_LOG_EVENTS`]
/// events and, unless one event is larger, [`MAX_LOG_BYTES`] bytes.
fn split_events(events: Vec<Event>) -> Vec<Vec<Event>> {
    let mut out: Vec<Vec<Event>> = Vec::new();
    let mut bytes = 0;
    for e in events {
        let len = e.encoded_len();
        match out.last_mut() {
            Some(last) if last.len() < MAX_LOG_EVENTS && bytes + len <= MAX_LOG_BYTES => {
                bytes += len;
                last.push(e);
            }
            _ => {
                bytes = len;
                out.push(vec![e]);
            }
        }
    }
    out
}

/// `log` without its events' records, as logs are stored.
pub(crate) fn strip_records(mut log: EventLog) -> EventLog {
    for e in &mut log.events {
        e.record.clear();
    }
    log
}

/// The record at `key`.
pub(crate) async fn get_record<M: Message + Default>(
    tx: &mut impl Read,
    key: &str,
) -> Result<Option<M>, Error> {
    let (kv, sk) = tx.parts();
    let Some(sealed) = kv.get(key).await? else {
        return Ok(None);
    };
    let plain = sk.open(&sealed)?;
    Ok(Some(M::decode(plain.as_slice())?))
}

/// Every record whose key starts with `prefix`, in key order.
pub(crate) async fn scan_records<M: Message + Default>(
    tx: &mut impl Read,
    prefix: &str,
) -> Result<Vec<M>, Error> {
    let (kv, sk) = tx.parts();
    let entries = kv.scan(prefix, None, usize::MAX).await?;
    let mut out = Vec::with_capacity(entries.len());
    for (_, sealed) in entries {
        let plain = sk.open(&sealed)?;
        out.push(M::decode(plain.as_slice())?);
    }
    Ok(out)
}

/// Whether any key starts with `prefix`.
pub(crate) async fn has_prefix(tx: &mut impl Read, prefix: &str) -> Result<bool, Error> {
    let (kv, _) = tx.parts();
    Ok(!kv.scan(prefix, None, 1).await?.is_empty())
}

/// The ids index `ns` maps `key` to.
pub(crate) async fn index_ids(
    tx: &mut impl Read,
    ns: Namespace,
    key: &[u8],
) -> Result<Vec<u64>, Error> {
    let prefix = {
        let (_, sk) = tx.parts();
        index_prefix(ns, sk.bytes(), key)
    };
    let entries: Vec<SecondaryIndexKey> = scan_records(tx, &prefix).await?;
    Ok(entries
        .into_iter()
        .filter(|e| e.key == key)
        .map(|e| e.id)
        .collect())
}
