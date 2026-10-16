// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::future::Future;
use std::rc::Rc;

use futures_util::{FutureExt, StreamExt};
use proptest::prelude::*;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_kv::MemoryKv;
use strims_proto::chat::v1::{UiConfigHighlight, WhisperThread};
use strims_proto::profile::v1 as profile;
use strims_proto::replication::v1::Event;

use super::*;
use crate::events::Recv;
use crate::schema::chat::{ChatUiConfigHighlights, WhisperThreads};
use crate::schema::profile::Profile;
use crate::seal::StorageKey;
use prost::Message as _;

fn block<T>(f: impl Future<Output = T>) -> T {
    f.now_or_never()
        .expect("memory store operations are ready immediately")
}

const T0: Time = Time::from_unix_millis(1_700_000_000_000);

/// A device of profile 1, with its own kv store.
struct Replica {
    store: ReplicatedStore<MemoryKv>,
    kv: MemoryKv,
}

fn replica(device: u64) -> Replica {
    let kv = MemoryKv::new();
    let store = block(async {
        let s = ProfileStore::create(
            kv.clone(),
            1,
            StorageKey::from_bytes([1; 32]),
            det_rng(device),
            T0,
        )
        .await
        .unwrap();
        let mut tx = s.write(T0).await.unwrap();
        Profile::set(
            &mut tx,
            profile::Profile {
                id: 1,
                device_id: device,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        // Disjoint id ranges per device.
        ProfileIds::set(
            &mut tx,
            ids::from_ranges(&[(device * 1_000_000, (device + 1) * 1_000_000)]).unwrap(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        ReplicatedStore::new(s).await.unwrap()
    });
    Replica { store, kv }
}

impl Replica {
    fn id(&self) -> u64 {
        self.store.replica_id()
    }

    fn checkpoint(&self) -> Checkpoint {
        block(checkpoint(self.store.store(), self.id()))
            .unwrap()
            .unwrap_or_default()
    }

    /// Sends what `to` lacks, as Sync does.
    fn sync_to(&self, to: &Replica, now: Time) {
        let remote = to.checkpoint().version.unwrap_or_default();
        let logs = block(compressed_delta(self.store.store(), &remote)).unwrap();
        block(apply_event_logs(&to.store, self.id(), logs, now)).unwrap();
    }

    fn bootstrap_to(&self, to: &Replica, now: Time) {
        let b = block(dump(self.store.store())).unwrap();
        block(apply_bootstrap(&to.store, b, now)).unwrap();
    }

    fn write<F, Fut>(&self, now: Time, f: F)
    where
        F: FnOnce(WriteTx<MemoryKv>) -> Fut,
        Fut: Future<Output = Result<WriteTx<MemoryKv>, Error>>,
    {
        let tx = block(self.store.write(now)).unwrap();
        match block(f(tx)) {
            Ok(tx) => block(tx.commit()).unwrap(),
            Err(Error::UniqueViolation(_) | Error::Deleted) => {}
            Err(e) => panic!("{e}"),
        }
    }

    fn records(&self) -> Vec<Event> {
        let mut events = block(dump(self.store.store())).unwrap().events;
        events.sort_by_key(|e| (e.namespace, e.id));
        events
    }
}

fn highlight(id: u64, alias: &str, key: u8) -> UiConfigHighlight {
    UiConfigHighlight {
        id,
        alias: alias.into(),
        peer_key: vec![key],
        ..Default::default()
    }
}

async fn upsert_highlight(
    mut tx: WriteTx<MemoryKv>,
    h: UiConfigHighlight,
) -> Result<WriteTx<MemoryKv>, Error> {
    let h = match ChatUiConfigHighlights::get(&mut tx, h.id).await? {
        Some(p) => UiConfigHighlight {
            version: p.version,
            ..h
        },
        None => h,
    };
    ChatUiConfigHighlights::upsert(&mut tx, h).await?;
    Ok(tx)
}

async fn delete_highlight(mut tx: WriteTx<MemoryKv>, id: u64) -> Result<WriteTx<MemoryKv>, Error> {
    ChatUiConfigHighlights::delete(&mut tx, id).await?;
    Ok(tx)
}

async fn touch_thread(
    mut tx: WriteTx<MemoryKv>,
    id: u64,
    key: u8,
    last: i64,
) -> Result<WriteTx<MemoryKv>, Error> {
    WhisperThreads::transform(&mut tx, id, |t| {
        t.id = id;
        t.peer_key = vec![key];
        t.last_message_time = last;
        t.has_unread |= last % 2 == 0;
        Ok(())
    })
    .await?;
    Ok(tx)
}

#[test]
fn a_new_replica_bootstraps_then_syncs() {
    let a = replica(1);
    let b = replica(2);
    a.write(T0, |tx| upsert_highlight(tx, highlight(10, "x", 1)));
    a.write(T0, |tx| touch_thread(tx, 11, 2, 5));
    a.bootstrap_to(&b, T0);
    assert_eq!(a.records(), b.records());
    assert_eq!(b.checkpoint().version.unwrap().value, [(1, 2)].into());

    b.write(T0, |tx| upsert_highlight(tx, highlight(10, "y", 1)));
    b.write(T0, |tx| upsert_highlight(tx, highlight(20, "z", 3)));
    b.sync_to(&a, T0);
    assert_eq!(a.records(), b.records());
    let h = a.records().into_iter().find(|e| e.id == 10).unwrap();
    assert_eq!(h.version.unwrap().value, [(1, 1), (2, 1)].into());
}

#[test]
fn concurrent_edits_converge() {
    let a = replica(1);
    let b = replica(2);
    a.write(T0, |tx| upsert_highlight(tx, highlight(10, "base", 1)));
    a.write(T0, |tx| touch_thread(tx, 11, 2, 5));
    a.bootstrap_to(&b, T0);

    let later = T0 + Duration::SECOND;
    a.write(T0, |tx| upsert_highlight(tx, highlight(10, "from a", 1)));
    b.write(later, |tx| upsert_highlight(tx, highlight(10, "from b", 1)));
    a.write(later, |tx| touch_thread(tx, 11, 2, 9));
    b.write(T0, |tx| touch_thread(tx, 11, 2, 7));
    // Same millisecond: content breaks the tie.
    a.write(T0, |tx| upsert_highlight(tx, highlight(30, "a", 4)));
    b.bootstrap_to(&a, T0);
    a.sync_to(&b, T0);
    a.write(later, |tx| upsert_highlight(tx, highlight(30, "tie a", 4)));
    b.write(later, |tx| upsert_highlight(tx, highlight(30, "tie b", 4)));

    a.sync_to(&b, later);
    b.sync_to(&a, later);
    assert_eq!(a.records(), b.records());
    let alias = |id: u64| {
        let e = a.records().into_iter().find(|e| e.id == id).unwrap();
        UiConfigHighlight::decode(e.record.as_slice())
            .unwrap()
            .alias
    };
    assert_eq!(alias(10), "from b");
    assert_eq!(alias(30), "tie b");
    let t = a.records().into_iter().find(|e| e.id == 11).unwrap();
    assert_eq!(
        WhisperThread::decode(t.record.as_slice())
            .unwrap()
            .last_message_time,
        9
    );
}

#[test]
fn deletes_win_over_concurrent_writes() {
    let a = replica(1);
    let b = replica(2);
    a.write(T0, |tx| upsert_highlight(tx, highlight(10, "x", 1)));
    a.bootstrap_to(&b, T0);
    a.write(T0, |tx| delete_highlight(tx, 10));
    b.write(T0 + Duration::SECOND, |tx| {
        upsert_highlight(tx, highlight(10, "edited", 1))
    });

    b.sync_to(&a, T0);
    a.sync_to(&b, T0);
    assert_eq!(a.records(), b.records());
    assert!(a.records().iter().all(|e| e.id != 10));
}

#[test]
fn unique_keys_converge_on_the_lower_id() {
    let a = replica(1);
    let b = replica(2);
    a.bootstrap_to(&b, T0);
    b.bootstrap_to(&a, T0);
    a.write(T0, |tx| upsert_highlight(tx, highlight(30, "a", 7)));
    b.write(T0, |tx| upsert_highlight(tx, highlight(20, "b", 7)));
    a.sync_to(&b, T0);
    b.sync_to(&a, T0);
    assert_eq!(a.records(), b.records());
    let ids: Vec<u64> = a.records().iter().map(|e| e.id).collect();
    assert_eq!(ids, [20]);
}

#[test]
fn logs_are_collected_once_every_replica_has_them() {
    let a = replica(1);
    let b = replica(2);
    a.write(T0, |tx| upsert_highlight(tx, highlight(10, "x", 1)));
    a.write(T0, |tx| upsert_highlight(tx, highlight(11, "y", 2)));
    a.bootstrap_to(&b, T0);
    b.write(T0, |tx| upsert_highlight(tx, highlight(12, "z", 3)));
    b.sync_to(&a, T0);
    a.sync_to(&b, T0);

    let all = block(checkpoints(a.store.store())).unwrap();
    let threshold = checkpoints::CheckpointMap::new(all).min_version();
    let logs_before = block(dump(a.store.store())).unwrap().logs.len();
    let n = block(garbage_collect(a.store.store(), &threshold, T0)).unwrap();
    assert_eq!(n, logs_before);
    assert!(block(dump(a.store.store())).unwrap().logs.is_empty());
}

#[test]
fn applied_logs_announce_only_progress() {
    let a = replica(1);
    let b = replica(2);
    a.bootstrap_to(&b, T0);
    let mut sub = b.store.events().subscribe();
    a.write(T0, |tx| upsert_highlight(tx, highlight(10, "x", 1)));
    let logs = block(compressed_delta(
        a.store.store(),
        &b.checkpoint().version.unwrap(),
    ))
    .unwrap();
    block(apply_event_logs(&b.store, 1, logs.clone(), T0)).unwrap();
    block(apply_event_logs(&b.store, 1, logs, T0)).unwrap();

    let mut applied = 0;
    while let Some(Some(Recv::Event(e))) = sub.next().now_or_never() {
        if let StoreEvent::LogsApplied { from, logs } = Rc::unwrap_or_clone(e) {
            assert_eq!(from, 1);
            assert_eq!(logs.len(), 1);
            assert!(!logs[0].events[0].record.is_empty());
            applied += 1;
        }
    }
    assert_eq!(applied, 1);
}

#[test]
fn ids_move_between_replicas() {
    let a = replica(1);
    let b = replica(2);
    let given = block(allocate_ids(a.store.store(), 10_000, T0))
        .unwrap()
        .unwrap();
    assert_eq!(ids::free_count(&given), 10_000);
    assert_eq!(block(free_ids(a.store.store())).unwrap(), 990_000);
    block(push_ids(b.store.store(), given, T0)).unwrap();
    assert_eq!(block(free_ids(b.store.store())).unwrap(), 1_010_000);
    let bad = ids::from_ranges(&[(9, 1)]).unwrap();
    assert!(block(push_ids(b.store.store(), bad, T0)).is_err());
}

#[test]
fn replica_count_is_bounded() {
    let a = replica(1);
    let many: Vec<Checkpoint> = (0..MAX_REPLICAS as u64 + 1)
        .map(|id| Checkpoint {
            id: id + 100,
            ..Default::default()
        })
        .collect();
    assert!(matches!(
        block(merge_checkpoints(a.store.store(), &many, T0)),
        Err(Error::Invalid(_))
    ));
    assert!(block(checkpoints(a.store.store())).unwrap().is_empty());
}

#[derive(Clone, Debug)]
enum Op {
    Create {
        at: usize,
        alias: u8,
        key: u8,
        ms: i64,
    },
    Update {
        at: usize,
        pick: usize,
        alias: u8,
        key: u8,
        ms: i64,
    },
    Delete {
        at: usize,
        pick: usize,
        ms: i64,
    },
    Thread {
        at: usize,
        key: u8,
        last: i64,
        ms: i64,
    },
    Touch {
        at: usize,
        pick: usize,
        last: i64,
        ms: i64,
    },
    Sync {
        from: usize,
        to: usize,
    },
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..3usize, 0..4u8, 0..3u8, 0..3i64).prop_map(|(at, alias, key, ms)| Op::Create {
            at,
            alias,
            key,
            ms
        }),
        (0..3usize, 0..4usize, 0..4u8, 0..3u8, 0..3i64).prop_map(|(at, pick, alias, key, ms)| {
            Op::Update {
                at,
                pick,
                alias,
                key,
                ms,
            }
        }),
        (0..3usize, 0..4usize, 0..3i64).prop_map(|(at, pick, ms)| Op::Delete { at, pick, ms }),
        (0..3usize, 0..2u8, 0..20i64, 0..3i64).prop_map(|(at, key, last, ms)| Op::Thread {
            at,
            key,
            last,
            ms
        }),
        (0..3usize, 0..4usize, 0..20i64, 0..3i64).prop_map(|(at, pick, last, ms)| Op::Touch {
            at,
            pick,
            last,
            ms
        }),
        (0..3usize, 0..3usize).prop_map(|(from, to)| Op::Sync { from, to }),
    ]
}

/// The id of the `pick`th record of `T` the replica holds, if any.
fn pick<T: Table>(r: &Replica, pick: usize) -> Option<u64> {
    let all = block(async {
        let mut tx = r.store.read().await.unwrap();
        T::get_all(&mut tx).await.unwrap()
    });
    let n = all.len();
    (n > 0).then(|| all.get(pick % n).map(T::id)).flatten()
}

async fn create_highlight(
    mut tx: WriteTx<MemoryKv>,
    alias: u8,
    key: u8,
) -> Result<WriteTx<MemoryKv>, Error> {
    let id = tx.generate_id().await?;
    ChatUiConfigHighlights::insert(&mut tx, highlight(id, &alias.to_string(), key)).await?;
    Ok(tx)
}

async fn create_thread(
    mut tx: WriteTx<MemoryKv>,
    key: u8,
    last: i64,
) -> Result<WriteTx<MemoryKv>, Error> {
    let id = tx.generate_id().await?;
    touch_thread(tx, id, key, last).await
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn replicas_converge_under_any_delivery_order(ops in proptest::collection::vec(op(), 1..60)) {
        let rs = [replica(1), replica(2), replica(3)];
        rs[0].bootstrap_to(&rs[1], T0);
        rs[0].bootstrap_to(&rs[2], T0);
        let at = |ms: i64| T0 + Duration::from_millis(ms);
        for op in ops {
            match op {
                Op::Create { at: r, alias, key, ms } => rs[r].write(at(ms), |tx| create_highlight(tx, alias, key)),
                Op::Update { at: r, pick: p, alias, key, ms } => {
                    if let Some(id) = pick::<ChatUiConfigHighlights>(&rs[r], p) {
                        rs[r].write(at(ms), |tx| upsert_highlight(tx, highlight(id, &alias.to_string(), key)))
                    }
                }
                Op::Delete { at: r, pick: p, ms } => {
                    if let Some(id) = pick::<ChatUiConfigHighlights>(&rs[r], p) {
                        rs[r].write(at(ms), |tx| delete_highlight(tx, id))
                    }
                }
                Op::Thread { at: r, key, last, ms } => rs[r].write(at(ms), |tx| create_thread(tx, key, last)),
                Op::Touch { at: r, pick: p, last, ms } => {
                    if let Some(id) = pick::<WhisperThreads>(&rs[r], p) {
                        let key = block(async {
                            let mut tx = rs[r].store.read().await.unwrap();
                            WhisperThreads::get(&mut tx, id).await.unwrap().unwrap().peer_key[0]
                        });
                        rs[r].write(at(ms), |tx| touch_thread(tx, id, key, last))
                    }
                }
                Op::Sync { from, to } if from != to => rs[from].sync_to(&rs[to], at(3)),
                Op::Sync { .. } => {}
            }
        }
        for _ in 0..3 {
            for from in 0..3 {
                for to in 0..3 {
                    if from != to {
                        rs[from].sync_to(&rs[to], at(3));
                    }
                }
            }
        }
        prop_assert_eq!(rs[0].records(), rs[1].records());
        prop_assert_eq!(rs[1].records(), rs[2].records());
        prop_assert_eq!(rs[0].kv.open_transactions(), 0);
    }
}
