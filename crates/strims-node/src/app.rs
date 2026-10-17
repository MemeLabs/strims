// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The app layer on a node: the node's clock as the app's [`Timer`], the
//! node as replication's [`Host`], and every peer's RPC link served with
//! the app's peer services.
//!
//! App-layer futures aren't `Send`: [`PeerServices`] spawns its
//! connections as local tasks, so it runs inside a tokio `LocalSet`.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use strims_base::time::{Duration, Time};
use strims_kv::Kv;
use strims_proto::types::Key;
use strims_replication::{Host, ReplicationControl};
use strims_rpc::{Config, Router, Timer, peer_connection};
use strims_vpn::{HashTableGetOptions, HashTablePublisherId, NetworkId};
use tokio::time::Sleep;

use crate::{Clock, HostId, Node, NodeError, NodeEvent};

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

/// A node as replication's [`Host`], on the networks the driver adds.
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

/// Serves the app's peer services on every peer's RPC link.
pub struct PeerServices<K: Kv, C: Clock> {
    node: Node,
    replication: ReplicationControl<K, NodeHost, NodeTimer<C>>,
}

impl<K: Kv, C: Clock> PeerServices<K, C> {
    /// Peer services for `node`, introducing peers to `replication`.
    pub fn new(node: Node, replication: ReplicationControl<K, NodeHost, NodeTimer<C>>) -> Self {
        PeerServices { node, replication }
    }

    /// Takes a node event: a new peer's RPC channels become a connection
    /// serving the app's peer services, and replication learns of it.
    pub async fn handle_event(&self, e: &NodeEvent) {
        match e {
            NodeEvent::PeerAdded(p) => {
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
                self.replication
                    .add_peer(p.host_id, &p.profile_key, &router, client);
                let (replication, host) = (self.replication.clone(), p.host_id);
                tokio::task::spawn_local(async move {
                    let res = conn.await;
                    tracing::debug!(peer = %host, ?res, "peer rpc ended");
                    replication.remove_peer(host);
                });
            }
            NodeEvent::PeerRemoved { host_id } => self.replication.remove_peer(*host_id),
            _ => {}
        }
    }
}
