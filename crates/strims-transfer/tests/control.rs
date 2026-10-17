// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Transfer controls on fake nodes, linked over in-memory channel pairs.
//! Each fake node keeps the swarm channels the control ran, and refuses
//! a second channel per swarm per peer as the ppspp runner does. Time is
//! tokio's paused clock.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use strims_base::time::{Duration, Time};
use strims_kademlia::ID;
use strims_ppspp::SwarmId;
use strims_proto::service::CallError;
use strims_proto::transfer::v1::{
    TransferPeerAnnounceRequest, TransferPeerClient, TransferPeerCloseRequest,
};
use strims_rpc::{Client, Router, Timer, peer_connection};
use strims_transfer::{Config, Host, TransferControl, TransferId};
use tokio::task::{LocalSet, spawn_local};
use tokio::time::{Instant, Sleep};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);
const NET: u8 = 1;

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

// --- fake nodes -------------------------------------------------------------

/// A node operation, as a fake node saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Rec {
    /// A run started (it ends with `Run` or fails).
    RunStart(ID, u32),
    Run(ID, u32, u64, u64),
    Stop(ID, u32),
}

type IndexKey = (u8, Vec<u8>, Vec<u8>);
/// (peer, swarm) → (channel, peer channel).
type Channels = BTreeMap<(ID, u32), (u64, u64)>;

#[derive(Default)]
struct WorldState {
    /// Every node's operations, in the order they happened.
    log: Vec<(ID, Rec)>,
    index: BTreeMap<IndexKey, BTreeSet<ID>>,
    /// Link requests: (from, to).
    connects: Vec<(ID, ID)>,
}

#[derive(Clone, Default)]
struct World(Rc<RefCell<WorldState>>);

#[derive(Clone)]
struct FakeHost {
    id: ID,
    world: World,
    /// The node's swarm channels: (peer, swarm) → (channel, peer channel).
    running: Rc<RefCell<Channels>>,
    /// How long each swarm operation takes.
    delay: std::time::Duration,
    fail_runs: Rc<Cell<bool>>,
}

impl FakeHost {
    fn log(&self, r: Rec) {
        self.world.0.borrow_mut().log.push((self.id, r));
    }
}

impl Host for FakeHost {
    type Network = u8;
    type Swarm = u32;
    type Publication = IndexKey;
    type Search = futures_util::stream::Iter<std::vec::IntoIter<ID>>;
    type Error = String;

    fn host_id(&self) -> ID {
        self.id
    }

    async fn run_swarm(
        &self,
        host: ID,
        swarm: u32,
        channel: u64,
        peer_channel: u64,
    ) -> Result<(), String> {
        self.log(Rec::RunStart(host, swarm));
        tokio::time::sleep(self.delay).await;
        if self.fail_runs.get() {
            return Err("refused".into());
        }
        if self.running.borrow().contains_key(&(host, swarm)) {
            return Err("channel running".into());
        }
        self.running
            .borrow_mut()
            .insert((host, swarm), (channel, peer_channel));
        self.log(Rec::Run(host, swarm, channel, peer_channel));
        Ok(())
    }

    async fn stop_swarm(&self, host: ID, swarm: u32) -> Result<(), String> {
        tokio::time::sleep(self.delay).await;
        self.running.borrow_mut().remove(&(host, swarm));
        self.log(Rec::Stop(host, swarm));
        Ok(())
    }

    async fn index_publish(
        &self,
        net: u8,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<IndexKey, String> {
        let k = (net, key, salt);
        self.world
            .0
            .borrow_mut()
            .index
            .entry(k.clone())
            .or_default()
            .insert(self.id);
        Ok(k)
    }

    async fn index_unpublish(&self, k: IndexKey) -> Result<(), String> {
        if let Some(hosts) = self.world.0.borrow_mut().index.get_mut(&k) {
            hosts.remove(&self.id);
        }
        Ok(())
    }

    async fn index_search(
        &self,
        net: u8,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<Self::Search, String> {
        let w = self.world.0.borrow();
        let hosts = w.index.get(&(net, key, salt));
        let hosts: Vec<ID> = hosts
            .map(|h| h.iter().copied().collect())
            .unwrap_or_default();
        Ok(futures_util::stream::iter(hosts))
    }

    async fn connect_peer(&self, _: u8, host: ID) -> Result<bool, String> {
        self.world.0.borrow_mut().connects.push((self.id, host));
        Ok(true)
    }
}

struct Node {
    id: ID,
    host: FakeHost,
    control: TransferControl<FakeHost, TokioTimer>,
}

fn host_id(n: u8) -> ID {
    ID::from_bytes(&[n; 32])
}

fn node(world: &World, n: u8, config: Config, delay_ms: u64) -> Node {
    let id = host_id(n);
    let host = FakeHost {
        id,
        world: world.clone(),
        running: Rc::default(),
        delay: std::time::Duration::from_millis(delay_ms),
        fail_runs: Rc::default(),
    };
    let timer = TokioTimer {
        base: Instant::now(),
    };
    let control = TransferControl::new(host.clone(), timer, config);
    let c = control.clone();
    spawn_local(async move { c.run().await });
    Node { id, host, control }
}

fn quiet() -> Config {
    Config {
        discovery: false,
        ..Config::default()
    }
}

/// Links `a` and `b` as the node would, and reports both on [`NET`].
/// Returns the pipes (to cut the link) and each side's client.
fn link(a: &Node, b: &Node) -> (Vec<Pipe>, Client, Client) {
    let (ab, ba) = (Pipe::default(), Pipe::default());
    let (ab2, ba2) = (Pipe::default(), Pipe::default());
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
    a.control.add_peer(b.id, &a_router, a_client.clone());
    b.control.add_peer(a.id, &b_router, b_client.clone());
    a.control.add_network_peer(NET, b.id);
    b.control.add_network_peer(NET, a.id);
    spawn_local(async move {
        let _ = a_conn.await;
    });
    spawn_local(async move {
        let _ = b_conn.await;
    });
    (vec![ab, ba, ab2, ba2], a_client, b_client)
}

fn swarm_id() -> SwarmId {
    SwarmId::new(b"swarm")
}

/// Adds the transfer of [`swarm_id`] on `swarm` and publishes it on
/// [`NET`].
fn publish(n: &Node, swarm: u32) -> TransferId {
    let id = n.control.add(swarm, &swarm_id(), Vec::new());
    assert!(n.control.publish(id, NET));
    id
}

/// Waits until `done` holds, at most `limit`.
async fn settle(limit: std::time::Duration, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < limit, "did not settle within {limit:?}");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// Lets every queued operation and call finish.
async fn idle() {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}

/// The node's channels for `swarm` with `peer`.
fn node_channels(n: &Node, peer: ID, swarm: u32) -> Option<(u64, u64)> {
    n.host.running.borrow().get(&(peer, swarm)).copied()
}

/// The control's view of the transfer with `peer` matches the node's.
fn consistent(n: &Node, peer: ID, id: TransferId, swarm: u32) -> bool {
    n.control.channels(peer, id) == node_channels(n, peer, swarm)
}

fn bound(a: &Node, b: &Node, id: TransferId, (sa, sb): (u32, u32)) -> bool {
    match (node_channels(a, b.id, sa), node_channels(b, a.id, sb)) {
        (Some((ca, pa)), Some((cb, pb))) => ca == pb && cb == pa && consistent(a, b.id, id, sa),
        _ => false,
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

use std::future::Future;

// --- tests ------------------------------------------------------------------

#[test]
fn two_nodes_bind_a_transfer_over_peer_rpc() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 0);
        let b = node(&world, 2, quiet(), 0);
        let _link = link(&a, &b);
        let id = publish(&a, 10);
        idle().await;
        assert_eq!(node_channels(&a, b.id, 10), None);
        assert_eq!(publish(&b, 20), id);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (10, 20))
        })
        .await;
        let (ma, mb) = (a.control.metrics(), b.control.metrics());
        assert_eq!((ma.runs, mb.runs), (1, 1));
        assert_eq!(ma.announces_received, 1);
        assert_eq!(ma.announces_answered, 1);
        assert_eq!(mb.announces_accepted, 1);
    });
}

#[test]
fn announces_are_answered_once_the_swarm_runs() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 100);
        let b = node(&world, 2, quiet(), 100);
        let _link = link(&a, &b);
        let id = publish(&a, 10);
        idle().await;
        publish(&b, 20);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (10, 20))
        })
        .await;
        // b's announce makes a run first; b runs only on a's answer
        let log = world.0.borrow().log.clone();
        let a_ran = log
            .iter()
            .position(|(n, r)| *n == a.id && matches!(r, Rec::Run(..)))
            .unwrap();
        let b_started = log
            .iter()
            .position(|(n, r)| *n == b.id && matches!(r, Rec::RunStart(..)))
            .unwrap();
        assert!(a_ran < b_started, "{log:?}");
    });
}

#[test]
fn node_ops_apply_in_transition_order() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 50);
        let b = node(&world, 2, quiet(), 50);
        let _link = link(&a, &b);
        let id = publish(&a, 10);
        idle().await;
        publish(&b, 20);
        // close while a's run is still in progress
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(
            world
                .0
                .borrow()
                .log
                .iter()
                .any(|(n, r)| *n == a.id && matches!(r, Rec::RunStart(..)))
        );
        assert!(b.control.remove(id));
        idle().await;
        assert_eq!(node_channels(&a, b.id, 10), None);
        assert_eq!(node_channels(&b, a.id, 20), None);
        assert!(consistent(&a, b.id, id, 10));
        let a_ops: Vec<Rec> = world
            .0
            .borrow()
            .log
            .iter()
            .filter(|(n, r)| *n == a.id && !matches!(r, Rec::RunStart(..)))
            .map(|(_, r)| r.clone())
            .collect();
        assert!(
            matches!(a_ops[..], [Rec::Run(..), Rec::Stop(..)]),
            "{a_ops:?}"
        );
    });
}

#[test]
fn a_transfer_readded_while_its_announce_is_answered_binds_afresh() {
    run_local(async {
        let world = World::default();
        // a's ops outlast a call attempt, so b's close is sent again
        let a = node(&world, 1, quiet(), 600);
        let b = node(&world, 2, quiet(), 0);
        let _link = link(&a, &b);
        let id = publish(&a, 10);
        idle().await;
        publish(&b, 20);
        // a is running b's announce: b's close and new announce wait for
        // its answer, and the new announce for the close's
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(b.control.remove(id));
        publish(&b, 21);
        settle(std::time::Duration::from_secs(20), || {
            bound(&a, &b, id, (10, 21))
        })
        .await;
        idle().await;
        assert!(bound(&a, &b, id, (10, 21)));
        assert!(consistent(&b, a.id, id, 21));
        assert_eq!(node_channels(&b, a.id, 20), None);
    });
}

#[test]
fn removing_a_transfer_stops_the_peer_channel() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 0);
        let b = node(&world, 2, quiet(), 0);
        let _link = link(&a, &b);
        let id = publish(&a, 10);
        publish(&b, 20);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (10, 20))
        })
        .await;
        assert!(a.control.remove(id));
        settle(std::time::Duration::from_secs(5), || {
            node_channels(&b, a.id, 20).is_none()
        })
        .await;
        assert_eq!(node_channels(&a, b.id, 10), None);
        assert_eq!(b.control.channels(a.id, id), None);
        assert_eq!(b.control.metrics().closes_received, 1);
        // b still publishes it: a's new transfer binds again
        publish(&a, 11);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (11, 20))
        })
        .await;
    });
}

#[test]
fn a_failed_run_refuses_the_announce() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 0);
        let b = node(&world, 2, quiet(), 0);
        let _link = link(&a, &b);
        a.host.fail_runs.set(true);
        let id = publish(&a, 10);
        idle().await;
        publish(&b, 20);
        idle().await;
        assert_eq!(a.control.metrics().run_errors, 1);
        assert_eq!(a.control.metrics().announces_answered, 0);
        assert_eq!(node_channels(&b, a.id, 20), None);
        assert!(consistent(&a, b.id, id, 10));
        assert!(consistent(&b, a.id, id, 20));
    });
}

#[test]
fn a_frame_error_closes_the_peer_until_it_relinks() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 0);
        let b = node(&world, 2, quiet(), 0);
        let (pipes, ..) = link(&a, &b);
        let id = publish(&a, 10);
        publish(&b, 20);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (10, 20))
        })
        .await;
        // the node stops the peer's channels, then reports
        a.host.running.borrow_mut().clear();
        a.control.transfer_failed(b.id);
        settle(std::time::Duration::from_secs(5), || {
            node_channels(&b, a.id, 20).is_none()
        })
        .await;
        idle().await;
        assert_eq!(node_channels(&a, b.id, 10), None);
        assert!(consistent(&a, b.id, id, 10));

        // the link goes and a new one comes
        for p in pipes {
            p.close();
        }
        a.control.remove_peer(b.id);
        b.control.remove_peer(a.id);
        b.host.running.borrow_mut().clear();
        let _link = link(&a, &b);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (10, 20))
        })
        .await;
    });
}

#[test]
fn malformed_transfer_ids_are_refused() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 0);
        let b = node(&world, 2, quiet(), 0);
        let (_, a_client, _) = link(&a, &b);
        let client = TransferPeerClient::new(a_client);
        let res = client
            .announce(&TransferPeerAnnounceRequest {
                id: vec![1, 2, 3],
                channel: 1,
            })
            .await;
        assert!(matches!(res, Err(CallError::Remote(_))), "{res:?}");
        let res = client
            .close(&TransferPeerCloseRequest { id: vec![0; 33] })
            .await;
        assert!(matches!(res, Err(CallError::Remote(_))), "{res:?}");
        // a well-formed announce of an unknown transfer gets no channel
        let res = client
            .announce(&TransferPeerAnnounceRequest {
                id: vec![0; 32],
                channel: 1,
            })
            .await
            .unwrap();
        assert_eq!(res.body, None);
    });
}

#[test]
fn discovery_links_hosts_found_in_the_peer_index() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, Config::default(), 0);
        let c = node(&world, 3, Config::default(), 0);
        let id = publish(&a, 10);
        idle().await;
        publish(&c, 30);
        settle(std::time::Duration::from_secs(5), || {
            world.0.borrow().connects.contains(&(c.id, a.id))
        })
        .await;
        assert!(c.control.metrics().searches >= 1);
        let _link = link(&c, &a);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &c, id, (10, 30))
        })
        .await;

        // searching goes on every interval, but the linked host isn't
        // asked for again
        let before = world.0.borrow().connects.len();
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        assert!(c.control.metrics().searches >= 4);
        assert_eq!(world.0.borrow().connects.len(), before);

        // removing the transfer withdraws it from the index
        a.control.remove(id);
        idle().await;
        let indexed = world.0.borrow().index.values().any(|h| h.contains(&a.id));
        assert!(!indexed);
    });
}

#[test]
fn a_dropped_link_leaves_no_state_behind() {
    run_local(async {
        let world = World::default();
        let a = node(&world, 1, quiet(), 0);
        let b = node(&world, 2, quiet(), 0);
        let (pipes, ..) = link(&a, &b);
        let id = publish(&a, 10);
        publish(&b, 20);
        settle(std::time::Duration::from_secs(5), || {
            bound(&a, &b, id, (10, 20))
        })
        .await;
        for p in pipes {
            p.close();
        }
        a.control.remove_peer(b.id);
        assert_eq!(a.control.channels(b.id, id), None);
        // an op for the gone link is moot: removing the transfer doesn't
        // touch the node
        a.host.running.borrow_mut().clear();
        let ops = world.0.borrow().log.len();
        a.control.remove(id);
        idle().await;
        assert_eq!(world.0.borrow().log.len(), ops);
    });
}
