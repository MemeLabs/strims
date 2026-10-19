// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The [`Host`]: links, peers, ports, writers and timers.
//!
//! # Links
//!
//! A link runs, in order: the dialer's `TCPMuxInit` and the AES link
//! handshake (TCP only, see [`LinkConfig::tcp_dial`]), then `PeerInit` both
//! ways, and then carries frames. [`PeerKey`] names the link from
//! [`Host::add_link`] on, and the peer on it once the handshake completes.
//! Certificates are verified at the `now` passed in.
//!
//! # Ingress
//!
//! Frames are dispatched in stream order, so order per (peer, port) holds. A
//! frame that lies inside one input chunk is lent straight from the driver's
//! buffer; only a frame split across reads is copied, into one reusable
//! buffer per link. Frames for ports without a handler are dropped and
//! counted ([`HostMetrics::frame_handler_not_found_count`]). The dispatch
//! callback can't reach the host: owners record what they need and act (for
//! example [`Host::set_backlog`]) after the call returns.
//!
//! # Egress and QoS
//!
//! - A writer ([`Host::open_writer`]) is one QoS session sending to one
//!   (peer, port).
//! - An owner with data calls [`Host::set_backlog`]; repeating it is cheap.
//! - [`Host::poll_egress`] dequeues sessions in QoS order and lends each a
//!   frame body inside the link buffer ([`EgressFrame::body`], at most
//!   MTU − 4 bytes). The owner fills it and answers with a [`Fill`]; vnic
//!   writes the header, encrypts AES links and commits the frame.
//! - A link whose buffer has less than one MTU free blocks all its writers'
//!   sessions; [`Host::advance_link_transmit`] unblocks them once an MTU is
//!   free. A writer opened on a blocked link starts blocked.
//! - vpn adds its host class under [`Host::qos_root`] and a class per
//!   network under that, holding the networks' writers. Other owners add
//!   their own classes under the root.
//!
//! # Ports
//!
//! Ports below [`ports::RESERVED_PORT_COUNT`] are well known (see
//! [`ports`]). [`Host::reserve_port`] draws uniformly from the range above;
//! after 64 draws that hit taken ports it scans from a random start, so a
//! full table returns [`Error::NoFreePorts`].
//!
//! # Events
//!
//! [`Host::poll_event`] reports peers added and removed, and links the
//! driver must close. When a new link to an already-connected host
//! completes, the old peer is replaced, and the events come in the order
//! `PeerRemoved(old)`, `LinkClosed(old, Replaced)`, `PeerAdded(new)`. A host
//! id can move to a new link, so owners key their state by [`PeerKey`].
//!
//! # Timers
//!
//! [`Host::poll_timeout`] reports the next deadline (QoS rate-limit release
//! or a link timer) and [`Host::handle_timeout`] runs the link timers;
//! [`Host::poll_egress`] runs them too. Each link has at most one deadline:
//!
//! - Before `PeerInit`, the handshake deadline: the peer's `PeerInit` must
//!   arrive within [`Host::handshake_timeout`] of `add_link`, however slowly
//!   its bytes trickle in, or the link is closed with
//!   [`Error::HandshakeTimeout`].
//! - After it, the keepalive check: an established link that has queued
//!   nothing for [`Host::keepalive_interval`] queues an empty frame on
//!   [`ports::KEEPALIVE_PORT`]. Frames only update a timestamp, and the
//!   timer re-arms from it when it fires, so a busy link costs one wakeup per
//!   interval.
//!
//! Received keepalives are dropped before handler lookup, counted in
//! [`HostMetrics::keepalive_read_count`], and set
//! [`Peer::sends_keepalives`].

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use slotmap::SlotMap;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    CertificateErrors, certificate_parent, key_to_curve25519, new_certificate_request,
    new_self_signed_certificate, public_key_to_curve25519, sign_certificate_request,
    verify_certificate,
};
use strims_kademlia::ID;
use strims_proto::types::{Certificate, Key, KeyUsage, certificate::ParentOneof};
use strims_proto::vnic::v1::{PeerInit, TcpMuxInit};
use strims_qos::{ClassId, Control, SessionId};

use crate::aes_link::{AesLink, AesLinkHandshake};
use crate::frame::{FRAME_HEADER_LEN, FrameDecoder, FrameHeader, MAX_FRAME_BODY_LEN};
use crate::metrics::{HostMetrics, LinkMetrics};
use crate::out_buf::OutBuf;
use crate::stream::{MessageReader, StreamError, decode_stream_message, encode_stream_message};
use crate::{Error, PROTOCOL_VERSION};

/// Port numbers.
pub mod ports {
    /// Ports below this are well known; [`crate::Host::reserve_port`] draws
    /// from `RESERVED_PORT_COUNT..u16::MAX`.
    pub const RESERVED_PORT_COUNT: u16 = 1000;

    // default network service ports
    pub const HASH_TABLE_PORT: u16 = 10;
    pub const PEER_INDEX_PORT: u16 = 11;
    pub const PEER_EXCHANGE_PORT: u16 = 12;
    pub const TRANSFER_PORT: u16 = 13;
    pub const SNIPPET_PORT: u16 = 14;

    // peer link ports
    pub const NETWORK_INIT_PORT: u16 = 0;
    pub const NETWORK_BROKER_PORT: u16 = 1;
    pub const BOOTSTRAP_PORT: u16 = 2;
    pub const SWARM_PORT: u16 = 3;
    pub const PEER_RPC_CLIENT_PORT: u16 = 4;
    pub const PEER_RPC_SERVER_PORT: u16 = 5;
    pub const REPLICATION_PORT: u16 = 6;

    /// vnic's keepalive frames (empty bodies). No service may bind it:
    /// [`crate::Host::set_handler`], [`crate::Host::open_writer`] and the
    /// channel calls refuse it with [`crate::Error::PortInUse`], and
    /// [`crate::Host::reserve_port`] never draws it.
    pub const KEEPALIVE_PORT: u16 = u16::MAX;
}

/// How long a link may take from [`Host::add_link`] to the peer's
/// `PeerInit` (the dialer's TCP mux init, the AES handshake and `PeerInit`)
/// before vnic closes it. Transport read timeouts don't bound this: a peer
/// can trickle bytes or answer WebSocket pings indefinitely.
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(25);

/// How long an established link may queue nothing before vnic queues a
/// keepalive frame. Must stay below the 25 s transport read timeouts
/// ([`crate::TcpOptions::read_timeout`], [`crate::ws::WsOptions::read_timeout`]),
/// with room for timer and write latency.
pub const DEFAULT_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);

/// How many links may be in the handshake at once. Adding one more closes
/// the oldest (see [`Host::set_max_pending_links`]).
pub const DEFAULT_MAX_PENDING_LINKS: usize = 128;

/// A keepalive frame: [`ports::KEEPALIVE_PORT`], empty body.
const KEEPALIVE_FRAME: [u8; FRAME_HEADER_LEN] = [0xff, 0xff, 0, 0];

/// Lifetime of the certificates sent in `PeerInit`.
pub const HOST_CERT_VALID_DURATION: Duration = Duration::HOUR;

/// Weight of each writer's QoS session.
pub const WRITER_QOS_WEIGHT: u64 = 1;

slotmap::new_key_type! {
    /// A link and, once its handshake completes, the peer on it.
    pub struct PeerKey;
    /// A frame writer: one QoS session sending frames to one port of one
    /// peer.
    pub struct WriterId;
}

/// The transport under a link, for metrics and logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkKind {
    Tcp,
    Ws,
    WebRtc,
    /// Anything else, such as in-memory test links.
    Other,
}

/// What a link runs before `PeerInit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AesLinkConfig {
    /// The X25519 public key the peer must present; `None` accepts any key.
    pub peer_key: Option<[u8; 32]>,
}

/// How to run a new link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkConfig {
    pub kind: LinkKind,
    /// The largest frame, header included, and so the largest write.
    /// Between 5 and `4 + 65535`.
    pub mtu: usize,
    /// Outbound buffer size; at least `mtu`. A link whose buffer has less
    /// than `mtu` free blocks its writers.
    pub send_buffer: usize,
    /// Write a `TCPMuxInit` naming this profile key first (TCP dialer).
    pub tcp_mux_init: Option<[u8; 32]>,
    /// Run the AES link handshake (TCP).
    pub aes: Option<AesLinkConfig>,
}

impl LinkConfig {
    /// A plain link with a send buffer of two MTUs.
    pub fn new(kind: LinkKind, mtu: usize) -> LinkConfig {
        LinkConfig {
            kind,
            mtu,
            send_buffer: mtu.saturating_mul(2),
            tcp_mux_init: None,
            aes: None,
        }
    }

    /// A TCP dialer's link: mux init for the peer's profile key, then the
    /// AES handshake expecting that key's X25519 form.
    pub fn tcp_dial(peer_profile_key: &[u8; 32]) -> LinkConfig {
        LinkConfig {
            tcp_mux_init: Some(*peer_profile_key),
            aes: Some(AesLinkConfig {
                peer_key: Some(public_key_to_curve25519(peer_profile_key)),
            }),
            ..LinkConfig::new(LinkKind::Tcp, crate::tcp::TCP_MTU)
        }
    }

    /// A TCP listener's link, once [`crate::TcpMux`] has routed the
    /// connection here: the AES handshake, accepting any key.
    pub fn tcp_accept() -> LinkConfig {
        LinkConfig {
            aes: Some(AesLinkConfig { peer_key: None }),
            ..LinkConfig::new(LinkKind::Tcp, crate::tcp::TCP_MTU)
        }
    }

    /// A WebSocket link (either side).
    pub fn ws() -> LinkConfig {
        LinkConfig::new(LinkKind::Ws, crate::ws::WS_MTU)
    }

    /// A WebRTC data channel link (either side).
    pub fn webrtc() -> LinkConfig {
        LinkConfig::new(LinkKind::WebRtc, crate::webrtc::DATA_CHANNEL_MTU)
    }

    /// Overrides the MTU, keeping a send buffer of two MTUs.
    pub fn with_mtu(self, mtu: usize) -> LinkConfig {
        LinkConfig {
            mtu,
            send_buffer: mtu.saturating_mul(2),
            ..self
        }
    }
}

/// Host options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostConfig {
    /// The host certificate's subject.
    pub label: String,
    /// Upload limit in bytes per second.
    pub rate_limit: Option<u64>,
    /// Sent in `PeerInit`.
    pub node_platform: String,
    /// Sent in `PeerInit`.
    pub node_version: String,
}

impl Default for HostConfig {
    fn default() -> HostConfig {
        HostConfig {
            label: String::new(),
            rate_limit: None,
            node_platform: String::new(),
            node_version: "dev".to_owned(),
        }
    }
}

/// Why vnic closed a link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// The handshake failed, including [`Error::HandshakeTimeout`] (the
    /// handshake deadline passed) and [`Error::TooManyPendingLinks`] (a
    /// newer link took its place).
    Failed(Error),
    /// A newer link to the same host replaced this one.
    Replaced,
    /// [`Host::close_peer`] or [`Host::close`].
    Closed,
}

/// Notifications for the owner and the driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A link finished its handshake. Register handlers and open writers
    /// before feeding the link's remaining input.
    PeerAdded { peer: PeerKey, host_id: ID },
    /// An established peer went away. Its handlers and writers are gone;
    /// owners drop their ids for it. Comes before the `PeerAdded` of a
    /// replacing link.
    PeerRemoved { peer: PeerKey, host_id: ID },
    /// vnic closed the link: the driver drops the transport. Not emitted
    /// for [`Host::remove_link`], which the driver calls itself.
    LinkClosed { link: PeerKey, reason: CloseReason },
}

/// Handshake failures.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PeerInitError {
    #[error("generating init cert failed: {0}")]
    Cert(strims_crypto::Error),
    #[error("writing peer init: {0}")]
    Write(StreamError),
    #[error("reading peer init: {0}")]
    Read(StreamError),
    #[error("peer cert verification: {0}")]
    Certificate(CertificateErrors),
    /// No certificate, or one without a parent.
    #[error("invalid peer certificate")]
    InvalidPeerCertificate,
    #[error("peer host id malformed")]
    MalformedHostId,
    /// The peer's host id is this host's: the link loops back to it.
    #[error("the peer is this host")]
    OwnHost,
}

/// A frame delivered by [`Host::handle_link_input`].
#[derive(Debug)]
pub struct InboundFrame<'a, T> {
    pub peer: PeerKey,
    pub host_id: ID,
    pub port: u16,
    /// The tag given to [`Host::set_handler`] for this port.
    pub handler: T,
    pub body: &'a [u8],
}

/// A send opportunity offered by [`Host::poll_egress`].
#[derive(Debug)]
pub struct EgressFrame<'a, T> {
    pub writer: WriterId,
    pub peer: PeerKey,
    /// The destination port.
    pub port: u16,
    /// The tag given to [`Host::open_writer`].
    pub tag: T,
    /// Room for one frame body, inside the link's outbound buffer.
    pub body: &'a mut [u8],
}

/// An owner's answer to an [`EgressFrame`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Body bytes written. 0 sends nothing and clears the backlog.
    pub len: usize,
    /// Whether the owner has another frame ready. `false` clears the
    /// writer's backlog.
    pub more: bool,
}

impl Fill {
    /// Nothing to send.
    pub const NONE: Fill = Fill {
        len: 0,
        more: false,
    };

    /// `len` bytes and nothing after them.
    pub fn last(len: usize) -> Fill {
        Fill { len, more: false }
    }

    /// `len` bytes and more to come.
    pub fn more(len: usize) -> Fill {
        Fill { len, more: true }
    }
}

/// The peer on an established link.
#[derive(Debug)]
pub struct Peer<T> {
    host_id: ID,
    /// The peer's profile certificate (the parent of its host certificate).
    certificate: Certificate,
    protocol_version: u32,
    node_platform: String,
    node_version: String,
    /// Bounded by the owners' calls, not by peer input.
    handlers: BTreeMap<u16, T>,
    reservations: BTreeSet<u16>,
    writers: BTreeSet<WriterId>,
    /// Whether a keepalive frame has arrived from the peer.
    sends_keepalives: bool,
}

impl<T: Copy> Peer<T> {
    /// The peer's host certificate key.
    pub fn host_id(&self) -> ID {
        self.host_id
    }

    /// The peer's profile certificate.
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    pub fn node_platform(&self) -> &str {
        &self.node_platform
    }

    pub fn node_version(&self) -> &str {
        &self.node_version
    }

    /// The handler tag for `port`.
    pub fn handler(&self, port: u16) -> Option<T> {
        self.handlers.get(&port).copied()
    }

    /// The peer's writers.
    pub fn writers(&self) -> impl Iterator<Item = WriterId> + '_ {
        self.writers.iter().copied()
    }

    /// Whether a keepalive frame has arrived from the peer. A peer that
    /// sends keepalives writes at least once per keepalive interval, so a
    /// long silence means the link is dead. While this is false an idle link
    /// may stay silent: a driver that keeps such links can skip its read
    /// timeout for them and rely on TCP keepalive to find dead ones.
    pub fn sends_keepalives(&self) -> bool {
        self.sends_keepalives
    }
}

#[derive(Debug)]
enum Stage<T> {
    /// Waiting for the peer's `AESLinkInit`.
    Aes(AesLinkHandshake),
    /// Waiting for the peer's `PeerInit`.
    PeerInit(MessageReader),
    Established(Box<Peer<T>>),
}

#[derive(Debug)]
struct Link<T> {
    kind: LinkKind,
    mtu: usize,
    send_buffer: usize,
    stage: Stage<T>,
    aes: Option<AesLink>,
    /// Handshake bytes not yet transmitted, sent before any frame. Bounded
    /// by the three handshake messages (each at most `2 + 65535`).
    control: Vec<u8>,
    /// Allocated when the peer is established.
    out: Option<OutBuf>,
    decoder: FrameDecoder,
    /// Whether the link's writers are blocked on a full `out`.
    blocked: bool,
    metrics: LinkMetrics,
    /// When [`Host::add_link`] ran: starts the handshake deadline and
    /// orders links in the handshake for eviction.
    added: Time,
    /// When vnic last queued bytes on the link, for keepalives.
    last_send: Time,
    /// This link's entry in [`Host`]'s `timers`.
    timer: Option<Time>,
}

impl<T> Link<T> {
    fn established(&self) -> bool {
        matches!(self.stage, Stage::Established(_))
    }
}

#[derive(Debug)]
struct Writer<T> {
    peer: PeerKey,
    port: u16,
    /// For channels: the port whose handler this channel installed.
    read_port: Option<u16>,
    session: SessionId,
    tag: T,
}

/// The virtual NIC. `T` is the owners' routing tag for
/// port handlers and writers (for example an enum naming the ppspp runner,
/// a vpn network or an rpc channel).
#[derive(Debug)]
pub struct Host<T: Copy = ()> {
    profile_key: Key,
    /// The profile key as X25519, for AES links.
    profile_x25519: Key,
    key: Key,
    id: ID,
    label: String,
    node_platform: String,
    node_version: String,
    rng: DetRng,
    /// Links in any stage. Bounded by the driver's connections.
    links: SlotMap<PeerKey, Link<T>>,
    /// Established peers by host id: at most one per id.
    peers: BTreeMap<ID, PeerKey>,
    /// Bounded by the owners' calls.
    writers: SlotMap<WriterId, Writer<T>>,
    sessions: BTreeMap<SessionId, WriterId>,
    qos: Control,
    max_peers: usize,
    /// Bounded by link state changes between polls.
    events: VecDeque<Event>,
    metrics: HostMetrics,
    /// Link deadlines: the handshake deadline before `PeerInit`, the next
    /// keepalive check after. At most one entry per link, so bounded by
    /// `links`.
    timers: BTreeSet<(Time, PeerKey)>,
    handshake_timeout: Option<Duration>,
    keepalive_interval: Option<Duration>,
    /// 0 means unbounded.
    max_pending_links: usize,
}

impl<T: Copy> Host<T> {
    /// Creates a host with a fresh host key drawn from `rng`. Fails unless
    /// `profile_key` is an ed25519 signing key.
    pub fn new(profile_key: Key, config: HostConfig, mut rng: DetRng) -> Result<Host<T>, Error> {
        strims_crypto::Signer::new(&profile_key)?;
        let profile_x25519 = key_to_curve25519(&profile_key)?;
        let key = strims_crypto::generate_key(&mut rng);
        let (id, _) =
            ID::unmarshal(&key.public).map_err(|_| strims_crypto::Error::InvalidKeyLength)?;
        let mut qos = Control::new();
        if let Some(limit) = config.rate_limit {
            qos.set_rate_limit(limit);
        }
        Ok(Host {
            profile_key,
            profile_x25519,
            key,
            id,
            label: config.label,
            node_platform: config.node_platform,
            node_version: config.node_version,
            rng,
            links: SlotMap::with_key(),
            peers: BTreeMap::new(),
            writers: SlotMap::with_key(),
            sessions: BTreeMap::new(),
            qos,
            max_peers: 0,
            events: VecDeque::new(),
            metrics: HostMetrics::default(),
            timers: BTreeSet::new(),
            handshake_timeout: Some(DEFAULT_HANDSHAKE_TIMEOUT),
            keepalive_interval: Some(DEFAULT_KEEPALIVE_INTERVAL),
            max_pending_links: DEFAULT_MAX_PENDING_LINKS,
        })
    }

    /// The host key's public key.
    pub fn id(&self) -> ID {
        self.id
    }

    /// The host key.
    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn profile_key(&self) -> &Key {
        &self.profile_key
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    /// A host certificate for `PeerInit`: the host key, signed by the
    /// profile key, whose self-signed certificate is the parent. Both are
    /// valid for an hour.
    pub fn cert(&mut self, now: Time) -> Result<Certificate, Error> {
        let usage = KeyUsage::Sign as i32;
        let profile_cert = new_self_signed_certificate(
            &self.profile_key,
            usage,
            HOST_CERT_VALID_DURATION,
            "",
            now,
            &mut self.rng,
        )
        .map_err(PeerInitError::Cert)?;
        let csr =
            new_certificate_request(&self.key, usage, &self.label).map_err(PeerInitError::Cert)?;
        let mut cert = sign_certificate_request(
            &csr,
            HOST_CERT_VALID_DURATION,
            &self.profile_key,
            now,
            &mut self.rng,
        )
        .map_err(PeerInitError::Cert)?;
        cert.parent_oneof = Some(ParentOneof::Parent(Box::new(profile_cert)));
        Ok(cert)
    }

    /// The `PeerInit` this host sends, with a fresh certificate.
    pub fn peer_init(&mut self, now: Time) -> Result<PeerInit, Error> {
        Ok(PeerInit {
            protocol_version: PROTOCOL_VERSION,
            certificate: Some(self.cert(now)?),
            node_platform: self.node_platform.clone(),
            node_version: self.node_version.clone(),
        })
    }

    // --- links ----------------------------------------------------------

    /// Starts a link over a new transport connection and queues its first
    /// handshake bytes. The handshake completes in
    /// [`Host::handle_link_input`].
    ///
    /// The handshake must complete within [`Host::handshake_timeout`]. When
    /// [`Host::max_pending_links`] links are already in the handshake, the
    /// oldest of them is closed to make room
    /// (`LinkClosed { reason: Failed(TooManyPendingLinks) }`).
    pub fn add_link(&mut self, now: Time, config: LinkConfig) -> Result<PeerKey, Error> {
        if config.mtu <= FRAME_HEADER_LEN || config.mtu > FRAME_HEADER_LEN + MAX_FRAME_BODY_LEN {
            return Err(Error::InvalidMtu(config.mtu));
        }
        if config.send_buffer < config.mtu {
            return Err(Error::SendBufferTooSmall(config.send_buffer));
        }
        let mut control = Vec::new();
        if let Some(peer_key) = config.tcp_mux_init {
            let init = TcpMuxInit {
                protocol_version: PROTOCOL_VERSION,
                peer_key: peer_key.to_vec(),
            };
            encode_stream_message(&init, &mut control)?;
        }
        let stage = match config.aes {
            Some(aes) => {
                let hs =
                    AesLinkHandshake::new(self.profile_x25519.clone(), aes.peer_key, &mut self.rng);
                hs.write_init(&mut control)?;
                Stage::Aes(hs)
            }
            None => Stage::PeerInit(MessageReader::new()),
        };
        let plain = matches!(stage, Stage::PeerInit(_));
        self.evict_pending_links();
        let key = self.links.insert(Link {
            kind: config.kind,
            mtu: config.mtu,
            send_buffer: config.send_buffer,
            stage,
            aes: None,
            control,
            out: None,
            decoder: FrameDecoder::new(),
            blocked: false,
            metrics: LinkMetrics::default(),
            added: now,
            last_send: now,
            timer: None,
        });
        if plain && let Err(e) = self.queue_peer_init(key, now) {
            self.links.remove(key);
            return Err(e);
        }
        self.arm_timer(key);
        Ok(key)
    }

    /// Closes the oldest links in the handshake until there is room for one
    /// more.
    fn evict_pending_links(&mut self) {
        if self.max_pending_links == 0 {
            return;
        }
        loop {
            let mut pending = 0;
            let mut oldest: Option<(Time, PeerKey)> = None;
            for (k, l) in &self.links {
                if !l.established() {
                    pending += 1;
                    if oldest.is_none_or(|o| (l.added, k) < o) {
                        oldest = Some((l.added, k));
                    }
                }
            }
            let Some((_, k)) = oldest.filter(|_| pending >= self.max_pending_links) else {
                return;
            };
            self.metrics.handshake_evict_count += 1;
            self.close_link(k, Some(CloseReason::Failed(Error::TooManyPendingLinks)));
        }
    }

    /// Feeds bytes read from the link. `data` is decrypted in place on AES
    /// links. Frames for ports with a handler go to `dispatch`, in order.
    ///
    /// Returns the bytes consumed. When the `PeerInit` handshake completes,
    /// processing stops right after it so that owners can register their
    /// handlers on [`Event::PeerAdded`] before any frame is read. The driver
    /// drains events and calls again with the rest:
    ///
    /// ```text
    /// let mut off = 0;
    /// while off < n {
    ///     off += host.handle_link_input(link, now, &mut buf[off..n], &mut dispatch)?;
    ///     while let Some(event) = host.poll_event() { /* ... */ }
    /// }
    /// ```
    ///
    /// An error means the handshake failed, or its deadline passed
    /// ([`Error::HandshakeTimeout`], checked here too in case the driver
    /// hasn't run [`Host::handle_timeout`] yet): the link is closed and an
    /// [`Event::LinkClosed`] is queued.
    ///
    /// Frames on [`ports::KEEPALIVE_PORT`] are dropped and counted
    /// ([`HostMetrics::keepalive_read_count`]).
    pub fn handle_link_input(
        &mut self,
        link: PeerKey,
        now: Time,
        data: &mut [u8],
        dispatch: &mut impl FnMut(InboundFrame<'_, T>),
    ) -> Result<usize, Error> {
        let l = self.links.get(link).ok_or(Error::UnknownLink)?;
        // The deadline holds even if the driver hasn't run the timer yet.
        let res = if self.handshake_expired(l, now) {
            self.metrics.handshake_timeout_count += 1;
            Err(Error::HandshakeTimeout)
        } else {
            self.link_input(link, now, data, dispatch)
        };
        if let Err(e) = &res {
            self.close_link(link, Some(CloseReason::Failed(e.clone())));
        }
        res
    }

    fn link_input(
        &mut self,
        key: PeerKey,
        now: Time,
        data: &mut [u8],
        dispatch: &mut impl FnMut(InboundFrame<'_, T>),
    ) -> Result<usize, Error> {
        let len = data.len();
        let mut off = 0;
        loop {
            let link = self.links.get_mut(key).ok_or(Error::UnknownLink)?;
            match &mut link.stage {
                Stage::Aes(hs) => {
                    let (n, cipher) = hs.handle_input(data.get(off..).unwrap_or_default())?;
                    off += n;
                    let Some(cipher) = cipher else {
                        break;
                    };
                    link.aes = Some(cipher);
                    link.stage = Stage::PeerInit(MessageReader::new());
                    self.queue_peer_init(key, now)?;
                }
                Stage::PeerInit(reader) => {
                    let mut init = None;
                    while off < len && init.is_none() {
                        let k = reader.needed().min(len - off);
                        let Some(chunk) = data.get_mut(off..off + k) else {
                            break;
                        };
                        if let Some(aes) = &mut link.aes {
                            aes.decrypt(chunk);
                        }
                        let pushed = reader.push(chunk);
                        off += pushed.consumed;
                        if let Some(body) = pushed.message {
                            init = Some(
                                decode_stream_message::<PeerInit>(body)
                                    .map_err(PeerInitError::Read),
                            );
                        }
                    }
                    if let Some(init) = init {
                        self.establish(key, now, init?)?;
                    }
                    break;
                }
                Stage::Established(peer) => {
                    let rest = data.get_mut(off..).unwrap_or_default();
                    if let Some(aes) = &mut link.aes {
                        aes.decrypt(rest);
                    }
                    let metrics = &mut self.metrics;
                    link.decoder.decode(rest, &mut |f| {
                        if f.port == ports::KEEPALIVE_PORT {
                            metrics.keepalive_read_count += 1;
                            peer.sends_keepalives = true;
                            return;
                        }
                        metrics.frame_read_count += 1;
                        metrics.frame_read_bytes += f.body.len() as u64;
                        match peer.handlers.get(&f.port) {
                            Some(&handler) => dispatch(InboundFrame {
                                peer: key,
                                host_id: peer.host_id,
                                port: f.port,
                                handler,
                                body: f.body,
                            }),
                            None => metrics.frame_handler_not_found_count += 1,
                        }
                    });
                    off = len;
                    break;
                }
            }
        }
        if let Some(link) = self.links.get_mut(key) {
            link.metrics.read_bytes += off as u64;
            link.metrics.read_count += 1;
        }
        Ok(off)
    }

    /// Appends this host's `PeerInit`, encrypted on AES links.
    fn queue_peer_init(&mut self, key: PeerKey, now: Time) -> Result<(), Error> {
        let init = self.peer_init(now)?;
        let mut bytes = Vec::new();
        encode_stream_message(&init, &mut bytes).map_err(PeerInitError::Write)?;
        let link = self.links.get_mut(key).ok_or(Error::UnknownLink)?;
        if let Some(aes) = &mut link.aes {
            aes.encrypt(&mut bytes);
        }
        link.control.extend_from_slice(&bytes);
        Ok(())
    }

    /// Checks the peer's `PeerInit` and makes the link a peer.
    fn establish(&mut self, key: PeerKey, now: Time, init: PeerInit) -> Result<(), Error> {
        let cert = init
            .certificate
            .ok_or(PeerInitError::InvalidPeerCertificate)?;
        verify_certificate(&cert, now).map_err(PeerInitError::Certificate)?;
        let peer_cert = certificate_parent(&cert)
            .cloned()
            .ok_or(PeerInitError::InvalidPeerCertificate)?;
        let (host_id, _) = ID::unmarshal(&cert.key).map_err(|_| PeerInitError::MalformedHostId)?;
        if host_id == self.id {
            return Err(PeerInitError::OwnHost.into());
        }

        let link = self.links.get_mut(key).ok_or(Error::UnknownLink)?;
        link.stage = Stage::Established(Box::new(Peer {
            host_id,
            certificate: peer_cert,
            protocol_version: init.protocol_version,
            node_platform: init.node_platform,
            node_version: init.node_version,
            handlers: BTreeMap::new(),
            reservations: BTreeSet::new(),
            writers: BTreeSet::new(),
            sends_keepalives: false,
        }));
        link.out = Some(OutBuf::new(link.send_buffer));
        link.blocked = false;
        link.last_send = now;
        self.metrics.links_active += 1;
        // The handshake deadline gives way to keepalives.
        self.arm_timer(key);

        if let Some(prev) = self.peers.insert(host_id, key)
            && prev != key
        {
            self.close_link(prev, Some(CloseReason::Replaced));
        }
        self.events
            .push_back(Event::PeerAdded { peer: key, host_id });
        Ok(())
    }

    /// The bytes to send next on the link: handshake bytes first, then
    /// frames. Empty when there is nothing to send. Message-oriented
    /// transports send at most the link MTU per message.
    pub fn poll_link_transmit(&self, link: PeerKey) -> Result<&[u8], Error> {
        let l = self.links.get(link).ok_or(Error::UnknownLink)?;
        if !l.control.is_empty() {
            return Ok(&l.control);
        }
        Ok(l.out.as_ref().map(OutBuf::pending).unwrap_or_default())
    }

    /// Reports that the first `n` bytes from [`Host::poll_link_transmit`]
    /// were sent. Unblocks the link's writers once an MTU is free.
    pub fn advance_link_transmit(&mut self, link: PeerKey, n: usize) -> Result<(), Error> {
        let l = self.links.get_mut(link).ok_or(Error::UnknownLink)?;
        if !l.control.is_empty() {
            if n > l.control.len() {
                return Err(Error::AdvancePastEnd);
            }
            l.control.drain(..n);
            if l.control.is_empty() {
                l.control = Vec::new();
            }
        } else {
            let pending = l.out.as_ref().map_or(0, OutBuf::len);
            if n > pending {
                return Err(Error::AdvancePastEnd);
            }
            if let Some(out) = &mut l.out {
                out.advance(n);
            }
        }
        l.metrics.write_bytes += n as u64;
        l.metrics.write_count += 1;
        update_link_blocked(&mut self.links, &self.writers, &mut self.qos, link);
        Ok(())
    }

    /// Links with bytes waiting in [`Host::poll_link_transmit`].
    pub fn links_with_transmit(&self) -> impl Iterator<Item = PeerKey> + '_ {
        self.links.iter().filter_map(|(k, l)| {
            let pending = !l.control.is_empty() || l.out.as_ref().is_some_and(|o| o.len() > 0);
            pending.then_some(k)
        })
    }

    /// The driver's transport closed or failed. Returns whether the link
    /// existed.
    pub fn remove_link(&mut self, link: PeerKey) -> bool {
        self.close_link(link, None)
    }

    /// Closes a link from the owner's side.
    pub fn close_peer(&mut self, link: PeerKey) -> bool {
        self.close_link(link, Some(CloseReason::Closed))
    }

    /// Closes every link.
    pub fn close(&mut self) {
        let keys: Vec<PeerKey> = self.links.keys().collect();
        for k in keys {
            self.close_link(k, Some(CloseReason::Closed));
        }
    }

    fn close_link(&mut self, key: PeerKey, reason: Option<CloseReason>) -> bool {
        let Some(link) = self.links.remove(key) else {
            return false;
        };
        if let Some(t) = link.timer {
            self.timers.remove(&(t, key));
        }
        if let Stage::Established(peer) = link.stage {
            for w in &peer.writers {
                if let Some(wr) = self.writers.remove(*w) {
                    self.sessions.remove(&wr.session);
                    let _ = self.qos.remove(wr.session);
                }
            }
            // The entry may already name the link that replaced this one.
            if self.peers.get(&peer.host_id) == Some(&key) {
                self.peers.remove(&peer.host_id);
            }
            self.metrics.links_active = self.metrics.links_active.saturating_sub(1);
            self.events.push_back(Event::PeerRemoved {
                peer: key,
                host_id: peer.host_id,
            });
        }
        if let Some(reason) = reason {
            self.events
                .push_back(Event::LinkClosed { link: key, reason });
        }
        true
    }

    /// Whether the link exists (in any stage).
    pub fn has_link(&self, link: PeerKey) -> bool {
        self.links.contains_key(link)
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Links still in the handshake (added but without a peer yet).
    pub fn pending_link_count(&self) -> usize {
        self.links.values().filter(|l| !l.established()).count()
    }

    pub fn link_kind(&self, link: PeerKey) -> Option<LinkKind> {
        self.links.get(link).map(|l| l.kind)
    }

    pub fn link_mtu(&self, link: PeerKey) -> Option<usize> {
        self.links.get(link).map(|l| l.mtu)
    }

    pub fn link_metrics(&self, link: PeerKey) -> Option<LinkMetrics> {
        self.links.get(link).map(|l| l.metrics)
    }

    /// Bytes held for a frame or handshake message split across reads.
    pub fn link_input_buffered(&self, link: PeerKey) -> Option<usize> {
        self.links.get(link).map(|l| match &l.stage {
            Stage::Aes(hs) => hs.buffered(),
            Stage::PeerInit(r) => r.buffered(),
            Stage::Established(_) => l.decoder.buffered(),
        })
    }

    // --- peers ----------------------------------------------------------

    /// Whether a peer with this host id is established.
    pub fn has_peer(&self, host_id: &ID) -> bool {
        self.peers.contains_key(host_id)
    }

    /// The established link to this host id.
    pub fn get_peer(&self, host_id: &ID) -> Option<PeerKey> {
        self.peers.get(host_id).copied()
    }

    /// The peer on an established link.
    pub fn peer(&self, peer: PeerKey) -> Option<&Peer<T>> {
        match &self.links.get(peer)?.stage {
            Stage::Established(p) => Some(p),
            _ => None,
        }
    }

    /// Established peers, ordered by host id.
    pub fn peers(&self) -> impl Iterator<Item = (PeerKey, &Peer<T>)> + '_ {
        self.peers
            .values()
            .filter_map(|&k| self.peer(k).map(|p| (k, p)))
    }

    /// The number of established peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Sets the peer limit vpn's peer exchange reads; vnic doesn't enforce
    /// it. 0 means unlimited (`i32::MAX`).
    pub fn set_max_peers(&mut self, n: u32) {
        self.max_peers = if n == 0 {
            i32::MAX as usize
        } else {
            n as usize
        };
    }

    /// The peer limit. 0 until [`Host::set_max_peers`] is called, so peer
    /// exchange accepts nobody until then.
    pub fn max_peers(&self) -> usize {
        self.max_peers
    }

    fn peer_mut(&mut self, peer: PeerKey) -> Result<&mut Peer<T>, Error> {
        match &mut self.links.get_mut(peer).ok_or(Error::UnknownLink)?.stage {
            Stage::Established(p) => Ok(p),
            _ => Err(Error::PeerNotEstablished),
        }
    }

    // --- ports ----------------------------------------------------------

    /// Routes frames arriving on `port` to the dispatch callback with
    /// `handler`. [`Error::PortInUse`] if the port has a handler or is
    /// [`ports::KEEPALIVE_PORT`].
    pub fn set_handler(&mut self, peer: PeerKey, port: u16, handler: T) -> Result<(), Error> {
        let p = self.peer_mut(peer)?;
        if port == ports::KEEPALIVE_PORT || p.handlers.contains_key(&port) {
            return Err(Error::PortInUse(port));
        }
        p.handlers.insert(port, handler);
        Ok(())
    }

    /// Removes the port's handler and returns it.
    pub fn remove_handler(&mut self, peer: PeerKey, port: u16) -> Option<T> {
        self.peer_mut(peer).ok()?.handlers.remove(&port)
    }

    /// The handler tag for `port` on the peer.
    pub fn handler(&self, peer: PeerKey, port: u16) -> Option<T> {
        self.peer(peer)?.handler(port)
    }

    /// A random port at or above [`ports::RESERVED_PORT_COUNT`] with no
    /// handler or reservation, now reserved.
    pub fn reserve_port(&mut self, peer: PeerKey) -> Result<u16, Error> {
        let Host { links, rng, .. } = self;
        let p = match &mut links.get_mut(peer).ok_or(Error::UnknownLink)?.stage {
            Stage::Established(p) => p,
            _ => return Err(Error::PeerNotEstablished),
        };
        let free = |p: &Peer<T>, port: u16| {
            !p.handlers.contains_key(&port) && !p.reservations.contains(&port)
        };
        let span = u32::from(u16::MAX - ports::RESERVED_PORT_COUNT);
        // Random draws, then a scan so a full table can't spin.
        for _ in 0..64 {
            let port = random_below(rng, span) as u16 + ports::RESERVED_PORT_COUNT;
            if free(p, port) {
                p.reservations.insert(port);
                return Ok(port);
            }
        }
        let start = random_below(rng, span);
        for i in 0..span {
            let port = ((start + i) % span) as u16 + ports::RESERVED_PORT_COUNT;
            if free(p, port) {
                p.reservations.insert(port);
                return Ok(port);
            }
        }
        Err(Error::NoFreePorts)
    }

    /// Releases a port from [`Host::reserve_port`].
    pub fn release_port(&mut self, peer: PeerKey, port: u16) {
        if let Ok(p) = self.peer_mut(peer) {
            p.reservations.remove(&port);
        }
    }

    // --- writers and channels --------------------------------------------

    /// Opens a writer sending frames to `port` on the peer, as a QoS
    /// session of weight [`WRITER_QOS_WEIGHT`] under `class`. The session
    /// starts idle; see [`Host::set_backlog`]. [`ports::KEEPALIVE_PORT`] is
    /// refused with [`Error::PortInUse`].
    pub fn open_writer(
        &mut self,
        peer: PeerKey,
        port: u16,
        class: ClassId,
        tag: T,
    ) -> Result<WriterId, Error> {
        self.open_writer_inner(peer, port, None, class, tag)
    }

    fn open_writer_inner(
        &mut self,
        peer: PeerKey,
        port: u16,
        read_port: Option<u16>,
        class: ClassId,
        tag: T,
    ) -> Result<WriterId, Error> {
        let blocked = self.links.get(peer).ok_or(Error::UnknownLink)?.blocked;
        self.peer_mut(peer)?;
        if port == ports::KEEPALIVE_PORT {
            return Err(Error::PortInUse(port));
        }
        let session = self.qos.add_session(class, WRITER_QOS_WEIGHT)?;
        if blocked {
            self.qos.set_blocked(session, true)?;
        }
        let w = self.writers.insert(Writer {
            peer,
            port,
            read_port,
            session,
            tag,
        });
        self.sessions.insert(session, w);
        self.peer_mut(peer)?.writers.insert(w);
        Ok(w)
    }

    /// Closes a writer and its QoS session.
    pub fn close_writer(&mut self, w: WriterId) -> Result<(), Error> {
        let wr = self.writers.remove(w).ok_or(Error::UnknownWriter)?;
        self.sessions.remove(&wr.session);
        let _ = self.qos.remove(wr.session);
        if let Ok(p) = self.peer_mut(wr.peer) {
            p.writers.remove(&w);
        }
        Ok(())
    }

    /// A channel on one port: frames arriving on `port` go to `tag`, and the
    /// returned writer sends to `port`. On failure nothing stays installed.
    pub fn channel(
        &mut self,
        peer: PeerKey,
        port: u16,
        class: ClassId,
        tag: T,
    ) -> Result<WriterId, Error> {
        self.set_handler(peer, port, tag)?;
        match self.open_writer_inner(peer, port, Some(port), class, tag) {
            Ok(w) => Ok(w),
            Err(e) => {
                self.remove_handler(peer, port);
                Err(e)
            }
        }
    }

    /// A symmetric channel pair. On failure nothing stays installed.
    ///
    /// - The first writer sends to `port0` and receives what arrives on
    ///   `port1` (handler `tag0`).
    /// - The second sends to `port1` and receives what arrives on `port0`
    ///   (handler `tag1`).
    ///
    /// Two hosts that both call this with the same ports connect each one's
    /// first channel to the other's second (rpc client to rpc server).
    pub fn channel_pair(
        &mut self,
        peer: PeerKey,
        port0: u16,
        port1: u16,
        class: ClassId,
        tag0: T,
        tag1: T,
    ) -> Result<(WriterId, WriterId), Error> {
        self.set_handler(peer, port1, tag0)?;
        if let Err(e) = self.set_handler(peer, port0, tag1) {
            self.remove_handler(peer, port1);
            return Err(e);
        }
        let w0 = self.open_writer_inner(peer, port0, Some(port1), class, tag0);
        let w1 = self.open_writer_inner(peer, port1, Some(port0), class, tag1);
        match (w0, w1) {
            (Ok(w0), Ok(w1)) => Ok((w0, w1)),
            (w0, w1) => {
                for w in [w0.ok(), w1.ok()].into_iter().flatten() {
                    let _ = self.close_writer(w);
                }
                self.remove_handler(peer, port0);
                self.remove_handler(peer, port1);
                Err(Error::UnknownWriter)
            }
        }
    }

    /// Closes a channel: removes the handler it installed (its read port)
    /// and its writer.
    pub fn close_channel(&mut self, w: WriterId) -> Result<(), Error> {
        let wr = self.writers.get(w).ok_or(Error::UnknownWriter)?;
        let (peer, read_port) = (wr.peer, wr.read_port);
        if let Some(port) = read_port {
            self.remove_handler(peer, port);
        }
        self.close_writer(w)
    }

    /// The writer's peer and destination port.
    pub fn writer(&self, w: WriterId) -> Option<(PeerKey, u16)> {
        self.writers.get(w).map(|wr| (wr.peer, wr.port))
    }

    /// The writer's QoS session.
    pub fn writer_session(&self, w: WriterId) -> Option<SessionId> {
        self.writers.get(w).map(|wr| wr.session)
    }

    /// The largest frame body the writer can send.
    pub fn writer_max_body(&self, w: WriterId) -> Result<usize, Error> {
        let wr = self.writers.get(w).ok_or(Error::UnknownWriter)?;
        let l = self.links.get(wr.peer).ok_or(Error::UnknownLink)?;
        Ok(l.mtu - FRAME_HEADER_LEN)
    }

    /// Sets the weight of the writer's QoS session.
    pub fn set_writer_weight(&mut self, w: WriterId, weight: u64) -> Result<(), Error> {
        let wr = self.writers.get(w).ok_or(Error::UnknownWriter)?;
        self.qos.set_weight(wr.session, weight)?;
        Ok(())
    }

    /// Marks the writer as having data, with a full-MTU hint. Repeating it
    /// is cheap.
    pub fn set_backlog(&mut self, w: WriterId) -> Result<(), Error> {
        self.set_backlog_hint(w, usize::MAX)
    }

    /// Like [`Host::set_backlog`] with the expected body size of the next
    /// frame (clamped to the MTU).
    pub fn set_backlog_hint(&mut self, w: WriterId, body_len: usize) -> Result<(), Error> {
        let wr = self.writers.get(w).ok_or(Error::UnknownWriter)?;
        let l = self.links.get(wr.peer).ok_or(Error::UnknownLink)?;
        let hint = body_len.min(l.mtu - FRAME_HEADER_LEN) + FRAME_HEADER_LEN;
        self.qos.set_backlog(wr.session, hint)?;
        Ok(())
    }

    /// Marks the writer as having nothing to send.
    pub fn clear_backlog(&mut self, w: WriterId) -> Result<(), Error> {
        let wr = self.writers.get(w).ok_or(Error::UnknownWriter)?;
        self.qos.clear_backlog(wr.session)?;
        Ok(())
    }

    // --- qos --------------------------------------------------------------

    /// The scheduler, read-only.
    pub fn qos(&self) -> &Control {
        &self.qos
    }

    pub fn qos_root(&self) -> ClassId {
        self.qos.root()
    }

    /// Adds a QoS class under `parent`.
    pub fn add_qos_class(&mut self, parent: ClassId, weight: u64) -> Result<ClassId, Error> {
        Ok(self.qos.add_class(parent, weight)?)
    }

    /// Removes a class with everything under it, closing the writers whose
    /// sessions it held (and the handlers their channels installed).
    pub fn remove_qos_class(&mut self, class: ClassId) -> Result<(), Error> {
        self.qos.remove(class)?;
        let dead: Vec<WriterId> = self
            .writers
            .iter()
            .filter(|(_, wr)| !self.qos.contains(wr.session))
            .map(|(w, _)| w)
            .collect();
        for w in dead {
            let _ = self.close_channel(w);
        }
        Ok(())
    }

    pub fn set_qos_weight(&mut self, class: ClassId, weight: u64) -> Result<(), Error> {
        Ok(self.qos.set_weight(class, weight)?)
    }

    /// Bytes per second; 0 stops egress.
    pub fn set_rate_limit(&mut self, limit: u64) {
        self.qos.set_rate_limit(limit);
    }

    // --- egress -----------------------------------------------------------

    /// Offers send opportunities in QoS order until no writer may send: for
    /// each, `fill` writes one frame body for the writer and returns what it
    /// wrote. vnic adds the header, encrypts AES links, and queues the frame
    /// on the link. Returns the number of frames queued.
    ///
    /// Call it after input, backlog changes, [`Host::advance_link_transmit`]
    /// and at [`Host::poll_timeout`]. Then send what the links have.
    ///
    /// It first runs [`Host::handle_timeout`] if a link deadline is due, so
    /// a driver that calls it at every [`Host::poll_timeout`] needs no
    /// separate `handle_timeout` call. Like `handle_timeout`, it can then
    /// queue [`Event::LinkClosed`] for links whose handshake timed out.
    pub fn poll_egress(
        &mut self,
        now: Time,
        fill: &mut impl FnMut(EgressFrame<'_, T>) -> Fill,
    ) -> usize {
        if self.timers.first().is_some_and(|&(t, _)| t <= now) {
            self.handle_timeout(now);
        }
        let Host {
            links,
            writers,
            sessions,
            qos,
            metrics,
            ..
        } = self;
        let mut frames = 0;
        while let Some((s, _)) = qos.dequeue(now) {
            let Some(wr) = sessions
                .get(&s)
                .and_then(|w| writers.get(*w).map(|wr| (*w, wr)))
            else {
                // invariant: every session belongs to a writer; drop strays.
                let _ = qos.remove(s);
                continue;
            };
            let (w, peer, port, tag) = (wr.0, wr.1.peer, wr.1.port, wr.1.tag);
            let Some(link) = links.get_mut(peer) else {
                let _ = qos.remove(s);
                continue;
            };
            let mtu = link.mtu;
            let Link {
                out,
                aes,
                last_send,
                ..
            } = link;
            let Some(slot) = out.as_mut().and_then(|o| o.slot(mtu)) else {
                // A full link's writers are blocked, so this is unexpected:
                // give the turn back without losing the backlog.
                let _ = qos.complete(s, 0, now);
                let _ = qos.set_backlog(s, mtu);
                update_link_blocked(links, writers, qos, peer);
                continue;
            };
            let filled = {
                let body = slot.get_mut(FRAME_HEADER_LEN..).unwrap_or_default();
                let max = body.len();
                let f = fill(EgressFrame {
                    writer: w,
                    peer,
                    port,
                    tag,
                    body,
                });
                Fill {
                    len: f.len.min(max),
                    more: f.more,
                }
            };
            if filled.len == 0 {
                let _ = qos.complete(s, 0, now);
                continue;
            }
            let frame_len = FRAME_HEADER_LEN + filled.len;
            // invariant: len <= mtu - 4 <= u16::MAX (checked in add_link).
            let length = u16::try_from(filled.len).unwrap_or(u16::MAX);
            if let Some(h) = slot.get_mut(..FRAME_HEADER_LEN) {
                h.copy_from_slice(&FrameHeader { port, length }.to_bytes());
            }
            if let (Some(aes), Some(frame)) = (aes.as_mut(), slot.get_mut(..frame_len)) {
                aes.encrypt(frame);
            }
            if let Some(o) = out.as_mut() {
                o.commit(frame_len);
            }
            *last_send = now;
            let _ = qos.complete(s, frame_len, now);
            if !filled.more {
                let _ = qos.clear_backlog(s);
            }
            metrics.frame_write_count += 1;
            metrics.frame_write_bytes += filled.len as u64;
            frames += 1;
            update_link_blocked(links, writers, qos, peer);
        }
        frames
    }

    /// When to call [`Host::handle_timeout`] and [`Host::poll_egress`] next
    /// if nothing else happens: the earliest of the rate limit's release
    /// time, a handshake deadline and a keepalive check. `None` when
    /// nothing is waiting.
    pub fn poll_timeout(&self) -> Option<Time> {
        let link = self.timers.first().map(|&(t, _)| t);
        match (self.qos.poll_timeout(), link) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Runs the link deadlines due at `now`:
    ///
    /// - a link still in the handshake [`Host::handshake_timeout`] after
    ///   [`Host::add_link`] is closed with
    ///   `LinkClosed { reason: Failed(Error::HandshakeTimeout) }`;
    /// - an established link that has queued nothing for
    ///   [`Host::keepalive_interval`] queues a keepalive frame (an empty
    ///   body on [`ports::KEEPALIVE_PORT`], 4 bytes; skipped while bytes are
    ///   still waiting to be sent).
    ///
    /// The rate limit needs no handling: call [`Host::poll_egress`] after
    /// this (it also runs this itself), drain events, and send what the
    /// links have.
    pub fn handle_timeout(&mut self, now: Time) {
        while let Some(&(at, key)) = self.timers.first() {
            if at > now {
                break;
            }
            self.timers.pop_first();
            let Some(link) = self.links.get(key) else {
                continue;
            };
            let expired = self.handshake_expired(link, now);
            let keepalive = link.established()
                && self
                    .keepalive_interval
                    .is_some_and(|i| link.last_send + i <= now);
            if let Some(link) = self.links.get_mut(key) {
                link.timer = None;
            }
            if keepalive {
                self.queue_keepalive(key, now);
            }
            if expired {
                self.metrics.handshake_timeout_count += 1;
                self.close_link(key, Some(CloseReason::Failed(Error::HandshakeTimeout)));
                continue;
            }
            // Re-arm at the link's next deadline, which is after `now`:
            // keepalives moved `last_send` to `now` or it was already later.
            self.arm_timer(key);
        }
    }

    /// Whether the link is still in the handshake past its deadline.
    fn handshake_expired(&self, link: &Link<T>, now: Time) -> bool {
        !link.established()
            && self
                .handshake_timeout
                .is_some_and(|t| link.added + t <= now)
    }

    /// Puts the link's entry in `timers` at its next deadline: the
    /// handshake deadline before `PeerInit`, the keepalive check after.
    /// Keepalive checks are lazy: frames move `last_send` without touching
    /// the timer, and the check re-arms from `last_send` when it runs.
    fn arm_timer(&mut self, key: PeerKey) {
        let Some(link) = self.links.get_mut(key) else {
            return;
        };
        let at = if link.established() {
            self.keepalive_interval.map(|i| link.last_send + i)
        } else {
            self.handshake_timeout.map(|t| link.added + t)
        };
        if link.timer == at {
            return;
        }
        if let Some(old) = link.timer.take() {
            self.timers.remove(&(old, key));
        }
        if let Some(at) = at {
            self.timers.insert((at, key));
            link.timer = Some(at);
        }
    }

    /// Queues a keepalive frame unless bytes are still waiting to be sent
    /// (they reach the peer first, and a stuck transport isn't helped by
    /// more). Either way the keepalive clock restarts at `now`.
    fn queue_keepalive(&mut self, key: PeerKey, now: Time) {
        let Host {
            links,
            writers,
            qos,
            metrics,
            ..
        } = self;
        let Some(link) = links.get_mut(key) else {
            return;
        };
        link.last_send = now;
        let Link {
            control, out, aes, ..
        } = link;
        let Some(out) = out.as_mut().filter(|o| o.len() == 0 && control.is_empty()) else {
            return;
        };
        // An empty buffer always has room for a header (mtu > 4).
        let Some(slot) = out.slot(FRAME_HEADER_LEN) else {
            return;
        };
        slot.copy_from_slice(&KEEPALIVE_FRAME);
        if let Some(aes) = aes.as_mut() {
            aes.encrypt(slot);
        }
        out.commit(FRAME_HEADER_LEN);
        metrics.keepalive_write_count += 1;
        update_link_blocked(links, writers, qos, key);
    }

    /// See [`Host::set_handshake_timeout`].
    pub fn handshake_timeout(&self) -> Option<Duration> {
        self.handshake_timeout
    }

    /// Sets how long a link may take from [`Host::add_link`] to the peer's
    /// `PeerInit`; `None` (or a duration that isn't positive) disables the
    /// deadline. Applies to links already in the handshake too. Default
    /// [`DEFAULT_HANDSHAKE_TIMEOUT`].
    pub fn set_handshake_timeout(&mut self, timeout: Option<Duration>) {
        self.handshake_timeout = timeout.filter(|t| *t > Duration::ZERO);
        self.rearm_timers();
    }

    /// See [`Host::set_keepalive_interval`].
    pub fn keepalive_interval(&self) -> Option<Duration> {
        self.keepalive_interval
    }

    /// Sets how long an established link may queue nothing before vnic
    /// queues a keepalive frame; `None` (or a duration that isn't positive)
    /// disables keepalives. Applies to established links too. Default
    /// [`DEFAULT_KEEPALIVE_INTERVAL`].
    pub fn set_keepalive_interval(&mut self, interval: Option<Duration>) {
        self.keepalive_interval = interval.filter(|i| *i > Duration::ZERO);
        self.rearm_timers();
    }

    /// See [`Host::set_max_pending_links`].
    pub fn max_pending_links(&self) -> usize {
        self.max_pending_links
    }

    /// Bounds the links in the handshake at once; 0 means unbounded.
    /// Default [`DEFAULT_MAX_PENDING_LINKS`].
    ///
    /// When the bound is reached, [`Host::add_link`] closes the oldest link
    /// in the handshake (`LinkClosed { reason: Failed(TooManyPendingLinks) }`)
    /// and accepts the new one, so a peer holding connections open without
    /// finishing the handshake can't lock others out for a whole handshake
    /// timeout: it has to keep opening connections faster than honest
    /// handshakes complete. Established links never count.
    pub fn set_max_pending_links(&mut self, n: usize) {
        self.max_pending_links = n;
    }

    fn rearm_timers(&mut self) {
        let keys: Vec<PeerKey> = self.links.keys().collect();
        for k in keys {
            self.arm_timer(k);
        }
    }

    /// The next event.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    pub fn metrics(&self) -> HostMetrics {
        self.metrics
    }
}

/// Blocks or unblocks a link's writers when its free space crosses an MTU.
fn update_link_blocked<T: Copy>(
    links: &mut SlotMap<PeerKey, Link<T>>,
    writers: &SlotMap<WriterId, Writer<T>>,
    qos: &mut Control,
    key: PeerKey,
) {
    let Some(link) = links.get_mut(key) else {
        return;
    };
    let Some(out) = &link.out else {
        return;
    };
    let blocked = out.free() < link.mtu;
    if blocked == link.blocked {
        return;
    }
    link.blocked = blocked;
    if let Stage::Established(peer) = &link.stage {
        for w in &peer.writers {
            if let Some(wr) = writers.get(*w) {
                let _ = qos.set_blocked(wr.session, blocked);
            }
        }
    }
}

/// Uniform in `0..n` (`n > 0`).
fn random_below(rng: &mut DetRng, n: u32) -> u32 {
    let n = u64::from(n.max(1));
    // Rejection sampling below the largest multiple of n.
    let limit = (1u64 << 32) / n * n;
    loop {
        let v = u64::from(rand_core::Rng::next_u32(rng));
        if v < limit {
            // invariant: v % n < n <= u32::MAX.
            return (v % n) as u32;
        }
    }
}
