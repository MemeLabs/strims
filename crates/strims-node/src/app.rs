// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The app layer on a node: the node's clock as the app's [`Timer`], the
//! node as the [`Host`] of replication and of transfers, and every peer's
//! RPC link served with the app's [`PeerHandler`]s.
//!
//! App-layer futures aren't `Send`: [`PeerServices`] spawns its
//! connections as local tasks, so it runs inside a tokio `LocalSet`.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_util::Stream;
use strims_base::time::{Duration, Time};
use strims_kv::Kv;
use strims_proto::types::Key;
use strims_replication::{Host, ReplicationControl};
use strims_rpc::{Client, Config, Router, Timer, peer_connection};
use strims_transfer::TransferControl;
use strims_vpn::{HashTableGetOptions, HashTablePublisherId, NetworkId, PeerIndexPublisherId};
use tokio::time::Sleep;

use crate::{Clock, HostId, Node, NodeError, NodeEvent, PeerIndexSearch, PeerInfo, SwarmKey};

/// A node's clock as the app layer's [`Timer`].
#[derive(Clone)]
pub struct NodeTimer<C: Clock>(pub C);

impl<C: Clock> Timer for NodeTimer<C> {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        self.0.now()
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        tokio::time::sleep_until(self.0.deadline(deadline))
    }
}

/// How long a discovery lookup waits for answers.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);

/// A node as replication's [`Host`], on the networks the driver adds, and
/// as the transfers' [`strims_transfer::Host`].
#[derive(Clone)]
pub struct NodeHost {
    node: Node,
    networks: Rc<RefCell<BTreeSet<NetworkId>>>,
}

impl NodeHost {
    /// The host for `node`, on no networks yet.
    pub fn new(node: Node) -> NodeHost {
        NodeHost {
            node,
            networks: Rc::default(),
        }
    }

    /// Publishes and looks up devices on `net` too.
    pub fn add_network(&self, net: NetworkId) {
        self.networks.borrow_mut().insert(net);
    }

    /// Stops using `net`.
    pub fn remove_network(&self, net: NetworkId) {
        self.networks.borrow_mut().remove(&net);
    }
}

impl Host for NodeHost {
    type Network = NetworkId;
    type Publication = HashTablePublisherId;
    type Error = NodeError;

    fn host_id(&self) -> HostId {
        self.node.host_id()
    }

    async fn networks(&self) -> Vec<NetworkId> {
        self.networks.borrow().iter().copied().collect()
    }

    async fn publish(
        &self,
        net: NetworkId,
        key: &Key,
        salt: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<HashTablePublisherId, NodeError> {
        self.node
            .hash_table_set(net, key.clone(), salt, value)
            .await
    }

    async fn unpublish(&self, publication: HashTablePublisherId) -> Result<(), NodeError> {
        self.node.hash_table_close(publication).await
    }

    async fn lookup(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
        max: usize,
    ) -> Result<Vec<Vec<u8>>, NodeError> {
        let options = HashTableGetOptions {
            disable_cache: true,
            timeout: LOOKUP_TIMEOUT,
        };
        let mut get = self.node.hash_table_get(net, key, salt, options).await?;
        let mut out = Vec::new();
        while out.len() < max {
            match get.next().await {
                Some(v) if !out.contains(&v) => out.push(v),
                Some(_) => {}
                None => break,
            }
        }
        Ok(out)
    }

    async fn connect_peer(&self, net: NetworkId, host: HostId) -> Result<bool, NodeError> {
        self.node.connect_peer(net, host).await
    }
}

impl strims_transfer::Host for NodeHost {
    type Network = NetworkId;
    type Swarm = SwarmKey;
    type Publication = PeerIndexPublisherId;
    type Search = IndexHosts;
    type Error = NodeError;

    fn host_id(&self) -> HostId {
        self.node.host_id()
    }

    async fn run_swarm(
        &self,
        host: HostId,
        swarm: SwarmKey,
        channel: u64,
        peer_channel: u64,
    ) -> Result<(), NodeError> {
        self.node
            .run_swarm(host, swarm, channel, peer_channel)
            .await
    }

    async fn stop_swarm(&self, host: HostId, swarm: SwarmKey) -> Result<(), NodeError> {
        match self.node.stop_swarm(host, swarm).await {
            // its channels went with it
            Err(NodeError::UnknownPeer) => Ok(()),
            r => r,
        }
    }

    async fn index_publish(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<PeerIndexPublisherId, NodeError> {
        self.node.peer_index_publish(net, key, salt, 0).await
    }

    async fn index_unpublish(&self, publication: PeerIndexPublisherId) -> Result<(), NodeError> {
        self.node.peer_index_unpublish(publication).await
    }

    async fn index_search(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<IndexHosts, NodeError> {
        let search = self.node.peer_index_search(net, key, salt).await?;
        Ok(IndexHosts(search))
    }

    async fn connect_peer(&self, net: NetworkId, host: HostId) -> Result<bool, NodeError> {
        self.node.connect_peer(net, host).await
    }
}

/// The host ids a peer index search finds.
#[derive(Debug)]
pub struct IndexHosts(PeerIndexSearch);

impl Stream for IndexHosts {
    type Item = HostId;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<HostId>> {
        self.0.poll_next(cx).map(|h| h.map(|h| h.host_id))
    }
}

/// An app service on every peer's RPC link.
pub trait PeerHandler {
    /// A peer linked: serve it on `router`, and call it through `client`.
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client);

    /// The peer's link went.
    fn remove_peer(&self, host: HostId);

    /// Takes every node event, after [`PeerServices`] has handled it.
    fn handle_event(&self, _: &NodeEvent) {}
}

/// Replication serves the peers that hold the profile's key.
impl<K: Kv, T: Timer> PeerHandler for ReplicationControl<K, NodeHost, T> {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        ReplicationControl::add_peer(self, peer.host_id, &peer.profile_key, router, client);
    }

    fn remove_peer(&self, host: HostId) {
        ReplicationControl::remove_peer(self, host);
    }
}

/// Transfers learn the peers' networks and ppspp failures from the
/// node's events.
impl<T: Timer> PeerHandler for TransferControl<NodeHost, T> {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        TransferControl::add_peer(self, peer.host_id, router, client);
    }

    fn remove_peer(&self, host: HostId) {
        TransferControl::remove_peer(self, host);
    }

    fn handle_event(&self, e: &NodeEvent) {
        match *e {
            NodeEvent::NetworkPeerAdded { network, host_id } => {
                self.add_network_peer(network, host_id);
            }
            NodeEvent::NetworkPeerRemoved { network, host_id } => {
                self.remove_network_peer(network, host_id);
            }
            NodeEvent::TransferError { host_id, .. } => self.transfer_failed(host_id),
            _ => {}
        }
    }
}

/// Serves the app's peer handlers on every peer's RPC link.
pub struct PeerServices {
    node: Node,
    handlers: Vec<Rc<dyn PeerHandler>>,
    /// Each linked peer's link, so a connection that ends after its peer
    /// linked again leaves the new link alone. Bound: the node's peers.
    links: Rc<RefCell<BTreeMap<HostId, u64>>>,
    next_link: Cell<u64>,
}

impl PeerServices {
    /// Peer services for `node`, with no handlers yet.
    pub fn new(node: Node) -> Self {
        PeerServices {
            node,
            handlers: Vec::new(),
            links: Rc::default(),
            next_link: Cell::new(0),
        }
    }

    /// Introduces every peer to `handler` too.
    pub fn with(mut self, handler: Rc<dyn PeerHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Takes a node event: a new peer's RPC channels become a connection
    /// serving the handlers, which learn of the peer, and of its going
    /// when the peer or its connection ends. Then every handler sees the
    /// event.
    pub async fn handle_event(&self, e: &NodeEvent) {
        match e {
            NodeEvent::PeerAdded(p) => self.add_peer(p).await,
            NodeEvent::PeerRemoved { host_id } => self.remove_peer(*host_id),
            _ => {}
        }
        for h in &self.handlers {
            h.handle_event(e);
        }
    }

    fn remove_peer(&self, host: HostId) {
        let linked = self.links.borrow_mut().remove(&host).is_some();
        if linked {
            for h in &self.handlers {
                h.remove_peer(host);
            }
        }
    }

    async fn add_peer(&self, p: &PeerInfo) {
        let rpc = match self.node.take_peer_rpc(p.host_id).await {
            Ok(rpc) => rpc,
            Err(e) => {
                tracing::debug!(peer = %p.host_id, error = %e, "taking the peer's rpc channels failed");
                return;
            }
        };
        let router = Router::new();
        let (client, conn) = peer_connection(
            Config::peer(),
            (rpc.client.reader, rpc.client.writer),
            (rpc.server.reader, rpc.server.writer),
            router.clone(),
        );
        let link = self.next_link.get() + 1;
        self.next_link.set(link);
        self.links.borrow_mut().insert(p.host_id, link);
        for h in &self.handlers {
            h.add_peer(p, &router, client.clone());
        }
        let (handlers, links, host) = (self.handlers.clone(), self.links.clone(), p.host_id);
        tokio::task::spawn_local(async move {
            let res = conn.await;
            tracing::debug!(peer = %host, ?res, "peer rpc ended");
            let current = links.borrow().get(&host) == Some(&link);
            if current {
                links.borrow_mut().remove(&host);
                for h in &handlers {
                    h.remove_peer(host);
                }
            }
        });
    }
}
