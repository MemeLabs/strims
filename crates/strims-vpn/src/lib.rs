// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sans-IO overlay network, DHT, peer index and peer exchange.
//!
//! A [`Host`] joins networks over the peers of a [`strims_vnic::Host`].
//! Messages hop between hosts, signed at every hop, towards a destination id
//! (unicast) or a fan-out of hosts near an id (broadcast). Each network runs
//! the hash table (port 10), the peer index (11), peer exchange (12) and
//! link announcements ([`LINK_ANNOUNCE_PORT`], 20).
//!
//! The vpn host never stores the vnic host; calls that need it take
//! `&mut VnicHost<T>`. `H` is the app's handler tag, and deliveries come
//! back tagged with it.
//!
//! # Driver contract
//!
//! ```text
//! vnic PeerAdded { peer, .. }     vpn.add_peer(&mut vnic, net, peer, src_port, dst_port, Tag::Vpn(net))
//! vnic PeerRemoved { peer, .. }   vpn.handle_peer_removed(peer)
//! vnic.handle_link_input(link, now, buf, &mut |f| match f.handler {
//!     Tag::Vpn(net) => { let _ = vpn.handle_frame(now, net, f.peer, f.body); }
//!     ..
//! })
//! after every input, app call and timeout: vpn.flush(&mut vnic, now), then drain vpn.poll_event()
//! vnic.poll_egress(now, &mut |f| match f.tag { Tag::Vpn(net) => vpn.fill_egress(net, f.peer, f.body), .. })
//! timer at min(vnic.poll_timeout(), vpn.poll_timeout()): vpn.handle_timeout(now), then flush
//! ```
//!
//! - The app picks a peer's networks. The `add_peer` tag must bring the
//!   link's frames and send opportunities back to that network.
//! - [`Host::handle_frame`] doesn't take the vnic host, because vnic lends
//!   frames while it is mutably borrowed. Relays and replies wait on
//!   per-link queues, and [`Host::flush`] sets the links' backlogs on vnic.
//!   Link offers need vnic's peer table, so `flush` answers them too.
//! - `flush` schedules link announcements and record pushes after link
//!   changes (`add_peer` and `remove_peer` take no `now`), so read
//!   [`Host::poll_timeout`] after `flush`.
//! - [`Host::fill_egress`] copies one message into each frame and reports
//!   `more` while the link queue holds another. Each (network, link) writer
//!   is a QoS session under its network's class, under the vpn class, under
//!   vnic's root, all with weight 1.
//! - Peer exchange: the driver keeps one set of link candidates
//!   (`strims_vnic::interface::LinkCandidatePool`) per [`ExchangeId`],
//!   created on its first [`Event::GatherLocalDescriptions`] or
//!   [`Event::DialRemoteDescriptions`] and dropped on
//!   [`Event::ExchangeClosed`], and reports each step with
//!   [`Host::set_local_descriptions`] and
//!   [`Host::set_remote_descriptions_result`]. An exchange closes
//!   [`peer_exchange::LINK_TIMEOUT`] after it starts; a link it made stays.
//! - Incoming link offers are refused while vnic has no room for peers.
//!   `vnic.max_peers()` is 0 until the app calls `vnic.set_max_peers`.
//!
//! # Messaging
//!
//! - **Ports.** [`Host::set_handler`] fails with [`Error::PortInUse`] on a
//!   taken port, so an app can't take over a service port.
//!   [`Host::reserve_port`] picks a free port from 1000 up.
//! - **Sending.** [`Host::send`] and [`Host::send_proto`] encrypt for the
//!   destination ([`M_STD_FLAGS`]). [`Host::broadcast`] goes towards this
//!   host's own id, in the clear, and is also delivered here with 0 hops.
//!   The `*_with_flags` forms take [`M_ENCRYPT`], [`M_BROADCAST`] and
//!   [`M_NORELAY`]; [`M_COMPRESS`] is refused. [`Channel`] pairs a handler
//!   with one destination.
//! - **Deliveries** are [`Event::Message`]s with the body decrypted. Hand
//!   the body back with [`Host::recycle`] to keep the hot path free of
//!   allocations. [`Delivery::src`] is the sender (trailer entry 0); an rpc
//!   layer must check a peer certificate's key against it. A broadcast is
//!   relayed whatever the app does with it; one a service handler rejects
//!   is not.
//! - **Size.** A message must fit one frame: 42 header bytes, the body (28
//!   more when encrypted) and 104 bytes per trailer entry, within
//!   [`Config::max_message_len`]. `send` checks this with one entry. Each
//!   relay adds an entry, up to five, so a message meant to cross five hops
//!   keeps its body within `max_message_len - 42 - 5 * 104` (less 28 when
//!   encrypted: [`MAX_RELAYED_BODY_LEN`] for the standard frame size). A
//!   copy too large for a link is dropped and counted in
//!   [`Metrics::too_large`].
//! - **Addresses.** A service's [`HostAddr`] is published in the hash table
//!   as an encoded `NetworkAddress`.
//! - **Services** report through events: hash table values until
//!   [`Event::HashTableGetClosed`], peer index hosts (each once per search)
//!   until [`Event::PeerIndexSearchClosed`], and an [`ExchangeOutcome`] per
//!   exchange.
//!
//! # Routing and verification
//!
//! [`Config::routing`] picks the links a message goes to: [`Routing`] and
//! the [`network`] module describe both modes. [`Config::verify`] decides
//! what happens to trailer signatures: see [`VerifyMode`].
//!
//! # Bounds
//!
//! Peer input can't grow anything past these:
//!
//! - link queues: [`Config::link_queue_messages`] and
//!   [`Config::link_queue_bytes`] per (network, link); copies for a full
//!   queue are dropped ([`Metrics::link_queue_full`]), so a slow link never
//!   blocks the others;
//! - queued outbound messages: referenced by link queues only; free
//!   buffers: 32 of at most 64 KiB;
//! - dedup table: [`Config::recent_message_ids`] entries of about 120 bytes,
//!   oldest evicted ([`Metrics::dedup_evictions`]);
//! - next-hop hints ([`Routing::Greedy`]): [`Config::next_hop_capacity`]
//!   per network, expiring after [`Config::next_hop_ttl`];
//! - learned link graph ([`Routing::Learned`]):
//!   [`Config::topology_max_nodes`] hosts per network (least recently seen
//!   evicted) and [`Config::topology_max_degree`] links each (oldest
//!   evicted), ignored after [`Config::topology_ttl`]; one announced list of
//!   at most 64 ids per link;
//! - deliveries: [`Config::max_queued_deliveries`] events, then dropped
//!   ([`Metrics::deliveries_dropped`]);
//! - hash table store: [`Config::hash_table_max_records`] and
//!   [`Config::hash_table_max_bytes`] host-wide, the record farthest from
//!   this host evicted first;
//! - peer index store: [`Config::peer_index_max_records`] per network, the
//!   least recently received evicted first;
//! - link offers and incoming exchanges: [`Config::max_pending_offers`] and
//!   [`Config::max_incoming_exchanges`];
//! - cipher and verifying-key caches: 64 and 256 entries, oldest evicted.
//!
//! Publishers, gets, searches and outgoing exchanges are bounded by the
//! owner's calls.

mod addr;
mod announce;
mod buf;
mod cache;
pub mod channel;
pub mod hash_table;
pub mod host;
pub mod message;
pub mod message_filter;
pub mod message_id_lru;
pub mod network;
pub mod peer_exchange;
pub mod peer_index;
mod timeout_queue;
mod topology;

pub use addr::HostAddr;
pub use announce::LINK_ANNOUNCE_PORT;
pub use channel::Channel;
pub use hash_table::{HashTableGetOptions, hash_table_record_hash};
pub use host::{
    Config, DEFAULT_MAX_MESSAGE_LEN, Delivery, Event, Host, MAX_RELAYED_BODY_LEN, Metrics,
    RECORD_PUSH_DELAY, RECORD_PUSH_INTERVAL, Routing, VerifyMode,
};
pub use message::{
    M_BROADCAST, M_COMPRESS, M_ENCRYPT, M_NORELAY, M_STD_FLAGS, MessageHeader, MessageId,
    MessageView,
};
pub use peer_exchange::ExchangeOutcome;
pub use peer_index::{PeerIndexHost, peer_index_record_hash};
pub use strims_vnic::LinkDescription;

/// The vnic host vpn composes with.
pub type VnicHost<T> = strims_vnic::Host<T>;

slotmap::new_key_type! {
    /// A network this host is in.
    pub struct NetworkId;
    /// A hash table publisher.
    pub struct HashTablePublisherId;
    /// A hash table get.
    pub struct HashTableGetId;
    /// A peer index publisher.
    pub struct PeerIndexPublisherId;
    /// A peer index search.
    pub struct PeerIndexSearchId;
    /// A peer exchange: the driver keys its link candidates by it.
    pub struct ExchangeId;
}

/// Errors.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("duplicate network key")]
    DuplicateNetworkKey,
    #[error("network not found")]
    NetworkNotFound,
    /// The host ran out of network store ids.
    #[error("network id out of range")]
    NetworkIdBounds,
    /// The vnic peer isn't established.
    #[error("unknown vnic peer")]
    UnknownPeer,
    /// A frame for a link the network doesn't have.
    #[error("unknown link")]
    UnknownLink,
    /// The port already has a handler.
    #[error("port {0} already in use")]
    PortInUse(u16),
    #[error("no free ports")]
    NoFreePorts,
    /// [`M_COMPRESS`] is neither sent nor accepted.
    #[error("message compression is not supported")]
    CompressionUnsupported,
    /// Header, body and the first trailer entry exceed
    /// `Config::max_message_len`.
    #[error("message of {0} bytes is too large")]
    MessageTooLarge(usize),
    /// The last trailer entry isn't the host of the link the frame came
    /// from.
    #[error("message last hop trailer mismatch")]
    LastHopMismatch,
    /// Dropped by `VerifyMode::Enforce`.
    #[error("message signature verification failed")]
    Unverified,
    #[error("message: {0}")]
    Message(#[from] message::MessageError),
    #[error("cipher: {0}")]
    Cipher(#[from] message_filter::CipherError),
    #[error("decoding: {0}")]
    Decode(String),
    #[error("unexpected message type")]
    UnexpectedMessageType,
    #[error("invalid record: {0}")]
    InvalidRecord(&'static str),
    /// A peer index record's host isn't its sender.
    #[error("host id mismatch")]
    HostIdMismatch,
    /// A publisher, request or exchange id that doesn't exist (anymore),
    /// or a reply in the wrong state.
    #[error("unknown request")]
    UnknownRequest,
    #[error("crypto: {0}")]
    Crypto(#[from] strims_crypto::Error),
    #[error("vnic: {0}")]
    Vnic(#[from] strims_vnic::Error),
}
