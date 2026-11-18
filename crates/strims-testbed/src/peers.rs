// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A testbed node's peers: every peer's RPC link through node-core's
//! [`PeerServices`], with strims-transfer's [`TransferControl`] on it,
//! which runs the testbed's swarm through the transfers' announce and
//! answer as a deployed node does, but without the network membership run
//! that precedes them on a deployed node (strims-network's, which binds a
//! peer to a network before its transfers announce to it). The control
//! runs the swarm with a peer once the two sides' announce and answer
//! have bound its channels ([`strims_transfer`]'s crate docs): each side
//! announces the swarm as its link to the peer comes up, and runs it on
//! the peer's announce or on the answer to its own, whichever reaches it
//! first. So the side whose link comes up last runs it as it takes the
//! other's announce, and the other a trip after that side's link came up.
//!
//! The swarm is published on one network, `()`, which every peer joins as
//! it links: a node's peers are its run's nodes and its relay, which with
//! a relay all join the testbed's network ([`crate::topology`]). The relay
//! serves the transfers too ([`Peers`]) and runs no swarm, so it refuses
//! each announce. Discovery is off: the topology's links stand in for it.

use std::rc::Rc;

use futures_util::stream::Empty;
use strims_node_core::app::{PeerHandler, PeerServices, Spawn};
use strims_node_core::{HostId, Node, NodeError, NodeEvent, PeerInfo, SwarmId, SwarmKey};
use strims_rpc::{Client, Router, Timer};
use strims_transfer::{Config, Host, TransferControl};
use tokio::sync::mpsc;

/// A node's peers: the node's events that [`serves`] takes, the peer
/// services they drive, and the transfers on those services.
pub struct Peers<T: Timer, S: Spawn> {
    events: mpsc::UnboundedReceiver<NodeEvent>,
    services: PeerServices<S>,
    transfers: TransferControl<Swarms, T>,
}

impl<T: Timer, S: Spawn> Peers<T, S> {
    /// `node`'s peers, taken from `events`: the transfers are timed on
    /// `timer`, and each peer's RPC connection is spawned on `spawn`.
    pub fn new(
        node: &Node,
        events: mpsc::UnboundedReceiver<NodeEvent>,
        timer: T,
        spawn: S,
    ) -> Self {
        let transfers = TransferControl::new(
            Swarms { node: node.clone() },
            timer,
            Config {
                discovery: false,
                ..Config::default()
            },
        );
        let services = PeerServices::with_spawner(node.clone(), spawn)
            .with(Rc::new(Transfers(transfers.clone())));
        Peers {
            events,
            services,
            transfers,
        }
    }

    /// Runs `swarm`, whose id is `id`, with every peer that has it: its
    /// transfer, published on `()`.
    pub fn publish(&self, swarm: SwarmKey, id: &SwarmId) {
        let t = self.transfers.add(swarm, id, Vec::new());
        self.transfers.publish(t, ());
    }

    /// Serves the peers, and the transfers' calls and node operations,
    /// until the node's events end. `added` takes each peer as it links,
    /// before the services do.
    pub async fn serve(&mut self, mut added: impl FnMut(&PeerInfo)) {
        let Peers {
            events,
            services,
            transfers,
        } = self;
        let events = async {
            while let Some(e) = events.recv().await {
                if let NodeEvent::PeerAdded(p) = &e {
                    added(p);
                }
                services.handle_event(&e).await;
            }
        };
        tokio::select! {
            biased;
            () = events => {}
            () = transfers.run() => {}
        }
    }
}

/// Whether [`Peers`] takes the node event `e`: a peer's link, its going,
/// or its ppspp failure.
pub fn serves(e: &NodeEvent) -> bool {
    matches!(
        e,
        NodeEvent::PeerAdded(_) | NodeEvent::PeerRemoved { .. } | NodeEvent::TransferError { .. }
    )
}

/// The transfers as a peer handler: every peer joins `()` as it links.
struct Transfers<T: Timer>(TransferControl<Swarms, T>);

impl<T: Timer> PeerHandler for Transfers<T> {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        self.0.add_peer(peer.host_id, router, client);
        self.0.add_network_peer((), peer.host_id);
    }

    fn remove_peer(&self, host: HostId) {
        self.0.remove_peer(host);
    }

    fn handle_event(&self, e: &NodeEvent) {
        if let NodeEvent::TransferError { host_id, .. } = e {
            self.0.transfer_failed(*host_id);
        }
    }
}

/// The node as the transfers' host: its swarm channels, on the one
/// network `()`. It has no peer index, which discovery alone would use.
struct Swarms {
    node: Node,
}

/// What a peer index call answers: discovery is off.
const NO_INDEX: NodeError = NodeError::Unsupported("the testbed's transfers have no peer index");

impl Host for Swarms {
    type Network = ();
    type Swarm = SwarmKey;
    type Publication = ();
    type Search = Empty<HostId>;
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
            // Its channels went with it.
            Err(NodeError::UnknownPeer) => Ok(()),
            r => r,
        }
    }

    async fn index_publish(&self, (): (), _: Vec<u8>, _: Vec<u8>) -> Result<(), NodeError> {
        Err(NO_INDEX)
    }

    async fn index_unpublish(&self, (): ()) -> Result<(), NodeError> {
        Err(NO_INDEX)
    }

    async fn index_search(
        &self,
        (): (),
        _: Vec<u8>,
        _: Vec<u8>,
    ) -> Result<Empty<HostId>, NodeError> {
        Err(NO_INDEX)
    }

    async fn connect_peer(&self, (): (), _: HostId) -> Result<bool, NodeError> {
        Err(NO_INDEX)
    }
}
