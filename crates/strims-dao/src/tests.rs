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

use data_encoding::BASE64URL_NOPAD;
use futures_util::future::LocalBoxFuture;
use futures_util::{FutureExt, StreamExt};
use proptest::prelude::*;
use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, certificate_body, certificate_parent, generate_key, new_certificate_request,
    sign_certificate_request_with_network,
};
use strims_kv::{Kv, MemoryKv, ReadTx as _};
use strims_proto::autoseed::v1::{Config as AutoseedConfigRecord, Rule};
use strims_proto::chat::v1::{self as chat, UiConfigHighlight};
use strims_proto::dao::v1::{SecondaryIndexKey, StoreVersion};
use strims_proto::network::v1::bootstrap;
use strims_proto::network::v1::directory::{Listing, ListingSnippet, UserModeration, listing};
use strims_proto::network::v1::{Network, ServerConfig};
use strims_proto::notification::v1::Notification;
use strims_proto::notification::v1::notification::Status;
use strims_proto::profile::v1 as profile;
use strims_proto::replication::v1::Checkpoint;
use strims_proto::types::{Cache, CacheMeta, Certificate, CertificateBody, Key, KeyUsage, cache};
use strims_proto::video::v1::video_channel::{self, Owner};
use strims_proto::video::v1::{HlsEgressConfig, VideoChannel, VideoIngressConfig};
use strims_proto::vnic::v1::Config as VnicConfigRecord;

use crate::events::{Record, Recv, StoreEvent, Subscription};
use crate::key::{Namespace, profile_table};
use crate::schema::autoseed::{AutoseedConfig, AutoseedRules, new_rule};
use crate::schema::chat::{
    ChatEmotes, ChatModifiers, ChatProfiles, ChatServerIcons, ChatServers, ChatTags,
    profile_peer_key,
};
use crate::schema::directory::{
    DirectoryListingRecords, DirectoryUserRecords, listing_key, new_listing_record, new_user_record,
};
use crate::schema::network::{
    AliasReservations, BootstrapConfig, CertificateLogs, NetworkPeers, NetworkUiConfig, Networks,
    RESERVED_FOREVER, network_key, network_scoped_key, new_certificate_log,
};
use crate::schema::notification::{
    MAX_NOTIFICATIONS, Notifications, dispatch, network_subject, new_notification,
};
use crate::schema::profile::{Devices, Profile, ProfileIds};
use crate::schema::swarm::{
    MAX_SWARM_CACHES, SwarmCacheMetas, SwarmCaches, cache_checksum, delete_cache, get_cache,
    set_cache,
};
use crate::schema::system::{Checkpoints, EventLogs, StoreVersions};
use crate::schema::video::{
    DEFAULT_INGRESS_SERVER_ADDR, HlsConfig, IngressConfig, StreamKeyError, TOKEN_SIZE,
    VideoChannels, channel_by_stream_key, delete_for_network, format_stream_key, new_channel,
    owner_key, parse_stream_key,
};
use crate::schema::vnic::VnicConfig;
use crate::seal::StorageKey;
use crate::store::{MAX_LOG_EVENTS, ProfileStore, ReplicatedStore, STORE_VERSION, WriteTx};
use crate::table::{Index, Replication, Singleton, Table, UniqueConflict, ref_key};
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
        StoreVersions::set(&mut tx, StoreVersion { version: 7 })
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    assert!(matches!(
        block(ProfileStore::open(kv.clone(), 1, key(), det_rng(1))),
        Err(Error::UnsupportedStoreVersion { have: 7 })
    ));
    assert_eq!(STORE_VERSION, 8);
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

/// A network as its owner creates it: the owner's membership under the
/// network's root, and the network's key in its server config.
fn owned_network(id: u64, seed: u64) -> Network {
    let mut rng = det_rng(seed);
    let (network_key, owner) = (generate_key(&mut rng), generate_key(&mut rng));
    let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
    let csr = new_certificate_request(&owner, usage, "owner").unwrap();
    let cert = sign_certificate_request_with_network(
        &csr,
        &network_key,
        "net",
        DEFAULT_CERT_TTL,
        T0,
        &mut rng,
    )
    .unwrap();
    Network {
        id,
        certificate: Some(cert),
        server_config: Some(ServerConfig {
            name: "net".into(),
            key: Some(network_key),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn ca_record_ids(tx: &mut WriteTx<MemoryKv>, network: u64) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
    let k = ref_key(network);
    block(async {
        (
            CertificateLogs::index_ids(tx, &CertificateLogs::BY_NETWORK, &k)
                .await
                .unwrap(),
            NetworkPeers::index_ids(tx, &NetworkPeers::BY_NETWORK, &k)
                .await
                .unwrap(),
            AliasReservations::index_ids(tx, &AliasReservations::BY_NETWORK, &k)
                .await
                .unwrap(),
        )
    })
}

#[test]
fn owned_networks_get_their_ca_records() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    let n = block(Networks::insert(&mut tx, owned_network(10, 1))).unwrap();
    let cert = n.certificate.unwrap();
    let root = certificate_parent(&cert).unwrap().clone();
    let (owner, root_body) = (
        certificate_body(&cert).unwrap(),
        certificate_body(&root).unwrap(),
    );

    let (logs, peers, reservations) = ca_record_ids(&mut tx, 10);
    assert_eq!((logs.len(), peers.len(), reservations.len()), (2, 1, 1));
    block(async {
        let member = CertificateLogs::index_get(
            &mut tx,
            &CertificateLogs::BY_SUBJECT,
            &network_scoped_key(10, b"owner"),
        )
        .await
        .unwrap()
        .unwrap();
        let logged = member.certificate.unwrap();
        assert!(logged.parent.is_none());
        let logged = certificate_body(&logged).unwrap();
        assert_eq!(logged.key, owner.key);
        assert_eq!(logged.issuer_serial_number, root_body.serial_number);
        let root_log = CertificateLogs::index_get(
            &mut tx,
            &CertificateLogs::BY_SERIAL_NUMBER,
            &network_scoped_key(10, &root_body.serial_number),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(root_log.certificate.unwrap(), root);

        let peer = NetworkPeers::index_get(
            &mut tx,
            &NetworkPeers::BY_PUBLIC_KEY,
            &network_scoped_key(10, &owner.key),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            (peer.alias.as_str(), peer.inviter_peer_id, peer.invite_quota),
            ("owner", 0, 3)
        );
        assert_eq!(peer.created_at, T0.unix());
        let r = AliasReservations::index_get(
            &mut tx,
            &AliasReservations::BY_ALIAS,
            &network_scoped_key(10, b"owner"),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            (r.peer_key, r.reserved_until),
            (owner.key, RESERVED_FOREVER)
        );

        // a joined network, and one applied from another replica, get none
        let mut joined = owned_network(11, 2);
        joined.server_config = None;
        Networks::insert(&mut tx, joined).await.unwrap();
        tx.set_applying(77);
        Networks::insert(&mut tx, owned_network(12, 3))
            .await
            .unwrap();
    });
    assert_eq!(ca_record_ids(&mut tx, 11), (vec![], vec![], vec![]));
    assert_eq!(ca_record_ids(&mut tx, 12), (vec![], vec![], vec![]));
}

#[test]
fn deleting_a_network_deletes_its_ca_records() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    block(async {
        Networks::insert(&mut tx, owned_network(10, 1))
            .await
            .unwrap();
        Networks::insert(&mut tx, owned_network(20, 2))
            .await
            .unwrap();
        Networks::delete(&mut tx, 10).await.unwrap();
    });
    assert_eq!(ca_record_ids(&mut tx, 10), (vec![], vec![], vec![]));
    assert_eq!(ca_record_ids(&mut tx, 20).0.len(), 2);

    // a deletion applied from another replica too
    tx.set_applying(77);
    block(Networks::delete(&mut tx, 20)).unwrap();
    assert_eq!(ca_record_ids(&mut tx, 20), (vec![], vec![], vec![]));
    let left = block(CertificateLogs::get_all(&mut tx)).unwrap();
    assert!(left.is_empty());
}

fn directory_record_ids(tx: &mut WriteTx<MemoryKv>, network: u64) -> (Vec<u64>, Vec<u64>) {
    let k = ref_key(network);
    block(async {
        (
            DirectoryListingRecords::index_ids(tx, &DirectoryListingRecords::BY_NETWORK, &k)
                .await
                .unwrap(),
            DirectoryUserRecords::index_ids(tx, &DirectoryUserRecords::BY_NETWORK, &k)
                .await
                .unwrap(),
        )
    })
}

fn chat_listing(key: &[u8]) -> Listing {
    Listing {
        content: Some(listing::Content::Chat(listing::Chat {
            key: key.to_vec(),
            name: "chat".into(),
        })),
    }
}

#[test]
fn an_owned_network_makes_its_owner_directory_admin() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    let n = block(Networks::insert(&mut tx, owned_network(10, 1))).unwrap();
    let owner_key = certificate_body(&n.certificate.unwrap()).unwrap().key;

    let (listings, users) = directory_record_ids(&mut tx, 10);
    assert_eq!((listings.len(), users.len()), (0, 1));
    let admin = block(DirectoryUserRecords::index_get(
        &mut tx,
        &DirectoryUserRecords::BY_PEER_KEY,
        &network_scoped_key(10, &owner_key),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(admin.id, users[0]);
    assert_eq!(
        admin.moderation,
        Some(UserModeration {
            is_admin: Some(true),
            is_moderator: Some(true),
            ..Default::default()
        })
    );
    let ids = ca_record_ids(&mut tx, 10);
    assert!(
        ids.0
            .iter()
            .chain(&ids.1)
            .chain(&ids.2)
            .all(|&id| id < admin.id)
    );

    // one applied from another replica gets none
    tx.set_applying(77);
    block(Networks::insert(&mut tx, owned_network(12, 3))).unwrap();
    assert_eq!(directory_record_ids(&mut tx, 12), (vec![], vec![]));
}

#[test]
fn a_joined_network_has_no_directory_user_record() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    let mut joined = owned_network(11, 2);
    joined.server_config = None;
    block(Networks::insert(&mut tx, joined)).unwrap();
    assert_eq!(directory_record_ids(&mut tx, 11), (vec![], vec![]));
    assert!(
        block(DirectoryUserRecords::get_all(&mut tx))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn deleting_a_network_deletes_its_directory_records() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    block(async {
        for (network, seed) in [(10, 1), (20, 2)] {
            Networks::insert(&mut tx, owned_network(network, seed))
                .await
                .unwrap();
            let id = tx.generate_id().await.unwrap();
            DirectoryListingRecords::insert(
                &mut tx,
                new_listing_record(id, network, chat_listing(b"room"), None),
            )
            .await
            .unwrap();
            let id = tx.generate_id().await.unwrap();
            DirectoryUserRecords::insert(
                &mut tx,
                new_user_record(id, network, b"user", UserModeration::default()),
            )
            .await
            .unwrap();
        }
        Networks::delete(&mut tx, 10).await.unwrap();
    });
    assert_eq!(directory_record_ids(&mut tx, 10), (vec![], vec![]));
    let (listings, users) = directory_record_ids(&mut tx, 20);
    assert_eq!((listings.len(), users.len()), (1, 2));
    let by_listing = block(DirectoryListingRecords::index_ids(
        &mut tx,
        &DirectoryListingRecords::BY_LISTING,
        &listing_key(10, Some(&chat_listing(b"room"))),
    ))
    .unwrap();
    assert!(by_listing.is_empty());

    // a deletion applied from another replica too
    tx.set_applying(77);
    block(Networks::delete(&mut tx, 20)).unwrap();
    assert_eq!(directory_record_ids(&mut tx, 20), (vec![], vec![]));
    assert!(
        block(DirectoryListingRecords::get_all(&mut tx))
            .unwrap()
            .is_empty()
    );
    assert!(
        block(DirectoryUserRecords::get_all(&mut tx))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn directory_records_are_unique_per_listing_and_user() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    block(async {
        let listing = |id, network| new_listing_record(id, network, chat_listing(b"room"), None);
        DirectoryListingRecords::insert(&mut tx, listing(1, 7))
            .await
            .unwrap();
        assert!(matches!(
            DirectoryListingRecords::insert(&mut tx, listing(2, 7)).await,
            Err(Error::UniqueViolation(Namespace(9003)))
        ));
        DirectoryListingRecords::insert(&mut tx, listing(3, 8))
            .await
            .unwrap();

        let user = |id, network| new_user_record(id, network, b"user", UserModeration::default());
        DirectoryUserRecords::insert(&mut tx, user(4, 7))
            .await
            .unwrap();
        assert!(matches!(
            DirectoryUserRecords::insert(&mut tx, user(5, 7)).await,
            Err(Error::UniqueViolation(Namespace(9005)))
        ));
        DirectoryUserRecords::insert(&mut tx, user(6, 8))
            .await
            .unwrap();
    });
}

/// A chat server and a record of each kind for it.
struct ChatServerRecords {
    server: chat::Server,
    icon: chat::ServerIcon,
    emote: chat::Emote,
    modifier: chat::Modifier,
    tag: chat::Tag,
    profile: chat::Profile,
}

/// Chat server `id` on the network with `network_key`, its records
/// numbered from `id + 1`, and a profile of the user `b"user"`.
fn chat_server(id: u64, network_key: &[u8]) -> ChatServerRecords {
    ChatServerRecords {
        server: chat::Server {
            id,
            network_key: network_key.to_vec(),
            ..Default::default()
        },
        icon: chat::ServerIcon { id, image: None },
        emote: chat::Emote {
            id: id + 1,
            server_id: id,
            name: "emote".into(),
            ..Default::default()
        },
        modifier: chat::Modifier {
            id: id + 2,
            server_id: id,
            name: "modifier".into(),
            ..Default::default()
        },
        tag: chat::Tag {
            id: id + 3,
            server_id: id,
            name: "tag".into(),
            ..Default::default()
        },
        profile: chat::Profile {
            id: id + 4,
            server_id: id,
            peer_key: b"user".to_vec(),
            alias: "user".into(),
            ..Default::default()
        },
    }
}

fn insert_chat_server(tx: &mut WriteTx<MemoryKv>, r: &ChatServerRecords) {
    block(async {
        ChatServers::insert(tx, r.server.clone()).await.unwrap();
        ChatServerIcons::insert(tx, r.icon.clone()).await.unwrap();
        ChatEmotes::insert(tx, r.emote.clone()).await.unwrap();
        ChatModifiers::insert(tx, r.modifier.clone()).await.unwrap();
        ChatTags::insert(tx, r.tag.clone()).await.unwrap();
        ChatProfiles::insert(tx, r.profile.clone()).await.unwrap();
    });
}

/// The rows and index entries of chat server `id` and its records.
fn chat_server_entries(tx: &mut WriteTx<MemoryKv>, id: u64) -> usize {
    let k = ref_key(id);
    block(async {
        let mut n = usize::from(ChatServers::get(tx, id).await.unwrap().is_some());
        n += usize::from(ChatServerIcons::get(tx, id).await.unwrap().is_some());
        n += ChatEmotes::index_get_all(tx, &ChatEmotes::BY_SERVER, &k)
            .await
            .unwrap()
            .len();
        n += ChatModifiers::index_get_all(tx, &ChatModifiers::BY_SERVER, &k)
            .await
            .unwrap()
            .len();
        n += ChatTags::index_get_all(tx, &ChatTags::BY_SERVER, &k)
            .await
            .unwrap()
            .len();
        n += ChatProfiles::index_get_all(tx, &ChatProfiles::BY_SERVER, &k)
            .await
            .unwrap()
            .len();
        n += ChatProfiles::index_ids(
            tx,
            &ChatProfiles::BY_PEER_KEY,
            &profile_peer_key(id, b"user"),
        )
        .await
        .unwrap()
        .len();
        n
    })
}

/// The ids of the rows left in the chat server tables.
fn chat_row_ids(tx: &mut WriteTx<MemoryKv>) -> Vec<u64> {
    block(async {
        let mut ids: Vec<u64> = ChatServers::get_all(tx)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        ids.extend(
            ChatServerIcons::get_all(tx)
                .await
                .unwrap()
                .iter()
                .map(|r| r.id),
        );
        ids.extend(ChatEmotes::get_all(tx).await.unwrap().iter().map(|r| r.id));
        ids.extend(
            ChatModifiers::get_all(tx)
                .await
                .unwrap()
                .iter()
                .map(|r| r.id),
        );
        ids.extend(ChatTags::get_all(tx).await.unwrap().iter().map(|r| r.id));
        ids.extend(
            ChatProfiles::get_all(tx)
                .await
                .unwrap()
                .iter()
                .map(|r| r.id),
        );
        ids.sort();
        ids
    })
}

#[test]
fn deleting_a_chat_server_deletes_its_records() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let (doomed, kept) = (chat_server(10, b"net"), chat_server(20, b"net"));
    let mut tx = block(store.write(T0)).unwrap();
    insert_chat_server(&mut tx, &doomed);
    insert_chat_server(&mut tx, &kept);
    assert_eq!(chat_server_entries(&mut tx, 10), 7);
    block(tx.commit()).unwrap();

    let mut sub = store.events().subscribe();
    let mut tx = block(store.write(T0)).unwrap();
    assert!(block(ChatServers::delete(&mut tx, 10)).unwrap());
    assert_eq!(chat_server_entries(&mut tx, 10), 0);
    assert_eq!(chat_server_entries(&mut tx, 20), 7);
    assert_eq!(chat_row_ids(&mut tx), [20, 20, 21, 22, 23, 24]);
    block(tx.commit()).unwrap();
    assert_eq!(
        drain(&mut sub),
        [
            StoreEvent::Deleted(Record::ChatServerIcon(doomed.icon)),
            StoreEvent::Deleted(Record::ChatEmote(doomed.emote)),
            StoreEvent::Deleted(Record::ChatModifier(doomed.modifier)),
            StoreEvent::Deleted(Record::ChatTag(doomed.tag)),
            StoreEvent::Deleted(Record::ChatServer(doomed.server)),
        ]
    );

    // the user's profile key is taken on the kept server, and free on the
    // deleted one
    let mut tx = block(store.write(T0)).unwrap();
    let profile = chat::Profile {
        id: 30,
        ..kept.profile
    };
    block(async {
        assert!(matches!(
            ChatProfiles::insert(&mut tx, profile.clone()).await,
            Err(Error::UniqueViolation(Namespace(6011)))
        ));
        let moved = chat::Profile {
            server_id: 10,
            ..profile
        };
        ChatProfiles::insert(&mut tx, moved).await.unwrap();
    });
}

#[test]
fn deleting_a_network_deletes_its_chat_servers() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    let (a, b) = (owned_network(1, 1), owned_network(2, 2));
    let keyless = Network {
        id: 3,
        ..Default::default()
    };
    block(async {
        for n in [&a, &b, &keyless] {
            Networks::insert(&mut tx, n.clone()).await.unwrap();
        }
    });
    insert_chat_server(&mut tx, &chat_server(10, &network_key(&a)));
    insert_chat_server(&mut tx, &chat_server(20, &network_key(&b)));
    let unassigned = chat_server(30, b"").server;
    block(ChatServers::insert(&mut tx, unassigned)).unwrap();

    block(Networks::delete(&mut tx, 1)).unwrap();
    assert_eq!(chat_server_entries(&mut tx, 10), 0);
    assert_eq!(chat_server_entries(&mut tx, 20), 7);

    // a network without a key has no chat servers to delete
    block(Networks::delete(&mut tx, 3)).unwrap();
    assert_eq!(chat_row_ids(&mut tx), [20, 20, 21, 22, 23, 24, 30]);

    // a deletion applied from another replica too
    tx.set_applying(77);
    block(Networks::delete(&mut tx, 2)).unwrap();
    assert_eq!(chat_row_ids(&mut tx), [30]);
}

#[test]
fn newer_certificate_logs_take_over_their_subject_and_key() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let cert = |serial: u8| Certificate {
        body: CertificateBody {
            key: b"k".to_vec(),
            subject: "s".into(),
            serial_number: vec![serial; 16],
            ..Default::default()
        }
        .encode_to_vec(),
        ..Default::default()
    };
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        CertificateLogs::insert(&mut tx, new_certificate_log(1, 7, &cert(1)))
            .await
            .unwrap();
        CertificateLogs::insert(&mut tx, new_certificate_log(2, 7, &cert(2)))
            .await
            .unwrap();
        for (ix, key) in [
            (&CertificateLogs::BY_SUBJECT, b"s"),
            (&CertificateLogs::BY_KEY, b"k"),
        ] {
            let ids = CertificateLogs::index_ids(&mut tx, ix, &network_scoped_key(7, key))
                .await
                .unwrap();
            assert_eq!(ids, [2]);
        }
        assert!(CertificateLogs::get(&mut tx, 1).await.unwrap().is_some());
        assert!(matches!(
            CertificateLogs::insert(&mut tx, new_certificate_log(3, 7, &cert(2))).await,
            Err(Error::UniqueViolation(Namespace(4005)))
        ));
        assert_eq!(
            NetworkUiConfig::get(&mut tx).await.unwrap(),
            Some(Default::default())
        );
    });
}

#[test]
fn the_bootstrap_config_is_closed_by_default_and_stays_on_the_device() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    block(async {
        let mut tx = store.read().await.unwrap();
        assert_eq!(
            BootstrapConfig::get(&mut tx).await.unwrap(),
            Some(bootstrap::Config {
                enable_publishing: false
            })
        );
        drop(tx);
        let mut tx = store.write(T0).await.unwrap();
        let on = bootstrap::Config {
            enable_publishing: true,
        };
        BootstrapConfig::set(&mut tx, on).await.unwrap();
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        assert_eq!(BootstrapConfig::get(&mut tx).await.unwrap(), Some(on));
        assert!(EventLogs::get_all(&mut tx).await.unwrap().is_empty());
        assert_eq!(
            drain(&mut sub),
            [StoreEvent::Changed(Record::BootstrapConfig(on))]
        );
    });
}

/// A notification `id` about the network `network`, raised `ms` after T0.
fn network_notification(id: u64, network: u64, ms: i64) -> Notification {
    new_notification(
        id,
        T0 + Duration::from_millis(ms),
        Status::Success,
        "renewed",
        "",
        Some(network_subject(network)),
    )
}

fn notification_ids(tx: &mut WriteTx<MemoryKv>) -> Vec<u64> {
    let all = block(Notifications::get_all(tx)).unwrap();
    all.iter().map(|n| n.id).collect()
}

#[test]
fn a_subjects_notification_replaces_its_last() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut sub = store.events().subscribe();
    let (n1, n2, n3) = (
        network_notification(1, 10, 0),
        network_notification(2, 20, 1),
        network_notification(3, 10, 2),
    );
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        dispatch(&mut tx, n1.clone()).await.unwrap();
        dispatch(&mut tx, n2.clone()).await.unwrap();
        tx.commit().await.unwrap();
    });
    drain(&mut sub);

    let mut tx = block(store.write(T0)).unwrap();
    block(dispatch(&mut tx, n3.clone())).unwrap();
    assert_eq!(notification_ids(&mut tx), [2, 3]);
    block(tx.commit()).unwrap();
    assert_eq!(
        drain(&mut sub),
        [
            StoreEvent::Deleted(Record::Notification(n1)),
            StoreEvent::Changed(Record::Notification(n3)),
        ]
    );
}

#[test]
fn dispatch_evicts_the_oldest_notifications_past_the_cap() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    let extra = 5;
    let count = MAX_NOTIFICATIONS + extra;
    block(async {
        // newest first, so creation order differs from id order
        for i in 0..count as u64 {
            let mut n = network_notification(100 + i, 0, (count as i64) - i as i64);
            n.subject = None;
            Notifications::insert(&mut tx, n).await.unwrap();
        }
        dispatch(&mut tx, network_notification(1, 10, 1000))
            .await
            .unwrap();
    });
    let ids = notification_ids(&mut tx);
    assert_eq!(ids.len(), MAX_NOTIFICATIONS);
    assert_eq!(ids[0], 1);
    let oldest = (count - extra - 1) as u64..count as u64;
    for id in oldest.map(|i| 100 + i) {
        assert!(!ids.contains(&id), "{id} kept");
    }
}

#[test]
fn deleting_a_network_deletes_its_notifications() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let mut tx = block(store.write(T0)).unwrap();
    block(async {
        for (network, seed) in [(10, 1), (20, 2)] {
            Networks::insert(&mut tx, owned_network(network, seed))
                .await
                .unwrap();
        }
        dispatch(&mut tx, network_notification(1, 10, 0))
            .await
            .unwrap();
        dispatch(&mut tx, network_notification(2, 20, 1))
            .await
            .unwrap();
        Networks::delete(&mut tx, 10).await.unwrap();
    });
    assert_eq!(notification_ids(&mut tx), [2]);

    // a deletion applied from another replica too
    tx.set_applying(77);
    block(Networks::delete(&mut tx, 20)).unwrap();
    assert!(notification_ids(&mut tx).is_empty());
}

/// A certificate for `key` issued by the CA of `network`.
fn member_certificate(network: &Network, key: &Key, seed: u64) -> Certificate {
    let ca = network
        .server_config
        .as_ref()
        .unwrap()
        .key
        .as_ref()
        .unwrap();
    let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
    let csr = new_certificate_request(key, usage, "member").unwrap();
    sign_certificate_request_with_network(&csr, ca, "net", DEFAULT_CERT_TTL, T0, &mut det_rng(seed))
        .unwrap()
}

fn snippet(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        ..Default::default()
    }
}

fn local_owner(auth_key: &[u8], network_key: &[u8]) -> Owner {
    Owner::Local(video_channel::Local {
        auth_key: auth_key.to_vec(),
        network_key: network_key.to_vec(),
    })
}

fn share_owner(certificate: Certificate) -> Owner {
    Owner::LocalShare(video_channel::LocalShare {
        certificate: Some(certificate),
    })
}

fn remote_owner(network_key: &[u8], service_key: &[u8]) -> Owner {
    Owner::RemoteShare(video_channel::RemoteShare {
        id: 9,
        network_key: network_key.to_vec(),
        service_key: service_key.to_vec(),
        ..Default::default()
    })
}

fn video_channel(id: u64, owner: Owner) -> VideoChannel {
    VideoChannel {
        id,
        owner: Some(owner),
        ..Default::default()
    }
}

fn video_channel_ids(tx: &mut WriteTx<MemoryKv>) -> Vec<u64> {
    let all = block(VideoChannels::get_all(tx)).unwrap();
    all.iter().map(|c| c.id).collect()
}

/// The video channel and config events among `events`.
fn video_events(events: Vec<StoreEvent>) -> Vec<StoreEvent> {
    events
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                StoreEvent::Changed(
                    Record::VideoChannel(_)
                        | Record::VideoIngressConfig(_)
                        | Record::HlsEgressConfig(_)
                ) | StoreEvent::Deleted(Record::VideoChannel(_))
            )
        })
        .collect()
}

#[test]
fn video_channels_stay_on_the_device_and_are_observable() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    let profile_key = generate_key(&mut det_rng(3));
    let owner = || local_owner(&profile_key.public, b"net");
    let (c, renamed) = block(async {
        let mut tx = store.write(T0).await.unwrap();
        let c = new_channel(&mut tx, owner(), snippet("first"))
            .await
            .unwrap();
        let other = new_channel(&mut tx, owner(), snippet("second"))
            .await
            .unwrap();
        assert_eq!(c.token.len(), TOKEN_SIZE);
        assert_eq!(c.key.as_ref().unwrap().public.len(), 32);
        assert_ne!(c.id, other.id);
        assert_ne!(c.token, other.token);
        assert_ne!(c.key, other.key);
        assert_eq!(c.owner, Some(owner()));
        assert_eq!(c.directory_listing_snippet, Some(snippet("first")));
        VideoChannels::insert(&mut tx, c.clone()).await.unwrap();
        let renamed = VideoChannel {
            directory_listing_snippet: Some(snippet("renamed")),
            ..c.clone()
        };
        VideoChannels::update(&mut tx, renamed.clone())
            .await
            .unwrap();
        tx.commit().await.unwrap();
        (c, renamed)
    });

    let mut tx = block(store.write(T0)).unwrap();
    block(async {
        assert_eq!(
            VideoChannels::get(&mut tx, c.id).await.unwrap(),
            Some(renamed.clone())
        );
        let by_owner = VideoChannels::index_get(&mut tx, &VideoChannels::BY_OWNER, &owner_key(&c))
            .await
            .unwrap();
        assert_eq!(by_owner, Some(renamed.clone()));
        assert!(VideoChannels::delete(&mut tx, c.id).await.unwrap());
        assert!(
            VideoChannels::index_ids(&mut tx, &VideoChannels::BY_OWNER, &owner_key(&c))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(EventLogs::get_all(&mut tx).await.unwrap().is_empty());
    });
    assert!(video_channel_ids(&mut tx).is_empty());
    block(tx.commit()).unwrap();
    assert_eq!(
        drain(&mut sub),
        [
            StoreEvent::Changed(Record::VideoChannel(c)),
            StoreEvent::Changed(Record::VideoChannel(renamed.clone())),
            StoreEvent::Deleted(Record::VideoChannel(renamed)),
        ]
    );
}

#[test]
fn a_certificate_key_owns_one_shared_channel() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let n = owned_network(1, 1);
    let (a, b) = (generate_key(&mut det_rng(3)), generate_key(&mut det_rng(4)));
    let mut tx = block(store.write(T0)).unwrap();
    block(async {
        let shared = video_channel(10, share_owner(member_certificate(&n, &a, 5)));
        VideoChannels::insert(&mut tx, shared.clone())
            .await
            .unwrap();
        VideoChannels::update(
            &mut tx,
            VideoChannel {
                token: b"new token".to_vec(),
                ..shared
            },
        )
        .await
        .unwrap();
        // a renewed certificate for the same key names the same owner
        let renewed = video_channel(11, share_owner(member_certificate(&n, &a, 6)));
        assert!(matches!(
            VideoChannels::insert(&mut tx, renewed).await,
            Err(Error::UniqueViolation(Namespace(7002)))
        ));
        let other = video_channel(12, share_owner(member_certificate(&n, &b, 7)));
        VideoChannels::insert(&mut tx, other).await.unwrap();

        // local channels are keyed by id, remote shares by network and
        // service
        for id in [13, 14] {
            let local = video_channel(id, local_owner(&a.public, b"net"));
            VideoChannels::insert(&mut tx, local).await.unwrap();
        }
        let remote = video_channel(15, remote_owner(b"net", b"svc"));
        VideoChannels::insert(&mut tx, remote).await.unwrap();
        let again = video_channel(16, remote_owner(b"net", b"svc"));
        assert!(matches!(
            VideoChannels::insert(&mut tx, again).await,
            Err(Error::UniqueViolation(Namespace(7002)))
        ));
        let elsewhere = video_channel(17, remote_owner(b"net", b"svc2"));
        VideoChannels::insert(&mut tx, elsewhere).await.unwrap();
    });
    assert_eq!(video_channel_ids(&mut tx), [10, 12, 13, 14, 15, 17]);
}

#[test]
fn stream_keys_round_trip() {
    let key = generate_key(&mut det_rng(3));
    for id in [0, 1, 127, 128, 300, 0x0102_0304_0506_0708, u64::MAX] {
        let token = [id as u8; TOKEN_SIZE];
        let s = format_stream_key(id, &token, &key).unwrap();
        assert!(
            s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
            "{s}"
        );
        let (got, signature) = parse_stream_key(&s).unwrap();
        assert_eq!(got, id);
        assert!(strims_crypto::sign::verify(
            strims_crypto::Domain::VideoStreamKey,
            &key.public,
            &[&token],
            &signature
        ));
    }
    assert!(matches!(
        format_stream_key(1, b"token", &Key::default()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn malformed_stream_keys_are_rejected() {
    let sig = [7u8; 64];
    let enc = |parts: &[&[u8]]| BASE64URL_NOPAD.encode(&parts.concat());
    // an id encoded longer than needed, and the largest id
    assert_eq!(parse_stream_key(&enc(&[&[0x85, 0x00], &sig])), Ok((5, sig)));
    assert_eq!(
        parse_stream_key(&enc(&[&[0xff; 9], &[0x01], &sig])),
        Ok((u64::MAX, sig))
    );
    let cases = [
        // ids past 64 bits, or truncated
        (enc(&[&[0xff; 9], &[0x02], &sig]), StreamKeyError::Id),
        (enc(&[&[0x80; 10], &[0x01], &sig[..63]]), StreamKeyError::Id),
        (enc(&[&[0x80; 5]]), StreamKeyError::Id),
        (String::new(), StreamKeyError::Id),
        // signatures of the wrong length, and overlong keys
        (enc(&[&[5], &sig[..63]]), StreamKeyError::Length),
        (enc(&[&[5], &sig, &[0]]), StreamKeyError::Length),
        (enc(&[&[0x80; 10], &[0x01], &sig]), StreamKeyError::Length),
        ("A".repeat(1000), StreamKeyError::Length),
        // not unpadded base64url
        (format!("{}=", enc(&[&[5], &sig])), StreamKeyError::Encoding),
        (
            enc(&[&[5], &[0xff; 64]]).replace('_', "/"),
            StreamKeyError::Encoding,
        ),
        ("!!!!".into(), StreamKeyError::Encoding),
        ("A".into(), StreamKeyError::Encoding),
    ];
    for (key, err) in cases {
        assert_eq!(parse_stream_key(&key), Err(err), "{key}");
    }
}

proptest! {
    #[test]
    fn stream_key_parsing_never_panics(s in "\\PC{0,120}") {
        let _ = parse_stream_key(&s);
    }

    #[test]
    fn decoded_stream_keys_are_an_id_then_a_signature(
        bytes in proptest::collection::vec(any::<u8>(), 0..80)
    ) {
        if let Ok((_, signature)) = parse_stream_key(&BASE64URL_NOPAD.encode(&bytes)) {
            let id_len = bytes.len() - signature.len();
            prop_assert!((1..=10).contains(&id_len));
            prop_assert_eq!(&bytes[id_len..], &signature[..]);
            prop_assert!(bytes[..id_len - 1].iter().all(|&b| b >= 0x80));
            prop_assert!(bytes[id_len - 1] < 0x80);
        }
    }
}

fn stream_key_channel(tx: &mut WriteTx<MemoryKv>, key: &str) -> Option<u64> {
    block(channel_by_stream_key(tx, key)).unwrap().map(|c| c.id)
}

#[test]
fn a_stream_key_names_its_channel_only_when_the_owner_signed_it() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let n = owned_network(1, 1);
    let profile_key = generate_key(&mut det_rng(3));
    let share_key = generate_key(&mut det_rng(4));
    let cert = member_certificate(&n, &share_key, 5);
    let (local, share, remote, short) = block(async {
        let mut tx = store.write(T0).await.unwrap();
        let owner = local_owner(&profile_key.public, &network_key(&n));
        let local = new_channel(&mut tx, owner, snippet("local")).await.unwrap();
        let share = new_channel(&mut tx, share_owner(cert), snippet("share"))
            .await
            .unwrap();
        let owner = remote_owner(b"net", b"svc");
        let remote = new_channel(&mut tx, owner, snippet("remote"))
            .await
            .unwrap();
        let owner = local_owner(b"short", b"other");
        let short = new_channel(&mut tx, owner, snippet("short")).await.unwrap();
        for c in [&local, &share, &remote, &short] {
            VideoChannels::insert(&mut tx, c.clone()).await.unwrap();
        }
        tx.commit().await.unwrap();
        (local, share, remote, short)
    });
    let key = |c: &VideoChannel, signer: &Key| format_stream_key(c.id, &c.token, signer).unwrap();

    let mut tx = block(store.write(T0)).unwrap();
    assert_eq!(
        stream_key_channel(&mut tx, &key(&local, &profile_key)),
        Some(local.id)
    );
    assert_eq!(
        stream_key_channel(&mut tx, &key(&share, &share_key)),
        Some(share.id)
    );
    // another signer, another token, an unknown id, a malformed key
    assert_eq!(stream_key_channel(&mut tx, &key(&local, &share_key)), None);
    assert_eq!(
        stream_key_channel(&mut tx, &key(&share, &profile_key)),
        None
    );
    let other_token = format_stream_key(local.id, b"other", &profile_key).unwrap();
    assert_eq!(stream_key_channel(&mut tx, &other_token), None);
    let unknown = format_stream_key(999, &local.token, &profile_key).unwrap();
    assert_eq!(stream_key_channel(&mut tx, &unknown), None);
    assert_eq!(stream_key_channel(&mut tx, "!!!"), None);
    // a remote share's key isn't this device's to check
    let remote_key = key(&remote, remote.key.as_ref().unwrap());
    assert_eq!(stream_key_channel(&mut tx, &remote_key), None);
    // an auth key of the wrong length verifies nothing
    assert_eq!(
        stream_key_channel(&mut tx, &key(&short, &profile_key)),
        None
    );

    // a new token retires the old key
    let old = key(&local, &profile_key);
    let retoken = VideoChannel {
        token: vec![1; TOKEN_SIZE],
        ..local.clone()
    };
    block(VideoChannels::update(&mut tx, retoken.clone())).unwrap();
    block(tx.commit()).unwrap();
    let mut read = block(store.read()).unwrap();
    assert_eq!(block(channel_by_stream_key(&mut read, &old)).unwrap(), None);
    let new = key(&retoken, &profile_key);
    let found = block(channel_by_stream_key(&mut read, &new)).unwrap();
    assert_eq!(found, Some(retoken));
}

#[test]
fn deleting_a_network_deletes_its_video_channels() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let (a, b) = (owned_network(1, 1), owned_network(2, 2));
    let (ka, kb) = (network_key(&a), network_key(&b));
    let member = generate_key(&mut det_rng(3));
    let channels = |n: &Network, first: u64| {
        let k = network_key(n);
        [
            video_channel(first, local_owner(&member.public, &k)),
            video_channel(
                first + 1,
                share_owner(member_certificate(n, &member, first)),
            ),
            video_channel(first + 2, remote_owner(&k, b"svc")),
        ]
    };
    let (on_a, on_b) = (channels(&a, 10), channels(&b, 20));
    let unowned = VideoChannel {
        id: 30,
        ..Default::default()
    };
    let config = VideoIngressConfig {
        enabled: true,
        service_network_keys: vec![ka, b"other".to_vec(), kb.clone()],
        ..IngressConfig::default_value().unwrap()
    };
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        for n in [&a, &b] {
            Networks::insert(&mut tx, n.clone()).await.unwrap();
        }
        for c in on_a.iter().chain(&on_b).chain([&unowned]) {
            VideoChannels::insert(&mut tx, c.clone()).await.unwrap();
        }
        IngressConfig::set(&mut tx, config.clone()).await.unwrap();
        tx.commit().await.unwrap();
    });

    let mut sub = store.events().subscribe();
    let mut tx = block(store.write(T0)).unwrap();
    block(Networks::delete(&mut tx, 1)).unwrap();
    assert_eq!(video_channel_ids(&mut tx), [20, 21, 22, 30]);
    for c in &on_a {
        let ids = block(VideoChannels::index_ids(
            &mut tx,
            &VideoChannels::BY_OWNER,
            &owner_key(c),
        ))
        .unwrap();
        assert!(ids.is_empty());
    }
    let pruned = VideoIngressConfig {
        service_network_keys: vec![b"other".to_vec(), kb],
        ..config
    };
    assert_eq!(
        block(IngressConfig::get(&mut tx)).unwrap(),
        Some(pruned.clone())
    );
    block(tx.commit()).unwrap();
    let mut expected: Vec<StoreEvent> = on_a
        .iter()
        .map(|c| StoreEvent::Deleted(Record::VideoChannel(c.clone())))
        .collect();
    expected.push(StoreEvent::Changed(Record::VideoIngressConfig(pruned)));
    assert_eq!(video_events(drain(&mut sub)), expected);

    // a deletion applied from another replica too
    let mut tx = block(store.write(T0)).unwrap();
    tx.set_applying(77);
    block(Networks::delete(&mut tx, 2)).unwrap();
    assert_eq!(video_channel_ids(&mut tx), [30]);
    let keys = block(IngressConfig::get(&mut tx))
        .unwrap()
        .unwrap()
        .service_network_keys;
    assert_eq!(keys, [b"other".to_vec()]);
}

#[test]
fn video_configs_have_defaults_and_stay_on_the_device() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    let ingress = VideoIngressConfig {
        enabled: true,
        server_addr: "0.0.0.0:1935".into(),
        public_server_addr: "rtmp.example:1935".into(),
        service_network_keys: vec![b"net".to_vec()],
    };
    let hls = HlsEgressConfig {
        enabled: true,
        public_server_addr: "https://hls.example".into(),
    };
    block(async {
        let mut tx = store.read().await.unwrap();
        let default = IngressConfig::get(&mut tx).await.unwrap().unwrap();
        assert_eq!(default.server_addr, DEFAULT_INGRESS_SERVER_ADDR);
        assert_eq!(default.server_addr, "127.0.0.1:1935");
        assert!(!default.enabled);
        assert!(default.public_server_addr.is_empty());
        assert!(default.service_network_keys.is_empty());
        assert_eq!(
            HlsConfig::get(&mut tx).await.unwrap(),
            Some(HlsEgressConfig::default())
        );
        drop(tx);

        let mut tx = store.write(T0).await.unwrap();
        // pruning a key the config doesn't hold writes nothing
        delete_for_network(&mut tx, b"absent").await.unwrap();
        IngressConfig::set(&mut tx, ingress.clone()).await.unwrap();
        HlsConfig::set(&mut tx, hls.clone()).await.unwrap();
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        assert_eq!(
            IngressConfig::get(&mut tx).await.unwrap(),
            Some(ingress.clone())
        );
        assert_eq!(HlsConfig::get(&mut tx).await.unwrap(), Some(hls.clone()));
        assert!(EventLogs::get_all(&mut tx).await.unwrap().is_empty());
    });
    assert_eq!(
        drain(&mut sub),
        [
            StoreEvent::Changed(Record::VideoIngressConfig(ingress)),
            StoreEvent::Changed(Record::HlsEgressConfig(hls)),
        ]
    );
}

#[test]
fn vnic_and_autoseed_configs_have_defaults_and_stay_on_the_device() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    let vnic = VnicConfigRecord {
        max_upload_bytes_per_second: 1 << 20,
        max_peers: 8,
    };
    let autoseed = AutoseedConfigRecord { enable: true };
    block(async {
        let mut tx = store.read().await.unwrap();
        assert_eq!(
            VnicConfig::get(&mut tx).await.unwrap(),
            Some(VnicConfigRecord {
                max_upload_bytes_per_second: 1 << 40,
                max_peers: 25,
            })
        );
        assert_eq!(
            AutoseedConfig::get(&mut tx).await.unwrap(),
            Some(AutoseedConfigRecord { enable: false })
        );
        drop(tx);

        let mut tx = store.write(T0).await.unwrap();
        VnicConfig::set(&mut tx, vnic).await.unwrap();
        AutoseedConfig::set(&mut tx, autoseed).await.unwrap();
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        assert_eq!(VnicConfig::get(&mut tx).await.unwrap(), Some(vnic));
        assert_eq!(AutoseedConfig::get(&mut tx).await.unwrap(), Some(autoseed));
        assert!(EventLogs::get_all(&mut tx).await.unwrap().is_empty());
    });
    assert!(VnicConfig::REPLICATION.is_none());
    assert_eq!(
        drain(&mut sub),
        [
            StoreEvent::Changed(Record::VnicConfig(vnic)),
            StoreEvent::Changed(Record::AutoseedConfig(autoseed)),
        ]
    );
}

/// A rule `id` naming a swarm on the network with key `network_key`.
fn autoseed_rule(id: u64, network_key: &[u8], label: &str) -> Rule {
    Rule {
        id,
        network_key: network_key.to_vec(),
        swarm_id: b"swarm".to_vec(),
        salt: b"salt".to_vec(),
        label: label.into(),
    }
}

fn autoseed_rule_ids(tx: &mut WriteTx<MemoryKv>) -> Vec<u64> {
    let mut ids: Vec<u64> = block(AutoseedRules::get_all(tx))
        .unwrap()
        .iter()
        .map(|r| r.id)
        .collect();
    ids.sort_unstable();
    ids
}

#[test]
fn autoseed_rules_get_new_ids_stay_on_the_device_and_are_observable() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    let (a, b) = block(async {
        let mut tx = store.write(T0).await.unwrap();
        let a = new_rule(&mut tx, autoseed_rule(999, b"net", "a"))
            .await
            .unwrap();
        let b = new_rule(&mut tx, autoseed_rule(999, b"net", "b"))
            .await
            .unwrap();
        // a new rule isn't stored
        assert!(AutoseedRules::get_all(&mut tx).await.unwrap().is_empty());
        AutoseedRules::insert(&mut tx, a.clone()).await.unwrap();
        AutoseedRules::insert(&mut tx, b.clone()).await.unwrap();
        tx.commit().await.unwrap();
        (a, b)
    });
    assert!(a.id != 999 && b.id != 999 && a.id != b.id);
    assert_eq!(a, autoseed_rule(a.id, b"net", "a"));

    let renamed = Rule {
        label: "renamed".into(),
        ..a.clone()
    };
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        AutoseedRules::update(&mut tx, renamed.clone())
            .await
            .unwrap();
        assert!(AutoseedRules::delete(&mut tx, b.id).await.unwrap());
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        assert_eq!(
            AutoseedRules::get_all(&mut tx).await.unwrap(),
            std::slice::from_ref(&renamed)
        );
        assert!(EventLogs::get_all(&mut tx).await.unwrap().is_empty());
    });
    assert_eq!(
        drain(&mut sub),
        [
            StoreEvent::Changed(Record::AutoseedRule(a)),
            StoreEvent::Changed(Record::AutoseedRule(b.clone())),
            StoreEvent::Changed(Record::AutoseedRule(renamed)),
            StoreEvent::Deleted(Record::AutoseedRule(b)),
        ]
    );
}

#[test]
fn deleting_a_network_deletes_its_autoseed_rules() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let (a, b) = (owned_network(1, 1), owned_network(2, 2));
    let (ka, kb) = (network_key(&a), network_key(&b));
    let keyless = Network {
        id: 3,
        ..Default::default()
    };
    let rules = [
        autoseed_rule(10, &ka, "a1"),
        autoseed_rule(11, &ka, "a2"),
        autoseed_rule(20, &kb, "b"),
        autoseed_rule(30, b"", "keyless"),
        autoseed_rule(31, b"other", "other"),
    ];
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        for n in [&a, &b, &keyless] {
            Networks::insert(&mut tx, n.clone()).await.unwrap();
        }
        for r in &rules {
            AutoseedRules::insert(&mut tx, r.clone()).await.unwrap();
        }
        tx.commit().await.unwrap();
    });

    let mut sub = store.events().subscribe();
    let mut tx = block(store.write(T0)).unwrap();
    block(Networks::delete(&mut tx, 1)).unwrap();
    assert_eq!(autoseed_rule_ids(&mut tx), [20, 30, 31]);
    // a network without a key has no rules to delete
    block(Networks::delete(&mut tx, 3)).unwrap();
    assert_eq!(autoseed_rule_ids(&mut tx), [20, 30, 31]);
    block(tx.commit()).unwrap();
    let deleted: Vec<StoreEvent> = drain(&mut sub)
        .into_iter()
        .filter(|e| matches!(e, StoreEvent::Deleted(Record::AutoseedRule(_))))
        .collect();
    assert_eq!(
        deleted,
        [
            StoreEvent::Deleted(Record::AutoseedRule(rules[0].clone())),
            StoreEvent::Deleted(Record::AutoseedRule(rules[1].clone())),
        ]
    );

    // a deletion applied from another replica too
    let mut tx = block(store.write(T0)).unwrap();
    tx.set_applying(77);
    block(Networks::delete(&mut tx, 2)).unwrap();
    assert_eq!(autoseed_rule_ids(&mut tx), [30, 31]);
}

/// A cache of a swarm holding `data`.
fn swarm_cache(data: &[u8]) -> Cache {
    Cache {
        uri: "magnet:?xt=urn:ppspp:swarm".into(),
        epoch: Some(cache::Epoch {
            timestamp: 5,
            signature: vec![1; 64],
        }),
        integrity: Some(cache::Integrity {
            merkle_integrity: Some(cache::MerkleIntegrity {
                timestamps: vec![5],
                signatures: vec![vec![2; 64]],
            }),
            ..Default::default()
        }),
        data: data.to_vec(),
        ..Default::default()
    }
}

/// The stored rows of the cache tables: metas, index entries and caches.
fn cache_rows(kv: &MemoryKv) -> [usize; 3] {
    [
        SwarmCacheMetas::NS,
        SwarmCacheMetas::BY_KEY.ns,
        SwarmCaches::NS,
    ]
    .map(|ns| raw_keys(kv, &ns.prefix()).len())
}

#[test]
fn a_swarm_cache_round_trips_and_stays_on_the_device() {
    let kv = MemoryKv::new();
    let store = replicated(&kv);
    let mut sub = store.events().subscribe();
    let c = swarm_cache(b"data");
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        assert!(
            set_cache(&mut tx, b"swarm", b"salt", c.clone())
                .await
                .unwrap()
        );
        tx.commit().await.unwrap();

        let mut tx = store.read().await.unwrap();
        let got = get_cache(&mut tx, b"swarm", b"salt")
            .await
            .unwrap()
            .unwrap();
        assert_ne!(got.id, 0);
        assert_eq!(
            got,
            Cache {
                id: got.id,
                ..c.clone()
            }
        );
        assert_eq!(
            SwarmCacheMetas::get(&mut tx, got.id).await.unwrap(),
            Some(CacheMeta {
                id: got.id,
                swarm_id: b"swarm".to_vec(),
                swarm_salt: b"salt".to_vec(),
                checksum: cache_checksum(&c),
            })
        );
        let ids = SwarmCacheMetas::index_ids(&mut tx, &SwarmCacheMetas::BY_KEY, b"swarmsalt")
            .await
            .unwrap();
        assert_eq!(ids, [got.id]);
        assert_eq!(get_cache(&mut tx, b"swarm", b"").await.unwrap(), None);
        assert!(EventLogs::get_all(&mut tx).await.unwrap().is_empty());
    });
    assert!(drain(&mut sub).is_empty());
    assert_eq!(cache_rows(&kv), [1, 1, 1]);
}

#[test]
fn pairs_with_the_same_cache_key_replace_each_other() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        let c = swarm_cache(b"ab|c");
        assert!(set_cache(&mut tx, b"ab", b"c", c.clone()).await.unwrap());
        assert_eq!(get_cache(&mut tx, b"a", b"bc").await.unwrap(), None);
        assert!(!delete_cache(&mut tx, b"a", b"bc").await.unwrap());
        // the same cache under the other pair is a write
        assert!(set_cache(&mut tx, b"a", b"bc", c).await.unwrap());
        assert_eq!(get_cache(&mut tx, b"ab", b"c").await.unwrap(), None);
        let got = get_cache(&mut tx, b"a", b"bc").await.unwrap().unwrap();
        assert_eq!(got.data, b"ab|c");
        tx.commit().await.unwrap();
    });
    assert_eq!(cache_rows(&kv), [1, 1, 1]);
}

#[test]
fn an_unchanged_cache_checksum_skips_the_write() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let c = swarm_cache(b"data");
    let first = block(async {
        let mut tx = store.write(T0).await.unwrap();
        assert!(
            set_cache(&mut tx, b"swarm", b"salt", c.clone())
                .await
                .unwrap()
        );
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        get_cache(&mut tx, b"swarm", b"salt")
            .await
            .unwrap()
            .unwrap()
    });
    let before = kv.dump();
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        assert!(
            !set_cache(&mut tx, b"swarm", b"salt", c.clone())
                .await
                .unwrap()
        );
        // the stored cache carries its id, which the checksum ignores
        assert!(
            !set_cache(&mut tx, b"swarm", b"salt", first.clone())
                .await
                .unwrap()
        );
        tx.commit().await.unwrap();
    });
    assert_eq!(kv.dump(), before);

    // a changed cache moves to a new id
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        let changed = swarm_cache(b"more data");
        assert!(
            set_cache(&mut tx, b"swarm", b"salt", changed)
                .await
                .unwrap()
        );
        tx.commit().await.unwrap();
        let mut tx = store.read().await.unwrap();
        let got = get_cache(&mut tx, b"swarm", b"salt")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.data, b"more data");
        assert!(got.id > first.id);
        assert_eq!(SwarmCacheMetas::get(&mut tx, first.id).await.unwrap(), None);
        assert_eq!(SwarmCaches::get(&mut tx, first.id).await.unwrap(), None);
    });
    assert_eq!(cache_rows(&kv), [1, 1, 1]);
}

#[test]
fn deleting_a_swarm_cache_deletes_its_data() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        for swarm in [&b"a"[..], b"b"] {
            set_cache(&mut tx, swarm, b"", swarm_cache(swarm))
                .await
                .unwrap();
        }
        assert!(delete_cache(&mut tx, b"a", b"").await.unwrap());
        assert!(!delete_cache(&mut tx, b"a", b"").await.unwrap());
        assert_eq!(get_cache(&mut tx, b"a", b"").await.unwrap(), None);
        assert!(get_cache(&mut tx, b"b", b"").await.unwrap().is_some());
        tx.commit().await.unwrap();
    });
    assert_eq!(cache_rows(&kv), [1, 1, 1]);

    // deleting a meta deletes its cache
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        let id = get_cache(&mut tx, b"b", b"").await.unwrap().unwrap().id;
        assert!(SwarmCacheMetas::delete(&mut tx, id).await.unwrap());
        tx.commit().await.unwrap();
    });
    assert_eq!(cache_rows(&kv), [0, 0, 0]);
}

#[test]
fn the_cache_cap_evicts_the_least_recently_written_cache() {
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    let swarm = |i: usize| format!("swarm-{i}").into_bytes();
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        for i in 0..MAX_SWARM_CACHES {
            set_cache(&mut tx, &swarm(i), b"", swarm_cache(&[i as u8]))
                .await
                .unwrap();
        }
        // rewriting the first cache leaves the second the least recent
        set_cache(&mut tx, &swarm(0), b"", swarm_cache(b"rewritten"))
            .await
            .unwrap();
        set_cache(&mut tx, &swarm(MAX_SWARM_CACHES), b"", swarm_cache(b"new"))
            .await
            .unwrap();
        assert_eq!(get_cache(&mut tx, &swarm(1), b"").await.unwrap(), None);
        for i in (0..=MAX_SWARM_CACHES).filter(|&i| i != 1) {
            assert!(get_cache(&mut tx, &swarm(i), b"").await.unwrap().is_some());
        }
        tx.commit().await.unwrap();
    });
    assert_eq!(cache_rows(&kv), [MAX_SWARM_CACHES; 3]);

    // a store holding more than the cap sheds the excess on its next cache
    let kv = MemoryKv::new();
    let store = new_store(&kv);
    block(async {
        let mut tx = store.write(T0).await.unwrap();
        for i in 0..MAX_SWARM_CACHES + 2 {
            let id = i as u64 + 1;
            let meta = CacheMeta {
                id,
                swarm_id: swarm(i),
                swarm_salt: vec![],
                checksum: 0,
            };
            SwarmCacheMetas::insert(&mut tx, meta).await.unwrap();
            let c = Cache {
                id,
                ..swarm_cache(&[i as u8])
            };
            SwarmCaches::insert(&mut tx, c).await.unwrap();
        }
        set_cache(&mut tx, b"newest", b"", swarm_cache(b"new"))
            .await
            .unwrap();
        for i in 0..3 {
            assert_eq!(get_cache(&mut tx, &swarm(i), b"").await.unwrap(), None);
        }
        assert!(get_cache(&mut tx, &swarm(3), b"").await.unwrap().is_some());
        tx.commit().await.unwrap();
    });
    assert_eq!(cache_rows(&kv), [MAX_SWARM_CACHES; 3]);
}
