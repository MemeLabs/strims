// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The app layer on a node: the node's clock as the app's [`Timer`], the
//! node as the [`Host`] of the services a profile runs (replication,
//! transfers and their record streams, network membership, the directory,
//! chat, whispers and video), and every peer's RPC link served with the
//! app's [`PeerHandler`]s.
//!
//! A profile's services share one [`NodeHost`]. The directory control is
//! built on the profile's network control and transfer control, and its
//! swarms move only while the transfer control learns every peer: give
//! [`PeerServices`] both controls. The chat and video controls are built
//! on those and the directory control, the whisper control on the network
//! control. The video control's RTMP ingress listens through a
//! [`NetVideoHost`] on the node's [`Net`].
//!
//! The node is bootstrap's [`strims_network::bootstrap::Host`] too: it
//! dials the profile's bootstrap clients.
//!
//! [`NodeFactory`] is the [`SessionFactory`] that composes all of this for
//! a signed-in profile: it starts the profile's node and every control,
//! and serves the session's frontends on the UI's connection. It holds
//! the [`HlsRoutes`] the running profile's HLS streams are served on, for
//! the driver's HTTP server.
//!
//! App-layer futures aren't `Send`: [`PeerServices`] spawns its
//! connections as local tasks, and [`NodeFactory`] its controls, so they
//! run inside a tokio `LocalSet`.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::Stream;
use rand_core::SeedableRng;
use strims_base::rng::{DetRng, fork};
use strims_base::time::{Duration, Time};
use strims_chat::{ChatControl, WhisperConfig, WhisperControl};
use strims_directory::{DirectoryControl, Fetch};
use strims_kv::Kv;
use strims_network::NetworkControl;
use strims_network::bootstrap::{BootstrapControl, Failure};
use strims_proto::types::Key;
use strims_replication::{Host, ReplicationControl};
use strims_rpc::{Client, Config, Router, Timer, peer_connection};
use strims_session::{Session, SessionFactory};
use strims_transfer::{ReadFailure, SwarmRead, SwarmWrite, TransferControl};
use strims_video::VideoControl;
use strims_video::hls::HlsRoutes;
use strims_vpn::{
    HashTableGetOptions, HashTablePublisherId, HostAddr, NetworkId, PeerIndexPublisherId,
};
use tokio::task::AbortHandle;
use tokio::time::Sleep;

use crate::fetch::{NetFetch, tls_client_config};
use crate::video_host::NetVideoHost;
use crate::{
    Clock, HostId, Net, NetworkOptions, Node, NodeConfig, NodeError, NodeEvent, NodeInfo,
    PeerIndexSearch, PeerInfo, ReadError, RpcReader, RpcWriter, SwarmId, SwarmKey, SwarmOptions,
    SwarmReader, SwarmWriter, VpnReader, VpnWriter, WriterOptions,
};

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

/// A node as replication's [`Host`], on the networks the driver adds, as
/// the transfers' [`strims_transfer::Host`] and
/// [`strims_transfer::SwarmHost`], as network membership's
/// [`strims_network::Host`], which adds and removes the networks it uses,
/// and as the directory's [`strims_directory::Host`].
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

    async fn remove_swarm(&self, swarm: SwarmKey) -> Result<(), NodeError> {
        self.node.remove_swarm(swarm).await
    }

    async fn open_reader(&self, swarm: SwarmKey) -> Result<SwarmReader, NodeError> {
        self.node.open_reader(swarm).await
    }
}

/// Underruns and resets are discontinuities; the reader ends when its
/// swarm or the node does.
impl SwarmRead for SwarmReader {
    type Error = ReadError;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<ReadError>> {
        SwarmReader::read_at(self, out).await.map_err(|e| match e {
            ReadError::Underrun { .. } | ReadError::Reset => ReadFailure::Discontinuity,
            ReadError::Closed | ReadError::Stopped => ReadFailure::Closed,
            e @ ReadError::Store(_) => ReadFailure::Failed(e),
        })
    }
}

impl SwarmWrite for SwarmWriter {
    type Error = NodeError;

    async fn write(&mut self, data: &[u8]) -> Result<(), NodeError> {
        SwarmWriter::write(self, data).await
    }

    async fn reset(&mut self) -> Result<(), NodeError> {
        SwarmWriter::reset(self).await
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

// --- bootstrap ----------------------------------------------------------------

/// A node as bootstrap's [`strims_network::bootstrap::Host`]: it dials
/// `tcp://` and `ws://` link URIs. A URI the node can't dial, or one that
/// reaches it, fails permanently.
impl strims_network::bootstrap::Host for NodeHost {
    async fn dial(&self, uri: &str) -> Result<HostId, Failure> {
        if uri.starts_with("wss://") {
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

// --- sessions -----------------------------------------------------------------

/// How a profile's node and services run.
#[derive(Clone)]
pub struct AppConfig {
    /// `host:port` the node accepts TCP links on; port 0 picks one.
    pub tcp_listen: Option<String>,
    /// `host:port` the node accepts WebSocket links on, at
    /// `/<hex profile key>`.
    pub ws_listen: Option<String>,
    /// The address link URIs advertise for a listener on an unspecified
    /// address.
    pub public_ip: Option<IpAddr>,
    /// The host WebSocket link URIs advertise; empty for `public_ip`.
    pub ws_public_hostname: String,
    /// The port WebSocket link URIs advertise; 0 for the listener's.
    pub ws_public_port: u16,
    /// Whether the node dials `ws://` links, which aren't encrypted by TLS.
    pub allow_insecure_ws: bool,
    /// vnic's peer budget, which gates peer exchange.
    pub max_peers: u32,
    /// The host certificate's subject.
    pub label: String,
    pub network: strims_network::Config,
    pub bootstrap: strims_network::bootstrap::Config,
    pub transfer: strims_transfer::Config,
    pub replication: strims_replication::Config,
    pub directory: strims_directory::Config,
    pub chat: strims_chat::Config,
    pub whispers: WhisperConfig,
    pub video: strims_video::Config,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            tcp_listen: None,
            ws_listen: None,
            public_ip: None,
            ws_public_hostname: String::new(),
            ws_public_port: 0,
            allow_insecure_ws: false,
            max_peers: 16,
            label: String::new(),
            network: strims_network::Config::default(),
            bootstrap: strims_network::bootstrap::Config::default(),
            transfer: strims_transfer::Config::default(),
            replication: strims_replication::Config::default(),
            directory: strims_directory::Config::default(),
            chat: strims_chat::Config::default(),
            whispers: WhisperConfig::default(),
            video: strims_video::Config::default(),
        }
    }
}

/// Why a profile's node didn't start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
    #[error("starting the node: {0}")]
    Node(#[from] NodeError),
    #[error("drawing a seed from OS entropy: {0}")]
    Entropy(getrandom::Error),
    #[error("profile has no key")]
    NoProfileKey,
    /// The listeners belong to another profile's node.
    #[error("this process runs another profile's node, and it runs one at a time")]
    ProfileRunning,
}

/// The [`SessionFactory`] that runs a signed-in profile's node and
/// services. A process runs one profile's node at a time, since the node
/// listens on the configured addresses: starting another profile's
/// session fails with [`StartError::ProfileRunning`].
///
/// A session's node runs replication, transfers, network membership,
/// bootstrap, the directory, chat, whispers and video, each control as a
/// local task, and serves every peer through [`PeerServices`]. The video
/// control's RTMP ingress listens on the factory's [`Net`], and the
/// directory's servers make their HTTPS requests over it ([`NetFetch`]).
pub struct NodeFactory<N: Net, C: Clock> {
    net: N,
    clock: C,
    config: AppConfig,
    /// The TLS client config of every node's HTTPS requests.
    tls: Arc<rustls::ClientConfig>,
    /// The profile whose node runs or starts, and the node once it runs.
    running: RefCell<Option<(u64, Option<NodeInfo>)>>,
    hls: HlsRoutes,
}

impl<N: Net, C: Clock> NodeFactory<N, C> {
    /// A factory of nodes on `net`, timed by `clock`.
    pub fn new(net: N, clock: C, config: AppConfig) -> Self {
        NodeFactory {
            net,
            clock,
            config,
            tls: tls_client_config(),
            running: RefCell::new(None),
            hls: HlsRoutes::new(),
        }
    }

    /// The running node, if a session started one.
    pub fn running(&self) -> Option<NodeInfo> {
        self.running.borrow().as_ref().and_then(|(_, n)| n.clone())
    }

    /// The routes of the running profile's HLS streams, for the driver's
    /// HTTP server to answer requests from. The video control serves its
    /// streams there only where [`strims_video::HlsSettings::serve`] is
    /// set in [`AppConfig::video`].
    pub fn hls_routes(&self) -> &HlsRoutes {
        &self.hls
    }

    fn node_config(&self, key: Key, seed: u64) -> NodeConfig {
        let c = &self.config;
        let mut config = NodeConfig::new(key, seed);
        config.label.clone_from(&c.label);
        config.tcp_listen.clone_from(&c.tcp_listen);
        config.ws_listen.clone_from(&c.ws_listen);
        config.public_ip = c.public_ip;
        config.ws_public_hostname.clone_from(&c.ws_public_hostname);
        config.ws_public_port = c.ws_public_port;
        config.allow_insecure_ws = c.allow_insecure_ws;
        config.max_peers = c.max_peers;
        config.node_platform = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);
        config.node_version = env!("CARGO_PKG_VERSION").to_owned();
        config
    }

    async fn start_node<K: Kv>(
        &self,
        session: &Rc<Session<K>>,
    ) -> Result<AppRuntime<K, N, C>, StartError> {
        let key = session
            .profile
            .key
            .clone()
            .ok_or(StartError::NoProfileKey)?;
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(StartError::Entropy)?;
        let mut rng = DetRng::from_seed(seed);
        let node_seed = rand_core::Rng::next_u64(&mut rng);
        let config = self.node_config(key, node_seed);
        let (node, mut events) = Node::spawn(config, self.net.clone(), self.clock.clone()).await?;

        let c = &self.config;
        let timer = NodeTimer(self.clock.clone());
        let (store, profile) = (&session.store, &session.profile);
        let host = NodeHost::new(node.clone());
        let replication = ReplicationControl::new(
            store.clone(),
            profile.clone(),
            host.clone(),
            timer.clone(),
            fork(&mut rng),
            c.replication.clone(),
        );
        let network = NetworkControl::new(
            host.clone(),
            store.clone(),
            profile.clone(),
            timer.clone(),
            fork(&mut rng),
            c.network,
        );
        let bootstrap = BootstrapControl::new(
            host.clone(),
            store.clone(),
            profile.clone(),
            timer.clone(),
            fork(&mut rng),
            c.bootstrap,
        );
        let transfers = TransferControl::new(host.clone(), timer.clone(), c.transfer);
        let directory = DirectoryControl::new(
            host.clone(),
            store.clone(),
            network.clone(),
            transfers.clone(),
            timer.clone(),
            fork(&mut rng),
            c.directory.clone(),
        )
        .with_fetch(Fetch::new(NetFetch::new(
            self.net.clone(),
            self.tls.clone(),
        )));
        let chat = ChatControl::new(
            host.clone(),
            store.clone(),
            profile.clone(),
            network.clone(),
            transfers.clone(),
            directory.clone(),
            timer.clone(),
            fork(&mut rng),
            c.chat.clone(),
        );
        let whispers = WhisperControl::new(
            host.clone(),
            store.clone(),
            profile.clone(),
            network.clone(),
            timer.clone(),
            fork(&mut rng),
            c.whispers.clone(),
        );
        let video = VideoControl::new(
            host,
            NetVideoHost(self.net.clone()),
            store.clone(),
            profile.clone(),
            network.clone(),
            transfers.clone(),
            directory.clone(),
            self.hls.clone(),
            timer.clone(),
            fork(&mut rng),
            c.video.clone(),
        );

        let mut tasks = Vec::new();
        let mut spawn = |f: futures_util::future::LocalBoxFuture<'static, ()>| {
            tasks.push(tokio::task::spawn_local(f).abort_handle());
        };
        spawn({
            let replication = replication.clone();
            Box::pin(async move { replication.run().await })
        });
        spawn({
            let network = network.clone();
            Box::pin(async move { network.run().await })
        });
        spawn({
            let bootstrap = bootstrap.clone();
            Box::pin(async move { bootstrap.run().await })
        });
        spawn({
            let transfers = transfers.clone();
            Box::pin(async move { transfers.run().await })
        });
        spawn({
            let directory = directory.clone();
            Box::pin(async move { directory.run().await })
        });
        spawn({
            let chat = chat.clone();
            Box::pin(async move { chat.run().await })
        });
        spawn({
            let whispers = whispers.clone();
            Box::pin(async move { whispers.run().await })
        });
        spawn({
            let video = video.clone();
            Box::pin(async move { video.run().await })
        });
        let services = PeerServices::new(node.clone())
            .with(Rc::new(network.clone()))
            .with(Rc::new(transfers))
            .with(Rc::new(replication))
            .with(Rc::new(bootstrap.clone()));
        spawn(Box::pin(async move {
            while let Some(e) = events.recv().await {
                services.handle_event(&e).await;
            }
        }));

        tracing::info!(
            profile = %session.profile.name,
            ws_uri = node.ws_uri().unwrap_or_default(),
            tcp_uri = node.tcp_uri().unwrap_or_default(),
            "node started",
        );
        Ok(AppRuntime {
            node,
            timer,
            network,
            bootstrap,
            directory,
            chat,
            whispers,
            video,
            tasks,
        })
    }
}

impl<K: Kv, N: Net, C: Clock> SessionFactory<K> for NodeFactory<N, C> {
    type Runtime = AppRuntime<K, N, C>;
    type Error = StartError;

    async fn start(&self, session: &Rc<Session<K>>) -> Result<AppRuntime<K, N, C>, StartError> {
        let id = session.profile.id;
        {
            let mut running = self.running.borrow_mut();
            match &*running {
                Some((other, _)) if *other != id => return Err(StartError::ProfileRunning),
                _ => *running = Some((id, None)),
            }
        }
        // Dropped unstarted, a start gives the listeners back.
        let mut mark = StartMark {
            running: &self.running,
            started: false,
        };
        let runtime = self.start_node(session).await?;
        *self.running.borrow_mut() = Some((id, Some(runtime.node.info().clone())));
        mark.started = true;
        Ok(runtime)
    }

    fn bind(&self, runtime: &AppRuntime<K, N, C>, session: &Rc<Session<K>>, router: &Router) {
        strims_session::bind_frontends(session, &runtime.timer, router);
        strims_network::bind_frontend(router, &runtime.network);
        strims_network::bootstrap::bind_frontend(router, &runtime.bootstrap);
        strims_directory::bind_frontend(router, &runtime.directory);
        strims_chat::bind_frontend(router, &runtime.chat, &runtime.whispers);
        strims_video::bind_frontend(router, &runtime.video);
    }
}

/// Clears the factory's running profile unless its start finished.
struct StartMark<'a> {
    running: &'a RefCell<Option<(u64, Option<NodeInfo>)>>,
    started: bool,
}

impl Drop for StartMark<'_> {
    fn drop(&mut self) {
        if !self.started {
            *self.running.borrow_mut() = None;
        }
    }
}

/// A running profile's node and the controls its frontends serve. Dropping
/// it stops the controls' tasks and the node.
pub struct AppRuntime<K: Kv, N: Net, C: Clock> {
    node: Node,
    timer: NodeTimer<C>,
    network: NetworkControl<K, NodeHost, NodeTimer<C>>,
    bootstrap: BootstrapControl<K, NodeHost, NodeTimer<C>>,
    directory: DirectoryControl<K, NodeHost, NodeTimer<C>>,
    chat: ChatControl<K, NodeHost, NodeTimer<C>>,
    whispers: WhisperControl<K, NodeHost, NodeTimer<C>>,
    video: VideoControl<K, NodeHost, NetVideoHost<N>, NodeTimer<C>>,
    tasks: Vec<AbortHandle>,
}

impl<K: Kv, N: Net, C: Clock> AppRuntime<K, N, C> {
    /// The profile's node.
    pub fn node(&self) -> &Node {
        &self.node
    }
}

impl<K: Kv, N: Net, C: Clock> Drop for AppRuntime<K, N, C> {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
        if tokio::runtime::Handle::try_current().is_ok() {
            let node = self.node.clone();
            tokio::spawn(async move {
                let _ = node.shutdown().await;
            });
        }
    }
}
