// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Fake nodes for network controls, linked over in-memory pipes: each
//! peer's RPC channel pair and its broker channel, and a fake vpn that
//! carries messages between ports and holds service addresses. Each fake
//! node keeps the networks it joined and the peers it bound to them. The
//! networks come from real profile stores. Time is tokio's paused clock.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, generate_key, new_certificate_request, sign_certificate_request_with_network,
};
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReplicatedStore, Singleton, Table};
use strims_kademlia::ID;
use strims_kv::MemoryKv;
use strims_network::{Config, Host, NetworkControl};
use strims_proto::network::v1::Network;
use strims_proto::profile::v1::Profile;
use strims_proto::types::{Certificate, Key, KeyUsage};
use strims_rpc::{Router, Timer, peer_connection};
use strims_vpn::{Delivery, HostAddr, M_ENCRYPT, NetworkId};
use tokio::task::{LocalSet, spawn_local};
use tokio::time::{Instant, Sleep};

pub const T0: Time = Time::from_unix_millis(1_700_000_000_000);

#[derive(Clone)]
pub struct TokioTimer {
    pub base: Instant,
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
pub struct Closed;

impl std::fmt::Display for Closed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("closed")
    }
}

#[derive(Default)]
pub struct PipeState {
    queue: VecDeque<Vec<u8>>,
    reader: Option<Waker>,
    closed: bool,
}

#[derive(Clone, Default)]
pub struct Pipe(pub Rc<RefCell<PipeState>>);

impl Pipe {
    pub fn close(&self) {
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

pub struct Reader(Pipe);
pub struct Writer(Pipe);

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

// --- a fake vpn -------------------------------------------------------------

#[derive(Default)]
pub struct PortState {
    queue: VecDeque<Delivery<u16>>,
    reader: Option<Waker>,
}

pub type Port = Rc<RefCell<PortState>>;

/// Every node's vpn ports, by (host, network key, port), and the hash
/// table's service addresses, by (key, salt).
#[derive(Default)]
pub struct WorldState {
    pub ports: BTreeMap<(ID, Vec<u8>, u16), Port>,
    pub next_port: u16,
    pub addresses: BTreeMap<(Vec<u8>, Vec<u8>), HostAddr>,
}

#[derive(Clone, Default)]
pub struct World(pub Rc<RefCell<WorldState>>);

pub struct VpnReader(Port);

impl Stream for VpnReader {
    type Item = Result<Delivery<u16>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut p = self.0.borrow_mut();
        if let Some(d) = p.queue.pop_front() {
            return Poll::Ready(Some(Ok(d)));
        }
        p.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

pub struct VpnWriter {
    world: World,
    src: ID,
    src_port: u16,
    network: Vec<u8>,
    timer: TokioTimer,
}

impl Sink<(HostAddr, Vec<u8>)> for VpnWriter {
    type Error = Closed;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, (to, body): (HostAddr, Vec<u8>)) -> Result<(), Closed> {
        let port = self
            .world
            .0
            .borrow()
            .ports
            .get(&(to.host, self.network.clone(), to.port))
            .cloned();
        let Some(port) = port else {
            return Ok(());
        };
        let w = {
            let mut p = port.borrow_mut();
            p.queue.push_back(Delivery {
                network: NetworkId::default(),
                handler: to.port,
                src: self.src,
                src_port: self.src_port,
                dst: to.host,
                dst_port: to.port,
                flags: M_ENCRYPT,
                hops: 0,
                verified: None,
                body,
                received: self.timer.now(),
            });
            p.reader.take()
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
        Poll::Ready(Ok(()))
    }
}

// --- fake nodes -------------------------------------------------------------

#[derive(Default)]
pub struct NodeState {
    /// Network key -> handle.
    pub networks: BTreeMap<Vec<u8>, u32>,
    pub next_net: u32,
    pub next_port: u16,
    /// (peer, network) -> (local port, peer port).
    pub bound: BTreeMap<(ID, u32), (u16, u16)>,
    /// Each peer's broker channel, until taken.
    pub brokers: BTreeMap<ID, (Reader, Writer)>,
    /// Joining a network fails.
    pub refuse_networks: bool,
    /// How long joining a network takes.
    pub join_delay: std::time::Duration,
}

#[derive(Clone)]
pub struct FakeHost {
    pub id: ID,
    pub state: Rc<RefCell<NodeState>>,
    pub world: World,
    pub timer: TokioTimer,
}

impl FakeHost {
    pub fn network_key(&self, net: u32) -> Option<Vec<u8>> {
        let s = self.state.borrow();
        s.networks
            .iter()
            .find(|(_, n)| **n == net)
            .map(|(k, _)| k.clone())
    }
}

impl Host for FakeHost {
    type Network = u32;
    type BrokerReader = Reader;
    type BrokerWriter = Writer;
    type VpnReader = VpnReader;
    type VpnWriter = VpnWriter;
    type Publication = (Vec<u8>, Vec<u8>);
    type Error = Closed;

    fn host_id(&self) -> ID {
        self.id
    }

    async fn add_network(&self, key: &[u8]) -> Result<u32, Closed> {
        let delay = self.state.borrow().join_delay;
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        let mut s = self.state.borrow_mut();
        if s.refuse_networks {
            return Err(Closed);
        }
        s.next_net += 1;
        let net = s.next_net;
        s.networks.insert(key.to_vec(), net);
        Ok(net)
    }

    async fn remove_network(&self, net: u32) -> Result<(), Closed> {
        let mut s = self.state.borrow_mut();
        s.networks.retain(|_, n| *n != net);
        s.bound.retain(|(_, n), _| *n != net);
        Ok(())
    }

    async fn take_broker(&self, host: ID) -> Result<(Reader, Writer), Closed> {
        self.state.borrow_mut().brokers.remove(&host).ok_or(Closed)
    }

    async fn reserve_peer_port(&self, _: ID) -> Result<u16, Closed> {
        let mut s = self.state.borrow_mut();
        s.next_port += 1;
        Ok(1000 + s.next_port)
    }

    async fn release_peer_port(&self, _: ID, _: u16) -> Result<(), Closed> {
        Ok(())
    }

    async fn bind_peer(
        &self,
        host: ID,
        net: u32,
        local_port: u16,
        peer_port: u16,
    ) -> Result<(), Closed> {
        self.state
            .borrow_mut()
            .bound
            .insert((host, net), (local_port, peer_port));
        Ok(())
    }

    async fn unbind_peer(&self, host: ID, net: u32) -> Result<(), Closed> {
        self.state.borrow_mut().bound.remove(&(host, net));
        Ok(())
    }

    async fn open_vpn_port(&self, net: u32) -> Result<(u16, VpnReader, VpnWriter), Closed> {
        let network = self.network_key(net).ok_or(Closed)?;
        let mut w = self.world.0.borrow_mut();
        w.next_port += 1;
        let port = 1000 + w.next_port;
        let state = Port::default();
        w.ports
            .insert((self.id, network.clone(), port), state.clone());
        let writer = VpnWriter {
            world: self.world.clone(),
            src: self.id,
            src_port: port,
            network,
            timer: self.timer.clone(),
        };
        Ok((port, VpnReader(state), writer))
    }

    async fn publish_address(
        &self,
        _: u32,
        key: Key,
        salt: Vec<u8>,
        port: u16,
    ) -> Result<(Vec<u8>, Vec<u8>), Closed> {
        let k = (key.public, salt);
        self.world
            .0
            .borrow_mut()
            .addresses
            .insert(k.clone(), HostAddr::new(self.id, port));
        Ok(k)
    }

    async fn unpublish_address(&self, k: (Vec<u8>, Vec<u8>)) -> Result<(), Closed> {
        self.world.0.borrow_mut().addresses.remove(&k);
        Ok(())
    }

    async fn resolve_address(
        &self,
        _: u32,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<Option<HostAddr>, Closed> {
        Ok(self.world.0.borrow().addresses.get(&(key, salt)).copied())
    }
}

pub struct Node {
    pub id: ID,
    pub profile: Profile,
    pub host: FakeHost,
    pub store: ReplicatedStore<MemoryKv>,
    pub control: NetworkControl<MemoryKv, FakeHost, TokioTimer>,
}

impl Node {
    pub fn profile_key(&self) -> &Key {
        self.profile.key.as_ref().unwrap()
    }

    /// The (local, peer) ports `peer` is bound to the network on.
    pub fn bound(&self, peer: &Node, network: &Key) -> Option<(u16, u16)> {
        let s = self.host.state.borrow();
        let net = *s.networks.get(&network.public)?;
        s.bound.get(&(peer.id, net)).copied()
    }

    pub async fn write_network(&self, n: Network) {
        let mut tx = self.store.write(T0).await.unwrap();
        Networks::upsert(&mut tx, n).await.unwrap();
        tx.commit().await.unwrap();
    }

    pub async fn delete_network(&self, id: u64) {
        let mut tx = self.store.write(T0).await.unwrap();
        Networks::delete(&mut tx, id).await.unwrap();
        tx.commit().await.unwrap();
    }
}

pub async fn node(n: u8) -> Node {
    node_in(&World::default(), n).await
}

pub async fn node_in(world: &World, n: u8) -> Node {
    let id = ID::from_bytes(&[n; 32]);
    let mut rng = det_rng(u64::from(n));
    let key = generate_key(&mut rng);
    let profile = Profile {
        id: u64::from(n),
        name: format!("node-{n}"),
        key: Some(key),
        device_id: 100 + u64::from(n),
        ..Default::default()
    };
    let kv = MemoryKv::new();
    let store = ProfileStore::create(
        kv,
        u64::from(n),
        StorageKey::from_bytes([n; 32]),
        det_rng(u64::from(n) + 1),
        T0,
    )
    .await
    .unwrap();
    let mut tx = store.write(T0).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();
    let timer = TokioTimer {
        base: Instant::now(),
    };
    let host = FakeHost {
        id,
        state: Rc::default(),
        world: world.clone(),
        timer: timer.clone(),
    };
    let control = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        timer,
        det_rng(u64::from(n) + 2),
        Config::default(),
    );
    let c = control.clone();
    spawn_local(async move { c.run().await });
    Node {
        id,
        profile,
        host,
        store,
        control,
    }
}

/// A member's certificate on the network whose key is `network`.
pub fn membership(network: &Key, profile: &Key, n: u64) -> Certificate {
    let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
    let csr = new_certificate_request(profile, usage, "member").unwrap();
    let mut rng = det_rng(n);
    sign_certificate_request_with_network(&csr, network, "net", DEFAULT_CERT_TTL, T0, &mut rng)
        .unwrap()
}

/// The profile's record of a network it is a member of.
pub fn joined(id: u64, network: &Key, node: &Node) -> Network {
    Network {
        id,
        certificate: Some(membership(network, node.profile_key(), id)),
        ..Default::default()
    }
}

/// Links `a` and `b` as the node would: an RPC channel pair and a broker
/// channel each way. Returns the pipes, to cut the link.
pub fn link(a: &Node, b: &Node) -> Vec<Pipe> {
    let pipes: Vec<Pipe> = (0..6).map(|_| Pipe::default()).collect();
    let [ab, ba, ab2, ba2, ab3, ba3] = <[Pipe; 6]>::try_from(pipes.clone()).ok().unwrap();
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
    a.host
        .state
        .borrow_mut()
        .brokers
        .insert(b.id, (Reader(ba3.clone()), Writer(ab3.clone())));
    b.host
        .state
        .borrow_mut()
        .brokers
        .insert(a.id, (Reader(ab3.clone()), Writer(ba3.clone())));
    a.control
        .add_peer(b.id, &b.profile_key().public, &a_router, a_client);
    b.control
        .add_peer(a.id, &a.profile_key().public, &b_router, b_client);
    spawn_local(async move {
        let _ = a_conn.await;
    });
    spawn_local(async move {
        let _ = b_conn.await;
    });
    pipes
}

pub async fn settle() {
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
}

/// Bound crosswise on both nodes.
pub fn assert_bound(a: &Node, b: &Node, network: &Key) {
    let ab = a.bound(b, network);
    let ba = b.bound(a, network);
    let (Some((al, ap)), Some((bl, bp))) = (ab, ba) else {
        panic!("not bound both ways: {ab:?} {ba:?}");
    };
    assert_eq!((al, ap), (bp, bl));
}

pub fn run(f: impl std::future::Future<Output = ()> + 'static) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f));
}

pub async fn read_network(node: &Node, id: u64) -> Network {
    let mut tx = node.store.read().await.unwrap();
    Networks::get(&mut tx, id).await.unwrap().unwrap()
}
