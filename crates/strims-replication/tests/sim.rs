// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Devices of one profile replicating through [`ReplicationControl`]s on
//! in-memory stores, with an in-memory hash table and peer links that the
//! world makes when a control asks the node for one. Time is tokio's
//! paused clock.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_dao::auth::{self, ProfileKey};
use strims_dao::replication::{checkpoints, dump};
use strims_dao::schema::chat::{ChatUiConfigHighlights, WhisperRecords};
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::{Devices, Profile};
use strims_dao::{ReplicatedStore, Singleton, Table as _};
use strims_kademlia::ID;
use strims_kv::MemoryKv;
use strims_proto::chat::v1::{UiConfigHighlight, WhisperRecord};
use strims_proto::network::v1::Network;
use strims_proto::replication::v1::Event;
use strims_proto::types::{Certificate, Key};
use strims_replication::{Config, Host, ReplicationControl, ReplicationFrontend};
use strims_rpc::{Router, Timer, peer_connection};
use tokio::task::{LocalSet, spawn_local};
use tokio::time::{Instant, Sleep};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);

// --- clock ------------------------------------------------------------------

#[derive(Clone)]
struct TokioTimer {
    base: Instant,
}

impl Timer for TokioTimer {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        T0 + Duration::from(self.base.elapsed())
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        let after = (deadline - T0).to_std().unwrap_or_default();
        tokio::time::sleep_until(self.base + after)
    }
}

// --- pipes ------------------------------------------------------------------

#[derive(Debug)]
struct Closed;

#[derive(Default)]
struct PipeState {
    queue: VecDeque<Vec<u8>>,
    reader: Option<Waker>,
    closed: bool,
}

#[derive(Clone, Default)]
struct Pipe(Rc<RefCell<PipeState>>);

impl Pipe {
    fn close(&self) {
        let w = {
            let mut s = self.0.borrow_mut();
            s.closed = true;
            s.reader.take()
        };
        if let Some(w) = w {
            w.wake();
        }
    }
}

struct Reader(Pipe);
struct Writer(Pipe);

impl Stream for Reader {
    type Item = Result<Vec<u8>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut s = self.0.0.borrow_mut();
        if let Some(b) = s.queue.pop_front() {
            return Poll::Ready(Some(Ok(b)));
        }
        if s.closed {
            return Poll::Ready(None);
        }
        s.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Sink<Vec<u8>> for Writer {
    type Error = Closed;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        if self.0.0.borrow().closed {
            return Poll::Ready(Err(Closed));
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Closed> {
        let w = {
            let mut s = self.0.0.borrow_mut();
            s.queue.push_back(item);
            s.reader.take()
        };
        if let Some(w) = w {
            w.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        self.0.close();
        Poll::Ready(Ok(()))
    }
}

// --- world ------------------------------------------------------------------

/// Hash table values by (key, salt).
type Table = BTreeMap<(Vec<u8>, Vec<u8>), BTreeSet<Vec<u8>>>;

/// The shared hash table and the link requests nodes made.
#[derive(Default)]
struct WorldState {
    table: Table,
    requests: Vec<(ID, ID)>,
    partitioned: BTreeSet<(ID, ID)>,
}

#[derive(Clone, Default)]
struct World(Rc<RefCell<WorldState>>);

#[derive(Clone)]
struct FakeHost {
    id: ID,
    world: World,
}

impl Host for FakeHost {
    type Network = u8;
    type Publication = (Vec<u8>, Vec<u8>, Vec<u8>);
    type Error = String;

    fn host_id(&self) -> ID {
        self.id
    }

    async fn networks(&self) -> Vec<u8> {
        vec![1]
    }

    async fn publish(
        &self,
        _: u8,
        key: &Key,
        salt: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<Self::Publication, String> {
        let k = (key.public.clone(), salt);
        self.world
            .0
            .borrow_mut()
            .table
            .entry(k.clone())
            .or_default()
            .insert(value.clone());
        Ok((k.0, k.1, value))
    }

    async fn unpublish(&self, p: Self::Publication) -> Result<(), String> {
        if let Some(values) = self.world.0.borrow_mut().table.get_mut(&(p.0, p.1)) {
            values.remove(&p.2);
        }
        Ok(())
    }

    async fn lookup(
        &self,
        _: u8,
        key: Vec<u8>,
        salt: Vec<u8>,
        max: usize,
    ) -> Result<Vec<Vec<u8>>, String> {
        let w = self.world.0.borrow();
        Ok(w.table
            .get(&(key, salt))
            .map(|v| v.iter().take(max).cloned().collect())
            .unwrap_or_default())
    }

    async fn connect_peer(&self, _: u8, host: ID) -> Result<bool, String> {
        self.world.0.borrow_mut().requests.push((self.id, host));
        Ok(true)
    }
}

/// One device: its kv, store and control.
struct Device {
    host: ID,
    kv: MemoryKv,
    store: ReplicatedStore<MemoryKv>,
    control: ReplicationControl<MemoryKv, FakeHost, TokioTimer>,
    profile_key: Vec<u8>,
}

fn host_id(n: u8) -> ID {
    ID::from_bytes(&[n; 32])
}

fn config() -> Config {
    Config {
        push_queue: 8,
        ..Config::default()
    }
}

/// The first device of a new profile, with a network to pair through.
async fn first_device(world: &World, timer: &TokioTimer, seed: u64) -> (Device, ProfileKey) {
    let kv = MemoryKv::new();
    let mut rng = det_rng(seed);
    let (pk, store) = auth::sign_up(&kv, "alice", "pw", "linux", &mut rng, timer.now())
        .await
        .unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();
    let mut tx = store.write(timer.now()).await.unwrap();
    let id = tx.generate_id().await.unwrap();
    Networks::insert(
        &mut tx,
        Network {
            id,
            certificate: Some(Certificate {
                key: vec![9; 32],
                not_after: u64::MAX,
                ..Default::default()
            }),
            alias: "net".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (start(world, timer, 1, kv, store, seed).await, pk)
}

/// A new device paired with `from`.
async fn pair(
    world: &World,
    timer: &TokioTimer,
    from: &Device,
    pk: &ProfileKey,
    n: u8,
    seed: u64,
) -> Device {
    let token = ReplicationFrontend::new(from.store.clone(), timer.clone())
        .pairing_token(0, 0)
        .await
        .unwrap();
    let kv = MemoryKv::new();
    let mut rng = det_rng(seed + u64::from(n));
    let store = auth::import_pairing_token(&kv, &token, pk, &mut rng, timer.now())
        .await
        .unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();
    let mut tx = store.write(timer.now()).await.unwrap();
    let device = store.replica_id();
    Devices::transform(&mut tx, device, |d| {
        d.id = device;
        d.os = format!("device {n}");
        Ok(())
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();
    start(world, timer, n, kv, store, seed).await
}

async fn start(
    world: &World,
    timer: &TokioTimer,
    n: u8,
    kv: MemoryKv,
    store: ReplicatedStore<MemoryKv>,
    seed: u64,
) -> Device {
    let mut tx = store.read().await.unwrap();
    let profile = Profile::get(&mut tx).await.unwrap().unwrap();
    drop(tx);
    let profile_key = profile.key.as_ref().unwrap().public.clone();
    let host = FakeHost {
        id: host_id(n),
        world: world.clone(),
    };
    let control = ReplicationControl::new(
        store.clone(),
        profile,
        host,
        timer.clone(),
        det_rng(seed ^ u64::from(n)),
        config(),
    );
    let c = control.clone();
    spawn_local(async move { c.run().await });
    Device {
        host: host_id(n),
        kv,
        store,
        control,
        profile_key,
    }
}

/// Links `a` and `b` as the node would: a channel pair each way, served
/// and called through `peer_connection`.
fn link(a: &Device, b: &Device) -> Vec<Pipe> {
    let (ab, ba) = (Pipe::default(), Pipe::default());
    let (ab2, ba2) = (Pipe::default(), Pipe::default());
    // a's client channel is b's server channel, and the reverse.
    let a_router = Router::new();
    let b_router = Router::new();
    let (a_client, a_conn) = peer_connection(
        strims_rpc::Config::peer(),
        (Reader(ba.clone()), Writer(ab.clone())),
        (Reader(ba2.clone()), Writer(ab2.clone())),
        a_router.clone(),
    );
    let (b_client, b_conn) = peer_connection(
        strims_rpc::Config::peer(),
        (Reader(ab2.clone()), Writer(ba2.clone())),
        (Reader(ab.clone()), Writer(ba.clone())),
        b_router.clone(),
    );
    assert!(
        a.control
            .add_peer(b.host, &b.profile_key, &a_router, a_client)
    );
    assert!(
        b.control
            .add_peer(a.host, &a.profile_key, &b_router, b_client)
    );
    spawn_local(async move {
        let _ = a_conn.await;
    });
    spawn_local(async move {
        let _ = b_conn.await;
    });
    vec![ab, ba, ab2, ba2]
}

/// Makes the links nodes asked for, once per pair.
fn serve_links(world: &World, devices: &[&Device], linked: &mut BTreeSet<(ID, ID)>) {
    let requests = std::mem::take(&mut world.0.borrow_mut().requests);
    for (from, to) in requests {
        let pair = if from < to { (from, to) } else { (to, from) };
        if linked.contains(&pair) || world.0.borrow().partitioned.contains(&pair) {
            continue;
        }
        let a = devices.iter().find(|d| d.host == from);
        let b = devices.iter().find(|d| d.host == to);
        if let (Some(a), Some(b)) = (a, b) {
            linked.insert(pair);
            link(a, b);
        }
    }
}

async fn records(d: &Device) -> Vec<Event> {
    let mut events = dump(d.store.store()).await.unwrap().events;
    events.sort_by_key(|e| (e.namespace, e.id));
    events
}

async fn converged(devices: &[&Device]) -> bool {
    let first = records(devices[0]).await;
    for d in &devices[1..] {
        if records(d).await != first {
            return false;
        }
    }
    true
}

async fn highlight(d: &Device, timer: &TokioTimer, alias: &str) -> u64 {
    let mut tx = d.store.write(timer.now()).await.unwrap();
    let id = tx.generate_id().await.unwrap();
    ChatUiConfigHighlights::insert(
        &mut tx,
        UiConfigHighlight {
            id,
            alias: alias.into(),
            peer_key: alias.as_bytes().to_vec(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

/// Steps the world until `done` holds or `limit` passes. Panics on
/// timeout.
async fn settle<F: AsyncFnMut() -> bool>(
    world: &World,
    devices: &[&Device],
    linked: &mut BTreeSet<(ID, ID)>,
    limit: Duration,
    mut done: F,
) {
    let start = Instant::now();
    loop {
        serve_links(world, devices, linked);
        if done().await {
            return;
        }
        if Duration::from(start.elapsed()) >= limit {
            for d in devices {
                let ids: Vec<(i64, u64)> = records(d)
                    .await
                    .iter()
                    .map(|e| (e.namespace, e.id))
                    .collect();
                eprintln!("{:?} {:?}\n  {:?}", d.host, d.control.metrics(), ids);
            }
            panic!("did not settle within {limit}; links {linked:?}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn run_local<F: Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f))
}

/// Pairs a device, lets both find each other, writes on each, and returns
/// a trace of the final state.
fn pair_and_converge(seed: u64) -> String {
    run_local(async move {
        let world = World::default();
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let (a, pk) = first_device(&world, &timer, seed).await;
        highlight(&a, &timer, "before pairing").await;
        let b = pair(&world, &timer, &a, &pk, 2, seed).await;
        let mut linked = BTreeSet::new();
        let devices = [&a, &b];

        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(60),
            async || converged(&devices).await,
        )
        .await;
        assert_eq!(linked.len(), 1, "one link between the devices");

        highlight(&a, &timer, "from a").await;
        highlight(&b, &timer, "from b").await;
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(10),
            async || converged(&devices).await,
        )
        .await;
        assert_eq!(
            records(&a)
                .await
                .iter()
                .filter(|e| e.namespace == 6018)
                .count(),
            3
        );

        // Logs everyone has seen are collected.
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(30),
            async || {
                a.control.metrics().logs_collected > 0 && b.control.metrics().logs_collected > 0
            },
        )
        .await;
        assert_eq!(a.kv.open_transactions(), 0);
        assert_eq!(b.kv.open_transactions(), 0);

        let mut trace = format!("profile {}\n", a.store.store().profile_id());
        for d in devices {
            let m = d.control.metrics();
            trace += &format!(
                "{:?} {:?}\n",
                d.host,
                (m.opens, m.syncs_sent, m.bootstraps_sent, m.logs_sent)
            );
            trace += &format!("{:?}\n", records(d).await);
            trace += &format!("{:?}\n", checkpoints(d.store.store()).await.unwrap());
        }
        trace
    })
}

#[test]
fn paired_devices_find_each_other_and_converge() {
    let a = pair_and_converge(7);
    assert_eq!(a, pair_and_converge(7), "same seed, same run");
    assert_ne!(a, pair_and_converge(8));
}

#[test]
fn logs_reach_devices_through_a_middle_device() {
    run_local(async {
        let world = World::default();
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let (a, pk) = first_device(&world, &timer, 1).await;
        let b = pair(&world, &timer, &a, &pk, 2, 1).await;
        let mut linked = BTreeSet::new();
        settle(
            &world,
            &[&a, &b],
            &mut linked,
            Duration::from_secs(60),
            async || converged(&[&a, &b]).await,
        )
        .await;
        // c pairs through b, and never links to a.
        let c = pair(&world, &timer, &b, &pk, 3, 1).await;
        world.0.borrow_mut().partitioned.insert(if a.host < c.host {
            (a.host, c.host)
        } else {
            (c.host, a.host)
        });
        let devices = [&a, &b, &c];
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(60),
            async || converged(&devices).await,
        )
        .await;
        assert_eq!(linked.len(), 2);

        let before = c.control.metrics().logs_received;
        highlight(&a, &timer, "forwarded").await;
        // Forwarded live, well before the next scan or resync.
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(2),
            async || converged(&devices).await,
        )
        .await;
        assert!(c.control.metrics().logs_received > before);
    });
}

#[test]
fn a_slow_peer_resyncs_after_its_queue_overflows() {
    run_local(async {
        let world = World::default();
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let (a, pk) = first_device(&world, &timer, 3).await;
        let b = pair(&world, &timer, &a, &pk, 2, 3).await;
        let mut linked = BTreeSet::new();
        let devices = [&a, &b];
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(60),
            async || converged(&devices).await,
        )
        .await;

        // More logs than the push queue holds, written without yielding.
        for i in 0..40 {
            highlight(&a, &timer, &format!("burst {i}")).await;
        }
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(30),
            async || converged(&devices).await,
        )
        .await;
    });
}

#[test]
fn large_stores_bootstrap_in_parts() {
    run_local(async {
        let world = World::default();
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let (a, pk) = first_device(&world, &timer, 4).await;
        // About 1.5 MiB of messages.
        let mut tx = a.store.write(timer.now()).await.unwrap();
        for i in 0..300u64 {
            let id = tx.generate_id().await.unwrap();
            WhisperRecords::insert(
                &mut tx,
                WhisperRecord {
                    id,
                    peer_key: vec![1; 32],
                    peer_message_id: i + 1,
                    message: Some(strims_proto::chat::v1::Message {
                        body: "x".repeat(5000),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();
        let b = pair(&world, &timer, &a, &pk, 2, 4).await;
        let mut linked = BTreeSet::new();
        let devices = [&a, &b];
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(60),
            async || converged(&devices).await,
        )
        .await;
        let sent = a.control.metrics().bootstraps_sent + b.control.metrics().bootstraps_sent;
        assert!(sent >= 4, "bootstrapped in {sent} parts");
    });
}

#[test]
fn deleted_devices_stop_replicating() {
    run_local(async {
        let world = World::default();
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let (a, pk) = first_device(&world, &timer, 5).await;
        let b = pair(&world, &timer, &a, &pk, 2, 5).await;
        let mut linked = BTreeSet::new();
        let devices = [&a, &b];
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(60),
            async || converged(&devices).await,
        )
        .await;

        let mut tx = a.store.write(timer.now()).await.unwrap();
        assert!(
            Devices::delete(&mut tx, b.store.replica_id())
                .await
                .unwrap()
        );
        tx.commit().await.unwrap();

        // b learns it was deleted and stops; a refuses it from then on.
        let b_device = b.store.replica_id();
        settle(
            &world,
            &devices,
            &mut linked,
            Duration::from_secs(30),
            async || {
                let c = strims_dao::replication::checkpoint(b.store.store(), b_device)
                    .await
                    .unwrap();
                c.is_some_and(|c| c.deleted)
            },
        )
        .await;
        highlight(&b, &timer, "after deletion").await;
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        assert!(records(&a).await.iter().all(|e| {
            e.namespace != ChatUiConfigHighlights::NS.0
                || !String::from_utf8_lossy(&e.record).contains("after deletion")
        }));
    });
}
