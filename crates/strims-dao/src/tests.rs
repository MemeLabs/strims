// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Store and record machinery, on test tables.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::future::Future;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;
use futures_util::{FutureExt, StreamExt};
use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_kv::{Kv, MemoryKv, ReadTx as _};
use strims_proto::chat::v1::UiConfigHighlight;
use strims_proto::dao::v1::{SecondaryIndexKey, StoreVersion};
use strims_proto::profile::v1 as profile;
use strims_proto::replication::v1::Checkpoint;

use crate::events::{Record, Recv, StoreEvent, Subscription};
use crate::key::{Namespace, profile_table};
use crate::schema::profile::{Devices, Profile, ProfileIds};
use crate::schema::system::{Checkpoints, EventLogs, StoreVersions};
use crate::seal::StorageKey;
use crate::store::{MAX_LOG_EVENTS, ProfileStore, ReplicatedStore, STORE_VERSION, WriteTx};
use crate::table::{Index, Replication, Singleton, Table, UniqueConflict};
use crate::{Error, lease};

fn block<T>(f: impl Future<Output = T>) -> T {
    f.now_or_never()
        .expect("memory store operations are ready immediately")
}

const T0: Time = Time::from_unix_millis(1_700_000_000_000);

fn key() -> StorageKey {
    StorageKey::from_bytes([9; 32])
}

fn new_store(kv: &MemoryKv) -> ProfileStore<MemoryKv> {
    block(ProfileStore::create(kv.clone(), 1, key(), det_rng(1), T0)).unwrap()
}

fn hl(id: u64, alias: &str, peer_key: &[u8]) -> UiConfigHighlight {
    UiConfigHighlight {
        id,
        alias: alias.into(),
        peer_key: peer_key.to_vec(),
        ..Default::default()
    }
}

/// Records indexed by alias, uniquely by non-empty peer key (rejecting
/// conflicts), replicated.
struct Things;

impl Things {
    const BY_ALIAS: Index<UiConfigHighlight> =
        Index::new(Namespace(90002), |r| r.alias.as_bytes().to_vec());
    const BY_KEY: Index<UiConfigHighlight> =
        Index::unique(Namespace(90003), |r: &UiConfigHighlight| r.peer_key.clone())
            .when(|r| !r.peer_key.is_empty());
}

impl Table for Things {
    type Record = UiConfigHighlight;
    const NS: Namespace = Namespace(90001);
    const INDEXES: &'static [Index<UiConfigHighlight>] = &[Things::BY_ALIAS, Things::BY_KEY];
    const REPLICATION: Option<Replication<UiConfigHighlight>> = Some(Replication::DEFAULT);

    fn id(r: &UiConfigHighlight) -> u64 {
        r.id
    }

    fn observe(r: &UiConfigHighlight) -> Option<Record> {
        Some(Record::ChatUiConfigHighlight(r.clone()))
    }

    fn on_change<'a, K: Kv>(
        _tx: &'a mut WriteTx<K>,
        m: &'a mut UiConfigHighlight,
        _p: Option<&'a UiConfigHighlight>,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        if m.alias == "fail" {
            return Box::pin(async { Err(Error::Invalid("alias")) });
        }
        m.alias = m.alias.trim().to_owned();
        Box::pin(async { Ok(()) })
    }

    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a UiConfigHighlight,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            Tombstones::insert(tx, hl(p.id, "deleted", b"")).await?;
            Ok(())
        })
    }
}

/// Records whose unique peer key moves to the newest writer.
struct Replacing;

impl Replacing {
    const BY_KEY: Index<UiConfigHighlight> =
        Index::unique(Namespace(90005), |r| r.peer_key.clone());
}

impl Table for Replacing {
    type Record = UiConfigHighlight;
    const NS: Namespace = Namespace(90004);
    const INDEXES: &'static [Index<UiConfigHighlight>] = &[Replacing::BY_KEY];

    fn id(r: &UiConfigHighlight) -> u64 {
        r.id
    }

    fn on_unique_conflict(
        _: &Index<UiConfigHighlight>,
        m: &mut UiConfigHighlight,
        existing: &UiConfigHighlight,
    ) -> UniqueConflict {
        m.alias = format!("{}+{}", m.alias, existing.alias);
        UniqueConflict::Replace
    }

    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a UiConfigHighlight,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            Tombstones::insert(tx, hl(p.id, "deleted", b"")).await?;
            Ok(())
        })
    }
}

/// Records whose unique peer key moves to the newest writer, which leaves
/// the older record unindexed under it.
struct Stealing;

impl Stealing {
    const BY_KEY: Index<UiConfigHighlight> =
        Index::unique(Namespace(90008), |r| r.peer_key.clone());
}

impl Table for Stealing {
    type Record = UiConfigHighlight;
    const NS: Namespace = Namespace(90007);
    const INDEXES: &'static [Index<UiConfigHighlight>] = &[Stealing::BY_KEY];

    fn id(r: &UiConfigHighlight) -> u64 {
        r.id
    }

    fn on_unique_conflict(
        _: &Index<UiConfigHighlight>,
        _: &mut UiConfigHighlight,
        _: &UiConfigHighlight,
    ) -> UniqueConflict {
        UniqueConflict::Steal
    }
}

/// Where the test tables' delete hooks leave a mark.
struct Tombstones;

impl Table for Tombstones {
    type Record = UiConfigHighlight;
    const NS: Namespace = Namespace(90006);

    fn id(r: &UiConfigHighlight) -> u64 {
        r.id
    }
}

fn drain(s: &mut Subscription) -> Vec<StoreEvent> {
    let mut out = Vec::new();
    while let Some(Some(r)) = s.next().now_or_never() {
        match r {
            Recv::Event(e) => out.push(Rc::unwrap_or_clone(e)),
            Recv::Lagged => panic!("lagged"),
        }
    }
    out
}

fn raw_keys(kv: &MemoryKv, prefix: &str) -> Vec<String> {
    kv.dump()[&profile_table(1)]
        .keys()
        .filter(|k| k.starts_with(prefix))
        .cloned()
        .collect()
}

/// A store whose profile has device 77, opened for replication.
fn replicated(kv: &MemoryKv) -> ReplicatedStore<MemoryKv> {
    let store = new_store(kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Profile::set(
            &mut tx,
            profile::Profile {
                id: 1,
                device_id: 77,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        ReplicatedStore::new(store).await.unwrap()
    })
}

#[test]
fn stores_open_only_at_the_current_version() {
    let kv = MemoryKv::new();
    assert!(matches!(
        block(ProfileStore::open(kv.clone(), 1, key(), det_rng(1))),
        Err(Error::Kv(strims_kv::Error::NoTable))
    ));
    let store = new_store(&kv);
    assert!(matches!(
        block(ProfileStore::create(kv.clone(), 1, key(), det_rng(1), T0)),
        Err(Error::StoreExists)
    ));
    block(ProfileStore::open(kv.clone(), 1, key(), det_rng(2))).unwrap();

    block(async {
        let mut tx = store.write(T0).await.unwrap();
        StoreVersions::set(&mut tx, StoreVersion { version: 6 })
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    assert!(matches!(
        block(ProfileStore::open(kv.clone(), 1, key(), det_rng(1))),
        Err(Error::UnsupportedStoreVersion { have: 6 })
    ));
    assert_eq!(STORE_VERSION, 7);
}

#[test]
fn values_are_sealed_with_the_store_key() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Tombstones::insert(&mut tx, hl(5, "visible?", b""))
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    let sealed = kv.dump()[&profile_table(1)][&Tombstones::NS.record_key(5)].clone();
    assert!(!sealed.windows(8).any(|w| w == b"visible?"));
    let plain = key().open(&sealed).unwrap();
    assert_eq!(
        UiConfigHighlight::decode(plain.as_slice()).unwrap().alias,
        "visible?"
    );

    let other = block(ProfileStore::open(
        kv.clone(),
        1,
        StorageKey::from_bytes([8; 32]),
        det_rng(1),
    ));
    assert!(matches!(other, Err(Error::Seal(_))));
}

#[test]
fn table_operations() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        assert_eq!(Things::get(&mut tx, 1).await.unwrap(), None);
        assert!(matches!(
            Things::update(&mut tx, hl(1, "a", b"")).await,
            Err(Error::NotFound)
        ));
        Things::insert(&mut tx, hl(2, " b ", b"")).await.unwrap();
        Things::upsert(&mut tx, hl(1, "a", b"")).await.unwrap();
        assert_eq!(Things::get(&mut tx, 2).await.unwrap().unwrap().alias, "b");
        let all: Vec<u64> = Things::get_all(&mut tx)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(all, [1, 2]);

        let t = Things::transform(&mut tx, 1, |r| {
            r.alias = "a2".into();
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(t.alias, "a2");
        let created = Things::transform(&mut tx, 3, |r| {
            r.id = 3;
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(created.id, 3);

        assert!(Things::delete(&mut tx, 3).await.unwrap());
        assert!(!Things::delete(&mut tx, 3).await.unwrap());
        assert_eq!(
            Tombstones::get(&mut tx, 3).await.unwrap().unwrap().alias,
            "deleted"
        );
        assert!(matches!(
            Things::insert(&mut tx, hl(4, "fail", b"")).await,
            Err(Error::Invalid(_))
        ));
        tx.commit().await.unwrap();
    });
}

#[test]
fn events_follow_commit() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut sub = store.events().subscribe();
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Things::insert(&mut tx, hl(1, "a", b"")).await.unwrap();
        Things::update(&mut tx, hl(1, "a", b"")).await.unwrap();
        assert!(drain(&mut sub).is_empty());
        tx.commit().await.unwrap();
    });
    assert_eq!(
        drain(&mut sub),
        [StoreEvent::Changed(Record::ChatUiConfigHighlight(hl(
            1, "a", b""
        )))]
    );

    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Things::delete(&mut tx, 1).await.unwrap();
        Things::insert(&mut tx, hl(2, "b", b"")).await.unwrap();
        drop(tx);
        let mut tx = store.read().await.unwrap();
        assert!(Things::get(&mut tx, 1).await.unwrap().is_some());
        assert!(Things::get(&mut tx, 2).await.unwrap().is_none());
    });
    assert!(drain(&mut sub).is_empty());

    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Things::delete(&mut tx, 1).await.unwrap();
        tx.commit().await.unwrap();
    });
    assert_eq!(
        drain(&mut sub),
        [StoreEvent::Deleted(Record::ChatUiConfigHighlight(hl(
            1, "a", b""
        )))]
    );
}

#[test]
fn indexes_follow_their_records() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Things::insert(&mut tx, hl(1, "x", b"k1")).await.unwrap();
        Things::insert(&mut tx, hl(2, "x", b"")).await.unwrap();
        assert_eq!(
            Things::index_ids(&mut tx, &Things::BY_ALIAS, b"x")
                .await
                .unwrap(),
            [1, 2]
        );
        assert_eq!(
            Things::index_get(&mut tx, &Things::BY_KEY, b"k1")
                .await
                .unwrap()
                .unwrap()
                .id,
            1
        );
        assert!(
            Things::index_ids(&mut tx, &Things::BY_KEY, b"")
                .await
                .unwrap()
                .is_empty()
        );

        Things::update(&mut tx, hl(1, "y", b"k2")).await.unwrap();
        Things::update(&mut tx, hl(2, "x", b"k3")).await.unwrap();
        assert_eq!(
            Things::index_ids(&mut tx, &Things::BY_ALIAS, b"x")
                .await
                .unwrap(),
            [2]
        );
        assert_eq!(
            Things::index_ids(&mut tx, &Things::BY_ALIAS, b"y")
                .await
                .unwrap(),
            [1]
        );
        assert!(
            Things::index_get(&mut tx, &Things::BY_KEY, b"k1")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            Things::index_get(&mut tx, &Things::BY_KEY, b"k3")
                .await
                .unwrap()
                .unwrap()
                .id,
            2
        );

        Things::update(&mut tx, hl(2, "x", b"")).await.unwrap();
        assert!(
            Things::index_ids(&mut tx, &Things::BY_KEY, b"k3")
                .await
                .unwrap()
                .is_empty()
        );
        Things::delete(&mut tx, 1).await.unwrap();
        assert!(
            Things::index_ids(&mut tx, &Things::BY_ALIAS, b"y")
                .await
                .unwrap()
                .is_empty()
        );
        let names = Things::index_get_all(&mut tx, &Things::BY_ALIAS, b"x")
            .await
            .unwrap();
        assert_eq!(names.len(), 1);
        tx.commit().await.unwrap();
    });

    // One alias entry for record 2, and nothing else.
    assert_eq!(raw_keys(&kv, &Things::BY_ALIAS.ns.prefix()).len(), 1);
    assert_eq!(raw_keys(&kv, &Things::BY_KEY.ns.prefix()).len(), 0);
    let entry = kv.dump()[&profile_table(1)]
        .iter()
        .find(|(k, _)| k.starts_with(&Things::BY_ALIAS.ns.prefix()))
        .map(|(_, v)| v.clone())
        .unwrap();
    let entry = SecondaryIndexKey::decode(key().open(&entry).unwrap().as_slice()).unwrap();
    assert_eq!(
        entry,
        SecondaryIndexKey {
            key: b"x".to_vec(),
            id: 2
        }
    );
}

#[test]
fn unique_conflicts_reject_or_replace() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Things::insert(&mut tx, hl(1, "a", b"k")).await.unwrap();
        assert!(matches!(
            Things::insert(&mut tx, hl(2, "b", b"k")).await,
            Err(Error::UniqueViolation(Namespace(90003)))
        ));
        Things::upsert(&mut tx, hl(1, "a2", b"k")).await.unwrap();

        Replacing::insert(&mut tx, hl(1, "old", b"k"))
            .await
            .unwrap();
        let m = Replacing::insert(&mut tx, hl(2, "new", b"k"))
            .await
            .unwrap();
        assert_eq!(m.alias, "new+old");
        assert!(Replacing::get(&mut tx, 1).await.unwrap().is_none());
        assert_eq!(
            Replacing::index_ids(&mut tx, &Replacing::BY_KEY, b"k")
                .await
                .unwrap(),
            [2]
        );
        assert!(Tombstones::get(&mut tx, 1).await.unwrap().is_none());
        tx.commit().await.unwrap();
    });
}

#[test]
fn stolen_unique_keys_leave_the_other_record_unindexed() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Stealing::insert(&mut tx, hl(1, "old", b"k")).await.unwrap();
        Stealing::insert(&mut tx, hl(2, "new", b"k")).await.unwrap();
        let ids = |tx: &mut WriteTx<MemoryKv>| {
            Stealing::index_ids(tx, &Stealing::BY_KEY, b"k")
                .now_or_never()
                .unwrap()
                .unwrap()
        };
        assert_eq!(ids(&mut tx), [2]);
        assert_eq!(
            Stealing::get(&mut tx, 1).await.unwrap().unwrap().alias,
            "old"
        );

        // the older record's later delete leaves the stolen entry alone
        Stealing::delete(&mut tx, 1).await.unwrap();
        assert_eq!(ids(&mut tx), [2]);
        tx.commit().await.unwrap();
    });
}

#[test]
fn applied_unique_conflicts_keep_the_lower_id() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut sub = store.events().subscribe();
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        tx.set_applying(77);
        Things::insert(&mut tx, hl(5, "a", b"k")).await.unwrap();
        assert!(matches!(
            Things::insert(&mut tx, hl(9, "b", b"k")).await,
            Err(Error::Superseded)
        ));
        Things::insert(&mut tx, hl(3, "c", b"k")).await.unwrap();
        assert!(Things::get(&mut tx, 5).await.unwrap().is_none());
        assert_eq!(
            Things::index_ids(&mut tx, &Things::BY_KEY, b"k")
                .await
                .unwrap(),
            [3]
        );
        tx.commit().await.unwrap();
    });
    // The losers' deletions replicate as replica 77's.
    let log = drain(&mut sub)
        .into_iter()
        .find_map(|e| match e {
            StoreEvent::EventLog(l) => Some(l),
            _ => None,
        })
        .unwrap();
    assert_eq!(log.checkpoint.unwrap().id, 77);
    let deleted: Vec<(u64, bool)> = log.events.iter().map(|e| (e.id, e.delete)).collect();
    assert_eq!(deleted, [(9, true), (5, true)]);
}

#[test]
fn nested_transactions_are_refused() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let tx = store.write(T0).await.unwrap();
        assert!(matches!(
            store.read().await,
            Err(Error::Kv(strims_kv::Error::Busy))
        ));
        assert_eq!(kv.open_transactions(), 1);
        drop(tx);
        assert_eq!(kv.open_transactions(), 0);
    });
}

#[test]
fn replicated_writes_are_versioned_and_logged() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    let now = T0 + Duration::SECOND;
    block(async {
        let mut tx = store.write(now).await.unwrap();
        Things::insert(&mut tx, hl(1, "a", b"")).await.unwrap();
        Things::update(&mut tx, hl(1, "b", b"")).await.unwrap();
        Things::insert(&mut tx, hl(2, "c", b"")).await.unwrap();
        Things::delete(&mut tx, 2).await.unwrap();
        Tombstones::insert(&mut tx, hl(3, "unreplicated", b""))
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });

    let events = drain(&mut sub);
    let StoreEvent::EventLog(log) = events.last().unwrap() else {
        panic!("{events:?}");
    };
    let checkpoint = log.checkpoint.clone().unwrap();
    assert_eq!(checkpoint.id, 77);
    assert_eq!(checkpoint.version.as_ref().unwrap().value[&77], 1);
    let summary: Vec<(u64, bool, u64)> = log
        .events
        .iter()
        .map(|e| (e.id, e.delete, e.version.as_ref().unwrap().value[&77]))
        .collect();
    assert_eq!(
        summary,
        [(1, false, 1), (1, false, 2), (2, false, 1), (2, true, 2)]
    );
    let first = UiConfigHighlight::decode(log.events[0].record.as_slice()).unwrap();
    assert_eq!(first.alias, "a");
    assert_eq!(
        log.events[0].version.as_ref().unwrap().updated_at,
        now.unix_millis()
    );

    block(async {
        let mut tx = store.read().await.unwrap();
        let stored = Things::get(&mut tx, 1).await.unwrap().unwrap();
        assert_eq!(stored.version.unwrap().value[&77], 2);
        assert!(
            Tombstones::get(&mut tx, 3)
                .await
                .unwrap()
                .unwrap()
                .version
                .is_none()
        );
        let logs = EventLogs::get_all(&mut tx).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].events.iter().all(|e| e.record.is_empty()));
        assert_eq!(logs[0].id, log.id);
        let c = Checkpoints::get(&mut tx, 77).await.unwrap().unwrap();
        assert_eq!(c, checkpoint);
        let ranges = ProfileIds::get(&mut tx).await.unwrap().unwrap();
        assert_eq!(ranges.next_id, log.id + 1);
    });

    // Unreplicated writes and replicated transactions without replicated
    // changes log nothing.
    block(async {
        let mut tx = store.store().write(now).await.unwrap();
        Things::transform(&mut tx, 1, |r| {
            r.alias = "d".into();
            Ok(())
        })
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let mut tx = store.write(now).await.unwrap();
        Tombstones::insert(&mut tx, hl(4, "x", b"")).await.unwrap();
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        assert_eq!(EventLogs::get_all(&mut tx).await.unwrap().len(), 1);
        let stored = Things::get(&mut tx, 1).await.unwrap().unwrap();
        assert_eq!(stored.version.unwrap().value[&77], 2);
    });
}

#[test]
fn large_transactions_split_their_logs() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let n = MAX_LOG_EVENTS as u64 + 10;
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        for id in 1..=n {
            Things::insert(&mut tx, hl(id, "a", b"")).await.unwrap();
        }
        tx.commit().await.unwrap();
    });
    let logs = block(async {
        let mut tx = store.read().await.unwrap();
        EventLogs::get_all(&mut tx).await.unwrap()
    });
    let mut sizes: Vec<(u64, usize)> = logs
        .iter()
        .map(|l| {
            (
                l.checkpoint
                    .as_ref()
                    .unwrap()
                    .version
                    .as_ref()
                    .unwrap()
                    .value[&77],
                l.events.len(),
            )
        })
        .collect();
    sizes.sort();
    assert_eq!(sizes, [(1, MAX_LOG_EVENTS), (2, 10)]);
}

#[test]
fn ids_are_unique_across_rolled_back_reservations() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut seen = std::collections::BTreeSet::new();
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        let first = tx.generate_id().await.unwrap();
        drop(tx);
        let mut tx = store.write(T0).await.unwrap();
        assert_eq!(tx.generate_id().await.unwrap(), first);
        tx.commit().await.unwrap();
        seen.insert(first);
        for _ in 0..250 {
            let mut tx = store.write(T0).await.unwrap();
            assert!(seen.insert(tx.generate_id().await.unwrap()));
            if seen.len() % 7 == 0 {
                drop(tx);
            } else {
                tx.commit().await.unwrap();
            }
        }
    });
    assert_eq!(seen.first(), Some(&1));
}

#[test]
fn deleting_a_device_tombstones_its_checkpoint() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        Devices::insert(
            &mut tx,
            profile::Device {
                id: 5,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        Checkpoints::merge(
            &mut tx,
            &Checkpoint {
                id: 5,
                version: Some(crate::vv::seed(5, T0)),
                deleted: false,
            },
        )
        .await
        .unwrap();
        Devices::delete(&mut tx, 5).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = store.read().await.unwrap();
        let c = Checkpoints::get(&mut tx, 5).await.unwrap().unwrap();
        assert!(c.deleted);
        assert_eq!(c.version.unwrap().value[&5], 1);
    });
}

#[test]
fn checkpoints_only_grow() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        let v = |entries: &[(u64, u64)]| {
            Some(strims_proto::dao::v1::VersionVector {
                value: entries.iter().copied().collect(),
                updated_at: 0,
            })
        };
        Checkpoints::merge(
            &mut tx,
            &Checkpoint {
                id: 1,
                version: v(&[(1, 5), (2, 1)]),
                deleted: true,
            },
        )
        .await
        .unwrap();
        let c = Checkpoints::merge(
            &mut tx,
            &Checkpoint {
                id: 1,
                version: v(&[(1, 3), (3, 2)]),
                deleted: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(c.version, v(&[(1, 5), (2, 1), (3, 2)]));
        assert!(c.deleted);
        let c = Checkpoints::upsert(
            &mut tx,
            Checkpoint {
                id: 1,
                version: v(&[]),
                deleted: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(c.version, v(&[(1, 5), (2, 1), (3, 2)]));
        assert!(c.deleted);
    });
}

#[test]
fn singletons_default_and_skip_equal_writes() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        assert_eq!(
            ProfileIds::get(&mut tx).await.unwrap(),
            Some(crate::ids::initial_ranges())
        );
        assert_eq!(Profile::get(&mut tx).await.unwrap(), None);
        ProfileIds::set(&mut tx, crate::ids::initial_ranges())
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    assert!(raw_keys(&kv, &ProfileIds::NS.key()).is_empty());
}

#[test]
fn leases_exclude_other_holders() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut rng = det_rng(5);
    let (a, b) = (lease::new_token(&mut rng), lease::new_token(&mut rng));
    let k = lease::key(&["replication"]);
    block(async {
        lease::try_lock(&store, &k, &a, T0).await.unwrap();
        assert!(matches!(
            lease::try_lock(&store, &k, &b, T0).await,
            Err(Error::LeaseBusy)
        ));
        assert!(!lease::release(&store, &k, &b, T0).await.unwrap());
        lease::try_lock(&store, &k, &b, T0 + lease::TTL + Duration::SECOND)
            .await
            .unwrap();
        assert!(!lease::release(&store, &k, &a, T0).await.unwrap());
        assert!(lease::release(&store, &k, &b, T0).await.unwrap());
        lease::try_lock(&store, &k, &a, T0).await.unwrap();
    });
    let mut tx = block(kv.begin_read(&profile_table(1))).unwrap();
    assert!(block(tx.get("rt:replication")).unwrap().is_some());
}
