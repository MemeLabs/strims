// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node API: a cloneable [`Node`] handle that sends commands to the node
//! task, plus streams and swarm handles.
//!
//! Every call goes to the node task over a bounded channel and its reply
//! comes back over a oneshot. Swarm readers and writers keep their own
//! reply channel and buffer, so reading and publishing don't allocate per
//! call.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, fence};
use std::task::{Context, Poll};

use strims_base::time::Time;
use strims_kademlia::ID;
use strims_ppspp::{SwarmId, SwarmKey, SwarmOptions, WriterOptions};
use strims_proto::types::{Cache, Certificate, Key};
use strims_transfer::{ReadFailure, SwarmRead, SwarmWrite};
use strims_vpn::{
    HashTableGetOptions, HashTablePublisherId, HostAddr, M_BROADCAST, M_STD_FLAGS, NetworkId,
    PeerIndexHost, PeerIndexPublisherId,
};
use tokio::sync::{mpsc, oneshot};

use crate::config::{NetworkOptions, NodeConfig};
use crate::driver::{Call, Command, Interfaces};
use crate::error::{NodeError, ReadError};
use crate::event::{NodeEvent, NodeMetrics, PeerInfo};
use crate::rpc::{PeerRpc, RpcChannel};
use crate::state::{PublisherId, ReadReply, ReaderId, State, WriteOp, WriteReply};
use crate::vpn_port::VpnPort;

/// Results buffered per hash table get or peer index search.
const RESULT_QUEUE: usize = 16;
/// A swarm handle's buffer: the most bytes per read or write call.
const SWARM_IO_BUFFER: usize = 64 * 1024;

/// What a node is and where it listens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeInfo {
    /// The vnic host id (the host key's public key).
    pub host_id: ID,
    /// The profile public key.
    pub profile_key: Vec<u8>,
    pub tcp_addr: Option<SocketAddr>,
    pub ws_addr: Option<SocketAddr>,
    /// `tcp://<ip>:<port>/<hex profile key>`, when it can be formatted.
    pub tcp_uri: Option<String>,
    /// `ws://<host>:<port>/<hex profile key>`.
    pub ws_uri: Option<String>,
    /// The WebRTC interface's UDP sockets, as bound (empty when it is
    /// off).
    pub webrtc_addrs: Vec<SocketAddr>,
}

/// A running node. Cloning shares the node; it stops on
/// [`Node::shutdown`] or when every handle (swarm handles included) is
/// dropped.
#[derive(Clone, Debug)]
pub struct Node {
    tx: mpsc::Sender<Command>,
    /// Read-only calls, answered after the node's own passes.
    queries: mpsc::Sender<Call>,
    /// Set by the node task while its next pass is due within
    /// `NodeConfig::query_delay`.
    answers_soon: Arc<AtomicBool>,
    info: Arc<NodeInfo>,
    interfaces: Interfaces,
}

/// Spawns a node on a driver's executor: what [`Node::spawn`] calls.
///
/// A driver crate implements it for its clocks, over any of its socket
/// providers `N`: the clock is the side a driver defines whichever
/// sockets the caller brings.
pub trait SpawnNode<N> {
    /// Builds the node on this clock and `net`'s sockets, and spawns its
    /// task.
    fn spawn_node(
        self,
        config: NodeConfig,
        net: N,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>> + Send;
}

/// The node's event stream.
#[derive(Debug)]
pub struct NodeEvents {
    rx: mpsc::Receiver<NodeEvent>,
}

impl NodeEvents {
    pub(crate) fn new(rx: mpsc::Receiver<NodeEvent>) -> NodeEvents {
        NodeEvents { rx }
    }

    /// The next event; `None` once the node has stopped.
    pub async fn recv(&mut self) -> Option<NodeEvent> {
        self.rx.recv().await
    }

    pub fn try_recv(&mut self) -> Option<NodeEvent> {
        self.rx.try_recv().ok()
    }
}

impl Node {
    pub(crate) fn new(
        tx: mpsc::Sender<Command>,
        queries: mpsc::Sender<Call>,
        answers_soon: Arc<AtomicBool>,
        info: Arc<NodeInfo>,
        interfaces: Interfaces,
    ) -> Node {
        Node {
            tx,
            queries,
            answers_soon,
            info,
            interfaces,
        }
    }

    /// Builds the node on `net`'s sockets, binds its listeners and spawns
    /// the node task (see [`SpawnNode`]).
    pub fn spawn<N, C: SpawnNode<N>>(
        config: NodeConfig,
        net: N,
        clock: C,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>> + Send {
        clock.spawn_node(config, net)
    }

    pub fn info(&self) -> &NodeInfo {
        &self.info
    }

    /// The link interfaces the node's driver has.
    pub fn interfaces(&self) -> Interfaces {
        self.interfaces
    }

    pub fn host_id(&self) -> ID {
        self.info.host_id
    }

    pub fn tcp_uri(&self) -> Option<&str> {
        self.info.tcp_uri.as_deref()
    }

    pub fn ws_uri(&self) -> Option<&str> {
        self.info.ws_uri.as_deref()
    }

    /// Runs `f` on the node task and returns its result.
    async fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut State, Time) -> T + Send + 'static,
    ) -> Result<T, NodeError> {
        let (tx, rx) = oneshot::channel();
        let cmd = Command::Call(Box::new(move |s, now| {
            let _ = tx.send(f(s, now));
        }));
        self.tx.send(cmd).await.map_err(|_| NodeError::Stopped)?;
        rx.await.map_err(|_| NodeError::Stopped)
    }

    /// Runs a read-only `f` on the node task after its next pass and
    /// returns the result. It wakes the node only when no pass is due
    /// within [`NodeConfig::query_delay`]. `f` must not change anything the
    /// node does; reading, or advancing a rate window to `now` (which every
    /// later reader does the same way), is fine.
    async fn query<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut State, Time) -> T + Send + 'static,
    ) -> Result<T, NodeError> {
        let (tx, rx) = oneshot::channel();
        let q: Call = Box::new(move |s, now| {
            let _ = tx.send(f(s, now));
        });
        self.queries.send(q).await.map_err(|_| NodeError::Stopped)?;
        // Queue first, read the flag second (the node publishes, then
        // drains): either the node's drain sees the query, or this sees
        // the flag that drain published.
        fence(Ordering::SeqCst);
        if !self.answers_soon.load(Ordering::SeqCst) {
            self.tx
                .send(Command::Wake)
                .await
                .map_err(|_| NodeError::Stopped)?;
        }
        rx.await.map_err(|_| NodeError::Stopped)
    }

    /// Stops the node: closes every link and ends the node task.
    pub async fn shutdown(&self) -> Result<(), NodeError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Shutdown(tx))
            .await
            .map_err(|_| NodeError::Stopped)?;
        rx.await.map_err(|_| NodeError::Stopped)
    }

    // --- links and peers ----------------------------------------------------

    /// Dials a `tcp://` or `ws://` link URI and waits for the vnic
    /// handshake. Returns the peer's host id.
    pub async fn dial(&self, uri: &str) -> Result<ID, NodeError> {
        let (tx, rx) = oneshot::channel();
        let uri = uri.to_owned();
        self.call(move |s, _| s.start_dial(&uri, Some(tx)))
            .await??;
        rx.await.map_err(|_| NodeError::Stopped)?
    }

    /// The connected peers. A query: it doesn't change what the node does
    /// (see [`NodeConfig::query_delay`]).
    pub async fn peers(&self) -> Result<Vec<PeerInfo>, NodeError> {
        self.query(|s, _| s.peers()).await
    }

    /// Closes the link to `host_id`. Returns whether it was connected.
    pub async fn close_peer(&self, host_id: ID) -> Result<bool, NodeError> {
        self.call(move |s, _| s.close_peer(&host_id)).await
    }

    /// The peer's RPC channel pair (see [`crate::rpc`]). Each pair is taken
    /// once; a link that replaces the peer's comes with a new pair.
    pub async fn take_peer_rpc(&self, host_id: ID) -> Result<PeerRpc, NodeError> {
        self.call(move |s, _| s.take_peer_rpc(&host_id)).await?
    }

    /// The peer's network broker channel (see [`crate::rpc`]), taken once
    /// per link like its RPC pair.
    pub async fn take_peer_broker(&self, host_id: ID) -> Result<RpcChannel, NodeError> {
        self.call(move |s, _| s.take_peer_broker(&host_id)).await?
    }

    /// Changes the upload cap (see [`NodeConfig::rate_limit`]), and with
    /// it the upload capacity ppspp takes as known, unless
    /// [`NodeConfig::upload_capacity`] set one.
    pub async fn set_rate_limit(&self, bytes_per_sec: u64) -> Result<(), NodeError> {
        self.call(move |s, _| s.set_rate_limit(bytes_per_sec)).await
    }

    /// Changes vnic's peer budget (see [`NodeConfig::max_peers`]). Peers
    /// past a lowered budget stay connected.
    pub async fn set_max_peers(&self, n: u32) -> Result<(), NodeError> {
        self.call(move |s, _| s.vnic.set_max_peers(n)).await
    }

    /// The upload cap in bytes per second and the peer budget in effect,
    /// where an unlimited budget is `i32::MAX`. A query (see
    /// [`NodeConfig::query_delay`]).
    pub async fn vnic_limits(&self) -> Result<(u64, usize), NodeError> {
        self.query(|s, _| (s.vnic.qos().rate_limit(), s.vnic.max_peers()))
            .await
    }

    /// Every counter in the node: vnic, vpn, the driver, links, the ppspp
    /// runner (swarms, peers, channels) and vpn networks. Cheap enough to
    /// poll every second. A query: sampling doesn't change what the node
    /// does, even under a backlogged upload cap (see
    /// [`NodeConfig::query_delay`]).
    pub async fn metrics(&self) -> Result<NodeMetrics, NodeError> {
        self.query(|s, now| s.metrics(now)).await
    }

    // --- networks -----------------------------------------------------------

    /// Joins the network with this key (the network CA's public key).
    pub async fn add_network(
        &self,
        key: &[u8],
        options: NetworkOptions,
    ) -> Result<NetworkId, NodeError> {
        let key = key.to_vec();
        self.call(move |s, _| s.add_network(&key, options)).await?
    }

    /// Joins the network a certificate belongs to (its root's key).
    pub async fn add_network_certificate(
        &self,
        cert: &Certificate,
        options: NetworkOptions,
    ) -> Result<NetworkId, NodeError> {
        let chain = strims_crypto::CertificateChain::decode(cert)?;
        self.add_network(chain.network_key(), options).await
    }

    /// The network whose key is `key`, if the node has joined it.
    pub async fn network(&self, key: &[u8]) -> Result<Option<NetworkId>, NodeError> {
        let key = key.to_vec();
        self.call(move |s, _| s.vpn.network(&key)).await
    }

    pub async fn remove_network(&self, net: NetworkId) -> Result<(), NodeError> {
        self.call(move |s, _| s.remove_network(net)).await?
    }

    /// Binds a connected peer to a network: frames on `local_port` are the
    /// network's, and it sends to `peer_port`. The peer must bind the
    /// reverse pair. A new binding is reported as
    /// [`NodeEvent::NetworkPeerAdded`]. Explicit until the app layer
    /// negotiates network membership (see [Bindings](crate#bindings)).
    pub async fn bind_peer_network(
        &self,
        host_id: ID,
        net: NetworkId,
        local_port: u16,
        peer_port: u16,
    ) -> Result<(), NodeError> {
        self.call(move |s, _| s.bind_peer_network(&host_id, net, local_port, peer_port))
            .await?
    }

    /// Unbinds a peer from a network, reported as
    /// [`NodeEvent::NetworkPeerRemoved`]. Returns whether it was bound.
    pub async fn unbind_peer_network(
        &self,
        host_id: ID,
        net: NetworkId,
    ) -> Result<bool, NodeError> {
        self.call(move |s, _| s.unbind_peer_network(&host_id, net))
            .await
    }

    /// A free vnic port on the peer, for a binding.
    pub async fn reserve_peer_port(&self, host_id: ID) -> Result<u16, NodeError> {
        self.call(move |s, _| s.reserve_peer_port(&host_id)).await?
    }

    /// Releases a port from [`Node::reserve_peer_port`].
    pub async fn release_peer_port(&self, host_id: ID, port: u16) -> Result<(), NodeError> {
        self.call(move |s, _| s.release_peer_port(&host_id, port))
            .await?
    }

    // --- vpn messaging ------------------------------------------------------

    /// Delivers messages for `port` on the network as
    /// [`NodeEvent::Message`]. Ports 10–12 belong to the services.
    pub async fn set_vpn_handler(&self, net: NetworkId, port: u16) -> Result<(), NodeError> {
        self.call(move |s, _| s.vpn.set_handler(net, port, port).map_err(NodeError::from))
            .await?
    }

    /// Removes a handler set with [`Node::set_vpn_handler`]. Returns
    /// whether there was one.
    pub async fn remove_vpn_handler(&self, net: NetworkId, port: u16) -> Result<bool, NodeError> {
        self.call(move |s, _| s.remove_vpn_handler(net, port)).await
    }

    /// Takes `port` on the network, or a free port from 1000 up, and
    /// returns its messages and sends; see [`crate::vpn_port`].
    pub async fn open_vpn_port(
        &self,
        net: NetworkId,
        port: Option<u16>,
    ) -> Result<VpnPort, NodeError> {
        self.call(move |s, _| s.open_vpn_port(net, port)).await?
    }

    /// A free vpn port on the network.
    pub async fn reserve_vpn_port(&self, net: NetworkId) -> Result<u16, NodeError> {
        self.call(move |s, _| s.vpn.reserve_port(net).map_err(NodeError::from))
            .await?
    }

    /// Sends `body` to `dst`, encrypted.
    pub async fn send(
        &self,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        body: Vec<u8>,
    ) -> Result<(), NodeError> {
        self.send_with_flags(net, dst, dst_port, src_port, body, M_STD_FLAGS)
            .await
    }

    /// Sends `body` to `dst` with these message flags.
    pub async fn send_with_flags(
        &self,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        body: Vec<u8>,
        flags: u16,
    ) -> Result<(), NodeError> {
        self.call(move |s, now| {
            s.vpn
                .send_with_flags(now, net, dst, dst_port, src_port, &body, flags)
                .map_err(NodeError::from)
        })
        .await?
    }

    /// Broadcasts `body` in the clear.
    pub async fn broadcast(
        &self,
        net: NetworkId,
        dst_port: u16,
        src_port: u16,
        body: Vec<u8>,
    ) -> Result<(), NodeError> {
        self.call(move |s, now| {
            s.vpn
                .broadcast_with_flags(now, net, dst_port, src_port, &body, M_BROADCAST)
                .map_err(NodeError::from)
        })
        .await?
    }

    // --- vpn services -------------------------------------------------------

    /// Publishes `value` under (`key.public`, `salt`) until
    /// [`Node::hash_table_close`].
    pub async fn hash_table_set(
        &self,
        net: NetworkId,
        key: Key,
        salt: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<HashTablePublisherId, NodeError> {
        self.call(move |s, now| s.hash_table_set(now, net, &key, &salt, &value))
            .await?
    }

    /// Changes the value from the next publish on (every 10 minutes).
    pub async fn hash_table_update(
        &self,
        publisher: HashTablePublisherId,
        value: Vec<u8>,
    ) -> Result<(), NodeError> {
        self.call(move |s, _| {
            s.vpn
                .hash_table_update(publisher, &value)
                .map_err(NodeError::from)
        })
        .await?
    }

    pub async fn hash_table_close(&self, publisher: HashTablePublisherId) -> Result<(), NodeError> {
        self.call(move |s, now| {
            s.vpn
                .hash_table_close(now, publisher)
                .map_err(NodeError::from)
        })
        .await?
    }

    /// Looks up the value under (`key`, `salt`).
    pub async fn hash_table_get(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
        options: HashTableGetOptions,
    ) -> Result<HashTableGet, NodeError> {
        let (tx, rx) = mpsc::channel(RESULT_QUEUE);
        self.call(move |s, now| s.hash_table_get(now, net, &key, &salt, options, tx))
            .await??;
        Ok(HashTableGet { rx })
    }

    /// Publishes this host's `port` as the address of the service under
    /// (`key.public`, `salt`), until [`Node::hash_table_close`].
    pub async fn publish_vpn_address(
        &self,
        net: NetworkId,
        key: Key,
        salt: Vec<u8>,
        port: u16,
    ) -> Result<HashTablePublisherId, NodeError> {
        let value = HostAddr::new(self.host_id(), port).encode();
        self.hash_table_set(net, key, salt, value).await
    }

    /// The address of the service under (`key`, `salt`): the first value
    /// found that is a valid address, or `None` once the get closes
    /// without one.
    pub async fn resolve_vpn_address(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
        options: HashTableGetOptions,
    ) -> Result<Option<HostAddr>, NodeError> {
        let mut get = self.hash_table_get(net, key, salt, options).await?;
        while let Some(value) = get.next().await {
            if let Some(addr) = HostAddr::decode(&value) {
                return Ok(Some(addr));
            }
        }
        Ok(None)
    }

    /// Advertises that this host serves (`key`, `salt`) on `port`.
    pub async fn peer_index_publish(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
        port: u16,
    ) -> Result<PeerIndexPublisherId, NodeError> {
        self.call(move |s, now| s.peer_index_publish(now, net, &key, &salt, port))
            .await?
    }

    pub async fn peer_index_unpublish(
        &self,
        publisher: PeerIndexPublisherId,
    ) -> Result<(), NodeError> {
        self.call(move |s, now| {
            s.vpn
                .peer_index_unpublish(now, publisher)
                .map_err(NodeError::from)
        })
        .await?
    }

    /// Finds hosts that published (`key`, `salt`).
    pub async fn peer_index_search(
        &self,
        net: NetworkId,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<PeerIndexSearch, NodeError> {
        let (tx, rx) = mpsc::channel(RESULT_QUEUE);
        self.call(move |s, now| s.peer_index_search(now, net, &key, &salt, tx))
            .await??;
        Ok(PeerIndexSearch { rx })
    }

    /// Links to `host_id` through peer exchange: the two nodes trade link
    /// descriptions over the network (listener URIs, and WebRTC session
    /// descriptions when [`crate::WebRtcConfig`] is enabled on both), and
    /// one dials the other or they open a WebRTC data channel. Candidates
    /// are tried in the order TCP, WebSocket, WebRTC; a WebRTC connection
    /// that never opens closes with the exchange. The outcome arrives as
    /// [`NodeEvent::ExchangeClosed`]; this node's side of a WebRTC link it
    /// offered is up (`PeerAdded`) before `Connected`. Returns false when
    /// the host is this node, already a peer, or an exchange is in
    /// progress.
    pub async fn connect_peer(&self, net: NetworkId, host_id: ID) -> Result<bool, NodeError> {
        self.call(move |s, now| s.connect_peer(now, net, host_id))
            .await?
    }

    // --- swarms -------------------------------------------------------------

    /// Creates a swarm this node publishes, and its writer.
    pub async fn create_swarm(
        &self,
        options: WriterOptions,
    ) -> Result<(SwarmKey, SwarmId, SwarmWriter), NodeError> {
        let (reply_tx, reply) = mpsc::channel(1);
        let (swarm, id, publisher) = self
            .call(move |s, now| s.create_swarm(options, now, reply_tx))
            .await??;
        Ok((
            swarm,
            id,
            SwarmWriter {
                tx: self.tx.clone(),
                publisher,
                buf: Some(Vec::with_capacity(SWARM_IO_BUFFER)),
                reply,
            },
        ))
    }

    /// Adds a swarm to download and relay. `options` must match the
    /// publisher's (chunk size, integrity, stream count).
    pub async fn add_swarm(
        &self,
        id: SwarmId,
        options: SwarmOptions,
    ) -> Result<SwarmKey, NodeError> {
        self.call(move |s, _| s.add_swarm(id, options)).await?
    }

    /// Adds a swarm to download and relay, as [`Node::add_swarm`] does,
    /// starting from `cache`, which [`Node::export_cache`] gave. Returns
    /// whether the swarm took the cache; one that refused it starts empty.
    /// The swarm takes a cache of its own id and options whose epoch,
    /// signatures and data verify under its key. The node task hashes all
    /// of the cache's data to check it.
    pub async fn add_cached_swarm(
        &self,
        id: SwarmId,
        options: SwarmOptions,
        cache: Cache,
    ) -> Result<(SwarmKey, bool), NodeError> {
        self.call(move |s, _| s.add_cached_swarm(id, options, &cache))
            .await?
    }

    /// The swarm's stream as a cache: its data from offset 0, signatures
    /// and epoch. `None` for an unknown swarm, one that holds nothing, or
    /// one whose window has moved past offset 0.
    pub async fn export_cache(&self, swarm: SwarmKey) -> Result<Option<Cache>, NodeError> {
        self.call(move |s, _| s.export_cache(swarm)).await
    }

    /// Stops the swarm's channels; its readers and writers close.
    pub async fn remove_swarm(&self, swarm: SwarmKey) -> Result<(), NodeError> {
        self.call(move |s, _| s.remove_swarm(swarm)).await?
    }

    /// Runs the swarm with a peer: the peer's frames for it carry
    /// `channel`, ours carry `peer_channel`. Both sides call it with the
    /// pair reversed. `strims-transfer` negotiates the pair for published
    /// swarms; don't call it for a swarm a transfer runs (see
    /// [Bindings](crate#bindings)).
    pub async fn run_swarm(
        &self,
        host_id: ID,
        swarm: SwarmKey,
        channel: u64,
        peer_channel: u64,
    ) -> Result<(), NodeError> {
        self.call(move |s, now| s.run_swarm(&host_id, swarm, channel, peer_channel, now))
            .await?
    }

    /// Stops the swarm with a peer. Stopping a swarm that isn't running
    /// is not an error.
    pub async fn stop_swarm(&self, host_id: ID, swarm: SwarmKey) -> Result<(), NodeError> {
        self.call(move |s, _| s.stop_swarm(&host_id, swarm)).await?
    }

    /// A reader of the swarm's stream from its read offset.
    pub async fn open_reader(&self, swarm: SwarmKey) -> Result<SwarmReader, NodeError> {
        let (reply_tx, reply) = mpsc::channel(1);
        let reader = self
            .call(move |s, _| s.open_reader(swarm, reply_tx))
            .await??;
        Ok(SwarmReader {
            tx: self.tx.clone(),
            reader,
            buf: Some(vec![0; SWARM_IO_BUFFER]),
            reply,
        })
    }
}

/// Values of a hash table get, until the get closes (timeout, or answered
/// from the local copy).
#[derive(Debug)]
pub struct HashTableGet {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl HashTableGet {
    pub async fn next(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }
}

/// Hosts found by a peer index search, until it closes.
#[derive(Debug)]
pub struct PeerIndexSearch {
    rx: mpsc::Receiver<PeerIndexHost>,
}

impl PeerIndexSearch {
    pub async fn next(&mut self) -> Option<PeerIndexHost> {
        self.rx.recv().await
    }

    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<PeerIndexHost>> {
        self.rx.poll_recv(cx)
    }
}

/// Publishes into a swarm. Writes never block on peers: the live window
/// overwrites the oldest chunks.
#[derive(Debug)]
pub struct SwarmWriter {
    tx: mpsc::Sender<Command>,
    publisher: PublisherId,
    /// Handed to the node task with each call and returned with the reply.
    buf: Option<Vec<u8>>,
    reply: mpsc::Receiver<WriteReply>,
}

impl SwarmWriter {
    async fn op(&mut self, op: WriteOp, buf: Vec<u8>) -> Result<u64, NodeError> {
        let cmd = Command::Write {
            publisher: self.publisher,
            op,
            buf,
        };
        self.tx.send(cmd).await.map_err(|_| NodeError::Stopped)?;
        let r = self.reply.recv().await.ok_or(NodeError::Closed)?;
        self.buf = Some(r.buf);
        r.result
    }

    /// Appends `data` to the stream; complete integrity units are signed
    /// and published.
    pub async fn write(&mut self, data: &[u8]) -> Result<(), NodeError> {
        for chunk in data.chunks(SWARM_IO_BUFFER) {
            let mut buf = self.buf.take().ok_or(NodeError::Closed)?;
            buf.clear();
            buf.extend_from_slice(chunk);
            self.op(WriteOp::Write(chunk.len()), buf).await?;
        }
        Ok(())
    }

    /// Publishes the partial unit, zero-padded (`strims_ppspp::Writer::flush`),
    /// and returns the stream offset at which the next unit starts.
    pub async fn flush(&mut self) -> Result<u64, NodeError> {
        let buf = self.buf.take().ok_or(NodeError::Closed)?;
        self.op(WriteOp::Flush, buf).await
    }

    /// Starts a new epoch.
    pub async fn reset(&mut self) -> Result<(), NodeError> {
        let buf = self.buf.take().ok_or(NodeError::Closed)?;
        self.op(WriteOp::Reset, buf).await.map(|_| ())
    }
}

impl Drop for SwarmWriter {
    fn drop(&mut self) {
        let _ = self.tx.try_send(Command::ClosePublisher(self.publisher));
    }
}

/// Reads a swarm's stream: the non-blocking `BufferReader` in the node
/// task, awaited here.
#[derive(Debug)]
pub struct SwarmReader {
    tx: mpsc::Sender<Command>,
    reader: ReaderId,
    buf: Option<Vec<u8>>,
    reply: mpsc::Receiver<ReadReply>,
}

impl SwarmReader {
    /// Reads at least one byte into `out` (at most 64 KiB), waiting for
    /// data. Returns the stream offset of `out[0]` and the bytes read.
    ///
    /// Each read starts where the last one ended, except across a skip.
    /// After [`ReadError::Underrun`] the next read starts `skipped` bytes
    /// on; when another reader of the swarm recovered from the underrun
    /// first, the skip shows only as a gap between offsets. After
    /// [`ReadError::Reset`] the reads are of the new stream. An empty `out`
    /// waits for data the same way and returns its offset without reading
    /// it.
    pub async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadError> {
        let buf = self.buf.take().ok_or(ReadError::Closed)?;
        let max = out.len().min(buf.len());
        let cmd = Command::Read {
            reader: self.reader,
            buf,
            max,
        };
        self.tx.send(cmd).await.map_err(|_| ReadError::Stopped)?;
        let r = self.reply.recv().await.ok_or(ReadError::Stopped)?;
        // The buffer comes back with errors too: underruns and resets don't
        // end the reader.
        let buf = self.buf.insert(r.buf);
        let (offset, n) = r.result?;
        let src = buf.get(..n).unwrap_or_default();
        if let Some(dst) = out.get_mut(..src.len()) {
            dst.copy_from_slice(src);
        }
        Ok((offset, n))
    }

    /// Reads at least one byte into `out` (at most 64 KiB), waiting for
    /// data. An empty `out` reads nothing.
    pub async fn read(&mut self, out: &mut [u8]) -> Result<usize, ReadError> {
        if out.is_empty() {
            return Ok(0);
        }
        Ok(self.read_at(out).await?.1)
    }

    /// Reads until `out` is full. Underruns and resets are errors.
    pub async fn read_exact(&mut self, out: &mut [u8]) -> Result<(), ReadError> {
        let mut off = 0;
        while let Some(rest) = out.get_mut(off..)
            && !rest.is_empty()
        {
            off += self.read_at(rest).await?.1;
        }
        Ok(())
    }
}

impl Drop for SwarmReader {
    fn drop(&mut self) {
        let _ = self.tx.try_send(Command::CloseReader(self.reader));
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

    async fn flush(&mut self) -> Result<u64, NodeError> {
        SwarmWriter::flush(self).await
    }
}
