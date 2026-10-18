// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Every peer's RPC link, served with the app's peer handlers.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use strims_kv::Kv;
use strims_network::NetworkControl;
use strims_network::bootstrap::BootstrapControl;
use strims_replication::ReplicationControl;
use strims_rpc::{Client, Config, Router, Timer, peer_connection};
use strims_transfer::TransferControl;

use super::{NodeHost, Spawn};
use crate::{HostId, Node, NodeEvent, PeerInfo};

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

/// Network membership negotiates with every peer over its RPC link and
/// broker channel.
impl<K: Kv, T: Timer> PeerHandler for NetworkControl<K, NodeHost, T> {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        NetworkControl::add_peer(self, peer.host_id, &peer.profile_key, router, client);
    }

    fn remove_peer(&self, host: HostId) {
        NetworkControl::remove_peer(self, host);
    }
}

/// Bootstrap serves every peer its peer service and asks whether it
/// accepts published networks.
impl<K: Kv, T: Timer> PeerHandler for BootstrapControl<K, NodeHost, T> {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        BootstrapControl::add_peer(self, peer.host_id, &peer.profile_key, router, client);
    }

    fn remove_peer(&self, host: HostId) {
        BootstrapControl::remove_peer(self, host);
    }
}

/// Serves the app's peer handlers on every peer's RPC link. Each link's
/// connection runs as a local task on `S`, detached: it ends with the
/// peer's RPC channels.
pub struct PeerServices<S> {
    node: Node,
    spawn: S,
    handlers: Vec<Rc<dyn PeerHandler>>,
    /// Each linked peer's link, so a connection that ends after its peer
    /// linked again leaves the new link alone. Bound: the node's peers.
    links: Rc<RefCell<BTreeMap<HostId, u64>>>,
    next_link: Cell<u64>,
}

impl<S: Spawn> PeerServices<S> {
    /// Peer services for `node`, with no handlers yet, spawning on
    /// `S::default()`.
    pub fn new(node: Node) -> Self {
        PeerServices::with_spawner(node, S::default())
    }

    /// Peer services for `node`, with no handlers yet, spawning on `spawn`.
    pub fn with_spawner(node: Node, spawn: S) -> Self {
        PeerServices {
            node,
            spawn,
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
        self.spawn.spawn_local(Box::pin(async move {
            let res = conn.await;
            tracing::debug!(peer = %host, ?res, "peer rpc ended");
            let current = links.borrow().get(&host) == Some(&link);
            if current {
                links.borrow_mut().remove(&host);
                for h in &handlers {
                    h.remove_peer(host);
                }
            }
        }));
    }
}
