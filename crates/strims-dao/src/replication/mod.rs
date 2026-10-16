// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Replication storage: applying other replicas' changes, and what to
//! send them.
//!
//! Each local transaction that writes replicated records logs its changes
//! as an [`EventLog`] stamped with the writer's [`Checkpoint`] after the
//! write. A replica's checkpoint counts, for every replica, the logs of
//! that replica it has seen, so replicas catch up by exchanging the logs
//! the other's checkpoint lacks ([`compressed_delta`]), or, for a new
//! replica, everything ([`dump`]). Logs are kept, without their records,
//! until every live replica's checkpoint covers them
//! ([`garbage_collect`]).
//!
//! A deletion wins over every write to its record, and a deleted record
//! isn't created again. Stored logs remember deletions, so once a log is
//! collected, a write from a replica that missed the deletion could bring
//! the record back; checkpoints of live replicas cover the deletion by
//! then.
//!
//! Each operation here is one transaction.

pub mod checkpoints;
pub mod delta;
pub mod filter;
mod registry;

use std::collections::{BTreeMap, BTreeSet};

use strims_base::time::Time;
use strims_kv::Kv;
use strims_proto::dao::v1::VersionVector;
use strims_proto::profile::v1::ProfileId;
use strims_proto::replication::v1::{Checkpoint, EventLog};

pub use self::delta::Bootstrap;
pub use self::registry::REPLICATED;
use crate::error::Error;
use crate::events::StoreEvent;
use crate::ids;
use crate::key::Namespace;
use crate::schema::profile::ProfileIds;
use crate::schema::system::{Checkpoints, EventLogs, is_deleted};
use crate::store::{ProfileStore, ReplicatedStore, WriteTx, strip_records};
use crate::table::{Singleton, Table};
use crate::vv;

/// The most replicas whose checkpoints a store keeps. Checkpoints arrive
/// from other replicas, so this bounds what they can make the store hold.
pub const MAX_REPLICAS: usize = 1024;

/// The union of `logs`' checkpoints.
fn union_of(logs: &[EventLog]) -> VersionVector {
    let mut v = VersionVector::default();
    for l in logs {
        if let Some(c) = l.checkpoint.as_ref().and_then(|c| c.version.as_ref()) {
            vv::upgrade(&mut v, c);
        }
    }
    v
}

/// Merges checkpoints in `tx`, refusing to track more than
/// [`MAX_REPLICAS`] replicas.
async fn merge_all<K: Kv>(
    tx: &mut WriteTx<K>,
    cs: &[Checkpoint],
) -> Result<Vec<Checkpoint>, Error> {
    let known = Checkpoints::get_all(tx).await?;
    let mut ids: std::collections::BTreeSet<u64> = known.iter().map(|c| c.id).collect();
    ids.extend(cs.iter().map(|c| c.id));
    if ids.len() > MAX_REPLICAS {
        return Err(Error::Invalid("replica count"));
    }
    let mut out = Vec::with_capacity(cs.len());
    for c in cs {
        out.push(Checkpoints::merge(tx, c).await?);
    }
    Ok(out)
}

/// Applies logs that arrived from replica `from`, and resolves to this
/// replica's checkpoint after them. Logs that advance the checkpoint go
/// out to subscribers as [`StoreEvent::LogsApplied`].
pub async fn apply_event_logs<K: Kv>(
    store: &ReplicatedStore<K>,
    from: u64,
    logs: Vec<EventLog>,
    now: Time,
) -> Result<Checkpoint, Error> {
    let mut tx = store.store().write(now).await?;
    tx.set_applying(store.replica_id());

    let own_before = Checkpoints::get(&mut tx, store.replica_id())
        .await?
        .and_then(|c| c.version)
        .unwrap_or_default();
    let mut deleted = BTreeSet::new();
    for (ns, id) in filter::keys(&logs) {
        if is_deleted(&mut tx, Namespace(ns), id).await? {
            deleted.insert((ns, id));
        }
    }
    for e in filter::filter_incoming(&deleted, &logs) {
        registry::apply(&mut tx, &e).await?;
    }

    let mut origins: BTreeMap<u64, VersionVector> = BTreeMap::new();
    for l in &logs {
        let v = origins.entry(delta::origin(l)).or_default();
        if let Some(c) = l.checkpoint.as_ref().and_then(|c| c.version.as_ref()) {
            vv::upgrade(v, c);
        }
    }
    let replica_checkpoints: Vec<Checkpoint> = origins
        .into_iter()
        .map(|(id, version)| Checkpoint {
            id,
            version: Some(version),
            deleted: false,
        })
        .collect();
    merge_all(&mut tx, &replica_checkpoints).await?;
    let own = Checkpoints::merge(
        &mut tx,
        &Checkpoint {
            id: store.replica_id(),
            version: Some(union_of(&logs)),
            deleted: false,
        },
    )
    .await?;

    for l in &logs {
        EventLogs::insert(&mut tx, strip_records(l.clone())).await?;
    }
    let advanced: Vec<EventLog> = logs
        .into_iter()
        .filter(|l| delta::local_version(l) > vv::get(&own_before, delta::origin(l)))
        .collect();
    if !advanced.is_empty() {
        tx.emit(StoreEvent::LogsApplied {
            from,
            logs: advanced,
        });
    }
    tx.commit().await?;
    Ok(own)
}

/// Applies a bootstrap, or a part of one, and resolves to this replica's
/// checkpoint after it.
pub async fn apply_bootstrap<K: Kv>(
    store: &ReplicatedStore<K>,
    b: Bootstrap,
    now: Time,
) -> Result<Checkpoint, Error> {
    let mut tx = store.store().write(now).await?;
    tx.set_applying(store.replica_id());
    merge_all(&mut tx, &b.checkpoints).await?;
    for l in b.logs {
        EventLogs::insert(&mut tx, strip_records(l)).await?;
    }
    for e in &b.events {
        if !is_deleted(&mut tx, Namespace(e.namespace), e.id).await? {
            registry::apply(&mut tx, e).await?;
        }
    }
    let mut union = VersionVector::default();
    for c in &b.checkpoints {
        if let Some(v) = &c.version {
            vv::upgrade(&mut union, v);
        }
    }
    let own = Checkpoints::merge(
        &mut tx,
        &Checkpoint {
            id: store.replica_id(),
            version: Some(union),
            deleted: false,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(own)
}

/// Everything a new replica needs: every replicated record, every stored
/// log and every checkpoint.
pub async fn dump<K: Kv>(store: &ProfileStore<K>) -> Result<Bootstrap, Error> {
    let mut tx = store.read().await?;
    Ok(Bootstrap {
        events: registry::dump(&mut tx).await?,
        logs: EventLogs::get_all(&mut tx).await?,
        checkpoints: Checkpoints::get_all(&mut tx).await?,
    })
}

/// The logs a replica at `remote` lacks, compressed, with each write
/// carrying its record's current state, in causal order. Writes to records
/// this replica doesn't hold are left out: their deletions travel as
/// delete events, and a record can also be missing because it lost a
/// unique key to another.
pub async fn compressed_delta<K: Kv>(
    store: &ProfileStore<K>,
    remote: &VersionVector,
) -> Result<Vec<EventLog>, Error> {
    let mut tx = store.read().await?;
    let logs: Vec<EventLog> = EventLogs::get_all(&mut tx)
        .await?
        .into_iter()
        .filter(|l| delta::after(l, remote))
        .collect();
    let mut out = Vec::new();
    for mut l in delta::compress(logs) {
        let mut events = Vec::with_capacity(l.events.len());
        for e in l.events {
            if e.delete {
                events.push(e);
            } else if let Some(current) =
                registry::load(&mut tx, Namespace(e.namespace), e.id).await?
            {
                events.push(current);
            }
        }
        l.events = events;
        if !l.events.is_empty() {
            out.push(l);
        }
    }
    delta::causal_order(&mut out);
    Ok(out)
}

/// Deletes the stored logs `threshold` covers. Resolves to how many.
pub async fn garbage_collect<K: Kv>(
    store: &ProfileStore<K>,
    threshold: &VersionVector,
    now: Time,
) -> Result<usize, Error> {
    let mut tx = store.write(now).await?;
    let mut n = 0;
    for l in EventLogs::get_all(&mut tx).await? {
        if let Some(&v) = threshold.value.get(&delta::origin(&l))
            && delta::local_version(&l) <= v
        {
            EventLogs::delete(&mut tx, l.id).await?;
            n += 1;
        }
    }
    tx.commit().await?;
    Ok(n)
}

/// Every replica's checkpoint.
pub async fn checkpoints<K: Kv>(store: &ProfileStore<K>) -> Result<Vec<Checkpoint>, Error> {
    let mut tx = store.read().await?;
    Checkpoints::get_all(&mut tx).await
}

/// Replica `id`'s checkpoint.
pub async fn checkpoint<K: Kv>(
    store: &ProfileStore<K>,
    id: u64,
) -> Result<Option<Checkpoint>, Error> {
    let mut tx = store.read().await?;
    Checkpoints::get(&mut tx, id).await
}

/// Merges checkpoints from another replica.
pub async fn merge_checkpoints<K: Kv>(
    store: &ProfileStore<K>,
    cs: &[Checkpoint],
    now: Time,
) -> Result<Vec<Checkpoint>, Error> {
    let mut tx = store.write(now).await?;
    let out = merge_all(&mut tx, cs).await?;
    tx.commit().await?;
    Ok(out)
}

/// The number of ids this replica can still allocate.
pub async fn free_ids<K: Kv>(store: &ProfileStore<K>) -> Result<u64, Error> {
    let mut tx = store.read().await?;
    Ok(ProfileIds::get(&mut tx)
        .await?
        .map_or(0, |p| ids::free_count(&p)))
}

/// Gives up to `max` of this replica's ids, and at most half of them, to
/// another replica. `None` when there are none to give.
pub async fn allocate_ids<K: Kv>(
    store: &ProfileStore<K>,
    max: u64,
    now: Time,
) -> Result<Option<ProfileId>, Error> {
    let mut tx = store.write(now).await?;
    let mut ranges = ProfileIds::get(&mut tx)
        .await?
        .unwrap_or_else(ids::initial_ranges);
    let n = max.min(ids::free_count(&ranges) / 2);
    let Some(popped) = ids::pop(&mut ranges, n)? else {
        return Ok(None);
    };
    ProfileIds::set(&mut tx, ranges).await?;
    tx.commit().await?;
    Ok(Some(popped))
}

/// Adds ids another replica gave this one.
pub async fn push_ids<K: Kv>(
    store: &ProfileStore<K>,
    more: ProfileId,
    now: Time,
) -> Result<(), Error> {
    ids::validate(&more)?;
    let mut tx = store.write(now).await?;
    let mut ranges = ProfileIds::get(&mut tx)
        .await?
        .unwrap_or_else(ids::initial_ranges);
    ids::push(&mut ranges, more)?;
    ProfileIds::set(&mut tx, ranges).await?;
    tx.commit().await
}

#[cfg(test)]
mod tests;
