// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The store watches: `WatchWhispers` and `WatchUIConfig`.

use std::collections::{BTreeMap, VecDeque};

use futures_util::StreamExt;
use futures_util::stream::unfold;
use strims_dao::events::Record;
use strims_dao::schema::chat::{
    ChatUiConfig, ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords,
    WhisperThreads,
};
use strims_dao::{Read, Recv, ReplicatedStore, Singleton, StoreEvent, Subscription, Table};
use strims_kv::Kv;
use strims_proto::chat::v1::watch_ui_config_response::Config;
use strims_proto::chat::v1::watch_whispers_response::{Body, WhisperDelete, WhisperThreadDelete};
use strims_proto::chat::v1::{
    UiConfigHighlight, UiConfigIgnore, UiConfigTag, WatchUiConfigResponse, WatchWhispersResponse,
    WhisperThread,
};
use strims_proto::service::ServerStream;

use super::rpc_error;

/// What a watch makes of the store.
trait Feed: 'static {
    type Response: 'static;

    /// Reads the records the watch follows and queues them, with a
    /// deletion for each record sent before that is gone.
    async fn snapshot(&mut self) -> Result<(), strims_dao::Error>;

    /// Queues what the store event `e` changes.
    async fn event(&mut self, e: &StoreEvent) -> Result<(), strims_dao::Error>;

    /// The queued responses. Bound: one snapshot, plus what one event
    /// makes.
    fn pending(&mut self) -> &mut VecDeque<Self::Response>;
}

struct Watch<F> {
    events: Subscription,
    feed: F,
    /// The subscription lagged, or the watch has yet to read the store.
    resync: bool,
}

/// The responses of `feed`: its snapshot, then what the store's events
/// from `events` on change, taken as the UI takes the responses. A store
/// error ends the stream with that error.
fn watch<F: Feed>(events: Subscription, feed: F) -> ServerStream<F::Response> {
    let w = Watch {
        events,
        feed,
        resync: true,
    };
    unfold(Some(w), |w| async move {
        let mut w = w?;
        loop {
            if let Some(r) = w.feed.pending().pop_front() {
                return Some((Ok(r), Some(w)));
            }
            let res = if w.resync {
                w.resync = false;
                w.feed.snapshot().await
            } else {
                match w.events.next().await {
                    Some(Recv::Event(e)) => w.feed.event(&e).await,
                    Some(Recv::Lagged) => {
                        w.resync = true;
                        Ok(())
                    }
                    None => return None,
                }
            };
            if let Err(e) = res {
                return Some((Err(rpc_error(e)), None));
            }
        }
    })
    .boxed_local()
}

/// The whisper threads, then their changes and those of their whispers,
/// subscribed to with `events`.
pub(super) fn whispers<K: Kv>(
    store: ReplicatedStore<K>,
    events: Subscription,
) -> ServerStream<WatchWhispersResponse> {
    watch(
        events,
        WhispersFeed {
            store,
            threads: BTreeMap::new(),
            pending: VecDeque::new(),
        },
    )
}

/// The UI config, highlights, tags and ignores, then their changes,
/// subscribed to with `events`.
pub(super) fn ui_config<K: Kv>(
    store: ReplicatedStore<K>,
    events: Subscription,
) -> ServerStream<WatchUiConfigResponse> {
    watch(
        events,
        UiConfigFeed {
            store,
            highlights: BTreeMap::new(),
            tags: BTreeMap::new(),
            ignores: BTreeMap::new(),
            pending: VecDeque::new(),
        },
    )
}

/// The feed of a `WatchWhispers` call.
struct WhispersFeed<K: Kv> {
    store: ReplicatedStore<K>,
    /// The threads the UI was sent and holds: their ids by peer key.
    /// Bound: the store's threads.
    threads: BTreeMap<Vec<u8>, u64>,
    pending: VecDeque<WatchWhispersResponse>,
}

fn whisper_response(peer_key: Vec<u8>, body: Body) -> WatchWhispersResponse {
    WatchWhispersResponse {
        peer_key,
        body: Some(body),
    }
}

/// Sets the unread count of `t`, while it has unread whispers.
async fn count_unread<R: Read>(tx: &mut R, t: &mut WhisperThread) -> Result<(), strims_dao::Error> {
    if t.has_unread {
        let ix = &WhisperRecords::UNREAD_BY_PEER_KEY;
        let n = WhisperRecords::index_ids(tx, ix, &t.peer_key).await?.len();
        t.unread_count = u32::try_from(n).unwrap_or(u32::MAX);
    }
    Ok(())
}

impl<K: Kv> WhispersFeed<K> {
    fn thread_update(&mut self, t: WhisperThread) {
        self.threads.insert(t.peer_key.clone(), t.id);
        let peer_key = t.peer_key.clone();
        self.pending
            .push_back(whisper_response(peer_key, Body::ThreadUpdate(t)));
    }

    fn thread_delete(&mut self, peer_key: Vec<u8>) {
        self.threads.remove(&peer_key);
        self.pending.push_back(whisper_response(
            peer_key,
            Body::ThreadDelete(WhisperThreadDelete {}),
        ));
    }
}

impl<K: Kv> Feed for WhispersFeed<K> {
    type Response = WatchWhispersResponse;

    async fn snapshot(&mut self) -> Result<(), strims_dao::Error> {
        let mut tx = self.store.read().await?;
        let mut threads = WhisperThreads::get_all(&mut tx).await?;
        for t in &mut threads {
            count_unread(&mut tx, t).await?;
        }
        let gone: Vec<Vec<u8>> = self
            .threads
            .keys()
            .filter(|k| !threads.iter().any(|t| t.peer_key == **k))
            .cloned()
            .collect();
        for k in gone {
            self.thread_delete(k);
        }
        for t in threads {
            self.thread_update(t);
        }
        Ok(())
    }

    async fn event(&mut self, e: &StoreEvent) -> Result<(), strims_dao::Error> {
        match e {
            StoreEvent::Changed(Record::WhisperThread(t)) => {
                let mut t = t.clone();
                let mut tx = self.store.read().await?;
                count_unread(&mut tx, &mut t).await?;
                self.thread_update(t);
            }
            // a thread replaced by another for its peer is deleted
            // without changing what the UI holds
            StoreEvent::Deleted(Record::WhisperThread(t)) => {
                if self.threads.get(&t.peer_key) == Some(&t.id) {
                    self.thread_delete(t.peer_key.clone());
                }
            }
            StoreEvent::Changed(Record::WhisperRecord(r)) => {
                self.pending.push_back(whisper_response(
                    r.peer_key.clone(),
                    Body::WhisperUpdate(r.clone()),
                ));
            }
            StoreEvent::Deleted(Record::WhisperRecord(r)) => {
                self.pending.push_back(whisper_response(
                    r.peer_key.clone(),
                    Body::WhisperDelete(WhisperDelete { record_id: r.id }),
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn pending(&mut self) -> &mut VecDeque<WatchWhispersResponse> {
        &mut self.pending
    }
}

/// A UI config record kept per peer key: a highlight, tag or ignore.
trait PeerEntry: Clone {
    fn peer_key(&self) -> &[u8];
    fn id(&self) -> u64;
    fn changed(self) -> Config;
    fn deleted(self) -> Config;
}

macro_rules! peer_entry {
    ($record:ty, $changed:ident, $deleted:ident) => {
        impl PeerEntry for $record {
            fn peer_key(&self) -> &[u8] {
                &self.peer_key
            }
            fn id(&self) -> u64 {
                self.id
            }
            fn changed(self) -> Config {
                Config::$changed(self)
            }
            fn deleted(self) -> Config {
                Config::$deleted(self)
            }
        }
    };
}

peer_entry!(
    UiConfigHighlight,
    UiConfigHighlight,
    UiConfigHighlightDelete
);
peer_entry!(UiConfigTag, UiConfigTag, UiConfigTagDelete);
peer_entry!(UiConfigIgnore, UiConfigIgnore, UiConfigIgnoreDelete);

fn config_response(config: Config) -> WatchUiConfigResponse {
    WatchUiConfigResponse {
        config: Some(config),
    }
}

/// The entries of one kind the UI was sent and holds, by peer key. Bound:
/// the store's entries of the kind.
type Sent<R> = BTreeMap<Vec<u8>, R>;

/// Queues `now`, the entries of one kind in the store, with a deletion for
/// each entry in `sent` whose peer has none in `now`.
fn resync<R: PeerEntry>(
    sent: &mut Sent<R>,
    now: Vec<R>,
    pending: &mut VecDeque<WatchUiConfigResponse>,
) {
    let mut next: Sent<R> = now
        .into_iter()
        .map(|r| (r.peer_key().to_vec(), r))
        .collect();
    for (k, r) in std::mem::take(sent) {
        if !next.contains_key(&k) {
            pending.push_back(config_response(r.deleted()));
        }
    }
    for r in next.values() {
        pending.push_back(config_response(r.clone().changed()));
    }
    std::mem::swap(sent, &mut next);
}

fn changed<R: PeerEntry>(sent: &mut Sent<R>, r: &R, pending: &mut VecDeque<WatchUiConfigResponse>) {
    sent.insert(r.peer_key().to_vec(), r.clone());
    pending.push_back(config_response(r.clone().changed()));
}

/// Queues the deletion of `r`, unless the UI holds another entry for its
/// peer.
fn deleted<R: PeerEntry>(sent: &mut Sent<R>, r: &R, pending: &mut VecDeque<WatchUiConfigResponse>) {
    if sent.get(r.peer_key()).is_some_and(|s| s.id() == r.id()) {
        sent.remove(r.peer_key());
        pending.push_back(config_response(r.clone().deleted()));
    }
}

/// The feed of a `WatchUIConfig` call.
struct UiConfigFeed<K: Kv> {
    store: ReplicatedStore<K>,
    highlights: Sent<UiConfigHighlight>,
    tags: Sent<UiConfigTag>,
    ignores: Sent<UiConfigIgnore>,
    pending: VecDeque<WatchUiConfigResponse>,
}

impl<K: Kv> Feed for UiConfigFeed<K> {
    type Response = WatchUiConfigResponse;

    async fn snapshot(&mut self) -> Result<(), strims_dao::Error> {
        let mut tx = self.store.read().await?;
        let config = ChatUiConfig::get(&mut tx).await?;
        let highlights = ChatUiConfigHighlights::get_all(&mut tx).await?;
        let tags = ChatUiConfigTags::get_all(&mut tx).await?;
        let ignores = ChatUiConfigIgnores::get_all(&mut tx).await?;
        if let Some(c) = config {
            self.pending.push_back(config_response(Config::UiConfig(c)));
        }
        resync(&mut self.highlights, highlights, &mut self.pending);
        resync(&mut self.tags, tags, &mut self.pending);
        resync(&mut self.ignores, ignores, &mut self.pending);
        Ok(())
    }

    async fn event(&mut self, e: &StoreEvent) -> Result<(), strims_dao::Error> {
        let p = &mut self.pending;
        match e {
            StoreEvent::Changed(Record::ChatUiConfig(c)) => {
                p.push_back(config_response(Config::UiConfig(c.clone())));
            }
            StoreEvent::Changed(Record::ChatUiConfigHighlight(r)) => {
                changed(&mut self.highlights, r, p);
            }
            StoreEvent::Deleted(Record::ChatUiConfigHighlight(r)) => {
                deleted(&mut self.highlights, r, p);
            }
            StoreEvent::Changed(Record::ChatUiConfigTag(r)) => changed(&mut self.tags, r, p),
            StoreEvent::Deleted(Record::ChatUiConfigTag(r)) => deleted(&mut self.tags, r, p),
            StoreEvent::Changed(Record::ChatUiConfigIgnore(r)) => {
                changed(&mut self.ignores, r, p);
            }
            StoreEvent::Deleted(Record::ChatUiConfigIgnore(r)) => {
                deleted(&mut self.ignores, r, p);
            }
            _ => {}
        }
        Ok(())
    }

    fn pending(&mut self) -> &mut VecDeque<WatchUiConfigResponse> {
        &mut self.pending
    }
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
    use strims_base::time::Time;
    use strims_dao::events::SUBSCRIBER_CAPACITY;
    use strims_dao::schema::profile::Profile as ProfileRecord;
    use strims_dao::seal::StorageKey;
    use strims_dao::{ProfileStore, WriteTx};
    use strims_kv::MemoryKv;
    use strims_proto::chat::v1::{MessageState, WhisperRecord};
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

    /// The responses a watch has ready.
    fn ready<R>(s: &mut ServerStream<R>) -> Vec<R> {
        let mut out = Vec::new();
        while let Some(Some(r)) = s.next().now_or_never() {
            out.push(r.unwrap_or_else(|e| panic!("store error: {e}")));
        }
        out
    }

    fn highlight(id: u64, peer: u8, alias: &str) -> UiConfigHighlight {
        UiConfigHighlight {
            id,
            version: None,
            alias: alias.into(),
            peer_key: vec![peer; 32],
        }
    }

    /// Makes the subscriptions to `store`'s events lag.
    fn flood(store: &ReplicatedStore<MemoryKv>) {
        for i in 0..=SUBSCRIBER_CAPACITY {
            write(store, async |tx| {
                ChatUiConfigHighlights::upsert(tx, highlight(900, 9, &i.to_string())).await?;
                Ok(())
            });
        }
    }

    #[test]
    fn a_ui_config_watch_that_falls_behind_reads_the_store_again() {
        let store = store();
        let mut w = ui_config(store.clone(), store.events().subscribe());
        assert!(ready(&mut w).is_empty());
        let tag = UiConfigTag {
            id: 10,
            peer_key: vec![1; 32],
            color: "red".into(),
            ..Default::default()
        };
        write(&store, async |tx| {
            ChatUiConfigTags::insert(tx, tag.clone()).await?;
            ChatUiConfigHighlights::insert(tx, highlight(11, 2, "a")).await?;
            Ok(())
        });
        let bodies: Vec<Config> = ready(&mut w).into_iter().filter_map(|r| r.config).collect();
        assert!(
            matches!(
                bodies[..],
                [Config::UiConfigTag(_), Config::UiConfigHighlight(_)]
            ),
            "{bodies:?}"
        );

        write(&store, async |tx| {
            ChatUiConfigTags::delete(tx, 10).await?;
            ChatUiConfigHighlights::delete(tx, 11).await?;
            Ok(())
        });
        flood(&store);
        let bodies: Vec<Config> = ready(&mut w).into_iter().filter_map(|r| r.config).collect();
        let [
            Config::UiConfigHighlightDelete(gone_highlight),
            Config::UiConfigHighlight(flooded),
            Config::UiConfigTagDelete(gone_tag),
        ] = &bodies[..]
        else {
            panic!("{bodies:?}");
        };
        assert_eq!((gone_highlight.id, gone_tag.id), (11, 10));
        assert_eq!(flooded.alias, SUBSCRIBER_CAPACITY.to_string());
    }

    #[test]
    fn a_whispers_watch_that_falls_behind_reads_the_threads_again() {
        let store = store();
        let mut w = whispers(store.clone(), store.events().subscribe());
        assert!(ready(&mut w).is_empty());
        let thread = |id: u64, peer: u8, has_unread| WhisperThread {
            id,
            peer_key: vec![peer; 32],
            has_unread,
            ..Default::default()
        };
        let record = |id: u64, state: MessageState| WhisperRecord {
            id,
            peer_key: vec![1; 32],
            state: state as i32,
            ..Default::default()
        };
        write(&store, async |tx| {
            WhisperThreads::insert(tx, thread(1, 1, true)).await?;
            WhisperThreads::insert(tx, thread(2, 2, false)).await?;
            WhisperRecords::insert(tx, record(3, MessageState::Unread)).await?;
            WhisperRecords::insert(tx, record(4, MessageState::Unread)).await?;
            WhisperRecords::insert(tx, record(5, MessageState::Read)).await?;
            Ok(())
        });
        let got = ready(&mut w);
        assert_eq!(got.len(), 5, "{got:?}");
        assert!(matches!(&got[0].body, Some(Body::ThreadUpdate(t)) if t.unread_count == 2));

        write(&store, async |tx| {
            WhisperThreads::delete(tx, 2).await?;
            Ok(())
        });
        flood(&store);
        let got = ready(&mut w);
        let [
            WatchWhispersResponse {
                peer_key: gone,
                body: Some(Body::ThreadDelete(_)),
            },
            WatchWhispersResponse {
                peer_key: kept,
                body: Some(Body::ThreadUpdate(t)),
            },
        ] = &got[..]
        else {
            panic!("{got:?}");
        };
        assert_eq!((gone, kept), (&vec![2; 32], &vec![1; 32]));
        assert_eq!(t.unread_count, 2);
    }
}
