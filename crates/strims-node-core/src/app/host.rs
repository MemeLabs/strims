// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node as the host of a profile's services.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_util::Stream;
use strims_base::time::Duration;
use strims_network::bootstrap::Failure;
use strims_proto::types::Key;
use strims_qos::DEFAULT_RATE_LIMIT;
use strims_replication::Host;
use strims_session::{VnicHost, VnicLimits};
use strims_vpn::{
    HashTableGetOptions, HashTablePublisherId, HostAddr, NetworkId, PeerIndexPublisherId,
};

use crate::{
    HostId, NetworkOptions, Node, NodeError, PeerIndexSearch, RpcReader, RpcWriter, SwarmId,
    SwarmKey, SwarmOptions, SwarmReader, SwarmWriter, VpnReader, VpnWriter, WriterOptions,
};

/// How long a discovery lookup waits for answers.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);

/// A node as replication's [`Host`], on the networks the driver adds, as
/// the transfers' [`strims_transfer::Host`] and
/// [`strims_transfer::SwarmHost`], as network membership's
/// [`strims_network::Host`], which adds and removes the networks it uses,
/// as bootstrap's [`strims_network::bootstrap::Host`], as the directory's
/// [`strims_directory::Host`], and as the [`VnicHost`] of the profile's
/// vnic limits.
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

/// The node's swarms, for services' record streams.
impl strims_transfer::SwarmHost for NodeHost {
    type Writer = SwarmWriter;
    type Reader = SwarmReader;

    async fn create_swarm(
        &self,
        options: WriterOptions,
    ) -> Result<(SwarmKey, SwarmId, SwarmWriter), NodeError> {
        self.node.create_swarm(options).await
    }

    async fn add_swarm(&self, id: SwarmId, options: SwarmOptions) -> Result<SwarmKey, NodeError> {
        self.node.add_swarm(id, options).await
    }

    async fn add_cached_swarm(
        &self,
        id: SwarmId,
        options: SwarmOptions,
        cache: strims_proto::types::Cache,
    ) -> Result<(SwarmKey, bool), NodeError> {
        self.node.add_cached_swarm(id, options, cache).await
    }

    /// `None` once the node has stopped.
    async fn export_cache(&self, swarm: SwarmKey) -> Option<strims_proto::types::Cache> {
        self.node.export_cache(swarm).await.ok().flatten()
    }

    async fn remove_swarm(&self, swarm: SwarmKey) -> Result<(), NodeError> {
        self.node.remove_swarm(swarm).await
    }

    async fn open_reader(&self, swarm: SwarmKey) -> Result<SwarmReader, NodeError> {
        self.node.open_reader(swarm).await
    }
}

impl strims_network::Host for NodeHost {
    type Network = NetworkId;
    type BrokerReader = RpcReader;
    type BrokerWriter = RpcWriter;
    type VpnReader = VpnReader;
    type VpnWriter = VpnWriter;
    type Publication = HashTablePublisherId;
    type Error = NodeError;

    fn host_id(&self) -> HostId {
        self.node.host_id()
    }

    async fn add_network(&self, key: &[u8]) -> Result<NetworkId, NodeError> {
        let net = match self.node.add_network(key, NetworkOptions::default()).await {
            Err(NodeError::Vpn(strims_vpn::Error::DuplicateNetworkKey)) => self
                .node
                .network(key)
                .await?
                .ok_or(NodeError::Vpn(strims_vpn::Error::DuplicateNetworkKey))?,
            r => r?,
        };
        self.networks.borrow_mut().insert(net);
        Ok(net)
    }

    async fn remove_network(&self, net: NetworkId) -> Result<(), NodeError> {
        self.networks.borrow_mut().remove(&net);
        self.node.remove_network(net).await
    }

    async fn take_broker(&self, host: HostId) -> Result<(RpcReader, RpcWriter), NodeError> {
        let channel = self.node.take_peer_broker(host).await?;
        Ok((channel.reader, channel.writer))
    }

    async fn reserve_peer_port(&self, host: HostId) -> Result<u16, NodeError> {
        self.node.reserve_peer_port(host).await
    }

    async fn release_peer_port(&self, host: HostId, port: u16) -> Result<(), NodeError> {
        self.node.release_peer_port(host, port).await
    }

    async fn bind_peer(
        &self,
        host: HostId,
        net: NetworkId,
        local_port: u16,
        peer_port: u16,
    ) -> Result<(), NodeError> {
        self.node
            .bind_peer_network(host, net, local_port, peer_port)
            .await
    }

    async fn unbind_peer(&self, host: HostId, net: NetworkId) -> Result<(), NodeError> {
        self.node.unbind_peer_network(host, net).await.map(drop)
    }

    async fn open_vpn_port(
        &self,
        net: NetworkId,
    ) -> Result<(u16, VpnReader, VpnWriter), NodeError> {
        let p = self.node.open_vpn_port(net, None).await?;
        Ok((p.port, p.reader, p.writer))
    }

    async fn publish_address(
        &self,
        net: NetworkId,
        key: Key,
        salt: Vec<u8>,
        port: u16,
    ) -> Result<HashTablePublisherId, NodeError> {
        self.node.publish_vpn_address(net, key, salt, port).await
    }

    async fn unpublish_address(&self, publication: HashTablePublisherId) -> Result<(), NodeError> {
        self.node.hash_table_close(publication).await
    }

    async fn resolve_address(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<Option<HostAddr>, NodeError> {
        let options = HashTableGetOptions {
            disable_cache: true,
            timeout: LOOKUP_TIMEOUT,
        };
        self.node.resolve_vpn_address(net, key, salt, options).await
    }
}

/// The directory opens its snippet port by number.
impl strims_directory::Host for NodeHost {
    async fn open_vpn_port_at(
        &self,
        net: NetworkId,
        port: u16,
    ) -> Result<(VpnReader, VpnWriter), NodeError> {
        let p = self.node.open_vpn_port(net, Some(port)).await?;
        Ok((p.reader, p.writer))
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

/// A node as bootstrap's [`strims_network::bootstrap::Host`]: it dials
/// `tcp://` and `ws://` link URIs, and `wss://` where its driver has it
/// ([`crate::Interfaces::wss`]). A URI the node can't dial, or one that
/// reaches it, fails permanently.
impl strims_network::bootstrap::Host for NodeHost {
    async fn dial(&self, uri: &str) -> Result<HostId, Failure> {
        if uri.starts_with("wss://") && !self.node.interfaces().wss {
            return Err(Failure::Permanent("wss:// isn't supported".to_owned()));
        }
        match self.node.dial(uri).await {
            Ok(host) => Ok(host),
            Err(
                e @ (NodeError::InvalidUri(_) | NodeError::OwnHost | NodeError::Unsupported(_)),
            ) => Err(Failure::Permanent(e.to_string())),
            Err(e) => Err(Failure::Retry(e.to_string())),
        }
    }

    fn own_uris(&self) -> Vec<String> {
        let info = self.node.info();
        info.tcp_uri.iter().chain(&info.ws_uri).cloned().collect()
    }
}

/// The node's upload cap and peer budget, from the profile's vnic config.
impl VnicHost for NodeHost {
    type Error = NodeError;

    async fn set_vnic_limits(&self, limits: VnicLimits) -> Result<(), NodeError> {
        let (rate_limit, max_peers) = node_limits(limits);
        self.node.set_rate_limit(rate_limit).await?;
        self.node.set_max_peers(max_peers).await
    }
}

/// `limits` as [`crate::NodeConfig::rate_limit`] and
/// [`crate::NodeConfig::max_peers`]. QoS's [`DEFAULT_RATE_LIMIT`] is both
/// the unlimited upload cap and the largest, and a peer budget of 0 is
/// unlimited.
pub(crate) fn node_limits(limits: VnicLimits) -> (u64, u32) {
    let upload = limits
        .upload
        .map_or(DEFAULT_RATE_LIMIT, |u| u.min(DEFAULT_RATE_LIMIT));
    (upload, limits.max_peers.unwrap_or(0))
}
