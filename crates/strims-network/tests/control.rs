// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network controls on fake nodes, linked over in-memory pipes: each
//! peer's RPC channel pair and its broker channel. Each fake node keeps
//! the networks it joined and the peers it bound to them. The networks
//! come from real profile stores. Time is tokio's paused clock.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, generate_key, new_certificate_request, sign_certificate_request_with_network,
};
use strims_dao::schema::network::{NetworkPeers, Networks, network_scoped_key};
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReplicatedStore, Singleton, Table};
use strims_kademlia::ID;
use strims_kv::MemoryKv;
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0, new_network};
use strims_network::{Config, Host, NetworkControl, NetworkEvent};
use strims_proto::network::v1::Network;
use strims_proto::profile::v1::Profile;
use strims_proto::types::{Certificate, Key, KeyUsage};
use strims_rpc::{Router, Timer, peer_connection};
use strims_vpn::{Delivery, HostAddr, M_ENCRYPT, NetworkId};
use tokio::task::{LocalSet, spawn_local};
use tokio::time::{Instant, Sleep};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);

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

impl std::fmt::Display for Closed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("closed")
    }
}

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

// --- a fake vpn -------------------------------------------------------------

#[derive(Default)]
struct PortState {
    queue: VecDeque<Delivery<u16>>,
    reader: Option<Waker>,
}

type Port = Rc<RefCell<PortState>>;

/// Every node's vpn ports, by (host, network key, port), and the hash
/// table's service addresses, by (key, salt).
#[derive(Default)]
struct WorldState {
    ports: BTreeMap<(ID, Vec<u8>, u16), Port>,
    next_port: u16,
    addresses: BTreeMap<(Vec<u8>, Vec<u8>), HostAddr>,
}

#[derive(Clone, Default)]
struct World(Rc<RefCell<WorldState>>);

struct VpnReader(Port);

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

struct VpnWriter {
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
struct NodeState {
    /// Network key -> handle.
    networks: BTreeMap<Vec<u8>, u32>,
    next_net: u32,
    next_port: u16,
    /// (peer, network) -> (local port, peer port).
    bound: BTreeMap<(ID, u32), (u16, u16)>,
    /// Each peer's broker channel, until taken.
    brokers: BTreeMap<ID, (Reader, Writer)>,
}

#[derive(Clone)]
struct FakeHost {
    id: ID,
    state: Rc<RefCell<NodeState>>,
    world: World,
    timer: TokioTimer,
}

impl FakeHost {
    fn network_key(&self, net: u32) -> Option<Vec<u8>> {
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

struct Node {
    id: ID,
    profile: Profile,
    host: FakeHost,
    store: ReplicatedStore<MemoryKv>,
    control: NetworkControl<MemoryKv, FakeHost, TokioTimer>,
}

impl Node {
    fn profile_key(&self) -> &Key {
        self.profile.key.as_ref().unwrap()
    }

    /// The (local, peer) ports `peer` is bound to the network on.
    fn bound(&self, peer: &Node, network: &Key) -> Option<(u16, u16)> {
        let s = self.host.state.borrow();
        let net = *s.networks.get(&network.public)?;
        s.bound.get(&(peer.id, net)).copied()
    }

    async fn write_network(&self, n: Network) {
        let mut tx = self.store.write(T0).await.unwrap();
        Networks::upsert(&mut tx, n).await.unwrap();
        tx.commit().await.unwrap();
    }

    async fn delete_network(&self, id: u64) {
        let mut tx = self.store.write(T0).await.unwrap();
        Networks::delete(&mut tx, id).await.unwrap();
        tx.commit().await.unwrap();
    }
}

async fn node(n: u8) -> Node {
    node_in(&World::default(), n).await
}

async fn node_in(world: &World, n: u8) -> Node {
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
fn membership(network: &Key, profile: &Key, n: u64) -> Certificate {
    let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
    let csr = new_certificate_request(profile, usage, "member").unwrap();
    let mut rng = det_rng(n);
    sign_certificate_request_with_network(&csr, network, "net", DEFAULT_CERT_TTL, T0, &mut rng)
        .unwrap()
}

/// The profile's record of a network it is a member of.
fn joined(id: u64, network: &Key, node: &Node) -> Network {
    Network {
        id,
        certificate: Some(membership(network, node.profile_key(), id)),
        ..Default::default()
    }
}

/// Links `a` and `b` as the node would: an RPC channel pair and a broker
/// channel each way. Returns the pipes, to cut the link.
fn link(a: &Node, b: &Node) -> Vec<Pipe> {
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

async fn settle() {
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
}

/// Bound crosswise on both nodes.
fn assert_bound(a: &Node, b: &Node, network: &Key) {
    let ab = a.bound(b, network);
    let ba = b.bound(a, network);
    let (Some((al, ap)), Some((bl, bp))) = (ab, ba) else {
        panic!("not bound both ways: {ab:?} {ba:?}");
    };
    assert_eq!((al, ap), (bp, bl));
}

fn run(f: impl std::future::Future<Output = ()> + 'static) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f));
}

/// Two nodes bind the networks their profiles share, and only those, and
/// report the bound peers.
#[test]
fn nodes_bind_the_networks_their_profiles_share() {
    run(async {
        let a = node(1).await;
        let b = node(2).await;
        let mut rng = det_rng(10);
        let nets: Vec<Key> = (0..4).map(|_| generate_key(&mut rng)).collect();
        for (i, net) in nets[..3].iter().enumerate() {
            a.write_network(joined(10 + i as u64, net, &a)).await;
        }
        for (i, net) in nets[1..].iter().enumerate() {
            b.write_network(joined(20 + i as u64, net, &b)).await;
        }
        let mut events = a.control.subscribe();
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &nets[1]);
        assert_bound(&a, &b, &nets[2]);
        assert_eq!(a.bound(&b, &nets[0]), None);
        assert_eq!(b.bound(&a, &nets[3]), None);

        let mut counts = BTreeMap::new();
        while let Some(Some(strims_dao::Recv::Event(e))) = events.next().now_or_never() {
            if let NetworkEvent::PeerCount { network_id, count } = *e {
                counts.insert(network_id, count);
            }
        }
        assert_eq!(counts, BTreeMap::from([(11, 1), (12, 1)]));
        let infos = a.control.networks();
        assert_eq!(infos.len(), 3);
        assert_eq!(
            infos.iter().map(|i| i.peer_count).sum::<usize>(),
            2,
            "{infos:?}"
        );
        let m = a.control.metrics();
        assert_eq!((m.runs_completed, m.runs_failed), (1, 0), "{m:?}");
    });
}

/// A network joined after the peers linked binds too; one deleted from
/// the store is closed and unbound on both sides.
#[test]
fn networks_follow_the_store() {
    run(async {
        let a = node(3).await;
        let b = node(4).await;
        let mut rng = det_rng(11);
        let nets: Vec<Key> = (0..2).map(|_| generate_key(&mut rng)).collect();
        a.write_network(joined(30, &nets[0], &a)).await;
        b.write_network(joined(40, &nets[0], &b)).await;
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &nets[0]);

        a.write_network(joined(31, &nets[1], &a)).await;
        b.write_network(joined(41, &nets[1], &b)).await;
        settle().await;
        assert_bound(&a, &b, &nets[1]);

        a.delete_network(30).await;
        settle().await;
        assert_eq!(a.bound(&b, &nets[0]), None);
        assert_eq!(b.bound(&a, &nets[0]), None);
        assert_bound(&a, &b, &nets[1]);
        assert!(!a.host.state.borrow().networks.contains_key(&nets[0].public));
    });
}

/// A peer that relinks negotiates again on the new link.
#[test]
fn a_relinked_peer_binds_again() {
    run(async {
        let a = node(5).await;
        let b = node(6).await;
        let net = generate_key(&mut det_rng(12));
        a.write_network(joined(50, &net, &a)).await;
        b.write_network(joined(60, &net, &b)).await;
        let pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &net);

        for p in pipes {
            p.close();
        }
        a.control.remove_peer(b.id);
        b.control.remove_peer(a.id);
        a.host.state.borrow_mut().bound.clear();
        b.host.state.borrow_mut().bound.clear();
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &net);
    });
}

async fn read_network(node: &Node, id: u64) -> Network {
    let mut tx = node.store.read().await.unwrap();
    Networks::get(&mut tx, id).await.unwrap().unwrap()
}

/// A profile invited to its network renews its provisional certificate
/// through the owner, whose CA signs it: then both bind, and the CA
/// records the new peer under its inviter.
#[test]
fn an_invitee_renews_through_its_inviter_and_binds() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 7).await;
        let invitee = node_in(&world, 8).await;
        let mut rng = det_rng(70);
        let owned = new_network(70, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
        let owner_cert = owned.certificate.clone().unwrap();
        let net_key = owned.server_config.as_ref().unwrap().key.clone().unwrap();
        owner.write_network(owned).await;
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(80, &invitation, &invitee.profile, "", T0, &mut rng)
                .unwrap();
        invitee.write_network(joined).await;

        let _pipes = link(&owner, &invitee);
        settle().await;
        assert_bound(&owner, &invitee, &net_key);
        let renewed = read_network(&invitee, 80).await;
        let cert = renewed.certificate.unwrap();
        assert!(strims_crypto::membership_trusted(&cert, T0));
        assert_eq!(renewed.certificate_renewal_error, 0);

        let mut tx = owner.store.read().await.unwrap();
        let scoped = |k: &[u8]| network_scoped_key(70, k);
        let owner_peer = NetworkPeers::index_get(
            &mut tx,
            &NetworkPeers::BY_PUBLIC_KEY,
            &scoped(&owner.profile_key().public),
        )
        .await
        .unwrap()
        .unwrap();
        let peer = NetworkPeers::index_get(
            &mut tx,
            &NetworkPeers::BY_PUBLIC_KEY,
            &scoped(&invitee.profile_key().public),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(peer.inviter_peer_id, owner_peer.id);
        assert_eq!(owner_peer.invite_quota, 2);
    });
}

/// A trusted member whose alias changes renews at the network's CA over
/// vpn, under the new alias.
#[test]
fn a_member_renews_at_the_ca_over_vpn() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 9).await;
        let member = node_in(&world, 10).await;
        let mut rng = det_rng(90);
        let owned = new_network(90, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
        let owner_cert = owned.certificate.clone().unwrap();
        owner.write_network(owned).await;
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(100, &invitation, &member.profile, "", T0, &mut rng)
                .unwrap();
        member.write_network(joined).await;
        let _pipes = link(&owner, &member);
        settle().await;
        let n = read_network(&member, 100).await;
        assert!(strims_crypto::membership_trusted(
            n.certificate.as_ref().unwrap(),
            T0
        ));

        member
            .write_network(Network {
                alias: "bob".into(),
                ..n
            })
            .await;
        settle().await;
        let n = read_network(&member, 100).await;
        assert_eq!(n.certificate.unwrap().subject, "bob");
        assert_eq!(n.certificate_renewal_error, 0);
        let m = member.control.metrics();
        assert_eq!(m.runs_failed, 0, "{m:?}");
    });
}
