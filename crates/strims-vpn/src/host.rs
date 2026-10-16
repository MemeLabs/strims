// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A [`Host`] holds every network this node is in, the host-wide dedup
//! table and hash table store, and the local services (hash table, peer
//! index and peer exchange on ports 10, 11 and 12 of every network). It is
//! a sans-IO state machine composed with a [`strims_vnic::Host`] by the
//! driver:
//!
//! - **Topology** calls take the vnic host: [`Host::new`],
//!   [`Host::add_network`], [`Host::remove_network`], [`Host::add_peer`],
//!   [`Host::remove_peer`]. They set handlers, open writers and create QoS
//!   classes on it.
//! - **Ingress.** The driver passes each frame vnic dispatches to a vpn
//!   link tag to [`Host::handle_frame`]. It doesn't take the vnic host,
//!   because vnic lends the frame while it is borrowed.
//! - **Egress.** Sending queues marshaled messages on bounded per-link
//!   queues. [`Host::flush`] sets the backlog of links with queued messages
//!   on vnic, and the driver's `poll_egress` callback hands each vpn send
//!   opportunity to [`Host::fill_egress`].
//! - **Deliveries and service results** come out of [`Host::poll_event`].
//! - **Timers** (dedup pruning, record expiry, republishing, request and
//!   exchange timeouts) are reported by [`Host::poll_timeout`] and run in
//!   [`Host::handle_timeout`].

use std::collections::{BTreeMap, VecDeque};

use slotmap::SlotMap;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::ed25519_dalek::{SigningKey, VerifyingKey};
use strims_kademlia::ID;
use strims_proto::types::Key;
use strims_vnic::{Fill, PeerKey, ports};

use crate::buf::{BufArena, BufPool};
use crate::cache::BoundedMap;
use crate::hash_table::{HashTableService, HashTableStore};
use crate::message::{
    M_BROADCAST, M_COMPRESS, M_ENCRYPT, M_NORELAY, M_STD_FLAGS, MAX_MESSAGE_HOPS, MAX_TRAILER_LEN,
    MESSAGE_HEADER_LEN, MessageHeader, MessageId, MessageView, TRAILER_ENTRY_LEN, encode_message,
    encode_relay,
};
use crate::message_filter::{CIPHER_OVERHEAD, MessageCipher, NONCE_LEN};
use crate::message_id_lru::MessageIdLru;
use crate::network::{Backlog, Handler, Link, Network, NextHopMap};
use crate::peer_exchange::{ExchangeOutcome, PeerExchangeService};
use crate::peer_index::{PeerIndexHost, PeerIndexService, PeerIndexStore};
use crate::topology::Topology;
use crate::{
    Error, ExchangeId, HashTableGetId, LinkDescription, NetworkId, PeerIndexSearchId, VnicHost,
};

/// Reference dedup table size. [`Host`] uses [`Config::recent_message_ids`].
// TODO: unused; remove at the next API break.
pub const RECENT_MESSAGE_ID_HISTORY_DEFAULT_SIZE: usize = 1024;
/// How long a message id is remembered for dedup.
pub const RECENT_MESSAGE_ID_HISTORY_TTL: Duration = Duration::from_secs(30);
/// Weight of the vpn QoS class under the vnic root, and of each network's
/// class under it.
pub const QOS_CLASS_WEIGHT: u64 = 1;
/// How often learned link graphs drop expired links, at most.
const TOPOLOGY_PRUNE_INTERVAL: Duration = Duration::from_secs(30);
/// How long after a link comes up this host pushes its records to it
/// ([`Host::flush`]), so that links that come up together share a push.
pub const RECORD_PUSH_DELAY: Duration = Duration::SECOND;
/// The least time between two record pushes in a network: a peer that
/// keeps reconnecting gets this host's records at most this often.
pub const RECORD_PUSH_INTERVAL: Duration = Duration::from_secs(10);

/// The largest frame body of every standard link (TCP, MTU 65535).
pub const DEFAULT_MAX_MESSAGE_LEN: usize =
    strims_vnic::tcp::TCP_MTU - strims_vnic::FRAME_HEADER_LEN;

/// The largest encrypted body that fits [`DEFAULT_MAX_MESSAGE_LEN`] with
/// [`MAX_MESSAGE_HOPS`] trailer entries, so every relay can forward it.
pub const MAX_RELAYED_BODY_LEN: usize =
    DEFAULT_MAX_MESSAGE_LEN - MESSAGE_HEADER_LEN - MAX_TRAILER_LEN - CIPHER_OVERHEAD;

/// What to do with the trailer signatures of received messages.
///
/// Unverified, any host can forge a message's sender, tamper with the
/// messages it relays, and feed forged trailer entries into routing state
/// and the dedup table. A message with more than
/// [`MAX_MESSAGE_HOPS`] entries fails
/// verification unchecked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerifyMode {
    /// Don't verify.
    #[default]
    Off,
    /// Verify every hop and count failures in [`Metrics::verify_failures`],
    /// but process the message anyway. [`Delivery::verified`] reports the
    /// result.
    Audit,
    /// Drop messages that don't verify before they touch next-hop hints or
    /// the dedup table.
    Enforce,
}

/// How this host picks the links a message goes to. Both modes keep the
/// wire format, send a message to at most `MAX_MESSAGE_REPLICAS - hops`
/// links, and route broadcasts alike; [`crate::network`] has the details.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Routing {
    /// The routed links closest to the destination by XOR distance: a
    /// direct link alone, else a learned next hop first. Links are the
    /// app's choice and unrelated to XOR distance, so on sparse link graphs
    /// this is a bounded flood that often dead-ends.
    Greedy,
    /// Unicasts follow the shortest paths of a bounded link graph learned
    /// from trailers and link announcements, and take the greedy fan-out,
    /// best-connected links first, towards destinations with no known
    /// path. A unicast for a neighbour goes over the direct link alone
    /// unless that link is backlogged. A unicast copy that arrives on a
    /// shorter path than every earlier one is relayed again.
    #[default]
    Learned,
}

/// Host options.
#[derive(Clone, Debug)]
pub struct Config {
    pub verify: VerifyMode,
    pub routing: Routing,
    /// [`Routing::Learned`]: links a unicast with a known path goes to at
    /// each hop, never more than `MAX_MESSAGE_REPLICAS - hops`. A direct
    /// link to the destination gets the only copy unless it is
    /// backlogged (its queue at least half full, or a copy dropped on it
    /// in the last 10 s); then it gets a backup.
    pub route_copies: usize,
    /// [`Routing::Learned`]: hosts in the learned link graph, per network.
    pub topology_max_nodes: usize,
    /// [`Routing::Learned`]: learned links kept per host.
    pub topology_max_degree: usize,
    /// [`Routing::Learned`]: how long a learned link is used after a
    /// trailer last showed it.
    pub topology_ttl: Duration,
    /// [`Routing::Learned`]: how far link announcements go. 1: to this
    /// host's links only ([`M_NORELAY`]). 2: also one link past each
    /// neighbour that doesn't announce: the announcement is an ordinary
    /// broadcast padded to four trailer entries, which the relay rule
    /// sends on to exactly one link. Larger values count as 2, since less
    /// padding lets every relay fan out.
    pub announce_reach: usize,
    /// The largest message, trailer included, this host sends or relays.
    /// A message must fit in one frame (link MTU - 4); larger messages are
    /// refused by `send` and dropped on links with a smaller MTU.
    pub max_message_len: usize,
    /// Messages queued per link before new ones are dropped.
    pub link_queue_messages: usize,
    /// Bytes queued per link before new messages are dropped.
    pub link_queue_bytes: usize,
    /// Dedup table bound.
    pub recent_message_ids: usize,
    /// How long a message id is remembered.
    pub recent_message_id_ttl: Duration,
    /// [`Routing::Greedy`]: next-hop hints per network.
    pub next_hop_capacity: usize,
    /// [`Routing::Greedy`]: next-hop hint lifetime.
    pub next_hop_ttl: Duration,
    /// Deliveries waiting in the event queue before new ones are dropped.
    pub max_queued_deliveries: usize,
    /// Stored hash table records, host-wide.
    pub hash_table_max_records: usize,
    /// Host-wide byte bound on stored hash table records.
    pub hash_table_max_bytes: usize,
    /// Stored peer index records, per network.
    pub peer_index_max_records: usize,
    /// Incoming peer exchanges in progress, host-wide.
    pub max_incoming_exchanges: usize,
    /// Link offers waiting for `flush`; more are dropped.
    pub max_pending_offers: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            verify: VerifyMode::Off,
            routing: Routing::default(),
            route_copies: 2,
            topology_max_nodes: 512,
            topology_max_degree: 16,
            topology_ttl: Duration::from_secs(5 * 60),
            announce_reach: 2,
            max_message_len: DEFAULT_MAX_MESSAGE_LEN,
            link_queue_messages: 64,
            link_queue_bytes: 256 * 1024,
            recent_message_ids: 32 * 1024,
            recent_message_id_ttl: RECENT_MESSAGE_ID_HISTORY_TTL,
            next_hop_capacity: 4096,
            next_hop_ttl: Duration::from_secs(5 * 60),
            max_queued_deliveries: 1024,
            hash_table_max_records: crate::hash_table::HASH_TABLE_MAX_SIZE,
            hash_table_max_bytes: 8 * 1024 * 1024,
            peer_index_max_records: crate::peer_index::PEER_INDEX_MAX_SIZE,
            max_incoming_exchanges: 8,
            max_pending_offers: 16,
        }
    }
}

/// Counters for the driver to export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Messages this host originated (link announcements aside).
    pub messages_sent: u64,
    /// Messages this host relayed.
    pub messages_relayed: u64,
    /// Frames passed to `handle_frame`.
    pub frames_received: u64,
    /// Message copies queued on links.
    pub frames_queued: u64,
    /// Message copies handed to vnic by `fill_egress`.
    pub frames_sent: u64,
    /// Messages delivered to local handlers (services and apps; link
    /// announcements aside).
    pub messages_delivered: u64,
    pub duplicates: u64,
    pub malformed: u64,
    /// The last trailer entry isn't the link's host.
    pub trailer_mismatch: u64,
    /// Received with `M_COMPRESS`.
    pub compressed_dropped: u64,
    pub verify_failures: u64,
    /// Dropped by [`VerifyMode::Enforce`].
    pub unverified_dropped: u64,
    /// Not relayed: the trailer is full (or, with [`Routing::Learned`],
    /// would be full at a next host that isn't the destination).
    pub hop_limit: u64,
    pub link_queue_full: u64,
    /// Larger than a link's frames.
    pub too_large: u64,
    pub no_handler: u64,
    pub decrypt_failures: u64,
    /// Service handlers that rejected a message.
    pub handler_errors: u64,
    /// Deliveries dropped on a full event queue.
    pub deliveries_dropped: u64,
    /// Dedup entries evicted before their ttl by the size bound.
    pub dedup_evictions: u64,
    /// [`Routing::Learned`]: unicast duplicates relayed again because
    /// they took fewer hops than every earlier copy.
    pub shorter_relays: u64,
    /// [`Routing::Learned`]: unicasts sent or relayed over a learned path
    /// (or a direct link), and with the greedy fan-out for lack of one.
    pub learned_routes: u64,
    pub fallback_routes: u64,
    /// [`Routing::Learned`]: link announcements sent (one message, queued
    /// on every link) and used.
    pub link_announcements_sent: u64,
    pub link_announcements_received: u64,
    /// Peer index records pushed to new links, and hash table records
    /// published again once the first links came up (see
    /// [`RECORD_PUSH_DELAY`]); messages, not frames.
    pub records_pushed: u64,
}

/// A message for a local app port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery<H> {
    pub network: NetworkId,
    /// The handler set for `dst_port`.
    pub handler: H,
    /// The sender: trailer entry 0.
    pub src: ID,
    pub src_port: u16,
    pub dst: ID,
    pub dst_port: u16,
    pub flags: u16,
    /// Trailer entries (0 for this host's own messages).
    pub hops: usize,
    /// Whether every trailer signature verified; `None` under
    /// [`VerifyMode::Off`].
    pub verified: Option<bool>,
    /// Decrypted. Return it with [`Host::recycle`] to avoid allocations.
    pub body: Vec<u8>,
    /// When this host received it.
    pub received: Time,
}

/// Notifications for the driver and the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event<H> {
    /// A message for an app handler.
    Message(Delivery<H>),
    /// A hash table value for a get.
    HashTableValue {
        network: NetworkId,
        get: HashTableGetId,
        value: Vec<u8>,
    },
    /// The get finished (timeout, cancel, network removed, or answered from
    /// the cache).
    HashTableGetClosed {
        network: NetworkId,
        get: HashTableGetId,
    },
    /// A host holding the searched key.
    PeerIndexHost {
        network: NetworkId,
        search: PeerIndexSearchId,
        host: PeerIndexHost,
    },
    PeerIndexSearchClosed {
        network: NetworkId,
        search: PeerIndexSearchId,
    },
    /// Peer exchange: create this exchange's link candidates (a
    /// `LinkCandidatePool`) if it has none, gather their local descriptions and report them with
    /// [`Host::set_local_descriptions`].
    GatherLocalDescriptions {
        network: NetworkId,
        exchange: ExchangeId,
        host_id: ID,
    },
    /// Peer exchange: create the candidates if needed, try to connect with
    /// the remote descriptions and report with [`Host::set_remote_descriptions_result`].
    DialRemoteDescriptions {
        network: NetworkId,
        exchange: ExchangeId,
        host_id: ID,
        descriptions: Vec<LinkDescription>,
    },
    /// The exchange is over: drop its candidates. A link it made stays.
    ExchangeClosed {
        network: NetworkId,
        exchange: ExchangeId,
        host_id: ID,
        outcome: ExchangeOutcome,
    },
}

/// A message arriving at a local port.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Inbound<'a> {
    pub(crate) header: MessageHeader,
    pub(crate) src: ID,
    pub(crate) hops: usize,
    /// Leading trailer entries written by `src` (1 for an ordinary
    /// message, 0 for this host's own). Link announcements carry several
    /// to limit their reach.
    pub(crate) lead: usize,
    pub(crate) verified: Option<bool>,
    /// Plaintext.
    pub(crate) body: &'a [u8],
}

/// The vpn host and the networks it is in.
#[derive(Debug)]
pub struct Host<H: Copy = ()> {
    pub(crate) id: ID,
    /// The vnic host key: signs trailers and peer index records, and keys
    /// message ciphers.
    pub(crate) key: Key,
    pub(crate) signer: SigningKey,
    pub(crate) config: Config,
    pub(crate) rng: DetRng,
    qos_class: strims_qos::ClassId,
    pub(crate) networks: SlotMap<NetworkId, Network<H>>,
    by_key: BTreeMap<Vec<u8>, NetworkId>,
    next_store_id: u32,
    /// Keyed by (network store id, message id).
    recent: MessageIdLru<(u32, MessageId)>,
    pub(crate) hash_table: HashTableService,
    pub(crate) peer_index: PeerIndexService,
    pub(crate) peer_exchange: PeerExchangeService,
    bufs: BufArena,
    pub(crate) pool: BufPool,
    ciphers: BoundedMap<ID, MessageCipher>,
    verifiers: BoundedMap<ID, Option<VerifyingKey>>,
    /// Deliveries are bounded by `max_queued_deliveries`; the rest by local
    /// requests and exchanges.
    events: VecDeque<Event<H>>,
    queued_deliveries: usize,
    pub(crate) metrics: Metrics,
    /// When learned graphs are next pruned of expired links.
    next_topology_prune: Time,
}

impl<H: Copy> Host<H> {
    /// Creates the vpn host for a vnic host, with its QoS class under the
    /// vnic root.
    pub fn new<T: Copy>(
        vnic: &mut VnicHost<T>,
        config: Config,
        rng: DetRng,
    ) -> Result<Host<H>, Error> {
        let key = vnic.key().clone();
        let signer = strims_crypto::signing_key(&key)?;
        let qos_class = vnic.add_qos_class(vnic.qos_root(), QOS_CLASS_WEIGHT)?;
        let id = vnic.id();
        Ok(Host {
            id,
            hash_table: HashTableService::new(HashTableStore::new(
                id,
                config.hash_table_max_records,
                config.hash_table_max_bytes,
            )),
            peer_index: PeerIndexService::default(),
            peer_exchange: PeerExchangeService::default(),
            recent: MessageIdLru::new(config.recent_message_ids, config.recent_message_id_ttl),
            key,
            signer,
            config,
            rng,
            qos_class,
            networks: SlotMap::with_key(),
            by_key: BTreeMap::new(),
            next_store_id: 0,
            bufs: BufArena::default(),
            pool: BufPool::new(32, 64 * 1024),
            ciphers: BoundedMap::new(64),
            verifiers: BoundedMap::new(256),
            events: VecDeque::new(),
            queued_deliveries: 0,
            metrics: Metrics::default(),
            next_topology_prune: Time::NIL,
        })
    }

    /// The vnic host id.
    pub fn id(&self) -> ID {
        self.id
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn metrics(&self) -> Metrics {
        Metrics {
            dedup_evictions: self.recent.evictions(),
            ..self.metrics
        }
    }

    // --- networks -----------------------------------------------------------

    /// Joins a network: a QoS class under the vpn class and the services on
    /// ports 10 to 12 and 20.
    pub fn add_network<T: Copy>(
        &mut self,
        vnic: &mut VnicHost<T>,
        key: &[u8],
    ) -> Result<NetworkId, Error> {
        if self.by_key.contains_key(key) {
            return Err(Error::DuplicateNetworkKey);
        }
        let class = vnic.add_qos_class(self.qos_class, QOS_CLASS_WEIGHT)?;
        self.next_store_id = self
            .next_store_id
            .checked_add(1)
            .ok_or(Error::NetworkIdBounds)?;
        let net = self.networks.insert(Network::new(
            key.to_vec(),
            self.next_store_id,
            class,
            self.id,
            NextHopMap::new(self.config.next_hop_capacity, self.config.next_hop_ttl),
            Topology::new(
                self.id,
                self.config.topology_max_nodes,
                self.config.topology_max_degree,
                self.config.topology_ttl,
            ),
            PeerIndexStore::new(self.config.peer_index_max_records),
        ));
        self.by_key.insert(key.to_vec(), net);
        Ok(net)
    }

    /// Leaves a network: detaches its links, removes its class, drops its
    /// records and ends its requests and exchanges (with closed events).
    pub fn remove_network<T: Copy>(
        &mut self,
        vnic: &mut VnicHost<T>,
        net: NetworkId,
    ) -> Result<(), Error> {
        let mut network = self.networks.remove(net).ok_or(Error::NetworkNotFound)?;
        self.by_key.remove(&network.key);
        let peers: Vec<PeerKey> = network.links.keys().copied().collect();
        for peer in peers {
            if let Some(link) = network.remove_link(peer) {
                vnic.remove_handler(peer, link.src_port);
                let _ = vnic.close_writer(link.writer);
                self.release_queue(link);
            }
        }
        let _ = vnic.remove_qos_class(network.class);
        self.hash_table.store.remove_store(network.store_id);
        self.hash_table_network_removed(net);
        self.peer_index_network_removed(net);
        self.peer_exchange_network_removed(net);
        Ok(())
    }

    /// The network with this key.
    pub fn network(&self, key: &[u8]) -> Option<NetworkId> {
        self.by_key.get(key).copied()
    }

    /// The network's key.
    pub fn network_key(&self, net: NetworkId) -> Option<&[u8]> {
        self.networks.get(net).map(|n| n.key.as_slice())
    }

    /// Every network, in key order.
    pub fn networks(&self) -> impl Iterator<Item = (NetworkId, &[u8])> + '_ {
        self.by_key.iter().map(|(k, n)| (*n, k.as_slice()))
    }

    // --- links --------------------------------------------------------------

    /// Adds a vnic peer to the network: frames arriving on `src_port` are
    /// tagged `tag` (the driver hands them to [`Host::handle_frame`]), and
    /// a writer under the network's class sends to `dst_port`. Adding a
    /// host that is already a link is a no-op, and a removed host can be
    /// added again.
    ///
    /// A link that doesn't fit its routing-table bucket (20 per bucket) is
    /// still read but not routed through until a bucket entry is removed.
    pub fn add_peer<T: Copy>(
        &mut self,
        vnic: &mut VnicHost<T>,
        net: NetworkId,
        peer: PeerKey,
        src_port: u16,
        dst_port: u16,
        tag: T,
    ) -> Result<(), Error> {
        let host_id = vnic.peer(peer).ok_or(Error::UnknownPeer)?.host_id();
        let network = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
        if let Some(&old) = network.by_host.get(&host_id) {
            if old == peer {
                return Ok(());
            }
            // A stale link: vnic replaced the peer and its handlers are gone.
            if let Some(link) = network.remove_link(old) {
                self.release_queue(link);
            }
        }
        let class = self
            .networks
            .get(net)
            .map(|n| n.class)
            .ok_or(Error::NetworkNotFound)?;
        vnic.set_handler(peer, src_port, tag)?;
        let writer = match vnic.open_writer(peer, dst_port, class, tag) {
            Ok(w) => w,
            Err(e) => {
                vnic.remove_handler(peer, src_port);
                return Err(e.into());
            }
        };
        let max_body = match vnic.writer_max_body(writer) {
            Ok(m) => m.min(self.config.max_message_len),
            Err(e) => {
                let _ = vnic.close_writer(writer);
                vnic.remove_handler(peer, src_port);
                return Err(e.into());
            }
        };
        let network = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
        network.insert_link(
            peer,
            Link {
                host_id,
                src_port,
                writer,
                max_body,
                queue: VecDeque::new(),
                queued_bytes: 0,
                routed: false,
                dropped_at: Time::NIL,
            },
        );
        Ok(())
    }

    /// Removes a host's link from the network: its handler, writer and
    /// queue.
    pub fn remove_peer<T: Copy>(
        &mut self,
        vnic: &mut VnicHost<T>,
        net: NetworkId,
        host_id: &ID,
    ) -> bool {
        let Some(network) = self.networks.get_mut(net) else {
            return false;
        };
        let Some(&peer) = network.by_host.get(host_id) else {
            return false;
        };
        let Some(link) = network.remove_link(peer) else {
            return false;
        };
        vnic.remove_handler(peer, link.src_port);
        let _ = vnic.close_writer(link.writer);
        self.release_queue(link);
        true
    }

    /// vnic reported `Event::PeerRemoved`: forget the peer's links (vnic
    /// already dropped their handlers and writers).
    pub fn handle_peer_removed(&mut self, peer: PeerKey) {
        let nets: Vec<NetworkId> = self.networks.keys().collect();
        for net in nets {
            if let Some(link) = self.networks.get_mut(net).and_then(|n| n.remove_link(peer)) {
                self.release_queue(link);
            }
        }
    }

    /// Hosts in the network's learned link graph ([`Routing::Learned`]).
    pub fn topology_len(&self, net: NetworkId) -> usize {
        self.networks.get(net).map_or(0, |n| n.topology.len())
    }

    /// Whether the host is a routed link.
    pub fn has_peer(&self, net: NetworkId, host_id: &ID) -> bool {
        self.networks.get(net).is_some_and(|n| n.has_peer(host_id))
    }

    /// The network's links: (vnic peer, host id, routed).
    pub fn links(&self, net: NetworkId) -> impl Iterator<Item = (PeerKey, ID, bool)> + '_ {
        self.networks
            .get(net)
            .into_iter()
            .flat_map(|n| n.links.iter().map(|(k, l)| (*k, l.host_id, l.routed)))
    }

    fn release_queue(&mut self, link: Link) {
        for id in link.queue {
            if let Some(b) = self.bufs.release(id) {
                self.pool.put(b);
            }
        }
    }

    // --- ports ----------------------------------------------------------------

    /// Delivers messages for `port` as [`Event::Message`]s tagged `handler`.
    /// Fails with [`Error::PortInUse`] if the port has a handler.
    pub fn set_handler(&mut self, net: NetworkId, port: u16, handler: H) -> Result<(), Error> {
        let n = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
        if n.handlers.contains_key(&port) {
            return Err(Error::PortInUse(port));
        }
        n.handlers.insert(port, Handler::App(handler));
        n.reservations.insert(port);
        Ok(())
    }

    /// Removes an app handler and releases its port. Service ports keep
    /// their handlers.
    pub fn remove_handler(&mut self, net: NetworkId, port: u16) -> Option<H> {
        let n = self.networks.get_mut(net)?;
        match n.handlers.get(&port) {
            Some(Handler::App(h)) => {
                let h = *h;
                n.handlers.remove(&port);
                n.reservations.remove(&port);
                Some(h)
            }
            _ => None,
        }
    }

    /// A random free port at or above 1000, now reserved, or
    /// [`Error::NoFreePorts`].
    pub fn reserve_port(&mut self, net: NetworkId) -> Result<u16, Error> {
        let Host { networks, rng, .. } = self;
        let n = networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
        let span = u32::from(u16::MAX - ports::RESERVED_PORT_COUNT);
        let free = |p: u16| !n.reservations.contains(&p);
        for _ in 0..64 {
            let port = random_below(rng, span) as u16 + ports::RESERVED_PORT_COUNT;
            if free(port) {
                n.reservations.insert(port);
                return Ok(port);
            }
        }
        let start = random_below(rng, span);
        for i in 0..span {
            let port = ((start + i) % span) as u16 + ports::RESERVED_PORT_COUNT;
            if free(port) {
                n.reservations.insert(port);
                return Ok(port);
            }
        }
        Err(Error::NoFreePorts)
    }

    /// Releases a reserved port that has no handler.
    pub fn release_port(&mut self, net: NetworkId, port: u16) {
        if let Some(n) = self.networks.get_mut(net)
            && !n.handlers.contains_key(&port)
        {
            n.reservations.remove(&port);
        }
    }

    // --- sending ----------------------------------------------------------

    /// Sends `body` to `dst`'s `dst_port` from `src_port`, encrypted.
    pub fn send(
        &mut self,
        now: Time,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        body: &[u8],
    ) -> Result<(), Error> {
        self.send_with_flags(now, net, dst, dst_port, src_port, body, M_STD_FLAGS)
    }

    /// Broadcasts `body` to `dst_port` on every host it reaches, in the
    /// clear.
    pub fn broadcast(
        &mut self,
        now: Time,
        net: NetworkId,
        dst_port: u16,
        src_port: u16,
        body: &[u8],
    ) -> Result<(), Error> {
        self.broadcast_with_flags(now, net, dst_port, src_port, body, M_BROADCAST)
    }

    /// Broadcasts towards this host's own id, with [`M_BROADCAST`] added to
    /// `flags`.
    pub fn broadcast_with_flags(
        &mut self,
        now: Time,
        net: NetworkId,
        dst_port: u16,
        src_port: u16,
        body: &[u8],
        flags: u16,
    ) -> Result<(), Error> {
        let id = self.id;
        self.send_with_flags(now, net, id, dst_port, src_port, body, flags | M_BROADCAST)
    }

    /// Sends with explicit flags.
    ///
    /// A broadcast is also delivered to this host's own handler (with 0
    /// hops), and a message to this host's id only to it. The
    /// message is queued on up to five links; [`Host::flush`] then sets
    /// their backlogs. With [`Routing::Learned`], a broadcast flagged
    /// [`M_NORELAY`] goes to every routed link.
    #[allow(clippy::too_many_arguments)]
    pub fn send_with_flags(
        &mut self,
        now: Time,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        body: &[u8],
        flags: u16,
    ) -> Result<(), Error> {
        self.send_message(now, net, dst, dst_port, src_port, body, flags, true)
            .map(|_| ())
    }

    /// [`Host::send_with_flags`]; `count` is false for this crate's own
    /// routing traffic, which has its own counters. Returns the number of
    /// copies queued on links.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_message(
        &mut self,
        now: Time,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        body: &[u8],
        flags: u16,
        count: bool,
    ) -> Result<usize, Error> {
        let header = self.new_header(net, dst, dst_port, src_port, body.len(), flags, 1)?;

        let broadcast = flags & M_BROADCAST != 0;
        if broadcast || dst == self.id {
            let inbound = Inbound {
                header,
                src: self.id,
                hops: 0,
                lead: 0,
                verified: (self.config.verify != VerifyMode::Off).then_some(true),
                body,
            };
            self.call_handler(now, net, &inbound, None)?;
            if !broadcast {
                return Ok(0);
            }
        }

        let wire = self.encode_new(now, &header, body)?;
        if count {
            self.metrics.messages_sent += 1;
        }
        if broadcast && flags & M_NORELAY != 0 && self.config.routing == Routing::Learned {
            // Every neighbour must get it: each neighbour of a peer index
            // publisher stores its record.
            let peers = self.neighbours(now, net);
            Ok(self.enqueue(now, net, wire, &peers))
        } else {
            Ok(self.route(now, net, wire, &dst, !broadcast, &[]))
        }
    }

    /// Sends a message for the neighbours (an [`M_BROADCAST`] |
    /// [`M_NORELAY`] message from this host, unencrypted) to the links
    /// `peers` only, without delivering it here. Returns the copies
    /// queued.
    pub(crate) fn send_to_links(
        &mut self,
        now: Time,
        net: NetworkId,
        port: u16,
        body: &[u8],
        peers: &[PeerKey],
    ) -> Result<usize, Error> {
        let flags = M_BROADCAST | M_NORELAY;
        let header = self.new_header(net, self.id, port, port, body.len(), flags, 1)?;
        let wire = self.encode_new(now, &header, body)?;
        self.metrics.messages_sent += 1;
        Ok(self.enqueue(now, net, wire, peers))
    }

    /// The links a message for the neighbours ([`M_NORELAY`] broadcast)
    /// goes to: every routed link with [`Routing::Learned`], the five
    /// closest to this host's id with [`Routing::Greedy`].
    pub(crate) fn neighbours(&self, now: Time, net: NetworkId) -> Vec<PeerKey> {
        let Some(n) = self.networks.get(net) else {
            return Vec::new();
        };
        match self.config.routing {
            Routing::Learned => n.routes.iter().map(|r| r.peer).collect(),
            Routing::Greedy => n
                .select_greedy(now, &self.id, false, 0, |_| false)
                .iter()
                .map(|r| r.peer)
                .collect(),
        }
    }

    /// A header for a new message, with the network's next sequence
    /// number, after checking that the message fits a frame with
    /// `entries` trailer entries.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_header(
        &mut self,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        body_len: usize,
        flags: u16,
        entries: usize,
    ) -> Result<MessageHeader, Error> {
        if flags & M_COMPRESS != 0 {
            return Err(Error::CompressionUnsupported);
        }
        let encrypt = flags & M_ENCRYPT != 0;
        let body_len = body_len + if encrypt { CIPHER_OVERHEAD } else { 0 };
        let total = MESSAGE_HEADER_LEN + body_len + entries * TRAILER_ENTRY_LEN;
        if total > self.config.max_message_len || body_len > usize::from(u16::MAX) {
            return Err(Error::MessageTooLarge(total));
        }
        let n = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
        n.seq = n.seq.wrapping_add(1);
        Ok(MessageHeader {
            dst_id: dst,
            dst_port,
            src_port,
            // The wire carries the low 16 bits.
            seq: n.seq as u16,
            // invariant: checked above.
            length: body_len as u16,
            flags,
        })
    }

    /// Marshals a new message signed by this host (sealed for its
    /// destination when the header says so) into a pooled buffer.
    pub(crate) fn encode_new(
        &mut self,
        now: Time,
        header: &MessageHeader,
        body: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let mut wire = self.pool.take();
        let res = if header.flags & M_ENCRYPT != 0 {
            let mut sealed = self.pool.take();
            let r = self.seal(&header.dst_id, body, &mut sealed).and_then(|()| {
                encode_message(&mut wire, header, &sealed, now, &self.id, &self.signer)
                    .map_err(Error::from)
            });
            self.pool.put(sealed);
            r
        } else {
            encode_message(&mut wire, header, body, now, &self.id, &self.signer)
                .map_err(Error::from)
        };
        match res {
            Ok(()) => Ok(wire),
            Err(e) => {
                self.pool.put(wire);
                Err(e)
            }
        }
    }

    /// [`Host::send`] of a protobuf message.
    pub fn send_proto<M: prost::Message>(
        &mut self,
        now: Time,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        msg: &M,
    ) -> Result<(), Error> {
        self.send_proto_with_flags(now, net, dst, dst_port, src_port, msg, M_STD_FLAGS)
    }

    /// [`Host::send_with_flags`] of a protobuf message.
    #[allow(clippy::too_many_arguments)]
    pub fn send_proto_with_flags<M: prost::Message>(
        &mut self,
        now: Time,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        msg: &M,
        flags: u16,
    ) -> Result<(), Error> {
        self.send_proto_message(now, net, dst, dst_port, src_port, msg, flags)
            .map(|_| ())
    }

    /// [`Host::send_proto_with_flags`], returning the copies queued.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_proto_message<M: prost::Message>(
        &mut self,
        now: Time,
        net: NetworkId,
        dst: ID,
        dst_port: u16,
        src_port: u16,
        msg: &M,
        flags: u16,
    ) -> Result<usize, Error> {
        let mut b = self.pool.take();
        // invariant: encoding into a Vec can't run out of space.
        let _ = msg.encode(&mut b);
        let r = self.send_message(now, net, dst, dst_port, src_port, &b, flags, true);
        self.pool.put(b);
        r
    }

    /// [`Host::broadcast_with_flags`] of a protobuf message.
    pub fn broadcast_proto_with_flags<M: prost::Message>(
        &mut self,
        now: Time,
        net: NetworkId,
        dst_port: u16,
        src_port: u16,
        msg: &M,
        flags: u16,
    ) -> Result<(), Error> {
        let id = self.id;
        self.send_proto_with_flags(now, net, id, dst_port, src_port, msg, flags | M_BROADCAST)
    }

    fn seal(&mut self, dst: &ID, body: &[u8], out: &mut Vec<u8>) -> Result<(), Error> {
        let mut nonce = [0u8; NONCE_LEN];
        rand_core::Rng::fill_bytes(&mut self.rng, &mut nonce);
        let key = &self.key;
        let c = self
            .ciphers
            .get_or_try_insert(*dst, || MessageCipher::new(key, dst))?;
        c.seal(&nonce, body, out)?;
        Ok(())
    }

    /// Queues a signed message on the links routing picks. `trailer` holds
    /// the hosts it came through (fewer than `MAX_MESSAGE_HOPS`).
    fn route(
        &mut self,
        now: Time,
        net: NetworkId,
        wire: Vec<u8>,
        dst: &ID,
        unicast: bool,
        trailer: &[ID],
    ) -> usize {
        let Host {
            networks,
            pool,
            metrics,
            config,
            ..
        } = self;
        let Some(n) = networks.get_mut(net) else {
            pool.put(wire);
            return 0;
        };
        let targets = n.select_links(
            now,
            dst,
            unicast,
            trailer,
            config.routing,
            config.route_copies,
            backlog(config),
        );
        if unicast && config.routing == Routing::Learned {
            if targets.learned {
                metrics.learned_routes += 1;
            } else if targets.iter().next().is_none() && trailer.len() + 1 >= MAX_MESSAGE_HOPS {
                // The next host could neither be `dst` nor relay it.
                metrics.hop_limit += 1;
            } else {
                metrics.fallback_routes += 1;
            }
        }
        let mut peers = [PeerKey::default(); crate::message::MAX_MESSAGE_REPLICAS * 2];
        let mut len = 0;
        for (slot, r) in peers.iter_mut().zip(targets.iter()) {
            *slot = r.peer;
            len += 1;
        }
        self.enqueue(now, net, wire, peers.get(..len).unwrap_or_default())
    }

    /// Queues one marshaled message on the network's links to `peers`
    /// (distinct), skipping full queues and links whose frames are too
    /// small. Returns the number of copies queued.
    pub(crate) fn enqueue(
        &mut self,
        now: Time,
        net: NetworkId,
        wire: Vec<u8>,
        peers: &[PeerKey],
    ) -> usize {
        let Host {
            networks,
            bufs,
            pool,
            metrics,
            config,
            ..
        } = self;
        let Some(n) = networks.get_mut(net) else {
            pool.put(wire);
            return 0;
        };
        let len = wire.len();
        let fits = |link: &Link| {
            len <= link.max_body
                && link.queue.len() < config.link_queue_messages
                && link.queued_bytes + len <= config.link_queue_bytes
        };
        let mut count: u32 = 0;
        for peer in peers {
            let Some(link) = n.links.get_mut(peer) else {
                continue;
            };
            if fits(link) {
                count += 1;
            } else if len > link.max_body {
                metrics.too_large += 1;
            } else {
                metrics.link_queue_full += 1;
                link.dropped_at = now;
            }
        }
        if count == 0 {
            pool.put(wire);
            return 0;
        }
        // The peers are distinct, so the same links fit again below and
        // the buffer is referenced exactly `count` times.
        let id = bufs.insert(wire, count);
        for peer in peers {
            if let Some(link) = n.links.get_mut(peer)
                && fits(link)
            {
                link.queue.push_back(id);
                link.queued_bytes += len;
                n.dirty.insert(*peer);
                metrics.frames_queued += 1;
            }
        }
        count as usize
    }

    // --- ingress ------------------------------------------------------------

    /// Handles a frame vnic dispatched to this network's link on `peer`.
    /// Call [`Host::flush`] afterwards (once
    /// per batch is enough): relays and replies are queued, not sent.
    ///
    /// Errors are the peer's fault and are for logging; the frame is
    /// dropped.
    pub fn handle_frame(
        &mut self,
        now: Time,
        net: NetworkId,
        peer: PeerKey,
        body: &[u8],
    ) -> Result<(), Error> {
        self.metrics.frames_received += 1;
        let n = self.networks.get(net).ok_or(Error::NetworkNotFound)?;
        let link_host = n.links.get(&peer).ok_or(Error::UnknownLink)?.host_id;
        let store_id = n.store_id;
        let msg = match MessageView::decode(body) {
            Ok(m) => m,
            Err(e) => {
                self.metrics.malformed += 1;
                return Err(e.into());
            }
        };
        let hops = msg.hops();
        let last = hops.checked_sub(1);
        if last.and_then(|i| msg.host_id(i)) != Some(link_host) {
            self.metrics.trailer_mismatch += 1;
            return Err(Error::LastHopMismatch);
        }
        if msg.header.flags & M_COMPRESS != 0 {
            self.metrics.compressed_dropped += 1;
            return Err(Error::CompressionUnsupported);
        }
        let verified = match self.config.verify {
            VerifyMode::Off => None,
            _ => Some(self.verify_all(&msg)),
        };
        if verified == Some(false) {
            self.metrics.verify_failures += 1;
            if self.config.verify == VerifyMode::Enforce {
                self.metrics.unverified_dropped += 1;
                return Err(Error::Unverified);
            }
        }

        let learn = self.config.routing == Routing::Learned;
        if let (Some(last), Some(n)) = (last, self.networks.get_mut(net)) {
            for i in 0..last {
                let Some(target) = msg.host_id(i) else {
                    continue;
                };
                if learn {
                    // Consecutive trailer entries are linked. Every copy
                    // counts, duplicates included: each shows a path.
                    if let Some(next) = msg.host_id(i + 1) {
                        n.topology.learn_edge(now, target, next);
                    }
                } else {
                    // Every earlier hop is reachable through this link.
                    n.next_hop.insert(now, target, link_host, last - i);
                }
            }
        }

        let Some(id) = msg.id() else {
            return Err(Error::LastHopMismatch);
        };
        let h = u8::try_from(hops).unwrap_or(u8::MAX);
        match self.recent.insert_hops(now, (store_id, id), h) {
            None => self.handle_message(now, net, &msg, verified),
            // A unicast copy that took fewer hops than every earlier one
            // can still get further: relay it, but don't deliver it again.
            // At most MAX_MESSAGE_HOPS times per message. (Not broadcasts:
            // a service handler that rejected the first copy stopped it.)
            Some(fewest) if learn && h < fewest && msg.header.flags & M_BROADCAST == 0 => {
                self.metrics.shorter_relays += 1;
                self.relay_message(now, net, &msg);
                Ok(())
            }
            Some(_) => {
                self.metrics.duplicates += 1;
                Ok(())
            }
        }
    }

    /// Checks every trailer signature, with cached verifying keys. A
    /// message with more entries than [`MAX_MESSAGE_HOPS`] (which no host
    /// sends) fails without checking.
    fn verify_all(&mut self, msg: &MessageView<'_>) -> bool {
        let hops = msg.hops();
        if hops > MAX_MESSAGE_HOPS {
            return false;
        }
        let verifiers = &mut self.verifiers;
        (0..hops).all(|hop| {
            msg.verify_with(hop, |id| {
                verifiers
                    .get_or_try_insert::<()>(*id, || {
                        Ok(VerifyingKey::from_bytes(&id.to_bytes()).ok())
                    })
                    .ok()
                    .copied()
                    .flatten()
            })
        })
    }

    /// Delivers a received message if it is for this host, and relays it.
    fn handle_message(
        &mut self,
        now: Time,
        net: NetworkId,
        msg: &MessageView<'_>,
        verified: Option<bool>,
    ) -> Result<(), Error> {
        let h = msg.header;
        let hops = msg.hops();
        let src = msg.src_host_id().ok_or(Error::LastHopMismatch)?;
        let lead = (0..hops)
            .take_while(|&i| msg.host_id(i) == Some(src))
            .count();
        let inbound = Inbound {
            header: h,
            src,
            hops,
            lead,
            verified,
            body: msg.body,
        };
        let encrypted = h.flags & M_ENCRYPT != 0;
        if h.flags & M_BROADCAST != 0 {
            // A broadcast a service handler rejects goes no further.
            self.call_handler(now, net, &inbound, encrypted.then_some(&src))?;
        } else if h.dst_id == self.id {
            return self.call_handler(now, net, &inbound, encrypted.then_some(&src));
        }

        self.relay_message(now, net, msg);
        Ok(())
    }

    /// Relays a received message within the hop limit.
    fn relay_message(&mut self, now: Time, net: NetworkId, msg: &MessageView<'_>) {
        let h = msg.header;
        let hops = msg.hops();
        let unicast = h.flags & M_BROADCAST == 0;
        if unicast && h.dst_id == self.id {
            return;
        }
        if h.flags & M_NORELAY != 0 && hops > 0 {
            return;
        }
        // An announcement's reach is set by its sender's trailer padding;
        // what this host learns from it goes into its own announcements.
        if h.dst_port == crate::announce::LINK_ANNOUNCE_PORT
            && self.config.routing == Routing::Learned
        {
            return;
        }
        if hops >= MAX_MESSAGE_HOPS {
            self.metrics.hop_limit += 1;
            return;
        }

        let mut trailer = [ID::MIN; MAX_MESSAGE_HOPS];
        for (i, slot) in trailer.iter_mut().enumerate().take(hops) {
            *slot = msg.host_id(i).unwrap_or(ID::MIN);
        }
        let mut wire = self.pool.take();
        encode_relay(&mut wire, msg, now, &self.id, &self.signer);
        self.metrics.messages_relayed += 1;
        self.route(
            now,
            net,
            wire,
            &h.dst_id,
            unicast,
            trailer.get(..hops).unwrap_or_default(),
        );
    }

    /// Hands a message to its port's handler, decrypting it with the
    /// sender's key when `decrypt_from` is set.
    fn call_handler(
        &mut self,
        now: Time,
        net: NetworkId,
        msg: &Inbound<'_>,
        decrypt_from: Option<&ID>,
    ) -> Result<(), Error> {
        let Some(handler) = self
            .networks
            .get(net)
            .and_then(|n| n.handlers.get(&msg.header.dst_port))
            .copied()
        else {
            self.metrics.no_handler += 1;
            return Ok(());
        };
        let mut plain = self.pool.take();
        match decrypt_from {
            Some(src) => {
                let key = &self.key;
                let res = self
                    .ciphers
                    .get_or_try_insert(*src, || MessageCipher::new(key, src))
                    .map_err(Error::from)
                    .and_then(|c| c.open(msg.body, &mut plain).map_err(Error::from));
                if let Err(e) = res {
                    self.metrics.decrypt_failures += 1;
                    self.pool.put(plain);
                    return Err(e);
                }
            }
            None => {
                if let Handler::App(_) = handler {
                    plain.extend_from_slice(msg.body);
                }
            }
        }
        let body = if decrypt_from.is_some() {
            &plain[..]
        } else {
            msg.body
        };
        let inbound = Inbound { body, ..*msg };
        let res = match handler {
            Handler::App(h) => {
                if self.queued_deliveries >= self.config.max_queued_deliveries {
                    self.metrics.deliveries_dropped += 1;
                    self.pool.put(plain);
                    return Ok(());
                }
                self.queued_deliveries += 1;
                self.metrics.messages_delivered += 1;
                self.events.push_back(Event::Message(Delivery {
                    network: net,
                    handler: h,
                    src: msg.src,
                    src_port: msg.header.src_port,
                    dst: msg.header.dst_id,
                    dst_port: msg.header.dst_port,
                    flags: msg.header.flags,
                    hops: msg.hops,
                    verified: msg.verified,
                    body: plain,
                    received: now,
                }));
                return Ok(());
            }
            Handler::HashTable => {
                self.metrics.messages_delivered += 1;
                self.hash_table_handle(now, net, &inbound)
            }
            Handler::PeerIndex => {
                self.metrics.messages_delivered += 1;
                self.peer_index_handle(now, net, &inbound)
            }
            Handler::PeerExchange => {
                self.metrics.messages_delivered += 1;
                self.peer_exchange_handle(now, net, &inbound)
            }
            // Counted in `link_announcements_received`.
            Handler::LinkAnnounce if self.config.routing == Routing::Learned => {
                self.link_announce_handle(now, net, &inbound)
            }
            Handler::LinkAnnounce => Ok(()),
        };
        self.pool.put(plain);
        if res.is_err() {
            self.metrics.handler_errors += 1;
        }
        res
    }

    // --- egress -------------------------------------------------------------

    /// Fills one vnic frame body with the next message queued on the
    /// network's link to `peer`. Call it from the driver's
    /// `vnic.poll_egress` callback for frames tagged with this network.
    pub fn fill_egress(&mut self, net: NetworkId, peer: PeerKey, body: &mut [u8]) -> Fill {
        let Host {
            networks,
            bufs,
            pool,
            metrics,
            ..
        } = self;
        let Some(link) = networks.get_mut(net).and_then(|n| n.links.get_mut(&peer)) else {
            return Fill::NONE;
        };
        while let Some(id) = link.queue.pop_front() {
            let len = bufs.get(id).map_or(0, <[u8]>::len);
            link.queued_bytes = link.queued_bytes.saturating_sub(len);
            let copied = match (bufs.get(id), body.get_mut(..len)) {
                (Some(src), Some(dst)) => {
                    dst.copy_from_slice(src);
                    true
                }
                _ => false,
            };
            if let Some(b) = bufs.release(id) {
                pool.put(b);
            }
            if copied {
                metrics.frames_sent += 1;
                return Fill {
                    len,
                    more: !link.queue.is_empty(),
                };
            }
            metrics.too_large += 1;
        }
        Fill::NONE
    }

    /// Applies queued work to vnic: sets the backlog of every link with
    /// queued messages and answers link offers (which need vnic's peer
    /// table). Call it after `handle_frame`, sends, service calls and
    /// `handle_timeout`, before `vnic.poll_egress`.
    pub fn flush<T: Copy>(&mut self, vnic: &mut VnicHost<T>, now: Time) {
        self.peer_exchange_flush(vnic, now);
        let learned = self.config.routing == Routing::Learned;
        let Host {
            networks,
            bufs,
            peer_index,
            hash_table,
            ..
        } = self;
        for (net, n) in networks.iter_mut() {
            if std::mem::take(&mut n.links_changed) && learned {
                let at = now + crate::announce::ANNOUNCE_DELAY;
                if n.announce_at.is_nil() || at < n.announce_at {
                    n.announce_at = at;
                }
            }
            // New links get this host's records (`push_records`).
            if !n.new_links.is_empty() && n.push_at.is_nil() {
                if peer_index.publishes_in(net) || hash_table.unsent_in(net) {
                    let at = now + RECORD_PUSH_DELAY;
                    n.push_at = if n.pushed_at.is_nil() {
                        at
                    } else {
                        at.max(n.pushed_at + RECORD_PUSH_INTERVAL)
                    };
                } else {
                    n.new_links.clear();
                }
            }
            let dirty = std::mem::take(&mut n.dirty);
            for peer in dirty {
                let Some(link) = n.links.get(&peer) else {
                    continue;
                };
                if let Some(len) = link
                    .queue
                    .front()
                    .and_then(|id| bufs.get(*id))
                    .map(<[u8]>::len)
                {
                    // A writer vnic already closed (its peer is gone) is
                    // forgotten at `handle_peer_removed`.
                    let _ = vnic.set_backlog_hint(link.writer, len);
                }
            }
        }
    }

    /// Whether `flush` has backlogs to set.
    pub fn needs_flush(&self) -> bool {
        self.networks.values().any(|n| {
            !n.dirty.is_empty()
                || n.links_changed
                || (!n.new_links.is_empty() && n.push_at.is_nil())
        }) || !self.peer_exchange.pending.is_empty()
    }

    /// Messages queued on the network's link to `peer`.
    pub fn queued(&self, net: NetworkId, peer: PeerKey) -> usize {
        self.networks
            .get(net)
            .and_then(|n| n.links.get(&peer))
            .map_or(0, |l| l.queue.len())
    }

    // --- events and timers ------------------------------------------------

    pub fn poll_event(&mut self) -> Option<Event<H>> {
        let e = self.events.pop_front()?;
        if let Event::Message(_) = e {
            self.queued_deliveries = self.queued_deliveries.saturating_sub(1);
        }
        Some(e)
    }

    pub(crate) fn push_event(&mut self, e: Event<H>) {
        self.events.push_back(e);
    }

    /// Returns a delivery's body to the buffer pool.
    pub fn recycle(&mut self, body: Vec<u8>) {
        self.pool.put(body);
    }

    /// The earliest deadline: dedup pruning, record expiry and garbage
    /// collection, republishing, and request and exchange timeouts.
    pub fn poll_timeout(&self) -> Option<Time> {
        let mut t: Option<Time> = None;
        let mut min = |x: Option<Time>| {
            if let Some(x) = x {
                t = Some(t.map_or(x, |t| t.min(x)));
            }
        };
        min(self.recent.poll_timeout());
        min(self.hash_table_poll_timeout());
        min(self.peer_index_poll_timeout());
        min(self.peer_exchange_poll_timeout());
        for n in self.networks.values() {
            if !n.announce_at.is_nil() {
                min(Some(n.announce_at));
            }
            if !n.push_at.is_nil() {
                min(Some(n.push_at));
            }
        }
        t
    }

    /// Runs the timers that are due.
    pub fn handle_timeout(&mut self, now: Time) {
        self.recent.handle_timeout(now);
        // Expired learned links are ignored when searching; pruning only
        // frees their memory, so it piggybacks on other timers.
        let prune_topology = !now.before(self.next_topology_prune);
        if prune_topology {
            self.next_topology_prune = now + TOPOLOGY_PRUNE_INTERVAL;
        }
        for (_, n) in self.networks.iter_mut() {
            n.next_hop.prune(now);
            if prune_topology {
                n.topology.prune(now);
            }
        }
        self.hash_table_timeout(now);
        self.peer_index_timeout(now);
        self.peer_exchange_timeout(now);
        let due: Vec<NetworkId> = self
            .networks
            .iter()
            .filter(|(_, n)| !n.announce_at.is_nil() && !now.before(n.announce_at))
            .map(|(k, _)| k)
            .collect();
        for net in due {
            if let Some(n) = self.networks.get_mut(net) {
                n.announce_at = now + crate::announce::ANNOUNCE_INTERVAL;
            }
            // A failed announcement is retried at the next interval.
            let _ = self.link_announce(now, net);
        }
        let push: Vec<NetworkId> = self
            .networks
            .iter()
            .filter(|(_, n)| !n.push_at.is_nil() && !now.before(n.push_at))
            .map(|(k, _)| k)
            .collect();
        for net in push {
            self.push_records(now, net);
        }
    }

    /// Gives the links added since the last push this host's records.
    /// Publishes repeat every 10 minutes, so without a push a record
    /// published before a link came up is out of that link's reach until
    /// the next publish.
    ///
    /// - **Peer index:** each record goes to the new links its publish
    ///   would have gone to ([`Host::neighbours`]), as the record last
    ///   published (same timestamp and signature).
    /// - **Hash table:** a record whose last publish reached no link is
    ///   published again (towards its hash, from the new links on). A
    ///   record that did reach the network is already where gets look.
    fn push_records(&mut self, now: Time, net: NetworkId) {
        let Some(n) = self.networks.get_mut(net) else {
            return;
        };
        n.push_at = Time::NIL;
        n.pushed_at = now;
        let new = std::mem::take(&mut n.new_links);
        let peers: Vec<PeerKey> = self
            .neighbours(now, net)
            .into_iter()
            .filter(|p| new.contains(p))
            .collect();
        self.peer_index_push(now, net, &peers);
        self.hash_table_publish_unsent(now, net);
    }
}

/// The queue level at which [`Routing::Learned`] adds a backup copy to a
/// unicast over a direct link: half the queue bounds.
fn backlog(config: &Config) -> Backlog {
    Backlog {
        messages: (config.link_queue_messages / 2).max(1),
        bytes: (config.link_queue_bytes / 2).max(1),
    }
}

/// Uniform in `0..n` (`n > 0`).
fn random_below(rng: &mut DetRng, n: u32) -> u32 {
    let n = u64::from(n.max(1));
    let limit = (1u64 << 32) / n * n;
    loop {
        let v = u64::from(rand_core::Rng::next_u32(rng));
        if v < limit {
            // invariant: v % n < n <= u32::MAX.
            return (v % n) as u32;
        }
    }
}

/// Signs a record with this host's key: sets `key`, then signs the
/// record's stable encoding.
pub(crate) fn sign_record<T: strims_proto::stable::Signable>(r: &mut T, signer: &SigningKey) {
    use strims_crypto::ed25519_dalek::Signer;
    r.set_key(&signer.verifying_key().to_bytes());
    let b = r.stable_encode_unsigned_to_vec();
    r.set_signature(signer.sign(&b).to_bytes().to_vec());
}

/// A random u64 (request and exchange ids).
pub(crate) fn random_u64(rng: &mut DetRng) -> u64 {
    rand_core::Rng::next_u64(rng)
}
