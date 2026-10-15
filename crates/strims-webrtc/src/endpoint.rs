// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The WebRTC interface: peer connections, each carrying one vnic link,
//! over UDP sockets the driver owns.
//!
//! One [`WebRtcEndpoint`] holds every connection. Datagrams from all
//! sockets go to [`WebRtcEndpoint::handle_udp_input`], which routes STUN
//! binding requests by ICE username (the local ufrag), STUN responses by
//! transaction, and DTLS by the (local, remote) address pair that last
//! carried authenticated STUN for a connection. So any number of
//! connections can share one socket, or each can have its own.
//!
//! Two connections between the same pair of sockets can't be told apart
//! for DTLS: the one that last authenticated STUN gets the datagrams. vpn
//! peer exchange never links a host it is already peered with, so this
//! needs no handling.
//!
//! str0m discovers no addresses. Host candidates are the driver's sockets,
//! and server-reflexive candidates come from binding requests sent from
//! each host socket to each STUN server ([`crate::stun`]).

// str0m's API is in `Instant`s; they are made only by `crate::clock`.
#![allow(clippy::disallowed_types)]

use std::collections::{BTreeMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::task::Poll;

use slotmap::SlotMap;
use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
use str0m::channel::{ChannelConfig, ChannelId, Reliability};
use str0m::config::DtlsCert;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, IceConnectionState, IceCreds, Input, Output, Rtc};
use strims_base::rng::{DetRng, fork};
use strims_base::time::{Duration, Time};
use strims_vnic::webrtc::{
    DATA_CHANNEL_INIT, DATA_CHANNEL_MTU, DEFAULT_ICE_SERVERS, RemoteDescriptionAction, SdpType,
    SessionDescription, WebRtcSignaling,
};

use crate::clock::{Clock, ceil_precision};
use crate::stun::{self, BindingResponse, TransactionId};

slotmap::new_key_type! {
    /// A peer connection, carrying one vnic link.
    pub struct ConnId;
}

/// Endpoint settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointConfig {
    /// ICE server URLs. `None` means [`DEFAULT_ICE_SERVERS`];
    /// `Some(vec![])` means none. Only `stun:` URLs are used
    /// (see [`crate::stun::parse_stun_url`]); the driver resolves them and
    /// passes the addresses to [`WebRtcEndpoint::set_stun_servers`].
    pub ice_servers: Option<Vec<String>>,
    /// The IP that host candidates on sockets of its address family
    /// advertise (a 1:1 NAT). Datagrams arriving on such sockets are
    /// matched against it, so a socket bound to the wildcard address needs
    /// it.
    pub host_ip: Option<IpAddr>,
    /// How long gathering waits for STUN servers.
    pub stun_gather_timeout: Duration,
    /// A connection whose data channel hasn't opened this long after it was
    /// created is closed with [`CloseReason::ConnectTimeout`]. `None` leaves
    /// it to the owner.
    pub connect_timeout: Option<Duration>,
    /// How long ICE may stay disconnected before the connection closes with
    /// [`CloseReason::IceFailed`].
    pub failed_timeout: Duration,
    /// Bytes the data channel may hold unacknowledged (str0m's
    /// `sctp_max_buffered_amount`). It caps throughput at about this much
    /// per round trip; the receiver's SCTP window (1 MiB in sctp-proto)
    /// caps it too. At least `2 * DATA_CHANNEL_MTU`.
    pub send_buffer: usize,
    /// The most connections at once. Each vpn peer exchange creates one,
    /// so this bounds what remote peers can make a node hold.
    pub max_connections: usize,
}

impl Default for EndpointConfig {
    fn default() -> EndpointConfig {
        EndpointConfig {
            ice_servers: None,
            host_ip: None,
            stun_gather_timeout: Duration::from_secs(5),
            connect_timeout: Some(Duration::from_secs(20)),
            failed_timeout: Duration::from_secs(25),
            send_buffer: 1024 * 1024,
            max_connections: 1024,
        }
    }
}

impl EndpointConfig {
    /// The ICE server URLs in effect.
    pub fn ice_server_urls(&self) -> Vec<String> {
        match &self.ice_servers {
            Some(v) => v.clone(),
            None => DEFAULT_ICE_SERVERS
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }

    /// The STUN servers to resolve, skipping URLs str0m can't use (TURN,
    /// STUN over TLS) and malformed ones.
    pub fn stun_servers(&self) -> Vec<stun::StunServer> {
        self.ice_server_urls()
            .iter()
            .filter_map(|u| stun::parse_stun_url(u).ok())
            .collect()
    }
}

/// Errors from the endpoint's API.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unknown webrtc connection")]
    UnknownConnection,
    #[error("too many webrtc connections")]
    TooManyConnections,
    #[error("no usable host candidate address")]
    NoHostCandidates,
    #[error("unsupported session description type {0:?}")]
    UnsupportedSdpType(SdpType),
    #[error("remote description already set")]
    RemoteDescriptionSet,
    #[error("received an offer after creating one")]
    UnexpectedOffer,
    #[error("received an answer without a pending offer")]
    NoPendingOffer,
    #[error("invalid session description: {0}")]
    Sdp(String),
    #[error("webrtc: {0}")]
    Rtc(String),
    #[error("data channel is not open")]
    NotOpen,
}

/// Why a connection ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// [`WebRtcEndpoint::close`].
    Local,
    /// The peer closed DTLS or the data channel.
    Remote,
    /// The data channel didn't open within `connect_timeout`.
    ConnectTimeout,
    /// ICE stayed disconnected for `failed_timeout`.
    IceFailed,
    /// str0m failed (handshake error, bad description, ...).
    Error(String),
}

/// What the endpoint tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Gathering finished and the description asked for with
    /// [`WebRtcEndpoint::poll_local_description`] is ready: poll again.
    LocalDescription(ConnId),
    /// The data channel opened. Add the vnic link now, with
    /// `LinkConfig::webrtc().with_mtu(mtu)`; `mtu` is the most
    /// [`WebRtcEndpoint::link_write`] puts in one message.
    Open { conn: ConnId, mtu: usize },
    /// One data-channel message, in order: the next bytes of the link's
    /// stream. Feed them to `Host::handle_link_input`.
    Data { conn: ConnId, data: Vec<u8> },
    /// The channel has room again after a short [`WebRtcEndpoint::link_write`].
    Writable(ConnId),
    /// The connection is gone and its id is no longer valid. Remove the
    /// vnic link (or fail the candidate if it never opened).
    Closed { conn: ConnId, reason: CloseReason },
}

/// A datagram to send: from the driver's socket bound to `source`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transmit {
    pub source: SocketAddr,
    pub destination: SocketAddr,
    pub contents: Vec<u8>,
}

/// Where a connection is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// No local description requested yet, or gathering still running.
    New,
    /// The local description is out; waiting for the other side and ICE,
    /// DTLS and SCTP.
    Connecting,
    /// The data channel is open.
    Open,
    /// Closing: draining the DTLS close.
    Closing,
}

#[derive(Debug)]
enum LocalDescription {
    NotRequested,
    Pending(SdpType),
    Ready(SessionDescription),
}

/// A binding request to a STUN server for one host socket.
#[derive(Debug)]
struct StunTxn {
    id: TransactionId,
    socket: SocketAddr,
    base: SocketAddr,
    server: SocketAddr,
    sent: u32,
    next: Time,
    deadline: Time,
}

/// STUN retransmission: RFC 8489's 500 ms RTO, doubling.
const STUN_RTO: Duration = Duration::from_millis(500);

/// Authenticated (local, remote) routes kept per connection: enough for a
/// peer's several candidates, and a bound on what one peer can add.
const ROUTES_PER_CONN: usize = 8;

struct Conn {
    rtc: Rtc,
    signaling: WebRtcSignaling,
    /// (socket, advertised host candidate address).
    host: Vec<(SocketAddr, SocketAddr)>,
    gather: Vec<StunTxn>,
    gathered: bool,
    local: LocalDescription,
    pending_offer: Option<SdpPendingOffer>,
    remote_offer: Option<SdpOffer>,
    channel: Option<ChannelId>,
    mtu: usize,
    open: bool,
    blocked: bool,
    rtc_deadline: Option<Time>,
    connect_deadline: Option<Time>,
    disconnected_at: Option<Time>,
    closing: Option<CloseReason>,
    last_now: Time,
    /// Entries in `WebRtcEndpoint::routes` naming this connection.
    routes: usize,
}

/// Every WebRTC connection of one node. The crate docs give the driver
/// contract.
// TODO: connection, open, close-reason and byte counters for the driver to
// export.
pub struct WebRtcEndpoint {
    config: EndpointConfig,
    clock: Clock,
    rng: DetRng,
    crypto: std::sync::Arc<str0m::config::CryptoProvider>,
    /// Shared (mux) sockets, the host candidates of every new connection.
    local_addrs: Vec<SocketAddr>,
    stun_servers: Vec<SocketAddr>,
    conns: SlotMap<ConnId, Conn>,
    /// (advertised local, remote) → connection, from authenticated STUN.
    /// Bounded by `ROUTES_PER_CONN` per connection.
    routes: BTreeMap<(SocketAddr, SocketAddr), ConnId>,
    /// Drained by the driver after every call; str0m bounds what one call
    /// produces.
    transmits: VecDeque<Transmit>,
    /// Drained by the driver after every call.
    events: VecDeque<Event>,
    keys: Vec<ConnId>,
}

impl std::fmt::Debug for WebRtcEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebRtcEndpoint")
            .field("local_addrs", &self.local_addrs)
            .field("connections", &self.conns.len())
            .finish()
    }
}

fn channel_config() -> ChannelConfig {
    let init = DATA_CHANNEL_INIT;
    ChannelConfig {
        label: init.label.to_owned(),
        ordered: init.ordered,
        reliability: Reliability::Reliable,
        negotiated: Some(init.id),
        protocol: init.protocol.to_owned(),
    }
}

/// The largest message the peer takes, from `a=max-message-size` (RFC
/// 8841: absent means 64 KiB, 0 means no limit), capped at vnic's MTU.
fn message_mtu(sdp: &str) -> usize {
    let remote = sdp
        .lines()
        .find_map(|l| l.trim().strip_prefix("a=max-message-size:"))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(64 * 1024);
    match usize::try_from(remote) {
        Ok(0) | Err(_) => DATA_CHANNEL_MTU,
        Ok(n) => n.min(DATA_CHANNEL_MTU),
    }
}

/// ICE credentials from the connection's rng, so they're reproducible: 16
/// and 32 characters, above RFC 8839's minimums of 4 and 22.
fn ice_creds(rng: &mut DetRng) -> IceCreds {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut s = |n: usize| -> String {
        (0..n)
            .map(|_| {
                let i = rand_core::Rng::next_u32(rng) as usize % ALPHABET.len();
                ALPHABET.get(i).map_or('a', |&c| char::from(c))
            })
            .collect()
    };
    let ufrag = s(16);
    let pass = s(32);
    IceCreds { ufrag, pass }
}

/// RFC 7983: a first byte in 0..=3 is STUN.
fn is_stun(data: &[u8]) -> bool {
    data.first().is_some_and(|&b| b <= 3)
}

impl WebRtcEndpoint {
    /// A new endpoint. `rng` seeds every connection's certificate, ICE
    /// credentials and DTLS randomness.
    pub fn new(now: Time, config: EndpointConfig, rng: DetRng) -> WebRtcEndpoint {
        let mut config = config;
        config.send_buffer = config.send_buffer.max(2 * DATA_CHANNEL_MTU);
        WebRtcEndpoint {
            config,
            clock: Clock::new(now),
            rng,
            crypto: crate::crypto::provider(),
            local_addrs: Vec::new(),
            stun_servers: Vec::new(),
            conns: SlotMap::with_key(),
            routes: BTreeMap::new(),
            transmits: VecDeque::new(),
            events: VecDeque::new(),
            keys: Vec::new(),
        }
    }

    pub fn config(&self) -> &EndpointConfig {
        &self.config
    }

    /// Adds a shared socket: every connection created afterwards gets a
    /// host candidate on it. The address must be the one datagrams arrive
    /// on: a concrete IP, or the wildcard with `host_ip` set.
    pub fn add_local_addr(&mut self, addr: SocketAddr) {
        if !self.local_addrs.contains(&addr) {
            self.local_addrs.push(addr);
        }
    }

    pub fn local_addrs(&self) -> &[SocketAddr] {
        &self.local_addrs
    }

    /// The resolved STUN servers of [`EndpointConfig::stun_servers`].
    /// Connections created afterwards gather server-reflexive candidates
    /// from them.
    pub fn set_stun_servers(&mut self, addrs: Vec<SocketAddr>) {
        self.stun_servers = addrs;
    }

    /// The host candidate address for a socket (NAT 1:1 mapping).
    fn advertise(&self, socket: SocketAddr) -> SocketAddr {
        match self.config.host_ip {
            Some(ip) if ip.is_ipv4() == socket.is_ipv4() => SocketAddr::new(ip, socket.port()),
            _ => socket,
        }
    }

    /// A connection on the shared sockets. Gathering starts at once.
    pub fn create_connection(&mut self, now: Time) -> Result<ConnId, Error> {
        let addrs = self.local_addrs.clone();
        self.create_connection_on(now, &addrs)
    }

    /// A connection on its own sockets. Datagrams for them still go to
    /// [`WebRtcEndpoint::handle_udp_input`].
    pub fn create_connection_on(
        &mut self,
        now: Time,
        sockets: &[SocketAddr],
    ) -> Result<ConnId, Error> {
        if self.conns.len() >= self.config.max_connections {
            return Err(Error::TooManyConnections);
        }
        let mut rng = fork(&mut self.rng);
        let identity = crate::cert::generate(&mut rng, now);
        // Keep str0m's defaults of DTLS 1.2 and SNAP off (no `a=sctp-init`):
        // the deployed network requires both.
        // TODO: raise the SCTP receive window to 8 MiB once str0m exposes it.
        let mut rtc = Rtc::builder()
            .set_crypto_provider(self.crypto.clone())
            .set_dtls_cert(DtlsCert {
                certificate: identity.certificate,
                private_key: identity.private_key,
            })
            .set_local_ice_credentials(ice_creds(&mut rng))
            .set_sctp_max_buffered_amount(self.config.send_buffer)
            .build(self.clock.instant(now));

        let mut host = Vec::new();
        for &socket in sockets {
            let advertised = self.advertise(socket);
            let Ok(c) = Candidate::host(advertised, "udp") else {
                continue;
            };
            if rtc.add_local_candidate(c).is_some() {
                host.push((socket, advertised));
            }
        }
        if host.is_empty() {
            return Err(Error::NoHostCandidates);
        }

        // TODO: cache server-reflexive mappings per shared socket, so later
        // connections on it skip gathering.
        let mut gather = Vec::new();
        for &(socket, base) in &host {
            for &server in &self.stun_servers {
                if server.is_ipv4() != socket.is_ipv4() {
                    continue;
                }
                let mut id = [0u8; 12];
                rand_core::Rng::fill_bytes(&mut rng, &mut id);
                gather.push(StunTxn {
                    id,
                    socket,
                    base,
                    server,
                    sent: 0,
                    next: now,
                    deadline: now + self.config.stun_gather_timeout,
                });
            }
        }

        let conn = Conn {
            rtc,
            signaling: WebRtcSignaling::new(),
            host,
            gathered: gather.is_empty(),
            gather,
            local: LocalDescription::NotRequested,
            pending_offer: None,
            remote_offer: None,
            channel: None,
            mtu: DATA_CHANNEL_MTU,
            open: false,
            blocked: false,
            rtc_deadline: None,
            connect_deadline: self.config.connect_timeout.map(|d| now + d),
            disconnected_at: None,
            closing: None,
            last_now: now,
            routes: 0,
        };
        let key = self.conns.insert(conn);
        self.send_stun_requests(key, now);
        self.drive(key);
        Ok(key)
    }

    /// The local description: an offer if no remote description was set
    /// yet, else the answer. The first call decides which. The description
    /// is ready once gathering completes, so it carries every candidate,
    /// and [`Event::LocalDescription`] says so. Send it as
    /// `description.to_link_description()`.
    pub fn poll_local_description(
        &mut self,
        now: Time,
        conn: ConnId,
    ) -> Poll<Result<SessionDescription, Error>> {
        let Some(c) = self.conns.get_mut(conn) else {
            return Poll::Ready(Err(Error::UnknownConnection));
        };
        c.last_now = c.last_now.max(now);
        if let LocalDescription::NotRequested = c.local {
            let t = c.signaling.create_local_description();
            c.local = LocalDescription::Pending(t);
        }
        if let Err(e) = self.build_local_description(conn) {
            return Poll::Ready(Err(e));
        }
        match self.conns.get(conn).map(|c| &c.local) {
            Some(LocalDescription::Ready(d)) => Poll::Ready(Ok(d.clone())),
            Some(_) => Poll::Pending,
            None => Poll::Ready(Err(Error::UnknownConnection)),
        }
    }

    /// Makes the offer or answer once gathering completed. A failure closes
    /// the connection.
    fn build_local_description(&mut self, key: ConnId) -> Result<(), Error> {
        let Some(c) = self.conns.get_mut(key) else {
            return Err(Error::UnknownConnection);
        };
        let LocalDescription::Pending(t) = c.local else {
            return Ok(());
        };
        if !c.gathered {
            return Ok(());
        }
        let result = match t {
            SdpType::Offer => {
                let mut api = c.rtc.sdp_api();
                let id = api.add_channel_with_config(channel_config());
                match api.apply() {
                    Some((offer, pending)) => {
                        c.channel = Some(id);
                        c.pending_offer = Some(pending);
                        Ok(offer.to_sdp_string())
                    }
                    None => Err(Error::Rtc("no offer to make".into())),
                }
            }
            _ => match c.remote_offer.take() {
                Some(offer) => match c.rtc.sdp_api().accept_offer(offer) {
                    Ok(answer) => {
                        c.channel = Some(c.rtc.direct_api().create_data_channel(channel_config()));
                        Ok(answer.to_sdp_string())
                    }
                    Err(e) => Err(Error::Sdp(e.to_string())),
                },
                None => Err(Error::Rtc("no remote offer to answer".into())),
            },
        };
        match result {
            Ok(sdp) => {
                c.local = LocalDescription::Ready(SessionDescription { sdp_type: t, sdp });
                self.events.push_back(Event::LocalDescription(key));
                self.drive(key);
                Ok(())
            }
            Err(e) => {
                self.fail(key, CloseReason::Error(e.to_string()));
                Err(e)
            }
        }
    }

    /// Applies the peer's description. `AwaitLink`: this side offered, so
    /// the link opens on its own ([`Event::Open`]). `AwaitInBackground`:
    /// this was an offer; poll the answer next.
    ///
    /// An offer is only parsed and stored here. str0m accepts it when
    /// [`WebRtcEndpoint::poll_local_description`] finds gathering complete,
    /// so the answer carries every candidate, and an offer that parses but
    /// can't be accepted fails at that poll.
    pub fn set_remote_description(
        &mut self,
        now: Time,
        conn: ConnId,
        desc: &SessionDescription,
    ) -> Result<RemoteDescriptionAction, Error> {
        let Some(c) = self.conns.get_mut(conn) else {
            return Err(Error::UnknownConnection);
        };
        c.last_now = c.last_now.max(now);
        if c.signaling.has_remote_description() {
            return Err(Error::RemoteDescriptionSet);
        }
        match desc.sdp_type {
            SdpType::Offer => {
                if !matches!(c.local, LocalDescription::NotRequested) {
                    return Err(Error::UnexpectedOffer);
                }
                let offer =
                    SdpOffer::from_sdp_string(&desc.sdp).map_err(|e| Error::Sdp(e.to_string()))?;
                c.remote_offer = Some(offer);
            }
            SdpType::Answer => {
                let answer =
                    SdpAnswer::from_sdp_string(&desc.sdp).map_err(|e| Error::Sdp(e.to_string()))?;
                let pending = c.pending_offer.take().ok_or(Error::NoPendingOffer)?;
                if let Err(e) = c.rtc.sdp_api().accept_answer(pending, answer) {
                    let e = Error::Sdp(e.to_string());
                    self.fail(conn, CloseReason::Error(e.to_string()));
                    return Err(e);
                }
            }
            t => return Err(Error::UnsupportedSdpType(t)),
        }
        c.mtu = message_mtu(&desc.sdp);
        let action = c.signaling.set_remote_description();
        self.drive(conn);
        Ok(action)
    }

    /// A datagram the driver read from the socket bound to `dst`. Returns
    /// whether a connection took it; the rest are dropped.
    pub fn handle_udp_input(
        &mut self,
        now: Time,
        src: SocketAddr,
        dst: SocketAddr,
        data: &[u8],
    ) -> bool {
        if let Some((txn, response)) = stun::decode_binding_response(data)
            && self.handle_gather_response(now, src, txn, response)
        {
            return true;
        }
        let local = self.advertise(dst);
        let Ok(receive) = Receive::new(Protocol::Udp, src, local, data) else {
            return false;
        };
        let input = Input::Receive(self.clock.instant(now), receive);
        let stun = is_stun(data);

        let routed = self
            .routes
            .get(&(local, src))
            .copied()
            .filter(|k| self.conns.get(*k).is_some_and(|c| c.rtc.accepts(&input)));
        let found = routed.or_else(|| {
            self.conns
                .iter()
                .find(|(_, c)| c.rtc.accepts(&input))
                .map(|(k, _)| k)
        });
        let Some(key) = found else {
            return false;
        };
        let Some(c) = self.conns.get_mut(key) else {
            return false;
        };
        c.last_now = c.last_now.max(now);
        if let Err(e) = c.rtc.handle_input(input) {
            self.fail(key, CloseReason::Error(e.to_string()));
            return true;
        }
        if stun {
            self.learn_route(local, src, key);
        }
        self.drive(key);
        true
    }

    /// Routes `(local, remote)` to the connection that just accepted
    /// authenticated STUN from it. The last one wins.
    fn learn_route(&mut self, local: SocketAddr, remote: SocketAddr, key: ConnId) {
        if self.routes.get(&(local, remote)) == Some(&key) {
            return;
        }
        if self
            .conns
            .get(key)
            .is_none_or(|c| c.routes >= ROUTES_PER_CONN)
        {
            return;
        }
        if let Some(old) = self.routes.insert((local, remote), key)
            && let Some(o) = self.conns.get_mut(old)
        {
            o.routes = o.routes.saturating_sub(1);
        }
        if let Some(c) = self.conns.get_mut(key) {
            c.routes += 1;
        }
    }

    fn handle_gather_response(
        &mut self,
        now: Time,
        src: SocketAddr,
        txn: TransactionId,
        response: BindingResponse,
    ) -> bool {
        let found = self.conns.iter_mut().find_map(|(k, c)| {
            let i = c
                .gather
                .iter()
                .position(|t| t.id == txn && t.server == src)?;
            Some((k, c.gather.swap_remove(i)))
        });
        let Some((key, t)) = found else {
            return false;
        };
        if let (Some(c), BindingResponse::Mapped(addr)) = (self.conns.get_mut(key), response) {
            c.last_now = c.last_now.max(now);
            // A mapping equal to the host address is redundant (RFC 8445
            // §5.1.3).
            if addr != t.base
                && let Ok(cand) = Candidate::server_reflexive(addr, t.base, "udp")
            {
                c.rtc.add_local_candidate(cand);
            }
        }
        self.finish_gathering(key);
        true
    }

    fn finish_gathering(&mut self, key: ConnId) {
        let Some(c) = self.conns.get_mut(key) else {
            return;
        };
        if c.gathered || !c.gather.is_empty() {
            return;
        }
        c.gathered = true;
        // Errors close the connection and surface at the next poll.
        let _ = self.build_local_description(key);
    }

    /// Sends STUN requests that are due, and drops expired ones.
    fn send_stun_requests(&mut self, key: ConnId, now: Time) {
        let Some(c) = self.conns.get_mut(key) else {
            return;
        };
        let transmits = &mut self.transmits;
        c.gather.retain_mut(|t| {
            if t.deadline <= now {
                return false;
            }
            if t.next <= now {
                transmits.push_back(Transmit {
                    source: t.socket,
                    destination: t.server,
                    contents: stun::encode_binding_request(&t.id).to_vec(),
                });
                t.next = (now + STUN_RTO * (1i64 << t.sent.min(16))).min(t.deadline);
                t.sent += 1;
            }
            true
        });
        self.finish_gathering(key);
    }

    /// The next datagram to send.
    pub fn poll_udp_transmit(&mut self) -> Option<Transmit> {
        self.transmits.pop_front()
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    /// The earliest deadline (rounded up to `Time::PRECISION`).
    pub fn poll_timeout(&self) -> Option<Time> {
        let failed = self.config.failed_timeout;
        self.conns
            .values()
            .flat_map(|c| {
                [
                    c.rtc_deadline,
                    c.gather.iter().map(|t| t.next.min(t.deadline)).min(),
                    c.connect_deadline.filter(|_| !c.open),
                    c.disconnected_at.map(|t| t + failed),
                ]
            })
            .flatten()
            .min()
            .map(ceil_precision)
    }

    pub fn handle_timeout(&mut self, now: Time) {
        let mut keys = std::mem::take(&mut self.keys);
        keys.clear();
        keys.extend(self.conns.keys());
        for &key in &keys {
            self.timeout_conn(key, now);
        }
        self.keys = keys;
    }

    fn timeout_conn(&mut self, key: ConnId, now: Time) {
        let failed = self.config.failed_timeout;
        let Some(c) = self.conns.get_mut(key) else {
            return;
        };
        c.last_now = c.last_now.max(now);
        if c.gather.iter().any(|t| t.next <= now || t.deadline <= now) {
            self.send_stun_requests(key, now);
        }
        let Some(c) = self.conns.get_mut(key) else {
            return;
        };
        if !c.open && c.connect_deadline.is_some_and(|d| d <= now) {
            self.fail(key, CloseReason::ConnectTimeout);
            return;
        }
        if c.disconnected_at.is_some_and(|t| t + failed <= now) {
            self.fail(key, CloseReason::IceFailed);
            return;
        }
        if c.rtc_deadline.is_some_and(|d| d <= now) {
            let instant = self.clock.instant(now);
            if let Err(e) = c.rtc.handle_input(Input::Timeout(instant)) {
                self.fail(key, CloseReason::Error(e.to_string()));
                return;
            }
            self.drive(key);
        }
    }

    /// Writes link bytes: as many whole messages of at most the link MTU as
    /// the send buffer takes, returning the bytes accepted. A short count
    /// means the buffer is full; [`Event::Writable`] follows when it has
    /// drained to half.
    ///
    /// Readers treat the link as a byte stream, so a write may coalesce
    /// frames or split them across messages; no message exceeds the link
    /// MTU (at most [`DATA_CHANNEL_MTU`]).
    pub fn link_write(&mut self, conn: ConnId, data: &[u8]) -> Result<usize, Error> {
        let Some(c) = self.conns.get_mut(conn) else {
            return Err(Error::UnknownConnection);
        };
        let (true, Some(id), None) = (c.open, c.channel, &c.closing) else {
            return Err(Error::NotOpen);
        };
        let mtu = c.mtu;
        let mut written = 0;
        let mut result = Ok(());
        for chunk in data.chunks(mtu) {
            let Some(mut ch) = c.rtc.channel(id) else {
                result = Err(Error::NotOpen);
                break;
            };
            match ch.write(true, chunk) {
                Ok(true) => written += chunk.len(),
                Ok(false) => {
                    c.blocked = true;
                    break;
                }
                Err(e) => {
                    result = Err(Error::Rtc(e.to_string()));
                    break;
                }
            }
        }
        self.drive(conn);
        if let Err(e) = result
            && written == 0
        {
            return Err(e);
        }
        Ok(written)
    }

    /// The most bytes [`WebRtcEndpoint::link_write`] sends in one message.
    pub fn link_mtu(&self, conn: ConnId) -> Option<usize> {
        self.conns.get(conn).map(|c| c.mtu)
    }

    /// Bytes written but not yet acknowledged by the peer.
    pub fn buffered_amount(&mut self, conn: ConnId) -> usize {
        let Some(c) = self.conns.get_mut(conn) else {
            return 0;
        };
        match c.channel {
            Some(id) => c.rtc.channel(id).map_or(0, |mut ch| ch.buffered_amount()),
            None => 0,
        }
    }

    pub fn state(&self, conn: ConnId) -> Option<ConnectionState> {
        let c = self.conns.get(conn)?;
        Some(if c.closing.is_some() {
            ConnectionState::Closing
        } else if c.open {
            ConnectionState::Open
        } else if matches!(c.local, LocalDescription::Ready(_)) {
            ConnectionState::Connecting
        } else {
            ConnectionState::New
        })
    }

    pub fn connections(&self) -> impl Iterator<Item = ConnId> + '_ {
        self.conns.keys()
    }

    /// Closes a connection: SCTP shutdown and DTLS close_notify, then
    /// [`Event::Closed`] with [`CloseReason::Local`].
    pub fn close(&mut self, now: Time, conn: ConnId) {
        let Some(c) = self.conns.get_mut(conn) else {
            return;
        };
        c.last_now = c.last_now.max(now);
        if c.closing.is_some() {
            return;
        }
        c.closing = Some(CloseReason::Local);
        if c.rtc.close().is_err() {
            c.rtc.disconnect();
        }
        self.drive(conn);
    }

    /// Ends a connection at once, without a close handshake.
    fn fail(&mut self, key: ConnId, reason: CloseReason) {
        let Some(c) = self.conns.get_mut(key) else {
            return;
        };
        c.closing.get_or_insert(reason);
        c.rtc.disconnect();
        self.remove(key);
    }

    fn remove(&mut self, key: ConnId) {
        let Some(c) = self.conns.remove(key) else {
            return;
        };
        self.routes.retain(|_, k| *k != key);
        self.events.push_back(Event::Closed {
            conn: key,
            reason: c.closing.unwrap_or(CloseReason::Remote),
        });
    }

    /// Runs str0m until it has nothing more to say: queues its datagrams
    /// and events, and records its next deadline.
    fn drive(&mut self, key: ConnId) {
        loop {
            let Some(c) = self.conns.get_mut(key) else {
                return;
            };
            match c.rtc.poll_output() {
                Ok(Output::Transmit(t)) => {
                    let source = c
                        .host
                        .iter()
                        .find(|(_, adv)| *adv == t.source)
                        .map_or(t.source, |(socket, _)| *socket);
                    self.transmits.push_back(Transmit {
                        source,
                        destination: t.destination,
                        contents: t.contents.into(),
                    });
                }
                Ok(Output::Event(e)) => self.handle_rtc_event(key, e),
                Ok(Output::Timeout(t)) => {
                    c.rtc_deadline = self.clock.time(t);
                    break;
                }
                Err(e) => {
                    self.fail(key, CloseReason::Error(e.to_string()));
                    return;
                }
            }
        }
        if self.conns.get(key).is_some_and(|c| !c.rtc.is_alive()) {
            self.remove(key);
        }
    }

    fn handle_rtc_event(&mut self, key: ConnId, e: str0m::Event) {
        let Some(c) = self.conns.get_mut(key) else {
            return;
        };
        let ours = |id: ChannelId| c.channel == Some(id);
        match e {
            str0m::Event::IceConnectionStateChange(s) => match s {
                IceConnectionState::Disconnected => {
                    c.disconnected_at.get_or_insert(c.last_now);
                }
                IceConnectionState::Checking
                | IceConnectionState::Connected
                | IceConnectionState::Completed => c.disconnected_at = None,
                _ => {}
            },
            str0m::Event::ChannelOpen(id, _) if ours(id) => {
                c.open = true;
                if let Some(mut ch) = c.rtc.channel(id) {
                    ch.set_buffered_amount_low_threshold(self.config.send_buffer / 2);
                }
                let mtu = c.mtu;
                self.events.push_back(Event::Open { conn: key, mtu });
            }
            str0m::Event::ChannelData(d) if ours(d.id) => {
                self.events.push_back(Event::Data {
                    conn: key,
                    data: d.data,
                });
            }
            str0m::Event::ChannelBufferedAmountLow(id) if ours(id) => {
                if c.blocked {
                    c.blocked = false;
                    self.events.push_back(Event::Writable(key));
                }
            }
            str0m::Event::ChannelClose(id) if ours(id) => {
                // The channel is the connection's only use: closing it must
                // also stop ICE, DTLS and SCTP.
                if c.closing.is_none() {
                    c.closing = Some(CloseReason::Remote);
                    if c.rtc.close().is_err() {
                        c.rtc.disconnect();
                    }
                }
            }
            str0m::Event::Closed => {
                c.closing.get_or_insert(CloseReason::Remote);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_message_size() {
        assert_eq!(message_mtu("v=0\r\n"), 64 * 1024);
        assert_eq!(
            message_mtu("a=max-message-size:262144\r\n"),
            DATA_CHANNEL_MTU
        );
        assert_eq!(message_mtu("a=max-message-size:1200\r\n"), 1200);
        assert_eq!(message_mtu("a=max-message-size:0\r\n"), DATA_CHANNEL_MTU);
    }

    #[test]
    fn channel_config_is_data_channel_init() {
        let c = channel_config();
        assert_eq!(c.label, "data");
        assert_eq!(c.negotiated, Some(1));
        assert!(c.ordered);
        assert_eq!(c.reliability, Reliability::Reliable);
        assert_eq!(c.protocol, "");
    }
}
