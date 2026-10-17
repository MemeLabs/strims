// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Fake nodes for directory controls, each with its network and transfer
//! controls, in one fake world. Time is tokio's paused clock.
//!
//! - **Links.** In-memory pipes carry each linked peer's RPC channel pair
//!   and broker channel, for network membership.
//! - **Vpn.** The world carries messages between any two nodes' ports on
//!   the same network, holds the hash table's service addresses, and
//!   gives ports of a chosen number. A node can be made to open a chosen
//!   port under another number.
//! - **Swarms.** Transfers between nodes aren't modelled: every node's
//!   swarm with the same id and label (the label stands for the transfer
//!   salt, which tells two swarms with one id apart) shares one log in
//!   the world. A log keeps the last window of the stream (the swarm's
//!   live window in bytes) and its epoch. Creating a swarm, or resetting
//!   its writer, starts a new epoch at offset 0. A reader starts at the
//!   window's tail, reads at absolute offsets, and fails once with a
//!   discontinuity when the epoch changes or the window passed it.
//!   Removing a swarm closes its readers and writer.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code
)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::future::poll_fn;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::StreamExt;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, new_certificate_request, sign_certificate_request_with_network,
};
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, Recv, ReplicatedStore, Singleton, Subscription, Table};
use strims_directory::{DirectoryControl, DirectoryEvent};
use strims_kademlia::ID;
use strims_kv::MemoryKv;
use strims_network::NetworkControl;
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0, new_network};
use strims_ppspp::{SwarmId, SwarmOptions, WriterOptions};
use strims_proto::network::v1::Network;
use strims_proto::profile::v1::Profile;
use strims_proto::types::{Key, KeyUsage};
use strims_rpc::{Client, Router, Timer, connect, peer_connection};
use strims_transfer::{ReadFailure, SwarmRead, SwarmWrite, TransferControl};
use strims_vpn::{Delivery, HostAddr, M_ENCRYPT, NetworkId};
use tokio::task::{JoinHandle, LocalSet, spawn_local};
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

// --- the world --------------------------------------------------------------

#[derive(Default)]
pub struct PortState {
    queue: VecDeque<Delivery<u16>>,
    reader: Option<Waker>,
}

pub type Port = Rc<RefCell<PortState>>;

/// A swarm's stream, shared by every node's swarm with its id and label.
#[derive(Default)]
pub struct LogState {
    pub epoch: u64,
    /// The last `window` bytes of the stream, from offset `tail`.
    pub data: VecDeque<u8>,
    pub tail: u64,
    pub window: usize,
    /// Bytes written in all epochs.
    pub written: u64,
    wakers: Vec<Waker>,
}

impl LogState {
    fn wake(&mut self) -> Vec<Waker> {
        std::mem::take(&mut self.wakers)
    }
}

pub type Log = Rc<RefCell<LogState>>;

fn wake_all(wakers: Vec<Waker>) {
    for w in wakers {
        w.wake();
    }
}

/// Every node's vpn ports, by (host, network key, port); the hash table's
/// service addresses, by (key, salt); and the swarm logs, by (swarm id,
/// label).
#[derive(Default)]
pub struct WorldState {
    pub ports: BTreeMap<(ID, Vec<u8>, u16), Port>,
    pub next_port: u16,
    pub addresses: BTreeMap<(Vec<u8>, Vec<u8>), HostAddr>,
    pub logs: BTreeMap<(SwarmId, String), Log>,
    /// The instant every node's clock reads as [`T0`].
    base: Option<Instant>,
}

#[derive(Clone, Default)]
pub struct World(pub Rc<RefCell<WorldState>>);

impl World {
    /// The world's clock: nodes started later read the same time.
    pub fn timer(&self) -> TokioTimer {
        let base = *self.0.borrow_mut().base.get_or_insert_with(Instant::now);
        TokioTimer { base }
    }

    fn log(&self, id: &SwarmId, label: &str, window: usize) -> Log {
        let mut w = self.0.borrow_mut();
        w.logs
            .entry((id.clone(), label.to_owned()))
            .or_insert_with(|| {
                Rc::new(RefCell::new(LogState {
                    window,
                    ..LogState::default()
                }))
            })
            .clone()
    }

    /// The directory address published under `key`.
    pub fn directory_address(&self, key: &[u8]) -> Option<HostAddr> {
        let k = (key.to_vec(), strims_directory::ADDRESS_SALT.to_vec());
        self.0.borrow().addresses.get(&k).copied()
    }

    /// Bytes written to the log of `id` under `label` in all epochs.
    pub fn written(&self, id: &SwarmId, label: &str) -> u64 {
        let w = self.0.borrow();
        w.logs
            .get(&(id.clone(), label.to_owned()))
            .map_or(0, |l| l.borrow().written)
    }
}

/// A port's deliveries; dropping it closes the port.
pub struct VpnReader {
    port: Port,
    world: World,
    key: (ID, Vec<u8>, u16),
}

impl Stream for VpnReader {
    type Item = Result<Delivery<u16>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut p = self.port.borrow_mut();
        if let Some(d) = p.queue.pop_front() {
            return Poll::Ready(Some(Ok(d)));
        }
        p.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for VpnReader {
    fn drop(&mut self) {
        let removed = {
            let mut w = self.world.0.borrow_mut();
            match w.ports.get(&self.key) {
                Some(p) if Rc::ptr_eq(p, &self.port) => w.ports.remove(&self.key),
                _ => None,
            }
        };
        drop(removed);
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

/// A swarm's writer.
pub struct FakeWriter {
    log: Log,
    closed: Rc<Cell<bool>>,
}

impl SwarmWrite for FakeWriter {
    type Error = Closed;

    async fn write(&mut self, data: &[u8]) -> Result<(), Closed> {
        if self.closed.get() {
            return Err(Closed);
        }
        let wakers = {
            let mut l = self.log.borrow_mut();
            l.data.extend(data);
            l.written += data.len() as u64;
            let excess = l.data.len().saturating_sub(l.window);
            l.data.drain(..excess);
            l.tail += excess as u64;
            l.wake()
        };
        wake_all(wakers);
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), Closed> {
        if self.closed.get() {
            return Err(Closed);
        }
        let wakers = {
            let mut l = self.log.borrow_mut();
            l.epoch += 1;
            l.data.clear();
            l.tail = 0;
            l.wake()
        };
        wake_all(wakers);
        Ok(())
    }
}

/// A swarm's reader.
pub struct FakeReader {
    log: Log,
    closed: Rc<Cell<bool>>,
    epoch: u64,
    pos: u64,
}

impl SwarmRead for FakeReader {
    type Error = Closed;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<Closed>> {
        poll_fn(|cx| {
            if self.closed.get() {
                return Poll::Ready(Err(ReadFailure::Closed));
            }
            let mut l = self.log.borrow_mut();
            if l.epoch != self.epoch || self.pos < l.tail {
                self.epoch = l.epoch;
                self.pos = l.tail;
                return Poll::Ready(Err(ReadFailure::Discontinuity));
            }
            let end = l.tail + l.data.len() as u64;
            if self.pos >= end {
                l.wakers.push(cx.waker().clone());
                return Poll::Pending;
            }
            let start = (self.pos - l.tail) as usize;
            let n = out.len().min((end - self.pos) as usize);
            for (o, b) in out.iter_mut().zip(l.data.range(start..start + n)) {
                *o = *b;
            }
            let at = self.pos;
            self.pos += n as u64;
            Poll::Ready(Ok((at, n)))
        })
        .await
    }
}

// --- fake nodes -------------------------------------------------------------

pub struct SwarmEntry {
    pub id: SwarmId,
    pub label: String,
    /// Whether this node created (writes) the swarm.
    pub created: bool,
    log: Log,
    closed: Rc<Cell<bool>>,
}

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
    pub swarms: BTreeMap<u32, SwarmEntry>,
    pub next_swarm: u32,
    /// Ports opened by number under another number.
    pub port_remap: BTreeMap<u16, u16>,
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

    pub fn net(&self, key: &[u8]) -> Option<u32> {
        self.state.borrow().networks.get(key).copied()
    }

    /// Opens `port` on the network, whatever `port_remap` says.
    pub fn open_port(&self, net: u32, port: u16) -> Result<(VpnReader, VpnWriter), Closed> {
        let network = self.network_key(net).ok_or(Closed)?;
        let key = (self.id, network.clone(), port);
        let state = Port::default();
        {
            let mut w = self.world.0.borrow_mut();
            if w.ports.contains_key(&key) {
                return Err(Closed);
            }
            w.ports.insert(key.clone(), state.clone());
        }
        let writer = VpnWriter {
            world: self.world.clone(),
            src: self.id,
            src_port: port,
            network,
            timer: self.timer.clone(),
        };
        let reader = VpnReader {
            port: state,
            world: self.world.clone(),
            key,
        };
        Ok((reader, writer))
    }

    /// The (created, added) swarms of the node with id `id`.
    pub fn swarms_of(&self, id: &SwarmId) -> (usize, usize) {
        let s = self.state.borrow();
        let of = s.swarms.values().filter(|e| e.id == *id);
        of.fold(
            (0, 0),
            |(c, a), e| {
                if e.created { (c + 1, a) } else { (c, a + 1) }
            },
        )
    }

    fn add_entry(
        &self,
        id: SwarmId,
        options: &SwarmOptions,
        created: bool,
    ) -> (u32, Log, Rc<Cell<bool>>) {
        let window = options.live_window * options.chunk_size;
        let log = self.world.log(&id, &options.label, window);
        let closed = Rc::new(Cell::new(false));
        let mut s = self.state.borrow_mut();
        s.next_swarm += 1;
        let h = s.next_swarm;
        s.swarms.insert(
            h,
            SwarmEntry {
                id,
                label: options.label.clone(),
                created,
                log: log.clone(),
                closed: closed.clone(),
            },
        );
        (h, log, closed)
    }
}

impl strims_network::Host for FakeHost {
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
        let mut s = self.state.borrow_mut();
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
        let port = {
            let mut w = self.world.0.borrow_mut();
            w.next_port += 1;
            1000 + w.next_port
        };
        let (reader, writer) = self.open_port(net, port)?;
        Ok((port, reader, writer))
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
        let mut w = self.world.0.borrow_mut();
        if w.addresses.get(&k).is_some_and(|a| a.host == self.id) {
            w.addresses.remove(&k);
        }
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

impl strims_transfer::Host for FakeHost {
    type Network = u32;
    type Swarm = u32;
    type Publication = ();
    type Search = futures_util::stream::Empty<ID>;
    type Error = Closed;

    fn host_id(&self) -> ID {
        self.id
    }

    async fn run_swarm(&self, _: ID, _: u32, _: u64, _: u64) -> Result<(), Closed> {
        Ok(())
    }

    async fn stop_swarm(&self, _: ID, _: u32) -> Result<(), Closed> {
        Ok(())
    }

    async fn index_publish(&self, _: u32, _: Vec<u8>, _: Vec<u8>) -> Result<(), Closed> {
        Ok(())
    }

    async fn index_unpublish(&self, _: ()) -> Result<(), Closed> {
        Ok(())
    }

    async fn index_search(
        &self,
        _: u32,
        _: Vec<u8>,
        _: Vec<u8>,
    ) -> Result<futures_util::stream::Empty<ID>, Closed> {
        Ok(futures_util::stream::empty())
    }

    async fn connect_peer(&self, _: u32, _: ID) -> Result<bool, Closed> {
        Ok(false)
    }
}

impl strims_transfer::SwarmHost for FakeHost {
    type Writer = FakeWriter;
    type Reader = FakeReader;

    async fn create_swarm(
        &self,
        options: WriterOptions,
    ) -> Result<(u32, SwarmId, FakeWriter), Closed> {
        let id = SwarmId::new(&options.key.public);
        let (h, log, closed) = self.add_entry(id.clone(), &options.swarm_options, true);
        let wakers = {
            let mut l = log.borrow_mut();
            l.epoch += 1;
            l.data.clear();
            l.tail = 0;
            l.wake()
        };
        wake_all(wakers);
        Ok((h, id, FakeWriter { log, closed }))
    }

    async fn add_swarm(&self, id: SwarmId, options: SwarmOptions) -> Result<u32, Closed> {
        Ok(self.add_entry(id, &options, false).0)
    }

    async fn remove_swarm(&self, swarm: u32) -> Result<(), Closed> {
        let e = self
            .state
            .borrow_mut()
            .swarms
            .remove(&swarm)
            .ok_or(Closed)?;
        e.closed.set(true);
        let wakers = e.log.borrow_mut().wake();
        wake_all(wakers);
        Ok(())
    }

    async fn open_reader(&self, swarm: u32) -> Result<FakeReader, Closed> {
        let s = self.state.borrow();
        let e = s.swarms.get(&swarm).ok_or(Closed)?;
        let l = e.log.borrow();
        Ok(FakeReader {
            log: e.log.clone(),
            closed: e.closed.clone(),
            epoch: l.epoch,
            pos: l.tail,
        })
    }
}

impl strims_directory::Host for FakeHost {
    async fn open_vpn_port_at(
        &self,
        net: u32,
        port: u16,
    ) -> Result<(VpnReader, VpnWriter), Closed> {
        let port = self
            .state
            .borrow()
            .port_remap
            .get(&port)
            .copied()
            .unwrap_or(port);
        self.open_port(net, port)
    }
}

pub type Directory = DirectoryControl<MemoryKv, FakeHost, TokioTimer>;

pub struct Node {
    pub id: ID,
    pub profile: Profile,
    pub host: FakeHost,
    pub store: ReplicatedStore<MemoryKv>,
    pub network: NetworkControl<MemoryKv, FakeHost, TokioTimer>,
    pub transfers: TransferControl<FakeHost, TokioTimer>,
    pub directory: Directory,
    pub timer: TokioTimer,
    /// The network, transfer and directory controls' tasks, in that order.
    pub tasks: Vec<JoinHandle<()>>,
}

impl Node {
    pub fn profile_key(&self) -> &Key {
        self.profile.key.as_ref().unwrap()
    }

    pub async fn write_network(&self, n: Network) {
        let mut tx = self.store.write(T0).await.unwrap();
        Networks::upsert(&mut tx, n).await.unwrap();
        tx.commit().await.unwrap();
    }

    pub async fn read_network(&self, id: u64) -> Network {
        let mut tx = self.store.read().await.unwrap();
        Networks::get(&mut tx, id).await.unwrap().unwrap()
    }

    /// Changes the profile's record of network `id`.
    pub async fn change_network(&self, id: u64, f: impl FnOnce(&mut Network)) {
        let mut n = self.read_network(id).await;
        f(&mut n);
        self.write_network(n).await;
    }

    /// Stops the node's controls, as if its process ended.
    pub fn stop(&self) {
        for t in &self.tasks {
            t.abort();
        }
    }

    /// Stops only the directory control.
    pub fn stop_directory(&self) {
        if let Some(t) = self.tasks.last() {
            t.abort();
        }
    }

    pub fn net(&self, key: &[u8]) -> u32 {
        self.host.net(key).unwrap()
    }

    pub fn serving(&self, key: &[u8]) -> bool {
        self.directory
            .networks()
            .iter()
            .any(|n| n.network_key == key && n.serving)
    }
}

/// A node of a new profile, with its own store.
pub async fn node_in(world: &World, n: u8) -> Node {
    let mut rng = det_rng(u64::from(n));
    let key = strims_crypto::generate_key(&mut rng);
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
    start_node(world, n, profile, store)
}

/// A node with id `n` of `other`'s profile, sharing its store, as another
/// process of the profile.
pub fn node_sharing(world: &World, n: u8, other: &Node) -> Node {
    start_node(world, n, other.profile.clone(), other.store.clone())
}

fn start_node(world: &World, n: u8, profile: Profile, store: ReplicatedStore<MemoryKv>) -> Node {
    let id = ID::from_bytes(&[n; 32]);
    let timer = world.timer();
    let host = FakeHost {
        id,
        state: Rc::default(),
        world: world.clone(),
        timer: timer.clone(),
    };
    let network = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        timer.clone(),
        det_rng(u64::from(n) + 2),
        strims_network::Config::default(),
    );
    let transfers = TransferControl::new(
        host.clone(),
        timer.clone(),
        strims_transfer::Config {
            discovery: false,
            ..strims_transfer::Config::default()
        },
    );
    let directory = DirectoryControl::new(
        host.clone(),
        store.clone(),
        network.clone(),
        transfers.clone(),
        timer.clone(),
        det_rng(u64::from(n) + 3),
        strims_directory::Config::default(),
    );
    let c = network.clone();
    let t = transfers.clone();
    let d = directory.clone();
    let tasks = vec![
        spawn_local(async move { c.run().await }),
        spawn_local(async move { t.run().await }),
        spawn_local(async move { d.run().await }),
    ];
    Node {
        id,
        profile,
        host,
        store,
        network,
        transfers,
        directory,
        timer,
        tasks,
    }
}

/// A UI's connection to the frontend services on `router`, over in-memory
/// pipes: the UI's client.
pub fn frontend_client(router: Router) -> Client {
    let (up, down) = (Pipe::default(), Pipe::default());
    let (_, server) = connect(
        strims_rpc::Config::frontend(),
        Reader(up.clone()),
        Writer(down.clone()),
        Some(router),
    );
    let (client, conn) = connect(
        strims_rpc::Config::frontend(),
        Reader(down),
        Writer(up),
        None,
    );
    spawn_local(async move {
        let _ = server.await;
    });
    spawn_local(async move {
        let _ = conn.await;
    });
    client
}

/// Links `a` and `b` as the node would, for network membership: an RPC
/// channel pair and a broker channel each way. Returns the pipes, to cut
/// the link.
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
    a.network
        .add_peer(b.id, &b.profile_key().public, &a_router, a_client);
    b.network
        .add_peer(a.id, &a.profile_key().public, &b_router, b_client);
    spawn_local(async move {
        let _ = a_conn.await;
    });
    spawn_local(async move {
        let _ = b_conn.await;
    });
    pipes
}

/// A network `owner` creates as network `id`, with each of `members`
/// invited to it as their network `id` and linked to the owner, once their
/// certificates are renewed and their member ports are open: the network's
/// root key, and the links' pipes.
pub async fn owned_network(owner: &Node, members: &[&Node], id: u64) -> (Key, Vec<Pipe>) {
    owned_network_with(owner, members, id, |_| {}).await
}

/// [`owned_network`], with the owner's record changed by `f` first.
pub async fn owned_network_with(
    owner: &Node,
    members: &[&Node],
    id: u64,
    f: impl FnOnce(&mut Network),
) -> (Key, Vec<Pipe>) {
    let mut rng = det_rng(id);
    let mut owned = new_network(id, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
    let owner_cert = owned.certificate.clone().unwrap();
    let key = owned.server_config.as_ref().unwrap().key.clone().unwrap();
    f(&mut owned);
    owner.write_network(owned).await;
    let mut pipes = Vec::new();
    for m in members {
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(id, &invitation, &m.profile, "", T0, &mut rng).unwrap();
        m.write_network(joined).await;
        pipes.extend(link(owner, m));
    }
    settle().await;
    for m in members {
        let n = m.read_network(id).await;
        assert!(strims_crypto::membership_trusted(
            n.certificate.as_ref().unwrap(),
            T0
        ));
    }
    (key, pipes)
}

/// The record of network `id` for a member whose membership the network's
/// root signed, so that it is trusted at once.
pub fn signed_member(root: &Key, member: &Node, id: u64) -> Network {
    let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
    let csr = new_certificate_request(member.profile_key(), usage, &member.profile.name).unwrap();
    let mut rng = det_rng(id);
    let cert =
        sign_certificate_request_with_network(&csr, root, "net", DEFAULT_CERT_TTL, T0, &mut rng)
            .unwrap();
    Network {
        id,
        certificate: Some(cert),
        ..Default::default()
    }
}

pub async fn sleep_secs(secs: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
}

pub async fn settle() {
    sleep_secs(30).await;
}

/// Waits up to `secs` seconds, polling every 100 ms, for `cond`.
pub async fn wait_for(secs: u64, what: &str, mut cond: impl FnMut() -> bool) {
    let steps = secs * 10;
    for _ in 0..steps {
        if cond() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(cond(), "timed out waiting for {what}");
}

/// The events a subscription holds now.
pub fn drain(events: &mut Subscription<DirectoryEvent>) -> Vec<DirectoryEvent> {
    let mut out = Vec::new();
    while let Some(Some(r)) = futures_util::FutureExt::now_or_never(events.next()) {
        match r {
            Recv::Event(e) => out.push((*e).clone()),
            Recv::Lagged => panic!("lagged"),
        }
    }
    out
}

pub fn run(f: impl std::future::Future<Output = ()> + 'static) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f));
}
