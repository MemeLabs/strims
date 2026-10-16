// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The store's own records: its format version and replication state.

use futures_util::future::LocalBoxFuture;
use strims_kv::Kv;
use strims_proto::dao::v1::{StoreVersion, VersionVector};
use strims_proto::replication::v1::{Checkpoint, EventLog};

use crate::error::Error;
use crate::events::Record;
use crate::key::{Namespace, Part};
use crate::store::{Read, WriteTx, has_prefix};
use crate::table::{Singleton, Table};
use crate::vv;

/// The store's format version.
pub struct StoreVersions;

impl Singleton for StoreVersions {
    type Record = StoreVersion;
    const NS: Namespace = Namespace(1002);
}

/// Replication logs: the changes each local transaction made, and the
/// logs received from other replicas, with their events' records
/// stripped. A stored log keeps a tombstone for each record it deletes
/// ([`is_deleted`]).
pub struct EventLogs;

impl Table for EventLogs {
    type Record = EventLog;
    const NS: Namespace = Namespace(1003);

    fn id(r: &EventLog) -> u64 {
        r.id
    }

    fn on_change<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        m: &'a mut EventLog,
        _p: Option<&'a EventLog>,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            for e in m.events.iter().filter(|e| e.delete) {
                tx.put_record(&tombstone_key(e.namespace, e.id, m.id), &())
                    .await?;
            }
            Ok(())
        })
    }

    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a EventLog,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            for e in p.events.iter().filter(|e| e.delete) {
                tx.delete_key(&tombstone_key(e.namespace, e.id, p.id))
                    .await?;
            }
            Ok(())
        })
    }
}

/// The namespace of tombstones: `<ns>:<record ns>:<record id>:<log id>`.
const TOMBSTONES: Namespace = Namespace(1005);

fn tombstone_key(ns: i64, id: u64, log: u64) -> String {
    TOMBSTONES.format(&[
        Part::Str(&Namespace(ns).to_string()),
        Part::Id(id),
        Part::Id(log),
    ])
}

/// Whether a stored log deletes record `id` of namespace `ns`. Tombstones
/// last as long as their logs, which every live replica has seen by the
/// time they are collected.
pub async fn is_deleted<R: Read>(tx: &mut R, ns: Namespace, id: u64) -> Result<bool, Error> {
    let prefix = TOMBSTONES.format_prefix(&[Part::Str(&ns.to_string()), Part::Id(id)]);
    has_prefix(tx, &prefix).await
}

/// Each replica's checkpoint: the logs of every replica it has seen.
/// Checkpoints only grow, and a deleted replica's checkpoint stays
/// deleted.
pub struct Checkpoints;

impl Table for Checkpoints {
    type Record = Checkpoint;
    const NS: Namespace = Namespace(1004);

    fn id(r: &Checkpoint) -> u64 {
        r.id
    }

    fn observe(r: &Checkpoint) -> Option<Record> {
        Some(Record::Checkpoint(r.clone()))
    }

    fn on_change<'a, K: Kv>(
        _tx: &'a mut WriteTx<K>,
        m: &'a mut Checkpoint,
        p: Option<&'a Checkpoint>,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        if let Some(p) = p {
            if let Some(pv) = &p.version {
                vv::upgrade(m.version.get_or_insert_with(VersionVector::default), pv);
            }
            m.deleted |= p.deleted;
        }
        Box::pin(async { Ok(()) })
    }
}

impl Checkpoints {
    /// Records one more log by `replica` in its own checkpoint.
    pub async fn increment<K: Kv>(tx: &mut WriteTx<K>, replica: u64) -> Result<Checkpoint, Error> {
        let now = tx.now();
        Checkpoints::transform(tx, replica, |c| {
            c.id = replica;
            vv::increment(
                c.version.get_or_insert_with(VersionVector::default),
                replica,
                now,
            );
            Ok(())
        })
        .await
    }

    /// Merges `c` into its replica's checkpoint.
    pub async fn merge<K: Kv>(tx: &mut WriteTx<K>, c: &Checkpoint) -> Result<Checkpoint, Error> {
        Checkpoints::transform(tx, c.id, |m| {
            m.id = c.id;
            if let Some(v) = &c.version {
                vv::upgrade(m.version.get_or_insert_with(VersionVector::default), v);
            }
            m.deleted |= c.deleted;
            Ok(())
        })
        .await
    }
}
