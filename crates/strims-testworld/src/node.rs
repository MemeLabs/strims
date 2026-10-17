// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Fake nodes, their links and their networks.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use strims_base::rng::{DetRng, det_rng, fork};
use strims_crypto::{
    DEFAULT_CERT_TTL, new_certificate_request, sign_certificate_request_with_network,
};
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReplicatedStore, Singleton, Table};
use strims_directory::DirectoryControl;
use strims_kademlia::ID;
use strims_kv::MemoryKv;
use strims_network::NetworkControl;
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0, new_network};
use strims_proto::network::v1::Network;
use strims_proto::profile::v1::Profile;
use strims_proto::types::{Key, KeyUsage};
use strims_rpc::{Client, Router, connect, peer_connection};
use strims_transfer::TransferControl;
use tokio::task::{AbortHandle, JoinHandle, spawn_local};

use crate::exec::{T0, TokioTimer, settle};
use crate::host::FakeHost;
use crate::pipe::{Pipe, Reader, Writer};
use crate::world::World;

pub type Directory = DirectoryControl<MemoryKv, FakeHost, TokioTimer>;

/// A node: one process of a profile, with its network, transfer and
/// directory controls running.
pub struct Node {
    pub id: ID,
    pub profile: Profile,
    pub host: FakeHost,
    pub store: ReplicatedStore<MemoryKv>,
    pub network: NetworkControl<MemoryKv, FakeHost, TokioTimer>,
    pub transfers: TransferControl<FakeHost, TokioTimer>,
    pub directory: Directory,
    pub timer: TokioTimer,
    rng: RefCell<DetRng>,
    tasks: RefCell<Vec<AbortHandle>>,
    directory_task: AbortHandle,
}

impl Node {
    pub fn profile_key(&self) -> &Key {
        self.profile.key.as_ref().unwrap()
    }

    /// A new rng, forked from the node's.
    pub fn rng(&self) -> DetRng {
        fork(&mut self.rng.borrow_mut())
    }

    /// Spawns `f` on the test's local task set as one of the node's tasks,
    /// which [`Node::stop`] aborts.
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let task = spawn_local(f);
        self.tasks.borrow_mut().push(task.abort_handle());
        task
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

    /// Stops the node's tasks, as if its process ended.
    pub fn stop(&self) {
        for t in self.tasks.borrow().iter() {
            t.abort();
        }
    }

    /// Stops only the directory control.
    pub fn stop_directory(&self) {
        self.directory_task.abort();
    }

    pub fn net(&self, key: &[u8]) -> u32 {
        self.host.net(key).unwrap()
    }

    /// Whether the node serves the directory of the network with key `key`.
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
    let network_task = spawn_local(async move { c.run().await }).abort_handle();
    let transfer_task = spawn_local(async move { t.run().await }).abort_handle();
    let directory_task = spawn_local(async move { d.run().await }).abort_handle();
    Node {
        id,
        profile,
        host,
        store,
        network,
        transfers,
        directory,
        timer,
        rng: RefCell::new(det_rng(u64::from(n) + 4)),
        tasks: RefCell::new(vec![network_task, transfer_task, directory_task.clone()]),
        directory_task,
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
