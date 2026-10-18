// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use futures_util::StreamExt;
use futures_util::stream::unfold;
use strims_dao::events::Record;
use strims_dao::schema::notification::Notifications;
use strims_dao::{Recv, ReplicatedStore, StoreEvent, Subscription, Table};
use strims_kv::Kv;
use strims_proto::notification::v1::event::Body;
use strims_proto::notification::v1::{
    DismissRequest, DismissResponse, Event, Notification, NotificationFrontendService,
    WatchRequest, WatchResponse,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_rpc::Timer;

use crate::rpc_error;

/// Serves `strims.notification.v1.NotificationFrontend` for a profile's
/// store: `Watch` streams its notifications and their changes, and
/// `Dismiss` deletes them.
pub struct NotificationService<K: Kv, T: Timer> {
    store: ReplicatedStore<K>,
    timer: T,
}

impl<K: Kv, T: Timer> NotificationService<K, T> {
    /// The service of `store`.
    pub fn new(store: ReplicatedStore<K>, timer: T) -> Self {
        NotificationService { store, timer }
    }
}

impl<K: Kv, T: Timer> NotificationFrontendService for NotificationService<K, T> {
    async fn watch(
        &self,
        _: &Context,
        _: WatchRequest,
    ) -> Result<ServerStream<WatchResponse>, RpcError> {
        Ok(watch(self.store.clone()))
    }

    /// Ids the store doesn't hold are ignored.
    async fn dismiss(&self, _: &Context, req: DismissRequest) -> Result<DismissResponse, RpcError> {
        let ids: BTreeSet<u64> = req.ids.into_iter().collect();
        let now = self.timer.now();
        let mut tx = self.store.write(now).await.map_err(rpc_error)?;
        for n in Notifications::get_all(&mut tx).await.map_err(rpc_error)? {
            if ids.contains(&n.id) {
                Notifications::delete(&mut tx, n.id)
                    .await
                    .map_err(rpc_error)?;
            }
        }
        tx.commit().await.map_err(rpc_error)?;
        Ok(DismissResponse {})
    }
}

fn response(body: Body) -> WatchResponse {
    WatchResponse {
        event: Some(Event { body: Some(body) }),
    }
}

/// A `Watch` call's state. The UI appends every notification it is sent
/// and removes one only when it is dismissed, so a notification goes out
/// once while the UI holds it, and a changed one is dismissed before it
/// goes out again.
struct Watch<K: Kv> {
    store: ReplicatedStore<K>,
    events: Subscription,
    /// The notifications the UI holds, by id. Bound: the store's
    /// notifications.
    sent: BTreeMap<u64, Notification>,
    /// Bound: one snapshot, or what one event makes.
    pending: VecDeque<WatchResponse>,
    /// The subscription lagged, or the watch has yet to read the store.
    resync: bool,
}

impl<K: Kv> Watch<K> {
    /// Brings the UI up to the store: dismisses what it holds that the
    /// store doesn't, and sends the rest, oldest first.
    async fn snapshot(&mut self) -> Result<(), strims_dao::Error> {
        let mut tx = self.store.read().await?;
        let mut all = Notifications::get_all(&mut tx).await?;
        drop(tx);
        all.sort_by_key(|n| (n.created_at, n.id));
        let held: BTreeSet<u64> = all.iter().map(|n| n.id).collect();
        let gone: Vec<u64> = self
            .sent
            .keys()
            .filter(|id| !held.contains(id))
            .copied()
            .collect();
        for id in gone {
            self.dismiss(id);
        }
        for n in all {
            self.send(n);
        }
        Ok(())
    }

    fn event(&mut self, e: &StoreEvent) {
        match e {
            StoreEvent::Changed(Record::Notification(n)) => self.send(n.clone()),
            StoreEvent::Deleted(Record::Notification(n)) => self.dismiss(n.id),
            _ => {}
        }
    }

    fn send(&mut self, n: Notification) {
        match self.sent.get(&n.id) {
            Some(p) if *p == n => return,
            Some(_) => self.pending.push_back(response(Body::Dismiss(n.id))),
            None => {}
        }
        self.sent.insert(n.id, n.clone());
        self.pending.push_back(response(Body::Notification(n)));
    }

    fn dismiss(&mut self, id: u64) {
        if self.sent.remove(&id).is_some() {
            self.pending.push_back(response(Body::Dismiss(id)));
        }
    }
}

/// The notifications of `store`, then their changes. A store error ends
/// the stream with that error.
fn watch<K: Kv>(store: ReplicatedStore<K>) -> ServerStream<WatchResponse> {
    let events = store.events().subscribe();
    let w = Watch {
        store,
        events,
        sent: BTreeMap::new(),
        pending: VecDeque::new(),
        resync: true,
    };
    unfold(Some(w), |w| async move {
        let mut w = w?;
        loop {
            if let Some(r) = w.pending.pop_front() {
                return Some((Ok(r), Some(w)));
            }
            if w.resync {
                w.resync = false;
                if let Err(e) = w.snapshot().await {
                    return Some((Err(rpc_error(e)), None));
                }
                continue;
            }
            match w.events.next().await {
                Some(Recv::Event(e)) => w.event(&e),
                Some(Recv::Lagged) => w.resync = true,
                None => return None,
            }
        }
    })
    .boxed_local()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use std::future::Future;

    use futures_util::FutureExt;
    use strims_base::rng::det_rng;
    use strims_base::time::{Duration, Time};
    use strims_dao::events::SUBSCRIBER_CAPACITY;
    use strims_dao::schema::notification::{dispatch, network_subject, new_notification};
    use strims_dao::schema::profile::Profile as ProfileRecord;
    use strims_dao::seal::StorageKey;
    use strims_dao::{ProfileStore, Singleton, WriteTx};
    use strims_kv::MemoryKv;
    use strims_proto::notification::v1::notification::Status;
    use strims_proto::profile::v1::Profile;

    use super::*;

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);

    fn block<T>(f: impl Future<Output = T>) -> T {
        f.now_or_never().expect("memory store operations are ready")
    }

    fn store() -> ReplicatedStore<MemoryKv> {
        block(async {
            let kv = MemoryKv::new();
            let key = StorageKey::from_bytes([1; 32]);
            let store = ProfileStore::create(kv, 1, key, det_rng(1), T0)
                .await
                .unwrap();
            let mut tx = store.write(T0).await.unwrap();
            let profile = Profile {
                id: 1,
                device_id: 7,
                ..Default::default()
            };
            ProfileRecord::set(&mut tx, profile).await.unwrap();
            tx.commit().await.unwrap();
            ReplicatedStore::new(store).await.unwrap()
        })
    }

    /// Runs `f` in a write transaction of `store`.
    fn write(
        store: &ReplicatedStore<MemoryKv>,
        f: impl AsyncFnOnce(&mut WriteTx<MemoryKv>) -> Result<(), strims_dao::Error>,
    ) {
        block(async {
            let mut tx = store.write(T0).await.unwrap();
            f(&mut tx).await.unwrap();
            tx.commit().await.unwrap();
        });
    }

    /// The bodies a watch has ready.
    fn ready(s: &mut ServerStream<WatchResponse>) -> Vec<Body> {
        let mut out = Vec::new();
        while let Some(Some(r)) = s.next().now_or_never() {
            let r = r.unwrap_or_else(|e| panic!("store error: {e}"));
            out.push(r.event.unwrap().body.unwrap());
        }
        out
    }

    /// Notification `id` about network `network`, raised `ms` after T0.
    fn note(id: u64, network: u64, ms: i64) -> Notification {
        new_notification(
            id,
            T0 + Duration::from_millis(ms),
            Status::Success,
            "renewed",
            "",
            Some(network_subject(network)),
        )
    }

    fn ids(bodies: &[Body]) -> Vec<(bool, u64)> {
        bodies
            .iter()
            .map(|b| match b {
                Body::Notification(n) => (true, n.id),
                Body::Dismiss(id) => (false, *id),
            })
            .collect()
    }

    #[test]
    fn a_watch_sends_the_stored_notifications_oldest_first() {
        let store = store();
        write(&store, async |tx| {
            Notifications::insert(tx, note(1, 10, 5)).await?;
            Notifications::insert(tx, note(2, 20, 1)).await?;
            Ok(())
        });
        let mut w = watch(store.clone());
        assert_eq!(ids(&ready(&mut w)), [(true, 2), (true, 1)]);
        assert!(ready(&mut w).is_empty());
    }

    #[test]
    fn a_watch_follows_new_replaced_and_deleted_notifications() {
        let store = store();
        let mut w = watch(store.clone());
        assert!(ready(&mut w).is_empty());

        write(&store, async |tx| {
            dispatch(tx, note(1, 10, 0)).await?;
            Ok(())
        });
        assert_eq!(ids(&ready(&mut w)), [(true, 1)]);

        // a newer notification about the same network
        write(&store, async |tx| {
            dispatch(tx, note(2, 10, 1)).await?;
            Ok(())
        });
        assert_eq!(ids(&ready(&mut w)), [(false, 1), (true, 2)]);

        // a changed one goes out again, after its dismissal
        write(&store, async |tx| {
            let mut n = note(2, 10, 1);
            n.title = "changed".into();
            Notifications::upsert(tx, n).await?;
            Ok(())
        });
        let bodies = ready(&mut w);
        assert_eq!(ids(&bodies), [(false, 2), (true, 2)]);
        assert!(matches!(&bodies[1], Body::Notification(n) if n.title == "changed"));

        write(&store, async |tx| {
            Notifications::delete(tx, 2).await?;
            Ok(())
        });
        assert_eq!(ids(&ready(&mut w)), [(false, 2)]);
    }

    #[test]
    fn a_watch_that_falls_behind_sends_only_what_changed() {
        let store = store();
        write(&store, async |tx| {
            Notifications::insert(tx, note(1, 10, 0)).await?;
            Notifications::insert(tx, note(2, 20, 1)).await?;
            Ok(())
        });
        let mut w = watch(store.clone());
        assert_eq!(ids(&ready(&mut w)), [(true, 1), (true, 2)]);

        write(&store, async |tx| {
            Notifications::delete(tx, 1).await?;
            Ok(())
        });
        for i in 0..=SUBSCRIBER_CAPACITY {
            write(&store, async |tx| {
                let mut n = note(3, 30, 2);
                n.message = i.to_string();
                Notifications::upsert(tx, n).await?;
                Ok(())
            });
        }
        let bodies = ready(&mut w);
        assert_eq!(ids(&bodies), [(false, 1), (true, 3)]);
        let last = SUBSCRIBER_CAPACITY.to_string();
        assert!(matches!(&bodies[1], Body::Notification(n) if n.message == last));
    }

    #[test]
    fn dismiss_deletes_the_notifications_it_names() {
        let store = store();
        write(&store, async |tx| {
            Notifications::insert(tx, note(1, 10, 0)).await?;
            Notifications::insert(tx, note(2, 20, 1)).await?;
            Ok(())
        });
        let svc = NotificationService::new(store.clone(), FixedTimer);
        let req = DismissRequest { ids: vec![1, 99] };
        block(svc.dismiss(&Context::default(), req)).unwrap();
        let mut tx = block(store.read()).unwrap();
        let left = block(Notifications::get_all(&mut tx)).unwrap();
        assert_eq!(left, [note(2, 20, 1)]);
    }

    #[derive(Clone)]
    struct FixedTimer;

    impl Timer for FixedTimer {
        type Sleep = std::future::Pending<()>;

        fn now(&self) -> Time {
            T0
        }

        fn sleep_until(&self, _: Time) -> Self::Sleep {
            std::future::pending()
        }
    }
}
